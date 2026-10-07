# smagical-storage

`smagical-storage` 是 **smalux-ssh** 的数据持久化与底层存储实现层 crate。它负责实现 `smagical-core` 中定义的统一数据仓储契约 (`AppStorage` 及 7 大 Repository Trait)，提供**工业级安全加密保险库**、**基于 SeaORM 的 SQLite 物理持久化引擎**、**开箱即用的内存 Mock 仿真存储**以及**跨进程存储模式首选项治理**。

---

## 📁 目录结构与模块全景

```text
crates/smagical-storage/
├── Cargo.toml                  # 依赖清单 (sea-orm, rusqlite, aes-gcm, argon2, base64 等)
├── README.md                   # 模块架构与子模块职责文档 (本文档)
└── src/
    ├── lib.rs                  # 顶层入口导出 (SeaOrmStorage, MockStorage, storage_mode 工具函数)
    ├── storage_mode.rs         # 存储模式首选项持久化治理 (~/.config/smalux-ssh/storage_mode.txt)
    ├── crypto/                 # 工业级安全保险库与密码学底层 (AES-256-GCM + Argon2id)
    │   ├── mod.rs              # CryptoService: KDF 密钥派生、信封加密、金丝雀校验 (RFC 9106)
    │   └── vault.rs            # VaultManager: 数据加密密钥 (DEK) 内存生命周期、动态解密与内存抹零
    ├── entities/               # SeaORM 关系型实体映射定义 (13 个实体)
    │   ├── mod.rs              # 实体统一聚合导出
    │   ├── host.rs             # 主机资产数据表 (hosts)
    │   ├── group.rs            # 资产层级分组表 (groups)
    │   ├── credential.rs       # 安全凭据表 (credentials, 密文存储)
    │   ├── tunnel.rs           # 隧道代理表 (tunnels)
    │   ├── snippet.rs          # 运维代码片段表 (snippets)
    │   ├── snippet_group.rs    # 代码片段多级分组表 (snippet_groups)
    │   ├── history.rs          # 会话连接审计表 (histories)
    │   ├── snapshot.rs         # 终端输出屏幕快照表 (history_snapshots)
    │   ├── config.rs           # 全局系统偏好配置表 (app_configs)
    │   ├── vault_security.rs   # 保险库主安全表 (vault_securities: Salt, KDF 参数, Canary, 加密 DEK)
    │   ├── system_meta.rs      # 系统元数据与版本追踪表 (system_meta)
    │   ├── backup_task.rs      # 多端定时云同步与备份任务表 (backup_tasks)
    │   └── backup_snapshot.rs  # 本地与远程备份快照归档元数据表 (backup_snapshots)
    ├── seaorm/                 # 基于 SQLite 的 SeaORM 物理持久化仓储实现
    │   ├── mod.rs              # SeaOrmStorage 聚合门面与 AppStorage 实现 (含主密码生命周期)
    │   ├── connection.rs       # 统一连接池、自适应建表 DDL 初始化与保险库 Bootstrap
    │   ├── host_repo.rs        # 主机资产数据库 CRUD
    │   ├── group_repo.rs       # 分组层级与树移动持久化
    │   ├── credential_repo.rs  # 密文凭据存取、绑定查询与全文检索
    │   ├── tunnel_repo.rs      # 隧道代理规则持久化与度量指标更新
    │   ├── snippet_repo.rs     # 片段与片段分组持久化
    │   ├── history_repo.rs     # 会话审计日志与屏幕快照存取
    │   └── config_repo.rs      # 系统偏好持久化与原子更新
    └── mock/                   # 纯内存种子仿真存储实现 (用于演示、快速启动与端到端测试)
        ├── mod.rs              # MockStorage 门面与 AppStorage 实现
        ├── seed_data.rs        # 预设真实生产/开发分组、主机与凭据种子生成器
        ├── host_repo.rs        # 基于 Arc<RwLock<Vec<HostRecord>>> 的主机仓储
        ├── group_repo.rs       # 支持 BFS 树层级深度重排的内存分组仓储
        ├── credential_repo.rs  # 内存凭据仓储
        ├── tunnel_repo.rs      # 内存网络隧道仓储
        ├── snippet_repo.rs     # 内存代码片段仓储
        ├── history_repo.rs     # 内存连接历史仓储
        └── config_repo.rs      # 内存全局配置仓储
```

---

