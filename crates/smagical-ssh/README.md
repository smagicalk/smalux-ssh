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

## 二、现状基线与纯 Rust 驱动实现全景

系统已彻底完成从“外部命令行调用”向“自研纯 Rust 异步协议栈”的全面跃迁，生产级驱动已全量落地并网：

| 模块 | 纯 Rust 驱动实现 | 核心技术选型 | 现状状态 |
| :--- | :--- | :--- | :--- |
| **SSH 会话与交互终端** | `RusshSessionDriver` | `russh` 异步协议栈，直接开辟 PTY Channel 与 Exec Channel，支持双向流 | 🟢 **100% 落地生产** |
| **SFTP 远程操作** | `RusshSftpDriver` | `russh-sftp` 二进制通道，递归遍历目录、流式上传下载、断点统计 | 🟢 **100% 落地生产** |
| **网络隧道与端口转发** | `RusshTunnelDriver` | `Direct-TCPIP` 异步管道与 `tokio::net::TcpListener` 双向流量泵 | 🟢 **100% 落地生产** |
| **密钥对现场生成** | `NativeKeygenService` | `ed25519-dalek` + `rsa` + `ssh-key`，纯内存生成与指纹提取，零临时文件 | 🟢 **100% 落地生产** |
| **主机公钥验真 (TOFU)** | `known_hosts.rs` | 纯 Rust 解析、追加与校验 `~/.ssh/known_hosts`，抵御中间人攻击 (MITM) | 🟢 **100% 落地生产** |
| **系统监控指标采集** | `RusshMetricsDriver` | 复用长连接开辟单次轻量 Exec，极速解析 Linux Jiffies/内存/网络 | 🟢 **100% 落地生产** |
| **资产导入无损解析** | `importer.rs` | 纯 Rust 解析 OpenSSH config、Termius、Xshell (.xsh) 与 CSV/TSV | 🟢 **100% 落地生产** |

---

## 三、自研纯 Rust 协议驱动核心技术实现

