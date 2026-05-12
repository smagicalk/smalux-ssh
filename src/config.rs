//! 应用配置的默认值。
//!
//! 这个模块只负责给启动流程提供稳定的基础配置，并提供全局配置与主机覆盖配置的合并逻辑。
//! 后续落盘和迁移会放在存储层处理，避免 UI 和连接逻辑直接依赖文件格式。

use crate::model::{BackgroundProfile, Host, ThemeProfile};

const MIN_BACKGROUND_ROTATION_SECS: u64 = 5;
const MAX_BACKGROUND_BLUR: f32 = 64.0;

/// 桌面端运行所需的全局配置。
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub app_name: String,
    pub theme: ThemeProfile,
    pub background: BackgroundProfile,
}

/// 主机最终生效的视觉配置。
///
/// UI、终端渲染和背景轮转只应读取解析后的结果，避免每个调用点重复处理覆盖和边界值。
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedVisualConfig {
    pub theme: ThemeProfile,
    pub background: BackgroundProfile,
}

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
    use crate::model::{
        AuthProfile, BackgroundProfile, Host, HostId, ImageSource, SecretRef, ThemeProfile,
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
