/// 用眼时长记录 & 查询服务
/// 对应原始项目的 EyeTimeRecorder.js + EyeTimeQuery.js
use crate::db;
use crate::kv;
use crate::models::*;
use crate::utils::{self, TimezoneHelper};
use std::collections::HashMap;
use worker::*;

// ==================== 用眼时长记录 ====================

/// 记录设备活动状态（用眼时长追踪）
pub async fn record_activity(
    d1: &D1Database,
    kv_store: &kv::KvStore,
    device_id: &str,
    is_active: bool,
    offset_hours: i32,
    now_ms: u64,
) -> Result<()> {
    // 获取之前的全局状态
    let mut global_state = kv::get_eye_global_state(kv_store).await?;
    let was_global_active = global_state.active;

    // 更新设备状态
    kv::set_eye_device_state(
        kv_store,
        device_id,
        &EyeDeviceState {
            is_active,
            last_update_time: now_ms,
        },
    )
    .await?;
    kv::ensure_eye_device_registered(kv_store, device_id).await?;

    // 重新计算全局活跃状态
    let eye_device_ids = kv::get_eye_device_ids(kv_store).await?;
    let mut new_global_active = false;
    for did in &eye_device_ids {
        if let Some(state) = kv::get_eye_device_state(kv_store, did).await? {
            if state.is_active {
                new_global_active = true;
                break;
            }
        }
    }

    let tz = TimezoneHelper::new(offset_hours);

    // 全局从活跃变为非活跃：记录最后一段时间
    if was_global_active && !new_global_active {
        if let Some(last_time) = global_state.last_record_time {
            save_eye_time(d1, last_time, now_ms, &tz).await?;
        }
        global_state.active = false;
        global_state.last_record_time = None;
    }
    // 全局从非活跃变为活跃：开始新记录
    else if !was_global_active && new_global_active {
        global_state.active = true;
        global_state.last_record_time = Some(now_ms);
    }
    // 全局持续活跃：记录这段时间并更新起始时间
    else if was_global_active && new_global_active {
        if let Some(last_time) = global_state.last_record_time {
            save_eye_time(d1, last_time, now_ms, &tz).await?;
        }
        global_state.active = true;
        global_state.last_record_time = Some(now_ms);
    }

    kv::set_eye_global_state(kv_store, &global_state).await?;
    Ok(())
}

/// 保存使用时间到数据库
async fn save_eye_time(
    d1: &D1Database,
    start_ms: u64,
    end_ms: u64,
    tz: &TimezoneHelper,
) -> Result<()> {
    if end_ms <= start_ms {
        return Ok(());
    }

    // 将时间段按小时和日期拆分
    let segments = split_time_segments(start_ms, end_ms, tz);

    for (date_str, hour, minutes) in &segments {
        update_eye_time_record(d1, date_str, *hour, *minutes).await?;
    }

    Ok(())
}

/// 将时间段按本地时区的日期和小时拆分
fn split_time_segments(
    start_ms: u64,
    end_ms: u64,
    tz: &TimezoneHelper,
) -> Vec<(String, usize, f64)> {
    let mut segments = Vec::new();
    let mut current_ms = start_ms;

    while current_ms < end_ms {
        let local_date = tz.get_local_date(current_ms);
        let current_hour = tz.get_local_hour(current_ms) as usize;
        let current_minute = tz.get_local_minute(current_ms) as f64;
        let current_second = tz.get_local_second(current_ms) as f64;

        // 计算到下一个小时的剩余毫秒
        let ms_in_current_pos =
            ((current_minute * 60.0 + current_second) * 1000.0) as u64;
        let ms_to_next_hour = 3600 * 1000 - ms_in_current_pos;

        // 确定这段时间的结束
        let segment_end = end_ms.min(current_ms + ms_to_next_hour);
        let duration_ms = segment_end - current_ms;
        let minutes = duration_ms as f64 / 60000.0;

        let date_str = utils::format_date(&local_date);
        segments.push((date_str, current_hour, minutes));

        current_ms = segment_end;
    }

    segments
}

