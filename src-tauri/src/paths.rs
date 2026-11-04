use std::path::{Path, PathBuf};
use std::fs;
use crate::models::Preferences;

pub const DEFAULT_SCHEMA_VERSION: u32 = 1;

/// Get the AppData directory for the Launcher (e.g. %APPDATA%\ClaudeLauncher or ~/Library/Application Support/ClaudeLauncher)
pub fn get_app_data_dir() -> PathBuf {
    if let Some(config_dir) = dirs::config_dir() {
        let dir = config_dir.join("ClaudeLauncher");
        let _ = fs::create_dir_all(&dir);
        let _ = fs::create_dir_all(dir.join("backups"));
        let _ = fs::create_dir_all(dir.join("logs"));
        return dir;
    }
    
    // Fallback to home dir
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let dir = home.join(".claude_launcher");
    let _ = fs::create_dir_all(&dir);
    let _ = fs::create_dir_all(dir.join("backups"));
    let _ = fs::create_dir_all(dir.join("logs"));
    dir
}

/// Resolves the active Claude configuration directory based on priorities:
/// 1. Custom directory from preferences (if set and valid)
/// 2. `CLAUDE_CONFIG_DIR` environment variable
/// 3. Standard user home directory: `~/.claude` (Windows: `%USERPROFILE%\.claude`)
pub fn resolve_claude_config_dir(prefs: &Preferences) -> (PathBuf, String) {
    // 1. Custom directory
    if prefs.config_dir_mode == "custom" {
        if let Some(ref custom_path) = prefs.custom_claude_config_dir {
            let p = PathBuf::from(custom_path.trim());
            if p.is_dir() {
                return (p, "custom".to_string());
            }
        }
    }

    // 2. CLAUDE_CONFIG_DIR env var
    if let Ok(env_path) = std::env::var("CLAUDE_CONFIG_DIR") {
        let p = PathBuf::from(env_path.trim());
        if p.is_dir() {
            return (p, "env".to_string());
        }
    }

    // 3. User home ~/.claude
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let default_dir = home.join(".claude");
    (default_dir, "default".to_string())
}

/// Sanitize and validate slug for settings file naming `my-<slug>.json`
/// Rules:
/// - Only ASCII lowercase, numbers, and hyphens
/// - Cannot be empty
/// - Cannot be a Windows reserved name (CON, PRN, AUX, NUL, COM1-9, LPT1-9)
/// - Strip any directory path components to prevent path traversal
pub fn sanitize_slug(input: &str) -> Result<String, String> {
    let mut slug = String::new();
    let mut prev_hyphen = false;

    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            prev_hyphen = false;
        } else if !prev_hyphen && !slug.is_empty() {
            slug.push('-');
            prev_hyphen = true;
        }
    }

    let slug = slug.trim_matches('-').to_string();

    if slug.is_empty() {
        return Err("标识符(slug)不能为空，且必须包含有效英文字符或数字".to_string());
    }

    // Windows reserved device names
    let reserved = [
        "con", "prn", "aux", "nul",
        "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8", "com9",
        "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
    ];

    if reserved.contains(&slug.as_str()) {
        return Err(format!("'{}' 是系统保留名称，不可作为分组标识符", slug));
    }

    Ok(slug)
}

/// Build safe file path for a group within the claude config directory
/// e.g. `<claude_config_dir>/my-<slug>.json`
pub fn get_safe_group_file_path(claude_config_dir: &Path, filename: &str) -> Result<PathBuf, String> {
    // Check if filename contains path separators
    if filename.contains('/') || filename.contains('\\') || filename.contains("..") {
        return Err("文件名不能包含路径分隔符或上级目录引用(..)".to_string());
    }

    let full_path = claude_config_dir.join(filename);
    Ok(full_path)
}

/// Validate directory existence and read/write permissions
pub fn validate_directory_permissions(dir: &Path) -> Result<(), String> {
    if !dir.exists() {
        // Try creating it
        fs::create_dir_all(dir)
            .map_err(|e| format!("目录不存在且无法创建: {} ({})", dir.display(), e))?;
    }

    // Test write permission with a temp probe file
    let probe_file = dir.join(format!(".write_test_{}", uuid::Uuid::new_v4()));
    fs::write(&probe_file, b"test")
        .map_err(|e| format!("目录无写入权限: {} ({})", dir.display(), e))?;
    
    // Clean up
    let _ = fs::remove_file(&probe_file);

    Ok(())
}
