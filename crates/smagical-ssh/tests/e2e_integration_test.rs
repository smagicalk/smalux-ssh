//! 端到端纯 Rust SSH 协议栈原生回环集成测试 (E2E Integration Test)
//!
//! 在独立隔离环境中启动纯 Rust 嵌入式 SSHv2 异步测试服务端，
//! 全面检验 `smagical-ssh` 驱动体系在真实网络回环、真实握手与报文流转下的行为：
//! 1. 真实密码认证通过与错误密码严格拒绝
//! 2. 真实现场 Ed25519 密钥对生成与公钥免密认证
//! 3. 真实交互式 PTY 终端会话建立、双向全双工字节流读写与终端 Echo 验证
//! 4. 真实远程非阻塞命令单次执行 (`exec`) 与退出码/回包捕获
//! 5. 真实 OpenSSH `known_hosts` TOFU（首次信任登记）与中间人公钥篡改拦截警报 (MITM Mismatch)

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use russh::server::{Auth, ChannelOpenHandle, Handler, Msg, Server, Session};
use russh::{Channel, ChannelId};
use ssh_key::PrivateKey;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use smagical_core::domain::{CredentialRecord, CredentialType, HostRecord};
use smagical_core::service::{
    CommandExecutionOutput, KeyAlgorithm, KeygenService, SshServiceError, SshSessionService,
};
use smagical_ssh::{NativeKeygenService, RusshSessionDriver};

/// 嵌入式原生测试 SSH 服务端事件处理器
#[derive(Clone)]
struct E2eTestServerHandler {
    _server_pubkey: ssh_key::PublicKey,
}

impl Server for E2eTestServerHandler {
    type Handler = Self;
    fn new_client(&mut self, _: Option<std::net::SocketAddr>) -> Self {
        self.clone()
    }
}

impl Handler for E2eTestServerHandler {
    type Error = russh::Error;

    async fn auth_password(&mut self, user: &str, password: &str) -> Result<Auth, Self::Error> {
        if user == "smalux_user" && password == "secret_password_123" {
            Ok(Auth::Accept)
        } else {
            Ok(Auth::reject())
        }
    }

    async fn auth_publickey(&mut self, user: &str, _key: &ssh_key::PublicKey) -> Result<Auth, Self::Error> {
        if user == "smalux_user" {
            Ok(Auth::Accept)
        } else {
            Ok(Auth::reject())
        }
    }

    async fn channel_open_session(
        &mut self,
        _channel: Channel<Msg>,
        reply: ChannelOpenHandle,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        reply.accept().await;
        Ok(())
    }

    async fn shell_request(&mut self, channel: ChannelId, session: &mut Session) -> Result<(), Self::Error> {
        let _ = session.channel_success(channel);
        let _ = session.data(channel, b"WELCOME_TO_SMALUX\r\n".to_vec());
        Ok(())
    }

    async fn exec_request(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        let cmd = String::from_utf8_lossy(data);
        let _ = session.channel_success(channel);
        let output = format!("OUTPUT_FOR_{}\n", cmd.trim());
        let _ = session.data(channel, output.into_bytes());
        let _ = session.exit_status_request(channel, 0);
        let _ = session.close(channel);
        Ok(())
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        // 全双工回显：给发送的数据增加前缀 "ECHO:" 并原样发回
        let mut resp = b"ECHO:".to_vec();
        resp.extend_from_slice(data);
        let _ = session.data(channel, resp);
        Ok(())
    }
}

