use std::collections::HashMap;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let common_dir = manifest_dir.join("../../common/ui");

    let mut library_paths = HashMap::new();
    library_paths.insert("common".to_string(), common_dir.clone());

    let include_paths = vec![
        common_dir.clone(),
        common_dir.join("themes"),
        common_dir.join("shared"),
        common_dir.join("components"),
        common_dir.join("features"),
        manifest_dir.join("ui"),
        manifest_dir.join("ui/drawers"),
        manifest_dir.join("ui/modals"),
        manifest_dir.join("ui/views"),
    ];

    println!("cargo:rerun-if-changed=ui/hosts_plugin.slint");
    println!("cargo:rerun-if-changed=ui/drawers");
    println!("cargo:rerun-if-changed=ui/modals");
    println!("cargo:rerun-if-changed=ui/views");

    let config = slint_build::CompilerConfiguration::new()
        .with_library_paths(library_paths)
        .with_include_paths(include_paths);

    slint_build::compile_with_config("ui/hosts_plugin.slint", config).expect("Hosts Plugin UI 应该可以编译");
}
