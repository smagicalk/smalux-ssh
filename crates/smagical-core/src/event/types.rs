//! 核心领域强类型事件定义 (Domain Events)。
//!
//! 包含全系统通用的业务事实载荷，所有事件均为只读不可变、满足物理安全脱敏原则。

use serde::{Deserialize, Serialize};
use crate::domain::credential::CredentialType;

/// 凭据机密复制动作类型 (用于精准安全审计)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CredentialCopyType {
    /// 复制公钥文本 (低敏感)。
    PublicKey,
    /// 复制私钥 PEM 文本 (高敏感 ★)。
    PrivateKey,
    /// 复制登录明文密码 (高敏感 ★)。
    Password,
    /// 复制 Agent 命名管道路径 (低敏感)。
    AgentPipe,
    /// 复制公钥指纹 (低敏感)。
    Fingerprint,
}

/// 凭据保存 (新建或更新) 领域事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CredentialSavedEvent {
    /// 凭据全局唯一 ID。
    pub cred_id: String,
    /// 凭据可读名称。
    pub name: String,
    /// 凭据大类 (密钥/密码/Agent/证书)。
    pub cred_type: CredentialType,
    /// 密钥算法或哈希算法 (如 "Ed25519", "RSA-4096")。
    pub algorithm: String,
    /// 关联用户名 (可选)。
    pub username: Option<String>,
    /// 公钥 SHA-256 指纹 (可选)。
    pub fingerprint: Option<String>,
    /// 是否为全新创建 (true 为新建，false 为编辑更新)。
    pub is_new: bool,
}

/// 凭据从保管库安全删除领域事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CredentialDeletedEvent {
    /// 被删除的凭据唯一 ID。
    pub cred_id: String,
}

/// 凭据查看与选择领域事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CredentialSelectedEvent {
    /// 选中的凭据唯一 ID。
    pub cred_id: String,
}

/// 敏感机密提取与复制安全审计事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CredentialSecretCopiedEvent {
    /// 凭据全局唯一 ID。
    pub cred_id: String,
    /// 凭据名称。
    pub name: String,
    /// 复制的具体数据类型。
    pub copy_type: CredentialCopyType,
    /// 是否属于高危机密提取 (PrivateKey / Password)。
    pub is_sensitive: bool,
}

/// 密钥生成器完成密钥对生成事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyGeneratedEvent {
    /// 生成的算法规格 (如 "Ed25519", "RSA-4096")。
    pub algorithm: String,
    /// 生成的公钥指纹。
    pub fingerprint: String,
}

/// 强密码生成器完成密码生成事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PasswordGeneratedEvent {
    /// 生成时间戳 (UNIX 秒)。
    pub timestamp: u64,
}

/// 全局系统参数与配置热变动事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigChangedEvent {
    /// 变动的配置项键名 (如 "terminal.font_size", "appearance.theme")。
    pub key: String,
    /// 变动前原值 (字符串序列化)。
    pub old_val: String,
    /// 变动后新值 (字符串序列化)。
    pub new_val: String,
    /// 变动来源 (如 "user_ui", "preset_import")。
    pub source: String,
}

/// 主机资产创建或更新事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostAssetChangedEvent {
    /// 主机资产唯一 ID。
    pub host_id: String,
    /// 主机显示名称。
    pub name: String,
    /// 主机 IP 地址或域名。
    pub address: String,
    /// 关联凭据 ID (可选)。
    pub credential_id: Option<String>,
    /// 动作类型 ("created", "updated", "deleted")。
    pub action: String,
}

/// 主机分组折叠/展开事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostGroupToggledEvent {
    /// 分组唯一 ID。
    pub group_id: String,
    /// 是否为展开状态 (true 为展开，false 为收起)。
    pub is_expanded: bool,
}

/// 主机资产树拖拽调序事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostTreeReorderedEvent {
    /// 拖拽源节点 ID。
    pub source_id: String,
    /// 目标放置节点 ID。
    pub target_id: String,
    /// 放置位置 ("inside", "before", "after")。
    pub position: String,
}

/// 主机资产关键词搜索过滤事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostSearchFilteredEvent {
    /// 搜索关键词。
    pub query: String,
    /// 过滤命中结果数量。
    pub match_count: usize,
}

/// 终端会话生命周期事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalSessionEvent {
    /// 终端会话唯一 Tab ID。
    pub session_id: String,
    /// 关联主机 ID (如果是本地 Shell 则为 "local")。
    pub host_id: String,
    /// 动作类型 ("opened", "closed", "focus_changed")。
    pub action: String,
}