## 🧩 各子模块 (`mod`) 详细职责解析

### 1. `crypto` - 工业级安全保险库 (Cryptographic Vault)

负责保护敏感资产（如 SSH 私钥、私钥密码、明文密码、跳板认证凭据）：

- **[`crypto::mod`](src/crypto/mod.rs)** - **`CryptoService`**：
  - **Argon2id 密钥派生 (RFC 9106)**：使用内存硬化算法抵御 GPU/ASIC 暴力破解（默认安全参数：64MB 内存、3轮迭代、4并发线程）；
  - **AES-256-GCM 认证加密**：256 位密钥 + 96 位随机 Nonce + 128 位 GCM 认证 Tag，输出标准格式密文 `enc:v<version>:<base64_nonce>:<base64_ciphertext_with_tag>`；
  - **金丝雀安全校验 (Canary Verification)**：使用 KEK 加密固定魔数串 `SMALUX_SSH_VAULT_CANARY_V1`；在用户输入主密码时，仅需尝试解密金丝雀即可判定密码正误，绝不泄漏实际资产密钥。
- **[`crypto::vault`](src/crypto/vault.rs)** - **`VaultManager`**：
  - **信封加密体系 (Envelope Encryption)**：用户主密码派生出的 MasterKey (KEK) 负责保护真正的数据加密密钥 (DEK)；
  - **内存安全即用即抹**：未解锁时 DEK 在 RAM 中为 `None`；锁定 (`lock()`) 时立即清零擦除；
  - **平滑兼容历史数据**：解密时若字符串非 `enc:v` 前缀，则视作历史明文平滑放行，避免旧数据损坏。

---

### 2. `entities` - SeaORM 关系型实体映射

定义 SQLite 物理数据库中的各个表 Schema，每个 Entity 对应业务领域模型：

- **`VaultSecurity`** (`vault_securities`)：存储保险库安全根配置（`kdf_algorithm`, `kdf_salt`, `canary_ciphertext`, `encrypted_dek`, `dek_version`, `password_hint` 等）；
- **`Host`** (`hosts`)：记录主机网络地址、SSH 端口、归属分组、在线状态、排序索引；
- **`Group`** (`groups`)：记录分组多叉树结构、展开状态、父级 ID；
- **`Credential`** (`credentials`)：存储凭据类型、用户名、算法、公钥指纹以及被 Vault 物理加密的密码/私钥密文；
- **`Tunnel`** (`tunnels`)：记录本地/远端转发规则、SOCKS5 代理、跳板拓扑、运行状态与累计收发字节数；
- **`Snippet`** & **`SnippetGroup`** (`snippets`, `snippet_groups`)：存储脚本模版内容与多级分组目录；
- **`History`** & **`HistorySnapshot`** (`histories`, `history_snapshots`)：存储终端会话审计足迹与屏幕输出快照；
- **`BackupTask`** & **`BackupSnapshot`** (`backup_tasks`, `backup_snapshots`)：存储多端云同步配置策略（S3 / WebDAV / Gist / 本地）与备份归档元数据快照。

---

### 3. `seaorm` - 物理数据库引擎与仓储实现

- **[`seaorm::connection`](src/seaorm/connection.rs)**：
  - 自动定位跨平台本地标准数据存储路径（Windows: `%APPDATA%/smalux-ssh/data.db`，Linux: `~/.local/share/smalux-ssh/data.db`）；
  - 提供 `establish_connection`：在启动时连接 SQLite 并利用 SeaORM `Schema` 机制**自动执行 DDL 建表**，无需手动执行复杂的 SQL 迁移脚本；
  - 提供 `bootstrap_vault`：首次初始化时自动以机器安全种子派生初始 DEK；若已存在用户自定义主密码，则初始挂载为锁定状态等待用户解锁。
- **仓储实现集**：
  - `host_repo`, `group_repo`, `credential_repo`, `tunnel_repo`, `snippet_repo`, `history_repo`, `config_repo` 均实现了 `smagical-core::storage` 中对应的异步 Trait；
  - 针对字段加密字段（如凭据的秘密数据），在写入数据库前调用 `VaultManager::encrypt_string`，在读出后调用 `VaultManager::decrypt_string`。
