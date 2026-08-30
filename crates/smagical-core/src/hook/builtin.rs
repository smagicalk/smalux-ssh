//! 内置开箱即用示范插件库。

use super::decision::HookDecision;

use super::error::TerminalError;
use super::traits::TerminalHook;
use super::types::{CommandInteractionFrame, SessionContext};

/// 生产机高危指令防手抖拦截守卫插件。
#[derive(Debug, Default)]
pub struct DangerousCommandGuard {
    /// 拦截规则关键词列表
    dangerous_keywords: Vec<&'static str>,
}

impl DangerousCommandGuard {
    /// 创建高危命令拦截守卫实例。
    pub fn new() -> Self {
        Self {
            dangerous_keywords: vec![
                "rm -rf /",
                "rm -fr /",
                "rm -rf /*",
                "mkfs",
                "dd if=/dev/zero",
                "dd if=/dev/urandom",
                ":(){ :|:& };:", // Fork Bomb
                "shutdown -h now",
                "init 0",
            ],
        }
    }
}

impl TerminalHook for DangerousCommandGuard {
    fn name(&self) -> &'static str {
        "dangerous_command_guard"
    }

    fn priority(&self) -> i32 {
        // 最高安全级别优先级，最先执行
        100
    }

    fn on_command_start(&self, frame: &CommandInteractionFrame) -> HookDecision<Vec<u8>> {
        let cmd = frame.command_line.trim();
        for kw in &self.dangerous_keywords {
            if cmd.contains(kw) {
                // 如果是生产机，坚决拦截阻断
                let reason = format!(
                    "已拦截目标主机 [{}] 上的高危破坏性指令: \"{}\" (命中敏感模式: \"{}\")",
                    frame.session.host.host_name, cmd, kw
                );
                tracing::warn!(target: "smagical_core::security", "{}", reason);
                return HookDecision::Abort { reason };
            }
        }
        HookDecision::Continue
    }
}

/// 结构化会话时序审计日志监听插件。
#[derive(Debug, Default)]
pub struct SessionAuditLogger;

impl SessionAuditLogger {
    /// 创建审计监听器实例。
    pub fn new() -> Self {
        Self
    }
}

impl TerminalHook for SessionAuditLogger {
    fn name(&self) -> &'static str {
        "session_audit_logger"
    }

    fn priority(&self) -> i32 {
        // 普通只读审计优先级
        0
    }

    fn on_post_open(&self, ctx: &SessionContext) {
        tracing::info!(
            target: "smagical_core::audit",
            "[会话开启] 会话ID: {}, 主机: {} ({}:{}), 分组: {}",
            ctx.session_id,
            ctx.host.host_name,
            ctx.host.address,
            ctx.host.port,
            ctx.host.group_path
        );
    }

    fn on_command_completed(&self, frame: &CommandInteractionFrame) {
        tracing::info!(
            target: "smagical_core::audit",
            "[命令完成] TraceID: {}, 主机: {}, 命令: \"{}\", 耗时: {:?}, 退出码: {:?}",
            frame.trace_id,
            frame.session.host.host_name,
            frame.command_line,
            frame.duration,
            frame.exit_code
        );
    }

    fn on_command_failed(&self, frame: &CommandInteractionFrame, err: &TerminalError) {
        tracing::warn!(
            target: "smagical_core::audit",
            "[命令失败] TraceID: {}, 主机: {}, 命令: \"{}\", 错误: {}",
            frame.trace_id,
            frame.session.host.host_name,
            frame.command_line,
            err
        );
    }

    fn on_post_close(&self, ctx: &SessionContext) {
        tracing::info!(
            target: "smagical_core::audit",
            "[会话关闭] 会话ID: {}, 主机: {}",
            ctx.session_id,
            ctx.host.host_name
        );
    }
}

/// 命令完成事件回调闭包类型别名。
pub type CommandCompletedCallback = Box<dyn Fn(&CommandInteractionFrame) + Send + Sync + 'static>;

/// 函数式轻量闭包单事件监听适配器。
pub struct FunctionalHook {
    name: &'static str,
    priority: i32,
    callback: CommandCompletedCallback,
}

impl FunctionalHook {
    /// 创建闭包 Hook 适配器。
    pub fn new<F>(name: &'static str, priority: i32, callback: F) -> Self
    where
        F: Fn(&CommandInteractionFrame) + Send + Sync + 'static,
    {
        Self {
            name,
            priority,
            callback: Box::new(callback),
        }
    }
}

impl TerminalHook for FunctionalHook {
    fn name(&self) -> &'static str {
        self.name
    }

    fn priority(&self) -> i32 {
        self.priority
    }

    fn on_command_completed(&self, frame: &CommandInteractionFrame) {
        (self.callback)(frame);
    }
}

