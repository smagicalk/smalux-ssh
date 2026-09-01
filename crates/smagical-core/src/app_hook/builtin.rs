//! 全局应用级内置插件与适配器。

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use crate::hook::HookDecision;
use super::traits::AppGlobalHook;
use super::types::{AppExitContext, ConfigChangeEvent};


/// 全局参数变动自动备份与退出归档插件。
///
/// 实时监听 `on_global_config_changed` 与 `on_app_exit`，自动记录配置变动审计并统计快照生成次数。
pub struct AutoConfigBackupHook {
    backup_count: AtomicUsize,
}

impl Default for AutoConfigBackupHook {
    fn default() -> Self {
        Self::new()
    }
}

impl AutoConfigBackupHook {
    /// 创建一个新的自动备份插件实例。
    pub fn new() -> Self {
        Self {
            backup_count: AtomicUsize::new(0),
        }
    }

    /// 获取自启动以来累计自动触发的备份次数。
    pub fn backup_count(&self) -> usize {
        self.backup_count.load(Ordering::Relaxed)
    }
}

impl AppGlobalHook for AutoConfigBackupHook {
    fn name(&self) -> &'static str {
        "builtin_auto_config_backup"
    }

    fn priority(&self) -> i32 {
        50
    }

    fn on_config_changed(&self, event: &ConfigChangeEvent) {
        let count = self.backup_count.fetch_add(1, Ordering::Relaxed) + 1;
        tracing::info!(
            target: "smalux::backup",
            "[自动备份 #{}]: 检测到全局参数变更 [{}] ({} -> {}), 来源: {}, 已触发增量备份快照",
            count,
            event.key,
            event.old_val,
            event.new_val,
            event.source
        );
    }

    fn on_app_exit(&self, ctx: &AppExitContext) {
        tracing::info!(
            target: "smalux::backup",
            "[退出归档]: 应用程序正在退出 (活跃会话: {}, 状态码: {}), 已执行最终全量数据持久化归档",
            ctx.active_sessions_count,
            ctx.exit_code
        );
    }
}

/// 主机资产全生命周期与终端会话操作审计日志插件。
///
/// 实时监听主机资产变动 (`on_host_asset_*`)、终端会话生命周期 (`on_host_terminal_*`)、
/// 历史会话操作 (`on_history_*`) 与配置变动，生成符合规范的结构化审计日志。
pub struct HostAuditLogHook;

impl Default for HostAuditLogHook {
    fn default() -> Self {
        Self::new()
    }
}

impl HostAuditLogHook {
    /// 创建一个新的主机与会话审计日志插件实例。
    pub fn new() -> Self {
        Self
    }
}

impl AppGlobalHook for HostAuditLogHook {
    fn name(&self) -> &'static str {
        "builtin_host_audit_logger"
    }

    fn priority(&self) -> i32 {
        40
    }

    fn on_host_asset_created(&self, host: &crate::domain::HostRecord) {
        tracing::info!(
            target: "audit::host",
            "[资产审计:新增] 主机 ID: [{}], 名称: '{}', 地址: {}:{}",
            host.id, host.name, host.address, host.port
        );
    }

    fn on_host_asset_updated(&self, old_host: &crate::domain::HostRecord, new_host: &crate::domain::HostRecord) {
        tracing::warn!(
            target: "audit::host",
            "[资产审计:修改] 主机 ID: [{}], 名称: '{}' -> '{}', 地址: {}:{} -> {}:{}",
            new_host.id, old_host.name, new_host.name, old_host.address, old_host.port, new_host.address, new_host.port
        );
    }

    fn on_host_asset_deleted(&self, host_id: &str) {
        tracing::warn!(
            target: "audit::host",
            "[资产审计:删除] 主机 ID: [{}]",
            host_id
        );
    }

    fn on_host_asset_group_toggled(&self, group_id: &str, is_expanded: bool) {
        tracing::debug!(
            target: "audit::host",
            "[资产偏好] 分组 ID: [{}] 切换为: {}",
            group_id, if is_expanded { "展开" } else { "折叠" }
        );
    }

    fn on_host_asset_tree_reordered(&self, src_id: &str, target_id: &str, position: &str) {
        tracing::info!(
            target: "audit::host",
            "[资产拓扑] 节点 [{}] 移动至 [{}] 模式: {}",
            src_id, target_id, position
        );
    }

    fn on_host_terminal_opened(&self, session_id: &str, ctx: &crate::domain::ActiveTerminalSessionContext) {
        tracing::info!(
            target: "audit::terminal",
            "[会话审计:建立] 会话 ID: [{}], 目标主机: '{}' ({}:{}), 登录用户: '{}'",
            session_id, ctx.host_name, ctx.host_ip, ctx.port, ctx.username
        );
    }

    fn on_host_terminal_closed(&self, session_id: &str, duration_secs: u64) {
        tracing::info!(
            target: "audit::terminal",
            "[会话审计:关闭] 会话 ID: [{}], 在线持续时长: {} 秒",
            session_id, duration_secs
        );
    }

    fn on_history_item_deleted(&self, history_id: &str) {
        tracing::info!(
            target: "audit::history",
            "[历史审计:删除] 历史记录 ID: [{}]",
            history_id
        );
    }

    fn on_history_cleared(&self) {
        tracing::warn!(
            target: "audit::history",
            "[历史审计:清空] 用户清空了非置顶历史会话记录"
        );
    }
}


