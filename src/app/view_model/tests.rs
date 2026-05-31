use super::*;
use crate::model::{
    AgentSource, AppState, AuthProfile, CredentialKind, CredentialMetadata, Host, HostId,
    KeyAlgorithm, KnownHostEntry, LanguageMode, Message, QuickHostAuthField, QuickHostAuthKind,
    QuickHostDraftField, SecretRef, SessionId, TunnelRuntimeState, TunnelStatus,
};
use crate::storage::{SqliteStorage, ThemeProfileRecord};
use uuid::Uuid;

#[test]
fn app_view_model_uses_local_terminal_when_no_tab_is_open() {
    let mut state = AppState::default();
    state.ui.workspace.language = LanguageMode::English;

    let vm = app_view_model(&state);

    assert_eq!(
        vm.terminal.title,
        crate::model::DEFAULT_LOCAL_TERMINAL_TITLE
    );
    assert_eq!(vm.terminal.status, "Ready");
    assert!(vm.terminal.can_send_input);
}

#[test]
fn auth_label_covers_password_without_secret_leakage() {
    let mut state = AppState::default();
    state.ui.workspace.language = LanguageMode::English;
    state.storage.upsert_host(Host {
        id: HostId(Uuid::new_v4()),
        name: "root".to_owned(),
        group_id: None,
        icon_key: "server".to_owned(),
        tags: Vec::new(),
        address: "example.com".to_owned(),
        port: 22,
        auth: AuthProfile::Password {
            username: "root".to_owned(),
            secret: SecretRef("password:root".to_owned()),
        },
        proxy: None,
        jumps: Vec::new(),
        theme_override: None,
        background_override: None,
    });

    let vm = app_view_model(&state);

    assert_eq!(vm.hosts[0].auth, "Password");
}

#[test]
fn app_view_model_filters_new_session_hosts_without_changing_host_list() {
    let mut state = AppState::default();
    state.ui.workspace.language = LanguageMode::English;
    state.storage.upsert_host(Host {
        id: HostId(Uuid::new_v4()),
        name: "Production".to_owned(),
        group_id: None,
        icon_key: "server".to_owned(),
        tags: vec!["prod".to_owned()],
        address: "prod.example.com".to_owned(),
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
    });
    state.storage.upsert_host(Host {
        id: HostId(Uuid::new_v4()),
        name: "Staging".to_owned(),
        group_id: None,
        icon_key: "server".to_owned(),
        tags: vec!["stage".to_owned()],
        address: "staging.example.com".to_owned(),
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
    });
    state.ui.workspace.set_new_session_search_query("prod");

    let vm = app_view_model(&state);

    assert_eq!(vm.hosts.len(), 2);
    assert_eq!(vm.new_session_hosts.len(), 1);
    assert_eq!(vm.new_session_hosts[0].name, "Production");
}

#[test]
fn app_view_model_keeps_local_terminal_visible_for_local_new_session_search() {
    let mut state = AppState::default();
    state.storage.upsert_host(Host {
        id: HostId(Uuid::new_v4()),
        name: "Production".to_owned(),
        group_id: None,
        icon_key: "server".to_owned(),
        tags: vec!["prod".to_owned()],
        address: "prod.example.com".to_owned(),
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
    });
    state.ui.workspace.set_new_session_search_query("local");

    let vm = app_view_model(&state);

    assert!(vm.new_session_local_visible);
    assert!(vm.new_session_hosts.is_empty());
}

#[test]
fn app_view_model_projects_known_hosts_for_tool_panel() {
    let mut state = AppState::default();
    state.ui.workspace.language = LanguageMode::English;
    state.storage.upsert_known_host(KnownHostEntry::untrusted(
        "example.com",
        22,
        KeyAlgorithm::Ed25519,
        "SHA256:new",
    ));

    let vm = app_view_model(&state);

    assert_eq!(vm.known_hosts.len(), 1);
    assert_eq!(vm.known_hosts[0].host, "example.com");
    assert_eq!(vm.known_hosts[0].port, 22);
    assert_eq!(vm.known_hosts[0].fingerprint, "SHA256:new");
    assert_eq!(vm.known_hosts[0].status_key, "pending");
    assert_eq!(vm.known_hosts[0].status, "pending");
}

