//! String, buffer and validation helpers exported by `src/lib.c`.

use core::ffi::{c_char, c_int, c_void};
use core::ptr;

use crate::cruntime;

/// `typedef int (*operation_func)(int);`
///
/// Modelled as an `Option<...>` so that the C-side NULL is representable while
/// keeping the same single-pointer ABI (null pointer optimisation).
pub type OperationFunc = Option<unsafe extern "C" fn(c_int) -> c_int>;

/// ```c
/// int is_string_empty(const char *str) {
///     if (!str) return 1;
///     if (*str) {
///         return 0;
///     }
///     return 1;
/// }
/// ```
///
/// Note the NULL pointer and the empty string are conflated into the same `1`
/// result; that is the C behaviour and is kept as-is.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn is_string_empty(str: *const c_char) -> c_int {
    if str.is_null() {
        return 1;
    }
    // SAFETY: non-NULL was just established; the caller guarantees the pointer
    // is dereferenceable, exactly as the C original requires.
    if unsafe { *str } != 0 {
        return 0;
    }
    1
}

/// ```c
/// char* find_char_in_buffer(const char *buffer, size_t size, char target) {
///     if (!buffer) return NULL;
///     return (char*)memchr(buffer, target, size);
/// }
/// ```
///
/// `target` is a `char`, which on the x86-64 SysV ABI is signed. C promotes it
/// to `int` (sign extending) for the `memchr` call, and `memchr` then truncates
/// back to `unsigned char`; casting through `c_int` here reproduces that chain.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_char_in_buffer(
    buffer: *const c_char,
    size: usize,
    target: c_char,
) -> *mut c_char {
    if buffer.is_null() {
        return ptr::null_mut();
    }
    // SAFETY: delegated to libc's memchr under the same contract the C code
    // imposes on its callers (`buffer` readable for `size` bytes).
    unsafe { cruntime::memchr(buffer as *const c_void, target as c_int, size) as *mut c_char }
}

/// ```c
/// char* create_buffer(const char *initial) {
///     if (!initial) return NULL;
///     size_t len = strlen(initial);
///     char *buffer = (char*)malloc(len + 1);
///     if (buffer) {
///         strcpy(buffer, initial);
///     }
///     return buffer;
/// }
/// ```
///
/// The allocation must come from libc `malloc`: the returned pointer is handed
/// to callers (and to `charinbuf`) that release it with `free`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_buffer(initial: *const c_char) -> *mut c_char {
    if initial.is_null() {
        return ptr::null_mut();
    }

    // SAFETY: `initial` is a non-NULL NUL-terminated string per the C contract.
    let len = unsafe { cruntime::strlen(initial) };
    // `len + 1` is written exactly as in C, including its wrap-around behaviour
    // on a (practically unreachable) SIZE_MAX-length string.
    let buffer = unsafe { cruntime::malloc(len.wrapping_add(1)) } as *mut c_char;

    if !buffer.is_null() {
        // SAFETY: `buffer` holds `len + 1` bytes, enough for the string plus NUL.
        unsafe { cruntime::strcpy(buffer, initial) };
    }

    buffer
}

/// ```c
/// int validate_uint16_range(int value) {
///     if (value < 0) return 0;
///     if (value > UINT16_MAX) return 0;
///     return 1;
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn validate_uint16_range(value: c_int) -> c_int {
    if value < 0 {
        return 0;
    }
    if value > crate::UINT16_MAX {
        return 0;
    }
    1
}

/// ```c
/// int apply_operation(operation_func op, int value) {
///     if (!op) return -1;
///     return op(value);
/// }
/// ```
///
/// The `-1` sentinel for a NULL callback is indistinguishable from an operation
/// that legitimately returns `-1`; that ambiguity exists in the C and is kept.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn apply_operation(op: OperationFunc, value: c_int) -> c_int {
    match op {
        None => -1,
        // SAFETY: the caller supplies a valid `int (*)(int)`, as in C.
        Some(op) => unsafe { op(value) },
    }
}
