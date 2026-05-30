use super::*;
use crate::model::{AgentSource, AuthProfile, GroupId, Host, HostGroup, HostId, LanguageMode};
use uuid::Uuid;

fn agent_host(name: &str, address: &str, tags: &[&str]) -> Host {
    Host {
        id: HostId(Uuid::new_v4()),
        name: name.to_owned(),
        group_id: None,
        icon_key: "server".to_owned(),
        tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
        address: address.to_owned(),
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
fn host_rows_do_not_expose_auth_secrets() {
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
            secret: crate::model::SecretRef("password:root".to_owned()),
        },
        proxy: None,
        jumps: Vec::new(),
        theme_override: None,
        background_override: None,
    });

    let rows = hosts(&state);

    assert_eq!(rows[0].auth, "Password");
    assert!(!rows[0].endpoint.contains("password"));
}

#[test]
fn host_rows_project_icon_key() {
    let mut state = AppState::default();
    let mut host = agent_host("database", "db.example.com", &["prod"]);
    host.icon_key = "database".to_owned();
    state.storage.upsert_host(host);

    let rows = hosts(&state);

    assert_eq!(rows[0].icon_key, "database");
}

#[test]
fn host_rows_follow_search_query() {
    let mut state = AppState::default();
    state
        .storage
        .upsert_host(agent_host("Production", "prod.example.com", &["prod"]));
    state
        .storage
        .upsert_host(agent_host("Staging", "staging.example.com", &["stage"]));
    state.ui.workspace.set_host_search_query("prod");

    let rows = hosts(&state);

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "Production");
}

#[test]
fn host_tree_projects_empty_and_nested_groups() {
    let mut state = AppState::default();
    let parent_id = GroupId(Uuid::new_v4());
    let empty_child_id = GroupId(Uuid::new_v4());
    let host_child_id = GroupId(Uuid::new_v4());
    state.storage.upsert_group(HostGroup {
        id: parent_id,
        name: "生产".to_owned(),
        parent_id: None,
    });
    state.storage.upsert_group(HostGroup {
        id: empty_child_id,
        name: "空文件夹".to_owned(),
        parent_id: Some(parent_id),
    });
    state.storage.upsert_group(HostGroup {
        id: host_child_id,
        name: "华东".to_owned(),
        parent_id: Some(parent_id),
    });
    let mut host = agent_host("API", "api.example.com", &["prod"]);
    host.group_id = Some(host_child_id);
    state.storage.upsert_host(host);

    let rows = host_tree(&state);

    let root = rows
        .iter()
        .find(|row| row.kind == "Root")
        .expect("root group should be projected");
    assert_eq!(root.name, "所有主机和分组");
    assert_eq!(root.depth, 0);
    let parent = rows
        .iter()
        .find(|row| row.group_id == parent_id.0.to_string())
        .expect("parent group should be projected");
    assert_eq!(parent.kind, "Group");
    assert_eq!(parent.depth, 1);
    let empty_child = rows
        .iter()
        .find(|row| row.group_id == empty_child_id.0.to_string())
        .expect("empty child group should be projected");
    assert_eq!(empty_child.kind, "Group");
    assert_eq!(empty_child.depth, 2);
    let host_row = rows
        .iter()
        .find(|row| row.name == "API")
        .expect("nested host should be projected");
    assert_eq!(host_row.kind, "Host");
    assert_eq!(host_row.depth, 3);
}

#[test]
fn host_tree_marks_ancestor_guides_for_later_siblings() {
    let mut state = AppState::default();
    let first_id = GroupId(Uuid::new_v4());
    let first_child_id = GroupId(Uuid::new_v4());
    let second_id = GroupId(Uuid::new_v4());
    state.storage.upsert_group(HostGroup {
        id: first_id,
        name: "aaa".to_owned(),
        parent_id: None,
    });
    state.storage.upsert_group(HostGroup {
        id: first_child_id,
        name: "aa".to_owned(),
        parent_id: Some(first_id),
    });
    state.storage.upsert_group(HostGroup {
        id: second_id,
        name: "bb".to_owned(),
        parent_id: None,
    });

    let rows = host_tree(&state);

    let first = rows
        .iter()
        .find(|row| row.group_id == first_id.0.to_string())
        .expect("first top-level group should be visible");
    assert!(first.has_next_sibling);
    let first_child = rows
        .iter()
        .find(|row| row.group_id == first_child_id.0.to_string())
        .expect("nested child group should be visible");
    assert!(first_child.guide_1);
    assert!(!first_child.has_next_sibling);
    let second = rows
        .iter()
        .find(|row| row.group_id == second_id.0.to_string())
        .expect("second top-level group should be visible");
    assert!(!second.has_next_sibling);
}

