use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dynlist = manifest_dir.join("exports.ver");
    println!("cargo:rerun-if-changed={}", dynlist.display());
    // Add additional symbols to the dynamic export list. Rust already passes
    // its own --version-script to restrict exports to #[no_mangle] symbols,
    // so we use --dynamic-list which is additive (it cannot remove symbols
    // restricted by the version script). To keep our extra symbols, the
    // linker will see them as exported because they are referenced by the
    // dynamic list, even though they are linker-generated.
    println!(
        "cargo:rustc-link-arg=-Wl,--dynamic-list={}",
        dynlist.display()
    );
}
