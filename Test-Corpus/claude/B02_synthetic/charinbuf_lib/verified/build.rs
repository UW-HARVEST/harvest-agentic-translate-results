use std::path::PathBuf;

fn main() {
    // Ensure _init and _fini symbols are exported from the cdylib so its
    // dynamic symbol table matches the C reference library's.
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dynamic_list = manifest_dir.join("dynamic.list");
    println!(
        "cargo:rustc-link-arg=-Wl,--dynamic-list={}",
        dynamic_list.display()
    );
    println!("cargo:rerun-if-changed={}", dynamic_list.display());
}
