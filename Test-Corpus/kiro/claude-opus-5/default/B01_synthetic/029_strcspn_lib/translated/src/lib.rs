// Rust translation of c_src/src/driver.c
//
// Original C:
//     void driver(const char *s1, const char *s2) {
//         printf("%zu\n", strcspn(s1, s2));
//     }
//
// Output must be byte-identical to the C version, so we print through the C
// runtime's `printf` (same stdio stream / buffering as the original library).

use std::ffi::{c_char, c_int};

unsafe extern "C" {
    #[link_name = "printf"]
    unsafe fn c_printf(fmt: *const c_char, ...) -> c_int;
}

/// Reimplementation of `strcspn`: length of the initial segment of `s1` that
/// contains no byte from `s2`.
///
/// Matches the C library semantics, including:
/// - an empty `s2` yields `strlen(s1)`
/// - the terminating NUL of `s2` is not treated as a member of the reject set
///
/// # Safety
/// `s1` and `s2` must be valid NUL-terminated C strings, exactly as required by
/// the C original (which performs no NULL checks).
unsafe fn strcspn(s1: *const c_char, s2: *const c_char) -> usize {
    // Build a 256-entry reject table from s2 (bytes of s2, excluding its NUL).
    let mut reject = [false; 256];
    let mut p = s2;
    unsafe {
        loop {
            let b = *p as u8;
            if b == 0 {
                break;
            }
            reject[b as usize] = true;
            p = p.add(1);
        }

        let mut n: usize = 0;
        let mut q = s1;
        loop {
            let b = *q as u8;
            if b == 0 || reject[b as usize] {
                return n;
            }
            n += 1;
            q = q.add(1);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(s1: *const c_char, s2: *const c_char) {
    unsafe {
        let n = strcspn(s1, s2);
        // "%zu\n" as a NUL-terminated format string.
        c_printf(c"%zu\n".as_ptr(), n);
    }
}
