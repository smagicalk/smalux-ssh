//! 终端 Hook 调度与多监听者注册引擎。

use std::panic::AssertUnwindSafe;
use std::sync::{Arc, RwLock};

use super::decision::HookDecision;
use super::error::TerminalError;
use super::traits::TerminalHook;
use super::types::{CommandInteractionFrame, HostMetadata, SessionContext};

/// 终端 Hook 统一调度与注册引擎 (线程安全)。
#[derive(Default)]
pub struct HookEngine {
    hooks: RwLock<Vec<Arc<dyn TerminalHook>>>,
}

impl HookEngine {
    /// 创建一个新的 HookEngine 实例。
    pub fn new() -> Self {
        Self {
            hooks: RwLock::new(Vec::new()),
        }
    }

    /// 动态注册一个外部 Hook 监听器 (按优先级降序排序，优先级高的先执行)。
    pub fn register(&self, hook: Arc<dyn TerminalHook>) {
        let mut list = self.hooks.write().unwrap_or_else(|e| e.into_inner());
        // 移除同名的已有 Hook
        list.retain(|h| h.name() != hook.name());
        list.push(hook);
        // 按 priority 降序排列 (100 -> 50 -> 0 -> -10)
        list.sort_by_key(|b| std::cmp::Reverse(b.priority()));
    }


    /// 动态注销指定名称的 Hook 监听器。
    pub fn unregister(&self, name: &str) {
        let mut list = self.hooks.write().unwrap_or_else(|e| e.into_inner());
        list.retain(|h| h.name() != name);
    }

