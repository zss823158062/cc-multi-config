use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use crate::models::{
    LauncherConfig, NodesConfig, Group, RelayNode, Preferences,
    ConfigDirInfo, BaseSettingsStatus, SettingsDiffResult
};
use crate::paths::{
    get_app_data_dir, resolve_claude_config_dir, sanitize_slug,
    get_safe_group_file_path, validate_directory_permissions
};
use crate::backup::{create_backup_for_file, atomic_write_file};
use crate::settings_merger::{clean_managed_fields, merge_node_to_settings, generate_preview_diff};

pub struct AppState {
    pub config_lock: Mutex<()>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            config_lock: Mutex::new(()),
        }
    }
}

fn config_path() -> PathBuf {
    get_app_data_dir().join("config.json")
}

fn nodes_path() -> PathBuf {
    get_app_data_dir().join("nodes.json")
}

/// Load LauncherConfig from app_data/config.json, or initialize default
pub fn load_launcher_config() -> LauncherConfig {
    let p = config_path();
    if p.exists() {
        if let Ok(content) = fs::read_to_string(&p) {
            if let Ok(cfg) = serde_json::from_str::<LauncherConfig>(&content) {
                return cfg;
            }
        }
    }
    let default_cfg = LauncherConfig::default();
    let _ = save_launcher_config(&default_cfg);
    default_cfg
}

/// Save LauncherConfig to app_data/config.json
pub fn save_launcher_config(cfg: &LauncherConfig) -> Result<(), String> {
    let p = config_path();
    let json_str = serde_json::to_string_pretty(cfg)
        .map_err(|e| format!("序列化配置失败: {}", e))?;
    atomic_write_file(&p, &json_str)
}

/// Load NodesConfig from app_data/nodes.json, or initialize default
pub fn load_nodes_config() -> NodesConfig {
    let p = nodes_path();
    if p.exists() {
        if let Ok(content) = fs::read_to_string(&p) {
            if let Ok(cfg) = serde_json::from_str::<NodesConfig>(&content) {
                return cfg;
            }
        }
    }
    let default_cfg = NodesConfig::default();
    let _ = save_nodes_config(&default_cfg);
    default_cfg
}

/// Save NodesConfig to app_data/nodes.json
pub fn save_nodes_config(cfg: &NodesConfig) -> Result<(), String> {
    let p = nodes_path();
    let json_str = serde_json::to_string_pretty(cfg)
        .map_err(|e| format!("序列化节点配置失败: {}", e))?;
    atomic_write_file(&p, &json_str)
}

/// Get detailed status of the active Claude config directory
pub fn get_claude_config_dir_info(prefs: &Preferences) -> ConfigDirInfo {
    let (dir, mode) = resolve_claude_config_dir(prefs);
    let is_valid = validate_directory_permissions(&dir).is_ok();
    let base_settings_file = dir.join("settings.json");
    let has_base_settings = base_settings_file.exists() && base_settings_file.is_file();

    ConfigDirInfo {
        mode,
        resolved_path: dir.to_string_lossy().to_string(),
        is_valid,
        has_base_settings,
        base_settings_path: base_settings_file.to_string_lossy().to_string(),
        message: if !is_valid {
            Some("配置目录不可写或路径不存在，请检查权限".to_string())
        } else if !has_base_settings {
            Some("未检测到基础 settings.json，新建分组时将基于空白模板创建".to_string())
        } else {
            None
        },
    }
}

/// Check base settings.json file status
pub fn check_base_settings(prefs: &Preferences) -> BaseSettingsStatus {
    let (dir, _) = resolve_claude_config_dir(prefs);
    let base_file = dir.join("settings.json");
    let path_str = base_file.to_string_lossy().to_string();

    if !base_file.exists() {
        return BaseSettingsStatus {
            exists: false,
            readable: false,
            path: path_str,
            error: Some("基础 settings.json 文件不存在".to_string()),
        };
    }

    match fs::read_to_string(&base_file) {
        Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(_) => BaseSettingsStatus {
                exists: true,
                readable: true,
                path: path_str,
                error: None,
            },
            Err(e) => BaseSettingsStatus {
                exists: true,
                readable: false,
                path: path_str,
                error: Some(format!("基础 settings.json 格式非法: {}", e)),
            },
        },
        Err(e) => BaseSettingsStatus {
            exists: true,
            readable: false,
            path: path_str,
            error: Some(format!("读取基础 settings.json 失败: {}", e)),
        },
    }
}

