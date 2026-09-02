//! 网络隧道、代理与跳板机核心领域实体模型。
//!
//! 统一涵盖端口转发 (Local -L, Remote -R, Dynamic -D)、跳板堡垒机 (ProxyJump -J) 与外置代理池 (SOCKS5, HTTP)。

use serde::{Deserialize, Serialize};

/// 隧道与网络连接大类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TunnelType {
    /// 本地端口转发 (-L): 本地监听 -> 经 SSH 会话 -> 转发至远程目标服务 (如本地直连内网 MySQL/Redis)
    Local,
    /// 远程反向端口转发 (-R): 远程监听 -> 经 SSH 会话回传 -> 转发至本地服务 (内网穿透)
    Remote,
    /// 动态端口转发 (-D / SOCKS5): 本地监听为标准 SOCKS5 代理网关
    Dynamic,
    /// 反向动态端口转发 (-R / 远端 SOCKS5 网关): 远端监听为标准 SOCKS5 代理网关 (OpenSSH 7.6+)
    ReverseDynamic,
    /// 跳板机 / 堡垒机 (-J / ProxyJump): 多级串联跳板拓扑节点
    JumpHost,
    /// 外置代理服务器 (HTTP / HTTPS / SOCKS5)
    ProxyServer,
}

impl TunnelType {
    /// 获取隧道类型标准字符串标识 (如 "Local", "Remote", "Dynamic", "ReverseDynamic", "JumpHost", "ProxyServer")
    pub fn as_str(&self) -> &'static str {
        match self {
            TunnelType::Local => "Local",
            TunnelType::Remote => "Remote",
            TunnelType::Dynamic => "Dynamic",
            TunnelType::ReverseDynamic => "ReverseDynamic",
            TunnelType::JumpHost => "JumpHost",
            TunnelType::ProxyServer => "ProxyServer",
        }
    }

    /// 获取界面显示的标志文本徽章 (如 "LOCAL -L", "REMOTE -R", "SOCKS5 -D", "SOCKS5 -R", "BASTION -J")
    pub fn display_badge(&self) -> &'static str {
        match self {
            TunnelType::Local => "LOCAL -L",
            TunnelType::Remote => "REMOTE -R",
            TunnelType::Dynamic => "SOCKS5 -D",
            TunnelType::ReverseDynamic => "SOCKS5 -R",
            TunnelType::JumpHost => "BASTION -J",
            TunnelType::ProxyServer => "PROXY",
        }
    }
}

impl std::fmt::Display for TunnelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for TunnelType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "local" | "local -l" => Ok(TunnelType::Local),
            "remote" | "remote -r" => Ok(TunnelType::Remote),
            "dynamic" | "socks5" | "dynamic -d" | "socks5 -d" => Ok(TunnelType::Dynamic),
            "reversedynamic" | "reverse-dynamic" | "rev-socks" | "socks5 -r" => Ok(TunnelType::ReverseDynamic),
            "jumphost" | "jump" | "bastion" | "bastion -j" => Ok(TunnelType::JumpHost),
            "proxyserver" | "proxy" | "http" => Ok(TunnelType::ProxyServer),
            _ => Ok(TunnelType::Local),
        }
    }
}

/// 跳板机级联节点记录 (支持多跳串联 ProxyJump)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JumpHopRecord {
    /// 关联的主机资产 ID
    pub host_id: String,
    /// 主机名称 (展示用)
    pub host_name: String,
    /// 主机 IP 地址或域名
    pub host_address: String,
    /// SSH 服务端口
    pub host_port: u16,
    /// 是否启用该节点参与跳板链路
    pub enabled: bool,
}

