//! 终端 Hook 核心 Trait 定义。

use super::decision::HookDecision;
use super::error::TerminalError;
use super::types::{CommandInteractionFrame, HostMetadata, SessionContext};

/// 终端生命周期、异常故障与输入输出时序拦截接口。
pub trait TerminalHook: Send + Sync {
    /// 插件/Hook 唯一名称标识。
    fn name(&self) -> &'static str;

    /// 插件优先级 (数值越大越先执行，默认为 0，安全拦截插件可设为 100)。
    fn priority(&self) -> i32 {
        0
    }

    // =========================================================================
    // 1. 打开与初始化阶段 (Open Phase)
    // =========================================================================

    /// 【打开前】：参数拦截、跳板机建立、权限校验。
    fn on_pre_open(&self, _ctx: &mut SessionContext) -> HookDecision {
        HookDecision::Continue
    }

    /// 【打开后】：会话已创建，Tab 注册完毕。
    fn on_post_open(&self, _ctx: &SessionContext) {}

    /// 【打开失败】：PTY 创建失败或系统资源超限。
    fn on_open_failed(&self, _ctx: &SessionContext, _err: &TerminalError) -> HookDecision {
        HookDecision::Continue
    }

    // =========================================================================
    // 2. 连接与网络握手阶段 (Connect & Auth Phase)
    // =========================================================================

    /// 【连接前】：网络前置探活、代理链路准备。
    fn on_pre_connect(&self, _ctx: &SessionContext) -> HookDecision {
        HookDecision::Continue
    }

    /// 【连接成功】：SSH 握手完成，已获取远端系统指纹。
    fn on_post_connect(&self, _ctx: &SessionContext) {}

    /// 【认证失败】：密码错误、密钥被拒、2FA 超时。
    fn on_auth_failed(&self, _ctx: &SessionContext, _err: &TerminalError) -> HookDecision {
        HookDecision::Continue
    }

    /// 【连接失败/超时】：目标主机端口不通或握手超时。
    fn on_connect_failed(&self, _ctx: &SessionContext, _err: &TerminalError) -> HookDecision {
        HookDecision::Continue
    }

    // =========================================================================
    // 3. 命令输入-输出时序交互阶段 (Command Tracing Phase)
    // =========================================================================

    /// 【命令即将发送执行】：高危指令拦截、宏替换、输入审计。
    fn on_command_start(&self, _frame: &CommandInteractionFrame) -> HookDecision<Vec<u8>> {
        HookDecision::Continue
    }

    /// 【收到属于该命令的输出流分块】：零拷贝只读切片 (用于实时录屏、关键词触发器)。
    fn on_command_output_chunk(&self, _trace_id: &str, _host: &HostMetadata, _chunk: &[u8]) {}

    /// 【命令执行结束】：耗时与返回码已就绪，可交付 AI 诊断或结构化审计。
    fn on_command_completed(&self, _frame: &CommandInteractionFrame) {}

    /// 【命令执行异常/拦截中断】：记录对应机器的高危告警日志。
    fn on_command_failed(&self, _frame: &CommandInteractionFrame, _err: &TerminalError) {}

    // =========================================================================
    // 4. 运行期故障与网络波动 (Runtime Faults)
    // =========================================================================

    /// 【网络中断/闪断】：针对该特定主机决策是否触发自动重连退避。
    fn on_connection_broken(&self, _ctx: &SessionContext, _err: &TerminalError) -> HookDecision {
        HookDecision::Continue
    }

    // =========================================================================
    // 5. 关闭与清理阶段 (Close Phase)
    // =========================================================================

    /// 【关闭前】：守护该主机上的前台运行任务。
    fn on_pre_close(&self, _ctx: &SessionContext) -> HookDecision {
        HookDecision::Continue
    }

    /// 【关闭后】：彻底释放该主机占用的隧道、端口映射与 PTY 资源。
    fn on_post_close(&self, _ctx: &SessionContext) {}
}
