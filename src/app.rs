//! Iced 应用装配入口。
//!
//! 这里只负责把状态、消息更新和视图函数交给 iced，业务逻辑继续下沉到
//! model、session、storage 和 terminal 等模块。

use iced::{Element, Result, Task, application};

use crate::backend::RusshBackendExecutor;
use crate::model::{AppState, Message};
use crate::security::KeyringSecretStore;
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

    (
        AppState::default().with_backend_executor(executor),
        Task::none(),
    )
}

/// Iced 消息更新函数。
fn update(state: &mut AppState, message: Message) -> Task<Message> {
    state.apply(message);
    state.drain_backend_queue_with_executor();
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
    fn view_builds_element_from_default_state() {
        let state = AppState::default();

        let _element = view(&state);
    }
}
