fn main() {
    // Only link XCB when the x11 feature is enabled
    #[cfg(feature = "x11")]
    println!("cargo:rustc-link-lib=xcb");
    
    println!("cargo:rerun-if-changed=build.rs");
}
