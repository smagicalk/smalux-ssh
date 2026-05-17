use super::*;
use crate::model::{AuthProfile, SecretRef};
use uuid::Uuid;

fn host_id() -> HostId {
    HostId(Uuid::new_v4())
}

#[test]
fn quick_host_draft_builds_agent_host() {
    let draft = QuickHostDraft {
        name: "prod".to_owned(),
        address: "prod.example.com".to_owned(),
        port: "2222".to_owned(),
        username: "deploy".to_owned(),
        tags: "prod, linux".to_owned(),
        auth: QuickHostAuthDraft {
            kind: QuickHostAuthKind::Agent,
            key_hint: "id_ed25519".to_owned(),
            ..QuickHostAuthDraft::default()
        },
    };

    let host = draft
        .build_agent_host(host_id())
        .expect("有效主机草稿应该可以生成主机配置");

    assert_eq!(host.name, "prod");
    assert_eq!(host.address, "prod.example.com");
    assert_eq!(host.port, 2222);
    assert_eq!(host.tags, vec!["prod", "linux"]);
    assert!(matches!(
        host.auth,
        AuthProfile::Agent {
            username,
            key_hint: Some(key_hint),
        } if username == "deploy" && key_hint == "id_ed25519"
    ));
}

#[test]
fn quick_host_draft_validates_required_fields() {
    let draft = QuickHostDraft::default();

    assert_eq!(
        draft.build_host(host_id()),
        Err(QuickHostDraftError::EmptyAddress)
    );

    let missing_user = QuickHostDraft {
        address: "example.com".to_owned(),
        ..QuickHostDraft::default()
    };
    assert_eq!(
        missing_user.build_host(host_id()),
        Err(QuickHostDraftError::EmptyUsername)
    );

    let missing_password_ref = QuickHostDraft {
        address: "example.com".to_owned(),
        username: "root".to_owned(),
        auth: QuickHostAuthDraft {
            kind: QuickHostAuthKind::Password,
            ..QuickHostAuthDraft::default()
        },
        ..QuickHostDraft::default()
    };
    assert_eq!(
        missing_password_ref.build_host(host_id()),
        Err(QuickHostDraftError::MissingPasswordSecretRef)
    );

    let missing_private_key_ref = QuickHostDraft {
        address: "example.com".to_owned(),
        username: "deploy".to_owned(),
        auth: QuickHostAuthDraft {
            kind: QuickHostAuthKind::Key,
            ..QuickHostAuthDraft::default()
        },
        ..QuickHostDraft::default()
    };
    assert_eq!(
        missing_private_key_ref.build_host(host_id()),
        Err(QuickHostDraftError::MissingPrivateKeyRef)
    );

    let missing_certificate_ref = QuickHostDraft {
        address: "example.com".to_owned(),
        username: "deploy".to_owned(),
        auth: QuickHostAuthDraft {
            kind: QuickHostAuthKind::Certificate,
            private_key_ref: "key:deploy".to_owned(),
            ..QuickHostAuthDraft::default()
        },
        ..QuickHostDraft::default()
    };
    assert_eq!(
        missing_certificate_ref.build_host(host_id()),
        Err(QuickHostDraftError::MissingCertificateRef)
    );
}

#[test]
fn quick_host_draft_builds_password_host() {
    let draft = QuickHostDraft {
        name: "root".to_owned(),
        address: "root.example.com".to_owned(),
        port: "22".to_owned(),
        username: "root".to_owned(),
        tags: String::new(),
        auth: QuickHostAuthDraft {
            kind: QuickHostAuthKind::Password,
            password_secret_ref: "password:root".to_owned(),
            ..QuickHostAuthDraft::default()
        },
    };

    let host = draft
        .build_host(host_id())
        .expect("密码草稿应该可以生成主机配置");

    assert!(matches!(
        host.auth,
        AuthProfile::Password {
            username,
            secret: SecretRef(ref secret_ref),
        } if username == "root" && secret_ref == "password:root"
    ));
}

#[test]
fn quick_host_draft_builds_key_host_with_passphrase() {
    let draft = QuickHostDraft {
        name: "deploy".to_owned(),
        address: "deploy.example.com".to_owned(),
        port: "2200".to_owned(),
        username: "deploy".to_owned(),
        tags: String::new(),
        auth: QuickHostAuthDraft {
            kind: QuickHostAuthKind::Key,
            private_key_ref: "key:deploy".to_owned(),
            passphrase_ref: "passphrase:deploy".to_owned(),
            ..QuickHostAuthDraft::default()
        },
    };

    let host = draft
        .build_host(host_id())
        .expect("私钥草稿应该可以生成主机配置");

    assert!(matches!(
        host.auth,
        AuthProfile::Key {
            username,
            key: SecretRef(ref key_ref),
            passphrase: Some(SecretRef(ref passphrase_ref)),
        } if username == "deploy"
            && key_ref == "key:deploy"
            && passphrase_ref == "passphrase:deploy"
    ));
}

#[test]
fn quick_host_draft_builds_certificate_host() {
    let draft = QuickHostDraft {
        name: "cert".to_owned(),
        address: "cert.example.com".to_owned(),
        port: "2222".to_owned(),
        username: "cert-user".to_owned(),
        tags: String::new(),
        auth: QuickHostAuthDraft {
            kind: QuickHostAuthKind::Certificate,
            private_key_ref: "key:cert-user".to_owned(),
            passphrase_ref: "passphrase:cert-user".to_owned(),
            certificate_ref: "cert:cert-user".to_owned(),
            ..QuickHostAuthDraft::default()
        },
    };

    let host = draft
        .build_host(host_id())
        .expect("证书草稿应该可以生成主机配置");

    assert!(matches!(
        host.auth,
        AuthProfile::Certificate {
            username,
            key: SecretRef(ref key_ref),
            passphrase: Some(SecretRef(ref passphrase_ref)),
            certificate: SecretRef(ref certificate_ref),
        } if username == "cert-user"
            && key_ref == "key:cert-user"
            && passphrase_ref == "passphrase:cert-user"
            && certificate_ref == "cert:cert-user"
    ));
}

#[test]
fn ui_state_quick_host_messages_update_form_only() {
    let mut state = UiState::default();

    state.set_quick_host_field(QuickHostDraftField::Address, "example.com");
    state.set_quick_host_field(QuickHostDraftField::Username, "ops");
    state.set_quick_host_auth_kind(QuickHostAuthKind::Password);
    state.set_quick_host_auth_field(QuickHostAuthField::PasswordSecretRef, "password:ops");
    state.reset_quick_host();

    assert_eq!(state.quick_host.address, "");
    assert_eq!(state.quick_host.username, "");
    assert_eq!(state.quick_host.port, "22");
    assert!(matches!(
        state.quick_host.auth.kind,
        QuickHostAuthKind::Agent
    ));
}
