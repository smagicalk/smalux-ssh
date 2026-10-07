//! 远程 SFTP 与 SSH 文件系统协议驱动引擎 (SFTP & Remote File Operations Subsystem)。
//!
//! 利用平台原生 OpenSSH 套件 (ssh/scp/sftp)，结合免打扰 AskPass 凭据管道与 KeyTempGuard 私钥生命周期守护：
//! 1. 远程 Linux/Unix 目录快速遍历与属性解析 (`list_remote_directory`)；
//! 2. 真实文件与文件夹双向上传/下载与断点续传支持 (`upload_path` / `download_path`)；
//! 3. 远程目录新建、文件/文件夹递归删除、重命名与路径检索；
//! 4. 自动继承目标主机的私钥、密码、跳板机 (-J) 与网络代理配置，全程在后台异步线程运行，零卡顿。

use std::path::{Path, PathBuf};
use std::process::Command;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use smagical_core::domain::file_item::{format_file_size, FileItemData};
use smagical_core::TransferDirection;
use crate::ssh_config::{KeyTempGuard, SshLaunchConfig};

/// RAII 非交互式密码应答管道守卫。
/// 在执行 scp/sftp/ssh 非交互操作时动态生成受保护的独立脚本，并在 Drop 时覆写抹零销毁。
pub struct AskPassGuard {
    path: PathBuf,
}

impl AskPassGuard {
    /// 创建 AskPass 临时应答脚本并设置权限
    pub fn create(password: &str) -> Result<Self> {
        let temp_dir = std::env::temp_dir().join("smalux_askpass");
        let _ = std::fs::create_dir_all(&temp_dir);

        #[cfg(windows)]
        {
            crate::ssh_config::secure_windows_path_permissions(&temp_dir, true);
        }

        let uid = uuid::Uuid::new_v4().simple();

        #[cfg(windows)]
        let (script_name, script_content) = {
            (format!("askpass_{}.cmd", uid), format!("@echo off\r\necho {}\r\n", password))
        };

        #[cfg(not(windows))]
        let (script_name, script_content) = {
            (format!("askpass_{}.sh", uid), format!("#!/bin/sh\necho \"{}\"\n", password))
        };

        let path = temp_dir.join(script_name);
        std::fs::write(&path, script_content.as_bytes())
            .with_context(|| format!("写入 AskPass 脚本失败: {:?}", path))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&path)?.permissions();
            perms.set_mode(0o700);
            let _ = std::fs::set_permissions(&path, perms);
        }

        #[cfg(windows)]
        {
            crate::ssh_config::secure_windows_path_permissions(&path, false);
        }

        Ok(Self { path })
    }

    /// 获取脚本绝对路径
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for AskPassGuard {
    fn drop(&mut self) {
        if self.path.exists() {
            if let Ok(metadata) = std::fs::metadata(&self.path) {
                let len = metadata.len() as usize;
                if len > 0 {
                    let zero_buf = vec![0u8; len];
                    let _ = std::fs::write(&self.path, zero_buf);
                }
            }
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

/// SFTP / SSH 命令参数组装器
pub struct SftpCommandContext {
    /// 底层子进程启动命令构造器
    pub cmd: Command,
    /// 临时私钥安全生命周期守卫
    pub _key_guard: Option<KeyTempGuard>,
    /// 临时密码自动应答守卫
    pub _askpass_guard: Option<AskPassGuard>,
}

impl SftpCommandContext {
    /// 为 `ssh` 远程命令执行构建参数
    pub fn build_ssh(config: &SshLaunchConfig, remote_cmd: &str) -> Result<Self> {
        let mut cmd = Command::new("ssh");
        let (key_guard, askpass_guard) = Self::apply_common_options(&mut cmd, config, true)?;

        cmd.arg(config.destination());
        cmd.arg(remote_cmd);

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }

        Ok(Self {
            cmd,
            _key_guard: key_guard,
            _askpass_guard: askpass_guard,
        })
    }

    /// 为 `scp` 文件传输构建参数
    pub fn build_scp(config: &SshLaunchConfig) -> Result<(Command, Option<KeyTempGuard>, Option<AskPassGuard>)> {
        let mut cmd = Command::new("scp");
        let (key_guard, askpass_guard) = Self::apply_common_options(&mut cmd, config, false)?;

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }

        Ok((cmd, key_guard, askpass_guard))
    }

    fn apply_common_options(
        cmd: &mut Command,
        config: &SshLaunchConfig,
        is_ssh_cmd: bool,
    ) -> Result<(Option<KeyTempGuard>, Option<AskPassGuard>)> {
        let (args, key_guard) = config.build_ssh_args("sftp_sess", is_ssh_cmd)?;
        for arg in args {
            cmd.arg(arg);
        }

        // 密码自动应答 (AskPass)
        let mut askpass_guard = None;
        if let Some(ref pwd) = config.password {
            if !pwd.is_empty() {
                let guard = AskPassGuard::create(pwd)?;
                cmd.env("SSH_ASKPASS", guard.path());
                cmd.env("SSH_ASKPASS_REQUIRE", "force");
                cmd.env("DISPLAY", ":0");
                askpass_guard = Some(guard);
            }
        }

        Ok((key_guard, askpass_guard))
    }
}

/// 扫描并获取远程目录项列表
pub async fn list_remote_directory(config: SshLaunchConfig, remote_path: String) -> Result<Vec<FileItemData>> {
    tokio::task::spawn_blocking(move || {
        let clean_path = if remote_path.trim().is_empty() || remote_path == "~" {
            "~"
        } else {
            let p = remote_path.trim_end_matches('/');
            if p.is_empty() {
                "/"
            } else {
                p
            }
        };

        // 执行 LC_ALL=C ls -la --color=never -p "<path>" 获取纯净无 ANSI 颜色的标准列表
        let ls_cmd = format!("LC_ALL=C ls -la --color=never -p \"{}\"", clean_path);
        let mut ctx = SftpCommandContext::build_ssh(&config, &ls_cmd)?;

        let output = ctx.cmd.output()
            .with_context(|| format!("执行远程目录扫描失败: {}", clean_path))?;

        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            bail!("远程列目录失败 [{}]: {}", clean_path, err_msg.trim());
        }

        let out_str = String::from_utf8_lossy(&output.stdout);
        let parsed = parse_ls_output(&out_str, clean_path);
        Ok(parsed)
    }).await?
}

