//! 本地终端环境动态探测与命令构建模块 (Local Shells Probing & Command Resolution)
//!
//! 跨平台智能检测当前系统 (Windows / Linux / macOS) 实际已安装且可交互的 Shell 终端环境。
//! 遵循“不存在则绝对不展示”原则，彻底杜绝调用缺失环境（如无 Linux 发行版的 WSL 或错误的 bash.exe 跳板）。

use std::path::PathBuf;
#[cfg(not(windows))]
use std::path::Path;
use std::sync::RwLock;
use slint::ComponentHandle;
use crate::LocalShellItemData;

/// 本地终端配置描述体 (包含完整的二进制路径与启动参数)
#[derive(Debug, Clone)]
pub struct LocalShellProfile {
    /// 唯一标识 ID (如 "local-gitbash")
    pub id: String,
    /// 显示标题 (如 "Git Bash")
    pub title: String,
    /// 辅助说明副标题
    pub subtitle: String,
    /// 分类标签 (如 "Git")
    pub tag: String,
    /// 物理可执行文件绝对路径
    pub executable_path: PathBuf,
    /// 启动附带参数列表 (如 ["--login", "-i"])
    pub args: Vec<String>,
    /// 自定义环境变量注入
    pub envs: Vec<(String, String)>,
    /// 启动时初始工作目录
    pub cwd: Option<PathBuf>,
}

/// 运行时已发现的可用 Shell 终端全局注册表
static DISCOVERED_SHELLS: RwLock<Vec<LocalShellProfile>> = RwLock::new(Vec::new());

/// 根据终端标识获取已探测到的 Shell 配置
pub fn get_shell_profile(shell_id: &str) -> Option<LocalShellProfile> {
    if let Ok(guard) = DISCOVERED_SHELLS.read() {
        for profile in guard.iter() {
            if profile.id == shell_id {
                return Some(profile.clone());
            }
        }
    }
    None
}

/// 根据本地 Shell 标识构建 PTY 命令行启动器配置 (直接使用探测到的绝对路径与参数)
pub fn resolve_command_for_shell(shell_id: &str) -> portable_pty::CommandBuilder {
    // 1. 优先查阅已发现的 Profile 缓存
    if let Some(profile) = get_shell_profile(shell_id) {
        let mut cmd = portable_pty::CommandBuilder::new(profile.executable_path);
        for arg in &profile.args {
            cmd.arg(arg);
        }
        for (k, v) in &profile.envs {
            cmd.env(k, v);
        }
        if let Some(cwd) = &profile.cwd {
            cmd.cwd(cwd);
        }
        return cmd;
    }

    // 2. 缓存未命中时进行安全动态解析兜底
    fallback_resolve_command(shell_id)
}

/// 探测当前系统中所有可用且有效的本地 Shell 列表 (不存在的绝不返回)
pub fn detect_local_shells() -> Vec<LocalShellItemData> {
    #[cfg(windows)]
    let (profiles, items) = detect_windows_shells();

    #[cfg(target_os = "linux")]
    let (profiles, items) = detect_linux_shells();

    #[cfg(target_os = "macos")]
    let (profiles, items) = detect_macos_shells();

    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    let (profiles, items) = fallback_detect();

    // 更新全局 Profile 注册表
    if let Ok(mut guard) = DISCOVERED_SHELLS.write() {
        *guard = profiles;
    }

    items
}

/// 快速预设默认本地 Shell 列表 (执行毫秒级安全检查，严格只显示存在的终端)
pub fn fast_default_shells() -> Vec<LocalShellItemData> {
    detect_local_shells()
}

/// 检查某个可执行文件是否在环境变量 PATH 中，若存在则返回其完整 PathBuf
fn which_find_in_path(binary: &str) -> Option<PathBuf> {
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let full_path = dir.join(binary);
            if full_path.is_file() {
                return Some(full_path);
            }
        }
    }
    None
}

