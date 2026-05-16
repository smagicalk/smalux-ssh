//! 视觉配置草稿。

use std::fmt;

use crate::model::{BackgroundProfile, ImageSource, ThemeProfile};

use super::{
    BackgroundProfile as UiBackgroundProfile, HostId, ThemeProfile as UiThemeProfile, UiState,
};

/// 全局视觉配置草稿字段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualSettingsDraftField {
    ThemeName,
    FontFamily,
    FontSize,
    BackgroundSources,
    RotationIntervalSecs,
    Opacity,
    Blur,
}

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
        Self {
            theme_name: theme.name.clone(),
            font_family: theme.font_family.clone(),
            font_size: theme.font_size.to_string(),
            background_enabled: background.enabled,
            background_sources: format_background_sources(&background.sources),
            rotation_interval_secs: background.rotation_interval_secs.to_string(),
            opacity: background.opacity.to_string(),
            blur: background.blur.to_string(),
        }
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
        let theme_name = normalized_string_or_current(&self.theme_name, &current.name);
        let font_family = normalized_string_or_current(&self.font_family, &current.font_family);
        let font_size = parse_optional_positive_f32(&self.font_size, current.font_size)?;

        Ok(ThemeProfile {
            name: theme_name,
            font_family,
            font_size,
        })
    }

    /// 生成最终背景配置。
    pub fn build_background_profile(
        &self,
        current: &BackgroundProfile,
    ) -> Result<BackgroundProfile, VisualSettingsDraftError> {
        let sources = parse_background_sources(&self.background_sources)?;
        let rotation_interval_secs =
            parse_optional_u64(&self.rotation_interval_secs, current.rotation_interval_secs)?;
        let opacity = parse_optional_opacity(&self.opacity, current.opacity)?;
        let blur = parse_optional_blur(&self.blur, current.blur)?;

        Ok(BackgroundProfile {
            enabled: self.background_enabled,
            sources,
            rotation_interval_secs,
            opacity,
            blur,
        })
        .map(|background| background.normalized())
    }
}

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
        theme: &UiThemeProfile,
        background: &UiBackgroundProfile,
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
        fallback_theme: &UiThemeProfile,
        fallback_background: &UiBackgroundProfile,
    ) {
        self.ensure_host_visual_settings_draft(host_id, fallback_theme, fallback_background)
            .set_field(field, value);
    }

    /// 更新指定主机的背景开关草稿。
    pub fn set_host_visual_background_enabled(
        &mut self,
        host_id: HostId,
        enabled: bool,
        fallback_theme: &UiThemeProfile,
        fallback_background: &UiBackgroundProfile,
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

/// 视觉配置草稿错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VisualSettingsDraftError {
    InvalidFontSize(String),
    InvalidRotationIntervalSecs(String),
    InvalidOpacity(String),
    InvalidBlur(String),
    InvalidBackgroundSource(String),
}

impl fmt::Display for VisualSettingsDraftError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFontSize(value) => write!(f, "无效的字号：{value}"),
            Self::InvalidRotationIntervalSecs(value) => write!(f, "无效的轮转间隔：{value}"),
            Self::InvalidOpacity(value) => write!(f, "无效的透明度：{value}"),
            Self::InvalidBlur(value) => write!(f, "无效的模糊度：{value}"),
            Self::InvalidBackgroundSource(value) => write!(f, "无效的背景来源：{value}"),
        }
    }
}

fn normalized_string_or_current(value: &str, current: &str) -> String {
    let value = value.trim();

    if value.is_empty() {
        current.to_owned()
    } else {
        value.to_owned()
    }
}

fn parse_optional_positive_f32(value: &str, current: f32) -> Result<f32, VisualSettingsDraftError> {
    let value = value.trim();

    if value.is_empty() {
        return Ok(current);
    }

    let parsed = value
        .parse::<f32>()
        .map_err(|_| VisualSettingsDraftError::InvalidFontSize(value.to_owned()))?;
    if parsed <= 0.0 || !parsed.is_finite() {
        return Err(VisualSettingsDraftError::InvalidFontSize(value.to_owned()));
    }

    Ok(parsed)
}

fn parse_optional_u64(value: &str, current: u64) -> Result<u64, VisualSettingsDraftError> {
    let value = value.trim();

    if value.is_empty() {
        return Ok(current);
    }

    value
        .parse::<u64>()
        .map_err(|_| VisualSettingsDraftError::InvalidRotationIntervalSecs(value.to_owned()))
}

fn parse_optional_opacity(value: &str, current: f32) -> Result<f32, VisualSettingsDraftError> {
    let value = value.trim();

    if value.is_empty() {
        return Ok(current);
    }

    value
        .parse::<f32>()
        .map_err(|_| VisualSettingsDraftError::InvalidOpacity(value.to_owned()))
        .and_then(|parsed| {
            if parsed.is_finite() {
                Ok(parsed)
            } else {
                Err(VisualSettingsDraftError::InvalidOpacity(value.to_owned()))
            }
        })
}

fn parse_optional_blur(value: &str, current: f32) -> Result<f32, VisualSettingsDraftError> {
    let value = value.trim();

    if value.is_empty() {
        return Ok(current);
    }

    value
        .parse::<f32>()
        .map_err(|_| VisualSettingsDraftError::InvalidBlur(value.to_owned()))
        .and_then(|parsed| {
            if parsed.is_finite() {
                Ok(parsed)
            } else {
                Err(VisualSettingsDraftError::InvalidBlur(value.to_owned()))
            }
        })
}

fn parse_background_sources(raw: &str) -> Result<Vec<ImageSource>, VisualSettingsDraftError> {
    raw.split(['\n', ',', ';'])
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(parse_background_source)
        .collect()
}

fn parse_background_source(value: &str) -> Result<ImageSource, VisualSettingsDraftError> {
    if let Some(url) = value.strip_prefix("url:") {
        let url = url.trim();
        if url.is_empty() {
            return Err(VisualSettingsDraftError::InvalidBackgroundSource(
                value.to_owned(),
            ));
        }

        return Ok(ImageSource::Url(url.to_owned()));
    }

    if value.contains("://") {
        return Ok(ImageSource::Url(value.to_owned()));
    }

    Ok(ImageSource::LocalPath(value.to_owned()))
}

fn format_background_sources(sources: &[ImageSource]) -> String {
    sources
        .iter()
        .map(|source| match source {
            ImageSource::LocalPath(path) => path.clone(),
            ImageSource::Url(url) => format!("url:{url}"),
        })
        .collect::<Vec<_>>()
        .join(", ")
}
