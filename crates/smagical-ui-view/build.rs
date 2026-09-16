fn main() {
    println!("cargo:rerun-if-changed=ui/main.slint");
    println!("cargo:rerun-if-changed=ui/themes");
    println!("cargo:rerun-if-changed=ui/assets");
    println!("cargo:rerun-if-changed=translations");

    let config = slint_build::CompilerConfiguration::new()
        .with_bundled_translations("translations")
        .with_default_translation_context(slint_build::DefaultTranslationContext::None);
    slint_build::compile_with_config("ui/main.slint", config).expect("Slint UI 应该可以编译");
}
