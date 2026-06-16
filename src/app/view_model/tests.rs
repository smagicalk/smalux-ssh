use super::*;
use crate::app::state::DesktopAppState;
use crate::core::CoreState;
use crate::model::{
    AgentSource, AuthProfile, CredentialGroup, CredentialGroupId, CredentialInspection,
    CredentialKind, CredentialMetadata, ForwardAsset, ForwardId, Host, HostId,
    HostNetworkSelection, JumpChainAsset, JumpChainId, JumpProfile, KeyAlgorithm, KnownHostEntry,
    LanguageMode, Message, ProxyAsset, ProxyId, ProxyProfile, QuickHostAuthField,
    QuickHostAuthKind, QuickHostDraftField, SecretMaterialKind, SecretRecord, SecretRef, SessionId,
    Snippet, SnippetGroup, SnippetGroupId, SnippetImplementation, SnippetImplementationId,
    SnippetScope, SnippetShell, SnippetSupportTarget, SnippetSupportTargetId, TunnelKind,
    TunnelRule, TunnelRuntimeState, TunnelStatus, UiState,
};
use crate::storage::{SqliteStorage, ThemeProfileRecord};
use uuid::Uuid;

fn desktop_state() -> DesktopAppState {
    desktop_state_with_core(CoreState::default())
}

fn desktop_state_with_core(core: CoreState) -> DesktopAppState {
    let ui = UiState::from_visual(&core.config.theme, &core.config.background);
    DesktopAppState { core, ui }
}

#[test]
fn app_view_model_uses_local_terminal_when_no_tab_is_open() {
    let mut state = desktop_state();
    state.ui.workspace.language = LanguageMode::English;

    let vm = app_view_model(&state);

    assert_eq!(
        vm.terminal_workspace.terminal.title,
        crate::model::DEFAULT_LOCAL_TERMINAL_TITLE
    );
    assert_eq!(vm.terminal_workspace.terminal.status, "Ready");
    assert!(vm.terminal_workspace.terminal.can_send_input);
}

#[test]
fn app_view_model_projects_workspace_page_models() {
    let mut state = desktop_state();
    state.ui.workspace.language = LanguageMode::English;
    state.ui.workspace.credential_search_query = "deploy".to_owned();
    state.ui.workspace.snippet_search_query = "nginx".to_owned();

    let vm = app_view_model(&state);

    assert_eq!(
        vm.terminal_workspace.terminal.title,
        crate::model::DEFAULT_LOCAL_TERMINAL_TITLE
    );
    assert_eq!(vm.terminal_workspace.sftp.current_dir, "/");
    assert!(vm.terminal_workspace.tabs.is_empty());
    assert!(vm.terminal_workspace.history.is_empty());
    assert_eq!(vm.terminal_workspace.tool_panel_width, 328);
    assert_eq!(vm.terminal_workspace.tool_panel_mode_key, "Closed");
    assert_eq!(vm.security_workspace.search_query, "deploy");
    assert!(vm.security_workspace.credentials.is_empty());
    assert!(!vm.security_workspace.credential_rows.is_empty());
    assert_eq!(vm.snippet_workspace.search_query, "nginx");
    assert!(vm.snippet_workspace.snippets.is_empty());
    assert!(!vm.snippet_workspace.rows.is_empty());
    assert!(vm.snippet_workspace.target_options.is_empty());
    assert_eq!(vm.settings_workspace.settings.text.title, "Settings");
}

#[test]
fn auth_label_covers_password_without_secret_leakage() {
    let mut state = desktop_state();
    state.ui.workspace.language = LanguageMode::English;
    state.core.storage.upsert_host(Host {
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
        network: Default::default(),
        proxies: Vec::new(),
        jumps: Vec::new(),
        theme_override: None,
        background_override: None,
    });

    let vm = app_view_model(&state);

    assert_eq!(vm.hosts[0].auth, "Password");
}

