use std::collections::HashMap;

use uuid::Uuid;

use super::*;
use crate::backend::ssh::executor::cache::tunnels::{RuleNamedTunnel, StoppableTunnel};

fn session_id() -> SessionId {
    SessionId(Uuid::new_v4())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TestTunnel {
    session_id: SessionId,
    rule_name: String,
}

impl TunnelOwner for TestTunnel {
    fn session_id(&self) -> SessionId {
        self.session_id
    }
}

impl RuleNamedTunnel for TestTunnel {
    fn rule_name(&self) -> &str {
        &self.rule_name
    }
}

impl StoppableTunnel for TestTunnel {
    fn stop(&self) {}
}

#[test]
fn taking_cached_session_resources_removes_only_target_session() {
    let target_session_id = session_id();
    let other_session_id = session_id();
    let mut shells = HashMap::from([
        (target_session_id, "target-shell"),
        (other_session_id, "other-shell"),
    ]);
    let mut sftps = HashMap::from([
        (target_session_id, "target-sftp"),
        (other_session_id, "other-sftp"),
    ]);
    let mut connections = HashMap::from([
        (target_session_id, "target-connection"),
        (other_session_id, "other-connection"),
    ]);

    let resources =
        take_cached_session_resources(&mut shells, &mut sftps, &mut connections, target_session_id);

    assert_eq!(
        resources,
        CachedSessionResources {
            shell: Some("target-shell"),
            sftp: Some("target-sftp"),
            connection: Some("target-connection"),
        }
    );
    assert!(!shells.contains_key(&target_session_id));
    assert!(!sftps.contains_key(&target_session_id));
    assert!(!connections.contains_key(&target_session_id));
    assert_eq!(shells.get(&other_session_id), Some(&"other-shell"));
    assert_eq!(sftps.get(&other_session_id), Some(&"other-sftp"));
    assert_eq!(
        connections.get(&other_session_id),
        Some(&"other-connection")
    );
}

#[test]
fn taking_cached_session_resources_is_idempotent_for_missing_session() {
    let missing_session_id = session_id();
    let other_session_id = session_id();
    let mut shells = HashMap::from([(other_session_id, "other-shell")]);
    let mut sftps = HashMap::from([(other_session_id, "other-sftp")]);
    let mut connections = HashMap::from([(other_session_id, "other-connection")]);

    let resources = take_cached_session_resources(
        &mut shells,
        &mut sftps,
        &mut connections,
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
    assert_eq!(shells.get(&other_session_id), Some(&"other-shell"));
    assert_eq!(sftps.get(&other_session_id), Some(&"other-sftp"));
    assert_eq!(
        connections.get(&other_session_id),
        Some(&"other-connection")
    );
}

#[test]
fn taking_cached_session_resources_detaches_all_target_resources_before_close() {
    let session_id = session_id();
    let mut shells = HashMap::from([(session_id, "shell")]);
    let mut sftps = HashMap::from([(session_id, "sftp")]);
    let mut connections = HashMap::from([(session_id, "connection")]);

    let resources =
        take_cached_session_resources(&mut shells, &mut sftps, &mut connections, session_id);

    assert_eq!(
        resources,
        CachedSessionResources {
            shell: Some("shell"),
            sftp: Some("sftp"),
            connection: Some("connection"),
        }
    );
    assert!(shells.is_empty());
    assert!(sftps.is_empty());
    assert!(connections.is_empty());
}

#[test]
fn taking_cached_session_runtime_resources_detaches_owned_tunnels() {
    let target_session_id = session_id();
    let other_session_id = session_id();
    let mut shells = HashMap::from([(target_session_id, "target-shell")]);
    let mut sftps = HashMap::from([(target_session_id, "target-sftp")]);
    let mut connections = HashMap::from([(target_session_id, "target-connection")]);
    let mut tunnels = HashMap::from([
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
        &mut shells,
        &mut sftps,
        &mut connections,
        &mut tunnels,
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
    assert!(shells.is_empty());
    assert!(sftps.is_empty());
    assert!(connections.is_empty());
    assert!(!tunnels.contains_key("proxy"));
    assert_eq!(
        tunnels.get("metrics"),
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
    let mut shells = HashMap::from([(other_session_id, "other-shell")]);
    let mut sftps = HashMap::from([(other_session_id, "other-sftp")]);
    let mut connections = HashMap::from([(other_session_id, "other-connection")]);
    let mut tunnels = HashMap::from([(
        "metrics".to_owned(),
        TestTunnel {
            session_id: other_session_id,
            rule_name: "metrics".to_owned(),
        },
    )]);

    let resources = take_cached_session_runtime_resources(
        &mut shells,
        &mut sftps,
        &mut connections,
        &mut tunnels,
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
    assert_eq!(shells.get(&other_session_id), Some(&"other-shell"));
    assert_eq!(sftps.get(&other_session_id), Some(&"other-sftp"));
    assert_eq!(
        connections.get(&other_session_id),
        Some(&"other-connection")
    );
    assert_eq!(
        tunnels.get("metrics"),
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
    let mut shells = HashMap::from([
        (target_session_id, "target-shell"),
        (other_session_id, "other-shell"),
    ]);
    let mut sftps = HashMap::from([
        (target_session_id, "target-sftp"),
        (other_session_id, "other-sftp"),
    ]);

    let resources = take_cached_session_subresources(&mut shells, &mut sftps, target_session_id);

    assert_eq!(
        resources,
        CachedSessionSubresources {
            shell: Some("target-shell"),
            sftp: Some("target-sftp"),
        }
    );
    assert!(!shells.contains_key(&target_session_id));
    assert!(!sftps.contains_key(&target_session_id));
    assert_eq!(shells.get(&other_session_id), Some(&"other-shell"));
    assert_eq!(sftps.get(&other_session_id), Some(&"other-sftp"));
}

#[test]
fn taking_cached_session_subresources_is_idempotent_for_missing_session() {
    let missing_session_id = session_id();
    let other_session_id = session_id();
    let mut shells = HashMap::from([(other_session_id, "other-shell")]);
    let mut sftps = HashMap::from([(other_session_id, "other-sftp")]);

    let resources = take_cached_session_subresources(&mut shells, &mut sftps, missing_session_id);

    assert_eq!(
        resources,
        CachedSessionSubresources {
            shell: None::<&str>,
            sftp: None::<&str>,
        }
    );
    assert_eq!(shells.get(&other_session_id), Some(&"other-shell"));
    assert_eq!(sftps.get(&other_session_id), Some(&"other-sftp"));
}

#[test]
fn replacing_cached_shell_returns_previous_shell_for_same_session() {
    let session_id = session_id();
    let mut shells = HashMap::from([(session_id, "old-shell")]);

    let previous = replace_cached_shell(&mut shells, session_id, "new-shell");

    assert_eq!(previous, Some("old-shell"));
    assert_eq!(shells.get(&session_id), Some(&"new-shell"));
}

#[test]
fn replacing_cached_shell_keeps_other_sessions() {
    let target_session_id = session_id();
    let other_session_id = session_id();
    let mut shells = HashMap::from([(other_session_id, "other-shell")]);

    let previous = replace_cached_shell(&mut shells, target_session_id, "target-shell");

    assert_eq!(previous, None);
    assert_eq!(shells.get(&target_session_id), Some(&"target-shell"));
    assert_eq!(shells.get(&other_session_id), Some(&"other-shell"));
}

#[test]
fn replacing_cached_sftp_returns_previous_sftp_for_same_session() {
    let session_id = session_id();
    let mut sftps = HashMap::from([(session_id, "old-sftp")]);

    let previous = replace_cached_sftp(&mut sftps, session_id, "new-sftp");

    assert_eq!(previous, Some("old-sftp"));
    assert_eq!(sftps.get(&session_id), Some(&"new-sftp"));
}

#[test]
fn replacing_cached_sftp_keeps_other_sessions() {
    let target_session_id = session_id();
    let other_session_id = session_id();
    let mut sftps = HashMap::from([(other_session_id, "other-sftp")]);

    let previous = replace_cached_sftp(&mut sftps, target_session_id, "target-sftp");

    assert_eq!(previous, None);
    assert_eq!(sftps.get(&target_session_id), Some(&"target-sftp"));
    assert_eq!(sftps.get(&other_session_id), Some(&"other-sftp"));
}
