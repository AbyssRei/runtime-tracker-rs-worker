/// 公开 API 路由处理器
/// 对应原始项目的 apiRoutes.js
use crate::kv;
use crate::models::*;
use crate::services::{ai, query, recorder};
use crate::utils;
use std::collections::HashMap;
use worker::*;

/// 获取通用参数
fn get_offset_hours(ctx: &RouteContext<()>) -> i32 {
    utils::get_env_or(&ctx.env, "DEFAULT_TIMEZONE_OFFSET", "8")
        .parse()
        .unwrap_or(8)
}

fn get_now_ms() -> u64 {
    Date::now().as_millis()
}

fn json_error(message: &str, status: u16) -> Result<Response> {
    let body = serde_json::json!({
        "error": message,
        "success": false
    });
    Ok(Response::from_json(&body)?.with_status(status))
}

// ==================== 应用上报 ====================

/// POST /api - 应用上报
pub async fn report(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: ReportRequest = match req.json().await {
        Ok(b) => b,
        Err(e) => return json_error(&format!("Invalid request body: {}", e), 400),
    };

    let secret = utils::get_env_or(&ctx.env, "SECRET", "default-secret-key");
    if body.secret.as_deref() != Some(&secret) {
        return json_error("Invalid secret", 401);
    }

    let device = match &body.device {
        Some(d) if !d.is_empty() => d.clone(),
        _ => return json_error("Missing device", 400),
    };

    let d1 = ctx.env.d1("DB")?;
    let kv_store = ctx.env.kv("KV")?;
    let offset_hours = get_offset_hours(&ctx);
    let now_ms = get_now_ms();

    // 注册设备
    kv::ensure_device_registered(&kv_store, &device).await?;

    // 1. 处理电池信息
    if let Some(level) = body.battery_level {
        if level > 0 && level <= 100 {
            let is_charging = body.is_charging.unwrap_or(false);
            recorder::record_battery(&kv_store, &device, level, is_charging, now_ms).await?;
        }
    }

    // 2. 处理应用信息
    if body.app_name.is_some() || body.running.is_some() {
        if body.running != Some(false) && body.app_name.is_none() {
            return json_error("Missing app_name when running is true", 400);
        }

        recorder::record_usage(
            &d1,
            &kv_store,
            &device,
            body.app_name.as_deref(),
            body.running,
            body.package_name.as_deref(),
            offset_hours,
            now_ms,
        )
        .await?;
    }

    let battery_info = kv::get_battery_info(&kv_store, &device).await?;

    Response::from_json(&serde_json::json!({
        "success": true,
        "batteryInfo": battery_info,
        "timestamp": utils::utc_ms_to_iso(now_ms)
    }))
}

// ==================== 设备列表 ====================

/// GET /api/devices - 获取设备列表
pub async fn devices(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let kv_store = ctx.env.kv("KV")?;
    let devices = query::get_devices(&kv_store).await?;
    Response::from_json(&devices)
}

// ==================== 应用切换记录 ====================

/// GET /api/recentall - 获取所有设备的切换记录
pub async fn recent_all(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let kv_store = ctx.env.kv("KV")?;
    let device_ids = kv::get_device_list(&kv_store).await?;

    let mut all_records: HashMap<String, Vec<AppSwitch>> = HashMap::new();
    for device_id in &device_ids {
        let switches = kv::get_recent_switches(&kv_store, device_id).await?;
        all_records.insert(device_id.clone(), switches);
    }

    Response::from_json(&serde_json::json!({
        "success": true,
        "data": all_records,
        "count": all_records.len()
    }))
}

/// GET /api/recent/:device_id - 获取特定设备切换记录
pub async fn recent(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let device_id = ctx.param("device_id").unwrap();
    let kv_store = ctx.env.kv("KV")?;
    let records = kv::get_recent_switches(&kv_store, device_id).await?;

    Response::from_json(&serde_json::json!({
        "success": true,
        "data": records,
        "count": records.len()
    }))
}

// ==================== 统计查询 ====================

/// 检查是否允许访问汇总数据
fn is_summary_allowed(device_id: &str, ctx: &RouteContext<()>) -> bool {
    if device_id != "summary" {
        return true;
    }
    let web_summary = utils::get_env_or(&ctx.env, "WEB_SUMMARY", "true");
    utils::parse_bool_env(&web_summary, true)
}

