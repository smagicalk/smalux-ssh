use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use crate::model::SessionId;
use uuid::Uuid;

use super::tunnel;

#[test]
fn remote_tunnel_reports_endpoint_and_can_stop() {
    let session_id = SessionId(Uuid::new_v4());
    let running = Arc::new(AtomicBool::new(true));
    let tunnel = tunnel(
        session_id,
        "proxy".to_owned(),
        running.clone(),
        "127.0.0.1".to_owned(),
        1080,
    );

    assert_eq!(tunnel.session_id(), session_id);
    assert_eq!(tunnel.rule_name(), "proxy");
    assert_eq!(tunnel.bind_endpoint(), "127.0.0.1:1080");
    tunnel.stop();
    assert!(!running.load(Ordering::SeqCst));
}
