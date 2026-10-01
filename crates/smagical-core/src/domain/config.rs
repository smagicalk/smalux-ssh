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
    /// 全局操作提示气泡停留时长 ("1.5", "3", "5", "8", "never")，默认 "3"
    #[serde(default = "default_toast_duration")]
    pub toast_duration: String,
    /// 主窗口始终置顶显示
    #[serde(default)]
    pub always_on_top: bool,

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
    /// 启用链路层 gzip 数据流压缩 (ssh -C)
    #[serde(default)]
    pub compression: bool,
    /// 兼容遗留设备老旧加密算法 (Legacy Ciphers Fallback)
    #[serde(default)]
    pub legacy_ciphers: bool,
    /// 网络异常断线自动重连 (Auto Reconnect)
    #[serde(default = "default_true")]
    pub auto_reconnect: bool,

    // --- 传输与文件管理 (SFTP & File Transfers) ---
    /// 默认本地下载存储目录
    #[serde(default = "default_sftp_local")]
    pub sftp_default_local: String,
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
    /// 显示隐藏文件与点文件
    #[serde(default)]
    pub sftp_show_hidden: bool,
    /// 文件存在重名冲突策略 ("ask", "overwrite", "skip", "rename")
    #[serde(default = "default_sftp_conflict_policy")]
    pub sftp_conflict_policy: String,

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

    // --- AI 助手与大模型管理 (AI Copilot & Endpoints) ---
    /// 当前激活的 AI 厂商 ("deepseek", "claude", "openai", "custom")
    #[serde(default = "default_ai_provider")]
    pub ai_active_provider: String,
    /// AI 系统提示词
    #[serde(default = "default_ai_system_prompt")]
    pub ai_system_prompt: String,
    /// 自动化命令安全审查等级 ("strict", "warn", "permissive")
    #[serde(default = "default_ai_audit_level")]
    pub ai_auto_audit_level: String,
    /// AI 端点列表
    #[serde(default = "default_ai_endpoints")]
    pub ai_endpoints: Vec<AiEndpointProfileRecord>,

    // --- 自定义终端高亮规则 ---
    /// 自定义关键词高亮规则列表
    #[serde(default = "default_keyword_rules")]
    pub keyword_highlight_rules: Vec<KeywordHighlightRuleRecord>,

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
            confirm_close_tab: false,
            confirm_close_active: false,
            custom_data_dir: String::new(),
            toast_duration: default_toast_duration(),
            always_on_top: false,

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
            paste_on_right_click: false,
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
            compression: false,
            legacy_ciphers: false,
            auto_reconnect: true,

            // 传输与文件管理
            sftp_default_local: default_sftp_local(),
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
            sftp_show_hidden: false,
            sftp_conflict_policy: default_sftp_conflict_policy(),

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

            // AI 助手与大模型管理
            ai_active_provider: default_ai_provider(),
            ai_system_prompt: default_ai_system_prompt(),
            ai_auto_audit_level: default_ai_audit_level(),
            ai_endpoints: default_ai_endpoints(),

            // 自定义高亮规则
            keyword_highlight_rules: default_keyword_rules(),

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

