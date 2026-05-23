use super::super::super::{CachedSessionResources, take_cached_session_runtime_resources};
use super::super::common::{TestTunnel, connections, session_id, sftps, shells, tunnels};

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
