//! SSH 认证执行。

use std::sync::Arc;

use russh::client;
use russh::keys::{Certificate, HashAlg, PrivateKeyWithHashAlg};
use smagical_ssh_client_core::{
    authentication_error, authentication_rejected_error, decode_private_key,
};

use super::SshClientHandler;
use crate::backend::BackendExecutionError;
use crate::backend::ssh::SshAuthPlan;

#[path = "auth/agent.rs"]
mod agent;

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
        SshAuthPlan::Agent {
            username,
            source,
            key_hint,
        } => agent::authenticate_agent(handle, username, source, key_hint.as_deref()).await?,
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
        Err(authentication_rejected_error(
            auth.username(),
            auth.method(),
        ))
    }
}

pub(super) async fn best_supported_rsa_hash(
    handle: &client::Handle<SshClientHandler>,
    username: &str,
) -> Result<Option<HashAlg>, BackendExecutionError> {
    handle
        .best_supported_rsa_hash()
        .await
        .map(|hash_alg| hash_alg.flatten())
        .map_err(|error| authentication_error(username, error))
}
