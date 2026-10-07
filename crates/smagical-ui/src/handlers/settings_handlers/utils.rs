//! 设置中心辅助函数集 (对话框、字体扫描、限速与 SSH 配置解析)

use std::path::{Path, PathBuf};

/// 解析网络传输限速字符串为数值与单位的二元组 `(数值文本, 单位文本)`
///
/// # 支持格式
/// - `"unlimited"` / `"off"` / `"0"` / 空串 -> `("0", "off")`；
/// - 带 `"mb"` / `"m"` / `"mb/s"` 结尾 -> `("5", "MB/s")`；
/// - 带 `"kb"` / `"k"` / `"kb/s"` 结尾 -> `("500", "KB/s")`；
/// - 纯数字 -> 默认为 `("5", "MB/s")`。
///
/// # 参数
/// - `limit_str`: 原始输入限速配置字符串。
///
/// # 返回值
/// 返回 `(速度数值, 计量单位)`。
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

/// 将标准 6 位十六进制颜色格式字符串解析为 Slint `Color`
///
/// # 参数
/// - `hex`: 形如 `"#EF4444"` 或 `"EF4444"` 的颜色字符串。
///
/// # 返回值
/// 解析成功返回对应 ARGB 颜色（Alpha=255），失败时安全回退至红色 `#EF4444`。
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

/// 探测操作系统已安装字体，并合并预置高品质跨平台编程字体
///
/// # 探测与匹配策略
/// 1. 预置业界主流等宽与无衬线字体（JetBrains Mono, Fira Code, Cascadia Code, Inter, 微软雅黑等）；
/// 2. Windows 下自动读取 `%WINDIR%\Fonts` 目录，匹配安装的字体文件名（如 `msyh.ttc`, `cascadia.ttf` 等）；
/// 3. 执行集合去重，优先保留常用字体在前。
///
/// # 返回值
/// 包含所有检测到的可用字体族名称集合 `Vec<String>`。
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



/// 获取系统默认备份导出存放目录
///
/// # 查找优先级
/// 1. 当前用户 `Downloads` 下载目录；
/// 2. 用户家目录下的隐式数据目录 `~/.smalux-ssh/backups`；
/// 3. 当前运行工作目录相对路径 `backups/`。
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

/// 调起 Windows 原生“另存为”文件对话框 (SaveFileDialog)
///
/// # 跨平台与安全处理
/// - Windows 下添加 `CREATE_NO_WINDOW (0x08000000)` 隐藏控制台弹窗；
/// - 指定 `-STA` 单线程单元模型，保障 COM UI 组件兼容性。
///
/// # 参数
/// - `filter`: 文件类型过滤规则串（如 `"ZIP (*.zip)|*.zip"`）；
/// - `default_filename`: 预填默认文件名（如 `smalux_backup_2026-09-25.zip`）。
///
/// # 返回值
/// - `Some(PathBuf)`: 用户确认选择的完整保存文件绝对路径；
/// - `None`: 用户取消对话框。
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

/// 调起 Windows 原生“打开文件”对话框 (OpenFileDialog)
///
/// # 参数
/// - `filter`: 文件扩展名过滤条件（如 `"Zip Archives (*.zip)|*.zip"`）。
///
/// # 返回值
/// - `Some(PathBuf)`: 用户成功选中的目标还原文件绝对路径；
/// - `None`: 用户取消或文件不存在。
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
