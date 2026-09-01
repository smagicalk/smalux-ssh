# smagical-core

`smagical-core` 是 **smalux-ssh** 的核心业务领域与数据存储抽象层 crate。它采用纯 Rust 编写，**不依赖任何图形界面框架**，保证业务逻辑的高内聚、可移植性与可测试性。

---

## 📁 目录结构

```text
crates/smagical-core/
├── Cargo.toml
└── src/
    ├── lib.rs                  # 根导出文件
    ├── domain/                 # 核心领域实体模型
    │   ├── mod.rs              # 领域模块导出
    │   ├── host.rs             # HostRecord 结构体与 HostStatus 状态枚举
    │   ├── group.rs            # GroupRecord 层级分组结构体
    │   ├── file_item.rs        # FileItemData, LocalFileTabSession, RemoteFileTabSession, TransferTask
    │   ├── right_panel.rs      # 右侧工具面板注册实体
    │   └── terminal_context.rs # 终端上下文模型
    ├── event/                  # 强类型泛型事件分发总线与集中管理器 (EventDispatcher / EventManager)
    ├── storage/                # 数据存储与持久化抽象层
    │   ├── mod.rs              # AppStorage, HostRepository, GroupRepository Trait 定义与 StorageError
    │   └── mock_storage.rs     # 线程安全并发内存存储实现 (MockStorage) 与种子预设引擎
    ├── state/                  # 应用全局状态机
    │   ├── mod.rs
    │   └── core_state.rs       # CoreState: 统一持有 Arc<dyn AppStorage>
    └── theme/                  # 主题元数据模型、TOML 解析、校验与文件仓库
        ├── mod.rs
        ├── models.rs           # ThemeDefinition, UiTheme, TerminalTheme
        ├── parser.rs           # TOML 序列化与反序列化
        ├── validator.rs        # 校验规则与对比度检查
        ├── resolver.rs         # 继承解析器 (多级 Base 主题递归展开)
        ├── repository.rs       # FileThemeRepository 文件持久化仓储
        └── service.rs          # ThemeService 主题门面服务
```

---

## 🧩 核心模块说明

### 1. 领域实体 (Domain Models)

- **`HostRecord`**：SSH 主机资产记录实体。
  - 字段：`id`, `name`, `address`, `port` (u16), `parent_group_id`, `status` (`HostStatus`), `ping_ms`, `sort_order`, `notes`。
  - **`HostStatus` 枚举**：`Online`（在线）、`Warning`（高延迟/警告）、`Error`（错误）、`Offline`（离线），原生支持 Serde 序列化与 `Display` / `From<&str>` 双向安全转换。
- **`GroupRecord`**：层级分组实体。
  - 字段：`id`, `name`, `parent_id` (Option<String>), `level`, `is_expanded`, `sort_order`。
  - 提供 `GroupRecord::root(...)` 与 `GroupRecord::child(...)` 便捷构造器。
- **`FileItemData`**：双盘文件浏览器统一节点模型。
  - 字段：`id`, `name`, `path`, `is_dir`, `size`, `size_formatted`, `modified_at`, `modified_formatted`, `permissions`, `owner`, `group`, `is_symlink`, `is_hidden`, `is_expanded`, `level`, `item_count`。
  - 提供 `new_file` 与 `new_dir` 构造工厂，内置人类可读大小与 ISO 格式时间格式化工具函数。
- **`LocalFileTabSession` / `RemoteFileTabSession`**：双栏会话 Tab 模型与导航历史管理。
  - 包含 `history: Vec<String>` 与 `history_index: usize` 双向历史记录栈，提供 `push_path`（自动清理前插栈分支）、`go_back`、`go_forward`、`can_go_back`、`can_go_forward`。
- **`TransferTask`**：文件传输任务实体（支持单文件与多层级文件夹树形递归拆解）。
  - 字段：`id`, `parent_id` (Option<String>), `session_id`, `filename`, `is_dir`, `is_expanded`, `level`, `item_count_text`, `source_path`, `target_path`, `direction` (`TransferDirection`), `total_bytes`, `transferred_bytes`, `speed_bytes_per_sec`, `status` (`TransferStatus`), `error_message`。
  - 提供 `progress() -> f32` 进度计算与 `speed_formatted() -> String` 速度格式化。

### 2. 存储抽象层 (Storage Abstraction)

通过 Trait 定义存储契约，使得上层 UI 与业务逻辑完全与具体存储介质解耦：

- **`HostRepository` Trait**：
  ```rust
  pub trait HostRepository: Send + Sync {
      fn list_all(&self) -> StorageResult<Vec<HostRecord>>;
      fn get_by_id(&self, id: &str) -> StorageResult<Option<HostRecord>>;
      fn save(&self, host: &HostRecord) -> StorageResult<()>;
      fn save_batch(&self, hosts: &[HostRecord]) -> StorageResult<()>;
      fn delete(&self, id: &str) -> StorageResult<bool>;
      fn update_list_order(&self, ordered_ids: &[String]) -> StorageResult<()>;
  }
  ```
- **`GroupRepository` Trait**：
  ```rust
  pub trait GroupRepository: Send + Sync {
      fn list_all(&self) -> StorageResult<Vec<GroupRecord>>;
      fn get_by_id(&self, id: &str) -> StorageResult<Option<GroupRecord>>;
      fn save(&self, group: &GroupRecord) -> StorageResult<()>;
      fn delete(&self, id: &str) -> StorageResult<bool>;
      fn set_expanded(&self, id: &str, expanded: bool) -> StorageResult<()>;
      fn move_group(&self, id: &str, new_parent_id: Option<&str>) -> StorageResult<()>;
  }
  ```
- **`AppStorage` Trait**：聚合门面，提供 `hosts()`、`groups()`、`reload()` 与 `flush()` 统一入口。
- **`MockStorage`**：基于 `Arc<RwLock<Vec<...>>>` 的并发内存实现，内置 6 组 10 主机真实种子数据。`move_group` 支持 BFS 递归更新所有子孙节点的层级 (`level`)。

### 3. 应用核心状态 (`CoreState`)

```rust
use smagical_core::{CoreState, MockStorage};
use std::sync::Arc;

// 默认使用内置种子内存存储
let state = CoreState::new_mock();

// 或注入自定义存储后端 (如 SQLite / JsonFileStorage)
let custom_storage = Arc::new(MockStorage::new());
let state = CoreState::new(custom_storage);

let all_hosts = state.storage().hosts().list_all()?;
```

### 4. 主题系统 (`ThemeService`)

- 支持 UI 主题（界面配色与布局尺寸令牌）与 Terminal 主题（ANSI 16 色）独立管理；
- 支持主题多级递归继承 (`base`)，自动补全缺省令牌；
- 内置低对比度可读性预警检查；
- 支持从 Windows Terminal JSON 格式直接导入配色预设。

---

## 🧪 单元测试

```bash
cargo test -p smagical-core
```
涵盖领域实体构造、MockStorage CRUD、批量排序与层级迁移、主题继承解析与文件持久化等 28 项测试。
