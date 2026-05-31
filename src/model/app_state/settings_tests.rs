use std::fs;
use std::path::PathBuf;

use super::AppState;
use crate::model::{AgentSource, AuthProfile, BuiltInTheme, Host, HostId, Message};
use crate::storage::SqliteStorage;
use crate::theme::{ThemeExchangeFormat, built_in_theme_document};
use uuid::Uuid;

fn temp_path(name: &str, extension: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "smagicalssh-{name}-{}.{}",
        Uuid::new_v4(),
        extension
    ))
}

fn host(name: &str) -> Host {
    Host {
        id: HostId(Uuid::new_v4()),
        name: name.to_owned(),
        group_id: None,
        icon_key: "server".to_owned(),
        tags: Vec::new(),
        address: format!("{name}.example.com"),
        port: 22,
        auth: AuthProfile::Agent {
            username: "deploy".to_owned(),
            source: AgentSource::Auto,
            key_hint: None,
        },
        proxy: None,
        jumps: Vec::new(),
        theme_override: None,
        background_override: None,
    }
}

#[test]
fn settings_storage_backup_writes_loadable_sqlite_copy() {
    let sqlite_path = temp_path("settings-backup-source", "sqlite");
    let backup_path = temp_path("settings-backup-target", "sqlite");
    let backend = SqliteStorage::new(&sqlite_path);
    let mut state = AppState::default().with_storage_backend(backend);
    state.storage.upsert_host(host("production"));

    let outcome = state.apply(Message::BackupStorage {
        target_path: backup_path.to_string_lossy().into_owned(),
    });

    assert!(outcome.error.is_none());
    let loaded = SqliteStorage::new(&backup_path)
        .load()
        .expect("backup should load");
    assert_eq!(loaded.host_count(), 1);
    assert_eq!(loaded.hosts[0].name, "production");
    let _ = fs::remove_file(sqlite_path);
    let _ = fs::remove_file(backup_path);
}

#[test]
fn settings_snapshot_import_replaces_current_state() {
    let source_path = temp_path("settings-import-source", "sqlite");
    let target_path = temp_path("settings-import-target", "sqlite");
    let snapshot_path = temp_path("settings-import-source", "toml");
    let source_backend = SqliteStorage::new(&source_path);
    let target_backend = SqliteStorage::new(&target_path);
    let mut source_state = AppState::default().with_storage_backend(source_backend);
    let mut target_state = AppState::default().with_storage_backend(target_backend);
    source_state.storage.upsert_host(host("source"));
    target_state.storage.upsert_host(host("target"));

    let export_outcome = source_state.apply(Message::ExportStorageSnapshot {
        target_path: snapshot_path.to_string_lossy().into_owned(),
    });
    assert!(export_outcome.error.is_none());

    let outcome = target_state.apply(Message::ImportStorageSnapshot {
        source_path: snapshot_path.to_string_lossy().into_owned(),
    });

    assert!(outcome.changed());
    assert_eq!(target_state.storage.host_count(), 1);
    assert_eq!(target_state.storage.hosts[0].name, "source");
    let _ = fs::remove_file(source_path);
    let _ = fs::remove_file(target_path);
    let _ = fs::remove_file(snapshot_path);
}

#[test]
fn settings_storage_operations_report_missing_backend() {
    let mut state = AppState::default();

    let outcome = state.apply(Message::BackupStorage {
        target_path: temp_path("missing-backend", "sqlite")
            .to_string_lossy()
            .into_owned(),
    });

    assert!(outcome.error.is_some());
    assert!(
        state
            .ui
            .last_error
            .as_deref()
            .is_some_and(|error| error.contains("SQLite"))
    );
}

