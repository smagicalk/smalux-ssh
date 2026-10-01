//! # OpenSSH Known Hosts 原生文件管理与主机公钥验真引擎 (TOFU)
//!
//! 提供基于纯 Rust 的 `~/.ssh/known_hosts` 解析、验证、防劫持报警与自动受信任追加功能：
//! 1. 严格兼容 OpenSSH 标准已知主机条目格式；
//! 2. 支持普通主机名、IP、带端口格式 `[hostname]:port`；
//! 3. 首次连接智能信任并自动记录（TOFU）；
//! 4. 实时计算 SHA256 指纹，若发生公钥变更则触发安全警报并断开连接。

use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use base64::prelude::*;
use russh::keys::PublicKeyOrCertificate;
use smagical_core::service::HostKeyVerificationResult;

/// 默认获取当前用户主目录下的 OpenSSH known_hosts 文件路径
pub fn get_default_known_hosts_path() -> Option<PathBuf> {
    directories::UserDirs::new().map(|u| u.home_dir().join(".ssh").join("known_hosts"))
}

/// 解析单行 known_hosts 记录中的主机模式、算法与公钥 Base64
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownHostEntry {
    /// 主机模式字段 (如 "192.168.1.100", "[10.0.0.1]:2222", "github.com")
    pub host_patterns: Vec<String>,
    /// 公钥算法 (如 "ssh-ed25519", "ssh-rsa", "ecdsa-sha2-nistp256")
    pub algorithm: String,
    /// 公钥数据的 Base64 字符串
    pub key_base64: String,
}

impl KnownHostEntry {
    /// 从单行 OpenSSH known_hosts 纯文本解析
    pub fn parse_line(line: &str) -> Option<Self> {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return None;
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() < 3 {
            return None;
        }

        // 跳过散列已知主机 (|1|salt|hash) 或标记 (@cert-authority, @revoked)
        if parts[0].starts_with('|') || parts[0].starts_with('@') {
            return None;
        }

        let host_patterns = parts[0].split(',').map(|s| s.to_string()).collect();
        let algorithm = parts[1].to_string();
        let key_base64 = parts[2].to_string();

        Some(Self {
            host_patterns,
            algorithm,
            key_base64,
        })
    }

    /// 判断当前条目是否匹配指定的主机和端口
    pub fn matches(&self, host: &str, port: u16) -> bool {
        let target_pattern_with_port = format!("[{}]:{}", host, port);
        for pattern in &self.host_patterns {
            if port == 22 && (pattern == host || pattern == &target_pattern_with_port) {
                return true;
            }
            if pattern == &target_pattern_with_port {
                return true;
            }
        }
        false
    }
}

/// 提取 russh 远端公钥的基础算法名称、Base64 编码与 SHA256 指纹
pub fn extract_server_public_key_info(server_key: &PublicKeyOrCertificate) -> (String, String, String) {
    match server_key {
        PublicKeyOrCertificate::PublicKey { key, .. } => {
            let algo = key.algorithm().as_str().to_string();
            let fp = key.fingerprint(ssh_key::HashAlg::Sha256).to_string();
            let full_openssh = key.to_openssh().unwrap_or_default();
            let parts: Vec<&str> = full_openssh.split_whitespace().collect();
            let b64 = if parts.len() >= 2 { parts[1].to_string() } else { String::new() };
            (algo, b64, fp)
        }
        PublicKeyOrCertificate::Certificate(cert) => {
            let pk = ssh_key::PublicKey::new(cert.public_key().clone(), "");
            let algo = pk.algorithm().as_str().to_string();
            let fp = pk.fingerprint(ssh_key::HashAlg::Sha256).to_string();
            let full_openssh = pk.to_openssh().unwrap_or_default();
            let parts: Vec<&str> = full_openssh.split_whitespace().collect();
            let b64 = if parts.len() >= 2 { parts[1].to_string() } else { String::new() };
            (algo, b64, fp)
        }
    }
}

