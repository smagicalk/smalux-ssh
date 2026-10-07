# smalux-cli: 纯 Rust 双模运维与资产命令行客户端

> **模块定位**：面向脚本自动化、无头服务器（Headless Server）与轻量终端环境的独立命令行工具。它完全基于纯 Rust 技术栈打造，提供**交互式终端界面 (TUI)** 与**命令行自动化 (CLI)** 双模能力，零依赖系统外部 `ssh.exe` 或 `ssh-keygen`。

---

## 一、 模块职责与着力点 (Focus & Scope)

1. **双模运行能力**：
   - **交互式 TUI 模式**（基于 Ratatui + Crossterm）：支持在纯文本终端中全屏渲染资产树、模糊搜索、凭据状态提示与回车一键拉起原生 PTY 交互会话；
   - **自动化 CLI 模式**（基于 Clap 4.5）：支持 `smalux host list --json`、`smalux host exec` 等标准子命令，便于 CI/CD 流水线与运维脚本集成。
2. **零外部二进制依赖**：
   - 远程连接与命令执行由 `smagical-ssh` 纯 Rust 协议驱动驱动；
   - 密钥对现场生成由 `NativeKeygenService` 内存计算完成；
   - 资产与凭据由 `smagical-storage` SQLite / 内存库无缝提供。
3. **安全保险库解锁托管**：
   - 集成 `auth::ensure_vault_unlocked`，支持通过 CLI 参数 `-p / --master-password`、环境变量 `SMALUX_MASTER_PASSWORD` 或标准输入安全密码输入框解锁主密钥。

---

## 二、 核心命令与命令行参数 (Commands & Arguments)

### 顶层参数 (`Cli`)

| 参数 / 选项 | 简写 | 类型 | 默认值 | 说明 |
| :--- | :--- | :--- | :--- | :--- |
| `--tui` | `-i` | `bool` | `false` | 强制进入全屏交互式 TUI 仪表盘模式 |
| `--master-password` | `-p` | `Option<String>` | `None` | 本地资产保险库主密码（亦可读取 `SMALUX_MASTER_PASSWORD`） |
| `command` | - | `Option<Commands>` | `None` | 子命令；若未指定则默认进入全屏 TUI 模式 |

### 子命令集 (`Commands`)

#### 1. 主机资产管理 (`smalux host`)
- **`list`**：列出所有主机资产
  - `--json` (`bool`)：以美化 JSON 格式输出资产清单，方便 `jq` 或 Python 管道过滤。
- **`connect`**：建立原生交互式 SSH 终端连接
  - `-i, --id <ID>` (`String`)：目标主机记录唯一 ID。
- **`exec`**：单次远程执行命令
  - `-i, --id <ID>` (`String`)：目标主机记录唯一 ID；
  - `-c, --cmd <CMD>` (`String`)：待执行的远程 Bash/Shell 指令，返回退出码与标准流。

#### 2. 密钥对生成 (`smalux keygen`)
- `-a, --algo <ALGO>` (`String`，默认 `"ed25519"`): 算法类型，支持 `ed25519` / `rsa` / `ecdsa`；
- `-p, --passphrase <PASS>` (`Option<String>`): 可选私钥加密口令；
- `-b, --bits <BITS>` (`Option<u32>`): RSA 密钥长度（如 2048, 4096）；
- `-o, --output <PATH>` (`Option<String>`): 输出私钥路径（自动伴生 `.pub` 公钥）。

#### 3. 隧道查看 (`smalux tunnel list`)
- 列出全部本地/远程端口转发配置。

---

## 三、 核心 Rust 函数与 API 契约 (Core Functions & Signatures)

### 1. 存储层智能初始化
```rust
pub async fn init_storage() -> Arc<dyn AppStorage>
```
- **参数**：无；
- **返回值**：实现了 `smagical_core::storage::AppStorage` 的动态派发引用；
- **机制**：优先探测并开启本地物理 SQLite 数据库 (`SeaOrmStorage::open_default().await`)；若物理库不存在或无法加载，无缝降级回退至带模拟数据的 `MockStorage`。

### 2. 保险库主密码认证与就绪检查
```rust
// auth.rs
pub async fn ensure_vault_unlocked(
    storage: &Arc<dyn AppStorage>,
    cmd_pwd: Option<&str>,
) -> Result<()>
```
- **参数**：
  - `storage`: 当前初始化的数据仓储抽象；
  - `cmd_pwd`: 命令行显式传入的密码参数（若有）。
- **说明**：自动检查当前是否开启了主密码保护；若未解锁，依序尝试传入参数、环境变量与终端隐蔽输入 (`rpassword::prompt_password`)。

### 3. 原生交互式 SSH 终端会话
```rust
// terminal_session.rs
pub async fn run_interactive_session(
    host: &HostRecord,
    credential: Option<&CredentialRecord>,
    ssh_svc: &Arc<dyn SshSessionService>,
) -> Result<()>
```
- **参数**：
  - `host`: 目标主机领域实体；
  - `credential`: 解密后的凭据领域实体（可选）；
  - `ssh_svc`: 纯 Rust SSH 协议驱动实例。
- **说明**：将宿主机当前终端切换为 Raw 模式，监听视口尺寸调整 (`SIGWINCH` / Windows 控制台事件)，建立双向流并在会话退出后安全重置控制台。

### 4. 全屏终端 TUI 仪表盘启动
```rust
// tui.rs
pub async fn run_tui(
    storage: Arc<dyn AppStorage>,
    ssh_svc: Arc<dyn SshSessionService>,
    master_password: Option<&str>,
) -> Result<()>
```
- **参数**：仓储引用、SSH 驱动引用与可选主密码；
- **说明**：构建 Ratatui 交互式 TUI 应用，支持快捷键导航、主机过滤与快速发起远程连接。

---

## 四、 调用与使用示例 (Usage Examples)

### 场景 1：以 JSON 格式输出主机清单并交由 jq 处理
```bash
smalux host list --json | jq '.[] | {id: .id, name: .name, address: .address}'
```

### 场景 2：现场零依赖生成 Ed25519 运维密钥对
```bash
smalux keygen --algo ed25519 -o ~/.ssh/id_ed25519_smalux
```

### 场景 3：非交互式单次远程执行命令
```bash
smalux host exec --id "host-prod-01" --cmd "uname -a && uptime"
```

### 场景 4：拉起全屏交互 TUI
```bash
smalux -i
```
