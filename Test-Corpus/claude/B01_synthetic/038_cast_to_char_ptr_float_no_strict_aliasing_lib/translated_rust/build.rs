fn main() {
    // Promote linker-generated _init / _fini symbols (from crti.o / crtn.o)
    // to be dynamically exported, so the Rust .so matches the C .so's
    // exported symbol set. We use a --dynamic-list file because older linkers
    // don't support --export-dynamic-symbol.
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo:rerun-if-changed=exports.list");
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,--dynamic-list={}/exports.list",
        manifest
    );
}
