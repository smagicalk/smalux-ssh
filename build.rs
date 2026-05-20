fn main() {
    for entry in std::fs::read_dir("ui").expect("UI 目录应该存在") {
        let path = entry.expect("UI 文件应该可以读取").path();
        if path
            .extension()
            .is_some_and(|extension| extension == "slint")
        {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
    println!("cargo:rerun-if-changed=build.rs");

    slint_build::compile("ui/main.slint").expect("Slint UI 应该可以编译");
}
