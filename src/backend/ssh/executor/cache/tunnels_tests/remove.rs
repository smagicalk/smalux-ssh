use std::collections::HashMap;

use super::super::remove_tunnel_for_session_rule;
use super::common::{OwnedTunnel, session_id};

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
