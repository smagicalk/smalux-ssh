//! 内存仿真网络与协议服务实现 (用于单元测试、快速启动与无网络 UI 调试)。

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use async_trait::async_trait;
use tokio::io::duplex;
use tokio::sync::mpsc::UnboundedSender;

use crate::domain::file_item::generate_mock_remote_directory;
use crate::domain::{CredentialRecord, FileItemData, HostRecord, TunnelRecord};
use crate::service::error::{SshServiceError, SshServiceResult};
use crate::service::keygen::{GeneratedKeyPair, KeyAlgorithm, KeygenService, ParsedKeyInfo};
use crate::service::metrics::{HostMetricsService, SystemMetricsSnapshot};
use crate::service::sftp::{SftpService, TransferProgress};
use crate::service::ssh::{CommandExecutionOutput, SshSessionService, SshStreamChannel};
use crate::service::tunnel::{TunnelHandle, TunnelMetricsSnapshot, TunnelService};

/// 内存仿真 SFTP 文件传输服务
#[derive(Debug, Default, Clone)]
pub struct MockSftpService {
    mock_files: Arc<RwLock<HashMap<String, Vec<FileItemData>>>>,
}

impl MockSftpService {
    /// 创建带有默认根目录种子数据的仿真 SFTP 服务
    pub fn new() -> Self {
        let mut map = HashMap::new();
        map.insert("/".to_string(), generate_mock_remote_directory("/"));
        map.insert("/root".to_string(), generate_mock_remote_directory("/root"));
        map.insert("/var/log".to_string(), generate_mock_remote_directory("/var/log"));
        Self {
            mock_files: Arc::new(RwLock::new(map)),
        }
    }
}

#[async_trait]
impl SftpService for MockSftpService {
    async fn list_dir(&self, _session_id: &str, remote_path: &str) -> SshServiceResult<Vec<FileItemData>> {
        let guard = self.mock_files.read().unwrap();
        if let Some(items) = guard.get(remote_path) {
            Ok(items.clone())
        } else {
            Ok(generate_mock_remote_directory(remote_path))
        }
    }

    async fn stat_path(&self, _session_id: &str, remote_path: &str) -> SshServiceResult<FileItemData> {
        let filename = remote_path.split(['/', '\\']).last().unwrap_or("file");
        Ok(FileItemData::new_file(
            filename,
            remote_path,
            1024 * 64,
            0,
            "-rw-r--r--",
        ))
    }

    async fn create_dir(&self, _session_id: &str, remote_path: &str) -> SshServiceResult<()> {
        let mut guard = self.mock_files.write().unwrap();
        guard.entry(remote_path.to_string()).or_default();
        Ok(())
    }

    async fn remove_path(&self, _session_id: &str, remote_path: &str, _recursive: bool) -> SshServiceResult<()> {
        let mut guard = self.mock_files.write().unwrap();
        guard.remove(remote_path);
        Ok(())
    }

    async fn rename(&self, _session_id: &str, old_path: &str, new_path: &str) -> SshServiceResult<()> {
        let mut guard = self.mock_files.write().unwrap();
        if let Some(items) = guard.remove(old_path) {
            guard.insert(new_path.to_string(), items);
        }
        Ok(())
    }

    async fn download_file(
        &self,
        _session_id: &str,
        _remote_path: &str,
        _local_path: &str,
        progress_tx: Option<UnboundedSender<TransferProgress>>,
    ) -> SshServiceResult<()> {
        if let Some(tx) = progress_tx {
            let total = 1024 * 1024 * 5; // 5 MB
            let _ = tx.send(TransferProgress {
                task_id: "mock-download".to_string(),
                transferred_bytes: total / 2,
                total_bytes: total,
                speed_bytes_per_sec: 1024 * 1024 * 2,
            });
            let _ = tx.send(TransferProgress {
                task_id: "mock-download".to_string(),
                transferred_bytes: total,
                total_bytes: total,
                speed_bytes_per_sec: 1024 * 1024 * 3,
            });
        }
        Ok(())
    }

