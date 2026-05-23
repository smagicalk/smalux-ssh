//! 视觉配置草稿类型。

use crate::model::{BackgroundProfile, ThemeProfile};

use super::{VisualSettingsDraftError, VisualSettingsDraftField, profiles};

/// 全局视觉配置草稿。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisualSettingsDraft {
    pub theme_name: String,
    pub font_family: String,
    pub font_size: String,
    pub background_enabled: bool,
    pub background_sources: String,
    pub rotation_interval_secs: String,
    pub opacity: String,
    pub blur: String,
}

impl Default for VisualSettingsDraft {
    fn default() -> Self {
        Self {
            theme_name: "Default Dark".to_owned(),
            font_family: "JetBrains Mono".to_owned(),
            font_size: "14".to_owned(),
            background_enabled: false,
            background_sources: String::new(),
            rotation_interval_secs: "300".to_owned(),
            opacity: "0.18".to_owned(),
            blur: "8".to_owned(),
        }
    }
}

impl VisualSettingsDraft {
    /// 使用当前配置生成可编辑草稿。
    pub fn from_profiles(theme: &ThemeProfile, background: &BackgroundProfile) -> Self {
        profiles::visual_settings_draft_from_profiles(theme, background)
    }

    /// 更新草稿字段。
    pub fn set_field(&mut self, field: VisualSettingsDraftField, value: impl Into<String>) {
        let value = value.into();

        match field {
            VisualSettingsDraftField::ThemeName => self.theme_name = value,
            VisualSettingsDraftField::FontFamily => self.font_family = value,
            VisualSettingsDraftField::FontSize => self.font_size = value,
            VisualSettingsDraftField::BackgroundSources => self.background_sources = value,
            VisualSettingsDraftField::RotationIntervalSecs => self.rotation_interval_secs = value,
            VisualSettingsDraftField::Opacity => self.opacity = value,
            VisualSettingsDraftField::Blur => self.blur = value,
        }
    }

    /// 设置背景开关。
    pub fn set_background_enabled(&mut self, enabled: bool) {
        self.background_enabled = enabled;
    }

    /// 生成最终主题配置。
    pub fn build_theme_profile(
        &self,
        current: &ThemeProfile,
    ) -> Result<ThemeProfile, VisualSettingsDraftError> {
        profiles::build_theme_profile(self, current)
    }

    /// 生成最终背景配置。
    pub fn build_background_profile(
        &self,
        current: &BackgroundProfile,
    ) -> Result<BackgroundProfile, VisualSettingsDraftError> {
        profiles::build_background_profile(self, current)
    }
}
