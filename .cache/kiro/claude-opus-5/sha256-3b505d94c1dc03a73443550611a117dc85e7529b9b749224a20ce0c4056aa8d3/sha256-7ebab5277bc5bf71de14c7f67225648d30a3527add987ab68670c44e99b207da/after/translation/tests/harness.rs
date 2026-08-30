//! Sanity checks on the loading harness itself and on the exported symbol set.

mod common;

use common::{c_api, c_lib, capture, rust_api, rust_lib, sym, PrintIntLineFn};
use std::ffi::c_char;

/// The two libraries must resolve to genuinely distinct code, otherwise every
/// comparison below would be vacuously true. `dlopen` defaults to `RTLD_LOCAL`,
/// so neither library's definitions leak into the other's resolution scope.
fn libraries_are_independent() {
    let c_print = sym::<PrintIntLineFn>(c_lib(), "printIntLine");
    let rust_print = sym::<PrintIntLineFn>(rust_lib(), "printIntLine");
    assert_ne!(
        *c_print as usize, *rust_print as usize,
        "C and Rust `printIntLine` resolved to the same address"
    );

    for name in ["printLine", "bad", "good", "driver"] {
        let c_addr = *sym::<*const ()>(c_lib(), name);
        let rust_addr = *sym::<*const ()>(rust_lib(), name);
        assert_ne!(c_addr, rust_addr, "`{name}` resolved to the same address");
    }
}

/// Every symbol exported by the C shared object must also be exported by the
/// Rust one, under the exact same name.
fn rust_exports_every_c_symbol() {
    for name in ["printLine", "printIntLine", "bad", "good", "driver"] {
        let _ = sym::<*const ()>(c_lib(), name);
        let _ = sym::<*const ()>(rust_lib(), name);
    }
}

/// Guards the capture helper: stdout redirection must be restored and the bytes
/// must be attributed to the right library.
fn capture_round_trips() {
    let text = c"harness check";
    let c_bytes = capture(|| unsafe { (c_api().print_line)(text.as_ptr() as *const c_char) });
    let rust_bytes = capture(|| unsafe { (rust_api().print_line)(text.as_ptr() as *const c_char) });
    assert_eq!(c_bytes, b"harness check\n");
    assert_eq!(rust_bytes, b"harness check\n");
}

fn main() {
    common::run_suite(
        "harness",
        &[
        ("libraries_are_independent", libraries_are_independent),
        ("rust_exports_every_c_symbol", rust_exports_every_c_symbol),
        ("capture_round_trips", capture_round_trips),
        ],
    )
}
