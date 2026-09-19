//! 全局气泡通知 (Toast / Notification) 管理服务。
//!
//! 提供非阻塞、多方位堆叠、自动倒计时消隐与手动关闭的轻量通知机制。

use std::sync::{Arc, Mutex};
use std::time::Duration;

use slint::ComponentHandle;
use crate::generated::{AppWindow, ToastItemData, WindowBridge};

/// 气泡通知业务项
#[derive(Debug, Clone)]
pub struct ToastNotification {
    /// 唯一标识 ID
    pub id: String,
    /// 提示标题
    pub title: String,
    /// 详细提示正文消息
    pub message: String,
    /// 提示级别 ("info" | "success" | "warning" | "error")
    pub level: String,
    /// 弹出位置 ("top-right" | "top-center" | "bottom-right")
    pub position: String,
    /// 自动消隐倒计时毫秒数 (0 为常驻不自动关闭)
    pub duration_ms: u64,
    /// 是否显示右上角关闭按钮
    pub closable: bool,
}

/// 全局通知管理器
#[derive(Clone)]
pub struct NotificationManager {
    toasts: Arc<Mutex<Vec<ToastNotification>>>,
    window: slint::Weak<AppWindow>,
    position: Arc<Mutex<String>>,
    duration_preset: Arc<Mutex<String>>,
}

impl NotificationManager {
    /// 创建并初始化全局通知管理器实例
    pub fn new(window: slint::Weak<AppWindow>) -> Self {
        Self {
            toasts: Arc::new(Mutex::new(Vec::new())),
            window,
            position: Arc::new(Mutex::new("top-right".to_string())),
            duration_preset: Arc::new(Mutex::new("3".to_string())),
        }
    }

    /// 设置提示信息停留时间预设 ("1.5", "3", "5", "8", "never")
    pub fn set_duration_preset(&self, preset: &str) {
        *self.duration_preset.lock().unwrap() = preset.to_string();
    }

    /// 获取当前基础停留毫秒数 (0 为常驻手动关闭)
    pub fn base_duration_ms(&self) -> u64 {
        match self.duration_preset.lock().unwrap().as_str() {
            "1.5" => 1500,
            "3" => 3000,
            "5" => 5000,
            "8" => 8000,
            "never" | "0" => 0,
            _ => 3000,
        }
    }

