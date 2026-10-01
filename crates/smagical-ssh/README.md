# 🛡️ Smagical SSH - 独立无 UI 核心协议与远程运维引擎

> **定位**：完全独立、零宿主二进制依赖（Zero Host Dependency）、面向生产级的 Headless SSHv2 / SFTP 协议与系统运维核心库。既作为 `smagical-ui` 的协议支撑层，也可直接作为无头后台守护进程或未来 CLI 命令行工具（如 `smalux-cli`）的核心引擎。

---

## 📌 目录
- [一、设计哲学与架构原则](#一设计哲学与架构原则)
- [二、现状基线与外部依赖短板评估](#二现状基线与外部依赖短板评估)
- [三、自研纯-rust-协议替代演进路线图](#三自研纯-rust-协议替代演进路线图)
  - [阶段一：纯 Rust 密码学与密钥对生成（脱离 ssh-keygen）](#阶段一纯-rust-密码学与密钥对生成脱离-ssh-keygen)
  - [阶段二：标准 RFC 二进制 SFTP 协议与长连接多路复用（脱离 scp 与 shell 伪实现）](#阶段二标准-rfc-二进制-sftp-协议与长连接多路复用脱离-scp-与-shell-伪实现)
  - [阶段三：原生代理穿透与常驻 TCP 隧道 / SOCKS5 服务（脱离 connect/nc）](#阶段三原生代理穿透与常驻-tcp-隧道--socks5-服务脱离-connectnc)
  - [阶段四：纯 Rust PTY 交互式通道与 CLI 终端适配](#阶段四纯-rust-pty-交互式通道与-cli-终端适配)
- [四、双轨制驱动策略 (Dual-Engine Architecture)](#四双轨制驱动策略-dual-engine-architecture)
- [五、UI 伴生缺陷清理备忘 (UI Polish Notes)](#五ui-伴生缺陷清理备忘-ui-polish-notes)
- [六、工程规范与验收红线](#六工程规范与验收红线)

---

## 一、设计哲学与架构原则

1. **零宿主外部依赖 (Zero Host Dependency)**：
   - 杜绝隐式假设宿主环境存在 `ssh`、`scp`、`ssh-keygen`、`powershell`、`connect` 或 `nc`；
   - 编译产物为单一静态自包含二进制，用户在任何纯净精简系统（Windows LTSC、Alpine Linux、极简容器）下载即可直接运行。
2. **纯内存级安全 (True In-Memory Security)**：
   - 私钥明文、口令与敏感会话数据驻留在受保护内存容器（`zeroize::Zeroizing`），用后立即物理抹零；
   - 彻底废除向物理磁盘写入临时 `.pem` 私钥文件或 `.cmd`/`.sh` 密码应答脚本的传统做法，做到真正物理磁盘 0 落地。
3. **连接多路复用 (Connection Multiplexing)**：
   - 单台主机维持单一物理 TCP/SSH 会话长连接，内部按需动态开辟 PTY Channel、SFTP Channel、Exec Channel、Direct-TCPIP Tunnel Channel，杜绝频繁 fork 进程导致的 CPU 抖动与握手延迟。
4. **协议级严谨性 (Protocol-Level Fidelity)**：
   - 远程文件传输严格采用标准二进制 SFTP 子系统（RFC 4716 / Draft-ietf-secsh-filexfer），全面支持企业受限无交互 Shell 账户（`internal-sftp` / `/sbin/nologin`）。

---

## 二、现状基线与外部依赖短板评估

### 1. 已建立的独立化基线 (Current Baseline)
- ✅ **无 UI 依赖**：不依赖 Slint 框架，类型系统完全纯净（`SshLaunchConfig`, `LinuxSystemMetrics`, `ImportedHostEntry` 等）；
- ✅ **配置与导入**：支持 OpenSSH `~/.ssh/config`、Termius (JSON/CSV/TSV)、Xshell (`.xsh`) 资产导入与自动分组解析；
- ✅ **指标计算**：包含 Linux CPU Jiffies、物理内存与 Swap、网络瞬时速率、以及 30 秒平滑 SVG 折线波形生成器。

### 2. 当前存在的外部工具依赖短板 (External Dependencies to Eliminate)

| 模块 | 当前实现方式 | 隐患与故障场景 | 改造目标 |
| :--- | :--- | :--- | :--- |
| **SFTP 远程操作** | 外部 `scp` + 远端执行 `ls -la/mkdir/touch/rm/mv` | 遇到无 Shell 权限的纯 SFTP 账户直接报错；遇到 Windows 目标机缺少 Linux 命令；大文件名带空格或引号解析易错 | 纯 Rust `russh-sftp` 二进制协议 |
| **网络代理穿透** | Windows 调用 `connect.exe`；Linux 调用 `nc -X 5` | `connect.exe` 仅在 Git Bash 存在；`nc -X 5` 仅限 OpenBSD netcat，标准环境直接崩溃 | 纯 Rust `tokio-socks` 原生握手 |
| **密钥对生成** | 外部子进程 `ssh-keygen` | 纯净 Windows 或精简容器未安装 OpenSSH 时直接报“找不到文件” | 纯 Rust `ed25519-dalek` + `rsa` |
| **密码应答管道** | 磁盘临时脚本 `askpass_*.cmd / .sh` | 依赖临时目录可写与脚本执行权限，物理磁盘发生 I/O 落地 | 纯 Rust 内存发送 `SSH_MSG_USERAUTH_REQUEST` |
| **SSH 交互终端** | 外部子进程 `ssh.exe` | 依赖系统 PATH 安装并支持当前 SSH 选项 | 纯 Rust `russh` 会话与 Channel 泵送 |

---

## 三、自研纯 Rust 协议替代演进路线图

### 阶段一：纯 Rust 密码学与密钥对生成（脱离 ssh-keygen）
- **目标**：彻底淘汰 `Command::new("ssh-keygen")`，实现毫秒级内存安全随机密钥生成与指纹提取。
- **技术实现**：
  1. 引入依赖：`ed25519-dalek`、`rsa`、`zeroize`、`sha2`、`base64`；
  2. 重构 [`src/keygen.rs`](file:///F:/code/rust/smalux-ssh/crates/smagical-ssh/src/keygen.rs)：
     - **Ed25519**：使用安全随机数源 (`rand::rngs::OsRng`) 现场生成私钥，直接格式化为标准 OpenSSH PEM (`-----BEGIN OPENSSH PRIVATE KEY-----`)；
     - **RSA (2048/4096)**：基于纯 Rust 实现生成 RSA 密钥对并编码为 PKCS#1 / PKCS#8 PEM；
     - **公钥与指纹**：直接在内存中计算 OpenSSH 单行公钥文本（`ssh-ed25519 AAAAC3... comment`）与 SHA256 指纹（`SHA256:xxxx`），全程无外部进程、无磁盘临时文件。

---

### 阶段二：标准 RFC 二进制 SFTP 协议与长连接多路复用（脱离 scp 与 shell 伪实现）
- **目标**：以真正的 SFTP 二进制协议取代当前的外部命令，彻底支持无 Shell 权限（`/sbin/nologin`）与 Windows 远端主机。
- **技术实现**：
  1. 引入依赖：`russh`、`russh-keys`、`russh-sftp`；
  2. 重构 [`src/sftp.rs`](file:///F:/code/rust/smalux-ssh/crates/smagical-ssh/src/sftp.rs)：
     - **底层会话接入**：建立 `russh::client::Handle` 并请求打开 `sftp` 子系统 Channel，包装为 `russh_sftp::client::SftpSession`；
     - **目录树递归遍历**：直接调用 `session.read_dir(path)`，返回强类型文件元数据（权限位、大小、修改时间、符号链接），彻底消除文本解析脆弱性；
     - **流式上传与下载**：基于 `tokio::io::copy` 实现分块流水线读写，天然支持精确传输字节计数与进度回调；
     - **原生远程操作**：调用 `session.create_dir`、`session.create`、`session.remove_file`、`session.remove_dir`、`session.rename`；
  3. 重构 [`src/monitor.rs`](file:///F:/code/rust/smalux-ssh/crates/smagical-ssh/src/monitor.rs)：
     - 复用已建立的 SSH 会话句柄，开辟轻量 `Channel::exec` 执行探针指令，将采样开销从 150ms 降至 5ms 以内。

---

### 阶段三：原生代理穿透与常驻 TCP 隧道 / SOCKS5 服务（脱离 connect/nc）
- **目标**：无需系统安装任何第三方网络工具，全平台原生支持 SOCKS5/HTTP 代理与 TCP 端口转发。
- **技术实现**：
  1. 引入依赖：`tokio-socks`；
  2. 纯 Rust 代理连接器：
     - 在发起 SSH 握手前，若配置了 SOCKS5 代理，直接通过 `tokio_socks::tcp::Socks5Stream::connect` 握手代理服务器；
     - 若配置了 HTTP 代理，执行标准的 `CONNECT host:port HTTP/1.1` 握手；
     - 将建立好的流式套接字直接交由 `russh` 执行 SSH 协议握手，完全抛弃 `ProxyCommand=connect -S` 与 `nc -X 5`；
  3. 新增 `crates/smagical-ssh/src/tunnel.rs`：
     - **本地转发 (Local Forward, `-L`)**：本地绑定 `TcpListener`，有新连接时请求 `session.channel_open_direct_tcpip(remote_host, remote_port)` 并双向泵送流量；
     - **远端转发 (Remote Forward, `-R`)**：向远端请求 `session.tcpip_forward(bind_address, port)`；
     - **动态代理 (Dynamic SOCKS5, `-D`)**：本地启动轻量 SOCKS5 服务端，将应用请求动态映射为目标 SSH 端的直接出网流量；
     - **守护看门狗 (`TunnelSupervisor`)**：监控心跳与断线自动重连，暴露启停控制与实时吞吐量统计指标。

---

### 阶段四：纯 Rust PTY 交互式通道与 CLI 终端适配
- **目标**：彻底摆脱对宿主系统 `ssh.exe` 进程的依赖，实现真正自包含的终端流。
- **技术实现**：
  1. 会话交互通道：
     - 通过 `session.channel_open_session()` 开辟交互通道；
     - 请求远端伪终端：`channel.request_pty(true, "xterm-256color", cols, rows, ...)`；
     - 请求启动交互 Shell：`channel.request_shell(true)`；
  2. 适配双向流：
     - **UI 模式**：将 Channel 的输出字节直接推送到 `TerminalParser`（Alacritty/VT100 光栅化渲染器）；
     - **CLI 模式**：直接通过 `tokio::io::copy` 将 Channel 输入输出与标准输入输出 (`stdin`/`stdout`) 对接，支持原生 RAW 模式与终端快捷键直通。

---

## 四、双轨制驱动策略 (Dual-Engine Architecture)

为了在追求 100% 纯 Rust 独立化的同时保持绝对的向下兼容，设计**双轨驱动切换机制**：

```rust
/// SSH 底层协议驱动引擎选择
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SshDriverEngine {
    /// 【默认推荐】纯 Rust 嵌入式自研驱动 (russh + russh-sftp)
    /// 优势：零外部依赖、物理磁盘零残留、单连接多路复用、超低延迟
    #[default]
    PureRust,

    /// 【备用兼容】平台原生 OpenSSH 外部可执行进程 (ssh / scp)
    /// 适用：特定企业内网环境、硬件智能卡 (YubiKey / PKCS#11) 或特殊 Kerberos GSSAPI 认证
    SystemOpenSsh,
}
```

---

## 五、UI 伴生缺陷清理备忘 (UI Polish Notes)

在深度检测过程中发现，`crates/smagical-ui` 中存在部分遗留实现，应在后续顺手清理：
1. **文件对话框统一化**：
   - 现存问题：`settings_handlers/utils.rs` 与 `theme_handlers.rs` 中多处调用 `Command::new("powershell")` 跑 WinForms 弹窗，导致 Linux/macOS 失效、Windows 卡顿且存在脚本拦截风险；
   - 改造动作：统一替换为已经在 `Cargo.toml` 中引入的跨平台原生文件对话框库 **`rfd::FileDialog`**（毫秒级原生 COM / Portal 弹窗）。
2. **开机自启系统调用优化**：
   - 将 `window_handlers.rs` 中的 `reg.exe` 外部进程调用，替换为直接调用已引入的 `windows-sys` 的原生 Win32 Registry API。

---

---

## 🔗 对标 smagical-core 契约体系的实现映射 (Core Trait Implementation Mapping)

`smagical-ssh` 作为官方推荐的**纯 Rust 原生协议驱动适配器**，将严格实现 [`smagical-core`](../smagical-core/README.md) 中声明的 5 大核心服务 Trait：

| Core 服务契约 (Trait) | `smagical-ssh` 对应实现类 | 核心技术选型与实现机制 |
| :--- | :--- | :--- |
| **`SftpService`** | `RusshSftpDriver` | 基于 `russh-sftp` 二进制协议通道，直接读写远端文件系统，支持流式分块进度回调 |
| **`SshSessionService`** | `RusshSessionDriver` | 基于 `russh::client::Handle`，提供鉴权、PTY Channel、远程单次 Exec 与连续双向读写流 |
| **`TunnelService`** | `RusshTunnelDriver` | 基于 `russh` 的 Direct-TCPIP Channel 与 `tokio::net::TcpListener` 流量双向转发泵 |
| **`KeygenService`** | `NativeKeygenService` | 基于 `ed25519-dalek` 与 `rsa`，纯内存生成 PEM 与 OpenSSH 格式密钥对，零进程调用 |
| **`HostMetricsService`** | `RusshMetricsDriver` | 复用已建立的长连接 SSH Session，执行轻量探针指令解析 Linux Jiffies/内存/网络 |

通过这套契约实现映射，任何上层消费者（Slint UI 桌面客户端、CLI 命令行工具）均只需依赖 `smagical-core` 的 Trait 接口，底层驱动可以在编译期或运行期无缝替换为 Mock 仿真驱动或第三方驱动。

---

## 六、工程规范与验收红线

在推进上述自研协议实现时，必须严格遵守以下原则：
- 🟢 **代码零缺陷**：构建必须保持 `0 errors, 0 warnings`（`#![deny(missing_docs)]` 必须严格遵守，所有公开项必须具备详细 Rustdoc）；
- 🟢 **全量自动化测试**：每个协议子模块均需配备单元测试与 Mock 测试，工作区测试全绿通过；
- 🟢 **版本控制红线**：严禁私自执行 `git commit` 或 `git push`，所有改动经由用户确认后进行。

