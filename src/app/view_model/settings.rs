//! 设置页展示模型。
//!
//! 设置页把语言、主题、存储和安全状态合成纯 Rust view model。这里不执行导入、导出、
//! 备份等副作用，只计算按钮是否可用、默认文件名、显示文案和计数摘要。

use crate::app::state::{AsDesktopStateView, DesktopStateView};
use crate::model::{BuiltInTheme, LanguageMode};

use super::i18n::{Locale, locale_for_state, tr};
use super::labels::theme_label;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::app) struct SettingOptionViewModel {
    pub key: &'static str,
    pub label: &'static str,
    pub selected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::app) struct ThemeFormatViewModel {
    pub key: &'static str,
    pub label: &'static str,
    pub extension: &'static str,
    pub supported: bool,
    pub selected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::app) struct SettingsViewModel {
    pub text: SettingsTextViewModel,
    pub language_options: Vec<SettingOptionViewModel>,
    pub theme_options: Vec<SettingOptionViewModel>,
    pub theme: SettingsThemeViewModel,
    pub storage: SettingsStorageViewModel,
    pub security: SettingsSecurityViewModel,
    pub file_actions: Vec<SettingsFileActionViewModel>,
    pub storage_summary: String,
    pub security_summary: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::app) struct SettingsTextViewModel {
    pub title: &'static str,
    pub language_title: &'static str,
    pub theme_title: &'static str,
    pub custom_theme_title: &'static str,
    pub storage_title: &'static str,
    pub file_actions_title: &'static str,
    pub security_title: &'static str,
    pub apply_label: &'static str,
    pub copy_label: &'static str,
    pub remove_label: &'static str,
    pub run_label: &'static str,
    pub choose_file_label: &'static str,
    pub selected_label: &'static str,
    pub unavailable_label: &'static str,
    pub no_custom_themes_label: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::app) struct SettingsThemeViewModel {
    pub current_theme_name: String,
    pub current_profile_name: String,
    pub built_in_theme_count: usize,
    pub custom_theme_count: usize,
    pub custom_theme_names: Vec<String>,
    pub custom_theme_profiles: Vec<CustomThemeProfileViewModel>,
    pub can_import: bool,
    pub can_export_current_theme: bool,
    pub import_formats: Vec<ThemeFormatViewModel>,
    pub export_formats: Vec<ThemeFormatViewModel>,
    pub default_import_extension: &'static str,
    pub default_export_file_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::app) struct CustomThemeProfileViewModel {
    pub name: String,
    pub source_label: &'static str,
    pub selected: bool,
    pub can_apply: bool,
    pub can_remove: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::app) struct SettingsFileActionViewModel {
    pub key: &'static str,
    pub label: &'static str,
    pub category_key: &'static str,
    pub category_label: &'static str,
    pub direction: &'static str,
    pub direction_label: &'static str,
    pub format_key: &'static str,
    pub format_label: &'static str,
    pub default_file_name: String,
    pub default_extension: &'static str,
    pub path_placeholder: &'static str,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::app) struct SettingsStorageViewModel {
    pub backend_label: &'static str,
    pub database_path: String,
    pub summary_items: Vec<SettingsStorageSummaryItemViewModel>,
    pub can_backup: bool,
    pub can_export_snapshot: bool,
    pub can_import_snapshot: bool,
    pub can_import_sqlite_backup: bool,
    pub actions: Vec<SettingsStorageActionViewModel>,
    pub default_backup_file_name: &'static str,
    pub default_snapshot_file_name: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::app) struct SettingsStorageSummaryItemViewModel {
    pub key: &'static str,
    pub label: &'static str,
    pub count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::app) struct SettingsStorageActionViewModel {
    pub key: &'static str,
    pub label: &'static str,
    pub enabled: bool,
    pub default_file_name: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::app) struct SettingsSecurityViewModel {
    pub encryption_key: &'static str,
    pub encryption_label: &'static str,
    pub encryption_enabled: bool,
    pub can_configure_encryption: bool,
    pub status_label: &'static str,
    pub detail_label: &'static str,
    pub kdf_label: &'static str,
    pub encryption_version_label: &'static str,
}

