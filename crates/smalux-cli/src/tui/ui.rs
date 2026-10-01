//! # Smalux TUI - 终端界面渲染引擎 (Ratatui UI)

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

use super::app::App;
use smagical_core::domain::HostStatus;

/// 顶层渲染主函数
pub fn render(frame: &mut Frame, app: &App) {
    let size = frame.area();

    // 垂直切分: Header(3) + Main(剩余) + Footer(3)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(3),
        ])
        .split(size);

    render_header(frame, chunks[0], app);
    render_main_body(frame, chunks[1], app);
    render_footer(frame, chunks[2], app);

    // 若保险库处于锁定状态，在顶层覆盖居中弹窗
    if app.is_vault_locked {
        render_unlock_modal(frame, size, app);
    }
}

/// 顶部标题栏
fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let vault_badge = if app.is_vault_locked {
        Span::styled(" [🔒 保险库已锁定] ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
    } else {
        Span::styled(" [🔓 保险库已就绪] ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
    };

    let title_line = Line::from(vec![
        Span::styled(" 🚀 Smalux ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("Headless SSH & Asset TUI", Style::default().fg(Color::White)),
        Span::raw(" | "),
        Span::styled(format!("资产数: {}", app.hosts.len()), Style::default().fg(Color::Yellow)),
        vault_badge,
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let paragraph = Paragraph::new(title_line).block(block);
    frame.render_widget(paragraph, area);
}

/// 主工作区分栏 (左: 主机列表, 右: 详情面板)
fn render_main_body(frame: &mut Frame, area: Rect, app: &App) {
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(45),
            Constraint::Percentage(55),
        ])
        .split(area);

    render_host_list(frame, body_chunks[0], app);
    render_host_details(frame, body_chunks[1], app);
}

/// 左侧主机资产列表面板
fn render_host_list(frame: &mut Frame, area: Rect, app: &App) {
    let filtered = app.filtered_hosts();

    let items: Vec<ListItem> = filtered
        .iter()
        .enumerate()
        .map(|(idx, host)| {
            let is_selected = idx == app.selected_index;

            let (status_icon, status_color) = match host.status {
                HostStatus::Online => ("●", Color::Green),
                HostStatus::Warning => ("▲", Color::Yellow),
                HostStatus::Offline => ("■", Color::Red),
                HostStatus::Error => ("✕", Color::Magenta),
            };

            let line = Line::from(vec![
                Span::styled(format!(" {} ", status_icon), Style::default().fg(status_color)),
                Span::styled(
                    format!("{:<18}", host.name),
                    if is_selected {
                        Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
                    },
                ),
                Span::styled(
                    format!(" {}:{}", host.address, host.port),
                    if is_selected {
                        Style::default().fg(Color::Black).bg(Color::Cyan)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    },
                ),
            ]);

            ListItem::new(line)
        })
        .collect();

    let title = if app.is_searching {
        format!(" 🔍 搜索: {}█ (Esc 退出) ", app.search_query)
    } else if !app.search_query.is_empty() {
        format!(" 资产列表 (过滤: '{}', 共 {} 台) ", app.search_query, filtered.len())
    } else {
        format!(" 资产列表 (共 {} 台) ", filtered.len())
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(if app.is_searching {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Cyan)
        });

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}

/// 右侧主机详情与详细配置面板
fn render_host_details(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" 📋 主机资产属性与详情 ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let content = if let Some(h) = app.selected_host() {
        let group_name = h
            .parent_group_id
            .as_ref()
            .and_then(|gid| app.group_names.get(gid))
            .map(|s| s.as_str())
            .unwrap_or("未分组 (根节点)");

        let (status_str, status_color) = match h.status {
            HostStatus::Online => ("在线 (Online)", Color::Green),
            HostStatus::Warning => ("警告 (Warning)", Color::Yellow),
            HostStatus::Offline => ("离线 (Offline)", Color::Red),
            HostStatus::Error => ("故障 (Error)", Color::Magenta),
        };

        vec![
            Line::from(vec![
                Span::styled("资产名称:     ", Style::default().fg(Color::DarkGray)),
                Span::styled(&h.name, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("资产 ID:       ", Style::default().fg(Color::DarkGray)),
                Span::styled(&h.id, Style::default().fg(Color::Gray)),
            ]),
            Line::from(vec![
                Span::styled("网络地址:     ", Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{}:{}", h.address, h.port), Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![
                Span::styled("所属分组:     ", Style::default().fg(Color::DarkGray)),
                Span::styled(group_name, Style::default().fg(Color::Yellow)),
            ]),
            Line::from(vec![
                Span::styled("运行状态:     ", Style::default().fg(Color::DarkGray)),
                Span::styled(status_str, Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" (延迟: {}ms)", h.ping_ms), Style::default().fg(Color::DarkGray)),
            ]),
            Line::raw(""),
            Line::from(vec![
                Span::styled("── 认证与连接凭据 ───────────────────────", Style::default().fg(Color::DarkGray)),
            ]),
            Line::from(vec![
                Span::styled("登录用户:     ", Style::default().fg(Color::DarkGray)),
                Span::styled(h.username.as_deref().unwrap_or("-"), Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("认证方式:     ", Style::default().fg(Color::DarkGray)),
                Span::styled(&h.auth_type, Style::default().fg(Color::Magenta)),
            ]),
            Line::from(vec![
                Span::styled("凭据中心绑定: ", Style::default().fg(Color::DarkGray)),
                Span::styled(h.credential_id.as_deref().unwrap_or("直接配置"), Style::default().fg(Color::Gray)),
            ]),
            Line::raw(""),
            Line::from(vec![
                Span::styled("── 终端与网络高级属性 ───────────────────", Style::default().fg(Color::DarkGray)),
            ]),
            Line::from(vec![
                Span::styled("终端类型:     ", Style::default().fg(Color::DarkGray)),
                Span::styled(h.term_type.as_deref().unwrap_or("xterm-256color"), Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("心跳 / 超时:  ", Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{}s / {}s", h.keepalive_interval, h.connect_timeout), Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("初始工作目录: ", Style::default().fg(Color::DarkGray)),
                Span::styled(h.initial_dir.as_deref().unwrap_or("~"), Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("启动远程命令: ", Style::default().fg(Color::DarkGray)),
                Span::styled(h.startup_cmd.as_deref().unwrap_or("-"), Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("备注说明:     ", Style::default().fg(Color::DarkGray)),
                Span::styled(if h.notes.is_empty() { "-" } else { &h.notes }, Style::default().fg(Color::Gray)),
            ]),
        ]
    } else {
        vec![
            Line::from(Span::styled("当前无选中主机资产", Style::default().fg(Color::DarkGray))),
        ]
    };

    let paragraph = Paragraph::new(content).block(block).wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}

/// 底部状态栏与按键快捷提示
fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let content = if let Some(ref msg) = app.status_message {
        Line::from(Span::styled(msg, Style::default().fg(Color::Green)))
    } else if app.is_searching {
        Line::from(vec![
            Span::styled(" [输入字符] ", Style::default().fg(Color::Black).bg(Color::Yellow)),
            Span::raw(" 实时筛选  "),
            Span::styled(" [Enter/Esc] ", Style::default().fg(Color::Black).bg(Color::Yellow)),
            Span::raw(" 结束搜索并锁定光标"),
        ])
    } else {
        Line::from(vec![
            Span::styled(" [Enter] ", Style::default().fg(Color::Black).bg(Color::Cyan)),
            Span::raw(" 连接终端  "),
            Span::styled(" [/] ", Style::default().fg(Color::Black).bg(Color::Cyan)),
            Span::raw(" 搜索过滤  "),
            Span::styled(" [r] ", Style::default().fg(Color::Black).bg(Color::Cyan)),
            Span::raw(" 刷新  "),
            Span::styled(" [l] ", Style::default().fg(Color::Black).bg(Color::Yellow)),
            Span::raw(" 锁定保险库  "),
            Span::styled(" [q/Esc] ", Style::default().fg(Color::Black).bg(Color::Red)),
            Span::raw(" 退出"),
        ])
    };

    let paragraph = Paragraph::new(content).block(block);
    frame.render_widget(paragraph, area);
}

/// 居中解锁弹窗模态框 (Modal)
fn render_unlock_modal(frame: &mut Frame, area: Rect, app: &App) {
    let popup_area = centered_rect(60, 30, area);

    // 清除弹窗背后的原有内容
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" 🔒 安全保险库已锁定 ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD));

    let masked_pwd: String = "*".repeat(app.unlock_input.len());

    let mut lines = vec![
        Line::raw(""),
        Line::from(Span::styled(
            "本地资产已开启主密码保护，敏感凭据与私钥处于加密锁定状态。",
            Style::default().fg(Color::White),
        )),
        Line::from(Span::styled(
            "请输入主密码以挂载 DEK 解密保险库:",
            Style::default().fg(Color::DarkGray),
        )),
        Line::raw(""),
        Line::from(vec![
            Span::styled(" 主密码: [ ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{:<20}", masked_pwd), Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled(" ]█", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]),
    ];

    if let Some(ref err) = app.unlock_error {
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            format!("❌ {}", err),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )));
    }

    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::styled(" [Enter] 确认解锁  ", Style::default().fg(Color::Black).bg(Color::Green)),
        Span::raw("  "),
        Span::styled(" [Esc] 退出程序 ", Style::default().fg(Color::Black).bg(Color::Red)),
    ]));

    let paragraph = Paragraph::new(lines).block(block).alignment(Alignment::Center);
    frame.render_widget(paragraph, popup_area);
}

/// 辅助函数：计算居中弹窗 Rect 尺寸
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
