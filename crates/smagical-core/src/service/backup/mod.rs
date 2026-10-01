//! 容灾备份驱动抽象与全量资产序列化封包服务 (Backup Drivers & Payload Service)。

pub mod local;
pub mod webdav;
pub mod s3;
pub mod gist;

use hmac::digest::Digest;
use sha2::Sha256;
use serde::{Deserialize, Serialize};

use crate::domain::backup::{BackupTaskRecord, BackupType};
use crate::domain::credential::CredentialRecord;
use crate::domain::group::GroupRecord;
use crate::domain::host::HostRecord;
use crate::domain::snippet::{SnippetGroupRecord, SnippetRecord};
use crate::domain::tunnel::TunnelRecord;
use crate::storage::AppStorage;

pub use local::LocalBackupDriver;
pub use webdav::WebdavBackupDriver;
pub use s3::S3BackupDriver;
pub use gist::GistBackupDriver;

/// 远端快照镜像元数据信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteSnapshotInfo {
    /// 远端资源标识符 (文件名或 SHA 或 Gist Commit ID)
    pub remote_id: String,
    /// 格式化时间字符串
    pub timestamp: String,
    /// UNIX 时间戳秒数
    pub epoch_secs: u64,
    /// 镜像字节体积
    pub size_bytes: u64,
    /// 快照内容 SHA-256 完整性哈希
    pub hash: String,
}

/// 统一备份存储驱动契约
#[async_trait::async_trait]
pub trait BackupDriver: Send + Sync {
    /// 测试与远端存储端点的连通性与认证鉴权
    async fn test_connection(&self) -> Result<(), String>;

    /// 上传/推送一份最新全量资产快照镜像
    async fn push_snapshot(&self, snapshot_id: &str, payload: &[u8]) -> Result<String, String>;

    /// 枚举远端存储上已存留的历史快照列表
    async fn list_snapshots(&self) -> Result<Vec<RemoteSnapshotInfo>, String>;

    /// 从远端拉取指定快照镜像的完整数据流
    async fn pull_snapshot(&self, remote_id: &str) -> Result<Vec<u8>, String>;

    /// 从远端存储物理删除指定的快照镜像 (用于生命周期清理)
    async fn delete_snapshot(&self, remote_id: &str) -> Result<(), String>;
}

/// 根据备份任务配置实例化对应的存储协议驱动
pub fn create_backup_driver(task: &BackupTaskRecord) -> Box<dyn BackupDriver> {
    match task.backup_type {
        BackupType::Local => Box::new(LocalBackupDriver::new(&task.endpoint)),
        BackupType::Webdav => Box::new(WebdavBackupDriver::new(
            &task.endpoint,
            &task.auth_user,
            &task.auth_secret,
        )),
        BackupType::S3 => Box::new(S3BackupDriver::new(
            &task.endpoint,
            &task.auth_user,
            &task.auth_secret,
        )),
        BackupType::Gist => Box::new(GistBackupDriver::new(
            &task.endpoint,
            &task.auth_secret,
        )),
        BackupType::SelfHosted => Box::new(WebdavBackupDriver::new(
            &task.endpoint,
            &task.auth_user,
            &task.auth_secret,
        )),
    }
}

/// 计算二进制数据的 SHA-256 校验和字符串 (如 "sha256:e3b0c442...")
pub fn compute_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    format!("sha256:{}", hex_encode(&result))
}

pub(crate) fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// 格式化字节体积为紧凑人类可读字符串 (如 "1.4 MB", "520 KB")
pub fn format_bytes_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

