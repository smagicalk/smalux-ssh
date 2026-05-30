//! 视觉设置消息路由。
//!
//! 这里处理全局视觉配置和主机级覆盖。它只修改配置和草稿，不关心 Slint 主题
//! 属性如何写入；实际颜色投影由 `app::projection` 完成。

use super::super::{AppState, AppUpdateOutcome, Message};

impl AppState {
    /// 分发视觉配置消息。
    pub(super) fn dispatch_visual_message(&mut self, message: Message) -> AppUpdateOutcome {
        match message {
            Message::UpdateVisualSettingsDraft { field, value } => {
                self.update_visual_settings_draft(field, value)
            }
            Message::SetVisualBackgroundEnabled { enabled } => {
                self.set_visual_background_enabled(enabled)
            }
            Message::ApplyVisualSettings => self.apply_visual_settings(),
            Message::UpdateHostVisualSettingsDraft {
                host_id,
                field,
                value,
            } => self.update_host_visual_settings_draft(host_id, field, value),
            Message::SetHostVisualBackgroundEnabled { host_id, enabled } => {
                self.set_host_visual_background_enabled(host_id, enabled)
            }
            Message::ApplyHostVisualSettings { host_id } => {
                self.apply_host_visual_settings(host_id)
            }
            Message::ClearHostVisualSettings { host_id } => {
                self.clear_host_visual_settings(host_id)
            }
            _ => unreachable!("非视觉设置消息不应进入视觉设置路由"),
        }
    }
}
