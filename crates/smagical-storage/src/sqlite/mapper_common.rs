use uuid::Uuid;

use smagical_core::{
    AgentSource, AuthProfile, CredentialKind, HostId, KeyAlgorithm, SecretMaterialKind,
    SnippetScope, SnippetShell, TunnelKind,
};

use crate::StoragePersistenceError;

#[derive(serde::Serialize, serde::Deserialize)]
struct StringListToml {
    items: Vec<String>,
}

pub(super) fn default_auth() -> AuthProfile {
    // 缺失认证行时使用空用户名的 agent 认证，避免旧库加载崩溃；UI 保存后会写回完整配置。
    AuthProfile::Agent {
        username: String::new(),
        source: AgentSource::Auto,
        key_hint: None,
    }
}

pub(super) fn agent_source_to_parts(source: &AgentSource) -> (String, Option<String>) {
    // 自定义 named pipe 需要额外保存 pipe，其余 agent source 只保存枚举 key。
    match source {
        AgentSource::Auto => ("auto".to_owned(), None),
        AgentSource::OpenSsh => ("openssh".to_owned(), None),
        AgentSource::Pageant => ("pageant".to_owned(), None),
        AgentSource::CustomNamedPipe(pipe) => ("custom_named_pipe".to_owned(), Some(pipe.clone())),
    }
}

pub(super) fn agent_source_from_parts(source: Option<&str>, pipe: Option<String>) -> AgentSource {
    match source {
        Some("openssh") => AgentSource::OpenSsh,
        Some("pageant") => AgentSource::Pageant,
        Some("custom_named_pipe") => AgentSource::CustomNamedPipe(pipe.unwrap_or_default()),
        _ => AgentSource::Auto,
    }
}

pub(super) fn credential_kind_to_str(kind: &CredentialKind) -> &'static str {
    match kind {
        CredentialKind::Password => "password",
        CredentialKind::PrivateKey => "private_key",
        CredentialKind::Agent => "agent",
        CredentialKind::Certificate => "certificate",
    }
}

pub(super) fn credential_kind_from_str(kind: &str) -> CredentialKind {
    match kind {
        "private_key" => CredentialKind::PrivateKey,
        "agent" => CredentialKind::Agent,
        "certificate" => CredentialKind::Certificate,
        _ => CredentialKind::Password,
    }
}

pub(super) fn secret_material_kind_to_string(kind: &SecretMaterialKind) -> String {
    match kind {
        SecretMaterialKind::Password => "password".to_owned(),
        SecretMaterialKind::PrivateKey => "private_key".to_owned(),
        SecretMaterialKind::Passphrase => "passphrase".to_owned(),
        SecretMaterialKind::Certificate => "certificate".to_owned(),
        SecretMaterialKind::Unknown(value) => value.clone(),
    }
}

pub(super) fn secret_material_kind_from_str(kind: &str) -> SecretMaterialKind {
    match kind {
        "password" => SecretMaterialKind::Password,
        "private_key" => SecretMaterialKind::PrivateKey,
        "passphrase" => SecretMaterialKind::Passphrase,
        "certificate" => SecretMaterialKind::Certificate,
        value => SecretMaterialKind::Unknown(value.to_owned()),
    }
}

pub(super) fn key_algorithm_to_parts(algorithm: &KeyAlgorithm) -> (Option<String>, Option<String>) {
    match algorithm {
        KeyAlgorithm::Ed25519 => (Some("ed25519".to_owned()), None),
        KeyAlgorithm::Rsa => (Some("rsa".to_owned()), None),
        KeyAlgorithm::Ecdsa => (Some("ecdsa".to_owned()), None),
        KeyAlgorithm::Unknown(value) => (Some("unknown".to_owned()), Some(value.clone())),
    }
}

pub(super) fn key_algorithm_from_parts(kind: &str, raw: Option<&str>) -> KeyAlgorithm {
    match kind {
        "ed25519" => KeyAlgorithm::Ed25519,
        "rsa" => KeyAlgorithm::Rsa,
        "ecdsa" => KeyAlgorithm::Ecdsa,
        _ => KeyAlgorithm::Unknown(raw.unwrap_or(kind).to_owned()),
    }
}

pub(super) fn snippet_scope_to_parts(scope: &SnippetScope) -> (&'static str, Option<String>) {
    match scope {
        SnippetScope::Global => ("global", None),
        SnippetScope::Host(id) => ("host", Some(id.0.to_string())),
    }
}

