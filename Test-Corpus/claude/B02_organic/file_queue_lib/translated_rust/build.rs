// Build script to ensure the Rust shared library exports the same dynamic
// symbols as the C reference build. In particular, `_init` and `_fini` are
// linker-generated `.init`/`.fini` section markers that the C build exposes
// in the dynamic symbol table. By default rustc/lld marks them as local;
// we use a `--dynamic-list` to force them into the dynamic symbol table.
use std::path::PathBuf;

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dynlist = PathBuf::from(&out_dir).join("dynlist.ld");
    std::fs::write(
        &dynlist,
        "{\n    _init;\n    _fini;\n};\n",
    )
    .unwrap();

    let target = std::env::var("TARGET").unwrap_or_default();
    if target.contains("linux") {
        println!(
            "cargo:rustc-link-arg=-Wl,--dynamic-list={}",
            dynlist.display()
        );
    }
    println!("cargo:rerun-if-changed=build.rs");
}
