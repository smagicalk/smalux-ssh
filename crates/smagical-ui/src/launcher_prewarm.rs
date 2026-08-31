//! 快速新建会话中心与启动器后台异步预热 Hook。
//!
//! 在应用启动就绪 (on_app_ready)、左侧主机资产抽屉切换 (on_left_menu_clicked) 以及资产数据变更时，
//! 在后台独立工作线程中异步将存储层的主机与分组数据转换为 Slint 展示模型并推送至 UI，
//! 彻底消除点击 + 号新建终端会话弹窗时的任何主 UI 线程同步计算与掉帧卡顿。

use std::sync::{Arc, RwLock};
use smagical_core::{AppGlobalHook, AppStorage, ConfigChangeEvent};
use crate::generated::{AppWindow, HostItemData};

/// 快速新建终端启动器后台异步预热 Hook
pub struct LauncherPrewarmHook {
    storage: Arc<dyn AppStorage>,
    window_weak: slint::Weak<AppWindow>,
    is_prewarming: Arc<RwLock<bool>>,
}

impl LauncherPrewarmHook {
    /// 创建一个新的启动器异步预热 Hook 实例
    pub fn new(
        storage: Arc<dyn AppStorage>,
        window_weak: slint::Weak<AppWindow>,
    ) -> Self {
        Self {
            storage,
            window_weak,
            is_prewarming: Arc::new(RwLock::new(false)),
        }
    }

    /// 在后台独立线程中异步拉取并推送最新主机资产列表到弹窗启动器模型
    pub fn trigger_async_prewarm(&self) {
        let storage = Arc::clone(&self.storage);
        let window_weak = self.window_weak.clone();
        let prewarming_flag = Arc::clone(&self.is_prewarming);

        // 防止多事件并发重叠触发冗余计算
        if let Ok(mut flag) = prewarming_flag.write() {
            if *flag {
                return;
            }
            *flag = true;
        }

        std::thread::Builder::new()
            .name("launcher-prewarmer".into())
            .spawn(move || {
                tracing::debug!(target: "smagical_ui::launcher", "开始在后台工作线程异步预热启动器主机数据...");
                
                let all_hosts = storage.hosts().list_all().unwrap_or_default();
                let all_groups = storage.groups().list_all().unwrap_or_default();

                let prewarmed_cards: Vec<HostItemData> = all_hosts
                    .into_iter()
                    .map(|h| {
                        let group_name = h
                            .parent_group_id
                            .as_deref()
                            .and_then(|p_id| all_groups.iter().find(|g| g.id == p_id).map(|g| g.name.clone()))
                            .unwrap_or_else(|| "未分组".to_string());
                        HostItemData {
                            id: h.id.into(),
                            name: h.name.into(),
                            address: h.address.into(),
                            port: h.port as i32,
                            group: group_name.into(),
                            status: h.status.to_string().into(),
                            ping_ms: h.ping_ms,
                        }
                    })
                    .collect();

                tracing::debug!(target: "smagical_ui::launcher", "启动器主机数据预热完成，共 {} 台主机，正在异步回推 UI 事件循环...", prewarmed_cards.len());

                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = window_weak.upgrade() {
                        w.set_launcher_host_items(slint::ModelRc::from(std::rc::Rc::new(
                            slint::VecModel::from(prewarmed_cards),
                        )));
                    }
                });

                if let Ok(mut flag) = prewarming_flag.write() {
                    *flag = false;
                }
            })
            .ok();
    }
}

impl AppGlobalHook for LauncherPrewarmHook {
    fn name(&self) -> &'static str {
        "launcher_prewarm_hook"
    }

    fn priority(&self) -> i32 {
        40
    }

    /// 应用首帧就绪时：在后台空闲期自动启动初次预热
    fn on_app_ready(&self) {
        self.trigger_async_prewarm();
    }

    /// 用户在左侧活动栏切换抽屉时：若进入或切换主机资产页面，静默同步预热
    fn on_left_menu_clicked(&self, menu_id: &str, _old_menu_id: &str) {
        if menu_id == "hosts" || menu_id.is_empty() {
            self.trigger_async_prewarm();
        }
    }

    /// 全局配置或资产变动时：静默更新预热模型
    fn on_global_config_changed(&self, event: &ConfigChangeEvent) {
        if event.affects("hosts") || event.affects("storage") {
            self.trigger_async_prewarm();
        }
    }
}