#[test]
fn app_view_model_projects_credentials_for_security_page() {
    let mut state = AppState::default();
    state.storage.upsert_credential(CredentialMetadata {
        name: "deploy".to_owned(),
        kind: CredentialKind::PrivateKey,
        username: Some("ubuntu".to_owned()),
        secret: Some(SecretRef("key:deploy".to_owned())),
        key_algorithm: Some(KeyAlgorithm::Ed25519),
        fingerprint: Some("SHA256:key".to_owned()),
    });

    let vm = app_view_model(&state);

    assert_eq!(vm.credentials.len(), 1);
    assert_eq!(vm.credentials[0].title, "deploy");
    assert_eq!(vm.credentials[0].subtitle, "ubuntu");
    assert_eq!(vm.credentials[0].meta, "SHA256:key");
}

#[test]
fn app_view_model_localizes_tool_panel_fallback_labels() {
    let mut state = AppState::default();
    state.ui.workspace.language = LanguageMode::Chinese;
    state.storage.upsert_credential(CredentialMetadata {
        name: "deploy-key".to_owned(),
        kind: CredentialKind::PrivateKey,
        username: None,
        secret: Some(SecretRef("key:deploy".to_owned())),
        key_algorithm: None,
        fingerprint: None,
    });
    state.sessions.tunnels.push(TunnelRuntimeState {
        session_id: SessionId(Uuid::new_v4()),
        rule_name: "local-db".to_owned(),
        host_id: None,
        status: TunnelStatus::Running,
        started_at_unix_secs: None,
        last_error: None,
    });

    let vm = app_view_model(&state);

    assert_eq!(vm.credentials[0].subtitle, "私钥");
    assert_eq!(vm.credentials[0].meta, "私钥");
    assert_eq!(vm.tunnels[0].subtitle, "运行态");
    assert_eq!(vm.tunnels[0].meta, "运行中");
}

#[test]
fn app_view_model_projects_quick_host_form() {
    let mut state = AppState::default();
    state
        .ui
        .set_quick_host_field(QuickHostDraftField::Name, "prod".to_owned());
    state
        .ui
        .set_quick_host_field(QuickHostDraftField::Address, "prod.example.com".to_owned());
    state
        .ui
        .set_quick_host_field(QuickHostDraftField::Port, "2202".to_owned());
    state
        .ui
        .set_quick_host_field(QuickHostDraftField::Username, "deploy".to_owned());
    state
        .ui
        .set_quick_host_field(QuickHostDraftField::Tags, "prod,linux".to_owned());
    state.ui.set_quick_host_auth_kind(QuickHostAuthKind::Key);
    state
        .ui
        .set_quick_host_auth_field(QuickHostAuthField::PrivateKeyRef, "key:prod".to_owned());

    let vm = app_view_model(&state);

    assert_eq!(vm.quick_host.name, "prod");
    assert_eq!(vm.quick_host.address, "prod.example.com");
    assert_eq!(vm.quick_host.port, "2202");
    assert_eq!(vm.quick_host.username, "deploy");
    assert_eq!(vm.quick_host.tags, "prod,linux");
    assert_eq!(vm.quick_host.auth_kind, "Key");
    assert_eq!(vm.quick_host.private_key_ref, "key:prod");
}

#[test]
fn app_view_model_projects_create_host_dialog_state() {
    let mut state = AppState::default();
    state.apply(Message::OpenCreateHostDialog);

    let vm = app_view_model(&state);

    assert!(vm.create_host_dialog_open);
}

#[test]
fn app_view_model_projects_create_host_dialog_text_by_language() {
    let mut state = AppState::default();

    state.ui.workspace.language = LanguageMode::Chinese;
    let zh = app_view_model(&state).create_host_dialog;
    assert_eq!(zh.dialog_title, "创建主机");
    assert_eq!(zh.address_label, "地址");
    assert_eq!(zh.agent_source_title, "Agent 来源");

    state.ui.workspace.language = LanguageMode::English;
    let en = app_view_model(&state).create_host_dialog;
    assert_eq!(en.dialog_title, "Create Host");
    assert_eq!(en.address_label, "Address");
    assert_eq!(en.agent_source_title, "Agent Source");
}

#[test]
fn app_view_model_projects_edit_host_dialog_text() {
    let mut state = AppState::default();
    state.ui.workspace.language = LanguageMode::Chinese;
    state.ui.quick_host.editing_host_id = Some(HostId(Uuid::new_v4()));

    let dialog = app_view_model(&state).create_host_dialog;

    assert!(dialog.editing);
    assert_eq!(dialog.dialog_title, "编辑主机");
    assert_eq!(dialog.create_label, "保存修改");
}

