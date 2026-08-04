// Promote linker-generated symbols (_init, _fini, __bss_start, _edata, _end)
// into the dynamic symbol table so the cdylib's exported symbol surface
// matches the C-built shared library's surface.
use std::path::PathBuf;

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let list = PathBuf::from(manifest_dir).join("extra_dynamic.list");
    println!("cargo:rerun-if-changed={}", list.display());
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,--dynamic-list={}",
        list.display()
    );
}
