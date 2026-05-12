pub mod app;
pub mod config;
pub mod model;
pub mod session;
pub mod storage;
pub mod terminal;
pub mod ui;

fn main() -> iced::Result {
    app::run()
}
