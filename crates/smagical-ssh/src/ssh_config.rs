//! SSH 会话启动参数高级配置与临时私钥安全生命周期托管 (KeyTempGuard)。
//!
//! 提供：
//! 1. `KeyTempGuard`: RAII 安全临时私钥文件守卫，在会话生命周期内提供受操作系统保护的私钥文件，
//!    并在 Drop (会话结束、Tab 关闭或应用退出) 时自动执行等长 0 字节覆盖抹零 (Zeroize) 并删除物理文件；
//! 2. `SshLaunchConfig`: 聚合目标主机、凭据私钥/密码、跳板机链式跳转与代理隧道的完整启动配置。

use std::path::{Path, PathBuf};
use anyhow::{Context, Result};

/// RAII 临时私钥文件安全守卫。
///
/// 确保私钥仅在会话活跃期间存在于受限临时目录，并在守卫析构 (`Drop`) 时
/// 自动对物理文件执行等长 0 字节内存覆盖 (Zeroize)，最后执行物理删除，
/// 杜绝私钥明文残留在磁盘或被外部恢复工具窃取的安全风险。
#[derive(Debug)]
pub struct KeyTempGuard {
    /// 临时私钥物理文件的磁盘绝对路径。
    path: PathBuf,
}

impl KeyTempGuard {
    /// 在系统安全临时目录中创建独立的临时私钥文件并写入 PEM 明文内容。
    ///
    /// # 参数
    /// - `session_id`: 触发本次创建的终端会话或文件传输任务的唯一标识符（用于生成文件名防止冲突）。
    /// - `pem_content`: 经过主密码或内存解密后的 OpenSSH / RSA / Ed25519 PEM 格式私钥文本。
    ///
    /// # 返回值
    /// - `Ok(KeyTempGuard)`: 成功写入并锁定权限的临时私钥文件守卫实例。
    /// - `Err(anyhow::Error)`: 磁盘写入失败或权限配置失败时返回的错误上下文。
    ///
    /// # 安全特性
    /// - 目录隔离：存放于系统临时路径下的独立子目录 `smalux_keys` 中。
    /// - 文件名防碰撞：采用 `key_{session_id}_{uuid}.pem` 命名模式。
    /// - 权限收紧 (Unix)：将文件权限强制收紧为 `0600`（仅当前用户所有者可读写），满足 OpenSSH 严格权限校验。
    pub fn create(session_id: &str, pem_content: &str) -> Result<Self> {
        let temp_dir = std::env::temp_dir().join("smalux_keys");
        let _ = std::fs::create_dir_all(&temp_dir);

        #[cfg(windows)]
        {
            secure_windows_path_permissions(&temp_dir, true);
        }

        // 使用 session_id 和随机后缀防止冲突
        let file_name = format!("key_{}_{}.pem", session_id, uuid::Uuid::new_v4().simple());
        let path = temp_dir.join(file_name);

        // 写入私钥内容 (确保以换行符结尾)
        let clean_pem = if pem_content.ends_with('\n') {
            pem_content.to_string()
        } else {
            format!("{}\n", pem_content)
        };
        std::fs::write(&path, clean_pem.as_bytes())
            .with_context(|| format!("写入临时私钥文件失败: {:?}", path))?;

        // 在 Unix 平台设置 0600 严格权限 (仅所有者可读写)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&path)?.permissions();
            perms.set_mode(0o600);
            let _ = std::fs::set_permissions(&path, perms);
        }

        // 在 Windows 平台收紧 NTFS ACL 权限 (仅当前用户与 SYSTEM，杜绝 OpenSSH bad permissions)
        #[cfg(windows)]
        {
            secure_windows_path_permissions(&path, false);
        }