/// Create an empty `{}` base settings.json if user does not have one
pub fn create_empty_base_settings_file(prefs: &Preferences) -> Result<String, String> {
    let (dir, _) = resolve_claude_config_dir(prefs);
    let _ = fs::create_dir_all(&dir);
    let base_file = dir.join("settings.json");

    if base_file.exists() {
        return Err("基础 settings.json 已经存在，不会重复创建".to_string());
    }

    atomic_write_file(&base_file, "{\n  \"env\": {},\n  \"permissions\": {\n    \"allow\": []\n  }\n}\n")?;
    Ok(base_file.to_string_lossy().to_string())
}

/// Create a new model group
pub fn create_group(
    name: &str,
    custom_slug: Option<&str>,
    initial_node_id: Option<&str>,
) -> Result<Group, String> {
    let mut config = load_launcher_config();

    let slug_source = custom_slug.unwrap_or(name);
    let slug = sanitize_slug(slug_source)?;

    // Check if group ID or settings file already exists in config
    let settings_filename = format!("my-{}.json", slug);
    if config.groups.iter().any(|g| g.id == slug || g.settings_file == settings_filename) {
        return Err(format!("分组标识符 '{}' 或文件名 '{}' 已存在，请更换名称", slug, settings_filename));
    }

    let (claude_dir, _) = resolve_claude_config_dir(&config.preferences);
    let target_file_path = get_safe_group_file_path(&claude_dir, &settings_filename)?;

    if target_file_path.exists() {
        return Err(format!("目标文件 '{}' 已在配置目录中存在，禁止静默覆盖", target_file_path.display()));
    }

    // Step 2 & 3: Check base settings.json
    let base_file = claude_dir.join("settings.json");
    let base_content = if base_file.exists() {
        fs::read_to_string(&base_file).unwrap_or_else(|_| "{}".to_string())
    } else {
        "{}".to_string()
    };

    // Clean existing connection & model override fields, keeping permissions, hooks, etc.
    let cleaned_base = clean_managed_fields(&base_content)?;
    let mut initial_json_str = serde_json::to_string_pretty(&cleaned_base).unwrap_or_else(|_| "{}".to_string());

    // Step 4 & 5: If initial node selected, merge node parameters
    let mut active_node_id: Option<String> = None;
    if let Some(node_id) = initial_node_id {
        let nodes_cfg = load_nodes_config();
        if let Some(node) = nodes_cfg.nodes.iter().find(|n| n.id == node_id) {
            initial_json_str = merge_node_to_settings(&initial_json_str, node, &slug)?;
            active_node_id = Some(node_id.to_string());
        }
    }

    // Step 6: Atomic write
    atomic_write_file(&target_file_path, &initial_json_str)?;

    let new_group = Group {
        id: slug.clone(),
        name: name.to_string(),
        settings_file: settings_filename,
        active_node_id: active_node_id.clone(),
        default_node_id: active_node_id,
    };

    config.groups.push(new_group.clone());
    save_launcher_config(&config)?;

    Ok(new_group)
}

/// Switch active node for a group
pub fn switch_group_node(group_id: &str, node_id: &str) -> Result<(), String> {
    let mut config = load_launcher_config();
    let group = config.groups.iter_mut().find(|g| g.id == group_id)
        .ok_or_else(|| format!("未找到分组: {}", group_id))?;

    let nodes_cfg = load_nodes_config();
    let node = nodes_cfg.nodes.iter().find(|n| n.id == node_id)
        .ok_or_else(|| format!("未找到中转节点: {}", node_id))?;

    let (claude_dir, _) = resolve_claude_config_dir(&config.preferences);
    let target_file_path = get_safe_group_file_path(&claude_dir, &group.settings_file)?;

    // 1. Read existing file content or copy from base if missing
    let current_content = if target_file_path.exists() {
        fs::read_to_string(&target_file_path)
            .map_err(|e| format!("读取分组配置失败: {}", e))?
    } else {
        let base_file = claude_dir.join("settings.json");
        if base_file.exists() {
            fs::read_to_string(&base_file).unwrap_or_else(|_| "{}".to_string())
        } else {
            "{}".to_string()
        }
    };

    // 2. Generate merged content
    let merged_content = merge_node_to_settings(&current_content, node, group_id)?;

    // 3. Backup before modifying
    if target_file_path.exists() {
        let _ = create_backup_for_file(group_id, &target_file_path)?;
    }

    // 4. Atomic write
    atomic_write_file(&target_file_path, &merged_content)?;

    // 5. Update active node in config
    group.active_node_id = Some(node_id.to_string());
    save_launcher_config(&config)?;

    Ok(())
}

