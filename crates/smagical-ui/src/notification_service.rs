//! 全局气泡通知 (Toast / Notification) 管理服务。
//!
//! 提供非阻塞、多方位堆叠、自动倒计时消隐与手动关闭的轻量通知机制。

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use crate::generated::{AppWindow, ToastItemData};

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
    toasts: Rc<RefCell<Vec<ToastNotification>>>,
    window: slint::Weak<AppWindow>,
    position: Rc<RefCell<String>>,
}

impl NotificationManager {
    /// 创建并初始化全局通知管理器实例
    pub fn new(window: slint::Weak<AppWindow>) -> Self {
        Self {
            toasts: Rc::new(RefCell::new(Vec::new())),
            window,
            position: Rc::new(RefCell::new("top-right".to_string())),
        }
    }

    /// 设置默认通知展示方位
    pub fn set_position(&self, position: &str) {
        *self.position.borrow_mut() = position.to_string();
        if let Some(w) = self.window.upgrade() {
            w.set_toast_position(position.into());
        }
    }

    /// 显示自定义气泡通知
    pub fn show(&self, toast: ToastNotification) {
        let toast_id = toast.id.clone();
        let duration_ms = toast.duration_ms;

        {
            let mut list = self.toasts.borrow_mut();
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
            slint::Timer::single_shot(Duration::from_millis(duration_ms), move || {
                manager.close(&id_clone);
            });
        }
    }

    /// 快捷显示成功通知 (默认 2200ms)
    pub fn success(&self, title: impl Into<String>, message: impl Into<String>) {
        let id = format!("toast-{}", uuid::Uuid::new_v4());
        self.show(ToastNotification {
            id,
            title: title.into(),
            message: message.into(),
            level: "success".into(),
            position: self.position.borrow().clone(),
            duration_ms: 2200,
            closable: true,
        });
    }

    /// 快捷显示消息通知 (默认 2000ms)
    pub fn info(&self, title: impl Into<String>, message: impl Into<String>) {
        let id = format!("toast-{}", uuid::Uuid::new_v4());
        self.show(ToastNotification {
            id,
            title: title.into(),
            message: message.into(),
            level: "info".into(),
            position: self.position.borrow().clone(),
            duration_ms: 2000,
            closable: true,
        });
    }

    /// 快捷显示警告通知 (默认 2800ms)
    pub fn warning(&self, title: impl Into<String>, message: impl Into<String>) {
        let id = format!("toast-{}", uuid::Uuid::new_v4());
        self.show(ToastNotification {
            id,
            title: title.into(),
            message: message.into(),
            level: "warning".into(),
            position: self.position.borrow().clone(),
            duration_ms: 2800,
            closable: true,
        });
    }

    /// 快捷显示错误通知 (默认 3000ms)
    pub fn error(&self, title: impl Into<String>, message: impl Into<String>) {
        let id = format!("toast-{}", uuid::Uuid::new_v4());
        self.show(ToastNotification {
            id,
            title: title.into(),
            message: message.into(),
            level: "error".into(),
            position: self.position.borrow().clone(),
            duration_ms: 3000,
            closable: true,
        });
    }

    /// 手动关闭指定通知
    pub fn close(&self, id: &str) {
        let mut list = self.toasts.borrow_mut();
        let before_len = list.len();
        list.retain(|t| t.id != id);
        if list.len() != before_len {
            drop(list);
            self.sync_ui();
        }
    }

    /// 清空所有通知
    pub fn clear_all(&self) {
        self.toasts.borrow_mut().clear();
        self.sync_ui();
    }

    /// 同步当前通知列表至 Slint UI
    fn sync_ui(&self) {
        if let Some(w) = self.window.upgrade() {
            let list = self.toasts.borrow();
            let ui_toasts: Vec<ToastItemData> = list
                .iter()
                .map(|t| ToastItemData {
                    id: t.id.clone().into(),
                    title: t.title.clone().into(),
                    message: t.message.clone().into(),
                    level: t.level.clone().into(),
                    position: t.position.clone().into(),
                    duration_ms: t.duration_ms as i32,
                    closable: t.closable,
                })
                .collect();
            w.set_active_toasts(slint::ModelRc::from(Rc::new(slint::VecModel::from(ui_toasts))));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_manager_lifecycle() {
        let mgr = NotificationManager {
            toasts: Rc::new(RefCell::new(Vec::new())),
            window: slint::Weak::default(),
            position: Rc::new(RefCell::new("top-right".to_string())),
        };

        // 1. 弹出消息
        mgr.info("提示", "这是一条测试消息");
        assert_eq!(mgr.toasts.borrow().len(), 1);
        assert_eq!(mgr.toasts.borrow()[0].level, "info");
        assert_eq!(mgr.toasts.borrow()[0].title, "提示");

        // 2. 连续推入直到超过最大限制 5 条
        for i in 1..=6 {
            mgr.success(format!("成功 {}", i), "完成");
        }
        assert_eq!(mgr.toasts.borrow().len(), 5);
        assert_eq!(mgr.toasts.borrow().last().unwrap().title, "成功 6");

        // 3. 关闭指定通知
        let target_id = mgr.toasts.borrow()[0].id.clone();
        mgr.close(&target_id);
        assert_eq!(mgr.toasts.borrow().len(), 4);
        assert!(mgr.toasts.borrow().iter().all(|t| t.id != target_id));

        // 4. 清空所有通知
        mgr.clear_all();
        assert_eq!(mgr.toasts.borrow().len(), 0);
    }
}

