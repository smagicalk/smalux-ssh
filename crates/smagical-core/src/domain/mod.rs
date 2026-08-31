//! 领域模型层。

/// 主机分组及其层级树模型。
pub mod group;
/// 历史会话记录模型。
pub mod history;
/// SSH 主机资产记录模型。
pub mod host;
/// 侧边栏动态注册模型。
pub mod activity_bar;
/// 统一页面导航与路由模型。
pub mod navigation;

pub use group::GroupRecord;
pub use history::{HistoryRecord, SessionSnapshotConfig};
pub use host::{HostRecord, HostStatus};
pub use activity_bar::{ActivityBarItem, ActivityBarRegistry};
pub use navigation::{NavigationRequest, NavigationRouter};



