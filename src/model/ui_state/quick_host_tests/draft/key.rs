use crate::model::ui_state::{QuickHostAuthDraft, QuickHostAuthKind, QuickHostDraft};
use crate::model::{AuthProfile, SecretRef};

use super::super::common::host_id;

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
        ..QuickHostDraft::default()
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