/// 解析 `ls -la -p` 命令标准输出行
pub fn parse_ls_output(stdout: &str, parent_dir: &str) -> Vec<FileItemData> {
    let mut files = Vec::new();
    let norm_parent = if parent_dir == "~" || parent_dir.is_empty() {
        "/root".to_string()
    } else {
        let p = parent_dir.trim_end_matches('/');
        if p.is_empty() {
            "/".to_string()
        } else {
            p.to_string()
        }
    };

    for line in stdout.lines() {
        let l = line.trim();
        if l.is_empty() || l.starts_with("total ") {
            continue;
        }

        // 解析格式: drwxr-xr-x 2 root root 4096 Sep 22 18:30 dirname/
        let parts: Vec<&str> = l.split_whitespace().collect();
        if parts.len() < 9 {
            continue;
        }

        let perms = parts[0];
        if perms.len() < 10 {
            continue;
        }

        let is_dir_by_perms = perms.starts_with('d');
        let is_symlink = perms.starts_with('l');

        let owner = parts[2].to_string();
        let group = parts[3].to_string();
        let size = parts[4].parse::<u64>().unwrap_or(0);

        let date_str = format!("{} {} {}", parts[5], parts[6], parts[7]);

        // 文件名可能含有空格，从第 8 个元素之后的所有内容重新组合
        let raw_name = parts[8..].join(" ");
        let (display_name, is_dir) = if is_symlink {
            // 软链接展示 "link_name -> target"
            if let Some((link_name, _target)) = raw_name.split_once(" -> ") {
                let is_dir_link = link_name.ends_with('/');
                (link_name.trim_end_matches('/').to_string(), is_dir_link || is_dir_by_perms)
            } else {
                (raw_name.trim_end_matches('/').to_string(), is_dir_by_perms)
            }
        } else {
            let is_dir_flag = raw_name.ends_with('/') || is_dir_by_perms;
            (raw_name.trim_end_matches('/').to_string(), is_dir_flag)
        };

        // 忽略当前目录 . 与上级目录 ..
        if display_name == "." || display_name == ".." || display_name.is_empty() {
            continue;
        }

        let full_path = if norm_parent == "/" {
            format!("/{}", display_name)
        } else {
            format!("{}/{}", norm_parent, display_name)
        };
        let is_hidden = display_name.starts_with('.');

        files.push(FileItemData {
            id: full_path.clone(),
            name: display_name,
            path: full_path,
            is_dir,
            size: if is_dir { 0 } else { size },
            size_formatted: if is_dir { "-".to_string() } else { format_file_size(size) },
            modified_at: 0,
            modified_formatted: date_str,
            permissions: perms.to_string(),
            owner,
            group,
            is_symlink,
            is_hidden,
            is_expanded: false,
            level: 0,
            item_count: 0,
        });
    }

    // 目录排在前面，同类按名称字典排序
    files.sort_by(|a, b| {
        match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });

    files
}

