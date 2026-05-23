//! ssh-agent 认证执行。

use russh::client;
use russh::keys::agent::client::{AgentClient, AgentStream};
use smagical_ssh_client_core::{
    agent_identity_authentication_error, authentication_error, select_agent_identity,
};

use crate::backend::BackendExecutionError;
use crate::backend::ssh::client::{SshClientHandler, auth::best_supported_rsa_hash};

#[cfg(windows)]
const WINDOWS_OPENSSH_AGENT_PIPE: &str = r"\\.\pipe\openssh-ssh-agent";
type DynamicAgentClient = AgentClient<Box<dyn AgentStream + Send + Unpin + 'static>>;

pub(super) async fn authenticate_agent(
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
    let public_key = select_agent_identity(&identities, key_hint)
        .ok_or_else(|| agent_identity_authentication_error(username, key_hint))?;
    let hash_alg = best_supported_rsa_hash(handle, username).await?;

    handle
        .authenticate_publickey_with(username.to_owned(), public_key, hash_alg, &mut agent)
        .await
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
