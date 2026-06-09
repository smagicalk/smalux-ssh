use super::*;

#[test]
fn update_snippet_argument_reports_unknown_variable() {
    let mut state = AppState::default();
    let snippet_id = SnippetId(Uuid::new_v4());

    let outcome = state.apply(Message::UpdateSnippetArgument {
        snippet_id,
        name: "service".to_owned(),
        value: "nginx".to_owned(),
    });

    assert!(outcome.changed());
    assert!(outcome.error.is_some());
}

#[test]
fn remove_snippet_deletes_existing_snippet_and_reports_missing() {
    let mut state = AppState::default();
    let host_id = HostId(Uuid::new_v4());
    let snippet = host_snippet(host_id, "uptime");
    let snippet_id = snippet.id;
    state.storage.upsert_snippet(snippet);

    let removed = state.apply(Message::RemoveSnippet { snippet_id });
    let missing = state.apply(Message::RemoveSnippet { snippet_id });

    assert!(removed.changed());
    assert_eq!(state.storage.snippet_count(), 0);
    assert!(missing.changed());
    assert!(missing.error.is_some());
}

#[test]
fn create_snippet_extracts_variables_and_assigns_group() {
    let mut state = AppState::default();
    state.apply(Message::CreateSnippetGroup {
        name: "服务".to_owned(),
        parent_id: None,
    });
    let group_id = state.storage.snippet_groups[0].id;

    let outcome = state.apply(Message::CreateSnippet {
        name: "  重启服务  ".to_owned(),
        description: "  systemd restart  ".to_owned(),
        command_template: "systemctl restart {{service}}".to_owned(),
        scope: SnippetScope::Global,
        group_id: Some(group_id),
    });

    assert!(outcome.changed());
    assert_eq!(state.storage.snippet_count(), 1);
    assert_eq!(state.storage.snippets[0].name, "重启服务");
    assert_eq!(
        state.storage.snippets[0].description.as_deref(),
        Some("systemd restart")
    );
    assert_eq!(state.storage.snippets[0].group_id, Some(group_id));
    assert_eq!(state.storage.snippets[0].variables[0].name, "service");
}

#[test]
fn update_snippet_refreshes_variables_and_keeps_matching_arguments() {
    let mut state = AppState::default();
    let mut snippet = Snippet::with_default_implementation(
        SnippetId(Uuid::new_v4()),
        "restart".to_owned(),
        None,
        SnippetScope::Global,
        None,
        "systemctl restart {{service}} --env {{env}}".to_owned(),
    );
    snippet
        .default_implementation_mut()
        .expect("默认实现应存在")
        .last_arguments = vec![
        SnippetArgument {
            name: "service".to_owned(),
            value: "nginx".to_owned(),
        },
        SnippetArgument {
            name: "env".to_owned(),
            value: "prod".to_owned(),
        },
    ];
    let snippet_id = snippet.id;
    state.storage.upsert_snippet(snippet);

    let outcome = state.apply(Message::UpdateSnippet {
        snippet_id,
        name: String::new(),
        description: String::new(),
        command_template: "systemctl status {{service}}".to_owned(),
        scope: SnippetScope::Global,
        group_id: None,
    });

    assert!(outcome.changed());
    assert_eq!(
        state.storage.snippets[0].name,
        "systemctl status {{service}}"
    );
    assert_eq!(state.storage.snippets[0].description, None);
    assert_eq!(state.storage.snippets[0].variables.len(), 1);
    assert_eq!(
        state.storage.snippets[0]
            .default_implementation()
            .expect("默认实现应存在")
            .last_arguments,
        vec![SnippetArgument {
            name: "service".to_owned(),
            value: "nginx".to_owned(),
        }]
    );
}

#[test]
fn snippet_group_messages_create_move_and_remove_empty_groups() {
    let mut state = AppState::default();

    let created = state.apply(Message::CreateSnippetGroup {
        name: "  运维  ".to_owned(),
        parent_id: None,
    });
    let group_id = state.storage.snippet_groups[0].id;
    let renamed = state.apply(Message::RenameSnippetGroup {
        group_id,
        name: "生产".to_owned(),
    });
    let removed = state.apply(Message::RemoveSnippetGroup { group_id });

    assert!(created.changed());
    assert!(renamed.changed());
    assert!(removed.changed());
    assert_eq!(state.storage.snippet_group_count(), 0);
}

