use std::ffi::c_char;

/// Computes the length of the initial segment of `s1` which consists
/// entirely of bytes not in `s2`. Mirrors C's `strcspn`.
///
/// # Safety
/// Both pointers must be valid, NUL-terminated C strings.
unsafe fn strcspn_rs(s1: *const c_char, s2: *const c_char) -> usize {
    // Build a 256-bit lookup of bytes that appear in s2 (excluding the
    // terminating NUL). strcspn returns the index of the first byte in s1
    // that appears in s2, or strlen(s1) if no such byte exists. The
    // terminating NUL of s1 is treated as a "match" since the NUL byte is
    // implicitly part of the s2 set (the empty string in s2 makes the NUL
    // a stop character).
    let mut set = [false; 256];
    let mut p = s2;
    unsafe {
        while *p != 0 {
            set[*p as u8 as usize] = true;
            p = p.add(1);
        }
    }
    // NUL terminator of s1 stops the scan as well.
    set[0] = true;

    let mut n: usize = 0;
    unsafe {
        loop {
            let b = *s1.add(n) as u8;
            if set[b as usize] {
                return n;
            }
            n += 1;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(s1: *const c_char, s2: *const c_char) {
    let n = unsafe { strcspn_rs(s1, s2) };
    // Match C's `printf("%zu\n", ...)` exactly — number followed by '\n'.
    println!("{}", n);
}
