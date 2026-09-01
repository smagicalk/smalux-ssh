//! 终端活跃会话上下文与指令交互模型。

use serde::{Deserialize, Serialize};

/// 中央终端当前活跃会话上下文快照。
///
/// 当用户在中央终端切换 Tab、分屏聚焦、新建或关闭会话时，由终端管理器向外广播此快照，供右侧栏及各插件实时感知。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveTerminalSessionContext {
    /// 会话唯一 ID (如 "sess-uuid-001")
    pub session_id: String,
    /// 选项卡显示标题 (如 "prod-k8s-master-01 (192.168.1.10)")
    pub title: String,
    /// 会话建立时间戳 (UNIX 毫秒)
    pub created_at: u64,
    /// 网络与 PTY 连接状态
    pub is_connected: bool,

    // --- 主机资产元数据 (若为 SSH 会话) ---
    /// 关联的主机 ID (可选)
    pub host_id: Option<String>,
    /// 主机别名
    pub host_name: String,
    /// 目标 IP 或域名
    pub host_ip: String,
    /// 目标端口 (如 22)
    pub port: u16,
    /// 登录用户名 (如 "root")
    pub username: String,
    /// 所在资产分组路径 (如 "生产集群/核心库")
    pub group_path: String,
    /// 标签集合 (如 ["k8s", "redis", "master"])
    pub tags: Vec<String>,

    // --- 运行期动态探测上下文 ---
    /// 远端操作系统类型 (如 "Ubuntu 22.04 LTS", "CentOS 7.9")
    pub remote_os: Option<String>,
    /// 终端当前工作目录 PWD (如 "/var/log/nginx")
    pub current_pwd: Option<String>,
    /// 前台执行的活跃进程 PID
    pub active_pid: Option<u32>,
}

impl ActiveTerminalSessionContext {
    /// 创建本地 Shell 会话上下文
    pub fn local_shell(session_id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            title: title.into(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
            is_connected: true,
            host_id: None,
            host_name: "Local Shell".into(),
            host_ip: "127.0.0.1".into(),
            port: 0,
            username: std::env::var("USERNAME").or_else(|_| std::env::var("USER")).unwrap_or_else(|_| "user".into()),
            group_path: "Local".into(),
            tags: vec!["local".into()],
            remote_os: Some(std::env::consts::OS.into()),
            current_pwd: std::env::current_dir().ok().map(|p| p.to_string_lossy().to_string()),
            active_pid: None,
        }
    }

    /// 创建 SSH 远程主机上下文
    #[allow(clippy::too_many_arguments)]
    pub fn ssh(
        session_id: impl Into<String>,
        host_id: impl Into<String>,
        host_name: impl Into<String>,
        host_ip: impl Into<String>,
        port: u16,
        username: impl Into<String>,
        group_path: impl Into<String>,
        tags: Vec<String>,
    ) -> Self {

        let h_name = host_name.into();
        let ip = host_ip.into();
        let title = format!("{} ({}:{})", h_name, ip, port);
        Self {
            session_id: session_id.into(),
            title,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
            is_connected: true,
            host_id: Some(host_id.into()),
            host_name: h_name,
            host_ip: ip,
            port,
            username: username.into(),
            group_path: group_path.into(),
            tags,
            remote_os: None,
            current_pwd: None,
            active_pid: None,
        }
    }
}

/// 右侧栏或外部插件向中央终端请求执行的动作
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerminalAction {
    /// 注入字符流并自动追加回车执行
    ExecuteCommand(String),
    /// 仅向光标所在位置粘贴文本 (不回车)
    PasteText(String),
    /// 切换工作目录
    ChangeDirectory(String),
    /// 清屏
    ClearScreen,
    /// 中断当前前台任务 (发送 Ctrl+C)
    Interrupt,
}
