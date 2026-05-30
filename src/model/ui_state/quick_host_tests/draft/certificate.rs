use crate::model::ui_state::{QuickHostAuthDraft, QuickHostAuthKind, QuickHostDraft};
use crate::model::{AuthProfile, SecretRef};

use super::super::common::host_id;

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
        ..QuickHostDraft::default()
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
