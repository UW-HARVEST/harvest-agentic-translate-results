// build.rs — registers triggers so cargo rebuilds when the linker
// wrapper changes. The wrapper itself (cc_linker_wrapper.sh) patches
// rustc's `--version-script` to add `_init` and `_fini` to the
// exported global symbols, matching the C `libdriver.so`.
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=cc_linker_wrapper.sh");
    println!("cargo:rerun-if-changed=.cargo/config.toml");
}