#[test]
fn settings_theme_export_writes_current_builtin_theme_document() {
    let export_path = temp_path("theme-export", "toml");
    let mut state = AppState::default();
    state.apply(Message::SetBuiltInTheme {
        theme: BuiltInTheme::Dracula,
    });

    let outcome = state.apply(Message::ExportCurrentTheme {
        target_path: export_path.to_string_lossy().into_owned(),
        format: ThemeExchangeFormat::NativeToml,
    });

    assert!(outcome.error.is_none());
    let content = fs::read_to_string(&export_path).expect("exported theme should read");
    assert!(content.contains("id = \"dracula\""));
    assert!(content.contains("[terminal.ansi]"));
    let _ = fs::remove_file(export_path);
}

#[test]
fn settings_theme_export_supports_common_external_formats() {
    let export_path = temp_path("theme-export-vscode", "json");
    let mut state = AppState::default();
    state.apply(Message::SetBuiltInTheme {
        theme: BuiltInTheme::Dracula,
    });

    let outcome = state.apply(Message::ExportCurrentTheme {
        target_path: export_path.to_string_lossy().into_owned(),
        format: ThemeExchangeFormat::VsCodeJson,
    });

    assert!(outcome.error.is_none());
    let content = fs::read_to_string(&export_path).expect("exported theme should read");
    assert!(content.contains("terminal.ansiRed"));
    let _ = fs::remove_file(export_path);
}

#[test]
fn settings_theme_export_reports_unsupported_iterm2() {
    let export_path = temp_path("theme-export-iterm", "itermcolors");
    let mut state = AppState::default();

    let outcome = state.apply(Message::ExportCurrentTheme {
        target_path: export_path.to_string_lossy().into_owned(),
        format: ThemeExchangeFormat::ItermColors,
    });

    assert!(outcome.error.is_some());
    assert!(!export_path.exists());
}

#[test]
fn settings_theme_export_current_profile_uses_stored_custom_theme() {
    let export_path = temp_path("theme-export-custom", "toml");
    let mut document = built_in_theme_document(BuiltInTheme::NordDark);
    document.name = "Stored Nord".to_owned();
    document.font.terminal.family = "Maple Mono".to_owned();
    document.font.terminal.size = 17;
    let mut state = AppState::default();
    state
        .storage
        .upsert_theme(crate::storage::ThemeProfileRecord {
            name: "Stored Nord".to_owned(),
            profile_toml: document.to_toml().expect("theme should encode"),
            builtin: false,
        });
    state.apply(Message::ApplyThemeProfile {
        name: "Stored Nord".to_owned(),
    });

    let outcome = state.apply(Message::ExportCurrentTheme {
        target_path: export_path.to_string_lossy().into_owned(),
        format: ThemeExchangeFormat::NativeToml,
    });

    assert!(outcome.error.is_none());
    let content = fs::read_to_string(&export_path).expect("exported theme should read");
    assert!(content.contains("name = \"Stored Nord\""));
    assert!(content.contains("family = \"Maple Mono\""));
    let _ = fs::remove_file(export_path);
}

#[test]
fn copy_current_built_in_theme_creates_custom_profile_and_applies_it() {
    let mut state = AppState::default();
    state.apply(Message::SetBuiltInTheme {
        theme: BuiltInTheme::Dracula,
    });

    let outcome = state.apply(Message::CopyCurrentBuiltInTheme);

    assert!(outcome.changed());
    assert_eq!(state.storage.theme_count(), 1);
    assert_eq!(state.config.theme.name, "Dracula 自定义");
    assert_eq!(state.storage.app_config.theme, state.config.theme);
    let stored = state
        .storage
        .theme_by_name("Dracula 自定义")
        .expect("copied theme should be stored");
    assert!(!stored.builtin);
    assert!(stored.profile_toml.contains("id = \"dracula-custom\""));

    let second = state.apply(Message::CopyCurrentBuiltInTheme);

    assert!(second.changed());
    assert_eq!(state.storage.theme_count(), 2);
    let stored_second = state
        .storage
        .theme_by_name("Dracula 自定义 2")
        .expect("second copied theme should be stored");
    assert!(
        stored_second
            .profile_toml
            .contains("id = \"dracula-custom-2\"")
    );
}

