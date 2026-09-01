//! SSH 凭据与密钥资产记录领域模型 (Credential Domain Model)。

use serde::{Deserialize, Serialize};

/// 凭据类型枚举。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CredentialType {
    /// SSH 私钥/公私钥对 (Ed25519, RSA, ECDSA 等)
    #[default]
    Key,
    /// 账号与密码
    Password,
    /// 外部 SSH Agent 代理 (OpenSSH Pipe, 1Password, Pageant 等)
    Agent,
    /// CA 签发的 SSH 证书
    Certificate,
}

impl CredentialType {
    /// 获取静态字符串标识
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Key => "key",
            Self::Password => "password",
            Self::Agent => "agent",
            Self::Certificate => "certificate",
        }
    }
}

impl std::fmt::Display for CredentialType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for CredentialType {
    fn from(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "key" | "ssh_key" | "private_key" => Self::Key,
            "password" | "pwd" => Self::Password,
            "agent" | "ssh_agent" => Self::Agent,
            "certificate" | "cert" => Self::Certificate,
            _ => Self::Key,
        }
    }
}

/// 凭据资产记录模型。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CredentialRecord {
    /// 凭据唯一 ID (如 "cred-prod-ed25519", "cred-root-password")
    pub id: String,
    /// 凭据展示名称
    pub name: String,
    /// 凭据类型 (Key / Password / Agent / Certificate)
    pub cred_type: CredentialType,
    /// 算法或子类型 (如 "Ed25519", "RSA-4096", "Password", "OpenSSH Agent", "1Password")
    pub algorithm: String,
    /// 默认绑定用户名 (如 "root", "ubuntu")
    pub username: Option<String>,
    /// 敏感保密数据 (私钥 PEM 文本 / 密码明文 / Agent 管道路径)
    pub secret_data: String,
    /// 私钥保护口令 (Passphrase，可选)
    pub passphrase: Option<String>,
    /// 公钥文本内容 (如 "ssh-ed25519 AAAAC3... user@smalux")
    pub public_key: Option<String>,
    /// 公钥指纹 (如 "SHA256:8f4a...e12")
    pub fingerprint: Option<String>,
    /// 关联的主机数量计数
    pub bound_host_count: usize,
    /// 创建时间
    pub created_at: String,
    /// 最近更新时间
    pub updated_at: String,
    /// 备注说明
    pub notes: String,
}

impl CredentialRecord {
    /// 创建一个 SSH 密钥凭据
    #[allow(clippy::too_many_arguments)]
    pub fn new_key(
        id: impl Into<String>,
        name: impl Into<String>,
        algorithm: impl Into<String>,
        private_key_pem: impl Into<String>,
        passphrase: Option<String>,
        public_key: Option<String>,
        fingerprint: Option<String>,
        notes: impl Into<String>,
    ) -> Self {
        let algo_str = algorithm.into();
        Self {
            id: id.into(),
            name: name.into(),
            cred_type: CredentialType::Key,
            algorithm: if algo_str.is_empty() { "Ed25519".to_string() } else { algo_str },
            username: None,
            secret_data: private_key_pem.into(),
            passphrase,
            public_key,
            fingerprint,
            bound_host_count: 0,
            created_at: "2026-09-01 12:00:00".to_string(),
            updated_at: "2026-09-01 12:00:00".to_string(),
            notes: notes.into(),
        }
    }

    /// 创建一个账号密码凭据
    pub fn new_password(
        id: impl Into<String>,
        name: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
        notes: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            cred_type: CredentialType::Password,
            algorithm: "Password".to_string(),
            username: Some(username.into()),
            secret_data: password.into(),
            passphrase: None,
            public_key: None,
            fingerprint: None,
            bound_host_count: 0,
            created_at: "2026-09-01 12:00:00".to_string(),
            updated_at: "2026-09-01 12:00:00".to_string(),
            notes: notes.into(),
        }
    }

    /// 创建一个 SSH Agent 凭据
    pub fn new_agent(
        id: impl Into<String>,
        name: impl Into<String>,
        agent_type: impl Into<String>,
        pipe_path: impl Into<String>,
        fingerprint: Option<String>,
        notes: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            cred_type: CredentialType::Agent,
            algorithm: agent_type.into(),
            username: None,
            secret_data: pipe_path.into(),
            passphrase: None,
            public_key: None,
            fingerprint,
            bound_host_count: 0,
            created_at: "2026-09-01 12:00:00".to_string(),
            updated_at: "2026-09-01 12:00:00".to_string(),
            notes: notes.into(),
        }
    }
}
