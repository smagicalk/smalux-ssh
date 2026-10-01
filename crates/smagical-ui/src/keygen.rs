//! SSH 密钥对现场生成器 (SSH Key Pair Generator)。
//!
//! 核心生成逻辑已下沉抽离至独立 Crate `smagical_ssh`，此处保留别名重导出以确保向下兼容与零断裂。

pub use smagical_ssh::keygen::*;
