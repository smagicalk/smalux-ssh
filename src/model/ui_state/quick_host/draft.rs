//! 快速新增主机表单草稿构建。

use crate::model::{Host, HostId};

use super::{QuickHostAuthDraft, QuickHostDraftError};

/// 快速新增主机表单草稿。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickHostDraft {
    pub name: String,
    pub address: String,
    pub port: String,
    pub username: String,
    pub tags: String,
    pub auth: QuickHostAuthDraft,
}

impl Default for QuickHostDraft {
    fn default() -> Self {
        Self {
            name: String::new(),
            address: String::new(),
            port: "22".to_owned(),
            username: String::new(),
            tags: String::new(),
            auth: QuickHostAuthDraft::default(),
        }
    }
}

impl QuickHostDraft {
    /// 将表单草稿转换为可保存主机。
    pub fn build_host(&self, id: HostId) -> Result<Host, QuickHostDraftError> {
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
        let auth = self.auth.build_auth(username)?;

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
            auth,
            proxy: None,
            jumps: Vec::new(),
            theme_override: None,
            background_override: None,
        })
    }

    /// 兼容旧调用的快捷入口，内部仍走统一主机构建逻辑。
    pub fn build_agent_host(&self, id: HostId) -> Result<Host, QuickHostDraftError> {
        self.build_host(id)
    }
}

fn parse_tags(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}
