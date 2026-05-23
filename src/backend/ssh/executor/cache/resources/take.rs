use std::collections::HashMap;

use crate::model::SessionId;

use super::super::tunnels::{TunnelOwner, take_tunnels_for_session};
use super::types::{
    CachedSessionResources, CachedSessionRuntimeResources, CachedSessionSubresources,
};

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
