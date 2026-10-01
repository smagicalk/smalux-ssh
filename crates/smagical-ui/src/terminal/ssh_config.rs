//! SSH 会话启动参数高级配置与临时私钥安全生命周期托管 (KeyTempGuard)。
//!
//! 核心协议逻辑已下沉抽离至独立 Crate `smagical_ssh`，此处保留别名重导出以确保向下兼容与零断裂。

pub use smagical_ssh::ssh_config::*;
