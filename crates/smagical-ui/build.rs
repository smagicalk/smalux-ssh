fn main() {
    println!("cargo:rerun-if-changed=ui/main.slint");
    slint_build::compile("ui/main.slint").expect("Slint UI 应该可以编译");
}
