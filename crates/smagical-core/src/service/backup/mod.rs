//! 容灾备份驱动抽象与全量资产序列化封包服务 (Backup Drivers & Payload Service)。

pub mod local;
pub mod webdav;
pub mod s3;
pub mod gist;

use hmac::digest::Digest;
use sha2::Sha256;
use serde::{Deserialize, Serialize};

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use rand::RngExt;

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

/// 加密快照信封固定魔数常量
pub const ENCRYPTED_SNAPSHOT_MAGIC: &str = "smalux_encrypted_snapshot_v1";

/// Argon2id 推荐内存硬化大小 (64 MB, RFC 9106)
pub const DEFAULT_ARGON2_MEMORY_KIB: u32 = 65536;
/// Argon2id 推荐迭代轮数 (3 轮, RFC 9106)
pub const DEFAULT_ARGON2_ITERATIONS: u32 = 3;
/// Argon2id 推荐并发线程数 (4 并发, RFC 9106)
pub const DEFAULT_ARGON2_PARALLELISM: u32 = 4;

/// 快照加密信封元数据容器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedSnapshotEnvelope {
    /// 密文格式与版本标识 (如 "smalux_encrypted_snapshot_v1")
    pub format: String,
    /// 密钥派生算法标识 (如 "argon2id")
    pub kdf: String,
    /// Base64 编码的 32 字节密码盐 (Salt)
    pub salt_b64: String,
    /// Base64 编码的 12 字节认证随机数 (Nonce)
    pub nonce_b64: String,
    /// Base64 编码的 AES-256-GCM 密文与 16 字节认证标签 (MAC Tag)
    pub ciphertext_b64: String,
    /// 快照生成时的 UNIX 纪元时间戳 (秒)
    pub epoch_secs: u64,
    /// 原始明文快照数据的 SHA-256 完整性哈希
    pub sha256: String,
}

/// 资产恢复冲突消解策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BackupConflictPolicy {
    /// 强制覆盖本地已存在的冲突资产
    #[default]
    Overwrite,
    /// 跳过已存在项，仅保留本地已有记录
    SkipExisting,
    /// 智能合并：比对更新时间戳或内容差异，保留较新记录
    MergeNewer,
}

/// 资产恢复执行报告统计
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BackupRestoreReport {
    /// 成功导入的主机数量
    pub imported_hosts: usize,
    /// 因已存在而跳过的主机数量
    pub skipped_hosts: usize,
    /// 成功导入的分组数量
    pub imported_groups: usize,
    /// 因已存在而跳过的分组数量
    pub skipped_groups: usize,
    /// 成功导入的凭据数量
    pub imported_credentials: usize,
    /// 因已存在而跳过的凭据数量
    pub skipped_credentials: usize,
    /// 成功导入的网络隧道数量
    pub imported_tunnels: usize,
    /// 因已存在而跳过的网络隧道数量
    pub skipped_tunnels: usize,
    /// 成功导入的代码片段数量
    pub imported_snippets: usize,
    /// 因已存在而跳过的代码片段数量
    pub skipped_snippets: usize,
    /// 执行恢复所使用的冲突消解策略
    pub policy: BackupConflictPolicy,
}

/// 备份快照结构化预检分析元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupSnapshotMetadata {
    /// 快照协议版本号
    pub version: String,
    /// 快照导出应用标识
    pub app: String,
    /// 导出时的 UNIX 纪元时间戳 (秒)
    pub exported_at_epoch: u64,
    /// 是否为加密快照信封
    pub is_encrypted: bool,
    /// 快照内包含的主机资产数量
    pub hosts_count: usize,
    /// 快照内包含的分组数量
    pub groups_count: usize,
    /// 快照内包含的凭据数量
    pub credentials_count: usize,
    /// 快照内包含的网络隧道数量
    pub tunnels_count: usize,
    /// 快照内包含的代码片段数量
    pub snippets_count: usize,
    /// 快照原始明文内容的 SHA-256 完整性校验和
    pub sha256: String,
}

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

    /// 自动依据保留规则修剪过期的陈旧远端历史快照镜像
    async fn prune_old_snapshots(&self, keep_count: usize) -> Result<usize, String> {
        let mut snaps = self.list_snapshots().await?;
        if snaps.len() <= keep_count || keep_count == 0 {
            return Ok(0);
        }
        // 降序排序，最新的在前
        snaps.sort_by(|a, b| b.epoch_secs.cmp(&a.epoch_secs));
        let to_delete = &snaps[keep_count..];
        let mut deleted = 0;
        for s in to_delete {
            if let Ok(()) = self.delete_snapshot(&s.remote_id).await {
                deleted += 1;
            }
        }
        Ok(deleted)
    }
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

