//! `russh` 客户端事件处理器。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use russh::Channel;
use russh::client::{self, Msg};
use tokio::sync::mpsc;

use crate::{HostKeyCheck, HostKeyPolicy};

pub type ForwardedChannel = Channel<Msg>;

#[derive(Debug, Clone)]
pub struct SshClientHandler {
    host: String,
    port: u16,
    host_key_policy: HostKeyPolicy,
    host_key_result: SharedHostKeyResult,
    forwarded_channels: SharedForwardedChannels,
}

impl SshClientHandler {
    /// 创建 `russh` 客户端事件处理器。
    pub fn new(
        host: String,
        port: u16,
        host_key_policy: HostKeyPolicy,
        host_key_result: SharedHostKeyResult,
        forwarded_channels: SharedForwardedChannels,
    ) -> Self {
        Self {
            host,
            port,
            host_key_policy,
            host_key_result,
            forwarded_channels,
        }
    }
}

impl client::Handler for SshClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &russh::keys::PublicKey,
    ) -> Result<bool, Self::Error> {
        let check = self
            .host_key_policy
            .check(&self.host, self.port, server_public_key);
        let accepted = check.accepted;
        self.host_key_result.set(check);
        Ok(accepted)
    }

    async fn server_channel_open_forwarded_tcpip(
        &mut self,
        channel: ForwardedChannel,
        connected_address: &str,
        connected_port: u32,
        _originator_address: &str,
        _originator_port: u32,
        _session: &mut client::Session,
    ) -> Result<(), Self::Error> {
        let port = u16::try_from(connected_port).map_err(|_| russh::Error::Inconsistent)?;
        self.forwarded_channels
            .send(connected_address, port, channel)
            .map_err(|_| russh::Error::ChannelOpenFailure(russh::ChannelOpenFailure::ConnectFailed))
    }
}

#[derive(Debug, Clone, Default)]
pub struct SharedHostKeyResult {
    value: Arc<Mutex<Option<HostKeyCheck>>>,
}

impl SharedHostKeyResult {
    /// 记录最近一次主机密钥检查结果。
    pub fn set(&self, result: HostKeyCheck) {
        if let Ok(mut value) = self.value.lock() {
            *value = Some(result);
        }
    }

    /// 返回最近一次主机密钥检查结果。
    pub fn get(&self) -> Option<HostKeyCheck> {
        self.value.lock().ok().and_then(|value| value.clone())
    }
}

#[derive(Debug, Clone, Default)]
pub struct SharedForwardedChannels {
    subscribers: Arc<Mutex<HashMap<(String, u16), mpsc::UnboundedSender<ForwardedChannel>>>>,
}

impl SharedForwardedChannels {
    /// 订阅指定 forwarded-tcpip endpoint。
    pub fn subscribe(
        &self,
        connected_address: &str,
        connected_port: u16,
    ) -> mpsc::UnboundedReceiver<ForwardedChannel> {
        let (sender, receiver) = mpsc::unbounded_channel();
        if let Ok(mut subscribers) = self.subscribers.lock() {
            subscribers.insert((connected_address.to_owned(), connected_port), sender);
        }
        receiver
    }

    /// 将服务端 forwarded channel 转发给匹配订阅者。
    pub fn send(
        &self,
        connected_address: &str,
        connected_port: u16,
        channel: ForwardedChannel,
    ) -> Result<(), mpsc::error::SendError<ForwardedChannel>> {
        let sender = self.subscribers.lock().ok().and_then(|subscribers| {
            subscribers
                .get(&(connected_address.to_owned(), connected_port))
                .cloned()
        });

        match sender {
            Some(sender) => sender.send(channel),
            None => Err(mpsc::error::SendError(channel)),
        }
    }
}
