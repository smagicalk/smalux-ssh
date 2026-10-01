//! 全屏偏好设置中心与配置联动事件处理器 (Settings Handlers Facade)
//!
//! 统一聚合偏好设置各业务子模块（AI、备份/迁移、语法高亮规则、快捷键映射与通用辅助函数），
//! 并挂载通用全局设置（字体、终端选项、代理与网络、SFTP 与日志级别）。
//! 全部配置读取与更新均为纯原生异步非阻塞操作 (0ms UI 阻塞)。

pub(crate) mod ai;
pub(crate) mod backup;
pub(crate) mod highlight;
pub(crate) mod security;
pub(crate) mod utils;

pub(crate) use security::{check_vault_lock_on_boot, start_auto_lock_monitor};
use utils::{detect_system_and_builtin_fonts, hex_to_slint_color, parse_speed_limit};

use std::rc::Rc;
use slint::ComponentHandle;
use slint::winit_030::WinitWindowAccessor;
use crate::async_util::spawn_async;
use crate::generated::{AppTheme, AppWindow, SettingsBridge, WindowBridge};
use crate::handlers::AppContext;
use super::theme_handlers::pick_folder;

/// 将持久化应用全局配置实体 `AppConfigRecord` 深度水合映射写入 Slint `SettingsBridge` 视图模型
///
/// # 映射覆盖范畴
/// 1. **常规与启动行为 (General)**：语言环境、窗口关闭行为（最小化托盘/退出）、标签关闭确认、开机自启、置顶、Toast 停留时间与自定义数据目录；
/// 2. **外观与壁纸 (Appearance)**：主题 ID、深色模式、UI 界面字体、壁纸模式、透明度、背景遮罩不透明度、轮播定时器规格与过渡动效；
/// 3. **终端排版与特效 (Terminal)**：字体名称/字号、光标样式与闪烁、回滚行数、选中文本自动复制、右键直接粘贴、多行危险粘贴二次确认、响铃样式、CRT 显像管复古着色器与关键词高亮规则；
/// 4. **网络与代理 (Network & Proxy)**：全局出网代理协议、主机、端口、用户名/密码与探测超时；
/// 5. **SFTP 与文件传输 (SFTP Transfer)**：冲突覆盖策略、隐藏文件展示、单通道并发度、上传/下载带宽限速、软链接解析与断点续传块大小；
/// 6. **AI 智能助手多端点 (AI Assistants)**：各端点 Profile、激活端点高级超参数与上下文预算。
///
/// # 参数
/// - `bridge`: Slint `SettingsBridge` 全局单例句柄；
/// - `cfg`: SQLite 仓储中拉取出的应用全局配置实体切片。
pub(crate) fn apply_config_to_settings_bridge(bridge: &SettingsBridge, cfg: &smagical_core::domain::config::AppConfigRecord) {
    // 1. 常规与启动行为 (General)
    bridge.set_setting_language(cfg.language.as_str().into());
    bridge.set_setting_close_action(cfg.close_action.as_str().into());
    bridge.set_setting_confirm_close_tab(cfg.confirm_close_tab);
    bridge.set_setting_confirm_close_active(cfg.confirm_close_active);
    bridge.set_setting_always_on_top(cfg.always_on_top);
    bridge.set_setting_start_on_boot(cfg.start_on_boot);
    bridge.set_setting_toast_duration(cfg.toast_duration.as_str().into());
    if !cfg.custom_data_dir.is_empty() {
        bridge.set_setting_data_dir(cfg.custom_data_dir.as_str().into());
    }

    // 2. 外观与壁纸 (Appearance)
    bridge.set_current_theme_id(cfg.theme_id.as_str().into());
    bridge.set_is_dark_mode(cfg.is_dark_mode);
    bridge.set_setting_ui_font(cfg.ui_font.as_str().into());
    bridge.set_setting_wallpaper_mode(cfg.wallpaper_mode.as_str().into());
    bridge.set_setting_wallpaper_path(cfg.wallpaper_path.as_str().into());
    bridge.set_setting_wallpaper_opacity(cfg.wallpaper_opacity);
    bridge.set_setting_modal_opacity(cfg.modal_opacity);

    let slint_strings: Vec<slint::SharedString> = cfg.wallpaper_list.iter().map(|s| s.as_str().into()).collect();
    bridge.set_setting_wallpaper_list(slint::ModelRc::new(slint::VecModel::from(slint_strings)));
    bridge.set_setting_wallpaper_active_index(cfg.wallpaper_active_index as i32);

    let slide_interval = cfg.wallpaper_slideshow_interval.clone();
    let (slide_num, slide_unit) = if slide_interval == "none" || slide_interval == "off" || slide_interval.is_empty() || slide_interval == "startup" {
        ("0", "off")
    } else if let Some(s) = slide_interval.strip_suffix('s') {
        (s, "s")
    } else if let Some(m) = slide_interval.strip_suffix('m') {
        (m, "m")
    } else if let Some(h) = slide_interval.strip_suffix('h') {
        (h, "h")
    } else if let Ok(_) = slide_interval.parse::<u64>() {
        (slide_interval.as_str(), "m")
    } else {
        ("0", "off")
    };
    bridge.set_setting_wallpaper_slideshow(slide_interval.as_str().into());
    bridge.set_slideshow_number_input(slide_num.into());
    bridge.set_slideshow_unit_input(slide_unit.into());
    bridge.set_setting_wallpaper_transition(cfg.wallpaper_transition_effect.as_str().into());

    // 3. 终端排版与特效 (Terminal)
    bridge.set_setting_terminal_font(cfg.font_family.as_str().into());
    bridge.set_setting_terminal_font_size(cfg.font_size as i32);
    bridge.set_setting_cursor_style(cfg.cursor_style.as_str().into());
    bridge.set_setting_cursor_blink(cfg.cursor_blink);
    bridge.set_setting_scrollback_lines(cfg.scrollback_lines as i32);
    bridge.set_scrollback_input(format!("{}", cfg.scrollback_lines).into());
    bridge.set_setting_copy_on_select(cfg.copy_on_select);
    bridge.set_setting_paste_on_right_click(cfg.paste_on_right_click);
    bridge.set_setting_warn_multiline_paste(cfg.warn_on_multiline_paste);
    bridge.set_setting_bell_style(cfg.terminal_bell_style.as_str().into());
    bridge.set_setting_terminal_url_click(cfg.terminal_url_click);
    bridge.set_setting_terminal_highlight_keywords(cfg.terminal_highlight_keywords);
    bridge.set_setting_terminal_custom_keywords(cfg.terminal_custom_keywords.as_str().into());
    bridge.set_flag_terminal_crt_shader(cfg.flag_terminal_crt_shader);
    bridge.set_setting_terminal_crt(cfg.flag_terminal_crt_shader);

    if !cfg.keyword_highlight_rules.is_empty() {
        let rules: Vec<crate::generated::KeywordHighlightRule> = cfg.keyword_highlight_rules.iter().map(|r| {
            crate::generated::KeywordHighlightRule {
                id: r.id.as_str().into(),
                pattern: r.pattern.as_str().into(),
                remark: r.remark.as_str().into(),
                color_hex: r.color_hex.as_str().into(),
                rule_color: hex_to_slint_color(r.color_hex.as_str()),
                enabled: r.enabled,
            }
        }).collect();
        bridge.set_terminal_keyword_rules(slint::ModelRc::new(slint::VecModel::from(rules)));
    }

    // 4. 网络与 SSH 设置 (Network & SSH)
    bridge.set_setting_global_proxy_mode(cfg.global_proxy_mode.as_str().into());
    bridge.set_setting_global_proxy_server(cfg.global_proxy_server.as_str().into());
    bridge.set_setting_global_proxy_auth(cfg.global_proxy_auth);
    bridge.set_setting_global_proxy_user(cfg.global_proxy_user.as_str().into());
    bridge.set_setting_global_proxy_pass(cfg.global_proxy_pass.as_str().into());
    bridge.set_setting_connect_timeout(cfg.ssh_timeout_seconds as i32);
    bridge.set_setting_keepalive_interval(cfg.keepalive_interval as i32);
    bridge.set_setting_keepalive_count_max(cfg.keepalive_count_max as i32);
    bridge.set_setting_host_key_policy(cfg.host_key_checking.as_str().into());
    bridge.set_setting_tcp_nodelay(cfg.tcp_nodelay);
    bridge.set_setting_compression(cfg.compression);
    bridge.set_setting_legacy_ciphers(cfg.legacy_ciphers);
    bridge.set_setting_auto_reconnect(cfg.auto_reconnect);

    // 5. 传输与文件管理 (Files & SFTP)
    bridge.set_setting_sftp_default_local(cfg.sftp_default_local.as_str().into());
    bridge.set_setting_sftp_default_remote(cfg.sftp_default_remote.as_str().into());
    bridge.set_setting_sftp_show_hidden(cfg.sftp_show_hidden);
    bridge.set_setting_sftp_confirm_delete(cfg.sftp_confirm_delete);
    bridge.set_setting_sftp_conflict_policy(cfg.sftp_conflict_policy.as_str().into());
    bridge.set_setting_sftp_concurrency(cfg.sftp_concurrency as i32);
    bridge.set_setting_sftp_resume(cfg.sftp_resume_transfer);
    bridge.set_setting_sftp_preserve_attributes(cfg.sftp_preserve_attributes);
    bridge.set_setting_sftp_upload_limit(cfg.sftp_upload_limit.as_str().into());
    let (up_num, up_unit) = parse_speed_limit(&cfg.sftp_upload_limit);
    bridge.set_upload_limit_num(up_num.into());
    bridge.set_upload_limit_unit(up_unit.into());

    bridge.set_setting_sftp_download_limit(cfg.sftp_download_limit.as_str().into());
    let (dl_num, dl_unit) = parse_speed_limit(&cfg.sftp_download_limit);
    bridge.set_download_limit_num(dl_num.into());
    bridge.set_download_limit_unit(dl_unit.into());
    bridge.set_setting_sftp_editor_mode(cfg.sftp_editor_mode.as_str().into());
    bridge.set_setting_sftp_custom_editor(cfg.sftp_custom_editor.as_str().into());
    bridge.set_setting_sftp_exclude_patterns(cfg.sftp_exclude_patterns.as_str().into());

    // 6. 云同步与备份 (Cloud Sync & Backup)
    bridge.set_setting_cloud_sync_backend(cfg.cloud_sync_backend.as_str().into());
    bridge.set_setting_cloud_sync_interval(cfg.cloud_sync_interval.as_str().into());
    bridge.set_setting_cloud_e2ee_pass(cfg.cloud_sync_e2ee_pass.as_str().into());
    bridge.set_setting_webdav_url(cfg.cloud_sync_webdav_url.as_str().into());
    bridge.set_setting_webdav_user(cfg.cloud_sync_webdav_user.as_str().into());
    bridge.set_setting_webdav_pass(cfg.cloud_sync_webdav_pass.as_str().into());
    bridge.set_setting_webdav_dir(cfg.cloud_sync_webdav_dir.as_str().into());
    bridge.set_setting_s3_endpoint(cfg.cloud_sync_s3_endpoint.as_str().into());
    bridge.set_setting_s3_bucket(cfg.cloud_sync_s3_bucket.as_str().into());
    bridge.set_setting_s3_region(cfg.cloud_sync_s3_region.as_str().into());
    bridge.set_setting_s3_key_id(cfg.cloud_sync_s3_key_id.as_str().into());
    bridge.set_setting_s3_access_key(cfg.cloud_sync_s3_access_key.as_str().into());
    bridge.set_setting_gist_id(cfg.cloud_sync_gist_id.as_str().into());
    bridge.set_setting_gist_token(cfg.cloud_sync_gist_token.as_str().into());
    bridge.set_setting_custom_sync_url(cfg.cloud_sync_custom_url.as_str().into());
    bridge.set_setting_custom_sync_client_id(cfg.cloud_sync_custom_client_id.as_str().into());
    bridge.set_setting_custom_sync_token(cfg.cloud_sync_custom_token.as_str().into());

    // 7. 安全与锁屏策略 (Security & Vault)
    bridge.set_setting_auto_lock_timeout(cfg.auto_lock_timeout.as_str().into());
    bridge.set_setting_lock_on_minimize(cfg.lock_on_minimize);
    bridge.set_setting_biometric_unlock(cfg.biometric_unlock);
    bridge.set_setting_clear_clipboard_timeout(cfg.clear_clipboard_timeout);
    bridge.set_setting_confirm_dangerous_commands(cfg.confirm_dangerous_commands);
    bridge.set_setting_session_audit_logging(cfg.session_audit_logging);

    // 8. 日志与追踪级别 (Diagnostics & Tracing)
    let cur_lvl = if cfg.log_level.is_empty() { "INFO".to_string() } else { cfg.log_level.to_uppercase() };
    crate::debug::tracing_layer::set_global_runtime_log_level(&cur_lvl);
    bridge.set_setting_log_level(cur_lvl.as_str().into());

    // 9. AI 助手配置与大模型推理 (AI Copilot & Endpoints)
    bridge.set_setting_ai_provider(cfg.ai_active_provider.as_str().into());
    bridge.set_setting_ai_system_prompt(cfg.ai_system_prompt.as_str().into());
    bridge.set_setting_ai_auto_audit_level(cfg.ai_auto_audit_level.as_str().into());

    if !cfg.ai_endpoints.is_empty() {
        let mut slint_endpoints: Vec<crate::generated::AiEndpointProfile> = Vec::new();
        let mut active_ep_opt = None;

        for ep in &cfg.ai_endpoints {
            let slint_ep = crate::generated::AiEndpointProfile {
                id: ep.id.as_str().into(),
                name: ep.name.as_str().into(),
                base_url: ep.base_url.as_str().into(),
                api_key: ep.api_key.as_str().into(),
                api_mode: ep.api_mode.as_str().into(),
                selected_model: ep.selected_model.as_str().into(),
                models_csv: ep.models_csv.as_str().into(),
                is_active: ep.is_active,
                status_text: ep.status_text.as_str().into(),
                thinking_degree: ep.thinking_degree.as_str().into(),
                timeout_secs: ep.timeout_secs,
                max_context: ep.max_context,
                max_retries: ep.max_retries,
                custom_headers: ep.custom_headers.as_str().into(),
                temperature: ep.temperature.as_str().into(),
            };
            if ep.is_active && active_ep_opt.is_none() {
                active_ep_opt = Some(ep.clone());
            }
            slint_endpoints.push(slint_ep);
        }

        if active_ep_opt.is_none() && !cfg.ai_endpoints.is_empty() {
            active_ep_opt = Some(cfg.ai_endpoints[0].clone());
            slint_endpoints[0].is_active = true;
            slint_endpoints[0].status_text = "已连接".into();
        }

        bridge.set_setting_ai_endpoints(slint::ModelRc::new(slint::VecModel::from(slint_endpoints)));

        if let Some(active) = active_ep_opt {
            bridge.set_active_endpoint_id(active.id.as_str().into());
            bridge.set_setting_ai_base_url(active.base_url.as_str().into());
            bridge.set_setting_ai_api_key(active.api_key.as_str().into());
            bridge.set_setting_ai_model(active.selected_model.as_str().into());

            let mut models_vec: Vec<slint::SharedString> = Vec::new();
            for m in active.models_csv.split(',') {
                let trimmed = m.trim();
                if !trimmed.is_empty() {
                    models_vec.push(trimmed.into());
                }
            }
            if models_vec.is_empty() && !active.selected_model.is_empty() {
                models_vec.push(active.selected_model.as_str().into());
            }
            bridge.set_setting_ai_current_models(slint::ModelRc::new(slint::VecModel::from(models_vec)));

            let th = if active.thinking_degree.is_empty() { "medium" } else { &active.thinking_degree };
            bridge.set_setting_ai_thinking_budget(th.into());
            let timeout = if active.timeout_secs <= 0 { 60 } else { active.timeout_secs };
            bridge.set_setting_ai_timeout_secs(timeout);
            bridge.set_setting_ai_timeout_secs_input(timeout.to_string().into());
            let ctx = if active.max_context <= 0 { 32768 } else { active.max_context };
            bridge.set_setting_ai_max_context(ctx);
            bridge.set_setting_ai_max_context_input(ctx.to_string().into());
            let retries = if active.max_retries < 0 { 2 } else { active.max_retries };
            bridge.set_setting_ai_max_retries(retries);
            bridge.set_setting_ai_max_retries_input(retries.to_string().into());
            bridge.set_setting_ai_custom_headers(active.custom_headers.as_str().into());
            let temp_str = if active.temperature.is_empty() { "0.3" } else { &active.temperature };
            bridge.set_setting_ai_temperature(temp_str.parse::<f32>().unwrap_or(0.3));
            bridge.set_setting_ai_temperature_input(temp_str.into());
        }
    }
}

