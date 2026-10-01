//! 第三方终端资产迁移与格式导入解析器 (Third-Party Asset Importer)。
//!
//! 支持从以下主流终端格式无损识别并导入主机资产与分组层级：
//! 1. Termius JSON 导出格式 (单文件列表或 `{ "items": [...] }` / `{ "hosts": [...] }`)；
//! 2. Termius CSV / TSV 导出格式 (带表头或标准逗号/制表符分隔)；
//! 3. Xshell `.xsh` 会话配置文件 (标准 INI 格式)；
//! 4. 通用 CSV / TSV 主机资产列表格式。

use smagical_core::domain::host::{HostRecord, HostStatus};

/// 导入解析出的单条主机资产及其目标分组名称
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedHostEntry {
    /// 主机展示名称
    pub name: String,
    /// 目标主机地址或 IP
    pub address: String,
    /// SSH 服务端口
    pub port: u16,
    /// 登录用户名 (可选)
    pub username: Option<String>,
    /// 所属分组名称 (可选)
    pub group_name: Option<String>,
    /// 备注说明信息
    pub notes: String,
}

impl ImportedHostEntry {
    /// 转换为核心领域持久化记录 `HostRecord`
    pub fn into_host_record(self, parent_group_id: Option<String>) -> HostRecord {
        HostRecord {
            id: format!("host-{}", uuid::Uuid::new_v4()),
            name: self.name,
            address: self.address,
            port: if self.port == 0 { 22 } else { self.port },
            parent_group_id,
            credential_id: None,
            status: HostStatus::Offline,
            ping_ms: 0,
            sort_order: 0,
            notes: self.notes,
            auth_type: "password".to_string(),
            username: self.username,
            password: None,
            key_data: None,
            key_passphrase: None,
            proxy_type: None,
            proxy_host: None,
            proxy_port: None,
            proxy_username: None,
            proxy_password: None,
            jump_chain_summary: None,
            keepalive_interval: 30,
            connect_timeout: 15,
            initial_dir: None,
            startup_cmd: None,
            term_type: Some("xterm-256color".to_string()),
        }
    }
}

/// 自动探测文件格式并解析出主机资产列表
pub fn parse_external_assets(content: &str, file_name: Option<&str>) -> Vec<ImportedHostEntry> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    // 1. 尝试作为 JSON 解析 (Termius JSON)
    if (trimmed.starts_with('{') && trimmed.ends_with('}'))
        || (trimmed.starts_with('[') && trimmed.ends_with(']'))
    {
        if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(trimmed) {
            let res = parse_termius_json(&json_val);
            if !res.is_empty() {
                return res;
            }
        }
    }

    // 2. 尝试作为 Xshell .xsh 会话文件 (INI 格式)
    if trimmed.contains("[CONNECTION]") || trimmed.contains("[CONNECTION:AUTHENTICATION]") {
        if let Some(entry) = parse_xshell_ini(trimmed, file_name) {
            return vec![entry];
        }
    }

    // 3. 尝试作为 CSV / TSV 解析 (Termius CSV 或通用资产表格)
    parse_delimited_table(trimmed)
}

/// 解析 Termius JSON 格式
fn parse_termius_json(val: &serde_json::Value) -> Vec<ImportedHostEntry> {
    let mut list = Vec::new();

    let items_array = if let Some(arr) = val.as_array() {
        Some(arr)
    } else if let Some(arr) = val.get("items").and_then(|v| v.as_array()) {
        Some(arr)
    } else if let Some(arr) = val.get("hosts").and_then(|v| v.as_array()) {
        Some(arr)
    } else {
        None
    };

    if let Some(arr) = items_array {
        for item in arr {
            if let Some(entry) = parse_single_termius_object(item) {
                list.push(entry);
            }
        }
    }

    list
}

