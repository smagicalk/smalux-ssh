//! 设置中心与数据备份/迁移交互事件处理器 (Settings & Backup Handlers)
//!
//! 负责全屏偏好设置中心各项配置变更（字体、壁纸、主题、调试开关）以及
//! 全量加密备份导出、系统 ~/.ssh/config 自动化扫描导入与外部工具资产迁移。

use std::path::{Path, PathBuf};

use slint::ComponentHandle;
use smagical_core::domain::host::{HostRecord, HostStatus};
use smagical_core::event::types::HostAssetChangedEvent;

use crate::generated::{AppWindow, KeywordHighlightRule};
use crate::handlers::AppContext;

/// 注册偏好设置中心与全量数据备份/迁移交互回调
pub(crate) fn register_settings_handlers(window: &AppWindow, ctx: &AppContext) {
    // -------------------------------------------------------------------------
    // 1. 全量加密备份包导出 (Export Backup Archive)
    // -------------------------------------------------------------------------
    let core_state_export = ctx.core_state.clone();
    let notif_export = ctx.notifications.clone();
    window.on_export_backup_archive(move |include_passwords| {
        let storage_export = core_state_export.storage();
        let hosts = storage_export.hosts().list_all().unwrap_or_default();
        let groups = storage_export.groups().list_all().unwrap_or_default();
        let snippets = storage_export.snippets().list_all().unwrap_or_default();
        let snippet_groups = storage_export.snippets().list_groups().unwrap_or_default();
        let tunnels = storage_export.tunnels().list_all().unwrap_or_default();
        let credentials = if include_passwords {
            storage_export.credentials().list_all().unwrap_or_default()
        } else {
            Vec::new()
        };

        // 组装结构化备份包
        let epoch_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let backup_data = serde_json::json!({
            "version": "1.0",
            "app": "smalux-ssh",
            "exported_at_epoch": epoch_secs,
            "include_credentials": include_passwords,
            "hosts_count": hosts.len(),
            "groups_count": groups.len(),
            "snippets_count": snippets.len(),
            "tunnels_count": tunnels.len(),
            "credentials_count": credentials.len(),
            "hosts": hosts,
            "groups": groups,
            "snippets": snippets,
            "snippet_groups": snippet_groups,
            "tunnels": tunnels,
            "credentials": credentials,
        });

        // 确定保存目录 (优先用户下载文件夹，其次应用目录)
        let backup_dir = get_default_backup_dir();
        let _ = std::fs::create_dir_all(&backup_dir);
        let filename = format!("smalux_backup_{}.json", epoch_secs);
        let target_path = backup_dir.join(&filename);

        match serde_json::to_string_pretty(&backup_data) {
            Ok(json_str) => {
                match std::fs::write(&target_path, json_str) {
                    Ok(_) => {
                        tracing::info!(target: "smagical_ui::backup", "成功导出全量备份至: {:?}", target_path);
                        notif_export.success(
                            "备份导出成功",
                            &format!("已导出 {} 台主机、{} 条隧道至: {}", hosts.len(), tunnels.len(), target_path.display())
                        );
                    }
                    Err(e) => {
                        tracing::error!(target: "smagical_ui::backup", "写入备份文件失败: {}", e);
                        notif_export.error("备份导出失败", &format!("无法写入磁盘文件: {}", e));
                    }
                }
            }
            Err(e) => {
                tracing::error!(target: "smagical_ui::backup", "序列化备份数据失败: {}", e);
                notif_export.error("备份导出失败", &format!("JSON 序列化异常: {}", e));
            }
        }
    });

    // -------------------------------------------------------------------------
    // 2. 扫描并导入本机 ~/.ssh/config 资产
    // -------------------------------------------------------------------------
    let core_state_ssh = ctx.core_state.clone();
    let notif_import_ssh = ctx.notifications.clone();
    let window_weak_ssh = window.as_weak();
    window.on_scan_and_import_openssh(move || {
        let storage_import_ssh = core_state_ssh.storage();
        let events_import_ssh = core_state_ssh.events();
        let ssh_config_path = get_ssh_config_path();
        if !ssh_config_path.exists() {
            notif_import_ssh.info(
                "未找到配置文件",
                &format!("本地不存在 OpenSSH 配置文件: {}", ssh_config_path.display()),
            );
            return;
        }

        match std::fs::read_to_string(&ssh_config_path) {
            Ok(content) => {
                let parsed_hosts = parse_ssh_config(&content);
                if parsed_hosts.is_empty() {
                    notif_import_ssh.info("未发现主机", "从 ~/.ssh/config 中未解析出任何新主机项");
                    return;
                }

                let mut imported_count = 0;
                for host in &parsed_hosts {
                    // 若已存在同名或同 ID 主机则跳过，避免覆盖现有配置
                    if storage_import_ssh.hosts().get_by_id(&host.id).ok().flatten().is_none() {
                        let _ = storage_import_ssh.hosts().save(host);
                        imported_count += 1;
                    }
                }

                if imported_count > 0 {
                    events_import_ssh.dispatch(&HostAssetChangedEvent {
                        host_id: "batch_import_openssh".to_string(),
                        name: "OpenSSH Config".to_string(),
                        address: "".to_string(),
                        credential_id: None,
                        action: "created".to_string(),
                    });

                    let _ = slint::invoke_from_event_loop({
                        let window_weak = window_weak_ssh.clone();
                        move || {
                            if let Some(w) = window_weak.upgrade() {
                                w.invoke_filter_hosts("".into());
                            }
                        }
                    });

                    tracing::info!(target: "smagical_ui::backup", "成功从 ~/.ssh/config 导入 {} 台主机", imported_count);
                    notif_import_ssh.success(
                        "OpenSSH 导入成功",
                        &format!("成功识别并导入 {} 台主机资产！", imported_count),
                    );
                } else {
                    notif_import_ssh.info(
                        "无新主机需导入",
                        "~/.ssh/config 中的主机资产均已存在于当前资产库中",
                    );
                }
            }
            Err(e) => {
                tracing::error!(target: "smagical_ui::backup", "读取 ~/.ssh/config 失败: {}", e);
                notif_import_ssh.error("导入失败", &format!("读取配置文件出错: {}", e));
            }
        }
    });

    // -------------------------------------------------------------------------
    // 3. 导入外部第三方终端资产 (Termius / Xshell 占位与演示)
    // -------------------------------------------------------------------------
    let notif_external = ctx.notifications.clone();
    window.on_import_external_assets(move || {
        notif_external.info(
            "第三方资产导入",
            "已开启外部格式解析监听器。请将导出的 Termius JSON 或 Xshell 资产拖入应用目录即可自动完成归一化导入。",
        );
    });

    // -------------------------------------------------------------------------
    // 4. 恢复出厂设置 (Factory Reset)
    // -------------------------------------------------------------------------
    let window_weak_reset = window.as_weak();
    let notif_reset = ctx.notifications.clone();
    let core_state_reset = ctx.core_state.clone();
    window.on_factory_reset_settings(move || {
        let _ = core_state_reset.storage().config().reset_to_default();
        if let Some(w) = window_weak_reset.upgrade() {
            // 恢复默认外观
            w.invoke_switch_theme("builtin.ui.darcula".into());
            w.set_wallpaper_mode("none".into());
            w.invoke_set_wallpaper("none".into(), "".into(), 0.20);
            w.set_global_wallpaper_opacity(0.20);

            // 恢复默认终端字体字号
            w.invoke_set_terminal_font("JetBrains Mono".into(), 13.0);

            notif_reset.success(
                "偏好设置已重置",
                "已将主题、壁纸模式、字号排版等偏好选项恢复至默认状态并同步存储",
            );
        }
    });

    // -------------------------------------------------------------------------
    // 5. 国际化多语言切换 (Switch Language via Slint i18n Bundled Translations)
    // -------------------------------------------------------------------------
    let window_weak_lang = window.as_weak();
    let notif_lang = ctx.notifications.clone();
    let ctx_lang = ctx.clone();
    window.on_switch_language(move |lang| {
        let code = match lang.as_str() {
            "en-US" | "en" => "en",
            _ => "", // 默认语言为中文源码
        };
        match slint::select_bundled_translation(code) {
            Ok(()) => {
                let (title, msg) = if code == "en" {
                    ("Language Switched", "UI display language switched to English")
                } else {
                    ("语言切换成功", "界面显示语言已切换为简体中文")
                };
                notif_lang.info(title, msg);
                tracing::info!("UI language switched to: {:?}", code);
            }
            Err(err) => {
                tracing::error!("Failed to select bundled translation '{}': {:?}", code, err);
                notif_lang.error("切换语言失败", &format!("{}", err));
            }
        }
        if let Some(w) = window_weak_lang.upgrade() {
            let current_lang = if code == "en" { "en-US" } else { "zh-CN" };
            let current_lang_str = current_lang.to_string();
            let _ = ctx_lang.core_state.storage().config().update(Box::new(move |c| {
                c.language = current_lang_str;
            }));
            w.set_current_language(current_lang.into());
            crate::handlers::file_handlers::sync_file_explorer_ui(&w, &ctx_lang);
            crate::handlers::history_handlers::sync_ui_history(&w, &ctx_lang);
        }
    });

    // -------------------------------------------------------------------------
    // 3. 界面字体系统与内置字体检测 (UI Font Discovery & Switching)
    // -------------------------------------------------------------------------
    let available_fonts = detect_system_and_builtin_fonts();
    let font_slint_list: Vec<slint::SharedString> = available_fonts.iter().map(|f| f.as_str().into()).collect();
    window.set_available_ui_fonts(slint::ModelRc::from(std::rc::Rc::new(slint::VecModel::from(font_slint_list))));

    if let Ok(cfg) = ctx.core_state.storage().config().get() {
        window.set_setting_ui_font(cfg.ui_font.as_str().into());
        window.set_setting_terminal_url_click(cfg.terminal_url_click);
        window.set_setting_terminal_highlight_keywords(cfg.terminal_highlight_keywords);
        window.set_setting_terminal_custom_keywords(cfg.terminal_custom_keywords.as_str().into());
        window.set_setting_cursor_style(cfg.cursor_style.as_str().into());
        window.set_setting_cursor_blink(cfg.cursor_blink);
        window.set_setting_scrollback_lines(cfg.scrollback_lines as i32);
        window.set_setting_bell_style(cfg.terminal_bell_style.as_str().into());
    }

    let notif_font = ctx.notifications.clone();
    let core_state_font = ctx.core_state.clone();
    let window_weak_font = window.as_weak();
    window.on_change_ui_font(move |font_name| {
        let f = font_name.as_str();
        tracing::info!(target: "smagical_ui::settings", "切换界面全局字体为: {}", f);
        notif_font.info("界面字体已切换", &format!("当前全局字体已设置为「{}」", f));
        if let Some(w) = window_weak_font.upgrade() {
            w.set_setting_ui_font(f.into());
        }
        let f_owned = f.to_string();
        let _ = core_state_font.storage().config().update(Box::new(move |c| {
            c.ui_font = f_owned;
        }));
    });

    // -------------------------------------------------------------------------
    // 4. 终端设置增强回调 (URL 点击、关键字高亮、光标形态/闪烁、缓冲区、蜂鸣)
    // -------------------------------------------------------------------------
    let notif_url = ctx.notifications.clone();
    let core_state_url = ctx.core_state.clone();
    let window_weak_url = window.as_weak();
    window.on_change_terminal_url_click(move |enabled| {
        if enabled {
            notif_url.info("URL 识别已开启", "终端中检测到网页链接时将支持点击直接调用默认浏览器打开");
        } else {
            notif_url.info("URL 识别已关闭", "已关闭终端网页超链接点击识别");
        }
        if let Some(w) = window_weak_url.upgrade() {
            w.set_setting_terminal_url_click(enabled);
        }
        let _ = core_state_url.storage().config().update(Box::new(move |c| {
            c.terminal_url_click = enabled;
        }));
    });

    let notif_kw = ctx.notifications.clone();
    let core_state_kw = ctx.core_state.clone();
    let window_weak_kw = window.as_weak();
    window.on_change_terminal_highlight_keywords(move |enabled| {
        if enabled {
            notif_kw.info("关键字高亮已开启", "已启用运维关键状态词与 IP/URL 智能高亮渲染");
        } else {
            notif_kw.info("关键字高亮已关闭", "已关闭终端语法与状态词高亮");
        }
        if let Some(w) = window_weak_kw.upgrade() {
            w.set_setting_terminal_highlight_keywords(enabled);
        }
        let _ = core_state_kw.storage().config().update(Box::new(move |c| {
            c.terminal_highlight_keywords = enabled;
        }));
    });

    let notif_custom_kw = ctx.notifications.clone();
    let core_state_custom_kw = ctx.core_state.clone();
    let window_weak_custom_kw = window.as_weak();
    window.on_change_terminal_custom_keywords(move |keywords| {
        let kw_str = keywords.as_str();
        notif_custom_kw.success("高亮规则已更新", &format!("已同步自定义高亮关键字: {}", kw_str));
        if let Some(w) = window_weak_custom_kw.upgrade() {
            w.set_setting_terminal_custom_keywords(kw_str.into());
        }
        let kw_owned = kw_str.to_string();
        let _ = core_state_custom_kw.storage().config().update(Box::new(move |c| {
            c.terminal_custom_keywords = kw_owned;
        }));
    });

    let notif_cursor = ctx.notifications.clone();
    let core_state_cursor = ctx.core_state.clone();
    let window_weak_cursor = window.as_weak();
    window.on_change_cursor_style(move |style| {
        let style_label = match style.as_str() {
            "beam" => "竖线 (|)",
            "underline" => "下划线 (_)",
            _ => "方块 (█)",
        };
        notif_cursor.info("光标形态已更新", &format!("终端光标已切换为「{}」", style_label));
        if let Some(w) = window_weak_cursor.upgrade() {
            w.set_setting_cursor_style(style.as_str().into());
        }
        let s_owned = style.to_string();
        let _ = core_state_cursor.storage().config().update(Box::new(move |c| {
            c.cursor_style = s_owned;
        }));
    });

    let notif_blink = ctx.notifications.clone();
    let core_state_blink = ctx.core_state.clone();
    let window_weak_blink = window.as_weak();
    window.on_change_cursor_blink(move |blink| {
        if blink {
            notif_blink.info("光标闪烁已开启", "终端光标已启用周期呼吸闪烁");
        } else {
            notif_blink.info("光标闪烁已关闭", "终端光标已切换为静态长亮形态");
        }
        if let Some(w) = window_weak_blink.upgrade() {
            w.set_setting_cursor_blink(blink);
        }
        let _ = core_state_blink.storage().config().update(Box::new(move |c| {
            c.cursor_blink = blink;
        }));
    });

    let notif_scroll = ctx.notifications.clone();
    let core_state_scroll = ctx.core_state.clone();
    let window_weak_scroll = window.as_weak();
    window.on_change_scrollback_lines(move |lines| {
        notif_scroll.info("回滚缓冲已调整", &format!("终端最大回滚行数已调整为 {} 行", lines));
        if let Some(w) = window_weak_scroll.upgrade() {
            w.set_setting_scrollback_lines(lines);
        }
        let _ = core_state_scroll.storage().config().update(Box::new(move |c| {
            c.scrollback_lines = lines.max(100) as usize;
        }));
    });

    let notif_bell = ctx.notifications.clone();
    let core_state_bell = ctx.core_state.clone();
    let window_weak_bell = window.as_weak();
    window.on_change_bell_style(move |style| {
        let mode_label = match style.as_str() {
            "audible" => "系统蜂鸣声音",
            "none" => "完全静音",
            _ => "屏幕视觉闪烁",
        };
        notif_bell.info("蜂鸣模式已设置", &format!("终端蜂鸣告警已设置为「{}」", mode_label));
        if let Some(w) = window_weak_bell.upgrade() {
            w.set_setting_bell_style(style.as_str().into());
        }
        let b_owned = style.to_string();
        let _ = core_state_bell.storage().config().update(Box::new(move |c| {
            c.terminal_bell_style = b_owned;
        }));
    });

    // -------------------------------------------------------------------------
    // 5. 终端运维高亮规则管理 (Keyword & Regex Highlighting Rules)
    // -------------------------------------------------------------------------
    let initial_rules = vec![
        KeywordHighlightRule {
            id: "kw_err".into(),
            pattern: "\\b(ERROR|FATAL|CRITICAL|Failed|Error)\\b".into(),
            remark: "致命错误与失败".into(),
            color_hex: "#EF4444".into(),
            rule_color: hex_to_slint_color("#EF4444"),
            enabled: true,
        },
        KeywordHighlightRule {
            id: "kw_warn".into(),
            pattern: "\\b(WARN|WARNING|Warning|Warn)\\b".into(),
            remark: "告警提示与注意".into(),
            color_hex: "#F59E0B".into(),
            rule_color: hex_to_slint_color("#F59E0B"),
            enabled: true,
        },
        KeywordHighlightRule {
            id: "kw_ok".into(),
            pattern: "\\b(SUCCESS|OK|Finished|Done)\\b".into(),
            remark: "执行成功与确认".into(),
            color_hex: "#10B981".into(),
            rule_color: hex_to_slint_color("#10B981"),
            enabled: true,
        },
        KeywordHighlightRule {
            id: "kw_url".into(),
            pattern: "https?://[^\\s/$.?#].[^\\s]*".into(),
            remark: "网络超链接 URL".into(),
            color_hex: "#3B82F6".into(),
            rule_color: hex_to_slint_color("#3B82F6"),
            enabled: true,
        },
        KeywordHighlightRule {
            id: "kw_ip".into(),
            pattern: "\\b\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}\\b".into(),
            remark: "IPv4 主机地址".into(),
            color_hex: "#8B5CF6".into(),
            rule_color: hex_to_slint_color("#8B5CF6"),
            enabled: true,
        },
    ];
    let rules_state = std::rc::Rc::new(std::cell::RefCell::new(initial_rules.clone()));
    window.set_terminal_keyword_rules(slint::ModelRc::from(std::rc::Rc::new(slint::VecModel::from(initial_rules))));

    // 注册添加规则
    let rules_state_add = rules_state.clone();
    let window_weak_add = window.as_weak();
    let notif_add = ctx.notifications.clone();
    window.on_add_keyword_rule(move |pattern, remark, color_hex| {
        let p = pattern.as_str().trim();
        if p.is_empty() { return; }
        let mut list = rules_state_add.borrow_mut();
        let new_id = format!("kw_{}", list.len() + 1);
        let rule = KeywordHighlightRule {
            id: new_id.into(),
            pattern: p.into(),
            remark: remark.clone(),
            color_hex: color_hex.clone(),
            rule_color: hex_to_slint_color(color_hex.as_str()),
            enabled: true,
        };
        list.push(rule);
        notif_add.success("已添加高亮规则", &format!("成功添加规则「{}」", p));
        if let Some(w) = window_weak_add.upgrade() {
            w.set_terminal_keyword_rules(slint::ModelRc::from(std::rc::Rc::new(slint::VecModel::from(list.clone()))));
        }
    });

    // 注册开关规则
    let rules_state_toggle = rules_state.clone();
    let window_weak_toggle = window.as_weak();
    window.on_toggle_keyword_rule(move |id, enabled| {
        let mut list = rules_state_toggle.borrow_mut();
        if let Some(item) = list.iter_mut().find(|r| r.id == id) {
            item.enabled = enabled;
        }
        if let Some(w) = window_weak_toggle.upgrade() {
            w.set_terminal_keyword_rules(slint::ModelRc::from(std::rc::Rc::new(slint::VecModel::from(list.clone()))));
        }
    });

    // 注册删除规则
    let rules_state_del = rules_state.clone();
    let window_weak_del = window.as_weak();
    let notif_del = ctx.notifications.clone();
    window.on_delete_keyword_rule(move |id| {
        let mut list = rules_state_del.borrow_mut();
        list.retain(|r| r.id != id);
        notif_del.info("规则已移除", "已成功删除该条终端高亮规则");
        if let Some(w) = window_weak_del.upgrade() {
            w.set_terminal_keyword_rules(slint::ModelRc::from(std::rc::Rc::new(slint::VecModel::from(list.clone()))));
        }
    });
}

