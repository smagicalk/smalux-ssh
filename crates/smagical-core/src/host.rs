//! 主机、分组、认证、代理和跳板机配置。

use serde::{Deserialize, Serialize};

use crate::{BackgroundProfile, GroupId, HostId, SecretRef, ThemeProfile};

/// 可保存的 SSH 主机配置。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Host {
    pub id: HostId,
    pub name: String,
    pub group_id: Option<GroupId>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub address: String,
    pub port: u16,
    pub auth: AuthProfile,
    pub proxy: Option<ProxyProfile>,
    pub jumps: Vec<JumpProfile>,
    pub theme_override: Option<ThemeProfile>,
    pub background_override: Option<BackgroundProfile>,
}

/// 树形主机分组。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostGroup {
    pub id: GroupId,
    pub name: String,
    pub parent_id: Option<GroupId>,
}

/// SSH 登录认证方式。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthProfile {
    Password {
        username: String,
        secret: SecretRef,
    },
    Key {
        username: String,
        key: SecretRef,
        passphrase: Option<SecretRef>,
    },
    Agent {
        username: String,
        key_hint: Option<String>,
    },
    Certificate {
        username: String,
        key: SecretRef,
        passphrase: Option<SecretRef>,
        certificate: SecretRef,
    },
}

/// 连接代理配置。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProxyProfile {
    Socks5 { host: String, port: u16 },
    Http { host: String, port: u16 },
}

/// 跳板机链路中的一个节点。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JumpProfile {
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