pub(super) fn settings(state: impl AsDesktopStateView) -> SettingsViewModel {
    let state = state.as_desktop_state_view();
    // 先分别计算存储和安全状态，文件操作按钮会依赖存储后端是否持久化。
    let locale = locale_for_state(state);
    let storage = storage_status(state, locale);
    let security = security_status(state, locale);

    SettingsViewModel {
        text: settings_text(locale),
        language_options: language_options(state.ui.workspace.language, locale),
        theme_options: theme_options(state.ui.workspace.theme, locale),
        theme: theme_status(state, locale),
        storage_summary: storage_summary(state, locale),
        security_summary: security.encryption_label,
        file_actions: file_actions(state, &storage, locale),
        storage,
        security,
    }
}

fn settings_text(locale: Locale) -> SettingsTextViewModel {
    SettingsTextViewModel {
        title: tr(locale, "page.settings"),
        language_title: tr(locale, "settings.section_language"),
        theme_title: tr(locale, "settings.section_theme"),
        custom_theme_title: tr(locale, "settings.section_custom_themes"),
        storage_title: tr(locale, "settings.section_storage"),
        file_actions_title: tr(locale, "settings.section_file_actions"),
        security_title: tr(locale, "settings.section_security"),
        apply_label: tr(locale, "settings.action_apply"),
        copy_label: tr(locale, "settings.action_copy"),
        remove_label: tr(locale, "settings.action_remove"),
        run_label: tr(locale, "settings.action_run"),
        choose_file_label: tr(locale, "settings.action_choose_file"),
        selected_label: tr(locale, "settings.status_selected"),
        unavailable_label: tr(locale, "settings.status_unavailable"),
        no_custom_themes_label: tr(locale, "settings.empty_custom_themes"),
    }
}

fn language_options(selected: LanguageMode, locale: Locale) -> Vec<SettingOptionViewModel> {
    // key 是回调协议值，label 是当前语言展示文案。
    [
        (
            "FollowSystem",
            tr(locale, "settings.language_follow_system"),
            LanguageMode::FollowSystem,
        ),
        (
            "Chinese",
            tr(locale, "settings.language_chinese"),
            LanguageMode::Chinese,
        ),
        (
            "English",
            tr(locale, "settings.language_english"),
            LanguageMode::English,
        ),
    ]
    .into_iter()
    .map(|(key, label, language)| SettingOptionViewModel {
        key,
        label,
        selected: selected == language,
    })
    .collect()
}

fn theme_options(selected: BuiltInTheme, locale: Locale) -> Vec<SettingOptionViewModel> {
    // 内置主题来自核心枚举，新增主题只需要扩展 BuiltInTheme::ALL 和 label。
    BuiltInTheme::ALL
        .into_iter()
        .map(|theme| SettingOptionViewModel {
            key: theme.key(),
            label: theme_label(theme, locale),
            selected: selected == theme,
        })
        .collect()
}

fn theme_status(state: DesktopStateView<'_>, locale: Locale) -> SettingsThemeViewModel {
    // 主题导入导出先暴露能力和格式，实际解析/写文件由 theme 模块处理。
    SettingsThemeViewModel {
        current_theme_name: theme_label(state.ui.workspace.theme, locale).to_owned(),
        current_profile_name: state.config.theme.name.clone(),
        built_in_theme_count: BuiltInTheme::ALL.len(),
        custom_theme_count: state.storage.theme_count(),
        custom_theme_names: state
            .storage
            .themes
            .iter()
            .map(|theme| theme.name.clone())
            .collect(),
        custom_theme_profiles: custom_theme_profiles(state, locale),
        can_import: true,
        can_export_current_theme: true,
        import_formats: theme_import_formats(locale),
        export_formats: theme_export_formats(locale),
        default_import_extension: "toml",
        default_export_file_name: current_theme_export_file_name(state),
    }
}

fn current_theme_export_file_name(state: DesktopStateView<'_>) -> String {
    let stem = state
        .storage
        .theme_by_name(&state.config.theme.name)
        .map(|theme| file_stem_from_theme_name(&theme.name))
        .unwrap_or_else(|| state.ui.workspace.theme.key().to_owned());
    format!("{stem}.smagical-theme.toml")
}

