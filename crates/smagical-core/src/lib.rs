//! smagicalssh 核心 crate。
//!
//! 这里不依赖具体 UI 框架，只负责领域模型、核心状态和服务接口。

#![deny(missing_docs)]

pub mod domain;
pub mod event;
pub mod state;
pub mod storage;
pub mod theme;

pub use event::{
    AppBeforeExitEvent, AppBootEvent, AppEvent, AppExitEvent, AppReadyEvent, ConfigChangedEvent,
    CredentialCopyType, CredentialDeletedEvent, CredentialSavedEvent, CredentialSecretCopiedEvent,
    CredentialSelectedEvent, EventDispatcher, EventManager, FileOperationBeforeEvent,
    FileOperationCompletedEvent, FileTabClosedEvent, FileTabFocusChangedEvent,
    FileTabNavigatedEvent, FileTabOpenedEvent, FileTabOpeningEvent, FileTransferStartedEvent,
    HistoryClearedEvent, HistoryItemDeletedEvent, HistoryPinToggledEvent,
    HistoryReconnectRequestedEvent, HostAssetChangedEvent, HostGroupToggledEvent,
    HostSearchFilteredEvent, HostTreeReorderedEvent, KeyGeneratedEvent, ListenerGuard,
    ModuleActivatedEvent, ModuleDeactivatedEvent, NavigationRequestedEvent,
    NavigationTabClickedEvent, PasswordGeneratedEvent, RightPanelRegisteredEvent,
    RightPanelSwitchedEvent, RightPanelUnregisteredEvent, TerminalActionRequestedEvent,
    TerminalFocusChangedEvent, TerminalSessionEvent, TerminalSplitChangedEvent, ThemeChangedEvent,
    ThemeModeToggledEvent, WindowStateChangedEvent,
};

pub use domain::{
    activity_bar::{ActivityBarItem, ActivityBarRegistry},
    file_item::{
        format_file_size, format_file_time, generate_mock_remote_directory, scan_local_directory,
        FileItemData, FileTabSession, LocalFileTabSession, RemoteFileTabSession, TransferDirection,
        TransferStatus, TransferTask,
    },

    credential::{CredentialRecord, CredentialType},
    group::GroupRecord,
    history::{HistoryRecord, SessionSnapshotConfig},
    host::{HostRecord, HostStatus},
    navigation::{NavigationRequest, NavigationRouter},
    right_panel::{RightPanelItem, RightPanelRegistry},
    terminal_context::{ActiveTerminalSessionContext, TerminalAction},
};

pub use state::core_state::CoreState;
pub use storage::{
    AppStorage, CredentialRepository, GroupRepository, HistoryRepository, HostRepository,
    MockStorage, StorageError, StorageResult,
};


