//! Translation of the buffer helpers `find_value_in_buffer` and
//! `create_numeric_buffer` from `c_src/src/lib.c`.

use core::ffi::c_char;
use core::ffi::c_int;

/// Equivalent of C `memchr`.
///
/// `memchr` interprets its `int` needle as an `unsigned char`, which is why the
/// caller passes an already-narrowed `u8`. Returns the index of the first match.
///
/// A zero `size` never dereferences `haystack`, matching glibc's behaviour when
/// the pointer is NULL and the length is zero.
#[inline]
pub fn memchr(haystack: *const c_char, needle: u8, size: usize) -> Option<usize> {
    if size == 0 {
        return None;
    }

    // SAFETY: the caller guarantees `haystack` points to at least `size`
    // readable bytes, exactly as the C code guarantees to `memchr`.
    let bytes = unsafe { core::slice::from_raw_parts(haystack as *const u8, size) };
    bytes.iter().position(|&b| b == needle)
}

/// C: `int find_value_in_buffer(const char *buffer, size_t size, int search_val)`
///
/// The original narrows `search_val` through `char` before calling `memchr`,
/// which then re-widens it and compares as `unsigned char`. The net effect is a
/// comparison against the low byte of `search_val`, so `find_value_in_buffer(b,
/// n, 300)` looks for byte 44 and `... , -1)` looks for byte 255.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_value_in_buffer(
    buffer: *const c_char,
    size: usize,
    search_val: c_int,
) -> c_int {
    // `char target = (char)search_val;` -- `char` is signed on x86-64 Linux,
    // but only the low 8 bits survive either way.
    let target = search_val as u8;

    match memchr(buffer, target, size) {
        // `return (int)((char*)result - buffer);` -- the pointer difference is
        // narrowed to `int` by the original cast.
        Some(offset) => offset as c_int,
        None => -1,
    }
}

/// C: `void create_numeric_buffer(char *buffer, int size, int seed)`
///
/// Fills `buffer[i] = (char)((seed + i * 7) % 256)`.
///
/// Two C details are preserved deliberately:
///
/// * `seed + i * 7` is `int` arithmetic and can overflow for extreme seeds.
///   That is UB in C but wraps in practice, so wrapping arithmetic is used.
/// * C's `%` truncates toward zero, so a negative `seed` yields a negative
///   remainder, and the subsequent `(char)` conversion wraps it into the signed
///   char range (e.g. `132` stays `-124` once reinterpreted).
///
/// A non-positive `size` simply performs no iterations, as in the original loop.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_numeric_buffer(buffer: *mut c_char, size: c_int, seed: c_int) {
    if size <= 0 {
        return;
    }

    // SAFETY: the caller guarantees `buffer` is writable for `size` bytes, the
    // same contract the C function relies on.
    let out = unsafe { core::slice::from_raw_parts_mut(buffer, size as usize) };

    for (i, slot) in out.iter_mut().enumerate() {
        let value = seed.wrapping_add((i as c_int).wrapping_mul(7)) % 256;
        // Rust's `%` truncates toward zero just like C's, and `as c_char`
        // performs the same truncating conversion GCC applies for `(char)`.
        *slot = value as c_char;
    }
}