#[test]
fn snippet_group_delete_rejects_non_empty_group_and_move_snippet_works() {
    let mut state = AppState::default();
    state.apply(Message::CreateSnippetGroup {
        name: "服务".to_owned(),
        parent_id: None,
    });
    let group_id = state.storage.snippet_groups[0].id;
    let snippet = host_snippet(HostId(Uuid::new_v4()), "uptime");
    let snippet_id = snippet.id;
    state.storage.upsert_snippet(snippet);

    let moved = state.apply(Message::MoveSnippet {
        snippet_id,
        group_id: Some(group_id),
    });
    let rejected = state.apply(Message::RemoveSnippetGroup { group_id });

    assert!(moved.changed());
    assert_eq!(state.storage.snippets[0].group_id, Some(group_id));
    assert!(rejected.changed());
    assert!(rejected.error.is_some());
    assert_eq!(state.storage.snippet_group_count(), 1);
}

#[test]
fn snippet_group_move_rejects_cycles_and_allows_moving_back_to_root() {
    let mut state = AppState::default();
    state.apply(Message::CreateSnippetGroup {
        name: "运维".to_owned(),
        parent_id: None,
    });
    let parent_id = state.storage.snippet_groups[0].id;
    state.apply(Message::CreateSnippetGroup {
        name: "服务".to_owned(),
        parent_id: Some(parent_id),
    });
    let child_id = state.storage.snippet_groups[1].id;

    let cycle = state.apply(Message::MoveSnippetGroup {
        group_id: parent_id,
        parent_id: Some(child_id),
    });

    assert!(cycle.changed());
    assert!(cycle.error.is_some());
    assert_eq!(state.storage.snippet_groups[0].parent_id, None);
    assert_eq!(state.storage.snippet_groups[1].parent_id, Some(parent_id));

    let root = state.apply(Message::MoveSnippetGroup {
        group_id: child_id,
        parent_id: None,
    });

    assert!(root.changed());
    assert_eq!(state.storage.snippet_groups[1].parent_id, None);
}

#[test]
fn snippet_move_can_return_to_root_group() {
    let mut state = AppState::default();
    state.apply(Message::CreateSnippetGroup {
        name: "服务".to_owned(),
        parent_id: None,
    });
    let group_id = state.storage.snippet_groups[0].id;
    let snippet = host_snippet(HostId(Uuid::new_v4()), "uptime");
    let snippet_id = snippet.id;
    state.storage.upsert_snippet(snippet);

    let moved_into_group = state.apply(Message::MoveSnippet {
        snippet_id,
        group_id: Some(group_id),
    });
    let moved_to_root = state.apply(Message::MoveSnippet {
        snippet_id,
        group_id: None,
    });

    assert!(moved_into_group.changed());
    assert!(moved_to_root.changed());
    assert_eq!(state.storage.snippets[0].group_id, None);
}

#[test]
fn snippet_group_recursive_delete_removes_children_and_snippets() {
    let mut state = AppState::default();
    state.apply(Message::CreateSnippetGroup {
        name: "运维".to_owned(),
        parent_id: None,
    });
    let parent_id = state.storage.snippet_groups[0].id;
    state.apply(Message::CreateSnippetGroup {
        name: "服务".to_owned(),
        parent_id: Some(parent_id),
    });
    let child_id = state.storage.snippet_groups[1].id;
    let mut snippet = Snippet::with_default_implementation(
        SnippetId(Uuid::new_v4()),
        "restart".to_owned(),
        None,
        SnippetScope::Global,
        Some(child_id),
        "systemctl restart nginx".to_owned(),
    );
    snippet.group_id = Some(child_id);
    state.storage.upsert_snippet(snippet);

    let outcome = state.apply(Message::RemoveSnippetGroupRecursive {
        group_id: parent_id,
    });

    assert!(outcome.changed());
    assert_eq!(state.storage.snippet_group_count(), 0);
    assert_eq!(state.storage.snippet_count(), 0);
}