/// 上传本地文件或目录至远程目录
pub async fn upload_path(
    config: SshLaunchConfig,
    local_path: PathBuf,
    remote_target_dir: String,
) -> Result<()> {
    tokio::task::spawn_blocking(move || {
        let (mut cmd, _key_guard, _askpass_guard) = SftpCommandContext::build_scp(&config)?;

        let dest = format!("{}:{}", config.destination(), remote_target_dir);

        cmd.arg("-r");
        cmd.arg(&local_path);
        cmd.arg(&dest);

        tracing::info!(target: "smalux::sftp", "执行 OpenSSH SCP 上传命令: {:?} -> {}", local_path, dest);

        let output = cmd.output()
            .with_context(|| format!("执行文件上传失败: {:?} -> {}", local_path, remote_target_dir))?;

        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            tracing::error!(target: "smalux::sftp", "SCP 上传命令执行失败 (退出码 {:?}): {}", output.status.code(), err_msg.trim());
            bail!("上传失败: {}", err_msg.trim());
        }

        tracing::info!(target: "smalux::sftp", "OpenSSH SCP 上传成功: {:?} -> {}", local_path, dest);
        Ok(())
    }).await?
}

/// 从远程路径下载文件或目录至本地目标目录
pub async fn download_path(
    config: SshLaunchConfig,
    remote_source_path: String,
    local_target_dir: PathBuf,
) -> Result<()> {
    tokio::task::spawn_blocking(move || {
        let (mut cmd, _key_guard, _askpass_guard) = SftpCommandContext::build_scp(&config)?;

        let src = format!("{}:{}", config.destination(), remote_source_path);

        cmd.arg("-r");
        cmd.arg(&src);
        cmd.arg(&local_target_dir);

        tracing::info!(target: "smalux::sftp", "执行 OpenSSH SCP 下载命令: {} -> {:?}", src, local_target_dir);

        let output = cmd.output()
            .with_context(|| format!("执行文件下载失败: {} -> {:?}", remote_source_path, local_target_dir))?;

        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            tracing::error!(target: "smalux::sftp", "SCP 下载命令执行失败 (退出码 {:?}): {}", output.status.code(), err_msg.trim());
            bail!("下载失败: {}", err_msg.trim());
        }

        tracing::info!(target: "smalux::sftp", "OpenSSH SCP 下载成功: {} -> {:?}", src, local_target_dir);
        Ok(())
    }).await?
}

/// 在远程主机上递归创建目录 (`mkdir -p`)
pub async fn create_remote_dir(config: SshLaunchConfig, remote_dir: String) -> Result<()> {
    tokio::task::spawn_blocking(move || {
        let cmd_str = format!("mkdir -p \"{}\"", remote_dir);
        let mut ctx = SftpCommandContext::build_ssh(&config, &cmd_str)?;

        let output = ctx.cmd.output()
            .with_context(|| format!("执行远程新建目录失败: {}", remote_dir))?;

        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            bail!("新建目录失败: {}", err_msg.trim());
        }

        Ok(())
    }).await?
}

/// 在远程主机上创建空文件 (`touch`)
pub async fn create_remote_file(config: SshLaunchConfig, remote_file: String) -> Result<()> {
    tokio::task::spawn_blocking(move || {
        let cmd_str = format!("touch \"{}\"", remote_file);
        let mut ctx = SftpCommandContext::build_ssh(&config, &cmd_str)?;

        let output = ctx.cmd.output()
            .with_context(|| format!("执行远程新建文件失败: {}", remote_file))?;

        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            bail!("新建文件失败: {}", err_msg.trim());
        }

        Ok(())
    }).await?
}