    /// 设置默认通知展示方位
    pub fn set_position(&self, position: &str) {
        *self.position.lock().unwrap() = position.to_string();
        let pos_str = position.to_string();
        let window = self.window.clone();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(w) = window.upgrade() {
                w.global::<WindowBridge>().set_toast_position(pos_str.into());
            }
        });
    }

    /// 显示自定义气泡通知
    pub fn show(&self, toast: ToastNotification) {
        let toast_id = toast.id.clone();
        let duration_ms = toast.duration_ms;

        {
            let mut list = self.toasts.lock().unwrap();
            if let Some(pos) = list.iter().position(|t| t.id == toast_id) {
                list[pos] = toast;
            } else {
                if list.len() >= 5 {
                    list.remove(0);
                }
                list.push(toast);
            }
        }

        self.sync_ui();

        // 启动自动消隐定时器 (duration_ms > 0)
        if duration_ms > 0 {
            let manager = self.clone();
            let id_clone = toast_id.clone();
            let _ = slint::invoke_from_event_loop(move || {
                slint::Timer::single_shot(Duration::from_millis(duration_ms), move || {
                    manager.close(&id_clone);
                });
            });
        }
    }

    /// 快捷显示成功通知 (遵循用户设定的停留时间)
    pub fn success(&self, title: impl Into<String>, message: impl Into<String>) {
        let id = format!("toast-{}", uuid::Uuid::new_v4());
        let base_ms = self.base_duration_ms();
        let position = self.position.lock().unwrap().clone();
        self.show(ToastNotification {
            id,
            title: title.into(),
            message: message.into(),
            level: "success".into(),
            position,
            duration_ms: base_ms,
            closable: true,
        });
    }

    /// 快捷显示消息通知 (遵循用户设定的停留时间)
    pub fn info(&self, title: impl Into<String>, message: impl Into<String>) {
        let id = format!("toast-{}", uuid::Uuid::new_v4());
        let base_ms = self.base_duration_ms();
        let position = self.position.lock().unwrap().clone();
        self.show(ToastNotification {
            id,
            title: title.into(),
            message: message.into(),
            level: "info".into(),
            position,
            duration_ms: base_ms,
            closable: true,
        });
    }

    /// 快捷显示警告通知 (在基础时间上适当多停留 500ms)
    pub fn warning(&self, title: impl Into<String>, message: impl Into<String>) {
        let id = format!("toast-{}", uuid::Uuid::new_v4());
        let base_ms = self.base_duration_ms();
        let duration_ms = if base_ms == 0 { 0 } else { base_ms + 500 };
        let position = self.position.lock().unwrap().clone();
        self.show(ToastNotification {
            id,
            title: title.into(),
            message: message.into(),
            level: "warning".into(),
            position,
            duration_ms,
            closable: true,
        });
    }

    /// 快捷显示错误通知 (在基础时间上额外多停留 1000ms 便于阅读诊断报错)
    pub fn error(&self, title: impl Into<String>, message: impl Into<String>) {
        let id = format!("toast-{}", uuid::Uuid::new_v4());
        let base_ms = self.base_duration_ms();
        let duration_ms = if base_ms == 0 { 0 } else { base_ms + 1000 };
        let position = self.position.lock().unwrap().clone();
        self.show(ToastNotification {
            id,
            title: title.into(),
            message: message.into(),
            level: "error".into(),
            position,
            duration_ms,
            closable: true,
        });
    }

    /// 手动关闭指定通知
    pub fn close(&self, id: &str) {
        let mut list = self.toasts.lock().unwrap();
        let before_len = list.len();
        list.retain(|t| t.id != id);
        if list.len() != before_len {
            drop(list);
            self.sync_ui();
        }
    }

    /// 清空所有通知
    pub fn clear_all(&self) {
        self.toasts.lock().unwrap().clear();
        self.sync_ui();
    }

    /// 同步当前通知列表至 Slint UI
    fn sync_ui(&self) {
        let list = self.toasts.lock().unwrap().clone();
        let window = self.window.clone();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(w) = window.upgrade() {
                let ui_toasts: Vec<ToastItemData> = list
                    .into_iter()
                    .map(|t| ToastItemData {
                        id: t.id.into(),
                        title: t.title.into(),
                        message: t.message.into(),
                        level: t.level.into(),
                        position: t.position.into(),
                        duration_ms: t.duration_ms as i32,
                        closable: t.closable,
                    })
                    .collect();
                w.global::<WindowBridge>().set_toasts(slint::ModelRc::from(std::rc::Rc::new(slint::VecModel::from(ui_toasts))));
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_manager_lifecycle() {
        let mgr = NotificationManager::new(slint::Weak::default());

        // 1. 弹出消息
        mgr.info("提示", "这是一条测试消息");
        assert_eq!(mgr.toasts.lock().unwrap().len(), 1);
        assert_eq!(mgr.toasts.lock().unwrap()[0].level, "info");
        assert_eq!(mgr.toasts.lock().unwrap()[0].title, "提示");

        // 2. 连续推入直到超过最大限制 5 条
        for i in 1..=6 {
            mgr.success(format!("成功 {}", i), "完成");
        }
        assert_eq!(mgr.toasts.lock().unwrap().len(), 5);
        assert_eq!(mgr.toasts.lock().unwrap().last().unwrap().title, "成功 6");

        // 3. 关闭指定通知
        let target_id = mgr.toasts.lock().unwrap()[0].id.clone();
        mgr.close(&target_id);
        assert_eq!(mgr.toasts.lock().unwrap().len(), 4);
        assert!(mgr.toasts.lock().unwrap().iter().all(|t| t.id != target_id));

        // 4. 清空所有通知
        mgr.clear_all();
        assert_eq!(mgr.toasts.lock().unwrap().len(), 0);
    }

    #[test]
    fn test_notification_duration_presets() {
        let mgr = NotificationManager::new(slint::Weak::default());
        assert_eq!(mgr.base_duration_ms(), 3000);

        mgr.set_duration_preset("1.5");
        assert_eq!(mgr.base_duration_ms(), 1500);

        mgr.set_duration_preset("5");
        assert_eq!(mgr.base_duration_ms(), 5000);

        mgr.set_duration_preset("8");
        assert_eq!(mgr.base_duration_ms(), 8000);

        mgr.set_duration_preset("never");
        assert_eq!(mgr.base_duration_ms(), 0);

        mgr.info("常驻", "常驻消息");
        assert_eq!(mgr.toasts.lock().unwrap()[0].duration_ms, 0);

        mgr.warning("常驻告警", "常驻告警消息");
        assert_eq!(mgr.toasts.lock().unwrap()[1].duration_ms, 0);

        mgr.set_duration_preset("3");
        mgr.warning("普通告警", "普通告警消息");
        assert_eq!(mgr.toasts.lock().unwrap()[2].duration_ms, 3500);

        mgr.error("普通错误", "普通错误消息");
        assert_eq!(mgr.toasts.lock().unwrap()[3].duration_ms, 4000);
    }
}

