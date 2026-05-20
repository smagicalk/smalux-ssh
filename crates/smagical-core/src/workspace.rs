//! 工作区、窗口和分屏恢复状态。

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{HostId, SessionId, SessionKind, WorkspaceId};

/// 可持久化的工作区状态。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceState {
    pub id: WorkspaceId,
    pub name: String,
    pub tabs: Vec<WorkspaceTabSnapshot>,
    pub active_tab: Option<SessionId>,
    pub layout: Option<WorkspaceLayoutNode>,
    pub window: WindowState,
}

impl WorkspaceState {
    /// 创建一个空工作区。
    pub fn empty(name: impl Into<String>) -> Self {
        Self {
            id: WorkspaceId(Uuid::new_v4()),
            name: name.into(),
            tabs: Vec::new(),
            active_tab: None,
            layout: None,
            window: WindowState::default(),
        }
    }

    /// 保存或替换标签页快照。
    pub fn upsert_tab(&mut self, tab: WorkspaceTabSnapshot) {
        self.tabs
            .retain(|existing| existing.session_id != tab.session_id);
        self.active_tab = Some(tab.session_id);
        self.tabs.push(tab);
    }

    /// 关闭标签页快照。
    pub fn close_tab(&mut self, session_id: SessionId) -> bool {
        let before = self.tabs.len();
        self.tabs.retain(|tab| tab.session_id != session_id);

        if self.active_tab == Some(session_id) {
            self.active_tab = self.tabs.last().map(|tab| tab.session_id);
        }

        before != self.tabs.len()
    }

    /// 工作区是否还有可恢复标签。
    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }

    /// 生成一个按顺序二分的最小布局树。
    pub fn rebuild_linear_layout(&mut self, axis: SplitAxis) {
        self.layout = WorkspaceLayoutNode::from_tabs(&self.tabs, axis);
    }
}

/// 工作区标签页快照。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceTabSnapshot {
    pub session_id: SessionId,
    pub host_id: Option<HostId>,
    pub kind: SessionKind,
    pub title: String,
    pub working_directory: Option<String>,
}

/// 分屏布局节点。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WorkspaceLayoutNode {
    Leaf {
        session_id: SessionId,
    },
    Split {
        axis: SplitAxis,
        ratio: f32,
        first: Box<WorkspaceLayoutNode>,
        second: Box<WorkspaceLayoutNode>,
    },
}

impl WorkspaceLayoutNode {
    /// 从标签页顺序生成简单分屏布局。
    pub fn from_tabs(tabs: &[WorkspaceTabSnapshot], axis: SplitAxis) -> Option<Self> {
        let mut tab_iter = tabs.iter();
        let first = tab_iter.next()?;
        let mut node = WorkspaceLayoutNode::Leaf {
            session_id: first.session_id,
        };

        for tab in tab_iter {
            node = WorkspaceLayoutNode::Split {
                axis,
                ratio: 0.5,
                first: Box::new(node),
                second: Box::new(WorkspaceLayoutNode::Leaf {
                    session_id: tab.session_id,
                }),
            };
        }

        Some(node)
    }

    /// 统计布局树中的叶子节点数量。
    pub fn leaf_count(&self) -> usize {
        match self {
            WorkspaceLayoutNode::Leaf { .. } => 1,
            WorkspaceLayoutNode::Split { first, second, .. } => {
                first.leaf_count() + second.leaf_count()
            }
        }
    }

    /// 返回适合渲染层使用的分割比例。
    pub fn normalized_ratio(&self) -> Option<f32> {
        match self {
            WorkspaceLayoutNode::Leaf { .. } => None,
            WorkspaceLayoutNode::Split { ratio, .. } => Some(ratio.clamp(0.1, 0.9)),
        }
    }
}

/// 分屏方向。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SplitAxis {
    Horizontal,
    Vertical,
}

/// 窗口恢复状态。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowState {
    pub width: u32,
    pub height: u32,
    pub maximized: bool,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 800,
            maximized: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_tracks_tabs_and_active_tab() {
        let mut workspace = WorkspaceState::empty("default");
        let first_id = SessionId(Uuid::new_v4());
        let second_id = SessionId(Uuid::new_v4());

        assert!(workspace.is_empty());

        workspace.upsert_tab(WorkspaceTabSnapshot {
            session_id: first_id,
            host_id: Some(HostId(Uuid::new_v4())),
            kind: SessionKind::Shell,
            title: "first".to_owned(),
            working_directory: Some("/home/ops".to_owned()),
        });
        workspace.upsert_tab(WorkspaceTabSnapshot {
            session_id: second_id,
            host_id: None,
            kind: SessionKind::RemoteCommand {
                command: "uptime".to_owned(),
                history_id: None,
            },
            title: "uptime".to_owned(),
            working_directory: None,
        });

        assert_eq!(workspace.tabs.len(), 2);
        assert_eq!(workspace.active_tab, Some(second_id));
        assert!(workspace.close_tab(second_id));
        assert_eq!(workspace.active_tab, Some(first_id));
        assert!(!workspace.close_tab(second_id));
    }

    #[test]
    fn workspace_layout_counts_leaves_and_normalizes_ratio() {
        let first_id = SessionId(Uuid::new_v4());
        let second_id = SessionId(Uuid::new_v4());
        let layout = WorkspaceLayoutNode::Split {
            axis: SplitAxis::Horizontal,
            ratio: 2.0,
            first: Box::new(WorkspaceLayoutNode::Leaf {
                session_id: first_id,
            }),
            second: Box::new(WorkspaceLayoutNode::Leaf {
                session_id: second_id,
            }),
        };

        assert_eq!(layout.leaf_count(), 2);
        assert_eq!(layout.normalized_ratio(), Some(0.9));
    }

    #[test]
    fn workspace_rebuilds_linear_layout_from_tabs() {
        let first_id = SessionId(Uuid::new_v4());
        let second_id = SessionId(Uuid::new_v4());
        let third_id = SessionId(Uuid::new_v4());
        let mut workspace = WorkspaceState::empty("layout");

        for session_id in [first_id, second_id, third_id] {
            workspace.upsert_tab(WorkspaceTabSnapshot {
                session_id,
                host_id: None,
                kind: SessionKind::Shell,
                title: "shell".to_owned(),
                working_directory: None,
            });
        }

        workspace.rebuild_linear_layout(SplitAxis::Vertical);

        let layout = workspace.layout.as_ref().expect("有标签页时应该生成布局树");
        assert_eq!(layout.leaf_count(), 3);
        assert_eq!(layout.normalized_ratio(), Some(0.5));
    }

    #[test]
    fn workspace_round_trips_through_toml() {
        let session_id = SessionId(Uuid::new_v4());
        let mut workspace = WorkspaceState::empty("restore");
        workspace.window = WindowState {
            width: 1600,
            height: 900,
            maximized: true,
        };
        workspace.upsert_tab(WorkspaceTabSnapshot {
            session_id,
            host_id: Some(HostId(Uuid::new_v4())),
            kind: SessionKind::Sftp,
            title: "SFTP /home/ops".to_owned(),
            working_directory: Some("/home/ops".to_owned()),
        });
        workspace.layout = Some(WorkspaceLayoutNode::Leaf { session_id });

        let encoded = toml::to_string(&workspace).expect("工作区状态应该可以序列化为 TOML");
        let decoded: WorkspaceState =
            toml::from_str(&encoded).expect("工作区状态应该可以从 TOML 反序列化");

        assert_eq!(decoded, workspace);
    }
}
