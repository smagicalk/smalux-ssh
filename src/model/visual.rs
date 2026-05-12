//! 主题和背景视觉配置。

use serde::{Deserialize, Serialize};

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
}