/// 针对指定 known_hosts 文件校验服务器公钥
pub fn verify_server_key_in_file(
    file_path: &Path,
    host: &str,
    port: u16,
    server_key: &PublicKeyOrCertificate,
) -> Result<HostKeyVerificationResult> {
    let (algo, actual_b64, actual_fp) = extract_server_public_key_info(server_key);
    let public_key_text = format!("{} {}", algo, actual_b64);

    if !file_path.exists() {
        return Ok(HostKeyVerificationResult::FirstTimeHost {
            fingerprint: actual_fp,
            public_key_text,
        });
    }

    let file = fs::File::open(file_path)
        .with_context(|| format!("读取 known_hosts 失败: {:?}", file_path))?;
    let reader = BufReader::new(file);

    for line in reader.lines().map_while(Result::ok) {
        if let Some(entry) = KnownHostEntry::parse_line(&line) {
            if entry.matches(host, port) {
                if entry.key_base64 == actual_b64 {
                    return Ok(HostKeyVerificationResult::Trusted);
                } else {
                    // 已记录过该主机，但公钥 Base64 不匹配！计算历史公钥的指纹进行详细报警
                    use ssh_key::sha2::{Digest, Sha256};
                    let expected_raw = BASE64_STANDARD.decode(&entry.key_base64).unwrap_or_default();
                    let mut hasher = Sha256::new();
                    hasher.update(&expected_raw);
                    let exp_fp = format!("SHA256:{}", BASE64_STANDARD_NO_PAD.encode(hasher.finalize()));

                    return Ok(HostKeyVerificationResult::Mismatch {
                        expected_fingerprint: exp_fp,
                        actual_fingerprint: actual_fp,
                    });
                }
            }
        }
    }

    // 文件中不存在该主机的匹配条目
    Ok(HostKeyVerificationResult::FirstTimeHost {
        fingerprint: actual_fp,
        public_key_text,
    })
}

/// 将新主机的公钥追加至指定的 known_hosts 文件
pub fn append_known_host_to_file(
    file_path: &Path,
    host: &str,
    port: u16,
    server_key: &PublicKeyOrCertificate,
) -> Result<()> {
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("创建 known_hosts 父目录失败: {:?}", parent))?;
    }

    let (algo, b64_key, _) = extract_server_public_key_info(server_key);
    let host_pattern = if port == 22 {
        host.to_string()
    } else {
        format!("[{}]:{}", host, port)
    };

    let entry_line = format!("{} {} {}\n", host_pattern, algo, b64_key);

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)
        .with_context(|| format!("写入 known_hosts 文件失败: {:?}", file_path))?;

    file.write_all(entry_line.as_bytes())
        .with_context(|| format!("追加 known_hosts 记录失败: {:?}", file_path))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_known_host_entry() {
        let line = "192.168.1.100,prod.corp ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIG... user@host";
        let entry = KnownHostEntry::parse_line(line).unwrap();
        assert_eq!(entry.algorithm, "ssh-ed25519");
        assert_eq!(entry.host_patterns, vec!["192.168.1.100", "prod.corp"]);
        assert_eq!(entry.key_base64, "AAAAC3NzaC1lZDI1NTE5AAAAIG...");
        assert!(entry.matches("192.168.1.100", 22));
        assert!(entry.matches("prod.corp", 22));
        assert!(!entry.matches("other.com", 22));
    }

    #[test]
    fn test_parse_port_entry() {
        let line = "[10.0.0.1]:2222 ssh-rsa AAAAB3NzaC1yc2E...";
        let entry = KnownHostEntry::parse_line(line).unwrap();
        assert_eq!(entry.host_patterns, vec!["[10.0.0.1]:2222"]);
        assert!(entry.matches("10.0.0.1", 2222));
        assert!(!entry.matches("10.0.0.1", 22));
    }

    #[test]
    fn test_verify_and_append_lifecycle() {
        let mut rng = rand::rng();
        let priv_key_1 = ssh_key::PrivateKey::random(&mut rng, ssh_key::Algorithm::Ed25519).unwrap();
        let pub_key_1 = priv_key_1.public_key().clone();
        let russh_key_1 = PublicKeyOrCertificate::PublicKey {
            key: pub_key_1,
            hash_alg: None,
        };

        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join(format!("known_hosts_test_{}.tmp", uuid::Uuid::new_v4().simple()));

        // 1. 首次校验应判定为 FirstTimeHost
        let res_first = verify_server_key_in_file(&temp_file, "192.168.1.50", 22, &russh_key_1).unwrap();
        assert!(matches!(res_first, HostKeyVerificationResult::FirstTimeHost { .. }));

        // 2. 追加至文件
        append_known_host_to_file(&temp_file, "192.168.1.50", 22, &russh_key_1).unwrap();

        // 3. 再次校验应判定为 Trusted
        let res_trusted = verify_server_key_in_file(&temp_file, "192.168.1.50", 22, &russh_key_1).unwrap();
        assert_eq!(res_trusted, HostKeyVerificationResult::Trusted);

        // 4. 伪造篡改的公钥应判定为 Mismatch (防劫持警报)
        let priv_key_2 = ssh_key::PrivateKey::random(&mut rng, ssh_key::Algorithm::Ed25519).unwrap();
        let russh_key_2 = PublicKeyOrCertificate::PublicKey {
            key: priv_key_2.public_key().clone(),
            hash_alg: None,
        };
        let res_mismatch = verify_server_key_in_file(&temp_file, "192.168.1.50", 22, &russh_key_2).unwrap();
        assert!(matches!(res_mismatch, HostKeyVerificationResult::Mismatch { .. }));

        let _ = std::fs::remove_file(&temp_file);
    }
}