/// 隧道与网络连接记录
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TunnelRecord {
    /// 唯一标识 ID (如 "tun-101")
    pub id: String,
    /// 可读名称 (如 "生产 MySQL 数据库直连")
    pub name: String,
    /// 隧道类型
    pub tunnel_type: TunnelType,
    /// 关联的 SSH 宿主主机 ID (如果是 JumpHost/Proxy 可为空或关联网关)
    pub ssh_host_id: Option<String>,
    /// 关联的 SSH 宿主主机名称 (展示冗余)
    pub ssh_host_name: String,
    /// 本地监听/绑定地址 (默认 "127.0.0.1", 局域网共享可设 "0.0.0.0")
    pub local_bind: String,
    /// 本地监听端口 (如 3306)
    pub local_port: u16,
    /// 远程目标主机/域名 (如 "192.168.1.50")
    pub remote_host: String,
    /// 远程目标端口 (如 3306)
    pub remote_port: u16,
    /// 跳板机级联节点列表 (按 Hop 1 -> Hop 2 -> Hop 3 顺序串联)
    #[serde(default)]
    pub jump_chain: Vec<JumpHopRecord>,
    /// 当前是否处于激活运行状态
    pub is_running: bool,
    /// 应用启动时是否静默自启
    pub auto_start: bool,
    /// 断线是否自动重连 (指数退避)
    pub auto_reconnect: bool,
    /// SOCKS5 模式下是否由远程解析 DNS (避免本地 DNS 污染/泄漏)
    pub remote_dns: bool,
    /// 启用传输压缩 (-C)
    pub compression: bool,
    /// 当前活跃 TCP 连接数
    pub active_connections: usize,
    /// 累计接收入站流量 (Bytes)
    pub total_bytes_in: u64,
    /// 累计发送出站流量 (Bytes)
    pub total_bytes_out: u64,
    /// 代理协议类型 (SOCKS5 / HTTP / HTTPS)
    #[serde(default)]
    pub proxy_proto: String,
    /// 代理身份认证用户名 (可选，留空为无需认证)
    #[serde(default)]
    pub proxy_username: String,
    /// 代理身份认证密码 (可选)
    #[serde(default)]
    pub proxy_password: String,
    /// 备注说明
    pub notes: String,
    /// 最近更新时间说明 (如 "2026-09-02 09:30")
    pub updated_at: String,
}

impl TunnelRecord {
    /// 格式化流量字节数字符串 (例如 "12.4 MB")
    pub fn format_bytes(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if bytes >= GB {
            format!("{:.2} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.1} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} B", bytes)
        }
    }

    /// 获取入站和出站流量的可读表示
    pub fn formatted_traffic(&self) -> (String, String) {
        (Self::format_bytes(self.total_bytes_in), Self::format_bytes(self.total_bytes_out))
    }

    /// 生成可视化路径摘要 (例如 "127.0.0.1:3306 -> 192.168.1.50:3306")
    pub fn route_summary(&self) -> String {
        match self.tunnel_type {
            TunnelType::Local => {
                format!("{}:{} -> {}:{}", self.local_bind, self.local_port, self.remote_host, self.remote_port)
            }
            TunnelType::Remote => {
                format!("{}:{} -> {}:{}", self.remote_host, self.remote_port, self.local_bind, self.local_port)
            }
            TunnelType::Dynamic => {
                format!("{}:{} (SOCKS5 代理网关)", self.local_bind, self.local_port)
            }
            TunnelType::ReverseDynamic => {
                format!("{}:{} (远端 SOCKS5 代理网关)", self.remote_host, self.remote_port)
            }
            TunnelType::JumpHost => {
                let active_hops: Vec<String> = self.jump_chain.iter()
                    .filter(|h| h.enabled)
                    .map(|h| h.host_name.clone())
                    .collect();
                if active_hops.is_empty() {
                    if !self.remote_host.is_empty() {
                        format!("ProxyJump: {}", self.remote_host)
                    } else {
                        "ProxyJump: 未配置跳板节点".to_string()
                    }
                } else {
                    format!("ProxyJump: {}", active_hops.join(" -> "))
                }
            }
            TunnelType::ProxyServer => {
                let proto = if self.proxy_proto.is_empty() { "SOCKS5".to_string() } else { self.proxy_proto.to_uppercase() };
                let auth_info = if !self.proxy_username.is_empty() {
                    format!(" (认证: {})", self.proxy_username)
                } else {
                    " (无需认证)".to_string()
                };
                format!("{} -> {}:{}{}", proto, self.remote_host, self.remote_port, auth_info)
            }
        }
    }

