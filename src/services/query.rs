/// 统计查询服务 - 查询应用使用统计
/// 对应原始项目的 StatsQuery.js
use crate::db;
use crate::kv;
use crate::models::*;
use crate::utils;
use std::collections::HashMap;
use worker::*;

/// 处理统计数据，聚合计算
fn process_stats(rows: &[DailyStatRow]) -> StatsResult {
    let mut daily_stats: HashMap<String, f64> = HashMap::new();
    let mut app_stats: HashMap<String, f64> = HashMap::new();
    let mut hourly_stats = vec![0.0_f64; 24];
    let mut app_hourly_stats: HashMap<String, Vec<f64>> = HashMap::new();
    let mut total_usage = 0.0_f64;

    for row in rows {
        let stat = row.to_model();
        let date_key = &stat.date;
        let app_name = &stat.app_name;

        // 初始化应用的小时统计
        if !app_hourly_stats.contains_key(app_name) {
            app_hourly_stats.insert(app_name.clone(), vec![0.0; 24]);
        }

        for (hour, &minutes) in stat.hourly_usage.iter().enumerate() {
            if minutes > 0.0 {
                *daily_stats.entry(date_key.clone()).or_insert(0.0) += minutes;
                *app_stats.entry(app_name.clone()).or_insert(0.0) += minutes;
                hourly_stats[hour] += minutes;
                if let Some(app_hourly) = app_hourly_stats.get_mut(app_name) {
                    app_hourly[hour] += minutes;
                }
                total_usage += minutes;
            }
        }
    }

    // 只保留有数据的应用
    let filtered_app_stats: HashMap<String, f64> =
        app_stats.into_iter().filter(|(_, v)| *v > 0.0).collect();

    let filtered_app_hourly: HashMap<String, Vec<f64>> = app_hourly_stats
        .into_iter()
        .filter(|(name, _)| filtered_app_stats.contains_key(name))
        .collect();

    StatsResult {
        total_usage,
        app_stats: filtered_app_stats,
        hourly_stats,
        app_hourly_stats: filtered_app_hourly,
    }
}

/// 处理范围统计数据（周/月）
fn process_range_stats(
    rows: &[DailyStatRow],
) -> (HashMap<String, f64>, HashMap<String, HashMap<String, f64>>) {
    let mut daily_totals: HashMap<String, f64> = HashMap::new();
    let mut app_daily_stats: HashMap<String, HashMap<String, f64>> = HashMap::new();

    for row in rows {
        let stat = row.to_model();
        let total_for_day: f64 = stat.hourly_usage.iter().sum();

        if total_for_day > 0.0 {
            *daily_totals.entry(stat.date.clone()).or_insert(0.0) += total_for_day;
            let app_daily = app_daily_stats
                .entry(stat.app_name.clone())
                .or_insert_with(HashMap::new);
            *app_daily.entry(stat.date.clone()).or_insert(0.0) += total_for_day;
        }
    }

    (daily_totals, app_daily_stats)
}

// ==================== 公开接口 ====================

/// 获取某天的统计数据（单设备）
pub async fn get_daily_stats(
    d1: &D1Database,
    device_id: &str,
    date: &str,
) -> Result<StatsResult> {
    let rows = db::query_stats_by_range(d1, device_id, date, date).await?;

    if rows.is_empty() {
        return Ok(StatsResult::empty());
    }

    Ok(process_stats(&rows))
}

/// 获取某天所有设备的统计数据
pub async fn get_daily_stats_all_devices(d1: &D1Database, date: &str) -> Result<StatsResult> {
    let rows = db::query_stats_all_devices_by_range(d1, date, date).await?;

    if rows.is_empty() {
        return Ok(StatsResult::empty());
    }

    Ok(process_stats(&rows))
}

/// 获取周统计数据（单设备）
pub async fn get_weekly_stats(
    d1: &D1Database,
    device_id: &str,
    app_name: Option<&str>,
    week_offset: i32,
    offset_hours: i32,
    now_ms: u64,
) -> Result<RangeStatsResult> {
    let (start, end) = utils::get_week_range(offset_hours, week_offset, now_ms);
    let start_str = utils::format_date(&start);
    let end_str = utils::format_date(&end);

    let rows = match app_name {
        Some(app) => {
            db::query_stats_by_range_with_app(d1, device_id, &start_str, &end_str, app).await?
        }
        None => db::query_stats_by_range(d1, device_id, &start_str, &end_str).await?,
    };

    let (daily_totals, app_daily_stats) = process_range_stats(&rows);

    let filtered_app_stats = match app_name {
        Some(app) => {
            let mut m = HashMap::new();
            if let Some(stats) = app_daily_stats.get(app) {
                m.insert(app.to_string(), stats.clone());
            }
            m
        }
        None => app_daily_stats,
    };

    Ok(RangeStatsResult {
        week_offset: Some(week_offset),
        month_offset: None,
        week_range: Some(DateRange {
            start: start_str,
            end: end_str,
        }),
        month_range: None,
        daily_totals,
        app_daily_stats: filtered_app_stats,
    })
}

