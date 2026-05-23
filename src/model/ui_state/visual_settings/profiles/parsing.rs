use super::super::VisualSettingsDraftError;

pub(super) fn normalized_string_or_current(value: &str, current: &str) -> String {
    let value = value.trim();

    if value.is_empty() {
        current.to_owned()
    } else {
        value.to_owned()
    }
}

pub(super) fn parse_optional_positive_f32(
    value: &str,
    current: f32,
) -> Result<f32, VisualSettingsDraftError> {
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

pub(super) fn parse_optional_u64(
    value: &str,
    current: u64,
) -> Result<u64, VisualSettingsDraftError> {
    let value = value.trim();

    if value.is_empty() {
        return Ok(current);
    }

    value
        .parse::<u64>()
        .map_err(|_| VisualSettingsDraftError::InvalidRotationIntervalSecs(value.to_owned()))
}

pub(super) fn parse_optional_opacity(
    value: &str,
    current: f32,
) -> Result<f32, VisualSettingsDraftError> {
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

pub(super) fn parse_optional_blur(
    value: &str,
    current: f32,
) -> Result<f32, VisualSettingsDraftError> {
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
