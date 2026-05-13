//! 主机操作区视图。

use iced::{
    Element, Length,
    widget::{button, column, row, text, text_input},
};

use crate::model::{
    AppState, AuthProfile, Host, HostId, Message, QuickHostAuthDraft, QuickHostAuthField,
    QuickHostAuthKind, QuickHostDraftField, Snippet, TunnelRule, VisualSettingsDraftField,
};

/// 渲染所有已保存主机的最小可用操作入口。
pub fn view(state: &AppState) -> Element<'_, Message> {
    let mut hosts = column![text("Hosts").size(22), quick_host_form(state)].spacing(12);

    if state.storage.hosts.is_empty() {
        return hosts.push(text("No saved hosts yet.")).into();
    }

    for host in &state.storage.hosts {
        hosts = hosts.push(host_card(state, host));
    }

    hosts.into()
}

fn quick_host_form(state: &AppState) -> Element<'_, Message> {
    let draft = &state.ui.quick_host;

    column![
        text("Quick host").size(18),
        row![
            quick_host_input("name", &draft.name, QuickHostDraftField::Name),
            quick_host_input("address", &draft.address, QuickHostDraftField::Address),
            quick_host_input("port", &draft.port, QuickHostDraftField::Port),
        ]
        .spacing(8),
        row![
            quick_host_input("username", &draft.username, QuickHostDraftField::Username),
            quick_host_input("tags", &draft.tags, QuickHostDraftField::Tags),
        ]
        .spacing(8),
        auth_kind_selector(draft.auth.kind),
        quick_host_auth_inputs(&draft.auth),
        button("Save host").on_press(Message::SaveQuickHost),
    ]
    .spacing(8)
    .into()
}

fn quick_host_input<'a>(
    placeholder: &'a str,
    value: &'a str,
    field: QuickHostDraftField,
) -> Element<'a, Message> {
    text_input(placeholder, value)
        .on_input(move |value| Message::UpdateQuickHostDraft { field, value })
        .width(Length::Fill)
        .into()
}

fn auth_kind_selector(selected: QuickHostAuthKind) -> Element<'static, Message> {
    row![
        auth_kind_button(QuickHostAuthKind::Password, selected),
        auth_kind_button(QuickHostAuthKind::Key, selected),
        auth_kind_button(QuickHostAuthKind::Agent, selected),
        auth_kind_button(QuickHostAuthKind::Certificate, selected),
    ]
    .spacing(8)
    .into()
}

fn auth_kind_button(
    kind: QuickHostAuthKind,
    selected: QuickHostAuthKind,
) -> Element<'static, Message> {
    let label = if kind == selected {
        format!("* {}", kind.label())
    } else {
        kind.label().to_owned()
    };

    button(text(label))
        .on_press(Message::UpdateQuickHostAuthKind { kind })
        .into()
}

fn quick_host_auth_inputs(auth: &QuickHostAuthDraft) -> Element<'_, Message> {
    match auth.kind {
        QuickHostAuthKind::Password => row![quick_host_auth_input(
            "password secret ref",
            &auth.password_secret_ref,
            QuickHostAuthField::PasswordSecretRef,
        )]
        .spacing(8)
        .into(),
        QuickHostAuthKind::Key => row![
            quick_host_auth_input(
                "private key ref",
                &auth.private_key_ref,
                QuickHostAuthField::PrivateKeyRef,
            ),
            quick_host_auth_input(
                "passphrase ref",
                &auth.passphrase_ref,
                QuickHostAuthField::PassphraseRef,
            ),
        ]
        .spacing(8)
        .into(),
        QuickHostAuthKind::Agent => row![quick_host_auth_input(
            "agent key hint",
            &auth.key_hint,
            QuickHostAuthField::KeyHint,
        )]
        .spacing(8)
        .into(),
        QuickHostAuthKind::Certificate => row![
            quick_host_auth_input(
                "private key ref",
                &auth.private_key_ref,
                QuickHostAuthField::PrivateKeyRef,
            ),
            quick_host_auth_input(
                "certificate ref",
                &auth.certificate_ref,
                QuickHostAuthField::CertificateRef,
            ),
        ]
        .spacing(8)
        .into(),
    }
}