/// 注册偏好设置中心各项配置变更交互回调
///
/// # 模块分发治理
/// 1. 委托子模块初始化：
///    - `backup::register_backup_handlers`: 快照备份、导入导出与出厂重置；
///    - `highlight::register_highlight_handlers`: 终端高亮规则增删改查与极坐标拾色器；
///    - `ai::register_ai_handlers`: AI 多端点生命周期与高级推理超参数；
///    - `security::register_security_handlers`: 主密码、信封封装与保险库加锁；
/// 2. 挂载通用偏好设置事件：
///    - 语言国际化即时切换 (`select_bundled_translation`) 并通知各抽屉与文件树重塑文案；
///    - 窗口关闭策略、开机自启系统注册表键修改；
///    - 终端字体与本地可用字体探测枚举 (`detect_system_and_builtin_fonts`)；
///    - CRT 显像管着色器特效、光标闪烁与回滚历史条数；
///    - SFTP 传输限速解析 (`parse_speed_limit`) 与并发通道策略；
///    - 网络出网代理参数动态更新。
///
/// # 参数
/// - `window`: Slint 顶级应用主窗口；
/// - `ctx`: 应用程序全局上下文引用。
pub(crate) fn register_settings_handlers(window: &AppWindow, ctx: &AppContext) {
    let bridge = window.global::<SettingsBridge>();

    // 1. 注册各子模块独立回调
    backup::register_backup_handlers(window, ctx);
    highlight::register_highlight_handlers(window, ctx);
    ai::register_ai_handlers(window, ctx);
    security::register_security_handlers(window, ctx);

    // 2. 国际化多语言切换 (Switch Language via Slint i18n Bundled Translations)
    let switch_lang_impl = {
        let window_weak = window.as_weak();
        let notif = ctx.notifications.clone();
        let ctx_clone = ctx.clone();
        move |lang: slint::SharedString| {
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
                    notif.info(title, msg);
                    tracing::info!("UI language switched to: {:?}", code);
                }
                Err(err) => {
                    tracing::error!("Failed to select bundled translation '{}': {:?}", code, err);
                    notif.error("切换语言失败", &format!("{}", err));
                }
            }
            if let Some(w) = window_weak.upgrade() {
                let current_lang = if code == "en" { "en-US" } else { "zh-CN" };
                let current_lang_str = current_lang.to_string();
                let storage = ctx_clone.core_state.storage();
                spawn_async(async move {
                    let _ = storage.config().update(Box::new(move |c| {
                        c.language = current_lang_str;
                    })).await;
                });
                w.global::<WindowBridge>().set_current_language(current_lang.into());
                w.global::<SettingsBridge>().set_setting_language(current_lang.into());
                crate::activity_bar_service::sync_activity_bar_ui(&w, &ctx_clone.core_state);
                crate::right_panel_service::sync_right_panel_ui(&w, &ctx_clone.core_state);
                crate::handlers::file_handlers::sync_file_explorer_ui(&w, &ctx_clone);
                crate::handlers::history_handlers::sync_ui_history(&w, &ctx_clone);
            }
        }
    };

    bridge.on_switch_language({
        let switch_fn = switch_lang_impl.clone();
        move |lang| switch_fn(lang)
    });

    window.global::<WindowBridge>().on_switch_language(move |lang| {
        switch_lang_impl(lang);
    });

    // 3. 界面字体系统与内置字体检测 (UI Font Discovery & Switching)
    let available_fonts = detect_system_and_builtin_fonts();
    let font_slint_list: Vec<slint::SharedString> = available_fonts.iter().map(|f| f.as_str().into()).collect();
    bridge.set_available_ui_fonts(slint::ModelRc::from(std::rc::Rc::new(slint::VecModel::from(font_slint_list))));

    // 异步加载初始持久化配置到 SettingsBridge (0ms UI 阻塞)
    let storage_init = ctx.core_state.storage();
    let w_weak_init = window.as_weak();
    spawn_async(async move {
        if let Ok(cfg) = storage_init.config().get().await {
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(w) = w_weak_init.upgrade() {
                    apply_config_to_settings_bridge(&w.global::<SettingsBridge>(), &cfg);
                    let sb = w.global::<SettingsBridge>();
                    let ai_b = w.global::<crate::generated::AiBridge>();
                    ai_b.set_available_models(sb.get_setting_ai_current_models());
                    ai_b.set_selected_model(sb.get_setting_ai_model());
                    if cfg.always_on_top {
                        w.window().with_winit_window(|win| {
                            win.set_window_level(slint::winit_030::winit::window::WindowLevel::AlwaysOnTop);
                        });
                    }
                }
            });
        }
    });

    let storage_mo = ctx.core_state.storage();
    let window_weak_mo = window.as_weak();
    bridge.on_change_modal_opacity(move |opacity| {
        let op = opacity.clamp(0.5, 1.0);
        if let Some(w) = window_weak_mo.upgrade() {
            w.global::<SettingsBridge>().set_setting_modal_opacity(op);
            let theme_global = w.global::<AppTheme>();
            theme_global.set_modal_opacity(op);
        }
        let storage = storage_mo.clone();
        spawn_async(async move {
            let _ = storage.config().update(Box::new(move |c| {
                c.modal_opacity = op;
            })).await;
        });
    });

    let storage_wp = ctx.core_state.storage().clone();
    let window_weak_wp = window.as_weak();
    let wallpapers_ref_wp = Rc::clone(&ctx.wallpapers);
    let active_idx_ref_wp = Rc::clone(&ctx.active_wallpaper_idx);
    bridge.on_change_wallpaper(move |mode, path, opacity| {
        let mode_str = mode.as_str().to_string();
        let mut path_str = path.as_str().to_string();
        let op = opacity.clamp(0.01, 1.0);

        if (path_str.is_empty() || !std::path::Path::new(&path_str).exists()) && mode_str != "none" {
            let wps = wallpapers_ref_wp.borrow();
            let idx = *active_idx_ref_wp.borrow();
            if !wps.is_empty() && idx < wps.len() && std::path::Path::new(&wps[idx]).exists() {
                path_str = wps[idx].clone();
            } else if let Some(first) = wps.iter().find(|p| std::path::Path::new(p).exists()) {
                path_str = first.clone();
            }
        }

        if let Some(w) = window_weak_wp.upgrade() {
            let sb = w.global::<SettingsBridge>();
            sb.set_setting_wallpaper_mode(mode_str.as_str().into());
            sb.set_setting_wallpaper_opacity(op);
            if !path_str.is_empty() {
                sb.set_setting_wallpaper_path(path_str.as_str().into());
            }

            let wb = w.global::<WindowBridge>();
            wb.set_wallpaper_mode(mode_str.as_str().into());
            wb.set_global_wallpaper_opacity(op);
            wb.set_terminal_wallpaper_opacity(op);
            if !path_str.is_empty() {
                wb.set_wallpaper_path(path_str.as_str().into());
            }

            let theme_global = w.global::<AppTheme>();
            theme_global.set_wallpaper_mode(mode_str.as_str().into());
            theme_global.set_wallpaper_opacity(op);

            wb.invoke_set_wallpaper(mode_str.as_str().into(), path_str.as_str().into(), op);
        }

        let storage = storage_wp.clone();
        let m_to_save = mode_str.clone();
        let p_to_save = path_str.clone();
        spawn_async(async move {
            let _ = storage.config().update(Box::new(move |c| {
                c.wallpaper_mode = m_to_save;
                c.wallpaper_path = p_to_save;
                c.wallpaper_opacity = op;
            })).await;
        });
    });

    let notif_font = ctx.notifications.clone();
    let storage_font = ctx.core_state.storage();
    let window_weak_font = window.as_weak();
    bridge.on_change_ui_font(move |font_name| {
        let f = font_name.as_str();
        tracing::info!(target: "smagical_ui::settings", "切换界面全局字体为: {}", f);
        notif_font.info("界面字体已切换", &format!("当前全局字体已设置为「{}」", f));
        if let Some(w) = window_weak_font.upgrade() {
            w.global::<SettingsBridge>().set_setting_ui_font(f.into());
        }
        let f_owned = f.to_string();
        let storage = storage_font.clone();
        spawn_async(async move {
            let _ = storage.config().update(Box::new(move |c| {
                c.ui_font = f_owned;
            })).await;
        });
    });

    // 4. 终端设置增强回调 (URL 点击、关键字高亮、光标形态/闪烁、缓冲区、蜂鸣)
    let notif_url = ctx.notifications.clone();
    let storage_url = ctx.core_state.storage();
    let window_weak_url = window.as_weak();
    bridge.on_change_terminal_url_click(move |enabled| {
        if enabled {
            notif_url.info("URL 识别已开启", "终端中检测到网页链接时将支持点击直接调用默认浏览器打开");
        } else {
            notif_url.info("URL 识别已关闭", "已关闭终端网页超链接点击识别");
        }
        if let Some(w) = window_weak_url.upgrade() {
            w.global::<SettingsBridge>().set_setting_terminal_url_click(enabled);
        }
        let storage = storage_url.clone();
        spawn_async(async move {
            let _ = storage.config().update(Box::new(move |c| {
                c.terminal_url_click = enabled;
            })).await;
        });
    });

    let notif_kw = ctx.notifications.clone();
    let storage_kw = ctx.core_state.storage();
    let window_weak_kw = window.as_weak();
    bridge.on_change_terminal_highlight_keywords(move |enabled| {
        if enabled {
            notif_kw.info("关键字高亮已开启", "已启用运维关键状态词与 IP/URL 智能高亮渲染");
        } else {
            notif_kw.info("关键字高亮已关闭", "已关闭终端语法与状态词高亮");
        }
        if let Some(w) = window_weak_kw.upgrade() {
            w.global::<SettingsBridge>().set_setting_terminal_highlight_keywords(enabled);
        }
        let storage = storage_kw.clone();
        spawn_async(async move {
            let _ = storage.config().update(Box::new(move |c| {
                c.terminal_highlight_keywords = enabled;
            })).await;
        });
    });

    let notif_custom_kw = ctx.notifications.clone();
    let storage_custom_kw = ctx.core_state.storage();
    let window_weak_custom_kw = window.as_weak();
    bridge.on_change_terminal_custom_keywords(move |keywords| {
        let kw_str = keywords.as_str();
        notif_custom_kw.success("高亮规则已更新", &format!("已同步自定义高亮关键字: {}", kw_str));
        if let Some(w) = window_weak_custom_kw.upgrade() {
            w.global::<SettingsBridge>().set_setting_terminal_custom_keywords(kw_str.into());
        }
        let kw_owned = kw_str.to_string();
        let storage = storage_custom_kw.clone();
        spawn_async(async move {
            let _ = storage.config().update(Box::new(move |c| {
                c.terminal_custom_keywords = kw_owned;
            })).await;
        });
    });

    let notif_cursor = ctx.notifications.clone();
    let storage_cursor = ctx.core_state.storage();
    let window_weak_cursor = window.as_weak();
    let renderer_cursor = Rc::clone(&ctx.terminal_renderer);
    let active_terminals_cursor = Rc::clone(&ctx.active_terminals);
    bridge.on_change_cursor_style(move |style| {
        let style_label = match style.as_str() {
            "beam" => "竖线 (|)",
            "underline" => "下划线 (_)",
            _ => "方块 (█)",
        };
        notif_cursor.info("光标形态已更新", &format!("终端光标已切换为「{}」", style_label));
        if let Some(w) = window_weak_cursor.upgrade() {
            w.global::<SettingsBridge>().set_setting_cursor_style(style.as_str().into());
        }
        let s_owned = style.to_string();
        if let Some(ref mut r) = *renderer_cursor.borrow_mut() {
            r.set_cursor_style(&s_owned);
        }
        for instance in active_terminals_cursor.borrow_mut().values_mut() {
            instance.parser.mark_dirty();
        }
        let storage = storage_cursor.clone();
        spawn_async(async move {
            let _ = storage.config().update(Box::new(move |c| {
                c.cursor_style = s_owned;
            })).await;
        });
    });

    let notif_blink = ctx.notifications.clone();
    let storage_blink = ctx.core_state.storage();
    let window_weak_blink = window.as_weak();
    let renderer_blink = Rc::clone(&ctx.terminal_renderer);
    let active_terminals_blink = Rc::clone(&ctx.active_terminals);
    bridge.on_change_cursor_blink(move |blink| {
        if blink {
            notif_blink.info("光标闪烁已开启", "终端光标已启用周期呼吸闪烁");
        } else {
            notif_blink.info("光标闪烁已关闭", "终端光标已切换为静态长亮形态");
        }
        if let Some(w) = window_weak_blink.upgrade() {
            w.global::<SettingsBridge>().set_setting_cursor_blink(blink);
        }
        if let Some(ref mut r) = *renderer_blink.borrow_mut() {
            r.set_cursor_blink(blink);
        }
        for instance in active_terminals_blink.borrow_mut().values_mut() {
            instance.parser.mark_dirty();
        }
        let storage = storage_blink.clone();
        spawn_async(async move {
            let _ = storage.config().update(Box::new(move |c| {
                c.cursor_blink = blink;
            })).await;
        });
    });

    let notif_scroll = ctx.notifications.clone();
    let storage_scroll = ctx.core_state.storage();
    let window_weak_scroll = window.as_weak();
    bridge.on_change_scrollback_lines(move |lines| {
        notif_scroll.info("回滚缓冲已调整", &format!("终端最大回滚行数已调整为 {} 行", lines));
        if let Some(w) = window_weak_scroll.upgrade() {
            w.global::<SettingsBridge>().set_setting_scrollback_lines(lines);
            w.global::<SettingsBridge>().set_scrollback_input(format!("{}", lines).into());
        }
        let storage = storage_scroll.clone();
        spawn_async(async move {
            let _ = storage.config().update(Box::new(move |c| {
                c.scrollback_lines = if lines == 0 { 0 } else { lines.max(100) as usize };
            })).await;
        });
    });

    let notif_sb_in = ctx.notifications.clone();
    let storage_sb_in = ctx.core_state.storage();
    let window_weak_sb_in = window.as_weak();
    bridge.on_apply_scrollback_input(move |input_str| {
        let trimmed = input_str.trim();
        if let Ok(parsed) = trimmed.parse::<i32>() {
            let clamped = parsed.max(0);
            if let Some(w) = window_weak_sb_in.upgrade() {
                w.global::<SettingsBridge>().set_setting_scrollback_lines(clamped);
                w.global::<SettingsBridge>().set_scrollback_input(format!("{}", clamped).into());
            }
            let storage = storage_sb_in.clone();
            spawn_async(async move {
                let _ = storage.config().update(Box::new(move |c| {
                    c.scrollback_lines = if clamped == 0 { 0 } else { clamped.max(100) as usize };
                })).await;
            });
            notif_sb_in.info("回滚缓冲已调整", &format!("终端最大回滚行数已更新为 {} 行", clamped));
        }
    });

    let notif_bell = ctx.notifications.clone();
    let storage_bell = ctx.core_state.storage();
    let window_weak_bell = window.as_weak();
    bridge.on_change_bell_style(move |style| {
        let mode_label = match style.as_str() {
            "audible" => "系统蜂鸣声音",
            "none" => "完全静音",
            _ => "屏幕视觉闪烁",
        };
        notif_bell.info("蜂鸣模式已设置", &format!("终端蜂鸣告警已设置为「{}」", mode_label));
        if let Some(w) = window_weak_bell.upgrade() {
            w.global::<SettingsBridge>().set_setting_bell_style(style.as_str().into());
        }
        let b_owned = style.to_string();
        let storage = storage_bell.clone();
        spawn_async(async move {
            let _ = storage.config().update(Box::new(move |c| {
                c.terminal_bell_style = b_owned;
            })).await;
        });
    });

    let notif_cos = ctx.notifications.clone();
    let storage_cos = ctx.core_state.storage();
    let window_weak_cos = window.as_weak();
    bridge.on_change_copy_on_select(move |enabled| {
        if enabled {
            notif_cos.info("划选自动复制已开启", "在终端中划选文字时将立即自动写入系统剪贴板");
        } else {
            notif_cos.info("划选自动复制已关闭", "划选文字后需手动按 Ctrl+Shift+C 复制");
        }
        if let Some(w) = window_weak_cos.upgrade() {
            w.global::<SettingsBridge>().set_setting_copy_on_select(enabled);
        }
        let storage = storage_cos.clone();
        spawn_async(async move {
            let _ = storage.config().update(Box::new(move |c| {
                c.copy_on_select = enabled;
            })).await;
        });
    });

    let notif_porc = ctx.notifications.clone();
    let storage_porc = ctx.core_state.storage();
    let window_weak_porc = window.as_weak();
    bridge.on_change_paste_on_right_click(move |enabled| {
        if enabled {
            notif_porc.info("右键快速粘贴已开启", "在终端视口中右键单击将直接粘贴系统剪贴板内容");
        } else {
            notif_porc.info("右键快捷菜单已恢复", "在终端视口中右键单击将呼出操作上下文菜单");
        }
        if let Some(w) = window_weak_porc.upgrade() {
            w.global::<SettingsBridge>().set_setting_paste_on_right_click(enabled);
        }
        let storage = storage_porc.clone();
        spawn_async(async move {
            let _ = storage.config().update(Box::new(move |c| {
                c.paste_on_right_click = enabled;
            })).await;
        });
    });

    let notif_wmp = ctx.notifications.clone();
    let storage_wmp = ctx.core_state.storage();
    let window_weak_wmp = window.as_weak();
    bridge.on_change_warn_multiline_paste(move |enabled| {
        if enabled {
            notif_wmp.info("多行粘贴告警已开启", "粘贴包含换行符的多行指令时将前置安全告警");
        } else {
            notif_wmp.info("多行粘贴告警已关闭", "粘贴多行命令时将直接执行无需告警");
        }
        if let Some(w) = window_weak_wmp.upgrade() {
            w.global::<SettingsBridge>().set_setting_warn_multiline_paste(enabled);
        }
        let storage = storage_wmp.clone();
        spawn_async(async move {
            let _ = storage.config().update(Box::new(move |c| {
                c.warn_on_multiline_paste = enabled;
            })).await;
        });
    });

    let notif_ca = ctx.notifications.clone();
    let storage_ca = ctx.core_state.storage();
    let window_weak_ca = window.as_weak();
    bridge.on_change_close_action(move |action| {
        let is_tray = action.as_str() == "tray";
        if is_tray {
            notif_ca.info("窗口关闭行为", "关闭主窗口时将最小化到系统托盘，保持会话持续在线");
        } else {
            notif_ca.info("窗口关闭行为", "关闭主窗口时将完全退出应用程序");
        }
        if let Some(w) = window_weak_ca.upgrade() {
            w.global::<SettingsBridge>().set_setting_close_action(action.as_str().into());
        }
        let a_str = action.to_string();
        let storage = storage_ca.clone();
        spawn_async(async move {
            let _ = storage.config().update(Box::new(move |c| {
                c.close_action = a_str;
            })).await;
        });
    });

    let notif_proxy = ctx.notifications.clone();
    let storage_proxy = ctx.core_state.storage();
    let window_weak_proxy = window.as_weak();
    bridge.on_change_global_proxy(move |mode, server, auth, user, pass| {
        let m_str = mode.to_string();
        let s_str = server.to_string();
        let u_str = user.to_string();
        let p_str = pass.to_string();

        let label = match m_str.as_str() {
            "direct" => "直连 (禁用代理)",
            "system" => "跟随系统代理",
            "custom" => "自定义代理",
            _ => "直连",
        };
        notif_proxy.success("出站代理已同步", &format!("当前代理策略已设置为「{}」", label));

        if let Some(w) = window_weak_proxy.upgrade() {
            w.global::<SettingsBridge>().set_setting_global_proxy_mode(m_str.as_str().into());
            w.global::<SettingsBridge>().set_setting_global_proxy_server(s_str.as_str().into());
            w.global::<SettingsBridge>().set_setting_global_proxy_auth(auth);
            w.global::<SettingsBridge>().set_setting_global_proxy_user(u_str.as_str().into());
            w.global::<SettingsBridge>().set_setting_global_proxy_pass(p_str.as_str().into());
        }

        let m_save = m_str.clone();
        let s_save = s_str.clone();
        let u_save = u_str.clone();
        let p_save = p_str.clone();
        let storage = storage_proxy.clone();
        spawn_async(async move {
            let _ = storage.config().update(Box::new(move |c| {
                c.global_proxy_mode = m_save;
                c.global_proxy_server = s_save;
                c.global_proxy_auth = auth;
                c.global_proxy_user = u_save;
                c.global_proxy_pass = p_save;
            })).await;
        });
    });

    let notif_handshake = ctx.notifications.clone();
    let storage_handshake = ctx.core_state.storage();
    let window_weak_handshake = window.as_weak();
    bridge.on_change_network_handshake(move |timeout, interval, count_max| {
        let t_val = timeout.clamp(5, 300) as u32;
        let i_val = interval.clamp(0, 300) as u32;
        let c_val = count_max.clamp(1, 20) as u32;

        if let Some(w) = window_weak_handshake.upgrade() {
            w.global::<SettingsBridge>().set_setting_connect_timeout(t_val as i32);
            w.global::<SettingsBridge>().set_setting_keepalive_interval(i_val as i32);
            w.global::<SettingsBridge>().set_setting_keepalive_count_max(c_val as i32);
        }

        let storage = storage_handshake.clone();
        spawn_async(async move {
            let _ = storage.config().update(Box::new(move |c| {
                c.ssh_timeout_seconds = t_val;
                c.keepalive_interval = i_val;
                c.keepalive_count_max = c_val;
            })).await;
        });

        let keepalive_desc = if i_val == 0 {
            "保活心跳已禁用".to_string()
        } else {
            format!("心跳 {}s / 重试 {} 次", i_val, c_val)
        };
        notif_handshake.success(
            "网络握手参数已更新",
            &format!("连接超时 {} 秒，{}", t_val, keepalive_desc),
        );
        tracing::info!(
            target: "smagical_ui::settings",
            "SSH握手参数变更: 超时={}s, 保活={}s, 最大重试={}",
            t_val, i_val, c_val
        );
    });

    let notif_hk = ctx.notifications.clone();
    let storage_hk = ctx.core_state.storage();
    let window_weak_hk = window.as_weak();
    bridge.on_change_host_key_policy(move |policy| {
        let p_str = policy.to_string();
        if let Some(w) = window_weak_hk.upgrade() {
            w.global::<SettingsBridge>().set_setting_host_key_policy(p_str.clone().into());
        }
        let p_save = p_str.clone();
        let storage = storage_hk.clone();
        spawn_async(async move {
            let _ = storage.config().update(Box::new(move |c| {
                c.host_key_checking = p_save;
            })).await;
        });
        notif_hk.info("主机公钥指纹策略已变更", &format!("已切换为「{}」", p_str));
    });

    let notif_tcp = ctx.notifications.clone();
    let storage_tcp = ctx.core_state.storage();
    let window_weak_tcp = window.as_weak();
    bridge.on_change_tcp_nodelay(move |enabled| {
        if let Some(w) = window_weak_tcp.upgrade() {
            w.global::<SettingsBridge>().set_setting_tcp_nodelay(enabled);
        }
        let storage = storage_tcp.clone();
        spawn_async(async move {
            let _ = storage.config().update(Box::new(move |c| {
                c.tcp_nodelay = enabled;
            })).await;
        });
        notif_tcp.info(
            if enabled { "TCP_NODELAY 已启用" } else { "TCP_NODELAY 已禁用" },
            if enabled { "已开启 Nagle 算法规避，最小化终端小包交互延迟" } else { "已恢复标准 TCP 缓冲合并" },
        );
    });

    let notif_comp = ctx.notifications.clone();
    let storage_comp = ctx.core_state.storage();
    let window_weak_comp = window.as_weak();
    bridge.on_change_compression(move |enabled| {
        if let Some(w) = window_weak_comp.upgrade() {
            w.global::<SettingsBridge>().set_setting_compression(enabled);
        }
        let storage = storage_comp.clone();
        spawn_async(async move {
            let _ = storage.config().update(Box::new(move |c| {
                c.compression = enabled;
            })).await;
        });
        notif_comp.info(
            if enabled { "数据流压缩已启用" } else { "数据流压缩已禁用" },
            if enabled { "已开启 gzip 链路压缩，提升跨国与低带宽网络速度" } else { "已关闭传输数据流压缩" },
        );
    });

    let notif_leg = ctx.notifications.clone();
    let storage_leg = ctx.core_state.storage();
    let window_weak_leg = window.as_weak();
    bridge.on_change_legacy_ciphers(move |enabled| {
        if let Some(w) = window_weak_leg.upgrade() {
            w.global::<SettingsBridge>().set_setting_legacy_ciphers(enabled);
        }
        let storage = storage_leg.clone();
        spawn_async(async move {
            let _ = storage.config().update(Box::new(move |c| {
                c.legacy_ciphers = enabled;
            })).await;
        });
        notif_leg.info(
            if enabled { "老旧算法兼容已开启" } else { "老旧算法兼容已关闭" },
            if enabled { "允许协商 3DES、DH Group 1 等已弃用算法" } else { "已恢复安全默认算法协商" },
        );
    });

    let notif_reconn = ctx.notifications.clone();
    let storage_reconn = ctx.core_state.storage();
    let window_weak_reconn = window.as_weak();
    bridge.on_change_auto_reconnect(move |enabled| {
        if let Some(w) = window_weak_reconn.upgrade() {
            w.global::<SettingsBridge>().set_setting_auto_reconnect(enabled);
        }
        let storage = storage_reconn.clone();
        spawn_async(async move {
            let _ = storage.config().update(Box::new(move |c| {
                c.auto_reconnect = enabled;
            })).await;
        });
        notif_reconn.info(
            if enabled { "自动断线重连已开启" } else { "自动断线重连已关闭" },
            if enabled { "网络闪断后将使用指数退避算法静默尝试重连" } else { "断线后保持断开状态" },
        );
    });

    let w_b_tab = window.as_weak();
    let storage_tab = ctx.core_state.storage();
    bridge.on_toggle_confirm_close_tab(move |val| {
        let storage = storage_tab.clone();
        spawn_async(async move {
            let _ = storage.config().update(Box::new(move |c| {
                c.confirm_close_tab = val;
            })).await;
        });
        if let Some(w) = w_b_tab.upgrade() {
            w.global::<SettingsBridge>().set_setting_confirm_close_tab(val);
        }
        tracing::info!(target: "smagical_ui::settings", "关闭标签页时防误触确认设置为: {}", val);
    });

    let w_b_toast = window.as_weak();
    let storage_toast = ctx.core_state.storage();
    let notif_toast = ctx.notifications.clone();
    bridge.on_change_toast_duration(move |val| {
        let val_str = val.to_string();
        notif_toast.set_duration_preset(&val_str);
        let val_clone = val_str.clone();
        let storage = storage_toast.clone();
        spawn_async(async move {
            let _ = storage.config().update(Box::new(move |c| {
                c.toast_duration = val_clone;
            })).await;
        });
        if let Some(w) = w_b_toast.upgrade() {
            w.global::<SettingsBridge>().set_setting_toast_duration(val_str.as_str().into());
        }
        tracing::info!(target: "smagical_ui::settings", "提示信息存在时间设置为: {}", val_str);
    });

    // 5. SFTP 传输与文件管理设置回调 (SFTP Settings Handlers)
    let notif_sftp = ctx.notifications.clone();
    let storage_sftp = ctx.core_state.storage();
    let window_weak_sftp = window.as_weak();
    bridge.on_browse_sftp_download_dir(move || {
        if let Some(folder_path) = pick_folder() {
            let path_str = folder_path.to_string_lossy().to_string();
            if let Some(w) = window_weak_sftp.upgrade() {
                w.global::<SettingsBridge>().set_setting_sftp_default_local(path_str.as_str().into());
            }
            let path_clone = path_str.clone();
            let storage = storage_sftp.clone();
            spawn_async(async move {
                let _ = storage.config().update(Box::new(move |c| {
                    c.sftp_default_local = path_clone;
                })).await;
            });
            notif_sftp.success("默认下载目录已更新", &format!("SFTP 下载路径已设置为: {}", path_str));
        }
    });

    let notif_dl_change = ctx.notifications.clone();
    let storage_dl_change = ctx.core_state.storage();
    bridge.on_change_sftp_download_dir(move |path| {
        let p_str = path.to_string();
        let p_clone = p_str.clone();
        let storage = storage_dl_change.clone();
        spawn_async(async move {
            let _ = storage.config().update(Box::new(move |c| {
                c.sftp_default_local = p_clone;
            })).await;
        });
        notif_dl_change.info("默认下载目录已更新", &format!("SFTP 下载路径已设置为: {}", p_str));
    });

    let notif_concur = ctx.notifications.clone();
    let storage_concur = ctx.core_state.storage();
    let window_weak_concur = window.as_weak();
    bridge.on_change_sftp_concurrency(move |concurrency| {
        let val = concurrency.clamp(1, 16) as u32;
        if let Some(w) = window_weak_concur.upgrade() {
            w.global::<SettingsBridge>().set_setting_sftp_concurrency(val as i32);
        }
        let storage = storage_concur.clone();
        spawn_async(async move {
            let _ = storage.config().update(Box::new(move |c| {
                c.sftp_concurrency = val;
            })).await;
        });
        notif_concur.info("SFTP 并发数已更新", &format!("最大并发连接数已设置为 {} 个", val));
    });

    let notif_up = ctx.notifications.clone();
    let storage_up = ctx.core_state.storage();
    let window_weak_up = window.as_weak();
    bridge.on_set_upload_limit_speed(move |num_str, unit_str| {
        let code = if unit_str == "off" || num_str == "0" {
            "unlimited".to_string()
        } else {
            format!("{}{}", num_str, if unit_str == "KB/s" { "kb" } else { "mb" })
        };
        if let Some(w) = window_weak_up.upgrade() {
            w.global::<SettingsBridge>().set_setting_sftp_upload_limit(code.as_str().into());
            w.global::<SettingsBridge>().set_upload_limit_num(num_str.clone());
            w.global::<SettingsBridge>().set_upload_limit_unit(unit_str.clone());
        }
        let code_clone = code.clone();
        let storage = storage_up.clone();
        spawn_async(async move {
            let _ = storage.config().update(Box::new(move |c| {
                c.sftp_upload_limit = code_clone;
            })).await;
        });
        notif_up.info("上传限速已调整", &format!("单任务上传速率限制已设定为: {}", if code == "unlimited" { "不限速" } else { &code }));
    });

    let notif_dl = ctx.notifications.clone();
    let storage_dl = ctx.core_state.storage();
    let window_weak_dl = window.as_weak();
    bridge.on_set_download_limit_speed(move |num_str, unit_str| {
        let code = if unit_str == "off" || num_str == "0" {
            "unlimited".to_string()
        } else {
            format!("{}{}", num_str, if unit_str == "KB/s" { "kb" } else { "mb" })
        };
        if let Some(w) = window_weak_dl.upgrade() {
            w.global::<SettingsBridge>().set_setting_sftp_download_limit(code.as_str().into());
            w.global::<SettingsBridge>().set_download_limit_num(num_str.clone());
            w.global::<SettingsBridge>().set_download_limit_unit(unit_str.clone());
        }
        let code_clone = code.clone();
        let storage = storage_dl.clone();
        spawn_async(async move {
            let _ = storage.config().update(Box::new(move |c| {
                c.sftp_download_limit = code_clone;
            })).await;
        });
        notif_dl.info("下载限速已调整", &format!("单任务下载速率限制已设定为: {}", if code == "unlimited" { "不限速" } else { &code }));
    });

    let notif_sh = ctx.notifications.clone();
    let storage_sh = ctx.core_state.storage();
    let window_weak_sh = window.as_weak();
    let ctx_sh = ctx.clone();
    bridge.on_change_sftp_show_hidden(move |enabled| {
        if let Some(w) = window_weak_sh.upgrade() {
            w.global::<SettingsBridge>().set_setting_sftp_show_hidden(enabled);
            crate::handlers::file_handlers::sync_file_explorer_ui(&w, &ctx_sh);
        }
        let storage = storage_sh.clone();
        spawn_async(async move {
            let _ = storage.config().update(Box::new(move |c| {
                c.sftp_show_hidden = enabled;
            })).await;
        });
        notif_sh.info(
            if enabled { "已开启显示隐藏文件" } else { "已隐藏以点开头的隐藏文件" },
            if enabled { "SFTP 文件浏览器中将展示以点开头的隐藏文件 (如 .bashrc, .env)" } else { "文件树仅显示常规文件" },
        );
    });

    let notif_cp = ctx.notifications.clone();
    let storage_cp = ctx.core_state.storage();
    let window_weak_cp = window.as_weak();
    bridge.on_change_sftp_conflict_policy(move |policy| {
        let p_str = policy.to_string();
        if let Some(w) = window_weak_cp.upgrade() {
            w.global::<SettingsBridge>().set_setting_sftp_conflict_policy(p_str.clone().into());
        }
        let p_save = p_str.clone();
        let storage = storage_cp.clone();
        spawn_async(async move {
            let _ = storage.config().update(Box::new(move |c| {
                c.sftp_conflict_policy = p_save;
            })).await;
        });
        let label = match p_str.as_str() {
            "overwrite" => "覆盖同名文件",
            "skip" => "跳过已存在文件",
            "rename" => "自动重命名后缀",
            _ => "每次弹窗询问",
        };
        notif_cp.info("传输冲突策略已更新", &format!("同名冲突处理策略已设定为「{}」", label));
    });

    // 6. 日志级别与开发者控制台调试联动
    let window_weak_lvl = window.as_weak();
    let notif_lvl = ctx.notifications.clone();
    let storage_lvl = ctx.core_state.storage();
    bridge.on_set_log_level(move |level_str| {
        let upper = level_str.to_uppercase();
        crate::debug::tracing_layer::set_global_runtime_log_level(&upper);
        if let Some(w) = window_weak_lvl.upgrade() {
            w.global::<SettingsBridge>().set_setting_log_level(upper.clone().into());
        }
        let upper_clone = upper.clone();
        let storage = storage_lvl.clone();
        spawn_async(async move {
            let _ = storage.config().update(Box::new(move |c| {
                c.log_level = upper_clone;
            })).await;
        });
        notif_lvl.info("日志等级已调整", &format!("全局运行时日志等级已切换为「{}」", upper));
    });

    let core_state_dbg = ctx.core_state.clone();
    let storage_dbg = ctx.core_state.storage();
    let window_weak_dbg = window.as_weak();
    bridge.on_toggle_debug_enabled(move |enabled| {
        crate::debug::set_debug_enabled(enabled);
        core_state_dbg.activity_bar().set_visible("debug", enabled);
        if let Some(w) = window_weak_dbg.upgrade() {
            w.global::<WindowBridge>().set_is_debug_enabled(enabled);
            w.global::<SettingsBridge>().set_setting_debug_enabled(enabled);
            crate::activity_bar_service::sync_activity_bar_ui(&w, &core_state_dbg);
            if !enabled {
                w.global::<crate::generated::DebugBridge>().set_is_open(false);
                if w.global::<WindowBridge>().get_active_left_tab() == "debug" {
                    w.global::<WindowBridge>().set_active_left_tab("hosts".into());
                }
            } else {
                crate::debug_ui::sync_ui_debug_logs(&w);
            }
            let storage = storage_dbg.clone();
            spawn_async(async move {
                let _ = storage.config().update(Box::new(move |c| {
                    c.debug_enabled = enabled;
                })).await;
            });
            tracing::info!(target: "smagical_ui::settings", "开发者调试控制台已{}", if enabled { "开启" } else { "关闭" });
        }
    });

    bridge.set_setting_debug_enabled(window.global::<WindowBridge>().get_is_debug_enabled());

    // 8. 终端字体与字号设置变更 (Terminal Font & Size)
    let window_weak_tfont = window.as_weak();
    let storage_tfont = ctx.core_state.storage();
    let renderer_tfont = Rc::clone(&ctx.terminal_renderer);
    let active_terminals_tfont = Rc::clone(&ctx.active_terminals);
    let notif_tfont = ctx.notifications.clone();
    bridge.on_change_terminal_font(move |font_name, font_size| {
        let f = font_name.as_str();
        let size = if font_size <= 0 { 13 } else { font_size };
        let size_f = size as f32;
        if let Some(w) = window_weak_tfont.upgrade() {
            w.global::<SettingsBridge>().set_setting_terminal_font(f.into());
            w.global::<SettingsBridge>().set_setting_terminal_font_size(size);
            w.global::<WindowBridge>().set_terminal_font_family(f.into());
            w.global::<WindowBridge>().set_terminal_font_size(size_f);
        }
        let f_owned = f.to_string();
        let storage = storage_tfont.clone();
        spawn_async(async move {
            let _ = storage.config().update(Box::new(move |c| {
                c.font_family = f_owned;
                c.font_size = size_f;
            })).await;
        });
        if let Some(ref mut r) = *renderer_tfont.borrow_mut() {
            if let Some(bytes) = crate::terminal::renderer::find_font_by_name(f) {
                let _ = r.update_font(&bytes, size_f);
            } else {
                let _ = r.update_font_size(size_f);
            }
        }
        for instance in active_terminals_tfont.borrow_mut().values_mut() {
            instance.parser.mark_dirty();
        }
        notif_tfont.info("终端字体已更新", &format!("当前字体: {}, 字号: {}px", f, size));
    });

    // 9. 终端 CRT 显像管特效开关 (Terminal CRT Shader Effect)
    let window_weak_crt = window.as_weak();
    let storage_crt = ctx.core_state.storage();
    bridge.on_toggle_terminal_crt(move |enabled| {
        if let Some(w) = window_weak_crt.upgrade() {
            w.global::<SettingsBridge>().set_flag_terminal_crt_shader(enabled);
            w.global::<SettingsBridge>().set_setting_terminal_crt(enabled);
        }
        let storage = storage_crt.clone();
        spawn_async(async move {
            let _ = storage.config().update(Box::new(move |c| {
                c.flag_terminal_crt_shader = enabled;
            })).await;
        });
    });

    // 10. SFTP 传输首选项完整闭环 Handlers
    let storage_sftp_r = ctx.core_state.storage();
    bridge.on_change_sftp_default_remote(move |path| {
        let p_str = path.to_string();
        let storage = storage_sftp_r.clone();
        spawn_async(async move {
            let _ = storage.config().update(Box::new(move |c| {
                c.sftp_default_remote = p_str;
            })).await;
        });
    });

    let storage_sftp_del = ctx.core_state.storage();
    bridge.on_change_sftp_confirm_delete(move |val| {
        let storage = storage_sftp_del.clone();
        spawn_async(async move {
            let _ = storage.config().update(Box::new(move |c| {
                c.sftp_confirm_delete = val;
            })).await;
        });
    });

    let storage_sftp_res = ctx.core_state.storage();
    bridge.on_change_sftp_resume(move |val| {
        let storage = storage_sftp_res.clone();
        spawn_async(async move {
            let _ = storage.config().update(Box::new(move |c| {
                c.sftp_resume_transfer = val;
            })).await;
        });
    });

    let storage_sftp_attr = ctx.core_state.storage();
    bridge.on_change_sftp_preserve_attributes(move |val| {
        let storage = storage_sftp_attr.clone();
        spawn_async(async move {
            let _ = storage.config().update(Box::new(move |c| {
                c.sftp_preserve_attributes = val;
            })).await;
        });
    });

    let storage_sftp_ed = ctx.core_state.storage();
    bridge.on_change_sftp_editor_mode(move |mode| {
        let m_str = mode.to_string();
        let storage = storage_sftp_ed.clone();
        spawn_async(async move {
            let _ = storage.config().update(Box::new(move |c| {
                c.sftp_editor_mode = m_str;
            })).await;
        });
    });

    let storage_sftp_ced = ctx.core_state.storage();
    bridge.on_change_sftp_custom_editor(move |cmd| {
        let c_str = cmd.to_string();
        let storage = storage_sftp_ced.clone();
        spawn_async(async move {
            let _ = storage.config().update(Box::new(move |c| {
                c.sftp_custom_editor = c_str;
            })).await;
        });
    });

    let storage_sftp_exc = ctx.core_state.storage();
    let window_weak_exc = window.as_weak();
    let ctx_exc = ctx.clone();
    bridge.on_change_sftp_exclude_patterns(move |pat| {
        let p_str = pat.to_string();
        if let Some(w) = window_weak_exc.upgrade() {
            w.global::<SettingsBridge>().set_setting_sftp_exclude_patterns(p_str.as_str().into());
            crate::handlers::file_handlers::sync_file_explorer_ui(&w, &ctx_exc);
        }
        let storage = storage_sftp_exc.clone();
        spawn_async(async move {
            let _ = storage.config().update(Box::new(move |c| {
                c.sftp_exclude_patterns = p_str;
            })).await;
        });
    });

    // 11. 安全与锁屏策略 Handlers
    let storage_sec_lk = ctx.core_state.storage();
    let notif_sec_lk = ctx.notifications.clone();
    bridge.on_change_auto_lock_timeout(move |timeout| {
        let t_str = timeout.to_string();
        let t_clone = t_str.clone();
        let storage = storage_sec_lk.clone();
        spawn_async(async move {
            let _ = storage.config().update(Box::new(move |c| {
                c.auto_lock_timeout = t_clone;
            })).await;
        });
        notif_sec_lk.info("自动锁定策略已更新", &format!("空闲超时设置为: {}", t_str));
    });

    let storage_sec_min = ctx.core_state.storage();
    bridge.on_change_lock_on_minimize(move |val| {
        let storage = storage_sec_min.clone();
        spawn_async(async move {
            let _ = storage.config().update(Box::new(move |c| {
                c.lock_on_minimize = val;
            })).await;
        });
    });

    let storage_sec_bio = ctx.core_state.storage();
    bridge.on_change_biometric_unlock(move |val| {
        let storage = storage_sec_bio.clone();
        spawn_async(async move {
            let _ = storage.config().update(Box::new(move |c| {
                c.biometric_unlock = val;
            })).await;
        });
    });

    let storage_sec_clip = ctx.core_state.storage();
    bridge.on_change_clear_clipboard_timeout(move |val| {
        let storage = storage_sec_clip.clone();
        spawn_async(async move {
            let _ = storage.config().update(Box::new(move |c| {
                c.clear_clipboard_timeout = val;
            })).await;
        });
    });

    let storage_sec_cmd = ctx.core_state.storage();
    bridge.on_change_confirm_dangerous_commands(move |val| {
        let storage = storage_sec_cmd.clone();
        spawn_async(async move {
            let _ = storage.config().update(Box::new(move |c| {
                c.confirm_dangerous_commands = val;
            })).await;
        });
    });

    let storage_sec_aud = ctx.core_state.storage();
    let notif_sec_aud = ctx.notifications.clone();
    bridge.on_change_session_audit_logging(move |val| {
        crate::audit_logger::set_audit_enabled(val);
        let storage = storage_sec_aud.clone();
        spawn_async(async move {
            let _ = storage.config().update(Box::new(move |c| {
                c.session_audit_logging = val;
            })).await;
        });
        notif_sec_aud.info(
            if val { "会话安全审计已开启" } else { "会话安全审计已关闭" },
            if val { "所有终端命令与操作将结构化记录至本地安全审计日志" } else { "已停止记录会话操作审计" },
        );
    });

    // 12. 关闭偏好设置中心
    let w_b = window.as_weak();
    bridge.on_close_settings(move || {
        if let Some(w) = w_b.upgrade() {
            let wb = w.global::<WindowBridge>();
            wb.set_main_view("terminal".into());
            wb.set_active_left_tab("hosts".into());
            wb.set_is_left_drawer_open(true);
            w.global::<SettingsBridge>().set_active_category("general".into());
        }
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_slint_select_bundled_translation() {
        let window = crate::generated::AppWindow::new();
        assert!(window.is_ok(), "AppWindow::new 应成功");

        let res_en = slint::select_bundled_translation("en");
        assert!(res_en.is_ok(), "select_bundled_translation('en') 应该成功: {:?}", res_en);

        let res_zh = slint::select_bundled_translation("");
        assert!(res_zh.is_ok(), "select_bundled_translation('') 应该成功: {:?}", res_zh);
    }
}