type ConfigCallback = Arc<dyn Fn(&ConfigChangeEvent) + Send + Sync>;
type LeftMenuCallback = Arc<dyn Fn(&str, &str) + Send + Sync>;
type BeforeExitCallback = Arc<dyn Fn(&AppExitContext) -> HookDecision + Send + Sync>;

/// 函数式轻量闭包单事件监听适配器。
///
/// 允许通过 Rust 闭包快速挂载单个全局事件监听，无需手写完整结构体。
pub struct FunctionalGlobalHook {
    name: &'static str,
    priority: i32,
    on_config_changed_fn: Option<ConfigCallback>,
    on_left_menu_fn: Option<LeftMenuCallback>,
    on_before_exit_fn: Option<BeforeExitCallback>,
}


impl FunctionalGlobalHook {
    /// 构造一个针对参数变动的轻量闭包 Hook。
    pub fn for_config_changed<F>(name: &'static str, f: F) -> Self
    where
        F: Fn(&ConfigChangeEvent) + Send + Sync + 'static,
    {
        Self {
            name,
            priority: 0,
            on_config_changed_fn: Some(Arc::new(f)),
            on_left_menu_fn: None,
            on_before_exit_fn: None,
        }
    }

    /// 构造一个针对左侧菜单切换的轻量闭包 Hook。
    pub fn for_left_menu<F>(name: &'static str, f: F) -> Self
    where
        F: Fn(&str, &str) + Send + Sync + 'static,
    {
        Self {
            name,
            priority: 0,
            on_config_changed_fn: None,
            on_left_menu_fn: Some(Arc::new(f)),
            on_before_exit_fn: None,
        }
    }

    /// 构造一个针对应用关闭前的拦截闭包 Hook。
    pub fn for_before_exit<F>(name: &'static str, priority: i32, f: F) -> Self
    where
        F: Fn(&AppExitContext) -> HookDecision + Send + Sync + 'static,
    {
        Self {
            name,
            priority,
            on_config_changed_fn: None,
            on_left_menu_fn: None,
            on_before_exit_fn: Some(Arc::new(f)),
        }
    }
}

impl AppGlobalHook for FunctionalGlobalHook {
    fn name(&self) -> &'static str {
        self.name
    }

    fn priority(&self) -> i32 {
        self.priority
    }

    fn on_config_changed(&self, event: &ConfigChangeEvent) {
        if let Some(ref f) = self.on_config_changed_fn {
            f(event);
        }
    }

    fn on_shell_left_menu_clicked(&self, menu_id: &str, old_menu_id: &str) {
        if let Some(ref f) = self.on_left_menu_fn {
            f(menu_id, old_menu_id);
        }
    }

    fn on_app_before_exit(&self, ctx: &AppExitContext) -> HookDecision {

        if let Some(ref f) = self.on_before_exit_fn {
            f(ctx)
        } else {
            HookDecision::Continue
        }
    }
}
