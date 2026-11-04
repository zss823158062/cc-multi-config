use std::path::Path;
use std::process::Command;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;
use crate::models::{
    Group, RelayNode, Preferences, ConfigDirInfo, BaseSettingsStatus,
    ClaudeBinaryInfo, TerminalInfo, NodeTestResult, SettingsDiffResult,
    BackupItem, LaunchResult, ConfigFileItem, OsInfo
};
use crate::store;
use crate::paths::{resolve_claude_config_dir, get_safe_group_file_path};
use crate::backup;
use crate::claude_detector;
use crate::node_tester;

#[tauri::command]
pub fn get_launcher_config() -> Result<crate::models::LauncherConfig, String> {
    Ok(store::load_launcher_config())
}

#[tauri::command]
pub fn get_groups() -> Result<Vec<Group>, String> {
    Ok(store::load_launcher_config().groups)
}

#[tauri::command]
pub fn create_group(name: String, slug: Option<String>, initial_node_id: Option<String>) -> Result<Group, String> {
    store::create_group(&name, slug.as_deref(), initial_node_id.as_deref())
}

#[tauri::command]
pub fn delete_group(group_id: String, delete_file: bool) -> Result<(), String> {
    store::delete_group(&group_id, delete_file)
}

#[tauri::command]
pub fn get_nodes() -> Result<Vec<RelayNode>, String> {
    Ok(store::load_nodes_config().nodes)
}

#[tauri::command]
pub fn save_node(node: RelayNode, apply_to_groups: bool) -> Result<RelayNode, String> {
    store::save_relay_node(node, apply_to_groups)
}

#[tauri::command]
pub fn delete_node(node_id: String) -> Result<(), String> {
    store::delete_relay_node(&node_id)
}

#[tauri::command]
pub async fn test_node(node: RelayNode, test_model: bool) -> Result<NodeTestResult, String> {
    Ok(node_tester::test_node_connectivity(&node, test_model).await)
}

#[tauri::command]
pub async fn test_all_nodes(test_model: bool) -> Result<Vec<NodeTestResult>, String> {
    let nodes = store::load_nodes_config().nodes;
    let mut results = Vec::new();
    for node in nodes {
        let res = node_tester::test_node_connectivity(&node, test_model).await;
        results.push(res);
    }
    Ok(results)
}

#[tauri::command]
pub fn switch_group_node(group_id: String, node_id: String) -> Result<(), String> {
    store::switch_group_node(&group_id, &node_id)
}

#[tauri::command]
pub fn get_group_settings_content(group_id: String) -> Result<String, String> {
    store::get_group_settings_content(&group_id)
}

#[tauri::command]
pub fn save_group_settings_content(group_id: String, content: String) -> Result<(), String> {
    store::save_group_settings_content(&group_id, &content)
}

#[tauri::command]
pub fn preview_settings_diff(group_id: String, node_id: String) -> Result<SettingsDiffResult, String> {
    store::preview_settings_diff(&group_id, &node_id)
}

#[tauri::command]
pub fn get_claude_config_dir_info() -> Result<ConfigDirInfo, String> {
    let config = store::load_launcher_config();
    Ok(store::get_claude_config_dir_info(&config.preferences))
}

#[tauri::command]
pub fn check_base_settings() -> Result<BaseSettingsStatus, String> {
    let config = store::load_launcher_config();
    Ok(store::check_base_settings(&config.preferences))
}

#[tauri::command]
pub fn create_empty_base_settings() -> Result<String, String> {
    let config = store::load_launcher_config();
    store::create_empty_base_settings_file(&config.preferences)
}

#[tauri::command]
pub fn get_preferences() -> Result<Preferences, String> {
    Ok(store::load_launcher_config().preferences)
}

#[tauri::command]
pub fn save_preferences(preferences: Preferences) -> Result<(), String> {
    let mut config = store::load_launcher_config();
    config.preferences = preferences;
    store::save_launcher_config(&config)
}

#[tauri::command]
pub fn detect_claude_binary(custom_path: Option<String>) -> Result<ClaudeBinaryInfo, String> {
    Ok(claude_detector::detect_claude_binary(custom_path.as_deref()))
}

#[tauri::command]
pub fn get_available_terminals() -> Result<Vec<TerminalInfo>, String> {
    Ok(claude_detector::get_available_terminals())
}

#[tauri::command]
pub fn list_backups(group_id: Option<String>) -> Result<Vec<BackupItem>, String> {
    Ok(backup::list_backups(group_id.as_deref()))
}

#[tauri::command]
pub fn restore_backup(backup_filename: String, group_id: String) -> Result<(), String> {
    let config = store::load_launcher_config();
    let group = config.groups.iter().find(|g| g.id == group_id)
        .ok_or_else(|| format!("未找到分组: {}", group_id))?;

    let (claude_dir, _) = resolve_claude_config_dir(&config.preferences);
    let target_file_path = get_safe_group_file_path(&claude_dir, &group.settings_file)?;

    backup::restore_backup_file(&backup_filename, &target_file_path, &group_id)
}

#[tauri::command]
pub fn open_in_explorer(path: String) -> Result<(), String> {
    let p = Path::new(&path);
    if !p.exists() {
        return Err(format!("路径不存在: {}", path));
    }

    #[cfg(windows)]
    {
        if p.is_dir() {
            let _ = Command::new("explorer.exe").arg(&path).spawn();
        } else {
            let _ = Command::new("explorer.exe").arg(format!("/select,{}", path)).spawn();
        }
        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        if p.is_dir() {
            let _ = Command::new("open").arg(&path).spawn();
        } else {
            let _ = Command::new("open").arg("-R").arg(&path).spawn();
        }
        Ok(())
    }

    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = open::that(&path).map_err(|e| format!("打开失败: {}", e))?;
        Ok(())
    }
}

