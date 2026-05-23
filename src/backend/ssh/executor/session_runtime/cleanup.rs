//! SSH executor 会话资源清理。

use crate::model::SessionId;
use crate::security::SecretStore;

use super::super::super::{RemoteSftp, RemoteShell, RusshConnection};
use super::super::RusshBackendExecutor;
use super::super::cache::{CachedSessionResources, CachedSessionSubresources};

impl<S: SecretStore + Send> RusshBackendExecutor<S> {
    pub(super) fn close_stale_session_resources(
        &self,
        session_id: SessionId,
        resources: CachedSessionResources<RemoteShell, RemoteSftp, RusshConnection>,
    ) {
        self.close_stale_session_subresources(
            session_id,
            CachedSessionSubresources {
                shell: resources.shell,
                sftp: resources.sftp,
            },
            "reconnecting",
        );

        if let Some(connection) = resources.connection
            && let Err(error) = self.runtime.block_on(connection.disconnect())
        {
            tracing::warn!(
                session_id = %session_id.0,
                error = %error,
                "failed to disconnect stale SSH connection before reconnect"
            );
        }
    }

    pub(super) fn close_disconnected_session_resources(
        &self,
        session_id: SessionId,
        resources: CachedSessionResources<RemoteShell, RemoteSftp, RusshConnection>,
    ) {
        self.close_stale_session_subresources(
            session_id,
            CachedSessionSubresources {
                shell: resources.shell,
                sftp: resources.sftp,
            },
            "disconnecting",
        );

        if let Some(connection) = resources.connection
            && let Err(error) = self.runtime.block_on(connection.disconnect())
        {
            tracing::warn!(
                session_id = %session_id.0,
                error = %error,
                "failed to disconnect SSH connection"
            );
        }
    }

    pub(in crate::backend::ssh::executor) fn close_stale_session_subresources(
        &self,
        session_id: SessionId,
        resources: CachedSessionSubresources<RemoteShell, RemoteSftp>,
        operation: &'static str,
    ) {
        self.close_detached_shell_input(session_id, resources.shell, operation);

        self.close_detached_sftp(session_id, resources.sftp, operation);
    }
}