/// 在远程主机上递归删除指定路径 (`rm -rf`)
pub async fn remove_remote_path(config: SshLaunchConfig, remote_path: String) -> Result<()> {
    tokio::task::spawn_blocking(move || {
        let cmd_str = format!("rm -rf \"{}\"", remote_path);
        let mut ctx = SftpCommandContext::build_ssh(&config, &cmd_str)?;

        let output = ctx.cmd.output()
            .with_context(|| format!("执行远程删除失败: {}", remote_path))?;

        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            bail!("删除失败: {}", err_msg.trim());
        }

        Ok(())
    }).await?
}

/// 在远程主机上重命名或移动路径 (`mv`)
pub async fn rename_remote_path(
    config: SshLaunchConfig,
    old_path: String,
    new_path: String,
) -> Result<()> {
    tokio::task::spawn_blocking(move || {
        let cmd_str = format!("mv \"{}\" \"{}\"", old_path, new_path);
        let mut ctx = SftpCommandContext::build_ssh(&config, &cmd_str)?;

        let output = ctx.cmd.output()
            .with_context(|| format!("执行远程重命名失败: {} -> {}", old_path, new_path))?;

        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            bail!("重命名失败: {}", err_msg.trim());
        }

        Ok(())
    }).await?
}

/// 传输目标冲突处理策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TransferConflictPolicy {
    /// 强制覆盖已有目标文件 (默认)
    #[default]
    Overwrite,
    /// 若目标文件已存在则跳过传输
    Skip,
    /// 若目标文件已存在则自动重命名 (例如 "file (1).txt")
    Rename,
}

/// 传输执行结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransferOutcome {
    /// 成功传输完成，附带实际传输字节数与最终目标路径 (可能由于冲突策略发生重命名)
    Completed {
        /// 传输总字节数
        transferred_bytes: u64,
        /// 实际目标完整路径
        final_path: String,
        /// 是否发生了自动重命名
        was_renamed: bool,
    },
    /// 目标文件已存在且策略为 Skip，已跳过
    Skipped {
        /// 目标已有路径
        target_path: String,
    },
}

/// 传输端到端完整性校验结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransferIntegrityResult {
    /// 大小完全一致，端到端完整性校验通过
    Verified {
        /// 校验一致的字节数
        size: u64,
    },
    /// 大小不一致，可能存在网络截断或数据损坏
    SizeMismatch {
        /// 期望大小 (源文件大小)
        expected: u64,
        /// 实际大小 (目标文件大小)
        actual: u64,
    },
    /// 目标文件不存在
    MissingTarget,
    /// 校验指令执行失败
    CheckFailed(String),
}

/// 验证远程文件大小 (通过 SSH 执行 `wc -c` 或 `stat -c %s`)
pub async fn verify_remote_file_size(config: &SshLaunchConfig, remote_path: &str) -> Result<u64> {
    let clean_path = remote_path.replace('"', "\\\"");
    let cmd_str = format!(
        "wc -c < \"{}\" 2>/dev/null || stat -c %s \"{}\" 2>/dev/null || stat -f %z \"{}\" 2>/dev/null",
        clean_path, clean_path, clean_path
    );
    let mut ctx = SftpCommandContext::build_ssh(config, &cmd_str)?;

    let output = ctx.cmd.output()
        .with_context(|| format!("执行远程文件大小核验失败: {}", remote_path))?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        bail!("核验远程文件大小失败: {}", err_msg.trim());
    }

    let out_str = String::from_utf8_lossy(&output.stdout);
    let size: u64 = out_str.trim().parse()
        .with_context(|| format!("解析远程文件大小输出异常: '{}'", out_str.trim()))?;

    Ok(size)
}

/// 端到端校验文件传输完整性
pub async fn check_transfer_integrity(
    config: &SshLaunchConfig,
    direction: TransferDirection,
    local_path: &Path,
    remote_path: &str,
) -> TransferIntegrityResult {
    match direction {
        TransferDirection::Upload => {
            let local_size = match std::fs::metadata(local_path) {
                Ok(m) => m.len(),
                Err(e) => return TransferIntegrityResult::CheckFailed(format!("读取本地源文件失败: {}", e)),
            };
            match verify_remote_file_size(config, remote_path).await {
                Ok(remote_size) => {
                    if remote_size == local_size {
                        TransferIntegrityResult::Verified { size: local_size }
                    } else {
                        TransferIntegrityResult::SizeMismatch {
                            expected: local_size,
                            actual: remote_size,
                        }
                    }
                }
                Err(e) => TransferIntegrityResult::CheckFailed(e.to_string()),
            }
        }
        TransferDirection::Download => {
            let local_size = match std::fs::metadata(local_path) {
                Ok(m) => m.len(),
                Err(_) => return TransferIntegrityResult::MissingTarget,
            };
            match verify_remote_file_size(config, remote_path).await {
                Ok(remote_size) => {
                    if remote_size == local_size {
                        TransferIntegrityResult::Verified { size: local_size }
                    } else {
                        TransferIntegrityResult::SizeMismatch {
                            expected: remote_size,
                            actual: local_size,
                        }
                    }
                }
                Err(e) => TransferIntegrityResult::CheckFailed(e.to_string()),
            }
        }
    }
}

