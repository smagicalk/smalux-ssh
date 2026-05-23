use std::collections::HashMap;

use super::super::replace_tunnel_stopping_previous;
use super::common::{TestTunnel, clear_stopped_tunnel_names, session_id, stopped_tunnel_names};

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
    clear_stopped_tunnel_names();

    replace_tunnel_stopping_previous(
        &mut tunnels,
        TestTunnel {
            session_id: new_session_id,
            rule_name: "proxy".to_owned(),
            stopped: false,
        },
    );

    assert_eq!(stopped_tunnel_names(), ["proxy"]);
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
    clear_stopped_tunnel_names();

    replace_tunnel_stopping_previous(
        &mut tunnels,
        TestTunnel {
            session_id: new_session_id,
            rule_name: "proxy".to_owned(),
            stopped: false,
        },
    );

    assert!(stopped_tunnel_names().is_empty());
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
