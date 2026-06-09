//! 工作区一级页面状态。

use serde::{Deserialize, Serialize};

/// 当前显示的一级页面。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkspacePage {
    Hosts,
    Terminal,
    Sftp,
    Tunnels,
    Snippets,
    History,
    Security,
    Proxy,
    Settings,
}

impl Default for WorkspacePage {
    fn default() -> Self {
        Self::Hosts
    }
}
