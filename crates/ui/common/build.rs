fn main() {
    println!("cargo:rerun-if-changed=ui/common.slint");
    println!("cargo:rerun-if-changed=ui/themes");
    println!("cargo:rerun-if-changed=ui/assets");
    println!("cargo:rerun-if-changed=ui/shared");
    println!("cargo:rerun-if-changed=ui/features");
    println!("cargo:rerun-if-changed=translations");

    let config = slint_build::CompilerConfiguration::new();
    slint_build::compile_with_config("ui/common.slint", config).expect("Slint UI Common 应该可以编译");
}