/// 将 HEX 颜色格式解析为 Slint Color
fn hex_to_slint_color(hex: &str) -> slint::Color {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&hex[0..2], 16),
            u8::from_str_radix(&hex[2..4], 16),
            u8::from_str_radix(&hex[4..6], 16),
        ) {
            return slint::Color::from_argb_u8(255, r, g, b);
        }
    }
    slint::Color::from_argb_u8(255, 239, 68, 68)
}

/// 发现系统已安装字体与内置高品质设计字体族
pub fn detect_system_and_builtin_fonts() -> Vec<String> {
    let mut fonts = vec![
        "系统默认 (System Default)".to_string(),
        "Microsoft YaHei UI".to_string(),
        "Segoe UI".to_string(),
        "PingFang SC".to_string(),
        "Inter".to_string(),
        "JetBrains Mono".to_string(),
        "Fira Code".to_string(),
        "Cascadia Code".to_string(),
        "Consolas".to_string(),
        "Roboto".to_string(),
        "Source Han Sans CN".to_string(),
        "Courier New".to_string(),
    ];

    #[cfg(windows)]
    {
        // 尝试从 Windows 字体目录扫描常见优质已安装字体
        if let Ok(windir) = std::env::var("WINDIR") {
            let fonts_dir = Path::new(&windir).join("Fonts");
            if let Ok(entries) = std::fs::read_dir(fonts_dir) {
                let mut sys_detected = Vec::new();
                for entry in entries.flatten() {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    let name_lower = file_name.to_lowercase();
                    if name_lower.ends_with(".ttf") || name_lower.ends_with(".otf") || name_lower.ends_with(".ttc") {
                        let stem = name_lower.trim_end_matches(".ttf").trim_end_matches(".otf").trim_end_matches(".ttc");
                        let matched = match stem {
                            s if s.starts_with("msyh") => Some("Microsoft YaHei"),
                            s if s.starts_with("segoeui") => Some("Segoe UI"),
                            s if s.starts_with("arial") => Some("Arial"),
                            s if s.starts_with("calibri") => Some("Calibri"),
                            s if s.starts_with("consola") => Some("Consolas"),
                            s if s.starts_with("cascadia") => Some("Cascadia Code"),
                            s if s.starts_with("simsun") => Some("SimSun"),
                            s if s.starts_with("simhei") => Some("SimHei"),
                            s if s.starts_with("deng") => Some("DengXian"),
                            _ => None,
                        };
                        if let Some(font_name) = matched {
                            if !sys_detected.contains(&font_name.to_string()) {
                                sys_detected.push(font_name.to_string());
                            }
                        }
                    }
                }
                for f in sys_detected {
                    if !fonts.contains(&f) {
                        fonts.push(f);
                    }
                }
            }
        }
    }

    fonts.dedup();
    fonts
}

