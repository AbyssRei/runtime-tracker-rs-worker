/// 工具函数：时区处理 + 日期范围计算
use chrono::{Datelike, Duration, NaiveDate};

// ==================== 时区工具 ====================

/// 时区辅助工具
pub struct TimezoneHelper {
    pub offset_hours: i32,
}

impl TimezoneHelper {
    pub fn new(offset_hours: i32) -> Self {
        Self { offset_hours }
    }

    /// 从环境变量字符串解析时区偏移
    #[allow(dead_code)]
    pub fn from_env_str(s: &str) -> Self {
        let offset = s.parse::<i32>().unwrap_or(8);
        Self::new(offset)
    }

    /// 从 UTC 毫秒时间戳获取本地日期
    pub fn get_local_date(&self, utc_ms: u64) -> NaiveDate {
        let total_seconds = (utc_ms / 1000) as i64 + (self.offset_hours as i64) * 3600;
        let days = if total_seconds >= 0 {
            total_seconds / 86400
        } else {
            (total_seconds - 86399) / 86400
        };
        let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
        epoch + Duration::days(days)
    }

    /// 获取当前本地日期
    pub fn get_today(&self, now_ms: u64) -> NaiveDate {
        self.get_local_date(now_ms)
    }

    /// 从 UTC 毫秒时间戳获取本地小时 (0-23)
    pub fn get_local_hour(&self, utc_ms: u64) -> u32 {
        let total_seconds = (utc_ms / 1000) as i64 + (self.offset_hours as i64) * 3600;
        let seconds_in_day = total_seconds.rem_euclid(86400);
        (seconds_in_day / 3600) as u32
    }

    /// 从 UTC 毫秒时间戳获取本地分钟 (0-59)
    pub fn get_local_minute(&self, utc_ms: u64) -> u32 {
        let total_seconds = (utc_ms / 1000) as i64 + (self.offset_hours as i64) * 3600;
        let seconds_in_day = total_seconds.rem_euclid(86400);
        ((seconds_in_day % 3600) / 60) as u32
    }

    /// 从 UTC 毫秒时间戳获取本地秒 (0-59)
    pub fn get_local_second(&self, utc_ms: u64) -> u32 {
        let total_seconds = (utc_ms / 1000) as i64 + (self.offset_hours as i64) * 3600;
        let seconds_in_day = total_seconds.rem_euclid(86400);
        (seconds_in_day % 60) as u32
    }

    /// 获取本地日期对应的 UTC 零点毫秒时间戳
    #[allow(dead_code)]
    pub fn local_date_to_utc_midnight_ms(&self, date: &NaiveDate) -> u64 {
        let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
        let days = (*date - epoch).num_days();
        let utc_seconds = days * 86400 - (self.offset_hours as i64) * 3600;
        (utc_seconds * 1000) as u64
    }
}

/// 格式化 NaiveDate 为 YYYY-MM-DD 字符串
pub fn format_date(date: &NaiveDate) -> String {
    format!(
        "{:04}-{:02}-{:02}",
        date.year(),
        date.month(),
        date.day()
    )
}

/// 解析 YYYY-MM-DD 字符串为 NaiveDate
pub fn parse_date(s: &str) -> Option<NaiveDate> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let year = parts[0].parse::<i32>().ok()?;
    let month = parts[1].parse::<u32>().ok()?;
    let day = parts[2].parse::<u32>().ok()?;
    NaiveDate::from_ymd_opt(year, month, day)
}

/// 从 UTC 毫秒时间戳生成 ISO 时间戳字符串
pub fn utc_ms_to_iso(utc_ms: u64) -> String {
    let total_secs = (utc_ms / 1000) as i64;
    let millis = utc_ms % 1000;
    let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
    let days = total_secs / 86400;
    let secs_in_day = total_secs % 86400;
    let date = epoch + Duration::days(days);
    let hours = secs_in_day / 3600;
    let minutes = (secs_in_day % 3600) / 60;
    let seconds = secs_in_day % 60;
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        date.year(),
        date.month(),
        date.day(),
        hours,
        minutes,
        seconds,
        millis
    )
}

/// 解析布尔环境变量
pub fn parse_bool_env(value: &str, default: bool) -> bool {
    let trimmed = value.trim().to_lowercase();
    if trimmed.is_empty() {
        return default;
    }
    matches!(trimmed.as_str(), "true" | "1" | "yes" | "on")
}

/// 获取环境变量，带默认值
pub fn get_env_or(env: &worker::Env, key: &str, default: &str) -> String {
    env.var(key)
        .map(|v| v.to_string())
        .unwrap_or_else(|_| default.to_string())
}

// ==================== 日期范围 ====================

/// 计算周范围 (周一 ~ 周日)
pub fn get_week_range(offset_hours: i32, week_offset: i32, now_ms: u64) -> (NaiveDate, NaiveDate) {
    let tz = TimezoneHelper::new(offset_hours);
    let today = tz.get_today(now_ms);

    // 计算本周一
    let day_of_week = today.weekday().num_days_from_monday();
    let this_monday = today - Duration::days(day_of_week as i64);

    // 目标周
    let target_monday = this_monday + Duration::weeks(week_offset as i64);
    let mut target_sunday = target_monday + Duration::days(6);

    // 本周时截止到今天
    if week_offset == 0 && target_sunday > today {
        target_sunday = today;
    }

    (target_monday, target_sunday)
}

/// 计算月范围
pub fn get_month_range(
    offset_hours: i32,
    month_offset: i32,
    now_ms: u64,
) -> (NaiveDate, NaiveDate) {
    let tz = TimezoneHelper::new(offset_hours);
    let today = tz.get_today(now_ms);

    // 计算目标月份
    let total_months = today.year() * 12 + today.month() as i32 - 1 + month_offset;
    let target_year = total_months.div_euclid(12);
    let target_month = (total_months.rem_euclid(12) + 1) as u32;

    let month_start = NaiveDate::from_ymd_opt(target_year, target_month, 1).unwrap();

    // 计算月末
    let next_month = if target_month == 12 {
        NaiveDate::from_ymd_opt(target_year + 1, 1, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(target_year, target_month + 1, 1).unwrap()
    };
    let mut month_end = next_month - Duration::days(1);

    // 本月时截止到今天
    if month_offset == 0 && month_end > today {
        month_end = today;
    }

    (month_start, month_end)
}

/// 生成日期范围内的所有日期
#[allow(dead_code)]
pub fn date_range_inclusive(start: &NaiveDate, end: &NaiveDate) -> Vec<NaiveDate> {
    let mut dates = Vec::new();
    let mut current = *start;
    while current <= *end {
        dates.push(current);
        current += Duration::days(1);
    }
    dates
}

/// 精确计算两个 UTC 毫秒时间戳之间的分钟数（保留两位小数）
pub fn calculate_precise_minutes(start_ms: u64, end_ms: u64) -> f64 {
    if end_ms <= start_ms {
        return 0.0;
    }
    let diff_ms = end_ms - start_ms;
    let minutes = diff_ms as f64 / 60000.0;
    (minutes * 100.0).round() / 100.0
}