#[tauri::command]
pub fn open_file_in_editor(path: String) -> Result<(), String> {
    let p = Path::new(&path);
    if !p.exists() {
        return Err(format!("文件不存在: {}", path));
    }
    open::that(&path).map_err(|e| format!("用默认程序打开失败: {}", e))
}

#[tauri::command]
pub fn select_folder(default_path: Option<String>) -> Result<Option<String>, String> {
    #[cfg(windows)]
    {
        let initial_dir_clause = if let Some(ref d) = default_path {
            format!("$f.SelectedPath = '{}';", d.replace('\'', "''"))
        } else {
            String::new()
        };

        let ps_code = format!(
            "Add-Type -AssemblyName System.Windows.Forms; $f = New-Object System.Windows.Forms.FolderBrowserDialog; $f.Description = '请选择项目工作目录'; {} if ($f.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {{ $f.SelectedPath }}",
            initial_dir_clause
        );

        let mut ps_cmd = Command::new("powershell.exe");
        #[cfg(windows)]
        ps_cmd.creation_flags(CREATE_NO_WINDOW);
        let output = ps_cmd
            .arg("-NoProfile")
            .arg("-Command")
            .arg(&ps_code)
            .output()
            .map_err(|e| format!("无法启动文件夹选择器: {}", e))?;

        if output.status.success() {
            let selected = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if selected.is_empty() {
                Ok(None)
            } else {
                Ok(Some(selected))
            }
        } else {
            Ok(None)
        }
    }

    #[cfg(target_os = "macos")]
    {
        let applescript = "POSIX path of (choose folder with prompt \"请选择项目工作目录\")";
        let output = Command::new("osascript")
            .arg("-e")
            .arg(applescript)
            .output()
            .map_err(|e| format!("无法启动文件夹选择器: {}", e))?;

        if output.status.success() {
            let selected = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if selected.is_empty() {
                Ok(None)
            } else {
                Ok(Some(selected))
            }
        } else {
            Ok(None)
        }
    }

    #[cfg(not(any(windows, target_os = "macos")))]
    {
        Ok(None)
    }
}

#[tauri::command]
pub fn launch_claude(
    group_id: String,
    project_dir: String,
    terminal_override: Option<String>,
) -> Result<LaunchResult, String> {
    let mut config = store::load_launcher_config();
    let group = config.groups.iter().find(|g| g.id == group_id)
        .ok_or_else(|| format!("未找到分组: {}", group_id))?;

    // Remember last project dir
    config.preferences.last_project_dir = Some(project_dir.clone());
    let _ = store::save_launcher_config(&config);

    // Detect claude executable
    let claude_info = claude_detector::detect_claude_binary(config.preferences.claude_executable_path.as_deref());
    if !claude_info.is_detected || claude_info.path.is_empty() {
        return Err("未找到 Claude Code 可执行文件，请在设置中配置正确路径".to_string());
    }

    // Resolve group settings path
    let (claude_dir, _) = resolve_claude_config_dir(&config.preferences);
    let settings_path = get_safe_group_file_path(&claude_dir, &group.settings_file)?;

    if !settings_path.exists() {
        return Err(format!("分组配置文件不存在: {}，请先切换或配置中转节点", settings_path.display()));
    }

    let proj_path = std::path::PathBuf::from(&project_dir);
    if !proj_path.exists() || !proj_path.is_dir() {
        return Err(format!("指定的工作目录不存在或不是文件夹: {}", project_dir));
    }

    let term_choice = terminal_override
        .unwrap_or_else(|| config.preferences.terminal.clone());

    claude_detector::launch_claude_process(
        &claude_info.path,
        &settings_path,
        &proj_path,
        &term_choice,
    )
}

#[tauri::command]
pub fn list_config_files() -> Result<Vec<ConfigFileItem>, String> {
    let config = store::load_launcher_config();
    let (claude_dir, _) = resolve_claude_config_dir(&config.preferences);
    let mut items = Vec::new();

    // 1. Base settings.json
    let base_file = claude_dir.join("settings.json");
    let base_exists = base_file.exists();
    items.push(ConfigFileItem {
        name: "settings.json".to_string(),
        file_type: "base".to_string(),
        path: base_file.to_string_lossy().to_string(),
        exists: base_exists,
        size_bytes: if base_exists { base_file.metadata().map(|m| m.len()).unwrap_or(0) } else { 0 },
    });

    // 2. Scan dir for group files and other json files
    if let Ok(entries) = std::fs::read_dir(&claude_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if file_name == "settings.json" || file_name.starts_with('.') {
                    continue;
                }
                // Only show my-*.json group files
                if file_name.starts_with("my-") && file_name.ends_with(".json") {
                    items.push(ConfigFileItem {
                        name: file_name.to_string(),
                        file_type: "group".to_string(),
                        path: path.to_string_lossy().to_string(),
                        exists: true,
                        size_bytes: entry.metadata().map(|m| m.len()).unwrap_or(0),
                    });
                }
            }
        }
    }

    Ok(items)
}

#[tauri::command]
pub fn get_os_info() -> Result<OsInfo, String> {
    let os_name = std::env::consts::OS.to_string();
    let is_windows = cfg!(windows);
    let is_macos = cfg!(target_os = "macos");
    let home = dirs::home_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    let default_repr = if is_windows {
        "%USERPROFILE%\\.claude\\".to_string()
    } else {
        "~/.claude/".to_string()
    };

    Ok(OsInfo {
        os: os_name,
        is_windows,
        is_macos,
        user_home: home,
        default_claude_dir_repr: default_repr,
    })
}
