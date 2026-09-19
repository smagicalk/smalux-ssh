//! 异步运行时与同步阻塞调度工具 (Async Runtime & Blocking Bridges)。
//!
//! 为 UI 主循环与异步存储/后台网络任务提供统一的无缝调度工具：
//! - `block_on`: 在同步上下文中安全等待异步任务（优先复用当前运行时，若无运行时则自适应创建单线程运行时，永不 panic）。
//! - `spawn_async`: 将耗时的异步 IO 操作派发到后台 Tokio 线程池中无阻塞并发执行。

use std::future::Future;

/// 在同步上下文安全执行并等待 Future 完成
pub fn block_on<F: Future>(future: F) -> F::Output {
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => tokio::task::block_in_place(|| handle.block_on(future)),
        Err(_) => {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to build local Tokio runtime");
            rt.block_on(future)
        }
    }
}

/// 发射后台异步任务至 Tokio 线程池并发执行 (UI 线程零阻塞)
pub fn spawn_async<F>(future: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => {
            handle.spawn(future);
        }
        Err(_) => {
            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("Failed to build local Tokio runtime");
                rt.block_on(future);
            });
        }
    }
}


