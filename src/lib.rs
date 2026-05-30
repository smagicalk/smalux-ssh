//! smagicalssh 的库入口。
//!
//! 代码按“核心能力”和“当前桌面 UI”分开组织：
//!
//! - `model` 是应用核心状态和消息调度层。新的 UI 应优先通过
//!   `model::AppState` 和 `model::Message` 与核心交互。
//! - `backend`、`session`、`terminal`、`storage`、`security`、`config`
//!   是核心能力模块，不依赖 Slint，也不应该读取 `ui/*.slint`。
//! - `app` 是当前 Slint 桌面 UI 的 Adapter，负责把 Slint 回调翻译成
//!   `Message`，并把 `AppState` 投影成 Slint 可展示的属性和列表。
//! - `ui/*.slint` 只描述当前界面。如果以后重写 UI，优先替换 `app`
//!   和 `ui`，不要把 Slint 类型下沉到 `model` 或 `crates/*`。
//!
//! 这个入口文件只公开模块，不承载业务流程；业务流程应留在各自模块内，
//! 保持“核心可测试、UI 可替换”的结构。

pub mod app;
pub mod backend;
pub mod config;
pub mod model;
pub mod security;
pub mod session;
pub mod storage;
pub mod terminal;
pub mod theme;