fn default_sftp_local() -> String {
    #[cfg(windows)]
    {
        if let Ok(profile) = std::env::var("USERPROFILE") {
            let p = std::path::Path::new(&profile).join("Downloads");
            if p.exists() {
                return p.to_string_lossy().to_string();
            }
        }
    }
    #[cfg(not(windows))]
    {
        if let Ok(home) = std::env::var("HOME") {
            let p = std::path::Path::new(&home).join("Downloads");
            if p.exists() {
                return p.to_string_lossy().to_string();
            }
        }
    }
    "~/Downloads".to_string()
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

fn default_toast_duration() -> String {
    "3".to_string()
}

/// AI 大模型端点持久化配置实体
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AiEndpointProfileRecord {
    /// 端点唯一标识
    pub id: String,
    /// 端点友好显示名称
    pub name: String,
    /// 接口基础地址 Base URL
    pub base_url: String,
    /// API 授权鉴权密钥
    pub api_key: String,
    /// 交互协议模式 ("chat" 或 "response")
    pub api_mode: String,
    /// 默认选中推理模型
    pub selected_model: String,
    /// 可用模型列表逗号分隔字符串
    pub models_csv: String,
    /// 是否作为当前全局激活端点
    pub is_active: bool,
    /// 连接状态提示文本
    pub status_text: String,
    /// 深度思考档位 ("disabled", "low", "medium", "high")
    pub thinking_degree: String,
    /// 请求超时时长 (秒)
    pub timeout_secs: i32,
    /// 最大上下文窗口 Tokens
    pub max_context: i32,
    /// 失败重试上限次数
    pub max_retries: i32,
    /// 自定义请求头 (JSON 或 Key-Value 格式)
    pub custom_headers: String,
    /// 推理随机采样温度
    pub temperature: String,
}

/// 自定义终端关键词高亮规则记录
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeywordHighlightRuleRecord {
    /// 规则唯一标识
    pub id: String,
    /// 正则表达式匹配表达式
    pub pattern: String,
    /// 规则用途说明与备注
    pub remark: String,
    /// 前景色十六进制值 (如 "#EF4444")
    pub color_hex: String,
    /// 是否激活生效
    pub enabled: bool,
}

fn default_ai_provider() -> String {
    "deepseek".to_string()
}

fn default_ai_system_prompt() -> String {
    "你是一名精通 Linux/Unix 操作系统内核、网络拓扑与现代运维架构的高级 SRE 运维专家。遵循生产安全第一原则，始终输出语法严谨、带有防御性容错参数的 Shell 指令，主动识别与规避高危操作风险，并在生成复杂命令时简明解释其参数逻辑。".to_string()
}

fn default_ai_audit_level() -> String {
    "warn".to_string()
}

fn default_ai_endpoints() -> Vec<AiEndpointProfileRecord> {
    vec![
        AiEndpointProfileRecord {
            id: "ep-deepseek".to_string(),
            name: "DeepSeek 官方".to_string(),
            base_url: "https://api.deepseek.com/v1".to_string(),
            api_key: String::new(),
            api_mode: "chat".to_string(),
            selected_model: "deepseek-reasoner".to_string(),
            models_csv: "deepseek-reasoner, deepseek-chat".to_string(),
            is_active: true,
            status_text: "已连接".to_string(),
            thinking_degree: "medium".to_string(),
            timeout_secs: 60,
            max_context: 32768,
            max_retries: 2,
            custom_headers: String::new(),
            temperature: "0.3".to_string(),
        },
        AiEndpointProfileRecord {
            id: "ep-claude".to_string(),
            name: "Claude (Anthropic)".to_string(),
            base_url: "https://api.anthropic.com/v1".to_string(),
            api_key: String::new(),
            api_mode: "chat".to_string(),
            selected_model: "claude-3-7-sonnet-20250219".to_string(),
            models_csv: "claude-3-7-sonnet-20250219, claude-3-5-sonnet-20241022".to_string(),
            is_active: false,
            status_text: "未激活".to_string(),
            thinking_degree: "high".to_string(),
            timeout_secs: 60,
            max_context: 65536,
            max_retries: 2,
            custom_headers: String::new(),
            temperature: "0.2".to_string(),
        },
    ]
}

fn default_keyword_rules() -> Vec<KeywordHighlightRuleRecord> {
    vec![
        KeywordHighlightRuleRecord {
            id: "kw_err".to_string(),
            pattern: r"\b(ERROR|FATAL|CRITICAL|Failed|Error)\b".to_string(),
            remark: "致命错误与失败".to_string(),
            color_hex: "#EF4444".to_string(),
            enabled: true,
        },
        KeywordHighlightRuleRecord {
            id: "kw_warn".to_string(),
            pattern: r"\b(WARN|WARNING|Warning|Warn)\b".to_string(),
            remark: "告警提示与注意".to_string(),
            color_hex: "#F59E0B".to_string(),
            enabled: true,
        },
        KeywordHighlightRuleRecord {
            id: "kw_ok".to_string(),
            pattern: r"\b(SUCCESS|OK|Finished|Done)\b".to_string(),
            remark: "执行成功与确认".to_string(),
            color_hex: "#10B981".to_string(),
            enabled: true,
        },
        KeywordHighlightRuleRecord {
            id: "kw_url".to_string(),
            pattern: r"https?://[^\s/$.?#].[^\s]*".to_string(),
            remark: "网络超链接 URL".to_string(),
            color_hex: "#3B82F6".to_string(),
            enabled: true,
        },
        KeywordHighlightRuleRecord {
            id: "kw_ip".to_string(),
            pattern: r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b".to_string(),
            remark: "IPv4 主机地址".to_string(),
            color_hex: "#8B5CF6".to_string(),
            enabled: true,
        },
    ]
}

fn default_sftp_conflict_policy() -> String {
    "ask".to_string()
}