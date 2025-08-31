fn main() {
    println!("cargo:rustc-link-lib=xcb");
    println!("cargo:rerun-if-changed=build.rs");
}
