//! 视觉配置草稿与持久配置之间的转换。

use crate::model::{BackgroundProfile, ThemeProfile};

use super::{VisualSettingsDraft, VisualSettingsDraftError};

#[path = "profiles/background_sources.rs"]
mod background_sources;
#[path = "profiles/parsing.rs"]
mod parsing;

use background_sources::{format_background_sources, parse_background_sources};
use parsing::{
    normalized_string_or_current, parse_optional_blur, parse_optional_opacity,
    parse_optional_positive_f32, parse_optional_u64,
};

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
