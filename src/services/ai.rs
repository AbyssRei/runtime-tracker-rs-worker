/// AI 总结服务
/// 对应原始项目的 AISummary.js
use crate::kv;
use crate::kv::KvStore;
use crate::models::*;
use crate::services::query;
use crate::utils::{self, TimezoneHelper};
use worker::*;

/// AI 配置
pub struct AiConfig {
    pub api_url: String,
    pub api_key: String,
    pub model: String,
    pub max_tokens: u32,
    pub prompt_suffix: String,
    pub enabled: bool,
    pub publish_enabled: bool,
    pub publish_api_url: String,
    pub publish_api_key: String,
}

impl AiConfig {
    /// 从环境变量构建配置（不含 KV 覆盖，用于无 KV 上下文处的保底调用）
    #[allow(dead_code)]
    pub fn from_env(env: &Env) -> Self {
        Self {
            api_url: utils::get_env_or(
                env,
                "AI_API_URL",
                "https://api.openai.com/v1/chat/completions",
            ),
            api_key: utils::get_env_or(env, "AI_API_KEY", ""),
            model: utils::get_env_or(env, "AI_MODEL", "gpt-4"),
            max_tokens: utils::get_env_or(env, "AI_MAX_TOKENS", "1000")
                .parse()
                .unwrap_or(1000),
            prompt_suffix: utils::get_env_or(env, "AI_PROMPT", "").replace("\\n", "\n"),
            enabled: utils::parse_bool_env(
                &utils::get_env_or(env, "AI_SUMMARY_ENABLED", "true"),
                true,
            ),
            publish_enabled: utils::parse_bool_env(
                &utils::get_env_or(env, "PUBLISH_ENABLED", "false"),
                false,
            ),
            publish_api_url: utils::get_env_or(env, "PUBLISH_API_URL", ""),
            publish_api_key: utils::get_env_or(env, "PUBLISH_API_KEY", ""),
        }
    }

    /// 从环境变量 + KV 运行时覆盖构建配置
    /// KV 中存储的 `runtime_config` 优先级高于 wrangler.toml `[vars]`。
    pub async fn from_env_and_kv(env: &Env, kv: &KvStore) -> Self {
        let g = |key: &str, default: &str| {
            let kv = &kv;
            let env = &env;
            let key = key.to_string();
            let default = default.to_string();
            async move { kv::get_effective_config(kv, env, &key, &default).await }
        };

        let api_url = g("AI_API_URL", "https://api.openai.com/v1/chat/completions").await;
        let api_key = g("AI_API_KEY", "").await;
        let model = g("AI_MODEL", "gpt-4").await;
        let max_tokens = g("AI_MAX_TOKENS", "1000").await.parse().unwrap_or(1000);
        let prompt_suffix = g("AI_PROMPT", "").await.replace("\\n", "\n");
        let enabled = utils::parse_bool_env(&g("AI_SUMMARY_ENABLED", "true").await, true);
        let publish_enabled = utils::parse_bool_env(&g("PUBLISH_ENABLED", "false").await, false);
        let publish_api_url = g("PUBLISH_API_URL", "").await;
        let publish_api_key = g("PUBLISH_API_KEY", "").await;

        Self {
            api_url,
            api_key,
            model,
            max_tokens,
            prompt_suffix,
            enabled,
            publish_enabled,
            publish_api_url,
            publish_api_key,
        }
    }
}

/// 手动触发 AI 总结
pub async fn trigger_summary(
    d1: &D1Database,
    kv_store: &kv::KvStore,
    device_id: &str,
    config: &AiConfig,
    date: Option<&str>,
    offset_hours: i32,
    now_ms: u64,
) -> Result<serde_json::Value> {
    if !config.enabled {
        return Ok(serde_json::json!({
            "success": false,
            "message": "AI summary is disabled"
        }));
    }

    if config.api_key.is_empty() {
        return Ok(serde_json::json!({
            "success": false,
            "message": "AI API key not configured"
        }));
    }

    generate_daily_summary(d1, kv_store, device_id, config, date, offset_hours, now_ms, "manual")
        .await
}

