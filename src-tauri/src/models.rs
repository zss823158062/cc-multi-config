use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub id: String,
    pub name: String,
    pub settings_file: String,
    pub active_node_id: Option<String>,
    pub default_node_id: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct NodeModels {
    pub opus: Option<String>,
    pub sonnet: Option<String>,
    pub haiku: Option<String>,
    pub default_model: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RelayNode {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub auth_mode: String, // "authToken" or "apiKey"
    pub auth_token: String,
    pub models: NodeModels,
    pub custom_env: Option<HashMap<String, String>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Preferences {
    pub config_dir_mode: String, // "auto" or "custom"
    pub custom_claude_config_dir: Option<String>,
    pub terminal: String,        // "system", "wt", "powershell", "cmd", "iterm2", "terminal"
    pub claude_executable_path: Option<String>,
    pub last_project_dir: Option<String>,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            config_dir_mode: "auto".to_string(),
            custom_claude_config_dir: None,
            terminal: "system".to_string(),
            claude_executable_path: None,
            last_project_dir: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LauncherConfig {
    pub schema_version: u32,
    pub groups: Vec<Group>,
    pub preferences: Preferences,
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            schema_version: 1,
            groups: vec![
                Group {
                    id: "opus".to_string(),
                    name: "Opus".to_string(),
                    settings_file: "my-opus.json".to_string(),
                    active_node_id: None,
                    default_node_id: None,
                },
                Group {
                    id: "sonnet".to_string(),
                    name: "Sonnet".to_string(),
                    settings_file: "my-sonnet.json".to_string(),
                    active_node_id: None,
                    default_node_id: None,
                },
            ],
            preferences: Preferences::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NodesConfig {
    pub schema_version: u32,
    pub nodes: Vec<RelayNode>,
}

impl Default for NodesConfig {
    fn default() -> Self {
        Self {
            schema_version: 1,
            nodes: vec![
                RelayNode {
                    id: "node-official".to_string(),
                    name: "Anthropic 官方直连".to_string(),
                    base_url: "https://api.anthropic.com".to_string(),
                    auth_mode: "authToken".to_string(),
                    auth_token: "".to_string(),
                    models: NodeModels {
                        opus: Some("claude-3-opus-20240229".to_string()),
                        sonnet: Some("claude-3-7-sonnet-20250219".to_string()),
                        haiku: Some("claude-3-5-haiku-20241022".to_string()),
                        default_model: Some("claude-3-7-sonnet-20250219".to_string()),
                    },
                    custom_env: None,
                }
            ],
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NodeTestResult {
    pub node_id: String,
    pub connectivity_ok: bool,
    pub latency_ms: Option<u64>,
    pub status_code: Option<u16>,
    pub model_ok: Option<bool>,
    pub message: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BackupItem {
    pub filename: String,
    pub group_id: String,
    pub timestamp: String,
    pub size_bytes: u64,
    pub full_path: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BaseSettingsStatus {
    pub exists: bool,
    pub readable: bool,
    pub path: String,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ConfigDirInfo {
    pub mode: String,
    pub resolved_path: String,
    pub is_valid: bool,
    pub has_base_settings: bool,
    pub base_settings_path: String,
    pub message: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeBinaryInfo {
    pub path: String,
    pub is_detected: bool,
    pub version: Option<String>,
    pub source: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TerminalInfo {
    pub id: String,
    pub name: String,
    pub is_available: bool,
    pub is_default: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SettingsDiffResult {
    pub group_id: String,
    pub current_content: String,
    pub preview_content: String,
    pub diff_summary: Vec<String>,
    pub target_file_path: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LaunchResult {
    pub success: bool,
    pub message: String,
    pub terminal_used: String,
    pub command: String,
}
