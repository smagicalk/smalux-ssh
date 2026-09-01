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
/// 终端活跃会话上下文与指令交互模型。
pub mod terminal_context;
/// 右侧辅助抽屉栏动态注册模型。
pub mod right_panel;
/// 双盘文件浏览器与 SFTP 传输模型。
pub mod file_item;
/// SSH 凭据与密钥管理模型。
pub mod credential;
/// 代码片段与多层层级分组模型。
pub mod snippet;

pub use group::GroupRecord;
pub use history::{HistoryRecord, SessionSnapshotConfig};
pub use host::{HostRecord, HostStatus};
pub use credential::{CredentialRecord, CredentialType};
pub use snippet::{SnippetGroupRecord, SnippetRecord, SnippetVariable};
pub use activity_bar::{ActivityBarItem, ActivityBarRegistry};
pub use navigation::{NavigationRequest, NavigationRouter};
pub use terminal_context::{ActiveTerminalSessionContext, TerminalAction};
pub use right_panel::{RightPanelItem, RightPanelRegistry};
pub use file_item::{
    format_file_size, format_file_time, generate_mock_remote_directory, scan_local_directory,
    FileItemData, FileTabSession, LocalFileTabSession, RemoteFileTabSession, TransferDirection,
    TransferStatus, TransferTask,
};