/// 生成每日总结
pub async fn generate_daily_summary(
    d1: &D1Database,
    kv_store: &kv::KvStore,
    device_id: &str,
    config: &AiConfig,
    date: Option<&str>,
    offset_hours: i32,
    now_ms: u64,
    trigger: &str,
) -> Result<serde_json::Value> {
    let tz = TimezoneHelper::new(offset_hours);

    // 确定目标日期
    let target_date = match date {
        Some(d) => d.to_string(),
        None => {
            let today = tz.get_today(now_ms);
            utils::format_date(&today)
        }
    };

    console_log!(
        "[AISummary] 开始为设备 {} 生成 {} 的总结 (触发方式: {})",
        device_id,
        target_date,
        trigger
    );

    // 1. 获取统计数据
    let stats = query::get_daily_stats(d1, device_id, &target_date).await?;

    if stats.total_usage == 0.0 {
        console_log!(
            "[AISummary] 设备 {} 在 {} 无使用数据",
            device_id,
            target_date
        );
        return Ok(serde_json::json!({
            "success": false,
            "message": "No usage data for this day"
        }));
    }

    // 2. 获取应用切换记录
    let switches = kv::get_recent_switches(kv_store, device_id).await?;

    // 3. 构建提示词并调用 AI
    let prompt = build_prompt(device_id, &target_date, &stats, &switches, &config.prompt_suffix);
    let summary = call_ai(config, &prompt).await?;

    // 4. 发布总结（如果启用）
    if config.publish_enabled && !config.publish_api_url.is_empty() {
        let _ = publish_summary(config, device_id, &target_date, &summary, now_ms).await;
    }

    // 5. 保存到 KV 缓存
    let record = AiSummaryRecord {
        summary: summary.clone(),
        date: target_date.clone(),
        timestamp: utils::utc_ms_to_iso(now_ms),
        trigger: trigger.to_string(),
    };
    kv::set_ai_summary(kv_store, device_id, &record).await?;

    console_log!("[AISummary] 设备 {} 总结完成", device_id);

    Ok(serde_json::json!({
        "success": true,
        "deviceId": device_id,
        "summary": summary,
        "date": target_date,
        "timestamp": record.timestamp,
        "trigger": trigger
    }))
}

/// 为所有设备生成总结（定时任务用）
pub async fn run_summary_for_all_devices(
    d1: &D1Database,
    kv_store: &kv::KvStore,
    config: &AiConfig,
    offset_hours: i32,
    now_ms: u64,
) -> Result<()> {
    if !config.enabled || config.api_key.is_empty() {
        console_log!("[AISummary] 未启用或未配置 API Key，跳过定时任务");
        return Ok(());
    }

    let device_ids = kv::get_device_list(kv_store).await?;
    console_log!(
        "[AISummary] 开始为 {} 个设备生成总结",
        device_ids.len()
    );

    for device_id in &device_ids {
        match generate_daily_summary(
            d1,
            kv_store,
            device_id,
            config,
            None,
            offset_hours,
            now_ms,
            "cron",
        )
        .await
        {
            Ok(_) => {}
            Err(e) => {
                console_log!("[AISummary] 设备 {} 总结失败: {}", device_id, e);
            }
        }
    }

    console_log!("[AISummary] 所有设备总结完成");
    Ok(())
}

/// 构建 AI 提示词
fn build_prompt(
    device_id: &str,
    date: &str,
    stats: &StatsResult,
    switches: &[AppSwitch],
    prompt_suffix: &str,
) -> String {
    let mut prompt = String::from("总结以下设备的应用使用情况，控制在300字以内\n\n");

    prompt.push_str(&format!("- 设备ID: {}\n", device_id));
    prompt.push_str(&format!("- 统计日期: {}\n\n", date));

    // 总体使用情况
    let total_hours = (stats.total_usage / 60.0) as u32;
    let total_mins = (stats.total_usage % 60.0) as u32;
    prompt.push_str("## 总体使用情况\n");
    prompt.push_str(&format!(
        "- 总使用时长: {}小时{}分钟\n",
        total_hours, total_mins
    ));
    prompt.push_str(&format!(
        "- 使用应用数量: {}个\n\n",
        stats.app_stats.len()
    ));

    // 应用使用占比 TOP 20
    let mut app_list: Vec<(&String, &f64)> = stats.app_stats.iter().collect();
    app_list.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());

    prompt.push_str("## 应用使用占比(TOP 20)\n");
    for &(app, &minutes) in app_list.iter().take(20) {
        let hours = (minutes / 60.0) as u32;
        let mins = (minutes % 60.0) as u32;
        let pct = if stats.total_usage > 0.0 {
            (minutes / stats.total_usage * 100.0 * 10.0).round() / 10.0
        } else {
            0.0
        };
        prompt.push_str(&format!("- {}: {}小时{}分钟 ({}%)\n", app, hours, mins, pct));
    }

    // 最近切换记录
    let recent_count = switches.len().min(100);
    prompt.push_str(&format!(
        "\n## 最近应用切换记录 (最新{}条)\n",
        recent_count
    ));
    for switch in switches.iter().take(10) {
        let status = if switch.running { "打开" } else { "关闭" };
        prompt.push_str(&format!(
            "- {} {} {}\n",
            switch.timestamp, status, switch.app_name
        ));
    }

    if !prompt_suffix.is_empty() {
        prompt.push_str(prompt_suffix);
    }
    prompt.push_str("注意：控制在300字以内，不要返回md格式，只能换行");

    prompt
}