fn file_stem_from_theme_name(name: &str) -> String {
    let mut stem = String::new();
    let mut last_was_separator = false;
    for ch in name.trim().chars() {
        let separator = ch.is_whitespace()
            || ch.is_control()
            || matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*');
        if separator {
            if !stem.is_empty() && !last_was_separator {
                stem.push('-');
            }
            last_was_separator = true;
        } else if ch.is_alphanumeric() || matches!(ch, '-' | '_' | '.') {
            stem.push(ch);
            last_was_separator = false;
        } else if !stem.is_empty() && !last_was_separator {
            stem.push('-');
            last_was_separator = true;
        }
    }

    let stem = stem.trim_matches(&['-', '.'][..]);
    if stem.is_empty() {
        "custom-theme".to_owned()
    } else {
        stem.to_owned()
    }
}

fn custom_theme_profiles(
    state: DesktopStateView<'_>,
    locale: Locale,
) -> Vec<CustomThemeProfileViewModel> {
    // 已选中的 profile 不能再次应用；内置 profile 不允许从用户存储中删除。
    state
        .storage
        .themes
        .iter()
        .map(|theme| {
            let selected = state.config.theme.name == theme.name;
            CustomThemeProfileViewModel {
                name: theme.name.clone(),
                source_label: if theme.builtin {
                    tr(locale, "settings.theme_source_builtin")
                } else {
                    tr(locale, "settings.theme_source_imported")
                },
                selected,
                can_apply: !selected,
                can_remove: !theme.builtin,
            }
        })
        .collect()
}

fn theme_import_formats(locale: Locale) -> Vec<ThemeFormatViewModel> {
    // ItermColors 先展示为未来支持项，按钮禁用，避免 UI 结构后续再变。
    [
        format_view_model(
            "NativeToml",
            tr(locale, "settings.theme_format_native"),
            "toml",
            true,
            true,
        ),
        format_view_model(
            "VsCodeJson",
            tr(locale, "settings.theme_format_vscode"),
            "json",
            true,
            false,
        ),
        format_view_model(
            "WindowsTerminalJson",
            tr(locale, "settings.theme_format_windows_terminal"),
            "json",
            true,
            false,
        ),
        format_view_model(
            "AlacrittyToml",
            tr(locale, "settings.theme_format_alacritty"),
            "toml",
            true,
            false,
        ),
        format_view_model(
            "ItermColors",
            tr(locale, "settings.theme_format_iterm2"),
            "itermcolors",
            false,
            false,
        ),
    ]
    .into()
}

fn theme_export_formats(locale: Locale) -> Vec<ThemeFormatViewModel> {
    // 导出格式和导入格式保持同一组 key，未实现的格式同样标记 unsupported。
    [
        format_view_model(
            "NativeToml",
            tr(locale, "settings.theme_format_native"),
            "toml",
            true,
            true,
        ),
        format_view_model(
            "VsCodeJson",
            tr(locale, "settings.theme_format_vscode"),
            "json",
            true,
            false,
        ),
        format_view_model(
            "WindowsTerminalJson",
            tr(locale, "settings.theme_format_windows_terminal"),
            "json",
            true,
            false,
        ),
        format_view_model(
            "AlacrittyToml",
            tr(locale, "settings.theme_format_alacritty"),
            "toml",
            true,
            false,
        ),
        format_view_model(
            "ItermColors",
            tr(locale, "settings.theme_format_iterm2"),
            "itermcolors",
            false,
            false,
        ),
    ]
    .into()
}

fn format_view_model(
    key: &'static str,
    label: &'static str,
    extension: &'static str,
    supported: bool,
    selected: bool,
) -> ThemeFormatViewModel {
    ThemeFormatViewModel {
        key,
        label,
        extension,
        supported,
        selected,
    }
}

