//! 高性能并发 SFTP 与本地文件传输队列调度管理器 (TransferQueueManager)。
//!
//! # 核心职责与治理机制
//! 1. **并发槽位受控调度 (Concurrency Throttling)**：默认 3 个并发传输槽位，超出任务进入 `Pending` 排队等待；
//! 2. **真实双缓冲流式进度遥测 (Streamed Telemetry)**：通过 `TransferProgress` 通道监听传输字节与瞬时速率，计算平滑 EMA 速率与 ETA 剩余时间；
//! 3. **UI 节流防抖同步 (Throttled UI Sync)**：以 100ms 滑动窗口批量调度回 Slint UI，防止密集刷新导致界面卡顿；
//! 4. **物理可取消任务句柄 (Abortable Tasks)**：通过 `tokio::task::AbortHandle` 真正物理中断传输连接与读写，释放文件与网络资源；
//! 5. **文件夹多级聚合统计 (Directory Aggregation)**：子文件并发推进时，动态累计父级文件夹的总已传输字节数与聚合速率；
//! 6. **生命周期闭环与桌面通知 (Lifecycle & Notifications)**：支持暂停/继续/重试/移除，并在传输完成时触发系统气泡通知与目标目录静默热刷新。

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::mpsc::UnboundedSender;
use tokio::task::AbortHandle;

use smagical_core::{TransferDirection, TransferProgress, TransferStatus};
use crate::generated::AppWindow;
use crate::notification_service::NotificationManager;
use crate::terminal::SshLaunchConfig;

/// 单个传输作业请求元数据
#[derive(Debug, Clone)]
pub struct TransferJob {
    /// 任务唯一 ID
    pub task_id: String,
    /// 所属父级文件夹任务 ID (若为文件夹下的子文件)
    pub parent_id: Option<String>,
    /// 所属会话标识
    pub session_id: String,
    /// 传输文件名
    pub filename: String,
    /// 是否为目录/文件夹任务
    pub is_dir: bool,
    /// 源文件物理路径
    pub source_path: String,
    /// 目标文件物理路径
    pub target_path: String,
    /// 传输方向 (上传/下载)
    pub direction: TransferDirection,
    /// 文件总字节数
    pub total_bytes: u64,
    /// 是否为本地互拷模式 (远程对端即为本机)
    pub is_remote_local: bool,
    /// 目标主机 ID
    pub host_id: String,
    /// 传输完成后需触发无感刷新的目标目录路径
    pub target_refresh_dir: String,
    /// 目标主机 SSH 启动配置 (用于 SFTP 降级命令执行)
    pub launch_cfg: Option<SshLaunchConfig>,
}

/// 全局多任务并发传输队列管理器
pub struct TransferQueueManager {
    /// 允许的最大同时并发传输数量 (默认 3，限制 1..=10)
    max_concurrency: AtomicUsize,
    /// 当前活跃任务表 (task_id -> tokio::task::AbortHandle)
    active_transfers: Arc<Mutex<HashMap<String, AbortHandle>>>,
    /// 等待调度的任务队列
    pending_queue: Arc<Mutex<VecDeque<TransferJob>>>,
    /// 任务作业元数据注册表 (task_id -> TransferJob)，用于支持失败重试
    jobs_registry: Arc<Mutex<HashMap<String, TransferJob>>>,
    /// Slint UI 主窗口弱引用句柄
    window_weak: slint::Weak<AppWindow>,
    /// 全局桌面气泡通知管理器
    notifications: NotificationManager,
}

impl TransferQueueManager {
    /// 构造全新的传输队列管理器实例
    pub fn new(
        window_weak: slint::Weak<AppWindow>,
        notifications: NotificationManager,
        max_concurrency: usize,
    ) -> Self {
        let capped = max_concurrency.clamp(1, 10);
        Self {
            max_concurrency: AtomicUsize::new(capped),
            active_transfers: Arc::new(Mutex::new(HashMap::new())),
            pending_queue: Arc::new(Mutex::new(VecDeque::new())),
            jobs_registry: Arc::new(Mutex::new(HashMap::new())),
            window_weak,
            notifications,
        }
    }

    /// 获取最大并发上限
    pub fn max_concurrency(&self) -> usize {
        self.max_concurrency.load(Ordering::Relaxed)
    }