fn quick_host_auth_input<'a>(
    placeholder: &'a str,
    value: &'a str,
    field: QuickHostAuthField,
) -> Element<'a, Message> {
    text_input(placeholder, value)
        .on_input(move |value| Message::UpdateQuickHostAuthField { field, value })
        .width(Length::Fill)
        .into()
}

fn host_card<'a>(state: &'a AppState, host: &'a Host) -> Element<'a, Message> {
    let host_id = host.id;
    let command_value = state.ui.remote_command_for(host_id);
    let command_for_exec = command_value.to_owned();
    let command_for_pty = command_value.to_owned();
    let sftp_dir_value = state.ui.sftp_initial_dir_for(host_id);
    let sftp_dir_for_open = sftp_dir_value.to_owned();

    let command_input = text_input("remote command", command_value)
        .on_input(move |command| Message::UpdateHostCommandDraft { host_id, command })
        .width(Length::Fill);
    let sftp_input = text_input("SFTP path", sftp_dir_value)
        .on_input(move |initial_dir| Message::UpdateHostSftpInitialDirDraft {
            host_id,
            initial_dir,
        })
        .width(Length::Fill);

    let mut content = column![
        text(host_title(host)).size(18),
        text(host_subtitle(host)),
        row![command_input, sftp_input].spacing(8),
        row![
            button("Shell").on_press(Message::OpenShell { host_id }),
            button("SFTP").on_press(Message::OpenSftp {
                host_id,
                initial_dir: sftp_dir_for_open,
            }),
            button("Run").on_press(Message::RunRemoteCommand {
                host_id,
                command: command_for_exec,
                request_pty: false,
            }),
            button("Run PTY").on_press(Message::RunRemoteCommand {
                host_id,
                command: command_for_pty,
                request_pty: true,
            }),
            button("Save snippet").on_press(Message::SaveHostCommandSnippet { host_id }),
        ]
        .spacing(8),
        host_visual_settings(state, host),
    ]
    .spacing(8);

    let snippets = host_snippets(state, host);
    if !snippets.is_empty() {
        content = content.push(snippet_list(host_id, snippets));
    }

    if !state.storage.tunnel_rules.is_empty() {
        content = content.push(tunnel_rules(host_id, &state.storage.tunnel_rules));
    }

    content.into()
}

fn host_snippets<'a>(state: &'a AppState, host: &'a Host) -> Vec<&'a Snippet> {
    state
        .storage
        .snippets
        .iter()
        .filter(|snippet| snippet.scope.applies_to_host(host))
        .collect()
}

fn snippet_list<'a>(host_id: HostId, snippets: Vec<&'a Snippet>) -> Element<'a, Message> {
    let mut rows = column![text("Snippets")].spacing(6);

    for snippet in snippets {
        let mut snippet_rows = column![
            row![
                text(snippet_label(snippet)),
                button("Run").on_press(Message::RunSnippet {
                    host_id,
                    snippet_id: snippet.id,
                }),
                button("Remove").on_press(Message::RemoveSnippet {
                    snippet_id: snippet.id,
                }),
            ]
            .spacing(8)
        ]
        .spacing(6);

        if !snippet.variables.is_empty() {
            snippet_rows = snippet_rows.push(snippet_argument_inputs(snippet));
        }

        rows = rows.push(snippet_rows);
    }

    rows.into()
}

fn snippet_argument_inputs<'a>(snippet: &'a Snippet) -> Element<'a, Message> {
    let mut inputs = row![text("Args")].spacing(8);

    for variable in &snippet.variables {
        let snippet_id = snippet.id;
        let name = variable.name.clone();
        inputs = inputs.push(
            text_input(
                &variable.name,
                snippet_argument_value(snippet, &variable.name),
            )
            .on_input(move |value| Message::UpdateSnippetArgument {
                snippet_id,
                name: name.clone(),
                value,
            })
            .width(Length::Fill),
        );
    }

    inputs.into()
}

fn snippet_argument_value<'a>(snippet: &'a Snippet, name: &str) -> &'a str {
    snippet
        .last_arguments
        .iter()
        .find(|argument| argument.name == name)
        .map(|argument| argument.value.as_str())
        .or_else(|| {
            snippet
                .variables
                .iter()
                .find(|variable| variable.name == name)
                .and_then(|variable| variable.default_value.as_deref())
        })
        .unwrap_or("")
}

