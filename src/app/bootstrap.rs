//! Slint 桌面启动状态装配。
//!
//! 这个文件属于 Slint 桌面 Adapter，而不是核心状态本身。它负责把真实桌面
//! 依赖接到核心状态上：SSH 执行器、系统密钥存储、SQLite 存储和默认本地终端。
//!
//! 如果重写 UI：
//!
//! - 可以复用这里的依赖装配思路。
//! - 也可以写自己的启动函数，只要最终得到一个配置好的 `AppState`。
//! - 不要把窗口类型或 UI 控件传进 `AppState`，核心层只需要后端执行器和存储后端。

use crate::backend::{DesktopBackendExecutor, RusshBackendExecutor};
use crate::model::{AppState, DEFAULT_LOCAL_TERMINAL_TITLE, LOCAL_TERMINAL_SESSION_ID, UiState};
use crate::security::KeyringSecretStore;
use crate::storage::{LegacyImportOutcome, RedbStorage, SqliteStorage};
use crate::terminal::TerminalTabState;

/// 构建应用初始状态，并尝试加载本地持久化配置。
pub(super) fn boot_state() -> AppState {
    // 真实 SSH 执行器依赖系统密钥存储，用于解析密码、私钥和证书引用。
    // 这里 panic 是启动级故障：没有可用执行器时桌面端无法发起真实连接。
    let remote_executor = RusshBackendExecutor::new(KeyringSecretStore::default())
        .unwrap_or_else(|error| panic!("无法创建真实 SSH 执行器：{error}"));
    let executor = DesktopBackendExecutor::new(remote_executor);
    let mut state = AppState::default().with_backend_executor(executor);

    // SQLite 是当前默认持久化后端。旧版本 redb 数据只在 SQLite 为空时导入，
    // 避免覆盖用户已经迁移或手动创建的新库。
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
                // 存储中的 AppConfig 是用户上次保存的配置快照。加载后要重新创建
                // UI 草稿状态，并把可持久化偏好同步回运行态。
                state.storage = storage;
                state.config = state.storage.app_config.clone();
                state.ui = UiState::from_visual(&state.config.theme, &state.config.background);
                state.apply_workspace_preferences();
            }
            Err(error) => tracing::warn!(
                path = %storage_backend.path().display(),
                error = %error,
                "无法加载本地存储，使用空存储启动"
            ),
        }
        state = state.with_storage_backend(storage_backend);
    }

    // 桌面启动时至少给用户一个本地终端标签。这样即使没有主机配置，首页和
    // 终端区也有可操作对象。真正打开 PTY 的工作仍然进入后端命令队列。
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
            .push(crate::backend::BackendCommand::OpenLocalShell {
                session_id: LOCAL_TERMINAL_SESSION_ID,
                pty: crate::backend::PtyRequest::xterm(crate::terminal::TerminalSize::default()),
            });
    }

    state
}