    /// 动态调整最大并发数
    pub fn set_max_concurrency(&self, val: usize) {
        let capped = val.clamp(1, 10);
        self.max_concurrency.store(capped, Ordering::SeqCst);
    }

    /// 获取当前正在活跃读写的任务数
    pub fn active_count(&self) -> usize {
        self.active_transfers.lock().unwrap().len()
    }

    /// 获取排队等待中的任务数
    pub fn pending_count(&self) -> usize {
        self.pending_queue.lock().unwrap().len()
    }

    /// 提交单个传输作业并调度执行
    pub fn submit_job(self: &Arc<Self>, job: TransferJob) {
        let task_id = job.task_id.clone();
        self.jobs_registry.lock().unwrap().insert(task_id.clone(), job.clone());

        let max_conc = self.max_concurrency();
        let active = self.active_transfers.lock().unwrap();

        if active.len() < max_conc {
            drop(active);
            self.spawn_worker(job);
        } else {
            drop(active);
            let mut pending = self.pending_queue.lock().unwrap();
            if pending.len() < 2000 {
                pending.push_back(job);
                drop(pending);
                self.update_task_status_on_ui(&task_id, TransferStatus::Pending, None);
            } else {
                drop(pending);
                tracing::warn!(target: "smagical_ui::transfer", "排队任务已达上限 (2000)，拒绝新任务: {}", task_id);
                self.update_task_status_on_ui(&task_id, TransferStatus::Failed, Some("排队队列已满".into()));
            }
        }
    }

    /// 批量提交一组传输作业 (例如上传文件夹包含的所有子文件)
    pub fn submit_batch(self: &Arc<Self>, jobs: Vec<TransferJob>) {
        for job in jobs {
            self.submit_job(job);
        }
    }

