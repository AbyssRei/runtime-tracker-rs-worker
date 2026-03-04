/// Runtime Tracker - Cloudflare Workers 后端
/// 使用 Rust 重构的设备应用使用时间追踪系统
///
/// 数据存储：
/// - D1 SQL 数据库：daily_stats（应用统计）、daily_eye_time（用眼时长）
/// - Workers KV：设备列表、电池信息、应用切换记录、AI 总结缓存、认证令牌
///
/// 定时任务：
/// - Cron Triggers（wrangler.toml 配置）触发 AI 总结生成

mod db;
mod kv;
mod models;
mod routes;
mod services;
mod utils;

use services::ai;
use worker::*;

/// 添加 CORS 头到响应
fn with_cors(response: Response) -> Result<Response> {
    let headers = Headers::new();
    headers.set("Access-Control-Allow-Origin", "*")?;
    headers.set(
        "Access-Control-Allow-Methods",
        "GET, POST, PUT, DELETE, OPTIONS",
    )?;
    headers.set(
        "Access-Control-Allow-Headers",
        "Content-Type, Authorization",
    )?;
    headers.set("Access-Control-Max-Age", "86400")?;

    Ok(response.with_headers(headers))
}

/// CORS 预检请求处理
fn handle_options() -> Result<Response> {
    let headers = Headers::new();
    headers.set("Access-Control-Allow-Origin", "*")?;
    headers.set(
        "Access-Control-Allow-Methods",
        "GET, POST, PUT, DELETE, OPTIONS",
    )?;
    headers.set(
        "Access-Control-Allow-Headers",
        "Content-Type, Authorization",
    )?;
    headers.set("Access-Control-Max-Age", "86400")?;

    Ok(Response::empty()?.with_status(204).with_headers(headers))
}

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    console_error_panic_hook::set_once();

    // 处理 CORS 预检
    if req.method() == Method::Options {
        return handle_options();
    }

    let router = Router::new();

    let response = router
        // ==================== 公开 API 路由 ====================
        .post_async("/api", routes::api::report)
        .get_async("/api/devices", routes::api::devices)
        .get_async("/api/recentall", routes::api::recent_all)
        .get_async("/api/recent/:device_id", routes::api::recent)
        .get_async("/api/stats/:device_id", routes::api::daily_stats)
        .get_async("/api/weekly/:device_id", routes::api::weekly_stats)
        .get_async("/api/monthly/:device_id", routes::api::monthly_stats)
        // AI 总结
        .get_async("/api/ai/summary/:device_id", routes::api::ai_summary)
        .get_async("/api/ai/summaries", routes::api::ai_summaries)
        .get_async("/api/ai/trigger/:device_id", routes::api::ai_trigger)
        .get_async("/api/ai/status", routes::api::ai_status)
        // 页面配置
        .get_async("/api/pageConfig", routes::api::page_config)
        .get_async("/api/ip", routes::api::ip)
        // ==================== 用眼时长路由 ====================
        .get_async("/api/eyetime/daily", routes::eyetime::daily)
        .get_async("/api/eyetime/weekly", routes::eyetime::weekly)
        .get_async("/api/eyetime/monthly", routes::eyetime::monthly)
        // ==================== 管理后台路由 ====================
        .post_async("/admin/login", routes::admin::login)
        .post_async("/admin/account/update", routes::admin::account_update)
        .post_async(
            "/admin/ai/trigger/:device_id",
            routes::admin::ai_trigger,
        )
        .post_async("/admin/ai/stop", routes::admin::ai_stop)
        .post_async("/admin/ai/start", routes::admin::ai_start)
        .get_async("/admin/config", routes::admin::get_config)
        .post_async("/admin/config", routes::admin::update_config)
        .post_async("/admin/restart", routes::admin::restart)
        // 运行路由
        .run(req, env)
        .await?;

    // 为所有响应添加 CORS 头
    with_cors(response)
}

/// 定时触发处理器 - AI 总结 Cron
#[event(scheduled)]
async fn scheduled(_event: ScheduledEvent, env: Env, _ctx: ScheduleContext) {
    console_log!("[Scheduled] Cron 触发，开始执行 AI 总结任务");

    let d1 = match env.d1("DB") {
        Ok(db) => db,
        Err(e) => {
            console_log!("[Scheduled] 获取 D1 失败: {}", e);
            return;
        }
    };

    let kv_store = match env.kv("KV") {
        Ok(kv) => kv,
        Err(e) => {
            console_log!("[Scheduled] 获取 KV 失败: {}", e);
            return;
        }
    };

    let config = ai::AiConfig::from_env(&env);
    let offset_hours: i32 = utils::get_env_or(&env, "DEFAULT_TIMEZONE_OFFSET", "8")
        .parse()
        .unwrap_or(8);
    let now_ms = Date::now().as_millis();

    match ai::run_summary_for_all_devices(&d1, &kv_store, &config, offset_hours, now_ms).await {
        Ok(_) => console_log!("[Scheduled] AI 总结任务完成"),
        Err(e) => console_log!("[Scheduled] AI 总结任务失败: {}", e),
    }
}