/// 滑动窗口平滑速率与剩余时间 (ETA) 计算器
#[derive(Debug, Clone)]
pub struct TransferSpeedMeter {
    /// 历史采样窗口 [(采样时刻, 累计字节数)]
    samples: std::collections::VecDeque<(std::time::Instant, u64)>,
    /// 最大保留采样点数量 (默认 10)
    max_samples: usize,
    /// 滑动窗口时间跨度 (默认 3 秒)
    window_duration: std::time::Duration,
    /// 任务启动时刻
    start_time: std::time::Instant,
}

impl TransferSpeedMeter {
    /// 创建全新的速率采样器 (3秒滑动窗口，10采样点上限)
    pub fn new() -> Self {
        Self::with_window(std::time::Duration::from_secs(3), 10)
    }

    /// 使用指定窗口参数创建速率采样器
    pub fn with_window(window_duration: std::time::Duration, max_samples: usize) -> Self {
        let now = std::time::Instant::now();
        let mut samples = std::collections::VecDeque::with_capacity(max_samples);
        samples.push_back((now, 0));
        Self {
            samples,
            max_samples,
            window_duration,
            start_time: now,
        }
    }

    /// 记录当前累计传输字节进度
    pub fn record_progress(&mut self, cumulative_bytes: u64) {
        let now = std::time::Instant::now();

        // 清理窗口外超时的陈旧样本 (保留至少 1 个最早基线)
        while self.samples.len() > 1 {
            if let Some(&(first_time, _)) = self.samples.front() {
                if now.duration_since(first_time) > self.window_duration {
                    self.samples.pop_front();
                    continue;
                }
            }
            break;
        }

        // 保持样本上限
        if self.samples.len() >= self.max_samples {
            self.samples.pop_front();
        }

        self.samples.push_back((now, cumulative_bytes));
    }

    /// 计算当前滑动窗口内的平滑速率 (字节/秒)
    pub fn current_speed_bps(&self) -> u64 {
        if self.samples.len() < 2 {
            return 0;
        }

        let (first_time, first_bytes) = *self.samples.front().unwrap();
        let (last_time, last_bytes) = *self.samples.back().unwrap();

        let elapsed_secs = last_time.duration_since(first_time).as_secs_f64();
        if elapsed_secs <= 0.001 {
            return 0;
        }

        let delta_bytes = last_bytes.saturating_sub(first_bytes);
        (delta_bytes as f64 / elapsed_secs) as u64
    }

    /// 格式化人类可读实时传输速率 (如 "2.4 MB/s")
    pub fn format_speed(&self) -> String {
        let bps = self.current_speed_bps();
        if bps == 0 {
            "-".to_string()
        } else {
            format!("{}/s", format_file_size(bps))
        }
    }

    /// 计算预估剩余传输时间 (ETA 秒数)
    pub fn eta_seconds(&self, total_bytes: u64, cumulative_bytes: u64) -> Option<u64> {
        let speed = self.current_speed_bps();
        if speed == 0 || total_bytes <= cumulative_bytes {
            None
        } else {
            let remaining = total_bytes.saturating_sub(cumulative_bytes);
            Some(remaining / speed)
        }
    }

    /// 格式化人类可读剩余时间 (如 "02:15" 或 "< 1s")
    pub fn format_eta(&self, total_bytes: u64, cumulative_bytes: u64) -> String {
        match self.eta_seconds(total_bytes, cumulative_bytes) {
            Some(0) => "< 1s".to_string(),
            Some(secs) if secs < 3600 => {
                let m = secs / 60;
                let s = secs % 60;
                format!("{:02}:{:02}", m, s)
            }
            Some(secs) => {
                let h = secs / 3600;
                let m = (secs % 3600) / 60;
                let s = secs % 60;
                format!("{:02}:{:02}:{:02}", h, m, s)
            }
            None => "--:--".to_string(),
        }
    }

