//! Slint 属性写入层。
//!
//! 本模块只把 view model 写入 Slint 窗口，不直接读取核心领域细节。
//!
//! 这层是当前 Slint UI 的“输出 Adapter”：
//!
//! - 输入是 `view_model::AppViewModel` 或局部 view model。
//! - 输出是 `AppWindow` 上的属性、列表模型和终端缓冲。
//! - 这里不做业务判断，不创建 `Message`，也不直接访问存储后端。
//!
//! 如果未来重写 UI，这个模块通常会被整体替换；核心 `model` 和
//! `view_model` 可以继续保留。

use crate::model::AppState;

use super::AppWindow;
use super::view_model::{AppViewModel, active_terminal, app_view_model};

mod collections;
mod models;
mod sftp;
mod terminal;
mod workspace;

use collections::sync_collection_models;
use sftp::sync_sftp_model;
use terminal::sync_terminal_model;
use workspace::sync_workspace_state;

/// 将当前应用状态同步到 Slint 窗口。
pub(super) fn sync_window(window: &AppWindow, state: &AppState) {
    sync_view_model(window, &app_view_model(state));
}

/// 只同步当前终端面板，用于回车和本地 PTY 输出刷新。
pub(super) fn sync_terminal_pane(window: &AppWindow, state: &AppState) {
    let model = active_terminal(state);
    sync_terminal_model(window, &model);
}

fn sync_view_model(window: &AppWindow, model: &AppViewModel) {
    sync_workspace_state(window, model);
    sync_terminal_model(window, &model.terminal);
    sync_sftp_model(window, model);
    sync_collection_models(window, model);
}
