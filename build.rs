use std::fs;
use std::path::PathBuf;

fn main() {
    generate_i18n_catalog();
    compile_desktop_ui();
}

#[cfg(feature = "desktop")]
fn compile_desktop_ui() {
    let mut ui_files = std::fs::read_dir("ui")
        .expect("UI 目录应该存在")
        .map(|entry| entry.expect("UI 文件应该可以读取").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "slint")
        })
        .collect::<Vec<_>>();
    ui_files.sort();

    for path in ui_files {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    println!("cargo:rerun-if-changed=assets/i18n");
    println!("cargo:rerun-if-changed=build.rs");

    // DragArea/DropArea 在 Slint 1.16 仍属于实验内建项；这里限定在 UI 编译阶段开启。
    unsafe {
        std::env::set_var("SLINT_ENABLE_EXPERIMENTAL_FEATURES", "1");
    }
    slint_build::compile("ui/main.slint").expect("Slint UI 应该可以编译");
}

#[cfg(not(feature = "desktop"))]
fn compile_desktop_ui() {}

fn generate_i18n_catalog() {
    let mut locale_files = std::fs::read_dir("assets/i18n")
        .expect("i18n 目录应该存在")
        .map(|entry| entry.expect("i18n 文件应该可以读取").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .map(|path| {
            let code = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .expect("i18n 文件名应该是有效 UTF-8")
                .to_owned();
            let content = fs::read_to_string(&path).expect("i18n 文件应该是有效 UTF-8");
            (code, path, content)
        })
        .collect::<Vec<_>>();

    locale_files.sort_by(|left, right| left.0.cmp(&right.0));

    for (_, path, _) in &locale_files {
        println!("cargo:rerun-if-changed={}", path.display());
    }

    let default_locale = locale_files
        .iter()
        .find(|(code, _, _)| code == "zh-CN")
        .map(|(code, _, _)| code.as_str())
        .or_else(|| locale_files.first().map(|(code, _, _)| code.as_str()))
        .unwrap_or("zh-CN");

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR should exist"));
    let generated = out_dir.join("i18n_catalog.rs");
    fs::write(
        &generated,
        render_i18n_catalog(default_locale, &locale_files),
    )
    .expect("i18n catalog should be writable");
}

fn render_i18n_catalog(default_locale: &str, locale_files: &[(String, PathBuf, String)]) -> String {
    let mut output = String::new();
    output.push_str("pub const DEFAULT_LOCALE_CODE: &str = ");
    output.push_str(&format!("{default_locale:?};\n"));
    output.push_str("pub const LOCALE_FILES: &[(&str, &str)] = &[\n");
    for (code, _, content) in locale_files {
        output.push_str("    (");
        output.push_str(&format!("{code:?}, {content:?}"));
        output.push_str("),\n");
    }
    output.push_str("];\n");
    output
}