#[test]
fn app_view_model_projects_workspace_text_by_language() {
    let mut state = AppState::default();

    state.ui.workspace.language = LanguageMode::Chinese;
    let zh = app_view_model(&state).workspace_text;
    assert_eq!(zh.nav_hosts, "主机");
    assert_eq!(zh.nav_security, "密钥");
    assert_eq!(zh.host_open, "终端");
    assert_eq!(zh.host_edit, "编辑");
    assert_eq!(zh.host_delete_title, "删除主机");
    assert_eq!(zh.tool_keys, "密钥");
    assert_eq!(zh.new_session_local_kind, "本地");
    assert_eq!(zh.new_session_remote_kind, "远程");
    assert_eq!(zh.new_session_ungrouped_detail, "未分组主机");

    state.ui.workspace.language = LanguageMode::English;
    let en = app_view_model(&state).workspace_text;
    assert_eq!(en.nav_hosts, "Hosts");
    assert_eq!(en.nav_security, "Keys");
    assert_eq!(en.host_open, "Shell");
    assert_eq!(en.host_edit, "Edit");
    assert_eq!(en.host_delete_title, "Delete Host");
    assert_eq!(en.tool_keys, "Keys");
    assert_eq!(en.new_session_local_kind, "Local");
    assert_eq!(en.new_session_remote_kind, "Remote");
    assert_eq!(en.new_session_ungrouped_detail, "Ungrouped host");
}

