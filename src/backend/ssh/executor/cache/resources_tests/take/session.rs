use super::super::super::{CachedSessionResources, take_cached_session_resources};
use super::super::common::{connections, session_id, sftps, shells};

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
