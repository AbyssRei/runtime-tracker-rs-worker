/// 管理后台路由处理器
/// 对应原始项目的 adminRoutes.js
use crate::kv;
use crate::models::*;
use crate::services::ai;
use crate::utils;
use std::collections::HashMap;
use worker::*;

fn json_error(message: &str, status: u16) -> Result<Response> {
    let body = serde_json::json!({
        "success": false,
        "message": message
    });
    Ok(Response::from_json(&body)?.with_status(status))
}

fn get_offset_hours(ctx: &RouteContext<()>) -> i32 {
    utils::get_env_or(&ctx.env, "DEFAULT_TIMEZONE_OFFSET", "8")
        .parse()
        .unwrap_or(8)
}

// ==================== 认证中间件 ====================

/// 验证请求中的 Bearer Token
async fn authenticate(req: &Request, ctx: &RouteContext<()>) -> Result<Option<Response>> {
    let kv_store = ctx.env.kv("KV")?;

    let auth_header = req.headers().get("Authorization").ok().flatten();
    let token = match auth_header {
        Some(header) if header.starts_with("Bearer ") => header[7..].to_string(),
        _ => {
            return Ok(Some(
                Response::from_json(&serde_json::json!({
                    "success": false,
                    "error": "Access token required"
                }))?
                .with_status(401),
            ));
        }
    };

    match kv::verify_auth_token(&kv_store, &token).await? {
        Some(_) => Ok(None), // 认证成功
        None => Ok(Some(
            Response::from_json(&serde_json::json!({
                "success": false,
                "error": "Invalid or expired token"
            }))?
            .with_status(403),
        )),
    }
}

// ==================== 登录 ====================

/// POST /admin/login - 管理员登录
pub async fn login(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: LoginRequest = match req.json().await {
        Ok(b) => b,
        Err(_) => return json_error("Invalid request body", 400),
    };

    let username = match &body.username {
        Some(u) if !u.is_empty() => u.clone(),
        _ => return json_error("用户名和密码不能为空", 400),
    };

    let password = match &body.password {
        Some(p) if !p.is_empty() => p.clone(),
        _ => return json_error("用户名和密码不能为空", 400),
    };

    let kv_store = ctx.env.kv("KV")?;
    let admin_user = kv::get_effective_config(&kv_store, &ctx.env, "ADMIN_USER", "admin").await;
    let admin_passwd = kv::get_effective_config(&kv_store, &ctx.env, "ADMIN_PASSWD", "admin").await;

    if username != admin_user {
        return json_error("用户名或密码错误", 401);
    }

    // 直接比较密码（CF Workers 环境变量已加密存储）
    if password != admin_passwd {
        return json_error("用户名或密码错误", 401);
    }

    // 生成随机 Token
    let token = generate_random_token();

    let token_data = kv::AuthTokenData {
        username: admin_user,
        role: "admin".to_string(),
        created_at: utils::utc_ms_to_iso(Date::now().as_millis()),
    };

    // 存储 Token（7 天过期）
    kv::set_auth_token(&kv_store, &token, &token_data).await?;

    Response::from_json(&serde_json::json!({
        "success": true,
        "token": token
    }))
}

/// 生成随机令牌
fn generate_random_token() -> String {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).unwrap_or_default();
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

// ==================== 账户管理 ====================

/// POST /admin/account/update - 更新管理员账户
pub async fn account_update(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    if let Some(err_response) = authenticate(&req, &ctx).await? {
        return Ok(err_response);
    }

    let body: AccountUpdateRequest = match req.json().await {
        Ok(b) => b,
        Err(_) => return json_error("Invalid request body", 400),
    };

    if body.username.is_none() && body.password.is_none() {
        return json_error("请提供要更新的用户名或密码", 400);
    }

    // 在 CF Workers 中，更新配置通过 KV 存储运行时覆盖
    let kv_store = ctx.env.kv("KV")?;
    let mut config = kv::get_runtime_config(&kv_store).await?;
    let mut updated_fields = Vec::new();

    if let Some(username) = &body.username {
        config.insert("ADMIN_USER".to_string(), username.clone());
        updated_fields.push("用户名");
    }

    if let Some(password) = &body.password {
        config.insert("ADMIN_PASSWD".to_string(), password.clone());
        updated_fields.push("密码");
    }

    kv::set_runtime_config(&kv_store, &config).await?;

    Response::from_json(&serde_json::json!({
        "success": true,
        "message": format!("账户信息已更新（{}）", updated_fields.join("、")),
        "requireRelogin": true,
        "note": "更新将在下次部署或 Worker 重新加载后生效，也可通过 KV 即时覆盖"
    }))
}