/// 更新数据库中的用眼时长记录
async fn update_eye_time_record(
    d1: &D1Database,
    date: &str,
    hour: usize,
    minutes: f64,
) -> Result<()> {
    let existing = db::find_eye_time(d1, date).await?;
    let mut hourly = match &existing {
        Some(row) => serde_json::from_str(&row.hourly_usage).unwrap_or_else(|_| vec![0.0; 24]),
        None => vec![0.0; 24],
    };

    if hour < 24 {
        hourly[hour] += minutes;
    }

    db::upsert_eye_time(d1, date, &hourly).await
}

// ==================== 用眼时长查询 ====================

/// 获取某日的用眼分钟数
pub async fn get_daily_minutes(
    d1: &D1Database,
    date: &str,
    offset_hours: i32,
) -> Result<EyeTimeDailyResult> {
    let rows = db::query_eye_time_by_range(d1, date, date).await?;

    let mut hourly_stats = vec![0.0_f64; 24];
    for row in &rows {
        let model = row.to_model();
        for (i, &v) in model.hourly_usage.iter().enumerate() {
            if i < 24 {
                hourly_stats[i] += v;
            }
        }
    }

    let total_usage: f64 = hourly_stats.iter().sum();
    let total_usage = (total_usage * 100.0).round() / 100.0;
    let hourly_stats: Vec<f64> = hourly_stats
        .iter()
        .map(|v| (v * 100.0).round() / 100.0)
        .collect();

    Ok(EyeTimeDailyResult {
        date: date.to_string(),
        total_usage,
        hourly_stats,
        timezone_offset: offset_hours,
    })
}

/// 获取周用眼统计
pub async fn get_weekly_minutes(
    d1: &D1Database,
    week_offset: i32,
    offset_hours: i32,
    now_ms: u64,
) -> Result<EyeTimeRangeResult> {
    let (start, end) = utils::get_week_range(offset_hours, week_offset, now_ms);
    let start_str = utils::format_date(&start);
    let end_str = utils::format_date(&end);

    let rows = db::query_eye_time_by_range(d1, &start_str, &end_str).await?;

    // 按日期聚合
    let mut daily_totals: HashMap<String, f64> = HashMap::new();
    for row in &rows {
        let model = row.to_model();
        let total: f64 = model.hourly_usage.iter().sum();
        *daily_totals.entry(model.date).or_insert(0.0) += total;
    }

    Ok(EyeTimeRangeResult {
        week_offset: Some(week_offset),
        month_offset: None,
        week_range: Some(DateRange {
            start: start_str,
            end: end_str,
        }),
        month_range: None,
        daily_totals,
        timezone_offset: offset_hours,
    })
}

/// 获取月用眼统计
pub async fn get_monthly_minutes(
    d1: &D1Database,
    month_offset: i32,
    offset_hours: i32,
    now_ms: u64,
) -> Result<EyeTimeRangeResult> {
    let (start, end) = utils::get_month_range(offset_hours, month_offset, now_ms);
    let start_str = utils::format_date(&start);
    let end_str = utils::format_date(&end);

    let rows = db::query_eye_time_by_range(d1, &start_str, &end_str).await?;

    // 按日期聚合
    let mut daily_totals: HashMap<String, f64> = HashMap::new();
    for row in &rows {
        let model = row.to_model();
        let total: f64 = model.hourly_usage.iter().sum();
        *daily_totals.entry(model.date).or_insert(0.0) += total;
    }

    Ok(EyeTimeRangeResult {
        week_offset: None,
        month_offset: Some(month_offset),
        week_range: None,
        month_range: Some(DateRange {
            start: start_str,
            end: end_str,
        }),
        daily_totals,
        timezone_offset: offset_hours,
    })
}