#[test]
fn settings_theme_import_updates_global_theme_profile() {
    let import_path = temp_path("theme-import", "toml");
    let mut document = built_in_theme_document(BuiltInTheme::NordDark);
    document.name = "Imported Nord".to_owned();
    document.font.terminal.family = "Maple Mono".to_owned();
    document.font.terminal.size = 16;
    fs::write(
        &import_path,
        document.to_toml().expect("theme should encode"),
    )
    .expect("theme fixture should write");
    let mut state = AppState::default();

    let outcome = state.apply(Message::ImportTheme {
        source_path: import_path.to_string_lossy().into_owned(),
    });

    assert!(outcome.changed());
    assert_eq!(state.config.theme.name, "Imported Nord");
    assert_eq!(state.config.theme.font_family, "Maple Mono");
    assert_eq!(state.config.theme.font_size, 16.0);
    assert_eq!(state.storage.app_config.theme, state.config.theme);
    assert_eq!(state.storage.theme_count(), 1);
    assert_eq!(
        state
            .storage
            .theme_by_name("Imported Nord")
            .map(|theme| theme.builtin),
        Some(false)
    );
    assert_eq!(state.ui.visual_settings.theme_name, "Imported Nord");
    let _ = fs::remove_file(import_path);
}

#[test]
fn settings_theme_import_accepts_partial_native_toml() {
    let import_path = temp_path("theme-import-partial", "toml");
    fs::write(
        &import_path,
        r##"
schema_version = 1
id = "partial-nord"
name = "Partial Nord"
extends = "nord-dark"

[font.terminal]
family = "Maple Mono"
size = 16

[overrides]
"terminal.background" = "#010203"
"button.primary.background_hover" = "#223344"
"##,
    )
    .expect("partial theme fixture should write");
    let mut state = AppState::default();

    let outcome = state.apply(Message::ImportTheme {
        source_path: import_path.to_string_lossy().into_owned(),
    });

    assert!(outcome.changed());
    assert_eq!(state.config.theme.name, "Partial Nord");
    assert_eq!(state.config.theme.font_family, "Maple Mono");
    assert_eq!(state.config.theme.font_size, 16.0);
    let stored = state
        .storage
        .theme_by_name("Partial Nord")
        .expect("partial theme should be stored");
    assert!(stored.profile_toml.contains("background = \"#010203\""));
    let _ = fs::remove_file(import_path);
}

#[test]
fn settings_theme_import_accepts_vscode_json_and_stores_native_toml() {
    let import_path = temp_path("theme-import-vscode", "json");
    let mut document = built_in_theme_document(BuiltInTheme::Dracula);
    document.name = "VS Code Dracula".to_owned();
    let exported = document
        .export(ThemeExchangeFormat::VsCodeJson)
        .expect("VS Code theme should export");
    fs::write(&import_path, exported.content).expect("theme fixture should write");
    let mut state = AppState::default();

    let outcome = state.apply(Message::ImportTheme {
        source_path: import_path.to_string_lossy().into_owned(),
    });

    assert!(outcome.changed());
    assert_eq!(state.config.theme.name, "VS Code Dracula");
    let stored = state
        .storage
        .theme_by_name("VS Code Dracula")
        .expect("imported theme should be stored");
    assert!(stored.profile_toml.contains("schema_version = 1"));
    assert!(stored.profile_toml.contains("[terminal.ansi]"));
    let _ = fs::remove_file(import_path);
}

#[test]
fn settings_theme_import_accepts_windows_terminal_json() {
    let import_path = temp_path("theme-import-windows-terminal", "json");
    let mut document = built_in_theme_document(BuiltInTheme::NordDark);
    document.name = "Windows Nord".to_owned();
    let exported = document
        .export(ThemeExchangeFormat::WindowsTerminalJson)
        .expect("Windows Terminal theme should export");
    fs::write(&import_path, exported.content).expect("theme fixture should write");
    let mut state = AppState::default();

    let outcome = state.apply(Message::ImportTheme {
        source_path: import_path.to_string_lossy().into_owned(),
    });

    assert!(outcome.changed());
    assert_eq!(state.config.theme.name, "Windows Nord");
    assert_eq!(state.storage.theme_count(), 1);
    let _ = fs::remove_file(import_path);
}

