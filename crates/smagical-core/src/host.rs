//! 主机、分组、认证、代理和跳板机配置。
//!
//! 这个 crate 只定义可序列化的领域数据，不依赖 UI、存储后端或 SSH 执行器。认证字段中
//! 保存的是 `SecretRef` 引用，不是明文密码或私钥内容；真正的秘密读取由 security 层完成。

use serde::{Deserialize, Serialize};

use crate::{BackgroundProfile, GroupId, HostId, SecretRef, ThemeProfile};

/// 可保存的 SSH 主机配置。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Host {
    /// 主机稳定 ID，用于会话、历史、书签和片段关联。
    pub id: HostId,
    /// 用户可编辑的显示名称。
    pub name: String,
    /// 所属树形分组；`None` 表示根分组。
    pub group_id: Option<GroupId>,
    /// UI 图标 key。使用 serde 默认值保证旧数据升级后仍有图标。
    #[serde(default = "default_host_icon_key")]
    pub icon_key: String,
    /// 用户自定义标签，用于筛选和视觉区分。
    #[serde(default)]
    pub tags: Vec<String>,
    /// SSH 目标地址，可以是主机名或 IP。
    pub address: String,
    /// SSH 端口。
    pub port: u16,
    /// 登录认证方式，内部只保存 SecretRef。
    pub auth: AuthProfile,
    /// 连接代理，后续执行器可以按需实现。
    pub proxy: Option<ProxyProfile>,
    /// 跳板机链路，当前以已保存主机 ID 引用。
    pub jumps: Vec<JumpProfile>,
    /// 主机级主题覆盖，不影响全局主题配置。
    pub theme_override: Option<ThemeProfile>,
    /// 主机级背景覆盖，不影响全局背景配置。
    pub background_override: Option<BackgroundProfile>,
}

pub fn default_host_icon_key() -> String {
    // 旧数据没有 icon_key 时使用通用服务器图标。
    "server".to_owned()
}

/// 树形主机分组。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostGroup {
    /// 分组稳定 ID，主机和子分组都通过它引用父级。
    pub id: GroupId,
    /// 分组显示名称。
    pub name: String,
    /// 父分组；`None` 表示挂在根节点。
    pub parent_id: Option<GroupId>,
}

/// SSH 登录认证方式。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthProfile {
    /// 密码认证。`secret` 指向安全存储中的密码。
    Password { username: String, secret: SecretRef },
    /// 私钥认证。`passphrase` 可选，仍然通过 SecretRef 引用。
    Key {
        username: String,
        key: SecretRef,
        passphrase: Option<SecretRef>,
    },
    /// ssh-agent 认证。不保存明文，只记录 agent 来源和可选 key hint。
    Agent {
        username: String,
        #[serde(default)]
        source: AgentSource,
        key_hint: Option<String>,
    },
    /// OpenSSH 证书认证。私钥、口令和证书都用引用表达。
    Certificate {
        username: String,
        key: SecretRef,
        passphrase: Option<SecretRef>,
        certificate: SecretRef,
    },
}

/// ssh-agent 连接来源。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AgentSource {
    /// 自动探测平台默认 agent。
    #[default]
    Auto,
    /// Windows OpenSSH agent named pipe。
    OpenSsh,
    /// Pageant named pipe。
    Pageant,
    /// 用户自定义 named pipe。
    CustomNamedPipe(String),
}

/// 连接代理配置。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProxyProfile {
    /// SOCKS5 代理。
    Socks5 { host: String, port: u16 },
    /// HTTP CONNECT 代理。
    Http { host: String, port: u16 },
}

/// 跳板机链路中的一个节点。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JumpProfile {
    /// 跳板机引用已保存主机，避免在链路里复制整份主机配置。
    pub host_id: HostId,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ImageSource, SecretRef};
    use uuid::Uuid;

    fn secret(name: &str) -> SecretRef {
        SecretRef(name.to_owned())
    }

    fn sample_host() -> Host {
        Host {
            id: HostId(Uuid::new_v4()),
            name: "production".to_owned(),
            group_id: Some(GroupId(Uuid::new_v4())),
            icon_key: default_host_icon_key(),
            tags: vec!["prod".to_owned(), "linux".to_owned()],
            address: "example.com".to_owned(),
            port: 22,
            auth: AuthProfile::Key {
                username: "deploy".to_owned(),
                key: secret("key:deploy"),
                passphrase: Some(secret("passphrase:deploy")),
            },
            proxy: Some(ProxyProfile::Socks5 {
                host: "127.0.0.1".to_owned(),
                port: 1080,
            }),
            jumps: vec![JumpProfile {
                host_id: HostId(Uuid::new_v4()),
            }],
            theme_override: Some(ThemeProfile {
                name: "Host Dark".to_owned(),
                font_family: "JetBrains Mono".to_owned(),
                font_size: 15.0,
            }),
            background_override: Some(BackgroundProfile {
                enabled: true,
                sources: vec![ImageSource::LocalPath("wallpapers/one.png".to_owned())],
                rotation_interval_secs: 60,
                opacity: 0.25,
                blur: 6.0,
            }),
        }
    }

    #[test]
    fn auth_profiles_round_trip_through_toml() {
        #[derive(serde::Serialize, serde::Deserialize)]
        struct AuthProfileList {
            profiles: Vec<AuthProfile>,
        }

        let profile_list = AuthProfileList {
            profiles: vec![
                AuthProfile::Password {
                    username: "root".to_owned(),
                    secret: secret("password:root"),
                },
                AuthProfile::Key {
                    username: "deploy".to_owned(),
                    key: secret("key:deploy"),
                    passphrase: Some(secret("passphrase:deploy")),
                },
                AuthProfile::Agent {
                    username: "agent-user".to_owned(),
                    source: AgentSource::Auto,
                    key_hint: Some("id_ed25519".to_owned()),
                },
                AuthProfile::Certificate {
                    username: "cert-user".to_owned(),
                    key: secret("key:cert-user"),
                    passphrase: Some(secret("passphrase:cert-user")),
                    certificate: secret("cert:cert-user"),
                },
            ],
        };

        let encoded = toml::to_string(&profile_list).expect("认证配置应该可以序列化为 TOML");
        let decoded: AuthProfileList =
            toml::from_str(&encoded).expect("认证配置应该可以从 TOML 反序列化");

        assert_eq!(decoded.profiles.len(), profile_list.profiles.len());
        assert!(matches!(decoded.profiles[0], AuthProfile::Password { .. }));
        assert!(matches!(decoded.profiles[1], AuthProfile::Key { .. }));
        assert!(matches!(decoded.profiles[2], AuthProfile::Agent { .. }));
        assert!(matches!(
            decoded.profiles[3],
            AuthProfile::Certificate { .. }
        ));
    }

    #[test]
    fn host_round_trips_with_overrides() {
        let host = sample_host();

        let encoded = toml::to_string(&host).expect("主机配置应该可以序列化为 TOML");
        let decoded: Host = toml::from_str(&encoded).expect("主机配置应该可以从 TOML 反序列化");

        assert_eq!(decoded.name, host.name);
        assert_eq!(decoded.address, host.address);
        assert_eq!(decoded.tags, host.tags);
        assert_eq!(decoded.port, 22);
        assert!(decoded.proxy.is_some());
        assert_eq!(decoded.jumps.len(), 1);
        assert!(decoded.theme_override.is_some());
        assert!(decoded.background_override.is_some());
    }
}
