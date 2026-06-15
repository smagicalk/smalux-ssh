use super::*;

#[path = "tunnel_guard.rs"]
mod guard;
#[path = "tunnel_pending.rs"]
mod pending;
#[path = "tunnel_runtime.rs"]
mod runtime;

fn tunnel_rule() -> TunnelRule {
    TunnelRule {
        name: "local-db".to_owned(),
        kind: TunnelKind::Local,
        bind_host: "127.0.0.1".to_owned(),
        bind_port: 15432,
        target_host: "10.0.0.5".to_owned(),
        target_port: 5432,
        auto_start: false,
        exit_on_failure: false,
    }
}
