//! 全局应用级 Hook 核心 Trait 接口定义。

use crate::hook::HookDecision;
use super::types::{AppBootContext, AppExitContext, ConfigChangeEvent, WindowState};

/// 全局应用生命周期、主框架交互路由与配置变更 Hook 接口。
///
/// 该 Trait 的所有生命周期方法均提供默认空实现，插件与监听者可按需选择性实现。
pub trait AppGlobalHook: Send + Sync {
    /// 插件/Hook 唯一名称标识。
    fn name(&self) -> &'static str;

    /// 插件优先级 (数值越大越先执行，默认为 0，核心守护插件可设为更高)。
    fn priority(&self) -> i32 {
        0
    }

    // =========================================================================
    // 1. 进程级生命周期 (Process Lifecycle)
    // =========================================================================

    /// 【应用程序启动引导】：命令行参数解析完毕，存储引擎初始化前。
    fn on_app_boot(&self, _ctx: &AppBootContext) {}

    /// 【主界面就绪】：Slint 主窗口首帧渲染完成，所有核心服务已就绪。
    fn on_app_ready(&self) {}

    /// 【应用即将退出】：用户点击关闭或触发退出指令前 (可拦截)。
    ///
    /// 若返回 `HookDecision::Abort`，则取消退出流程并保持应用运行。
    fn on_app_before_exit(&self, _ctx: &AppExitContext) -> HookDecision {
        HookDecision::Continue
    }

    /// 【应用完全退出】：主窗口即将销毁，执行最终全量归档备份与资源释放。
    fn on_app_exit(&self, _ctx: &AppExitContext) {}

    // =========================================================================
    // 2. 主框架导航与交互路由 (Shell Navigation)
    // =========================================================================

    /// 【左侧活动栏菜单点击】：用户点击左侧图标切换抽屉面板。
    ///
    /// - `menu_id`: 目标菜单标识 (如 "hosts", "history", "files", "credentials", "settings")
    /// - `old_menu_id`: 切换前的菜单标识
    fn on_left_menu_clicked(&self, _menu_id: &str, _old_menu_id: &str) {}

    /// 【主工作区视图切换】：在中央核心工作区之间流转。
    ///
    /// - `current_view`: 目标工作区视图 (如 "terminal" 终端视口 / "history" 历史会话中心)
    /// - `previous_view`: 切换前的工作区视图
    fn on_main_view_switched(&self, _current_view: &str, _previous_view: &str) {}


    /// 【全局外观模式切换】：左下角一键切换深浅色主题。
    ///
    /// - `is_dark`: 当前是否激活深色模式
    fn on_theme_mode_toggled(&self, _is_dark: bool) {}

    /// 【全局快捷指令执行】：响应 Ctrl+K 命令面板或全局快捷键分发的指令。
    ///
    /// - `command_id`: 指令唯一标识
    fn on_command_executed(&self, _command_id: &str) {}

    // =========================================================================
    // 3. 全局参数变动与自动备份 (Config Mutation & Backup)
    // =========================================================================

    /// 【全局参数变更通知】：当系统设置、终端参数或用户偏好修改时广播触发。
    ///
    /// 典型应用：自动生成 `.backup/` 配置文件增量快照或触发版本追溯。
    fn on_global_config_changed(&self, _event: &ConfigChangeEvent) {}

    // =========================================================================
    // 4. 窗口全局状态变动 (Window State)
    // =========================================================================

    /// 【窗口状态变动通知】：窗口最小化、最大化或获得/失去焦点。
    ///
    /// 典型应用：窗口失焦或最小化时自动降低后台轮询探针频率，实现节能降耗。
    fn on_window_state_changed(&self, _state: WindowState) {}
}