fn snippet_label(snippet: &Snippet) -> String {
    format!("{} | {}", snippet.name, snippet.command_template)
}

fn host_visual_settings<'a>(state: &'a AppState, host: &'a Host) -> Element<'a, Message> {
    let host_id = host.id;
    let draft = state.ui.host_visual_settings_for(host_id);
    let override_state = if host.theme_override.is_some() || host.background_override.is_some() {
        "override"
    } else {
        "global"
    };
    let background_enabled = draft
        .map(|draft| draft.background_enabled)
        .or_else(|| {
            host.background_override
                .as_ref()
                .map(|background| background.enabled)
        })
        .unwrap_or(state.config.background.enabled);

    column![
        text(format!("Host visual | {override_state}")),
        row![
            host_visual_input(
                host_id,
                "theme",
                draft
                    .map(|draft| draft.theme_name.as_str())
                    .unwrap_or(host_theme_name(host)),
                VisualSettingsDraftField::ThemeName,
            ),
            host_visual_input(
                host_id,
                "font",
                draft
                    .map(|draft| draft.font_family.as_str())
                    .unwrap_or(host_font_family(host)),
                VisualSettingsDraftField::FontFamily,
            ),
            host_visual_input(
                host_id,
                "size",
                draft.map(|draft| draft.font_size.as_str()).unwrap_or(""),
                VisualSettingsDraftField::FontSize,
            ),
        ]
        .spacing(8),
        row![
            button(if background_enabled {
                "Background on"
            } else {
                "Background off"
            })
            .on_press(Message::SetHostVisualBackgroundEnabled {
                host_id,
                enabled: !background_enabled,
            }),
            host_visual_input(
                host_id,
                "sources",
                draft
                    .map(|draft| draft.background_sources.as_str())
                    .unwrap_or(""),
                VisualSettingsDraftField::BackgroundSources,
            ),
        ]
        .spacing(8),
        row![
            host_visual_input(
                host_id,
                "rotation",
                draft
                    .map(|draft| draft.rotation_interval_secs.as_str())
                    .unwrap_or(""),
                VisualSettingsDraftField::RotationIntervalSecs,
            ),
            host_visual_input(
                host_id,
                "opacity",
                draft.map(|draft| draft.opacity.as_str()).unwrap_or(""),
                VisualSettingsDraftField::Opacity,
            ),
            host_visual_input(
                host_id,
                "blur",
                draft.map(|draft| draft.blur.as_str()).unwrap_or(""),
                VisualSettingsDraftField::Blur,
            ),
            button("Apply host visual").on_press(Message::ApplyHostVisualSettings { host_id }),
            button("Use global").on_press(Message::ClearHostVisualSettings { host_id }),
        ]
        .spacing(8),
    ]
    .spacing(8)
    .into()
}

fn host_theme_name(host: &Host) -> &str {
    host.theme_override
        .as_ref()
        .map(|theme| theme.name.as_str())
        .unwrap_or("")
}

fn host_font_family(host: &Host) -> &str {
    host.theme_override
        .as_ref()
        .map(|theme| theme.font_family.as_str())
        .unwrap_or("")
}

fn host_visual_input<'a>(
    host_id: HostId,
    placeholder: &'a str,
    value: &'a str,
    field: VisualSettingsDraftField,
) -> Element<'a, Message> {
    text_input(placeholder, value)
        .on_input(move |value| Message::UpdateHostVisualSettingsDraft {
            host_id,
            field,
            value,
        })
        .width(Length::Fill)
        .into()
}

fn tunnel_rules<'a>(host_id: HostId, rules: &'a [TunnelRule]) -> Element<'a, Message> {
    let mut rows = column![text("Tunnel rules")].spacing(6);

    for rule in rules {
        let rule_to_start = rule.clone();
        rows = rows.push(
            row![
                text(format!("{} | {}", rule.name, rule.display_endpoint())),
                button("Start").on_press(Message::StartTunnel {
                    host_id,
                    rule: rule_to_start,
                }),
            ]
            .spacing(8),
        );
    }

    rows.into()
}

