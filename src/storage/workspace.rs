//! 工作区快照的内存保存操作。

use crate::model::WorkspaceState;

use super::StorageManager;

impl StorageManager {
    /// 保存工作区快照。
    pub fn save_workspace(&mut self, workspace: WorkspaceState) {
        self.workspace = Some(workspace);
    }

    /// 清除工作区快照。
    pub fn clear_workspace(&mut self) -> bool {
        let existed = self.workspace.is_some();
        self.workspace = None;
        existed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{HostId, SessionId, SessionKind, WorkspaceTabSnapshot};
    use uuid::Uuid;

    #[test]
    fn workspace_can_be_saved_and_cleared() {
        let mut storage = StorageManager::default();
        let mut workspace = WorkspaceState::empty("restore");

        workspace.upsert_tab(WorkspaceTabSnapshot {
            session_id: SessionId(Uuid::new_v4()),
            host_id: Some(HostId(Uuid::new_v4())),
            kind: SessionKind::Shell,
            title: "production".to_owned(),
            working_directory: Some("/home/ops".to_owned()),
        });

        storage.save_workspace(workspace);

        assert_eq!(storage.workspace_tab_count(), 1);
        assert!(storage.clear_workspace());
        assert_eq!(storage.workspace_tab_count(), 0);
        assert!(!storage.clear_workspace());
    }
}
