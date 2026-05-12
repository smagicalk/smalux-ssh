//! Iced 应用装配入口。
//!
//! 这里只负责把状态、消息更新和视图函数交给 iced，业务逻辑继续下沉到
//! model、session、storage 和 terminal 等模块。

use iced::{Element, Result, Task, application};

use crate::model::{AppState, Message};
use crate::ui;

/// 启动桌面应用。
pub fn run() -> Result {
    application(AppState::boot, update, view)
        .title("smagicalssh")
        .theme(|state: &AppState| state.theme.clone())
        .run()
}

/// Iced 消息更新函数。
fn update(state: &mut AppState, message: Message) -> Task<Message> {
    state.apply(message);
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
    fn view_builds_element_from_default_state() {
        let state = AppState::default();

        let _element = view(&state);
    }
}