/// Preview settings diff when switching to a node
pub fn preview_settings_diff(group_id: &str, node_id: &str) -> Result<SettingsDiffResult, String> {
    let config = load_launcher_config();
    let group = config.groups.iter().find(|g| g.id == group_id)
        .ok_or_else(|| format!("未找到分组: {}", group_id))?;

    let nodes_cfg = load_nodes_config();
    let node = nodes_cfg.nodes.iter().find(|n| n.id == node_id)
        .ok_or_else(|| format!("未找到节点: {}", node_id))?;

    let (claude_dir, _) = resolve_claude_config_dir(&config.preferences);
    let target_file_path = get_safe_group_file_path(&claude_dir, &group.settings_file)?;

    let current_content = if target_file_path.exists() {
        fs::read_to_string(&target_file_path).unwrap_or_else(|_| "{}".to_string())
    } else {
        "{}".to_string()
    };

    generate_preview_diff(group_id, &current_content, node, &target_file_path.to_string_lossy())
}

/// Read settings file content of a group
pub fn get_group_settings_content(group_id: &str) -> Result<String, String> {
    let config = load_launcher_config();
    let group = config.groups.iter().find(|g| g.id == group_id)
        .ok_or_else(|| format!("未找到分组: {}", group_id))?;

    let (claude_dir, _) = resolve_claude_config_dir(&config.preferences);
    let target_file_path = get_safe_group_file_path(&claude_dir, &group.settings_file)?;

    if !target_file_path.exists() {
        return Ok("{\n  \"env\": {},\n  \"permissions\": {}\n}".to_string());
    }

    fs::read_to_string(&target_file_path)
        .map_err(|e| format!("读取设置文件失败: {}", e))
}

/// Save manual edit of a group settings file
pub fn save_group_settings_content(group_id: &str, content: &str) -> Result<(), String> {
    // Validate JSON
    let _: serde_json::Value = serde_json::from_str(content)
        .map_err(|e| format!("JSON 语法错误: {}", e))?;

    let config = load_launcher_config();
    let group = config.groups.iter().find(|g| g.id == group_id)
        .ok_or_else(|| format!("未找到分组: {}", group_id))?;

    let (claude_dir, _) = resolve_claude_config_dir(&config.preferences);
    let target_file_path = get_safe_group_file_path(&claude_dir, &group.settings_file)?;

    if target_file_path.exists() {
        let _ = create_backup_for_file(group_id, &target_file_path)?;
    }

    atomic_write_file(&target_file_path, content)?;
    Ok(())
}

/// Save (add/update) a relay node. Optionally propagate to associated groups.
pub fn save_relay_node(node: RelayNode, apply_to_groups: bool) -> Result<RelayNode, String> {
    let mut nodes_cfg = load_nodes_config();

    if let Some(pos) = nodes_cfg.nodes.iter().position(|n| n.id == node.id) {
        nodes_cfg.nodes[pos] = node.clone();
    } else {
        nodes_cfg.nodes.push(node.clone());
    }

    save_nodes_config(&nodes_cfg)?;

    if apply_to_groups {
        let config = load_launcher_config();
        for group in &config.groups {
            if group.active_node_id.as_deref() == Some(&node.id) {
                let _ = switch_group_node(&group.id, &node.id);
            }
        }
    }

    Ok(node)
}

/// Delete a relay node
pub fn delete_relay_node(node_id: &str) -> Result<(), String> {
    let mut nodes_cfg = load_nodes_config();
    nodes_cfg.nodes.retain(|n| n.id != node_id);
    save_nodes_config(&nodes_cfg)?;

    // Clear active_node_id if any group uses this node
    let mut config = load_launcher_config();
    let mut modified = false;
    for g in config.groups.iter_mut() {
        if g.active_node_id.as_deref() == Some(node_id) {
            g.active_node_id = None;
            modified = true;
        }
        if g.default_node_id.as_deref() == Some(node_id) {
            g.default_node_id = None;
            modified = true;
        }
    }
    if modified {
        let _ = save_launcher_config(&config);
    }

    Ok(())
}

/// Delete a group
pub fn delete_group(group_id: &str, delete_file: bool) -> Result<(), String> {
    let mut config = load_launcher_config();
    let idx = config.groups.iter().position(|g| g.id == group_id)
        .ok_or_else(|| format!("未找到分组: {}", group_id))?;

    let group = config.groups.remove(idx);

    if delete_file {
        let (claude_dir, _) = resolve_claude_config_dir(&config.preferences);
        if let Ok(path) = get_safe_group_file_path(&claude_dir, &group.settings_file) {
            if path.exists() {
                let _ = fs::remove_file(path);
            }
        }
    }

    save_launcher_config(&config)?;
    Ok(())
}
