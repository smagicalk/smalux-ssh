use std::collections::HashMap;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let common_dir = manifest_dir.join("../common/ui");
    let hosts_dir = manifest_dir.join("../plugins/hosts/ui");
    let session_tools_dir = manifest_dir.join("../plugins/session_tools/ui");
    let snippets_dir = manifest_dir.join("../plugins/snippets/ui");
    let tunnels_dir = manifest_dir.join("../plugins/tunnels/ui");

    let mut library_paths = HashMap::new();
    library_paths.insert("common".to_string(), common_dir.clone());
    library_paths.insert("plugin-hosts".to_string(), hosts_dir.clone());
    library_paths.insert("plugin-session-tools".to_string(), session_tools_dir.clone());
    library_paths.insert("plugin-snippets".to_string(), snippets_dir.clone());
    library_paths.insert("plugin-tunnels".to_string(), tunnels_dir.clone());

    let include_paths = vec![
        common_dir.clone(),
        common_dir.join("themes"),
        common_dir.join("shared"),
        common_dir.join("components"),
        common_dir.join("features"),
        manifest_dir.join("ui"),
        manifest_dir.join("ui/views"),
        manifest_dir.join("ui/views/center_terminal"),
        manifest_dir.join("ui/views/left_drawers"),
        manifest_dir.join("ui/components"),
    ];

    println!("cargo:rerun-if-changed=ui/kernel.slint");
    println!("cargo:rerun-if-changed=ui/views");
    println!("cargo:rerun-if-changed=ui/components");
    println!("cargo:rerun-if-changed=translations");

    let config = slint_build::CompilerConfiguration::new()
        .with_library_paths(library_paths)
        .with_include_paths(include_paths)
        .with_bundled_translations("translations")
        .with_default_translation_context(slint_build::DefaultTranslationContext::None);

    slint_build::compile_with_config("ui/kernel.slint", config).expect("smagical-ui-kernel 应该可以编译");
}
