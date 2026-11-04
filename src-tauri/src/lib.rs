pub mod models;
pub mod paths;
pub mod backup;
pub mod settings_merger;
pub mod claude_detector;
pub mod node_tester;
pub mod store;
pub mod commands;
#[cfg(test)]
mod tests;

use commands::*;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_launcher_config,
            get_groups,
            create_group,
            delete_group,
            get_nodes,
            save_node,
            delete_node,
            test_node,
            test_all_nodes,
            switch_group_node,
            get_group_settings_content,
            save_group_settings_content,
            preview_settings_diff,
            get_claude_config_dir_info,
            check_base_settings,
            create_empty_base_settings,
            get_preferences,
            save_preferences,
            detect_claude_binary,
            get_available_terminals,
            list_backups,
            restore_backup,
            open_in_explorer,
            open_file_in_editor,
            select_folder,
            launch_claude,
            list_config_files,
            get_os_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