    /// 启动单个任务的执行工作协程
    fn spawn_worker(self: &Arc<Self>, job: TransferJob) {
        let task_id = job.task_id.clone();
        let parent_id = job.parent_id.clone();
        let filename = job.filename.clone();
        let is_remote_local = job.is_remote_local;
        let target_refresh_dir = job.target_refresh_dir.clone();
        let direction = job.direction;
        let is_dir = job.is_dir;
        let target_path = job.target_path.clone();

        // 标记 UI 状态为 Transferring (传输中)
        self.update_task_status_on_ui(&task_id, TransferStatus::Transferring, None);

        let (progress_tx, mut progress_rx) =
            tokio::sync::mpsc::unbounded_channel::<TransferProgress>();

        // 1. 启动 UI 节流遥测同步流 (100ms 防抖更新速率与进度)
        let task_id_telemetry = task_id.clone();
        let parent_id_telemetry = parent_id.clone();
        let window_weak = self.window_weak.clone();

        let telemetry_handle = tokio::spawn(async move {
            let mut last_ui_update = Instant::now();
            let mut last_progress: Option<TransferProgress> = None;

            while let Some(prog) = progress_rx.recv().await {
                last_progress = Some(prog.clone());
                let now = Instant::now();
                if now.duration_since(last_ui_update) >= Duration::from_millis(100) {
                    last_ui_update = now;
                    let tid = task_id_telemetry.clone();
                    let pid = parent_id_telemetry.clone();
                    let transferred = prog.transferred_bytes;
                    let speed = prog.speed_bytes_per_sec;
                    let total = prog.total_bytes;
                    let w_weak = window_weak.clone();

                    let _ = slint::invoke_from_event_loop(move || {
                        crate::handlers::file_handlers::with_file_app_ctx(|ctx| {
                            let mut tasks = ctx.transfer_tasks.borrow_mut();
                            if let Some(t) = tasks.iter_mut().find(|t| t.id == tid) {
                                t.transferred_bytes = transferred;
                                t.speed_bytes_per_sec = speed;
                                if total > 0 {
                                    t.total_bytes = total;
                                }
                            }
                            if let Some(ref p_id) = pid {
                                let mut p_trans = 0u64;
                                let mut p_spd = 0u64;
                                for child in tasks.iter().filter(|t| t.parent_id.as_deref() == Some(p_id)) {
                                    p_trans += child.transferred_bytes;
                                    p_spd += child.speed_bytes_per_sec;
                                }
                                if let Some(pt) = tasks.iter_mut().find(|t| t.id == *p_id) {
                                    pt.transferred_bytes = p_trans;
                                    pt.speed_bytes_per_sec = p_spd;
                                }
                            }
                            drop(tasks);
                            if let Some(w) = w_weak.upgrade() {
                                crate::handlers::file_handlers::sync_file_explorer_ui(&w, ctx);
                            }
                        });
                    });
                }
            }

            // 传输结束前夕推送最后一次精准进度
            if let Some(prog) = last_progress {
                let tid = task_id_telemetry.clone();
                let pid = parent_id_telemetry.clone();
                let transferred = prog.transferred_bytes;
                let speed = prog.speed_bytes_per_sec;
                let w_weak = window_weak.clone();

                let _ = slint::invoke_from_event_loop(move || {
                    crate::handlers::file_handlers::with_file_app_ctx(|ctx| {
                        let mut tasks = ctx.transfer_tasks.borrow_mut();
                        if let Some(t) = tasks.iter_mut().find(|t| t.id == tid) {
                            t.transferred_bytes = transferred;
                            t.speed_bytes_per_sec = speed;
                        }
                        if let Some(ref p_id) = pid {
                            let mut p_trans = 0u64;
                            for child in tasks.iter().filter(|t| t.parent_id.as_deref() == Some(p_id)) {
                                p_trans += child.transferred_bytes;
                            }
                            if let Some(pt) = tasks.iter_mut().find(|t| t.id == *p_id) {
                                pt.transferred_bytes = p_trans;
                            }
                        }
                        drop(tasks);
                        if let Some(w) = w_weak.upgrade() {
                            crate::handlers::file_handlers::sync_file_explorer_ui(&w, ctx);
                        }
                    });
                });
            }
        });

        // 2. 启动真实传输执行协程
        let job_clone = job.clone();
        let tid_worker = task_id.clone();

        let worker_task = tokio::spawn(async move {
            let res = Self::execute_job_payload(job_clone, Some(progress_tx)).await;
            let _ = telemetry_handle.await;
            (tid_worker, res)
        });

        let abort_handle = worker_task.abort_handle();
        self.active_transfers.lock().unwrap().insert(task_id.clone(), abort_handle);

        // 3. 监控任务终态并驱动后续队列与通知
        let manager_finish = Arc::clone(self);
        tokio::spawn(async move {
            let (tid, res) = match worker_task.await {
                Ok(tuple) => tuple,
                Err(join_err) => {
                    let is_cancelled = join_err.is_cancelled();
                    (
                        task_id.clone(),
                        Err(if is_cancelled {
                            "任务已被手动取消".to_string()
                        } else {
                            format!("传输协程异常: {}", join_err)
                        }),
                    )
                }
            };

            // 从活跃表中注销
            manager_finish.active_transfers.lock().unwrap().remove(&tid);

            let success = res.is_ok();
            let err_msg = res.err();

            if success {
                tracing::info!(
                    target: "smalux::sftp",
                    "🎉 [SFTP 传输完成] 任务 ID [{}], 文件: '{}'",
                    tid, filename
                );
            } else if let Some(ref e) = err_msg {
                tracing::error!(
                    target: "smalux::sftp",
                    "💥 [SFTP 传输失败] 任务 ID [{}], 文件: '{}', 错误详情: {}",
                    tid, filename, e
                );
            }

            // 若传输异常中断或被取消，清理本地残留的不完整损坏文件
            if !success && !is_dir {
                if is_remote_local || direction == TransferDirection::Download {
                    let local_target = PathBuf::from(&target_path);
                    if local_target.exists() {
                        let _ = tokio::fs::remove_file(&local_target).await;
                        tracing::warn!("已清理传输失败或取消的本地残留文件: {}", target_path);
                    }
                }
            }

            // 更新 UI 任务终态
            let tid_clone = tid.clone();
            let pid_clone = parent_id.clone();
            let fn_clone = filename.clone();
            let w_weak = manager_finish.window_weak.clone();
            let notif = manager_finish.notifications.clone();
            let refresh_dir = target_refresh_dir.clone();

            let _ = slint::invoke_from_event_loop(move || {
                crate::handlers::file_handlers::with_file_app_ctx(|ctx| {
                    let mut tasks = ctx.transfer_tasks.borrow_mut();
                    if let Some(t) = tasks.iter_mut().find(|t| t.id == tid_clone) {
                        if success {
                            t.status = TransferStatus::Completed;
                            t.transferred_bytes = t.total_bytes;
                            t.speed_bytes_per_sec = 0;
                            t.error_message = None;
                        } else {
                            t.status = TransferStatus::Failed;
                            t.speed_bytes_per_sec = 0;
                            t.error_message = err_msg.clone();
                        }
                    }

                    // 检查父级文件夹任务是否全部子项均已完成
                    if let Some(ref p_id) = pid_clone {
                        let all_completed = tasks
                            .iter()
                            .filter(|t| t.parent_id.as_deref() == Some(p_id))
                            .all(|t| t.status == TransferStatus::Completed);
                        let any_failed = tasks
                            .iter()
                            .filter(|t| t.parent_id.as_deref() == Some(p_id))
                            .any(|t| t.status == TransferStatus::Failed);

                        if let Some(pt) = tasks.iter_mut().find(|t| t.id == *p_id) {
                            if all_completed {
                                pt.status = TransferStatus::Completed;
                                pt.transferred_bytes = pt.total_bytes;
                                pt.speed_bytes_per_sec = 0;
                            } else if any_failed {
                                pt.status = TransferStatus::Failed;
                                pt.speed_bytes_per_sec = 0;
                            }
                        }
                    }
                    drop(tasks);

                    if success {
                        notif.success("传输完成", &format!("文件「{}」传输完成", fn_clone));

                        // 精确失效目标目录缓存并触发静默热刷新
                        if is_remote_local {
                            crate::handlers::file_handlers::invalidate_dir_cache("local", &refresh_dir);
                            let cur = ctx.local_current_path.borrow().clone();
                            crate::handlers::file_handlers::refresh_local_path(ctx, &cur);
                        } else {
                            crate::handlers::file_handlers::invalidate_dir_cache("remote", &refresh_dir);
                            let _ = crate::handlers::file_handlers::try_refresh_remote_path(ctx, &refresh_dir, false);
                        }
                    } else if let Some(ref e) = err_msg {
                        if !e.contains("手动取消") {
                            notif.error("传输失败", &format!("文件「{}」传输失败: {}", fn_clone, e));
                        }
                    }

                    if let Some(w) = w_weak.upgrade() {
                        crate::handlers::file_handlers::sync_file_explorer_ui(&w, ctx);
                    }
                });
            });

            // 调度并拉起队列中的下一个待办任务
            manager_finish.schedule_next();
        });
    }

