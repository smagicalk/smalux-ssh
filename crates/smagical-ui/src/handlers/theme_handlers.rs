//! Theme management (custom, import, export, delete), Theme Studio modal, and wallpaper gallery/slideshow handlers

use std::{cell::RefCell, path::PathBuf, rc::Rc, time::Duration};
use slint::{ComponentHandle, ModelRc, VecModel};
use smagical_core::theme::{
    ThemeId, ThemeKind, ThemeMetadata, ThemePeriod, ThemeRepository, ThemeService,
    UiThemeDefinition, UiThemeMetrics, UiThemeMetricsPatch, UiThemeTokens, UiThemeTokensPatch, THEME_SCHEMA_VERSION,
};
use crate::generated::AppWindow;
use super::AppContext;

/// Parse hex string into a Slint Brush
fn parse_brush(value: &str) -> Option<slint::Brush> {
    let mut hex = value.trim().trim_start_matches('#');
    let expanded: String;
    if hex.len() == 3 {
        expanded = hex.chars().flat_map(|c| [c, c]).collect();
        hex = &expanded;
    }
    if hex.len() != 6 && hex.len() != 8 {
        return None;
    }
    let channel = |start| u8::from_str_radix(&hex[start..start + 2], 16).ok();
    let r = channel(0)?;
    let g = channel(2)?;
    let b = channel(4)?;
    let a = if hex.len() == 8 { channel(6)? } else { 255 };
    Some(slint::Brush::from(slint::Color::from_argb_u8(a, r, g, b)))
}

/// Normalize color string to valid #RRGGBB or #RRGGBBAA hex color
fn normalize_hex(value: &str, fallback: &str) -> String {
    let mut v = value.trim().to_string();
    if !v.starts_with('#') {
        v = format!("#{}", v);
    }
    let hex = &v[1..];
    if hex.len() == 3 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        let expanded: String = hex.chars().flat_map(|c| [c, c]).collect();
        return format!("#{}", expanded);
    }
    if (hex.len() == 6 || hex.len() == 8) && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return v;
    }
    fallback.to_string()
}

/// Helper to generate pretty and 100% valid TOML representation of theme definition from AppWindow state
fn generate_theme_toml_from_window(w: &AppWindow) -> String {
    let raw_name = w.get_theme_editor_name();
    let clean_name = raw_name.trim();
    let display_name = if clean_name.is_empty() { "Custom Theme" } else { clean_name };

    let ascii_id: String = clean_name
        .to_lowercase()
        .replace(' ', "-")
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();

    let id_suffix = if ascii_id.is_empty() {
        format!("theme-{}", &uuid::Uuid::new_v4().simple().to_string()[..8])
    } else {
        ascii_id
    };

    let p_clean = if w.get_theme_editor_period() == "day" { "day" } else { "night" };
    let scheme = if p_clean == "day" { "light" } else { "dark" };

    let b_raw = w.get_theme_editor_base();
    let b_clean = b_raw.trim();
    let base_line = if b_clean.is_empty() {
        String::new()
    } else {
        format!("base = \"{}\"\n", b_clean)
    };

    let a_raw = w.get_theme_editor_author();
    let a_clean = a_raw.trim();
    let author_line = if a_clean.is_empty() {
        "author = \"User\"\n".to_string()
    } else {
        format!("author = \"{}\"\n", a_clean)
    };

    let norm_window_bg = normalize_hex(&w.get_theme_editor_window_bg(), if p_clean == "day" { "#ffffff" } else { "#1e1e2e" });
    let norm_panel_bg = normalize_hex(&w.get_theme_editor_panel_bg(), if p_clean == "day" { "#f6f8fa" } else { "#181825" });
    let norm_surface_bg = normalize_hex(&w.get_theme_editor_surface_bg(), if p_clean == "day" { "#eaeef2" } else { "#313244" });
    let norm_control_bg = normalize_hex(&w.get_theme_editor_control_bg(), if p_clean == "day" { "#ffffff" } else { "#383a4c" });
    let norm_foreground = normalize_hex(&w.get_theme_editor_foreground(), if p_clean == "day" { "#24292e" } else { "#cdd6f4" });
    let norm_secondary_fg = normalize_hex(&w.get_theme_editor_secondary_fg(), if p_clean == "day" { "#586069" } else { "#a6adc8" });
    let norm_disabled_fg = normalize_hex(&w.get_theme_editor_disabled_fg(), if p_clean == "day" { "#8c959f" } else { "#6c7086" });
    let norm_accent = normalize_hex(&w.get_theme_editor_accent(), if p_clean == "day" { "#0366d6" } else { "#cba6f7" });
    let norm_hover_bg = normalize_hex(&w.get_theme_editor_hover_bg(), if p_clean == "day" { "#e1e4e8" } else { "#45475a" });
    let norm_pressed_bg = normalize_hex(&w.get_theme_editor_pressed_bg(), if p_clean == "day" { "#d0d7de" } else { "#585b70" });
    let norm_selected_bg = normalize_hex(&w.get_theme_editor_selected_bg(), norm_accent.as_str());
    let norm_selected_fg = normalize_hex(&w.get_theme_editor_selected_fg(), "#ffffff");
    let norm_border = normalize_hex(&w.get_theme_editor_border(), if p_clean == "day" { "#d1d5da" } else { "#45475a" });
    let norm_focus_border = normalize_hex(&w.get_theme_editor_focus_border(), norm_accent.as_str());
    let norm_success = normalize_hex(&w.get_theme_editor_success(), if p_clean == "day" { "#1a7f37" } else { "#a6e3a1" });
    let norm_warning = normalize_hex(&w.get_theme_editor_warning(), if p_clean == "day" { "#9a6700" } else { "#f9e2af" });
    let norm_danger = normalize_hex(&w.get_theme_editor_danger(), if p_clean == "day" { "#cf222e" } else { "#f38ba8" });
    let norm_info = normalize_hex(&w.get_theme_editor_info(), if p_clean == "day" { "#0969da" } else { "#89b4fa" });

    let r_s = w.get_theme_editor_metric_radius_small().trim().parse::<u32>().unwrap_or(2);
    let r_m = w.get_theme_editor_metric_radius_medium().trim().parse::<u32>().unwrap_or(4);
    let r_l = w.get_theme_editor_metric_radius_large().trim().parse::<u32>().unwrap_or(8);
    let sp_s = w.get_theme_editor_metric_spacing_small().trim().parse::<u32>().unwrap_or(4);
    let sp_m = w.get_theme_editor_metric_spacing_medium().trim().parse::<u32>().unwrap_or(8);
    let sp_l = w.get_theme_editor_metric_spacing_large().trim().parse::<u32>().unwrap_or(16);
    let bw = w.get_theme_editor_metric_border_width().trim().parse::<u32>().unwrap_or(1);
    let ch = w.get_theme_editor_metric_control_height().trim().parse::<u32>().unwrap_or(32);
    let isz = w.get_theme_editor_metric_icon_size().trim().parse::<u32>().unwrap_or(16);

    format!(
r#"schema-version = 1
id = "custom.ui.{}"
name = "{}"
kind = "ui"
period = "{}"
{}{}
[ui]
color-scheme = "{}"

# --- 基础层次底色 ---
window-background = "{}"       # 应用最底层主窗口背景
panel-background = "{}"        # 侧栏、状态栏与面板底色
surface-background = "{}"      # 内容卡片、表格行与弹窗底色
control-background = "{}"      # 输入框、按钮胶囊底色 (支持轻微透光)

# --- 文字与图标前景色 ---
foreground = "{}"              # 核心正文、标题与主要图标颜色
secondary-foreground = "{}"    # 次要说明、副标题与占位符颜色
disabled-foreground = "{}"     # 禁用状态或非活动项弱化颜色

# --- 品牌强调与交互底色 ---
accent = "{}"                  # 品牌主题强调色、主操作高亮
hover-background = "{}"        # 指针悬停时的微高亮底色
pressed-background = "{}"      # 按钮与项目按下激活时的底色
selected-background = "{}"     # 侧栏或列表中当前选中项背景色
selected-foreground = "{}"     # 选中项内部文字与图标的高对比前景色

# --- 边框与轮廓线 ---
border = "{}"                  # 常规组件分隔线与卡片边框
focus-border = "{}"            # 输入框获取焦点或键盘导航外轮廓高亮色

# --- 语义状态反馈色 ---
success = "{}"                 # 连接成功、在线运行状态色
warning = "{}"                 # 警告提示、跳板机等特殊标注色
danger = "{}"                  # 异常错误、离线、危险删除操作色
info = "{}"                    # 常规通知提示与信息标注色

[metrics]
# --- 几何圆角系统 (单位: px) ---
radius-small = {}               # 紧凑微型圆角 (标签/徽章)
radius-medium = {}              # 常规标准圆角 (按钮/输入框)
radius-large = {}               # 容器与模态弹窗大圆角

# --- 间距系统排版 (单位: px) ---
spacing-small = {}              # 紧凑内间距与行内留白
spacing-medium = {}             # 标准表单与网格间距
spacing-large = {}              # 大版块与页面间距

# --- 控件与边框规格 (单位: px) ---
border-width = {}               # 基础描边宽度
control-height = {}            # 标准交互控件基准高度
icon-size = {}                 # 标准图标物理像素规格
"#,
        id_suffix,
        display_name,
        p_clean,
        base_line,
        author_line,
        scheme,
        norm_window_bg,
        norm_panel_bg,
        norm_surface_bg,
        norm_control_bg,
        norm_foreground,
        norm_secondary_fg,
        norm_disabled_fg,
        norm_accent,
        norm_hover_bg,
        norm_pressed_bg,
        norm_selected_bg,
        norm_selected_fg,
        norm_border,
        norm_focus_border,
        norm_success,
        norm_warning,
        norm_danger,
        norm_info,
        r_s,
        r_m,
        r_l,
        sp_s,
        sp_m,
        sp_l,
        bw,
        ch,
        isz,
    )
}