pub(super) fn snippet_scope_from_parts(
    kind: &str,
    target_id: Option<&str>,
) -> Result<SnippetScope, StoragePersistenceError> {
    // host scope 必须带 target_id，缺失说明数据库数据不完整。
    Ok(match kind {
        "host" => SnippetScope::Host(HostId(parse_uuid(required_str(
            target_id,
            "snippets.scope_target_id",
        )?)?)),
        _ => SnippetScope::Global,
    })
}

pub(super) fn snippet_shell_to_parts(shell: &SnippetShell) -> (&'static str, Option<String>) {
    match shell {
        SnippetShell::Sh => ("sh", None),
        SnippetShell::Bash => ("bash", None),
        SnippetShell::Zsh => ("zsh", None),
        SnippetShell::PowerShell => ("powershell", None),
        SnippetShell::Cmd => ("cmd", None),
        SnippetShell::Custom(value) => ("custom", Some(value.clone())),
    }
}

pub(super) fn snippet_shell_from_parts(kind: &str, custom: Option<String>) -> SnippetShell {
    match kind {
        "sh" => SnippetShell::Sh,
        "zsh" => SnippetShell::Zsh,
        "powershell" => SnippetShell::PowerShell,
        "cmd" => SnippetShell::Cmd,
        "custom" => SnippetShell::Custom(custom.unwrap_or_default()),
        _ => SnippetShell::Bash,
    }
}

pub(super) fn tunnel_kind_to_str(kind: &TunnelKind) -> &'static str {
    match kind {
        TunnelKind::Local => "local",
        TunnelKind::Remote => "remote",
        TunnelKind::Dynamic => "dynamic",
    }
}

pub(super) fn tunnel_kind_from_str(kind: &str) -> TunnelKind {
    match kind {
        "local" => TunnelKind::Local,
        "remote" => TunnelKind::Remote,
        _ => TunnelKind::Dynamic,
    }
}

pub(super) fn encode_optional_toml<T: serde::Serialize>(
    value: Option<&T>,
) -> Result<Option<String>, StoragePersistenceError> {
    // 复杂扩展字段直接以 TOML 存储，避免 schema 为少量 override 过度膨胀。
    value.map(toml::to_string).transpose().map_err(Into::into)
}

pub(super) fn decode_optional_toml<T: serde::de::DeserializeOwned>(
    value: Option<&str>,
) -> Result<Option<T>, StoragePersistenceError> {
    value.map(toml::from_str).transpose().map_err(Into::into)
}

pub(super) fn encode_string_list_toml(items: &[String]) -> Result<String, StoragePersistenceError> {
    toml::to_string(&StringListToml {
        items: items.to_vec(),
    })
    .map_err(Into::into)
}

pub(super) fn decode_string_list_toml(value: &str) -> Result<Vec<String>, StoragePersistenceError> {
    Ok(toml::from_str::<StringListToml>(value)?.items)
}

pub(super) fn parse_uuid(value: &str) -> Result<Uuid, StoragePersistenceError> {
    // 数据库中 UUID 全部以字符串保存，加载时统一校验格式。
    Uuid::parse_str(value).map_err(|error| StoragePersistenceError::InvalidData(error.to_string()))
}

pub(super) fn to_u16(value: i32) -> Result<u16, StoragePersistenceError> {
    u16::try_from(value)
        .map_err(|_| StoragePersistenceError::InvalidData(format!("数值超出 u16 范围：{value}")))
}

pub(super) fn to_u64(value: i64) -> Result<u64, StoragePersistenceError> {
    u64::try_from(value)
        .map_err(|_| StoragePersistenceError::InvalidData(format!("数值超出 u64 范围：{value}")))
}

pub(super) fn required_field(
    value: Option<String>,
    field: &'static str,
) -> Result<String, StoragePersistenceError> {
    value.ok_or_else(|| StoragePersistenceError::InvalidData(format!("缺少字段：{field}")))
}

pub(super) fn required_str<'a>(
    value: Option<&'a str>,
    field: &'static str,
) -> Result<&'a str, StoragePersistenceError> {
    value.ok_or_else(|| StoragePersistenceError::InvalidData(format!("缺少字段：{field}")))
}

pub(super) fn principals_to_text(principals: &[String]) -> Option<String> {
    if principals.is_empty() {
        None
    } else {
        Some(principals.join("\n"))
    }
}

pub(super) fn principals_from_text(value: Option<&str>) -> Vec<String> {
    value
        .into_iter()
        .flat_map(str::lines)
        .map(str::trim)
        .filter(|principal| !principal.is_empty())
        .map(str::to_owned)
        .collect()
}