/// GET /api/stats/:device_id - 当天统计
pub async fn daily_stats(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let device_id = ctx.param("device_id").unwrap().to_string();

    if !is_summary_allowed(&device_id, &ctx) {
        return json_error("Summary is disabled", 403);
    }

    let d1 = ctx.env.d1("DB")?;
    let offset_hours = get_offset_hours(&ctx);
    let now_ms = get_now_ms();

    // 解析日期参数
    let url = req.url()?;
    let params: HashMap<String, String> = url.query_pairs().into_owned().collect();

    let date_str = match params.get("date") {
        Some(d) => {
            if utils::parse_date(d).is_none() {
                return json_error("Invalid date format. Please use YYYY-MM-DD format.", 400);
            }
            d.clone()
        }
        None => {
            let tz = utils::TimezoneHelper::new(offset_hours);
            let today = tz.get_today(now_ms);
            utils::format_date(&today)
        }
    };

    let stats = if device_id == "summary" {
        query::get_daily_stats_all_devices(&d1, &date_str).await?
    } else {
        query::get_daily_stats(&d1, &device_id, &date_str).await?
    };

    Response::from_json(&stats)
}

/// GET /api/weekly/:device_id - 周统计
pub async fn weekly_stats(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let device_id = ctx.param("device_id").unwrap().to_string();

    if !is_summary_allowed(&device_id, &ctx) {
        return json_error("Summary is disabled", 403);
    }

    let d1 = ctx.env.d1("DB")?;
    let offset_hours = get_offset_hours(&ctx);
    let now_ms = get_now_ms();

    let url = req.url()?;
    let params: HashMap<String, String> = url.query_pairs().into_owned().collect();
    let week_offset = params
        .get("weekOffset")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let app_name = params.get("appName").cloned();

    let stats = if device_id == "summary" {
        query::get_weekly_stats_all_devices(
            &d1,
            app_name.as_deref(),
            week_offset,
            offset_hours,
            now_ms,
        )
        .await?
    } else {
        query::get_weekly_stats(
            &d1,
            &device_id,
            app_name.as_deref(),
            week_offset,
            offset_hours,
            now_ms,
        )
        .await?
    };

    Response::from_json(&stats)
}

/// GET /api/monthly/:device_id - 月统计
pub async fn monthly_stats(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let device_id = ctx.param("device_id").unwrap().to_string();

    if !is_summary_allowed(&device_id, &ctx) {
        return json_error("Summary is disabled", 403);
    }

    let d1 = ctx.env.d1("DB")?;
    let offset_hours = get_offset_hours(&ctx);
    let now_ms = get_now_ms();

    let url = req.url()?;
    let params: HashMap<String, String> = url.query_pairs().into_owned().collect();
    let month_offset = params
        .get("monthOffset")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let app_name = params.get("appName").cloned();

    let stats = if device_id == "summary" {
        query::get_monthly_stats_all_devices(
            &d1,
            app_name.as_deref(),
            month_offset,
            offset_hours,
            now_ms,
        )
        .await?
    } else {
        query::get_monthly_stats(
            &d1,
            &device_id,
            app_name.as_deref(),
            month_offset,
            offset_hours,
            now_ms,
        )
        .await?
    };

    Response::from_json(&stats)
}

// ==================== AI 总结 ====================

/// GET /api/ai/summary/:device_id - 获取最近 AI 总结
pub async fn ai_summary(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let device_id = ctx.param("device_id").unwrap();
    let kv_store = ctx.env.kv("KV")?;

    match kv::get_ai_summary(&kv_store, device_id).await? {
        Some(record) => Response::from_json(&serde_json::json!({
            "success": true,
            "deviceId": device_id,
            "summary": record.summary,
            "date": record.date,
            "timestamp": record.timestamp,
            "trigger": record.trigger
        })),
        None => {
            let body = serde_json::json!({
                "success": false,
                "error": "No recent summary found for this device",
                "message": "该设备暂无AI总结记录"
            });
            Ok(Response::from_json(&body)?.with_status(404))
        }
    }
}

/// GET /api/ai/summaries - 获取所有设备的 AI 总结
pub async fn ai_summaries(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let kv_store = ctx.env.kv("KV")?;
    let device_ids = kv::get_device_list(&kv_store).await?;
    let summaries = kv::get_all_ai_summaries(&kv_store, &device_ids).await?;

    Response::from_json(&serde_json::json!({
        "success": true,
        "count": summaries.len(),
        "summaries": summaries
    }))
}