    /// 获取当前所有已注册 Hook 的名称列表。
    pub fn list_hooks(&self) -> Vec<&'static str> {
        let list = self.hooks.read().unwrap_or_else(|e| e.into_inner());
        list.iter().map(|h| h.name()).collect()
    }

    /// 获取已注册 Hook 的总数量。
    pub fn len(&self) -> usize {
        let list = self.hooks.read().unwrap_or_else(|e| e.into_inner());
        list.len()
    }

    /// 判断是否没有任何 Hook 注册。
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    // =========================================================================
    // 责任链与广播调度器实现 (带 Panic 物理隔离)
    // =========================================================================

    /// 调度：打开前 (责任链模式，高优先级先执行，若 Abort 则中止)
    pub fn dispatch_pre_open(&self, ctx: &mut SessionContext) -> HookDecision {
        let hooks = self.get_hooks_snapshot();
        for hook in hooks {
            let res = std::panic::catch_unwind(AssertUnwindSafe(|| {
                hook.on_pre_open(ctx)
            }));
            match res {
                Ok(decision) if !decision.is_continue() => return decision,
                Ok(_) => {}
                Err(_) => {
                    tracing::error!(target: "smagical_core::hook", "Hook [{}] 在 on_pre_open 发生 Panic", hook.name());
                }
            }
        }
        HookDecision::Continue
    }

    /// 调度：打开成功 (广播模式)
    pub fn dispatch_post_open(&self, ctx: &SessionContext) {
        let hooks = self.get_hooks_snapshot();
        for hook in hooks {
            let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
                hook.on_post_open(ctx);
            }));
        }
    }

    /// 调度：打开失败 (责任链模式，可由插件决策重试或静默)
    pub fn dispatch_open_failed(&self, ctx: &SessionContext, err: &TerminalError) -> HookDecision {
        let hooks = self.get_hooks_snapshot();
        for hook in hooks {
            let res = std::panic::catch_unwind(AssertUnwindSafe(|| {
                hook.on_open_failed(ctx, err)
            }));
            if let Ok(decision) = res && !decision.is_continue() {
                return decision;
            }
        }
        HookDecision::Continue
    }

    /// 调度：连接前 (责任链模式)
    pub fn dispatch_pre_connect(&self, ctx: &SessionContext) -> HookDecision {
        let hooks = self.get_hooks_snapshot();
        for hook in hooks {
            let res = std::panic::catch_unwind(AssertUnwindSafe(|| {
                hook.on_pre_connect(ctx)
            }));
            if let Ok(decision) = res && !decision.is_continue() {
                return decision;
            }
        }
        HookDecision::Continue
    }

    /// 调度：连接成功 (广播模式)
    pub fn dispatch_post_connect(&self, ctx: &SessionContext) {
        let hooks = self.get_hooks_snapshot();
        for hook in hooks {
            let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
                hook.on_post_connect(ctx);
            }));
        }
    }

    /// 调度：认证失败 (责任链模式)
    pub fn dispatch_auth_failed(&self, ctx: &SessionContext, err: &TerminalError) -> HookDecision {
        let hooks = self.get_hooks_snapshot();
        for hook in hooks {
            let res = std::panic::catch_unwind(AssertUnwindSafe(|| {
                hook.on_auth_failed(ctx, err)
            }));
            if let Ok(decision) = res && !decision.is_continue() {
                return decision;
            }
        }
        HookDecision::Continue
    }

    /// 调度：连接失败 (责任链模式)
    pub fn dispatch_connect_failed(&self, ctx: &SessionContext, err: &TerminalError) -> HookDecision {
        let hooks = self.get_hooks_snapshot();
        for hook in hooks {
            let res = std::panic::catch_unwind(AssertUnwindSafe(|| {
                hook.on_connect_failed(ctx, err)
            }));
            if let Ok(decision) = res && !decision.is_continue() {
                return decision;
            }
        }
        HookDecision::Continue
    }

    /// 调度：命令即将发送 (责任链拦截与数据修改模式)
    pub fn dispatch_command_start(&self, frame: &CommandInteractionFrame) -> HookDecision<Vec<u8>> {
        let hooks = self.get_hooks_snapshot();
        for hook in hooks {
            let res = std::panic::catch_unwind(AssertUnwindSafe(|| {
                hook.on_command_start(frame)
            }));
            match res {
                Ok(decision) if !decision.is_continue() => return decision,
                Ok(_) => {}
                Err(_) => {
                    tracing::error!(target: "smagical_core::hook", "Hook [{}] 在 on_command_start 发生 Panic", hook.name());
                }
            }
        }
        HookDecision::Continue
    }

    /// 调度：命令输出分块 (广播模式，只读切片)
    pub fn dispatch_command_output_chunk(&self, trace_id: &str, host: &HostMetadata, chunk: &[u8]) {
        let hooks = self.get_hooks_snapshot();
        for hook in hooks {
            let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
                hook.on_command_output_chunk(trace_id, host, chunk);
            }));
        }
    }

    /// 调度：命令执行完成 (广播模式)
    pub fn dispatch_command_completed(&self, frame: &CommandInteractionFrame) {
        let hooks = self.get_hooks_snapshot();
        for hook in hooks {
            let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
                hook.on_command_completed(frame);
            }));
        }
    }

    /// 调度：命令执行失败 (广播模式)
    pub fn dispatch_command_failed(&self, frame: &CommandInteractionFrame, err: &TerminalError) {
        let hooks = self.get_hooks_snapshot();
        for hook in hooks {
            let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
                hook.on_command_failed(frame, err);
            }));
        }
    }

    /// 调度：网络意外中断 (责任链模式，可由自动重试插件决定重试)
    pub fn dispatch_connection_broken(&self, ctx: &SessionContext, err: &TerminalError) -> HookDecision {
        let hooks = self.get_hooks_snapshot();
        for hook in hooks {
            let res = std::panic::catch_unwind(AssertUnwindSafe(|| {
                hook.on_connection_broken(ctx, err)
            }));
            if let Ok(decision) = res && !decision.is_continue() {
                return decision;
            }
        }
        HookDecision::Continue
    }

    /// 调度：关闭前 (责任链模式)
    pub fn dispatch_pre_close(&self, ctx: &SessionContext) -> HookDecision {
        let hooks = self.get_hooks_snapshot();
        for hook in hooks {
            let res = std::panic::catch_unwind(AssertUnwindSafe(|| {
                hook.on_pre_close(ctx)
            }));
            if let Ok(decision) = res && !decision.is_continue() {
                return decision;
            }
        }
        HookDecision::Continue
    }

    /// 调度：关闭后 (广播模式)
    pub fn dispatch_post_close(&self, ctx: &SessionContext) {
        let hooks = self.get_hooks_snapshot();
        for hook in hooks {
            let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
                hook.on_post_close(ctx);
            }));
        }
    }

    fn get_hooks_snapshot(&self) -> Vec<Arc<dyn TerminalHook>> {
        let list = self.hooks.read().unwrap_or_else(|e| e.into_inner());
        list.clone()
    }
}
