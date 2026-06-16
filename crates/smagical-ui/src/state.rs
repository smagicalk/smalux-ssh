//! UI 运行态。

use smagical_core::CoreState;

/// 当前桌面 UI 组合状态。
#[derive(Debug, Default)]
pub struct UiState {
    pub host_summary: String,
}

/// 当前桌面适配层状态。
#[derive(Debug, Default)]
pub struct DesktopAppState {
    pub core: CoreState,
    pub ui: UiState,
}

impl DesktopAppState {
    pub fn new(core: CoreState) -> Self {
        Self {
            core,
            ui: UiState::default(),
        }
    }
}
