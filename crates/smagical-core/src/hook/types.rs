//! 终端 Hook 体系核心数据结构与上下文模型。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

/// 主机资产与机器环境元数据模型。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct HostMetadata {
    /// 主机资产唯一标识 ID (如: "host-prod-k8s-01")
    pub host_id: String,
    /// 主机自定义展示名 (如: "生产核心-K8s主节点")
    pub host_name: String,
    /// 资产分组树路径 (如: "生产环境 / 核心集群 / 华东区")
    pub group_path: String,
    /// 资产标签分类集合 (如: ["prod", "k8s", "gpu"])
    pub tags: Vec<String>,
    /// 是否属于生产核心高危机器 (用于强化拦截策略)
    pub is_production: bool,

    /// 连接地址 (IP 或 域名，如 "192.168.1.100")
    pub address: String,
    /// 端口号 (如 22)
    pub port: u16,
    /// 连接协议类型 ("ssh" | "local_shell" | "serial" | "telnet")
    pub protocol: String,
    /// 跳板机 / 代理链路信息
    pub proxy_chain: Option<String>,

    /// 登录用户名 (如 "root" / "ubuntu")
    pub username: String,
    /// 认证模式 ("password" | "public_key" | "agent" | "certificate")
    pub auth_type: String,

    /// 远端操作系统类型 ("Linux" | "Windows" | "macOS" | "FreeBSD" | "Unknown")
    pub os_type: String,
    /// 发行版详细信息 (如 "Ubuntu 22.04.4 LTS", "CentOS Stream 9")
    pub os_distro: Option<String>,
    /// 远端 CPU 架构 ("x86_64" | "aarch64")
    pub architecture: Option<String>,
    /// 远端主机名 hostname (如 "k8s-master-node-01")
    pub remote_hostname: Option<String>,
    /// SSH 服务端公钥指纹 (如 "SHA256:...")
    pub host_key_fingerprint: Option<String>,

    /// 扩展自定义元数据键值对
    pub extra: HashMap<String, String>,
}

impl HostMetadata {
    /// 创建一个基础的本地 Shell 主机元数据。
    pub fn local_shell(session_name: &str) -> Self {
        Self {
            host_id: "host-local-shell".to_string(),
            host_name: session_name.to_string(),
            group_path: "本地终端".to_string(),
            tags: vec!["local".to_string()],
            is_production: false,
            address: "127.0.0.1".to_string(),
            port: 0,
            protocol: "local_shell".to_string(),
            proxy_chain: None,
            username: whoami(),
            auth_type: "none".to_string(),
            os_type: std::env::consts::OS.to_string(),
            os_distro: None,
            architecture: Some(std::env::consts::ARCH.to_string()),
            remote_hostname: None,
            host_key_fingerprint: None,
            extra: HashMap::new(),
        }
    }

    /// 快速构造远端 SSH 主机元数据。
    pub fn remote_ssh(
        host_id: impl Into<String>,
        host_name: impl Into<String>,
        address: impl Into<String>,
        port: u16,
        username: impl Into<String>,
    ) -> Self {
        Self {
            host_id: host_id.into(),
            host_name: host_name.into(),
            group_path: "默认分组".to_string(),
            tags: Vec::new(),
            is_production: false,
            address: address.into(),
            port,
            protocol: "ssh".to_string(),
            proxy_chain: None,
            username: username.into(),
            auth_type: "password".to_string(),
            os_type: "Linux".to_string(),
            os_distro: None,
            architecture: None,
            remote_hostname: None,
            host_key_fingerprint: None,
            extra: HashMap::new(),
        }
    }
}

fn whoami() -> String {
    std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "current_user".to_string())
}

/// 终端会话运行时全局上下文。
#[derive(Debug, Clone)]
pub struct SessionContext {
    /// 会话唯一 ID (如: "sess-8f3a-20260831")
    pub session_id: String,
    /// UI 窗格 ID (如: "pane-1")
    pub pane_id: String,
    /// 绑定的主机元数据 (线程安全共享引用)
    pub host: Arc<HostMetadata>,
    /// 会话启动时间
    pub created_at: SystemTime,
    /// 本地客户端操作系统环境 ("windows" / "linux" / "macos")
    pub client_platform: &'static str,
}