/// 使用 Argon2id 将用户口令与 Salt 派生为 256 位 (32 字节) 强密钥
pub fn derive_backup_key(passphrase: &str, salt: &[u8]) -> Result<[u8; 32], String> {
    let params = Params::new(
        DEFAULT_ARGON2_MEMORY_KIB,
        DEFAULT_ARGON2_ITERATIONS,
        DEFAULT_ARGON2_PARALLELISM,
        Some(32),
    )
    .map_err(|e| format!("Argon2 参数非法: {}", e))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

    let mut key = [0u8; 32];
    argon2
        .hash_password_into(passphrase.as_bytes(), salt, &mut key)
        .map_err(|e| format!("Argon2 密钥派生失败: {}", e))?;

    Ok(key)
}

/// 判断给定的二进制数据流是否为符合 smalux 规范的加密快照信封
pub fn is_encrypted_snapshot(payload_bytes: &[u8]) -> bool {
    if let Ok(val) = serde_json::from_slice::<serde_json::Value>(payload_bytes) {
        if let Some(format) = val.get("format").and_then(|f| f.as_str()) {
            return format == ENCRYPTED_SNAPSHOT_MAGIC;
        }
    }
    false
}

/// 使用口令对明文快照字节流执行 Argon2id + AES-256-GCM 认证加密
pub fn encrypt_snapshot_payload(payload_bytes: &[u8], passphrase: &str) -> Result<Vec<u8>, String> {
    if passphrase.trim().is_empty() {
        return Err("加密备份口令不能为空".to_string());
    }

    let mut salt = [0u8; 32];
    rand::rng().fill(&mut salt);
    let key = derive_backup_key(passphrase, &salt)?;

    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| format!("初始化 AES-256-GCM 密码机失败: {}", e))?;

    let mut nonce_bytes = [0u8; 12];
    rand::rng().fill(&mut nonce_bytes);
    let nonce = Nonce::from(nonce_bytes);

    let ciphertext = cipher
        .encrypt(&nonce, payload_bytes)
        .map_err(|e| format!("AES-256-GCM 加密失败: {}", e))?;

    let epoch_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let envelope = EncryptedSnapshotEnvelope {
        format: ENCRYPTED_SNAPSHOT_MAGIC.to_string(),
        kdf: "argon2id".to_string(),
        salt_b64: BASE64.encode(salt),
        nonce_b64: BASE64.encode(nonce_bytes),
        ciphertext_b64: BASE64.encode(ciphertext),
        epoch_secs,
        sha256: compute_sha256(payload_bytes),
    };

    serde_json::to_vec_pretty(&envelope).map_err(|e| format!("序列化加密信封失败: {}", e))
}

/// 使用口令对加密信封字节流执行完整性校验与解密还原
pub fn decrypt_snapshot_payload(payload_bytes: &[u8], passphrase: &str) -> Result<Vec<u8>, String> {
    if passphrase.trim().is_empty() {
        return Err("解密备份口令不能为空".to_string());
    }

    let envelope: EncryptedSnapshotEnvelope = serde_json::from_slice(payload_bytes)
        .map_err(|e| format!("解析加密信封失败: {}", e))?;

    if envelope.format != ENCRYPTED_SNAPSHOT_MAGIC {
        return Err(format!("不支持的快照加密信封格式: {}", envelope.format));
    }

    let salt = BASE64.decode(&envelope.salt_b64)
        .map_err(|e| format!("Base64 解码 Salt 失败: {}", e))?;
    let nonce_bytes = BASE64.decode(&envelope.nonce_b64)
        .map_err(|e| format!("Base64 解码 Nonce 失败: {}", e))?;
    let ciphertext = BASE64.decode(&envelope.ciphertext_b64)
        .map_err(|e| format!("Base64 解码密文失败: {}", e))?;

    if nonce_bytes.len() != 12 {
        return Err(format!("Nonce 长度非法: 期望 12 字节, 实际 {} 字节", nonce_bytes.len()));
    }

    let key = derive_backup_key(passphrase, &salt)?;
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| format!("初始化 AES-256-GCM 密码机失败: {}", e))?;

    let mut nb = [0u8; 12];
    nb.copy_from_slice(&nonce_bytes);
    let nonce = Nonce::from(nb);

    let plaintext = cipher
        .decrypt(&nonce, ciphertext.as_ref())
        .map_err(|e| format!("解密快照失败: 口令错误或密文损坏 ({})", e))?;

    // 校验完整性哈希
    let computed_hash = compute_sha256(&plaintext);
    if !envelope.sha256.is_empty() && envelope.sha256 != computed_hash {
        return Err(format!(
            "快照解密后完整性校验失败: 期望 {}, 实际 {}",
            envelope.sha256, computed_hash
        ));
    }

    Ok(plaintext)
}