#[test]
fn settings_theme_import_accepts_alacritty_toml() {
    let import_path = temp_path("theme-import-alacritty", "toml");
    let mut document = built_in_theme_document(BuiltInTheme::OceanDark);
    document.name = "Alacritty Ocean".to_owned();
    let exported = document
        .export(ThemeExchangeFormat::AlacrittyToml)
        .expect("Alacritty theme should export");
    fs::write(&import_path, exported.content).expect("theme fixture should write");
    let mut state = AppState::default();

    let outcome = state.apply(Message::ImportTheme {
        source_path: import_path.to_string_lossy().into_owned(),
    });

    assert!(outcome.changed());
    assert_eq!(
        state.config.theme.name,
        import_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .expect("fixture should have a file stem")
    );
    assert_eq!(state.storage.theme_count(), 1);
    let _ = fs::remove_file(import_path);
}

#[test]
fn settings_theme_import_reports_unsupported_iterm2() {
    let import_path = temp_path("theme-import-iterm", "itermcolors");
    fs::write(&import_path, "<plist></plist>").expect("theme fixture should write");
    let mut state = AppState::default();

    let outcome = state.apply(Message::ImportTheme {
        source_path: import_path.to_string_lossy().into_owned(),
    });

    assert!(outcome.error.is_some());
    assert_eq!(state.storage.theme_count(), 0);
    let _ = fs::remove_file(import_path);
}

#[test]
fn settings_theme_profile_can_be_applied_from_storage() {
    let mut document = built_in_theme_document(BuiltInTheme::NordDark);
    document.name = "Stored Nord".to_owned();
    document.font.terminal.family = "Maple Mono".to_owned();
    document.font.terminal.size = 17;
    let mut state = AppState::default();
    state
        .storage
        .upsert_theme(crate::storage::ThemeProfileRecord {
            name: "Stored Nord".to_owned(),
            profile_toml: document.to_toml().expect("theme should encode"),
            builtin: false,
        });

    let outcome = state.apply(Message::ApplyThemeProfile {
        name: "Stored Nord".to_owned(),
    });

    assert!(outcome.changed());
    assert_eq!(state.config.theme.name, "Stored Nord");
    assert_eq!(state.config.theme.font_family, "Maple Mono");
    assert_eq!(state.config.theme.font_size, 17.0);
    assert_eq!(state.storage.app_config.theme, state.config.theme);
    assert_eq!(state.ui.visual_settings.theme_name, "Stored Nord");
}

#[test]
fn settings_theme_profile_reapply_current_profile_is_noop() {
    let mut document = built_in_theme_document(BuiltInTheme::NordDark);
    document.name = "Stored Nord".to_owned();
    document.font.terminal.family = "Maple Mono".to_owned();
    document.font.terminal.size = 17;
    let mut state = AppState::default();
    state
        .storage
        .upsert_theme(crate::storage::ThemeProfileRecord {
            name: "Stored Nord".to_owned(),
            profile_toml: document.to_toml().expect("theme should encode"),
            builtin: false,
        });
    state.apply(Message::ApplyThemeProfile {
        name: "Stored Nord".to_owned(),
    });

    let outcome = state.apply(Message::ApplyThemeProfile {
        name: "Stored Nord".to_owned(),
    });

    assert!(!outcome.changed());
    assert_eq!(state.config.theme.name, "Stored Nord");
    assert_eq!(state.ui.visual_settings.theme_name, "Stored Nord");
}