/// 调用 AI API
async fn call_ai(config: &AiConfig, prompt: &str) -> Result<String> {
    let body = serde_json::json!({
        "model": config.model,
        "messages": [
            {
                "role": "system",
                "content": "你是一个时间分析师"
            },
            {
                "role": "user",
                "content": prompt
            }
        ],
        "max_tokens": config.max_tokens,
        "temperature": 0.7
    });

    let headers = Headers::new();
    headers.set("Content-Type", "application/json")?;
    headers.set("Authorization", &format!("Bearer {}", config.api_key))?;

    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_headers(headers)
        .with_body(Some(wasm_bindgen::JsValue::from_str(
            &serde_json::to_string(&body).map_err(|e| Error::RustError(e.to_string()))?,
        )));

    let request = Request::new_with_init(&config.api_url, &init)?;
    let mut response = Fetch::Request(request).send().await?;

    if response.status_code() != 200 {
        let error_text = response.text().await.unwrap_or_default();
        return Err(Error::RustError(format!(
            "AI API 请求失败: {} {}",
            response.status_code(),
            error_text
        )));
    }

    let result: serde_json::Value = response.json().await?;
    let content = result["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("AI 未返回内容")
        .to_string();

    Ok(content)
}

/// 发布总结到外部 API
async fn publish_summary(
    config: &AiConfig,
    device_id: &str,
    date: &str,
    summary: &str,
    now_ms: u64,
) -> Result<()> {
    let payload = serde_json::json!({
        "deviceId": device_id,
        "date": date,
        "timestamp": utils::utc_ms_to_iso(now_ms),
        "summary": summary
    });

    let headers = Headers::new();
    headers.set("Content-Type", "application/json")?;
    if !config.publish_api_key.is_empty() {
        headers.set(
            "Authorization",
            &format!("Bearer {}", config.publish_api_key),
        )?;
    }

    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_headers(headers)
        .with_body(Some(wasm_bindgen::JsValue::from_str(
            &serde_json::to_string(&payload).map_err(|e| Error::RustError(e.to_string()))?,
        )));

    let request = Request::new_with_init(&config.publish_api_url, &init)?;
    let response = Fetch::Request(request).send().await?;

    if response.status_code() == 200 {
        console_log!("[AISummary] 总结已成功发布");
    } else {
        console_log!("[AISummary] 发布失败: {}", response.status_code());
    }

    Ok(())
}

/// 获取 AI 总结状态信息
pub fn get_ai_status(_env: &Env, config: &AiConfig, offset_hours: i32) -> serde_json::Value {
    let tz_str = if offset_hours >= 0 {
        format!("UTC+{}", offset_hours)
    } else {
        format!("UTC{}", offset_hours)
    };

    serde_json::json!({
        "enabled": config.enabled,
        "aiConfigured": !config.api_key.is_empty(),
        "publishEnabled": config.publish_enabled,
        "cronJobsCount": 1,
        "schedules": generate_schedule_strings(offset_hours),
        "model": config.model,
        "defaultTimezone": tz_str
    })
}

/// 生成定时任务时间字符串（基于 wrangler.toml 中的 cron 配置）
fn generate_schedule_strings(offset_hours: i32) -> Vec<String> {
    // 默认每 4 小时执行一次（与 wrangler.toml 的 cron 配对）
    let interval = 4;
    let mut schedules = Vec::new();
    let mut hour = interval;
    while hour < 24 {
        // cron 是 UTC 的，转换为本地时间显示
        let local_hour = (hour + offset_hours + 24) % 24;
        schedules.push(format!("{:02}:00", local_hour));
        hour += interval;
    }
    schedules
}
