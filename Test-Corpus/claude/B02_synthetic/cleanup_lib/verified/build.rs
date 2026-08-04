// Make the linker-generated `_init` and `_fini` symbols globally exported,
// matching the C library's exported symbol set.

use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dynamic_list = manifest_dir.join("exports.list");
    println!("cargo:rerun-if-changed=exports.list");
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,--dynamic-list={}",
        dynamic_list.display()
    );
}