#[cfg(windows)]
mod win_reg {
    use std::path::PathBuf;
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegEnumKeyExW, RegOpenKeyExW, RegQueryValueExW,
        HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, REG_SZ, REG_EXPAND_SZ,
    };

    fn to_wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// 读取注册表指定路径下的字符串键值 (支持 REG_SZ 与 REG_EXPAND_SZ)
    pub fn read_string_value(root: HKEY, subkey: &str, value_name: &str) -> Option<String> {
        unsafe {
            let mut hkey: HKEY = std::mem::zeroed();
            let subkey_w = to_wide(subkey);
            if RegOpenKeyExW(root, subkey_w.as_ptr(), 0, KEY_READ, &mut hkey) != 0 {
                return None;
            }

            let value_w = to_wide(value_name);
            let mut val_type: u32 = 0;
            let mut data_len: u32 = 0;

            // 先获取所需字节长度
            if RegQueryValueExW(
                hkey,
                value_w.as_ptr(),
                std::ptr::null_mut(),
                &mut val_type,
                std::ptr::null_mut(),
                &mut data_len,
            ) != 0 || data_len == 0 {
                RegCloseKey(hkey);
                return None;
            }

            if val_type != REG_SZ && val_type != REG_EXPAND_SZ {
                RegCloseKey(hkey);
                return None;
            }

            let mut buffer = vec![0u16; (data_len as usize / 2) + 1];
            let mut actual_len = data_len;
            let ret = RegQueryValueExW(
                hkey,
                value_w.as_ptr(),
                std::ptr::null_mut(),
                &mut val_type,
                buffer.as_mut_ptr() as *mut u8,
                &mut actual_len,
            );

            RegCloseKey(hkey);

            if ret == 0 {
                let s = String::from_utf16_lossy(&buffer);
                let trimmed = s.trim_matches('\0').trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
            None
        }
    }

    /// 枚举指定注册表键下的所有直接子项名称
    pub fn enum_subkeys(root: HKEY, subkey: &str) -> Vec<String> {
        let mut subkeys = Vec::new();
        unsafe {
            let mut hkey: HKEY = std::mem::zeroed();
            let subkey_w = to_wide(subkey);
            if RegOpenKeyExW(root, subkey_w.as_ptr(), 0, KEY_READ, &mut hkey) != 0 {
                return subkeys;
            }

            let mut index = 0u32;
            let mut name_buf = vec![0u16; 256];
            loop {
                let mut name_len = name_buf.len() as u32;
                let ret = RegEnumKeyExW(
                    hkey,
                    index,
                    name_buf.as_mut_ptr(),
                    &mut name_len,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
                if ret != 0 {
                    break;
                }
                let key_name = String::from_utf16_lossy(&name_buf[..name_len as usize]);
                subkeys.push(key_name);
                index += 1;
            }

            RegCloseKey(hkey);
        }
        subkeys
    }

    /// 查询 Git for Windows 官方注册表安装根路径
    pub fn get_git_install_path() -> Option<PathBuf> {
        // 1. HKLM\SOFTWARE\GitForWindows
        if let Some(p) = read_string_value(HKEY_LOCAL_MACHINE, "SOFTWARE\\GitForWindows", "InstallPath") {
            let path = PathBuf::from(p);
            if path.is_dir() {
                return Some(path);
            }
        }
        // 2. HKCU\SOFTWARE\GitForWindows
        if let Some(p) = read_string_value(HKEY_CURRENT_USER, "SOFTWARE\\GitForWindows", "InstallPath") {
            let path = PathBuf::from(p);
            if path.is_dir() {
                return Some(path);
            }
        }
        // 3. WOW6432Node
        if let Some(p) = read_string_value(HKEY_LOCAL_MACHINE, "SOFTWARE\\WOW6432Node\\GitForWindows", "InstallPath") {
            let path = PathBuf::from(p);
            if path.is_dir() {
                return Some(path);
            }
        }
        None
    }

    /// 检查并枚举系统是否安装了真实、可交互的 Linux 发行版
    /// 自动排除 Docker Desktop / Rancher 等仅用于容器后台支撑的专用无 bash 发行版
    pub fn get_valid_wsl_distros() -> Vec<String> {
        let lxss_path = "Software\\Microsoft\\Windows\\CurrentVersion\\Lxss";
        let subkeys = enum_subkeys(HKEY_CURRENT_USER, lxss_path);
        let mut distros = Vec::new();

        for key in subkeys {
            let full_subkey = format!("{}\\{}", lxss_path, key);
            if let Some(distro_name) = read_string_value(HKEY_CURRENT_USER, &full_subkey, "DistributionName") {
                let lower = distro_name.to_lowercase();
                if !lower.starts_with("docker-desktop") && !lower.starts_with("rancher-desktop") {
                    distros.push(distro_name);
                }
            }
        }
        distros
    }
}

#[cfg(windows)]
fn find_git_bash() -> Option<PathBuf> {
    // 策略 1: 从环境变量 PATH 中的 git.exe 逆向探测安装根目录 (零开销，精准适配任意自定义盘符，如 D:\git)
    if let Some(git_exe) = which_find_in_path("git.exe") {
        if let Some(parent) = git_exe.parent() {
            let candidate_roots = [
                parent.to_path_buf(),
                parent.parent().map(|p| p.to_path_buf()).unwrap_or_default(),
            ];
            for root in &candidate_roots {
                let bash1 = root.join("bin").join("bash.exe");
                if bash1.is_file() {
                    return Some(bash1);
                }
                let bash2 = root.join("usr").join("bin").join("bash.exe");
                if bash2.is_file() {
                    return Some(bash2);
                }
            }
        }
    }

    // 策略 2: 从 Windows 注册表查询 GitForWindows 的 InstallPath
    if let Some(git_root) = win_reg::get_git_install_path() {
        let bash1 = git_root.join("bin").join("bash.exe");
        if bash1.is_file() {
            return Some(bash1);
        }
        let bash2 = git_root.join("usr").join("bin").join("bash.exe");
        if bash2.is_file() {
            return Some(bash2);
        }
    }

    // 策略 3: 扫描标准安装盘符和 AppData 目录
    let mut default_paths = vec![
        PathBuf::from("C:\\Program Files\\Git\\bin\\bash.exe"),
        PathBuf::from("C:\\Program Files (x86)\\Git\\bin\\bash.exe"),
        PathBuf::from("D:\\git\\bin\\bash.exe"),
        PathBuf::from("D:\\Program Files\\Git\\bin\\bash.exe"),
        PathBuf::from("E:\\git\\bin\\bash.exe"),
    ];
    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        default_paths.push(PathBuf::from(local_app_data).join("Programs\\Git\\bin\\bash.exe"));
    }
    if let Ok(prog_files) = std::env::var("ProgramFiles") {
        default_paths.push(PathBuf::from(prog_files).join("Git\\bin\\bash.exe"));
    }
    if let Ok(prog_w64) = std::env::var("ProgramW6432") {
        default_paths.push(PathBuf::from(prog_w64).join("Git\\bin\\bash.exe"));
    }

    for p in default_paths {
        if p.is_file() {
            return Some(p);
        }
    }

    // 策略 4: PATH 中查找 bash.exe (黑名单排除 Windows 系统 System32 / SysWOW64，绝不误调 WSL relay)
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let dir_str = dir.to_string_lossy().to_lowercase();
            if dir_str.contains("system32") || dir_str.contains("syswow64") {
                continue;
            }
            let candidate = dir.join("bash.exe");
            if candidate.is_file() {
                // 确保同目录或邻近目录有 git 相关组件或 MSYS 核心
                if dir.join("git.exe").is_file()
                    || dir.parent().map(|p| p.join("cmd\\git.exe").is_file()).unwrap_or(false)
                    || dir.join("msys-2.0.dll").is_file()
                {
                    return Some(candidate);
                }
            }
        }
    }

    None
}

