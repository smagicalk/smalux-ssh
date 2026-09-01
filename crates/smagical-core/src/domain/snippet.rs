//! 代码片段与多层层级分组领域模型。
//!
//! 支持代码片段的层级文件夹分类、多语言标记、动态模板占位符提取与参数化渲染。

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// 动态模板参数变量定义 (从 `{{key}}` 或 `{{key:default}}` 中提取)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnippetVariable {
    /// 占位符变量唯一键名 (如 "port", "container_name")
    pub key: String,
    /// 显示标签文案
    pub label: String,
    /// 缺省预填默认值 (可选)
    pub default_value: Option<String>,
}

/// 代码片段层级文件夹分组实体
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnippetGroupRecord {
    /// 分组全局唯一 ID (如 "sgrp-docker", "sgrp-linux")
    pub id: String,
    /// 分组名称 (如 "Docker 容器运维", "K8s 集群管理")
    pub name: String,
    /// 父级分组 ID (None 表示顶级分组，支持无限级多层嵌套)
    pub parent_id: Option<String>,
    /// 嵌套深度层级 (0 为根目录)
    pub level: u32,
    /// 当前是否处于展开状态
    pub is_expanded: bool,
    /// 显示排序权重 (数字越小越靠前)
    pub sort_order: i32,
}

impl SnippetGroupRecord {
    /// 构造顶级根目录分组
    pub fn root(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            parent_id: None,
            level: 0,
            is_expanded: true,
            sort_order: 0,
        }
    }

    /// 构造子级嵌套分组
    pub fn child(
        id: impl Into<String>,
        name: impl Into<String>,
        parent_id: impl Into<String>,
        level: u32,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            parent_id: Some(parent_id.into()),
            level,
            is_expanded: true,
            sort_order: 0,
        }
    }
}

/// 代码片段资产实体
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnippetRecord {
    /// 代码片段全局唯一 ID (如 "snip-docker-ps", "snip-nginx-reload")
    pub id: String,
    /// 所属文件夹分组 ID (None 表示存放在根目录下)
    pub parent_group_id: Option<String>,
    /// 代码片段标题名称
    pub title: String,
    /// 代码片段/脚本主体内容 (支持多行)
    pub content: String,
    /// 脚本语言类型 (如 "bash", "sh", "powershell", "python", "sql", "yaml")
    pub language: String,
    /// 标签分类列表 (如 ["docker", "ops", "monitor"])
    pub tags: Vec<String>,
    /// 注入终端后是否自动发送回车立即执行 (true: 立即执行; false: 仅粘贴到光标处)
    pub auto_execute: bool,
    /// 详细说明与使用备注
    pub description: String,
    /// 是否星标置顶
    pub is_favorite: bool,
    /// 显示排序权重
    pub sort_order: i32,
    /// 最后修改时间戳 (ISO 8601 格式)
    pub updated_at: String,
}

