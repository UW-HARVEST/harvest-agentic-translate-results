// Match the C shared library's exported symbol set. The C .so exports
// `_init` and `_fini` (no-op stubs synthesized by GCC's crti.o/crtn.o).
// To export the same symbols from a Rust cdylib without conflicting with
// the CRT-supplied definitions, we drop the start files via -nostartfiles
// and provide our own no-op `_init` / `_fini` in lib.rs.
//
// Use `rustc-cdylib-link-arg` so the flag only affects the cdylib build,
// not the test executables (which need crt1.o for `_start`).
fn main() {
    println!("cargo:rustc-cdylib-link-arg=-nostartfiles");
}
