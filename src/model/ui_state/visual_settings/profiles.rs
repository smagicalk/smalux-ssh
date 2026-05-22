//! 视觉配置草稿与持久配置之间的转换。

use crate::model::{BackgroundProfile, ImageSource, ThemeProfile};

use super::{VisualSettingsDraft, VisualSettingsDraftError};

pub(super) fn visual_settings_draft_from_profiles(
    theme: &ThemeProfile,
    background: &BackgroundProfile,
) -> VisualSettingsDraft {
    VisualSettingsDraft {
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

pub(super) fn build_theme_profile(
    draft: &VisualSettingsDraft,
    current: &ThemeProfile,
) -> Result<ThemeProfile, VisualSettingsDraftError> {
    let theme_name = normalized_string_or_current(&draft.theme_name, &current.name);
    let font_family = normalized_string_or_current(&draft.font_family, &current.font_family);
    let font_size = parse_optional_positive_f32(&draft.font_size, current.font_size)?;

    Ok(ThemeProfile {
        name: theme_name,
        font_family,
        font_size,
    })
}

pub(super) fn build_background_profile(
    draft: &VisualSettingsDraft,
    current: &BackgroundProfile,
) -> Result<BackgroundProfile, VisualSettingsDraftError> {
    let sources = parse_background_sources(&draft.background_sources)?;
    let rotation_interval_secs = parse_optional_u64(
        &draft.rotation_interval_secs,
        current.rotation_interval_secs,
    )?;
    let opacity = parse_optional_opacity(&draft.opacity, current.opacity)?;
    let blur = parse_optional_blur(&draft.blur, current.blur)?;

    Ok(BackgroundProfile {
        enabled: draft.background_enabled,
        sources,
        rotation_interval_secs,
        opacity,
        blur,
    }
    .normalized())
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
