//! 背景图片参数归一化。

use crate::model::BackgroundProfile;

const MIN_BACKGROUND_ROTATION_SECS: u64 = 5;
const MAX_BACKGROUND_BLUR: f32 = 64.0;

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
    use crate::model::ImageSource;

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
