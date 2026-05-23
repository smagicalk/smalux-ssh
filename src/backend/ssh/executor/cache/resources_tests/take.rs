use super::super::{
    CachedSessionResources, CachedSessionSubresources, take_cached_session_resources,
    take_cached_session_runtime_resources, take_cached_session_subresources,
};
use super::common::{TestTunnel, connections, session_id, sftps, shells, tunnels};

#[test]
fn taking_cached_session_resources_removes_only_target_session() {
    let target_session_id = session_id();
    let other_session_id = session_id();
    let mut shell_map = shells([
        (target_session_id, "target-shell"),
        (other_session_id, "other-shell"),
    ]);
    let mut sftp_map = sftps([
        (target_session_id, "target-sftp"),
        (other_session_id, "other-sftp"),
    ]);
    let mut connection_map = connections([
        (target_session_id, "target-connection"),
        (other_session_id, "other-connection"),
    ]);

    let resources = take_cached_session_resources(
        &mut shell_map,
        &mut sftp_map,
        &mut connection_map,
        target_session_id,
    );

    assert_eq!(
        resources,
        CachedSessionResources {
            shell: Some("target-shell"),
            sftp: Some("target-sftp"),
            connection: Some("target-connection"),
        }
    );
    assert!(!shell_map.contains_key(&target_session_id));
    assert!(!sftp_map.contains_key(&target_session_id));
    assert!(!connection_map.contains_key(&target_session_id));
    assert_eq!(shell_map.get(&other_session_id), Some(&"other-shell"));
    assert_eq!(sftp_map.get(&other_session_id), Some(&"other-sftp"));
    assert_eq!(
        connection_map.get(&other_session_id),
        Some(&"other-connection")
    );
}

#[test]
fn taking_cached_session_resources_is_idempotent_for_missing_session() {
    let missing_session_id = session_id();
    let other_session_id = session_id();
    let mut shell_map = shells([(other_session_id, "other-shell")]);
    let mut sftp_map = sftps([(other_session_id, "other-sftp")]);
    let mut connection_map = connections([(other_session_id, "other-connection")]);

    let resources = take_cached_session_resources(
        &mut shell_map,
        &mut sftp_map,
        &mut connection_map,
        missing_session_id,
    );

    assert_eq!(
        resources,
        CachedSessionResources {
            shell: None::<&str>,
            sftp: None::<&str>,
            connection: None::<&str>,
        }
    );
    assert_eq!(shell_map.get(&other_session_id), Some(&"other-shell"));
    assert_eq!(sftp_map.get(&other_session_id), Some(&"other-sftp"));
    assert_eq!(
        connection_map.get(&other_session_id),
        Some(&"other-connection")
    );
}

#[test]
fn taking_cached_session_resources_detaches_all_target_resources_before_close() {
    let session_id = session_id();
    let mut shell_map = shells([(session_id, "shell")]);
    let mut sftp_map = sftps([(session_id, "sftp")]);
    let mut connection_map = connections([(session_id, "connection")]);

    let resources = take_cached_session_resources(
        &mut shell_map,
        &mut sftp_map,
        &mut connection_map,
        session_id,
    );

    assert_eq!(
        resources,
        CachedSessionResources {
            shell: Some("shell"),
            sftp: Some("sftp"),
            connection: Some("connection"),
        }
    );
    assert!(shell_map.is_empty());
    assert!(sftp_map.is_empty());
    assert!(connection_map.is_empty());
}

