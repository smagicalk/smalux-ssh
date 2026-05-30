//! 全局视觉配置与主机覆盖配置解析。

use smagical_core::Host;

use super::{AppConfig, ResolvedVisualConfig};

impl AppConfig {
    /// 解析某个主机最终生效的主题和背景配置。
    pub fn resolve_visual_for_host(&self, host: Option<&Host>) -> ResolvedVisualConfig {
        let theme = host
            .and_then(|host| host.theme_override.clone())
            .unwrap_or_else(|| self.theme.clone());
        let background = host
            .and_then(|host| host.background_override.clone())
            .unwrap_or_else(|| self.background.clone())
            .normalized();

        ResolvedVisualConfig { theme, background }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smagical_core::{
        AuthProfile, BackgroundProfile, HostId, ImageSource, SecretRef, ThemeProfile,
    };
    use uuid::Uuid;

    fn host_with_overrides(
        theme_override: Option<ThemeProfile>,
        background_override: Option<BackgroundProfile>,
    ) -> Host {
        Host {
            id: HostId(Uuid::new_v4()),
            name: "visual-host".to_owned(),
            group_id: None,
            icon_key: "server".to_owned(),
            tags: Vec::new(),
            address: "example.com".to_owned(),
            port: 22,
            auth: AuthProfile::Password {
                username: "ops".to_owned(),
                secret: SecretRef("password:ops".to_owned()),
            },
            proxy: None,
            jumps: Vec::new(),
            theme_override,
            background_override,
        }
    }

    #[test]
    fn visual_resolution_uses_global_config_without_host_override() {
        let config = AppConfig::default();

        let resolved = config.resolve_visual_for_host(None);

        assert_eq!(resolved.theme, config.theme);
        assert_eq!(resolved.background, config.background.normalized());
    }

    #[test]
    fn visual_resolution_prefers_host_theme_and_background_override() {
        let config = AppConfig::default();
        let host_theme = ThemeProfile {
            name: "Host Light".to_owned(),
            font_family: "Maple Mono".to_owned(),
            font_size: 16.0,
        };
        let host_background = BackgroundProfile {
            enabled: true,
            sources: vec![ImageSource::LocalPath("wallpapers/host.png".to_owned())],
            rotation_interval_secs: 30,
            opacity: 0.35,
            blur: 10.0,
        };
        let host = host_with_overrides(Some(host_theme.clone()), Some(host_background.clone()));

        let resolved = config.resolve_visual_for_host(Some(&host));

        assert_eq!(resolved.theme, host_theme);
        assert_eq!(resolved.background, host_background);
    }

    #[test]
    fn visual_resolution_can_mix_host_theme_with_global_background() {
        let config = AppConfig::default();
        let host_theme = ThemeProfile {
            name: "Host Only Theme".to_owned(),
            font_family: "JetBrains Mono".to_owned(),
            font_size: 15.0,
        };
        let host = host_with_overrides(Some(host_theme.clone()), None);

        let resolved = config.resolve_visual_for_host(Some(&host));

        assert_eq!(resolved.theme, host_theme);
        assert_eq!(resolved.background, config.background.normalized());
    }
}
