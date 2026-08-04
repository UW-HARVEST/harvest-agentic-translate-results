// Translation of c_src/src/driver.c to Rust producing byte-identical output.

use std::ffi::c_char;

/// Compute the length of the initial segment of `s1` consisting entirely of
/// bytes NOT in `s2`. Mirrors C's `strcspn`.
///
/// # Safety
/// `s1` and `s2` must each point to a valid NUL-terminated C string.
unsafe fn strcspn_rust(s1: *const c_char, s2: *const c_char) -> usize {
    // Build a 256-bit lookup table of bytes present in s2.
    let mut reject = [false; 256];
    let mut p = s2;
    loop {
        let b = *p as u8;
        if b == 0 {
            break;
        }
        reject[b as usize] = true;
        p = p.add(1);
    }

    let mut count: usize = 0;
    let mut q = s1;
    loop {
        let b = *q as u8;
        if b == 0 || reject[b as usize] {
            return count;
        }
        count += 1;
        q = q.add(1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(s1: *const c_char, s2: *const c_char) {
    let n = strcspn_rust(s1, s2);
    // Match the C printf("%zu\n", ...) output exactly via libc::printf so
    // buffering and formatting are identical to the C version.
    let fmt = b"%zu\n\0".as_ptr() as *const c_char;
    libc::printf(fmt, n);
}
