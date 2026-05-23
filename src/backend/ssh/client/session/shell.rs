//! SSH session 的交互式 shell。

#[path = "shell/open.rs"]
mod open;
#[path = "shell/remote.rs"]
mod remote;

pub use open::OpenShellReport;
pub use remote::RemoteShell;
