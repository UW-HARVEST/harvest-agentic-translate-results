// Build script: tell the linker to include `_init` and `_fini` in the
// dynamic symbol table of the produced cdylib so the exported-symbol set
// matches the C .so (which exports them via crti.o by default).

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let dyn_list = format!("{manifest_dir}/dynamic.list");
    println!("cargo:rerun-if-changed=dynamic.list");
    println!("cargo:rustc-cdylib-link-arg=-Wl,--dynamic-list={dyn_list}");
}
