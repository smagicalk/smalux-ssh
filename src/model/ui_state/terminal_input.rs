//! 终端输入草稿。

#[path = "terminal_input/types.rs"]
mod types;
#[path = "terminal_input/ui.rs"]
mod ui;

pub use types::TerminalInputDraft;

#[cfg(test)]
#[path = "terminal_input/tests.rs"]
mod tests;
