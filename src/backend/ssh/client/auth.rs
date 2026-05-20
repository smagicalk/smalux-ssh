//! SSH 认证执行。

use std::sync::Arc;

use russh::client;
use russh::keys::agent::client::{AgentClient, AgentStream};
use russh::keys::{Certificate, HashAlg, PrivateKeyWithHashAlg};
use smagical_ssh_client_core::{
    agent_identity_error, authentication_error, decode_private_key, select_agent_identity,
};

use super::SshClientHandler;
use crate::backend::BackendExecutionError;
use crate::backend::ssh::SshAuthPlan;

#[cfg(windows)]
const WINDOWS_OPENSSH_AGENT_PIPE: &str = r"\\.\pipe\openssh-ssh-agent";
type DynamicAgentClient = AgentClient<Box<dyn AgentStream + Send + Unpin + 'static>>;

pub(super) async fn authenticate(
    handle: &mut client::Handle<SshClientHandler>,
    auth: &SshAuthPlan,
) -> Result<(), BackendExecutionError> {
    let result = match auth {
        SshAuthPlan::Password { username, password } => handle
            .authenticate_password(username.clone(), password.clone())
            .await
            .map_err(|error| authentication_error(username, error))?,
        SshAuthPlan::Key {
            username,
            private_key,
            passphrase,
        } => {
            let key = decode_private_key(private_key, passphrase.as_deref(), username)?;
            let hash_alg = best_supported_rsa_hash(handle, username).await?;
            handle
                .authenticate_publickey(
                    username.clone(),
                    PrivateKeyWithHashAlg::new(Arc::new(key), hash_alg),
                )
                .await
                .map_err(|error| authentication_error(username, error))?
        }
        SshAuthPlan::Agent { username, key_hint } => {
            authenticate_agent(handle, username, key_hint.as_deref()).await?
        }
        SshAuthPlan::Certificate {
            username,
            private_key,
            passphrase,
            certificate,
        } => {
            let key = decode_private_key(private_key, passphrase.as_deref(), username)?;
            let certificate = Certificate::from_openssh(certificate)
                .map_err(|error| authentication_error(username, error))?;
            handle
                .authenticate_openssh_cert(username.clone(), Arc::new(key), certificate)
                .await
                .map_err(|error| authentication_error(username, error))?
        }
    };

    if result.success() {
        Ok(())
    } else {
        Err(BackendExecutionError::AuthenticationFailed {
            username: auth.username().to_owned(),
            reason: format!("{} 认证被服务器拒绝", auth.method()),
        })
    }
}

async fn authenticate_agent(
    handle: &mut client::Handle<SshClientHandler>,
    username: &str,
    key_hint: Option<&str>,
) -> Result<client::AuthResult, BackendExecutionError> {
    let mut agent = connect_agent()
        .await
        .map_err(|error| authentication_error(username, error))?;
    let identities = agent
        .request_identities()
        .await
        .map_err(|error| authentication_error(username, error))?;
    let public_key = select_agent_identity(&identities, key_hint).ok_or_else(|| {
        BackendExecutionError::AuthenticationFailed {
            username: username.to_owned(),
            reason: agent_identity_error(key_hint),
        }
    })?;
    let hash_alg = best_supported_rsa_hash(handle, username).await?;

    handle
        .authenticate_publickey_with(username.to_owned(), public_key, hash_alg, &mut agent)
        .await
        .map_err(|error| authentication_error(username, error))
}

async fn best_supported_rsa_hash(
    handle: &client::Handle<SshClientHandler>,
    username: &str,
) -> Result<Option<HashAlg>, BackendExecutionError> {
    handle
        .best_supported_rsa_hash()
        .await
        .map(|hash_alg| hash_alg.flatten())
        .map_err(|error| authentication_error(username, error))
}

async fn connect_agent() -> Result<DynamicAgentClient, russh::keys::Error> {
    #[cfg(unix)]
    {
        AgentClient::connect_env().await.map(AgentClient::dynamic)
    }

    #[cfg(windows)]
    {
        match AgentClient::connect_named_pipe(WINDOWS_OPENSSH_AGENT_PIPE).await {
            Ok(agent) => Ok(agent.dynamic()),
            Err(named_pipe_error) => match AgentClient::connect_pageant().await {
                Ok(agent) => Ok(agent.dynamic()),
                Err(pageant_error) => Err(russh::keys::Error::IO(std::io::Error::other(format!(
                    "OpenSSH agent 连接失败：{named_pipe_error}; Pageant 连接失败：{pageant_error}"
                )))),
            },
        }
    }

    #[cfg(not(any(unix, windows)))]
    {
        Err(russh::keys::Error::IO(std::io::Error::other(
            "当前平台暂不支持 ssh-agent 自动发现",
        )))
    }
}