/// GET /api/ai/trigger/:device_id - 手动触发 AI 总结（需 secret）
pub async fn ai_trigger(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let device_id = ctx.param("device_id").unwrap().to_string();
    let secret = utils::get_env_or(&ctx.env, "SECRET", "default-secret-key");

    let url = req.url()?;
    let params: HashMap<String, String> = url.query_pairs().into_owned().collect();

    let req_secret = params.get("secret").cloned().unwrap_or_default();
    if req_secret != secret {
        return json_error("Invalid or missing secret", 401);
    }

    let d1 = ctx.env.d1("DB")?;
    let kv_store = ctx.env.kv("KV")?;
    let offset_hours = get_offset_hours(&ctx);
    let now_ms = get_now_ms();
    let config = ai::AiConfig::from_env_and_kv(&ctx.env, &kv_store).await;

    let date = params.get("date").map(|s| s.as_str());

    let result = ai::trigger_summary(
        &d1,
        &kv_store,
        &device_id,
        &config,
        date,
        offset_hours,
        now_ms,
    )
    .await?;

    Response::from_json(&result)
}

/// GET /api/ai/status - AI 功能状态
pub async fn ai_status(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let kv_store = ctx.env.kv("KV")?;
    let config = ai::AiConfig::from_env_and_kv(&ctx.env, &kv_store).await;
    let offset_hours = get_offset_hours(&ctx);
    let status = ai::get_ai_status(&ctx.env, &config, offset_hours);
    Response::from_json(&status)
}

// ==================== 页面配置 ====================

/// GET /api/pageConfig - 页面组件配置
pub async fn page_config(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let kv_store = ctx.env.kv("KV")?;
    let g = |key: &str, default: &str| {
        let kv = &kv_store;
        let env = &ctx.env;
        let key = key.to_string();
        let default = default.to_string();
        async move { kv::get_effective_config(kv, env, &key, &default).await }
    };

    let config = PageConfig {
        web_device_count: utils::parse_bool_env(&g("WEB_DEVICE_COUNT", "true").await, true),
        web_comment: utils::parse_bool_env(&g("WEB_COMMENT", "true").await, true),
        web_ai_summary: utils::parse_bool_env(&g("AI_SUMMARY_ENABLED", "true").await, true),
        web_summary: utils::parse_bool_env(&g("WEB_SUMMARY", "true").await, true),
        giscus_repo: g("GISCUS_REPO", "").await,
        giscus_repo_id: g("GISCUS_REPOID", "").await,
        giscus_category: g("GISCUS_CATEGORY", "").await,
        giscus_category_id: g("GISCUS_CATEGORYID", "").await,
        giscus_mapping: g("GISCUS_MAPPING", "pathname").await,
        giscus_reactions_enabled: utils::parse_bool_env(
            &g("GISCUS_REACTIONSENABLED", "true").await,
            true,
        ),
        giscus_emit_metadata: utils::parse_bool_env(
            &g("GISCUS_EMITMETADATA", "false").await,
            false,
        ),
        giscus_input_position: g("GISCUS_INPUTPOSITION", "bottom").await,
        giscus_theme: g("GISCUS_THEME", "light").await,
        giscus_lang: g("GISCUS_LANG", "zh-CN").await,
    };

    // DEFAULT_TIMEZONE_OFFSET 也应感知 KV 覆盖
    let tz_offset: i32 = kv::get_effective_config(&kv_store, &ctx.env, "DEFAULT_TIMEZONE_OFFSET", "8")
        .await
        .parse()
        .unwrap_or(8);

    Response::from_json(&serde_json::json!({
        "success": true,
        "config": config,
        "tzOffset": tz_offset
    }))
}

// ==================== 其他 ====================

/// GET /api/ip - 获取客户端 IP
pub async fn ip(req: Request, _ctx: RouteContext<()>) -> Result<Response> {
    let headers = req.headers();

    // 优先从 CF-Connecting-IP 获取（Cloudflare 特有）
    let client_ip = headers
        .get("CF-Connecting-IP")
        .ok()
        .flatten()
        .or_else(|| {
            headers
                .get("X-Forwarded-For")
                .ok()
                .flatten()
                .map(|s| s.split(',').next().unwrap_or("").trim().to_string())
        })
        .unwrap_or_else(|| "unknown".to_string());

    Response::from_json(&serde_json::json!({ "ip": client_ip }))
}
