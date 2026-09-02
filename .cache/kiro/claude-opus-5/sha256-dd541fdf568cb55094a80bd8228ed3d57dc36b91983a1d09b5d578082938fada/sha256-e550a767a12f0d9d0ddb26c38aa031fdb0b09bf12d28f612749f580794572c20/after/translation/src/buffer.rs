//! Translations of the buffer helpers: `find_value_in_buffer` and
//! `create_numeric_buffer`.

use core::ffi::{c_char, c_int};

/// ```c
/// int find_value_in_buffer(const char *buffer, size_t size, int search_val) {
///     char target = (char)search_val;
///     void *result = memchr(buffer, target, size);
///     if (result != NULL) {
///         return (int)((char*)result - buffer);
///     }
///     return -1;
/// }
/// ```
///
/// `search_val` is first narrowed to `char` and then widened back to `int` by
/// the default argument promotion of `memchr`; `memchr` in turn compares
/// against `(unsigned char)c`. The net effect is a search for the low byte of
/// `search_val`, which is what the byte comparison below performs.
///
/// # Safety
/// `buffer` must be valid for reads of `size` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_value_in_buffer(
    buffer: *const c_char,
    size: usize,
    search_val: c_int,
) -> c_int {
    let target = search_val as c_char;

    // `memchr` never dereferences the pointer when the length is zero.
    if size == 0 {
        return -1;
    }

    let haystack = unsafe { core::slice::from_raw_parts(buffer as *const u8, size) };
    let needle = target as u8;

    match haystack.iter().position(|&byte| byte == needle) {
        Some(offset) => offset as c_int,
        None => -1,
    }
}

/// ```c
/// void create_numeric_buffer(char *buffer, int size, int seed) {
///     for (int i = 0; i < size; i++) {
///         buffer[i] = (char)((seed + i * 7) % 256);
///     }
/// }
/// ```
///
/// `seed + i * 7` can overflow `int`, which is undefined behaviour in C but in
/// practice wraps on the target; `wrapping_*` reproduces that. C's `%` truncates
/// toward zero, matching Rust's `%`, so negative seeds keep producing negative
/// bytes exactly as the C does.
///
/// # Safety
/// `buffer` must be valid for writes of `size` bytes when `size > 0`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_numeric_buffer(buffer: *mut c_char, size: c_int, seed: c_int) {
    let mut i: c_int = 0;
    while i < size {
        let value = seed.wrapping_add(i.wrapping_mul(7)) % 256;
        unsafe { *buffer.add(i as usize) = value as c_char };
        i = i.wrapping_add(1);
    }
}