fn parse_single_termius_object(item: &serde_json::Value) -> Option<ImportedHostEntry> {
    // 提取地址
    let address = item.get("address")
        .or_else(|| item.get("hostname"))
        .or_else(|| item.get("host"))
        .or_else(|| item.get("ip"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())?;

    if address.is_empty() {
        return None;
    }

    // 提取名称
    let label = item.get("label")
        .or_else(|| item.get("name"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| address.clone());

    // 提取端口
    let port = item.get("port")
        .and_then(|v| {
            if let Some(n) = v.as_u64() {
                Some(n as u16)
            } else if let Some(s) = v.as_str() {
                s.trim().parse::<u16>().ok()
            } else {
                None
            }
        })
        .unwrap_or(22);

    // 提取用户名
    let username = item.get("username")
        .or_else(|| item.get("user"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    // 提取分组
    let group_name = item.get("group_label")
        .or_else(|| item.get("group"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    // 提取备注
    let notes = item.get("notes")
        .or_else(|| item.get("comment"))
        .or_else(|| item.get("description"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    Some(ImportedHostEntry {
        name: label,
        address,
        port,
        username,
        group_name,
        notes,
    })
}

/// 解析 Xshell `.xsh` 单文件
fn parse_xshell_ini(content: &str, file_name: Option<&str>) -> Option<ImportedHostEntry> {
    let mut host = String::new();
    let mut port = 22u16;
    let mut user = None;
    let mut desc = String::new();

    for line in content.lines() {
        let l = line.trim();
        if l.starts_with('#') || l.starts_with(';') || l.starts_with('[') {
            continue;
        }
        if let Some((k, v)) = l.split_once('=') {
            let key = k.trim().to_ascii_lowercase();
            let val = v.trim();
            match key.as_str() {
                "host" => host = val.to_string(),
                "port" => {
                    if let Ok(p) = val.parse::<u16>() {
                        port = p;
                    }
                }
                "username" | "user" => {
                    if !val.is_empty() {
                        user = Some(val.to_string());
                    }
                }
                "description" | "notes" => {
                    if !val.is_empty() {
                        desc = val.to_string();
                    }
                }
                _ => {}
            }
        }
    }

    if host.is_empty() {
        return None;
    }

    let default_name = file_name
        .map(|n| n.trim_end_matches(".xsh").trim_end_matches(".XSH"))
        .filter(|n| !n.is_empty())
        .unwrap_or(&host)
        .to_string();

    Some(ImportedHostEntry {
        name: default_name,
        address: host,
        port,
        username: user,
        group_name: Some("Xshell 导入".to_string()),
        notes: desc,
    })
}

/// 解析 CSV / TSV 分隔符表格
fn parse_delimited_table(content: &str) -> Vec<ImportedHostEntry> {
    let mut lines = content.lines().map(|l| l.trim()).filter(|l| !l.is_empty());
    let header_line = match lines.next() {
        Some(h) => h,
        None => return Vec::new(),
    };

    // 自动判断分隔符: 优先制表符 `\t`，次选逗号 `,`，再选分号 `;`
    let delim = if header_line.contains('\t') {
        '\t'
    } else if header_line.contains(',') {
        ','
    } else if header_line.contains(';') {
        ';'
    } else {
        return Vec::new();
    };

    let headers: Vec<String> = header_line.split(delim).map(|s| s.trim().to_ascii_lowercase()).collect();

    // 寻找列索引
    let col_name = headers.iter().position(|h| h.contains("label") || h.contains("name") || h == "session");
    let col_host = headers.iter().position(|h| h.contains("address") || h.contains("host") || h.contains("ip"));
    let col_port = headers.iter().position(|h| h.contains("port"));
    let col_user = headers.iter().position(|h| h.contains("user") || h.contains("username"));
    let col_group = headers.iter().position(|h| h.contains("group") || h.contains("folder"));
    let col_notes = headers.iter().position(|h| h.contains("notes") || h.contains("comment") || h.contains("desc"));

    if col_host.is_none() && col_name.is_none() {
        return Vec::new();
    }

    let mut result = Vec::new();

    for line in lines {
        let parts: Vec<&str> = line.split(delim).map(|s| s.trim().trim_matches('"')).collect();
        if parts.is_empty() {
            continue;
        }

        let address = col_host.and_then(|idx| parts.get(idx)).unwrap_or(&"").trim().to_string();
        if address.is_empty() {
            continue;
        }

        let name = col_name.and_then(|idx| parts.get(idx))
            .filter(|s| !s.is_empty())
            .unwrap_or(&address.as_str())
            .to_string();

        let port = col_port.and_then(|idx| parts.get(idx))
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(22);

        let username = col_user.and_then(|idx| parts.get(idx))
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let group_name = col_group.and_then(|idx| parts.get(idx))
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let notes = col_notes.and_then(|idx| parts.get(idx))
            .map(|s| s.to_string())
            .unwrap_or_default();

        result.push(ImportedHostEntry {
            name,
            address,
            port,
            username,
            group_name,
            notes,
        });
    }

    result
}

/// 解析原生 OpenSSH 配置文件 (`~/.ssh/config`) 内容为标准 `HostRecord` 资产列表。
///
/// 自动过滤通配符匹配主机 (如 `Host *`)，并解析 `HostName`、`Port`、`User`、`IdentityFile`。
pub fn parse_ssh_config(content: &str) -> Vec<HostRecord> {
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

/// 获取当前操作系统的默认 OpenSSH 用户配置文件绝对路径 (`~/.ssh/config`)。
pub fn get_default_ssh_config_path() -> std::path::PathBuf {
    #[cfg(windows)]
    {
        if let Ok(profile) = std::env::var("USERPROFILE") {
            return std::path::Path::new(&profile).join(".ssh").join("config");
        }
    }
    #[cfg(not(windows))]
    {
        if let Ok(home) = std::env::var("HOME") {
            return std::path::Path::new(&home).join(".ssh").join("config");
        }
    }
    std::path::PathBuf::from(".ssh/config")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_termius_json() {
        let json_data = r#"[
            {
                "label": "Web-Prod-01",
                "address": "10.0.0.1",
                "port": 2222,
                "username": "admin",
                "group_label": "Production",
                "comment": "Main API gateway"
            },
            {
                "label": "Db-Master",
                "address": "10.0.0.2",
                "port": 22
            }
        ]"#;

        let entries = parse_external_assets(json_data, None);
        assert_eq!(entries.len(), 2);

        assert_eq!(entries[0].name, "Web-Prod-01");
        assert_eq!(entries[0].address, "10.0.0.1");
        assert_eq!(entries[0].port, 2222);
        assert_eq!(entries[0].username.as_deref(), Some("admin"));
        assert_eq!(entries[0].group_name.as_deref(), Some("Production"));
        assert_eq!(entries[0].notes, "Main API gateway");

        assert_eq!(entries[1].name, "Db-Master");
        assert_eq!(entries[1].address, "10.0.0.2");
        assert_eq!(entries[1].port, 22);
        assert_eq!(entries[1].group_name, None);
    }

    #[test]
    fn test_parse_xshell_ini() {
        let xsh_data = r#"
        [CONNECTION]
        Host=192.168.1.55
        Port=2202
        Description=Dev cluster
        [CONNECTION:AUTHENTICATION]
        UserName=deploy
        "#;

        let entries = parse_external_assets(xsh_data, Some("k8s-node1.xsh"));
        assert_eq!(entries.len(), 1);
        let e = &entries[0];
        assert_eq!(e.name, "k8s-node1");
        assert_eq!(e.address, "192.168.1.55");
        assert_eq!(e.port, 2202);
        assert_eq!(e.username.as_deref(), Some("deploy"));
        assert_eq!(e.notes, "Dev cluster");
    }

    #[test]
    fn test_parse_csv_delimited() {
        let csv_data = "Name,Host,Port,User,Group\nRedis-01,172.16.0.10,6379,redis,Database\nNginx-01,172.16.0.11,22,root,Web\n";
        let entries = parse_external_assets(csv_data, None);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "Redis-01");
        assert_eq!(entries[0].address, "172.16.0.10");
        assert_eq!(entries[0].port, 6379);
        assert_eq!(entries[0].username.as_deref(), Some("redis"));
        assert_eq!(entries[0].group_name.as_deref(), Some("Database"));
    }

    #[test]
    fn test_parse_openssh_config() {
        let config_data = r#"
        # Global wildcard to ignore
        Host *
            ServerAliveInterval 60
            User root

        Host bastion-jump
            HostName 123.56.78.90
            Port 2222
            User ops_admin
            IdentityFile ~/.ssh/id_ed25519

        Host internal-k8s
            HostName 10.244.0.5
            Port 6443
        "#;

        let hosts = parse_ssh_config(config_data);
        assert_eq!(hosts.len(), 2);

        assert_eq!(hosts[0].name, "bastion-jump");
        assert_eq!(hosts[0].address, "123.56.78.90");
        assert_eq!(hosts[0].port, 2222);
        assert!(hosts[0].notes.contains("User: ops_admin"));
        assert!(hosts[0].notes.contains("~/.ssh/id_ed25519"));

        assert_eq!(hosts[1].name, "internal-k8s");
        assert_eq!(hosts[1].address, "10.244.0.5");
        assert_eq!(hosts[1].port, 6443);
    }
}
