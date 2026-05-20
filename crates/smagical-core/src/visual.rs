//! 主题和背景视觉配置。

use serde::{Deserialize, Serialize};

const MIN_BACKGROUND_ROTATION_SECS: u64 = 5;
const MAX_BACKGROUND_BLUR: f32 = 64.0;

/// 终端主题和字体配置。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThemeProfile {
    pub name: String,
    pub font_family: String,
    pub font_size: f32,
}

/// 背景图片轮转和渲染参数。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackgroundProfile {
    pub enabled: bool,
    pub sources: Vec<ImageSource>,
    pub rotation_interval_secs: u64,
    pub opacity: f32,
    pub blur: f32,
}

/// 背景图片来源。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageSource {
    LocalPath(String),
    Url(String),
}

impl BackgroundProfile {
    /// 返回适合渲染层直接使用的背景配置。
    pub fn normalized(&self) -> Self {
        Self {
            enabled: self.enabled && !self.sources.is_empty(),
            sources: self.sources.clone(),
            rotation_interval_secs: self
                .rotation_interval_secs
                .max(MIN_BACKGROUND_ROTATION_SECS),
            opacity: self.opacity.clamp(0.0, 1.0),
            blur: self.blur.clamp(0.0, MAX_BACKGROUND_BLUR),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn background_profile_round_trips_with_all_source_kinds() {
        let profile = BackgroundProfile {
            enabled: true,
            sources: vec![
                ImageSource::LocalPath("wallpapers/a.jpg".to_owned()),
                ImageSource::Url("https://example.com/b.jpg".to_owned()),
            ],
            rotation_interval_secs: 120,
            opacity: 0.4,
            blur: 12.0,
        };

        let encoded = toml::to_string(&profile).expect("背景配置应该可以序列化为 TOML");
        let decoded: BackgroundProfile =
            toml::from_str(&encoded).expect("背景配置应该可以从 TOML 反序列化");

        assert!(decoded.enabled);
        assert_eq!(decoded.sources.len(), 2);
        assert_eq!(decoded.rotation_interval_secs, 120);
        assert_eq!(decoded.opacity, 0.4);
        assert_eq!(decoded.blur, 12.0);
    }

    #[test]
    fn background_normalization_clamps_render_parameters() {
        let background = BackgroundProfile {
            enabled: true,
            sources: vec![ImageSource::Url(
                "https://example.com/wallpaper.jpg".to_owned(),
            )],
            rotation_interval_secs: 0,
            opacity: 2.0,
            blur: 128.0,
        };

        let normalized = background.normalized();

        assert!(normalized.enabled);
        assert_eq!(normalized.rotation_interval_secs, 5);
        assert_eq!(normalized.opacity, 1.0);
        assert_eq!(normalized.blur, 64.0);
    }

    #[test]
    fn background_normalization_disables_empty_playlists() {
        let background = BackgroundProfile {
            enabled: true,
            sources: Vec::new(),
            rotation_interval_secs: 60,
            opacity: -1.0,
            blur: -8.0,
        };

        let normalized = background.normalized();

        assert!(!normalized.enabled);
        assert_eq!(normalized.opacity, 0.0);
        assert_eq!(normalized.blur, 0.0);
    }
}