/// 启动嵌入式内存级 SSH 测试服务端
async fn start_embedded_ssh_server() -> (u16, tokio::sync::broadcast::Sender<()>, PathBuf) {
    let server_key = PrivateKey::random(&mut rand::rng(), ssh_key::Algorithm::Ed25519).unwrap();
    let server_pubkey = server_key.public_key().clone();

    let mut config = russh::server::Config::default();
    config.inactivity_timeout = None;
    config.auth_rejection_time = Duration::from_millis(50);
    config.keys.push(server_key);
    let config = Arc::new(config);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let (shutdown_tx, _) = tokio::sync::broadcast::channel::<()>(1);
    let mut shutdown_rx = shutdown_tx.subscribe();

    let mut server = E2eTestServerHandler {
        _server_pubkey: server_pubkey,
    };

    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    break;
                }
                accept_res = listener.accept() => {
                    if let Ok((socket, peer_addr)) = accept_res {
                        let client_handler = server.new_client(Some(peer_addr));
                        let cfg = Arc::clone(&config);
                        tokio::spawn(async move {
                            let _ = russh::server::run_stream(cfg, socket, client_handler).await;
                        });
                    } else {
                        break;
                    }
                }
            }
        }
    });

    // 为该测试实例分配隔离的临时 known_hosts 文件路径
    let temp_known_hosts = std::env::temp_dir().join(format!("test_known_hosts_{}.txt", uuid::Uuid::new_v4()));

    (port, shutdown_tx, temp_known_hosts)
}

fn build_test_host(port: u16) -> HostRecord {
    HostRecord {
        id: format!("e2e-host-{}", port),
        name: "E2E Test Host".to_string(),
        address: "127.0.0.1".to_string(),
        port,
        ..Default::default()
    }
}

// -----------------------------------------------------------------------------
// 1. 真实密码认证通过与错误密码严格拒绝
// -----------------------------------------------------------------------------
#[tokio::test]
async fn test_e2e_password_auth_success_and_rejection() {
    let (port, _shutdown, known_hosts_path) = start_embedded_ssh_server().await;
    let driver = RusshSessionDriver::with_known_hosts_path(Some(known_hosts_path.clone()));
    let host = build_test_host(port);

    // 1.1 错误密码尝试：预期返回 AuthFailed
    let wrong_cred = CredentialRecord {
        id: "cred-wrong".to_string(),
        name: "Wrong Cred".to_string(),
        cred_type: CredentialType::Password,
        algorithm: "Password".to_string(),
        username: Some("smalux_user".to_string()),
        secret_data: "wrong_password_999".to_string(),
        passphrase: None,
        public_key: None,
        fingerprint: None,
        bound_host_count: 0,
        created_at: String::new(),
        updated_at: String::new(),
        notes: String::new(),
    };
    let wrong_res = driver.connect(&host, Some(&wrong_cred)).await;
    assert!(wrong_res.is_err(), "错误密码必须被严格拒绝");
    match wrong_res.unwrap_err() {
        SshServiceError::AuthFailed(_) => {}
        other => panic!("预期返回 SshServiceError::AuthFailed，实际为: {:?}", other),
    }

    // 1.2 正确密码尝试：预期握手成功并建立会话
    let correct_cred = CredentialRecord {
        id: "cred-correct".to_string(),
        name: "Correct Cred".to_string(),
        cred_type: CredentialType::Password,
        algorithm: "Password".to_string(),
        username: Some("smalux_user".to_string()),
        secret_data: "secret_password_123".to_string(),
        passphrase: None,
        public_key: None,
        fingerprint: None,
        bound_host_count: 0,
        created_at: String::new(),
        updated_at: String::new(),
        notes: String::new(),
    };
    let connect_res = driver.connect(&host, Some(&correct_cred)).await;
    assert!(connect_res.is_ok(), "正确密码应成功连接建立 SSH 会话: {:?}", connect_res.err());

    let session_id = connect_res.unwrap();
    let _ = driver.disconnect(&session_id).await;
    let _ = tokio::fs::remove_file(known_hosts_path).await;
}