/// 将当前系统全部核心资产打包生成结构化快照 JSON 字节流
///
/// 返回: (快照 JSON 字节, SHA-256 哈希, 主机数, 隧道数)
pub async fn create_backup_payload(
    storage: &(dyn AppStorage + 'static),
    include_credentials: bool,
) -> Result<(Vec<u8>, String, usize, usize), String> {
    let hosts = storage.hosts().list_all().await.map_err(|e| e.to_string())?;
    let groups = storage.groups().list_all().await.map_err(|e| e.to_string())?;
    let snippets = storage.snippets().list_all().await.map_err(|e| e.to_string())?;
    let snippet_groups = storage.snippets().list_groups().await.map_err(|e| e.to_string())?;
    let tunnels = storage.tunnels().list_all().await.map_err(|e| e.to_string())?;
    let credentials = if include_credentials {
        storage.credentials().list_all().await.map_err(|e| e.to_string())?
    } else {
        Vec::new()
    };

    let epoch_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let hosts_count = hosts.len();
    let tunnels_count = tunnels.len();

    let payload = serde_json::json!({
        "version": "1.0",
        "app": "smalux-ssh",
        "exported_at_epoch": epoch_secs,
        "include_credentials": include_credentials,
        "hosts_count": hosts_count,
        "groups_count": groups.len(),
        "snippets_count": snippets.len(),
        "tunnels_count": tunnels_count,
        "credentials_count": credentials.len(),
        "hosts": hosts,
        "groups": groups,
        "snippets": snippets,
        "snippet_groups": snippet_groups,
        "tunnels": tunnels,
        "credentials": credentials,
    });

    let json_bytes = serde_json::to_vec_pretty(&payload).map_err(|e| e.to_string())?;
    let hash = compute_sha256(&json_bytes);

    Ok((json_bytes, hash, hosts_count, tunnels_count))
}

/// 从快照 JSON 字节流无损还原资产至底层仓储
///
/// 返回: (还原主机数, 还原分组数, 还原凭据数)
pub async fn restore_backup_payload(
    storage: &(dyn AppStorage + 'static),
    payload_bytes: &[u8],
) -> Result<(usize, usize, usize), String> {
    let val: serde_json::Value = serde_json::from_slice(payload_bytes)
        .map_err(|e| format!("快照文件 JSON 结构解析失败: {}", e))?;

    let mut imported_groups = 0;
    let mut imported_hosts = 0;
    let mut imported_creds = 0;

    // 1. 还原分组
    if let Some(groups_arr) = val.get("groups").and_then(|g| g.as_array()) {
        for g_val in groups_arr {
            if let Ok(group_rec) = serde_json::from_value::<GroupRecord>(g_val.clone()) {
                let _ = storage.groups().save(&group_rec).await;
                imported_groups += 1;
            }
        }
    }

    // 2. 还原主机
    if let Some(hosts_arr) = val.get("hosts").and_then(|h| h.as_array()) {
        for h_val in hosts_arr {
            if let Ok(host_rec) = serde_json::from_value::<HostRecord>(h_val.clone()) {
                let _ = storage.hosts().save(&host_rec).await;
                imported_hosts += 1;
            }
        }
    }

    // 3. 还原凭据
    if let Some(creds_arr) = val.get("credentials").and_then(|c| c.as_array()) {
        for c_val in creds_arr {
            if let Ok(cred_rec) = serde_json::from_value::<CredentialRecord>(c_val.clone()) {
                let _ = storage.credentials().save(&cred_rec).await;
                imported_creds += 1;
            }
        }
    }

    // 4. 还原隧道
    if let Some(tunnels_arr) = val.get("tunnels").and_then(|t| t.as_array()) {
        for t_val in tunnels_arr {
            if let Ok(tunnel_rec) = serde_json::from_value::<TunnelRecord>(t_val.clone()) {
                let _ = storage.tunnels().save(&tunnel_rec).await;
            }
        }
    }

    // 5. 还原代码片段与分组
    if let Some(snips_arr) = val.get("snippets").and_then(|s| s.as_array()) {
        for s_val in snips_arr {
            if let Ok(snip_rec) = serde_json::from_value::<SnippetRecord>(s_val.clone()) {
                let _ = storage.snippets().save(&snip_rec).await;
            }
        }
    }
    if let Some(groups_arr) = val.get("snippet_groups").and_then(|g| g.as_array()) {
        for g_val in groups_arr {
            if let Ok(snip_grp) = serde_json::from_value::<SnippetGroupRecord>(g_val.clone()) {
                let _ = storage.snippets().save_group(&snip_grp).await;
            }
        }
    }

    Ok((imported_hosts, imported_groups, imported_creds))
}