    async fn upload_file(
        &self,
        _session_id: &str,
        _local_path: &str,
        _remote_path: &str,
        progress_tx: Option<UnboundedSender<TransferProgress>>,
    ) -> SshServiceResult<()> {
        if let Some(tx) = progress_tx {
            let total = 1024 * 512; // 512 KB
            let _ = tx.send(TransferProgress {
                task_id: "mock-upload".to_string(),
                transferred_bytes: total,
                total_bytes: total,
                speed_bytes_per_sec: 1024 * 512,
            });
        }
        Ok(())
    }
}

/// 内存仿真 SSH 终端会话服务
#[derive(Debug, Default, Clone)]
pub struct MockSshSessionService;

#[async_trait]
impl SshSessionService for MockSshSessionService {
    async fn connect_with_progress(
        &self,
        host: &HostRecord,
        _credential: Option<&CredentialRecord>,
        progress: Option<crate::service::ssh::SshProgressCallback>,
    ) -> SshServiceResult<String> {
        if let Some(p) = progress {
            p("Mock SSH 连接中...");
        }
        Ok(format!("mock-session-{}", host.id))
    }

    async fn open_pty_channel(
        &self,
        _session_id: &str,
        _term_type: &str,
        _rows: u16,
        _cols: u16,
    ) -> SshServiceResult<Box<dyn SshStreamChannel>> {
        // 创建双向内存通道并模拟交互 Shell (防止立即 EOF 退出)
        let (client, mut server) = duplex(4096);
        tokio::spawn(async move {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let welcome = "\x1b[32m[smalux] Mock SSH 仿真终端已就绪\x1b[0m\r\n$ ";
            let _ = server.write_all(welcome.as_bytes()).await;
            let mut buf = [0u8; 1024];
            while let Ok(n) = server.read(&mut buf).await {
                if n == 0 {
                    break;
                }
                // 回显输入的字符
                let _ = server.write_all(&buf[..n]).await;
            }
        });
        Ok(Box::new(client))
    }

    async fn resize_pty(&self, _session_id: &str, _rows: u16, _cols: u16) -> SshServiceResult<()> {
        Ok(())
    }

    async fn execute_command(&self, _session_id: &str, command: &str) -> SshServiceResult<CommandExecutionOutput> {
        Ok(CommandExecutionOutput {
            exit_code: 0,
            stdout: format!("Mock stdout output for: {}\n", command).into_bytes(),
            stderr: Vec::new(),
        })
    }

    async fn disconnect(&self, _session_id: &str) -> SshServiceResult<()> {
        Ok(())
    }
}

/// 内存仿真网络隧道与代理服务
#[derive(Debug, Default, Clone)]
pub struct MockTunnelService {
    active_tunnels: Arc<RwLock<HashMap<String, TunnelHandle>>>,
}

#[async_trait]
impl TunnelService for MockTunnelService {
    async fn start_tunnel(
        &self,
        tunnel: &TunnelRecord,
        _credential: Option<&CredentialRecord>,
    ) -> SshServiceResult<TunnelHandle> {
        let handle = TunnelHandle {
            tunnel_id: tunnel.id.clone(),
            bound_address: format!("{}:{}", tunnel.local_bind, tunnel.local_port),
            is_active: true,
        };
        let mut guard = self.active_tunnels.write().unwrap();
        guard.insert(tunnel.id.clone(), handle.clone());
        Ok(handle)
    }

    async fn stop_tunnel(&self, tunnel_id: &str) -> SshServiceResult<()> {
        let mut guard = self.active_tunnels.write().unwrap();
        if guard.remove(tunnel_id).is_some() {
            Ok(())
        } else {
            Err(SshServiceError::NotFound(format!("隧道 {} 不在运行中", tunnel_id)))
        }
    }

    async fn query_metrics(&self, _tunnel_id: &str) -> SshServiceResult<TunnelMetricsSnapshot> {
        Ok(TunnelMetricsSnapshot {
            active_connections: 2,
            bytes_in: 1024 * 128,
            bytes_out: 1024 * 256,
        })
    }
}

/// 内存仿真 SSH 密钥对生成服务
#[derive(Debug, Default, Clone)]
pub struct MockKeygenService;

