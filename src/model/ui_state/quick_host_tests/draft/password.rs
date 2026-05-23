use crate::model::ui_state::{QuickHostAuthDraft, QuickHostAuthKind, QuickHostDraft};
use crate::model::{AuthProfile, SecretRef};

use super::super::common::host_id;

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
