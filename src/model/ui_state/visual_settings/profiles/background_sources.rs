use crate::model::ImageSource;

use super::super::VisualSettingsDraftError;

pub(super) fn parse_background_sources(
    raw: &str,
) -> Result<Vec<ImageSource>, VisualSettingsDraftError> {
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

pub(super) fn format_background_sources(sources: &[ImageSource]) -> String {
    sources
        .iter()
        .map(|source| match source {
            ImageSource::LocalPath(path) => path.clone(),
            ImageSource::Url(url) => format!("url:{url}"),
        })
        .collect::<Vec<_>>()
        .join(", ")
}
