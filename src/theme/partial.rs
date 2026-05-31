//! 精简主题 TOML 兼容层。
//!
//! 完整 `ThemeDocument` 适合机器保存和导入导出，但人工手写时字段太多。
//! 这里支持“继承一个内置主题 + 覆盖少量字段”的精简格式，再转换成完整
//! `ThemeDocument`，让后续存储、导出和 Slint 投影继续复用现有路径。

use crate::model::BuiltInTheme;

use super::builtin::built_in_theme_document;
use super::{ThemeDocument, ThemeError};

pub(super) fn theme_document_from_partial_toml(input: &str) -> Result<ThemeDocument, ThemeError> {
    let mut partial: toml::Value = toml::from_str(input)?;
    let base_theme = partial_base_theme(&partial)?;
    let root = partial
        .as_table_mut()
        .ok_or(ThemeError::InvalidExternalTheme("theme"))?;

    ensure_partial_identity(root)?;
    expand_overrides(root)?;

    let base_document = built_in_theme_document(base_theme);
    let base_toml = toml::to_string(&base_document)?;
    let mut merged: toml::Value = toml::from_str(&base_toml)?;
    merge_toml(&mut merged, partial);

    let document: ThemeDocument = merged.try_into()?;
    document.validate()?;
    Ok(document)
}

fn partial_base_theme(partial: &toml::Value) -> Result<BuiltInTheme, ThemeError> {
    let Some(root) = partial.as_table() else {
        return Err(ThemeError::InvalidExternalTheme("theme"));
    };
    let base = root
        .get("base")
        .or_else(|| root.get("extends"))
        .and_then(toml::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());

    match base {
        Some(identifier) => built_in_theme_from_identifier(identifier)
            .ok_or(ThemeError::InvalidExternalTheme("base")),
        None => Ok(BuiltInTheme::ProfessionalDark),
    }
}

fn built_in_theme_from_identifier(identifier: &str) -> Option<BuiltInTheme> {
    let normalized = normalize_theme_identifier(identifier);
    BuiltInTheme::ALL.into_iter().find(|theme| {
        theme.key().eq_ignore_ascii_case(identifier)
            || normalize_theme_identifier(theme.key()) == normalized
            || normalize_theme_identifier(&built_in_theme_document(*theme).id) == normalized
            || normalize_theme_identifier(&built_in_theme_document(*theme).name) == normalized
    })
}

fn normalize_theme_identifier(identifier: &str) -> String {
    identifier
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn ensure_partial_identity(root: &toml::Table) -> Result<(), ThemeError> {
    require_partial_string(root, "id")?;
    require_partial_string(root, "name")?;
    Ok(())
}

fn require_partial_string(root: &toml::Table, key: &'static str) -> Result<(), ThemeError> {
    root.get(key)
        .and_then(toml::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(ThemeError::InvalidExternalTheme(key))
        .map(|_| ())
}

fn expand_overrides(root: &mut toml::Table) -> Result<(), ThemeError> {
    let Some(overrides) = root.remove("overrides") else {
        return Ok(());
    };

    let mut entries = Vec::new();
    collect_override_entries(Vec::new(), &overrides, &mut entries)?;
    for (path, value) in entries {
        insert_path(root, &path, value)?;
    }
    Ok(())
}

fn collect_override_entries(
    prefix: Vec<String>,
    value: &toml::Value,
    entries: &mut Vec<(Vec<String>, toml::Value)>,
) -> Result<(), ThemeError> {
    if let toml::Value::Table(table) = value {
        for (key, value) in table {
            let mut next = prefix.clone();
            next.extend(split_override_key(key)?);
            collect_override_entries(next, value, entries)?;
        }
        return Ok(());
    }

    if prefix.is_empty() {
        return Err(ThemeError::InvalidExternalTheme("overrides"));
    }
    entries.push((prefix, value.clone()));
    Ok(())
}

fn split_override_key(key: &str) -> Result<Vec<String>, ThemeError> {
    let path = key
        .split('.')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if path.is_empty() {
        Err(ThemeError::InvalidExternalTheme("overrides"))
    } else {
        Ok(path)
    }
}

fn insert_path(
    table: &mut toml::Table,
    path: &[String],
    value: toml::Value,
) -> Result<(), ThemeError> {
    let Some((head, tail)) = path.split_first() else {
        return Err(ThemeError::InvalidExternalTheme("overrides"));
    };

    if tail.is_empty() {
        table.insert(head.clone(), value);
        return Ok(());
    }

    let entry = table
        .entry(head.clone())
        .or_insert_with(|| toml::Value::Table(toml::Table::new()));
    let child = entry
        .as_table_mut()
        .ok_or(ThemeError::InvalidExternalTheme("overrides"))?;
    insert_path(child, tail, value)
}

fn merge_toml(target: &mut toml::Value, patch: toml::Value) {
    match (target, patch) {
        (toml::Value::Table(target), toml::Value::Table(patch)) => {
            for (key, value) in patch {
                match target.get_mut(&key) {
                    Some(existing) => merge_toml(existing, value),
                    None => {
                        target.insert(key, value);
                    }
                }
            }
        }
        (target, patch) => *target = patch,
    }
}