#[cfg(windows)]
fn find_pwsh7() -> Option<PathBuf> {
    let candidates = [
        "C:\\Program Files\\PowerShell\\7\\pwsh.exe",
        "C:\\Program Files\\PowerShell\\7-preview\\pwsh.exe",
    ];
    for p in &candidates {
        let path = PathBuf::from(p);
        if path.is_file() {
            return Some(path);
        }
    }
    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        let user_pwsh = PathBuf::from(local_app_data).join("Microsoft\\PowerShell\\7\\pwsh.exe");
        if user_pwsh.is_file() {
            return Some(user_pwsh);
        }
    }
    which_find_in_path("pwsh.exe")
}

#[cfg(windows)]
fn find_windows_powershell() -> Option<PathBuf> {
    if let Ok(sys_root) = std::env::var("SystemRoot") {
        let p = PathBuf::from(sys_root).join("System32\\WindowsPowerShell\\v1.0\\powershell.exe");
        if p.is_file() {
            return Some(p);
        }
    }
    let p = PathBuf::from("C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe");
    if p.is_file() {
        return Some(p);
    }
    which_find_in_path("powershell.exe")
}

#[cfg(windows)]
fn find_cmd() -> Option<PathBuf> {
    if let Ok(comspec) = std::env::var("ComSpec") {
        let p = PathBuf::from(comspec);
        if p.is_file() {
            return Some(p);
        }
    }
    if let Ok(sys_root) = std::env::var("SystemRoot") {
        let p = PathBuf::from(sys_root).join("System32\\cmd.exe");
        if p.is_file() {
            return Some(p);
        }
    }
    let p = PathBuf::from("C:\\Windows\\System32\\cmd.exe");
    if p.is_file() {
        return Some(p);
    }
    which_find_in_path("cmd.exe")
}

