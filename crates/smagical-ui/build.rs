fn main() {
    println!("cargo:rerun-if-changed=ui/main.slint");
    println!("cargo:rerun-if-changed=ui/themes");
    println!("cargo:rerun-if-changed=ui/assets");
    slint_build::compile("ui/main.slint").expect("Slint UI 应该可以编译");
}
