//! Iced 应用装配入口。
//!
//! 这里只负责把状态、消息更新和视图函数交给 iced，业务逻辑继续下沉到
//! model、session、storage 和 terminal 等模块。

use iced::{Element, Result, Task, application};

use crate::backend::RusshBackendExecutor;
use crate::model::{AppState, Message};
use crate::security::KeyringSecretStore;
use crate::storage::RedbStorage;
use crate::ui;

/// 启动桌面应用。
pub fn run() -> Result {
    application(boot, update, view)
        .title("smagicalssh")
        .theme(|state: &AppState| state.theme.clone())
        .run()
}

fn boot() -> (AppState, Task<Message>) {
    let executor = RusshBackendExecutor::new(KeyringSecretStore::default())
        .unwrap_or_else(|error| panic!("无法创建真实 SSH 执行器：{error}"));
    let mut state = AppState::default().with_backend_executor(executor);

    if let Some(storage_backend) = RedbStorage::default_store() {
        match storage_backend.load() {
            Ok(storage) => state.storage = storage,
            Err(error) => tracing::warn!(
                path = %storage_backend.path().display(),
                error = %error,
                "无法加载本地存储，使用空存储启动"
            ),
        }
        state = state.with_storage_backend(storage_backend);
    }

    (state, Task::none())
}

/// Iced 消息更新函数。
fn update(state: &mut AppState, message: Message) -> Task<Message> {
    let storage_before = state.storage.clone();

    state.apply(message);
    state.drain_backend_queue_with_executor();

    if state.storage != storage_before {
        if let Err(error) = state.persist_storage() {
            tracing::error!(error = %error, "保存本地存储失败");
        }
    }

    Task::none()
}

/// Iced 视图函数。
fn view(state: &AppState) -> Element<'_, Message> {
    ui::view(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::Theme;

    #[test]
    fn update_delegates_message_to_app_state() {
        let mut state = AppState::default();

        let _task = update(&mut state, Message::ToggleTheme);

        assert!(matches!(state.theme, Theme::Light));
    }

    #[test]
    fn update_with_backend_executor_pumps_queued_commands() {
        use crate::backend::{
            BackendCommandKind, BackendEvent, ScriptedBackendExecutor, ScriptedBackendResponse,
        };
        use crate::model::Host;
        use crate::model::{AuthProfile, HostId};
        use uuid::Uuid;

        let mut state = AppState::default().with_backend_executor(ScriptedBackendExecutor::new());
        let host = Host {
            id: HostId(Uuid::new_v4()),
            name: "production".to_owned(),
            group_id: None,
            tags: vec![],
            address: "example.com".to_owned(),
            port: 22,
            auth: AuthProfile::Agent {
                username: "deploy".to_owned(),
                key_hint: Some("id_ed25519".to_owned()),
            },
            proxy: None,
            jumps: Vec::new(),
            theme_override: None,
            background_override: None,
        };
        let host_id = host.id;
        state.storage.upsert_host(host);
        state.backend_executor = crate::backend::shared_backend_executor({
            let mut executor = ScriptedBackendExecutor::new();
            executor.push_response(ScriptedBackendResponse::new(
                BackendCommandKind::Connect,
                vec![BackendEvent::Connected {
                    session_id: crate::model::SessionId(Uuid::new_v4()),
                }],
            ));
            executor
        });

        let _ = update(&mut state, Message::OpenShell { host_id });

        assert_eq!(state.backend_commands.pending_count(), 0);
    }

    #[test]
    fn update_persists_storage_when_it_changes() {
        use crate::model::QuickHostDraftField;

        let path = std::env::temp_dir().join(format!(
            "smagicalssh-app-update-{}.redb",
            uuid::Uuid::new_v4()
        ));
        let storage_backend = RedbStorage::new(&path);
        let mut state = AppState::default().with_storage_backend(storage_backend.clone());
        state.ui.set_quick_host_field(
            QuickHostDraftField::Address,
            "persist.example.com".to_owned(),
        );
        state
            .ui
            .set_quick_host_field(QuickHostDraftField::Username, "deploy".to_owned());

        let _ = update(&mut state, Message::SaveQuickHost);
        let loaded = storage_backend
            .load()
            .expect("update 应该在存储变化后保存 redb 快照");

        assert_eq!(loaded.host_count(), 1);
        assert_eq!(loaded.hosts[0].address, "persist.example.com");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn view_builds_element_from_default_state() {
        let state = AppState::default();

        let _element = view(&state);
    }
}