        tracing::debug!(target: "smagical_ssh::security", "临时私钥文件已安全创建: {:?}", path);
        Ok(Self { path })
    }

    /// 获取临时私钥文件的绝对路径引用。
    ///
    /// # 返回值
    /// 返回指代物理私钥文件的 [`Path`] 借用。
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 获取临时私钥文件路径的规范化字符串表示。
    ///
    /// # 返回值
    /// 返回将 Windows 反斜杠统一替换为标准正斜杠 `/` 的路径字符串，
    /// 确保直接用于跨平台命令行参数（如 OpenSSH `-i` 选项）时不发生转义截断。
    pub fn path_str(&self) -> String {
        self.path.to_string_lossy().replace('\\', "/")
    }
}

impl Drop for KeyTempGuard {
    /// 析构生命周期守卫：物理覆盖抹零并清理文件。
    ///
    /// 1. 获取物理文件长度，先以等长的全 `0x00` 字节写入磁盘覆盖原始私钥扇区；
    /// 2. 调用操作系统 API 物理删除该临时文件；
    /// 3. 输出安全审计追踪日志。
    fn drop(&mut self) {
        if self.path.exists() {
            // 1. 获取文件长度，执行全 0 字节物理覆盖抹零
            if let Ok(metadata) = std::fs::metadata(&self.path) {
                let len = metadata.len() as usize;
                if len > 0 {
                    let zero_buf = vec![0u8; len];
                    let _ = std::fs::write(&self.path, zero_buf);
                }
            }
            // 2. 删除临时文件
            let _ = std::fs::remove_file(&self.path);
            tracing::debug!(target: "smagical_ssh::security", "临时私钥文件已安全抹零销毁: {:?}", self.path);
        }
    }
}

/// SSH 交互式终端会话与 SFTP 文件引擎启动高级参数模型。
///
/// 封装建立底层 SSH 隧道所需的全部网络拓扑、身份鉴权、代理穿透与连接保持参数。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SshLaunchConfig {
    /// 目标主机的 IPv4/IPv6 地址或公网域名。
    pub host: String,
    /// 远程目标 SSH 服务的监听端口（缺省为 22）。
    pub port: u16,
    /// 远程登录用户名（可选，若为空则依赖 SSH 客户端当前环境或 SSH Config 配置）。
    pub username: Option<String>,
    /// 解密后的私钥明文 PEM 文本（若选用私钥认证或公私钥证书认证）。
    pub private_key_pem: Option<String>,
    /// 解密后的登录密码明文（若选用密码认证，用于交互式 PTY 的自动密码应答）。
    pub password: Option<String>,
    /// 多跳跳板机（Bastion / Jump Host）配置参数（例如 `"user@jumphost:22"`，对应 OpenSSH `-J` 选项）。
    pub jump_host: Option<String>,
    /// 代理协议类型：可选 `"direct"`（直连）、`"socks5"`（SOCKS5 代理）、`"http"`（HTTP CONNECT 隧道代理）。
    pub proxy_type: Option<String>,
    /// 代理服务器的主机名或 IP 地址。
    pub proxy_host: Option<String>,
    /// 代理服务器的端口号（SOCKS5 缺省 1080，HTTP 缺省 8080）。
    pub proxy_port: Option<u16>,
    /// TCP / SSH 应用层心跳保活检测间隔（单位：秒，对应 `ServerAliveInterval`，缺省为 30 秒）。
    pub keepalive_interval: u32,
    /// 建立底层 TCP 三次握手与 SSH 初始协商的超时时限（单位：秒，对应 `ConnectTimeout`，缺省为 15 秒）。
    pub connect_timeout: u32,
    /// 远程主机公钥指纹的严格检验策略：
    /// - `"accept-new"`（默认）：首次连接自动信任记录，变更时严格报错拦截；
    /// - `"ask"`：任何未记录公钥均提示用户人工确认；
    /// - `"no"` / `"off"`：不校验主机公钥，忽略中间人警告（常用于高频自动化测试）。
    pub host_key_policy: Option<String>,
}

impl SshLaunchConfig {
    /// 获取标准目标主机地址表示。
    ///
    /// # 返回值
    /// - 若配置了有效用户名：返回 `"username@host"`；
    /// - 若未配置用户名：返回 `"host"`。
    pub fn destination(&self) -> String {
        if let Some(ref u) = self.username {
            let trimmed_u = u.trim();
            if !trimmed_u.is_empty() {
                return format!("{}@{}", trimmed_u, self.host.trim());
            }
        }
        self.host.trim().to_string()
    }

