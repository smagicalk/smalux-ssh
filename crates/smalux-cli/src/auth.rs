//! # Smalux CLI - 统一安全认证与主密码保险库拦截器
//!
//! 支持三级主密码供给链：
//! 1. 命令行参数 `--master-password`
//! 2. 环境变量 `SMALUX_MASTER_PASSWORD`
//! 3. 交互式终端安全无回显输入 (`rpassword`)

use std::sync::Arc;
use anyhow::{bail, Context, Result};
use smagical_core::storage::AppStorage;

/// 确保安全保险库处于已解锁状态。
///
/// 若当前未设置自定义主密码，则直接放行（使用底层默认设备密钥）；
/// 若已设置主密码且当前处于锁定状态：
/// 1. 优先尝试从 `cli_password_opt` 获取；
/// 2. 其次尝试从环境变量 `SMALUX_MASTER_PASSWORD` 获取；
/// 3. 若处于交互式终端 (TTY)，提示用户输入无回显主密码；
/// 4. 否则报错拒绝访问。
pub async fn ensure_vault_unlocked(
    storage: &Arc<dyn AppStorage>,
    cli_password_opt: Option<&str>,
) -> Result<()> {
    let has_custom = storage
        .has_custom_master_password()
        .await
        .unwrap_or(false);

    // 未开启自定义主密码，开箱即用默认模式
    if !has_custom {
        return Ok(());
    }

    // 已开启主密码且已在内存中解锁
    if storage.is_vault_unlocked() {
        return Ok(());
    }

    // 1. 尝试从命令行参数获取
    if let Some(pwd) = cli_password_opt {
        if !pwd.is_empty() {
            let success = storage
                .unlock_vault(pwd)
                .await
                .context("解锁安全保险库通信异常")?;
            if success {
                tracing::info!(target: "smalux_cli::auth", "通过命令行参数成功解锁安全保险库");
                return Ok(());
            } else {
                bail!("命令行参数提供的主密码错误，拒绝解锁保险库");
            }
        }
    }

    // 2. 尝试从环境变量获取
    if let Ok(env_pwd) = std::env::var("SMALUX_MASTER_PASSWORD") {
        if !env_pwd.is_empty() {
            let success = storage
                .unlock_vault(&env_pwd)
                .await
                .context("解锁安全保险库通信异常")?;
            if success {
                tracing::info!(target: "smalux_cli::auth", "通过环境变量 SMALUX_MASTER_PASSWORD 成功解锁安全保险库");
                return Ok(());
            } else {
                bail!("环境变量 SMALUX_MASTER_PASSWORD 中的主密码错误，拒绝解锁保险库");
            }
        }
    }

    // 3. 检查是否为交互式终端，若是则弹出无回显输入提示
    let is_terminal = crossterm::tty::IsTty::is_tty(&std::io::stdin());
    if is_terminal {
        eprint!("🔒 [Smalux] 资产保险库已开启主密码保护，请输入主密码: ");
        let input_pwd = rpassword::read_password()
            .context("读取终端主密码输入失败")?;

        let success = storage
            .unlock_vault(&input_pwd)
            .await
            .context("解锁安全保险库通信异常")?;

        if success {
            eprintln!("✅ 主密码验证通过，保险库已解锁。\n");
            return Ok(());
        } else {
            bail!("主密码验证失败，拒绝访问受保护的主机与凭据资产");
        }
    }

    // 4. 非交互式终端且未提供密码
    bail!("本地资产已开启主密码保护。在非交互式脚本或管道环境中，请通过 --master-password 参数或 SMALUX_MASTER_PASSWORD 环境变量提供主密码。");
}
