use crate::domain::{
    ActiveTerminalSessionContext, GroupRecord, HistoryRecord, HostRecord, HostStatus,
    NavigationRequest, RightPanelItem, TerminalAction,
};
use crate::hook::HookDecision;
use super::types::{AppBootContext, AppExitContext, ConfigChangeEvent, WindowState};

/// 全局应用生命周期、三栏协同与主框架交互路由 Hook 接口。
///
/// 该 Trait 的所有生命周期方法均提供默认空实现，插件与监听者可按需选择性实现。
pub trait AppGlobalHook: Send + Sync {
    /// 插件/Hook 唯一名称标识。
    fn name(&self) -> &'static str;

    /// 插件优先级 (数值越大越先执行，默认为 0，核心守护插件可设为更高)。
    fn priority(&self) -> i32 {
        0
    }

    // =========================================================================
    // 1. 进程级生命周期 (Process Lifecycle - app_)
    // =========================================================================

    /// 【应用程序启动引导】：命令行参数解析完毕，存储引擎初始化前。
    fn on_app_boot(&self, _ctx: &AppBootContext) {}

    /// 【主界面首帧就绪】：Slint 主窗口首帧渲染完成，所有核心服务已就绪（触发后台异步预热）。
    fn on_app_ready(&self) {}

    /// 【应用即将退出】：用户点击关闭或触发退出指令前 (可拦截)。
    ///
    /// 若返回 `HookDecision::Abort`，则取消退出流程并保持应用运行。
    fn on_app_before_exit(&self, _ctx: &AppExitContext) -> HookDecision {
        HookDecision::Continue
    }

    /// 【应用完全退出】：主窗口即将销毁，执行最终全量归档备份与资源释放。
    fn on_app_exit(&self, _ctx: &AppExitContext) {}

    // =========================================================================
    // 2. 框架外壳与全局导航域 (Shell & Navigation - shell_)
    // =========================================================================

    /// 【页面跳转导航请求】：响应全局路由跳转意图。
    fn on_shell_navigation_requested(&self, _req: &NavigationRequest) {}

    /// 【模块激活挂载】：目标页面/抽屉被激活切入时调用。
    fn on_shell_module_activated(
        &self,
        _tab_id: &str,
        _sub_section: Option<&str>,
        _params: &std::collections::HashMap<String, String>,
    ) {}

    /// 【模块失活休眠】：页面/抽屉被切出或隐藏时调用 (用于暂停轮询、释放临时内存)。
    fn on_shell_module_deactivated(&self, _tab_id: &str) {}

    /// 【左侧活动栏菜单点击】：用户点击左侧图标切换抽屉面板。
    fn on_shell_left_menu_clicked(&self, _menu_id: &str, _old_menu_id: &str) {}

    /// 【主工作区视图切换】：在中央核心工作区之间流转 (如 terminal 终端 ⇄ history 历史中心)。
    fn on_shell_main_view_switched(&self, _current_view: &str, _previous_view: &str) {}

    /// 【全局快捷指令执行】：响应 Ctrl+K 命令面板或全局快捷键分发的指令。
    fn on_shell_command_executed(&self, _command_id: &str) {}

    /// 【全局模态弹窗显隐】：新建会话、Debug 等弹窗状态变更。
    fn on_shell_modal_toggled(&self, _modal_id: &str, _is_open: bool) {}

    /// 【窗口状态变动通知】：窗口最小化、最大化或获得/失去焦点。
    fn on_shell_window_state_changed(&self, _state: WindowState) {}

    // =========================================================================
    // 3. 左侧主机资产抽屉域 (Left Column - host_asset_)
    // =========================================================================

    /// 【主机资产创建成功】：写入存储后广播，驱动启动器与搜索索引更新。
    fn on_host_asset_created(&self, _host: &HostRecord) {}

    /// 【主机资产更新修改】：主机配置变更后广播，驱动右侧栏热重载与增量备份。
    fn on_host_asset_updated(&self, _old_host: &HostRecord, _new_host: &HostRecord) {}

    /// 【主机资产删除】：主机资产被删除后广播。
    fn on_host_asset_deleted(&self, _host_id: &str) {}

    /// 【主机分组创建】：新建多层级或顶级分组。
    fn on_host_asset_group_created(&self, _group: &GroupRecord) {}

    /// 【主机分组更新】：分组重命名或调整上级父节点。
    fn on_host_asset_group_updated(&self, _group: &GroupRecord) {}

    /// 【主机分组删除】：主机分组被删除。
    fn on_host_asset_group_deleted(&self, _group_id: &str) {}

    /// 【主机树分组折叠/展开】：左侧分组树展开状态变更。
    fn on_host_asset_group_toggled(&self, _group_id: &str, _is_expanded: bool) {}

    /// 【主机树节点拖拽调序】：拖拽物理迁移层级或列表排布调整。
    fn on_host_asset_tree_reordered(&self, _src_id: &str, _target_id: &str, _drop_position: &str) {}