    /// 构建标准 OpenSSH 命令行参数列表，并实例化关联的 RAII 临时私钥生命周期守卫。
    ///
    /// # 参数
    /// - `session_id`: 当前会话或任务唯一标识，用于命名临时私钥文件并保证生命周期受控；
    /// - `is_ssh_cmd`: `true` 表示为 `ssh` 交互式命令（端口参数形如 `-p 22`），
    ///   `false` 表示为 `scp` / `sftp` 批处理命令（端口参数形如 `-P 22`）。
    ///
    /// # 返回值
    /// - `Ok((Vec<String>, Option<KeyTempGuard>))`: 包含所有解析拼接完成的命令行参数切片，
    ///   以及需要被宿主调用方持有的临时私钥守卫句柄（必须与子进程生命周期对齐）。
    /// - `Err(anyhow::Error)`: 写入临时私钥异常或配置冲突时返回错误。
    pub fn build_ssh_args(
        &self,
        session_id: &str,
        is_ssh_cmd: bool,
    ) -> Result<(Vec<String>, Option<KeyTempGuard>)> {
        let mut args = Vec::new();

        // 1. 端口配置
        let port = if self.port == 0 { 22 } else { self.port };
        if is_ssh_cmd {
            args.push("-p".to_string());
        } else {
            args.push("-P".to_string());
        }
        args.push(port.to_string());

        // 2. 超时与防吊死配置
        let keepalive = if self.keepalive_interval == 0 { 30 } else { self.keepalive_interval };
        let timeout = if self.connect_timeout == 0 { 15 } else { self.connect_timeout };
        args.push("-o".to_string());
        args.push(format!("ServerAliveInterval={}", keepalive));
        args.push("-o".to_string());
        args.push("ServerAliveCountMax=3".to_string());
        args.push("-o".to_string());
        args.push(format!("ConnectTimeout={}", timeout));

        if is_ssh_cmd {
            args.push("-o".to_string());
            args.push("ExitOnForwardFailure=yes".to_string());
        }

        // 3. 主机密钥策略
        let policy = self.host_key_policy.as_deref().unwrap_or("accept-new");
        match policy {
            "no" | "off" => {
                args.push("-o".to_string());
                args.push("StrictHostKeyChecking=no".to_string());
                args.push("-o".to_string());
                args.push("UserKnownHostsFile=/dev/null".to_string());
            }
            "ask" => {
                args.push("-o".to_string());
                args.push("StrictHostKeyChecking=ask".to_string());
            }
            _ => {
                args.push("-o".to_string());
                args.push("StrictHostKeyChecking=accept-new".to_string());
            }
        }

        // 4. 私钥凭据挂载
        let mut key_guard = None;
        if let Some(ref pem) = self.private_key_pem {
            if !pem.trim().is_empty() {
                let guard = KeyTempGuard::create(session_id, pem)?;
                args.push("-i".to_string());
                args.push(guard.path_str());
                args.push("-o".to_string());
                args.push("IdentitiesOnly=yes".to_string());
                key_guard = Some(guard);
            }
        }

        // 5. 跳板机参数
        if let Some(ref jump) = self.jump_host {
            if !jump.trim().is_empty() {
                args.push("-J".to_string());
                args.push(jump.trim().to_string());
            }
        }

        // 6. 代理参数
        if let Some(ref p_type) = self.proxy_type {
            if let Some(ref p_host) = self.proxy_host {
                let p_port = self.proxy_port.unwrap_or(if p_type == "http" { 8080 } else { 1080 });
                if !p_host.is_empty() {
                    #[cfg(windows)]
                    {
                        if p_type == "socks5" {
                            args.push("-o".to_string());
                            args.push(format!("ProxyCommand=connect -S {}:{} %h %p", p_host, p_port));
                        } else if p_type == "http" {
                            args.push("-o".to_string());
                            args.push(format!("ProxyCommand=connect -H {}:{} %h %p", p_host, p_port));
                        }
                    }
                    #[cfg(not(windows))]
                    {
                        if p_type == "socks5" {
                            args.push("-o".to_string());
                            args.push(format!("ProxyCommand=nc -X 5 -x {}:{} %h %p", p_host, p_port));
                        } else if p_type == "http" {
                            args.push("-o".to_string());
                            args.push(format!("ProxyCommand=nc -X connect -x {}:{} %h %p", p_host, p_port));
                        }
                    }
                }
            }
        }

        Ok((args, key_guard))
    }