/// 终端分屏二叉树拓扑结构变动事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalSplitChangedEvent {
    /// 分屏 Pane 分组总数。
    pub group_count: usize,
    /// 当前活动 Pane ID。
    pub active_pane_id: String,
    /// 当前是否处于分屏状态。
    pub is_split: bool,
}

/// 主题切换事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemeChangedEvent {
    /// 主题标识 ID。
    pub theme_id: String,
    /// 是否为暗色主题。
    pub is_dark: bool,
}

/// 明暗模式快速切换事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemeModeToggledEvent {
    /// 切换后的目标模式是否为暗色。
    pub is_dark: bool,
}

/// 窗口状态变动事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowStateChangedEvent {
    /// 目标状态 ("minimized", "maximized", "restored", "fullscreen")。
    pub state: String,
}

/// 应用退出请求与清理事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppExitEvent {
    /// 退出原因或代码。
    pub exit_code: i32,
}

/// 导航菜单/活动栏切换事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NavigationTabClickedEvent {
    /// 导航目标 Tab ID (如 "hosts", "credentials", "history", "files", "settings")。
    pub tab_id: String,
    /// 附加参数 (可选)。
    pub query: String,
}

/// 历史记录双击发起重连事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryReconnectRequestedEvent {
    /// 历史记录唯一 ID。
    pub history_id: String,
}

/// 历史记录单条删除事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryItemDeletedEvent {
    /// 被删除的历史记录唯一 ID。
    pub history_id: String,
}

/// 历史记录全部清空事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryClearedEvent;

/// 历史记录置顶/取消置顶事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryPinToggledEvent {
    /// 历史记录唯一 ID。
    pub history_id: String,
    /// 是否置顶。
    pub is_pinned: bool,
}

/// 文件/SFTP Tab 焦点切换事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileTabFocusChangedEvent {
    /// Tab 唯一 ID (None 表示全部失焦)。
    pub tab_id: Option<String>,
    /// 是否为远程 SFTP 面板。
    pub is_remote: bool,
    /// 当前聚焦的工作路径。
    pub current_path: String,
}

/// 文件/SFTP Tab 关闭事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileTabClosedEvent {
    /// 被关闭的 Tab ID。
    pub tab_id: String,
}

/// 文件/SFTP Tab 打开事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileTabOpenedEvent {
    /// Tab 唯一 ID。
    pub tab_id: String,
    /// 关联的主机 ID ("local" 或远端主机 ID)。
    pub host_id: String,
    /// 初始工作路径。
    pub path: String,
}

/// 文件管理器路径导航跳转事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileTabNavigatedEvent {
    /// Tab 唯一 ID。
    pub tab_id: String,
    /// 是否为远程 SFTP。
    pub is_remote: bool,
    /// 跳转前原路径。
    pub old_path: String,
    /// 跳转后新路径。
    pub new_path: String,
}

/// 文件操作 (创建文件夹/新建文件/删除) 完成事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileOperationCompletedEvent {
    /// 操作类型 ("create_folder", "create_file", "delete")。
    pub action: String,
    /// 是否为远程 SFTP。
    pub is_remote: bool,
    /// 操作目标路径。
    pub path: String,
    /// 是否执行成功。
    pub success: bool,
}

/// 文件传输任务启动事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileTransferStartedEvent {
    /// 传输任务唯一 ID。
    pub task_id: String,
}

/// 文件操作前置拦截审查事件 (可被安全守护插件拦截中止)。
#[derive(Debug, Clone)]
pub struct FileOperationBeforeEvent {
    /// 操作类型 ("delete", "rename", "chmod")。
    pub action: String,
    /// 是否为远程 SFTP。
    pub is_remote: bool,
    /// 操作目标路径。
    pub path: String,
    cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
    abort_reason: std::sync::Arc<std::sync::RwLock<Option<String>>>,
}

impl FileOperationBeforeEvent {
    /// 创建新的文件操作前置审查事件。
    pub fn new(action: impl Into<String>, is_remote: bool, path: impl Into<String>) -> Self {
        Self {
            action: action.into(),
            is_remote,
            path: path.into(),
            cancelled: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            abort_reason: std::sync::Arc::new(std::sync::RwLock::new(None)),
        }
    }

