//! 快捷命令和变量渲染模型。

use serde::{Deserialize, Serialize};

use super::{GroupId, HostId, SnippetId};

/// 快捷命令。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snippet {
    pub id: SnippetId,
    pub name: String,
    pub description: Option<String>,
    pub command_template: String,
    pub scope: SnippetScope,
    pub variables: Vec<SnippetVariable>,
    pub last_arguments: Vec<SnippetArgument>,
}

impl Snippet {
    /// 渲染快捷命令模板。
    pub fn render(&self, arguments: &[SnippetArgument]) -> Result<String, SnippetRenderError> {
        let mut rendered = self.command_template.clone();

        for variable in &self.variables {
            let argument_value = arguments
                .iter()
                .find(|argument| argument.name == variable.name)
                .map(|argument| argument.value.as_str());
            let value = argument_value
                .or(variable.default_value.as_deref())
                .or(if variable.required { None } else { Some("") })
                .ok_or_else(|| SnippetRenderError::MissingVariable(variable.name.clone()))?;
            let placeholder = format!("{{{{{}}}}}", variable.name);
            rendered = rendered.replace(&placeholder, value);
        }

        if let Some(unresolved) = find_unresolved_placeholder(&rendered) {
            return Err(SnippetRenderError::UnknownVariable(unresolved));
        }

        Ok(rendered)
    }
}

/// 快捷命令作用域。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SnippetScope {
    Global,
    Host(HostId),
    Group(GroupId),
}

/// 快捷命令变量定义。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnippetVariable {
    pub name: String,
    pub default_value: Option<String>,
    pub required: bool,
}

/// 快捷命令参数值。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnippetArgument {
    pub name: String,
    pub value: String,
}

/// 快捷命令渲染错误。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SnippetRenderError {
    MissingVariable(String),
    UnknownVariable(String),
}

fn find_unresolved_placeholder(input: &str) -> Option<String> {
    let start = input.find("{{")?;
    let rest = &input[start + 2..];
    let end = rest.find("}}")?;
    Some(rest[..end].to_owned())
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
}
