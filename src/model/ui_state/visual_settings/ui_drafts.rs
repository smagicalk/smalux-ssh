//! UI 视觉配置草稿状态管理。

use crate::model::{BackgroundProfile, HostId, ThemeProfile};

use super::{VisualSettingsDraft, VisualSettingsDraftField};
use crate::model::UiState;

/// 单台主机的视觉配置草稿。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostVisualSettingsDraft {
    pub host_id: HostId,
    pub settings: VisualSettingsDraft,
}

impl UiState {
    /// 更新全局视觉配置草稿字段。
    pub fn set_visual_settings_field(
        &mut self,
        field: VisualSettingsDraftField,
        value: impl Into<String>,
    ) {
        self.visual_settings.set_field(field, value);
    }

    /// 更新全局背景开关草稿。
    pub fn set_visual_background_enabled(&mut self, enabled: bool) {
        self.visual_settings.set_background_enabled(enabled);
    }

    /// 返回指定主机的视觉配置草稿。
    pub fn host_visual_settings_for(&self, host_id: HostId) -> Option<&VisualSettingsDraft> {
        self.host_visual_settings_drafts
            .iter()
            .find(|draft| draft.host_id == host_id)
            .map(|draft| &draft.settings)
    }

    /// 准备指定主机的视觉配置草稿。
    pub fn ensure_host_visual_settings_draft(
        &mut self,
        host_id: HostId,
        theme: &ThemeProfile,
        background: &BackgroundProfile,
    ) -> &mut VisualSettingsDraft {
        if let Some(index) = self
            .host_visual_settings_drafts
            .iter()
            .position(|draft| draft.host_id == host_id)
        {
            return &mut self.host_visual_settings_drafts[index].settings;
        }

        self.host_visual_settings_drafts
            .push(HostVisualSettingsDraft {
                host_id,
                settings: VisualSettingsDraft::from_profiles(theme, background),
            });
        &mut self
            .host_visual_settings_drafts
            .last_mut()
            .expect("刚插入的主机视觉草稿应该存在")
            .settings
    }

    /// 更新指定主机的视觉配置草稿字段。
    pub fn set_host_visual_settings_field(
        &mut self,
        host_id: HostId,
        field: VisualSettingsDraftField,
        value: impl Into<String>,
        fallback_theme: &ThemeProfile,
        fallback_background: &BackgroundProfile,
    ) {
        self.ensure_host_visual_settings_draft(host_id, fallback_theme, fallback_background)
            .set_field(field, value);
    }

    /// 更新指定主机的背景开关草稿。
    pub fn set_host_visual_background_enabled(
        &mut self,
        host_id: HostId,
        enabled: bool,
        fallback_theme: &ThemeProfile,
        fallback_background: &BackgroundProfile,
    ) {
        self.ensure_host_visual_settings_draft(host_id, fallback_theme, fallback_background)
            .set_background_enabled(enabled);
    }

    /// 清除指定主机的视觉配置草稿。
    pub fn clear_host_visual_settings_draft(&mut self, host_id: HostId) {
        self.host_visual_settings_drafts
            .retain(|draft| draft.host_id != host_id);
    }
}
