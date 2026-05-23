use std::collections::HashMap;

use super::super::take_tunnels_for_session;
use super::common::{TestTunnel, session_id};

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
