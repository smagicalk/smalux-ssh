//! 容灾备份与多端快照同步全局后台守护进程 (Backup Daemon Service)。
//!
//! # 核心职责与治理机制
//! 1. **冷启动健康检查与遗漏补跑**：响应 `AppReadyEvent`，延迟 10 秒唤醒，规避冷启动 CPU/IO 争抢，自动补跑距离上次执行超时的 `hourly`/`daily` 计划任务；
//! 2. **5 秒防抖实时增量监听**：响应资产配置与密钥凭据变更事件 (`HostAssetChangedEvent`, `ConfigChangedEvent`, `CredentialSavedEvent` 等)，启动 5 秒滑动时间窗聚合防抖；
//! 3. **软件安全退出拦截保护**：监听 `AppBeforeExitEvent`，在 4 秒受控超时内强制推送 `OnExit` 策略同步任务，确保关机不丢资产；
//! 4. **周期巡检与精准调度**：后台每 30 秒进行一次时钟滴答，驱动周期性快照与失败任务退避重试；
//! 5. **智能指数退避重试与 401 熔断**：网络故障采用 `30s -> 2m -> 10m` 阶梯退避，遇 401 凭据鉴权失效时立即熔断并弹窗告警，杜绝无限无效重试；
//! 6. **脱机安全兜底 (.stash/)**：网络不可用时自动将生成的 AES-256 加密快照暂存至本地磁盘，待网络恢复后回传；
//! 7. **快照保留生命周期 (Retention) 修剪**：自动依据任务设定的保留数量，物理修剪远端及数据库中的过期陈旧历史快照。

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use slint::ComponentHandle;
use crate::common::{run_on_ui, to_model_rc};
use smagical_core::domain::backup::{BackupSnapshotRecord, BackupStrategy, BackupTaskRecord};
use smagical_core::event::{
    AppBeforeExitEvent, AppReadyEvent, ConfigChangedEvent, CredentialDeletedEvent,
    CredentialSavedEvent, EventManager, HostAssetChangedEvent, SnippetDeletedEvent,
    SnippetSavedEvent, TunnelDeletedEvent, TunnelSavedEvent,
};
use smagical_core::service::backup::{
    create_backup_driver, create_backup_payload_with_encryption, format_bytes_size,
};
use smagical_core::storage::AppStorage;
use tokio::sync::Mutex;

use crate::generated::{AppWindow, BackupTaskItem, SettingsBridge};
use crate::notification_service::NotificationManager;

/// 单个备份任务的失败退避重试状态跟踪。
#[derive(Debug, Clone)]
struct RetryState {
    /// 当前已连续失败的尝试次数 (从 1 起算，达到 3 次后放弃重试)
    attempt: u32,
    /// 依据指数退避算法计算得到的下一次允许执行的绝对时间戳
    next_retry_at: Instant,
}

/// 容灾备份与多端快照同步全局常驻后台守护服务。
pub struct BackupDaemonService {
    /// 核心持久化存储层门面 (包含主机、凭据、快照与任务实体仓储)
    storage: Arc<dyn AppStorage>,
    /// Slint UI 主窗口弱引用句柄，用于向前端界面投递刷新请求
    window_weak: slint::Weak<AppWindow>,
    /// 全局桌面 Toast 气泡通知服务管理器
    notifications: NotificationManager,
    /// 任务重试状态表 (`task_id` -> `RetryState`)，采用异步锁保护
    retries: Arc<Mutex<HashMap<String, RetryState>>>,
    /// 5 秒防抖时间窗原子标志位 (`true` 表示防抖延时等待中)
    is_debouncing: Arc<AtomicBool>,
}