    /// 将底层 I/O 错误转换为友好的本土化语义描述
    fn map_io_error(e: std::io::Error) -> String {
        match e.kind() {
            std::io::ErrorKind::PermissionDenied => "目标路径权限不足 (Permission Denied)".to_string(),
            std::io::ErrorKind::NotFound => "指定的文件或路径不存在 (Not Found)".to_string(),
            std::io::ErrorKind::AlreadyExists => "目标文件已存在 (Already Exists)".to_string(),
            std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionAborted => {
                "网络连接已断开 (Connection Reset)".to_string()
            }
            std::io::ErrorKind::TimedOut => "传输超时 (Timed Out)".to_string(),
            _ => {
                let msg = e.to_string();
                if msg.contains("space") || msg.contains("disk") || msg.contains("ENOSPC") || msg.contains("磁盘空间") {
                    "目标磁盘空间不足 (Disk Full)".to_string()
                } else {
                    format!("I/O 读写失败: {}", msg)
                }
            }
        }
    }

    /// 执行具体的单文件底层传输载荷 (双缓冲流式分块)
    async fn execute_job_payload(
        job: TransferJob,
        progress_tx: Option<UnboundedSender<TransferProgress>>,
    ) -> Result<(), String> {
        let is_local = job.is_remote_local;
        let direction = job.direction;
        let src = job.source_path.clone();
        let tgt = job.target_path.clone();
        let task_id = job.task_id.clone();

        if is_local {
            // 本地文件/目录流式读写互拷
            if job.is_dir {
                tokio::task::spawn_blocking(move || {
                    crate::handlers::file_handlers::copy_dir_all(Path::new(&src), Path::new(&tgt))
                })
                .await
                .map_err(|e| format!("Join error: {}", e))?
                .map_err(Self::map_io_error)?;
                Ok(())
            } else {
                Self::copy_file_with_progress(
                    Path::new(&src),
                    Path::new(&tgt),
                    progress_tx,
                    &task_id,
                )
                .await
                .map_err(Self::map_io_error)?;
                Ok(())
            }
        } else {
            tracing::info!(
                target: "smalux::sftp",
                "🚀 [SFTP 传输引擎] 开始执行底层任务: 任务ID [{}], 方向={:?}, 源={}, 目标={}, 主机={}",
                task_id, direction, src, tgt, job.host_id
            );

            // 确保本地目录存在 (若是下载)
            if direction == TransferDirection::Download {
                if let Some(p) = Path::new(&tgt).parent() {
                    let _ = tokio::fs::create_dir_all(p).await;
                }
            } else if direction == TransferDirection::Upload {
                // 确保远程目录存在 (若是上传)
                if let Some(p) = Path::new(&tgt).parent() {
                    let p_str = p.to_string_lossy().replace('\\', "/");
                    if !p_str.is_empty() && p_str != "/" {
                        if let Some(sftp_svc) = crate::handlers::file_handlers::with_file_app_ctx(|ctx| ctx.core_state.sftp()) {
                            let _ = sftp_svc.create_dir(&job.host_id, &p_str).await;
                        }
                    }
                }
            }

            // 优先通过核心 SftpService 原生驱动读写 (纯 Rust russh-sftp)
            let sftp_svc_opt = crate::handlers::file_handlers::with_file_app_ctx(|ctx| ctx.core_state.sftp());
            let driver_opt = crate::handlers::file_handlers::with_file_app_ctx(|ctx| ctx.sftp_driver.clone()).flatten();

            if let Some(sftp_svc) = sftp_svc_opt {
                let hid = job.host_id.clone();
                match direction {
                    TransferDirection::Upload => {
                        let mut res = sftp_svc.upload_file(&hid, &src, &tgt, progress_tx.clone()).await;
                        if res.is_ok() {
                            tracing::info!(
                                target: "smalux::sftp",
                                "✅ [SFTP 传输引擎] 原生 SFTP 上传成功: {} -> {}",
                                src, tgt
                            );
                            return Ok(());
                        }

                        // 初次尝试失败，记录原因并尝试自动握手重连
                        tracing::warn!(
                            target: "smalux::sftp",
                            "⚠️ [SFTP 传输引擎] 原生 SFTP 上传初次尝试失败: {:?}，正在尝试自动握手重连目标主机 [{}]...",
                            res.as_ref().err(), hid
                        );

                        if let (Some(driver), Some(cfg)) = (&driver_opt, &job.launch_cfg) {
                            match driver.connect_and_register_with_config(&hid, cfg).await {
                                Ok(()) => {
                                    tracing::info!(
                                        target: "smalux::sftp",
                                        "🔌 [SFTP 传输引擎] 原生 SFTP 自动握手重连成功，重新创建远程目录并重试上传..."
                                    );
                                    if let Some(p) = Path::new(&tgt).parent() {
                                        let p_str = p.to_string_lossy().replace('\\', "/");
                                        if !p_str.is_empty() && p_str != "/" {
                                            let _ = sftp_svc.create_dir(&hid, &p_str).await;
                                        }
                                    }
                                    res = sftp_svc.upload_file(&hid, &src, &tgt, progress_tx.clone()).await;
                                    if res.is_ok() {
                                        tracing::info!(
                                            target: "smalux::sftp",
                                            "🎉 [SFTP 传输引擎] 重建长连接后重试原生上传成功: {} -> {}",
                                            src, tgt
                                        );
                                        return Ok(());
                                    } else {
                                        tracing::warn!(
                                            target: "smalux::sftp",
                                            "⚠️ [SFTP 传输引擎] 原生 SFTP 重试上传仍失败: {:?}",
                                            res.as_ref().err()
                                        );
                                    }
                                }
                                Err(reconn_err) => {
                                    tracing::warn!(
                                        target: "smalux::sftp",
                                        "⚠️ [SFTP 传输引擎] 原生 SFTP 自动握手重连失败: {:?}",
                                        reconn_err
                                    );
                                }
                            }
                        }
                    }
                    TransferDirection::Download => {
                        if !job.is_dir {
                            let mut res = sftp_svc.download_file(&hid, &src, &tgt, progress_tx.clone()).await;
                            if res.is_ok() {
                                tracing::info!(
                                    target: "smalux::sftp",
                                    "✅ [SFTP 传输引擎] 原生 SFTP 下载成功: {} -> {}",
                                    src, tgt
                                );
                                return Ok(());
                            }

                            tracing::warn!(
                                target: "smalux::sftp",
                                "⚠️ [SFTP 传输引擎] 原生 SFTP 下载初次尝试失败: {:?}，正在尝试自动握手重连目标主机 [{}]...",
                                res.as_ref().err(), hid
                            );

                            if let (Some(driver), Some(cfg)) = (&driver_opt, &job.launch_cfg) {
                                match driver.connect_and_register_with_config(&hid, cfg).await {
                                    Ok(()) => {
                                        tracing::info!(
                                            target: "smalux::sftp",
                                            "🔌 [SFTP 传输引擎] 原生 SFTP 自动握手重连成功，重试下载..."
                                        );
                                        res = sftp_svc.download_file(&hid, &src, &tgt, progress_tx.clone()).await;
                                        if res.is_ok() {
                                            tracing::info!(
                                                target: "smalux::sftp",
                                                "🎉 [SFTP 传输引擎] 重建长连接后重试原生下载成功: {} -> {}",
                                                src, tgt
                                            );
                                            return Ok(());
                                        } else {
                                            tracing::warn!(
                                                target: "smalux::sftp",
                                                "⚠️ [SFTP 传输引擎] 原生 SFTP 重试下载仍失败: {:?}",
                                                res.as_ref().err()
                                            );
                                        }
                                    }
                                    Err(reconn_err) => {
                                        tracing::warn!(
                                            target: "smalux::sftp",
                                            "⚠️ [SFTP 传输引擎] 原生 SFTP 自动握手重连失败: {:?}",
                                            reconn_err
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // 若原生长连接未就绪或报错，则降级采用 OpenSSH 独立通道
            if let Some(launch_cfg) = job.launch_cfg {
                tracing::warn!(
                    target: "smalux::sftp",
                    "⚠️ [SFTP 传输引擎] 原生 SFTP 通道不可用，正在降级尝试系统 OpenSSH SCP 通道: 方向={:?}, 源={}, 目标={}",
                    direction, src, tgt
                );
                match direction {
                    TransferDirection::Upload => {
                        let local_p = PathBuf::from(&src);
                        let parent_dir = Path::new(&tgt)
                            .parent()
                            .map(|p| p.to_string_lossy().replace('\\', "/"))
                            .unwrap_or_else(|| "/".to_string());
                        let res = crate::sftp::upload_path_with_queue(
                            launch_cfg,
                            local_p,
                            parent_dir,
                            crate::sftp::TransferConflictPolicy::Overwrite,
                            false,
                        )
                        .await
                        .map(|_| ())
                        .map_err(|e| e.to_string());

                        if let Err(ref e) = res {
                            tracing::error!(
                                target: "smalux::sftp",
                                "❌ [SFTP 传输引擎] 降级 OpenSSH 上传失败: {}",
                                e
                            );
                        } else {
                            tracing::info!(
                                target: "smalux::sftp",
                                "✅ [SFTP 传输引擎] 降级 OpenSSH 上传成功: {} -> {}",
                                src, tgt
                            );
                        }
                        res
                    }
                    TransferDirection::Download => {
                        let local_p = PathBuf::from(&tgt);
                        let local_dir = local_p
                            .parent()
                            .map(|p| p.to_path_buf())
                            .unwrap_or_else(|| PathBuf::from("."));
                        let res = crate::sftp::download_path_with_queue(
                            launch_cfg,
                            src.clone(),
                            local_dir,
                            crate::sftp::TransferConflictPolicy::Overwrite,
                            false,
                        )
                        .await
                        .map(|_| ())
                        .map_err(|e| e.to_string());

                        if let Err(ref e) = res {
                            tracing::error!(
                                target: "smalux::sftp",
                                "❌ [SFTP 传输引擎] 降级 OpenSSH 下载失败: {}",
                                e
                            );
                        } else {
                            tracing::info!(
                                target: "smalux::sftp",
                                "✅ [SFTP 传输引擎] 降级 OpenSSH 下载成功: {} -> {}",
                                src, tgt
                            );
                        }
                        res
                    }
                }
            } else {
                let err = format!("无法解析目标主机 [{}] 的 SSH 连接配置", job.host_id);
                tracing::error!(target: "smalux::sftp", "❌ [SFTP 传输引擎] {}", err);
                Err(err)
            }
        }
    }

    /// 本地文件分块流式复制并派发实时进度
    async fn copy_file_with_progress(
        source: &Path,
        target: &Path,
        progress_tx: Option<UnboundedSender<TransferProgress>>,
        task_id: &str,
    ) -> std::io::Result<u64> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let mut src_file = tokio::fs::File::open(source).await?;
        let total_bytes = src_file.metadata().await?.len();
        if let Some(p) = target.parent() {
            tokio::fs::create_dir_all(p).await?;
        }
        let mut dst_file = tokio::fs::File::create(target).await?;

        let chunk_size = 128 * 1024; // 128 KB
        let mut buf = vec![0u8; chunk_size];
        let mut transferred = 0u64;
        let start_time = Instant::now();
        let mut last_sample_time = Instant::now();
        let mut last_sample_bytes = 0u64;
        let mut current_speed = 0u64;

        loop {
            let n = match src_file.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) => {
                    let _ = tokio::fs::remove_file(target).await;
                    return Err(e);
                }
            };
            if let Err(e) = dst_file.write_all(&buf[..n]).await {
                let _ = tokio::fs::remove_file(target).await;
                return Err(e);
            }
            transferred += n as u64;

            if let Some(ref tx) = progress_tx {
                let now = Instant::now();
                let sample_elapsed = now.duration_since(last_sample_time).as_secs_f64();
                if sample_elapsed >= 0.15 {
                    let bytes_in_sample = transferred.saturating_sub(last_sample_bytes);
                    let instant_speed = (bytes_in_sample as f64 / sample_elapsed) as u64;
                    current_speed = if current_speed == 0 {
                        instant_speed
                    } else {
                        ((instant_speed as f64 * 0.7) + (current_speed as f64 * 0.3)) as u64
                    };
                    last_sample_time = now;
                    last_sample_bytes = transferred;

                    let _ = tx.send(TransferProgress {
                        task_id: task_id.to_string(),
                        transferred_bytes: transferred,
                        total_bytes,
                        speed_bytes_per_sec: current_speed,
                    });
                }
            }
        }
        if let Err(e) = dst_file.flush().await {
            let _ = tokio::fs::remove_file(target).await;
            return Err(e);
        }

        if let Some(ref tx) = progress_tx {
            let elapsed = start_time.elapsed().as_secs_f64().max(0.001);
            let avg_speed = (transferred as f64 / elapsed) as u64;
            let _ = tx.send(TransferProgress {
                task_id: task_id.to_string(),
                transferred_bytes: transferred,
                total_bytes,
                speed_bytes_per_sec: avg_speed,
            });
        }

        Ok(transferred)
    }

    /// 调度等待队列中的下一个任务入池
    pub fn schedule_next(self: &Arc<Self>) {
        let max_conc = self.max_concurrency();
        let active_len = self.active_transfers.lock().unwrap().len();

        if active_len < max_conc {
            let next_job_opt = self.pending_queue.lock().unwrap().pop_front();
            if let Some(job) = next_job_opt {
                self.spawn_worker(job);
            }
        }
    }

    /// 物理取消/终止指定任务
    pub fn cancel_task(self: &Arc<Self>, task_id: &str) {
        // 1. 若处于活跃执行态，调用 AbortHandle 物理中断
        let mut active = self.active_transfers.lock().unwrap();
        if let Some(handle) = active.remove(task_id) {
            handle.abort();
            tracing::info!(target: "smagical_ui::transfer", "物理中止传输任务: {}", task_id);
        }
        drop(active);

        // 2. 若在排队等待队列中，直接剔除
        let mut pending = self.pending_queue.lock().unwrap();
        pending.retain(|j| j.task_id != task_id && j.parent_id.as_deref() != Some(task_id));
        drop(pending);

        // 3. 将 UI 状态标记为已停止
        self.update_task_status_on_ui(task_id, TransferStatus::Failed, Some("用户手动取消传输".into()));

        // 4. 释放并发槽位，调度下一个任务
        self.schedule_next();
    }

    /// 重新入队并重试失败的任务
    pub fn retry_task(self: &Arc<Self>, task_id: &str) {
        let job_opt = self.jobs_registry.lock().unwrap().get(task_id).cloned();
        if let Some(job) = job_opt {
            // 重置 UI 状态为 Pending
            let tid = task_id.to_string();
            let w_weak = self.window_weak.clone();
            let _ = slint::invoke_from_event_loop(move || {
                crate::handlers::file_handlers::with_file_app_ctx(|ctx| {
                    let mut tasks = ctx.transfer_tasks.borrow_mut();
                    if let Some(t) = tasks.iter_mut().find(|t| t.id == tid) {
                        t.status = TransferStatus::Pending;
                        t.transferred_bytes = 0;
                        t.speed_bytes_per_sec = 0;
                        t.error_message = None;
                    }
                    drop(tasks);
                    if let Some(w) = w_weak.upgrade() {
                        crate::handlers::file_handlers::sync_file_explorer_ui(&w, ctx);
                    }
                });
            });

            self.submit_job(job);
        }
    }

    /// 从任务列表与调度器中物理移除任务记录
    pub fn remove_task(self: &Arc<Self>, task_id: &str) {
        self.cancel_task(task_id);
        let tid = task_id.to_string();
        self.jobs_registry.lock().unwrap().remove(&tid);

        let w_weak = self.window_weak.clone();
        let _ = slint::invoke_from_event_loop(move || {
            crate::handlers::file_handlers::with_file_app_ctx(|ctx| {
                let mut tasks = ctx.transfer_tasks.borrow_mut();
                tasks.retain(|t| t.id != tid && t.parent_id.as_deref() != Some(&tid));
                drop(tasks);
                if let Some(w) = w_weak.upgrade() {
                    crate::handlers::file_handlers::sync_file_explorer_ui(&w, ctx);
                }
            });
        });
    }

    /// 清理所有已完成与非活跃的任务记录
    pub fn clear_completed(&self) {
        // 同步清理内部注册表，防止历史任务长期驻留内存导致无界堆积
        {
            let active = self.active_transfers.lock().unwrap();
            let pending = self.pending_queue.lock().unwrap();
            let mut registry = self.jobs_registry.lock().unwrap();
            registry.retain(|id, _| active.contains_key(id) || pending.iter().any(|j| &j.task_id == id));
        }

        let w_weak = self.window_weak.clone();
        let _ = slint::invoke_from_event_loop(move || {
            crate::handlers::file_handlers::with_file_app_ctx(|ctx| {
                let mut tasks = ctx.transfer_tasks.borrow_mut();
                tasks.retain(|t| t.status == TransferStatus::Transferring || t.status == TransferStatus::Pending);
                drop(tasks);
                if let Some(w) = w_weak.upgrade() {
                    crate::handlers::file_handlers::sync_file_explorer_ui(&w, ctx);
                }
            });
        });
    }

    /// 辅助方法：向 Slint UI 线程派发任务状态变更
    fn update_task_status_on_ui(
        &self,
        task_id: &str,
        status: TransferStatus,
        error_msg: Option<String>,
    ) {
        let tid = task_id.to_string();
        let w_weak = self.window_weak.clone();

        let _ = slint::invoke_from_event_loop(move || {
            crate::handlers::file_handlers::with_file_app_ctx(|ctx| {
                let mut tasks = ctx.transfer_tasks.borrow_mut();
                for t in tasks.iter_mut() {
                    if t.id == tid || t.parent_id.as_deref() == Some(&tid) {
                        t.status = status;
                        if status != TransferStatus::Transferring {
                            t.speed_bytes_per_sec = 0;
                        }
                        if error_msg.is_some() {
                            t.error_message = error_msg.clone();
                        }
                    }
                }
                drop(tasks);
                if let Some(w) = w_weak.upgrade() {
                    crate::handlers::file_handlers::sync_file_explorer_ui(&w, ctx);
                }
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transfer_queue_concurrency_bounds() {
        let notif = NotificationManager::new(slint::Weak::default());
        let queue = TransferQueueManager::new(slint::Weak::default(), notif, 0);
        assert_eq!(queue.max_concurrency(), 1);

        queue.set_max_concurrency(20);
        assert_eq!(queue.max_concurrency(), 10);

        queue.set_max_concurrency(4);
        assert_eq!(queue.max_concurrency(), 4);
    }
}