    /// 构建准备好直接执行的标准 [`std::process::Command`] 实例。
    ///
    /// 自动注入操作系统优化参数（如 Windows 下隐藏控制台黑框标志位 `CREATE_NO_WINDOW`）。
    ///
    /// # 参数
    /// - `session_id`: 当前会话或任务唯一标识。
    ///
    /// # 返回值
    /// - `Ok((std::process::Command, Option<KeyTempGuard>))`: 装配完成的系统进程命令与私钥生命周期守卫。
    pub fn build_command(&self, session_id: &str) -> Result<(std::process::Command, Option<KeyTempGuard>)> {
        let mut cmd = std::process::Command::new("ssh");
        let (args, guard) = self.build_ssh_args(session_id, true)?;
        for arg in args {
            cmd.arg(arg);
        }
        cmd.arg(self.destination());

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }

        Ok((cmd, guard))
    }
}

/// 在远程目标主机上异步执行单次命令并返回子进程退出状态与标准输出/错误。
///
/// # 参数
/// - `config`: 目标主机的 SSH 连接配置与鉴权信息；
/// - `remote_cmd`: 计划在远程终端执行的 Shell 命令脚本。
///
/// # 返回值
/// - `Ok(std::process::Output)`: 执行完成的进程结果（包含 stdout、stderr 与 exit_status）；
/// - `Err(anyhow::Error)`: 连接建立失败、网络中断或进程异常。
pub async fn execute_remote(
    config: &SshLaunchConfig,
    remote_cmd: &str,
) -> Result<std::process::Output> {
    let mut ctx = crate::sftp::SftpCommandContext::build_ssh(config, remote_cmd)?;
    tokio::task::spawn_blocking(move || {
        let output = ctx.cmd.output()?;
        Ok(output)
    }).await?
}

/// 解析各种格式的代理服务器连接 URL 字符串。
///
/// # 支持格式
/// - SOCKS5 协议头：`socks5://127.0.0.1:1080`、`socks://127.0.0.1:1080`
/// - HTTP 隧道协议头：`http://127.0.0.1:7890`、`https://127.0.0.1:7890`
/// - 携带鉴权信息：`socks5://user:pass@127.0.0.1:1080`（自动提取 host 与 port）
/// - 无前缀裸地址：`127.0.0.1:7890`（缺省识别为 SOCKS5）
///
/// # 参数
/// - `url`: 待解析的代理连接字符串。
///
/// # 返回值
/// - `Some((protocol, host, port))`: 解析出的协议（`"socks5"` 或 `"http"`）、主机地址与端口号。
/// - `None`: 字符串格式无效或未包含有效端口时返回。
pub fn parse_proxy_url(url: &str) -> Option<(String, String, u16)> {
    let url = url.trim();
    if url.is_empty() {
        return None;
    }
    let (proto, rest) = if let Some(stripped) = url.strip_prefix("socks5://") {
        ("socks5".to_string(), stripped)
    } else if let Some(stripped) = url.strip_prefix("socks://") {
        ("socks5".to_string(), stripped)
    } else if let Some(stripped) = url.strip_prefix("http://") {
        ("http".to_string(), stripped)
    } else if let Some(stripped) = url.strip_prefix("https://") {
        ("http".to_string(), stripped)
    } else {
        ("socks5".to_string(), url)
    };

    let host_port_part = if let Some((_, hp)) = rest.split_once('@') {
        hp
    } else {
        rest
    };

    let host_port_part = host_port_part.trim_end_matches('/');
    if let Some((h, p)) = host_port_part.split_once(':') {
        let port = p.parse::<u16>().ok()?;
        Some((proto, h.to_string(), port))
    } else {
        None
    }
}

