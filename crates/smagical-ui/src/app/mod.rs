//! UI 应用装配入口。

use crate::desktop::bootstrap::bootstrap_app;

pub fn run() -> anyhow::Result<()> {
    bootstrap_app()
}
