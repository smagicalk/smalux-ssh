use crate::app::state::DesktopAppState;
use crate::backend::BackendCommand;
use crate::core::CoreState;
use crate::model::{
    AuthProfile, Host, HostId, Message, QuickHostAuthField, QuickHostAuthKind, QuickHostDraftField,
    SecretRef, SessionId, SftpActionDraftField, ThemeProfile, UiState,
};
use uuid::Uuid;

mod draft_fields;
mod quick_host;
mod terminal_input;

fn desktop_state() -> DesktopAppState {
    let core = CoreState::default();
    let ui = UiState::from_visual(&core.config.theme, &core.config.background);
    DesktopAppState { core, ui }
}
