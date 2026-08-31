//! 本地终端环境动态探测模块 (Local Shells Probing)
//!
//! 跨平台智能检测当前系统 (Windows / Linux / macOS) 实际已安装且可用的 Shell 终端环境。

use crate::LocalShellItemData;
use std::path::Path;

/// 探测当前系统中所有可用且有效的本地 Shell 列表
pub fn detect_local_shells() -> Vec<LocalShellItemData> {
    #[cfg(windows)]
    {
        detect_windows_shells()
    }

    #[cfg(target_os = "linux")]
    {
        detect_linux_shells()
    }

    #[cfg(target_os = "macos")]
    {
        detect_macos_shells()
    }

    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    {
        fallback_shells()
    }
}

/// 检查某个可执行文件是否在环境变量 PATH 中
#[allow(dead_code)]
fn which_in_path(binary: &str) -> bool {
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let full_path = dir.join(binary);
            if full_path.is_file() {
                return true;
            }
        }
    }
    false
}

#[cfg(windows)]
fn detect_windows_shells() -> Vec<LocalShellItemData> {
    let mut shells = Vec::new();

    // 1. PowerShell 7 (Core) - pwsh.exe
    let pwsh_paths = [
        "C:\\Program Files\\PowerShell\\7\\pwsh.exe",
        "C:\\Program Files\\PowerShell\\7-preview\\pwsh.exe",
    ];
    let mut has_pwsh = false;
    for path in &pwsh_paths {
        if Path::new(path).exists() {
            shells.push(LocalShellItemData {
                id: "local-pwsh7".into(),
                title: "PowerShell 7".into(),
                subtitle: format!("PowerShell Core ({})", path).into(),
                tag: "pwsh 7".into(),
            });
            has_pwsh = true;
            break;
        }
    }
    if !has_pwsh && which_in_path("pwsh.exe") {
        shells.push(LocalShellItemData {
            id: "local-pwsh7".into(),
            title: "PowerShell 7".into(),
            subtitle: "PowerShell Core (pwsh.exe)".into(),
            tag: "pwsh 7".into(),
        });
    }

    // 2. Windows PowerShell (System default) - powershell.exe
    let win_ps_path = "C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe";
    if Path::new(win_ps_path).exists() || which_in_path("powershell.exe") {
        shells.push(LocalShellItemData {
            id: "local-powershell".into(),
            title: "PowerShell".into(),
            subtitle: "Windows PowerShell (powershell.exe)".into(),
            tag: "Local".into(),
        });
    }

    // 3. WSL (Windows Subsystem for Linux) - wsl.exe
    let wsl_path = "C:\\Windows\\System32\\wsl.exe";
    if Path::new(wsl_path).exists() {
        shells.push(LocalShellItemData {
            id: "local-wsl".into(),
            title: "WSL (Linux 子系统)".into(),
            subtitle: "WSL2 默认 Linux 发行版环境 (wsl.exe)".into(),
            tag: "WSL2".into(),
        });
    }

    // 4. Command Prompt (cmd.exe)
    let cmd_path = "C:\\Windows\\System32\\cmd.exe";
    if Path::new(cmd_path).exists() || which_in_path("cmd.exe") {
        shells.push(LocalShellItemData {
            id: "local-cmd".into(),
            title: "Command Prompt".into(),
            subtitle: "Windows 命令提示符 (cmd.exe)".into(),
            tag: "CMD".into(),
        });
    }

    // 5. Git Bash (bash.exe)
    let git_bash_paths = [
        "C:\\Program Files\\Git\\bin\\bash.exe",
        "C:\\Program Files (x86)\\Git\\bin\\bash.exe",
    ];
    let mut has_git_bash = false;
    for path in &git_bash_paths {
        if Path::new(path).exists() {
            shells.push(LocalShellItemData {
                id: "local-gitbash".into(),
                title: "Git Bash".into(),
                subtitle: format!("MINGW64 Git Bash ({})", path).into(),
                tag: "Git".into(),
            });
            has_git_bash = true;
            break;
        }
    }
    if !has_git_bash
        && let Ok(local_app_data) = std::env::var("LOCALAPPDATA")
    {
        let user_git = format!("{}\\Programs\\Git\\bin\\bash.exe", local_app_data);
        if Path::new(&user_git).exists() {
            shells.push(LocalShellItemData {
                id: "local-gitbash".into(),
                title: "Git Bash".into(),
                subtitle: format!("MINGW64 Git Bash ({})", user_git).into(),
                tag: "Git".into(),
            });
            has_git_bash = true;
        }
    }

    if !has_git_bash && which_in_path("bash.exe") {
        shells.push(LocalShellItemData {
            id: "local-gitbash".into(),
            title: "Git Bash".into(),
            subtitle: "Git for Windows (bash.exe)".into(),
            tag: "Git".into(),
        });
    }

    // 6. Nushell (nu.exe) if installed
    if which_in_path("nu.exe") {
        shells.push(LocalShellItemData {
            id: "local-nushell".into(),
            title: "Nushell".into(),
            subtitle: "A modern, structured data shell (nu.exe)".into(),
            tag: "Nu".into(),
        });
    }

    if shells.is_empty() {
        fallback_shells()
    } else {
        shells
    }
}

