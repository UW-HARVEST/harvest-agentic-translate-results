// Library entry point exposing the C-compatible API for the decisions
// processor. This crate is built as both an `rlib` (for the binary) and a
// `cdylib` (for FFI parity testing against the original C shared library).

pub mod lib_decisions;

use std::os::raw::{c_char, c_int};

/// C-compatible export of `process_decisions`.
///
/// Mirrors the original C signature:
/// ```c
/// int process_decisions(char *decision_string, size_t length,
///                       int operation, int param);
/// ```
///
/// # Safety
/// `decision_string` must either be null or point to at least `length`
/// readable bytes. The C version may temporarily reuse the buffer (rule
/// validation), so the buffer must also be writable when `operation == 3`.
#[no_mangle]
pub unsafe extern "C" fn process_decisions(
    decision_string: *mut c_char,
    length: usize,
    operation: c_int,
    param: c_int,
) -> c_int {
    if decision_string.is_null() || length == 0 {
        return -1;
    }
    let slice = std::slice::from_raw_parts(decision_string as *const u8, length);
    lib_decisions::process_decisions(slice, length, operation, param)
}
