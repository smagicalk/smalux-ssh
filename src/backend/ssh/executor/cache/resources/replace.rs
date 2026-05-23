use std::collections::HashMap;

use crate::model::SessionId;

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