impl KeygenService for MockKeygenService {
    fn generate_keypair(
        &self,
        algorithm: KeyAlgorithm,
        _bits: Option<u32>,
        _passphrase: Option<&str>,
    ) -> SshServiceResult<GeneratedKeyPair> {
        match algorithm {
            KeyAlgorithm::Ed25519 => Ok(GeneratedKeyPair {
                algorithm,
                public_key_openssh: "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIG5MockPublicKey user@smalux".to_string(),
                private_key_pem: "-----BEGIN OPENSSH PRIVATE KEY-----\nbW9ja1ByaXZhdGVLZXk=\n-----END OPENSSH PRIVATE KEY-----".to_string(),
                fingerprint: "SHA256:MockEd25519Fingerprint".to_string(),
            }),
            KeyAlgorithm::Rsa => Ok(GeneratedKeyPair {
                algorithm,
                public_key_openssh: "ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABAQDMockRsaPublicKey user@smalux".to_string(),
                private_key_pem: "-----BEGIN RSA PRIVATE KEY-----\nbW9ja1JzYVByaXZhdGVLZXk=\n-----END RSA PRIVATE KEY-----".to_string(),
                fingerprint: "SHA256:MockRsaFingerprint".to_string(),
            }),
            KeyAlgorithm::EcdsaP256 => Ok(GeneratedKeyPair {
                algorithm,
                public_key_openssh: "ecdsa-sha2-nistp256 AAAAE2VjZHNhLXNoYTItbmlzdHAyNTY= user@smalux".to_string(),
                private_key_pem: "-----BEGIN EC PRIVATE KEY-----\nbW9ja0VjZHNhUHJpdmF0ZUtleQ==\n-----END EC PRIVATE KEY-----".to_string(),
                fingerprint: "SHA256:MockEcdsaFingerprint".to_string(),
            }),
        }
    }

    fn compute_fingerprint(&self, _public_key_openssh: &str) -> SshServiceResult<String> {
        Ok("SHA256:MockComputedFingerprint123456789".to_string())
    }

    fn parse_private_key(
        &self,
        private_key_pem: &str,
        _passphrase: Option<&str>,
    ) -> SshServiceResult<ParsedKeyInfo> {
        let is_encrypted = private_key_pem.contains("ENCRYPTED") || private_key_pem.contains("aes");
        let (algo, pub_key, fp) = if private_key_pem.contains("RSA") {
            (
                "RSA".to_string(),
                Some("ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABAQDMockRsaPublicKey user@smalux".to_string()),
                Some("SHA256:MockRsaFingerprint".to_string()),
            )
        } else if private_key_pem.contains("EC") {
            (
                "ECDSA-P256".to_string(),
                Some("ecdsa-sha2-nistp256 AAAAE2VjZHNhLXNoYTItbmlzdHAyNTY= user@smalux".to_string()),
                Some("SHA256:MockEcdsaFingerprint".to_string()),
            )
        } else {
            (
                "Ed25519".to_string(),
                Some("ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIG5MockPublicKey user@smalux".to_string()),
                Some("SHA256:MockEd25519Fingerprint".to_string()),
            )
        };

        Ok(ParsedKeyInfo {
            algorithm: algo,
            public_key_openssh: pub_key,
            fingerprint: fp,
            is_encrypted,
            comment: Some("user@smalux".to_string()),
        })
    }
}

/// 内存仿真远程主机性能监控采集服务
#[derive(Debug, Default, Clone)]
pub struct MockHostMetricsService;

#[async_trait]
impl HostMetricsService for MockHostMetricsService {
    async fn sample_metrics(&self, _session_id: &str) -> SshServiceResult<SystemMetricsSnapshot> {
        Ok(SystemMetricsSnapshot {
            cpu_usage_percent: 18.5,
            memory_used_bytes: 1024 * 1024 * 1024 * 4,     // 4 GB
            memory_total_bytes: 1024 * 1024 * 1024 * 16,   // 16 GB
            disk_used_bytes: 1024 * 1024 * 1024 * 120,     // 120 GB
            disk_total_bytes: 1024 * 1024 * 1024 * 512,    // 512 GB
            rx_bytes_per_sec: 1024 * 256,                  // 256 KB/s
            tx_bytes_per_sec: 1024 * 64,                   // 64 KB/s
        })
    }
}
