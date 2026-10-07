//! 真实本地系统资产与生产级默认配置初始化模块 (Real Data Initialization)。
//!
//! 替代历史遗留的假数据 (Mock Seed Data)，在 SQLite 物理持久化模式下提供：
//! 1. 自动嗅探并导入本地真实 OpenSSH 资产 (~/.ssh/config 与 ~/.ssh/known_hosts)；
//! 2. 纯净默认主机分组 (我的主机)；
//! 3. 生产级常用运维命令片段 (Linux 监控与服务管理)；
//! 4. 彻底杜绝任何假服务器 IP (如 47.98.12.33, 192.168.1.100) 与 Mock 假凭据。

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use smagical_core::domain::{
    host::{HostRecord, HostStatus},
    snippet::{SnippetGroupRecord, SnippetRecord},
};

/// 获取用户主目录下的 .ssh 目录
pub fn get_user_ssh_dir() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|dirs| dirs.home_dir().join(".ssh"))
}

/// 解析 ~/.ssh/known_hosts 文件中的真实主机列表
pub fn parse_known_hosts_file(content: &str, parent_group_id: Option<String>) -> Vec<HostRecord> {
    let mut list = Vec::new();
    let mut seen = HashSet::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // 提取 host 字段
        let mut parts = trimmed.split_whitespace();
        let Some(host_part) = parts.next() else { continue };

        // 过滤散列化主机 (|1|...|...)
        if host_part.starts_with('|') {
            continue;
        }

        for sub in host_part.split(',') {
            let sub = sub.trim();
            if sub.is_empty() {
                continue;
            }

            let (host_addr, port) = if sub.starts_with('[') {
                if let Some(end_bracket) = sub.find(']') {
                    let addr = &sub[1..end_bracket];
                    let port = sub[end_bracket + 1..]
                        .strip_prefix(':')
                        .and_then(|p| p.parse::<u16>().ok())
                        .unwrap_or(22);
                    (addr.to_string(), port)
                } else {
                    (sub.to_string(), 22)
                }
            } else {
                (sub.to_string(), 22)
            };

            if host_addr.is_empty() || !seen.insert(format!("{}:{}", host_addr, port)) {
                continue;
            }

            let host_id = format!("known-{}", host_addr.replace(|c: char| !c.is_alphanumeric(), "-"));
            list.push(HostRecord {
                id: host_id,
                name: host_addr.clone(),
                address: host_addr,
                port,
                parent_group_id: parent_group_id.clone(),
                credential_id: None,
                status: HostStatus::Offline,
                ping_ms: 0,
                sort_order: 10,
                notes: "从系统 ~/.ssh/known_hosts 发现的真实已知主机".to_string(),
                auth_type: "credential".to_string(),
                username: Some("root".to_string()),
                ..Default::default()
            });
        }
    }

    list
}

/// 解析 ~/.ssh/config 文件中的真实主机列表
pub fn parse_ssh_config_file(content: &str, parent_group_id: Option<String>) -> Vec<HostRecord> {
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
            if let Some(h) = current_host.take() {
                if !h.name.contains('*') && !h.name.is_empty() {
                    hosts.push(h);
                }
            }

            if !val.contains('*') && !val.is_empty() {
                let host_id = format!("ssh-{}", val.replace(|c: char| !c.is_alphanumeric(), "-"));
                current_host = Some(HostRecord {
                    id: host_id,
                    name: val.to_string(),
                    address: val.to_string(),
                    port: 22,
                    parent_group_id: parent_group_id.clone(),
                    credential_id: None,
                    status: HostStatus::Offline,
                    ping_ms: 0,
                    sort_order: 5,
                    notes: "从系统 ~/.ssh/config 导入".to_string(),
                    auth_type: "credential".to_string(),
                    username: None,
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
                        h.username = Some(val.to_string());
                    }
                }
                "identityfile" => {
                    if !val.is_empty() {
                        h.notes = format!("IdentityFile: {}; {}", val, h.notes);
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

/// 扫描本机 ~/.ssh 目录并提取真实已知主机
pub fn discover_local_ssh_hosts(known_group_id: Option<String>, config_group_id: Option<String>) -> (Vec<HostRecord>, Vec<HostRecord>) {
    let mut config_hosts = Vec::new();
    let mut known_hosts = Vec::new();

    if let Some(ssh_dir) = get_user_ssh_dir() {
        let config_path = ssh_dir.join("config");
        if config_path.exists() {
            if let Ok(content) = fs::read_to_string(&config_path) {
                config_hosts = parse_ssh_config_file(&content, config_group_id);
            }
        }

        let known_path = ssh_dir.join("known_hosts");
        if known_path.exists() {
            if let Ok(content) = fs::read_to_string(&known_path) {
                known_hosts = parse_known_hosts_file(&content, known_group_id);
            }
        }
    }

    (config_hosts, known_hosts)
}

/// 生成真实开箱即用的生产运维代码片段集
pub fn generate_real_snippets() -> (Vec<SnippetGroupRecord>, Vec<SnippetRecord>) {
    let groups = vec![
        SnippetGroupRecord {
            id: "sg-ops".to_string(),
            name: "常用系统运维 (System Ops)".to_string(),
            parent_id: None,
            level: 0,
            is_expanded: true,
            sort_order: 0,
        },
        SnippetGroupRecord {
            id: "sg-net".to_string(),
            name: "网络与端口排查 (Network)".to_string(),
            parent_id: None,
            level: 0,
            is_expanded: true,
            sort_order: 1,
        },
        SnippetGroupRecord {
            id: "sg-docker".to_string(),
            name: "Docker 容器管理 (Containers)".to_string(),
            parent_id: None,
            level: 0,
            is_expanded: true,
            sort_order: 2,
        },
    ];

    let snippets = vec![
        SnippetRecord::new("snip-df", "查看磁盘空间占用", "df -h", "bash")
            .with_group("sg-ops")
            .with_description("以可读格式查看挂载磁盘分区及空间剩余")
            .with_auto_execute(false),
        SnippetRecord::new("snip-free", "查看物理内存与 Swap", "free -h", "bash")
            .with_group("sg-ops")
            .with_description("查看系统物理内存使用量与 Swap 交换区状态")
            .with_auto_execute(false),
        SnippetRecord::new("snip-top", "查看系统实时负载与 CPU", "top -b -n 1 | head -n 20", "bash")
            .with_group("sg-ops")
            .with_description("快照查看前 20 项占用 CPU 最多的进程")
            .with_auto_execute(false),
        SnippetRecord::new("snip-ports", "查看 TCP 监听端口与进程", "ss -tulnp || netstat -tulnp", "bash")
            .with_group("sg-net")
            .with_description("查看本机所有正在监听的 TCP/UDP 端口及对应程序")
            .with_auto_execute(false),
        SnippetRecord::new("snip-docker-ps", "查看活跃容器列表", "docker ps --format 'table {{.ID}}\t{{.Names}}\t{{.Status}}\t{{.Ports}}'", "bash")
            .with_group("sg-docker")
            .with_description("列出正在运行的容器名称、状态与端口映射")
            .with_auto_execute(false),
    ];

    (groups, snippets)
}
