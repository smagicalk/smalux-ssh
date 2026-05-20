fn main() {
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
    println!("cargo:rerun-if-changed=build.rs");

    slint_build::compile("ui/main.slint").expect("Slint UI 应该可以编译");
}
