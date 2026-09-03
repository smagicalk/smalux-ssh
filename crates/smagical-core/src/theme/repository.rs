use std::{
    fs,
    path::{Path, PathBuf},
};

use super::{TerminalThemeDefinition, ThemeError, ThemeId, UiThemeDefinition};

/// 从主题目录加载的主题文件。
#[derive(Debug, Clone, PartialEq)]
pub enum LoadedTheme {
    /// 从文件解析出的 UI 主题。
    Ui(UiThemeDefinition),
    /// 从文件解析出的终端主题。
    Terminal(TerminalThemeDefinition),
}

/// 可由内存、文件或未来云同步实现的主题存储接口。
pub trait ThemeRepository {
    /// 返回当前实际用于读写自定义主题的目录。
    fn active_directory(&self) -> &Path;
    /// 扫描目录并返回所有可解析且大小合规的主题。
    fn discover(&self) -> Result<Vec<LoadedTheme>, ThemeError>;
    /// 将 UI 主题保存为 TOML，并返回目标路径。
    fn save_ui(&self, theme: &UiThemeDefinition) -> Result<PathBuf, ThemeError>;
    /// 将终端主题保存为 TOML，并返回目标路径。
    fn save_terminal(&self, theme: &TerminalThemeDefinition) -> Result<PathBuf, ThemeError>;
    /// 删除指定 ID 对应的自定义主题文件；文件不存在时视为成功。
    fn delete(&self, id: &ThemeId) -> Result<(), ThemeError>;
}

/// 基于 TOML 文件的主题存储。优先使用程序旁目录，不可写时回退用户配置目录。
#[derive(Debug, Clone)]
pub struct FileThemeRepository {
    directory: PathBuf,
}

impl FileThemeRepository {
    /// 优先在程序目录旁创建 `themes/`，不可写时回退到用户配置目录。
    pub fn new(program_directory: impl AsRef<Path>) -> Result<Self, ThemeError> {
        let preferred = program_directory.as_ref().join("themes");
        if ensure_writable(&preferred).is_ok() {
            return Ok(Self {
                directory: preferred,
            });
        }
        let project = directories::ProjectDirs::from("dev", "smagical", "smalux-ssh")
            .ok_or(ThemeError::ConfigDirectoryUnavailable)?;
        let fallback = project.config_dir().join("themes");
        ensure_writable(&fallback)?;
        Ok(Self {
            directory: fallback,
        })
    }

    /// 使用调用方指定的目录创建文件仓储，主要用于测试和自定义集成。
    pub fn from_directory(directory: impl Into<PathBuf>) -> Result<Self, ThemeError> {
        let directory = directory.into();
        ensure_writable(&directory)?;
        Ok(Self { directory })
    }

    fn save_toml<T: serde::Serialize>(
        &self,
        id: &ThemeId,
        value: &T,
    ) -> Result<PathBuf, ThemeError> {
        if !id.is_valid() {
            return Err(ThemeError::InvalidId(id.clone()));
        }
        let target = self
            .directory
            .join(format!("{}.toml", safe_file_name(id.as_ref())));
        let temporary = target.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
        fs::write(&temporary, toml::to_string_pretty(value)?)?;
        if target.exists() {
            fs::remove_file(&target)?;
        }
        fs::rename(&temporary, &target)?;
        Ok(target)
    }
}

impl ThemeRepository for FileThemeRepository {
    fn active_directory(&self) -> &Path {
        &self.directory
    }

    fn discover(&self) -> Result<Vec<LoadedTheme>, ThemeError> {
        let mut loaded = Vec::new();
        for entry in fs::read_dir(&self.directory)? {
            let path = entry?.path();
            if path.extension().and_then(|value| value.to_str()) != Some("toml") {
                continue;
            }
            let metadata = fs::metadata(&path)?;
            if metadata.len() > 1_048_576 {
                continue;
            }
            let source = fs::read_to_string(path)?;
            let value: toml::Value = match toml::from_str(&source) {
                Ok(value) => value,
                Err(_) => continue,
            };
            match value.get("kind").and_then(toml::Value::as_str) {
                Some("ui") => {
                    if let Ok(theme) = toml::from_str(&source) {
                        loaded.push(LoadedTheme::Ui(theme));
                    }
                }
                Some("terminal") => {
                    if let Ok(theme) = toml::from_str(&source) {
                        loaded.push(LoadedTheme::Terminal(theme));
                    }
                }
                _ => {}
            }
        }
        Ok(loaded)
    }

