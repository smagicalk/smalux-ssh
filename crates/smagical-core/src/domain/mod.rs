//! 领域模型层。

/// 主机分组及其层级树模型。
pub mod group;
/// SSH 主机资产记录模型。
pub mod host;

pub use group::GroupRecord;
pub use host::{HostRecord, HostStatus};