// ==================== AI 管理 ====================

/// POST /admin/ai/trigger/:device_id - 手动触发 AI 总结
pub async fn ai_trigger(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    if let Some(err_response) = authenticate(&req, &ctx).await? {
        return Ok(err_response);
    }

    let device_id = ctx.param("device_id").unwrap().to_string();

    let body: AiTriggerRequest = req.json().await.unwrap_or(AiTriggerRequest {
        date: None,
        timezone_offset: None,
    });

    let d1 = ctx.env.d1("DB")?;
    let kv_store = ctx.env.kv("KV")?;
    let offset_hours = body
        .timezone_offset
        .unwrap_or_else(|| get_offset_hours(&ctx));
    let now_ms = Date::now().as_millis();
    let config = ai::AiConfig::from_env_and_kv(&ctx.env, &kv_store).await;

    let result = ai::trigger_summary(
        &d1,
        &kv_store,
        &device_id,
        &config,
        body.date.as_deref(),
        offset_hours,
        now_ms,
    )
    .await?;

    Response::from_json(&result)
}

/// POST /admin/ai/stop - 停止 AI 定时任务
/// 注意：在 CF Workers 中，定时任务由 Cron Triggers 控制
pub async fn ai_stop(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    if let Some(err_response) = authenticate(&req, &ctx).await? {
        return Ok(err_response);
    }

    Response::from_json(&serde_json::json!({
        "success": true,
        "message": "在 Cloudflare Workers 中，定时任务由 Cron Triggers 控制，请在 wrangler.toml 或 Cloudflare Dashboard 中管理"
    }))
}

/// POST /admin/ai/start - 启动 AI 定时任务
pub async fn ai_start(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    if let Some(err_response) = authenticate(&req, &ctx).await? {
        return Ok(err_response);
    }

    Response::from_json(&serde_json::json!({
        "success": true,
        "message": "在 Cloudflare Workers 中，定时任务由 Cron Triggers 控制，请在 wrangler.toml 或 Cloudflare Dashboard 中管理"
    }))
}

// ==================== 配置管理 ====================

/// GET /admin/config - 获取配置
pub async fn get_config(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    if let Some(err_response) = authenticate(&req, &ctx).await? {
        return Ok(err_response);
    }

    let config = serde_json::json!({
        "ADMIN_USER": utils::get_env_or(&ctx.env, "ADMIN_USER", "admin"),
        "AI_API_URL": utils::get_env_or(&ctx.env, "AI_API_URL", "未设置"),
        "AI_MODEL": utils::get_env_or(&ctx.env, "AI_MODEL", "gpt-4"),
        "AI_MAX_TOKENS": utils::get_env_or(&ctx.env, "AI_MAX_TOKENS", "1000"),
        "AI_PROMPT": utils::get_env_or(&ctx.env, "AI_PROMPT", "未设置"),
        "PUBLISH_ENABLED": utils::get_env_or(&ctx.env, "PUBLISH_ENABLED", "false"),
        "PUBLISH_API_URL": utils::get_env_or(&ctx.env, "PUBLISH_API_URL", "未设置"),
        "PUBLISH_API_KEY": utils::get_env_or(&ctx.env, "PUBLISH_API_KEY", "未设置"),
        "DEFAULT_TIMEZONE_OFFSET": utils::get_env_or(&ctx.env, "DEFAULT_TIMEZONE_OFFSET", "8"),
        "AI_SUMMARY_ENABLED": utils::get_env_or(&ctx.env, "AI_SUMMARY_ENABLED", "true"),
        "WEB_DEVICE_COUNT": utils::get_env_or(&ctx.env, "WEB_DEVICE_COUNT", "true"),
        "WEB_COMMENT": utils::get_env_or(&ctx.env, "WEB_COMMENT", "true"),
        "WEB_SUMMARY": utils::get_env_or(&ctx.env, "WEB_SUMMARY", "true"),
        "GISCUS_REPO": utils::get_env_or(&ctx.env, "GISCUS_REPO", ""),
        "GISCUS_REPOID": utils::get_env_or(&ctx.env, "GISCUS_REPOID", ""),
        "GISCUS_CATEGORY": utils::get_env_or(&ctx.env, "GISCUS_CATEGORY", ""),
        "GISCUS_CATEGORYID": utils::get_env_or(&ctx.env, "GISCUS_CATEGORYID", ""),
        "GISCUS_MAPPING": utils::get_env_or(&ctx.env, "GISCUS_MAPPING", ""),
        "GISCUS_REACTIONSENABLED": utils::get_env_or(&ctx.env, "GISCUS_REACTIONSENABLED", ""),
        "GISCUS_EMITMETADATA": utils::get_env_or(&ctx.env, "GISCUS_EMITMETADATA", ""),
        "GISCUS_INPUTPOSITION": utils::get_env_or(&ctx.env, "GISCUS_INPUTPOSITION", ""),
        "GISCUS_THEME": utils::get_env_or(&ctx.env, "GISCUS_THEME", ""),
        "GISCUS_LANG": utils::get_env_or(&ctx.env, "GISCUS_LANG", ""),
        "PLATFORM": "cloudflare-workers"
    });

    Response::from_json(&serde_json::json!({
        "success": true,
        "config": config
    }))
}

