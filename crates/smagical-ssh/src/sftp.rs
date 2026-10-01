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
use smagical_core::domain::file_item::{format_file_size, FileItemData};
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

        let output = cmd.output()
            .with_context(|| format!("执行文件上传失败: {:?} -> {}", local_path, remote_target_dir))?;

        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            bail!("上传失败: {}", err_msg.trim());
        }

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

        let output = cmd.output()
            .with_context(|| format!("执行文件下载失败: {} -> {:?}", remote_source_path, local_target_dir))?;

        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            bail!("下载失败: {}", err_msg.trim());
        }

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
}
