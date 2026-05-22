use std::collections::HashMap;

use uuid::Uuid;

use super::*;

fn session_id() -> SessionId {
    SessionId(Uuid::new_v4())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OwnedTunnel {
    session_id: SessionId,
}

impl TunnelOwner for OwnedTunnel {
    fn session_id(&self) -> SessionId {
        self.session_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TestTunnel {
    session_id: SessionId,
    rule_name: String,
    stopped: bool,
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
    fn stop(&self) {
        STOPPED_TEST_TUNNEL_NAMES.with(|names| names.borrow_mut().push(self.rule_name.clone()));
    }
}

thread_local! {
    static STOPPED_TEST_TUNNEL_NAMES: std::cell::RefCell<Vec<String>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

#[test]
fn removing_tunnel_requires_matching_session_and_rule() {
    let owner_session_id = session_id();
    let stale_session_id = session_id();
    let other_session_id = session_id();
    let mut tunnels = HashMap::from([
        (
            "proxy".to_owned(),
            OwnedTunnel {
                session_id: owner_session_id,
            },
        ),
        (
            "metrics".to_owned(),
            OwnedTunnel {
                session_id: other_session_id,
            },
        ),
    ]);

    let stale = remove_tunnel_for_session_rule(&mut tunnels, stale_session_id, "proxy");
    let missing = remove_tunnel_for_session_rule(&mut tunnels, owner_session_id, "missing");
    let removed = remove_tunnel_for_session_rule(&mut tunnels, owner_session_id, "proxy");

    assert_eq!(stale, None);
    assert_eq!(missing, None);
    assert_eq!(
        removed,
        Some(OwnedTunnel {
            session_id: owner_session_id,
        })
    );
    assert!(!tunnels.contains_key("proxy"));
    assert_eq!(
        tunnels.get("metrics"),
        Some(&OwnedTunnel {
            session_id: other_session_id,
        })
    );
}

#[test]
fn replacing_tunnel_stops_previous_same_rule() {
    let old_session_id = session_id();
    let new_session_id = session_id();
    let mut tunnels = HashMap::from([(
        "proxy".to_owned(),
        TestTunnel {
            session_id: old_session_id,
            rule_name: "proxy".to_owned(),
            stopped: false,
        },
    )]);
    STOPPED_TEST_TUNNEL_NAMES.with(|names| names.borrow_mut().clear());

    replace_tunnel_stopping_previous(
        &mut tunnels,
        TestTunnel {
            session_id: new_session_id,
            rule_name: "proxy".to_owned(),
            stopped: false,
        },
    );

    assert_eq!(
        STOPPED_TEST_TUNNEL_NAMES.with(|names| names.borrow().clone()),
        ["proxy"]
    );
    assert_eq!(
        tunnels.get("proxy"),
        Some(&TestTunnel {
            session_id: new_session_id,
            rule_name: "proxy".to_owned(),
            stopped: false,
        })
    );
}

#[test]
fn replacing_tunnel_keeps_unrelated_rules_running() {
    let existing_session_id = session_id();
    let new_session_id = session_id();
    let mut tunnels = HashMap::from([(
        "metrics".to_owned(),
        TestTunnel {
            session_id: existing_session_id,
            rule_name: "metrics".to_owned(),
            stopped: false,
        },
    )]);
    STOPPED_TEST_TUNNEL_NAMES.with(|names| names.borrow_mut().clear());

    replace_tunnel_stopping_previous(
        &mut tunnels,
        TestTunnel {
            session_id: new_session_id,
            rule_name: "proxy".to_owned(),
            stopped: false,
        },
    );

    assert!(STOPPED_TEST_TUNNEL_NAMES.with(|names| names.borrow().is_empty()));
    assert!(tunnels.contains_key("metrics"));
    assert_eq!(
        tunnels.get("proxy"),
        Some(&TestTunnel {
            session_id: new_session_id,
            rule_name: "proxy".to_owned(),
            stopped: false,
        })
    );
}

#[test]
fn taking_tunnels_for_session_removes_only_owned_tunnels() {
    let owner_session_id = session_id();
    let other_session_id = session_id();
    let mut tunnels = HashMap::from([
        (
            "proxy".to_owned(),
            TestTunnel {
                session_id: owner_session_id,
                rule_name: "proxy".to_owned(),
                stopped: false,
            },
        ),
        (
            "db".to_owned(),
            TestTunnel {
                session_id: owner_session_id,
                rule_name: "db".to_owned(),
                stopped: false,
            },
        ),
        (
            "metrics".to_owned(),
            TestTunnel {
                session_id: other_session_id,
                rule_name: "metrics".to_owned(),
                stopped: false,
            },
        ),
    ]);

    let removed = take_tunnels_for_session(&mut tunnels, owner_session_id);

    assert_eq!(removed.len(), 2);
    assert!(removed.iter().any(|tunnel| tunnel.rule_name == "proxy"));
    assert!(removed.iter().any(|tunnel| tunnel.rule_name == "db"));
    assert!(!tunnels.contains_key("proxy"));
    assert!(!tunnels.contains_key("db"));
    assert_eq!(
        tunnels.get("metrics"),
        Some(&TestTunnel {
            session_id: other_session_id,
            rule_name: "metrics".to_owned(),
            stopped: false,
        })
    );
}

#[test]
fn taking_tunnels_for_missing_session_keeps_all_tunnels() {
    let missing_session_id = session_id();
    let other_session_id = session_id();
    let mut tunnels = HashMap::from([(
        "metrics".to_owned(),
        TestTunnel {
            session_id: other_session_id,
            rule_name: "metrics".to_owned(),
            stopped: false,
        },
    )]);

    let removed = take_tunnels_for_session(&mut tunnels, missing_session_id);

    assert!(removed.is_empty());
    assert_eq!(
        tunnels.get("metrics"),
        Some(&TestTunnel {
            session_id: other_session_id,
            rule_name: "metrics".to_owned(),
            stopped: false,
        })
    );
}

#[test]
fn stopping_detached_tunnels_stops_each_removed_tunnel() {
    let session_id = session_id();
    let tunnels = vec![
        TestTunnel {
            session_id,
            rule_name: "proxy".to_owned(),
            stopped: false,
        },
        TestTunnel {
            session_id,
            rule_name: "db".to_owned(),
            stopped: false,
        },
    ];
    STOPPED_TEST_TUNNEL_NAMES.with(|names| names.borrow_mut().clear());

    stop_detached_tunnels(session_id, tunnels, "test");

    let mut stopped = STOPPED_TEST_TUNNEL_NAMES.with(|names| names.borrow().clone());
    stopped.sort();
    assert_eq!(stopped, ["db", "proxy"]);
}
