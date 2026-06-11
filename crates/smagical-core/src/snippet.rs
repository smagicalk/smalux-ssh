//! 快捷命令和变量渲染模型。
//!
//! 片段是可保存、可复用的命令模板。模板变量使用 `{{name}}` 语法，渲染时只替换显式声明
//! 的变量，遇到未知占位符会报错，避免误把未填写的命令发到远端。

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    Host, HostId, SnippetGroupId, SnippetId, SnippetImplementationId, SnippetSupportTargetId,
};

/// 默认脚本实现名称。
pub const DEFAULT_SNIPPET_IMPLEMENTATION_NAME: &str = "默认脚本";
/// 默认支持目标标记；第一版用 Linux 通用脚本作为新片段入口。
pub const DEFAULT_SNIPPET_SUPPORT_TARGET_KEY: &str = "linux";
/// 默认支持目标展示名。
pub const DEFAULT_SNIPPET_SUPPORT_TARGET_NAME: &str = "Linux";

/// 快捷命令分组。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnippetGroup {
    /// 分组稳定 ID。
    pub id: SnippetGroupId,
    /// 显示名称。
    pub name: String,
    /// 父分组，None 表示位于片段根目录。
    pub parent_id: Option<SnippetGroupId>,
    /// 同级排序值，UI 可用于拖动排序。
    #[serde(default)]
    pub sort_order: i32,
}

/// 快捷命令。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snippet {
    /// 片段稳定 ID。
    pub id: SnippetId,
    /// 显示名称。
    pub name: String,
    /// 可选说明，给 UI 列表和编辑器展示。
    pub description: Option<String>,
    /// 可用范围：全局或指定主机。
    pub scope: SnippetScope,
    /// 所属片段分组，None 表示位于根目录。
    #[serde(default)]
    pub group_id: Option<SnippetGroupId>,
    /// 片段级共享变量定义。
    pub variables: Vec<SnippetVariable>,
    /// 脚本实现，真正保存命令模板。
    #[serde(default)]
    pub implementations: Vec<SnippetImplementation>,
    /// 支持目标，树上展示的系统/环境入口。
    #[serde(default)]
    pub support_targets: Vec<SnippetSupportTarget>,
}

impl Snippet {
    /// 创建带一个默认脚本实现和一个默认支持目标的片段。
    pub fn with_default_implementation(
        id: SnippetId,
        name: String,
        description: Option<String>,
        scope: SnippetScope,
        group_id: Option<SnippetGroupId>,
        command_template: String,
    ) -> Self {
        let variables = variables_from_template(&command_template);
        let implementation_id = SnippetImplementationId(Uuid::new_v4());
        let support_target_id = SnippetSupportTargetId(Uuid::new_v4());
        Self {
            id,
            name,
            description,
            scope,
            group_id,
            variables,
            implementations: vec![SnippetImplementation {
                id: implementation_id,
                snippet_id: id,
                name: DEFAULT_SNIPPET_IMPLEMENTATION_NAME.to_owned(),
                shell: SnippetShell::Bash,
                command_template,
                notes: None,
                last_arguments: Vec::new(),
                sort_order: 0,
            }],
            support_targets: vec![SnippetSupportTarget {
                id: support_target_id,
                snippet_id: id,
                target_key: DEFAULT_SNIPPET_SUPPORT_TARGET_KEY.to_owned(),
                display_name: DEFAULT_SNIPPET_SUPPORT_TARGET_NAME.to_owned(),
                implementation_id,
                sort_order: 0,
            }],
        }
    }

    /// 返回默认支持目标指向的实现；旧数据缺少支持目标时回退到第一个实现。
    pub fn default_implementation(&self) -> Option<&SnippetImplementation> {
        self.support_targets
            .iter()
            .min_by_key(|target| target.sort_order)
            .and_then(|target| self.implementation_by_id(target.implementation_id))
            .or_else(|| {
                self.implementations
                    .iter()
                    .min_by_key(|implementation| implementation.sort_order)
            })
    }

