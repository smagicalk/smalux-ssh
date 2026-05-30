//! 快捷命令和变量渲染模型。
//!
//! 片段是可保存、可复用的命令模板。模板变量使用 `{{name}}` 语法，渲染时只替换显式声明
//! 的变量，遇到未知占位符会报错，避免误把未填写的命令发到远端。

use serde::{Deserialize, Serialize};

use crate::{GroupId, Host, HostId, SnippetId};

/// 快捷命令。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snippet {
    /// 片段稳定 ID。
    pub id: SnippetId,
    /// 显示名称。
    pub name: String,
    /// 可选说明，给 UI 列表和编辑器展示。
    pub description: Option<String>,
    /// 原始命令模板。
    pub command_template: String,
    /// 可用范围：全局、指定主机或指定分组。
    pub scope: SnippetScope,
    /// 模板变量定义。
    pub variables: Vec<SnippetVariable>,
    /// 上一次使用的参数，便于下次打开时回填。
    pub last_arguments: Vec<SnippetArgument>,
}

impl Snippet {
    /// 渲染快捷命令模板。
    pub fn render(&self, arguments: &[SnippetArgument]) -> Result<String, SnippetRenderError> {
        let mut rendered = self.command_template.clone();

        for variable in &self.variables {
            // 优先使用本次参数，其次使用默认值；必填变量空白值视为缺失。
            let argument_value = arguments
                .iter()
                .find(|argument| argument.name == variable.name)
                .and_then(|argument| non_empty_required_value(argument.value.as_str(), variable));
            let value = argument_value
                .or(variable.default_value.as_deref())
                .or(if variable.required { None } else { Some("") })
                .ok_or_else(|| SnippetRenderError::MissingVariable(variable.name.clone()))?;
            let placeholder = format!("{{{{{}}}}}", variable.name);
            rendered = rendered.replace(&placeholder, value);
        }

        // 显式检查残留占位符，防止拼错变量名时把 `{{name}}` 原样发到 shell。
        if let Some(unresolved) = find_unresolved_placeholder(&rendered) {
            return Err(SnippetRenderError::UnknownVariable(unresolved));
        }

        Ok(rendered)
    }
}

/// 从模板中提取 `{{name}}` 形式的变量定义。
pub fn variables_from_template(template: &str) -> Vec<SnippetVariable> {
    let mut variables: Vec<SnippetVariable> = Vec::new();
    let mut rest = template;

    while let Some(start) = rest.find("{{") {
        // 只接受完整闭合的 `{{name}}`；未闭合占位符交给渲染时报 UnknownVariable。
        let after_start = &rest[start + 2..];
        let Some(end) = after_start.find("}}") else {
            break;
        };
        let name = &after_start[..end];
        if name == name.trim()
            && is_valid_variable_name(name)
            && !variables.iter().any(|variable| variable.name == name)
        {
            // 从模板自动提取的变量默认必填，用户可在编辑器里改成可选或加默认值。
            variables.push(SnippetVariable {
                name: name.to_owned(),
                default_value: None,
                required: true,
            });
        }
        rest = &after_start[end + 2..];
    }

    variables
}

/// 快捷命令作用域。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SnippetScope {
    /// 所有主机都可用。
    Global,
    /// 只对指定主机可用。
    Host(HostId),
    /// 只对指定分组下的主机可用。
    Group(GroupId),
}

impl SnippetScope {
    /// 判断快捷命令是否可用于指定主机。
    pub fn applies_to_host(&self, host: &Host) -> bool {
        match self {
            Self::Global => true,
            Self::Host(host_id) => *host_id == host.id,
            Self::Group(group_id) => host.group_id == Some(*group_id),
        }
    }
}

/// 快捷命令变量定义。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnippetVariable {
    /// 变量名，对应模板中的 `{{name}}`。
    pub name: String,
    /// 未提供参数时使用的默认值。
    pub default_value: Option<String>,
    /// 必填变量不能使用空白参数。
    pub required: bool,
}

/// 快捷命令参数值。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnippetArgument {
    /// 参数名，对应变量名。
    pub name: String,
    /// 用户本次输入的值。
    pub value: String,
}

/// 快捷命令渲染错误。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SnippetRenderError {
    /// 声明的必填变量没有值。
    MissingVariable(String),
    /// 模板中存在未声明的占位符。
    UnknownVariable(String),
}

fn find_unresolved_placeholder(input: &str) -> Option<String> {
    // 这里只做轻量扫描；变量名合法性由 variables_from_template 控制。
    let start = input.find("{{")?;
    let rest = &input[start + 2..];
    let end = rest.find("}}")?;
    Some(rest[..end].to_owned())
}

fn non_empty_required_value<'a>(value: &'a str, variable: &SnippetVariable) -> Option<&'a str> {
    // 必填变量中只有空白字符时视为未提供。
    if variable.required && value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

