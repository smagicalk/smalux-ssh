//! 主题资料的内存索引操作。

use super::{StorageManager, ThemeProfileRecord};

impl StorageManager {
    /// 保存或更新主题资料。
    pub fn upsert_theme(&mut self, theme: ThemeProfileRecord) {
        if let Some(existing) = self
            .themes
            .iter_mut()
            .find(|existing| existing.name == theme.name)
        {
            *existing = theme;
        } else {
            self.themes.push(theme);
        }
    }

    /// 删除主题资料。
    pub fn remove_theme(&mut self, name: &str) -> bool {
        let before = self.themes.len();
        self.themes.retain(|theme| theme.name != name);
        before != self.themes.len()
    }

    /// 按名称查找主题资料。
    pub fn theme_by_name(&self, name: &str) -> Option<&ThemeProfileRecord> {
        self.themes.iter().find(|theme| theme.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn theme(name: &str, profile_toml: &str) -> ThemeProfileRecord {
        ThemeProfileRecord {
            name: name.to_owned(),
            profile_toml: profile_toml.to_owned(),
            builtin: false,
        }
    }

    #[test]
    fn themes_can_be_upserted_and_removed_by_name() {
        let mut storage = StorageManager::default();

        storage.upsert_theme(theme("Imported", "first"));
        storage.upsert_theme(theme("Imported", "second"));

        assert_eq!(storage.theme_count(), 1);
        assert_eq!(
            storage
                .theme_by_name("Imported")
                .map(|theme| theme.profile_toml.as_str()),
            Some("second")
        );
        assert!(storage.remove_theme("Imported"));
        assert!(!storage.remove_theme("Imported"));
        assert_eq!(storage.theme_count(), 0);
    }
}
