//! 应用核心门面。
//!
//! 这个模块只暴露与具体 GUI 无关的稳定入口，供当前 Slint 桌面 UI、未来重写的
//! 原生 UI、测试工具或命令行入口复用。
//!
//! 约束：
//!
//! - 这里不暴露 `slint` 类型。
//! - 这里不暴露 `ui/*.slint` 生成的窗口属性。
//! - 新 UI 如果只想接入核心，优先依赖这里，而不是直接穿透到 `app`。

pub use crate::backend;
pub use crate::config;
pub use crate::security;
pub use crate::session;
pub use crate::storage;
pub use crate::terminal;
pub use crate::theme;

use crate::backend::{
    BackendCommandQueue, SharedBackendExecutor, default_runtime_backend_executor,
    noop_shared_backend_executor, shared_backend_executor,
};
use crate::config::AppConfig;
use crate::security::KeyringSecretStore;
use crate::session::SessionManager;
use crate::storage::{
    LegacyImportOutcome, RedbStorage, SqliteStorage, StorageManager, StoragePersistenceError,
};
use crate::terminal::TerminalManager;

/// 不依赖任何 GUI 草稿状态的核心运行态。
///
/// 这是给未来非 Slint UI、CLI 或 headless 场景准备的最小核心状态组合。
/// 当前桌面 UI 通过 Adapter 组合 `CoreState` 和自己的 `UiState` 草稿，不应让
/// 核心反向依赖任何具体界面状态。
#[derive(Clone)]
pub struct CoreState {
    pub config: AppConfig,
    pub sessions: SessionManager,
    pub storage: StorageManager,
    pub storage_backend: Option<SqliteStorage>,
    pub terminal: TerminalManager,
    pub backend_commands: BackendCommandQueue,
    pub backend_executor: SharedBackendExecutor,
}

impl std::fmt::Debug for CoreState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CoreState")
            .field("config", &self.config)
            .field("sessions", &self.sessions)
            .field("storage", &self.storage)
            .field("storage_backend", &self.storage_backend)
            .field("terminal", &self.terminal)
            .field("backend_commands", &self.backend_commands)
            .field("backend_executor", &"<shared backend executor>")
            .finish()
    }
}

impl Default for CoreState {
    fn default() -> Self {
        let config = AppConfig::default();
        let mut storage = StorageManager::default();
        storage.app_config = config.clone();

        Self {
            config,
            sessions: SessionManager::default(),
            storage,
            storage_backend: None,
            terminal: TerminalManager::default(),
            backend_commands: BackendCommandQueue::default(),
            backend_executor: noop_shared_backend_executor(),
        }
    }
}

impl CoreState {
    /// 尝试使用默认运行时依赖构造一个可直接运行的核心状态。
    ///
    /// 这个入口不创建任何 UI 草稿；它只负责装配执行器、存储后端和默认本地终端，
    /// 便于 CLI、测试工具或未来其他 UI 在不经过 Slint bootstrap 的情况下复用同一套核心启动流程。
    pub fn try_default_runtime() -> std::io::Result<Self> {
        let remote_executor =
            crate::backend::RusshBackendExecutor::new(KeyringSecretStore::default())?;
        let executor = default_runtime_backend_executor(remote_executor);
        let mut core = Self::default().with_backend_executor(executor);

        if let Some(storage_backend) = SqliteStorage::default_store() {
            if let Some(legacy_backend) = RedbStorage::default_store() {
                match storage_backend.import_legacy_redb_if_empty(&legacy_backend) {
                    Ok(LegacyImportOutcome::Imported) => tracing::info!(
                        source = %legacy_backend.path().display(),
                        target = %storage_backend.path().display(),
                        "已将旧 redb 存储迁移到 SQLite 并删除旧文件"
                    ),
                    Ok(LegacyImportOutcome::DeletedEmptyLegacy) => tracing::info!(
                        source = %legacy_backend.path().display(),
                        "已删除空的旧 redb 存储文件"
                    ),
                    Ok(LegacyImportOutcome::SkippedSqliteNotEmpty) => tracing::info!(
                        source = %legacy_backend.path().display(),
                        target = %storage_backend.path().display(),
                        "SQLite 已有数据，跳过旧 redb 迁移"
                    ),
                    Ok(LegacyImportOutcome::NoLegacyFile) => {}
                    Err(error) => tracing::warn!(
                        source = %legacy_backend.path().display(),
                        target = %storage_backend.path().display(),
                        error = %error,
                        "旧 redb 存储迁移失败，继续使用 SQLite"
                    ),
                }
            }

            match storage_backend.load() {
                Ok(storage) => {
                    core.storage = storage;
                    core.config = core.storage.app_config.clone();
                }
                Err(error) => tracing::warn!(
                    path = %storage_backend.path().display(),
                    error = %error,
                    "无法加载本地存储，使用空存储启动"
                ),
            }

            core = core.with_storage_backend(storage_backend);
        }

        core.ensure_default_local_terminal();
        Ok(core)
    }