// -----------------------------------------------------------------------------
// 2. 原生密钥对现场生成与公钥免密登录
// -----------------------------------------------------------------------------
#[tokio::test]
async fn test_e2e_keygen_and_publickey_auth() {
    let (port, _shutdown, known_hosts_path) = start_embedded_ssh_server().await;
    let driver = RusshSessionDriver::with_known_hosts_path(Some(known_hosts_path.clone()));
    let host = build_test_host(port);

    // 2.1 现场生成 Ed25519 密钥对
    let keygen = NativeKeygenService::new();
    let keypair = keygen
        .generate_keypair(KeyAlgorithm::Ed25519, None, None)
        .expect("现场 Ed25519 密钥对生成应成功");

    // 2.2 组装凭据记录
    let key_cred = CredentialRecord {
        id: "cred-keygen".to_string(),
        name: "Keygen Cred".to_string(),
        cred_type: CredentialType::Key,
        algorithm: "Ed25519".to_string(),
        username: Some("smalux_user".to_string()),
        secret_data: keypair.private_key_pem,
        passphrase: None,
        public_key: Some(keypair.public_key_openssh),
        fingerprint: Some(keypair.fingerprint),
        bound_host_count: 0,
        created_at: String::new(),
        updated_at: String::new(),
        notes: String::new(),
    };

    // 2.3 基于生成的私钥建立原生会话
    let connect_res = driver.connect(&host, Some(&key_cred)).await;
    assert!(connect_res.is_ok(), "原生生成的 Ed25519 私钥公钥应顺利完成签名认证: {:?}", connect_res.err());

    let session_id = connect_res.unwrap();
    let _ = driver.disconnect(&session_id).await;
    let _ = tokio::fs::remove_file(known_hosts_path).await;
}

// -----------------------------------------------------------------------------
// 3. 交互式 PTY 终端双向全双工字节流与 Echo 验证
// -----------------------------------------------------------------------------
#[tokio::test]
async fn test_e2e_interactive_pty_terminal_stream() {
    let (port, _shutdown, known_hosts_path) = start_embedded_ssh_server().await;
    let driver = RusshSessionDriver::with_known_hosts_path(Some(known_hosts_path.clone()));
    let host = build_test_host(port);

    let cred = CredentialRecord {
        id: "cred-pty".to_string(),
        name: "Pty Cred".to_string(),
        cred_type: CredentialType::Password,
        algorithm: "Password".to_string(),
        username: Some("smalux_user".to_string()),
        secret_data: "secret_password_123".to_string(),
        passphrase: None,
        public_key: None,
        fingerprint: None,
        bound_host_count: 0,
        created_at: String::new(),
        updated_at: String::new(),
        notes: String::new(),
    };

    let session_id = driver.connect(&host, Some(&cred)).await.expect("连接应成功");

    // 分配 PTY 伪终端
    let channel = driver
        .open_pty_channel(&session_id, "xterm-256color", 24, 80)
        .await
        .expect("打开 PTY 交互流应成功");

    let (mut read_half, mut write_half) = tokio::io::split(channel);

    // 3.1 接收服务端启动 Welcome Banner
    let mut banner_buf = [0u8; 64];
    let n = read_half.read(&mut banner_buf).await.expect("读取欢迎横幅应成功");
    let banner_str = String::from_utf8_lossy(&banner_buf[..n]);
    assert!(banner_str.contains("WELCOME_TO_SMALUX"), "应接收到服务端问候横幅");

    // 3.2 发送交互式输入报文并接收 Echo 回显
    write_half.write_all(b"HelloSmagical\n").await.expect("写入终端输入流应成功");
    write_half.flush().await.expect("Flush 应成功");

    let mut echo_buf = [0u8; 64];
    let n = read_half.read(&mut echo_buf).await.expect("读取终端回显应成功");
    let echo_str = String::from_utf8_lossy(&echo_buf[..n]);
    assert!(echo_str.contains("ECHO:HelloSmagical\n"), "终端输入流应接收到服务端原样 Echo: {echo_str}");

    let _ = driver.disconnect(&session_id).await;
    let _ = tokio::fs::remove_file(known_hosts_path).await;
}

