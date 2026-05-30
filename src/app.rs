//! Slint 应用装配入口。
//!
//! 这里是当前桌面 UI 的最外层 Adapter。它知道 Slint，也知道核心
//! `AppState`，但不直接实现 SSH、存储、终端或业务状态变更。
//!
//! 启动流程固定为：
//!
//! 1. `bootstrap::boot_state` 构造核心状态和本地依赖。
//! 2. `callbacks::bind` 把 Slint 事件翻译成核心 `Message`。
//! 3. `pump::start_backend_pump` 把后端事件送回核心状态。
//! 4. `projection::sync_window` 把核心状态投影到 Slint 属性。
//!
//! 如果未来重写 UI，新的 UI 只需要复用这条思路：持有 `AppState`，
//! 提交 `Message`，再从 `view_model::app_view_model` 读取展示模型。

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

/// Slint 回调运行在单线程 UI 循环中，因此当前 Adapter 用 `Rc<RefCell<_>>`
/// 持有核心状态。
///
/// 这只是 Slint Adapter 的共享方式，不是核心层约束。其他 UI 可以改成
/// `Arc<Mutex<_>>`、通道或自己的状态容器，只要仍然通过 `Message` 改状态即可。
type SharedAppState = Rc<RefCell<AppState>>;

/// 启动桌面应用。
pub fn run() -> Result<(), slint::PlatformError> {
    select_window_backend()?;

    let state = Rc::new(RefCell::new(bootstrap::boot_state()));
    let window = AppWindow::new()?;

    callbacks::bind(&window, Rc::clone(&state));
    pump::start_backend_pump(&window, Rc::clone(&state));
    projection::sync_window(&window, &state.borrow());

    window.run()
}

fn select_window_backend() -> Result<(), slint::PlatformError> {
    slint::BackendSelector::new()
        .with_winit_window_attributes_hook(|attributes| {
            attributes.with_theme(Some(slint::winit_030::winit::window::Theme::Dark))
        })
        .select()
}
