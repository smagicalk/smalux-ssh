use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use crate::model::SessionId;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;

use super::{
    socks5::{read_socks5_target, write_socks5_success},
    tunnel,
};

#[test]
fn remote_tunnel_reports_endpoint_and_can_stop() {
    let session_id = SessionId(Uuid::new_v4());
    let running = Arc::new(AtomicBool::new(true));
    let tunnel = tunnel(
        session_id,
        "proxy".to_owned(),
        running.clone(),
        "127.0.0.1".to_owned(),
        1080,
    );

    assert_eq!(tunnel.session_id(), session_id);
    assert_eq!(tunnel.rule_name(), "proxy");
    assert_eq!(tunnel.bind_endpoint(), "127.0.0.1:1080");
    tunnel.stop();
    assert!(!running.load(Ordering::SeqCst));
}

async fn parse_socks5_target_from_client_bytes(
    bytes: &'static [u8],
) -> Result<(String, u16, Vec<u8>), String> {
    let (mut client, mut server) = tokio::io::duplex(128);
    let client_task = tokio::spawn(async move {
        client.write_all(bytes).await.unwrap();
        let mut response = [0_u8; 2];
        client.read_exact(&mut response).await.unwrap();
        response.to_vec()
    });

    let target = read_socks5_target(&mut server).await?;
    let response = client_task.await.unwrap();
    Ok((target.host, target.port, response))
}

#[tokio::test]
async fn socks5_target_reads_ipv4_connect_request() {
    let (host, port, response) = parse_socks5_target_from_client_bytes(&[
        0x05, 0x01, 0x00, // greeting: no auth
        0x05, 0x01, 0x00, 0x01, // CONNECT IPv4
        127, 0, 0, 1, 0x1F, 0x90, // 127.0.0.1:8080
    ])
    .await
    .unwrap();

    assert_eq!(host, "127.0.0.1");
    assert_eq!(port, 8080);
    assert_eq!(response, vec![0x05, 0x00]);
}

#[tokio::test]
async fn socks5_target_reads_domain_connect_request() {
    let (host, port, response) = parse_socks5_target_from_client_bytes(&[
        0x05, 0x01, 0x00, // greeting: no auth
        0x05, 0x01, 0x00, 0x03, // CONNECT domain
        11, b'e', b'x', b'a', b'm', b'p', b'l', b'e', b'.', b'c', b'o', b'm', 0x00, 0x16,
    ])
    .await
    .unwrap();

    assert_eq!(host, "example.com");
    assert_eq!(port, 22);
    assert_eq!(response, vec![0x05, 0x00]);
}

#[tokio::test]
async fn socks5_target_reads_ipv6_connect_request() {
    let (host, port, response) = parse_socks5_target_from_client_bytes(&[
        0x05, 0x01, 0x00, // greeting: no auth
        0x05, 0x01, 0x00, 0x04, // CONNECT IPv6
        0x20, 0x01, 0x0D, 0xB8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0x01, 0xBB,
    ])
    .await
    .unwrap();

    assert_eq!(host, "2001:db8::1");
    assert_eq!(port, 443);
    assert_eq!(response, vec![0x05, 0x00]);
}

#[tokio::test]
async fn socks5_target_rejects_non_connect_commands() {
    let result = parse_socks5_target_from_client_bytes(&[
        0x05, 0x01, 0x00, // greeting: no auth
        0x05, 0x02, 0x00, 0x01, // BIND is unsupported
        127, 0, 0, 1, 0x00, 0x16,
    ])
    .await;

    assert_eq!(
        result.unwrap_err(),
        "only SOCKS5 CONNECT is supported".to_owned()
    );
}

#[tokio::test]
async fn socks5_target_rejects_unknown_address_types() {
    let result = parse_socks5_target_from_client_bytes(&[
        0x05, 0x01, 0x00, // greeting: no auth
        0x05, 0x01, 0x00, 0x7F, // unknown address type
        0x00, 0x16,
    ])
    .await;

    assert_eq!(
        result.unwrap_err(),
        "unsupported SOCKS address type: 127".to_owned()
    );
}

#[tokio::test]
async fn socks5_success_response_uses_ipv4_unspecified_endpoint() {
    let (mut client, mut server) = tokio::io::duplex(16);
    let client_task = tokio::spawn(async move {
        let mut response = [0_u8; 10];
        client.read_exact(&mut response).await.unwrap();
        response
    });

    write_socks5_success(&mut server).await.unwrap();

    assert_eq!(
        client_task.await.unwrap(),
        [0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0]
    );
}