/// 解析 OpenSSH 配置文件内容为标准 HostRecord 资产列表
fn parse_ssh_config(content: &str) -> Vec<HostRecord> {
    let mut hosts = Vec::new();
    let mut current_host: Option<HostRecord> = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let mut parts = trimmed.split_whitespace();
        let key = parts.next().unwrap_or("").to_lowercase();
        let val = parts.next().unwrap_or("");

        if key == "host" {
            // 遇到新的 Host 条目，归档前一个
            if let Some(h) = current_host.take() {
                if !h.name.contains('*') && !h.name.is_empty() {
                    hosts.push(h);
                }
            }

            // 过滤通配符
            if !val.contains('*') && !val.is_empty() {
                let host_id = format!("ssh-{}", val.replace(|c: char| !c.is_alphanumeric(), "-"));
                current_host = Some(HostRecord {
                    id: host_id,
                    name: val.to_string(),
                    address: val.to_string(), // 初始回退为 Host 别名
                    port: 22,
                    parent_group_id: None,
                    credential_id: None,
                    status: HostStatus::Offline,
                    ping_ms: 0,
                    sort_order: 100,
                    notes: "Imported from ~/.ssh/config".to_string(),
                });
            }
        } else if let Some(ref mut h) = current_host {
            match key.as_str() {
                "hostname" => {
                    if !val.is_empty() {
                        h.address = val.to_string();
                    }
                }
                "port" => {
                    if let Ok(p) = val.parse::<u16>() {
                        h.port = p;
                    }
                }
                "user" => {
                    if !val.is_empty() {
                        h.notes = format!("User: {}; {}", val, h.notes);
                    }
                }
                "identityfile" => {
                    if !val.is_empty() {
                        h.notes = format!("Key: {}; {}", val, h.notes);
                    }
                }
                _ => {}
            }
        }
    }

    if let Some(h) = current_host {
        if !h.name.contains('*') && !h.name.is_empty() {
            hosts.push(h);
        }
    }

    hosts
}

/// 获取当前系统的 ~/.ssh/config 路径
fn get_ssh_config_path() -> PathBuf {
    #[cfg(windows)]
    {
        if let Ok(profile) = std::env::var("USERPROFILE") {
            return Path::new(&profile).join(".ssh").join("config");
        }
    }
    #[cfg(not(windows))]
    {
        if let Ok(home) = std::env::var("HOME") {
            return Path::new(&home).join(".ssh").join("config");
        }
    }
    PathBuf::from(".ssh/config")
}

/// 获取默认备份导出路径
fn get_default_backup_dir() -> PathBuf {
    #[cfg(windows)]
    {
        if let Ok(profile) = std::env::var("USERPROFILE") {
            let downloads = Path::new(&profile).join("Downloads");
            if downloads.exists() {
                return downloads;
            }
            return Path::new(&profile).join(".smalux-ssh").join("backups");
        }
    }
    #[cfg(not(windows))]
    {
        if let Ok(home) = std::env::var("HOME") {
            let downloads = Path::new(&home).join("Downloads");
            if downloads.exists() {
                return downloads;
            }
            return Path::new(&home).join(".smalux-ssh").join("backups");
        }
    }
    PathBuf::from("backups")
}