#[cfg(target_os = "linux")]
fn detect_linux_shells() -> Vec<LocalShellItemData> {
    let mut shells = Vec::new();
    let default_shell = std::env::var("SHELL").unwrap_or_default();

    let mut known_shells = Vec::new();
    if let Ok(content) = std::fs::read_to_string("/etc/shells") {
        for line in content.lines() {
            let line = line.trim();
            if !line.is_empty() && !line.starts_with('#') {
                known_shells.push(line.to_string());
            }
        }
    }

    if known_shells.is_empty() {
        for path in &["/bin/bash", "/usr/bin/bash", "/bin/zsh", "/usr/bin/zsh", "/usr/bin/fish", "/bin/sh"] {
            if Path::new(path).exists() {
                known_shells.push(path.to_string());
            }
        }
    }

    let mut added_ids = std::collections::HashSet::new();

    // 默认 shell 置顶
    if !default_shell.is_empty() && Path::new(&default_shell).exists() {
        let (title, sub, tag, id) = parse_unix_shell_info(&default_shell, true);
        if added_ids.insert(id.clone()) {
            shells.push(LocalShellItemData {
                id: id.into(),
                title: title.into(),
                subtitle: sub.into(),
                tag: tag.into(),
            });
        }
    }

    for shell_path in known_shells {
        if Path::new(&shell_path).exists() {
            let (title, sub, tag, id) = parse_unix_shell_info(&shell_path, false);
            if added_ids.insert(id.clone()) {
                shells.push(LocalShellItemData {
                    id: id.into(),
                    title: title.into(),
                    subtitle: sub.into(),
                    tag: tag.into(),
                });
            }
        }
    }

    if Path::new("/usr/bin/nu").exists() || Path::new("/usr/local/bin/nu").exists() {
        if added_ids.insert("local-nushell".to_string()) {
            shells.push(LocalShellItemData {
                id: "local-nushell".into(),
                title: "Nushell".into(),
                subtitle: "Modern Data Shell (/usr/bin/nu)".into(),
                tag: "Nu".into(),
            });
        }
    }

    if shells.is_empty() {
        fallback_shells()
    } else {
        shells
    }
}

