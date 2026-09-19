//! AI 智能助手与大模型多端点管理事件处理器

use std::rc::Rc;
use slint::{ComponentHandle, Model, ModelRc, VecModel};

use crate::generated::{
    AiBridge, AiEndpointProfile, AppWindow, SettingsBridge, ToastItemData, WindowBridge,
};
use crate::handlers::AppContext;

pub(crate) fn register_ai_handlers(window: &AppWindow, ctx: &AppContext) {
    let bridge = window.global::<SettingsBridge>();

    // -------------------------------------------------------------------------
    // AI 助手与大模型基础设置回调
    // -------------------------------------------------------------------------
    let window_weak_ai_save = window.as_weak();
    let notif_ai_save = ctx.notifications.clone();
    bridge.on_save_ai_settings(move || {
        if let Some(w) = window_weak_ai_save.upgrade() {
            let sb = w.global::<SettingsBridge>();
            if let Ok(ctx_val) = sb.get_setting_ai_max_context_input().trim().parse::<i32>() {
                sb.set_setting_ai_max_context(ctx_val);
            }
            if let Ok(to_val) = sb.get_setting_ai_timeout_secs_input().trim().parse::<i32>() {
                sb.set_setting_ai_timeout_secs(to_val);
            }
            if let Ok(ret_val) = sb.get_setting_ai_max_retries_input().trim().parse::<i32>() {
                sb.set_setting_ai_max_retries(ret_val);
            }
            if let Ok(temp_val) = sb.get_setting_ai_temperature_input().trim().parse::<f32>() {
                sb.set_setting_ai_temperature(temp_val);
            }
            notif_ai_save.success("保存成功", "AI 大模型配置与操作审查策略已更新保存");
        }
    });

    let window_weak_ai_reset = window.as_weak();
    bridge.on_reset_ai_system_prompt(move || {
        if let Some(w) = window_weak_ai_reset.upgrade() {
            let default_prompt = "你是一名精通 Linux/Unix 操作系统内核、网络拓扑与现代运维架构的高级 SRE 运维专家。遵循生产安全第一原则，始终输出语法严谨、带有防御性容错参数的 Shell 指令，主动识别与规避高危操作风险，并在生成复杂命令时简明解释其参数逻辑。";
            w.global::<SettingsBridge>().set_setting_ai_system_prompt(default_prompt.into());
        }
    });

    let window_weak_ai_provider = window.as_weak();
    bridge.on_switch_ai_provider(move |provider| {
        if let Some(w) = window_weak_ai_provider.upgrade() {
            let sb = w.global::<SettingsBridge>();
            match provider.as_str() {
                "deepseek" => {
                    sb.set_setting_ai_base_url("https://api.deepseek.com/v1".into());
                    sb.set_setting_ai_model("deepseek-reasoner".into());
                }
                "claude" => {
                    sb.set_setting_ai_base_url("https://api.anthropic.com/v1".into());
                    sb.set_setting_ai_model("claude-3-7-sonnet-20250219".into());
                }
                "openai" => {
                    sb.set_setting_ai_base_url("https://api.openai.com/v1".into());
                    sb.set_setting_ai_model("gpt-4o".into());
                }
                "ollama" => {
                    sb.set_setting_ai_base_url("http://localhost:11434/v1".into());
                    sb.set_setting_ai_model("qwen2.5:14b".into());
                }
                _ => {}
            }
        }
    });

    // -------------------------------------------------------------------------
    // AI 多端点端口管理与模型联动切换
    // -------------------------------------------------------------------------

    // 1. 一键切换 AI 端点 (switch_ai_endpoint)
    let window_weak_switch_ep = window.as_weak();
    let notif_switch_ep = ctx.notifications.clone();
    bridge.on_switch_ai_endpoint(move |target_id| {
        if let Some(w) = window_weak_switch_ep.upgrade() {
            let sb = w.global::<SettingsBridge>();
            let endpoints_model = sb.get_setting_ai_endpoints();
            let mut list: Vec<AiEndpointProfile> = endpoints_model.iter().collect();

            let mut switched_name = String::new();
            let mut new_base_url = String::new();
            let mut new_api_key = String::new();
            let mut new_selected_model = String::new();
            let mut new_models_vec: Vec<slint::SharedString> = Vec::new();
            let mut new_thinking_degree = String::new();
            let mut new_timeout = 60;
            let mut new_context = 32768;
            let mut new_retries = 2;
            let mut new_headers = String::new();
            let mut new_temp = "0.3".to_string();

            for ep in list.iter_mut() {
                if ep.id == target_id {
                    ep.is_active = true;
                    ep.status_text = "已连接".into();
                    switched_name = ep.name.to_string();
                    new_base_url = ep.base_url.to_string();
                    new_api_key = ep.api_key.to_string();
                    new_selected_model = ep.selected_model.to_string();
                    new_thinking_degree = ep.thinking_degree.to_string();
                    new_timeout = if ep.timeout_secs <= 0 { 60 } else { ep.timeout_secs };
                    new_context = if ep.max_context <= 0 { 32768 } else { ep.max_context };
                    new_retries = if ep.max_retries < 0 { 2 } else { ep.max_retries };
                    new_headers = ep.custom_headers.to_string();
                    new_temp = if ep.temperature.is_empty() { "0.3".to_string() } else { ep.temperature.to_string() };

                    for m in ep.models_csv.split(',') {
                        let trimmed = m.trim();
                        if !trimmed.is_empty() {
                            new_models_vec.push(trimmed.into());
                        }
                    }
                    if new_models_vec.is_empty() && !new_selected_model.is_empty() {
                        new_models_vec.push(new_selected_model.clone().into());
                    }
                } else {
                    ep.is_active = false;
                    ep.status_text = "未激活".into();
                }
            }

            sb.set_setting_ai_endpoints(ModelRc::from(Rc::new(VecModel::from(list))));
            sb.set_active_endpoint_id(target_id.clone());
            sb.set_setting_ai_base_url(new_base_url.into());
            sb.set_setting_ai_api_key(new_api_key.into());
            sb.set_setting_ai_model(new_selected_model.clone().into());
            sb.set_setting_ai_current_models(ModelRc::from(Rc::new(VecModel::from(new_models_vec.clone()))));

            sb.set_setting_ai_thinking_budget(if new_thinking_degree.is_empty() { "medium".into() } else { new_thinking_degree.into() });
            sb.set_setting_ai_timeout_secs(new_timeout);
            sb.set_setting_ai_timeout_secs_input(new_timeout.to_string().into());
            sb.set_setting_ai_max_context(new_context);
            sb.set_setting_ai_max_context_input(new_context.to_string().into());
            sb.set_setting_ai_max_retries(new_retries);
            sb.set_setting_ai_max_retries_input(new_retries.to_string().into());
            sb.set_setting_ai_custom_headers(new_headers.into());
            sb.set_setting_ai_temperature(new_temp.parse::<f32>().unwrap_or(0.3));
            sb.set_setting_ai_temperature_input(new_temp.into());

            let ai_b = w.global::<AiBridge>();
            ai_b.set_available_models(ModelRc::from(Rc::new(VecModel::from(new_models_vec))));
            ai_b.set_selected_model(new_selected_model.into());

            notif_switch_ep.success("端点切换成功", &format!("已无缝切换至: {}", switched_name));
        }
    });

    // 1b. 修改自动审核安全级别联动 (change_ai_auto_audit_level)
    let window_weak_audit_lvl = window.as_weak();
    bridge.on_change_ai_auto_audit_level(move |lvl| {
        if let Some(w) = window_weak_audit_lvl.upgrade() {
            let lvl_str = lvl.to_string();
            w.global::<SettingsBridge>().set_setting_ai_auto_audit_level(lvl.clone());
            crate::handlers::right_drawer_handlers::update_current_host_auto_audit_level(&w, &lvl_str);
            tracing::info!(target: "smagical_ui::settings", "AI 自动审核级别已切换为: {}", lvl_str);
        }
    });

    // 2. 打开添加端点弹窗 (open_add_ai_endpoint_modal)
    let window_weak_add_ep = window.as_weak();
    bridge.on_open_add_ai_endpoint_modal(move || {
        if let Some(w) = window_weak_add_ep.upgrade() {
            let sb = w.global::<SettingsBridge>();
            sb.set_is_ai_endpoint_modal_open(true);
            sb.set_is_ai_endpoint_editing(false);
            sb.set_modal_endpoint_id("".into());
            sb.set_modal_endpoint_name("".into());
            sb.set_modal_endpoint_base_url("https://api.deepseek.com/v1".into());
            sb.set_modal_endpoint_api_key("".into());
            sb.set_modal_endpoint_api_mode("chat".into());
            sb.set_modal_endpoint_selected_model("deepseek-reasoner".into());

            let default_models: Vec<slint::SharedString> = vec![
                "deepseek-reasoner".into(),
                "deepseek-chat".into(),
            ];
            sb.set_modal_endpoint_models_list(ModelRc::from(Rc::new(VecModel::from(default_models))));
            sb.set_modal_endpoint_new_model_input("".into());
            sb.set_modal_endpoint_thinking_degree("medium".into());
            sb.set_modal_endpoint_timeout_secs(60);
            sb.set_modal_endpoint_timeout_secs_input("60".into());
            sb.set_modal_endpoint_max_context(32768);
            sb.set_modal_endpoint_max_context_input("32768".into());
            sb.set_modal_endpoint_max_retries(2);
            sb.set_modal_endpoint_max_retries_input("2".into());
            sb.set_modal_endpoint_custom_headers("".into());
            sb.set_modal_endpoint_temperature("0.3".into());
            sb.set_modal_endpoint_show_advanced(false);
            sb.set_modal_endpoint_is_fetching_models(false);
            sb.set_modal_endpoint_test_status("idle".into());
            sb.set_modal_endpoint_test_message("".into());
        }
    });

    // 3. 打开编辑端点弹窗 (open_edit_ai_endpoint_modal)
    let window_weak_edit_ep = window.as_weak();
    bridge.on_open_edit_ai_endpoint_modal(move |target_id| {
        if let Some(w) = window_weak_edit_ep.upgrade() {
            let sb = w.global::<SettingsBridge>();
            let endpoints_model = sb.get_setting_ai_endpoints();
            if let Some(ep) = endpoints_model.iter().find(|e| e.id == target_id) {
                sb.set_modal_endpoint_id(ep.id.clone());
                sb.set_modal_endpoint_name(ep.name.clone());
                sb.set_modal_endpoint_base_url(ep.base_url.clone());
                sb.set_modal_endpoint_api_key(ep.api_key.clone());
                sb.set_modal_endpoint_api_mode(ep.api_mode.clone());
                sb.set_modal_endpoint_selected_model(ep.selected_model.clone());

                let mut models_vec: Vec<slint::SharedString> = Vec::new();
                for m in ep.models_csv.split(',') {
                    let t = m.trim();
                    if !t.is_empty() { models_vec.push(t.into()); }
                }
                if models_vec.is_empty() && !ep.selected_model.is_empty() {
                    models_vec.push(ep.selected_model.clone());
                }
                sb.set_modal_endpoint_models_list(ModelRc::from(Rc::new(VecModel::from(models_vec))));
                sb.set_modal_endpoint_new_model_input("".into());
                sb.set_modal_endpoint_thinking_degree(if ep.thinking_degree.is_empty() { "medium".into() } else { ep.thinking_degree.clone() });
                let timeout = if ep.timeout_secs <= 0 { 60 } else { ep.timeout_secs };
                sb.set_modal_endpoint_timeout_secs(timeout);
                sb.set_modal_endpoint_timeout_secs_input(timeout.to_string().into());
                let ctx = if ep.max_context <= 0 { 32768 } else { ep.max_context };
                sb.set_modal_endpoint_max_context(ctx);
                sb.set_modal_endpoint_max_context_input(ctx.to_string().into());
                let retries = if ep.max_retries < 0 { 2 } else { ep.max_retries };
                sb.set_modal_endpoint_max_retries(retries);
                sb.set_modal_endpoint_max_retries_input(retries.to_string().into());
                sb.set_modal_endpoint_custom_headers(ep.custom_headers.clone());
                sb.set_modal_endpoint_temperature(if ep.temperature.is_empty() { "0.3".into() } else { ep.temperature.clone() });
                sb.set_modal_endpoint_show_advanced(false);
                sb.set_modal_endpoint_is_fetching_models(false);
                sb.set_modal_endpoint_test_status("idle".into());
                sb.set_modal_endpoint_test_message("".into());

                sb.set_is_ai_endpoint_editing(true);
                sb.set_is_ai_endpoint_modal_open(true);
            }
        }
    });

    // 4. 保存端点弹窗数据 (save_ai_endpoint_modal)
    let window_weak_save_ep_modal = window.as_weak();
    let notif_save_ep = ctx.notifications.clone();
    bridge.on_save_ai_endpoint_modal(move || {
        if let Some(w) = window_weak_save_ep_modal.upgrade() {
            let sb = w.global::<SettingsBridge>();
            let is_editing = sb.get_is_ai_endpoint_editing();
            let name = sb.get_modal_endpoint_name().trim().to_string();
            let base_url = sb.get_modal_endpoint_base_url().trim().to_string();
            let api_key = sb.get_modal_endpoint_api_key().trim().to_string();
            let api_mode = sb.get_modal_endpoint_api_mode().trim().to_string();
            let api_mode = if api_mode == "response" { "response" } else { "chat" };

            let models_list = sb.get_modal_endpoint_models_list();
            let models_vec: Vec<String> = models_list.iter().map(|m| m.to_string()).collect();
            let mut models_csv = models_vec.join(", ");
            let mut selected_model = sb.get_modal_endpoint_selected_model().trim().to_string();

            let thinking_degree = sb.get_modal_endpoint_thinking_degree().to_string();
            let timeout_secs = sb.get_modal_endpoint_timeout_secs_input().trim().parse::<i32>().unwrap_or(60);
            let max_context = sb.get_modal_endpoint_max_context_input().trim().parse::<i32>().unwrap_or(32768);
            let max_retries = sb.get_modal_endpoint_max_retries_input().trim().parse::<i32>().unwrap_or(2);
            let custom_headers = sb.get_modal_endpoint_custom_headers().trim().to_string();
            let temperature = sb.get_modal_endpoint_temperature().trim().to_string();

            if name.is_empty() {
                notif_save_ep.error("保存失败", "端点名称不能为空");
                return;
            }
            if base_url.is_empty() {
                notif_save_ep.error("保存失败", "API 接口地址不能为空");
                return;
            }

            if models_csv.is_empty() {
                models_csv = "deepseek-reasoner, deepseek-chat".to_string();
                selected_model = "deepseek-reasoner".to_string();
            } else if selected_model.is_empty() {
                if let Some(first) = models_vec.first() {
                    selected_model = first.clone();
                }
            }

            let endpoints_model = sb.get_setting_ai_endpoints();
            let mut list: Vec<AiEndpointProfile> = endpoints_model.iter().collect();

            if is_editing {
                let id = sb.get_modal_endpoint_id();
                let mut should_sync_active = false;
                for ep in list.iter_mut() {
                    if ep.id == id {
                        ep.name = name.clone().into();
                        ep.base_url = base_url.clone().into();
                        ep.api_key = api_key.clone().into();
                        ep.api_mode = api_mode.into();
                        ep.models_csv = models_csv.clone().into();
                        ep.selected_model = selected_model.clone().into();
                        ep.thinking_degree = thinking_degree.clone().into();
                        ep.timeout_secs = timeout_secs;
                        ep.max_context = max_context;
                        ep.max_retries = max_retries;
                        ep.custom_headers = custom_headers.clone().into();
                        ep.temperature = temperature.clone().into();
                        if ep.is_active {
                            should_sync_active = true;
                        }
                    }
                }
                if should_sync_active {
                    sb.set_setting_ai_base_url(base_url.into());
                    sb.set_setting_ai_api_key(api_key.into());
                    sb.set_setting_ai_model(selected_model.clone().into());
                    let slint_models: Vec<slint::SharedString> = models_vec.iter().map(|s| s.as_str().into()).collect();
                    sb.set_setting_ai_current_models(ModelRc::from(Rc::new(VecModel::from(slint_models.clone()))));
                    let ai_b = w.global::<AiBridge>();
                    ai_b.set_available_models(ModelRc::from(Rc::new(VecModel::from(slint_models))));
                    ai_b.set_selected_model(selected_model.into());
                    sb.set_setting_ai_thinking_budget(thinking_degree.into());
                    sb.set_setting_ai_timeout_secs(timeout_secs);
                    sb.set_setting_ai_timeout_secs_input(timeout_secs.to_string().into());
                    sb.set_setting_ai_max_context(max_context);
                    sb.set_setting_ai_max_context_input(max_context.to_string().into());
                    sb.set_setting_ai_max_retries(max_retries);
                    sb.set_setting_ai_max_retries_input(max_retries.to_string().into());
                    sb.set_setting_ai_custom_headers(custom_headers.into());
                    sb.set_setting_ai_temperature(temperature.parse::<f32>().unwrap_or(0.3));
                    sb.set_setting_ai_temperature_input(temperature.into());
                }
                notif_save_ep.success("修改成功", &format!("已更新端点: {}", name));
            } else {
                let new_id = format!("ep-{}", uuid::Uuid::new_v4().simple());
                let is_first = list.is_empty();
                list.push(AiEndpointProfile {
                    id: new_id.into(),
                    name: name.clone().into(),
                    base_url: base_url.into(),
                    api_key: api_key.into(),
                    api_mode: api_mode.into(),
                    selected_model: selected_model.into(),
                    models_csv: models_csv.into(),
                    is_active: is_first,
                    status_text: if is_first { "已连接".into() } else { "未激活".into() },
                    thinking_degree: thinking_degree.into(),
                    timeout_secs,
                    max_context,
                    max_retries,
                    custom_headers: custom_headers.into(),
                    temperature: temperature.into(),
                });
                notif_save_ep.success("添加成功", &format!("已添加端点: {}", name));
            }

            sb.set_setting_ai_endpoints(ModelRc::from(Rc::new(VecModel::from(list))));
            sb.set_is_ai_endpoint_modal_open(false);
        }
    });

    // 4.1 弹窗内添加模型 (modal_add_model)
    let window_weak_modal_add = window.as_weak();
    bridge.on_modal_add_model(move |new_model| {
        let trimmed = new_model.trim().to_string();
        if trimmed.is_empty() { return; }
        if let Some(w) = window_weak_modal_add.upgrade() {
            let sb = w.global::<SettingsBridge>();
            let cur = sb.get_modal_endpoint_models_list();
            let mut list: Vec<slint::SharedString> = cur.iter().collect();
            if !list.iter().any(|m| m.as_str() == trimmed) {
                list.push(trimmed.clone().into());
                sb.set_modal_endpoint_models_list(ModelRc::from(Rc::new(VecModel::from(list))));
                if sb.get_modal_endpoint_selected_model().is_empty() {
                    sb.set_modal_endpoint_selected_model(trimmed.into());
                }
            }
            sb.set_modal_endpoint_new_model_input("".into());
        }
    });

    // 4.2 弹窗内删除模型 (modal_remove_model)
    let window_weak_modal_rm = window.as_weak();
    bridge.on_modal_remove_model(move |target| {
        if let Some(w) = window_weak_modal_rm.upgrade() {
            let sb = w.global::<SettingsBridge>();
            let cur = sb.get_modal_endpoint_models_list();
            let mut list: Vec<slint::SharedString> = cur.iter().collect();
            list.retain(|m| m.as_str() != target.as_str());
            let new_sel = if sb.get_modal_endpoint_selected_model() == target {
                list.first().cloned().unwrap_or_default()
            } else {
                sb.get_modal_endpoint_selected_model()
            };
            sb.set_modal_endpoint_selected_model(new_sel);
            sb.set_modal_endpoint_models_list(ModelRc::from(Rc::new(VecModel::from(list))));
        }
    });

    // 4.3 弹窗内设为默认模型 (modal_select_model)
    let window_weak_modal_sel = window.as_weak();
    bridge.on_modal_select_model(move |m| {
        if let Some(w) = window_weak_modal_sel.upgrade() {
            w.global::<SettingsBridge>().set_modal_endpoint_selected_model(m);
        }
    });

    // 4.4 弹窗内一键拉取模型 (modal_fetch_models)
    let window_weak_modal_fetch = window.as_weak();
    let notif_m_fetch = ctx.notifications.clone();
    bridge.on_modal_fetch_models(move || {
        if let Some(w) = window_weak_modal_fetch.upgrade() {
            let sb = w.global::<SettingsBridge>();
            let base_url = sb.get_modal_endpoint_base_url().trim().to_string();
            let api_key = sb.get_modal_endpoint_api_key().trim().to_string();
            let custom_headers = sb.get_modal_endpoint_custom_headers().trim().to_string();

            if base_url.is_empty() {
                notif_m_fetch.error("获取失败", "API 接口地址 Base URL 不能为空");
                return;
            }

            sb.set_modal_endpoint_is_fetching_models(true);
            let w_weak = w.as_weak();

            let endpoint_cfg = smagical_core::AiEndpointConfig {
                base_url,
                api_key,
                model: String::new(),
                temperature: 0.3,
                timeout_secs: 8,
                custom_headers: if custom_headers.is_empty() { None } else { Some(custom_headers) },
            };

            crate::async_util::spawn_async(async move {
                let discovered = match smagical_core::AiClient::new(endpoint_cfg) {
                    Ok(client) => client.fetch_models().await.unwrap_or_default(),
                    Err(_) => Vec::new(),
                };

                let discovered_copy = discovered.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(win) = w_weak.upgrade() {
                        let sb = win.global::<SettingsBridge>();
                        sb.set_modal_endpoint_is_fetching_models(false);
                        let slint_models: Vec<slint::SharedString> = discovered_copy.iter().map(|s| s.as_str().into()).collect();
                        sb.set_modal_endpoint_models_list(ModelRc::from(Rc::new(VecModel::from(slint_models))));
                        if !discovered_copy.is_empty() {
                            let cur_sel = sb.get_modal_endpoint_selected_model().to_string();
                            if !discovered_copy.contains(&cur_sel) {
                                sb.set_modal_endpoint_selected_model(discovered_copy[0].clone().into());
                            }
                        }
                    }
                });
            });
        }
    });

    // 4.5 弹窗内测试连通性 (modal_test_connection)
    let window_weak_m_test = window.as_weak();
    bridge.on_modal_test_connection(move || {
        if let Some(w) = window_weak_m_test.upgrade() {
            let sb = w.global::<SettingsBridge>();
            let base_url = sb.get_modal_endpoint_base_url().trim().to_string();
            let api_key = sb.get_modal_endpoint_api_key().trim().to_string();
            let custom_headers = sb.get_modal_endpoint_custom_headers().trim().to_string();

            if base_url.is_empty() {
                sb.set_modal_endpoint_test_status("failed".into());
                sb.set_modal_endpoint_test_message("API 接口 Base URL 不能为空".into());
                return;
            }

            sb.set_modal_endpoint_test_status("testing".into());
            sb.set_modal_endpoint_test_message("正在发起握手探活...".into());
            let w_weak = w.as_weak();

            let endpoint_cfg = smagical_core::AiEndpointConfig {
                base_url,
                api_key,
                model: String::new(),
                temperature: 0.3,
                timeout_secs: 8,
                custom_headers: if custom_headers.is_empty() { None } else { Some(custom_headers) },
            };

            crate::async_util::spawn_async(async move {
                let test_res = match smagical_core::AiClient::new(endpoint_cfg) {
                    Ok(client) => client.test_connection().await,
                    Err(e) => smagical_core::AiTestResult {
                        success: false,
                        latency_ms: 0,
                        status_code: 0,
                        message: format!("客户端初始化失败: {}", e),
                    },
                };

                let status_s = if test_res.success { "success" } else { "failed" };
                let msg_s = test_res.message;
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(win) = w_weak.upgrade() {
                        let sb = win.global::<SettingsBridge>();
                        sb.set_modal_endpoint_test_status(status_s.into());
                        sb.set_modal_endpoint_test_message(msg_s.into());
                    }
                });
            });
        }
    });

    // 4.6 外部独立连通性测试按钮回调
    let window_weak_test_ai = window.as_weak();
    bridge.on_test_ai_connection(move || {
        if let Some(w) = window_weak_test_ai.upgrade() {
            w.global::<SettingsBridge>().invoke_modal_test_connection();
        }
    });

    // 5. 删除 AI 端点 (delete_ai_endpoint)
    let window_weak_del_ep = window.as_weak();
    let notif_del_ep = ctx.notifications.clone();
    bridge.on_delete_ai_endpoint(move |target_id| {
        if let Some(w) = window_weak_del_ep.upgrade() {
            let sb = w.global::<SettingsBridge>();
            let endpoints_model = sb.get_setting_ai_endpoints();
            let mut list: Vec<AiEndpointProfile> = endpoints_model.iter().collect();

            if list.len() <= 1 {
                notif_del_ep.error("无法删除", "至少需要保留一个 AI 端点配置");
                return;
            }

            let was_active = list.iter().find(|e| e.id == target_id).map(|e| e.is_active).unwrap_or(false);
            list.retain(|e| e.id != target_id);

            if was_active && !list.is_empty() {
                list[0].is_active = true;
                list[0].status_text = "已连接".into();
                let target_active_id = list[0].id.clone();
                let target_base_url = list[0].base_url.clone();
                let target_api_key = list[0].api_key.clone();
                let target_model = list[0].selected_model.clone();
                let target_csv = list[0].models_csv.to_string();
                let target_thinking = list[0].thinking_degree.clone();
                let target_timeout = list[0].timeout_secs;
                let target_context = list[0].max_context;
                let target_retries = list[0].max_retries;
                let target_headers = list[0].custom_headers.clone();
                let target_temp = list[0].temperature.clone();

                sb.set_active_endpoint_id(target_active_id);
                sb.set_setting_ai_base_url(target_base_url);
                sb.set_setting_ai_api_key(target_api_key);
                sb.set_setting_ai_model(target_model.clone());

                let mut models_vec: Vec<slint::SharedString> = Vec::new();
                for m in target_csv.split(',') {
                    let t = m.trim();
                    if !t.is_empty() { models_vec.push(t.into()); }
                }
                sb.set_setting_ai_current_models(ModelRc::from(Rc::new(VecModel::from(models_vec.clone()))));
                sb.set_setting_ai_thinking_budget(target_thinking);
                sb.set_setting_ai_timeout_secs(target_timeout);
                sb.set_setting_ai_timeout_secs_input(target_timeout.to_string().into());
                sb.set_setting_ai_max_context(target_context);
                sb.set_setting_ai_max_context_input(target_context.to_string().into());
                sb.set_setting_ai_max_retries(target_retries);
                sb.set_setting_ai_max_retries_input(target_retries.to_string().into());
                sb.set_setting_ai_custom_headers(target_headers);
                sb.set_setting_ai_temperature(target_temp.parse::<f32>().unwrap_or(0.3));
                sb.set_setting_ai_temperature_input(target_temp);

                let ai_b = w.global::<AiBridge>();
                ai_b.set_available_models(ModelRc::from(Rc::new(VecModel::from(models_vec))));
                ai_b.set_selected_model(target_model);
            }

            sb.set_setting_ai_endpoints(ModelRc::from(Rc::new(VecModel::from(list))));
            notif_del_ep.success("删除成功", "已移除该端点配置");
        }
    });

    // 6. 一键向远端接口获取支持的模型 (fetch_endpoint_models)
    let window_weak_fetch = window.as_weak();
    let notif_fetch = ctx.notifications.clone();
    bridge.on_fetch_endpoint_models(move || {
        if let Some(w) = window_weak_fetch.upgrade() {
            let sb = w.global::<SettingsBridge>();
            let base_url = sb.get_setting_ai_base_url().trim().to_string();
            let api_key = sb.get_setting_ai_api_key().trim().to_string();
            let custom_headers = sb.get_setting_ai_custom_headers().trim().to_string();

            if base_url.is_empty() {
                notif_fetch.error("获取失败", "API Base URL 不能为空");
                return;
            }

            sb.set_setting_ai_is_fetching_models(true);
            let w_weak = w.as_weak();

            let endpoint_cfg = smagical_core::AiEndpointConfig {
                base_url,
                api_key,
                model: String::new(),
                temperature: 0.3,
                timeout_secs: 8,
                custom_headers: if custom_headers.is_empty() { None } else { Some(custom_headers) },
            };

            crate::async_util::spawn_async(async move {
                let discovered_models = match smagical_core::AiClient::new(endpoint_cfg) {
                    Ok(client) => client.fetch_models().await.unwrap_or_default(),
                    Err(_) => Vec::new(),
                };

                let models_to_save = discovered_models.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(win) = w_weak.upgrade() {
                        let sb = win.global::<SettingsBridge>();
                        sb.set_setting_ai_is_fetching_models(false);

                        let count = models_to_save.len();
                        let slint_models: Vec<slint::SharedString> = models_to_save.iter().map(|s| s.as_str().into()).collect();
                        sb.set_setting_ai_current_models(ModelRc::from(Rc::new(VecModel::from(slint_models.clone()))));

                        let ai_b = win.global::<AiBridge>();
                        ai_b.set_available_models(ModelRc::from(Rc::new(VecModel::from(slint_models.clone()))));

                        let cur_sel = sb.get_setting_ai_model().to_string();
                        let new_sel = if !models_to_save.contains(&cur_sel) && !models_to_save.is_empty() {
                            models_to_save[0].clone()
                        } else {
                            cur_sel
                        };
                        sb.set_setting_ai_model(new_sel.clone().into());
                        ai_b.set_selected_model(new_sel.clone().into());

                        let active_id = sb.get_active_endpoint_id();
                        let eps_model = sb.get_setting_ai_endpoints();
                        let mut list: Vec<AiEndpointProfile> = eps_model.iter().collect();
                        let csv_str = models_to_save.join(", ");
                        for ep in list.iter_mut() {
                            if ep.id == active_id {
                                ep.models_csv = csv_str.clone().into();
                                ep.selected_model = new_sel.clone().into();
                            }
                        }
                        sb.set_setting_ai_endpoints(ModelRc::from(Rc::new(VecModel::from(list))));
                        let cur_toasts = win.global::<WindowBridge>().get_toasts();
                        let mut toasts_vec: Vec<ToastItemData> = cur_toasts.iter().collect();
                        toasts_vec.push(ToastItemData {
                            id: format!("toast-{}", uuid::Uuid::new_v4()).into(),
                            title: "获取模型完成".into(),
                            message: format!("成功发现并同步 {} 个可用模型", count).into(),
                            level: "success".into(),
                            position: "top-right".into(),
                            duration_ms: 3000,
                            closable: true,
                        });
                        win.global::<WindowBridge>().set_toasts(ModelRc::from(Rc::new(VecModel::from(toasts_vec))));
                    }
                });
            });
        }
    });

    // 7. 手动新增模型至当前端点 (add_model_to_current)
    let window_weak_add_m = window.as_weak();
    let notif_add_m = ctx.notifications.clone();
    bridge.on_add_model_to_current(move |new_model| {
        let trimmed = new_model.trim().to_string();
        if trimmed.is_empty() {
            return;
        }
        if let Some(w) = window_weak_add_m.upgrade() {
            let sb = w.global::<SettingsBridge>();
            let cur_model = sb.get_setting_ai_current_models();
            let mut list: Vec<slint::SharedString> = cur_model.iter().collect();

            if !list.iter().any(|m| m.as_str() == trimmed) {
                list.push(trimmed.clone().into());
                sb.set_setting_ai_current_models(ModelRc::from(Rc::new(VecModel::from(list.clone()))));

                let ai_b = w.global::<AiBridge>();
                ai_b.set_available_models(ModelRc::from(Rc::new(VecModel::from(list.clone()))));

                let active_id = sb.get_active_endpoint_id();
                let eps_model = sb.get_setting_ai_endpoints();
                let mut ep_list: Vec<AiEndpointProfile> = eps_model.iter().collect();
                for ep in ep_list.iter_mut() {
                    if ep.id == active_id {
                        let str_list: Vec<String> = list.iter().map(|s| s.to_string()).collect();
                        ep.models_csv = str_list.join(", ").into();
                    }
                }
                sb.set_setting_ai_endpoints(ModelRc::from(Rc::new(VecModel::from(ep_list))));

                notif_add_m.success("添加成功", &format!("已添加新模型: {}", trimmed));
            }
            sb.set_setting_ai_new_model_input("".into());
        }
    });

    // 8. 从当前端点移除模型 (remove_model_from_current)
    let window_weak_rm_m = window.as_weak();
    let notif_rm_m = ctx.notifications.clone();
    bridge.on_remove_model_from_current(move |target_model| {
        if let Some(w) = window_weak_rm_m.upgrade() {
            let sb = w.global::<SettingsBridge>();
            let cur_model = sb.get_setting_ai_current_models();
            let mut list: Vec<slint::SharedString> = cur_model.iter().collect();

            if list.len() <= 1 {
                notif_rm_m.error("无法删除", "端点至少需保留一个可用模型");
                return;
            }

            list.retain(|m| m.as_str() != target_model.as_str());
            sb.set_setting_ai_current_models(ModelRc::from(Rc::new(VecModel::from(list.clone()))));

            let ai_b = w.global::<AiBridge>();
            ai_b.set_available_models(ModelRc::from(Rc::new(VecModel::from(list.clone()))));

            let cur_sel = sb.get_setting_ai_model();
            let mut new_sel = cur_sel.clone();
            if cur_sel == target_model {
                new_sel = list[0].clone();
                sb.set_setting_ai_model(new_sel.clone());
                ai_b.set_selected_model(new_sel.clone());
            }

            let active_id = sb.get_active_endpoint_id();
            let eps_model = sb.get_setting_ai_endpoints();
            let mut ep_list: Vec<AiEndpointProfile> = eps_model.iter().collect();
            for ep in ep_list.iter_mut() {
                if ep.id == active_id {
                    let str_list: Vec<String> = list.iter().map(|s| s.to_string()).collect();
                    ep.models_csv = str_list.join(", ").into();
                    ep.selected_model = new_sel.clone();
                }
            }
            sb.set_setting_ai_endpoints(ModelRc::from(Rc::new(VecModel::from(ep_list))));

            notif_rm_m.success("已移除", &format!("已将模型 {} 移出当前端点", target_model));
        }
    });

    // 9. 选择当前默认推理模型 (select_current_model)
    let window_weak_sel_m = window.as_weak();
    bridge.on_select_current_model(move |model_name| {
        if let Some(w) = window_weak_sel_m.upgrade() {
            let sb = w.global::<SettingsBridge>();
            sb.set_setting_ai_model(model_name.clone());

            let ai_b = w.global::<AiBridge>();
            ai_b.set_selected_model(model_name.clone());

            let active_id = sb.get_active_endpoint_id();
            let eps_model = sb.get_setting_ai_endpoints();
            let mut ep_list: Vec<AiEndpointProfile> = eps_model.iter().collect();
            for ep in ep_list.iter_mut() {
                if ep.id == active_id {
                    ep.selected_model = model_name.clone();
                }
            }
            sb.set_setting_ai_endpoints(ModelRc::from(Rc::new(VecModel::from(ep_list))));
        }
    });
}
