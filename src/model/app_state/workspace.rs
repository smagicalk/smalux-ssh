//! 工作区快照保存和恢复。

const DEFAULT_WORKSPACE_NAME: &str = "default";

#[path = "workspace/clear.rs"]
mod clear;
#[path = "workspace/restore.rs"]
mod restore;
#[path = "workspace/save.rs"]
mod save;
#[cfg(test)]
#[path = "workspace/tests.rs"]
mod tests;
