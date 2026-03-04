/// KV 存储操作层
/// 用于存储设备电池信息、应用切换记录、AI 总结缓存、认证令牌等
use crate::models::*;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
pub use worker::kv::KvStore;
use worker::*;

// ==================== 通用 KV 操作 ====================

/// 将 JSON 数据写入 KV
pub async fn kv_put_json<T: Serialize>(
    kv: &KvStore,
    key: &str,
    value: &T,
    ttl_seconds: Option<u64>,
) -> Result<()> {
    let json = serde_json::to_string(value).map_err(|e| Error::RustError(e.to_string()))?;
    let builder = kv.put(key, &json)?;
    match ttl_seconds {
        Some(ttl) => builder.expiration_ttl(ttl).execute().await?,
        None => builder.execute().await?,
    }
    Ok(())
}

/// 从 KV 读取 JSON 数据
pub async fn kv_get_json<T: DeserializeOwned>(kv: &KvStore, key: &str) -> Result<Option<T>> {
    match kv.get(key).text().await? {
        Some(text) => {
            let value =
                serde_json::from_str(&text).map_err(|e| Error::RustError(e.to_string()))?;
            Ok(Some(value))
        }
        None => Ok(None),
    }
}

// ==================== 设备列表 ====================

const DEVICES_KEY: &str = "devices";

/// 获取所有设备ID列表
pub async fn get_device_list(kv: &KvStore) -> Result<Vec<String>> {
    kv_get_json::<Vec<String>>(kv, DEVICES_KEY)
        .await
        .map(|opt| opt.unwrap_or_default())
}

/// 确保设备在列表中（如果不存在则添加）
pub async fn ensure_device_registered(kv: &KvStore, device_id: &str) -> Result<()> {
    let mut devices = get_device_list(kv).await?;
    if !devices.contains(&device_id.to_string()) {
        devices.push(device_id.to_string());
        kv_put_json(kv, DEVICES_KEY, &devices, None).await?;
    }
    Ok(())
}

// ==================== 电池信息 ====================

fn battery_key(device_id: &str) -> String {
    format!("battery:{}", device_id)
}

/// 获取设备电池信息
pub async fn get_battery_info(kv: &KvStore, device_id: &str) -> Result<BatteryInfo> {
    kv_get_json::<BatteryInfo>(kv, &battery_key(device_id))
        .await
        .map(|opt| opt.unwrap_or_default())
}

/// 保存设备电池信息
pub async fn set_battery_info(kv: &KvStore, device_id: &str, info: &BatteryInfo) -> Result<()> {
    // 7 天过期
    kv_put_json(kv, &battery_key(device_id), info, Some(604800)).await
}

// ==================== 应用切换记录 ====================

fn recent_key(device_id: &str) -> String {
    format!("recent:{}", device_id)
}

/// 获取设备最近的应用切换记录
pub async fn get_recent_switches(kv: &KvStore, device_id: &str) -> Result<Vec<AppSwitch>> {
    kv_get_json::<Vec<AppSwitch>>(kv, &recent_key(device_id))
        .await
        .map(|opt| opt.unwrap_or_default())
}

/// 保存设备应用切换记录（最多保留 20 条）
pub async fn set_recent_switches(
    kv: &KvStore,
    device_id: &str,
    switches: &[AppSwitch],
) -> Result<()> {
    let max_len = 20;
    let to_save: Vec<&AppSwitch> = switches.iter().take(max_len).collect();
    // 7 天过期
    kv_put_json(kv, &recent_key(device_id), &to_save, Some(604800)).await
}

// ==================== AI 总结缓存 ====================

fn ai_summary_key(device_id: &str) -> String {
    format!("ai_summary:{}", device_id)
}

/// 获取设备的最近 AI 总结
pub async fn get_ai_summary(kv: &KvStore, device_id: &str) -> Result<Option<AiSummaryRecord>> {
    kv_get_json::<AiSummaryRecord>(kv, &ai_summary_key(device_id)).await
}

/// 保存设备的 AI 总结
pub async fn set_ai_summary(
    kv: &KvStore,
    device_id: &str,
    record: &AiSummaryRecord,
) -> Result<()> {
    // 30 天过期
    kv_put_json(kv, &ai_summary_key(device_id), record, Some(2592000)).await
}

