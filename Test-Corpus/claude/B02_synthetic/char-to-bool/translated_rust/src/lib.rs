// Copyright 2025 MIT Lincoln Laboratory
// Translated to Rust to produce byte-identical output.
//
// This crate exposes the public C API of the original library so that
// callers (including integration tests loading the Rust .so via libloading)
// can invoke it just like the original C implementation.

pub mod decisions;

use core::ffi::c_int;

/// C-ABI export mirroring `int process_decisions(char *, size_t, int, int)`
/// from c_src/src/lib.c.
///
/// # Safety
/// `decision_string` must point to at least `length` writable bytes when
/// `operation == 3` (the C version reuses the buffer in `validate_sequence`).
/// For other operations a read-only buffer is sufficient. A null pointer is
/// permitted only when `length == 0` — exactly the C contract.
#[no_mangle]
pub unsafe extern "C" fn process_decisions(
    decision_string: *mut u8,
    length: usize,
    operation: c_int,
    param: c_int,
) -> c_int {
    // Mirror the C code's NULL-or-empty guard. The C function returns -1 if
    // either the pointer is NULL or the length is zero.
    if decision_string.is_null() || length == 0 {
        return -1;
    }

    let buf: &mut [u8] = core::slice::from_raw_parts_mut(decision_string, length);
    decisions::process_decisions(buf, length, operation as i32, param as i32) as c_int
}

// Re-export `_init` and `_fini` so the Rust .so's dynamic symbol table mirrors
// the C .so byte-for-byte. The C shared library picks up these stubs from
// crti.o / crtn.o; for the Rust cdylib we provide them ourselves and use
// `-nostartfiles` (applied via build.rs) to suppress crti/crtn so we don't
// double-define them. These stubs are skipped under `cfg(test)` because the
// test harness builds the lib as an executable that still links crti.o.
#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn _init() {}

#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn _fini() {}
