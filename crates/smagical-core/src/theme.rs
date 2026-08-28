//! 与 UI 框架无关的主题模型、解析与存储 API。

mod model;
mod repository;
mod selection;
mod service;
mod validation;

pub use model::{
    ColorScheme, ResolvedTerminalTheme, ResolvedUiTheme, TerminalThemeDefinition,
    TerminalThemeTokens, TerminalThemeTokensPatch, ThemeError, ThemeId, ThemeKind, ThemeMetadata,
    ThemePeriod, ThemeWarning, UiThemeDefinition, UiThemeMetrics, UiThemeMetricsPatch,
    UiThemeTokens, UiThemeTokensPatch,
};
pub use repository::{FileThemeRepository, LoadedTheme, ThemeRepository};
pub use selection::{ThemeDeleteImpact, ThemeSelectionConfig};
pub use service::{TerminalImport, ThemeService};

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, path::PathBuf};

    use uuid::Uuid;

    use super::{
        ColorScheme, FileThemeRepository, LoadedTheme, TerminalThemeDefinition, ThemeId,
        ThemePeriod, ThemeRepository, ThemeSelectionConfig,
    };
    use super::{TerminalImport, ThemeError, ThemeService, UiThemeDefinition};

    const DARCULA_TOML: &str = r##"
schema-version = 1
id = "builtin.ui.darcula"
name = "Darcula"
kind = "ui"
period = "night"

[ui]
color-scheme = "dark"
window-background = "#2B2B2B"
accent = "#589DF6"
"##;

    const TERMINAL_DARCULA_TOML: &str = r##"
schema-version = 1
id = "builtin.terminal.darcula"
name = "Darcula"
kind = "terminal"