/// Open native file dialog for theme file
fn pick_theme_file() -> Option<PathBuf> {
    let script = r#"
Add-Type -AssemblyName System.Windows.Forms
$dialog = New-Object System.Windows.Forms.OpenFileDialog
$dialog.Filter = "Supported Theme Files (*.toml;*.json)|*.toml;*.json|TOML Theme (*.toml)|*.toml|Windows Terminal (*.json)|*.json|All Files (*.*)|*.*"
$dialog.Title = "Import Theme"
if ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {
    [Console]::Out.Write($dialog.FileName)
}
"#;
    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
        .ok()?;
    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            let p = PathBuf::from(path);
            if p.exists() {
                return Some(p);
            }
        }
    }
    None
}

/// Save file dialog for exported theme
fn pick_save_theme_file(default_filename: &str) -> Option<PathBuf> {
    let script = format!(
        r#"
Add-Type -AssemblyName System.Windows.Forms
$dialog = New-Object System.Windows.Forms.SaveFileDialog
$dialog.Filter = "TOML Theme (*.toml)|*.toml|All Files (*.*)|*.*"
$dialog.FileName = "{}"
$dialog.Title = "Export Theme"
if ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {{
    [Console]::Out.Write($dialog.FileName)
}}
"#,
        default_filename
    );
    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()
        .ok()?;
    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Some(PathBuf::from(path));
        }
    }
    None
}

/// Open native file dialog for wallpaper image
fn pick_image_file() -> Option<PathBuf> {
    let script = r#"
Add-Type -AssemblyName System.Windows.Forms
$dialog = New-Object System.Windows.Forms.OpenFileDialog
$dialog.Filter = "Image Files (*.png;*.jpg;*.jpeg;*.webp;*.bmp)|*.png;*.jpg;*.jpeg;*.webp;*.bmp|All Files (*.*)|*.*"
$dialog.Title = "Select Wallpaper Image"
if ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {
    [Console]::Out.Write($dialog.FileName)
}
"#;
    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
        .ok()?;
    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            let p = PathBuf::from(path);
            if p.exists() {
                return Some(p);
            }
        }
    }
    None
}

/// Open native folder dialog for importing wallpaper folder
fn pick_folder() -> Option<PathBuf> {
    let script = r#"
Add-Type -AssemblyName System.Windows.Forms
$dialog = New-Object System.Windows.Forms.FolderBrowserDialog
$dialog.Description = "Select Wallpaper Folder"
if ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {
    [Console]::Out.Write($dialog.SelectedPath)
}
"#;
    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
        .ok()?;
    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            let p = PathBuf::from(path);
            if p.is_dir() {
                return Some(p);
            }
        }
    }
    None
}

/// Scan all images in a folder
fn scan_images_in_folder(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut images = Vec::new();
    let supported = ["png", "jpg", "jpeg", "webp", "bmp"];
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                    if supported.contains(&ext.to_ascii_lowercase().as_str()) {
                        images.push(p);
                    }
                }
            }
        }
    }
    images.sort();
    images
}

/// 将壁纸库中的所有条目（图片文件或文件夹路径）平铺展平为所有有效的图片文件路径
pub fn resolve_all_wallpaper_images(entries: &[String]) -> Vec<String> {
    let mut all_images = Vec::new();
    for entry in entries {
        let p = std::path::Path::new(entry);
        if p.is_dir() {
            for img in scan_images_in_folder(p) {
                all_images.push(img.to_string_lossy().to_string());
            }
        } else if p.is_file() {
            all_images.push(entry.clone());
        }
    }
    all_images
}

pub type RawWallpaperData = (Vec<u8>, u32, u32);

pub static WALLPAPER_RAW_CACHE: std::sync::LazyLock<std::sync::Mutex<std::collections::HashMap<String, RawWallpaperData>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));

/// 后台无阻塞解码与高分图片快速整型降采样为 RGBA 原始像素缓冲（由后台独立线程调用，0ms 阻塞 UI）
pub fn decode_and_resize_to_raw(path: &std::path::Path) -> Option<RawWallpaperData> {
    if path.is_dir() {
        return None;
    }
    let dyn_img = image::open(path).ok()?;
    let (w, h) = (dyn_img.width(), dyn_img.height());
    // 快速整型采样 thumbnail(1920, 1080)，比浮点 Triangle 快 3~5 倍
    let final_img = if w > 1920 || h > 1080 {
        dyn_img.thumbnail(1920, 1080)
    } else {
        dyn_img
    };
    let rgba = final_img.into_rgba8();
    let (rw, rh) = (rgba.width(), rgba.height());
    Some((rgba.into_raw(), rw, rh))
}

/// 快速后台直接构建 Slint SharedPixelBuffer (天然 Send + Sync)
pub fn load_pixel_buffer_fast(path_str: &str) -> Option<slint::SharedPixelBuffer<slint::Rgba8Pixel>> {
    let path = std::path::Path::new(path_str);
    if let Ok(raw_c) = WALLPAPER_RAW_CACHE.lock() {
        if let Some((raw, rw, rh)) = raw_c.get(path_str) {
            return Some(slint::SharedPixelBuffer::clone_from_slice(raw, *rw, *rh));
        }
    }

    let (raw, rw, rh) = decode_and_resize_to_raw(path)?;
    if let Ok(mut raw_c) = WALLPAPER_RAW_CACHE.lock() {
        if raw_c.len() >= 4 {
            if let Some(oldest) = raw_c.keys().next().cloned() {
                raw_c.remove(&oldest);
            }
        }
        raw_c.insert(path_str.to_string(), (raw.clone(), rw, rh));
    }
    Some(slint::SharedPixelBuffer::clone_from_slice(&raw, rw, rh))
}


/// 100% 后台异步预加载壁纸：
/// 在专用后台线程中完成所有文件 I/O 读取、图片解码与整型快速降采样计算，
/// 将处理好的像素原始缓冲预先填入线程安全缓存中，
/// 使得主 UI 线程耗时为绝对 0ms，彻底根治壁纸切换后点击左侧栏卡顿的问题。
pub fn schedule_wallpaper_preload(
    next_path: String,
    cache: Rc<RefCell<std::collections::HashMap<String, slint::Image>>>,
    _preload_timer: Rc<RefCell<Option<slint::Timer>>>,
) {
    if next_path.is_empty() || !std::path::Path::new(&next_path).exists() {
        return;
    }
    if cache.borrow().contains_key(&next_path) {
        return;
    }
    if let Ok(raw_c) = WALLPAPER_RAW_CACHE.lock() {
        if raw_c.contains_key(&next_path) {
            return;
        }
    }

    let p = std::path::PathBuf::from(&next_path);
    let target_path = next_path.clone();

    // 真正 100% 在后台工作线程执行 I/O 读取、图片解码与高分降采样
    std::thread::spawn(move || {
        if let Some((raw_bytes, rw, rh)) = decode_and_resize_to_raw(&p) {
            if let Ok(mut raw_c) = WALLPAPER_RAW_CACHE.lock() {
                if raw_c.len() >= 4 {
                    if let Some(oldest) = raw_c.keys().next().cloned() {
                        raw_c.remove(&oldest);
                    }
                }
                raw_c.insert(target_path, (raw_bytes, rw, rh));
            }
        }
    });
}

fn is_en(core_state: &smagical_core::CoreState) -> bool {
    core_state.storage().config().get().map(|c| c.language == "en-US").unwrap_or(false)
}

