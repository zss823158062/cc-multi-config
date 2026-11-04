use std::path::{Path, PathBuf};
use std::process::Command;
use crate::models::{ClaudeBinaryInfo, TerminalInfo, LaunchResult};

/// Detect claude binary path on the system
pub fn detect_claude_binary(custom_path: Option<&str>) -> ClaudeBinaryInfo {
    // 1. Check custom path if provided
    if let Some(p) = custom_path {
        let trimmed = p.trim();
        if !trimmed.is_empty() {
            let path = PathBuf::from(trimmed);
            if path.exists() {
                let ver = get_claude_version(&path);
                return ClaudeBinaryInfo {
                    path: path.to_string_lossy().to_string(),
                    is_detected: true,
                    version: ver,
                    source: "用户自定义设置".to_string(),
                };
            }
        }
    }

    // 2. Check PATH using where.exe or which
    #[cfg(windows)]
    {
        if let Ok(output) = Command::new("where.exe").arg("claude").output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        let path = PathBuf::from(trimmed);
                        if path.exists() {
                            let ver = get_claude_version(&path);
                            return ClaudeBinaryInfo {
                                path: path.to_string_lossy().to_string(),
                                is_detected: true,
                                version: ver,
                                source: "系统环境变量 (PATH)".to_string(),
                            };
                        }
                    }
                }
            }
        }
    }

    #[cfg(not(windows))]
    {
        if let Ok(output) = Command::new("which").arg("claude").output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let trimmed = stdout.trim();
                if !trimmed.is_empty() {
                    let path = PathBuf::from(trimmed);
                    if path.exists() {
                        let ver = get_claude_version(&path);
                        return ClaudeBinaryInfo {
                            path: path.to_string_lossy().to_string(),
                            is_detected: true,
                            version: ver,
                            source: "系统环境变量 (PATH)".to_string(),
                        };
                    }
                }
            }
        }
    }

    // 3. Check common known locations
    let mut candidate_paths: Vec<PathBuf> = Vec::new();

    if let Some(home) = dirs::home_dir() {
        #[cfg(windows)]
        {
            candidate_paths.push(home.join(".local").join("bin").join("claude.exe"));
            if let Some(appdata) = dirs::config_dir() {
                candidate_paths.push(appdata.join("npm").join("claude.cmd"));
            }
            if let Some(local_appdata) = dirs::data_local_dir() {
                candidate_paths.push(local_appdata.join("Programs").join("claude").join("claude.exe"));
            }
        }

        #[cfg(not(windows))]
        {
            candidate_paths.push(home.join(".local").join("bin").join("claude"));
            candidate_paths.push(PathBuf::from("/usr/local/bin/claude"));
            candidate_paths.push(PathBuf::from("/opt/homebrew/bin/claude"));
        }
    }

    for path in candidate_paths {
        if path.exists() {
            let ver = get_claude_version(&path);
            return ClaudeBinaryInfo {
                path: path.to_string_lossy().to_string(),
                is_detected: true,
                version: ver,
                source: "常见标准安装目录".to_string(),
            };
        }
    }

    ClaudeBinaryInfo {
        path: "".to_string(),
        is_detected: false,
        version: None,
        source: "未找到 (请在设置中指定路径)".to_string(),
    }
}

fn get_claude_version(path: &Path) -> Option<String> {
    let output = Command::new(path).arg("--version").output().ok()?;
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        Some(stdout.trim().to_string())
    } else {
        None
    }
}

/// Detect available terminals on the host OS
pub fn get_available_terminals() -> Vec<TerminalInfo> {
    let mut list = Vec::new();

    #[cfg(windows)]
    {
        // Check Windows Terminal (wt.exe)
        let wt_available = Command::new("where.exe").arg("wt.exe").output().map(|o| o.status.success()).unwrap_or(false);
        list.push(TerminalInfo {
            id: "wt".to_string(),
            name: "Windows Terminal (推荐)".to_string(),
            is_available: wt_available,
            is_default: wt_available,
        });

        // PowerShell
        list.push(TerminalInfo {
            id: "powershell".to_string(),
            name: "PowerShell".to_string(),
            is_available: true,
            is_default: !wt_available,
        });

        // CMD
        list.push(TerminalInfo {
            id: "cmd".to_string(),
            name: "Command Prompt (CMD)".to_string(),
            is_available: true,
            is_default: false,
        });
    }

    #[cfg(target_os = "macos")]
    {
        // Terminal.app is always available on macOS
        list.push(TerminalInfo {
            id: "terminal".to_string(),
            name: "Terminal.app (系统终端)".to_string(),
            is_available: true,
            is_default: true,
        });

        let iterm_installed = Path::new("/Applications/iTerm.app").exists() || Path::new("~/Applications/iTerm.app").exists();
        list.push(TerminalInfo {
            id: "iterm2".to_string(),
            name: "iTerm2".to_string(),
            is_available: iterm_installed,
            is_default: false,
        });
    }

    #[cfg(not(any(windows, target_os = "macos")))]
    {
        list.push(TerminalInfo {
            id: "system".to_string(),
            name: "系统默认终端".to_string(),
            is_available: true,
            is_default: true,
        });
    }

    list
}

