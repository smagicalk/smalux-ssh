//! 第三方终端资产迁移与格式导入解析器 (Third-Party Asset Importer)。
//!
//! 核心解析逻辑已下沉抽离至独立 Crate `smagical_ssh`，此处保留别名重导出以确保向下兼容与零断裂。

pub use smagical_ssh::importer::*;
