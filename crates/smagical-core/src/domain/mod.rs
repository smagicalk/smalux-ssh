//! 领域模型层。

/// 主机分组及其层级树模型。
pub mod group;
/// 历史会话记录模型。
pub mod history;
/// SSH 主机资产记录模型。
pub mod host;

pub use group::GroupRecord;
pub use history::{HistoryRecord, SessionSnapshotConfig};
pub use host::{HostRecord, HostStatus};


