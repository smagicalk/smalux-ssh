//! smalux-ssh 桌面应用程序入口。
//!
//! 负责初始化全局 Tracing 日志跟踪器并启动 Slint UI 主循环。

fn main() -> anyhow::Result<()> {
    // 初始化全局 Tokio 多线程异步运行时并进入上下文
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let _rt_guard = rt.enter();

    // 初始化日志跟踪系统
    let _tracing_guard = smagical_ui::debug::init_tracing("smalux", None)?;
    // 预先从磁盘读取用户持久化的渲染管线配置并注入环境变量 (确保 Slint 渲染器加载生效)
    let active_pipeline = smagical_ui::pipeline_config::init_runtime_pipeline();
    tracing::info!(target: "smalux::bootstrap", "底层图形渲染引擎预分发初始化: [{}]", active_pipeline);
    // 启动 Slint UI 桌面主窗口
    smagical_ui::run()?;
    Ok(())
}


