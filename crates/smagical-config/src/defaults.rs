//! 应用启动默认配置。

use smagical_core::{BackgroundProfile, ThemeProfile};

use super::{AppConfig, HostListModePreference, SecurityPreferences, WorkspacePreferences};

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
            workspace: WorkspacePreferences::default(),
            security: SecurityPreferences::default(),
        }
    }
}

impl Default for WorkspacePreferences {
    fn default() -> Self {
        Self {
            host_list_mode: HostListModePreference::Tree,
            language: super::LanguagePreference::FollowSystem,
            built_in_theme: super::BuiltInThemePreference::ProfessionalDark,
        }
    }
}

impl Default for SecurityPreferences {
    fn default() -> Self {
        Self {
            encryption: super::StorageEncryptionPreference::Disabled,
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

    #[test]
    fn default_workspace_preferences_use_tree_hosts() {
        let config = AppConfig::default();

        assert_eq!(
            config.workspace.host_list_mode,
            HostListModePreference::Tree
        );
        assert_eq!(
            config.workspace.language,
            crate::LanguagePreference::FollowSystem
        );
        assert_eq!(
            config.workspace.built_in_theme,
            crate::BuiltInThemePreference::ProfessionalDark
        );
    }

    #[test]
    fn default_security_preferences_leave_storage_unencrypted() {
        let config = AppConfig::default();

        assert_eq!(
            config.security.encryption,
            crate::StorageEncryptionPreference::Disabled
        );
    }
}