/// 预检并解析快照字节流的元数据概要（不将数据落盘）
pub fn inspect_backup_payload(
    payload_bytes: &[u8],
    passphrase: Option<&str>,
) -> Result<BackupSnapshotMetadata, String> {
    let (plain_bytes, is_enc, orig_sha256) = if is_encrypted_snapshot(payload_bytes) {
        let pass = passphrase.ok_or_else(|| "快照已加密，需要输入解密口令进行预检".to_string())?;
        let env: EncryptedSnapshotEnvelope = serde_json::from_slice(payload_bytes)
            .map_err(|e| format!("解析加密信封失败: {}", e))?;
        let decrypted = decrypt_snapshot_payload(payload_bytes, pass)?;
        (decrypted, true, env.sha256)
    } else {
        (payload_bytes.to_vec(), false, compute_sha256(payload_bytes))
    };

    let val: serde_json::Value = serde_json::from_slice(&plain_bytes)
        .map_err(|e| format!("快照文件 JSON 结构解析失败: {}", e))?;

    let version = val.get("version").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
    let app = val.get("app").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
    let exported_at_epoch = val.get("exported_at_epoch").and_then(|v| v.as_u64()).unwrap_or(0);
    let hosts_count = val.get("hosts").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
    let groups_count = val.get("groups").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
    let credentials_count = val.get("credentials").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
    let tunnels_count = val.get("tunnels").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
    let snippets_count = val.get("snippets").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);

    Ok(BackupSnapshotMetadata {
        version,
        app,
        exported_at_epoch,
        is_encrypted: is_enc,
        hosts_count,
        groups_count,
        credentials_count,
        tunnels_count,
        snippets_count,
        sha256: orig_sha256,
    })
}