#[cfg(windows)]
fn find_wsl() -> Option<(PathBuf, Vec<String>)> {
    // 1. 检查 wsl.exe 是否存在
    let wsl_exe = if let Ok(sys_root) = std::env::var("SystemRoot") {
        let p = PathBuf::from(sys_root).join("System32\\wsl.exe");
        if p.is_file() {
            Some(p)
        } else {
            None
        }
    } else {
        None
    }.or_else(|| {
        let p = PathBuf::from("C:\\Windows\\System32\\wsl.exe");
        if p.is_file() {
            Some(p)
        } else {
            which_find_in_path("wsl.exe")
        }
    })?;

    // 2. 检查注册表是否有可交互的常规 Linux 发行版 (无有效分发版则绝不显示 WSL)
    let valid_distros = win_reg::get_valid_wsl_distros();
    if valid_distros.is_empty() {
        return None;
    }

    Some((wsl_exe, valid_distros))
}

#[cfg(windows)]
fn find_nushell() -> Option<PathBuf> {
    which_find_in_path("nu.exe")
}

#[cfg(windows)]
fn find_msys2() -> Option<PathBuf> {
    let p = PathBuf::from("C:\\msys64\\usr\\bin\\bash.exe");
    if p.is_file() {
        Some(p)
    } else {
        None
    }
}

#[cfg(windows)]
fn detect_windows_shells() -> (Vec<LocalShellProfile>, Vec<LocalShellItemData>) {
    let mut profiles = Vec::new();

    // 1. PowerShell 7 (Core) - 若存在则置顶
    if let Some(pwsh_path) = find_pwsh7() {
        profiles.push(LocalShellProfile {
            id: "local-pwsh7".into(),
            title: "PowerShell 7".into(),
            subtitle: format!("PowerShell Core ({})", pwsh_path.display()),
            tag: "pwsh 7".into(),
            executable_path: pwsh_path,
            args: vec!["-NoLogo".into()],
            envs: Vec::new(),
            cwd: None,
        });
    }

    // 2. Windows PowerShell (System default)
    if let Some(ps_path) = find_windows_powershell() {
        profiles.push(LocalShellProfile {
            id: "local-powershell".into(),
            title: "PowerShell".into(),
            subtitle: format!("Windows PowerShell ({})", ps_path.display()),
            tag: "Local".into(),
            executable_path: ps_path,
            args: vec!["-NoLogo".into()],
            envs: Vec::new(),
            cwd: None,
        });
    }

    // 3. Command Prompt (cmd.exe)
    if let Some(cmd_path) = find_cmd() {
        profiles.push(LocalShellProfile {
            id: "local-cmd".into(),
            title: "Command Prompt".into(),
            subtitle: format!("Windows 命令提示符 ({})", cmd_path.display()),
            tag: "CMD".into(),
            executable_path: cmd_path,
            args: Vec::new(),
            envs: Vec::new(),
            cwd: None,
        });
    }

    // 4. Git Bash (bash.exe) - 全面修复自定义安装盘符与参数支持
    if let Some(git_bash_path) = find_git_bash() {
        profiles.push(LocalShellProfile {
            id: "local-gitbash".into(),
            title: "Git Bash".into(),
            subtitle: format!("MINGW64 Git Bash ({})", git_bash_path.display()),
            tag: "Git".into(),
            executable_path: git_bash_path,
            args: vec!["--login".into(), "-i".into()],
            envs: Vec::new(),
            cwd: None,
        });
    }

    // 5. WSL (Windows Subsystem for Linux) - 严格校验是否有实际 Linux 发行版
    if let Some((wsl_path, distros)) = find_wsl() {
        let sub = if distros.len() == 1 {
            format!("WSL2 Linux 环境: {} (wsl.exe)", distros[0])
        } else {
            format!("WSL2 Linux 发行版: {} (wsl.exe)", distros.join(", "))
        };
        profiles.push(LocalShellProfile {
            id: "local-wsl".into(),
            title: "WSL (Linux 子系统)".into(),
            subtitle: sub,
            tag: "WSL2".into(),
            executable_path: wsl_path,
            args: Vec::new(),
            envs: Vec::new(),
            cwd: None,
        });
    }

    // 6. Nushell (nu.exe)
    if let Some(nu_path) = find_nushell() {
        profiles.push(LocalShellProfile {
            id: "local-nushell".into(),
            title: "Nushell".into(),
            subtitle: format!("Modern structured data shell ({})", nu_path.display()),
            tag: "Nu".into(),
            executable_path: nu_path,
            args: Vec::new(),
            envs: Vec::new(),
            cwd: None,
        });
    }

    // 7. MSYS2 (bash.exe)
    if let Some(msys_path) = find_msys2() {
        profiles.push(LocalShellProfile {
            id: "local-msys2".into(),
            title: "MSYS2 Bash".into(),
            subtitle: format!("MSYS2 64-bit Bash ({})", msys_path.display()),
            tag: "MSYS2".into(),
            executable_path: msys_path,
            args: vec!["--login".into(), "-i".into()],
            envs: Vec::new(),
            cwd: None,
        });
    }

    let items = profiles
        .iter()
        .map(|p| LocalShellItemData {
            id: p.id.clone().into(),
            title: p.title.clone().into(),
            subtitle: p.subtitle.clone().into(),
            tag: p.tag.clone().into(),
        })
        .collect();

    (profiles, items)
}

