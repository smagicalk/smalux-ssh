use iced::{
    widget::{button, column, text},
    Element,
};

use crate::model::{AppState, Message};

pub fn view(state: &AppState) -> Element<'_, Message> {
    column![
        text(&state.config.app_name),
        text(format!("hosts: {}", state.storage.host_count())),
        text(format!("groups: {}", state.storage.group_count())),
        text(format!("tunnels: {}", state.storage.tunnel_rule_count())),
        text(format!("sessions: {}", state.sessions.active_count())),
        text(format!("tabs: {}", state.terminal.tab_count)),
        text(format!("theme: {}", state.config.theme.name)),
        text(format!(
            "background: {}",
            if state.config.background.enabled {
                "enabled"
            } else {
                "disabled"
            }
        )),
        button("Toggle theme").on_press(Message::ToggleTheme),
    ]
    .spacing(12)
    .padding(16)
    .into()
}