    /// 拦截并阻止该操作执行。
    pub fn abort(&self, reason: impl Into<String>) {
        self.cancelled.store(true, std::sync::atomic::Ordering::SeqCst);
        *self.abort_reason.write().unwrap() = Some(reason.into());
    }

    /// 检查操作是否被阻止。
    pub fn is_aborted(&self) -> bool {
        self.cancelled.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// 获取拦截原因。
    pub fn abort_reason(&self) -> Option<String> {
        self.abort_reason.read().unwrap().clone()
    }
}

/// 应用退出前安全审查事件 (可被退出守护插件拦截中止)。
#[derive(Debug, Clone)]
pub struct AppBeforeExitEvent {
    /// 当前活跃终端/传输会话总数。
    pub active_session_count: usize,
    cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
    abort_reason: std::sync::Arc<std::sync::RwLock<Option<String>>>,
}

impl AppBeforeExitEvent {
    /// 创建新的应用退出前审查事件。
    pub fn new(active_session_count: usize) -> Self {
        Self {
            active_session_count,
            cancelled: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            abort_reason: std::sync::Arc::new(std::sync::RwLock::new(None)),
        }
    }

    /// 拦截退出流程。
    pub fn abort(&self, reason: impl Into<String>) {
        self.cancelled.store(true, std::sync::atomic::Ordering::SeqCst);
        *self.abort_reason.write().unwrap() = Some(reason.into());
    }

    /// 检查退出是否被拦截。
    pub fn is_aborted(&self) -> bool {
        self.cancelled.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// 获取拦截原因。
    pub fn abort_reason(&self) -> Option<String> {
        self.abort_reason.read().unwrap().clone()
    }
}

/// 文件 Tab 打开前安全审查事件 (可被连接安全策略拦截)。
#[derive(Debug, Clone)]
pub struct FileTabOpeningEvent {
    /// 目标主机 ID。
    pub host_id: String,
    /// 目标路径。
    pub path: String,
    cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
    abort_reason: std::sync::Arc<std::sync::RwLock<Option<String>>>,
}

impl FileTabOpeningEvent {
    /// 创建新的文件 Tab 打开前审查事件。
    pub fn new(host_id: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            host_id: host_id.into(),
            path: path.into(),
            cancelled: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            abort_reason: std::sync::Arc::new(std::sync::RwLock::new(None)),
        }
    }

    /// 拦截打开流程。
    pub fn abort(&self, reason: impl Into<String>) {
        self.cancelled.store(true, std::sync::atomic::Ordering::SeqCst);
        *self.abort_reason.write().unwrap() = Some(reason.into());
    }

    /// 检查是否被拦截。
    pub fn is_aborted(&self) -> bool {
        self.cancelled.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// 获取拦截原因。
    pub fn abort_reason(&self) -> Option<String> {
        self.abort_reason.read().unwrap().clone()
    }
}

/// 应用引导启动生命周期事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppBootEvent;

/// 应用首帧渲染就绪生命周期事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppReadyEvent;

/// 终端聚焦变化事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalFocusChangedEvent {
    /// 当前聚焦的会话 ID。
    pub session_id: Option<String>,
    /// 目标主机 ID。
    pub host_id: Option<String>,
}

/// 右侧辅助面板展开/折叠状态切换事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RightPanelSwitchedEvent {
    /// 面板唯一 ID。
    pub panel_id: String,
    /// 当前是否处于展开状态。
    pub is_open: bool,
}

/// 右侧辅助伴生面板动态注册事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RightPanelRegisteredEvent {
    /// 面板唯一 ID。
    pub panel_id: String,
    /// 面板提示/标题。
    pub tooltip: String,
}

/// 右侧辅助伴生面板动态注销事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RightPanelUnregisteredEvent {
    /// 面板唯一 ID。
    pub panel_id: String,
}

/// 终端动作交互请求事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalActionRequestedEvent {
    /// 会话唯一 ID。
    pub session_id: String,
    /// 交互动作详情。
    pub action: crate::domain::TerminalAction,
}

/// 页面跳转导航请求事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NavigationRequestedEvent {
    /// 目标 Tab 标识。
    pub target_tab: String,
    /// 子模块/分区标识。
    pub sub_section: Option<String>,
}

/// 模块激活生命周期事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleActivatedEvent {
    /// 激活的模块标识。
    pub target_tab: String,
    /// 子模块/分区。
    pub sub_section: Option<String>,
}

/// 模块失活生命周期事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleDeactivatedEvent {
    /// 失活的模块标识。
    pub target_tab: String,
}
