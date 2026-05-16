pub mod app;
pub mod backend;
pub mod config;
pub mod model;
pub mod security;
pub mod session;
pub mod storage;
pub mod terminal;

fn main() -> Result<(), slint::PlatformError> {
    app::run()
}