    /// 返回默认支持目标指向的可变实现；旧数据缺少支持目标时回退到第一个实现。
    pub fn default_implementation_mut(&mut self) -> Option<&mut SnippetImplementation> {
        let implementation_id = self
            .support_targets
            .iter()
            .min_by_key(|target| target.sort_order)
            .map(|target| target.implementation_id);
        if let Some(implementation_id) = implementation_id {
            if let Some(index) = self
                .implementations
                .iter()
                .position(|implementation| implementation.id == implementation_id)
            {
                return self.implementations.get_mut(index);
            }
        }
        self.implementations
            .iter_mut()
            .min_by_key(|implementation| implementation.sort_order)
    }

    /// 按支持目标标记查找脚本实现。
    pub fn implementation_for_target(&self, target_key: &str) -> Option<&SnippetImplementation> {
        self.support_targets
            .iter()
            .find(|target| target.target_key == target_key)
            .and_then(|target| self.implementation_by_id(target.implementation_id))
    }

    /// 默认命令模板，供旧 UI 表单和搜索列表读取。
    pub fn default_command_template(&self) -> &str {
        self.default_implementation()
            .map(|implementation| implementation.command_template.as_str())
            .unwrap_or_default()
    }

    fn implementation_by_id(
        &self,
        implementation_id: SnippetImplementationId,
    ) -> Option<&SnippetImplementation> {
        self.implementations
            .iter()
            .find(|implementation| implementation.id == implementation_id)
    }
}

/// 快捷命令脚本实现。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnippetImplementation {
    /// 实现稳定 ID。
    pub id: SnippetImplementationId,
    /// 所属片段。
    pub snippet_id: SnippetId,
    /// 显示名称，例如“通用 Linux 脚本”。
    pub name: String,
    /// 执行 shell，例如 bash、sh、powershell。
    pub shell: SnippetShell,
    /// 原始命令模板。
    pub command_template: String,
    /// 可选备注。
    #[serde(default)]
    pub notes: Option<String>,
    /// 上一次使用的参数，便于下次打开时回填。
    #[serde(default)]
    pub last_arguments: Vec<SnippetArgument>,
    /// 同级排序值。
    #[serde(default)]
    pub sort_order: i32,
}

