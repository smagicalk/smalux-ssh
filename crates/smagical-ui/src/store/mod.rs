//! 集中式状态管理与增量渲染中枢 (Store)。
//!
//! 负责统一调度桌面端 UI 的全局状态（资产树、卡片、展开收缩状态）、
//! 执行 CPU 密集型数据运算的后台下沉 (Tokio Worker 调度)，
//! 以及通过增量比对引擎 (Diff Engine) 驱动 Slint 高性能就地渲染。

pub(crate) mod diff;
pub(crate) mod host_store;

pub(crate) use host_store::HostStore;

use std::sync::Arc;

/// 顶层集中式 UI 状态仓储中枢
#[derive(Clone)]
pub(crate) struct UiStore {
    /// 主机与资产树状态仓储
    pub(crate) host_store: Arc<HostStore>,
}

impl UiStore {
    /// 创建全新的 UiStore
    #[allow(dead_code)]
    pub(crate) fn new(host_store: HostStore) -> Self {
        Self {
            host_store: Arc::new(host_store),
        }
    }

    /// 从已有的 HostStore 共享引用创建 UiStore
    pub(crate) fn from_host_store(host_store: Arc<HostStore>) -> Self {
        Self { host_store }
    }

    /// 获取主机状态仓储引用
    #[allow(dead_code)]
    pub(crate) fn host(&self) -> &Arc<HostStore> {
        &self.host_store
    }
}
