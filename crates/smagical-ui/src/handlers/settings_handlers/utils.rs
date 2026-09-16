//! 设置中心辅助函数集 (对话框、字体扫描、限速与 SSH 配置解析)

use std::path::{Path, PathBuf};
use smagical_core::domain::host::{HostRecord, HostStatus};

/// 解析限速字符串为 (数值, 单位)
pub(crate) fn parse_speed_limit(limit_str: &str) -> (String, String) {
    let s = limit_str.trim().to_lowercase();
    if s == "unlimited" || s == "off" || s == "0" || s.is_empty() {
        return ("0".to_string(), "off".to_string());
    }
    if s.ends_with("mb") || s.ends_with("m") {
        let num = limit_str.trim_end_matches(|c: char| c.is_alphabetic() || c == '/').trim();
        (num.to_string(), "MB/s".to_string())
    } else if s.ends_with("kb") || s.ends_with("k") {
        let num = limit_str.trim_end_matches(|c: char| c.is_alphabetic() || c == '/').trim();
        (num.to_string(), "KB/s".to_string())
    } else {
        (limit_str.to_string(), "MB/s".to_string())
    }
}

/// 将 HEX 颜色格式解析为 Slint Color
pub(crate) fn hex_to_slint_color(hex: &str) -> slint::Color {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&hex[0..2], 16),
            u8::from_str_radix(&hex[2..4], 16),
            u8::from_str_radix(&hex[4..6], 16),
        ) {
            return slint::Color::from_argb_u8(255, r, g, b);
        }
    }
    slint::Color::from_argb_u8(255, 239, 68, 68)
}

/// 发现系统已安装字体与内置高品质设计字体族
pub fn detect_system_and_builtin_fonts() -> Vec<String> {
    let mut fonts = vec![
        "系统默认 (System Default)".to_string(),
        "Microsoft YaHei UI".to_string(),
        "Segoe UI".to_string(),
        "PingFang SC".to_string(),
        "Inter".to_string(),
        "JetBrains Mono".to_string(),
        "Fira Code".to_string(),
        "Cascadia Code".to_string(),
        "Consolas".to_string(),
        "Roboto".to_string(),
        "Source Han Sans CN".to_string(),
        "Courier New".to_string(),
    ];

    #[cfg(windows)]
    {
        // 尝试从 Windows 字体目录扫描常见优质已安装字体
        if let Ok(windir) = std::env::var("WINDIR") {
            let fonts_dir = Path::new(&windir).join("Fonts");
            if let Ok(entries) = std::fs::read_dir(fonts_dir) {
                let mut sys_detected = Vec::new();
                for entry in entries.flatten() {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    let name_lower = file_name.to_lowercase();
                    if name_lower.ends_with(".ttf") || name_lower.ends_with(".otf") || name_lower.ends_with(".ttc") {
                        let stem = name_lower.trim_end_matches(".ttf").trim_end_matches(".otf").trim_end_matches(".ttc");
                        let matched = match stem {
                            s if s.starts_with("msyh") => Some("Microsoft YaHei"),
                            s if s.starts_with("segoeui") => Some("Segoe UI"),
                            s if s.starts_with("arial") => Some("Arial"),
                            s if s.starts_with("calibri") => Some("Calibri"),
                            s if s.starts_with("consola") => Some("Consolas"),
                            s if s.starts_with("cascadia") => Some("Cascadia Code"),
                            s if s.starts_with("simsun") => Some("SimSun"),
                            s if s.starts_with("simhei") => Some("SimHei"),
                            s if s.starts_with("deng") => Some("DengXian"),
                            _ => None,
                        };
                        if let Some(font_name) = matched {
                            if !sys_detected.contains(&font_name.to_string()) {
                                sys_detected.push(font_name.to_string());
                            }
                        }
                    }
                }
                for f in sys_detected {
                    if !fonts.contains(&f) {
                        fonts.push(f);
                    }
                }
            }
        }
    }

    fonts.dedup();
    fonts
}