- **[`seaorm::mod`](src/seaorm/mod.rs)** - **`SeaOrmStorage`**：
  - 聚合统一门面，实现 `AppStorage`；
  - 实现了 `has_custom_master_password`、`unlock_vault`、`change_master_password` 与 `remove_master_password`；
  - 改密流程：使用旧密码验证金丝雀 -> 使用新密码重派生 KEK -> 重新加密现有 DEK -> 重新生成金丝雀密文 -> 事务落盘。

---

### 4. `mock` - 内存仿真与预设种子引擎

- **`MockStorage`**：
  - 基于 `Arc<RwLock<Vec<T>>>` 的高并发内存结构，拥有极快的读写速度与完全的无锁/细粒度读写锁并发保护；
  - 适合用于 UI 开发、自动化 CI 测试、演示 Demo 环境；
- **[`mock::seed_data`](src/mock/seed_data.rs)**：
  - 自动预装 6 组层级分类（生产环境、预发测试、边缘网关、个人实验室等）、10+ 真实模拟主机、经典 SSH 凭据模板、常用 Docker/K8s 代码片段与端口转发规则。

---

### 5. `storage_mode` - 存储模式本地配置治理

- **[`storage_mode.rs`](src/storage_mode.rs)**：
  - 存储模式配置文件路径：`~/.config/smalux-ssh/storage_mode.txt`；
  - 提供 `get_persisted_storage_mode() -> Option<String>` 与 `save_persisted_storage_mode(mode: &str) -> io::Result<()>`；
  - 支持的值：`"physical"`（默认 SQLite 物理持久化模式）与 `"mock"`（内存仿真模式）；
  - 使得 UI 设置界面的存储模式切换能无感跨进程、跨重启持久生效，亦便于后续自研 CLI 命令行工具读取相同的存储设置。

---

## 三、 核心 API 与调用方法 (Core APIs & Signatures)

### 1. 初始化仓储实例
```rust
// 打开本地物理 SQLite 数据库 (自动初始化 schema 并以 WAL 模式运行)
pub async fn open_default() -> Result<SeaOrmStorage, StorageError>;

// 打开纯内存临时数据库 (适合单元测试与轻量 CI)
pub async fn open_in_memory() -> Result<SeaOrmStorage, StorageError>;

// 实例化内存仿真种子仓储
pub fn new() -> MockStorage;
```

### 2. 保险库主密码与安全锁控制
```rust
// 查询当前是否设置了用户自定义主密码
pub async fn has_custom_master_password(&self) -> Result<bool, StorageError>;

// 使用密码尝试解密主数据密钥 (金丝雀验证)
pub async fn unlock_vault(&self, password: &str) -> Result<bool, StorageError>;

// 修改主密码 (原子重新派生 KEK 并重加密 DEK)
pub async fn change_master_password(&self, old_pwd: &str, new_pwd: &str) -> Result<(), StorageError>;

// 移除主密码 (回退至本地机器种子保护)
pub async fn remove_master_password(&self, old_pwd: &str) -> Result<(), StorageError>;
```

### 3. 七大业务仓储门面调用
```rust
let storage = SeaOrmStorage::open_default().await?;

// 主机资产 CRUD
let hosts = storage.hosts().list_all().await?;
let host = storage.hosts().get_by_id("host-id").await?;

// 安全凭据 CRUD (读取时自动透明解密私钥/密码)
let cred = storage.credentials().get_by_id("cred-id").await?;

// 端口隧道配置
let tunnels = storage.tunnels().list_all().await?;

// 运维脚本片段
let snippets = storage.snippets().list_all().await?;

// 会话审计足迹与屏幕快照
let histories = storage.history().list_recent(50).await?;
let snapshot = storage.history().get_snapshot("session-id").await?;
```

---

## 🔒 安全设计特性

1. **零外部命令行依赖**：所有的加密、哈希、随机数与数据库操作完全基于纯 Rust 实现（`aes-gcm`, `argon2`, `rusqlite`, `sea-orm`），无需系统安装任何外部工具；
2. **两级密钥保护体系 (KEK + DEK)**：更改主密码时，无需重新加密海量凭据数据，只需重新加密 32 字节的 DEK；
3. **金丝雀校验隔离**：主密码验证过程完全脱敏，验证失败不会波及已解密的数据内存。

---

## 🧪 单元测试

```bash
cargo test -p smagical-storage
```
覆盖 Argon2id 密钥派生、AES-256-GCM 加密解密、金丝雀校验、存储模式文本配置校验、Mock 树形移动层级更新与 SQLite 内存库 CRUD 等 17+ 项测试。
