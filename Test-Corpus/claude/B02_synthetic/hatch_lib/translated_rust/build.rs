// Build script that mirrors the C library's exported symbol set.
//
// `cc` (and the GNU linker) emits `_init` / `_fini` from crti.o/crtn.o into
// every shared library, but Rust's default `--version-script` confines the
// dynamic-symbol table to the symbols Rust itself exports, hiding `_init`
// and `_fini`. The C build does not apply such a version script, so those
// symbols stay public.
//
// To keep the dynamic-symbol table identical to the C build, we instruct
// `rustc` to allow multiple definitions and we provide our own (empty)
// `_init` and `_fini` functions in lib.rs marked `#[no_mangle]`. With
// `-z muldefs`, our definitions are emitted into the dynamic symbol table
// while the duplicate copies from crti.o / crtn.o are silently discarded.
fn main() {
    println!("cargo:rustc-link-arg=-Wl,-z,muldefs");
}
