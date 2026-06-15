use super::*;
use crate::backend::{BackendCommand, TunnelStopRequest};
use crate::model::{SessionId, TunnelKind, TunnelRule, TunnelStatus};

#[path = "tunnel_start.rs"]
mod start;
#[path = "tunnel_stop.rs"]
mod stop;

fn tunnel_rule(kind: TunnelKind) -> TunnelRule {
    TunnelRule {
        name: "local-db".to_owned(),
        kind,
        bind_host: "127.0.0.1".to_owned(),
        bind_port: 15432,
        target_host: "10.0.0.5".to_owned(),
        target_port: 5432,
        auto_start: false,
        exit_on_failure: false,
    }
}