impl BackupDaemonService {
    /// 构造全新的容灾备份后台守护服务实例。
    ///
    /// # 参数
    /// - `storage`: 底层持久化存储门面 Arc 句柄；
    /// - `window_weak`: Slint UI 主窗口弱引用句柄；
    /// - `notifications`: 全局气泡通知服务。
    pub fn new(
        storage: Arc<dyn AppStorage>,
        window_weak: slint::Weak<AppWindow>,
        notifications: NotificationManager,
    ) -> Self {
        Self {
            storage,
            window_weak,
            notifications,
            retries: Arc::new(Mutex::new(HashMap::new())),
            is_debouncing: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 将守护进程注册并接入整个应用程序的事件总线。
    ///
    /// # 注册机制
    /// 1. 注册冷启动健康探测 (`AppReadyEvent`)；
    /// 2. 注册退出安全拦截 (`AppBeforeExitEvent`)；
    /// 3. 注册领域实体变更实时监听 (`ConfigChangedEvent`, `HostAssetChangedEvent`, `CredentialSavedEvent` 等)；
    /// 4. 派生每 30 秒运行一次的后台周期性巡检协程。
    ///
    /// # 参数
    /// - `self`: 守护进程的 `Arc` 智能指针；
    /// - `events`: 核心状态层全局事件分发器句柄引用。
    pub fn register(self: Arc<Self>, events: &EventManager) {
        // 1. 冷启动探测与调度补跑 (延迟 10 秒执行，避免启动资源争抢)
        let s_ready = Arc::clone(&self);
        let g_ready = events.global().listen(move |_: &AppReadyEvent| {
            let daemon = Arc::clone(&s_ready);
            crate::async_util::spawn_async(async move {
                tokio::time::sleep(Duration::from_secs(10)).await;
                daemon.handle_app_startup_health_check().await;
            });
        });
        g_ready.detach();

        // 2. 软件退出前夕安全同步 (拦截退出并在限时内推送 on_exit 任务)
        let s_exit = Arc::clone(&self);
        let g_exit = events.global().listen(move |_: &AppBeforeExitEvent| {
            let daemon = Arc::clone(&s_exit);
            crate::async_util::spawn_async(async move {
                let _ = tokio::time::timeout(
                    Duration::from_secs(4),
                    daemon.handle_app_before_exit(),
                ).await;
            });
        });
        g_exit.detach();

        // 3. 5 秒防抖实时备份监听
        let bind_realtime = |daemon: Arc<Self>| {
            daemon.schedule_debounced_realtime_backup();
        };

        let s_cfg = Arc::clone(&self);
        let g_cfg = events.global().listen(move |_: &ConfigChangedEvent| {
            bind_realtime(Arc::clone(&s_cfg));
        });
        g_cfg.detach();

        let s_host = Arc::clone(&self);
        let g_host = events.global().listen(move |_: &HostAssetChangedEvent| {
            bind_realtime(Arc::clone(&s_host));
        });
        g_host.detach();

        let s_cs = Arc::clone(&self);
        let g_cs = events.global().listen(move |_: &CredentialSavedEvent| {
            bind_realtime(Arc::clone(&s_cs));
        });
        g_cs.detach();

        let s_cd = Arc::clone(&self);
        let g_cd = events.global().listen(move |_: &CredentialDeletedEvent| {
            bind_realtime(Arc::clone(&s_cd));
        });
        g_cd.detach();

        let s_ss = Arc::clone(&self);
        let g_ss = events.global().listen(move |_: &SnippetSavedEvent| {
            bind_realtime(Arc::clone(&s_ss));
        });
        g_ss.detach();

        let s_sd = Arc::clone(&self);
        let g_sd = events.global().listen(move |_: &SnippetDeletedEvent| {
            bind_realtime(Arc::clone(&s_sd));
        });
        g_sd.detach();

        let s_ts = Arc::clone(&self);
        let g_ts = events.global().listen(move |_: &TunnelSavedEvent| {
            bind_realtime(Arc::clone(&s_ts));
        });
        g_ts.detach();

        let s_td = Arc::clone(&self);
        let g_td = events.global().listen(move |_: &TunnelDeletedEvent| {
            bind_realtime(Arc::clone(&s_td));
        });
        g_td.detach();

        // 4. 启动周期巡检定时线程 (每 30 秒检查 hourly/daily 与待重试任务)
        let s_ticker = Arc::clone(&self);
        crate::async_util::spawn_async(async move {
            s_ticker.start_periodic_ticker().await;
        });

        tracing::info!(target: "smagical_ui::backup", "容灾备份常驻后台守护服务已注册就绪");
    }

    /// 触发防抖实时备份任务流水线。
    ///
    /// 开启 5 秒窗口防抖，若在此期间连续产生配置或凭据修改事件，将自动合流为单次增量备份，
    /// 避免在批量导入或高频编辑时对远端服务器发起快照轰炸。
    fn schedule_debounced_realtime_backup(&self) {
        if self.is_debouncing.swap(true, Ordering::SeqCst) {
            // 已有等待任务，防抖合并
            return;
        }

        let is_debouncing = Arc::clone(&self.is_debouncing);
        let storage = Arc::clone(&self.storage);
        let daemon_retries = Arc::clone(&self.retries);
        let window_weak = self.window_weak.clone();
        let notif = self.notifications.clone();

        crate::async_util::spawn_async(async move {
            tokio::time::sleep(Duration::from_secs(5)).await;
            is_debouncing.store(false, Ordering::SeqCst);

            // 获取全部启用的 realtime 备份任务并执行
            if let Ok(tasks) = storage.backup_tasks().list_all().await {
                let realtime_tasks: Vec<_> = tasks
                    .into_iter()
                    .filter(|t| t.enabled && t.strategy == BackupStrategy::Realtime)
                    .collect();

                if !realtime_tasks.is_empty() {
                    tracing::info!(target: "smagical_ui::backup", "防抖触发: 正在执行 {} 个实时备份任务", realtime_tasks.len());
                    for task in realtime_tasks {
                        Self::execute_task_pipeline(
                            task,
                            Arc::clone(&storage),
                            Arc::clone(&daemon_retries),
                            window_weak.clone(),
                            notif.clone(),
                            false,
                        ).await;
                    }
                }
            }
        });
    }

    /// 冷启动时对周期性计划备份任务进行健康巡检与遗漏补跑。
    ///
    /// 若用户在关机期间错过了预定的每小时 (`Hourly`) 或每日 (`Daily`) 备份，冷启动时将自动识别并立即补跑一次。
    async fn handle_app_startup_health_check(&self) {
        let tasks = match self.storage.backup_tasks().list_all().await {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!(target: "smagical_ui::backup", "冷启动查询备份任务失败: {}", e);
                return;
            }
        };

        let now_epoch = chrono::Utc::now().timestamp() as u64;
        let today_str = chrono::Utc::now().format("%Y-%m-%d").to_string();

        for task in tasks {
            if !task.enabled {
                continue;
            }

            let need_run = match task.strategy {
                BackupStrategy::Hourly => {
                    // 若从未执行或最后执行时间超过 1 小时
                    if task.last_backup_time.is_empty() || task.last_backup_time == "从未执行" {
                        true
                    } else if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&task.last_backup_time, "%Y-%m-%d %H:%M:%S") {
                        let diff = now_epoch.saturating_sub(dt.and_utc().timestamp() as u64);
                        diff >= 3600
                    } else {
                        false
                    }
                }
                BackupStrategy::Daily => {
                    // 若今天尚未执行
                    if task.last_backup_time.is_empty() || task.last_backup_time == "从未执行" {
                        true
                    } else {
                        !task.last_backup_time.starts_with(&today_str)
                    }
                }
                _ => false,
            };

            if need_run {
                tracing::info!(target: "smagical_ui::backup", "冷启动补跑计划任务: 「{}」", task.name);
                Self::execute_task_pipeline(
                    task,
                    Arc::clone(&self.storage),
                    Arc::clone(&self.retries),
                    self.window_weak.clone(),
                    self.notifications.clone(),
                    false,
                ).await;
            }
        }
    }

