//! Slint 应用装配入口。
//!
//! 本模块只负责窗口生命周期；启动、回调绑定和状态投影拆在 `src/app/` 子模块中。

mod bootstrap;
mod callbacks;
mod ids;
mod projection;
mod pump;
mod view_model;

use std::cell::RefCell;
use std::rc::Rc;

use crate::model::AppState;

slint::include_modules!();

type SharedAppState = Rc<RefCell<AppState>>;

/// 启动桌面应用。
pub fn run() -> Result<(), slint::PlatformError> {
    let state = Rc::new(RefCell::new(bootstrap::boot_state()));
    let window = AppWindow::new()?;

    callbacks::bind(&window, Rc::clone(&state));
    pump::start_backend_pump(&window, Rc::clone(&state));
    projection::sync_window(&window, &state.borrow());

    window.run()
}