[terminal]
background = "#101010"
foreground = "#EEEEEE"
"##;

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            Self(std::env::temp_dir().join(format!("smalux-theme-test-{}", Uuid::new_v4())))
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn custom_ui_theme_inherits_unspecified_tokens() {
        let mut themes = ThemeService::new();
        let darcula: UiThemeDefinition = themes.import_ui_toml(DARCULA_TOML).unwrap();
        themes.register_builtin_ui(darcula).unwrap();

        let custom = themes
            .import_ui_toml(
                r##"
schema-version = 1
id = "4d94cfa2-c62b-46b8-aa67-c0a45cc68144"
name = "My Darcula"
kind = "ui"
period = "night"
base = "builtin.ui.darcula"

[ui]
accent = "#71A8FF"
"##,
            )
            .unwrap();
        themes.save_ui(custom).unwrap();

        let resolved = themes
            .resolve_ui("4d94cfa2-c62b-46b8-aa67-c0a45cc68144")
            .unwrap();
        assert_eq!(resolved.tokens.window_background, "#2B2B2B");
        assert_eq!(resolved.tokens.accent, "#71A8FF");
    }

    #[test]
    fn windows_terminal_import_returns_candidates_without_saving() {
        let themes = ThemeService::new();
        let imported = themes
            .import_windows_terminal_json(
                r##"{
                    "schemes": [{
                        "name": "Example",
                        "background": "#101010",
                        "foreground": "#EEEEEE",
                        "black": "#000000",
                        "red": "#CC0000",
                        "green": "#00CC00",
                        "yellow": "#CCCC00",
                        "blue": "#0000CC",
                        "purple": "#CC00CC",
                        "cyan": "#00CCCC",
                        "white": "#CCCCCC",
                        "brightBlack": "#666666",
                        "brightRed": "#FF0000",
                        "brightGreen": "#00FF00",
                        "brightYellow": "#FFFF00",
                        "brightBlue": "#0000FF",
                        "brightPurple": "#FF00FF",
                        "brightCyan": "#00FFFF",
                        "brightWhite": "#FFFFFF"
                    }]
                }"##,
            )
            .unwrap();

        let TerminalImport::Candidates(candidates) = imported;
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].metadata.name, "Example");
        assert_eq!(
            candidates[0].terminal.bright_blue.as_deref(),
            Some("#0000FF")
        );
        assert!(themes.list_terminal().is_empty());
    }

    #[test]
    fn invalid_color_is_rejected_before_theme_is_saved() {
        let themes = ThemeService::new();
        let error = themes
            .import_ui_toml(
                r##"
schema-version = 1
id = "invalid-theme"
name = "Invalid"
kind = "ui"
period = "night"

[ui]
window-background = "not-a-color"
"##,
            )
            .unwrap_err();

        assert!(matches!(
            error,
            ThemeError::InvalidColor { field, .. } if field == "window-background"
        ));
    }

    #[test]
    fn host_terminal_selection_overrides_global_default() {
        let host = Uuid::new_v4();
        let config = ThemeSelectionConfig {
            ui_theme: ThemeId::new("builtin.ui.darcula"),
            default_terminal_theme: ThemeId::new("builtin.terminal.darcula"),
            host_terminal_themes: HashMap::from([(host, ThemeId::new("custom.terminal.ops"))]),
        };

        assert_eq!(
            config.effective_terminal_theme(host).as_ref(),
            "custom.terminal.ops"
        );
        assert_eq!(
            config.effective_terminal_theme(Uuid::new_v4()).as_ref(),
            "builtin.terminal.darcula"
        );
    }

    #[test]
    fn file_repository_discovers_a_saved_ui_theme() {
        let directory = TestDirectory::new();
        let repository = FileThemeRepository::from_directory(&directory.0).unwrap();
        let theme: UiThemeDefinition = ThemeService::new().import_ui_toml(DARCULA_TOML).unwrap();

        repository.save_ui(&theme).unwrap();
        let discovered = repository.discover().unwrap();

        assert!(
            matches!(&discovered[..], [LoadedTheme::Ui(found)] if found.metadata.id == theme.metadata.id)
        );
    }

    #[test]
    fn builtin_theme_cannot_be_replaced_or_removed() {
        let mut themes = ThemeService::new();
        let darcula = themes.import_ui_toml(DARCULA_TOML).unwrap();
        themes.register_builtin_ui(darcula.clone()).unwrap();

        assert!(matches!(
            themes.replace_ui(darcula),
            Err(ThemeError::ReadOnlyBuiltin(id)) if id.as_ref() == "builtin.ui.darcula"
        ));
        assert!(matches!(
            themes.remove_ui("builtin.ui.darcula"),
            Err(ThemeError::ReadOnlyBuiltin(id)) if id.as_ref() == "builtin.ui.darcula"
        ));
    }

    #[test]
    fn terminal_theme_inherits_missing_ansi_tokens() {
        let mut themes = ThemeService::new();
        let base: TerminalThemeDefinition = themes
            .import_terminal_toml(
                r##"
schema-version = 1
id = "builtin.terminal.base"
name = "Base"
kind = "terminal"

[terminal]
background = "#101010"
"##,
            )
            .unwrap();
        themes.register_builtin_terminal(base).unwrap();
        let child = themes
            .import_terminal_toml(
                r##"
schema-version = 1
id = "custom.terminal.child"
name = "Child"
kind = "terminal"
base = "builtin.terminal.base"

[terminal]
foreground = "#EEEEEE"
"##,
            )
            .unwrap();
        themes.save_terminal(child).unwrap();

        let resolved = themes.resolve_terminal("custom.terminal.child").unwrap();
        assert_eq!(resolved.tokens.background, "#101010");
        assert_eq!(resolved.tokens.foreground, "#EEEEEE");
        assert_eq!(resolved.tokens.bright_blue, "#75A8D3");
    }

    #[test]
    fn theme_selection_round_trips_through_toml() {
        let original = ThemeSelectionConfig::default();
        let encoded = original.to_toml().unwrap();
        let decoded = ThemeSelectionConfig::from_toml(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn low_contrast_theme_returns_warning_without_failing_validation() {
        let themes = ThemeService::new();
        let theme = themes
            .import_ui_toml(
                r##"
schema-version = 1
id = "low-contrast"
name = "Low Contrast"
kind = "ui"
period = "night"

[ui]
window-background = "#333333"
foreground = "#383838"
"##,
            )
            .unwrap();

        let warnings = themes.validate_ui(&theme).unwrap();
        assert!(matches!(
            warnings.as_slice(),
            [super::ThemeWarning::LowContrast { .. }]
        ));
    }

    #[test]
    fn cyclic_theme_inheritance_is_rejected_when_resolving() {
        let mut themes = ThemeService::new();
        let first = themes
            .import_ui_toml(
                r##"
schema-version = 1
id = "cycle-a"
name = "Cycle A"
kind = "ui"
period = "night"
base = "cycle-b"

[ui]
"##,
            )
            .unwrap();
        let second = themes
            .import_ui_toml(
                r##"
schema-version = 1
id = "cycle-b"
name = "Cycle B"
kind = "ui"
period = "night"
base = "cycle-a"

[ui]
"##,
            )
            .unwrap();
        themes.save_ui(first).unwrap();
        themes.save_ui(second).unwrap();

        assert!(matches!(
            themes.resolve_ui("cycle-a"),
            Err(ThemeError::InheritanceCycle(_))
        ));
    }

    #[test]
    fn ui_and_terminal_toml_round_trip_without_data_loss() {
        let themes = ThemeService::new();
        let ui = themes.import_ui_toml(DARCULA_TOML).unwrap();
        let ui_encoded = themes.export_ui_toml(&ui).unwrap();
        assert_eq!(themes.import_ui_toml(&ui_encoded).unwrap(), ui);

        let terminal = themes.import_terminal_toml(TERMINAL_DARCULA_TOML).unwrap();
        let terminal_encoded = themes.export_terminal_toml(&terminal).unwrap();
        assert_eq!(
            themes.import_terminal_toml(&terminal_encoded).unwrap(),
            terminal
        );
    }

    #[test]
    fn invalid_metadata_returns_specific_errors() {
        let themes = ThemeService::new();

        let missing_period = DARCULA_TOML.replace("period = \"night\"", "");
        assert!(matches!(
            themes.import_ui_toml(&missing_period),
            Err(ThemeError::MissingPeriod(_))
        ));

        let invalid_period = TERMINAL_DARCULA_TOML.replace(
            "kind = \"terminal\"",
            "kind = \"terminal\"\nperiod = \"night\"",
        );
        assert!(matches!(
            themes.import_terminal_toml(&invalid_period),
            Err(ThemeError::InvalidPeriod(_))
        ));

        let unsupported = DARCULA_TOML.replace("schema-version = 1", "schema-version = 99");
        assert!(matches!(
            themes.import_ui_toml(&unsupported),
            Err(ThemeError::UnsupportedSchema(99))
        ));

        let empty_name = DARCULA_TOML.replace("name = \"Darcula\"", "name = \"   \"");
        assert!(matches!(
            themes.import_ui_toml(&empty_name),
            Err(ThemeError::EmptyName)
        ));

        let invalid_id = DARCULA_TOML.replace("id = \"builtin.ui.darcula\"", "id = \"invalid id\"");
        assert!(matches!(
            themes.import_ui_toml(&invalid_id),
            Err(ThemeError::InvalidId(_))
        ));

        let wrong_kind = DARCULA_TOML.replace("kind = \"ui\"", "kind = \"terminal\"");
        assert!(matches!(
            themes.import_ui_toml(&wrong_kind),
            Err(ThemeError::KindMismatch { .. })
        ));
    }

    #[test]
    fn period_must_match_the_resolved_color_scheme() {
        let mut themes = ThemeService::new();
        let mismatched = themes
            .import_ui_toml(
                r##"
schema-version = 1
id = "mismatched"
name = "Mismatched"
kind = "ui"
period = "day"

[ui]
color-scheme = "dark"
"##,
            )
            .unwrap();
        themes.save_ui(mismatched).unwrap();

        assert!(matches!(
            themes.resolve_ui("mismatched"),
            Err(ThemeError::PeriodSchemeMismatch {
                period: ThemePeriod::Day,
                color_scheme: ColorScheme::Dark,
                ..
            })
        ));
    }

    #[test]
    fn custom_theme_can_be_replaced_and_removed() {
        let mut themes = ThemeService::new();
        let mut custom = themes
            .import_ui_toml(
                r##"
schema-version = 1
id = "custom.ui.lifecycle"
name = "Original"
kind = "ui"
period = "night"

[ui]
accent = "#112233"
"##,
            )
            .unwrap();
        themes.save_ui(custom.clone()).unwrap();

        custom.metadata.name = "Updated".into();
        custom.ui.accent = Some("#445566".into());
        themes.replace_ui(custom).unwrap();
        assert_eq!(
            themes.get_ui("custom.ui.lifecycle").unwrap().metadata.name,
            "Updated"
        );

        let removed = themes.remove_ui("custom.ui.lifecycle").unwrap();
        assert_eq!(removed.metadata.name, "Updated");
        assert!(themes.get_ui("custom.ui.lifecycle").is_none());
    }

    #[test]
    fn duplicate_theme_ids_are_rejected() {
        let mut themes = ThemeService::new();
        let theme = themes.import_ui_toml(DARCULA_TOML).unwrap();
        themes.save_ui(theme.clone()).unwrap();

        assert!(matches!(
            themes.save_ui(theme),
            Err(ThemeError::DuplicateId(id)) if id.as_ref() == "builtin.ui.darcula"
        ));
    }

    #[test]
    fn theme_lists_are_sorted_and_filter_by_period() {
        let mut themes = ThemeService::new();
        for source in [
            r##"
schema-version = 1
id = "custom.ui.zulu"
name = "Zulu"
kind = "ui"
period = "day"
[ui]
color-scheme = "light"
"##,
            r##"
schema-version = 1
id = "custom.ui.alpha"
name = "Alpha"
kind = "ui"
period = "night"
[ui]
color-scheme = "dark"
"##,
        ] {
            let theme = themes.import_ui_toml(source).unwrap();
            themes.save_ui(theme).unwrap();
        }

        let names: Vec<_> = themes
            .list_ui()
            .into_iter()
            .map(|theme| theme.metadata.name.as_str())
            .collect();
        assert_eq!(names, ["Alpha", "Zulu"]);
        assert_eq!(
            themes.list_ui_by_period(ThemePeriod::Day)[0]
                .metadata
                .id
                .as_ref(),
            "custom.ui.zulu"
        );
    }

    #[test]
    fn missing_theme_ids_fall_back_to_registered_defaults() {
        let mut themes = ThemeService::new();
        let ui = themes.import_ui_toml(DARCULA_TOML).unwrap();
        themes.register_builtin_ui(ui).unwrap();
        let terminal = themes.import_terminal_toml(TERMINAL_DARCULA_TOML).unwrap();
        themes.register_builtin_terminal(terminal).unwrap();

        assert_eq!(
            themes
                .resolve_ui_or_default("missing.ui")
                .unwrap()
                .metadata
                .id
                .as_ref(),
            "builtin.ui.darcula"
        );
        assert_eq!(
            themes
                .resolve_terminal_or_default("missing.terminal")
                .unwrap()
                .metadata
                .id
                .as_ref(),
            "builtin.terminal.darcula"
        );
    }

    #[test]
    fn deleting_a_theme_migrates_every_config_reference() {
        let affected_host = Uuid::new_v4();
        let unaffected_host = Uuid::new_v4();
        let deleted = ThemeId::new("custom.terminal.deleted");
        let mut config = ThemeSelectionConfig {
            ui_theme: ThemeId::new("custom.ui.deleted"),
            default_terminal_theme: deleted.clone(),
            host_terminal_themes: HashMap::from([
                (affected_host, deleted.clone()),
                (unaffected_host, ThemeId::new("custom.terminal.other")),
            ]),
        };

        let impact = config.migrate_deleted(
            &deleted,
            ThemeId::new("builtin.ui.darcula"),
            ThemeId::new("builtin.terminal.darcula"),
        );
        assert!(impact.terminal_default);
        assert_eq!(impact.host_ids, [affected_host]);
        assert_eq!(
            config.default_terminal_theme.as_ref(),
            "builtin.terminal.darcula"
        );
        assert!(!config.host_terminal_themes.contains_key(&affected_host));
        assert!(config.host_terminal_themes.contains_key(&unaffected_host));

        config.migrate_deleted(
            &ThemeId::new("custom.ui.deleted"),
            ThemeId::new("builtin.ui.darcula"),
            ThemeId::new("builtin.terminal.darcula"),
        );
        assert_eq!(config.ui_theme.as_ref(), "builtin.ui.darcula");
    }

    #[test]
    fn repository_delete_is_idempotent_and_removes_the_saved_file() {
        let directory = TestDirectory::new();
        let repository = FileThemeRepository::from_directory(&directory.0).unwrap();
        let theme = ThemeService::new().import_ui_toml(DARCULA_TOML).unwrap();
        let path = repository.save_ui(&theme).unwrap();
        assert!(path.exists());

        repository.delete(&theme.metadata.id).unwrap();
        assert!(!path.exists());
        repository.delete(&theme.metadata.id).unwrap();
        assert!(repository.discover().unwrap().is_empty());
    }

    #[test]
    fn malformed_and_oversized_repository_files_are_skipped() {
        let directory = TestDirectory::new();
        let repository = FileThemeRepository::from_directory(&directory.0).unwrap();
        std::fs::write(directory.0.join("malformed.toml"), "not = [valid").unwrap();
        std::fs::write(directory.0.join("oversized.toml"), vec![b'x'; 1_048_577]).unwrap();

        assert!(repository.discover().unwrap().is_empty());
    }

    #[test]
    fn invalid_metrics_are_rejected_and_rgba_colors_are_supported() {
        let themes = ThemeService::new();
        let invalid = DARCULA_TOML.replace(
            "accent = \"#589DF6\"",
            "accent = \"#589DF6\"\n\n[metrics]\nradius-small = -1",
        );
        assert!(matches!(
            themes.import_ui_toml(&invalid),
            Err(ThemeError::InvalidMetric { field, .. }) if field == "radius-small"
        ));

        let rgba = DARCULA_TOML.replace("#589DF6", "#589DF680");
        let parsed = themes.import_ui_toml(&rgba).unwrap();
        assert_eq!(parsed.ui.accent.as_deref(), Some("#589DF680"));
    }
}