#[test]
fn snippet_target_messages_create_share_update_split_and_remove_targets() {
    let mut state = AppState::default();
    let host_id = HostId(Uuid::new_v4());
    let snippet = parameterized_host_snippet(host_id, "ls {{path}}");
    let snippet_id = snippet.id;
    let linux_target_id = snippet.support_targets[0].id;
    state.storage.upsert_snippet(snippet);

    let created_windows = state.apply(Message::CreateSnippetTarget {
        snippet_id,
        target_keys: vec!["Windows".to_owned()],
        display_name: "Windows".to_owned(),
        command_template: "dir {{path}}".to_owned(),
        share_target_id: None,
    });
    assert!(created_windows.changed());
    assert_eq!(state.storage.snippets[0].support_targets.len(), 2);
    assert_eq!(state.storage.snippets[0].implementations.len(), 2);
    assert_eq!(
        state.storage.snippets[0].support_targets[1].target_key,
        "windows"
    );

    let created_multi = state.apply(Message::CreateSnippetTarget {
        snippet_id,
        target_keys: vec![
            "debian-ubuntu".to_owned(),
            "rhel-centos".to_owned(),
            "debian-ubuntu".to_owned(),
        ],
        display_name: String::new(),
        command_template: String::new(),
        share_target_id: Some(linux_target_id),
    });
    assert!(created_multi.changed());
    assert_eq!(state.storage.snippets[0].support_targets.len(), 4);
    assert!(
        state.storage.snippets[0]
            .support_targets
            .iter()
            .any(|target| target.target_key == "debian-ubuntu")
    );
    assert!(
        state.storage.snippets[0]
            .support_targets
            .iter()
            .any(|target| target.target_key == "rhel-centos")
    );

    let created_debian = state.apply(Message::CreateSnippetTarget {
        snippet_id,
        target_keys: vec!["debian".to_owned()],
        display_name: "Debian".to_owned(),
        command_template: String::new(),
        share_target_id: Some(linux_target_id),
    });
    assert!(created_debian.changed());
    assert_eq!(state.storage.snippets[0].support_targets.len(), 5);
    assert_eq!(state.storage.snippets[0].implementations.len(), 2);
    let debian_target_id = state.storage.snippets[0]
        .support_targets
        .iter()
        .find(|target| target.target_key == "debian")
        .expect("Debian 支持目标应存在")
        .id;

    let updated_debian = state.apply(Message::UpdateSnippetTarget {
        snippet_id,
        target_id: debian_target_id,
        target_key: "debian".to_owned(),
        display_name: "Debian".to_owned(),
        command_template: "apt list {{package}}".to_owned(),
    });
    assert!(updated_debian.changed());
    let linux_implementation_id = state.storage.snippets[0].support_targets[0].implementation_id;
    let linux_implementation = state.storage.snippets[0]
        .implementations
        .iter()
        .find(|implementation| implementation.id == linux_implementation_id)
        .expect("共享实现应存在");
    assert_eq!(
        linux_implementation.command_template,
        "apt list {{package}}"
    );
    assert!(
        state.storage.snippets[0]
            .variables
            .iter()
            .any(|variable| variable.name == "package")
    );

    let split = state.apply(Message::SplitSnippetTargetImplementation {
        snippet_id,
        target_id: debian_target_id,
    });
    assert!(split.changed());
    assert_eq!(state.storage.snippets[0].implementations.len(), 3);
    let debian_implementation_id = state.storage.snippets[0]
        .support_targets
        .iter()
        .find(|target| target.id == debian_target_id)
        .expect("Debian 支持目标应存在")
        .implementation_id;
    assert_ne!(debian_implementation_id, linux_implementation_id);

    let updated_debian_after_split = state.apply(Message::UpdateSnippetTarget {
        snippet_id,
        target_id: debian_target_id,
        target_key: "debian".to_owned(),
        display_name: "Debian".to_owned(),
        command_template: "dpkg -l {{package}}".to_owned(),
    });
    assert!(updated_debian_after_split.changed());
    let linux_implementation = state.storage.snippets[0]
        .implementations
        .iter()
        .find(|implementation| implementation.id == linux_implementation_id)
        .expect("Linux 实现应存在");
    let debian_implementation = state.storage.snippets[0]
        .implementations
        .iter()
        .find(|implementation| implementation.id == debian_implementation_id)
        .expect("Debian 实现应存在");
    assert_eq!(
        linux_implementation.command_template,
        "apt list {{package}}"
    );
    assert_eq!(
        debian_implementation.command_template,
        "dpkg -l {{package}}"
    );

    let removed = state.apply(Message::RemoveSnippetTarget {
        snippet_id,
        target_id: debian_target_id,
    });
    assert!(removed.changed());
    assert_eq!(state.storage.snippets[0].support_targets.len(), 4);
    assert!(
        !state.storage.snippets[0]
            .implementations
            .iter()
            .any(|implementation| implementation.id == debian_implementation_id)
    );
}
