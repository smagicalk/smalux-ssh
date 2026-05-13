//! UI 交互草稿状态。
//!
//! 这里仅保存尚未提交给后端的轻量输入值，避免把输入框状态混入 SSH 会话模型。

use std::fmt;

use super::{AuthProfile, Host, HostId};

pub const DEFAULT_REMOTE_COMMAND: &str = "uptime";
pub const DEFAULT_SFTP_INITIAL_DIR: &str = "/";

/// 每台主机在操作区的输入草稿。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostActionDraft {
    pub host_id: HostId,
    pub remote_command: String,
    pub sftp_initial_dir: String,
}

impl HostActionDraft {
    /// 为主机创建可直接使用的默认操作草稿。
    pub fn new(host_id: HostId) -> Self {
        Self {
            host_id,
            remote_command: DEFAULT_REMOTE_COMMAND.to_owned(),
            sftp_initial_dir: DEFAULT_SFTP_INITIAL_DIR.to_owned(),
        }
    }
}

/// 快速新增主机表单草稿。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickHostDraft {
    pub name: String,
    pub address: String,
    pub port: String,
    pub username: String,
    pub key_hint: String,
    pub tags: String,
}

impl Default for QuickHostDraft {
    fn default() -> Self {
        Self {
            name: String::new(),
            address: String::new(),
            port: "22".to_owned(),
            username: String::new(),
            key_hint: String::new(),
            tags: String::new(),
        }
    }
}

impl QuickHostDraft {
    /// 将表单草稿转换为可保存主机；首版快速入口默认使用 ssh-agent。
    pub fn build_agent_host(&self, id: HostId) -> Result<Host, QuickHostDraftError> {
        let address = self.address.trim();
        if address.is_empty() {
            return Err(QuickHostDraftError::EmptyAddress);
        }

        let username = self.username.trim();
        if username.is_empty() {
            return Err(QuickHostDraftError::EmptyUsername);
        }

        let port = self
            .port
            .trim()
            .parse::<u16>()
            .map_err(|_| QuickHostDraftError::InvalidPort)?;
        if port == 0 {
            return Err(QuickHostDraftError::InvalidPort);
        }

        let name = self.name.trim();
        let key_hint = self.key_hint.trim();

        Ok(Host {
            id,
            name: if name.is_empty() {
                address.to_owned()
            } else {
                name.to_owned()
            },
            group_id: None,
            tags: parse_tags(&self.tags),
            address: address.to_owned(),
            port,
            auth: AuthProfile::Agent {
                username: username.to_owned(),
                key_hint: if key_hint.is_empty() {
                    None
                } else {
                    Some(key_hint.to_owned())
                },
            },
            proxy: None,
            jumps: Vec::new(),
            theme_override: None,
            background_override: None,
        })
    }
}

/// 快速新增主机表单字段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickHostDraftField {
    Name,
    Address,
    Port,
    Username,
    KeyHint,
    Tags,
}

/// 快速新增主机表单校验错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuickHostDraftError {
    EmptyAddress,
    EmptyUsername,
    InvalidPort,
}

impl fmt::Display for QuickHostDraftError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyAddress => f.write_str("地址不能为空"),
            Self::EmptyUsername => f.write_str("用户名不能为空"),
            Self::InvalidPort => f.write_str("端口必须是 1 到 65535"),
        }
    }
}

/// 纯 UI 层运行态。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UiState {
    pub quick_host: QuickHostDraft,
    pub host_action_drafts: Vec<HostActionDraft>,
}

impl UiState {
    /// 返回指定主机的远程命令草稿；没有草稿时使用默认命令。
    pub fn remote_command_for(&self, host_id: HostId) -> &str {
        self.host_action_drafts
            .iter()
            .find(|draft| draft.host_id == host_id)
            .map(|draft| draft.remote_command.as_str())
            .unwrap_or(DEFAULT_REMOTE_COMMAND)
    }

    /// 返回指定主机的 SFTP 初始路径草稿；没有草稿时使用根目录。
    pub fn sftp_initial_dir_for(&self, host_id: HostId) -> &str {
        self.host_action_drafts
            .iter()
            .find(|draft| draft.host_id == host_id)
            .map(|draft| draft.sftp_initial_dir.as_str())
            .unwrap_or(DEFAULT_SFTP_INITIAL_DIR)
    }

    /// 更新远程命令输入草稿。
    pub fn set_remote_command(&mut self, host_id: HostId, command: impl Into<String>) {
        self.ensure_host_action_draft(host_id).remote_command = command.into();
    }