#[test]
fn taking_cached_session_runtime_resources_detaches_owned_tunnels() {
    let target_session_id = session_id();
    let other_session_id = session_id();
    let mut shell_map = shells([(target_session_id, "target-shell")]);
    let mut sftp_map = sftps([(target_session_id, "target-sftp")]);
    let mut connection_map = connections([(target_session_id, "target-connection")]);
    let mut tunnel_map = tunnels([
        (
            "proxy".to_owned(),
            TestTunnel {
                session_id: target_session_id,
                rule_name: "proxy".to_owned(),
            },
        ),
        (
            "metrics".to_owned(),
            TestTunnel {
                session_id: other_session_id,
                rule_name: "metrics".to_owned(),
            },
        ),
    ]);

    let resources = take_cached_session_runtime_resources(
        &mut shell_map,
        &mut sftp_map,
        &mut connection_map,
        &mut tunnel_map,
        target_session_id,
    );

    assert_eq!(
        resources.cached_resources,
        CachedSessionResources {
            shell: Some("target-shell"),
            sftp: Some("target-sftp"),
            connection: Some("target-connection"),
        }
    );
    assert_eq!(resources.tunnels.len(), 1);
    assert_eq!(resources.tunnels[0].rule_name, "proxy");
    assert!(shell_map.is_empty());
    assert!(sftp_map.is_empty());
    assert!(connection_map.is_empty());
    assert!(!tunnel_map.contains_key("proxy"));
    assert_eq!(
        tunnel_map.get("metrics"),
        Some(&TestTunnel {
            session_id: other_session_id,
            rule_name: "metrics".to_owned(),
        })
    );
}

#[test]
fn taking_cached_session_runtime_resources_is_idempotent_for_missing_session() {
    let missing_session_id = session_id();
    let other_session_id = session_id();
    let mut shell_map = shells([(other_session_id, "other-shell")]);
    let mut sftp_map = sftps([(other_session_id, "other-sftp")]);
    let mut connection_map = connections([(other_session_id, "other-connection")]);
    let mut tunnel_map = tunnels([(
        "metrics".to_owned(),
        TestTunnel {
            session_id: other_session_id,
            rule_name: "metrics".to_owned(),
        },
    )]);

    let resources = take_cached_session_runtime_resources(
        &mut shell_map,
        &mut sftp_map,
        &mut connection_map,
        &mut tunnel_map,
        missing_session_id,
    );

    assert_eq!(
        resources.cached_resources,
        CachedSessionResources {
            shell: None::<&str>,
            sftp: None::<&str>,
            connection: None::<&str>,
        }
    );
    assert!(resources.tunnels.is_empty());
    assert_eq!(shell_map.get(&other_session_id), Some(&"other-shell"));
    assert_eq!(sftp_map.get(&other_session_id), Some(&"other-sftp"));
    assert_eq!(
        connection_map.get(&other_session_id),
        Some(&"other-connection")
    );
    assert_eq!(
        tunnel_map.get("metrics"),
        Some(&TestTunnel {
            session_id: other_session_id,
            rule_name: "metrics".to_owned(),
        })
    );
}

#[test]
fn taking_cached_session_subresources_removes_only_target_session() {
    let target_session_id = session_id();
    let other_session_id = session_id();
    let mut shell_map = shells([
        (target_session_id, "target-shell"),
        (other_session_id, "other-shell"),
    ]);
    let mut sftp_map = sftps([
        (target_session_id, "target-sftp"),
        (other_session_id, "other-sftp"),
    ]);

    let resources =
        take_cached_session_subresources(&mut shell_map, &mut sftp_map, target_session_id);

    assert_eq!(
        resources,
        CachedSessionSubresources {
            shell: Some("target-shell"),
            sftp: Some("target-sftp"),
        }
    );
    assert!(!shell_map.contains_key(&target_session_id));
    assert!(!sftp_map.contains_key(&target_session_id));
    assert_eq!(shell_map.get(&other_session_id), Some(&"other-shell"));
    assert_eq!(sftp_map.get(&other_session_id), Some(&"other-sftp"));
}

#[test]
fn taking_cached_session_subresources_is_idempotent_for_missing_session() {
    let missing_session_id = session_id();
    let other_session_id = session_id();
    let mut shell_map = shells([(other_session_id, "other-shell")]);
    let mut sftp_map = sftps([(other_session_id, "other-sftp")]);

    let resources =
        take_cached_session_subresources(&mut shell_map, &mut sftp_map, missing_session_id);

    assert_eq!(
        resources,
        CachedSessionSubresources {
            shell: None::<&str>,
            sftp: None::<&str>,
        }
    );
    assert_eq!(shell_map.get(&other_session_id), Some(&"other-shell"));
    assert_eq!(sftp_map.get(&other_session_id), Some(&"other-sftp"));
}