fn host_title(host: &Host) -> String {
    format!("{} ({})", host.name, host.address)
}

fn host_subtitle(host: &Host) -> String {
    let tags = if host.tags.is_empty() {
        "no tags".to_owned()
    } else {
        host.tags.join(", ")
    };

    format!(
        "{}:{} | auth: {} | tags: {}",
        host.address,
        host.port,
        auth_label(&host.auth),
        tags
    )
}

fn auth_label(auth: &AuthProfile) -> &'static str {
    match auth {
        AuthProfile::Password { .. } => "password",
        AuthProfile::Key { .. } => "key",
        AuthProfile::Agent { .. } => "ssh-agent",
        AuthProfile::Certificate { .. } => "certificate",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AuthProfile, SecretRef, SnippetArgument, SnippetScope, SnippetVariable};
    use uuid::Uuid;

    fn host(tags: Vec<String>) -> Host {
        Host {
            id: HostId(Uuid::new_v4()),
            name: "staging".to_owned(),
            group_id: None,
            tags,
            address: "staging.example.com".to_owned(),
            port: 22,
            auth: AuthProfile::Password {
                username: "ops".to_owned(),
                secret: SecretRef("password:ops".to_owned()),
            },
            proxy: None,
            jumps: Vec::new(),
            theme_override: None,
            background_override: None,
        }
    }

    #[test]
    fn host_subtitle_includes_endpoint_and_tags() {
        let host = host(vec!["linux".to_owned(), "prod".to_owned()]);

        assert_eq!(
            host_subtitle(&host),
            "staging.example.com:22 | auth: password | tags: linux, prod"
        );
    }

    #[test]
    fn host_action_view_accepts_populated_state() {
        let mut state = AppState::default();
        let host = host(Vec::new());
        let host_id = host.id;
        state.storage.upsert_host(host);
        state.storage.upsert_snippet(Snippet {
            id: crate::model::SnippetId(Uuid::new_v4()),
            name: "uptime".to_owned(),
            description: None,
            command_template: "systemctl restart {{service}}".to_owned(),
            scope: SnippetScope::Host(host_id),
            variables: vec![SnippetVariable {
                name: "service".to_owned(),
                default_value: None,
                required: true,
            }],
            last_arguments: vec![SnippetArgument {
                name: "service".to_owned(),
                value: "sshd".to_owned(),
            }],
        });

        let _element = view(&state);
    }

    #[test]
    fn snippet_argument_value_prefers_last_argument_then_default() {
        let snippet = Snippet {
            id: crate::model::SnippetId(Uuid::new_v4()),
            name: "restart".to_owned(),
            description: None,
            command_template: "systemctl restart {{service}} {{mode}}".to_owned(),
            scope: SnippetScope::Global,
            variables: vec![
                SnippetVariable {
                    name: "service".to_owned(),
                    default_value: Some("nginx".to_owned()),
                    required: true,
                },
                SnippetVariable {
                    name: "mode".to_owned(),
                    default_value: Some("--now".to_owned()),
                    required: false,
                },
            ],
            last_arguments: vec![SnippetArgument {
                name: "service".to_owned(),
                value: "sshd".to_owned(),
            }],
        };

        assert_eq!(snippet_argument_value(&snippet, "service"), "sshd");
        assert_eq!(snippet_argument_value(&snippet, "mode"), "--now");
        assert_eq!(snippet_argument_value(&snippet, "missing"), "");
    }

    #[test]
    fn quick_host_form_accepts_default_state() {
        let state = AppState::default();

        let _element = quick_host_form(&state);
    }

    #[test]
    fn quick_host_form_accepts_certificate_auth_state() {
        let mut state = AppState::default();
        state
            .ui
            .set_quick_host_auth_kind(QuickHostAuthKind::Certificate);
        state.ui.set_quick_host_auth_field(
            QuickHostAuthField::PrivateKeyRef,
            "key:cert-user".to_owned(),
        );
        state.ui.set_quick_host_auth_field(
            QuickHostAuthField::CertificateRef,
            "cert:cert-user".to_owned(),
        );

        let _element = quick_host_form(&state);
    }
}
