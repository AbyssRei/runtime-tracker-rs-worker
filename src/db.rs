/// D1 数据库操作层
use crate::models::{DailyEyeTimeRow, DailyStatRow};
use worker::*;
use wasm_bindgen::JsValue;

// ==================== DailyStat 操作 ====================

/// 查询单条统计记录
pub async fn find_daily_stat(
    db: &D1Database,
    device_id: &str,
    date: &str,
    app_name: &str,
    package_name: &str,
) -> Result<Option<DailyStatRow>> {
    let stmt = db.prepare(
        "SELECT * FROM daily_stats WHERE device_id = ?1 AND date = ?2 AND app_name = ?3 AND package_name = ?4"
    );
    let stmt = stmt.bind(&[
        JsValue::from_str(device_id),
        JsValue::from_str(date),
        JsValue::from_str(app_name),
        JsValue::from_str(package_name),
    ])?;
    stmt.first::<DailyStatRow>(None).await
}

/// 插入或更新统计记录（使用 UPSERT）
pub async fn upsert_daily_stat(
    db: &D1Database,
    device_id: &str,
    date: &str,
    app_name: &str,
    package_name: &str,
    hourly_usage: &[f64],
) -> Result<()> {
    let hourly_json = serde_json::to_string(hourly_usage)
        .map_err(|e| Error::RustError(e.to_string()))?;

    let stmt = db.prepare(
        "INSERT INTO daily_stats (device_id, date, app_name, package_name, hourly_usage)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(device_id, date, app_name, package_name)
         DO UPDATE SET hourly_usage = ?5"
    );
    let stmt = stmt.bind(&[
        JsValue::from_str(device_id),
        JsValue::from_str(date),
        JsValue::from_str(app_name),
        JsValue::from_str(package_name),
        JsValue::from_str(&hourly_json),
    ])?;
    stmt.run().await?;
    Ok(())
}

/// 按日期范围查询单设备统计
pub async fn query_stats_by_range(
    db: &D1Database,
    device_id: &str,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<DailyStatRow>> {
    let stmt = db.prepare(
        "SELECT * FROM daily_stats WHERE device_id = ?1 AND date >= ?2 AND date <= ?3"
    );
    let stmt = stmt.bind(&[
        JsValue::from_str(device_id),
        JsValue::from_str(start_date),
        JsValue::from_str(end_date),
    ])?;
    let result = stmt.all().await?;
    result.results::<DailyStatRow>()
}

/// 按日期范围查询所有设备统计
pub async fn query_stats_all_devices_by_range(
    db: &D1Database,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<DailyStatRow>> {
    let stmt = db.prepare(
        "SELECT * FROM daily_stats WHERE date >= ?1 AND date <= ?2"
    );
    let stmt = stmt.bind(&[
        JsValue::from_str(start_date),
        JsValue::from_str(end_date),
    ])?;
    let result = stmt.all().await?;
    result.results::<DailyStatRow>()
}

/// 按应用名过滤查询单设备统计
pub async fn query_stats_by_range_with_app(
    db: &D1Database,
    device_id: &str,
    start_date: &str,
    end_date: &str,
    app_name: &str,
) -> Result<Vec<DailyStatRow>> {
    let stmt = db.prepare(
        "SELECT * FROM daily_stats WHERE device_id = ?1 AND date >= ?2 AND date <= ?3 AND app_name = ?4"
    );
    let stmt = stmt.bind(&[
        JsValue::from_str(device_id),
        JsValue::from_str(start_date),
        JsValue::from_str(end_date),
        JsValue::from_str(app_name),
    ])?;
    let result = stmt.all().await?;
    result.results::<DailyStatRow>()
}

/// 按应用名过滤查询所有设备统计
pub async fn query_stats_all_devices_by_range_with_app(
    db: &D1Database,
    start_date: &str,
    end_date: &str,
    app_name: &str,
) -> Result<Vec<DailyStatRow>> {
    let stmt = db.prepare(
        "SELECT * FROM daily_stats WHERE date >= ?1 AND date <= ?2 AND app_name = ?3"
    );
    let stmt = stmt.bind(&[
        JsValue::from_str(start_date),
        JsValue::from_str(end_date),
        JsValue::from_str(app_name),
    ])?;
    let result = stmt.all().await?;
    result.results::<DailyStatRow>()
}

/// 获取所有不重复的设备ID
#[allow(dead_code)]
pub async fn get_distinct_device_ids(db: &D1Database) -> Result<Vec<String>> {
    let stmt = db.prepare("SELECT DISTINCT device_id FROM daily_stats");
    let result = stmt.all().await?;

    #[derive(serde::Deserialize)]
    struct Row {
        device_id: String,
    }

    let rows = result.results::<Row>()?;
    Ok(rows.into_iter().map(|r| r.device_id).collect())
}

// ==================== DailyEyeTime 操作 ====================

/// 查询单日用眼时长记录
pub async fn find_eye_time(
    db: &D1Database,
    date: &str,
) -> Result<Option<DailyEyeTimeRow>> {
    let stmt = db.prepare("SELECT * FROM daily_eye_time WHERE date = ?1");
    let stmt = stmt.bind(&[JsValue::from_str(date)])?;
    stmt.first::<DailyEyeTimeRow>(None).await
}

/// 插入或更新用眼时长记录
pub async fn upsert_eye_time(
    db: &D1Database,
    date: &str,
    hourly_usage: &[f64],
) -> Result<()> {
    let hourly_json = serde_json::to_string(hourly_usage)
        .map_err(|e| Error::RustError(e.to_string()))?;

    let stmt = db.prepare(
        "INSERT INTO daily_eye_time (date, hourly_usage)
         VALUES (?1, ?2)
         ON CONFLICT(date)
         DO UPDATE SET hourly_usage = ?2"
    );
    let stmt = stmt.bind(&[
        JsValue::from_str(date),
        JsValue::from_str(&hourly_json),
    ])?;
    stmt.run().await?;
    Ok(())
}

/// 按日期范围查询用眼时长
pub async fn query_eye_time_by_range(
    db: &D1Database,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<DailyEyeTimeRow>> {
    let stmt = db.prepare(
        "SELECT * FROM daily_eye_time WHERE date >= ?1 AND date <= ?2"
    );
    let stmt = stmt.bind(&[
        JsValue::from_str(start_date),
        JsValue::from_str(end_date),
    ])?;
    let result = stmt.all().await?;
    result.results::<DailyEyeTimeRow>()
}