#[cfg(target_os = "linux")]
fn detect_linux_shells() -> (Vec<LocalShellProfile>, Vec<LocalShellItemData>) {
    let mut profiles = Vec::new();
    let default_shell = std::env::var("SHELL").unwrap_or_default();
    let mut added_paths = std::collections::HashSet::new();

    let mut candidate_paths = Vec::new();

    // 1. 默认 shell 置顶
    if !default_shell.is_empty() && Path::new(&default_shell).is_file() {
        candidate_paths.push((default_shell.clone(), true));
    }

    // 2. 从 /etc/shells 读取
    if let Ok(content) = std::fs::read_to_string("/etc/shells") {
        for line in content.lines() {
            let line = line.trim();
            if !line.is_empty() && !line.starts_with('#') {
                if Path::new(line).is_file() {
                    candidate_paths.push((line.to_string(), false));
                }
            }
        }
    }

    // 3. 常见候选路径
    for p in &["/bin/bash", "/usr/bin/bash", "/bin/zsh", "/usr/bin/zsh", "/usr/bin/fish", "/bin/sh"] {
        if Path::new(p).is_file() {
            candidate_paths.push((p.to_string(), false));
        }
    }

    // 4. Nushell
    for nu in &["/usr/bin/nu", "/usr/local/bin/nu"] {
        if Path::new(nu).is_file() {
            candidate_paths.push((nu.to_string(), false));
        }
    }
    if let Some(nu_path) = which_find_in_path("nu") {
        candidate_paths.push((nu_path.to_string_lossy().to_string(), false));
    }

    for (path_str, is_default) in candidate_paths {
        let path = PathBuf::from(&path_str);
        if !path.is_file() {
            continue;
        }
        let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
        if !added_paths.insert(canonical) {
            continue;
        }

        let (title, sub, tag, id) = parse_unix_shell_info(&path_str, is_default);
        profiles.push(LocalShellProfile {
            id,
            title,
            subtitle: sub,
            tag,
            executable_path: path,
            args: Vec::new(),
            envs: Vec::new(),
            cwd: None,
        });
    }

    let items = profiles
        .iter()
        .map(|p| LocalShellItemData {
            id: p.id.clone().into(),
            title: p.title.clone().into(),
            subtitle: p.subtitle.clone().into(),
            tag: p.tag.clone().into(),
        })
        .collect();

    (profiles, items)
}