    /// 生成原生 OpenSSH 命令 (便于用户复制或在终端中手动测试)
    pub fn generate_ssh_command(&self, host_address: &str, host_user: &str) -> String {
        let user_part = if host_user.is_empty() {
            "root".to_string()
        } else {
            host_user.to_string()
        };
        let host_part = if host_address.is_empty() {
            "remote-host"
        } else {
            host_address
        };

        let mut flags = Vec::new();
        if self.compression {
            flags.push("-C");
        }

        match self.tunnel_type {
            TunnelType::Local => {
                format!("ssh {} -N -L {}:{}:{} {}@{}", flags.join(" "), self.local_port, self.remote_host, self.remote_port, user_part, host_part).trim().to_string()
            }
            TunnelType::Remote => {
                format!("ssh {} -N -R {}:{}:{} {}@{}", flags.join(" "), self.remote_port, self.local_bind, self.local_port, user_part, host_part).trim().to_string()
            }
            TunnelType::Dynamic => {
                format!("ssh {} -N -D {}:{} {}@{}", flags.join(" "), self.local_bind, self.local_port, user_part, host_part).trim().to_string()
            }
            TunnelType::ReverseDynamic => {
                format!("ssh {} -N -R {} {}@{}", flags.join(" "), self.remote_port, user_part, host_part).trim().to_string()
            }
            TunnelType::JumpHost => {
                let active_hops: Vec<String> = self.jump_chain.iter()
                    .filter(|h| h.enabled)
                    .map(|h| {
                        let addr = if h.host_address.is_empty() { h.host_name.as_str() } else { h.host_address.as_str() };
                        if h.host_port == 22 || h.host_port == 0 {
                            addr.to_string()
                        } else {
                            format!("{}:{}", addr, h.host_port)
                        }
                    })
                    .collect();
                let jump_param = if !active_hops.is_empty() {
                    active_hops.join(",")
                } else if !self.remote_host.is_empty() {
                    self.remote_host.clone()
                } else {
                    "<jump-hosts>".to_string()
                };
                format!("ssh -J {} target-user@target-host", jump_param)
            }
            TunnelType::ProxyServer => {
                let proto = if self.proxy_proto.is_empty() { "socks5" } else { self.proxy_proto.as_str() }.to_lowercase();
                let auth_part = if !self.proxy_username.is_empty() {
                    if !self.proxy_password.is_empty() {
                        format!("{}:{}@", self.proxy_username, self.proxy_password)
                    } else {
                        format!("{}@", self.proxy_username)
                    }
                } else {
                    String::new()
                };
                if proto == "http" || proto == "https" {
                    format!("export http_proxy=http://{}{}:{} && export https_proxy=http://{}{}:{}", auth_part, self.remote_host, self.remote_port, auth_part, self.remote_host, self.remote_port)
                } else {
                    format!("ALL_PROXY={}://{}{}:{} ssh {}@{}", proto, auth_part, self.remote_host, self.remote_port, user_part, host_part)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tunnel_type_parsing_and_display() {
        assert_eq!(TunnelType::Local.display_badge(), "LOCAL -L");
        assert_eq!(TunnelType::Remote.display_badge(), "REMOTE -R");
        assert_eq!(TunnelType::Dynamic.display_badge(), "SOCKS5 -D");
        assert_eq!(TunnelType::ReverseDynamic.display_badge(), "SOCKS5 -R");
        assert_eq!(TunnelType::JumpHost.display_badge(), "BASTION -J");

        assert_eq!("local".parse::<TunnelType>().unwrap(), TunnelType::Local);
        assert_eq!("remote".parse::<TunnelType>().unwrap(), TunnelType::Remote);
        assert_eq!("socks5".parse::<TunnelType>().unwrap(), TunnelType::Dynamic);
        assert_eq!("reversedynamic".parse::<TunnelType>().unwrap(), TunnelType::ReverseDynamic);
        assert_eq!("jump".parse::<TunnelType>().unwrap(), TunnelType::JumpHost);
    }

    #[test]
    fn test_format_traffic_and_routes() {
        let tun = TunnelRecord {
            id: "tun-1".to_string(),
            name: "MySQL 隧道".to_string(),
            tunnel_type: TunnelType::Local,
            ssh_host_id: Some("host-1".to_string()),
            ssh_host_name: "Bastion 1".to_string(),
            local_bind: "127.0.0.1".to_string(),
            local_port: 3306,
            remote_host: "10.0.0.8".to_string(),
            remote_port: 3306,
            jump_chain: Vec::new(),
            is_running: true,
            auto_start: true,
            auto_reconnect: true,
            remote_dns: false,
            compression: true,
            active_connections: 5,
            total_bytes_in: 1024 * 1024 * 15,
            total_bytes_out: 1024 * 512,
            proxy_proto: String::new(),
            proxy_username: String::new(),
            proxy_password: String::new(),
            notes: "用于生产内网库直连".to_string(),
            updated_at: "2026-09-02".to_string(),
        };

        let (in_str, out_str) = tun.formatted_traffic();
        assert_eq!(in_str, "15.00 MB");
        assert_eq!(out_str, "512.0 KB");
        assert_eq!(tun.route_summary(), "127.0.0.1:3306 -> 10.0.0.8:3306");

        let cmd = tun.generate_ssh_command("bastion.corp.com", "ubuntu");
        assert!(cmd.contains("-L 3306:10.0.0.8:3306"));
        assert!(cmd.contains("ubuntu@bastion.corp.com"));
    }

    #[test]
    fn test_multihop_jump_chain() {
        let jump = TunnelRecord {
            id: "tun-jump".to_string(),
            name: "三级跳板链路".to_string(),
            tunnel_type: TunnelType::JumpHost,
            ssh_host_id: None,
            ssh_host_name: "".to_string(),
            local_bind: "".to_string(),
            local_port: 0,
            remote_host: "".to_string(),
            remote_port: 0,
            jump_chain: vec![
                JumpHopRecord {
                    host_id: "h1".to_string(),
                    host_name: "香港跳板".to_string(),
                    host_address: "hk.bastion.com".to_string(),
                    host_port: 22,
                    enabled: true,
                },
                JumpHopRecord {
                    host_id: "h2".to_string(),
                    host_name: "内网中转".to_string(),
                    host_address: "10.0.1.5".to_string(),
                    host_port: 2222,
                    enabled: true,
                },
                JumpHopRecord {
                    host_id: "h3".to_string(),
                    host_name: "备用中继".to_string(),
                    host_address: "10.0.1.6".to_string(),
                    host_port: 22,
                    enabled: false,
                },
            ],
            is_running: true,
            auto_start: true,
            auto_reconnect: true,
            remote_dns: false,
            compression: false,
            active_connections: 0,
            total_bytes_in: 0,
            total_bytes_out: 0,
            proxy_proto: String::new(),
            proxy_username: String::new(),
            proxy_password: String::new(),
            notes: "".to_string(),
            updated_at: "2026-09-02".to_string(),
        };

        assert_eq!(jump.route_summary(), "ProxyJump: 香港跳板 -> 内网中转");
        let cmd = jump.generate_ssh_command("", "");
        assert_eq!(cmd, "ssh -J hk.bastion.com,10.0.1.5:2222 target-user@target-host");
    }

    #[test]
    fn test_proxyserver_routes_and_auth() {
        let proxy_no_auth = TunnelRecord {
            id: "tun-proxy-1".to_string(),
            name: "公司 SOCKS5".to_string(),
            tunnel_type: TunnelType::ProxyServer,
            ssh_host_id: None,
            ssh_host_name: "".to_string(),
            local_bind: "127.0.0.1".to_string(),
            local_port: 1080,
            remote_host: "192.168.1.100".to_string(),
            remote_port: 1080,
            jump_chain: Vec::new(),
            is_running: true,
            auto_start: false,
            auto_reconnect: true,
            remote_dns: true,
            compression: false,
            active_connections: 0,
            total_bytes_in: 0,
            total_bytes_out: 0,
            proxy_proto: "SOCKS5".to_string(),
            proxy_username: "".to_string(),
            proxy_password: "".to_string(),
            notes: "".to_string(),
            updated_at: "2026-09-02".to_string(),
        };
        assert_eq!(proxy_no_auth.route_summary(), "SOCKS5 -> 192.168.1.100:1080 (无需认证)");

        let proxy_with_auth = TunnelRecord {
            id: "tun-proxy-2".to_string(),
            name: "认证代理".to_string(),
            tunnel_type: TunnelType::ProxyServer,
            ssh_host_id: None,
            ssh_host_name: "".to_string(),
            local_bind: "127.0.0.1".to_string(),
            local_port: 7890,
            remote_host: "10.0.0.1".to_string(),
            remote_port: 7890,
            jump_chain: Vec::new(),
            is_running: true,
            auto_start: false,
            auto_reconnect: true,
            remote_dns: true,
            compression: false,
            active_connections: 0,
            total_bytes_in: 0,
            total_bytes_out: 0,
            proxy_proto: "HTTP".to_string(),
            proxy_username: "admin".to_string(),
            proxy_password: "pass".to_string(),
            notes: "".to_string(),
            updated_at: "2026-09-02".to_string(),
        };
        assert_eq!(proxy_with_auth.route_summary(), "HTTP -> 10.0.0.1:7890 (认证: admin)");
    }
}