/// 创建快照，并在指定了口令时执行 Argon2id + AES-256-GCM 加密封包
pub async fn create_backup_payload_with_encryption(
    storage: &(dyn AppStorage + 'static),
    include_credentials: bool,
    passphrase: Option<&str>,
) -> Result<(Vec<u8>, String, usize, usize), String> {
    let (plain_bytes, plain_hash, hosts_count, tunnels_count) =
        create_backup_payload(storage, include_credentials).await?;

    if let Some(pass) = passphrase.filter(|p| !p.trim().is_empty()) {
        let enc_bytes = encrypt_snapshot_payload(&plain_bytes, pass)?;
        let enc_hash = compute_sha256(&enc_bytes);
        Ok((enc_bytes, enc_hash, hosts_count, tunnels_count))
    } else {
        Ok((plain_bytes, plain_hash, hosts_count, tunnels_count))
    }
}

/// 根据指定的冲突策略从快照数据流（支持明文或加密信封）恢复资产
pub async fn restore_backup_payload_with_policy(
    storage: &(dyn AppStorage + 'static),
    payload_bytes: &[u8],
    passphrase: Option<&str>,
    policy: BackupConflictPolicy,
) -> Result<BackupRestoreReport, String> {
    let plain_bytes = if is_encrypted_snapshot(payload_bytes) {
        let pass = passphrase.ok_or_else(|| "快照已加密，必须提供解密口令进行还原".to_string())?;
        decrypt_snapshot_payload(payload_bytes, pass)?
    } else {
        payload_bytes.to_vec()
    };

    let val: serde_json::Value = serde_json::from_slice(&plain_bytes)
        .map_err(|e| format!("快照文件 JSON 结构解析失败: {}", e))?;

    let mut report = BackupRestoreReport {
        policy,
        ..Default::default()
    };

    // 1. 还原分组
    if let Some(groups_arr) = val.get("groups").and_then(|g| g.as_array()) {
        for g_val in groups_arr {
            if let Ok(group_rec) = serde_json::from_value::<GroupRecord>(g_val.clone()) {
                let existing = storage.groups().get_by_id(&group_rec.id).await.unwrap_or(None);
                let should_save = match policy {
                    BackupConflictPolicy::Overwrite => true,
                    BackupConflictPolicy::SkipExisting => existing.is_none(),
                    BackupConflictPolicy::MergeNewer => match existing {
                        None => true,
                        Some(ref ex) => ex != &group_rec,
                    },
                };

                if should_save {
                    if storage.groups().save(&group_rec).await.is_ok() {
                        report.imported_groups += 1;
                    }
                } else {
                    report.skipped_groups += 1;
                }
            }
        }
    }

    // 2. 还原主机
    if let Some(hosts_arr) = val.get("hosts").and_then(|h| h.as_array()) {
        for h_val in hosts_arr {
            if let Ok(host_rec) = serde_json::from_value::<HostRecord>(h_val.clone()) {
                let existing = storage.hosts().get_by_id(&host_rec.id).await.unwrap_or(None);
                let should_save = match policy {
                    BackupConflictPolicy::Overwrite => true,
                    BackupConflictPolicy::SkipExisting => existing.is_none(),
                    BackupConflictPolicy::MergeNewer => match existing {
                        None => true,
                        Some(ref ex) => ex != &host_rec,
                    },
                };

                if should_save {
                    if storage.hosts().save(&host_rec).await.is_ok() {
                        report.imported_hosts += 1;
                    }
                } else {
                    report.skipped_hosts += 1;
                }
            }
        }
    }

    // 3. 还原凭据
    if let Some(creds_arr) = val.get("credentials").and_then(|c| c.as_array()) {
        for c_val in creds_arr {
            if let Ok(cred_rec) = serde_json::from_value::<CredentialRecord>(c_val.clone()) {
                let existing = storage.credentials().get_by_id(&cred_rec.id).await.unwrap_or(None);
                let should_save = match policy {
                    BackupConflictPolicy::Overwrite => true,
                    BackupConflictPolicy::SkipExisting => existing.is_none(),
                    BackupConflictPolicy::MergeNewer => match existing {
                        None => true,
                        Some(ref ex) => cred_rec.updated_at >= ex.updated_at,
                    },
                };

                if should_save {
                    if storage.credentials().save(&cred_rec).await.is_ok() {
                        report.imported_credentials += 1;
                    }
                } else {
                    report.skipped_credentials += 1;
                }
            }
        }
    }

    // 4. 还原隧道
    if let Some(tunnels_arr) = val.get("tunnels").and_then(|t| t.as_array()) {
        for t_val in tunnels_arr {
            if let Ok(tunnel_rec) = serde_json::from_value::<TunnelRecord>(t_val.clone()) {
                let existing = storage.tunnels().get_by_id(&tunnel_rec.id).await.unwrap_or(None);
                let should_save = match policy {
                    BackupConflictPolicy::Overwrite => true,
                    BackupConflictPolicy::SkipExisting => existing.is_none(),
                    BackupConflictPolicy::MergeNewer => match existing {
                        None => true,
                        Some(ref ex) => ex != &tunnel_rec,
                    },
                };

                if should_save {
                    if storage.tunnels().save(&tunnel_rec).await.is_ok() {
                        report.imported_tunnels += 1;
                    }
                } else {
                    report.skipped_tunnels += 1;
                }
            }
        }
    }

    // 5. 还原代码片段与分组
    if let Some(snips_arr) = val.get("snippets").and_then(|s| s.as_array()) {
        for s_val in snips_arr {
            if let Ok(snip_rec) = serde_json::from_value::<SnippetRecord>(s_val.clone()) {
                let existing = storage.snippets().get_by_id(&snip_rec.id).await.unwrap_or(None);
                let should_save = match policy {
                    BackupConflictPolicy::Overwrite => true,
                    BackupConflictPolicy::SkipExisting => existing.is_none(),
                    BackupConflictPolicy::MergeNewer => match existing {
                        None => true,
                        Some(ref ex) => ex != &snip_rec,
                    },
                };

                if should_save {
                    if storage.snippets().save(&snip_rec).await.is_ok() {
                        report.imported_snippets += 1;
                    }
                } else {
                    report.skipped_snippets += 1;
                }
            }
        }
    }
    if let Some(groups_arr) = val.get("snippet_groups").and_then(|g| g.as_array()) {
        for g_val in groups_arr {
            if let Ok(snip_grp) = serde_json::from_value::<SnippetGroupRecord>(g_val.clone()) {
                let existing = storage.snippets().get_group_by_id(&snip_grp.id).await.unwrap_or(None);
                let should_save = match policy {
                    BackupConflictPolicy::Overwrite => true,
                    BackupConflictPolicy::SkipExisting => existing.is_none(),
                    BackupConflictPolicy::MergeNewer => match existing {
                        None => true,
                        Some(ref ex) => ex != &snip_grp,
                    },
                };

                if should_save {
                    let _ = storage.snippets().save_group(&snip_grp).await;
                }
            }
        }
    }

    Ok(report)
}

/// 从快照 JSON 字节流无损还原资产至底层仓储 (向下兼容 Overwrite 策略与未加密快照)
///
/// 返回: (还原主机数, 还原分组数, 还原凭据数)
pub async fn restore_backup_payload(
    storage: &(dyn AppStorage + 'static),
    payload_bytes: &[u8],
) -> Result<(usize, usize, usize), String> {
    let report = restore_backup_payload_with_policy(
        storage,
        payload_bytes,
        None,
        BackupConflictPolicy::Overwrite,
    )
    .await?;

    Ok((
        report.imported_hosts,
        report.imported_groups,
        report.imported_credentials,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes_size() {
        assert_eq!(format_bytes_size(500), "500 B");
        assert_eq!(format_bytes_size(1024), "1.0 KB");
        assert_eq!(format_bytes_size(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes_size(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn test_compute_sha256() {
        let hash = compute_sha256(b"hello world");
        assert!(hash.starts_with("sha256:"));
        assert_eq!(hash.len(), 7 + 64);
    }

    #[test]
    fn test_encryption_and_decryption_roundtrip() {
        let plain = b"{\"hosts\": [\"host-1\", \"host-2\"], \"version\": \"1.0\"}";
        let pass = "StrongP@ssw0rd!2026";

        let encrypted = encrypt_snapshot_payload(plain, pass).expect("encryption should succeed");
        assert!(is_encrypted_snapshot(&encrypted));

        let decrypted = decrypt_snapshot_payload(&encrypted, pass).expect("decryption should succeed");
        assert_eq!(decrypted, plain);

        // 错误口令解密必须失败
        let wrong_err = decrypt_snapshot_payload(&encrypted, "wrong-pass");
        assert!(wrong_err.is_err());
    }

    #[test]
    fn test_inspect_backup_payload() {
        let plain = serde_json::json!({
            "version": "1.0",
            "app": "smalux-ssh",
            "exported_at_epoch": 1720000000u64,
            "hosts": [{"id": "h1"}, {"id": "h2"}],
            "groups": [{"id": "g1"}],
            "credentials": [],
            "tunnels": [],
            "snippets": []
        });
        let plain_bytes = serde_json::to_vec(&plain).unwrap();

        // 1. 预检明文快照
        let meta1 = inspect_backup_payload(&plain_bytes, None).expect("inspect plain should succeed");
        assert!(!meta1.is_encrypted);
        assert_eq!(meta1.hosts_count, 2);
        assert_eq!(meta1.groups_count, 1);
        assert_eq!(meta1.app, "smalux-ssh");

        // 2. 预检密文快照
        let pass = "vault-pass-999";
        let encrypted = encrypt_snapshot_payload(&plain_bytes, pass).unwrap();
        let meta2 = inspect_backup_payload(&encrypted, Some(pass)).expect("inspect enc should succeed");
        assert!(meta2.is_encrypted);
        assert_eq!(meta2.hosts_count, 2);

        // 未提供口令预检加密快照报错
        assert!(inspect_backup_payload(&encrypted, None).is_err());
    }
}
