//! 数据存储抽象层 (Storage Abstraction Layer)
//!
//! 提供领域资产与配置的仓储 Trait 接口定义，完全解耦具体存储介质 (内存、JSON 文件、SQLite、加密数据库等)。

/// 基于纯内存与预设种子的 Mock 存储实现。
pub mod mock_storage;

use crate::domain::{group::GroupRecord, history::HistoryRecord, host::HostRecord};

/// 存储操作统一结果类型
pub type StorageResult<T> = Result<T, StorageError>;

/// 存储层统一错误枚举
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// 目标实体记录未找到
    #[error("数据未找到: {0}")]
    NotFound(String),
    /// 数据序列化/反序列化失败
    #[error("数据序列化失败: {0}")]
    Serialization(String),
    /// 底层 IO 读写异常
    #[error("存储 IO 异常: {0}")]
    Io(#[from] std::io::Error),
    /// 底层数据库或存储后端特定错误
    #[error("存储后端错误: {0}")]
    Backend(String),
}

/// 主机资产仓储接口契约
pub trait HostRepository: Send + Sync {
    /// 获取全部主机列表
    fn list_all(&self) -> StorageResult<Vec<HostRecord>>;

    /// 根据唯一 ID 查询单条主机记录
    fn get_by_id(&self, id: &str) -> StorageResult<Option<HostRecord>>;

    /// 根据关联凭据 ID 查询所有引用该凭据的主机
    fn list_by_credential(&self, credential_id: &str) -> StorageResult<Vec<HostRecord>>;

    /// 保存或更新主机记录
    fn save(&self, host: &HostRecord) -> StorageResult<()>;

    /// 批量保存主机记录
    fn save_batch(&self, hosts: &[HostRecord]) -> StorageResult<()>;

    /// 删除指定主机记录 (返回 true 表示成功删除，false 表示此前不存在)
    fn delete(&self, id: &str) -> StorageResult<bool>;

    /// 更新列表模式下的主机显示排序
    fn update_list_order(&self, ordered_ids: &[String]) -> StorageResult<()>;
}

/// 分组层级仓储接口契约
pub trait GroupRepository: Send + Sync {
    /// 获取全部分组列表
    fn list_all(&self) -> StorageResult<Vec<GroupRecord>>;

    /// 根据唯一 ID 查询分组记录
    fn get_by_id(&self, id: &str) -> StorageResult<Option<GroupRecord>>;

    /// 保存或更新分组记录
    fn save(&self, group: &GroupRecord) -> StorageResult<()>;

    /// 删除指定分组记录
    fn delete(&self, id: &str) -> StorageResult<bool>;

    /// 设置指定分组的折叠/展开状态
    fn set_expanded(&self, id: &str, expanded: bool) -> StorageResult<()>;

    /// 移动分组至新的父级分组
    fn move_group(&self, id: &str, new_parent_id: Option<&str>) -> StorageResult<()>;
}

/// 历史会话仓储接口契约
pub trait HistoryRepository: Send + Sync {
    /// 获取全部历史会话记录（按连接时间倒序排列）
    fn list_all(&self) -> StorageResult<Vec<HistoryRecord>>;

    /// 根据唯一 ID 查询单条历史记录
    fn get_by_id(&self, id: &str) -> StorageResult<Option<HistoryRecord>>;

    /// 保存或更新历史记录
    fn save(&self, record: &HistoryRecord) -> StorageResult<()>;

    /// 批量保存历史记录
    fn save_batch(&self, records: &[HistoryRecord]) -> StorageResult<()>;

    /// 删除指定历史记录
    fn delete(&self, id: &str) -> StorageResult<bool>;

    /// 清空全部历史记录 (keep_pinned: 是否保留置顶项)
    fn clear_all(&self, keep_pinned: bool) -> StorageResult<()>;

    /// 切换或设置指定历史记录的置顶标星状态
    fn toggle_pin(&self, id: &str) -> StorageResult<bool>;

    /// 保存指定历史会话的终端屏幕输出快照 (按 max_lines 限制最新行数，0 为不限)
    fn save_snapshot(&self, history_id: &str, content: &str, max_lines: usize) -> StorageResult<()>;

    /// 读取指定历史会话的终端屏幕快照完整文本
    fn get_snapshot(&self, history_id: &str) -> StorageResult<Option<String>>;

    /// 删除指定历史会话的终端屏幕快照
    fn delete_snapshot(&self, history_id: &str) -> StorageResult<bool>;
}