    /// 【主机资产搜索过滤】：左侧搜索框输入字符过滤树与卡片。
    fn on_host_asset_search_filtered(&self, _query: &str, _match_count: usize) {}

    /// 【主机卡片单击选中预览】：单击卡片或树节点（未双击连接）时预览静态信息。
    fn on_host_asset_selected_for_preview(&self, _host: Option<&HostRecord>) {}

    /// 【主机后台探活状态更新】：后台定时探针检测到主机在线状态/延迟变化。
    fn on_host_asset_status_probed(&self, _host_id: &str, _status: HostStatus, _ping_ms: i32) {}

    // =========================================================================
    // 4. 中央终端工作区域 (Center Column - host_terminal_)
    // =========================================================================

    /// 【终端打开前拦截（责任链）】：双击准备发起连接前触发，可检查配额或准备跳板代理。
    fn on_host_terminal_opening(&self, _host_id: &str, _is_local: bool) -> HookDecision {
        HookDecision::Continue
    }

    /// 【终端会话建立就绪】：PTY 分配完毕并成功挂载 Tab。
    fn on_host_terminal_opened(&self, _session_id: &str, _ctx: &ActiveTerminalSessionContext) {}

    /// 【终端活跃聚焦会话变更】：当用户切换 Tab、多分屏聚焦或关闭会话时通知（单向焦点中枢）。
    fn on_host_terminal_focus_changed(&self, _ctx: Option<&ActiveTerminalSessionContext>) {}

    /// 【终端多分屏拓扑变更】：分屏新建、分屏合并、分栏比例调整。
    fn on_host_terminal_split_changed(&self, _pane_count: usize, _active_pane_id: &str, _is_split: bool) {}

    /// 【终端标题重命名】：用户双击 Tab 改名或远端 OSC 0/2 协议设置标题。
    fn on_host_terminal_title_renamed(&self, _session_id: &str, _new_title: &str) {}

    /// 【终端响铃提醒】：终端捕获 \x07 蜂鸣符，驱动主窗口闪烁或提示。
    fn on_host_terminal_bell_triggered(&self, _session_id: &str) {}

    /// 【终端关闭前守护（责任链）】：关闭会话 Tab 前确认，可防止误关正在编译的窗口。
    fn on_host_terminal_closing(&self, _session_id: &str) -> HookDecision {
        HookDecision::Continue
    }

    /// 【终端会话彻底销毁】：会话已退出并完成快照与耗时落盘。
    fn on_host_terminal_closed(&self, _session_id: &str, _duration_secs: u64) {}

    // =========================================================================
    // 5. 右侧辅助伴生抽屉域 (Right Column - host_right_)
    // =========================================================================

    /// 【右侧伴生抽屉展开/折叠】：右侧抽屉展开或收起（收起时面板进入挂起休眠以节能）。
    fn on_host_right_drawer_toggled(&self, _is_open: bool, _active_panel_id: &str) {}

    /// 【右侧伴生抽屉拖拽调整宽度】：拖拽调整宽度并记忆偏好。
    fn on_host_right_drawer_resized(&self, _width: f32) {}

    /// 【右侧伴生面板切换】：用户在 info、snippets、sftp、ai 等面板间切换。
    fn on_host_right_panel_switched(&self, _panel_id: &str, _is_open: bool) {}

    /// 【右侧伴生插件动态注册】：动态接入新的右侧工具箱（热插拔支持）。
    fn on_host_right_panel_registered(&self, _item: &RightPanelItem) {}

    /// 【右侧伴生插件注销】：卸载或隐藏右侧面板。
    fn on_host_right_panel_unregistered(&self, _panel_id: &str) {}

    /// 【右侧面板请求向当前终端注入指令/动作】：如执行代码片段、输入密码或 AI 修复命令。
    fn on_host_terminal_action_requested(&self, _session_id: &str, _action: &TerminalAction) {}

    /// 【右侧 SFTP 穿透目录同步请求】：SFTP 面板请求跟随终端当前工作目录同步。
    fn on_host_right_sftp_sync_requested(&self, _session_id: &str, _remote_path: &str) {}

    // =========================================================================
    // 6. 历史会话中心域 (history_)
    // =========================================================================

    /// 【会话历史记录沉淀】：终端关闭后时序与快照沉淀入库。
    fn on_history_session_recorded(&self, _history: &HistoryRecord) {}

    /// 【单条历史项删除】：用户删除特定历史记录。
    fn on_history_item_deleted(&self, _history_id: &str) {}

    /// 【会话历史一键清空】：清空全部会话历史。
    fn on_history_cleared(&self) {}

    /// 【会话历史置顶切换】：Pin / Unpin 常用历史项。
    fn on_history_pin_toggled(&self, _history_id: &str, _is_pinned: bool) {}

