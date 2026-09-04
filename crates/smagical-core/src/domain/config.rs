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
    /// 实验特性：壁纸模式下模态弹窗与下拉面板不透明度 (0.50 ~ 1.0，默认为 1.0 完全纯色遮挡防重叠)
    #[serde(default = "default_modal_opacity")]
    pub modal_opacity: f32,

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
    /// 全局代理模式 ("direct", "system", "custom")
    #[serde(default = "default_proxy_mode")]
    pub global_proxy_mode: String,
    /// 全局代理服务器与端口
    #[serde(default = "default_proxy_server")]
    pub global_proxy_server: String,
    /// 全局代理是否启用身份认证
    #[serde(default)]
    pub global_proxy_auth: bool,
    /// 全局代理认证用户名
    #[serde(default)]
    pub global_proxy_user: String,
    /// 全局代理认证密码
    #[serde(default)]
    pub global_proxy_pass: String,
    /// 启用 TCP_NODELAY 规避 Nagle 算法降低交互延迟
    #[serde(default = "default_true")]
    pub tcp_nodelay: bool,

    // --- 传输与文件管理 (SFTP & File Transfers) ---
    /// 默认远程初始工作目录 (如 "~" 或 "/")
    #[serde(default = "default_sftp_remote")]
    pub sftp_default_remote: String,
    /// 删除远程文件二次确认
    #[serde(default = "default_true")]
    pub sftp_confirm_delete: bool,
    /// 大文件断点续传支持
    #[serde(default = "default_true")]
    pub sftp_resume_transfer: bool,
    /// 保留原始 POSIX 权限与时间戳
    #[serde(default = "default_true")]
    pub sftp_preserve_attributes: bool,
    /// 最大并发传输连接数
    #[serde(default = "default_sftp_concurrency")]
    pub sftp_concurrency: u32,
    /// 单任务上传速率限制 ("unlimited", "1mb", "5mb", "10mb")
    #[serde(default = "default_unlimited")]
    pub sftp_upload_limit: String,
    /// 单任务下载速率限制 ("unlimited", "2mb", "10mb", "20mb")
    #[serde(default = "default_unlimited")]
    pub sftp_download_limit: String,
    /// 远程文件双击打开方式 ("builtin", "system", "custom")
    #[serde(default = "default_sftp_editor")]
    pub sftp_editor_mode: String,
    /// 自定义外部编辑器程序命令
    #[serde(default)]
    pub sftp_custom_editor: String,
    /// 传输过滤忽略黑名单
    #[serde(default = "default_sftp_excludes")]
    pub sftp_exclude_patterns: String,

    // --- 多端云同步与数据备份 (Cloud Sync & Backup Matrix) ---
    /// 云同步后端协议 ("off", "webdav", "s3", "gist", "custom")
    #[serde(default = "default_off")]
    pub cloud_sync_backend: String,
    /// WebDAV 服务器地址
    #[serde(default)]
    pub cloud_sync_webdav_url: String,
    /// WebDAV 用户名
    #[serde(default)]
    pub cloud_sync_webdav_user: String,
    /// WebDAV 密码或应用令牌
    #[serde(default)]
    pub cloud_sync_webdav_pass: String,
    /// WebDAV 远程备份子目录
    #[serde(default = "default_webdav_dir")]
    pub cloud_sync_webdav_dir: String,
    /// S3 自定义 Endpoint
    #[serde(default)]
    pub cloud_sync_s3_endpoint: String,
    /// S3 存储桶 Bucket
    #[serde(default)]
    pub cloud_sync_s3_bucket: String,
    /// S3 Access Key ID
    #[serde(default)]
    pub cloud_sync_s3_key_id: String,
    /// S3 Secret Access Key
    #[serde(default)]
    pub cloud_sync_s3_access_key: String,
    /// S3 区域 Region
    #[serde(default = "default_s3_region")]
    pub cloud_sync_s3_region: String,
    /// GitHub Gist Personal Access Token
    #[serde(default)]
    pub cloud_sync_gist_token: String,
    /// GitHub Gist ID
    #[serde(default)]
    pub cloud_sync_gist_id: String,
    /// 自建备份服务器 API 端点
    #[serde(default = "default_custom_sync_url")]
    pub cloud_sync_custom_url: String,
    /// 自建备份服务器 Bearer Token
    #[serde(default)]
    pub cloud_sync_custom_token: String,
    /// 设备客户端标识符
    #[serde(default = "default_custom_client_id")]
    pub cloud_sync_custom_client_id: String,
    /// 端到端客户端加密口令 (E2EE)
    #[serde(default)]
    pub cloud_sync_e2ee_pass: String,
    /// 自动同步调度频率 ("manual", "startup", "1h", "daily")
    #[serde(default = "default_sync_interval")]
    pub cloud_sync_interval: String,

    // --- 安全与高级诊断 (Security & Vault Protection) ---
    /// 启用本地凭据主密码保护
    #[serde(default)]
    pub master_password_enabled: bool,
    /// 空闲自动锁定超时 ("never", "5m", "15m", "30m", "1h")
    #[serde(default = "default_auto_lock")]
    pub auto_lock_timeout: String,
    /// Windows Hello / 生物识别快捷解锁
    #[serde(default)]
    pub biometric_unlock: bool,
    /// 窗口最小化或退出时立即锁定
    #[serde(default)]
    pub lock_on_minimize: bool,
    /// 敏感剪贴板自动清除 (30秒)
    #[serde(default = "default_true")]
    pub clear_clipboard_timeout: bool,
    /// 高危破坏性指令执行预警
    #[serde(default = "default_true")]
    pub confirm_dangerous_commands: bool,
    /// 会话操作安全审计日志
    #[serde(default)]
    pub session_audit_logging: bool,

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
            modal_opacity: default_modal_opacity(),

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
            global_proxy_mode: default_proxy_mode(),
            global_proxy_server: default_proxy_server(),
            global_proxy_auth: false,
            global_proxy_user: String::new(),
            global_proxy_pass: String::new(),
            tcp_nodelay: true,

            // 传输与文件管理
            sftp_default_remote: default_sftp_remote(),
            sftp_confirm_delete: true,
            sftp_resume_transfer: true,
            sftp_preserve_attributes: true,
            sftp_concurrency: default_sftp_concurrency(),
            sftp_upload_limit: default_unlimited(),
            sftp_download_limit: default_unlimited(),
            sftp_editor_mode: default_sftp_editor(),
            sftp_custom_editor: String::new(),
            sftp_exclude_patterns: default_sftp_excludes(),

            // 云同步与数据备份
            cloud_sync_backend: default_off(),
            cloud_sync_webdav_url: "https://dav.jianguoyun.com/dav/".to_string(),
            cloud_sync_webdav_user: String::new(),
            cloud_sync_webdav_pass: String::new(),
            cloud_sync_webdav_dir: default_webdav_dir(),
            cloud_sync_s3_endpoint: String::new(),
            cloud_sync_s3_bucket: String::new(),
            cloud_sync_s3_key_id: String::new(),
            cloud_sync_s3_access_key: String::new(),
            cloud_sync_s3_region: default_s3_region(),
            cloud_sync_gist_token: String::new(),
            cloud_sync_gist_id: String::new(),
            cloud_sync_custom_url: default_custom_sync_url(),
            cloud_sync_custom_token: String::new(),
            cloud_sync_custom_client_id: default_custom_client_id(),
            cloud_sync_e2ee_pass: String::new(),
            cloud_sync_interval: default_sync_interval(),

            // 安全与高级诊断
            master_password_enabled: false,
            auto_lock_timeout: default_auto_lock(),
            biometric_unlock: false,
            lock_on_minimize: false,
            clear_clipboard_timeout: true,
            confirm_dangerous_commands: true,
            session_audit_logging: false,

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

fn default_modal_opacity() -> f32 {
    1.0
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

fn default_proxy_mode() -> String {
    "direct".to_string()
}

fn default_proxy_server() -> String {
    "127.0.0.1:7890".to_string()
}

fn default_sftp_remote() -> String {
    "~".to_string()
}

fn default_sftp_concurrency() -> u32 {
    4
}

fn default_unlimited() -> String {
    "unlimited".to_string()
}

fn default_sftp_editor() -> String {
    "builtin".to_string()
}

fn default_sftp_excludes() -> String {
    ".git, .DS_Store, node_modules, __pycache__, *.tmp".to_string()
}

fn default_off() -> String {
    "off".to_string()
}

fn default_webdav_dir() -> String {
    "/smalux_backup".to_string()
}

fn default_s3_region() -> String {
    "us-east-1".to_string()
}

fn default_custom_sync_url() -> String {
    "https://api.smalux.internal/v1/sync".to_string()
}

fn default_custom_client_id() -> String {
    "desktop-client".to_string()
}

fn default_sync_interval() -> String {
    "manual".to_string()
}

fn default_auto_lock() -> String {
    "never".to_string()
}