#[cfg(target_os = "macos")]
fn detect_macos_shells() -> Vec<LocalShellItemData> {
    let mut shells = Vec::new();
    let default_shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());

    let candidates = [
        ("/bin/zsh", "Zsh", "macOS 默认 Z Shell", "Zsh"),
        ("/bin/bash", "Bash", "GNU Bourne-Again Shell", "Bash"),
        ("/opt/homebrew/bin/fish", "Fish", "Friendly Interactive Shell (Homebrew)", "Fish"),
        ("/usr/local/bin/fish", "Fish", "Friendly Interactive Shell", "Fish"),
        ("/opt/homebrew/bin/zsh", "Homebrew Zsh", "Homebrew 最新 Z Shell", "Zsh"),
        ("/opt/homebrew/bin/bash", "Homebrew Bash", "Homebrew 最新 Bash", "Bash"),
        ("/bin/sh", "Sh", "POSIX Standard Shell", "POSIX"),
    ];

    let mut added_ids = std::collections::HashSet::new();

    if Path::new(&default_shell).exists() {
        let (title, sub, tag, id) = parse_unix_shell_info(&default_shell, true);
        if added_ids.insert(id.clone()) {
            shells.push(LocalShellItemData {
                id: id.into(),
                title: title.into(),
                subtitle: sub.into(),
                tag: tag.into(),
            });
        }
    }

    for (path, name, desc, tag) in &candidates {
        if Path::new(path).exists() {
            let id = format!("local-{}", name.to_lowercase().replace(' ', "-"));
            if added_ids.insert(id.clone()) {
                shells.push(LocalShellItemData {
                    id: id.into(),
                    title: (*name).into(),
                    subtitle: format!("{} ({})", desc, path).into(),
                    tag: (*tag).into(),
                });
            }
        }
    }

    if shells.is_empty() {
        fallback_shells()
    } else {
        shells
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn parse_unix_shell_info(path: &str, is_default: bool) -> (String, String, String, String) {
    let file_name = Path::new(path)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or(path);

    let (title, tag) = match file_name {
        "bash" => ("Bash".to_string(), if is_default { "Default".to_string() } else { "Bash".to_string() }),
        "zsh" => ("Zsh".to_string(), if is_default { "Default".to_string() } else { "Zsh".to_string() }),
        "fish" => ("Fish".to_string(), "Fish".to_string()),
        "sh" => ("Sh".to_string(), "POSIX".to_string()),
        "dash" => ("Dash".to_string(), "POSIX".to_string()),
        "nu" => ("Nushell".to_string(), "Nu".to_string()),
        other => (other.to_string(), "Shell".to_string()),
    };

    let title_display = if is_default {
        format!("{} (默认)", title)
    } else {
        title
    };

    let sub = format!("{} 终端环境 ({})", file_name, path);
    let id = format!("local-{}", file_name);

    (title_display, sub, tag, id)
}

/// 快速预设默认本地 Shell 列表 (0 磁盘 I/O，首帧 0ms 瞬间展示)
pub fn fast_default_shells() -> Vec<LocalShellItemData> {
    fallback_shells()
}

fn fallback_shells() -> Vec<LocalShellItemData> {
    #[cfg(windows)]
    {
        vec![
            LocalShellItemData {
                id: "local-powershell".into(),
                title: "PowerShell".into(),
                subtitle: "Windows PowerShell (powershell.exe)".into(),
                tag: "Local".into(),
            },
            LocalShellItemData {
                id: "local-cmd".into(),
                title: "Command Prompt".into(),
                subtitle: "Windows 命令提示符 (cmd.exe)".into(),
                tag: "CMD".into(),
            },
        ]
    }

    #[cfg(not(windows))]
    {
        vec![
            LocalShellItemData {
                id: "local-bash".into(),
                title: "Bash".into(),
                subtitle: "GNU Bourne-Again Shell (/bin/bash)".into(),
                tag: "Default".into(),
            },
            LocalShellItemData {
                id: "local-sh".into(),
                title: "Sh".into(),
                subtitle: "POSIX Shell (/bin/sh)".into(),
                tag: "POSIX".into(),
            },
        ]
    }
}

/// 本地终端异步预热与全量环境后台探测 Hook (实现 `AppGlobalHook`)
pub struct LocalShellDiscoveryHook {
    cached_shells: std::sync::Arc<std::sync::RwLock<Vec<LocalShellItemData>>>,
    window_weak: slint::Weak<crate::generated::AppWindow>,
}

impl LocalShellDiscoveryHook {
    /// 创建一个新的本地终端探测 Hook
    pub fn new(
        cached_shells: std::sync::Arc<std::sync::RwLock<Vec<LocalShellItemData>>>,
        window_weak: slint::Weak<crate::generated::AppWindow>,
    ) -> Self {
        Self {
            cached_shells,
            window_weak,
        }
    }
}

impl smagical_core::AppGlobalHook for LocalShellDiscoveryHook {
    fn name(&self) -> &'static str {
        "local_shell_discovery_hook"
    }

    fn priority(&self) -> i32 {
        50
    }

    /// 应用引导启动时：在后台工作线程中异步探测所有本地终端，0 毫秒阻塞主 UI 线程
    fn on_app_boot(&self, _ctx: &smagical_core::AppBootContext) {
        let cached_shells = std::sync::Arc::clone(&self.cached_shells);
        let window_weak = self.window_weak.clone();

        std::thread::Builder::new()
            .name("local-shell-detector".into())
            .spawn(move || {
                tracing::info!(target: "smagical_ui::local_shells", "开始在后台异步探测本地 Shell 终端环境...");
                let detected = detect_local_shells();
                tracing::info!(target: "smagical_ui::local_shells", "后台探测完成，发现 {} 个可用本地终端环境", detected.len());

                // 1. 更新内存共享缓存
                if let Ok(mut write_guard) = cached_shells.write() {
                    *write_guard = detected.clone();
                }

                // 2. 异步回推到 UI 事件循环，就地更新 Slint 启动器数据模型
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = window_weak.upgrade() {
                        w.set_launcher_local_items(slint::ModelRc::from(std::rc::Rc::new(
                            slint::VecModel::from(detected),
                        )));
                    }
                });
            })
            .ok();
    }
}

