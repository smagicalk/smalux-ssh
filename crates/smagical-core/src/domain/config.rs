//! 全局应用配置与偏好领域模型 (App Configuration & Preferences Domain Model)

use serde::{Deserialize, Serialize};

/// 全局偏好配置持久化实体
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppConfigRecord {
    /// 界面当前显示语言 (如 "zh-CN", "en-US")
    pub language: String,
    /// 软件启动时默认进入的视口 ("terminal", "hosts", "settings")
    pub startup_view: String,
    /// 主窗口关闭按钮行为 ("tray", "confirm", "exit")
    pub close_action: String,
    /// 开机随系统自动启动
    pub start_on_boot: bool,
    /// 关闭单个标签页时二次确认
    pub confirm_close_tab: bool,
    /// 关闭存在活跃运行中命令会话的标签页时确认
    pub confirm_close_active: bool,
    /// 自定义资产数据存放物理目录
    pub custom_data_dir: String,

    /// 当前激活生效的 UI 配色主题 ID (如 "builtin.ui.darcula")
    pub theme_id: String,
    /// 是否处于暗色深色调模式
    pub is_dark_mode: bool,
    /// 背景壁纸渲染透底模式 ("none", "terminal", "global")
    pub wallpaper_mode: String,
    /// 当前激活的单张壁纸绝对路径
    pub wallpaper_path: String,
    /// 壁纸图片画廊集合路径列表
    pub wallpaper_list: Vec<String>,
    /// 当前激活壁纸在画廊中的索引下标
    pub wallpaper_active_index: usize,
    /// 全局背景壁纸透明度 (0.0 ~ 1.0)
    pub wallpaper_opacity: f32,
    /// 壁纸画廊轮播切换时间间隔 ("off", "5m", "15m", "1h", "startup")
    pub wallpaper_slideshow_interval: String,
    /// 壁纸轮播动效过渡模式 ("fade", "slide", "blur")
    pub wallpaper_transition_effect: String,
    /// 界面全局字体名称 (如 "Microsoft YaHei UI", "Inter")
    #[serde(default = "default_ui_font")]
    pub ui_font: String,

    /// 终端使用的等宽字体名称
    pub font_family: String,
    /// 终端字符字号大小 (点阵磅值)
    pub font_size: f32,
    /// 终端文本垂直排版行高比例因子 (通常 1.0 ~ 2.0)
    pub line_height: f32,
    /// 终端光标外观样式 ("block", "beam", "underline")
    pub cursor_style: String,
    /// 终端光标是否启用周期性闪烁
    pub cursor_blink: bool,
    /// 终端视口缓冲区最大可回滚行数
    pub scrollback_lines: usize,
    /// 终端划词选中时自动复制至系统剪贴板
    pub copy_on_select: bool,
    /// 终端视口内点击鼠标右键直接粘贴剪贴板内容
    pub paste_on_right_click: bool,
    /// 粘贴包含换行符的多行指令前向用户弹出确认弹窗
    pub warn_on_multiline_paste: bool,
    /// 终端识别 URL 并支持点击在默认浏览器打开
    #[serde(default = "default_true")]
    pub terminal_url_click: bool,
    /// 终端关键诊断与语法智能高亮
    #[serde(default = "default_true")]
    pub terminal_highlight_keywords: bool,
    /// 终端自定义高亮关键字 (逗号分隔)
    #[serde(default = "default_custom_keywords")]
    pub terminal_custom_keywords: String,
    /// 终端蜂鸣告警模式 ("visual", "audible", "none")
    #[serde(default = "default_bell_style")]
    pub terminal_bell_style: String,

    /// SSH 连接默认端口号 (默认为 22)
    pub default_ssh_port: u16,
    /// 网络建立与握手超时时间 (秒)
    pub ssh_timeout_seconds: u32,
    /// SSH 链路保活心跳发送周期 (秒)
    pub keepalive_interval: u32,
    /// 心跳连续超时丢包判定断开的最大重试次数
    pub keepalive_count_max: u32,
    /// 主机公钥指纹安全检查严格程度 ("accept-new", "strict", "off")
    pub host_key_checking: String,

    /// 开发者调试控制台启用开关 (F12)
    pub debug_enabled: bool,
    /// 全局日志输出等级过滤阈值 ("TRACE", "DEBUG", "INFO", "WARN", "ERROR")
    pub log_level: String,
    /// 系统级桌面托盘气泡通知实验特性开关
    pub flag_desktop_notifications: bool,
    /// 终端复古 CRT 扫描线着色器实验特性开关
    pub flag_terminal_crt_shader: bool,
    /// 云端多端加密备份与同步实验特性开关
    pub flag_cloud_sync: bool,
    /// 终端临时划词便签划词板实验特性开关
    pub flag_terminal_scratchpad: bool,
}

impl Default for AppConfigRecord {
    fn default() -> Self {
        Self {
            // 通用
            language: "zh-CN".to_string(),
            startup_view: "terminal".to_string(),
            close_action: "tray".to_string(),
            start_on_boot: false,
            confirm_close_tab: true,
            confirm_close_active: true,
            custom_data_dir: String::new(),

            // 外观
            theme_id: "builtin.ui.darcula".to_string(),
            is_dark_mode: true,
            wallpaper_mode: "none".to_string(),
            wallpaper_path: String::new(),
            wallpaper_list: Vec::new(),
            wallpaper_active_index: 0,
            wallpaper_opacity: 0.20,
            wallpaper_slideshow_interval: "off".to_string(),
            wallpaper_transition_effect: "fade".to_string(),
            ui_font: default_ui_font(),

            // 终端
            font_family: "JetBrains Mono".to_string(),
            font_size: 13.0,
            line_height: 1.2,
            cursor_style: "block".to_string(),
            cursor_blink: true,
            scrollback_lines: 10000,
            copy_on_select: false,
            paste_on_right_click: true,
            warn_on_multiline_paste: true,
            terminal_url_click: true,
            terminal_highlight_keywords: true,
            terminal_custom_keywords: default_custom_keywords(),
            terminal_bell_style: default_bell_style(),

            // 网络
            default_ssh_port: 22,
            ssh_timeout_seconds: 30,
            keepalive_interval: 30,
            keepalive_count_max: 3,
            host_key_checking: "accept-new".to_string(),

            // 调试与特性门控
            debug_enabled: true,
            log_level: "INFO".to_string(),
            flag_desktop_notifications: false,
            flag_terminal_crt_shader: false,
            flag_cloud_sync: false,
            flag_terminal_scratchpad: false,
        }
    }
}

fn default_ui_font() -> String {
    "系统默认 (System Default)".to_string()
}

fn default_true() -> bool {
    true
}

fn default_custom_keywords() -> String {
    "error,failed,fatal,warning,warn,success,ok,done".to_string()
}

fn default_bell_style() -> String {
    "visual".to_string()
}