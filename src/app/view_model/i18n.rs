//! i18n 配置加载与文案访问。
//!
//! 文案从配置文件生成的 catalog 中读取，不写死在 UI 代码里。缺少 key 时按
//! “当前语言 -> 英文 -> key 本身”回退，保证新增文案未翻译时应用仍能启动。

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::model::{AppState, LanguageMode};

pub(super) type Locale = &'static str;

const ENGLISH_LOCALE_CODE: Locale = "en-US";

#[derive(Debug, serde::Deserialize)]
struct LocaleFile {
    strings: HashMap<String, String>,
}

#[derive(Debug)]
struct Catalog {
    default_locale: Locale,
    files: HashMap<Locale, LocaleFile>,
}

static CATALOG: OnceLock<Catalog> = OnceLock::new();

include!(concat!(env!("OUT_DIR"), "/i18n_catalog.rs"));

pub(super) fn locale_for_state(state: &AppState) -> Locale {
    // FollowSystem 只影响 locale 选择；具体 key 和回退策略仍由 tr_from_files 统一处理。
    match state.ui.workspace.language {
        LanguageMode::Chinese => resolve_locale("zh-CN"),
        LanguageMode::English => resolve_locale("en-US"),
        LanguageMode::FollowSystem => sys_locale::get_locale()
            .as_deref()
            .map(resolve_locale)
            .unwrap_or_else(default_locale),
    }
}

pub(super) fn tr(locale: Locale, key: &'static str) -> &'static str {
    // 对外返回 &'static str，便于 Slint Row 和 ViewModel 直接持有文案引用。
    tr_from_files(&catalog_root().files, locale, key)
}

pub(super) fn tr_for_state(state: &AppState, key: &'static str) -> &'static str {
    tr(locale_for_state(state), key)
}

pub(super) fn default_locale() -> Locale {
    catalog_root().default_locale
}

pub(super) fn english_locale() -> Locale {
    resolve_locale(ENGLISH_LOCALE_CODE)
}

#[cfg(test)]
pub(super) fn available_locales() -> Vec<Locale> {
    let mut locales = catalog_root().files.keys().copied().collect::<Vec<_>>();
    locales.sort();
    locales
}

fn resolve_locale(locale: &str) -> Locale {
    let root = catalog_root();
    let normalized = normalize_locale_code(locale);

    // 先精确匹配 zh-CN，再按语言前缀匹配 zh，最后回退默认语言。
    root.files
        .keys()
        .copied()
        .find(|candidate| candidate.eq_ignore_ascii_case(&normalized))
        .or_else(|| {
            let language = normalized.split('-').next().unwrap_or(&normalized);
            root.files.keys().copied().find(|candidate| {
                candidate
                    .split('-')
                    .next()
                    .is_some_and(|part| part == language)
            })
        })
        .unwrap_or(root.default_locale)
}

fn tr_from_files<'a>(
    files: &'a HashMap<Locale, LocaleFile>,
    locale: Locale,
    key: &'a str,
) -> &'a str {
    // 缺少 key 不 panic，直接返回 key，便于开发阶段发现但不阻断界面运行。
    tr_in_files(files, locale, key)
        .or_else(|| tr_in_files(files, ENGLISH_LOCALE_CODE, key))
        .unwrap_or(key)
}

fn tr_in_files<'a>(
    files: &'a HashMap<Locale, LocaleFile>,
    locale: Locale,
    key: &str,
) -> Option<&'a str> {
    files
        .get(locale)
        .and_then(|catalog| catalog.strings.get(key).map(String::as_str))
}

fn catalog_root() -> &'static Catalog {
    CATALOG.get_or_init(|| {
        // LOCALE_FILES 由 build.rs 扫描 i18n 配置生成，运行时不再遍历文件系统。
        let files = LOCALE_FILES
            .iter()
            .map(|(code, content)| (*code, parse_locale(content)))
            .collect::<HashMap<_, _>>();
        let default_locale = files
            .keys()
            .copied()
            .find(|code| *code == DEFAULT_LOCALE_CODE)
            .or_else(|| files.keys().copied().next())
            .expect("at least one i18n locale file should exist");

        // 启动时只验证必需 catalog 存在；具体 key 缺失走运行时回退。
        validate_catalogs(&files, default_locale);

        Catalog {
            default_locale,
            files,
        }
    })
}

fn parse_locale(content: &'static str) -> LocaleFile {
    serde_json::from_str(content).expect("i18n locale file should be valid JSON")
}

fn normalize_locale_code(locale: &str) -> String {
    locale.replace('_', "-")
}

fn validate_catalogs(files: &HashMap<Locale, LocaleFile>, default_locale: Locale) {
    let _ = files
        .get(default_locale)
        .unwrap_or_else(|| panic!("missing default i18n locale `{default_locale}`"));
    let _ = files
        .get(ENGLISH_LOCALE_CODE)
        .unwrap_or_else(|| panic!("missing fallback i18n locale `{ENGLISH_LOCALE_CODE}`"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_catalog_discovers_configured_locale_files() {
        assert!(available_locales().contains(&"zh-CN"));
        assert!(available_locales().contains(&"en-US"));
    }

    #[test]
    fn tr_falls_back_to_english_then_key() {
        let files = HashMap::from([
            (
                "zh-CN",
                LocaleFile {
                    strings: HashMap::from([("present".to_owned(), "中文".to_owned())]),
                },
            ),
            (
                "en-US",
                LocaleFile {
                    strings: HashMap::from([
                        ("present".to_owned(), "English".to_owned()),
                        ("fallback_only".to_owned(), "Fallback".to_owned()),
                    ]),
                },
            ),
        ]);

        assert_eq!(tr_from_files(&files, "zh-CN", "present"), "中文");
        assert_eq!(tr_from_files(&files, "zh-CN", "fallback_only"), "Fallback");
        assert_eq!(
            tr_from_files(&files, "zh-CN", "__missing_test_key__"),
            "__missing_test_key__"
        );
    }

    #[test]
    fn tr_returns_key_when_catalogs_do_not_define_it() {
        assert_eq!(
            tr(default_locale(), "__missing_test_key__"),
            "__missing_test_key__"
        );
    }
}