/// Launch Claude Code in a new terminal window
pub fn launch_claude_process(
    claude_bin: &str,
    settings_path: &Path,
    project_dir: &Path,
    terminal_choice: &str,
) -> Result<LaunchResult, String> {
    if !Path::new(claude_bin).exists() {
        return Err(format!("Claude 可执行文件不存在: {}", claude_bin));
    }
    if !settings_path.exists() {
        return Err(format!("分组 settings 文件不存在: {}", settings_path.display()));
    }
    if !project_dir.exists() {
        return Err(format!("指定的工作目录不存在: {}", project_dir.display()));
    }

    let settings_abs = settings_path.canonicalize()
        .unwrap_or_else(|_| settings_path.to_path_buf());
    let settings_str = settings_abs.to_string_lossy().to_string();
    let project_str = project_dir.canonicalize()
        .unwrap_or_else(|_| project_dir.to_path_buf()).to_string_lossy().to_string();

    #[cfg(windows)]
    {
        let term = if terminal_choice == "system" {
            let available = get_available_terminals();
            if available.iter().any(|t| t.id == "wt" && t.is_available) {
                "wt"
            } else {
                "powershell"
            }
        } else {
            terminal_choice
        };

        match term {
            "wt" => {
                // wt.exe -d "<project_dir>" cmd /k ""claude_bin" --settings "settings_path""
                // This ensures Chinese paths and spaces in paths are respected correctly
                let cmd_arg = format!("\"\"{}\" --settings \"{}\"\"", claude_bin, settings_str);
                let mut child = Command::new("wt.exe");
                child.arg("-d").arg(&project_str)
                    .arg("cmd.exe").arg("/k").arg(&cmd_arg);

                child.spawn().map_err(|e| format!("启动 Windows Terminal 失败: {}", e))?;

                Ok(LaunchResult {
                    success: true,
                    message: "已在 Windows Terminal 中启动 Claude Code".to_string(),
                    terminal_used: "Windows Terminal".to_string(),
                    command: format!("wt -d \"{}\" \"{}\" --settings \"{}\"", project_str, claude_bin, settings_str),
                })
            }
            "powershell" => {
                // powershell -NoExit -Command "Set-Location -LiteralPath '...'; & '...' --settings '...'"
                let ps_script = format!(
                    "Set-Location -LiteralPath '{}'; & '{}' --settings '{}'",
                    project_str.replace('\'', "''"),
                    claude_bin.replace('\'', "''"),
                    settings_str.replace('\'', "''")
                );

                let mut child = Command::new("powershell.exe");
                child.arg("-NoExit").arg("-Command").arg(&ps_script);
                child.current_dir(project_dir);

                child.spawn().map_err(|e| format!("启动 PowerShell 失败: {}", e))?;

                Ok(LaunchResult {
                    success: true,
                    message: "已在 PowerShell 中启动 Claude Code".to_string(),
                    terminal_used: "PowerShell".to_string(),
                    command: ps_script,
                })
            }
            "cmd" => {
                // cmd.exe /k "cd /d "..." && "..." --settings "...""
                let cmd_str = format!(
                    "cd /d \"{}\" && \"{}\" --settings \"{}\"",
                    project_str, claude_bin, settings_str
                );

                let mut child = Command::new("cmd.exe");
                child.arg("/k").arg(&cmd_str);
                child.current_dir(project_dir);

                child.spawn().map_err(|e| format!("启动 CMD 终端失败: {}", e))?;

                Ok(LaunchResult {
                    success: true,
                    message: "已在 CMD 终端中启动 Claude Code".to_string(),
                    terminal_used: "CMD".to_string(),
                    command: cmd_str,
                })
            }
            _ => {
                // Default fallback to powershell
                let ps_script = format!(
                    "Set-Location -LiteralPath '{}'; & '{}' --settings '{}'",
                    project_str.replace('\'', "''"),
                    claude_bin.replace('\'', "''"),
                    settings_str.replace('\'', "''")
                );

                let mut child = Command::new("powershell.exe");
                child.arg("-NoExit").arg("-Command").arg(&ps_script);
                child.current_dir(project_dir);
                child.spawn().map_err(|e| format!("启动终端失败: {}", e))?;

                Ok(LaunchResult {
                    success: true,
                    message: "已启动 Claude Code".to_string(),
                    terminal_used: "PowerShell".to_string(),
                    command: ps_script,
                })
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let term = if terminal_choice == "iterm2" { "iterm2" } else { "terminal" };
        if term == "iterm2" {
            let applescript = format!(
                "tell application \"iTerm\"
                    activate
                    tell current window
                        create tab with default profile
                        tell current session
                            write text \"cd {} && '{}' --settings '{}'\"
                        end tell
                    end tell
                end tell",
                shell_escape::escape(project_str.into()),
                claude_bin,
                settings_str
            );
            Command::new("osascript").arg("-e").arg(&applescript).spawn()
                .map_err(|e| format!("在 iTerm2 中启动失败: {}", e))?;
        } else {
            let applescript = format!(
                "tell application \"Terminal\"
                    activate
                    do script \"cd {} && '{}' --settings '{}'\"
                end tell",
                shell_escape::escape(project_str.into()),
                claude_bin,
                settings_str
            );
            Command::new("osascript").arg("-e").arg(&applescript).spawn()
                .map_err(|e| format!("在系统终端中启动失败: {}", e))?;
        }

        Ok(LaunchResult {
            success: true,
            message: "已在 macOS 终端中启动 Claude Code".to_string(),
            terminal_used: term.to_string(),
            command: format!("cd \"{}\" && \"{}\" --settings \"{}\"", project_str, claude_bin, settings_str),
        })
    }

    #[cfg(not(any(windows, target_os = "macos")))]
    {
        // Linux / Unix fallback
        let mut child = Command::new("x-terminal-emulator");
        child.arg("-e").arg(format!("cd \"{}\" && \"{}\" --settings \"{}\"", project_str, claude_bin, settings_str));
        child.spawn().map_err(|e| format!("启动终端失败: {}", e))?;

        Ok(LaunchResult {
            success: true,
            message: "已启动终端会话".to_string(),
            terminal_used: "Linux Terminal".to_string(),
            command: format!("\"{}\" --settings \"{}\"", claude_bin, settings_str),
        })
    }
}