fn file_actions(
    state: DesktopStateView<'_>,
    storage: &SettingsStorageViewModel,
    locale: Locale,
) -> Vec<SettingsFileActionViewModel> {
    // 文件操作统一投影成列表，UI 不需要为主题和存储分别写不同的按钮逻辑。
    [
        file_action(
            "ImportTheme",
            tr(locale, "settings.file_action_import_theme"),
            "Theme",
            "Import",
            "NativeToml",
            "",
            "toml",
            true,
            locale,
        ),
        file_action(
            "ExportCurrentTheme",
            tr(locale, "settings.file_action_export_current_theme"),
            "Theme",
            "Export",
            "NativeToml",
            current_theme_export_file_name(state),
            "toml",
            true,
            locale,
        ),
        file_action(
            "BackupSqlite",
            tr(locale, "settings.storage_action_backup"),
            "Storage",
            "Export",
            "Sqlite",
            storage.default_backup_file_name,
            "sqlite",
            storage.can_backup,
            locale,
        ),
        file_action(
            "ExportSnapshot",
            tr(locale, "settings.storage_action_export_snapshot"),
            "Storage",
            "Export",
            "SnapshotToml",
            storage.default_snapshot_file_name,
            "toml",
            storage.can_export_snapshot,
            locale,
        ),
        file_action(
            "ImportSnapshot",
            tr(locale, "settings.storage_action_import_snapshot"),
            "Storage",
            "Import",
            "SnapshotToml",
            "",
            "toml",
            storage.can_import_snapshot,
            locale,
        ),
        file_action(
            "ImportSqliteBackup",
            tr(locale, "settings.storage_action_import_sqlite"),
            "Storage",
            "Import",
            "Sqlite",
            "",
            "sqlite",
            storage.can_import_sqlite_backup,
            locale,
        ),
    ]
    .into()
}

fn file_action(
    key: &'static str,
    label: &'static str,
    category_key: &'static str,
    direction: &'static str,
    format_key: &'static str,
    default_file_name: impl Into<String>,
    default_extension: &'static str,
    enabled: bool,
    locale: Locale,
) -> SettingsFileActionViewModel {
    // category/direction/format 同时保留 key 和 label：key 用于回调，label 用于显示。
    SettingsFileActionViewModel {
        key,
        label,
        category_key,
        category_label: file_category_label(category_key, locale),
        direction,
        direction_label: file_direction_label(direction, locale),
        format_key,
        format_label: file_format_label(format_key, locale),
        default_file_name: default_file_name.into(),
        default_extension,
        path_placeholder: file_path_placeholder(direction, locale),
        enabled,
    }
}

