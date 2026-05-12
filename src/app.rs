use iced::{application, Element, Result, Task};

use crate::model::{AppState, Message};
use crate::ui;

pub fn run() -> Result {
    application(AppState::boot, update, view)
        .title("smagicalssh")
        .theme(|state: &AppState| state.theme.clone())
        .run()
}

fn update(state: &mut AppState, message: Message) -> Task<Message> {
    state.apply(message);
    Task::none()
}

fn view(state: &AppState) -> Element<'_, Message> {
    ui::view(state)
}
