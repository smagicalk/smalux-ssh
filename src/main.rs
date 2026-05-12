mod app;
mod config;
mod model;
mod session;
mod storage;
mod terminal;
mod ui;

fn main() -> iced::Result {
    app::run()
}

