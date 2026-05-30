//! 设置页核心操作。

use std::path::Path;

use super::{AppState, AppUpdateOutcome};

impl AppState {
    pub(in crate::model::app_state) fn export_built_in_theme(
        &mut self,
        target_path: &str,
        format: crate::theme::ThemeExchangeFormat,
    ) -> AppUpdateOutcome {
        let document = crate::theme::built_in_theme_document(self.ui.workspace.theme);
        match document.export(format) {
            Ok(exported) => write_text_file(normalized_path(target_path), exported.content)
                .map_or_else(settings_error, |_| AppUpdateOutcome::default()),
            Err(error) => settings_error(format!("导出主题失败：{error}")),
        }
    }

    pub(in crate::model::app_state) fn import_theme(
        &mut self,
        source_path: &str,
    ) -> AppUpdateOutcome {
        let source_path = normalized_path(source_path);
        let content = match std::fs::read_to_string(source_path) {
            Ok(content) => content,
            Err(error) => return settings_error(format!("读取主题失败：{error}")),
        };
        let source_name = source_path.display().to_string();
        let document = match parse_theme_document(&content, &source_name) {
            Ok(document) => document,
            Err(error) => return settings_error(format!("导入主题失败：{error}")),
        };
        let theme_name = document.name.clone();
        let profile_toml = match document.to_toml() {
            Ok(profile_toml) => profile_toml,
            Err(error) => return settings_error(format!("导入主题失败：{error}")),
        };
        self.storage
            .upsert_theme(crate::storage::ThemeProfileRecord {
                name: theme_name.clone(),
                profile_toml,
                builtin: false,
            });

        self.apply_theme_document(document);
        AppUpdateOutcome {
            state_changed: true,
            ..AppUpdateOutcome::default()
        }
    }

    pub(in crate::model::app_state) fn apply_theme_profile(
        &mut self,
        name: &str,
    ) -> AppUpdateOutcome {
        let Some(theme) = self.storage.theme_by_name(name.trim()) else {
            return settings_error(format!("主题资料不存在：{}", name.trim()));
        };
        let document = match parse_theme_document(&theme.profile_toml, &theme.name) {
            Ok(document) => document,
            Err(error) => return settings_error(format!("应用主题失败：{error}")),
        };

        let changed = self.theme_document_would_change_profile(&document);
        self.apply_theme_document(document);
        AppUpdateOutcome {
            state_changed: changed,
            ..AppUpdateOutcome::default()
        }
    }

    pub(in crate::model::app_state) fn remove_theme_profile(
        &mut self,
        name: &str,
    ) -> AppUpdateOutcome {
        let removed = self.storage.remove_theme(name.trim());
        if !removed {
            return settings_error(format!("主题资料不存在：{}", name.trim()));
        }
        self.storage.app_config = self.config.clone();

        AppUpdateOutcome {
            state_changed: true,
            ..AppUpdateOutcome::default()
        }
    }

    pub(in crate::model::app_state) fn backup_storage(
        &mut self,
        target_path: &str,
    ) -> AppUpdateOutcome {
        let Some(backend) = self.storage_backend.as_ref() else {
            return settings_error("没有可用的 SQLite 存储后端");
        };

        if let Err(error) = backend.save(&self.storage) {
            return settings_error(format!("备份前保存存储失败：{error}"));
        }

        match backend.backup_to(normalized_path(target_path)) {
            Ok(()) => AppUpdateOutcome::default(),
            Err(error) => settings_error(format!("备份数据库失败：{error}")),
        }
    }

    pub(in crate::model::app_state) fn export_storage_snapshot(
        &mut self,
        target_path: &str,
    ) -> AppUpdateOutcome {
        let Some(backend) = self.storage_backend.as_ref() else {
            return settings_error("没有可用的 SQLite 存储后端");
        };

        if let Err(error) = backend.save(&self.storage) {
            return settings_error(format!("导出前保存存储失败：{error}"));
        }

        match backend.export_snapshot_to(normalized_path(target_path)) {
            Ok(()) => AppUpdateOutcome::default(),
            Err(error) => settings_error(format!("导出快照失败：{error}")),
        }
    }

    pub(in crate::model::app_state) fn import_storage_snapshot(
        &mut self,
        source_path: &str,
    ) -> AppUpdateOutcome {
        let Some(backend) = self.storage_backend.as_ref() else {
            return settings_error("没有可用的 SQLite 存储后端");
        };

        let import_result = backend.import_snapshot_from(normalized_path(source_path));

        match import_result {
            Ok(()) => self.reload_storage_after_import(),
            Err(error) => settings_error(format!("导入快照失败：{error}")),
        }
    }

    pub(in crate::model::app_state) fn import_sqlite_backup(
        &mut self,
        source_path: &str,
    ) -> AppUpdateOutcome {
        let Some(backend) = self.storage_backend.as_ref() else {
            return settings_error("没有可用的 SQLite 存储后端");
        };

        let import_result = backend.import_sqlite_backup_from(normalized_path(source_path));

        match import_result {
            Ok(()) => self.reload_storage_after_import(),
            Err(error) => settings_error(format!("导入 SQLite 备份失败：{error}")),
        }
    }

    fn reload_storage_after_import(&mut self) -> AppUpdateOutcome {
        let Some(backend) = self.storage_backend.as_ref() else {
            return settings_error("没有可用的 SQLite 存储后端");
        };
        let storage = match backend.load() {
            Ok(storage) => storage,
            Err(error) => return settings_error(format!("导入后重新加载存储失败：{error}")),
        };

        self.storage = storage;
        self.config = self.storage.app_config.clone();
        self.ui.visual_settings = crate::model::VisualSettingsDraft::from_profiles(
            &self.config.theme,
            &self.config.background,
        );
        self.apply_workspace_preferences();
        AppUpdateOutcome {
            state_changed: true,
            ..AppUpdateOutcome::default()
        }
    }

    fn apply_theme_document(&mut self, document: crate::theme::ThemeDocument) {
        self.config.theme = crate::model::ThemeProfile {
            name: document.name,
            font_family: document.font.terminal.family,
            font_size: document.font.terminal.size as f32,
        };
        self.storage.app_config = self.config.clone();
        self.ui.visual_settings = crate::model::VisualSettingsDraft::from_profiles(
            &self.config.theme,
            &self.config.background,
        );
    }

    fn theme_document_would_change_profile(&self, document: &crate::theme::ThemeDocument) -> bool {
        self.config.theme.name != document.name
            || self.config.theme.font_family != document.font.terminal.family
            || self.config.theme.font_size != document.font.terminal.size as f32
    }
}

fn normalized_path(value: &str) -> &Path {
    Path::new(value.trim())
}

fn write_text_file(path: &Path, content: String) -> Result<(), String> {
    if path.exists() {
        return Err(format!("导出目标已存在：{}", path.display()));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| format!("创建目录失败：{error}"))?;
    }
    std::fs::write(path, content).map_err(|error| format!("写入文件失败：{error}"))
}

fn parse_theme_document(
    content: &str,
    source_name: &str,
) -> Result<crate::theme::ThemeDocument, crate::theme::ThemeError> {
    crate::theme::ThemeDocument::from_import(content, source_name)
}

fn settings_error(message: impl Into<String>) -> AppUpdateOutcome {
    AppUpdateOutcome {
        state_changed: false,
        error: Some(message.into()),
        ..AppUpdateOutcome::default()
    }
}
