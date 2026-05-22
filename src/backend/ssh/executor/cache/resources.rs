//! SSH executor 会话资源缓存操作。

use std::collections::HashMap;

use crate::model::SessionId;

use super::tunnels::{TunnelOwner, take_tunnels_for_session};

#[derive(Debug, PartialEq, Eq)]
pub(in crate::backend::ssh::executor) struct CachedSessionSubresources<TShell, TSftp> {
    pub(in crate::backend::ssh::executor) shell: Option<TShell>,
    pub(in crate::backend::ssh::executor) sftp: Option<TSftp>,
}

#[derive(Debug, PartialEq, Eq)]
pub(in crate::backend::ssh::executor) struct CachedSessionResources<TShell, TSftp, TConnection> {
    pub(in crate::backend::ssh::executor) shell: Option<TShell>,
    pub(in crate::backend::ssh::executor) sftp: Option<TSftp>,
    pub(in crate::backend::ssh::executor) connection: Option<TConnection>,
}

#[derive(Debug, PartialEq, Eq)]
pub(in crate::backend::ssh::executor) struct CachedSessionRuntimeResources<
    TShell,
    TSftp,
    TConnection,
    TTunnel,
> {
    pub(in crate::backend::ssh::executor) cached_resources:
        CachedSessionResources<TShell, TSftp, TConnection>,
    pub(in crate::backend::ssh::executor) tunnels: Vec<TTunnel>,
}

/// 一次性取出会话拥有的所有后端运行态，调用方随后负责关闭或停止。
pub(in crate::backend::ssh::executor) fn take_cached_session_runtime_resources<
    TShell,
    TSftp,
    TConnection,
    TTunnel,
>(
    shells: &mut HashMap<SessionId, TShell>,
    sftps: &mut HashMap<SessionId, TSftp>,
    connections: &mut HashMap<SessionId, TConnection>,
    tunnels: &mut HashMap<String, TTunnel>,
    session_id: SessionId,
) -> CachedSessionRuntimeResources<TShell, TSftp, TConnection, TTunnel>
where
    TTunnel: TunnelOwner,
{
    CachedSessionRuntimeResources {
        cached_resources: take_cached_session_resources(shells, sftps, connections, session_id),
        tunnels: take_tunnels_for_session(tunnels, session_id),
    }
}

pub(in crate::backend::ssh::executor) fn take_cached_session_subresources<TShell, TSftp>(
    shells: &mut HashMap<SessionId, TShell>,
    sftps: &mut HashMap<SessionId, TSftp>,
    session_id: SessionId,
) -> CachedSessionSubresources<TShell, TSftp> {
    CachedSessionSubresources {
        shell: shells.remove(&session_id),
        sftp: sftps.remove(&session_id),
    }
}

pub(in crate::backend::ssh::executor) fn take_cached_session_resources<
    TShell,
    TSftp,
    TConnection,
>(
    shells: &mut HashMap<SessionId, TShell>,
    sftps: &mut HashMap<SessionId, TSftp>,
    connections: &mut HashMap<SessionId, TConnection>,
    session_id: SessionId,
) -> CachedSessionResources<TShell, TSftp, TConnection> {
    CachedSessionResources {
        shell: shells.remove(&session_id),
        sftp: sftps.remove(&session_id),
        connection: connections.remove(&session_id),
    }
}

pub(in crate::backend::ssh::executor) fn replace_cached_shell<TShell>(
    shells: &mut HashMap<SessionId, TShell>,
    session_id: SessionId,
    shell: TShell,
) -> Option<TShell> {
    shells.insert(session_id, shell)
}

pub(in crate::backend::ssh::executor) fn replace_cached_sftp<TSftp>(
    sftps: &mut HashMap<SessionId, TSftp>,
    session_id: SessionId,
    sftp: TSftp,
) -> Option<TSftp> {
    sftps.insert(session_id, sftp)
}

#[cfg(test)]
#[path = "resources_tests.rs"]
mod tests;
