// Build script for the `driver` translation crate.
//
// The library itself needs no build-time code generation; this script exists
// only to support the differential test-suite in `tests/`:
//
//   * `-rdynamic` (`--export-dynamic`) on the *test* binaries so that a
//     `#[no_mangle] extern "C" fn malloc` defined in a test can interpose on
//     the `malloc` calls made by the two dlopen'ed shared objects (the C
//     `libdriver.so` and the Rust `libdriver.so`).  That is the only portable
//     way to deterministically exercise the C code's allocation-failure
//     branches (`create_task_manager` -> NULL, `driver` -> EXIT_FAILURE) in
//     both implementations and compare them.
//
// `rustc-link-arg-tests` applies to test targets only, so the shipped cdylib
// is unaffected.
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") {
        println!("cargo:rustc-link-arg-tests=-rdynamic");
    }
}