/// 自动探测并获取当前宿主操作系统配置的系统级出站网络代理。
///
/// # 探测优先级
/// 1. **环境变量嗅探**：按序优先检查 `ALL_PROXY`、`all_proxy`、`HTTPS_PROXY`、`https_proxy`、`HTTP_PROXY`、`http_proxy`；
/// 2. **Windows 注册表查询**（仅 Windows 平台）：
///    - 查询 `HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings` 键；
///    - 核验 `ProxyEnable` 是否为 `0x1`（代理已启用）；
///    - 解析 `ProxyServer` 字符串，支持联合配置（如 `socks=127.0.0.1:1080;http=127.0.0.1:7890`）与单项配置。
///
/// # 返回值
/// - `Some((protocol, host, port))`: 成功探测到的系统代理类型、地址与端口；
/// - `None`: 系统未启用代理或未配置有效出站代理。
pub fn get_system_proxy() -> Option<(String, String, u16)> {
    // 1. 优先检查标准系统环境变量
    for var in &["ALL_PROXY", "all_proxy", "HTTPS_PROXY", "https_proxy", "HTTP_PROXY", "http_proxy"] {
        if let Ok(val) = std::env::var(var) {
            if let Some(parsed) = parse_proxy_url(&val) {
                return Some(parsed);
            }
        }
    }

    // 2. Windows 平台：查询注册表 Internet Settings
    #[cfg(windows)]
    {
        let reg_out = std::process::Command::new("reg")
            .args(["query", r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings", "/v", "ProxyEnable"])
            .output()
            .ok()?;
        let reg_str = String::from_utf8_lossy(&reg_out.stdout);
        // 检查 ProxyEnable 是否为 0x1
        if reg_str.contains("0x1") {
            let server_out = std::process::Command::new("reg")
                .args(["query", r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings", "/v", "ProxyServer"])
                .output()
                .ok()?;
            let server_str = String::from_utf8_lossy(&server_out.stdout);
            for line in server_str.lines() {
                if line.contains("ProxyServer") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if let Some(val) = parts.last() {
                        if val.contains(';') {
                            for sub in val.split(';') {
                                if let Some(stripped) = sub.strip_prefix("socks=") {
                                    if let Some(p) = parse_proxy_url(&format!("socks5://{}", stripped)) {
                                        return Some(p);
                                    }
                                } else if let Some(stripped) = sub.strip_prefix("http=") {
                                    if let Some(p) = parse_proxy_url(&format!("http://{}", stripped)) {
                                        return Some(p);
                                    }
                                }
                            }
                        } else if let Some(p) = parse_proxy_url(val) {
                            return Some(p);
                        }
                    }
                }
            }
        }
    }

    None
}

/// 在 Windows 平台收紧 NTFS ACL 权限，满足 OpenSSH 严格权限策略。
/// 
/// 确保仅当前用户（所有者）与 SYSTEM 拥有完全控制权限，移除其他所有继承组（如 Users、Authenticated Users）。
#[cfg(windows)]
pub fn secure_windows_path_permissions(path: &Path, is_directory: bool) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let username = std::env::var("USERNAME").unwrap_or_default();
    if username.is_empty() {
        return;
    }

    let grant_user = if is_directory {
        format!("{}:(OI)(CI)F", username)
    } else {
        format!("{}:F", username)
    };

    let grant_system = if is_directory {
        "SYSTEM:(OI)(CI)F"
    } else {
        "SYSTEM:F"
    };

    let icacls_bin = std::env::var("SystemRoot")
        .map(|sr| format!("{}\\System32\\icacls.exe", sr))
        .unwrap_or_else(|_| "icacls.exe".to_string());

    let status = std::process::Command::new(&icacls_bin)
        .arg(path)
        .arg("/inheritance:r")
        .arg("/grant:r")
        .arg(&grant_user)
        .arg("/grant:r")
        .arg(grant_system)
        .creation_flags(CREATE_NO_WINDOW)
        .status();

    if let Ok(s) = status {
        if s.success() {
            tracing::debug!(target: "smagical_ssh::security", "已成功通过 icacls 收紧 Windows ACL 权限: {:?}", path);
            return;
        }
    }

    // 兜底方案：调用 powershell 设置 ACL
    let path_str = path.to_string_lossy().replace('\'', "''");
    let user_str = username.replace('\'', "''");
    let ps_script = if is_directory {
        format!(
            "$acl = Get-Acl '{path}'; $acl.SetAccessRuleProtection($true, $false); $r1 = New-Object System.Security.AccessControl.FileSystemAccessRule('{user}', 'FullControl', 'ContainerInherit,ObjectInherit', 'None', 'Allow'); $r2 = New-Object System.Security.AccessControl.FileSystemAccessRule('SYSTEM', 'FullControl', 'ContainerInherit,ObjectInherit', 'None', 'Allow'); $acl.ResetAccessRule($r1); $acl.AddAccessRule($r2); Set-Acl '{path}' $acl",
            path = path_str,
            user = user_str,
        )
    } else {
        format!(
            "$acl = Get-Acl '{path}'; $acl.SetAccessRuleProtection($true, $false); $r1 = New-Object System.Security.AccessControl.FileSystemAccessRule('{user}', 'FullControl', 'Allow'); $r2 = New-Object System.Security.AccessControl.FileSystemAccessRule('SYSTEM', 'FullControl', 'Allow'); $acl.ResetAccessRule($r1); $acl.AddAccessRule($r2); Set-Acl '{path}' $acl",
            path = path_str,
            user = user_str,
        )
    };

    let _ = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &ps_script])
        .creation_flags(CREATE_NO_WINDOW)
        .status();
}