/// 凭据与密钥资产仓储接口契约
pub trait CredentialRepository: Send + Sync {
    /// 获取全部凭据记录
    fn list_all(&self) -> StorageResult<Vec<crate::domain::credential::CredentialRecord>>;

    /// 按分类查询 (Key / Password / Agent / Certificate)
    fn list_by_type(&self, cred_type: crate::domain::credential::CredentialType) -> StorageResult<Vec<crate::domain::credential::CredentialRecord>>;

    /// 根据唯一 ID 查询单条凭据记录
    fn get_by_id(&self, id: &str) -> StorageResult<Option<crate::domain::credential::CredentialRecord>>;

    /// 模糊搜索凭据 (按名称、用户名、公钥指纹、备注)
    fn search(&self, query: &str) -> StorageResult<Vec<crate::domain::credential::CredentialRecord>>;

    /// 获取引用/绑定了该凭据的主机 ID 列表
    fn get_bound_hosts(&self, id: &str) -> StorageResult<Vec<String>>;

    /// 保存或更新凭据记录
    fn save(&self, record: &crate::domain::credential::CredentialRecord) -> StorageResult<()>;

    /// 批量保存凭据记录
    fn save_batch(&self, records: &[crate::domain::credential::CredentialRecord]) -> StorageResult<()>;

    /// 删除指定凭据记录
    fn delete(&self, id: &str) -> StorageResult<bool>;
}

/// 代码片段与层级分组仓储接口契约
pub trait SnippetRepository: Send + Sync {
    /// 获取全部代码片段记录
    fn list_all(&self) -> StorageResult<Vec<crate::domain::snippet::SnippetRecord>>;

    /// 按所属文件夹分组获取代码片段 (None 表示根目录下)
    fn list_by_group(&self, group_id: Option<&str>) -> StorageResult<Vec<crate::domain::snippet::SnippetRecord>>;

    /// 根据唯一 ID 查询代码片段
    fn get_by_id(&self, id: &str) -> StorageResult<Option<crate::domain::snippet::SnippetRecord>>;

    /// 模糊搜索代码片段 (按标题、命令、标签、备注)
    fn search(&self, query: &str) -> StorageResult<Vec<crate::domain::snippet::SnippetRecord>>;

    /// 保存或更新代码片段
    fn save(&self, record: &crate::domain::snippet::SnippetRecord) -> StorageResult<()>;

    /// 批量保存代码片段
    fn save_batch(&self, records: &[crate::domain::snippet::SnippetRecord]) -> StorageResult<()>;

    /// 删除指定代码片段
    fn delete(&self, id: &str) -> StorageResult<bool>;

    /// 切换星标置顶状态
    fn toggle_favorite(&self, id: &str) -> StorageResult<bool>;

    /// 获取全部代码片段层级分组
    fn list_groups(&self) -> StorageResult<Vec<crate::domain::snippet::SnippetGroupRecord>>;

    /// 根据唯一 ID 查询分组
    fn get_group_by_id(&self, id: &str) -> StorageResult<Option<crate::domain::snippet::SnippetGroupRecord>>;

    /// 保存或更新分组
    fn save_group(&self, group: &crate::domain::snippet::SnippetGroupRecord) -> StorageResult<()>;

    /// 删除指定分组 (以及其关联的子项/将其子项回退至父级)
    fn delete_group(&self, id: &str) -> StorageResult<bool>;

    /// 设置分组折叠/展开状态
    fn set_group_expanded(&self, id: &str, expanded: bool) -> StorageResult<()>;

    /// 移动分组至新的父级分组
    fn move_group(&self, id: &str, new_parent_id: Option<&str>) -> StorageResult<()>;
}

/// 聚合存储服务门面契约
pub trait AppStorage: Send + Sync {
    /// 主机仓储句柄
    fn hosts(&self) -> &dyn HostRepository;

    /// 分组仓储句柄
    fn groups(&self) -> &dyn GroupRepository;

    /// 历史会话仓储句柄
    fn history(&self) -> &dyn HistoryRepository;

    /// 凭据仓储句柄
    fn credentials(&self) -> &dyn CredentialRepository;

    /// 代码片段仓储句柄
    fn snippets(&self) -> &dyn SnippetRepository;

    /// 强制从物理介质重新加载数据
    fn reload(&self) -> StorageResult<()>;

    /// 强制将内存缓冲数据持久化刷盘
    fn flush(&self) -> StorageResult<()>;
}

pub use mock_storage::MockStorage;

