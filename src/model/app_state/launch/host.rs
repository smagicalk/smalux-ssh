//! 启动流程共享主机查询和连接命令。

use crate::backend::{BackendCommand, ConnectionTarget};
use crate::model::{Host, HostId, KnownHostEntry, RecentConnection, SessionId};

use super::super::AppState;
use super::unix_now_secs;

impl AppState {
    pub(in crate::model::app_state) fn host_by_id(&self, host_id: HostId) -> Option<Host> {
        self.storage
            .hosts
            .iter()
            .find(|host| host.id == host_id)
            .cloned()
    }

    pub(in crate::model::app_state) fn record_recent_connection(&mut self, host: &Host) {
        self.storage.record_recent_connection(RecentConnection {
            host_id: host.id,
            label: host.name.clone(),
            connected_at_unix_secs: unix_now_secs(),
        });
    }
}

pub(in crate::model::app_state) fn connect_command_with_known_hosts(
    session_id: SessionId,
    host: &Host,
    known_hosts: Vec<KnownHostEntry>,
) -> BackendCommand {
    BackendCommand::Connect {
        session_id,
        target: ConnectionTarget::from_host_with_known_hosts(host, known_hosts),
    }
}