    fn save_ui(&self, theme: &UiThemeDefinition) -> Result<PathBuf, ThemeError> {
        self.save_toml(&theme.metadata.id, theme)
    }
    fn save_terminal(&self, theme: &TerminalThemeDefinition) -> Result<PathBuf, ThemeError> {
        self.save_toml(&theme.metadata.id, theme)
    }
    fn delete(&self, id: &ThemeId) -> Result<(), ThemeError> {
        if !id.is_valid() {
            return Err(ThemeError::InvalidId(id.clone()));
        }
        let path = self
            .directory
            .join(format!("{}.toml", safe_file_name(id.as_ref())));
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }
}

/// 纯内存/数据层主题仓储实现 (0 磁盘 I/O，不写入本地物理文件，直接在数据层维护)
#[derive(Debug, Clone)]
pub struct MemoryThemeRepository {
    virtual_directory: PathBuf,
    themes: std::sync::Arc<std::sync::RwLock<Vec<LoadedTheme>>>,
}

impl Default for MemoryThemeRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryThemeRepository {
    /// 创建全新的内存主题仓储
    pub fn new() -> Self {
        Self {
            virtual_directory: PathBuf::from("memory://themes"),
            themes: std::sync::Arc::new(std::sync::RwLock::new(Vec::new())),
        }
    }

    /// 使用已有主题列表初始化
    pub fn with_themes(initial: Vec<LoadedTheme>) -> Self {
        Self {
            virtual_directory: PathBuf::from("memory://themes"),
            themes: std::sync::Arc::new(std::sync::RwLock::new(initial)),
        }
    }

    /// 获取底层主题并发读写锁
    pub fn themes_raw(&self) -> std::sync::Arc<std::sync::RwLock<Vec<LoadedTheme>>> {
        self.themes.clone()
    }
}

impl ThemeRepository for MemoryThemeRepository {
    fn active_directory(&self) -> &Path {
        &self.virtual_directory
    }

    fn discover(&self) -> Result<Vec<LoadedTheme>, ThemeError> {
        let guard = self.themes.read().map_err(|e| ThemeError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
        Ok(guard.clone())
    }

    fn save_ui(&self, theme: &UiThemeDefinition) -> Result<PathBuf, ThemeError> {
        let mut guard = self.themes.write().map_err(|e| ThemeError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
        guard.retain(|t| match t {
            LoadedTheme::Ui(u) => u.metadata.id != theme.metadata.id,
            _ => true,
        });
        guard.push(LoadedTheme::Ui(theme.clone()));
        Ok(self.virtual_directory.join(format!("{}.toml", theme.metadata.id)))
    }

    fn save_terminal(&self, theme: &TerminalThemeDefinition) -> Result<PathBuf, ThemeError> {
        let mut guard = self.themes.write().map_err(|e| ThemeError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
        guard.retain(|t| match t {
            LoadedTheme::Terminal(tm) => tm.metadata.id != theme.metadata.id,
            _ => true,
        });
        guard.push(LoadedTheme::Terminal(theme.clone()));
        Ok(self.virtual_directory.join(format!("{}.toml", theme.metadata.id)))
    }

    fn delete(&self, id: &ThemeId) -> Result<(), ThemeError> {
        let mut guard = self.themes.write().map_err(|e| ThemeError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
        guard.retain(|t| match t {
            LoadedTheme::Ui(u) => &u.metadata.id != id,
            LoadedTheme::Terminal(tm) => &tm.metadata.id != id,
        });
        Ok(())
    }
}

fn ensure_writable(directory: &Path) -> Result<(), std::io::Error> {
    fs::create_dir_all(directory)?;
    let probe = directory.join(format!(".write-test-{}", uuid::Uuid::new_v4()));
    fs::write(&probe, b"ok")?;
    fs::remove_file(probe)
}

fn safe_file_name(id: &str) -> String {
    id.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}