impl SessionContext {
    /// 创建新的会话上下文。
    pub fn new(session_id: impl Into<String>, pane_id: impl Into<String>, host: HostMetadata) -> Self {
        Self {
            session_id: session_id.into(),
            pane_id: pane_id.into(),
            host: Arc::new(host),
            created_at: SystemTime::now(),
            client_platform: std::env::consts::OS,
        }
    }
}

/// 命令触发来源。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CommandSource {
    /// 用户键盘直接输入
    Keyboard,
    /// 快捷命令片段宏展开 (Snippet)
    Snippet,
    /// AI 运维助手推荐执行
    AiCopilot,
    /// 自动化脚本或计划任务
    Script,
}

/// 命令交互追踪帧执行状态。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FrameStatus {
    /// 命令正在执行并持续接收输出流
    Running,
    /// 命令已正常执行完毕
    Completed,
    /// 命令执行异常或中途退出 (包含错误信息)
    Failed(String),
    /// 被前置安全 Hook 拦截并阻断
    Blocked(String),
}

/// 终端命令-输出交互时序追踪帧模型。
#[derive(Debug, Clone)]
pub struct CommandInteractionFrame {
    /// 链路追踪全局唯一 ID (如: "trace-sess01-seq0042")
    pub trace_id: String,
    /// 窗格内的自增命令序号 (1, 2, 3...)
    pub seq_id: u64,
    /// 关联的完整会话上下文
    pub session: SessionContext,

    /// 发送的命令文本 (如 "systemctl restart nginx")
    pub command_line: String,
    /// 命令开始时间
    pub started_at: SystemTime,
    /// 命令触发源
    pub source: CommandSource,

    /// 对应的远端原始输出字节流
    pub output_raw: Vec<u8>,
    /// 剥离或解析后的输出文本
    pub output_text: String,
    /// 输出行数统计
    pub output_lines: usize,

    /// 首字节返回延迟 TTFB (Time to First Byte)
    pub ttfb: Option<Duration>,
    /// 命令总执行耗时
    pub duration: Duration,
    /// 进程退出状态码 (若可用)
    pub exit_code: Option<i32>,
    /// 当前帧状态
    pub status: FrameStatus,
}

impl CommandInteractionFrame {
    /// 初始化新建一个正在运行的命令交互追踪帧。
    pub fn new(
        seq_id: u64,
        session: SessionContext,
        command_line: impl Into<String>,
        source: CommandSource,
    ) -> Self {
        let trace_id = format!("trace-{}-seq{:05}", session.session_id, seq_id);
        Self {
            trace_id,
            seq_id,
            session,
            command_line: command_line.into(),
            started_at: SystemTime::now(),
            source,
            output_raw: Vec::new(),
            output_text: String::new(),
            output_lines: 0,
            ttfb: None,
            duration: Duration::ZERO,
            exit_code: None,
            status: FrameStatus::Running,
        }
    }

    /// 向追踪帧中追加远端输出数据块。
    pub fn append_output(&mut self, chunk: &[u8]) {
        if self.ttfb.is_none() {
            self.ttfb = self.started_at.elapsed().ok();
        }
        self.output_raw.extend_from_slice(chunk);
        let text_chunk = String::from_utf8_lossy(chunk);
        self.output_lines += text_chunk.matches('\n').count();
        self.output_text.push_str(&text_chunk);
    }

    /// 标记命令执行完成。
    pub fn mark_completed(&mut self, exit_code: Option<i32>) {
        self.duration = self.started_at.elapsed().unwrap_or(Duration::ZERO);
        self.exit_code = exit_code;
        self.status = match exit_code {
            Some(code) if code != 0 => FrameStatus::Failed(format!("Exit code {}", code)),
            _ => FrameStatus::Completed,
        };
    }

    /// 标记命令执行失败。
    pub fn mark_failed(&mut self, reason: impl Into<String>) {
        self.duration = self.started_at.elapsed().unwrap_or(Duration::ZERO);
        self.status = FrameStatus::Failed(reason.into());
    }

    /// 标记命令被前置拦截。
    pub fn mark_blocked(&mut self, reason: impl Into<String>) {
        self.duration = Duration::ZERO;
        self.status = FrameStatus::Blocked(reason.into());
    }
}
