//! Slint 桌面启动状态装配。
//!
//! 这个文件属于 Slint 桌面 Adapter，而不是核心状态本身。它负责把真实桌面
//! 依赖接到核心状态上：SSH 执行器、系统密钥存储、SQLite 存储和默认本地终端。
//!
//! 如果重写 UI：
//!
//! - 可以复用这里的依赖装配思路。
//! - 也可以写自己的启动函数，只要最终得到一个配置好的 `CoreState`。
//! - 不要把窗口类型或 UI 控件传进 `CoreState`，核心层只需要后端执行器和存储后端。

use super::state::DesktopAppState;

use crate::core::CoreState;
use crate::model::UiState;

/// 构建应用初始状态，并尝试加载本地持久化配置。
pub(super) fn boot_state() -> DesktopAppState {
    let core = CoreState::try_default_runtime()
        .unwrap_or_else(|error| panic!("无法创建真实 SSH 执行器：{error}"));
    let mut ui = UiState::from_visual(&core.config.theme, &core.config.background);
    ui.apply_workspace_preferences_from_config(&core.config);

    DesktopAppState { core, ui }
}
