//! smalux-ssh 桌面应用程序入口。
//!
//! 负责初始化全局 Tracing 日志跟踪器并启动 Slint UI 主循环。

fn main() -> anyhow::Result<()> {
    // 初始化日志跟踪系统
    let _tracing_guard = smagical_debug::init_tracing("smalux", None)?;
    // 启动 Slint UI 桌面主窗口
    smagical_ui::run()?;
    Ok(())
}


