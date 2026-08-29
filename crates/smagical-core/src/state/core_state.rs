use std::sync::Arc;
use crate::storage::{AppStorage, MockStorage};

/// 无界面依赖的核心状态引擎 (Core State Engine)
///
/// 内部封装聚合存储门面 (`Arc<dyn AppStorage>`)，供 UI、CLI 或无头服务驱动业务。
#[derive(Clone)]
pub struct CoreState {
    storage: Arc<dyn AppStorage>,
}

impl CoreState {
    /// 默认采用预设种子内存存储创建 CoreState
    pub fn new_mock() -> Self {
        tracing::debug!(target: "smagical_core", "初始化 CoreState 核心状态引擎 (MockStorage 模式)");
        Self::new(Arc::new(MockStorage::new_seeded()))
    }

    /// 使用任意实现了 `AppStorage` Trait 的存储后端创建 CoreState (可热插拔接入 JsonFileStorage / SqliteStorage)
    pub fn new(storage: Arc<dyn AppStorage>) -> Self {
        Self { storage }
    }

    /// 获取底层存储门面引用
    pub fn storage(&self) -> &Arc<dyn AppStorage> {
        &self.storage
    }
}

impl Default for CoreState {
    fn default() -> Self {
        Self::new_mock()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_mock_state_has_seeded_groups_and_hosts() {
        let state = CoreState::new_mock();
        let groups = state.storage().groups().list_all().unwrap();
        let hosts = state.storage().hosts().list_all().unwrap();

        assert!(!groups.is_empty());
        assert!(!hosts.is_empty());
        assert_eq!(groups[0].id, "grp-prod");
    }
}
