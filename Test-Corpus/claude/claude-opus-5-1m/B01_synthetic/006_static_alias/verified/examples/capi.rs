// C ABI shared object exposing the very same symbol surface as the C
// translation unit c_src/src/main.c, i.e. `main` and `static_alias`.
//
// `static_alias` is exported by the library itself (`#[no_mangle]` in
// src/lib.rs) and is re-exported by this cdylib; `main` is exported here so that
// the `#[no_mangle]` entry point cannot collide with the `driver` binary's Rust
// entry point.
//
// Build:  cargo build --example capi   ->  target/<profile>/examples/libcapi.so

/// `int main(int argc, char **argv)` of c_src/src/main.c.
///
/// # Safety
/// `argv` must be a valid `argc`-element array of NUL-terminated C strings.
///
/// `cfg(not(test))`: when cargo compiles this target in test mode
/// (`cargo test --all-targets`) rustc generates its own libtest entry point, and
/// two `main` symbols cannot coexist. The cdylib the differential tests load is
/// always built in normal mode, where the export is present.
#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn main(
    argc: std::ffi::c_int,
    argv: *mut *mut std::ffi::c_char,
) -> std::ffi::c_int {
    driver::c_main(argc, argv)
}