### 阶段一：纯 Rust 密码学与密钥对生成（脱离 ssh-keygen）
- **实现模块**：[`src/keygen.rs`](file:///F:/code/rust/smalux-ssh/crates/smagical-ssh/src/keygen.rs) (`NativeKeygenService`)；
- **核心成果**：
  1. **Ed25519**：使用安全随机数源现场生成私钥，直接格式化为标准 OpenSSH PEM (`-----BEGIN OPENSSH PRIVATE KEY-----`)；
  2. **RSA (2048/4096)**：基于纯 Rust 实现生成 RSA 密钥对并编码为 PKCS#1 / PKCS#8 PEM；
  3. **公钥与指纹**：直接在内存中计算 OpenSSH 单行公钥文本（`ssh-ed25519 AAAAC3... comment`）与 SHA256 指纹（`SHA256:xxxx`），全程无外部进程、无磁盘临时文件。

---

### 阶段二：标准 RFC 二进制 SFTP 协议与长连接多路复用（脱离 scp 与 shell 伪实现）
- **实现模块**：[`src/sftp_driver.rs`](file:///F:/code/rust/smalux-ssh/crates/smagical-ssh/src/sftp_driver.rs) (`RusshSftpDriver`)；
- **核心成果**：
  1. **底层会话接入**：基于 `russh::client::Handle` 打开 `sftp` 子系统 Channel，包装为 `russh_sftp::client::SftpSession`；
  2. **目录树递归遍历**：直接调用 `session.read_dir(path)`，返回强类型文件元数据（权限位、大小、修改时间、符号链接），彻底消除文本解析脆弱性；
  3. **流式上传与下载**：基于 `tokio::io::copy` 实现分块流水线读写，天然支持精确传输字节计数与进度回调；
  4. **原生远程操作**：调用 `session.create_dir`、`session.create`、`session.remove_file`、`session.remove_dir`、`session.rename`。

---

### 阶段三：原生代理穿透与常驻 TCP 隧道 / SOCKS5 服务（脱离 connect/nc）
- **实现模块**：[`src/tunnel_driver.rs`](file:///F:/code/rust/smalux-ssh/crates/smagical-ssh/src/tunnel_driver.rs) (`RusshTunnelDriver`)；
- **核心成果**：
  1. **本地转发 (Local Forward, `-L`)**：本地绑定 `TcpListener`，有新连接时请求 `session.channel_open_direct_tcpip(remote_host, remote_port)` 并双向泵送流量；
  2. **远端转发 (Remote Forward, `-R`)**：向远端请求 `session.tcpip_forward(bind_address, port)`；
  3. **动态代理 (Dynamic SOCKS5, `-D`)**：本地启动轻量 SOCKS5 服务端，将应用请求动态映射为目标 SSH 端的直接出网流量。

---

### 阶段四：纯 Rust PTY 交互式通道与 CLI/GUI 终端双向适配
- **实现模块**：[`src/session_driver.rs`](file:///F:/code/rust/smalux-ssh/crates/smagical-ssh/src/session_driver.rs) (`RusshSessionDriver`)；
- **核心成果**：
  1. 会话交互通道：通过 `session.channel_open_session()` 开辟交互通道，请求远端伪终端与交互 Shell；
  2. 适配双向流：
     - **UI 模式 (`smagical-ui`)**：将 Channel 的输出字节直接推送到 `TerminalParser`（Alacritty/VT100 光栅化渲染器）；
     - **CLI 模式 (`smalux-cli`)**：直接将 Channel 输入输出与标准输入输出 (`stdin`/`stdout`) 对接，支持原生 RAW 模式与终端快捷键直通。

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

## 七、 核心 API 与调用方法 (Core APIs & Signatures)

### 1. SSH 会话驱动 (`RusshSessionDriver`)
```rust
let driver = RusshSessionDriver::new();

// 建立纯 Rust 异步 SSH 会话 (支持 15s 心跳保活与超时防护)
let session_id = driver.connect(&host, cred.as_ref()).await?;

// 打开交互式 PTY 双向异步数据流
let (mut pty_writer, mut pty_reader) = driver.open_pty_channel(&session_id, 80, 24).await?;

// 单次非交互执行命令并获取输出
let output = driver.execute_command(&session_id, "uptime && free -m").await?;
println!("退出码: {}, 标准输出: {}", output.exit_code, String::from_utf8_lossy(&output.stdout));

// 优雅关闭会话
driver.disconnect(&session_id).await?;
```

### 2. SFTP 文件传输驱动 (`RusshSftpDriver`)
```rust
let sftp = RusshSftpDriver::new();

// 连接远端 SFTP 子系统
sftp.connect_raw(&host, cred.as_ref()).await?;

// 遍历目录
let entries = sftp.list_dir("/var/log").await?;
for e in entries {
    println!("文件: {} (大小: {} 字节, 是否目录: {})", e.name, e.size, e.is_dir);
}
```

### 3. 网络隧道驱动 (`RusshTunnelDriver`)
```rust
let tunnel_driver = RusshTunnelDriver::new();

// 开启本地端口转发或 SOCKS5 代理
let handle = tunnel_driver.start_tunnel(&tunnel_record, cred.as_ref()).await?;
println!("隧道已就绪，本地监听地址: {}", handle.bound_address);

// 获取实时双向流量度量
let metrics = tunnel_driver.poll_metrics().await;

// 停止隧道并释放本地端口
tunnel_driver.stop_tunnel(&tunnel_record.id).await?;
```

### 4. 纯内存密钥生成器 (`NativeKeygenService`)
```rust
let keygen = NativeKeygenService::new();

// 现场生成 Ed25519 密钥对 (零外部进程拉起)
let pair = keygen.generate_keypair(KeyAlgorithm::Ed25519, None, None)?;
println!("公钥指纹: {}", pair.fingerprint);
println!("OpenSSH 公钥: {}", pair.public_key_openssh);
println!("PEM 私钥: {}", pair.private_key_pem);
```

### 5. RAII 私钥文件守卫 (`KeyTempGuard`)
```rust
// 创建临时私钥并在析构 (Drop) 时自动执行等长 0 字节抹零覆盖并删除物理文件
let guard = KeyTempGuard::create("sess_1001", &pem_content)?;
println!("临时私钥路径: {:?}", guard.path());
// 当 guard 离开作用域时自动 Zeroize 擦除
```

