//! 远程 SFTP 与 SSH 文件系统协议驱动引擎 (SFTP Subsystem)。
//!
//! 核心文件系统传输引擎已下沉抽离至独立 Crate `smagical_ssh`，此处保留别名重导出以确保向下兼容与零断裂。

pub use smagical_ssh::sftp::*;
