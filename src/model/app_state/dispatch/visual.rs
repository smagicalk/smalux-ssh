//! 视觉设置消息路由。
//!
//! 这里处理全局视觉配置和主机级覆盖。它只修改配置和草稿，不关心 Slint 主题
//! 属性如何写入；实际颜色投影由 `app::projection` 完成。

use crate::model::VisualSettingsDraft;

use super::super::{AppState, AppUpdateOutcome, Message};

impl AppState {
    /// 分发视觉配置消息。
    pub(super) fn dispatch_visual_message(&mut self, message: Message) -> AppUpdateOutcome {
        match message {
            Message::UpdateVisualSettingsDraft { field, value } => {
                self.ui.set_visual_settings_field(field, value);
                draft_changed()
            }
            Message::SetVisualBackgroundEnabled { enabled } => {
                self.ui.set_visual_background_enabled(enabled);
                draft_changed()
            }
            Message::ApplyVisualSettings => {
                let draft = self.ui.visual_settings.clone();
                let theme = match draft.build_theme_profile(&self.config.theme) {
                    Ok(theme) => theme,
                    Err(error) => return invalid_visual_settings(error.to_string()),
                };
                let background = match draft.build_background_profile(&self.config.background) {
                    Ok(background) => background,
                    Err(error) => return invalid_visual_settings(error.to_string()),
                };

                let outcome = self.core.apply_visual_profiles_action(theme, background);
                self.ui.visual_settings =
                    VisualSettingsDraft::from_profiles(&self.config.theme, &self.config.background);
                outcome
            }
            Message::UpdateHostVisualSettingsDraft {
                host_id,
                field,
                value,
            } => {
                let Some((theme, background)) = host_visual_fallbacks(self, host_id) else {
                    return missing_host(host_id);
                };
                self.ui
                    .set_host_visual_settings_field(host_id, field, value, &theme, &background);
                draft_changed()
            }
            Message::SetHostVisualBackgroundEnabled { host_id, enabled } => {
                let Some((theme, background)) = host_visual_fallbacks(self, host_id) else {
                    return missing_host(host_id);
                };
                self.ui
                    .set_host_visual_background_enabled(host_id, enabled, &theme, &background);
                draft_changed()
            }
            Message::ApplyHostVisualSettings { host_id } => {
                let Some((fallback_theme, fallback_background)) =
                    host_visual_fallbacks(self, host_id)
                else {
                    return missing_host(host_id);
                };
                let draft = self
                    .ui
                    .host_visual_settings_for(host_id)
                    .cloned()
                    .unwrap_or_else(|| {
                        VisualSettingsDraft::from_profiles(&fallback_theme, &fallback_background)
                    });
                let theme = match draft.build_theme_profile(&fallback_theme) {
                    Ok(theme) => theme,
                    Err(error) => return invalid_visual_settings(error.to_string()),
                };
                let background = match draft.build_background_profile(&fallback_background) {
                    Ok(background) => background,
                    Err(error) => return invalid_visual_settings(error.to_string()),
                };

                let outcome = self
                    .core
                    .apply_host_visual_profiles_action(host_id, theme, background);
                self.ui.clear_host_visual_settings_draft(host_id);
                outcome
            }
            Message::ClearHostVisualSettings { host_id } => {
                let outcome = self.core.clear_host_visual_profiles_action(host_id);
                self.ui.clear_host_visual_settings_draft(host_id);
                outcome
            }
            _ => unreachable!("非视觉设置消息不应进入视觉设置路由"),
        }
    }
}

fn draft_changed() -> AppUpdateOutcome {
    AppUpdateOutcome {
        state_changed: true,
        ..AppUpdateOutcome::default()
    }
}

fn invalid_visual_settings(error: String) -> AppUpdateOutcome {
    AppUpdateOutcome {
        error: Some(format!("视觉配置无效：{error}")),
        ..AppUpdateOutcome::default()
    }
}

fn missing_host(host_id: crate::model::HostId) -> AppUpdateOutcome {
    AppUpdateOutcome {
        error: Some(format!("找不到主机：{}", host_id.0)),
        ..AppUpdateOutcome::default()
    }
}

fn host_visual_fallbacks(
    state: &AppState,
    host_id: crate::model::HostId,
) -> Option<(crate::model::ThemeProfile, crate::model::BackgroundProfile)> {
    state
        .storage
        .hosts
        .iter()
        .find(|host| host.id == host_id)
        .map(|host| {
            (
                host.theme_override
                    .clone()
                    .unwrap_or_else(|| state.config.theme.clone()),
                host.background_override
                    .clone()
                    .unwrap_or_else(|| state.config.background.clone()),
            )
        })
}
