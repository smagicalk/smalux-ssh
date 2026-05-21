//! 远程命令后端请求构造。

use crate::backend::{PtyRequest, RemoteCommandRequest};
use crate::terminal::TerminalSize;

pub(super) fn remote_command_request(
    command: String,
    size: TerminalSize,
    request_pty: bool,
) -> RemoteCommandRequest {
    if request_pty {
        RemoteCommandRequest::with_pty(command, PtyRequest::xterm(size))
    } else {
        RemoteCommandRequest::exec(command)
    }
}
