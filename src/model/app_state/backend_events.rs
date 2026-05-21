//! 后端事件归约和共享执行器入口。
//!
//! 负责把后端事件应用到会话和终端状态，以及从共享执行器泵出后台命令。

#[path = "backend_events/apply.rs"]
mod apply;
#[path = "backend_events/remote_command_history.rs"]
mod remote_command_history;