fn file_category_label(key: &'static str, locale: Locale) -> &'static str {
    match key {
        "Theme" => tr(locale, "settings.file_category_theme"),
        "Storage" => tr(locale, "settings.file_category_storage"),
        _ => key,
    }
}

fn file_direction_label(direction: &'static str, locale: Locale) -> &'static str {
    match direction {
        "Import" => tr(locale, "settings.file_direction_import"),
        "Export" => tr(locale, "settings.file_direction_export"),
        _ => direction,
    }
}

fn file_format_label(format_key: &'static str, locale: Locale) -> &'static str {
    match format_key {
        "NativeToml" => tr(locale, "settings.file_format_theme"),
        "Sqlite" => tr(locale, "settings.file_format_database"),
        "SnapshotToml" => tr(locale, "settings.file_format_snapshot"),
        _ => format_key,
    }
}

fn file_path_placeholder(direction: &'static str, locale: Locale) -> &'static str {
    match direction {
        "Import" => tr(locale, "settings.file_path_placeholder_import"),
        "Export" => tr(locale, "settings.file_path_placeholder_export"),
        _ => tr(locale, "settings.file_path_placeholder"),
    }
}

fn storage_summary(state: DesktopStateView<'_>, locale: Locale) -> String {
    // 顶部摘要只取前几项，详细列表由 settings_storage_summary 单独展示。
    storage_summary_items(state, locale)
        .into_iter()
        .take(4)
        .map(|item| count_label(item.count, item.label))
        .collect::<Vec<_>>()
        .join(" · ")
}

fn storage_status(state: DesktopStateView<'_>, locale: Locale) -> SettingsStorageViewModel {
    // 没有 SQLite 后端时仍能运行，但备份/导入导出按钮应禁用。
    let database_path = state
        .storage_backend
        .as_ref()
        .map(|backend| backend.path().display().to_string())
        .unwrap_or_else(|| tr(locale, "settings.storage_memory_only").to_owned());
    let persistent = state.storage_backend.is_some();

    SettingsStorageViewModel {
        backend_label: if persistent {
            tr(locale, "settings.storage_backend_sqlite")
        } else {
            tr(locale, "settings.storage_backend_memory")
        },
        database_path,
        summary_items: storage_summary_items(state, locale),
        can_backup: persistent,
        can_export_snapshot: persistent,
        can_import_snapshot: persistent,
        can_import_sqlite_backup: persistent,
        actions: storage_actions(persistent, locale),
        default_backup_file_name: "smagicalssh-backup.sqlite",
        default_snapshot_file_name: "smagicalssh-snapshot.toml",
    }
}

fn storage_summary_items(
    state: DesktopStateView<'_>,
    locale: Locale,
) -> Vec<SettingsStorageSummaryItemViewModel> {
    // 这些计数来自 StorageManager，后续新增模块时只需追加一个 summary item。
    [
        storage_summary_item(
            "Hosts",
            tr(locale, "settings.storage_hosts"),
            state.storage.host_count(),
        ),
        storage_summary_item(
            "Groups",
            tr(locale, "settings.storage_groups"),
            state.storage.group_count(),
        ),
        storage_summary_item(
            "Credentials",
            tr(locale, "settings.storage_credentials"),
            state.storage.credential_count(),
        ),
        storage_summary_item(
            "History",
            tr(locale, "settings.storage_history"),
            state.storage.command_history_count(),
        ),
        storage_summary_item(
            "Snippets",
            tr(locale, "settings.storage_snippets"),
            state.storage.snippet_count(),
        ),
        storage_summary_item(
            "Bookmarks",
            tr(locale, "settings.storage_bookmarks"),
            state.storage.sftp_bookmark_count(),
        ),
        storage_summary_item(
            "Tunnels",
            tr(locale, "settings.storage_tunnels"),
            state.storage.tunnel_rule_count(),
        ),
        storage_summary_item(
            "Themes",
            tr(locale, "settings.storage_themes"),
            state.storage.theme_count(),
        ),
        storage_summary_item(
            "WorkspaceTabs",
            tr(locale, "settings.storage_workspace_tabs"),
            state.storage.workspace_tab_count(),
        ),
    ]
    .into()
}

fn storage_summary_item(
    key: &'static str,
    label: &'static str,
    count: usize,
) -> SettingsStorageSummaryItemViewModel {
    SettingsStorageSummaryItemViewModel { key, label, count }
}

fn storage_actions(persistent: bool, locale: Locale) -> Vec<SettingsStorageActionViewModel> {
    // 存储动作保留独立模型，便于以后设置页改成卡片或命令菜单。
    [
        storage_action(
            "BackupSqlite",
            tr(locale, "settings.storage_action_backup"),
            persistent,
            "smagicalssh-backup.sqlite",
        ),
        storage_action(
            "ExportSnapshot",
            tr(locale, "settings.storage_action_export_snapshot"),
            persistent,
            "smagicalssh-snapshot.toml",
        ),
        storage_action(
            "ImportSnapshot",
            tr(locale, "settings.storage_action_import_snapshot"),
            persistent,
            "",
        ),
        storage_action(
            "ImportSqliteBackup",
            tr(locale, "settings.storage_action_import_sqlite"),
            persistent,
            "",
        ),
    ]
    .into()
}

fn storage_action(
    key: &'static str,
    label: &'static str,
    enabled: bool,
    default_file_name: &'static str,
) -> SettingsStorageActionViewModel {
    SettingsStorageActionViewModel {
        key,
        label,
        enabled,
        default_file_name,
    }
}

fn security_status(state: DesktopStateView<'_>, locale: Locale) -> SettingsSecurityViewModel {
    // 当前只实现未加密状态；后续设置密码后在这里扩展启用、KDF 和版本展示。
    match state.config.security.encryption {
        crate::config::StorageEncryptionPreference::Disabled => SettingsSecurityViewModel {
            encryption_key: "Disabled",
            encryption_label: tr(locale, "settings.security_unencrypted"),
            encryption_enabled: false,
            can_configure_encryption: false,
            status_label: tr(locale, "settings.security_status_unencrypted"),
            detail_label: tr(locale, "settings.security_detail_password_future"),
            kdf_label: tr(locale, "settings.security_kdf_unconfigured"),
            encryption_version_label: tr(locale, "settings.security_version_none"),
        },
    }
}

fn count_label(count: usize, label: &'static str) -> String {
    // 这里保持简单拼接，复杂复数规则以后可下沉到 i18n 层。
    format!("{count} {label}")
}