/// POST /admin/config - 更新配置
pub async fn update_config(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    if let Some(err_response) = authenticate(&req, &ctx).await? {
        return Ok(err_response);
    }

    let updates: HashMap<String, String> = match req.json().await {
        Ok(u) => u,
        Err(_) => return json_error("Invalid request body", 400),
    };

    if updates.is_empty() {
        return json_error("请提供要更新的配置项", 400);
    }

    // 允许修改的配置项白名单
    let allowed_keys: Vec<&str> = vec![
        "AI_API_URL",
        "AI_API_KEY",
        "AI_MODEL",
        "AI_MAX_TOKENS",
        "AI_PROMPT",
        "PUBLISH_ENABLED",
        "PUBLISH_API_URL",
        "PUBLISH_API_KEY",
        "DEFAULT_TIMEZONE_OFFSET",
        "AI_SUMMARY_ENABLED",
        "WEB_DEVICE_COUNT",
        "WEB_COMMENT",
        "WEB_SUMMARY",
        "GISCUS_REPO",
        "GISCUS_REPOID",
        "GISCUS_CATEGORY",
        "GISCUS_CATEGORYID",
        "GISCUS_MAPPING",
        "GISCUS_REACTIONSENABLED",
        "GISCUS_EMITMETADATA",
        "GISCUS_INPUTPOSITION",
        "GISCUS_THEME",
        "GISCUS_LANG",
    ];

    let invalid_keys: Vec<&String> = updates
        .keys()
        .filter(|k| !allowed_keys.contains(&k.as_str()))
        .collect();

    if !invalid_keys.is_empty() {
        let keys_str: Vec<&str> = invalid_keys.iter().map(|s| s.as_str()).collect();
        return json_error(
            &format!(
                "不允许修改的配置项: {}，请在 Cloudflare Dashboard 或 wrangler.toml 中修改",
                keys_str.join(", ")
            ),
            400,
        );
    }

    // 将配置保存到 KV（运行时覆盖）
    let kv_store = ctx.env.kv("KV")?;
    let mut config = kv::get_runtime_config(&kv_store).await?;

    for (key, value) in &updates {
        config.insert(key.clone(), value.clone());
    }

    kv::set_runtime_config(&kv_store, &config).await?;

    Response::from_json(&serde_json::json!({
        "success": true,
        "message": "配置已更新",
        "details": "配置已保存到 KV 存储，在 Worker 运行时即时生效"
    }))
}

/// POST /admin/restart - 重启服务
/// 在 CF Workers 中不适用
pub async fn restart(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    if let Some(err_response) = authenticate(&req, &ctx).await? {
        return Ok(err_response);
    }

    Response::from_json(&serde_json::json!({
        "success": true,
        "message": "Cloudflare Workers 为无状态服务，无需手动重启。配置更改通过 KV 即时生效，代码更新通过 wrangler deploy 部署。"
    }))
}
