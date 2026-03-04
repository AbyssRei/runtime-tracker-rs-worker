use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ==================== D1 行类型 (匹配 SQL 列) ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyStatRow {
    pub id: Option<f64>,
    pub device_id: String,
    pub date: String,
    pub app_name: String,
    pub package_name: String,
    pub hourly_usage: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyEyeTimeRow {
    pub id: Option<f64>,
    pub date: String,
    pub hourly_usage: String,
}

// ==================== 领域模型 ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyStat {
    pub device_id: String,
    pub date: String,
    pub app_name: String,
    pub package_name: String,
    pub hourly_usage: Vec<f64>,
}

impl DailyStatRow {
    pub fn to_model(&self) -> DailyStat {
        DailyStat {
            device_id: self.device_id.clone(),
            date: self.date.clone(),
            app_name: self.app_name.clone(),
            package_name: self.package_name.clone(),
            hourly_usage: serde_json::from_str(&self.hourly_usage)
                .unwrap_or_else(|_| vec![0.0; 24]),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyEyeTime {
    pub date: String,
    pub hourly_usage: Vec<f64>,
}

impl DailyEyeTimeRow {
    pub fn to_model(&self) -> DailyEyeTime {
        DailyEyeTime {
            date: self.date.clone(),
            hourly_usage: serde_json::from_str(&self.hourly_usage)
                .unwrap_or_else(|_| vec![0.0; 24]),
        }
    }
}

// ==================== KV 存储模型 ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryInfo {
    pub level: u8,
    #[serde(rename = "isCharging")]
    pub is_charging: bool,
    pub timestamp: Option<String>,
}

impl Default for BatteryInfo {
    fn default() -> Self {
        Self {
            level: 0,
            is_charging: false,
            timestamp: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSwitch {
    #[serde(rename = "appName")]
    pub app_name: String,
    #[serde(rename = "packageName")]
    pub package_name: Option<String>,
    pub timestamp: String,
    pub running: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EyeDeviceState {
    pub is_active: bool,
    pub last_update_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EyeGlobalState {
    pub active: bool,
    pub last_record_time: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiSummaryRecord {
    pub summary: String,
    pub date: String,
    pub timestamp: String,
    pub trigger: String,
}

// ==================== 请求类型 ====================

#[derive(Debug, Deserialize)]
pub struct ReportRequest {
    pub secret: Option<String>,
    pub device: Option<String>,
    pub app_name: Option<String>,
    pub running: Option<bool>,
    #[serde(rename = "batteryLevel")]
    pub battery_level: Option<u8>,
    #[serde(rename = "isCharging")]
    pub is_charging: Option<bool>,
    pub package_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AccountUpdateRequest {
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AiTriggerRequest {
    pub date: Option<String>,
    #[serde(rename = "timezoneOffset")]
    pub timezone_offset: Option<i32>,
}

// ==================== 响应类型 ====================

#[derive(Debug, Serialize)]
pub struct DeviceInfo {
    pub device: String,
    #[serde(rename = "currentApp")]
    pub current_app: String,
    #[serde(rename = "currentPackageName")]
    pub current_package_name: Option<String>,
    pub running: bool,
    #[serde(rename = "runningSince")]
    pub running_since: String,
    #[serde(rename = "batteryLevel")]
    pub battery_level: u8,
    #[serde(rename = "isCharging")]
    pub is_charging: bool,
    #[serde(rename = "batteryTimestamp")]
    pub battery_timestamp: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct StatsResult {
    #[serde(rename = "totalUsage")]
    pub total_usage: f64,
    #[serde(rename = "appStats")]
    pub app_stats: HashMap<String, f64>,
    #[serde(rename = "hourlyStats")]
    pub hourly_stats: Vec<f64>,
    #[serde(rename = "appHourlyStats")]
    pub app_hourly_stats: HashMap<String, Vec<f64>>,
}

impl StatsResult {
    pub fn empty() -> Self {
        Self {
            total_usage: 0.0,
            app_stats: HashMap::new(),
            hourly_stats: vec![0.0; 24],
            app_hourly_stats: HashMap::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DateRange {
    pub start: String,
    pub end: String,
}

#[derive(Debug, Serialize)]
pub struct RangeStatsResult {
    #[serde(rename = "weekOffset", skip_serializing_if = "Option::is_none")]
    pub week_offset: Option<i32>,
    #[serde(rename = "monthOffset", skip_serializing_if = "Option::is_none")]
    pub month_offset: Option<i32>,
    #[serde(rename = "weekRange", skip_serializing_if = "Option::is_none")]
    pub week_range: Option<DateRange>,
    #[serde(rename = "monthRange", skip_serializing_if = "Option::is_none")]
    pub month_range: Option<DateRange>,
    #[serde(rename = "dailyTotals")]
    pub daily_totals: HashMap<String, f64>,
    #[serde(rename = "appDailyStats")]
    pub app_daily_stats: HashMap<String, HashMap<String, f64>>,
}

#[derive(Debug, Serialize)]
pub struct EyeTimeDailyResult {
    pub date: String,
    #[serde(rename = "totalUsage")]
    pub total_usage: f64,
    #[serde(rename = "hourlyStats")]
    pub hourly_stats: Vec<f64>,
    #[serde(rename = "timezoneOffset")]
    pub timezone_offset: i32,
}

#[derive(Debug, Serialize)]
pub struct EyeTimeRangeResult {
    #[serde(rename = "weekOffset", skip_serializing_if = "Option::is_none")]
    pub week_offset: Option<i32>,
    #[serde(rename = "monthOffset", skip_serializing_if = "Option::is_none")]
    pub month_offset: Option<i32>,
    #[serde(rename = "weekRange", skip_serializing_if = "Option::is_none")]
    pub week_range: Option<DateRange>,
    #[serde(rename = "monthRange", skip_serializing_if = "Option::is_none")]
    pub month_range: Option<DateRange>,
    #[serde(rename = "dailyTotals")]
    pub daily_totals: HashMap<String, f64>,
    #[serde(rename = "timezoneOffset")]
    pub timezone_offset: i32,
}

#[derive(Debug, Serialize)]
pub struct PageConfig {
    #[serde(rename = "WEB_DEVICE_COUNT")]
    pub web_device_count: bool,
    #[serde(rename = "WEB_COMMENT")]
    pub web_comment: bool,
    #[serde(rename = "WEB_AI_SUMMARY")]
    pub web_ai_summary: bool,
    #[serde(rename = "WEB_SUMMARY")]
    pub web_summary: bool,
    #[serde(rename = "GISCUS_REPO")]
    pub giscus_repo: String,
    #[serde(rename = "GISCUS_REPOID")]
    pub giscus_repo_id: String,
    #[serde(rename = "GISCUS_CATEGORY")]
    pub giscus_category: String,
    #[serde(rename = "GISCUS_CATEGORYID")]
    pub giscus_category_id: String,
    #[serde(rename = "GISCUS_MAPPING")]
    pub giscus_mapping: String,
    #[serde(rename = "GISCUS_REACTIONSENABLED")]
    pub giscus_reactions_enabled: bool,
    #[serde(rename = "GISCUS_EMITMETADATA")]
    pub giscus_emit_metadata: bool,
    #[serde(rename = "GISCUS_INPUTPOSITION")]
    pub giscus_input_position: String,
    #[serde(rename = "GISCUS_THEME")]
    pub giscus_theme: String,
    #[serde(rename = "GISCUS_LANG")]
    pub giscus_lang: String,
}