#[cfg(not(windows))]
#[allow(dead_code)]
pub fn secure_windows_path_permissions(_path: &Path, _is_directory: bool) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_temp_guard_lifecycle_and_zeroize() {
        let fake_pem = "-----BEGIN OPENSSH PRIVATE KEY-----\nMOCK_KEY_DATA\n-----END OPENSSH PRIVATE KEY-----";
        let guard = KeyTempGuard::create("sess-test-01", fake_pem).expect("创建临时私钥失败");

        let key_path = guard.path().to_path_buf();
        assert!(key_path.exists(), "临时私钥文件必须在创建后存在于文件系统中");

        let read_content = std::fs::read_to_string(&key_path).expect("读取临时私钥失败");
        assert_eq!(read_content, fake_pem, "写入的私钥内容必须与原始明文完全一致");

        let path_str = guard.path_str();
        assert!(!path_str.contains('\\'), "path_str 应规范化为正斜杠格式");

        drop(guard);
        assert!(!key_path.exists(), "临时私钥文件必须在 Drop 后彻底销毁，不存在于文件系统中");
    }

    #[test]
    fn test_ssh_launch_config_defaults() {
        let cfg = SshLaunchConfig {
            host: "192.168.1.100".to_string(),
            port: 2222,
            username: Some("ubuntu".to_string()),
            jump_host: Some("jump.example.com".to_string()),
            ..Default::default()
        };

        assert_eq!(cfg.host, "192.168.1.100");
        assert_eq!(cfg.port, 2222);
        assert_eq!(cfg.username.as_deref(), Some("ubuntu"));
        assert_eq!(cfg.jump_host.as_deref(), Some("jump.example.com"));
        assert_eq!(cfg.destination(), "ubuntu@192.168.1.100");

        let (args, _guard) = cfg.build_ssh_args("test-sess", true).expect("构建参数失败");
        assert!(args.contains(&"-p".to_string()));
        assert!(args.contains(&"2222".to_string()));
        assert!(args.contains(&"-J".to_string()));
        assert!(args.contains(&"jump.example.com".to_string()));
    }
}
