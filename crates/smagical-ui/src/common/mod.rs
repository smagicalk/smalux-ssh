//! smagical-ui 公共工具体系 (Common Utilities)。
//!
//! 统一提供跨模块线程派发、零拷贝字符串转换、极速过滤匹配与集合模型工具。

pub mod dispatch;
pub mod filter;
pub mod model;
pub mod string;

pub use dispatch::*;
pub use filter::*;
pub use model::*;
pub use string::*;
