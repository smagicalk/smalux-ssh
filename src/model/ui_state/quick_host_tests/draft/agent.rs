use crate::model::ui_state::{
    QuickHostAgentSource, QuickHostAuthDraft, QuickHostAuthKind, QuickHostDraft,
    QuickHostDraftError,
};
use crate::model::{AgentSource, AuthProfile};

use super::super::common::host_id;

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
        ..QuickHostDraft::default()
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
            source: AgentSource::Auto,
            key_hint: Some(key_hint),
        } if username == "deploy" && key_hint == "id_ed25519"
    ));
}

#[test]
fn quick_host_draft_builds_custom_agent_source() {
    let draft = QuickHostDraft {
        address: "prod.example.com".to_owned(),
        username: "deploy".to_owned(),
        auth: QuickHostAuthDraft {
            kind: QuickHostAuthKind::Agent,
            agent_source: QuickHostAgentSource::CustomNamedPipe,
            agent_custom_pipe: r"\\.\pipe\custom-agent".to_owned(),
            ..QuickHostAuthDraft::default()
        },
        ..QuickHostDraft::default()
    };

    let host = draft
        .build_host(host_id())
        .expect("自定义 agent 管道应该可以生成主机配置");

    assert!(matches!(
        host.auth,
        AuthProfile::Agent {
            source: AgentSource::CustomNamedPipe(pipe),
            ..
        } if pipe == r"\\.\pipe\custom-agent"
    ));
}

#[test]
fn quick_host_draft_rejects_empty_custom_agent_source() {
    let draft = QuickHostDraft {
        address: "prod.example.com".to_owned(),
        username: "deploy".to_owned(),
        auth: QuickHostAuthDraft {
            kind: QuickHostAuthKind::Agent,
            agent_source: QuickHostAgentSource::CustomNamedPipe,
            ..QuickHostAuthDraft::default()
        },
        ..QuickHostDraft::default()
    };

    assert_eq!(
        draft.build_host(host_id()),
        Err(QuickHostDraftError::MissingAgentPipePath)
    );
}