    /// 拦截应用退出流程，执行所有策略为 `OnExit` (退出时备份) 的同步任务。
    ///
    /// 受到最多 4 秒的超时强制熔断保护，防止远端服务器挂死导致桌面客户端退出卡死。
    async fn handle_app_before_exit(&self) {
        let tasks = match self.storage.backup_tasks().list_all().await {
            Ok(t) => t,
            Err(_) => return,
        };

        let on_exit_tasks: Vec<_> = tasks
            .into_iter()
            .filter(|t| t.enabled && t.strategy == BackupStrategy::OnExit)
            .collect();

        if on_exit_tasks.is_empty() {
            return;
        }

        tracing::info!(target: "smagical_ui::backup", "应用退出前夕: 正在执行 {} 个安全退出同步任务", on_exit_tasks.len());
        for task in on_exit_tasks {
            Self::execute_task_pipeline(
                task,
                Arc::clone(&self.storage),
                Arc::clone(&self.retries),
                self.window_weak.clone(),
                self.notifications.clone(),
                false,
            ).await;
        }
    }

    /// 后台周期巡检时钟循环 (每 30 秒执行一次检测)。
    ///
    /// # 巡检职责
    /// 1. 检查失败待重试队列中到期的任务，触发指数退避重试；
    /// 2. 检查 `Hourly` 策略（距离上次执行 >= 3600s）与 `Daily` 策略（当天未执行）的任务并驱动运行。
    async fn start_periodic_ticker(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;

            // 若 UI 主窗口已销毁，终止周期巡检协程，释放 Arc 资源
            if self.window_weak.upgrade().is_none() {
                tracing::info!("UI 主窗口已关闭，终止备份巡检后台协程");
                break;
            }

            // 1. 检查失败待重试队列
            let due_retry_task_ids = {
                let mut guard = self.retries.lock().await;
                let now = Instant::now();
                let mut due = Vec::new();
                for (id, state) in guard.iter() {
                    if now >= state.next_retry_at {
                        due.push(id.clone());
                    }
                }
                for id in &due {
                    guard.remove(id);
                }
                due
            };

            for id in due_retry_task_ids {
                if let Ok(Some(task)) = self.storage.backup_tasks().get_by_id(&id).await {
                    if task.enabled {
                        tracing::info!(target: "smagical_ui::backup", "执行退避重试任务: 「{}」", task.name);
                        Self::execute_task_pipeline(
                            task,
                            Arc::clone(&self.storage),
                            Arc::clone(&self.retries),
                            self.window_weak.clone(),
                            self.notifications.clone(),
                            false,
                        ).await;
                    }
                }
            }

            // 2. 检查 Hourly 与 Daily 计划任务
            if let Ok(tasks) = self.storage.backup_tasks().list_all().await {
                let now_epoch = chrono::Utc::now().timestamp() as u64;
                let today_str = chrono::Utc::now().format("%Y-%m-%d").to_string();

                for task in tasks {
                    if !task.enabled {
                        continue;
                    }

                    let should_run = match task.strategy {
                        BackupStrategy::Hourly => {
                            if task.last_backup_time.is_empty() || task.last_backup_time == "从未执行" {
                                true
                            } else if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&task.last_backup_time, "%Y-%m-%d %H:%M:%S") {
                                now_epoch.saturating_sub(dt.and_utc().timestamp() as u64) >= 3600
                            } else {
                                false
                            }
                        }
                        BackupStrategy::Daily => {
                            if task.last_backup_time.is_empty() || task.last_backup_time == "从未执行" {
                                true
                            } else {
                                !task.last_backup_time.starts_with(&today_str)
                            }
                        }
                        _ => false,
                    };

                    if should_run {
                        Self::execute_task_pipeline(
                            task,
                            Arc::clone(&self.storage),
                            Arc::clone(&self.retries),
                            self.window_weak.clone(),
                            self.notifications.clone(),
                            false,
                        ).await;
                    }
                }
            }
        }
    }

    /// 手动立即触发全部启用的备份节点。
    ///
    /// 供设置中心界面的“全部立即备份”按钮点击时调用。
    pub async fn trigger_all_now(self: &Arc<Self>) {
        if let Ok(tasks) = self.storage.backup_tasks().list_all().await {
            let active: Vec<_> = tasks.into_iter().filter(|t| t.enabled).collect();
            if active.is_empty() {
                self.notifications.info("无可执行的备份任务", "当前未启用任何容灾备份节点");
                return;
            }

            self.notifications.info("正在执行快照备份", &format!("已向 {} 个备份节点分发最新资产镜像", active.len()));
            for task in active {
                Self::execute_task_pipeline(
                    task,
                    Arc::clone(&self.storage),
                    Arc::clone(&self.retries),
                    self.window_weak.clone(),
                    self.notifications.clone(),
                    true,
                ).await;
            }
        }
    }

    /// 执行单条备份任务的完整处理流水线。
    ///
    /// # 标准执行流水线
    /// 1. **全量快照打包**：序列化全部资产与配置，生成 SHA-256 完整性哈希；
    /// 2. **协议适配器推送**：由驱动负责 Local/WebDAV/S3/Gist 上传；
    /// 3. **401 认证熔断判定**：若远端返回 401 Unauthorized，立即阻断后续重试并弹窗警告用户修正口令；
    /// 4. **脱机暂存与退避重试**：网络偶发故障时将快照写入本地 `.stash/` 目录，并按 `30s -> 2m -> 10m` 登记退避重试；
    /// 5. **快照历史修剪**：解析 `Retention` 保留份数，物理清理超限历史快照；
    /// 6. **状态回写与 UI 刷新**：更新 SQLite 任务状态并派发到 Slint 前端。
    ///
    /// # 参数
    /// - `task`: 待执行的备份任务实体记录；
    /// - `storage`: 存储服务 Arc 句柄；
    /// - `retries`: 失败重试状态哈希表锁；
    /// - `window_weak`: UI 视图模型弱引用；
    /// - `notif`: 桌面通知管理器；
    /// - `is_manual`: 是否为用户手动触发。
    async fn execute_task_pipeline(
        task: BackupTaskRecord,
        storage: Arc<dyn AppStorage>,
        retries: Arc<Mutex<HashMap<String, RetryState>>>,
        window_weak: slint::Weak<AppWindow>,
        notif: NotificationManager,
        is_manual: bool,
    ) {
        let task_id = task.id.clone();
        let task_name = task.name.clone();

        // 1. 打包生成全量快照 payload (支持端到端 AEAD 加密封包)
        let (payload_bytes, hash, hosts_count, _tunnels_count) =
            match create_backup_payload_with_encryption(storage.as_ref(), true, None).await {
                Ok(res) => res,
                Err(err) => {
                    tracing::error!(target: "smagical_ui::backup", "任务「{}」打包失败: {}", task_name, err);
                    let _ = storage.backup_tasks().update_status(&task_id, "failed", &err, "", task.snapshot_count).await;
                    return;
                }
            };

        let snap_id = format!("snap-{}", &uuid::Uuid::new_v4().to_string()[..8]);
        let now_dt = chrono::Utc::now();
        let timestamp_str = now_dt.format("%Y-%m-%d %H:%M:%S").to_string();
        let epoch_secs = now_dt.timestamp() as u64;
        let size_bytes = payload_bytes.len() as u64;
        let size_str = format_bytes_size(size_bytes);

        // 2. 实例化驱动并尝试推送快照
        let driver = create_backup_driver(&task);
        let push_result = driver.push_snapshot(&snap_id, &payload_bytes).await;

        match push_result {
            Ok(remote_id) => {
                // 上传成功：清除此任务的重试记录
                retries.lock().await.remove(&task_id);

                // 保存快照记录至仓储
                let snap_rec = BackupSnapshotRecord {
                    id: snap_id.clone(),
                    task_id: task_id.clone(),
                    timestamp: timestamp_str.clone(),
                    epoch_secs,
                    size_str: size_str.clone(),
                    size_bytes,
                    remark: if is_manual { "手动立即备份".into() } else { format!("自动增量快照 ({} 台主机)", hosts_count) },
                    hash: hash.clone(),
                    remote_id,
                };
                let _ = storage.backup_snapshots().save(&snap_rec).await;

                // 3. 执行生命周期保留规则 (Retention) 物理与元数据修剪
                let keep_count = Self::parse_retention_count(&task.retention);
                if keep_count > 0 {
                    if let Ok(pruned) = storage.backup_snapshots().prune_old_snapshots(&task_id, keep_count).await {
                        for p in pruned {
                            let _ = driver.delete_snapshot(&p.remote_id).await;
                        }
                    }
                    // 同步修剪远端存储多端遗留的陈旧镜像
                    let _ = driver.prune_old_snapshots(keep_count).await;
                }

                // 4. 更新任务最新状态
                let new_count = storage.backup_snapshots().list_by_task(&task_id).await.map(|l| l.len() as u32).unwrap_or(task.snapshot_count + 1);
                let _ = storage.backup_tasks().update_status(&task_id, "success", "", &timestamp_str, new_count).await;

                tracing::info!(target: "smagical_ui::backup", "备份任务「{}」同步成功 (体积: {}, 快照: {})", task_name, size_str, snap_id);

                if is_manual {
                    notif.success("备份快照已生成", &format!("「{}」成功生成最新快照，已归档 {} 台主机资产", task_name, hosts_count));
                }

                // 5. 刷新 UI 状态
                Self::refresh_ui_tasks(storage, window_weak).await;
            }
            Err(err_msg) => {
                tracing::error!(target: "smagical_ui::backup", "任务「{}」上传失败: {}", task_name, err_msg);

                // 401 熔断判定：认证失败不进行盲目无限重试
                let is_auth_fail = err_msg.contains("401") || err_msg.contains("鉴权失败") || err_msg.contains("Unauthorized") || err_msg.contains("认证失败");
                if is_auth_fail {
                    let _ = storage.backup_tasks().update_status(&task_id, "failed", &format!("认证鉴权失败: {}", err_msg), "", task.snapshot_count).await;
                    notif.error("备份鉴权失败", &format!("节点「{}」凭据认证未通过，请检查密码或令牌", task_name));
                    Self::refresh_ui_tasks(storage, window_weak).await;
                    return;
                }

                // 本地脱机暂存兜底 (.stash/)
                Self::stash_offline_snapshot(&task_id, &snap_id, &payload_bytes);

                // 智能指数退避重试 (30s -> 2m -> 10m)
                let mut retry_guard = retries.lock().await;
                let cur_attempt = retry_guard.get(&task_id).map(|s| s.attempt).unwrap_or(0);
                if cur_attempt < 3 {
                    let delay_secs = match cur_attempt {
                        0 => 30,
                        1 => 120,
                        _ => 600,
                    };
                    retry_guard.insert(task_id.clone(), RetryState {
                        attempt: cur_attempt + 1,
                        next_retry_at: Instant::now() + Duration::from_secs(delay_secs),
                    });
                    tracing::warn!(target: "smagical_ui::backup", "任务「{}」将在 {} 秒后进行第 {} 次退避重试", task_name, delay_secs, cur_attempt + 1);
                } else {
                    retry_guard.remove(&task_id);
                    notif.error("备份任务连续重试失败", &format!("节点「{}」多次尝试上传均未成功: {}", task_name, err_msg));
                }

                let _ = storage.backup_tasks().update_status(&task_id, "failed", &err_msg, "", task.snapshot_count).await;
                Self::refresh_ui_tasks(storage, window_weak).await;
            }
        }
    }

    /// 在网络通信故障或无法连接远端端点时，将快照数据保存至本地脱机暂存目录 (`~/.smalux/stash/`)。
    ///
    /// # 参数
    /// - `task_id`: 目标备份节点 ID；
    /// - `snap_id`: 快照编号；
    /// - `data`: AES 加密后的备份数据字节流。
    fn stash_offline_snapshot(task_id: &str, snap_id: &str, data: &[u8]) {
        if let Some(base) = directories::BaseDirs::new() {
            let stash_dir = base.home_dir().join(".smalux").join("stash");
            if std::fs::create_dir_all(&stash_dir).is_ok() {
                let file_path = stash_dir.join(format!("{}_{}.json", task_id, snap_id));
                let _ = std::fs::write(file_path, data);
            }
        }
    }

    /// 解析任务配置中的快照保留策略字符串并转为保留份数整型数值。
    ///
    /// # 规则
    /// - 包含 `"永久"` 或 `"unlimited"` -> 返回 `0` (不修剪，永久全量保留)；
    /// - 包含数字 (如 `"保留最近 10 份"`, `"5"`, `"30"`) -> 提取数字返回对应阈值；
    /// - 兜底默认保留 10 份。
    fn parse_retention_count(retention_str: &str) -> usize {
        if retention_str.contains("永久") || retention_str.contains("unlimited") {
            0
        } else {
            let digits: String = retention_str.chars().filter(|c| c.is_ascii_digit()).collect();
            digits.parse::<usize>().unwrap_or(10)
        }
    }

    /// 重新从 SQLite 仓储异步查询全量备份任务列表并同步渲染至 Slint 前端 `SettingsBridge`。
    ///
    /// # 参数
    /// - `storage`: 存储服务 Arc 句柄；
    /// - `window_weak`: Slint UI 主窗口弱引用。
    pub async fn refresh_ui_tasks(storage: Arc<dyn AppStorage>, window_weak: slint::Weak<AppWindow>) {
        if let Ok(tasks) = storage.backup_tasks().list_all().await {
            let ui_tasks: Vec<BackupTaskItem> = tasks
                .into_iter()
                .map(|t| BackupTaskItem {
                    id: t.id.into(),
                    name: t.name.into(),
                    backup_type: t.backup_type.as_str().into(),
                    endpoint: t.endpoint.into(),
                    strategy: t.strategy.as_str().into(),
                    retention: t.retention.into(),
                    enabled: t.enabled,
                    last_backup_time: t.last_backup_time.into(),
                    snapshot_count: t.snapshot_count as i32,
                })
                .collect();

            let _ = run_on_ui(window_weak, move |w| {
                w.global::<SettingsBridge>().set_backup_tasks(to_model_rc(ui_tasks));
            });
        }
    }
}