#[test]
fn settings_theme_profile_remove_updates_storage_without_resetting_current_theme() {
    let mut document = built_in_theme_document(BuiltInTheme::NordDark);
    document.name = "Disposable Nord".to_owned();
    let mut state = AppState::default();
    state
        .storage
        .upsert_theme(crate::storage::ThemeProfileRecord {
            name: "Disposable Nord".to_owned(),
            profile_toml: document.to_toml().expect("theme should encode"),
            builtin: false,
        });
    state.apply(Message::ApplyThemeProfile {
        name: "Disposable Nord".to_owned(),
    });

    let outcome = state.apply(Message::RemoveThemeProfile {
        name: "Disposable Nord".to_owned(),
    });

    assert!(outcome.changed());
    assert_eq!(state.storage.theme_count(), 0);
    assert_eq!(state.config.theme.name, "Disposable Nord");
    assert_eq!(state.storage.app_config.theme, state.config.theme);
}

#[test]
fn settings_theme_import_roundtrips_through_sqlite_storage() {
    let sqlite_path = temp_path("settings-theme-import-roundtrip", "sqlite");
    let import_path = temp_path("settings-theme-import-roundtrip", "toml");
    let backend = SqliteStorage::new(&sqlite_path);
    let mut document = built_in_theme_document(BuiltInTheme::OceanDark);
    document.name = "Persistent Ocean".to_owned();
    document.font.terminal.family = "Maple Mono".to_owned();
    document.font.terminal.size = 15;
    fs::write(
        &import_path,
        document.to_toml().expect("theme should encode"),
    )
    .expect("theme fixture should write");
    let mut state = AppState::default().with_storage_backend(backend.clone());

    let outcome = state.apply(Message::ImportTheme {
        source_path: import_path.to_string_lossy().into_owned(),
    });
    assert!(outcome.changed());
    state
        .persist_storage()
        .expect("theme import should persist to SQLite");

    let loaded = backend.load().expect("persisted storage should load");
    assert_eq!(loaded.app_config.theme.name, "Persistent Ocean");
    assert_eq!(loaded.app_config.theme.font_family, "Maple Mono");
    assert_eq!(loaded.app_config.theme.font_size, 15.0);
    let stored = loaded
        .theme_by_name("Persistent Ocean")
        .expect("imported theme profile should roundtrip");
    assert!(!stored.builtin);
    assert!(stored.profile_toml.contains("schema_version = 1"));
    assert!(stored.profile_toml.contains("[terminal.ansi]"));
    let _ = fs::remove_file(sqlite_path);
    let _ = fs::remove_file(import_path);
}

#[test]
fn settings_theme_profile_apply_roundtrips_current_config_through_sqlite_storage() {
    let sqlite_path = temp_path("settings-theme-apply-roundtrip", "sqlite");
    let backend = SqliteStorage::new(&sqlite_path);
    let mut document = built_in_theme_document(BuiltInTheme::NordDark);
    document.name = "Persistent Nord".to_owned();
    document.font.terminal.family = "JetBrains Mono".to_owned();
    document.font.terminal.size = 18;
    let mut state = AppState::default().with_storage_backend(backend.clone());
    state
        .storage
        .upsert_theme(crate::storage::ThemeProfileRecord {
            name: "Persistent Nord".to_owned(),
            profile_toml: document.to_toml().expect("theme should encode"),
            builtin: false,
        });

    let outcome = state.apply(Message::ApplyThemeProfile {
        name: "Persistent Nord".to_owned(),
    });
    assert!(outcome.changed());
    state
        .persist_storage()
        .expect("theme profile apply should persist to SQLite");

    let loaded = backend.load().expect("persisted storage should load");
    assert_eq!(loaded.app_config.theme.name, "Persistent Nord");
    assert_eq!(loaded.app_config.theme.font_family, "JetBrains Mono");
    assert_eq!(loaded.app_config.theme.font_size, 18.0);
    assert!(loaded.theme_by_name("Persistent Nord").is_some());
    let _ = fs::remove_file(sqlite_path);
}

#[test]
fn settings_theme_profile_missing_name_reports_error() {
    let mut state = AppState::default();

    let outcome = state.apply(Message::ApplyThemeProfile {
        name: "Missing".to_owned(),
    });

    assert!(outcome.error.is_some());
    assert_eq!(
        state.config.theme,
        crate::config::AppConfig::default().theme
    );
    assert_eq!(state.storage.theme_count(), 0);
}
