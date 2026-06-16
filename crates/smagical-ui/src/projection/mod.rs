//! UI 投影层。

use crate::desktop::bootstrap::AppWindow;
use crate::view_model::HomeViewModel;

/// 把 view model 同步到窗口属性。
pub fn sync_home(window: &AppWindow, view_model: &HomeViewModel) {
    window.set_host_summary(view_model.host_summary.clone().into());
}