#[test]
fn app_view_model_projects_settings_options_and_storage_summary() {
    let mut state = AppState::default();
    state.ui.workspace.language = LanguageMode::Chinese;
    state.apply(Message::SetBuiltInTheme {
        theme: crate::model::BuiltInTheme::Dracula,
    });
    state.storage.upsert_theme(ThemeProfileRecord {
        name: "Imported".to_owned(),
        profile_toml: "name = \"Imported\"".to_owned(),
        builtin: false,
    });
    state.storage.upsert_host(Host {
        id: HostId(Uuid::new_v4()),
        name: "Production".to_owned(),
        group_id: None,
        icon_key: "server".to_owned(),
        tags: vec!["prod".to_owned()],
        address: "prod.example.com".to_owned(),
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
    });

    let settings = app_view_model(&state).settings;

    assert_eq!(settings.text.title, "设置");
    assert_eq!(settings.text.language_title, "语言");
    assert_eq!(settings.text.theme_title, "内置主题");
    assert_eq!(settings.text.custom_theme_title, "主题资料");
    assert_eq!(settings.text.file_actions_title, "文件操作");
    assert_eq!(settings.text.apply_label, "应用");
    assert_eq!(settings.text.copy_label, "复制");
    assert_eq!(settings.text.choose_file_label, "选择");
    assert_eq!(settings.text.no_custom_themes_label, "暂无导入主题");
    assert!(
        settings
            .language_options
            .iter()
            .any(|option| option.key == "Chinese" && option.selected)
    );
    assert!(
        settings
            .theme_options
            .iter()
            .any(|option| option.key == "Dracula" && option.selected)
    );
    assert_eq!(settings.theme.current_theme_name, "Dracula");
    assert_eq!(settings.theme.current_profile_name, "Default Dark");
    assert_eq!(settings.theme.built_in_theme_count, 7);
    assert_eq!(settings.theme.custom_theme_count, 1);
    assert_eq!(
        settings.theme.custom_theme_names,
        vec!["Imported".to_owned()]
    );
    assert_eq!(settings.theme.custom_theme_profiles.len(), 1);
    assert_eq!(settings.theme.custom_theme_profiles[0].name, "Imported");
    assert_eq!(settings.theme.custom_theme_profiles[0].source_label, "导入");
    assert!(!settings.theme.custom_theme_profiles[0].selected);
    assert!(settings.theme.custom_theme_profiles[0].can_apply);
    assert!(settings.theme.custom_theme_profiles[0].can_remove);
    assert!(settings.theme.can_import);
    assert!(settings.theme.can_export_current_theme);
    assert_eq!(settings.theme.import_formats.len(), 5);
    assert_eq!(settings.theme.import_formats[0].key, "NativeToml");
    assert_eq!(settings.theme.import_formats[0].extension, "toml");
    assert!(settings.theme.import_formats[0].supported);
    assert!(settings.theme.import_formats[0].selected);
    assert!(
        settings
            .theme
            .import_formats
            .iter()
            .any(|format| format.key == "VsCodeJson"
                && format.extension == "json"
                && format.supported)
    );
    assert!(
        settings
            .theme
            .import_formats
            .iter()
            .any(|format| format.key == "WindowsTerminalJson"
                && format.extension == "json"
                && format.supported)
    );
    assert!(
        settings
            .theme
            .import_formats
            .iter()
            .any(|format| format.key == "AlacrittyToml"
                && format.extension == "toml"
                && format.supported)
    );
    assert!(
        settings
            .theme
            .import_formats
            .iter()
            .any(|format| format.key == "ItermColors" && !format.supported)
    );
    assert_eq!(settings.theme.export_formats.len(), 5);
    assert_eq!(settings.theme.export_formats[0].key, "NativeToml");
    assert_eq!(settings.theme.export_formats[0].label, "SmagicalSSH 主题");
    assert_eq!(settings.theme.export_formats[0].extension, "toml");
    assert!(settings.theme.export_formats[0].supported);
    assert!(settings.theme.export_formats[0].selected);
    assert!(settings.theme.export_formats.iter().any(
        |format| format.key == "WindowsTerminalJson" && format.label == "Windows Terminal 主题"
    ));
    assert!(
        settings
            .theme
            .export_formats
            .iter()
            .any(|format| format.key == "AlacrittyToml" && format.label == "Alacritty 主题")
    );
    assert!(
        settings
            .theme
            .export_formats
            .iter()
            .any(|format| format.key == "ItermColors" && !format.supported)
    );
    assert_eq!(settings.theme.default_import_extension, "toml");
    assert_eq!(
        settings.theme.default_export_file_name,
        "Dracula.smagical-theme.toml"
    );
    assert_eq!(settings.file_actions.len(), 6);
    assert_eq!(settings.file_actions[0].key, "ImportTheme");
    assert_eq!(settings.file_actions[0].label, "导入主题");
    assert_eq!(settings.file_actions[0].category_key, "Theme");
    assert_eq!(settings.file_actions[0].category_label, "主题");
    assert_eq!(settings.file_actions[0].direction, "Import");
    assert_eq!(settings.file_actions[0].direction_label, "导入");
    assert_eq!(settings.file_actions[0].format_key, "NativeToml");
    assert_eq!(settings.file_actions[0].format_label, "主题文件");
    assert_eq!(settings.file_actions[0].default_file_name, "");
    assert_eq!(settings.file_actions[0].default_extension, "toml");
    assert_eq!(
        settings.file_actions[0].path_placeholder,
        "输入要导入的文件路径"
    );
    assert!(settings.file_actions[0].enabled);
    assert_eq!(settings.file_actions[1].key, "ExportCurrentTheme");
    assert_eq!(settings.file_actions[1].label, "导出当前主题");
    assert_eq!(settings.file_actions[1].category_key, "Theme");
    assert_eq!(settings.file_actions[1].direction, "Export");
    assert_eq!(settings.file_actions[1].direction_label, "导出");
    assert_eq!(settings.file_actions[1].format_key, "NativeToml");
    assert_eq!(
        settings.file_actions[1].default_file_name,
        "Dracula.smagical-theme.toml"
    );
    assert_eq!(
        settings.file_actions[1].path_placeholder,
        "输入导出目标路径"
    );
    assert!(settings.file_actions[1].enabled);
    assert_eq!(settings.file_actions[2].key, "BackupSqlite");
    assert_eq!(settings.file_actions[2].label, "备份数据库");
    assert_eq!(settings.file_actions[2].category_key, "Storage");
    assert_eq!(settings.file_actions[2].category_label, "存储");
    assert_eq!(settings.file_actions[2].direction, "Export");
    assert_eq!(settings.file_actions[2].format_label, "数据库");
    assert_eq!(
        settings.file_actions[2].default_file_name,
        "smagicalssh-backup.sqlite"
    );
    assert!(!settings.file_actions[2].enabled);
    assert_eq!(settings.file_actions[4].key, "ImportSnapshot");
    assert_eq!(settings.file_actions[4].direction, "Import");
    assert_eq!(settings.file_actions[4].default_extension, "toml");
    assert!(settings.storage_summary.contains("1 主机"));
    assert_eq!(settings.storage.summary_items.len(), 9);
    assert_eq!(settings.storage.summary_items[0].key, "Hosts");
    assert_eq!(settings.storage.summary_items[0].label, "主机");
    assert_eq!(settings.storage.summary_items[0].count, 1);
    assert_eq!(settings.storage.summary_items[1].key, "Groups");
    assert_eq!(settings.storage.summary_items[1].label, "分组");
    assert_eq!(settings.storage.summary_items[1].count, 0);
    assert_eq!(settings.storage.summary_items[6].key, "Tunnels");
    assert_eq!(settings.storage.summary_items[6].label, "隧道");
    assert_eq!(settings.storage.summary_items[7].key, "Themes");
    assert_eq!(settings.storage.summary_items[7].label, "主题");
    assert_eq!(settings.storage.summary_items[7].count, 1);
    assert_eq!(settings.storage.summary_items[8].key, "WorkspaceTabs");
    assert_eq!(settings.storage.summary_items[8].label, "标签页");
    assert_eq!(settings.security_summary, "未启用数据库加密");
    assert_eq!(settings.storage.backend_label, "内存存储");
    assert_eq!(settings.storage.database_path, "当前未绑定本地数据库");
    assert!(!settings.storage.can_backup);
    assert!(!settings.storage.can_export_snapshot);
    assert!(!settings.storage.can_import_snapshot);
    assert!(!settings.storage.can_import_sqlite_backup);
    assert_eq!(settings.storage.actions.len(), 4);
    assert_eq!(settings.storage.actions[0].key, "BackupSqlite");
    assert_eq!(settings.storage.actions[0].label, "备份数据库");
    assert_eq!(
        settings.storage.actions[0].default_file_name,
        "smagicalssh-backup.sqlite"
    );
    assert!(!settings.storage.actions[0].enabled);
    assert_eq!(settings.storage.actions[1].key, "ExportSnapshot");
    assert_eq!(settings.storage.actions[1].label, "导出快照");
    assert_eq!(
        settings.storage.actions[1].default_file_name,
        "smagicalssh-snapshot.toml"
    );
    assert!(!settings.storage.actions[1].enabled);
    assert_eq!(settings.storage.actions[2].key, "ImportSnapshot");
    assert_eq!(settings.storage.actions[2].label, "导入快照");
    assert_eq!(settings.storage.actions[2].default_file_name, "");
    assert!(!settings.storage.actions[2].enabled);
    assert_eq!(settings.storage.actions[3].key, "ImportSqliteBackup");
    assert_eq!(settings.storage.actions[3].label, "导入数据库");
    assert_eq!(settings.storage.actions[3].default_file_name, "");
    assert!(!settings.storage.actions[3].enabled);
    assert_eq!(
        settings.storage.default_backup_file_name,
        "smagicalssh-backup.sqlite"
    );
    assert_eq!(
        settings.storage.default_snapshot_file_name,
        "smagicalssh-snapshot.toml"
    );
    assert_eq!(settings.security.encryption_key, "Disabled");
    assert_eq!(settings.security.encryption_label, "未启用数据库加密");
    assert!(!settings.security.encryption_enabled);
    assert!(!settings.security.can_configure_encryption);
    assert_eq!(settings.security.status_label, "未加密");
    assert_eq!(settings.security.detail_label, "可在后续设置密码后启用");
    assert_eq!(settings.security.kdf_label, "未配置");
    assert_eq!(settings.security.encryption_version_label, "无");
}

