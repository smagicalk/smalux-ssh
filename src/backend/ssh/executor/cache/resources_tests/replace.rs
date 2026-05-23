use super::super::{replace_cached_sftp, replace_cached_shell};
use super::common::{session_id, sftps, shells};

#[test]
fn replacing_cached_shell_returns_previous_shell_for_same_session() {
    let session_id = session_id();
    let mut shell_map = shells([(session_id, "old-shell")]);

    let previous = replace_cached_shell(&mut shell_map, session_id, "new-shell");

    assert_eq!(previous, Some("old-shell"));
    assert_eq!(shell_map.get(&session_id), Some(&"new-shell"));
}

#[test]
fn replacing_cached_shell_keeps_other_sessions() {
    let target_session_id = session_id();
    let other_session_id = session_id();
    let mut shell_map = shells([(other_session_id, "other-shell")]);

    let previous = replace_cached_shell(&mut shell_map, target_session_id, "target-shell");

    assert_eq!(previous, None);
    assert_eq!(shell_map.get(&target_session_id), Some(&"target-shell"));
    assert_eq!(shell_map.get(&other_session_id), Some(&"other-shell"));
}

#[test]
fn replacing_cached_sftp_returns_previous_sftp_for_same_session() {
    let session_id = session_id();
    let mut sftp_map = sftps([(session_id, "old-sftp")]);

    let previous = replace_cached_sftp(&mut sftp_map, session_id, "new-sftp");

    assert_eq!(previous, Some("old-sftp"));
    assert_eq!(sftp_map.get(&session_id), Some(&"new-sftp"));
}

#[test]
fn replacing_cached_sftp_keeps_other_sessions() {
    let target_session_id = session_id();
    let other_session_id = session_id();
    let mut sftp_map = sftps([(other_session_id, "other-sftp")]);

    let previous = replace_cached_sftp(&mut sftp_map, target_session_id, "target-sftp");

    assert_eq!(previous, None);
    assert_eq!(sftp_map.get(&target_session_id), Some(&"target-sftp"));
    assert_eq!(sftp_map.get(&other_session_id), Some(&"other-sftp"));
}
