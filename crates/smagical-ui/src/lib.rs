//! smagicalssh UI crate。
//!
//! 这里依赖 `smagical-core`，负责桌面装配、界面状态投影和展示。

pub mod app;
pub mod callbacks;
pub mod desktop;
pub mod presentation;
pub mod projection;
pub mod state;
pub mod view_model;

pub fn run() -> anyhow::Result<()> {
    app::run()
}
