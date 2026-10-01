//! # Smalux TUI - 全屏终端资产交互式运维系统
//!
//! 基于 Ratatui + Crossterm 构建，支持资产模糊检索、配置快速浏览、保险库弹窗解锁与 SSH 交互终端无感切入。

pub mod app;
pub mod ui;

use std::io::stdout;
use std::sync::Arc;
use std::time::Duration;
use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use app::App;
use smagical_core::storage::AppStorage;
use smagical_ssh::RusshSessionDriver;

/// TUI 终端模式管理守卫，退出时恢复原始终端状态
struct TerminalGuard;

impl TerminalGuard {
    fn new() -> Result<Self> {
        enable_raw_mode().context("启用终端 Raw Mode 失败")?;
        execute!(stdout(), EnterAlternateScreen).context("进入 Alternate Screen 失败")?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(stdout(), LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}

/// 运行 Smalux TUI 主事件循环
pub async fn run_tui(
    storage: Arc<dyn AppStorage>,
    ssh_svc: Arc<RusshSessionDriver>,
    cli_password_opt: Option<&str>,
) -> Result<()> {
    let _guard = TerminalGuard::new()?;
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend).context("初始化 Ratatui Terminal 失败")?;
    terminal.clear()?;

    let mut app = App::new(storage, ssh_svc, cli_password_opt).await?;

    loop {
        // 渲染当前帧
        terminal.draw(|f| ui::render(f, &app))?;

        // 轮询按键与终端事件 (100ms 超时)
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                // 1. 保险库锁定模态弹窗状态下的按键处理
                if app.is_vault_locked {
                    match key.code {
                        KeyCode::Enter => {
                            let _ = app.try_unlock().await;
                        }
                        KeyCode::Esc => {
                            app.should_quit = true;
                        }
                        KeyCode::Char(c) => {
                            app.unlock_input.push(c);
                        }
                        KeyCode::Backspace => {
                            app.unlock_input.pop();
                        }
                        _ => {}
                    }
                    continue;
                }

                // 2. 搜索框激活状态下的按键处理
                if app.is_searching {
                    match key.code {
                        KeyCode::Enter | KeyCode::Esc => {
                            app.is_searching = false;
                        }
                        KeyCode::Char(c) => {
                            app.search_query.push(c);
                            app.selected_index = 0;
                        }
                        KeyCode::Backspace => {
                            app.search_query.pop();
                            app.selected_index = 0;
                        }
                        _ => {}
                    }
                    continue;
                }

                // 3. 常规资产浏览导航按键
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        app.should_quit = true;
                    }
                    KeyCode::Char('/') => {
                        app.is_searching = true;
                    }
                    KeyCode::Char('r') => {
                        let _ = app.reload_data().await;
                        app.status_message = Some("已刷新本地资产列表".to_string());
                    }
                    KeyCode::Char('l') => {
                        app.lock_vault();
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        app.previous();
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        app.next();
                    }
                    KeyCode::Enter => {
                        if let Some(host) = app.selected_host() {
                            app.connect_target = Some(host.clone());
                        }
                    }
                    _ => {}
                }
            }
        }

        // 4. 用户请求在选中的主机上发起交互式 SSH 终端连接
        if let Some(target_host) = app.connect_target.take() {
            // 临时切出 TUI AlternateScreen，复原终端环境
            execute!(stdout(), LeaveAlternateScreen)?;
            disable_raw_mode()?;

            println!("\r\n========================================================");
            println!(" 准备连接主机: {} ({}:{})", target_host.name, target_host.address, target_host.port);
            println!("========================================================\r\n");

            let cred = if let Some(ref cid) = target_host.credential_id {
                app.storage.credentials().get_by_id(cid).await.ok().flatten()
            } else {
                None
            };

            let conn_res = crate::terminal_session::run_interactive_session(
                &target_host,
                cred.as_ref(),
                &app.ssh_svc,
            ).await;

            if let Err(e) = conn_res {
                eprintln!("\r\n❌ SSH 连接异常: {}\r\n", e);
            }

            println!("\r\n[按 Enter 键返回 Smalux TUI 界面...]");
            let mut pause_buf = String::new();
            let _ = std::io::stdin().read_line(&mut pause_buf);

            // 重新切回 TUI AlternateScreen 并恢复 Raw Mode
            enable_raw_mode()?;
            execute!(stdout(), EnterAlternateScreen)?;
            terminal.clear()?;
            let _ = app.reload_data().await;
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