impl SnippetImplementation {
    /// 渲染快捷命令模板。
    pub fn render(
        &self,
        variables: &[SnippetVariable],
        arguments: &[SnippetArgument],
    ) -> Result<String, SnippetRenderError> {
        let mut rendered = self.command_template.clone();

        for variable in variables {
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

/// 快捷命令支持目标。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnippetSupportTarget {
    /// 目标稳定 ID。
    pub id: SnippetSupportTargetId,
    /// 所属片段。
    pub snippet_id: SnippetId,
    /// 目标标记，例如 linux、debian-ubuntu、windows-powershell。
    pub target_key: String,
    /// 显示名称。
    pub display_name: String,
    /// 指向的脚本实现；多个目标可以共享一个实现。
    pub implementation_id: SnippetImplementationId,
    /// 同级排序值。
    #[serde(default)]
    pub sort_order: i32,
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
}

impl SnippetScope {
    /// 判断快捷命令是否可用于指定主机。
    pub fn applies_to_host(&self, host: &Host) -> bool {
        match self {
            Self::Global => true,
            Self::Host(host_id) => *host_id == host.id,
        }
    }
}

/// 脚本执行 shell。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SnippetShell {
    Sh,
    Bash,
    Zsh,
    PowerShell,
    Cmd,
    Custom(String),
}

impl SnippetShell {
    pub fn key(&self) -> &str {
        match self {
            Self::Sh => "sh",
            Self::Bash => "bash",
            Self::Zsh => "zsh",
            Self::PowerShell => "powershell",
            Self::Cmd => "cmd",
            Self::Custom(value) => value.as_str(),
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
    use crate::GroupId;
    use uuid::Uuid;

    #[test]
    fn snippet_renders_required_default_and_optional_variables() {
        let mut snippet = Snippet::with_default_implementation(
            SnippetId(Uuid::new_v4()),
            "restart service".to_owned(),
            None,
            SnippetScope::Global,
            None,
            "systemctl restart {{service}} {{suffix}} --user={{user}}".to_owned(),
        );
        snippet.variables[1].required = false;
        snippet.variables[2].default_value = Some("root".to_owned());
        snippet.variables[2].required = false;

        let rendered = snippet
            .default_implementation()
            .expect("默认实现应存在")
            .render(
                &snippet.variables,
                &[SnippetArgument {
                    name: "service".to_owned(),
                    value: "sshd".to_owned(),
                }],
            )
            .expect("快捷命令应该可以渲染");

        assert_eq!(rendered, "systemctl restart sshd  --user=root");
    }

    #[test]
    fn snippet_render_reports_missing_and_unknown_variables() {
        let missing = Snippet::with_default_implementation(
            SnippetId(Uuid::new_v4()),
            "missing".to_owned(),
            None,
            SnippetScope::Global,
            None,
            "echo {{name}}".to_owned(),
        );
        let mut unknown = Snippet::with_default_implementation(
            SnippetId(Uuid::new_v4()),
            "unknown".to_owned(),
            None,
            SnippetScope::Global,
            None,
            "echo {{declared}} {{extra}}".to_owned(),
        );
        unknown
            .variables
            .retain(|variable| variable.name == "declared");
        unknown.variables[0].default_value = Some("ok".to_owned());
        unknown.variables[0].required = false;

        assert_eq!(
            missing
                .default_implementation()
                .expect("默认实现应存在")
                .render(&missing.variables, &[]),
            Err(SnippetRenderError::MissingVariable("name".to_owned()))
        );
        assert_eq!(
            unknown
                .default_implementation()
                .expect("默认实现应存在")
                .render(&unknown.variables, &[]),
            Err(SnippetRenderError::UnknownVariable("extra".to_owned()))
        );
    }

    #[test]
    fn snippet_render_treats_empty_required_argument_as_missing() {
        let snippet = Snippet::with_default_implementation(
            SnippetId(Uuid::new_v4()),
            "restart".to_owned(),
            None,
            SnippetScope::Global,
            None,
            "systemctl restart {{service}}".to_owned(),
        );

        assert_eq!(
            snippet
                .default_implementation()
                .expect("默认实现应存在")
                .render(
                    &snippet.variables,
                    &[SnippetArgument {
                        name: "service".to_owned(),
                        value: "  ".to_owned(),
                    }]
                ),
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
        let mut snippet = Snippet::with_default_implementation(
            SnippetId(Uuid::new_v4()),
            "tail logs".to_owned(),
            Some("查看服务日志".to_owned()),
            SnippetScope::Host(HostId(Uuid::new_v4())),
            None,
            "tail -f {{path}}".to_owned(),
        );
        snippet.variables[0].default_value = Some("/var/log/syslog".to_owned());
        snippet
            .default_implementation_mut()
            .expect("默认实现应存在")
            .last_arguments = vec![SnippetArgument {
            name: "path".to_owned(),
            value: "/var/log/auth.log".to_owned(),
        }];

        let encoded = toml::to_string(&snippet).expect("快捷命令应该可以序列化为 TOML");
        let decoded: Snippet = toml::from_str(&encoded).expect("快捷命令应该可以从 TOML 反序列化");

        assert_eq!(decoded, snippet);
    }

    #[test]
    fn snippet_scope_matches_global_and_host() {
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
            network: crate::HostNetworkSelection::default(),
            proxies: Vec::new(),
            jumps: Vec::new(),
            theme_override: None,
            background_override: None,
        };

        assert!(SnippetScope::Global.applies_to_host(&host));
        assert!(SnippetScope::Host(host_id).applies_to_host(&host));
        assert!(!SnippetScope::Host(HostId(Uuid::new_v4())).applies_to_host(&host));
    }
}