    /// 【从历史记录发起重新连接】：双击历史项触发重建会话。
    fn on_history_reconnect_requested(&self, _history_id: &str) {}

    // =========================================================================
    // 7. 凭据与密钥管理域 (credential_)
    // =========================================================================

    /// 【SSH 密钥/凭据创建】：新建密钥对、密码或证书。
    fn on_credential_created(&self, _cred_id: &str, _name: &str) {}

    /// 【SSH 密钥/凭据更新】：凭据更新。
    fn on_credential_updated(&self, _cred_id: &str) {}

    /// 【SSH 密钥/凭据删除】：凭据被删除。
    fn on_credential_deleted(&self, _cred_id: &str) {}

    // =========================================================================
    // 8. 运维代码片段域 (snippet_)
    // =========================================================================

    /// 【代码片段创建】：新建快捷脚本。
    fn on_snippet_created(&self, _snippet_id: &str, _title: &str) {}

    /// 【代码片段更新】：修改脚本内容或标签。
    fn on_snippet_updated(&self, _snippet_id: &str) {}

    /// 【代码片段删除】：删除脚本。
    fn on_snippet_deleted(&self, _snippet_id: &str) {}

    /// 【代码片段一键执行】：片段被注入到终端。
    fn on_snippet_executed(&self, _snippet_id: &str, _session_id: &str) {}

    // =========================================================================
    // 9. 设置、主题与配置变更域 (config_ / theme_)
    // =========================================================================

    /// 【全局参数变更通知】：当系统设置、终端参数或用户偏好修改时广播触发。
    fn on_config_changed(&self, _event: &ConfigChangeEvent) {}

    /// 【配置恢复默认预设】：用户重置特定模块配置。
    fn on_config_reset(&self, _section: &str) {}

    /// 【全局外观模式切换】：左下角一键切换深浅色主题。
    fn on_theme_mode_toggled(&self, _is_dark: bool) {}

    /// 【预设主题切换】：应用特定主题预设。
    fn on_theme_changed(&self, _theme_id: &str, _is_dark: bool) {}

    // =========================================================================
    // 10. 双盘文件浏览器与 SFTP 传输域 (File Explorer & Transfer - file_)
    // =========================================================================

    /// 【文件会话打开前拦截（责任链）】：打开 SFTP 会话前触发，可校验权限或准备跳板代理。
    fn on_file_tab_opening(&self, _host_id: &str, _initial_path: &str) -> HookDecision {
        HookDecision::Continue
    }

    /// 【文件会话建立就绪】：SFTP 或本地文件 Tab 成功建立并挂载。
    fn on_file_tab_opened(&self, _session_id: &str, _host_id: &str, _initial_path: &str) {}

    /// 【文件会话活跃聚焦变更】：双栏 Tab 切换或选栏时广播当前活动路径（供右侧工具栏伴生感知）。
    fn on_file_tab_focus_changed(&self, _session_id: Option<&str>, _is_remote: bool, _current_path: &str) {}

    /// 【文件目录导航跳转】：路径导航跳转后触发（前进、后退、向上、回车直达）。
    fn on_file_tab_navigated(&self, _session_id: &str, _is_remote: bool, _from_path: &str, _to_path: &str) {}

    /// 【文件会话关闭】：文件会话 Tab 被关闭。
    fn on_file_tab_closed(&self, _session_id: &str) {}

    /// 【文件高危操作前置拦截（责任链）】：在删除/覆写/修改权限前触发，用于敏感路径阻断。
    fn on_file_operation_before(&self, _op_type: &str, _is_remote: bool, _path: &str) -> HookDecision {
        HookDecision::Continue
    }

    /// 【文件/目录操作完成】：创建、删除、重命名、修改权限完成。
    fn on_file_operation_completed(&self, _op_type: &str, _is_remote: bool, _path: &str, _success: bool) {}

    /// 【传输任务入队前校验（责任链）】：传输任务创建与入队前校验（如文件大小配额、敏感后缀过滤）。
    fn on_file_transfer_enqueued(&self, _task: &crate::domain::TransferTask) -> HookDecision {
        HookDecision::Continue
    }

    /// 【传输任务开始执行】：传输任务开始。
    fn on_file_transfer_started(&self, _task_id: &str) {}

    /// 【传输进度与速率更新】：传输进度与速率更新（用于度量监控与底部抽屉统计）。
    fn on_file_transfer_progress(&self, _task_id: &str, _transferred: u64, _total: u64, _speed_bps: u64) {}

    /// 【传输任务完成】：传输任务完成（统一触发 Toast 成功通知与对侧目录增量刷新）。
    fn on_file_transfer_completed(&self, _task: &crate::domain::TransferTask) {}

    /// 【传输任务失败】：传输任务失败（统一触发 Toast 错误通知与重试标记）。
    fn on_file_transfer_failed(&self, _task: &crate::domain::TransferTask, _error_message: &str) {}
}



