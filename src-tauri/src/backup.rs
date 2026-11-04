use std::fs;
use std::path::{Path, PathBuf};
use chrono::Local;
use crate::models::BackupItem;
use crate::paths::get_app_data_dir;

const MAX_BACKUPS_PER_GROUP: usize = 10;

/// Atomically write content to a file.
/// First writes to a temporary file in the same directory, flushes, then renames/replaces it.
pub fn atomic_write_file(target: &Path, content: &str) -> Result<(), String> {
    let parent = target.parent().ok_or_else(|| "无法获取目标文件所在目录".to_string())?;
    let _ = fs::create_dir_all(parent);

    let temp_name = format!(
        ".{}.tmp_{}",
        target.file_name().and_then(|n| n.to_str()).unwrap_or("file"),
        uuid::Uuid::new_v4()
    );
    let temp_path = parent.join(temp_name);

    // Write content
    fs::write(&temp_path, content.as_bytes())
        .map_err(|e| format!("写入临时文件失败 ({}): {}", temp_path.display(), e))?;

    // Attempt replace/rename
    #[cfg(windows)]
    {
        // On Windows, if target exists, std::fs::rename might fail if not handled or on older fs.
        // If target exists, try remove first or direct rename.
        if target.exists() {
            let _ = fs::remove_file(target);
        }
        if let Err(e) = fs::rename(&temp_path, target) {
            let _ = fs::remove_file(&temp_path);
            return Err(format!("原子重命名失败: {}", e));
        }
    }

    #[cfg(not(windows))]
    {
        if let Err(e) = fs::rename(&temp_path, target) {
            let _ = fs::remove_file(&temp_path);
            return Err(format!("原子重命名失败: {}", e));
        }
    }

    Ok(())
}

/// Create a timestamped backup before modifying a settings file.
/// Saves to `<app_data>/backups/<group_slug>.<timestamp>.json`
pub fn create_backup_for_file(group_id: &str, file_path: &Path) -> Result<PathBuf, String> {
    if !file_path.exists() {
        return Ok(PathBuf::new());
    }

    let content = fs::read_to_string(file_path)
        .map_err(|e| format!("读取待备份文件失败: {}", e))?;

    let backups_dir = get_app_data_dir().join("backups");
    let _ = fs::create_dir_all(&backups_dir);

    let now_str = Local::now().format("%Y%m%d_%H%M%S").to_string();
    let backup_filename = format!("{}.{}.json", group_id, now_str);
    let backup_path = backups_dir.join(&backup_filename);

    atomic_write_file(&backup_path, &content)?;

    // Prune backups for this group to keep at most MAX_BACKUPS_PER_GROUP
    prune_old_backups(group_id, &backups_dir);

    Ok(backup_path)
}

/// Keep only the latest MAX_BACKUPS_PER_GROUP backups for a given group
fn prune_old_backups(group_id: &str, backups_dir: &Path) {
    if let Ok(entries) = fs::read_dir(backups_dir) {
        let prefix = format!("{}.", group_id);
        let mut group_backups: Vec<PathBuf> = Vec::new();

        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if file_name.starts_with(&prefix) && file_name.ends_with(".json") {
                    group_backups.push(path);
                }
            }
        }

        // Sort by filename (which includes timestamp) descending
        group_backups.sort_by(|a, b| b.cmp(a));

        // Delete any beyond the limit
        if group_backups.len() > MAX_BACKUPS_PER_GROUP {
            for old_file in &group_backups[MAX_BACKUPS_PER_GROUP..] {
                let _ = fs::remove_file(old_file);
            }
        }
    }
}

/// List all available backups, optionally filtered by group_id
pub fn list_backups(filter_group_id: Option<&str>) -> Vec<BackupItem> {
    let backups_dir = get_app_data_dir().join("backups");
    let mut items = Vec::new();

    if let Ok(entries) = fs::read_dir(&backups_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if file_name.ends_with(".json") {
                    let parts: Vec<&str> = file_name.trim_end_matches(".json").split('.').collect();
                    if parts.len() >= 2 {
                        let group_id = parts[0];
                        let timestamp = parts[1];

                        if let Some(target_group) = filter_group_id {
                            if group_id != target_group {
                                continue;
                            }
                        }

                        let size_bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);
                        items.push(BackupItem {
                            filename: file_name.to_string(),
                            group_id: group_id.to_string(),
                            timestamp: timestamp.to_string(),
                            size_bytes,
                            full_path: path.to_string_lossy().to_string(),
                        });
                    }
                }
            }
        }
    }

    // Sort newest first
    items.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    items
}

/// Restore a backup to the target group file
pub fn restore_backup_file(backup_filename: &str, target_path: &Path, group_id: &str) -> Result<(), String> {
    let backups_dir = get_app_data_dir().join("backups");
    let backup_path = backups_dir.join(backup_filename);

    if !backup_path.exists() {
        return Err(format!("备份文件不存在: {}", backup_filename));
    }

    let backup_content = fs::read_to_string(&backup_path)
        .map_err(|e| format!("读取备份文件失败: {}", e))?;

    // Create safety backup of current target if it exists
    if target_path.exists() {
        let _ = create_backup_for_file(group_id, target_path);
    }

    // Write restored content atomically
    atomic_write_file(target_path, &backup_content)?;

    Ok(())
}