#[test]
fn app_view_model_marks_current_custom_theme_profile_selected() {
    let mut state = AppState::default();
    state.ui.workspace.language = LanguageMode::Chinese;
    state.config.theme.name = "Imported".to_owned();
    state.storage.app_config = state.config.clone();
    state.storage.upsert_theme(ThemeProfileRecord {
        name: "Imported".to_owned(),
        profile_toml: "name = \"Imported\"".to_owned(),
        builtin: false,
    });

    let settings = app_view_model(&state).settings;

    assert_eq!(settings.theme.current_profile_name, "Imported");
    assert_eq!(
        settings.theme.default_export_file_name,
        "Imported.smagical-theme.toml"
    );
    assert_eq!(settings.theme.custom_theme_profiles.len(), 1);
    assert!(settings.theme.custom_theme_profiles[0].selected);
    assert!(!settings.theme.custom_theme_profiles[0].can_apply);
    assert!(settings.theme.custom_theme_profiles[0].can_remove);
}

#[test]
fn app_view_model_projects_sqlite_storage_settings_status() {
    let sqlite_path =
        std::env::temp_dir().join(format!("smagicalssh-settings-vm-{}.sqlite", Uuid::new_v4()));
    let mut state = AppState::default().with_storage_backend(SqliteStorage::new(&sqlite_path));
    state.ui.workspace.language = LanguageMode::English;

    let settings = app_view_model(&state).settings;

    assert_eq!(settings.storage.backend_label, "SQLite local database");
    assert!(settings.storage.database_path.ends_with(".sqlite"));
    assert!(settings.storage.can_backup);
    assert!(settings.storage.can_export_snapshot);
    assert!(settings.storage.can_import_snapshot);
    assert!(settings.storage.can_import_sqlite_backup);
    assert!(settings.storage.actions.iter().all(|action| action.enabled));
}