    /// 使用默认运行时依赖构造一个可直接运行的核心状态。
    ///
    /// 当前桌面启动把执行器构造失败视为启动级故障，因此保留这个 panic 版本；
    /// 无 UI 入口应优先使用 `try_default_runtime()`。
    pub fn with_default_runtime() -> Self {
        Self::try_default_runtime()
            .unwrap_or_else(|error| panic!("无法创建真实 SSH 执行器：{error}"))
    }

    /// 使用指定共享执行器替换默认占位执行器。
    pub fn with_backend_executor<E>(mut self, executor: E) -> Self
    where
        E: crate::backend::BackendExecutor + 'static,
    {
        self.backend_executor = shared_backend_executor(executor);
        self
    }

    /// 使用指定本地存储后端启用持久化。
    pub fn with_storage_backend(mut self, storage_backend: SqliteStorage) -> Self {
        self.storage_backend = Some(storage_backend);
        self
    }

    /// 从已配置的本地存储后端保存当前持久化状态。
    pub fn persist_storage(&self) -> Result<(), StoragePersistenceError> {
        if let Some(storage_backend) = &self.storage_backend {
            storage_backend.save(&self.storage)?;
        }

        Ok(())
    }

    /// 使用核心持有的共享后端执行器执行当前排队命令，直到队列为空或遇到错误。
    ///
    /// 这是无 UI 运行模式的同步入口。当前 Slint 桌面为了避免阻塞界面，会用自己的
    /// worker/pump 逐条提交命令；CLI、测试工具或未来其他 UI 可以先用这个简单入口。
    pub fn drain_backend_queue_with_shared_executor(&mut self) -> crate::model::AppUpdateOutcome {
        let executor = self.backend_executor.clone();
        let mut executor = executor
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        self.drain_backend_queue(&mut **executor)
    }

    /// 提交一条消息，然后用核心持有的后端执行器同步执行排队命令。
    pub fn apply_and_drain_backend_queue(
        &mut self,
        message: crate::model::Message,
    ) -> crate::model::AppUpdateOutcome {
        let mut outcome = self.apply(message);
        merge_outcome(
            &mut outcome,
            self.drain_backend_queue_with_shared_executor(),
        );
        outcome
    }

    /// 提交一条不依赖桌面草稿的核心消息。
    ///
    /// UI 草稿、弹窗和页面状态由具体 Adapter 管理；核心入口只接受已经能在
    /// `CoreState` 内独立完成的消息。
    pub fn apply(&mut self, message: crate::model::Message) -> crate::model::AppUpdateOutcome {
        self.apply_core_message(message)
    }

    /// 确保默认本地终端存在。
    pub fn ensure_default_local_terminal(&mut self) {
        if self.sessions.tab_count() != 0 {
            return;
        }

        self.sessions.open_local_shell_tab(
            crate::model::LOCAL_TERMINAL_SESSION_ID,
            crate::model::DEFAULT_LOCAL_TERMINAL_TITLE,
        );
        self.terminal
            .open_tab(crate::terminal::TerminalTabState::new(
                crate::model::LOCAL_TERMINAL_SESSION_ID,
                crate::model::DEFAULT_LOCAL_TERMINAL_TITLE,
            ));
        self.backend_commands
            .push(crate::backend::BackendCommand::OpenLocalShell {
                session_id: crate::model::LOCAL_TERMINAL_SESSION_ID,
                pty: crate::backend::PtyRequest::xterm(crate::terminal::TerminalSize::default()),
            });
    }

    /// 生成新的私钥凭据。
    pub fn generate_private_key_credential_action(
        &mut self,
        name: String,
        group_id: Option<crate::model::CredentialGroupId>,
        algorithm: Option<crate::model::KeyAlgorithm>,
    ) -> crate::model::AppUpdateOutcome {
        self.generate_private_key_credential(name, group_id, algorithm)
    }

    /// 创建凭据元数据。
    pub fn create_credential_metadata_action(
        &mut self,
        kind: crate::model::CredentialKind,
        name: String,
        group_id: Option<crate::model::CredentialGroupId>,
        secret_ref: String,
        algorithm: Option<crate::model::KeyAlgorithm>,
    ) -> crate::model::AppUpdateOutcome {
        self.create_credential_metadata(kind, name, group_id, secret_ref, algorithm)
    }

    /// 保存密码凭据。
    pub fn save_password_credential_action(
        &mut self,
        name: String,
        group_id: Option<crate::model::CredentialGroupId>,
        password: String,
    ) -> crate::model::AppUpdateOutcome {
        self.save_password_credential(name, group_id, password)
    }

