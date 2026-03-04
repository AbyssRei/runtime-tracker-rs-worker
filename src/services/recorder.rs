/// 统计记录服务 - 记录应用使用时间
/// 对应原始项目的 StatsRecorder.js
use crate::db;
use crate::kv;
use crate::models::*;
use crate::services::eyetime;
use crate::utils::{self, TimezoneHelper};
use worker::*;

/// 记录电池信息
pub async fn record_battery(
    kv_store: &kv::KvStore,
    device_id: &str,
    level: u8,
    is_charging: bool,
    now_ms: u64,
) -> Result<()> {
    let info = BatteryInfo {
        level,
        is_charging,
        timestamp: Some(utils::utc_ms_to_iso(now_ms)),
    };
    kv::set_battery_info(kv_store, device_id, &info).await
}

/// 记录应用使用情况
/// 核心逻辑：计算应用切换的时间差，分配到每个小时的使用时间
pub async fn record_usage(
    d1: &D1Database,
    kv_store: &kv::KvStore,
    device_id: &str,
    app_name: Option<&str>,
    running: Option<bool>,
    package_name: Option<&str>,
    offset_hours: i32,
    now_ms: u64,
) -> Result<()> {
    let tz = TimezoneHelper::new(offset_hours);
    let now_iso = utils::utc_ms_to_iso(now_ms);

    // 记录用眼时长
    let is_active = running.unwrap_or(true);
    eyetime::record_activity(d1, kv_store, device_id, is_active, offset_hours, now_ms).await?;

    // 获取现有切换记录
    let mut switches = kv::get_recent_switches(kv_store, device_id).await?;

    // 处理停止运行的情况
    if running == Some(false) {
        if !switches.is_empty() {
            let last_switch = &switches[0];
            if last_switch.running {
                // 计算上一个应用的使用时间
                let last_ts = parse_iso_to_ms(&last_switch.timestamp);
                let minutes = utils::calculate_precise_minutes(last_ts, now_ms);
                update_daily_stat(
                    d1,
                    device_id,
                    &last_switch.app_name,
                    last_switch.package_name.as_deref().unwrap_or(""),
                    last_ts,
                    minutes,
                    &tz,
                )
                .await?;
            }

            // 标记上一条记录为已停止，并添加待机记录
            if let Some(first) = switches.first_mut() {
                first.running = false;
            }
            switches.insert(
                0,
                AppSwitch {
                    app_name: "设备待机".to_string(),
                    package_name: None,
                    timestamp: now_iso,
                    running: false,
                },
            );

            // 保留最多 20 条
            switches.truncate(20);
            kv::set_recent_switches(kv_store, device_id, &switches).await?;
        }
        return Ok(());
    }

    // 处理正在运行的情况
    let app_name = app_name.unwrap_or("Unknown");

    // 如果有上一条记录且正在运行，计算使用时间
    if !switches.is_empty() {
        let last_switch = &switches[0];
        if last_switch.running {
            let last_ts = parse_iso_to_ms(&last_switch.timestamp);
            let minutes = utils::calculate_precise_minutes(last_ts, now_ms);
            update_daily_stat(
                d1,
                device_id,
                &last_switch.app_name,
                last_switch.package_name.as_deref().unwrap_or(""),
                last_ts,
                minutes,
                &tz,
            )
            .await?;
        }
    }

    // 添加新的切换记录
    switches.insert(
        0,
        AppSwitch {
            app_name: app_name.to_string(),
            package_name: package_name.map(|s| s.to_string()),
            timestamp: now_iso,
            running: true,
        },
    );

    // 保留最多 20 条
    switches.truncate(20);
    kv::set_recent_switches(kv_store, device_id, &switches).await?;

    Ok(())
}

/// 更新每日统计记录
async fn update_daily_stat(
    d1: &D1Database,
    device_id: &str,
    app_name: &str,
    package_name: &str,
    start_ms: u64,
    duration_minutes: f64,
    tz: &TimezoneHelper,
) -> Result<()> {
    if duration_minutes <= 0.0 {
        return Ok(());
    }

    // 获取本地日期
    let local_date = tz.get_local_date(start_ms);
    let date_str = utils::format_date(&local_date);

    // 查找或创建统计记录
    let existing = db::find_daily_stat(d1, device_id, &date_str, app_name, package_name).await?;
    let mut hourly = match &existing {
        Some(row) => serde_json::from_str(&row.hourly_usage).unwrap_or_else(|_| vec![0.0; 24]),
        None => vec![0.0; 24],
    };

    // 分配使用时间到各小时
    distribute_minutes(&mut hourly, start_ms, duration_minutes, tz, d1, device_id, app_name, package_name).await?;

    // 保存
    db::upsert_daily_stat(d1, device_id, &date_str, app_name, package_name, &hourly).await
}