/// 获取周统计数据（所有设备）
pub async fn get_weekly_stats_all_devices(
    d1: &D1Database,
    app_name: Option<&str>,
    week_offset: i32,
    offset_hours: i32,
    now_ms: u64,
) -> Result<RangeStatsResult> {
    let (start, end) = utils::get_week_range(offset_hours, week_offset, now_ms);
    let start_str = utils::format_date(&start);
    let end_str = utils::format_date(&end);

    let rows = match app_name {
        Some(app) => {
            db::query_stats_all_devices_by_range_with_app(d1, &start_str, &end_str, app).await?
        }
        None => db::query_stats_all_devices_by_range(d1, &start_str, &end_str).await?,
    };

    let (daily_totals, app_daily_stats) = process_range_stats(&rows);

    let filtered_app_stats = match app_name {
        Some(app) => {
            let mut m = HashMap::new();
            if let Some(stats) = app_daily_stats.get(app) {
                m.insert(app.to_string(), stats.clone());
            }
            m
        }
        None => app_daily_stats,
    };

    Ok(RangeStatsResult {
        week_offset: Some(week_offset),
        month_offset: None,
        week_range: Some(DateRange {
            start: start_str,
            end: end_str,
        }),
        month_range: None,
        daily_totals,
        app_daily_stats: filtered_app_stats,
    })
}

/// 获取月统计数据（单设备）
pub async fn get_monthly_stats(
    d1: &D1Database,
    device_id: &str,
    app_name: Option<&str>,
    month_offset: i32,
    offset_hours: i32,
    now_ms: u64,
) -> Result<RangeStatsResult> {
    let (start, end) = utils::get_month_range(offset_hours, month_offset, now_ms);
    let start_str = utils::format_date(&start);
    let end_str = utils::format_date(&end);

    let rows = match app_name {
        Some(app) => {
            db::query_stats_by_range_with_app(d1, device_id, &start_str, &end_str, app).await?
        }
        None => db::query_stats_by_range(d1, device_id, &start_str, &end_str).await?,
    };

    let (daily_totals, app_daily_stats) = process_range_stats(&rows);

    let filtered_app_stats = match app_name {
        Some(app) => {
            let mut m = HashMap::new();
            if let Some(stats) = app_daily_stats.get(app) {
                m.insert(app.to_string(), stats.clone());
            }
            m
        }
        None => app_daily_stats,
    };

    Ok(RangeStatsResult {
        week_offset: None,
        month_offset: Some(month_offset),
        week_range: None,
        month_range: Some(DateRange {
            start: start_str,
            end: end_str,
        }),
        daily_totals,
        app_daily_stats: filtered_app_stats,
    })
}

/// 获取月统计数据（所有设备）
pub async fn get_monthly_stats_all_devices(
    d1: &D1Database,
    app_name: Option<&str>,
    month_offset: i32,
    offset_hours: i32,
    now_ms: u64,
) -> Result<RangeStatsResult> {
    let (start, end) = utils::get_month_range(offset_hours, month_offset, now_ms);
    let start_str = utils::format_date(&start);
    let end_str = utils::format_date(&end);

    let rows = match app_name {
        Some(app) => {
            db::query_stats_all_devices_by_range_with_app(d1, &start_str, &end_str, app).await?
        }
        None => db::query_stats_all_devices_by_range(d1, &start_str, &end_str).await?,
    };

    let (daily_totals, app_daily_stats) = process_range_stats(&rows);

    let filtered_app_stats = match app_name {
        Some(app) => {
            let mut m = HashMap::new();
            if let Some(stats) = app_daily_stats.get(app) {
                m.insert(app.to_string(), stats.clone());
            }
            m
        }
        None => app_daily_stats,
    };

    Ok(RangeStatsResult {
        week_offset: None,
        month_offset: Some(month_offset),
        week_range: None,
        month_range: Some(DateRange {
            start: start_str,
            end: end_str,
        }),
        daily_totals,
        app_daily_stats: filtered_app_stats,
    })
}

/// 获取设备列表（从 KV 获取设备信息）
pub async fn get_devices(
    kv_store: &kv::KvStore,
) -> Result<Vec<DeviceInfo>> {
    let device_ids = kv::get_device_list(kv_store).await?;
    let mut devices = Vec::new();

    for device_id in device_ids {
        let battery = kv::get_battery_info(kv_store, &device_id).await?;
        let switches = kv::get_recent_switches(kv_store, &device_id).await?;

        let (current_app, current_package, running, running_since) = if let Some(last) =
            switches.first()
        {
            (
                last.app_name.clone(),
                last.package_name.clone(),
                last.running,
                last.timestamp.clone(),
            )
        } else {
            (
                "Unknown".to_string(),
                None,
                false,
                utils::utc_ms_to_iso(0),
            )
        };

        devices.push(DeviceInfo {
            device: device_id,
            current_app,
            current_package_name: current_package,
            running,
            running_since,
            battery_level: battery.level,
            is_charging: battery.is_charging,
            battery_timestamp: battery.timestamp,
        });
    }

    Ok(devices)
}