fn is_valid_variable_name(name: &str) -> bool {
    // 限制变量字符集，避免模板解析和 shell 命令之间出现歧义。
    !name.is_empty()
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn snippet_renders_required_default_and_optional_variables() {
        let snippet = Snippet {
            id: SnippetId(Uuid::new_v4()),
            name: "restart service".to_owned(),
            description: None,
            command_template: "systemctl restart {{service}} {{suffix}} --user={{user}}".to_owned(),
            scope: SnippetScope::Global,
            variables: vec![
                SnippetVariable {
                    name: "service".to_owned(),
                    default_value: None,
                    required: true,
                },
                SnippetVariable {
                    name: "suffix".to_owned(),
                    default_value: None,
                    required: false,
                },
                SnippetVariable {
                    name: "user".to_owned(),
                    default_value: Some("root".to_owned()),
                    required: false,
                },
            ],
            last_arguments: Vec::new(),
        };

        let rendered = snippet
            .render(&[SnippetArgument {
                name: "service".to_owned(),
                value: "sshd".to_owned(),
            }])
            .expect("快捷命令应该可以渲染");

        assert_eq!(rendered, "systemctl restart sshd  --user=root");
    }

    #[test]
    fn snippet_render_reports_missing_and_unknown_variables() {
        let missing = Snippet {
            id: SnippetId(Uuid::new_v4()),
            name: "missing".to_owned(),
            description: None,
            command_template: "echo {{name}}".to_owned(),
            scope: SnippetScope::Global,
            variables: vec![SnippetVariable {
                name: "name".to_owned(),
                default_value: None,
                required: true,
            }],
            last_arguments: Vec::new(),
        };
        let unknown = Snippet {
            id: SnippetId(Uuid::new_v4()),
            name: "unknown".to_owned(),
            description: None,
            command_template: "echo {{declared}} {{extra}}".to_owned(),
            scope: SnippetScope::Global,
            variables: vec![SnippetVariable {
                name: "declared".to_owned(),
                default_value: Some("ok".to_owned()),
                required: false,
            }],
            last_arguments: Vec::new(),
        };

        assert_eq!(
            missing.render(&[]),
            Err(SnippetRenderError::MissingVariable("name".to_owned()))
        );
        assert_eq!(
            unknown.render(&[]),
            Err(SnippetRenderError::UnknownVariable("extra".to_owned()))
        );
    }

    #[test]
    fn snippet_render_treats_empty_required_argument_as_missing() {
        let snippet = Snippet {
            id: SnippetId(Uuid::new_v4()),
            name: "restart".to_owned(),
            description: None,
            command_template: "systemctl restart {{service}}".to_owned(),
            scope: SnippetScope::Global,
            variables: vec![SnippetVariable {
                name: "service".to_owned(),
                default_value: None,
                required: true,
            }],
            last_arguments: Vec::new(),
        };

        assert_eq!(
            snippet.render(&[SnippetArgument {
                name: "service".to_owned(),
                value: "  ".to_owned(),
            }]),
            Err(SnippetRenderError::MissingVariable("service".to_owned()))
        );
    }

    #[test]
    fn variables_from_template_extracts_unique_valid_names() {
        let variables =
            variables_from_template("echo {{service}} {{ service }} {{bad.name}} {{service}}");

        assert_eq!(
            variables
                .iter()
                .map(|variable| variable.name.as_str())
                .collect::<Vec<_>>(),
            vec!["service"]
        );
        assert!(variables.iter().all(|variable| variable.required));
    }

    #[test]
    fn snippet_round_trips_through_toml() {
        let snippet = Snippet {
            id: SnippetId(Uuid::new_v4()),
            name: "tail logs".to_owned(),
            description: Some("查看服务日志".to_owned()),
            command_template: "tail -f {{path}}".to_owned(),
            scope: SnippetScope::Host(HostId(Uuid::new_v4())),
            variables: vec![SnippetVariable {
                name: "path".to_owned(),
                default_value: Some("/var/log/syslog".to_owned()),
                required: true,
            }],
            last_arguments: vec![SnippetArgument {
                name: "path".to_owned(),
                value: "/var/log/auth.log".to_owned(),
            }],
        };

        let encoded = toml::to_string(&snippet).expect("快捷命令应该可以序列化为 TOML");
        let decoded: Snippet = toml::from_str(&encoded).expect("快捷命令应该可以从 TOML 反序列化");

        assert_eq!(decoded, snippet);
    }

    #[test]
    fn snippet_scope_matches_global_host_and_group() {
        let host_id = HostId(Uuid::new_v4());
        let group_id = GroupId(Uuid::new_v4());
        let host = Host {
            id: host_id,
            name: "staging".to_owned(),
            group_id: Some(group_id),
            icon_key: "server".to_owned(),
            tags: Vec::new(),
            address: "staging.example.com".to_owned(),
            port: 22,
            auth: crate::AuthProfile::Agent {
                username: "ops".to_owned(),
                source: crate::AgentSource::Auto,
                key_hint: None,
            },
            proxy: None,
            jumps: Vec::new(),
            theme_override: None,
            background_override: None,
        };

        assert!(SnippetScope::Global.applies_to_host(&host));
        assert!(SnippetScope::Host(host_id).applies_to_host(&host));
        assert!(SnippetScope::Group(group_id).applies_to_host(&host));
        assert!(!SnippetScope::Host(HostId(Uuid::new_v4())).applies_to_host(&host));
    }
}