/// 获取所有设备的 AI 总结
pub async fn get_all_ai_summaries(
    kv: &KvStore,
    device_ids: &[String],
) -> Result<std::collections::HashMap<String, AiSummaryRecord>> {
    let mut summaries = std::collections::HashMap::new();
    for device_id in device_ids {
        if let Some(record) = get_ai_summary(kv, device_id).await? {
            summaries.insert(device_id.clone(), record);
        }
    }
    Ok(summaries)
}

// ==================== 用眼时长设备状态 ====================

fn eye_device_key(device_id: &str) -> String {
    format!("eye_device:{}", device_id)
}

const EYE_GLOBAL_KEY: &str = "eye_global";

/// 获取用眼时长设备状态
pub async fn get_eye_device_state(
    kv: &KvStore,
    device_id: &str,
) -> Result<Option<EyeDeviceState>> {
    kv_get_json::<EyeDeviceState>(kv, &eye_device_key(device_id)).await
}

/// 保存用眼时长设备状态
pub async fn set_eye_device_state(
    kv: &KvStore,
    device_id: &str,
    state: &EyeDeviceState,
) -> Result<()> {
    kv_put_json(kv, &eye_device_key(device_id), state, Some(86400)).await
}

/// 获取全局用眼状态
pub async fn get_eye_global_state(kv: &KvStore) -> Result<EyeGlobalState> {
    kv_get_json::<EyeGlobalState>(kv, EYE_GLOBAL_KEY)
        .await
        .map(|opt| {
            opt.unwrap_or(EyeGlobalState {
                active: false,
                last_record_time: None,
            })
        })
}

/// 保存全局用眼状态
pub async fn set_eye_global_state(kv: &KvStore, state: &EyeGlobalState) -> Result<()> {
    kv_put_json(kv, EYE_GLOBAL_KEY, state, Some(86400)).await
}

/// 获取所有用眼设备ID列表
pub async fn get_eye_device_ids(kv: &KvStore) -> Result<Vec<String>> {
    kv_get_json::<Vec<String>>(kv, "eye_devices")
        .await
        .map(|opt| opt.unwrap_or_default())
}

/// 确保用眼设备已注册
pub async fn ensure_eye_device_registered(kv: &KvStore, device_id: &str) -> Result<()> {
    let mut devices = get_eye_device_ids(kv).await?;
    if !devices.contains(&device_id.to_string()) {
        devices.push(device_id.to_string());
        kv_put_json(kv, "eye_devices", &devices, None).await?;
    }
    Ok(())
}

// ==================== 认证令牌 ====================

fn auth_token_key(token: &str) -> String {
    format!("auth_token:{}", token)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthTokenData {
    pub username: String,
    pub role: String,
    pub created_at: String,
}

/// 保存认证令牌
pub async fn set_auth_token(kv: &KvStore, token: &str, data: &AuthTokenData) -> Result<()> {
    // 7 天过期
    kv_put_json(kv, &auth_token_key(token), data, Some(604800)).await
}

/// 验证认证令牌
pub async fn verify_auth_token(kv: &KvStore, token: &str) -> Result<Option<AuthTokenData>> {
    kv_get_json::<AuthTokenData>(kv, &auth_token_key(token)).await
}

/// 删除认证令牌
#[allow(dead_code)]
pub async fn delete_auth_token(kv: &KvStore, token: &str) -> Result<()> {
    kv.delete(&auth_token_key(token)).await?;
    Ok(())
}

// ==================== 运行时配置 ====================

const CONFIG_KEY: &str = "runtime_config";

/// 获取运行时配置覆盖
pub async fn get_runtime_config(
    kv: &KvStore,
) -> Result<std::collections::HashMap<String, String>> {
    kv_get_json::<std::collections::HashMap<String, String>>(kv, CONFIG_KEY)
        .await
        .map(|opt| opt.unwrap_or_default())
}

/// 保存运行时配置覆盖
pub async fn set_runtime_config(
    kv: &KvStore,
    config: &std::collections::HashMap<String, String>,
) -> Result<()> {
    kv_put_json(kv, CONFIG_KEY, config, None).await
}

/// 读取配置值：优先取 KV 运行时覆盖，再回退到 wrangler.toml 环境变量
///
/// 使用场景：所有需要感知 `admin/config` 动态修改的配置项都应通过此函数读取。
pub async fn get_effective_config(
    kv: &KvStore,
    env: &worker::Env,
    key: &str,
    default: &str,
) -> String {
    if let Ok(config) = get_runtime_config(kv).await {
        if let Some(value) = config.get(key) {
            return value.clone();
        }
    }
    crate::utils::get_env_or(env, key, default)
}
