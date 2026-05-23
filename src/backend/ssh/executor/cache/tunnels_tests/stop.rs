use super::super::stop_detached_tunnels;
use super::common::{TestTunnel, clear_stopped_tunnel_names, session_id, stopped_tunnel_names};

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
    clear_stopped_tunnel_names();

    stop_detached_tunnels(session_id, tunnels, "test");

    let mut stopped = stopped_tunnel_names();
    stopped.sort();
    assert_eq!(stopped, ["db", "proxy"]);
}
