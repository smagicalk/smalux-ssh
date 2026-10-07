//! UI 线程安全调度与异步任务派发公共工具 (UI Dispatch Utilities)。
//!
//! 统一封装跨线程调度到 Slint UI 主事件循环的样板逻辑：
//! - `run_on_ui`: 安全升级 Weak 引用并调度执行闭包；
//! - `spawn_ui_task`: 派发后台异步 IO 操作并在就绪后安全投递至 UI 线程。

use std::future::Future;
use crate::async_util::spawn_async;

/// 在 Slint UI 事件主循环上安全执行闭包 (自动处理 Weak::upgrade 兜底)。
#[inline]
pub fn run_on_ui<W, F>(window_weak: slint::Weak<W>, f: F)
where
    W: slint::ComponentHandle + 'static,
    F: FnOnce(W) + Send + 'static,
{
    let _ = slint::invoke_from_event_loop(move || {
        if let Some(w) = window_weak.upgrade() {
            f(w);
        }
    });
}

/// 派发异步后台 IO 任务，并在完成计算/拉取后自动调度回 UI 线程执行更新回调。
#[inline]
pub fn spawn_ui_task<W, T, Fut, F, U>(window_weak: slint::Weak<W>, task: F, on_ui: U)
where
    W: slint::ComponentHandle + 'static,
    T: Send + 'static,
    Fut: Future<Output = T> + Send + 'static,
    F: FnOnce() -> Fut + Send + 'static,
    U: FnOnce(W, T) + Send + 'static,
{
    spawn_async(async move {
        let res = task().await;
        run_on_ui(window_weak, move |w| {
            on_ui(w, res);
        });
    });
}