/// 将使用时间精确分配到各小时段（支持跨小时、跨日）
async fn distribute_minutes(
    hourly: &mut Vec<f64>,
    start_ms: u64,
    total_minutes: f64,
    tz: &TimezoneHelper,
    d1: &D1Database,
    device_id: &str,
    app_name: &str,
    package_name: &str,
) -> Result<()> {
    let mut remaining = total_minutes;
    let mut current_ms = start_ms;
    let start_date = tz.get_local_date(start_ms);

    while remaining > 0.01 {
        let current_date = tz.get_local_date(current_ms);
        let current_hour = tz.get_local_hour(current_ms) as usize;
        let current_minute = tz.get_local_minute(current_ms) as f64;
        let current_second = tz.get_local_second(current_ms) as f64;

        // 计算到下一个小时还有多少分钟
        let minutes_to_next_hour = 60.0 - current_minute - (current_second / 60.0);

        // 如果跨日了，需要操作不同日期的记录
        if current_date != start_date {
            let cross_date_str = utils::format_date(&current_date);
            let existing =
                db::find_daily_stat(d1, device_id, &cross_date_str, app_name, package_name)
                    .await?;
            let mut cross_hourly = match &existing {
                Some(row) => {
                    serde_json::from_str(&row.hourly_usage).unwrap_or_else(|_| vec![0.0; 24])
                }
                None => vec![0.0; 24],
            };

            let available = (60.0_f64 - cross_hourly[current_hour]).max(0.0);
            let to_add = remaining.min(minutes_to_next_hour).min(available);

            if to_add > 0.0 {
                let precise = (to_add * 100.0).round() / 100.0;
                cross_hourly[current_hour] =
                    ((cross_hourly[current_hour] + precise) * 100.0).round() / 100.0;
                db::upsert_daily_stat(
                    d1,
                    device_id,
                    &cross_date_str,
                    app_name,
                    package_name,
                    &cross_hourly,
                )
                .await?;
                remaining = ((remaining - precise) * 100.0).round() / 100.0;
            }
        } else {
            // 同日：直接修改当前 hourly 数组
            let available = (60.0 - hourly[current_hour]).max(0.0);
            let to_add = remaining.min(minutes_to_next_hour).min(available);

            if to_add > 0.0 {
                let precise = (to_add * 100.0).round() / 100.0;
                hourly[current_hour] = ((hourly[current_hour] + precise) * 100.0).round() / 100.0;
                remaining = ((remaining - precise) * 100.0).round() / 100.0;
            }
        }

        // 移动到下一个小时的开始
        if remaining > 0.01 {
            let ms_to_next_hour = (minutes_to_next_hour * 60000.0) as u64;
            current_ms += ms_to_next_hour;
        } else {
            break;
        }

        // 防止无限循环（最多处理 30 天）
        if current_ms - start_ms > 30 * 24 * 3600 * 1000 {
            console_log!(
                "超过30天限制，剩余 {} 分钟无法分配，设备: {}, 应用: {}",
                remaining, device_id, app_name
            );
            break;
        }
    }

    Ok(())
}

/// 解析 ISO 时间戳为 UTC 毫秒
fn parse_iso_to_ms(iso: &str) -> u64 {
    // 简化解析：支持 "YYYY-MM-DDTHH:MM:SS.mmmZ" 格式
    let s = iso.trim_end_matches('Z');
    let parts: Vec<&str> = s.split('T').collect();
    if parts.len() != 2 {
        return 0;
    }

    let date_parts: Vec<i64> = parts[0].split('-').filter_map(|p| p.parse().ok()).collect();
    if date_parts.len() != 3 {
        return 0;
    }

    let time_str = parts[1];
    let (time_main, millis) = if let Some(dot_pos) = time_str.find('.') {
        let ms_str = &time_str[dot_pos + 1..];
        let ms = ms_str.parse::<u64>().unwrap_or(0);
        (&time_str[..dot_pos], ms)
    } else {
        (time_str, 0u64)
    };

    let time_parts: Vec<i64> = time_main.split(':').filter_map(|p| p.parse().ok()).collect();
    if time_parts.len() != 3 {
        return 0;
    }

    let year = date_parts[0];
    let month = date_parts[1];
    let day = date_parts[2];
    let hour = time_parts[0];
    let minute = time_parts[1];
    let second = time_parts[2];

    // 简化的日期计算（足够精确用于分钟级差值）
    let date = chrono::NaiveDate::from_ymd_opt(year as i32, month as u32, day as u32);
    if let Some(d) = date {
        let epoch = chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
        let days = (d - epoch).num_days();
        let total_secs = days * 86400 + hour * 3600 + minute * 60 + second;
        return (total_secs as u64) * 1000 + millis;
    }

    0
}
