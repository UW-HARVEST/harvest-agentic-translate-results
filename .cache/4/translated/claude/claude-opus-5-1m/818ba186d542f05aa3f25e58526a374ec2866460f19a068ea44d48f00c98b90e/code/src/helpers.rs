// Translation of the string / buffer / validation helpers, plus the function
// pointer dispatcher. Every one of these is a non-static function in the C
// source and therefore part of the exported ABI.

use core::ffi::{c_char, c_int};
use core::ptr;

use crate::cstd;

/// `UINT16_MAX` from <stdint.h>. In C this macro has type `int` (65535 fits in
/// an `int`), which is what the `>` comparison below is performed in.
pub const UINT16_MAX: c_int = 65535;

/// The `operation_func` typedef: `typedef int (*operation_func)(int);`
///
/// Modelled as `Option<...>` so a null pointer is representable, matching the
/// `if (!op)` check in `apply_operation`.
pub type OperationFunc = extern "C" fn(c_int) -> c_int;

/// `int is_string_empty(const char *str)`
///
/// Returns 1 for a null pointer or an empty string, 0 otherwise.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn is_string_empty(str: *const c_char) -> c_int {
    if str.is_null() {
        return 1;
    }
    // `if (*str)` -- non-zero first byte means non-empty.
    if unsafe { *str } != 0 {
        return 0;
    }
    1
}

/// `char *find_char_in_buffer(const char *buffer, size_t size, char target)`
///
/// Null buffer yields NULL, otherwise this is a straight `memchr`. `memchr`
/// compares bytes as `unsigned char`, so `target` is reinterpreted through
/// `u8` before being widened to the `int` parameter -- this matters on targets
/// where plain `char` is signed and `target` is negative.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_char_in_buffer(
    buffer: *const c_char,
    size: usize,
    target: c_char,
) -> *mut c_char {
    if buffer.is_null() {
        return ptr::null_mut();
    }
    unsafe { cstd::memchr(buffer.cast(), c_int::from(target as u8), size).cast() }
}

/// `char *create_buffer(const char *initial)`
///
/// Duplicates `initial` into a fresh `malloc` allocation. The C version does
/// not free the allocation on success and leaves a failed `malloc` uncopied but
/// still returned (as NULL); both behaviours are preserved. Because the result
/// is handed back to the caller -- who frees it with `free` -- the allocation
/// must come from the C allocator, not Rust's.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_buffer(initial: *const c_char) -> *mut c_char {
    if initial.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        let len = cstd::strlen(initial);
        let buffer = cstd::malloc(len + 1).cast::<c_char>();

        if !buffer.is_null() {
            cstd::strcpy(buffer, initial);
        }

        buffer
    }
}

/// `int validate_uint16_range(int value)`
///
/// 1 when `0 <= value <= UINT16_MAX`, else 0. Check order preserved.
#[unsafe(no_mangle)]
pub extern "C" fn validate_uint16_range(value: c_int) -> c_int {
    if value < 0 {
        return 0;
    }
    if value > UINT16_MAX {
        return 0;
    }
    1
}

/// `int apply_operation(operation_func op, int value)`
///
/// Returns -1 for a null callback, otherwise the callback's result.
#[unsafe(no_mangle)]
pub extern "C" fn apply_operation(op: Option<OperationFunc>, value: c_int) -> c_int {
    match op {
        None => -1,
        Some(op) => op(value),
    }
}