impl SnippetRecord {
    /// 创建一个新的代码片段
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        content: impl Into<String>,
        language: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            parent_group_id: None,
            title: title.into(),
            content: content.into(),
            language: language.into(),
            tags: Vec::new(),
            auto_execute: true,
            description: String::new(),
            is_favorite: false,
            sort_order: 0,
            updated_at: "2026-09-01T20:00:00Z".to_string(),
        }
    }

    /// 设置所属文件夹分组
    pub fn with_group(mut self, group_id: impl Into<String>) -> Self {
        self.parent_group_id = Some(group_id.into());
        self
    }

    /// 设置标签列表
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// 设置描述说明
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// 设置自动执行模式
    pub fn with_auto_execute(mut self, auto: bool) -> Self {
        self.auto_execute = auto;
        self
    }

    /// 从代码内容中自动提取所有动态参数占位符 `{{key}}` 或 `{{key:default}}`
    pub fn extract_variables(&self) -> Vec<SnippetVariable> {
        let mut vars = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let text = &self.content;

        let mut start_idx = 0;
        while let Some(open_pos) = text[start_idx..].find("{{") {
            let actual_open = start_idx + open_pos + 2;
            if let Some(close_pos) = text[actual_open..].find("}}") {
                let actual_close = actual_open + close_pos;
                let raw_token = text[actual_open..actual_close].trim();
                if !raw_token.is_empty() {
                    let (key, default_val) = if let Some((k, d)) = raw_token.split_once(':') {
                        (k.trim().to_string(), Some(d.trim().to_string()))
                    } else if let Some((k, d)) = raw_token.split_once('=') {
                        (k.trim().to_string(), Some(d.trim().to_string()))
                    } else {
                        (raw_token.to_string(), None)
                    };

                    if !key.is_empty() && !seen.contains(&key) {
                        seen.insert(key.clone());
                        vars.push(SnippetVariable {
                            label: key.clone(),
                            key,
                            default_value: default_val,
                        });
                    }
                }
                start_idx = actual_close + 2;
            } else {
                break;
            }
        }
        vars
    }

    /// 根据用户提供的参数映射表渲染生成最终的可执行命令字符串
    pub fn render_content(&self, params: &HashMap<String, String>) -> String {
        let mut rendered = self.content.clone();
        for var in self.extract_variables() {
            let user_val = params.get(&var.key)
                .cloned()
                .or_else(|| var.default_value.clone())
                .unwrap_or_default();

            // 替换无默认值形式 {{key}}
            let pattern1 = format!("{{{{{}}}}}", var.key);
            rendered = rendered.replace(&pattern1, &user_val);

            // 替换带默认值形式 {{key:...}}
            if let Some(ref def) = var.default_value {
                let pattern2 = format!("{{{{{}:{}}}}}", var.key, def);
                rendered = rendered.replace(&pattern2, &user_val);
                let pattern3 = format!("{{{{{}: {}}}}}", var.key, def);
                rendered = rendered.replace(&pattern3, &user_val);
                let pattern4 = format!("{{{{{}: {}}}}}", var.key, def);
                rendered = rendered.replace(&pattern4, &user_val);
                let pattern5 = format!("{{{{{}: {} }}}}", var.key, def);
                rendered = rendered.replace(&pattern5, &user_val);
                let pattern_eq = format!("{{{{{}= {}}}}}", var.key, def);
                rendered = rendered.replace(&pattern_eq, &user_val);
                let pattern_eq2 = format!("{{{{{}: {} }}}}", var.key, def);
                rendered = rendered.replace(&pattern_eq2, &user_val);
            }
        }
        rendered
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snippet_variable_extraction_and_rendering() {
        let snippet = SnippetRecord::new(
            "snip-docker-log",
            "Docker 日志查看",
            "docker logs -f --tail={{lines:100}} {{container_name}}",
            "bash",
        );

        let vars = snippet.extract_variables();
        assert_eq!(vars.len(), 2);
        assert_eq!(vars[0].key, "lines");
        assert_eq!(vars[0].default_value, Some("100".to_string()));
        assert_eq!(vars[1].key, "container_name");
        assert_eq!(vars[1].default_value, None);

        let mut params = HashMap::new();
        params.insert("container_name".to_string(), "nginx-proxy".to_string());
        // lines 未提供，自动使用 default 100
        let rendered = snippet.render_content(&params);
        assert_eq!(rendered, "docker logs -f --tail=100 nginx-proxy");

        // lines 提供自定义值 50
        params.insert("lines".to_string(), "50".to_string());
        let rendered2 = snippet.render_content(&params);
        assert_eq!(rendered2, "docker logs -f --tail=50 nginx-proxy");
    }

    #[test]
    fn test_snippet_groups_hierarchy() {
        let root = SnippetGroupRecord::root("sgrp-ops", "常用运维");
        let child = SnippetGroupRecord::child("sgrp-ops-docker", "Docker", "sgrp-ops", 1);

        assert_eq!(root.level, 0);
        assert!(root.parent_id.is_none());
        assert_eq!(child.level, 1);
        assert_eq!(child.parent_id.as_deref(), Some("sgrp-ops"));
    }
}