#[test]
fn app_view_model_localizes_theme_name() {
    let mut state = AppState::default();

    state.ui.workspace.language = LanguageMode::Chinese;
    assert_eq!(app_view_model(&state).theme_name, "专业暗色");

    state.ui.workspace.language = LanguageMode::English;
    assert_eq!(app_view_model(&state).theme_name, "Professional Dark");
}

#[test]
fn app_view_model_projects_pending_remove_host_dialog() {
    let mut state = AppState::default();
    state.ui.workspace.language = LanguageMode::Chinese;
    let host = Host {
        id: HostId(Uuid::new_v4()),
        name: "prod".to_owned(),
        group_id: None,
        icon_key: "server".to_owned(),
        tags: Vec::new(),
        address: "prod.example.com".to_owned(),
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
    };
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.apply(Message::RequestRemoveHost { host_id });

    let vm = app_view_model(&state);

    assert!(vm.remove_host_dialog_open);
    assert_eq!(vm.remove_host_dialog_name, "prod");
}

#[test]
fn app_view_model_keeps_logic_keys_stable_when_text_is_chinese() {
    let mut state = AppState::default();
    state.ui.workspace.language = LanguageMode::Chinese;

    let vm = app_view_model(&state);

    assert_eq!(vm.active_page_key, "Hosts");
    assert_eq!(vm.active_page, "主机");
    assert_eq!(vm.tool_panel_mode_key, "Closed");
    assert_eq!(vm.tool_panel_mode, "关闭");
}

#[test]
fn app_view_model_localizes_connected_status_but_keeps_status_key_stable() {
    let mut state = AppState::default();
    state.ui.workspace.language = LanguageMode::Chinese;
    let host_id = HostId(Uuid::new_v4());
    let session_id = crate::model::SessionId(Uuid::new_v4());
    state.storage.upsert_host(Host {
        id: host_id,
        name: "prod".to_owned(),
        group_id: None,
        icon_key: "server".to_owned(),
        tags: Vec::new(),
        address: "prod.example.com".to_owned(),
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
    });
    state.sessions.open_shell_tab(session_id, host_id, "prod");
    state
        .sessions
        .set_status(session_id, crate::model::SessionStatus::Connected);

    let vm = app_view_model(&state);

    assert_eq!(vm.tabs[0].status_key, "Connected");
    assert_eq!(vm.tabs[0].status, "已连接");
    assert_eq!(vm.hosts[0].status_key, "Connected");
    assert_eq!(vm.hosts[0].status, "已连接");
}

#[test]
fn app_view_model_keeps_sftp_panel_on_active_host_without_browser() {
    let mut state = AppState::default();
    let sftp_host = Host {
        id: HostId(Uuid::new_v4()),
        name: "files".to_owned(),
        group_id: None,
        icon_key: "server".to_owned(),
        tags: Vec::new(),
        address: "files.example.com".to_owned(),
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
    };
    let shell_host = Host {
        id: HostId(Uuid::new_v4()),
        name: "shell".to_owned(),
        group_id: None,
        icon_key: "server".to_owned(),
        tags: Vec::new(),
        address: "shell.example.com".to_owned(),
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
    };
    let sftp_host_id = sftp_host.id;
    let shell_host_id = shell_host.id;
    state.storage.upsert_host(sftp_host);
    state.storage.upsert_host(shell_host);
    state.sessions.open_sftp_tab(
        crate::model::SessionId(Uuid::new_v4()),
        sftp_host_id,
        "/var/log",
    );
    state.sessions.open_shell_tab(
        crate::model::SessionId(Uuid::new_v4()),
        shell_host_id,
        "shell",
    );

    let vm = app_view_model(&state);

    assert_eq!(vm.sftp.host_id, shell_host_id.0.to_string());
    assert_eq!(vm.sftp.title, "SFTP · shell");
    assert_eq!(vm.sftp.current_dir, "/");
    assert!(vm.sftp.entries.is_empty());
}
