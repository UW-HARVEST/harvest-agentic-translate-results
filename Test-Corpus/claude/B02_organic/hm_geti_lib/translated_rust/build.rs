// Match the export surface of the C shared library: in addition to the
// public `hm_geti` entry point and the stbds_* helpers (re-exported from
// lib.rs as no_mangle stubs), the C cmake build also leaves the standard
// ELF crt-section symbols `_init` and `_fini` in the dynamic symbol table.
//
// Rust's default cdylib link script hides every symbol that isn't
// marked `#[no_mangle]`, so we explicitly re-add `_init` and `_fini`
// via a linker argument to preserve byte-for-byte parity in `nm -D`.
fn main() {
    if cfg!(target_os = "linux") {
        // The C cmake build produces a libtranslated_rust.so whose dynsym
        // includes the crt-section symbols `_init` and `_fini` (defined
        // by crti.o / crtn.o as GLOBAL). Rust's cdylib link prepends an
        // auto-generated `--version-script` whose `local: *;` clause
        // localizes every symbol not on the explicit `global:` list,
        // which strips `_init` and `_fini` from the dynamic table.
        //
        // The link is routed through `cc_wrapper.sh` (configured in
        // `.cargo/config.toml`), which patches that version script in
        // flight to also include `_init` and `_fini`. We pass
        // `--undefined=_init/_fini` here as a belt-and-braces measure
        // to keep the symbols from being dropped.
        println!("cargo:rustc-link-arg=-Wl,--undefined=_init");
        println!("cargo:rustc-link-arg=-Wl,--undefined=_fini");
    }
}