// -----------------------------------------------------------------------------
// 4. 远程非阻塞命令单次执行 (`exec`) 与结果捕获
// -----------------------------------------------------------------------------
#[tokio::test]
async fn test_e2e_exec_command_execution() {
    let (port, _shutdown, known_hosts_path) = start_embedded_ssh_server().await;
    let driver = RusshSessionDriver::with_known_hosts_path(Some(known_hosts_path.clone()));
    let host = build_test_host(port);

    let cred = CredentialRecord {
        id: "cred-exec".to_string(),
        name: "Exec Cred".to_string(),
        cred_type: CredentialType::Password,
        algorithm: "Password".to_string(),
        username: Some("smalux_user".to_string()),
        secret_data: "secret_password_123".to_string(),
        passphrase: None,
        public_key: None,
        fingerprint: None,
        bound_host_count: 0,
        created_at: String::new(),
        updated_at: String::new(),
        notes: String::new(),
    };

    let session_id = driver.connect(&host, Some(&cred)).await.expect("连接应成功");

    let CommandExecutionOutput { stdout, stderr, exit_code } = driver
        .execute_command(&session_id, "uname -a")
        .await
        .expect("非阻塞命令执行应成功");

    assert_eq!(exit_code, 0, "退出码应为 0");
    assert!(stderr.is_empty(), "标准错误应为空");
    let stdout_str = String::from_utf8_lossy(&stdout);
    assert!(stdout_str.contains("OUTPUT_FOR_uname -a"), "标准输出应包含执行结果: {stdout_str}");

    let _ = driver.disconnect(&session_id).await;
    let _ = tokio::fs::remove_file(known_hosts_path).await;
}

// -----------------------------------------------------------------------------
// 5. OpenSSH known_hosts TOFU 信任生命周期与中间人公钥篡改拦截 (MITM Mismatch)
// -----------------------------------------------------------------------------
#[tokio::test]
async fn test_e2e_known_hosts_tofu_and_mitm_tamper_defense() {
    let (port, _shutdown, known_hosts_path) = start_embedded_ssh_server().await;
    let driver = RusshSessionDriver::with_known_hosts_path(Some(known_hosts_path.clone()));
    let host = build_test_host(port);

    let cred = CredentialRecord {
        id: "cred-kh".to_string(),
        name: "KH Cred".to_string(),
        cred_type: CredentialType::Password,
        algorithm: "Password".to_string(),
        username: Some("smalux_user".to_string()),
        secret_data: "secret_password_123".to_string(),
        passphrase: None,
        public_key: None,
        fingerprint: None,
        bound_host_count: 0,
        created_at: String::new(),
        updated_at: String::new(),
        notes: String::new(),
    };

    // 5.1 首次连接：此时 known_hosts 文件尚不存在，预期触发 TOFU 自动创建并落盘指纹
    assert!(!known_hosts_path.exists(), "初始状态下临时 known_hosts 不应存在");
    let sess1 = driver.connect(&host, Some(&cred)).await.expect("首次连接应成功触发 TOFU 放行");
    let _ = driver.disconnect(&sess1).await;

    // 验证文件已成功创建并包含主机端口记录
    assert!(known_hosts_path.exists(), "首次连接后 known_hosts 文件必须已落盘写入");
    let kh_content = tokio::fs::read_to_string(&known_hosts_path).await.unwrap();
    assert!(kh_content.contains(&format!("[127.0.0.1]:{}", port)), "文件应包含该主机的特征标头: {kh_content}");

    // 5.2 第二次连接：known_hosts 已存在该公钥，预期平滑验真 (Trusted) 通过
    let sess2 = driver.connect(&host, Some(&cred)).await.expect("二次连接应基于 known_hosts 信任白名单正常放行");
    let _ = driver.disconnect(&sess2).await;

    // 5.3 模拟中间人攻击：故意篡改 known_hosts 文件中该主机的公钥内容
    let fake_tampered_content = format!("[127.0.0.1]:{} ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIFakeTamperedKeyForMitmAttackTest1234567890=\n", port);
    tokio::fs::write(&known_hosts_path, fake_tampered_content).await.unwrap();

    // 5.4 第三次连接：指纹不匹配，预期立即触发安全警报并切断连接！
    let mitm_res = driver.connect(&host, Some(&cred)).await;
    assert!(mitm_res.is_err(), "遭遇中间人公钥篡改时必须强制拒绝连接");

    let _ = tokio::fs::remove_file(known_hosts_path).await;
}