/// 解析 OpenSSH 配置文件内容为标准 HostRecord 资产列表
pub(crate) fn parse_ssh_config(content: &str) -> Vec<HostRecord> {
    let mut hosts = Vec::new();
    let mut current_host: Option<HostRecord> = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let mut parts = trimmed.split_whitespace();
        let key = parts.next().unwrap_or("").to_lowercase();
        let val = parts.next().unwrap_or("");

        if key == "host" {
            // 遇到新的 Host 条目，归档前一个
            if let Some(h) = current_host.take() {
                if !h.name.contains('*') && !h.name.is_empty() {
                    hosts.push(h);
                }
            }

            // 过滤通配符
            if !val.contains('*') && !val.is_empty() {
                let host_id = format!("ssh-{}", val.replace(|c: char| !c.is_alphanumeric(), "-"));
                current_host = Some(HostRecord {
                    id: host_id,
                    name: val.to_string(),
                    address: val.to_string(), // 初始回退为 Host 别名
                    port: 22,
                    parent_group_id: None,
                    credential_id: None,
                    status: HostStatus::Offline,
                    ping_ms: 0,
                    sort_order: 100,
                    notes: "Imported from ~/.ssh/config".to_string(),
                    ..Default::default()
                });
            }
        } else if let Some(ref mut h) = current_host {
            match key.as_str() {
                "hostname" => {
                    if !val.is_empty() {
                        h.address = val.to_string();
                    }
                }
                "port" => {
                    if let Ok(p) = val.parse::<u16>() {
                        h.port = p;
                    }
                }
                "user" => {
                    if !val.is_empty() {
                        h.notes = format!("User: {}; {}", val, h.notes);
                    }
                }
                "identityfile" => {
                    if !val.is_empty() {
                        h.notes = format!("Key: {}; {}", val, h.notes);
                    }
                }
                _ => {}
            }
        }
    }

    if let Some(h) = current_host {
        if !h.name.contains('*') && !h.name.is_empty() {
            hosts.push(h);
        }
    }

    hosts
}

/// 获取当前系统的 ~/.ssh/config 路径
pub(crate) fn get_ssh_config_path() -> PathBuf {
    #[cfg(windows)]
    {
        if let Ok(profile) = std::env::var("USERPROFILE") {
            return Path::new(&profile).join(".ssh").join("config");
        }
    }
    #[cfg(not(windows))]
    {
        if let Ok(home) = std::env::var("HOME") {
            return Path::new(&home).join(".ssh").join("config");
        }
    }
    PathBuf::from(".ssh/config")
}

/// 获取默认备份导出路径
#[allow(dead_code)]
pub(crate) fn get_default_backup_dir() -> PathBuf {
    #[cfg(windows)]
    {
        if let Ok(profile) = std::env::var("USERPROFILE") {
            let downloads = Path::new(&profile).join("Downloads");
            if downloads.exists() {
                return downloads;
            }
            return Path::new(&profile).join(".smalux-ssh").join("backups");
        }
    }
    #[cfg(not(windows))]
    {
        if let Ok(home) = std::env::var("HOME") {
            let downloads = Path::new(&home).join("Downloads");
            if downloads.exists() {
                return downloads;
            }
            return Path::new(&home).join(".smalux-ssh").join("backups");
        }
    }
    PathBuf::from("backups")
}

/// 打开 Windows 原生另存为文件对话框
pub(crate) fn pick_save_file(filter: &str, default_filename: &str) -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let script = format!(
            r#"
Add-Type -AssemblyName System.Windows.Forms
$dialog = New-Object System.Windows.Forms.SaveFileDialog
$dialog.Filter = "{}"
$dialog.FileName = "{}"
$dialog.Title = "选择备份保存路径"
if ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {{
    [Console]::Out.Write($dialog.FileName)
}}
"#,
            filter, default_filename
        );
        let mut cmd = std::process::Command::new("powershell");
        cmd.args(["-STA", "-NoProfile", "-NonInteractive", "-Command", &script]);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        let output = cmd.output().ok()?;
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(PathBuf::from(path));
            }
        }
    }
    None
}

/// 打开 Windows 原生打开文件对话框
pub(crate) fn pick_open_file(filter: &str) -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let script = format!(
            r#"
Add-Type -AssemblyName System.Windows.Forms
$dialog = New-Object System.Windows.Forms.OpenFileDialog
$dialog.Filter = "{}"
$dialog.Title = "选择备份还原文件"
if ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {{
    [Console]::Out.Write($dialog.FileName)
}}
"#,
            filter
        );
        let mut cmd = std::process::Command::new("powershell");
        cmd.args(["-STA", "-NoProfile", "-NonInteractive", "-Command", &script]);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        let output = cmd.output().ok()?;
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                let p = PathBuf::from(path);
                if p.is_file() {
                    return Some(p);
                }
            }
        }
    }
    None
}
