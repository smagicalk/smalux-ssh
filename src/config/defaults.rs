//! 应用启动默认配置。

use crate::model::{BackgroundProfile, ThemeProfile};

use super::AppConfig;

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            app_name: String::from("smagicalssh"),
            theme: ThemeProfile {
                name: String::from("Default Dark"),
                font_family: String::from("JetBrains Mono"),
                font_size: 14.0,
            },
            background: BackgroundProfile {
                enabled: false,
                sources: Vec::new(),
                rotation_interval_secs: 300,
                opacity: 0.18,
                blur: 8.0,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_expected_identity() {
        let config = AppConfig::default();

        assert_eq!(config.app_name, "smagicalssh");
    }

    #[test]
    fn default_theme_uses_terminal_friendly_baseline() {
        let config = AppConfig::default();

        assert_eq!(config.theme.name, "Default Dark");
        assert_eq!(config.theme.font_family, "JetBrains Mono");
        assert_eq!(config.theme.font_size, 14.0);
    }

    #[test]
    fn default_background_is_disabled_but_tunable() {
        let config = AppConfig::default();

        assert!(!config.background.enabled);
        assert!(config.background.sources.is_empty());
        assert_eq!(config.background.rotation_interval_secs, 300);
        assert_eq!(config.background.opacity, 0.18);
        assert_eq!(config.background.blur, 8.0);
    }
}
