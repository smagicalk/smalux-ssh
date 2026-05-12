use iced::Task;
use iced::Theme;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::AppConfig;
use crate::session::SessionManager;
use crate::storage::StorageManager;
use crate::terminal::TerminalManager;

#[derive(Debug, Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub sessions: SessionManager,
    pub storage: StorageManager,
    pub terminal: TerminalManager,
    pub theme: Theme,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            config: AppConfig::default(),
            sessions: SessionManager::default(),
            storage: StorageManager::default(),
            terminal: TerminalManager::default(),
            theme: Theme::Dark,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    ToggleTheme,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HostId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GroupId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub Uuid);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SecretRef(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Host {
    pub id: HostId,
    pub name: String,
    pub group_id: Option<GroupId>,
    pub address: String,
    pub port: u16,
    pub auth: AuthProfile,
    pub proxy: Option<ProxyProfile>,
    pub jumps: Vec<JumpProfile>,
    pub theme_override: Option<ThemeProfile>,
    pub background_override: Option<BackgroundProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostGroup {
    pub id: GroupId,
    pub name: String,
    pub parent_id: Option<GroupId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
        certificate: SecretRef,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProxyProfile {
    Socks5 { host: String, port: u16 },
    Http { host: String, port: u16 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JumpProfile {
    pub host_id: HostId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelRule {
    pub name: String,
    pub kind: TunnelKind,
    pub bind_host: String,
    pub bind_port: u16,
    pub target_host: String,
    pub target_port: u16,
    pub auto_start: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TunnelKind {
    Local,
    Remote,
    Dynamic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeProfile {
    pub name: String,
    pub font_family: String,
    pub font_size: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundProfile {
    pub enabled: bool,
    pub sources: Vec<ImageSource>,
    pub rotation_interval_secs: u64,
    pub opacity: f32,
    pub blur: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImageSource {
    LocalPath(String),
    Url(String),
}

impl AppState {
    pub fn boot() -> (Self, Task<Message>) {
        (Self::default(), Task::none())
    }

    pub fn apply(&mut self, message: Message) {
        match message {
            Message::ToggleTheme => {
                self.theme = if matches!(self.theme, Theme::Dark) {
                    Theme::Light
                } else {
                    Theme::Dark
                };
            }
        }
    }
}
