//! SOCKS5 CONNECT 握手解析。

use std::net::{IpAddr, Ipv4Addr};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub(super) struct Socks5Target {
    pub host: String,
    pub port: u16,
}

pub(super) async fn read_socks5_target(stream: &mut TcpStream) -> Result<Socks5Target, String> {
    let mut greeting = [0_u8; 2];
    stream
        .read_exact(&mut greeting)
        .await
        .map_err(|error| error.to_string())?;
    if greeting[0] != 0x05 {
        return Err("unsupported SOCKS version".to_owned());
    }

    let mut methods = vec![0_u8; greeting[1] as usize];
    stream
        .read_exact(&mut methods)
        .await
        .map_err(|error| error.to_string())?;
    stream
        .write_all(&[0x05, 0x00])
        .await
        .map_err(|error| error.to_string())?;

    let mut header = [0_u8; 4];
    stream
        .read_exact(&mut header)
        .await
        .map_err(|error| error.to_string())?;
    if header[0] != 0x05 || header[1] != 0x01 {
        return Err("only SOCKS5 CONNECT is supported".to_owned());
    }

    let host = match header[3] {
        0x01 => read_ipv4(stream).await?,
        0x03 => read_domain(stream).await?,
        0x04 => read_ipv6(stream).await?,
        other => return Err(format!("unsupported SOCKS address type: {other}")),
    };

    let mut port_bytes = [0_u8; 2];
    stream
        .read_exact(&mut port_bytes)
        .await
        .map_err(|error| error.to_string())?;

    Ok(Socks5Target {
        host,
        port: u16::from_be_bytes(port_bytes),
    })
}

pub(super) async fn write_socks5_success(stream: &mut TcpStream) -> std::io::Result<()> {
    stream
        .write_all(&[0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
        .await
}

async fn read_ipv4(stream: &mut TcpStream) -> Result<String, String> {
    let mut octets = [0_u8; 4];
    stream
        .read_exact(&mut octets)
        .await
        .map_err(|error| error.to_string())?;
    Ok(IpAddr::V4(Ipv4Addr::from(octets)).to_string())
}

async fn read_ipv6(stream: &mut TcpStream) -> Result<String, String> {
    let mut octets = [0_u8; 16];
    stream
        .read_exact(&mut octets)
        .await
        .map_err(|error| error.to_string())?;
    Ok(IpAddr::V6(octets.into()).to_string())
}

async fn read_domain(stream: &mut TcpStream) -> Result<String, String> {
    let mut length = [0_u8; 1];
    stream
        .read_exact(&mut length)
        .await
        .map_err(|error| error.to_string())?;
    let mut name = vec![0_u8; length[0] as usize];
    stream
        .read_exact(&mut name)
        .await
        .map_err(|error| error.to_string())?;
    String::from_utf8(name).map_err(|error| error.to_string())
}
