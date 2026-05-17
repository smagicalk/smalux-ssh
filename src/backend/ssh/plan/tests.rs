use super::*;
use crate::backend::{BackendAuth, ConnectionTarget};
use crate::model::{HostId, SecretRef};
use crate::security::MemorySecretStore;
use crate::security::SecretStore;
use uuid::Uuid;

fn target(auth: BackendAuth) -> ConnectionTarget {
    ConnectionTarget {
        host_id: HostId(Uuid::new_v4()),
        address: "example.com".to_owned(),
        port: 2222,
        auth,
    }
}

#[test]
fn password_plan_resolves_secret_and_endpoint() {
    let mut store = MemorySecretStore::new();
    let secret = SecretRef("password:root".to_owned());
    store
        .set_secret(&secret, "s3cret")
        .expect("内存凭据应该可以写入");
    let target = target(BackendAuth::Password {
        username: "root".to_owned(),
        secret,
    });

    let plan = SshConnectionPlan::from_target(&target, &store).expect("密码计划应该可以构建");

    assert_eq!(plan.host, "example.com");
    assert_eq!(plan.port, 2222);
    assert_eq!(plan.endpoint, "example.com:2222");
    assert_eq!(plan.username(), "root");
    assert!(matches!(
        plan.auth,
        SshAuthPlan::Password {
            password,
            ..
        } if password == "s3cret"
    ));
}

#[test]
fn key_plan_resolves_optional_passphrase() {
    let mut store = MemorySecretStore::new();
    let key = SecretRef("key:deploy".to_owned());
    let passphrase = SecretRef("passphrase:deploy".to_owned());
    store
        .set_secret(&key, "PRIVATE KEY")
        .expect("私钥应该可以写入");
    store
        .set_secret(&passphrase, "phrase")
        .expect("私钥口令应该可以写入");
    let target = target(BackendAuth::Key {
        username: "deploy".to_owned(),
        key,
        passphrase: Some(passphrase),
    });

    let plan = SshConnectionPlan::from_target(&target, &store).expect("私钥计划应该可以构建");

    assert_eq!(plan.username(), "deploy");
    assert!(matches!(
        plan.auth,
        SshAuthPlan::Key {
            private_key,
            passphrase: Some(passphrase),
            ..
        } if private_key == "PRIVATE KEY" && passphrase == "phrase"
    ));
}

#[test]
fn agent_plan_does_not_read_secret_store() {
    let store = MemorySecretStore::new();
    let target = target(BackendAuth::Agent {
        username: "agent-user".to_owned(),
        key_hint: Some("id_ed25519".to_owned()),
    });

    let plan = SshConnectionPlan::from_target(&target, &store).expect("agent 计划应该可以构建");

    assert_eq!(plan.auth.method(), "agent");
    assert!(matches!(
        plan.auth,
        SshAuthPlan::Agent {
            key_hint: Some(key_hint),
            ..
        } if key_hint == "id_ed25519"
    ));
}

#[test]
fn certificate_plan_resolves_key_and_certificate() {
    let mut store = MemorySecretStore::new();
    let key = SecretRef("key:cert".to_owned());
    let passphrase = SecretRef("passphrase:cert".to_owned());
    let certificate = SecretRef("cert:deploy".to_owned());
    store
        .set_secret(&key, "PRIVATE KEY")
        .expect("私钥应该可以写入");
    store
        .set_secret(&passphrase, "phrase")
        .expect("证书私钥口令应该可以写入");
    store
        .set_secret(&certificate, "CERT")
        .expect("证书应该可以写入");
    let target = target(BackendAuth::Certificate {
        username: "deploy".to_owned(),
        key,
        passphrase: Some(passphrase),
        certificate,
    });

    let plan = SshConnectionPlan::from_target(&target, &store).expect("证书计划应该可以构建");

    assert_eq!(plan.auth.method(), "certificate");
    assert!(matches!(
        plan.auth,
        SshAuthPlan::Certificate {
            private_key,
            passphrase: Some(passphrase),
            certificate,
            ..
        } if private_key == "PRIVATE KEY" && passphrase == "phrase" && certificate == "CERT"
    ));
}

#[test]
fn missing_secret_maps_to_authentication_failure() {
    let store = MemorySecretStore::new();
    let target = target(BackendAuth::Password {
        username: "root".to_owned(),
        secret: SecretRef("missing".to_owned()),
    });

    let error =
        SshConnectionPlan::from_target(&target, &store).expect_err("缺失凭据应该映射为认证失败");

    assert!(matches!(
        error,
        BackendExecutionError::AuthenticationFailed {
            username,
            reason,
        } if username == "root" && reason.contains("找不到凭据引用")
    ));
}