#[test]
fn app_view_model_filters_new_session_hosts_without_changing_host_list() {
    let mut state = desktop_state();
    state.ui.workspace.language = LanguageMode::English;
    state.core.storage.upsert_host(Host {
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
        network: Default::default(),
        proxies: Vec::new(),
        jumps: Vec::new(),
        theme_override: None,
        background_override: None,
    });
    state.core.storage.upsert_host(Host {
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
        network: Default::default(),
        proxies: Vec::new(),
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
    let mut state = desktop_state();
    state.core.storage.upsert_host(Host {
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
        network: Default::default(),
        proxies: Vec::new(),
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
fn app_view_model_projects_snippet_tree_and_filters_by_search() {
    let mut state = desktop_state();
    state.ui.workspace.language = LanguageMode::English;
    let host_id = HostId(Uuid::new_v4());
    let snippet_group_id = SnippetGroupId(Uuid::new_v4());
    state.core.storage.upsert_host(Host {
        id: host_id,
        name: "Production".to_owned(),
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
        network: Default::default(),
        proxies: Vec::new(),
        jumps: Vec::new(),
        theme_override: None,
        background_override: None,
    });
    state.core.storage.upsert_snippet_group(SnippetGroup {
        id: snippet_group_id,
        name: "Operations".to_owned(),
        parent_id: None,
        sort_order: 0,
    });
    state
        .core
        .storage
        .upsert_snippet(Snippet::with_default_implementation(
            crate::model::SnippetId(Uuid::new_v4()),
            "Kubernetes pods".to_owned(),
            Some("List pods".to_owned()),
            SnippetScope::Global,
            Some(snippet_group_id),
            "kubectl get pods".to_owned(),
        ));
    let mut restart_snippet = Snippet::with_default_implementation(
        crate::model::SnippetId(Uuid::new_v4()),
        "restart service".to_owned(),
        Some("Restart a service".to_owned()),
        SnippetScope::Host(host_id),
        None,
        "systemctl restart {{service}}".to_owned(),
    );
    restart_snippet
        .default_implementation_mut()
        .expect("默认实现应存在")
        .last_arguments = vec![crate::model::SnippetArgument {
        name: "service".to_owned(),
        value: "nginx".to_owned(),
    }];
    let shared_implementation_id = SnippetImplementationId(Uuid::new_v4());
    restart_snippet.implementations.push(SnippetImplementation {
        id: shared_implementation_id,
        snippet_id: restart_snippet.id,
        name: "Linux shared".to_owned(),
        shell: SnippetShell::Bash,
        command_template: "systemctl status {{service}}".to_owned(),
        notes: None,
        last_arguments: Vec::new(),
        sort_order: 1,
    });
    restart_snippet.support_targets.push(SnippetSupportTarget {
        id: SnippetSupportTargetId(Uuid::new_v4()),
        snippet_id: restart_snippet.id,
        target_key: "debian-ubuntu".to_owned(),
        display_name: "Ubuntu legacy label".to_owned(),
        implementation_id: shared_implementation_id,
        sort_order: 1,
    });
    restart_snippet.support_targets.push(SnippetSupportTarget {
        id: SnippetSupportTargetId(Uuid::new_v4()),
        snippet_id: restart_snippet.id,
        target_key: "rhel-centos".to_owned(),
        display_name: "RHEL legacy label".to_owned(),
        implementation_id: shared_implementation_id,
        sort_order: 2,
    });
    state.core.storage.upsert_snippet(restart_snippet);

    let vm = app_view_model(&state);

    assert_eq!(vm.snippet_workspace.rows[0].id, "snippet-folder:all");
    assert!(
        vm.snippet_workspace
            .rows
            .iter()
            .any(|row| row.icon_key == "folder")
    );
    assert!(
        vm.snippet_workspace
            .rows
            .iter()
            .any(|row| row.icon_key == "code")
    );
    assert!(
        vm.snippet_workspace
            .rows
            .iter()
            .any(|row| row.name == "Operations" && row.node_kind == "Group")
    );
    assert!(
        vm.snippet_workspace
            .rows
            .iter()
            .any(|row| row.node_kind == "SnippetTarget"
                && row.scope_key == "SupportTargetShared"
                && row.meta == "shared 2")
    );
    let shared_target = vm
        .snippet_workspace
        .rows
        .iter()
        .find(|row| row.node_kind == "SnippetTarget" && row.name == "debian-ubuntu")
        .expect("片段目标节点应展示标签名");
    assert_eq!(shared_target.description, "debian-ubuntu;rhel-centos");
    assert!(shared_target.target_debian_selected);
    assert!(shared_target.target_rhel_selected);
    assert!(!shared_target.target_linux_selected);
    assert!(!shared_target.target_debian_disabled);
    assert!(!shared_target.target_rhel_disabled);
    assert!(shared_target.target_linux_disabled);
    assert_eq!(shared_target.icon_key, "debian");
    assert_eq!(shared_target.accent_index, 1);
    let restart_row = vm
        .snippet_workspace
        .rows
        .iter()
        .find(|row| row.node_kind == "Snippet" && row.name == "restart service")
        .expect("片段节点应存在");
    assert!(restart_row.target_linux_disabled);
    assert!(restart_row.target_debian_disabled);
    assert!(restart_row.target_rhel_disabled);
    assert!(!restart_row.target_macos_disabled);
    assert!(
        vm.snippet_workspace
            .target_options
            .iter()
            .any(|row| row.node_kind == "SnippetTarget" && row.name == "rhel-centos")
    );

    state
        .ui
        .workspace
        .collapsed_snippet_tree_nodes
        .push("snippet-folder:all".to_owned());
    let collapsed_vm = app_view_model(&state);
    assert_eq!(collapsed_vm.snippet_workspace.rows.len(), 1);
    assert!(
        collapsed_vm
            .snippet_workspace
            .target_options
            .iter()
            .any(|row| row.node_kind == "SnippetTarget" && row.name == "rhel-centos")
    );
    state.ui.workspace.collapsed_snippet_tree_nodes.clear();

    state.ui.workspace.set_snippet_search_query("restart");
    let filtered = app_view_model(&state);

    assert!(
        filtered
            .snippet_workspace
            .rows
            .iter()
            .any(|row| row.name == "restart service")
    );
    assert!(
        filtered
            .snippet_workspace
            .rows
            .iter()
            .any(|row| row.node_kind == "SnippetTarget" && row.argument_values == "service=nginx")
    );
    assert!(
        filtered
            .snippet_workspace
            .rows
            .iter()
            .any(|row| row.node_kind == "SnippetTarget" && row.variable_names == "service")
    );
    assert!(
        !filtered
            .snippet_workspace
            .rows
            .iter()
            .any(|row| row.name == "Kubernetes pods")
    );
}

#[test]
fn app_view_model_projects_known_hosts_for_tool_panel() {
    let mut state = desktop_state();
    state.ui.workspace.language = LanguageMode::English;
    state
        .core
        .storage
        .upsert_known_host(KnownHostEntry::untrusted(
            "example.com",
            22,
            KeyAlgorithm::Ed25519,
            "SHA256:new",
        ));

    let vm = app_view_model(&state);

    assert_eq!(vm.terminal_workspace.known_hosts.len(), 1);
    assert_eq!(vm.terminal_workspace.known_hosts[0].host, "example.com");
    assert_eq!(vm.terminal_workspace.known_hosts[0].port, 22);
    assert_eq!(
        vm.terminal_workspace.known_hosts[0].fingerprint,
        "SHA256:new"
    );
    assert_eq!(vm.terminal_workspace.known_hosts[0].status_key, "pending");
    assert_eq!(vm.terminal_workspace.known_hosts[0].status, "pending");
}

#[test]
fn app_view_model_projects_credentials_for_security_page() {
    let mut state = desktop_state();
    state
        .core
        .storage
        .upsert_secret(SecretRecord::local_plaintext(
            SecretRef("key:deploy".to_owned()),
            SecretMaterialKind::PrivateKey,
            b"private-key".to_vec(),
        ));
    state.core.storage.upsert_credential(CredentialMetadata {
        id: crate::model::CredentialId(uuid::Uuid::new_v4()),
        name: "deploy".to_owned(),
        kind: CredentialKind::PrivateKey,
        group_id: None,
        username: Some("ubuntu".to_owned()),
        secret: Some(SecretRef("key:deploy".to_owned())),
        key_algorithm: Some(KeyAlgorithm::Ed25519),
        fingerprint: Some("SHA256:key".to_owned()),
    });

    let vm = app_view_model(&state);

    let security = &vm.security_workspace;
    assert_eq!(security.credentials.len(), 1);
    assert_eq!(security.credentials[0].title, "deploy");
    assert_eq!(security.credentials[0].subtitle, "ubuntu");
    assert_eq!(security.credentials[0].meta, "SHA256:key");
    assert_eq!(security.credential_rows[0].id, "group:all");
    assert_eq!(security.credential_rows[1].id, "group:PrivateKey");
    assert_eq!(security.credential_rows[2].id, "credential:deploy");
    assert!(security.credential_rows[0].expandable);
    assert!(security.credential_rows[0].expanded);
    assert!(security.credential_rows[1].expandable);
    assert!(security.credential_rows[1].expanded);
    assert!(!security.credential_rows[2].expandable);
    assert_eq!(security.credential_rows[2].secret_ref, "已保存，可查看");
    assert!(security.credential_rows[2].secret_available);
    assert_eq!(security.credential_rows[2].algorithm, "ed25519");
    assert_eq!(security.credential_rows[2].algorithm_key, "Ed25519");
    assert_eq!(security.credential_rows[2].group_path, "私钥");
}

#[test]
fn app_view_model_projects_credential_detail_fields_from_inspection() {
    let mut state = desktop_state();
    state.ui.workspace.language = LanguageMode::Chinese;
    let credential_id = crate::model::CredentialId(uuid::Uuid::new_v4());
    state
        .core
        .storage
        .upsert_secret(SecretRecord::local_plaintext(
            SecretRef("secret://keys/deploy".to_owned()),
            SecretMaterialKind::PrivateKey,
            b"private-key".to_vec(),
        ));
    state.core.storage.upsert_credential(CredentialMetadata {
        id: credential_id,
        name: "deploy".to_owned(),
        kind: CredentialKind::PrivateKey,
        group_id: None,
        username: Some("ubuntu".to_owned()),
        secret: Some(SecretRef("secret://keys/deploy".to_owned())),
        key_algorithm: Some(KeyAlgorithm::Rsa),
        fingerprint: Some("SHA256:old".to_owned()),
    });
    state
        .core
        .storage
        .upsert_credential_inspection(CredentialInspection {
            credential_id,
            kind: CredentialKind::PrivateKey,
            payload_hash: "hash".to_owned(),
            parser_version: 1,
            parse_error: None,
            algorithm: Some(KeyAlgorithm::Ed25519),
            fingerprint: Some("SHA256:new".to_owned()),
            public_key: Some("ssh-ed25519 AAAA deploy".to_owned()),
            comment: Some("deploy".to_owned()),
            encrypted: Some(false),
            password_length: None,
            certificate: None,
        });

    let vm = app_view_model(&state);
    let fields = vm
        .security_workspace
        .detail_fields
        .iter()
        .filter(|field| field.credential_id == "credential:deploy")
        .map(|field| (field.label.as_str(), field.value.as_str()))
        .collect::<Vec<_>>();

    assert!(fields.contains(&("类型", "私钥")));
    assert!(fields.contains(&("用户名", "ubuntu")));
    assert!(fields.contains(&("分组", "私钥")));
    assert!(fields.contains(&("本地内容", "已保存，可查看")));
    assert!(fields.contains(&("算法", "ed25519")));
    assert!(fields.contains(&("指纹", "SHA256:new")));
    assert!(fields.contains(&("解析状态", "正常")));
    assert!(fields.contains(&("是否加密", "否")));
    assert!(fields.contains(&("备注", "deploy")));
    assert!(fields.contains(&("公钥", "ssh-ed25519 AAAA deploy")));
}

#[test]
fn app_view_model_omits_agent_credentials_from_security_page() {
    let mut state = desktop_state();
    state.core.storage.upsert_credential(CredentialMetadata {
        id: crate::model::CredentialId(uuid::Uuid::new_v4()),
        name: "local-agent".to_owned(),
        kind: CredentialKind::Agent,
        group_id: None,
        username: Some("deploy".to_owned()),
        secret: Some(SecretRef("agent://auto".to_owned())),
        key_algorithm: None,
        fingerprint: None,
    });

    let vm = app_view_model(&state);

    assert!(
        !vm.security_workspace
            .credential_rows
            .iter()
            .any(|row| row.kind_key == "Agent" || row.id == "credential:local-agent")
    );
    assert!(
        !vm.security_workspace
            .credentials
            .iter()
            .any(|item| item.title == "local-agent")
    );
}

#[test]
fn app_view_model_projects_custom_credential_groups() {
    let mut state = desktop_state();
    let group_id = CredentialGroupId(Uuid::new_v4());
    let child_id = CredentialGroupId(Uuid::new_v4());
    state.core.storage.upsert_credential_group(CredentialGroup {
        id: group_id,
        name: "生产密钥".to_owned(),
        kind: CredentialKind::PrivateKey,
        parent_id: None,
        sort_order: 0,
    });
    state.core.storage.upsert_credential_group(CredentialGroup {
        id: child_id,
        name: "API".to_owned(),
        kind: CredentialKind::PrivateKey,
        parent_id: Some(group_id),
        sort_order: 0,
    });

    let vm = app_view_model(&state);
    let rows = &vm.security_workspace.credential_rows;
    let group_index = rows
        .iter()
        .position(|row| row.id == format!("credential-group:{}", group_id.0))
        .expect("parent credential group should be visible");
    let child_index = rows
        .iter()
        .position(|row| row.id == format!("credential-group:{}", child_id.0))
        .expect("child credential group should be visible");

    assert_eq!(rows[group_index].name, "生产密钥");
    assert_eq!(rows[group_index].depth, 2);
    assert_eq!(rows[group_index].group_path, "生产密钥");
    assert_eq!(rows[child_index].name, "API");
    assert_eq!(rows[child_index].depth, 3);
    assert_eq!(rows[child_index].group_path, "生产密钥 / API");
    assert!(group_index < child_index);
}

#[test]
fn app_view_model_projects_credential_group_contents_for_detail_panel() {
    let mut state = desktop_state();
    state.ui.workspace.language = LanguageMode::Chinese;
    let group_id = CredentialGroupId(Uuid::new_v4());
    let child_id = CredentialGroupId(Uuid::new_v4());
    state.core.storage.upsert_credential_group(CredentialGroup {
        id: group_id,
        name: "生产密钥".to_owned(),
        kind: CredentialKind::PrivateKey,
        parent_id: None,
        sort_order: 0,
    });
    state.core.storage.upsert_credential_group(CredentialGroup {
        id: child_id,
        name: "API".to_owned(),
        kind: CredentialKind::PrivateKey,
        parent_id: Some(group_id),
        sort_order: 0,
    });
    state.core.storage.upsert_credential(CredentialMetadata {
        id: crate::model::CredentialId(uuid::Uuid::new_v4()),
        name: "root-key".to_owned(),
        kind: CredentialKind::PrivateKey,
        group_id: None,
        username: Some("root".to_owned()),
        secret: None,
        key_algorithm: Some(KeyAlgorithm::Ed25519),
        fingerprint: None,
    });
    state.core.storage.upsert_credential(CredentialMetadata {
        id: crate::model::CredentialId(uuid::Uuid::new_v4()),
        name: "prod-key".to_owned(),
        kind: CredentialKind::PrivateKey,
        group_id: Some(group_id),
        username: Some("deploy".to_owned()),
        secret: None,
        key_algorithm: Some(KeyAlgorithm::Rsa),
        fingerprint: Some("SHA256:prod".to_owned()),
    });

    let vm = app_view_model(&state);

    let root_contents = vm
        .security_workspace
        .group_contents
        .iter()
        .filter(|row| row.parent_id == "group:all")
        .collect::<Vec<_>>();
    assert_eq!(root_contents.len(), 3);
    assert_eq!(root_contents[0].id, "group:PrivateKey");
    assert_eq!(root_contents[0].name, "私钥");
    assert_eq!(root_contents[0].detail, "凭据分类");
    assert_eq!(root_contents[0].meta, "2 项");

    let private_key_contents = vm
        .security_workspace
        .group_contents
        .iter()
        .filter(|row| row.parent_id == "group:PrivateKey")
        .collect::<Vec<_>>();
    assert_eq!(private_key_contents.len(), 2);
    assert_eq!(private_key_contents[0].node_kind, "CredentialGroup");
    assert_eq!(
        private_key_contents[0].id,
        format!("credential-group:{}", group_id.0)
    );
    assert_eq!(private_key_contents[0].name, "生产密钥");
    assert_eq!(private_key_contents[0].meta, "文件夹");
    assert_eq!(private_key_contents[1].node_kind, "Credential");
    assert_eq!(private_key_contents[1].name, "root-key");

    let custom_group_contents = vm
        .security_workspace
        .group_contents
        .iter()
        .filter(|row| row.parent_id == format!("credential-group:{}", group_id.0))
        .collect::<Vec<_>>();
    assert_eq!(custom_group_contents.len(), 2);
    assert_eq!(custom_group_contents[0].node_kind, "CredentialGroup");
    assert_eq!(custom_group_contents[0].name, "API");
    assert_eq!(custom_group_contents[1].node_kind, "Credential");
    assert_eq!(custom_group_contents[1].name, "prod-key");
    assert_eq!(custom_group_contents[1].meta, "SHA256:prod");

    let empty_child_contents = vm
        .security_workspace
        .group_contents
        .iter()
        .filter(|row| row.parent_id == format!("credential-group:{}", child_id.0))
        .collect::<Vec<_>>();
    assert_eq!(empty_child_contents.len(), 1);
    assert_eq!(empty_child_contents[0].node_kind, "Empty");
    assert_eq!(empty_child_contents[0].name, "此分组为空");
}

#[test]
fn app_view_model_marks_credential_tree_guides_for_later_siblings() {
    let mut state = desktop_state();
    let first_group_id = CredentialGroupId(Uuid::new_v4());
    let child_group_id = CredentialGroupId(Uuid::new_v4());
    let second_group_id = CredentialGroupId(Uuid::new_v4());
    state.core.storage.upsert_credential_group(CredentialGroup {
        id: first_group_id,
        name: "生产密钥".to_owned(),
        kind: CredentialKind::PrivateKey,
        parent_id: None,
        sort_order: 0,
    });
    state.core.storage.upsert_credential_group(CredentialGroup {
        id: child_group_id,
        name: "API".to_owned(),
        kind: CredentialKind::PrivateKey,
        parent_id: Some(first_group_id),
        sort_order: 0,
    });
    state.core.storage.upsert_credential_group(CredentialGroup {
        id: second_group_id,
        name: "测试密钥".to_owned(),
        kind: CredentialKind::PrivateKey,
        parent_id: None,
        sort_order: 1,
    });

    let vm = app_view_model(&state);
    let first_group = vm
        .security_workspace
        .credential_rows
        .iter()
        .find(|row| row.id == format!("credential-group:{}", first_group_id.0))
        .expect("first credential group should be visible");
    let child_group = vm
        .security_workspace
        .credential_rows
        .iter()
        .find(|row| row.id == format!("credential-group:{}", child_group_id.0))
        .expect("child credential group should be visible");
    let second_group = vm
        .security_workspace
        .credential_rows
        .iter()
        .find(|row| row.id == format!("credential-group:{}", second_group_id.0))
        .expect("second credential group should be visible");

    assert!(first_group.has_next_sibling);
    assert!(child_group.guide_1);
    assert!(!second_group.has_next_sibling);
}

#[test]
fn app_view_model_collapses_credential_tree_nodes_without_affecting_search() {
    let mut state = desktop_state();
    let group_id = CredentialGroupId(Uuid::new_v4());
    let child_id = CredentialGroupId(Uuid::new_v4());
    state.core.storage.upsert_credential_group(CredentialGroup {
        id: group_id,
        name: "生产密钥".to_owned(),
        kind: CredentialKind::PrivateKey,
        parent_id: None,
        sort_order: 0,
    });
    state.core.storage.upsert_credential_group(CredentialGroup {
        id: child_id,
        name: "API".to_owned(),
        kind: CredentialKind::PrivateKey,
        parent_id: Some(group_id),
        sort_order: 0,
    });
    let parent_row_id = format!("credential-group:{}", group_id.0);
    let child_row_id = format!("credential-group:{}", child_id.0);

    state
        .ui
        .workspace
        .collapsed_credential_tree_nodes
        .push(parent_row_id.clone());
    let collapsed = app_view_model(&state);

    let parent = collapsed
        .security_workspace
        .credential_rows
        .iter()
        .find(|row| row.id == parent_row_id)
        .expect("collapsed parent group should stay visible");
    assert!(parent.expandable);
    assert!(!parent.expanded);
    assert!(
        !collapsed
            .security_workspace
            .credential_rows
            .iter()
            .any(|row| row.id == child_row_id)
    );

    state.ui.workspace.credential_search_query = "api".to_owned();
    let searched = app_view_model(&state);

    assert!(
        searched
            .security_workspace
            .credential_rows
            .iter()
            .any(|row| row.id == child_row_id)
    );
}

#[test]
fn app_view_model_filters_credential_rows_by_search_query() {
    let mut state = desktop_state();
    state.core.storage.upsert_credential(CredentialMetadata {
        id: crate::model::CredentialId(uuid::Uuid::new_v4()),
        name: "deploy".to_owned(),
        kind: CredentialKind::PrivateKey,
        group_id: None,
        username: Some("ubuntu".to_owned()),
        secret: Some(SecretRef("key:deploy".to_owned())),
        key_algorithm: Some(KeyAlgorithm::Ed25519),
        fingerprint: Some("SHA256:key".to_owned()),
    });
    state.core.storage.upsert_credential(CredentialMetadata {
        id: crate::model::CredentialId(uuid::Uuid::new_v4()),
        name: "db-cert".to_owned(),
        kind: CredentialKind::Certificate,
        group_id: None,
        username: None,
        secret: Some(SecretRef("cert:db".to_owned())),
        key_algorithm: Some(KeyAlgorithm::Rsa),
        fingerprint: Some("SHA256:cert".to_owned()),
    });
    state.ui.workspace.set_credential_search_query("deploy");

    let vm = app_view_model(&state);

    assert_eq!(vm.security_workspace.credential_rows[0].id, "group:all");
    assert_eq!(
        vm.security_workspace.credential_rows[1].id,
        "group:PrivateKey"
    );
    assert_eq!(
        vm.security_workspace.credential_rows[2].id,
        "credential:deploy"
    );
    assert_eq!(vm.security_workspace.credential_rows.len(), 3);
}

#[test]
fn app_view_model_does_not_count_agent_credentials_in_security_search_root() {
    let mut state = desktop_state();
    state.core.storage.upsert_credential(CredentialMetadata {
        id: crate::model::CredentialId(uuid::Uuid::new_v4()),
        name: "deploy".to_owned(),
        kind: CredentialKind::PrivateKey,
        group_id: None,
        username: Some("ubuntu".to_owned()),
        secret: Some(SecretRef("key:deploy".to_owned())),
        key_algorithm: Some(KeyAlgorithm::Ed25519),
        fingerprint: Some("SHA256:key".to_owned()),
    });
    state.core.storage.upsert_credential(CredentialMetadata {
        id: crate::model::CredentialId(uuid::Uuid::new_v4()),
        name: "deploy-agent".to_owned(),
        kind: CredentialKind::Agent,
        group_id: None,
        username: Some("ubuntu".to_owned()),
        secret: Some(SecretRef("agent://auto".to_owned())),
        key_algorithm: None,
        fingerprint: None,
    });
    state.ui.workspace.set_credential_search_query("deploy");

    let vm = app_view_model(&state);

    assert_eq!(vm.security_workspace.credential_rows[0].id, "group:all");
    assert_eq!(vm.security_workspace.credential_rows[0].meta, "1 项");
    assert!(
        !vm.security_workspace
            .credential_rows
            .iter()
            .any(|row| row.id == "credential:deploy-agent")
    );
}

#[test]
fn app_view_model_localizes_tool_panel_fallback_labels() {
    let mut state = desktop_state();
    state.ui.workspace.language = LanguageMode::Chinese;
    state.core.storage.upsert_credential(CredentialMetadata {
        id: crate::model::CredentialId(uuid::Uuid::new_v4()),
        name: "deploy-key".to_owned(),
        kind: CredentialKind::PrivateKey,
        group_id: None,
        username: None,
        secret: Some(SecretRef("key:deploy".to_owned())),
        key_algorithm: None,
        fingerprint: None,
    });
    state.core.sessions.tunnels.push(TunnelRuntimeState {
        session_id: SessionId(Uuid::new_v4()),
        rule_name: "local-db".to_owned(),
        host_id: None,
        status: TunnelStatus::Running,
        started_at_unix_secs: None,
        last_error: None,
    });

    let vm = app_view_model(&state);

    assert_eq!(vm.security_workspace.credentials[0].subtitle, "私钥");
    assert_eq!(vm.security_workspace.credentials[0].meta, "私钥");
    assert_eq!(vm.terminal_workspace.tunnels[0].subtitle, "运行态");
    assert_eq!(vm.terminal_workspace.tunnels[0].meta, "运行中");
}

#[test]
fn app_view_model_projects_quick_host_form() {
    let mut state = desktop_state();
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
    let mut state = desktop_state();
    state.apply_message(Message::OpenCreateHostDialog);

    let vm = app_view_model(&state);

    assert!(vm.create_host_dialog_open);
}

#[test]
fn app_view_model_projects_create_host_dialog_text_by_language() {
    let mut state = desktop_state();

    state.ui.workspace.language = LanguageMode::Chinese;
    let zh = app_view_model(&state).create_host_dialog;
    assert_eq!(zh.dialog_title, "创建主机");
    assert_eq!(zh.address_label, "地址");
    assert_eq!(zh.agent_source_title, "认证代理来源");

    state.ui.workspace.language = LanguageMode::English;
    let en = app_view_model(&state).create_host_dialog;
    assert_eq!(en.dialog_title, "Create Host");
    assert_eq!(en.address_label, "Address");
    assert_eq!(en.agent_source_title, "Agent Source");
}

#[test]
fn app_view_model_projects_edit_host_dialog_text() {
    let mut state = desktop_state();
    state.ui.workspace.language = LanguageMode::Chinese;
    state.ui.quick_host.editing_host_id = Some(HostId(Uuid::new_v4()));

    let dialog = app_view_model(&state).create_host_dialog;

    assert!(dialog.editing);
    assert_eq!(dialog.dialog_title, "编辑主机");
    assert_eq!(dialog.create_label, "保存修改");
}

#[test]
fn app_view_model_projects_workspace_text_by_language() {
    let mut state = desktop_state();

    state.ui.workspace.language = LanguageMode::Chinese;
    let zh = app_view_model(&state).workspace_text;
    assert_eq!(zh.nav_hosts, "主机");
    assert_eq!(zh.nav_security, "凭据");
    assert_eq!(zh.nav_proxy, "网络");
    assert_eq!(zh.host_open, "终端");
    assert_eq!(zh.host_edit, "编辑");
    assert_eq!(zh.host_delete_title, "删除主机");
    assert_eq!(zh.tool_keys, "凭据");
    assert_eq!(zh.tool_proxy, "网络");
    assert_eq!(zh.proxy_empty, "代理池、跳板链和端口转发");
    assert_eq!(zh.proxy_section_route, "代理池");
    assert_eq!(zh.proxy_section_forward, "端口转发");
    assert_eq!(zh.proxy_section_host, "跳板链");
    assert_eq!(zh.proxy_search_empty, "没有匹配的网络资源");
    assert_eq!(zh.proxy_clear_selection, "清空");
    assert_eq!(
        zh.proxy_jump_caption,
        "按顺序进入堡垒机、网关或隔离网段主机。"
    );
    assert_eq!(
        zh.proxy_command_caption,
        "为不同出口维护多个代理配置，再由主机按需选择。"
    );
    assert_eq!(zh.proxy_forward_caption, "本地、远端和动态转发独立管理。");
    assert_eq!(
        zh.proxy_host_hint_caption,
        "跳板只负责 SSH 进入链，代理只负责网络出口。"
    );
    assert_eq!(zh.security_private_keys, "私钥");
    assert_eq!(zh.security_certificates, "证书");
    assert_eq!(zh.security_passwords, "密码");
    assert_eq!(zh.security_agents, "认证代理");
    assert_eq!(zh.security_credential_replace_secret, "替换内容");
    assert_eq!(
        zh.security_credential_generating_private_key,
        "正在生成私钥..."
    );
    assert_eq!(
        zh.security_credential_replace_secret_confirm_title,
        "确认替换内容"
    );
    assert_eq!(zh.security_credential_replace_secret_confirm, "确认替换");
    assert_eq!(zh.security_certificate_text_label, "证书文本");
    assert_eq!(zh.security_secret_copy_success, "已复制");
    assert_eq!(zh.security_field_secret_ref, "本地内容");
    assert_eq!(zh.new_session_local_kind, "本地");
    assert_eq!(zh.new_session_remote_kind, "远程");
    assert_eq!(zh.new_session_ungrouped_detail, "未分组主机");
    assert_eq!(zh.snippets_new_target, "新建目标");
    assert_eq!(zh.snippets_create_target_title, "创建支持目标");
    assert_eq!(zh.snippets_edit_target_title, "编辑支持目标");
    assert_eq!(zh.snippets_target_key_label, "目标标记");
    assert_eq!(zh.snippets_target_mode_new, "新脚本");
    assert_eq!(zh.snippets_target_mode_share, "共享脚本");
    assert_eq!(zh.snippets_split_target, "拆分为独立");

    state.ui.workspace.language = LanguageMode::English;
    let en = app_view_model(&state).workspace_text;
    assert_eq!(en.nav_hosts, "Hosts");
    assert_eq!(en.nav_security, "Creds");
    assert_eq!(en.nav_proxy, "Net");
    assert_eq!(en.host_open, "Shell");
    assert_eq!(en.host_edit, "Edit");
    assert_eq!(en.host_delete_title, "Delete Host");
    assert_eq!(en.tool_keys, "Credentials");
    assert_eq!(en.tool_proxy, "Network");
    assert_eq!(
        en.proxy_empty,
        "Proxy pools, jump chains, and port forwarding"
    );
    assert_eq!(en.proxy_section_route, "Proxy Pool");
    assert_eq!(en.proxy_section_forward, "Port Forwarding");
    assert_eq!(en.proxy_section_host, "Jump Chain");
    assert_eq!(en.proxy_search_empty, "No matching network resources");
    assert_eq!(en.proxy_clear_selection, "Clear");
    assert_eq!(
        en.proxy_jump_caption,
        "Enter bastions, gateways, or isolated hosts in order."
    );
    assert_eq!(
        en.proxy_command_caption,
        "Maintain multiple proxy profiles and let hosts pick one when needed."
    );
    assert_eq!(
        en.proxy_forward_caption,
        "Manage local, remote, and dynamic forwarding separately."
    );
    assert_eq!(
        en.proxy_host_hint_caption,
        "Jumps are SSH entry hops; proxies are network exits."
    );
    assert_eq!(
        en.proxy_search_placeholder,
        "Search proxies, jumps, forwards, hosts, or ports"
    );
    assert_eq!(en.security_private_keys, "Private keys");
    assert_eq!(en.security_certificates, "Certificates");
    assert_eq!(en.security_passwords, "Passwords");
    assert_eq!(en.security_agents, "Agents");
    assert_eq!(en.security_credential_replace_secret, "Replace content");
    assert_eq!(
        en.security_credential_generating_private_key,
        "Generating private key..."
    );
    assert_eq!(
        en.security_credential_replace_secret_confirm_title,
        "Confirm replacement"
    );
    assert_eq!(en.security_credential_replace_secret_confirm, "Replace");
    assert_eq!(en.security_certificate_text_label, "Certificate text");
    assert_eq!(en.security_secret_copy_success, "Copied");
    assert_eq!(en.security_field_secret_ref, "Local content");
    assert_eq!(en.new_session_local_kind, "Local");
    assert_eq!(en.new_session_remote_kind, "Remote");
    assert_eq!(en.new_session_ungrouped_detail, "Ungrouped host");
    assert_eq!(en.snippets_new_target, "New target");
    assert_eq!(en.snippets_create_target_title, "Create target");
    assert_eq!(en.snippets_edit_target_title, "Edit target");
    assert_eq!(en.snippets_target_key_label, "Target key");
    assert_eq!(en.snippets_target_mode_new, "New script");
    assert_eq!(en.snippets_target_mode_share, "Shared script");
    assert_eq!(en.snippets_split_target, "Split independent");
}

#[test]
fn app_view_model_projects_network_workspace_items() {
    let mut state = desktop_state();
    state.ui.workspace.language = LanguageMode::English;
    let jump_host_id = HostId(Uuid::new_v4());
    let route_host_id = HostId(Uuid::new_v4());
    let session_id = SessionId(Uuid::new_v4());
    let proxy_id = ProxyId(Uuid::new_v4());
    let chain_id = JumpChainId(Uuid::new_v4());
    let forward_id = ForwardId(Uuid::new_v4());

    state.core.storage.upsert_host(Host {
        id: jump_host_id,
        name: "Jump Box".to_owned(),
        group_id: None,
        icon_key: "server".to_owned(),
        tags: vec!["bastion".to_owned()],
        address: "jump.example.com".to_owned(),
        port: 22,
        auth: AuthProfile::Agent {
            username: "ops".to_owned(),
            source: AgentSource::Auto,
            key_hint: None,
        },
        network: HostNetworkSelection::default(),
        proxies: Vec::new(),
        jumps: Vec::new(),
        theme_override: None,
        background_override: None,
    });
    state.core.storage.upsert_host(Host {
        id: route_host_id,
        name: "Prod API".to_owned(),
        group_id: None,
        icon_key: "cloud".to_owned(),
        tags: vec!["prod".to_owned(), "api".to_owned()],
        address: "prod.example.com".to_owned(),
        port: 22,
        auth: AuthProfile::Agent {
            username: "deploy".to_owned(),
            source: AgentSource::Auto,
            key_hint: None,
        },
        network: HostNetworkSelection {
            proxy_ids: vec![proxy_id],
            jump_chain_ids: vec![chain_id],
            forward_ids: vec![forward_id],
        },
        proxies: vec![ProxyProfile::Socks5 {
            host: "127.0.0.1".to_owned(),
            port: 1080,
            auth: crate::model::ProxyAuth::None,
            remote_dns: false,
        }],
        jumps: vec![JumpProfile {
            host_id: jump_host_id,
            username_override: None,
            port_override: None,
            alias: None,
        }],
        theme_override: None,
        background_override: None,
    });
    state.core.storage.upsert_proxy_asset(ProxyAsset {
        id: proxy_id,
        name: "Office Proxy".to_owned(),
        tags: vec!["shared".to_owned()],
        profile: ProxyProfile::Http {
            host: "proxy.example.com".to_owned(),
            port: 8080,
            auth: crate::model::ProxyAuth::None,
        },
    });
    state.core.storage.upsert_jump_chain_asset(JumpChainAsset {
        id: chain_id,
        name: "Prod Chain".to_owned(),
        steps: vec![JumpProfile {
            host_id: jump_host_id,
            username_override: None,
            port_override: None,
            alias: None,
        }],
        stop_on_failure: true,
    });
    state.core.storage.upsert_forward_asset(ForwardAsset {
        id: forward_id,
        name: "DB Forward".to_owned(),
        tags: vec!["db".to_owned()],
        rule: TunnelRule {
            name: "db".to_owned(),
            kind: TunnelKind::Local,
            bind_host: "127.0.0.1".to_owned(),
            bind_port: 15432,
            target_host: "10.0.0.10".to_owned(),
            target_port: 5432,
            auto_start: false,
            exit_on_failure: false,
        },
        exit_on_failure: false,
    });
    state.core.sessions.start_tunnel(
        session_id,
        &crate::model::TunnelRule {
            name: "runtime".to_owned(),
            kind: crate::model::TunnelKind::Dynamic,
            bind_host: "127.0.0.1".to_owned(),
            bind_port: 1080,
            target_host: String::new(),
            target_port: 0,
            auto_start: false,
            exit_on_failure: false,
        },
        Some(route_host_id),
        1,
    );
    state
        .core
        .sessions
        .mark_tunnel_running(session_id, "runtime");

    let vm = app_view_model(&state);

    assert_eq!(vm.network_workspace.runtime_tunnels.len(), 1);
    assert_eq!(vm.network_workspace.runtime_tunnels[0].title, "runtime");
    assert_eq!(vm.network_workspace.proxy_assets.len(), 1);
    assert_eq!(vm.network_workspace.jump_chain_assets.len(), 1);
    assert_eq!(vm.network_workspace.forward_assets.len(), 1);
    assert!(
        vm.network_workspace
            .proxy_assets
            .iter()
            .any(|item| { item.kind_key == "ProxyAsset" && item.title == "Office Proxy" })
    );
    assert!(
        vm.network_workspace
            .jump_chain_assets
            .iter()
            .any(|item| { item.kind_key == "JumpChainAsset" && item.title == "Prod Chain" })
    );
    assert!(
        vm.network_workspace
            .forward_assets
            .iter()
            .any(|item| { item.kind_key == "ForwardAsset" && item.title == "DB Forward" })
    );
    assert!(vm.network_workspace.runtime_tunnels.iter().any(|item| {
        item.kind_key == "TunnelRuntime"
            && item.primary_action_key == "stop"
            && item.primary_action_enabled
    }));

    state.ui.workspace.set_network_search_query("jump");
    let filtered = app_view_model(&state);
    assert_eq!(filtered.network_workspace.search_query, "jump");
    assert!(filtered.network_workspace.proxy_assets.is_empty());
    assert_eq!(filtered.network_workspace.jump_chain_assets.len(), 1);
    assert!(filtered.network_workspace.forward_assets.is_empty());
    assert_eq!(
        filtered.network_workspace.jump_chain_assets[0].title,
        "Prod Chain"
    );
}

#[test]
fn app_view_model_projects_settings_options_and_storage_summary() {
    let mut state = desktop_state();
    state.ui.workspace.language = LanguageMode::Chinese;
    state.apply_message(Message::SetBuiltInTheme {
        theme: crate::model::BuiltInTheme::Dracula,
    });
    state.core.storage.upsert_theme(ThemeProfileRecord {
        name: "Imported".to_owned(),
        profile_toml: "name = \"Imported\"".to_owned(),
        builtin: false,
    });
    state.core.storage.upsert_host(Host {
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
        network: Default::default(),
        proxies: Vec::new(),
        jumps: Vec::new(),
        theme_override: None,
        background_override: None,
    });

    let settings = app_view_model(&state).settings_workspace.settings;

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
    assert_eq!(
        settings.security.detail_label,
        "当前明文保存，已预留主密码加密配置"
    );
    assert_eq!(settings.security.kdf_label, "未配置");
    assert_eq!(settings.security.encryption_version_label, "无");
}

#[test]
fn app_view_model_marks_current_custom_theme_profile_selected() {
    let mut state = desktop_state();
    state.ui.workspace.language = LanguageMode::Chinese;
    state.core.config.theme.name = "Imported".to_owned();
    state.core.storage.app_config = state.core.config.clone();
    state.core.storage.upsert_theme(ThemeProfileRecord {
        name: "Imported".to_owned(),
        profile_toml: "name = \"Imported\"".to_owned(),
        builtin: false,
    });

    let settings = app_view_model(&state).settings_workspace.settings;

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
    let mut state = desktop_state_with_core(
        CoreState::default().with_storage_backend(SqliteStorage::new(&sqlite_path)),
    );
    state.ui.workspace.language = LanguageMode::English;

    let settings = app_view_model(&state).settings_workspace.settings;

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
    let mut state = desktop_state();

    state.ui.workspace.language = LanguageMode::Chinese;
    assert_eq!(app_view_model(&state).theme_name, "专业暗色");

    state.ui.workspace.language = LanguageMode::English;
    assert_eq!(app_view_model(&state).theme_name, "Professional Dark");
}

#[test]
fn app_view_model_projects_pending_remove_host_dialog() {
    let mut state = desktop_state();
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
        network: Default::default(),
        proxies: Vec::new(),
        jumps: Vec::new(),
        theme_override: None,
        background_override: None,
    };
    let host_id = host.id;
    state.core.storage.upsert_host(host);
    state.apply_message(Message::RequestRemoveHost { host_id });

    let vm = app_view_model(&state);

    assert!(vm.remove_host_dialog_open);
    assert_eq!(vm.remove_host_dialog_name, "prod");
}

#[test]
fn app_view_model_keeps_logic_keys_stable_when_text_is_chinese() {
    let mut state = desktop_state();
    state.ui.workspace.language = LanguageMode::Chinese;

    let vm = app_view_model(&state);

    assert_eq!(vm.active_page_key, "Hosts");
    assert_eq!(vm.active_page, "主机");
    assert_eq!(vm.terminal_workspace.tool_panel_mode_key, "Closed");
    assert_eq!(vm.terminal_workspace.tool_panel_mode, "关闭");
}

#[test]
fn app_view_model_localizes_connected_status_but_keeps_status_key_stable() {
    let mut state = desktop_state();
    state.ui.workspace.language = LanguageMode::Chinese;
    let host_id = HostId(Uuid::new_v4());
    let session_id = crate::model::SessionId(Uuid::new_v4());
    state.core.storage.upsert_host(Host {
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
        network: Default::default(),
        proxies: Vec::new(),
        jumps: Vec::new(),
        theme_override: None,
        background_override: None,
    });
    state
        .core
        .sessions
        .open_shell_tab(session_id, host_id, "prod");
    state
        .core
        .sessions
        .set_status(session_id, crate::model::SessionStatus::Connected);

    let vm = app_view_model(&state);

    assert_eq!(vm.terminal_workspace.tabs[0].status_key, "Connected");
    assert_eq!(vm.terminal_workspace.tabs[0].status, "已连接");
    assert_eq!(vm.hosts[0].status_key, "Connected");
    assert_eq!(vm.hosts[0].status, "已连接");
}

#[test]
fn app_view_model_keeps_sftp_panel_on_active_host_without_browser() {
    let mut state = desktop_state();
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
        network: Default::default(),
        proxies: Vec::new(),
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
        network: Default::default(),
        proxies: Vec::new(),
        jumps: Vec::new(),
        theme_override: None,
        background_override: None,
    };
    let sftp_host_id = sftp_host.id;
    let shell_host_id = shell_host.id;
    state.core.storage.upsert_host(sftp_host);
    state.core.storage.upsert_host(shell_host);
    state.core.sessions.open_sftp_tab(
        crate::model::SessionId(Uuid::new_v4()),
        sftp_host_id,
        "/var/log",
    );
    state.core.sessions.open_shell_tab(
        crate::model::SessionId(Uuid::new_v4()),
        shell_host_id,
        "shell",
    );

    let vm = app_view_model(&state);

    assert_eq!(
        vm.terminal_workspace.sftp.host_id,
        shell_host_id.0.to_string()
    );
    assert_eq!(vm.terminal_workspace.sftp.title, "SFTP · shell");
    assert_eq!(vm.terminal_workspace.sftp.current_dir, "/");
    assert!(vm.terminal_workspace.sftp.entries.is_empty());
}
