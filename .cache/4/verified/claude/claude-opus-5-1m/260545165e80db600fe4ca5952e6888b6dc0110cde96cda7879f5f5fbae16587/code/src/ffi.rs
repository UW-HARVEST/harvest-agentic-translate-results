//! C ABI surface of the translated library: exports exactly the symbols the C
//! shared library built from `c_src/src/lib.c` exports, with the same
//! signatures, so that an external caller cannot tell the two apart.

use crate::mem::RawMem;
use std::os::raw::{c_char, c_int};

/// ```c
/// int process_strings(char *input, size_t input_len,
///                    const char *reference, size_t ref_len,
///                    int operation, uint32_t flags);
/// ```
///
/// # Safety
///
/// Same contract as the C function: `input` and `reference` are either NULL or
/// point to readable buffers.  Exactly like the C code, the strings are *not*
/// required to be NUL terminated, in which case the function reads past their
/// end (this is the behaviour the original code has and it is preserved here).
#[no_mangle]
pub unsafe extern "C" fn process_strings(
    input: *mut c_char,
    input_len: usize,
    reference: *const c_char,
    ref_len: usize,
    operation: c_int,
    flags: u32,
) -> c_int {
    crate::strcpy_fun::process_strings(
        &RawMem,
        input as usize,
        input_len,
        reference as usize,
        ref_len,
        operation,
        flags,
    )
}
