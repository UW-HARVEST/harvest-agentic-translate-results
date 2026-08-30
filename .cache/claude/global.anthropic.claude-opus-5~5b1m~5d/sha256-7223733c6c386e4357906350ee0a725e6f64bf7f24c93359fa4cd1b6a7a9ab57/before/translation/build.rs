// Emit the same SONAME the original CMake build stamps onto
// `libString_Slice.so`, so this cdylib is a drop-in replacement both for
// `dlopen()` and for direct linking (`-lString_Slice`).
fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();

    if target_env != "msvc" && target_os != "windows" {
        let flag = if target_os == "macos" || target_os == "ios" {
            "-Wl,-install_name,libString_Slice.so"
        } else {
            "-Wl,-soname,libString_Slice.so"
        };
        println!("cargo:rustc-cdylib-link-arg={flag}");
    }

    println!("cargo:rerun-if-changed=build.rs");
}
