//! Slint 桌面启动状态装配。

use crate::backend::{DesktopBackendExecutor, RusshBackendExecutor};
use crate::model::{AppState, DEFAULT_LOCAL_TERMINAL_TITLE, LOCAL_TERMINAL_SESSION_ID, UiState};
use crate::security::KeyringSecretStore;
use crate::storage::RedbStorage;
use crate::terminal::TerminalTabState;

/// 构建应用初始状态，并尝试加载本地持久化配置。
pub(super) fn boot_state() -> AppState {
    let remote_executor = RusshBackendExecutor::new(KeyringSecretStore::default())
        .unwrap_or_else(|error| panic!("无法创建真实 SSH 执行器：{error}"));
    let executor = DesktopBackendExecutor::new(remote_executor);
    let mut state = AppState::default().with_backend_executor(executor);

    if let Some(storage_backend) = RedbStorage::default_store() {
        match storage_backend.load() {
            Ok(storage) => {
                state.storage = storage;
                state.config = state.storage.app_config.clone();
                state.ui = UiState::from_visual(&state.config.theme, &state.config.background);
            }
            Err(error) => tracing::warn!(
                path = %storage_backend.path().display(),
                error = %error,
                "无法加载本地存储，使用空存储启动"
            ),
        }
        state = state.with_storage_backend(storage_backend);
    }

    if state.sessions.tab_count() == 0 {
        state
            .sessions
            .open_local_shell_tab(LOCAL_TERMINAL_SESSION_ID, DEFAULT_LOCAL_TERMINAL_TITLE);
        state.terminal.open_tab(TerminalTabState::new(
            LOCAL_TERMINAL_SESSION_ID,
            DEFAULT_LOCAL_TERMINAL_TITLE,
        ));
        state
            .backend_commands
            .push(crate::backend::BackendCommand::OpenShell {
                session_id: LOCAL_TERMINAL_SESSION_ID,
                pty: crate::backend::PtyRequest::xterm(crate::terminal::TerminalSize::default()),
            });
        state.drain_backend_queue_with_executor();
    }

    state
}
