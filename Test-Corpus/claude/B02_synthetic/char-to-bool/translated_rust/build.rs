// Copyright 2025 MIT Lincoln Laboratory
//
// Build helper: tell the linker to skip the standard crti.o / crtn.o startup
// files when producing the cdylib. We supply our own `_init` and `_fini`
// stubs in `src/lib.rs` (marked `#[no_mangle]`) so they appear in the Rust
// .so's dynamic symbol table, exactly as the C shared library exports them.

fn main() {
    if cfg!(target_family = "unix") {
        // -nostartfiles drops crti.o / crtn.o from the link line, leaving
        // our own `_init`/`_fini` symbols as the sole definitions.
        println!("cargo:rustc-cdylib-link-arg=-nostartfiles");
        println!("cargo:rerun-if-changed=build.rs");
    }
}