    /// 从文件导入私钥凭据。
    pub fn import_private_key_credential_action(
        &mut self,
        name: String,
        group_id: Option<crate::model::CredentialGroupId>,
        source_path: String,
        algorithm: Option<crate::model::KeyAlgorithm>,
    ) -> crate::model::AppUpdateOutcome {
        self.import_private_key_credential(name, group_id, source_path, algorithm)
    }

    /// 从文本导入私钥凭据。
    pub fn import_private_key_text_credential_action(
        &mut self,
        name: String,
        group_id: Option<crate::model::CredentialGroupId>,
        private_key_text: String,
        algorithm: Option<crate::model::KeyAlgorithm>,
    ) -> crate::model::AppUpdateOutcome {
        self.import_private_key_text_credential(name, group_id, private_key_text, algorithm)
    }

    /// 从文件导入证书凭据。
    pub fn import_certificate_credential_action(
        &mut self,
        name: String,
        group_id: Option<crate::model::CredentialGroupId>,
        source_path: String,
        algorithm: Option<crate::model::KeyAlgorithm>,
    ) -> crate::model::AppUpdateOutcome {
        self.import_certificate_credential(name, group_id, source_path, algorithm)
    }

    /// 从文本导入证书凭据。
    pub fn import_certificate_text_credential_action(
        &mut self,
        name: String,
        group_id: Option<crate::model::CredentialGroupId>,
        certificate_text: String,
        algorithm: Option<crate::model::KeyAlgorithm>,
    ) -> crate::model::AppUpdateOutcome {
        self.import_certificate_text_credential(name, group_id, certificate_text, algorithm)
    }

    /// 生成证书凭据。
    #[allow(clippy::too_many_arguments)]
    pub fn generate_certificate_credential_action(
        &mut self,
        name: String,
        group_id: Option<crate::model::CredentialGroupId>,
        ca_private_key_ref: String,
        subject_private_key_ref: String,
        cert_type: String,
        principals: String,
        valid_days: String,
        key_id: String,
        serial: String,
    ) -> crate::model::AppUpdateOutcome {
        self.generate_certificate_credential(
            name,
            group_id,
            ca_private_key_ref,
            subject_private_key_ref,
            cert_type,
            principals,
            valid_days,
            key_id,
            serial,
        )
    }
}

fn merge_outcome(
    merged: &mut crate::model::AppUpdateOutcome,
    outcome: crate::model::AppUpdateOutcome,
) {
    merged.state_changed |= outcome.state_changed;
    merged.queued_backend_commands += outcome.queued_backend_commands;
    merged.executed_backend_commands += outcome.executed_backend_commands;
    merged.applied_backend_events += outcome.applied_backend_events;
    if merged.worker_command.is_none() {
        merged.worker_command = outcome.worker_command;
    }
    if merged.error.is_none() {
        merged.error = outcome.error;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_state_apply_runs_core_message_without_gui_state() {
        let mut core = CoreState::default();

        let outcome = core.apply(crate::model::Message::CreateCredentialGroup {
            name: "生产密钥".to_owned(),
            kind: crate::model::CredentialKind::PrivateKey,
            parent_id: None,
        });

        assert!(outcome.state_changed);
        assert!(outcome.error.is_none());
        assert_eq!(core.storage.credential_groups.len(), 1);
        assert_eq!(core.storage.credential_groups[0].name, "生产密钥");
    }

    #[test]
    fn core_state_apply_rejects_desktop_draft_message() {
        let mut core = CoreState::default();

        let outcome = core.apply(crate::model::Message::UpdateNetworkSearchQuery {
            query: "proxy".to_owned(),
        });

        assert!(!outcome.state_changed);
        assert!(outcome.error.as_deref().unwrap_or("").contains("桌面草稿"));
        assert!(core.storage.credential_groups.is_empty());
    }

    #[test]
    fn core_state_default_runtime_seeds_local_terminal_without_ui() {
        let core = CoreState::with_default_runtime();

        assert_eq!(core.sessions.tab_count(), 1);
        assert_eq!(core.terminal.tab_count(), 1);
        assert_eq!(
            core.sessions.active_tab,
            Some(crate::model::LOCAL_TERMINAL_SESSION_ID)
        );
        assert!(matches!(
            core.backend_commands.front(),
            Some(crate::backend::BackendCommand::OpenLocalShell { session_id, .. })
                if *session_id == crate::model::LOCAL_TERMINAL_SESSION_ID
        ));
    }

    #[test]
    fn core_state_can_drain_backend_queue_without_gui_state() {
        let mut core = CoreState::default();
        let session_id = crate::model::SessionId(uuid::Uuid::new_v4());

        core.backend_commands
            .push(crate::backend::BackendCommand::Disconnect { session_id });
        let outcome = core.drain_backend_queue_with_shared_executor();

        assert!(outcome.error.is_some());
        assert!(core.backend_commands.is_empty());
    }
}
