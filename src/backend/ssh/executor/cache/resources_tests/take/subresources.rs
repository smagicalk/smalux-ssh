use super::super::super::{CachedSessionSubresources, take_cached_session_subresources};
use super::super::common::{session_id, sftps, shells};

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