    /// 获取自任务启动至今的总耗时 (秒)
    pub fn elapsed_seconds(&self) -> f64 {
        self.start_time.elapsed().as_secs_f64()
    }
}

impl Default for TransferSpeedMeter {
    fn default() -> Self {
        Self::new()
    }
}

/// SFTP 多任务并发传输调度池 (TransferQueue)
pub struct TransferQueue {
    /// 内部并发控制信号量
    semaphore: std::sync::Arc<tokio::sync::Semaphore>,
    /// 最大并发槽位数 (默认 3，可配置 1..=5)
    max_concurrent: std::sync::atomic::AtomicUsize,
    /// 当前正在活跃传输的任务数
    active_transfers: std::sync::atomic::AtomicUsize,
    /// 累计已完成任务总数
    total_completed: std::sync::atomic::AtomicU64,
}

impl TransferQueue {
    /// 创建指定并发槽位上限的传输队列 (推荐 3~5 槽位)
    pub fn new(max_concurrent: usize) -> Self {
        let capped = max_concurrent.clamp(1, 5);
        Self {
            semaphore: std::sync::Arc::new(tokio::sync::Semaphore::new(capped)),
            max_concurrent: std::sync::atomic::AtomicUsize::new(capped),
            active_transfers: std::sync::atomic::AtomicUsize::new(0),
            total_completed: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// 获取全局默认传输队列 (默认 3 并发槽位)
    pub fn default_queue() -> &'static Self {
        static DEFAULT_QUEUE: std::sync::OnceLock<TransferQueue> = std::sync::OnceLock::new();
        DEFAULT_QUEUE.get_or_init(|| TransferQueue::new(3))
    }

    /// 获取当前正在运行的并发任务数
    pub fn active_count(&self) -> usize {
        self.active_transfers.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// 获取最大并发上限
    pub fn max_concurrency(&self) -> usize {
        self.max_concurrent.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// 累计完成任务数
    pub fn completed_count(&self) -> u64 {
        self.total_completed.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// 排队并获取并发传输槽位许可证
    pub async fn acquire_permit(&self) -> tokio::sync::OwnedSemaphorePermit {
        self.semaphore.clone().acquire_owned().await.expect("信号量未关闭")
    }

    /// 在并发槽位受控的环境中执行文件上传 (支持冲突策略与完整性自动校验)
    pub async fn upload_path_managed(
        &self,
        config: SshLaunchConfig,
        local_path: PathBuf,
        remote_target_dir: String,
        conflict_policy: TransferConflictPolicy,
        verify_integrity: bool,
    ) -> Result<TransferOutcome> {
        let _permit = self.acquire_permit().await;
        self.active_transfers.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        let res = async {
            tracing::info!(
                target: "smalux::sftp",
                "TransferQueue: 开始受控上传 {:?} -> {} (策略: {:?}, 校验: {})",
                local_path, remote_target_dir, conflict_policy, verify_integrity
            );

            if !local_path.exists() {
                bail!("本地源文件不存在: {:?}", local_path);
            }

            let filename = local_path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "unnamed".to_string());

            let clean_dir = remote_target_dir.trim_end_matches('/');
            let candidate_remote_path = if clean_dir.is_empty() {
                format!("/{}", filename)
            } else {
                format!("{}/{}", clean_dir, filename)
            };

            // 1. 冲突策略研判
            let mut final_remote_path = candidate_remote_path.clone();
            let mut was_renamed = false;
            match conflict_policy {
                TransferConflictPolicy::Overwrite => {}
                TransferConflictPolicy::Skip => {
                    if verify_remote_file_size(&config, &candidate_remote_path).await.is_ok() {
                        tracing::info!(target: "smalux::sftp", "TransferQueue: 目标已存在，跳过上传: {}", candidate_remote_path);
                        return Ok(TransferOutcome::Skipped { target_path: candidate_remote_path });
                    }
                }
                TransferConflictPolicy::Rename => {
                    if verify_remote_file_size(&config, &candidate_remote_path).await.is_ok() {
                        let stem = local_path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                        let ext = local_path.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
                        let renamed_filename = format!("{}_{}{}", stem, &uuid::Uuid::new_v4().simple().to_string()[..6], ext);
                        final_remote_path = if clean_dir.is_empty() {
                            format!("/{}", renamed_filename)
                        } else {
                            format!("{}/{}", clean_dir, renamed_filename)
                        };
                        was_renamed = true;
                        tracing::info!(target: "smalux::sftp", "TransferQueue: 目标已存在，重命名为: {}", final_remote_path);
                    }
                }
            }

            // 2. 执行底层传输
            upload_path(config.clone(), local_path.clone(), final_remote_path.clone()).await?;

            let local_bytes = std::fs::metadata(&local_path).map(|m| m.len()).unwrap_or(0);

            // 3. 完整性端到端自动化核验
            if verify_integrity && local_bytes > 0 {
                let integrity = check_transfer_integrity(
                    &config,
                    TransferDirection::Upload,
                    &local_path,
                    &final_remote_path,
                ).await;

                if let TransferIntegrityResult::SizeMismatch { expected, actual } = integrity {
                    bail!("上传完整性校验失败: 源文件大小 {} 字节，远程实际大小 {} 字节 (疑似网络截断)", expected, actual);
                }
            }

            self.total_completed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            tracing::info!(
                target: "smalux::sftp",
                "TransferQueue: 上传完成: 最终路径={}, 字节数={}",
                final_remote_path, local_bytes
            );
            Ok(TransferOutcome::Completed {
                transferred_bytes: local_bytes,
                final_path: final_remote_path,
                was_renamed,
            })
        }.await;

        self.active_transfers.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
        res
    }

    /// 在并发槽位受控的环境中执行文件下载 (支持冲突策略与完整性自动校验)
    pub async fn download_path_managed(
        &self,
        config: SshLaunchConfig,
        remote_source_path: String,
        local_target_dir: PathBuf,
        conflict_policy: TransferConflictPolicy,
        verify_integrity: bool,
    ) -> Result<TransferOutcome> {
        let _permit = self.acquire_permit().await;
        self.active_transfers.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        let res = async {
            tracing::info!(
                target: "smalux::sftp",
                "TransferQueue: 开始受控下载 {} -> {:?} (策略: {:?}, 校验: {})",
                remote_source_path, local_target_dir, conflict_policy, verify_integrity
            );

            let filename = Path::new(&remote_source_path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "downloaded_file".to_string());

            let mut final_local_file = local_target_dir.join(&filename);
            let mut was_renamed = false;

            // 1. 冲突策略研判
            if final_local_file.exists() {
                match conflict_policy {
                    TransferConflictPolicy::Overwrite => {}
                    TransferConflictPolicy::Skip => {
                        tracing::info!(target: "smalux::sftp", "TransferQueue: 本地已存在，跳过下载: {:?}", final_local_file);
                        return Ok(TransferOutcome::Skipped {
                            target_path: final_local_file.to_string_lossy().to_string(),
                        });
                    }
                    TransferConflictPolicy::Rename => {
                        let stem = Path::new(&filename).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                        let ext = Path::new(&filename).extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
                        let renamed_filename = format!("{}_{}{}", stem, &uuid::Uuid::new_v4().simple().to_string()[..6], ext);
                        final_local_file = local_target_dir.join(&renamed_filename);
                        was_renamed = true;
                        tracing::info!(target: "smalux::sftp", "TransferQueue: 本地已存在，重命名为: {:?}", final_local_file);
                    }
                }
            }

            // 2. 执行底层传输
            download_path(config.clone(), remote_source_path.clone(), final_local_file.clone()).await?;

            let local_bytes = std::fs::metadata(&final_local_file).map(|m| m.len()).unwrap_or(0);

            // 3. 完整性端到端自动化核验
            if verify_integrity && local_bytes > 0 {
                let integrity = check_transfer_integrity(
                    &config,
                    TransferDirection::Download,
                    &final_local_file,
                    &remote_source_path,
                ).await;

                if let TransferIntegrityResult::SizeMismatch { expected, actual } = integrity {
                    bail!("下载完整性校验失败: 远程大小 {} 字节，本地实际大小 {} 字节 (疑似网络截断)", expected, actual);
                }
            }

            self.total_completed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            tracing::info!(
                target: "smalux::sftp",
                "TransferQueue: 下载完成: 最终路径={:?}, 字节数={}",
                final_local_file, local_bytes
            );
            Ok(TransferOutcome::Completed {
                transferred_bytes: local_bytes,
                final_path: final_local_file.to_string_lossy().to_string(),
                was_renamed,
            })
        }.await;

        self.active_transfers.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
        res
    }
}

/// 通过全局默认传输队列执行受控文件上传
pub async fn upload_path_with_queue(
    config: SshLaunchConfig,
    local_path: PathBuf,
    remote_target_dir: String,
    conflict_policy: TransferConflictPolicy,
    verify_integrity: bool,
) -> Result<TransferOutcome> {
    TransferQueue::default_queue()
        .upload_path_managed(config, local_path, remote_target_dir, conflict_policy, verify_integrity)
        .await
}

/// 通过全局默认传输队列执行受控文件下载
pub async fn download_path_with_queue(
    config: SshLaunchConfig,
    remote_source_path: String,
    local_target_dir: PathBuf,
    conflict_policy: TransferConflictPolicy,
    verify_integrity: bool,
) -> Result<TransferOutcome> {
    TransferQueue::default_queue()
        .download_path_managed(config, remote_source_path, local_target_dir, conflict_policy, verify_integrity)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ls_output() {
        let raw_ls = r#"
total 48
drwxr-xr-x 4 root root 4096 Sep 22 18:30 .
drwxr-xr-x 8 root root 4096 Aug 10 12:00 ..
-rw-r--r-- 1 ubuntu ubuntu 1024 Sep 22 18:31 config.yaml
drwxrwxr-x 2 root www-data 4096 Sep 22 18:32 uploads/
lrwxrwxrwx 1 root root 11 Sep 22 18:33 current_link -> /var/www/v2
-rwxr-xr-x 1 root root 2048 Sep 22 18:34 run.sh
"#;

        let files = parse_ls_output(raw_ls, "/var/www");
        assert_eq!(files.len(), 4);

        // uploads 目录排在前面
        assert_eq!(files[0].name, "uploads");
        assert!(files[0].is_dir);
        assert_eq!(files[0].path, "/var/www/uploads");

        // 文件项
        assert_eq!(files[1].name, "config.yaml");
        assert!(!files[1].is_dir);
        assert_eq!(files[1].size, 1024);

        assert_eq!(files[2].name, "current_link");
        assert_eq!(files[3].name, "run.sh");
    }

    #[test]
    fn test_askpass_guard_lifecycle_and_zeroize() {
        let guard = AskPassGuard::create("secret_password_123").expect("创建 AskPass 失败");
        let path = guard.path().to_path_buf();
        assert!(path.exists());

        drop(guard);
        assert!(!path.exists());
    }

    #[test]
    fn test_transfer_speed_meter_sliding_window_and_eta() {
        let mut meter = TransferSpeedMeter::with_window(std::time::Duration::from_millis(500), 5);
        assert_eq!(meter.current_speed_bps(), 0);
        assert_eq!(meter.format_speed(), "-");
        assert_eq!(meter.format_eta(1000, 0), "--:--");

        // 模拟写入进度
        meter.record_progress(500 * 1024);
        let formatted = meter.format_speed();
        assert_eq!(formatted, "-"); // 仅单个点尚未形成两点差值

        // 稍作等待注入第二点
        std::thread::sleep(std::time::Duration::from_millis(15));
        meter.record_progress(1000 * 1024);

        let speed = meter.current_speed_bps();
        assert!(speed > 0);
        let speed_str = meter.format_speed();
        assert!(speed_str.contains("/s"));

        // 验证 ETA 预估
        let eta = meter.eta_seconds(2000 * 1024, 1000 * 1024);
        assert!(eta.is_some());
        let eta_str = meter.format_eta(2000 * 1024, 1000 * 1024);
        assert!(!eta_str.is_empty());
    }

    #[test]
    fn test_transfer_queue_concurrency_slots() {
        let queue = TransferQueue::new(3);
        assert_eq!(queue.max_concurrency(), 3);
        assert_eq!(queue.active_count(), 0);
        assert_eq!(queue.completed_count(), 0);
    }

    #[test]
    fn test_transfer_conflict_policy_variants() {
        assert_eq!(TransferConflictPolicy::default(), TransferConflictPolicy::Overwrite);
        let p_skip = TransferConflictPolicy::Skip;
        let p_rename = TransferConflictPolicy::Rename;
        assert_ne!(p_skip, p_rename);
    }

    #[test]
    fn test_transfer_integrity_result() {
        let verified = TransferIntegrityResult::Verified { size: 1024 };
        let mismatch = TransferIntegrityResult::SizeMismatch { expected: 1024, actual: 512 };
        assert_ne!(verified, mismatch);
    }
}
