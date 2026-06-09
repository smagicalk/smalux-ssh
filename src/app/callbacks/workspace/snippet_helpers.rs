//! 工作区片段回调的字符串 ID 和参数解析。

use uuid::Uuid;

use crate::model::{SnippetArgument, SnippetGroupId, SnippetId, SnippetSupportTargetId};

pub(super) fn parse_optional_snippet_group_node_id(node_id: &str) -> Option<SnippetGroupId> {
    parse_snippet_group_node_id(node_id)
}

pub(super) fn parse_snippet_group_node_id(node_id: &str) -> Option<SnippetGroupId> {
    node_id
        .strip_prefix("snippet-folder:group:")
        .and_then(|id| Uuid::parse_str(id).ok())
        .map(SnippetGroupId)
}

pub(super) fn parse_snippet_arguments_text(input: &str) -> Vec<SnippetArgument> {
    input
        .split(['\n', ';'])
        .filter_map(|entry| {
            let (name, value) = entry.split_once('=')?;
            let name = name.trim();
            if name.is_empty() {
                return None;
            }
            Some(SnippetArgument {
                name: name.to_owned(),
                value: value.trim().to_owned(),
            })
        })
        .collect()
}

pub(super) fn parse_snippet_row_id(row_id: &str) -> Option<SnippetId> {
    row_id
        .strip_prefix("snippet:")
        .and_then(|id| Uuid::parse_str(id).ok())
        .map(SnippetId)
}

pub(super) fn parse_snippet_target_row_id(
    row_id: &str,
) -> Option<(SnippetId, SnippetSupportTargetId)> {
    let rest = row_id.strip_prefix("snippet-target:")?;
    let (snippet_id, target_id) = rest.split_once(':')?;
    Some((
        SnippetId(Uuid::parse_str(snippet_id).ok()?),
        SnippetSupportTargetId(Uuid::parse_str(target_id).ok()?),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_snippet_group_and_row_ids() {
        let group_id = SnippetGroupId(Uuid::new_v4());
        let snippet_id = SnippetId(Uuid::new_v4());

        assert_eq!(
            parse_snippet_group_node_id(&format!("snippet-folder:group:{}", group_id.0)),
            Some(group_id)
        );
        assert_eq!(parse_optional_snippet_group_node_id(""), None);
        assert_eq!(
            parse_snippet_row_id(&format!("snippet:{}", snippet_id.0)),
            Some(snippet_id)
        );
    }

    #[test]
    fn parses_snippet_target_row_id() {
        let snippet_id = SnippetId(Uuid::new_v4());
        let target_id = SnippetSupportTargetId(Uuid::new_v4());

        assert_eq!(
            parse_snippet_target_row_id(&format!(
                "snippet-target:{}:{}",
                snippet_id.0, target_id.0
            )),
            Some((snippet_id, target_id))
        );
        assert_eq!(parse_snippet_target_row_id("snippet-target:broken"), None);
    }

    #[test]
    fn parses_snippet_arguments_text() {
        let arguments = parse_snippet_arguments_text("name = nginx\nempty = ; ; port= 443");

        assert_eq!(arguments.len(), 3);
        assert_eq!(arguments[0].name, "name");
        assert_eq!(arguments[0].value, "nginx");
        assert_eq!(arguments[1].name, "empty");
        assert_eq!(arguments[1].value, "");
        assert_eq!(arguments[2].name, "port");
        assert_eq!(arguments[2].value, "443");
    }
}
