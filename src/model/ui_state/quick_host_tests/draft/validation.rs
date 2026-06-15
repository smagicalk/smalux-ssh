use crate::model::host_draft::MAX_QUICK_HOST_NAME_CHARS;
use crate::model::ui_state::{
    QuickHostAuthDraft, QuickHostAuthKind, QuickHostDraft, QuickHostDraftError,
};

use super::super::common::host_id;

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
fn quick_host_draft_limits_alias_length() {
    let draft = QuickHostDraft {
        name: "一".repeat(MAX_QUICK_HOST_NAME_CHARS + 6),
        address: "example.com".to_owned(),
        username: "root".to_owned(),
        ..QuickHostDraft::default()
    };

    let host = draft.build_host(host_id()).expect("host should be valid");

    assert_eq!(host.name.chars().count(), MAX_QUICK_HOST_NAME_CHARS);
}
