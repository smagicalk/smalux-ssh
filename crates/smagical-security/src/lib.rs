//! 安全与凭据解析入口。
//!
//! 领域模型只保存 `SecretRef`，本模块负责把引用解析为后端执行器可使用的临时认证材料。
//! 明文只在执行前短暂存在，持久化仍交给系统凭据库或测试用内存存储。

mod auth;
mod error;
mod keyring_store;
mod resolver;
mod store;

pub use auth::*;
pub use error::*;
pub use keyring_store::*;
pub use resolver::*;
pub use store::*;
