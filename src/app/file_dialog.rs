//! 当前 Slint 桌面 UI 的原生文件选择器适配层。
//!
//! 这个模块刻意放在 `app` 内：文件选择是 UI 关注点，导入、导出、备份和凭据持久化
//! 仍然由核心 `AppState` 消息处理器负责。

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileDialogDirection {
    Import,
    Export,
}

pub(super) fn choose_settings_file_action_path(
    current_path: &str,
    direction: &str,
    extension: &str,
) -> String {
    let direction = FileDialogDirection::from_key(direction);
    let selected = choose_file(current_path, direction, extension);

    selected_path_or_current(selected, current_path)
}

pub(super) fn choose_security_credential_file_path(current_path: &str) -> String {
    let selected = choose_file(current_path, FileDialogDirection::Import, "");

    selected_path_or_current(selected, current_path)
}

pub(super) fn choose_security_credential_export_path(current_path: &str, kind_key: &str) -> String {
    let selected = choose_file(
        current_path,
        FileDialogDirection::Export,
        security_credential_export_extension(kind_key),
    );

    selected_path_or_current(selected, current_path)
}

fn choose_file(
    current_path: &str,
    direction: FileDialogDirection,
    extension: &str,
) -> Option<PathBuf> {
    let dialog = file_dialog_with_hint(rfd::FileDialog::new(), current_path);
    let dialog = file_dialog_with_extension_filter(dialog, extension);

    match direction {
        FileDialogDirection::Import => dialog.pick_file(),
        FileDialogDirection::Export => dialog.save_file(),
    }
}

fn file_dialog_with_hint(dialog: rfd::FileDialog, current_path: &str) -> rfd::FileDialog {
    let trimmed = current_path.trim();
    if trimmed.is_empty() {
        return dialog;
    }

    let path = Path::new(trimmed);
    if path.is_dir() {
        return dialog.set_directory(path);
    }

    let dialog = match path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        Some(parent) => dialog.set_directory(parent),
        None => dialog,
    };

    match path.file_name().and_then(|name| name.to_str()) {
        Some(file_name) if !file_name.is_empty() => dialog.set_file_name(file_name),
        _ => dialog,
    }
}

fn file_dialog_with_extension_filter(dialog: rfd::FileDialog, extension: &str) -> rfd::FileDialog {
    let Some(extension) = normalized_extension(extension) else {
        return dialog;
    };

    let filter_name = extension.to_ascii_uppercase();
    dialog.add_filter(filter_name, &[extension])
}

fn selected_path_or_current(selected: Option<PathBuf>, current_path: &str) -> String {
    selected
        .map(normalized_path_string)
        .unwrap_or_else(|| current_path.trim().to_owned())
}

fn normalized_path_string(path: PathBuf) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn normalized_extension(extension: &str) -> Option<&str> {
    let extension = extension.trim().trim_start_matches('.');
    if extension.is_empty() {
        None
    } else {
        Some(extension)
    }
}

fn security_credential_export_extension(kind_key: &str) -> &'static str {
    match kind_key.trim() {
        "Certificate" => "pub",
        "Password" => "txt",
        _ => "",
    }
}

impl FileDialogDirection {
    fn from_key(value: &str) -> Self {
        match value.trim() {
            "Export" => Self::Export,
            _ => Self::Import,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dialog_direction_defaults_to_import() {
        assert_eq!(
            FileDialogDirection::from_key("Export"),
            FileDialogDirection::Export
        );
        assert_eq!(
            FileDialogDirection::from_key("Import"),
            FileDialogDirection::Import
        );
        assert_eq!(
            FileDialogDirection::from_key("missing"),
            FileDialogDirection::Import
        );
    }

    #[test]
    fn normalized_extension_rejects_empty_values() {
        assert_eq!(normalized_extension("toml"), Some("toml"));
        assert_eq!(normalized_extension(".sqlite"), Some("sqlite"));
        assert_eq!(normalized_extension(""), None);
        assert_eq!(normalized_extension("   "), None);
    }

    #[test]
    fn credential_export_extension_matches_kind() {
        assert_eq!(security_credential_export_extension("Certificate"), "pub");
        assert_eq!(security_credential_export_extension("Password"), "txt");
        assert_eq!(security_credential_export_extension("PrivateKey"), "");
        assert_eq!(security_credential_export_extension("Unknown"), "");
    }

    #[test]
    fn selected_path_or_current_keeps_current_path_when_cancelled() {
        assert_eq!(
            selected_path_or_current(None, " C:/Users/me/.ssh/id_ed25519 "),
            "C:/Users/me/.ssh/id_ed25519"
        );
    }
}