    /// 更新 SFTP 初始路径输入草稿。
    pub fn set_sftp_initial_dir(&mut self, host_id: HostId, initial_dir: impl Into<String>) {
        self.ensure_host_action_draft(host_id).sftp_initial_dir = initial_dir.into();
    }

    /// 更新快速新增主机表单字段。
    pub fn set_quick_host_field(&mut self, field: QuickHostDraftField, value: impl Into<String>) {
        let value = value.into();

        match field {
            QuickHostDraftField::Name => self.quick_host.name = value,
            QuickHostDraftField::Address => self.quick_host.address = value,
            QuickHostDraftField::Port => self.quick_host.port = value,
            QuickHostDraftField::Username => self.quick_host.username = value,
            QuickHostDraftField::KeyHint => self.quick_host.key_hint = value,
            QuickHostDraftField::Tags => self.quick_host.tags = value,
        }
    }

    /// 清空快速新增主机表单，保留默认 SSH 端口。
    pub fn reset_quick_host(&mut self) {
        self.quick_host = QuickHostDraft::default();
    }

    fn ensure_host_action_draft(&mut self, host_id: HostId) -> &mut HostActionDraft {
        if let Some(index) = self
            .host_action_drafts
            .iter()
            .position(|draft| draft.host_id == host_id)
        {
            return &mut self.host_action_drafts[index];
        }

        self.host_action_drafts.push(HostActionDraft::new(host_id));
        self.host_action_drafts
            .last_mut()
            .expect("刚插入的主机操作草稿应该存在")
    }
}

fn parse_tags(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn host_id() -> HostId {
        HostId(Uuid::new_v4())
    }

    #[test]
    fn default_values_are_actionable_without_saved_draft() {
        let ui = UiState::default();
        let host_id = host_id();

        assert_eq!(ui.remote_command_for(host_id), "uptime");
        assert_eq!(ui.sftp_initial_dir_for(host_id), "/");
    }

    #[test]
    fn host_action_drafts_are_scoped_per_host() {
        let mut ui = UiState::default();
        let first = host_id();
        let second = host_id();

        ui.set_remote_command(first, "df -h");
        ui.set_sftp_initial_dir(second, "/var/log");

        assert_eq!(ui.remote_command_for(first), "df -h");
        assert_eq!(ui.sftp_initial_dir_for(first), "/");
        assert_eq!(ui.remote_command_for(second), "uptime");
        assert_eq!(ui.sftp_initial_dir_for(second), "/var/log");
        assert_eq!(ui.host_action_drafts.len(), 2);
    }

    #[test]
    fn quick_host_draft_builds_agent_host() {
        let draft = QuickHostDraft {
            name: "prod".to_owned(),
            address: "prod.example.com".to_owned(),
            port: "2222".to_owned(),
            username: "deploy".to_owned(),
            key_hint: "id_ed25519".to_owned(),
            tags: "prod, linux".to_owned(),
        };

        let host = draft
            .build_agent_host(host_id())
            .expect("有效主机草稿应该可以生成主机配置");

        assert_eq!(host.name, "prod");
        assert_eq!(host.address, "prod.example.com");
        assert_eq!(host.port, 2222);
        assert_eq!(host.tags, vec!["prod", "linux"]);
        assert!(matches!(
            host.auth,
            AuthProfile::Agent {
                username,
                key_hint: Some(key_hint),
            } if username == "deploy" && key_hint == "id_ed25519"
        ));
    }

    #[test]
    fn quick_host_draft_validates_required_fields() {
        let draft = QuickHostDraft::default();

        assert_eq!(
            draft.build_agent_host(host_id()),
            Err(QuickHostDraftError::EmptyAddress)
        );

        let missing_user = QuickHostDraft {
            address: "example.com".to_owned(),
            ..QuickHostDraft::default()
        };
        assert_eq!(
            missing_user.build_agent_host(host_id()),
            Err(QuickHostDraftError::EmptyUsername)
        );
    }

    #[test]
    fn quick_host_field_updates_and_reset_are_scoped_to_form() {
        let mut ui = UiState::default();

        ui.set_quick_host_field(QuickHostDraftField::Address, "example.com");
        ui.set_quick_host_field(QuickHostDraftField::Username, "ops");
        ui.reset_quick_host();

        assert_eq!(ui.quick_host.address, "");
        assert_eq!(ui.quick_host.username, "");
        assert_eq!(ui.quick_host.port, "22");
    }
}