#[cfg(target_os = "macos")]
fn detect_macos_shells() -> (Vec<LocalShellProfile>, Vec<LocalShellItemData>) {
    let mut profiles = Vec::new();
    let default_shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    let mut added_paths = std::collections::HashSet::new();

    let candidates = [
        ("/bin/zsh", "Zsh", "macOS 系统内置 Z Shell", "Zsh"),
        ("/bin/bash", "Bash", "GNU Bourne-Again Shell", "Bash"),
        ("/opt/homebrew/bin/zsh", "Homebrew Zsh", "Homebrew Z Shell (Apple Silicon)", "Zsh"),
        ("/opt/homebrew/bin/bash", "Homebrew Bash", "Homebrew 最新 Bash (Apple Silicon)", "Bash"),
        ("/opt/homebrew/bin/fish", "Fish", "Friendly Interactive Shell (Apple Silicon)", "Fish"),
        ("/usr/local/bin/zsh", "Intel Homebrew Zsh", "Homebrew Z Shell (Intel)", "Zsh"),
        ("/usr/local/bin/bash", "Intel Homebrew Bash", "Homebrew 最新 Bash (Intel)", "Bash"),
        ("/usr/local/bin/fish", "Fish", "Friendly Interactive Shell (Intel)", "Fish"),
        ("/bin/sh", "Sh", "POSIX Standard Shell", "POSIX"),
    ];

    let mut all_to_check = Vec::new();
    if Path::new(&default_shell).is_file() {
        all_to_check.push((default_shell.as_str(), "Default", "macOS 当前登录 Shell", "Default", true));
    }
    for c in &candidates {
        all_to_check.push((c.0, c.1, c.2, c.3, false));
    }

    for (p_str, name, desc, tag, is_def) in all_to_check {
        let p = PathBuf::from(p_str);
        if !p.is_file() {
            continue;
        }
        let canonical = p.canonicalize().unwrap_or_else(|_| p.clone());
        if !added_paths.insert(canonical) {
            continue;
        }

        let file_name = p.file_name().and_then(|s| s.to_str()).unwrap_or("shell");
        let id = if is_def {
            format!("local-default-{}", file_name)
        } else {
            format!("local-{}", name.to_lowercase().replace(' ', "-"))
        };
        let title = if is_def {
            format!("{} (默认)", name)
        } else {
            name.to_string()
        };
        let subtitle = format!("{} ({})", desc, p.display());

        profiles.push(LocalShellProfile {
            id,
            title,
            subtitle,
            tag: tag.to_string(),
            executable_path: p,
            args: vec!["-l".to_string()],
            envs: Vec::new(),
            cwd: None,
        });
    }

    let items = profiles
        .iter()
        .map(|p| LocalShellItemData {
            id: p.id.clone().into(),
            title: p.title.clone().into(),
            subtitle: p.subtitle.clone().into(),
            tag: p.tag.clone().into(),
        })
        .collect();

    (profiles, items)
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

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn fallback_detect() -> (Vec<LocalShellProfile>, Vec<LocalShellItemData>) {
    let p = PathBuf::from("/bin/sh");
    let mut profiles = Vec::new();
    if p.is_file() {
        profiles.push(LocalShellProfile {
            id: "local-sh".into(),
            title: "Sh".into(),
            subtitle: "POSIX Shell (/bin/sh)".into(),
            tag: "POSIX".into(),
            executable_path: p,
            args: Vec::new(),
            envs: Vec::new(),
            cwd: None,
        });
    }
    let items = profiles
        .iter()
        .map(|p| LocalShellItemData {
            id: p.id.clone().into(),
            title: p.title.clone().into(),
            subtitle: p.subtitle.clone().into(),
            tag: p.tag.clone().into(),
        })
        .collect();
    (profiles, items)
}

fn fallback_resolve_command(shell_id: &str) -> portable_pty::CommandBuilder {
    #[cfg(windows)]
    {
        if shell_id == "local-gitbash" {
            if let Some(git_bash) = find_git_bash() {
                let mut cmd = portable_pty::CommandBuilder::new(git_bash);
                cmd.args(["--login", "-i"]);
                return cmd;
            }
        }
        if shell_id == "local-pwsh7" {
            if let Some(pwsh) = find_pwsh7() {
                let mut cmd = portable_pty::CommandBuilder::new(pwsh);
                cmd.arg("-NoLogo");
                return cmd;
            }
        }
        if shell_id == "local-cmd" {
            if let Some(cmd_path) = find_cmd() {
                return portable_pty::CommandBuilder::new(cmd_path);
            }
        }
        if let Some(ps_path) = find_windows_powershell() {
            let mut cmd = portable_pty::CommandBuilder::new(ps_path);
            cmd.arg("-NoLogo");
            return cmd;
        }
        if let Some(cmd_path) = find_cmd() {
            return portable_pty::CommandBuilder::new(cmd_path);
        }
        portable_pty::CommandBuilder::new("cmd.exe")
    }

    #[cfg(not(windows))]
    {
        if let Ok(default_sh) = std::env::var("SHELL") {
            if Path::new(&default_sh).is_file() {
                let mut cmd = portable_pty::CommandBuilder::new(default_sh);
                #[cfg(target_os = "macos")]
                cmd.arg("-l");
                return cmd;
            }
        }
        if Path::new("/bin/bash").is_file() {
            let mut cmd = portable_pty::CommandBuilder::new("/bin/bash");
            #[cfg(target_os = "macos")]
            cmd.arg("-l");
            return cmd;
        }
        let mut cmd = portable_pty::CommandBuilder::new("/bin/sh");
        #[cfg(target_os = "macos")]
        cmd.arg("-l");
        cmd
    }
}

/// 启动本地终端异步探测服务 (在后台异步工作池中探测所有本地终端，0 毫秒阻塞主 UI 线程)
pub fn start_local_shell_discovery(
    cached_shells: std::sync::Arc<std::sync::RwLock<Vec<LocalShellItemData>>>,
    window_weak: slint::Weak<crate::generated::AppWindow>,
) {
    crate::async_util::spawn_async(async move {
        tracing::info!(target: "smagical_ui::local_shells", "开始在后台异步探测本地 Shell 终端环境...");
        let detected = tokio::task::spawn_blocking(detect_local_shells).await.unwrap_or_default();
        tracing::info!(target: "smagical_ui::local_shells", "后台探测完成，发现 {} 个有效本地终端环境", detected.len());

        // 1. 更新内存共享缓存
        if let Ok(mut write_guard) = cached_shells.write() {
            *write_guard = detected.clone();
        }

        // 2. 异步回推到 UI 事件循环，就地更新 Slint 启动器数据模型
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(w) = window_weak.upgrade() {
                w.global::<crate::generated::WindowBridge>().set_launcher_local_items(slint::ModelRc::from(std::rc::Rc::new(
                    slint::VecModel::from(detected),
                )));
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_local_shells_strict_existence() {
        let shells = detect_local_shells();
        assert!(!shells.is_empty(), "当前测试环境应至少探测到一个可用终端 (如 cmd/pwsh/gitbash)");

        for shell in &shells {
            let profile = get_shell_profile(&shell.id);
            assert!(profile.is_some(), "每个 UI 项必须在 DISCOVERED_SHELLS 中有对应的 Profile: {}", shell.id);
            let profile = profile.unwrap();
            assert!(
                profile.executable_path.is_file(),
                "检测到的可执行文件必须真实存在: {:?}",
                profile.executable_path
            );
        }
    }

    #[cfg(windows)]
    #[test]
    fn test_git_bash_detection_and_no_wsl_relay_mistake() {
        let _shells = detect_local_shells();

        // 验证 Git Bash 的解析情况
        if let Some(git_profile) = get_shell_profile("local-gitbash") {
            let path_str = git_profile.executable_path.to_string_lossy().to_lowercase();
            assert!(
                !path_str.contains("system32"),
                "Git Bash 绝对不能解析到 System32 下的 WSL relay bash.exe: {:?}",
                git_profile.executable_path
            );
            assert!(
                git_profile.executable_path.is_file(),
                "Git Bash 真实路径必须有效存在: {:?}",
                git_profile.executable_path
            );
            assert_eq!(
                git_profile.args,
                vec!["--login", "-i"],
                "Git Bash 必须附带 --login -i 交互式启动参数"
            );
        }

        // 验证 WSL：如果只有 docker-desktop，则绝对不应暴露给用户
        let valid_distros = win_reg::get_valid_wsl_distros();
        if valid_distros.is_empty() {
            assert!(
                get_shell_profile("local-wsl").is_none(),
                "系统无有效交互式 Linux 发行版时，绝不可在列表中展示 WSL！"
            );
        }
    }
}

