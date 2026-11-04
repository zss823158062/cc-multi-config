use serde_json::{json, Value};
use crate::models::{RelayNode, SettingsDiffResult};

pub const MANAGED_ENV_KEYS: &[&str] = &[
    "ANTHROPIC_BASE_URL",
    "ANTHROPIC_AUTH_TOKEN",
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_MODEL",
    "ANTHROPIC_DEFAULT_OPUS_MODEL",
    "ANTHROPIC_DEFAULT_SONNET_MODEL",
    "ANTHROPIC_DEFAULT_HAIKU_MODEL",
];

/// Clean managed whitelist keys from an existing JSON string or base settings
pub fn clean_managed_fields(json_str: &str) -> Result<Value, String> {
    let mut root: Value = if json_str.trim().is_empty() {
        json!({})
    } else {
        serde_json::from_str(json_str).map_err(|e| format!("解析设置文件 JSON 失败: {}", e))?
    };

    if let Some(obj) = root.as_object_mut() {
        if let Some(env_val) = obj.get_mut("env") {
            if let Some(env_obj) = env_val.as_object_mut() {
                for key in MANAGED_ENV_KEYS {
                    env_obj.remove(*key);
                }
            }
        }
    }

    Ok(root)
}

/// Apply a RelayNode configuration to an existing or initial group JSON
pub fn merge_node_to_settings(
    existing_json_str: &str,
    node: &RelayNode,
    group_id: &str,
) -> Result<String, String> {
    let mut root = clean_managed_fields(existing_json_str)?;

    if !root.is_object() {
        root = json!({});
    }

    let root_obj = root.as_object_mut().unwrap();

    // Ensure `env` object exists
    if !root_obj.contains_key("env") || !root_obj["env"].is_object() {
        root_obj.insert("env".to_string(), json!({}));
    }

    let env_obj = root_obj.get_mut("env").unwrap().as_object_mut().unwrap();

    // 1. Base URL
    if !node.base_url.trim().is_empty() {
        env_obj.insert(
            "ANTHROPIC_BASE_URL".to_string(),
            Value::String(node.base_url.trim().to_string()),
        );
    }

    // 2. Auth Token / API Key
    if !node.auth_token.trim().is_empty() {
        if node.auth_mode == "apiKey" {
            env_obj.insert(
                "ANTHROPIC_API_KEY".to_string(),
                Value::String(node.auth_token.trim().to_string()),
            );
        } else {
            env_obj.insert(
                "ANTHROPIC_AUTH_TOKEN".to_string(),
                Value::String(node.auth_token.trim().to_string()),
            );
        }
    }

    // 3. Model mappings
    if let Some(ref opus) = node.models.opus {
        if !opus.trim().is_empty() {
            env_obj.insert(
                "ANTHROPIC_DEFAULT_OPUS_MODEL".to_string(),
                Value::String(opus.trim().to_string()),
            );
        }
    }

    if let Some(ref sonnet) = node.models.sonnet {
        if !sonnet.trim().is_empty() {
            env_obj.insert(
                "ANTHROPIC_DEFAULT_SONNET_MODEL".to_string(),
                Value::String(sonnet.trim().to_string()),
            );
        }
    }

    if let Some(ref haiku) = node.models.haiku {
        if !haiku.trim().is_empty() {
            env_obj.insert(
                "ANTHROPIC_DEFAULT_HAIKU_MODEL".to_string(),
                Value::String(haiku.trim().to_string()),
            );
        }
    }

    // Set active ANTHROPIC_MODEL based on group_id or default_model
    let selected_model = match group_id.to_lowercase().as_str() {
        "opus" => node.models.opus.as_ref().or(node.models.default_model.as_ref()),
        "sonnet" => node.models.sonnet.as_ref().or(node.models.default_model.as_ref()),
        "haiku" => node.models.haiku.as_ref().or(node.models.default_model.as_ref()),
        _ => node.models.default_model.as_ref().or(node.models.sonnet.as_ref()),
    };

    if let Some(model) = selected_model {
        if !model.trim().is_empty() {
            env_obj.insert(
                "ANTHROPIC_MODEL".to_string(),
                Value::String(model.trim().to_string()),
            );
        }
    }

    // 4. Custom extra env if present
    if let Some(ref custom_env) = node.custom_env {
        for (k, v) in custom_env {
            if !MANAGED_ENV_KEYS.contains(&k.as_str()) {
                env_obj.insert(k.clone(), Value::String(v.clone()));
            }
        }
    }

    // Format with 2 spaces indentation
    serde_json::to_string_pretty(&root).map_err(|e| format!("序列化 JSON 失败: {}", e))
}

/// Generate a comparison diff between current content and preview content
pub fn generate_preview_diff(
    group_id: &str,
    current_content: &str,
    target_node: &RelayNode,
    target_file_path: &str,
) -> Result<SettingsDiffResult, String> {
    let preview = merge_node_to_settings(current_content, target_node, group_id)?;

    let mut diff_summary = Vec::new();

    let cur_val: Value = serde_json::from_str(if current_content.trim().is_empty() { "{}" } else { current_content })
        .unwrap_or(json!({}));
    let prev_val: Value = serde_json::from_str(&preview).unwrap_or(json!({}));

    let cur_env = cur_val.get("env").and_then(|v| v.as_object()).cloned().unwrap_or_default();
    let prev_env = prev_val.get("env").and_then(|v| v.as_object()).cloned().unwrap_or_default();

    for key in MANAGED_ENV_KEYS {
        let old_v = cur_env.get(*key).and_then(|v| v.as_str());
        let new_v = prev_env.get(*key).and_then(|v| v.as_str());

        match (old_v, new_v) {
            (Some(o), Some(n)) if o != n => {
                if key.contains("TOKEN") || key.contains("KEY") {
                    diff_summary.push(format!("修改 {}: [******] -> [******]", key));
                } else {
                    diff_summary.push(format!("修改 {}: \"{}\" -> \"{}\"", key, o, n));
                }
            }
            (None, Some(n)) => {
                if key.contains("TOKEN") || key.contains("KEY") {
                    diff_summary.push(format!("新增 {}: [******]", key));
                } else {
                    diff_summary.push(format!("新增 {}: \"{}\"", key, n));
                }
            }
            (Some(o), None) => {
                diff_summary.push(format!("移除 {}: (原值: {})", key, if key.contains("TOKEN") || key.contains("KEY") { "******" } else { o }));
            }
            _ => {}
        }
    }

    if diff_summary.is_empty() {
        diff_summary.push("连接与模型配置无变化".to_string());
    }

    Ok(SettingsDiffResult {
        group_id: group_id.to_string(),
        current_content: current_content.to_string(),
        preview_content: preview,
        diff_summary,
        target_file_path: target_file_path.to_string(),
    })
}
