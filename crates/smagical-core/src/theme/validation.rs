use super::model::{
    TerminalThemeDefinition, ThemeError, ThemeWarning, UiThemeDefinition, UiThemeTokens,
};

pub(crate) fn validate_ui(definition: &UiThemeDefinition) -> Result<(), ThemeError> {
    let colors = [
        ("window-background", &definition.ui.window_background),
        ("panel-background", &definition.ui.panel_background),
        ("surface-background", &definition.ui.surface_background),
        ("control-background", &definition.ui.control_background),
        ("foreground", &definition.ui.foreground),
        ("secondary-foreground", &definition.ui.secondary_foreground),
        ("disabled-foreground", &definition.ui.disabled_foreground),
        ("accent", &definition.ui.accent),
        ("hover-background", &definition.ui.hover_background),
        ("pressed-background", &definition.ui.pressed_background),
        ("selected-background", &definition.ui.selected_background),
        ("selected-foreground", &definition.ui.selected_foreground),
        ("border", &definition.ui.border),
        ("focus-border", &definition.ui.focus_border),
        ("success", &definition.ui.success),
        ("warning", &definition.ui.warning),
        ("danger", &definition.ui.danger),
        ("info", &definition.ui.info),
    ];
    for (field, value) in colors {
        if let Some(value) = value
            && !is_hex_color(value)
        {
            return Err(ThemeError::InvalidColor {
                field: field.into(),
                value: value.clone(),
            });
        }
    }
    let metrics = [
        ("radius-small", definition.metrics.radius_small),
        ("radius-medium", definition.metrics.radius_medium),
        ("radius-large", definition.metrics.radius_large),
        ("spacing-small", definition.metrics.spacing_small),
        ("spacing-medium", definition.metrics.spacing_medium),
        ("spacing-large", definition.metrics.spacing_large),
        ("border-width", definition.metrics.border_width),
        ("control-height", definition.metrics.control_height),
        ("icon-size", definition.metrics.icon_size),
    ];
    for (field, value) in metrics {
        if let Some(value) = value
            && (!value.is_finite() || !(0.0..=256.0).contains(&value))
        {
            return Err(ThemeError::InvalidMetric {
                field: field.into(),
                value,
            });
        }
    }
    Ok(())
}

pub(crate) fn validate_terminal(definition: &TerminalThemeDefinition) -> Result<(), ThemeError> {
    let terminal = &definition.terminal;
    let values = [
        ("background", terminal.background.as_ref()),
        ("foreground", terminal.foreground.as_ref()),
        ("cursor-color", terminal.cursor_color.as_ref()),
        (
            "selection-background",
            terminal.selection_background.as_ref(),
        ),
        ("black", terminal.black.as_ref()),
        ("red", terminal.red.as_ref()),
        ("green", terminal.green.as_ref()),
        ("yellow", terminal.yellow.as_ref()),
        ("blue", terminal.blue.as_ref()),
        ("purple", terminal.purple.as_ref()),
        ("cyan", terminal.cyan.as_ref()),
        ("white", terminal.white.as_ref()),
        ("bright-black", terminal.bright_black.as_ref()),
        ("bright-red", terminal.bright_red.as_ref()),
        ("bright-green", terminal.bright_green.as_ref()),
        ("bright-yellow", terminal.bright_yellow.as_ref()),
        ("bright-blue", terminal.bright_blue.as_ref()),
        ("bright-purple", terminal.bright_purple.as_ref()),
        ("bright-cyan", terminal.bright_cyan.as_ref()),
        ("bright-white", terminal.bright_white.as_ref()),
    ];
    for (field, value) in values {
        if let Some(value) = value
            && !is_hex_color(value)
        {
            return Err(ThemeError::InvalidColor {
                field: field.into(),
                value: value.clone(),
            });
        }
    }
    Ok(())
}

pub(crate) fn warnings_for_ui(tokens: &UiThemeTokens) -> Vec<ThemeWarning> {
    let ratio = contrast_ratio(&tokens.foreground, &tokens.window_background).unwrap_or(21.0);
    (ratio < 4.5)
        .then(|| ThemeWarning::LowContrast {
            foreground: tokens.foreground.clone(),
            background: tokens.window_background.clone(),
            ratio,
        })
        .into_iter()
        .collect()
}

fn contrast_ratio(foreground: &str, background: &str) -> Option<f32> {
    let luminance = |color: &str| -> Option<f32> {
        let hex = color.strip_prefix('#')?;
        let channel = |start| {
            u8::from_str_radix(&hex[start..start + 2], 16)
                .ok()
                .map(|v| {
                    let value = v as f32 / 255.0;
                    if value <= 0.04045 {
                        value / 12.92
                    } else {
                        ((value + 0.055) / 1.055).powf(2.4)
                    }
                })
        };
        Some(0.2126 * channel(0)? + 0.7152 * channel(2)? + 0.0722 * channel(4)?)
    };
    let a = luminance(foreground)?;
    let b = luminance(background)?;
    Some((a.max(b) + 0.05) / (a.min(b) + 0.05))
}

pub(crate) fn is_hex_color(value: &str) -> bool {
    let Some(hex) = value.strip_prefix('#') else {
        return false;
    };
    matches!(hex.len(), 6 | 8) && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
}