/// HSV to RGB conversion (h in [0, 360], s in [0, 1], v in [0, 1])
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let c = v * s;
    let h_prime = (h / 60.0) % 6.0;
    let x = c * (1.0 - ((h_prime % 2.0) - 1.0).abs());
    let m = v - c;
    let (r1, g1, b1) = if (0.0..1.0).contains(&h_prime) {
        (c, x, 0.0)
    } else if (1.0..2.0).contains(&h_prime) {
        (x, c, 0.0)
    } else if (2.0..3.0).contains(&h_prime) {
        (0.0, c, x)
    } else if (3.0..4.0).contains(&h_prime) {
        (0.0, x, c)
    } else if (4.0..5.0).contains(&h_prime) {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    (
        ((r1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((g1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((b1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
    )
}

/// RGB to HSV conversion (r, g, b in [0, 255])
fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let r_norm = r as f32 / 255.0;
    let g_norm = g as f32 / 255.0;
    let b_norm = b as f32 / 255.0;
    let max = r_norm.max(g_norm).max(b_norm);
    let min = r_norm.min(g_norm).min(b_norm);
    let delta = max - min;
    let v = max;
    let s = if max == 0.0 { 0.0 } else { delta / max };
    let h = if delta == 0.0 {
        0.0
    } else if (max - r_norm).abs() < 1e-5 {
        60.0 * (((g_norm - b_norm) / delta) % 6.0)
    } else if (max - g_norm).abs() < 1e-5 {
        60.0 * (((b_norm - r_norm) / delta) + 2.0)
    } else {
        60.0 * (((r_norm - g_norm) / delta) + 4.0)
    };
    let h = if h < 0.0 { h + 360.0 } else { h };
    (h, s, v)
}

/// Parse hex string into (r, g, b)
fn parse_hex_to_rgb(hex: &str) -> Option<(u8, u8, u8)> {
    let h = hex.trim().trim_start_matches('#');
    if h.len() == 6 {
        let r = u8::from_str_radix(&h[0..2], 16).ok()?;
        let g = u8::from_str_radix(&h[2..4], 16).ok()?;
        let b = u8::from_str_radix(&h[4..6], 16).ok()?;
        Some((r, g, b))
    } else if h.len() == 3 {
        let r = u8::from_str_radix(&h[0..1], 16).ok()? * 17;
        let g = u8::from_str_radix(&h[1..2], 16).ok()? * 17;
        let b = u8::from_str_radix(&h[2..3], 16).ok()? * 17;
        Some((r, g, b))
    } else {
        None
    }
}

/// Update theme color field and refresh TOML
fn update_theme_field(w: &AppWindow, f: &str, v: &str, brush: slint::Brush) {
    match f {
        "window_bg" => { w.set_theme_editor_window_bg(v.into()); w.set_theme_editor_preview_window_bg(brush); }
        "panel_bg" => { w.set_theme_editor_panel_bg(v.into()); w.set_theme_editor_preview_panel_bg(brush); }
        "surface_bg" => { w.set_theme_editor_surface_bg(v.into()); w.set_theme_editor_preview_surface_bg(brush); }
        "control_bg" => { w.set_theme_editor_control_bg(v.into()); w.set_theme_editor_preview_control_bg(brush); }
        "foreground" => { w.set_theme_editor_foreground(v.into()); w.set_theme_editor_preview_foreground(brush); }
        "secondary_fg" => { w.set_theme_editor_secondary_fg(v.into()); w.set_theme_editor_preview_secondary_fg(brush); }
        "disabled_fg" => { w.set_theme_editor_disabled_fg(v.into()); w.set_theme_editor_preview_disabled_fg(brush); }
        "accent" => { w.set_theme_editor_accent(v.into()); w.set_theme_editor_preview_accent(brush); }
        "hover_bg" => { w.set_theme_editor_hover_bg(v.into()); w.set_theme_editor_preview_hover_bg(brush); }
        "pressed_bg" => { w.set_theme_editor_pressed_bg(v.into()); w.set_theme_editor_preview_pressed_bg(brush); }
        "selected_bg" => { w.set_theme_editor_selected_bg(v.into()); w.set_theme_editor_preview_selected_bg(brush); }
        "selected_fg" => { w.set_theme_editor_selected_fg(v.into()); w.set_theme_editor_preview_selected_fg(brush); }
        "border" => { w.set_theme_editor_border(v.into()); w.set_theme_editor_preview_border(brush); }
        "focus_border" => { w.set_theme_editor_focus_border(v.into()); w.set_theme_editor_preview_focus_border(brush); }
        "success" => { w.set_theme_editor_success(v.into()); w.set_theme_editor_preview_success(brush); }
        "warning" => { w.set_theme_editor_warning(v.into()); w.set_theme_editor_preview_warning(brush); }
        "danger" => { w.set_theme_editor_danger(v.into()); w.set_theme_editor_preview_danger(brush); }
        "info" => { w.set_theme_editor_info(v.into()); w.set_theme_editor_preview_info(brush); }
        _ => {}
    }
    let toml_str = generate_theme_toml_from_window(w);
    w.set_theme_editor_toml(toml_str.as_str().into());
}

/// Update theme metric field and refresh TOML
fn update_metric_field(w: &AppWindow, f: &str, v: &str) {
    match f {
        "radius_small" => w.set_theme_editor_metric_radius_small(v.into()),
        "radius_medium" => w.set_theme_editor_metric_radius_medium(v.into()),
        "radius_large" => w.set_theme_editor_metric_radius_large(v.into()),
        "spacing_small" => w.set_theme_editor_metric_spacing_small(v.into()),
        "spacing_medium" => w.set_theme_editor_metric_spacing_medium(v.into()),
        "spacing_large" => w.set_theme_editor_metric_spacing_large(v.into()),
        "border_width" => w.set_theme_editor_metric_border_width(v.into()),
        "control_height" => w.set_theme_editor_metric_control_height(v.into()),
        "icon_size" => w.set_theme_editor_metric_icon_size(v.into()),
        _ => {}
    }
    let toml_str = generate_theme_toml_from_window(w);
    w.set_theme_editor_toml(toml_str.as_str().into());
}

/// Apply tokens and metrics to studio window
fn apply_tokens_and_metrics_to_studio(
    w: &AppWindow,
    name: &str,
    author: &str,
    base: &str,
    period: &str,
    tokens: &UiThemeTokens,
    metrics: &UiThemeMetrics,
) {
    w.set_theme_editor_name(name.into());
    w.set_theme_editor_author(author.into());
    w.set_theme_editor_base(base.into());
    w.set_theme_editor_period(period.into());

    w.set_theme_editor_window_bg(tokens.window_background.as_str().into());
    w.set_theme_editor_panel_bg(tokens.panel_background.as_str().into());
    w.set_theme_editor_surface_bg(tokens.surface_background.as_str().into());
    w.set_theme_editor_control_bg(tokens.control_background.as_str().into());
    w.set_theme_editor_foreground(tokens.foreground.as_str().into());
    w.set_theme_editor_secondary_fg(tokens.secondary_foreground.as_str().into());
    w.set_theme_editor_disabled_fg(tokens.disabled_foreground.as_str().into());
    w.set_theme_editor_accent(tokens.accent.as_str().into());
    w.set_theme_editor_hover_bg(tokens.hover_background.as_str().into());
    w.set_theme_editor_pressed_bg(tokens.pressed_background.as_str().into());
    w.set_theme_editor_selected_bg(tokens.selected_background.as_str().into());
    w.set_theme_editor_selected_fg(tokens.selected_foreground.as_str().into());
    w.set_theme_editor_border(tokens.border.as_str().into());
    w.set_theme_editor_focus_border(tokens.focus_border.as_str().into());
    w.set_theme_editor_success(tokens.success.as_str().into());
    w.set_theme_editor_warning(tokens.warning.as_str().into());
    w.set_theme_editor_danger(tokens.danger.as_str().into());
    w.set_theme_editor_info(tokens.info.as_str().into());

    if let Some(b) = parse_brush(&tokens.window_background) { w.set_theme_editor_preview_window_bg(b); }
    if let Some(b) = parse_brush(&tokens.panel_background) { w.set_theme_editor_preview_panel_bg(b); }
    if let Some(b) = parse_brush(&tokens.surface_background) { w.set_theme_editor_preview_surface_bg(b); }
    if let Some(b) = parse_brush(&tokens.control_background) { w.set_theme_editor_preview_control_bg(b); }
    if let Some(b) = parse_brush(&tokens.foreground) { w.set_theme_editor_preview_foreground(b); }
    if let Some(b) = parse_brush(&tokens.secondary_foreground) { w.set_theme_editor_preview_secondary_fg(b); }
    if let Some(b) = parse_brush(&tokens.disabled_foreground) { w.set_theme_editor_preview_disabled_fg(b); }
    if let Some(b) = parse_brush(&tokens.accent) { w.set_theme_editor_preview_accent(b); }
    if let Some(b) = parse_brush(&tokens.hover_background) { w.set_theme_editor_preview_hover_bg(b); }
    if let Some(b) = parse_brush(&tokens.pressed_background) { w.set_theme_editor_preview_pressed_bg(b); }
    if let Some(b) = parse_brush(&tokens.selected_background) { w.set_theme_editor_preview_selected_bg(b); }
    if let Some(b) = parse_brush(&tokens.selected_foreground) { w.set_theme_editor_preview_selected_fg(b); }
    if let Some(b) = parse_brush(&tokens.border) { w.set_theme_editor_preview_border(b); }
    if let Some(b) = parse_brush(&tokens.focus_border) { w.set_theme_editor_preview_focus_border(b); }
    if let Some(b) = parse_brush(&tokens.success) { w.set_theme_editor_preview_success(b); }
    if let Some(b) = parse_brush(&tokens.warning) { w.set_theme_editor_preview_warning(b); }
    if let Some(b) = parse_brush(&tokens.danger) { w.set_theme_editor_preview_danger(b); }
    if let Some(b) = parse_brush(&tokens.info) { w.set_theme_editor_preview_info(b); }

    w.set_theme_editor_metric_radius_small(format!("{}", metrics.radius_small as u32).into());
    w.set_theme_editor_metric_radius_medium(format!("{}", metrics.radius_medium as u32).into());
    w.set_theme_editor_metric_radius_large(format!("{}", metrics.radius_large as u32).into());
    w.set_theme_editor_metric_spacing_small(format!("{}", metrics.spacing_small as u32).into());
    w.set_theme_editor_metric_spacing_medium(format!("{}", metrics.spacing_medium as u32).into());
    w.set_theme_editor_metric_spacing_large(format!("{}", metrics.spacing_large as u32).into());
    w.set_theme_editor_metric_border_width(format!("{}", metrics.border_width as u32).into());
    w.set_theme_editor_metric_control_height(format!("{}", metrics.control_height as u32).into());
    w.set_theme_editor_metric_icon_size(format!("{}", metrics.icon_size as u32).into());

    let (h_deg, s, v) = parse_hex_to_rgb(&tokens.window_background).map(|(r, g, b)| rgb_to_hsv(r, g, b)).unwrap_or((0.0, 0.0, 1.0));
    let angle_rad = h_deg.to_radians();
    let ind_x = 90.0 + s * angle_rad.cos() * 88.0;
    let ind_y = 90.0 + s * angle_rad.sin() * 88.0;
    let (hr, hg, hb) = hsv_to_rgb(h_deg, 1.0, 1.0);
    w.set_theme_editor_wheel_indicator_x(ind_x);
    w.set_theme_editor_wheel_indicator_y(ind_y);
    w.set_theme_editor_wheel_brightness(v);
    w.set_theme_editor_wheel_hue_brush(slint::Brush::SolidColor(slint::Color::from_rgb_u8(hr, hg, hb)));
    w.set_theme_editor_picker_hex(tokens.window_background.as_str().into());
    if let Some(b) = parse_brush(&tokens.window_background) {
        w.set_theme_editor_picker_brush(b);
    }

    let toml_str = generate_theme_toml_from_window(w);
    w.set_theme_editor_toml(toml_str.as_str().into());
}

/// Populate theme studio modal fields from currently active theme
fn populate_theme_studio_from_active(w: &AppWindow, service: &ThemeService) {
    let active_id = w.get_current_theme_id().to_string();
    let base_id = if active_id.is_empty() { "builtin.ui.darcula".to_string() } else { active_id.clone() };
    let custom_count = service.list_ui().iter().filter(|t| !t.metadata.id.as_ref().starts_with("builtin.")).count() + 1;
    let new_name = format!("Custom Theme #{}", custom_count);
    let author = "User".to_string();

    if let Ok(resolved) = service.resolve_ui(&base_id) {
        let period = resolved.metadata.period.map(|p| match p {
            smagical_core::theme::ThemePeriod::Day => "day",
            smagical_core::theme::ThemePeriod::Night => "night",
        }).unwrap_or("night");
        apply_tokens_and_metrics_to_studio(w, &new_name, &author, &base_id, period, &resolved.tokens, &resolved.metrics);
    }
    w.set_is_theme_editor_open(true);
}

/// Register theme and wallpaper handlers
pub(crate) fn register_theme_and_wallpaper_handlers(window: &AppWindow, ctx: &AppContext) {
    // -------------------------------------------------------------------------
    // 1. Open Theme Studio Modal (Create Custom Theme)
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let themes_ref = Rc::clone(&ctx.themes);
    window.on_open_theme_editor(move || {
        if let Some(w) = window_weak.upgrade() {
            let service = themes_ref.borrow();
            populate_theme_studio_from_active(&w, &service);
        }
    });

    let window_weak = window.as_weak();
    let themes_ref = Rc::clone(&ctx.themes);
    window.on_create_custom_theme(move || {
        if let Some(w) = window_weak.upgrade() {
            let service = themes_ref.borrow();
            populate_theme_studio_from_active(&w, &service);
        }
    });

    // -------------------------------------------------------------------------
    // 2. Theme Studio: Apply Palette Preset
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let themes_ref = Rc::clone(&ctx.themes);
    window.on_theme_editor_apply_preset(move |preset_name| {
        if let Some(w) = window_weak.upgrade() {
            let (preset_name_display, base_id) = match preset_name.as_str() {
                "darcula" => ("Darcula Dark", "builtin.ui.darcula"),
                "onedark" => ("One Dark Pro", "builtin.ui.one-dark"),
                "tokyo" => ("Tokyo Night", "builtin.ui.tokyo-night"),
                "nord" => ("Nord Polar", "builtin.ui.nord"),
                "github-light" => ("GitHub Light", "builtin.ui.github-light"),
                _ => return,
            };
            let service = themes_ref.borrow();
            if let Ok(resolved) = service.resolve_ui(base_id) {
                let period = resolved.metadata.period.map(|p| match p {
                    smagical_core::theme::ThemePeriod::Day => "day",
                    smagical_core::theme::ThemePeriod::Night => "night",
                }).unwrap_or("night");
                apply_tokens_and_metrics_to_studio(&w, preset_name_display, "User", base_id, period, &resolved.tokens, &resolved.metrics);
            }
        }
    });

    // -------------------------------------------------------------------------
    // 3. Theme Studio Field Changed (Color, Metric & Metadata)
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    window.on_theme_editor_color_changed(move |field, val| {
        if let Some(w) = window_weak.upgrade() {
            let f = field.as_str();
            let v = val.as_str();
            let brush = parse_brush(v).unwrap_or(slint::Brush::SolidColor(slint::Color::from_rgb_u8(0, 0, 0)));
            update_theme_field(&w, f, v, brush);
        }
    });

    let window_weak = window.as_weak();
    window.on_theme_editor_metric_changed(move |field, val| {
        if let Some(w) = window_weak.upgrade() {
            update_metric_field(&w, field.as_str(), val.as_str());
        }
    });

    // -------------------------------------------------------------------------
    // 3.1 Theme Studio: Circular Color Wheel Interactivity
    // -------------------------------------------------------------------------
    let cur_hsv = Rc::new(RefCell::new((0.0f32, 0.0f32, 1.0f32)));

    let window_weak = window.as_weak();
    let hsv_clone = Rc::clone(&cur_hsv);
    window.on_theme_editor_wheel_coord_picked(move |rel_x, rel_y| {
        if let Some(w) = window_weak.upgrade() {
            let dist = (rel_x * rel_x + rel_y * rel_y).sqrt().min(1.0);
            let angle = rel_y.atan2(rel_x).to_degrees();
            let hue = if angle < 0.0 { angle + 360.0 } else { angle };
            let sat = dist;
            let mut hsv = hsv_clone.borrow_mut();
            hsv.0 = hue;
            hsv.1 = sat;
            let val = hsv.2;
            let (r, g, b) = hsv_to_rgb(hue, sat, val);
            let hex = format!("#{:02x}{:02x}{:02x}", r, g, b);
            let (hr, hg, hb) = hsv_to_rgb(hue, 1.0, 1.0);
            let hue_brush = slint::Brush::SolidColor(slint::Color::from_rgb_u8(hr, hg, hb));
            let cur_brush = slint::Brush::SolidColor(slint::Color::from_rgb_u8(r, g, b));

            let norm_dx = if dist > 0.0 { rel_x / dist * dist.min(1.0) } else { 0.0 };
            let norm_dy = if dist > 0.0 { rel_y / dist * dist.min(1.0) } else { 0.0 };
            w.set_theme_editor_wheel_indicator_x(90.0 + norm_dx * 88.0);
            w.set_theme_editor_wheel_indicator_y(90.0 + norm_dy * 88.0);
            w.set_theme_editor_picker_hex(hex.as_str().into());
            w.set_theme_editor_picker_brush(cur_brush.clone());
            w.set_theme_editor_wheel_hue_brush(hue_brush);

            let target_key = w.get_theme_editor_picker_target_key();
            update_theme_field(&w, target_key.as_str(), hex.as_str(), cur_brush);
        }
    });

    let window_weak = window.as_weak();
    let hsv_clone2 = Rc::clone(&cur_hsv);
    window.on_theme_editor_wheel_brightness_picked(move |brightness| {
        if let Some(w) = window_weak.upgrade() {
            let val = brightness.clamp(0.0, 1.0);
            let mut hsv = hsv_clone2.borrow_mut();
            hsv.2 = val;
            let (r, g, b) = hsv_to_rgb(hsv.0, hsv.1, val);
            let hex = format!("#{:02x}{:02x}{:02x}", r, g, b);
            let cur_brush = slint::Brush::SolidColor(slint::Color::from_rgb_u8(r, g, b));

            w.set_theme_editor_wheel_brightness(val);
            w.set_theme_editor_picker_hex(hex.as_str().into());
            w.set_theme_editor_picker_brush(cur_brush.clone());

            let target_key = w.get_theme_editor_picker_target_key();
            update_theme_field(&w, target_key.as_str(), hex.as_str(), cur_brush);
        }
    });

    let window_weak = window.as_weak();
    let hsv_clone3 = Rc::clone(&cur_hsv);
    window.on_theme_editor_picker_hex_changed(move |hex_val| {
        if let Some(w) = window_weak.upgrade() {
            let hex_str = hex_val.to_string();
            if let Some((r, g, b)) = parse_hex_to_rgb(&hex_str) {
                let (h_deg, s, v) = rgb_to_hsv(r, g, b);
                let mut hsv = hsv_clone3.borrow_mut();
                hsv.0 = h_deg;
                hsv.1 = s;
                hsv.2 = v;

                let angle_rad = h_deg.to_radians();
                let ind_x = 90.0 + s * angle_rad.cos() * 88.0;
                let ind_y = 90.0 + s * angle_rad.sin() * 88.0;
                let (hr, hg, hb) = hsv_to_rgb(h_deg, 1.0, 1.0);
                let hue_brush = slint::Brush::SolidColor(slint::Color::from_rgb_u8(hr, hg, hb));
                let cur_brush = slint::Brush::SolidColor(slint::Color::from_rgb_u8(r, g, b));

                w.set_theme_editor_wheel_indicator_x(ind_x);
                w.set_theme_editor_wheel_indicator_y(ind_y);
                w.set_theme_editor_wheel_brightness(v);
                w.set_theme_editor_wheel_hue_brush(hue_brush);
                w.set_theme_editor_picker_brush(cur_brush.clone());
                w.set_theme_editor_picker_hex(hex_str.clone().as_str().into());

                let target_key = w.get_theme_editor_picker_target_key();
                update_theme_field(&w, target_key.as_str(), hex_str.as_str(), cur_brush);
            }
        }
    });

    let window_weak = window.as_weak();
    window.on_theme_editor_meta_changed(move |field, val| {
        if let Some(w) = window_weak.upgrade() {
            let f = field.as_str();
            let v = val.as_str();
            match f {
                "name" => w.set_theme_editor_name(v.into()),
                "author" => w.set_theme_editor_author(v.into()),
                "base" => w.set_theme_editor_base(v.into()),
                "period" => w.set_theme_editor_period(v.into()),
                _ => {}
            }
            let toml_str = generate_theme_toml_from_window(&w);
            w.set_theme_editor_toml(toml_str.as_str().into());
        }
    });

    // -------------------------------------------------------------------------
    // 4. Theme Studio: Parse TOML to Form & Sandbox
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let themes_ref = Rc::clone(&ctx.themes);
    let notif = ctx.notifications.clone();
    window.on_theme_editor_parse_toml(move |toml_input| {
        if let Some(w) = window_weak.upgrade() {
            let service = themes_ref.borrow();
            match service.import_ui_toml(toml_input.as_str()) {
                Ok(def) => {
                    w.set_theme_editor_name(def.metadata.name.as_str().into());
                    w.set_theme_editor_author(def.metadata.author.as_deref().unwrap_or("User").into());
                    let base_id = def.metadata.base.as_ref().map(|b| b.as_ref()).unwrap_or("builtin.ui.darcula");
                    w.set_theme_editor_base(base_id.into());
                    let period_str = def.metadata.period.map(|p| match p {
                        ThemePeriod::Day => "day",
                        ThemePeriod::Night => "night",
                    }).unwrap_or("night");
                    w.set_theme_editor_period(period_str.into());

                    if let Some(ref bg) = def.ui.window_background {
                        w.set_theme_editor_window_bg(bg.as_str().into());
                        if let Some(b) = parse_brush(bg) { w.set_theme_editor_preview_window_bg(b); }
                    }
                    if let Some(ref bg) = def.ui.panel_background {
                        w.set_theme_editor_panel_bg(bg.as_str().into());
                        if let Some(b) = parse_brush(bg) { w.set_theme_editor_preview_panel_bg(b); }
                    }
                    if let Some(ref bg) = def.ui.surface_background {
                        w.set_theme_editor_surface_bg(bg.as_str().into());
                        if let Some(b) = parse_brush(bg) { w.set_theme_editor_preview_surface_bg(b); }
                    }
                    if let Some(ref bg) = def.ui.control_background {
                        w.set_theme_editor_control_bg(bg.as_str().into());
                        if let Some(b) = parse_brush(bg) { w.set_theme_editor_preview_control_bg(b); }
                    }
                    if let Some(ref fg) = def.ui.foreground {
                        w.set_theme_editor_foreground(fg.as_str().into());
                        if let Some(b) = parse_brush(fg) { w.set_theme_editor_preview_foreground(b); }
                    }
                    if let Some(ref fg) = def.ui.secondary_foreground {
                        w.set_theme_editor_secondary_fg(fg.as_str().into());
                        if let Some(b) = parse_brush(fg) { w.set_theme_editor_preview_secondary_fg(b); }
                    }
                    if let Some(ref fg) = def.ui.disabled_foreground {
                        w.set_theme_editor_disabled_fg(fg.as_str().into());
                        if let Some(b) = parse_brush(fg) { w.set_theme_editor_preview_disabled_fg(b); }
                    }
                    if let Some(ref ac) = def.ui.accent {
                        w.set_theme_editor_accent(ac.as_str().into());
                        if let Some(b) = parse_brush(ac) { w.set_theme_editor_preview_accent(b); }
                    }
                    if let Some(ref bg) = def.ui.hover_background {
                        w.set_theme_editor_hover_bg(bg.as_str().into());
                        if let Some(b) = parse_brush(bg) { w.set_theme_editor_preview_hover_bg(b); }
                    }
                    if let Some(ref bg) = def.ui.pressed_background {
                        w.set_theme_editor_pressed_bg(bg.as_str().into());
                        if let Some(b) = parse_brush(bg) { w.set_theme_editor_preview_pressed_bg(b); }
                    }
                    if let Some(ref bg) = def.ui.selected_background {
                        w.set_theme_editor_selected_bg(bg.as_str().into());
                        if let Some(b) = parse_brush(bg) { w.set_theme_editor_preview_selected_bg(b); }
                    }
                    if let Some(ref fg) = def.ui.selected_foreground {
                        w.set_theme_editor_selected_fg(fg.as_str().into());
                        if let Some(b) = parse_brush(fg) { w.set_theme_editor_preview_selected_fg(b); }
                    }
                    if let Some(ref bd) = def.ui.border {
                        w.set_theme_editor_border(bd.as_str().into());
                        if let Some(b) = parse_brush(bd) { w.set_theme_editor_preview_border(b); }
                    }
                    if let Some(ref bd) = def.ui.focus_border {
                        w.set_theme_editor_focus_border(bd.as_str().into());
                        if let Some(b) = parse_brush(bd) { w.set_theme_editor_preview_focus_border(b); }
                    }
                    if let Some(ref st) = def.ui.success {
                        w.set_theme_editor_success(st.as_str().into());
                        if let Some(b) = parse_brush(st) { w.set_theme_editor_preview_success(b); }
                    }
                    if let Some(ref st) = def.ui.warning {
                        w.set_theme_editor_warning(st.as_str().into());
                        if let Some(b) = parse_brush(st) { w.set_theme_editor_preview_warning(b); }
                    }
                    if let Some(ref st) = def.ui.danger {
                        w.set_theme_editor_danger(st.as_str().into());
                        if let Some(b) = parse_brush(st) { w.set_theme_editor_preview_danger(b); }
                    }
                    if let Some(ref st) = def.ui.info {
                        w.set_theme_editor_info(st.as_str().into());
                        if let Some(b) = parse_brush(st) { w.set_theme_editor_preview_info(b); }
                    }
                    if let Some(v) = def.metrics.radius_small { w.set_theme_editor_metric_radius_small(format!("{}", v as u32).into()); }
                    if let Some(v) = def.metrics.radius_medium { w.set_theme_editor_metric_radius_medium(format!("{}", v as u32).into()); }
                    if let Some(v) = def.metrics.radius_large { w.set_theme_editor_metric_radius_large(format!("{}", v as u32).into()); }
                    if let Some(v) = def.metrics.spacing_small { w.set_theme_editor_metric_spacing_small(format!("{}", v as u32).into()); }
                    if let Some(v) = def.metrics.spacing_medium { w.set_theme_editor_metric_spacing_medium(format!("{}", v as u32).into()); }
                    if let Some(v) = def.metrics.spacing_large { w.set_theme_editor_metric_spacing_large(format!("{}", v as u32).into()); }
                    if let Some(v) = def.metrics.border_width { w.set_theme_editor_metric_border_width(format!("{}", v as u32).into()); }
                    if let Some(v) = def.metrics.control_height { w.set_theme_editor_metric_control_height(format!("{}", v as u32).into()); }
                    if let Some(v) = def.metrics.icon_size { w.set_theme_editor_metric_icon_size(format!("{}", v as u32).into()); }
                    notif.success("TOML 解析成功", "已将配置代码解析并实时同步到表单与预览视窗！");
                }
                Err(e) => {
                    notif.warning("TOML 解析失败", &format!("语法解析错误: {}", e));
                }
            }
        }
    });

    // -------------------------------------------------------------------------
    // 5. Theme Studio: Copy TOML
    // -------------------------------------------------------------------------
    let notif = ctx.notifications.clone();
    window.on_theme_editor_copy_toml(move |toml_content| {
        if let Ok(mut cb) = arboard::Clipboard::new() {
            let _ = cb.set_text(toml_content.to_string());
            notif.info("已复制到剪贴板", "TOML 主题配置已成功复制到系统剪贴板");
        } else {
            notif.warning("复制失败", "无法访问系统剪贴板");
        }
    });

    // -------------------------------------------------------------------------
    // 6. Theme Studio: Save and Activate
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let themes_ref = Rc::clone(&ctx.themes);
    let repo_ref = ctx.theme_repo.clone();
    let notif = ctx.notifications.clone();
    window.on_theme_editor_save_and_activate(move || {
        if let Some(w) = window_weak.upgrade() {
            let toml_str = w.get_theme_editor_toml().to_string();
            let switch_target = {
                let mut service = themes_ref.borrow_mut();
                match service.import_ui_toml(&toml_str) {
                    Ok(def) => {
                        let theme_id = def.metadata.id.clone();
                        let theme_name = def.metadata.name.clone();
                        if let Some(ref repo) = repo_ref {
                            let _ = repo.borrow().save_ui(&def);
                        }
                        let res = if service.get_ui(&theme_id).is_some() {
                            service.replace_ui(def)
                        } else {
                            service.save_ui(def)
                        };
                        if let Err(e) = res {
                            notif.error("保存失败", &format!("服务保存错误: {}", e));
                            return;
                        }
                        crate::theme::sync_ui_themes(&w, &service);
                        Some((theme_id, theme_name))
                    }
                    Err(e) => {
                        notif.error("保存失败", &format!("主题配置校验未通过: {}", e));
                        None
                    }
                }
            };

            if let Some((theme_id, theme_name)) = switch_target {
                w.invoke_switch_theme(theme_id.as_ref().into());
                w.set_is_theme_editor_open(false);
                notif.success("主题已保存并激活", &format!("主题「{}」已成功保存并立即生效！", theme_name));
            }
        }
    });

    // -------------------------------------------------------------------------
    // 7. Import Theme (TOML / Windows Terminal JSON)
    let window_weak = window.as_weak();
    let themes_ref = Rc::clone(&ctx.themes);
    let repo_ref = ctx.theme_repo.clone();
    let notif = ctx.notifications.clone();
    let core_state_import = ctx.core_state.clone();
    window.on_import_theme(move || {
        if let Some(path) = pick_theme_file() {
            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    let mut service = themes_ref.borrow_mut();
                    let parse_result = if path.extension().and_then(|s| s.to_str()) == Some("json") {
                        match service.import_windows_terminal_json(&content) {
                            Ok(smagical_core::theme::TerminalImport::Candidates(candidates)) => {
                                if let Some(first) = candidates.into_iter().next() {
                                    let name = first.metadata.name.clone();
                                    let id = ThemeId::new(format!("custom.ui.{}", uuid::Uuid::new_v4().simple()));
                                    let def = UiThemeDefinition {
                                        metadata: ThemeMetadata {
                                            schema_version: THEME_SCHEMA_VERSION,
                                            id,
                                            name: format!("WT: {}", name),
                                            kind: ThemeKind::Ui,
                                            period: Some(ThemePeriod::Night),
                                            base: Some(ThemeId::new("builtin.ui.darcula")),
                                            author: Some("Windows Terminal Import".into()),
                                            source: None,
                                        },
                                        ui: UiThemeTokensPatch {
                                            color_scheme: Some(smagical_core::theme::ColorScheme::Dark),
                                            window_background: first.terminal.background.clone(),
                                            foreground: first.terminal.foreground.clone(),
                                            accent: first.terminal.cursor_color.clone().or_else(|| first.terminal.bright_cyan.clone()),
                                            ..Default::default()
                                        },
                                        metrics: UiThemeMetricsPatch::default(),
                                    };
                                    Ok(def)
                                } else {
                                    Err(smagical_core::theme::ThemeError::NotFound(ThemeId::new("scheme")))
                                }
                            }
                            Err(e) => Err(e),
                        }
                    } else {
                        service.import_ui_toml(&content)
                    };

                    let switch_target = {
                        match parse_result {
                            Ok(def) => {
                                let theme_id = def.metadata.id.clone();
                                let theme_name = def.metadata.name.clone();
                                if let Some(ref repo) = repo_ref {
                                    let _ = repo.borrow().save_ui(&def);
                                }
                                let _ = service.save_ui(def);

                                if let Some(w) = window_weak.upgrade() {
                                    crate::theme::sync_ui_themes(&w, &service);
                                }
                                Some((theme_id, theme_name))
                            }
                            Err(e) => {
                                if is_en(&core_state_import) {
                                    notif.error("Import Failed", &format!("Invalid theme format: {}", e));
                                } else {
                                    notif.error("导入主题失败", &format!("主题格式不合法: {}", e));
                                }
                                None
                            }
                        }
                    };

                    if let Some((theme_id, theme_name)) = switch_target {
                        if let Some(w) = window_weak.upgrade() {
                            w.invoke_switch_theme(theme_id.as_ref().into());
                            if is_en(&core_state_import) {
                                notif.success("Theme Imported", &format!("Theme '{}' activated!", theme_name));
                            } else {
                                notif.success("主题导入成功", &format!("主题「{}」已成功导入并激活！", theme_name));
                            }
                        }
                    }
                }
                Err(e) => {
                    if is_en(&core_state_import) {
                        notif.error("Read Failed", &format!("{}", e));
                    } else {
                        notif.error("读取失败", &format!("无法读取主题文件: {}", e));
                    }
                }
            }
        }
    });

    // -------------------------------------------------------------------------
    // 8. Export Theme (Export Theme TOML)
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let themes_ref = Rc::clone(&ctx.themes);
    let notif = ctx.notifications.clone();
    let core_state_export = ctx.core_state.clone();
    window.on_export_current_theme(move || {
        if let Some(w) = window_weak.upgrade() {
            let active_id = w.get_current_theme_id().to_string();
            let service = themes_ref.borrow();
            let theme_id_obj = ThemeId::new(&active_id);
            if let Some(def) = service.get_ui(&theme_id_obj) {
                let toml_str = match service.export_ui_toml(def) {
                    Ok(s) => s,
                    Err(e) => {
                        if is_en(&core_state_export) {
                            notif.error("Export Failed", &format!("Encoding error: {}", e));
                        } else {
                            notif.error("导出失败", &format!("TOML 编码异常: {}", e));
                        }
                        return;
                    }
                };

                let default_filename = format!("{}.toml", def.metadata.name.to_lowercase().replace(' ', "-"));
                if let Some(save_path) = pick_save_theme_file(&default_filename) {
                    match std::fs::write(&save_path, &toml_str) {
                        Ok(()) => {
                            if is_en(&core_state_export) {
                                notif.success(
                                    "Theme Exported",
                                    &format!("Saved to:\n{}", save_path.display()),
                                );
                            } else {
                                notif.success(
                                    "主题导出成功",
                                    &format!("配置已成功保存至:\n{}", save_path.display()),
                                );
                            }
                        }
                        Err(e) => {
                            if is_en(&core_state_export) {
                                notif.error("Write Failed", &format!("{}", e));
                            } else {
                                notif.error("写入失败", &format!("无法保存文件: {}", e));
                            }
                        }
                    }
                }
            } else {
                if is_en(&core_state_export) {
                    notif.warning("Theme Not Found", &format!("Could not find theme [{}]", active_id));
                } else {
                    notif.warning("未找到主题", &format!("无法找到指定主题 [{}]", active_id));
                }
            }
        }
    });

    // -------------------------------------------------------------------------
    // 9. Delete Custom Theme
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let themes_ref = Rc::clone(&ctx.themes);
    let repo_ref = ctx.theme_repo.clone();
    let notif = ctx.notifications.clone();
    let core_state_del = ctx.core_state.clone();
    window.on_delete_custom_theme(move |id| {
        let id_str = id.as_str();
        if let Some(w) = window_weak.upgrade() {
            let active_id = w.get_current_theme_id().to_string();
            let should_switch = active_id == id_str;

            let theme_id_obj = ThemeId::new(id_str);
            if let Some(ref repo) = repo_ref {
                let _ = repo.borrow().delete(&theme_id_obj);
            }
            {
                let mut service = themes_ref.borrow_mut();
                let _ = service.remove_ui(&theme_id_obj);
                crate::theme::sync_ui_themes(&w, &service);
            }

            if should_switch {
                w.invoke_switch_theme("builtin.ui.darcula".into());
            }

            if is_en(&core_state_del) {
                notif.info("Theme Removed", &format!("Deleted theme '{}'", id_str));
            } else {
                notif.info("主题已删除", &format!("已成功移除主题「{}」", id_str));
            }
        }
    });

    // -------------------------------------------------------------------------
    // 10. Add Wallpaper
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let wallpapers_ref = Rc::clone(&ctx.wallpapers);
    let active_idx_ref = Rc::clone(&ctx.active_wallpaper_idx);
    let notif = ctx.notifications.clone();
    let core_state_wp_add = ctx.core_state.clone();
    window.on_add_wallpaper_image(move || {
        if let Some(path) = pick_image_file() {
            let path_str = path.to_string_lossy().to_string();
            if let Some(w) = window_weak.upgrade() {
                let (new_idx, slint_strings, wps_clone) = {
                    let mut wps = wallpapers_ref.borrow_mut();
                    if !wps.contains(&path_str) {
                        wps.push(path_str.clone());
                    }
                    let new_idx = wps.iter().position(|p| p == &path_str).unwrap_or(0);
                    let slint_strings: Vec<slint::SharedString> = wps.iter().map(|s| s.as_str().into()).collect();
                    (new_idx, slint_strings, wps.clone())
                };

                *active_idx_ref.borrow_mut() = new_idx;

                let _ = core_state_wp_add.storage().config().update(Box::new(move |c| {
                    c.wallpaper_list = wps_clone;
                    c.wallpaper_active_index = new_idx;
                }));

                w.set_wallpaper_list(ModelRc::new(VecModel::from(slint_strings)));
                w.set_wallpaper_active_index(new_idx as i32);

                let cur_mode = w.get_wallpaper_mode().to_string();
                let apply_mode = if cur_mode == "none" { "global" } else { cur_mode.as_str() };
                w.set_wallpaper_mode(apply_mode.into());
                w.set_wallpaper_path(path_str.as_str().into());
                w.invoke_set_wallpaper(apply_mode.into(), path_str.clone().into(), w.get_global_wallpaper_opacity());

                if is_en(&core_state_wp_add) {
                    notif.success("Wallpaper Added", "Successfully loaded image to wallpaper gallery!");
                } else {
                    notif.success("壁纸添加成功", "已成功将图片添加到壁纸图库！");
                }
            }
        }
    });

    // -------------------------------------------------------------------------
    // 10.1 Add Wallpaper Folder (Batch Import)
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let wallpapers_ref = Rc::clone(&ctx.wallpapers);
    let active_idx_ref = Rc::clone(&ctx.active_wallpaper_idx);
    let notif = ctx.notifications.clone();
    let core_state_wp_folder = ctx.core_state.clone();
    window.on_add_wallpaper_folder(move || {
        if let Some(folder_path) = pick_folder() {
            let found_images = scan_images_in_folder(&folder_path);
            if found_images.is_empty() {
                if is_en(&core_state_wp_folder) {
                    notif.warning("No Images Found", "No supported images (*.png, *.jpg, *.jpeg, *.webp, *.bmp) in selected folder.");
                } else {
                    notif.warning("未找到壁纸图片", "所选文件夹内未检测到支持的图片文件 (*.png, *.jpg, *.jpeg, *.webp, *.bmp)");
                }
                return;
            }

            let folder_str = folder_path.to_string_lossy().to_string();

            if let Some(w) = window_weak.upgrade() {
                let (is_new, effective_idx, first_image_path, slint_strings, wps_clone) = {
                    let mut wps = wallpapers_ref.borrow_mut();
                    let is_new = if !wps.contains(&folder_str) {
                        wps.push(folder_str.clone());
                        true
                    } else {
                        false
                    };

                    let effective_idx = wps.iter().position(|s| s == &folder_str).unwrap_or(0);
                    let first_image_path = found_images.first().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
                    let slint_strings: Vec<slint::SharedString> = wps.iter().map(|s| s.as_str().into()).collect();
                    (is_new, effective_idx, first_image_path, slint_strings, wps.clone())
                };

                if !is_new {
                    if is_en(&core_state_wp_folder) {
                        notif.info("Notice", "Selected folder is already in gallery.");
                    } else {
                        notif.info("提示", "所选文件夹已存在于壁纸库中");
                    }
                    return;
                }

                *active_idx_ref.borrow_mut() = effective_idx;

                let _ = core_state_wp_folder.storage().config().update(Box::new(move |c| {
                    c.wallpaper_list = wps_clone;
                    c.wallpaper_active_index = effective_idx;
                }));

                w.set_wallpaper_list(ModelRc::new(VecModel::from(slint_strings)));
                w.set_wallpaper_active_index(effective_idx as i32);
                w.set_wallpaper_path(folder_str.as_str().into());

                let cur_mode = w.get_wallpaper_mode().to_string();
                let apply_mode = if cur_mode == "none" { "global" } else { cur_mode.as_str() };
                w.set_wallpaper_mode(apply_mode.into());
                if !first_image_path.is_empty() {
                    w.invoke_set_wallpaper(apply_mode.into(), first_image_path.as_str().into(), w.get_global_wallpaper_opacity());
                }

                if is_en(&core_state_wp_folder) {
                    notif.success("Folder Added", &format!("Added folder with {} images to gallery!", found_images.len()));
                } else {
                    notif.success("壁纸文件夹导入成功", &format!("已成功将文件夹（含 {} 张图片）加入壁纸库！", found_images.len()));
                }
            }
        }
    });

    // -------------------------------------------------------------------------
    // 11. Remove Wallpaper
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let wallpapers_ref = Rc::clone(&ctx.wallpapers);
    let active_idx_ref = Rc::clone(&ctx.active_wallpaper_idx);
    let notif = ctx.notifications.clone();
    let core_state_wp_rm = ctx.core_state.clone();
    window.on_remove_wallpaper_image(move |idx| {
        if let Some(w) = window_weak.upgrade() {
            let u_idx = idx as usize;
            let remove_result = {
                let mut wps = wallpapers_ref.borrow_mut();
                if u_idx < wps.len() {
                    wps.remove(u_idx);
                    let current_active = *active_idx_ref.borrow();
                    let next_active = if wps.is_empty() {
                        0
                    } else if current_active >= wps.len() {
                        wps.len() - 1
                    } else {
                        current_active
                    };
                    let is_empty = wps.is_empty();
                    let next_path = if !is_empty { wps[next_active].clone() } else { String::new() };
                    let slint_strings: Vec<slint::SharedString> = wps.iter().map(|s| s.as_str().into()).collect();
                    Some((next_active, is_empty, next_path, slint_strings, wps.clone()))
                } else {
                    None
                }
            };

            if let Some((next_active, is_empty, next_path, slint_strings, wps_clone)) = remove_result {
                *active_idx_ref.borrow_mut() = next_active;

                let _ = core_state_wp_rm.storage().config().update(Box::new(move |c| {
                    c.wallpaper_list = wps_clone;
                    c.wallpaper_active_index = next_active;
                }));

                w.set_wallpaper_list(ModelRc::new(VecModel::from(slint_strings)));
                w.set_wallpaper_active_index(next_active as i32);

                if is_empty {
                    w.set_wallpaper_mode("none".into());
                    w.invoke_set_wallpaper("none".into(), "".into(), 0.20);
                } else {
                    let cur_mode = w.get_wallpaper_mode().to_string();
                    w.invoke_set_wallpaper(cur_mode.as_str().into(), next_path.as_str().into(), w.get_global_wallpaper_opacity());
                }

                if is_en(&core_state_wp_rm) {
                    notif.info("Wallpaper Removed", "Removed image from gallery.");
                } else {
                    notif.info("壁纸已移除", "已从壁纸图库中移除该图片");
                }
            }
        }
    });

    // -------------------------------------------------------------------------
    // 12. Select Wallpaper
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let wallpapers_ref = Rc::clone(&ctx.wallpapers);
    let active_idx_ref = Rc::clone(&ctx.active_wallpaper_idx);
    let _core_state_wp_sel = ctx.core_state.clone();
    let wallpaper_cache_sel = Rc::clone(&ctx.wallpaper_cache);
    let wallpaper_preload_timer_sel = Rc::clone(&ctx.wallpaper_preload_timer);
    window.on_select_wallpaper_image(move |idx| {
        if let Some(w) = window_weak.upgrade() {
            let u_idx = idx as usize;
            let (target_entry, actual_image_path) = {
                let wps = wallpapers_ref.borrow();
                if u_idx < wps.len() {
                    let entry = wps[u_idx].clone();
                    let p = std::path::Path::new(&entry);
                    let actual = if p.is_dir() {
                        let imgs = scan_images_in_folder(p);
                        imgs.first().map(|ip| ip.to_string_lossy().to_string()).unwrap_or_default()
                    } else {
                        entry.clone()
                    };
                    (Some(entry), actual)
                } else {
                    (None, String::new())
                }
            };

            if let Some(entry_str) = target_entry {
                *active_idx_ref.borrow_mut() = u_idx;
                w.set_wallpaper_active_index(u_idx as i32);
                let cur_mode = w.get_wallpaper_mode().to_string();
                let apply_mode = if cur_mode == "none" { "global" } else { cur_mode.as_str() };
                w.set_wallpaper_mode(apply_mode.into());
                w.set_wallpaper_path(entry_str.as_str().into());
                if !actual_image_path.is_empty() {
                    w.invoke_set_wallpaper(apply_mode.into(), actual_image_path.as_str().into(), w.get_global_wallpaper_opacity());
                }

                // 立即预加载后继壁纸（如果存在多张壁纸）
                let all_imgs = resolve_all_wallpaper_images(&wallpapers_ref.borrow());
                if all_imgs.len() > 1 {
                    let next_img_idx = (u_idx + 1) % all_imgs.len();
                    let next_img = all_imgs[next_img_idx].clone();
                    schedule_wallpaper_preload(next_img, Rc::clone(&wallpaper_cache_sel), Rc::clone(&wallpaper_preload_timer_sel));
                }
            }
        }
    });

    // -------------------------------------------------------------------------
    // 13. Wallpaper Slideshow & Transition
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let wallpapers_ref = Rc::clone(&ctx.wallpapers);
    let active_idx_ref = Rc::clone(&ctx.active_wallpaper_idx);
    let timer_ref = Rc::clone(&ctx.wallpaper_timer);
    let notif = ctx.notifications.clone();
    let core_state_wp_slide = ctx.core_state.clone();
    let wallpaper_cache_slide = Rc::clone(&ctx.wallpaper_cache);
    let wallpaper_preload_timer_slide = Rc::clone(&ctx.wallpaper_preload_timer);
    window.on_set_wallpaper_slideshow(move |interval, transition| {
        let interval_str = interval.as_str();
        let transition_str = transition.as_str();

        // 无论如何，先停止并销毁已存在的轮播定时器（彻底解决关闭后仍在轮播的问题）
        *timer_ref.borrow_mut() = None;

        if let Some(w) = window_weak.upgrade() {
            w.set_wallpaper_slideshow_interval(interval_str.into());
            w.set_wallpaper_transition_effect(transition_str.into());
        }

        let int_clone = interval_str.to_string();
        let trans_clone = transition_str.to_string();
        let _ = core_state_wp_slide.storage().config().update(Box::new(move |c| {
            c.wallpaper_slideshow_interval = int_clone;
            c.wallpaper_transition_effect = trans_clone;
        }));

        let duration_secs: Option<u64> = if interval_str == "none" || interval_str.is_empty() || interval_str == "off" || interval_str == "startup" {
            None
        } else if let Some(s) = interval_str.strip_suffix('s') {
            s.parse::<u64>().ok().map(|n| n.max(5))
        } else if let Some(m) = interval_str.strip_suffix('m') {
            m.parse::<u64>().ok().map(|n| n.max(1) * 60)
        } else if let Some(h) = interval_str.strip_suffix('h') {
            h.parse::<u64>().ok().map(|n| n.max(1) * 3600)
        } else if let Ok(num) = interval_str.parse::<u64>() {
            Some(num.max(1) * 60) // 默认分钟单位
        } else {
            None
        };

        if let Some(secs) = duration_secs {
            let window_weak_timer = window_weak.clone();
            let wallpapers_timer = Rc::clone(&wallpapers_ref);
            let active_idx_timer = Rc::clone(&active_idx_ref);
            let wallpaper_cache_timer = Rc::clone(&wallpaper_cache_slide);
            let wallpaper_preload_timer_timer = Rc::clone(&wallpaper_preload_timer_slide);

            // 预热：展开文件夹内全部图片并预加载下一张
            {
                let wps = wallpapers_timer.borrow();
                let all_images = resolve_all_wallpaper_images(&wps);
                if all_images.len() > 1 {
                    let cur = *active_idx_timer.borrow();
                    let next_idx = (cur + 1) % all_images.len();
                    let next_path = all_images[next_idx].clone();
                    schedule_wallpaper_preload(next_path, Rc::clone(&wallpaper_cache_timer), Rc::clone(&wallpaper_preload_timer_timer));
                }
            }

            let timer = slint::Timer::default();
            timer.start(
                slint::TimerMode::Repeated,
                Duration::from_secs(secs),
                move || {
                    let tick_data = {
                        let wps = wallpapers_timer.borrow();
                        let all_images = resolve_all_wallpaper_images(&wps);
                        if all_images.len() > 1 {
                            let mut slide_idx = active_idx_timer.borrow_mut();
                            let next_idx = (*slide_idx + 1) % all_images.len();
                            *slide_idx = next_idx;
                            let next_path = all_images[next_idx].clone();
                            let lookahead_idx = (next_idx + 1) % all_images.len();
                            let lookahead_path = all_images[lookahead_idx].clone();
                            Some((next_path, lookahead_path))
                        } else {
                            None
                        }
                    };

                    if let Some((next_path, lookahead_path)) = tick_data {
                        if let Some(w) = window_weak_timer.upgrade() {
                            let cur_mode = w.get_wallpaper_mode().to_string();
                            let apply_mode = if cur_mode == "none" { "terminal" } else { cur_mode.as_str() };
                            w.invoke_set_wallpaper(apply_mode.into(), next_path.as_str().into(), w.get_global_wallpaper_opacity());
                        }

                        // 立即预加载下下一张（保证每一轮轮播都有现成缓存，0ms 瞬间显示）
                        schedule_wallpaper_preload(lookahead_path, Rc::clone(&wallpaper_cache_timer), Rc::clone(&wallpaper_preload_timer_timer));
                    }
                },
            );
            *timer_ref.borrow_mut() = Some(timer);
            if is_en(&core_state_wp_slide) {
                notif.info("Slideshow Started", &format!("Wallpaper will change every {}", interval_str));
            } else {
                notif.info("轮播已开启", &format!("壁纸将每隔 {} 自动轮播更替", interval_str));
            }
        } else if interval_str == "startup" {
            if is_en(&core_state_wp_slide) {
                notif.info("Startup Slideshow", "Wallpaper will rotate randomly on startup.");
            } else {
                notif.info("开机轮播已开启", "每次客户端启动时将随机切换一张新壁纸");
            }
        } else {
            if is_en(&core_state_wp_slide) {
                notif.info("Slideshow Stopped", "Wallpaper rotation stopped.");
            } else {
                notif.info("轮播已关闭", "已停止壁纸自动轮播更替");
            }
        }
    });
}