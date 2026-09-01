//! smagicalssh 核心 crate。
//!
//! 这里不依赖具体 UI 框架，只负责领域模型、核心状态和服务接口。

#![deny(missing_docs)]

pub mod app_hook;
pub mod domain;
pub mod hook;
pub mod state;
pub mod storage;
pub mod theme;

pub use app_hook::{
    AppBootContext, AppExitContext, AppGlobalHook, AppGlobalHookEngine, AutoConfigBackupHook,
    ConfigChangeEvent, FunctionalGlobalHook, ListenerHandle, WindowState,
};

pub use domain::{
    activity_bar::{ActivityBarItem, ActivityBarRegistry},
    file_item::{
        format_file_size, format_file_time, generate_mock_remote_directory, scan_local_directory,
        FileItemData, FileTabSession, LocalFileTabSession, RemoteFileTabSession, TransferDirection,
        TransferStatus, TransferTask,
    },

    group::GroupRecord,
    history::{HistoryRecord, SessionSnapshotConfig},
    host::{HostRecord, HostStatus},
    navigation::{NavigationRequest, NavigationRouter},
    right_panel::{RightPanelItem, RightPanelRegistry},
    terminal_context::{ActiveTerminalSessionContext, TerminalAction},
};




pub use hook::{
    CommandInteractionFrame, CommandSource, DangerousCommandGuard, FallbackStrategy, FrameStatus,
    FunctionalHook, HistoryTrackingHook, HookDecision, HookEngine, HostMetadata, SessionAuditLogger,
    SessionContext, TerminalError, TerminalHook,
};



pub use state::core_state::CoreState;
pub use storage::{
    AppStorage, GroupRepository, HistoryRepository, HostRepository, MockStorage, StorageError,
    StorageResult,
};


