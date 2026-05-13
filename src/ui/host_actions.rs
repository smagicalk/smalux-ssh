//! 主机操作区视图。

use iced::{
    Element, Length,
    widget::{button, column, row, text, text_input},
};

use crate::model::{AppState, Host, HostId, Message, QuickHostDraftField, TunnelRule};

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
            quick_host_input(
                "agent key hint",
                &draft.key_hint,
                QuickHostDraftField::KeyHint
            ),
            quick_host_input("tags", &draft.tags, QuickHostDraftField::Tags),
        ]
        .spacing(8),
        button("Save agent host").on_press(Message::SaveQuickHost),
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
        ]
        .spacing(8),
    ]
    .spacing(8);

    if !state.storage.tunnel_rules.is_empty() {
        content = content.push(tunnel_rules(host_id, &state.storage.tunnel_rules));
    }

    content.into()
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

    format!("{}:{} | tags: {}", host.address, host.port, tags)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AuthProfile, SecretRef};
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
            "staging.example.com:22 | tags: linux, prod"
        );
    }

    #[test]
    fn host_action_view_accepts_populated_state() {
        let mut state = AppState::default();
        state.storage.upsert_host(host(Vec::new()));

        let _element = view(&state);
    }

    #[test]
    fn quick_host_form_accepts_default_state() {
        let state = AppState::default();

        let _element = quick_host_form(&state);
    }
}
