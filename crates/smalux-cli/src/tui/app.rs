//! # Smalux TUI - 应用状态机模型 (App State)

use std::collections::HashMap;
use std::sync::Arc;
use anyhow::Result;
use smagical_core::domain::HostRecord;
use smagical_core::storage::AppStorage;
use smagical_ssh::RusshSessionDriver;

/// TUI 运行状态
pub struct App {
    /// 统一存储持久层句柄
    pub storage: Arc<dyn AppStorage>,
    /// 纯 Rust SSH 协议驱动引擎
    pub ssh_svc: Arc<RusshSessionDriver>,

    /// 当前加载的所有主机列表
    pub hosts: Vec<HostRecord>,
    /// 分组映射 (group_id -> group_name)
    pub group_names: HashMap<String, String>,

    /// 当前选中的列表光标索引
    pub selected_index: usize,
    /// 搜索过滤词
    pub search_query: String,
    /// 是否处于搜索输入状态
    pub is_searching: bool,

    /// 保险库是否处于锁定状态 (需要弹出主密码解锁遮罩)
    pub is_vault_locked: bool,
    /// 主密码输入缓冲区
    pub unlock_input: String,
    /// 解锁错误提示
    pub unlock_error: Option<String>,

    /// 底部状态栏临时提示消息
    pub status_message: Option<String>,
    /// 是否请求退出 TUI
    pub should_quit: bool,
    /// 是否请求连接指定主机 (由 TUI 外层事件循环脱离 AlternateScreen 执行原生 SSH)
    pub connect_target: Option<HostRecord>,
}

impl App {
    /// 初始化 TUI 应用状态
    pub async fn new(
        storage: Arc<dyn AppStorage>,
        ssh_svc: Arc<RusshSessionDriver>,
        cli_password_opt: Option<&str>,
    ) -> Result<Self> {
        let has_custom = storage
            .has_custom_master_password()
            .await
            .unwrap_or(false);

        let mut is_vault_locked = false;

        if has_custom && !storage.is_vault_unlocked() {
            // 尝试通过命令行参数或环境变量直接预解锁
            let mut auto_unlocked = false;
            let candidate_pwd = cli_password_opt
                .map(|s| s.to_string())
                .or_else(|| std::env::var("SMALUX_MASTER_PASSWORD").ok());

            if let Some(pwd) = candidate_pwd {
                if let Ok(true) = storage.unlock_vault(&pwd).await {
                    auto_unlocked = true;
                }
            }

            if !auto_unlocked {
                is_vault_locked = true;
            }
        }

        let mut app = Self {
            storage,
            ssh_svc,
            hosts: Vec::new(),
            group_names: HashMap::new(),
            selected_index: 0,
            search_query: String::new(),
            is_searching: false,
            is_vault_locked,
            unlock_input: String::new(),
            unlock_error: None,
            status_message: None,
            should_quit: false,
            connect_target: None,
        };

        if !app.is_vault_locked {
            let _ = app.reload_data().await;
        }

        Ok(app)
    }

    /// 重新从存储层拉取最新的主机与分组资产
    pub async fn reload_data(&mut self) -> Result<()> {
        let hosts = self.storage.hosts().list_all().await.unwrap_or_default();
        let groups = self.storage.groups().list_all().await.unwrap_or_default();

        let mut map = HashMap::new();
        for g in groups {
            map.insert(g.id, g.name);
        }

        self.hosts = hosts;
        self.group_names = map;

        if self.selected_index >= self.filtered_hosts().len() && !self.hosts.is_empty() {
            self.selected_index = self.hosts.len() - 1;
        }

        Ok(())
    }

    /// 获取根据搜索关键词过滤后的主机列表
    pub fn filtered_hosts(&self) -> Vec<&HostRecord> {
        if self.search_query.trim().is_empty() {
            self.hosts.iter().collect()
        } else {
            let q = self.search_query.to_lowercase();
            self.hosts
                .iter()
                .filter(|h| {
                    h.name.to_lowercase().contains(&q)
                        || h.address.to_lowercase().contains(&q)
                        || h.username.as_deref().unwrap_or("").to_lowercase().contains(&q)
                })
                .collect()
        }
    }

    /// 获取当前选中的主机记录引用
    pub fn selected_host(&self) -> Option<&HostRecord> {
        let filtered = self.filtered_hosts();
        if filtered.is_empty() {
            None
        } else {
            Some(filtered[self.selected_index.min(filtered.len() - 1)])
        }
    }

    /// 光标向下移动一行
    pub fn next(&mut self) {
        let len = self.filtered_hosts().len();
        if len > 0 {
            self.selected_index = (self.selected_index + 1) % len;
        }
    }

    /// 光标向上移动一行
    pub fn previous(&mut self) {
        let len = self.filtered_hosts().len();
        if len > 0 {
            if self.selected_index == 0 {
                self.selected_index = len - 1;
            } else {
                self.selected_index -= 1;
            }
        }
    }

    /// 尝试使用当前输入的文本解锁保险库
    pub async fn try_unlock(&mut self) -> Result<bool> {
        let pwd = self.unlock_input.clone();
        match self.storage.unlock_vault(&pwd).await {
            Ok(true) => {
                self.is_vault_locked = false;
                self.unlock_error = None;
                self.unlock_input.clear();
                self.reload_data().await?;
                self.status_message = Some("✅ 保险库已成功解锁，资产就绪".to_string());
                Ok(true)
            }
            Ok(false) => {
                self.unlock_error = Some("主密码错误，请重新输入".to_string());
                self.unlock_input.clear();
                Ok(false)
            }
            Err(e) => {
                self.unlock_error = Some(format!("解锁异常: {}", e));
                self.unlock_input.clear();
                Ok(false)
            }
        }
    }

    /// 立即锁定保险库并抹除内存密钥
    pub fn lock_vault(&mut self) {
        self.storage.lock_vault();
        self.is_vault_locked = true;
        self.hosts.clear();
        self.status_message = Some("🔒 保险库已锁定，内存密钥已安全清除".to_string());
    }
}