#[test]
fn host_tree_hides_descendants_of_collapsed_group() {
    let mut state = AppState::default();
    let parent_id = GroupId(Uuid::new_v4());
    let child_id = GroupId(Uuid::new_v4());
    state.storage.upsert_group(HostGroup {
        id: parent_id,
        name: "生产".to_owned(),
        parent_id: None,
    });
    state.storage.upsert_group(HostGroup {
        id: child_id,
        name: "华东".to_owned(),
        parent_id: Some(parent_id),
    });
    state.ui.workspace.toggle_host_tree_group(Some(parent_id));

    let rows = host_tree(&state);

    assert!(
        rows.iter()
            .any(|row| row.group_id == parent_id.0.to_string())
    );
    assert!(
        !rows
            .iter()
            .any(|row| row.group_id == child_id.0.to_string())
    );
}

#[test]
fn host_tree_root_can_collapse_all_children() {
    let mut state = AppState::default();
    let group_id = GroupId(Uuid::new_v4());
    state.storage.upsert_group(HostGroup {
        id: group_id,
        name: "生产".to_owned(),
        parent_id: None,
    });
    state.ui.workspace.toggle_host_tree_group(None);

    let rows = host_tree(&state);

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].kind, "Root");
    assert!(!rows[0].expanded);
}

#[test]
fn create_group_dialog_projects_parent_options() {
    let mut state = AppState::default();
    state.ui.workspace.language = LanguageMode::Chinese;
    let parent_id = GroupId(Uuid::new_v4());
    let child_id = GroupId(Uuid::new_v4());
    state.storage.upsert_group(HostGroup {
        id: parent_id,
        name: "生产".to_owned(),
        parent_id: None,
    });
    state.storage.upsert_group(HostGroup {
        id: child_id,
        name: "华东".to_owned(),
        parent_id: Some(parent_id),
    });
    state.ui.quick_group.parent_id = Some(child_id);

    let vm = create_group_dialog(&state);

    assert_eq!(vm.parent_path, "生产 / 华东");
    assert_eq!(vm.parent_options[0].path, "未分组");
    let child = vm
        .parent_options
        .iter()
        .find(|group| group.id == child_id.0.to_string())
        .expect("child group option should be projected");
    assert!(child.selected);
}

#[test]
fn host_rows_use_stable_accent_mapping() {
    let mut state = AppState::default();
    let host = agent_host("Production", "prod.example.com", &["prod"]);
    state.storage.upsert_host(host.clone());

    let rows = hosts(&state);
    let accent = rows[0].accent_index;

    assert!((0..=4).contains(&accent));
    assert_eq!(accent, hosts(&state)[0].accent_index);
}

#[test]
fn quick_host_projects_group_path_options_and_selection() {
    let mut state = AppState::default();
    state.ui.workspace.language = LanguageMode::Chinese;
    let parent_id = GroupId(Uuid::new_v4());
    let child_id = GroupId(Uuid::new_v4());
    state.storage.upsert_group(HostGroup {
        id: child_id,
        name: "华东".to_owned(),
        parent_id: Some(parent_id),
    });
    state.storage.upsert_group(HostGroup {
        id: parent_id,
        name: "生产".to_owned(),
        parent_id: None,
    });
    state.ui.select_quick_host_group(Some(child_id));

    let vm = quick_host(&state);

    assert_eq!(vm.group_path, "生产 / 华东");
    assert_eq!(vm.group_options[0].path, "未分组");
    assert!(!vm.group_options[0].selected);
    let parent = vm
        .group_options
        .iter()
        .find(|group| group.id == parent_id.0.to_string())
        .expect("parent group option should be projected");
    assert_eq!(parent.path, "生产");
    assert_eq!(parent.depth, 0);
    assert!(!parent.selected);
    let child = vm
        .group_options
        .iter()
        .find(|group| group.id == child_id.0.to_string())
        .expect("child group option should be projected");
    assert_eq!(child.name, "华东");
    assert_eq!(child.path, "生产 / 华东");
    assert_eq!(child.depth, 1);
    assert!(child.selected);
}

#[test]
fn new_session_hosts_hide_empty_tags_after_localization() {
    let mut state = AppState::default();
    state.ui.workspace.language = LanguageMode::Chinese;
    state
        .storage
        .upsert_host(agent_host("Production", "prod.example.com", &[]));

    let host = new_session_hosts(&state).remove(0);

    assert_eq!(hosts(&state)[0].tags, "未标记");
    assert_eq!(host.tags, "");
}

#[test]
fn new_session_hosts_compact_real_tags_without_locale_sentinel() {
    let mut state = AppState::default();
    state.ui.workspace.language = LanguageMode::Chinese;
    state.storage.upsert_host(agent_host(
        "Production",
        "prod.example.com",
        &["prod", "api", "east"],
    ));

    let host = new_session_hosts(&state).remove(0);

    assert_eq!(host.tags, "prod +2");
}
