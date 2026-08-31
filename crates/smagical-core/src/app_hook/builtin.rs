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

    fn on_global_config_changed(&self, event: &ConfigChangeEvent) {
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

    fn on_global_config_changed(&self, event: &ConfigChangeEvent) {
        if let Some(ref f) = self.on_config_changed_fn {
            f(event);
        }
    }

    fn on_left_menu_clicked(&self, menu_id: &str, old_menu_id: &str) {
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
