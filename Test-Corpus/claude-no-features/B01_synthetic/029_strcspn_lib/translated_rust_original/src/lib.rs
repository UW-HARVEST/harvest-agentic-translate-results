// Copyright 2025 MIT Lincoln Laboratory
// Translated to Rust.

use std::ffi::c_char;

/// Compute the length of the initial segment of `s1` consisting entirely of
/// bytes not present in `s2`, matching C's `strcspn`.
///
/// # Safety
/// `s1` and `s2` must be valid, non-null, NUL-terminated C strings.
unsafe fn strcspn(s1: *const c_char, s2: *const c_char) -> usize {
    // Build a 256-byte lookup table of bytes contained in s2.
    let mut table = [false; 256];
    let mut p = s2;
    unsafe {
        while *p != 0 {
            table[*p as u8 as usize] = true;
            p = p.add(1);
        }
    }

    // Walk s1 until we hit either NUL or a byte present in s2.
    let mut count: usize = 0;
    let mut q = s1;
    unsafe {
        while *q != 0 && !table[*q as u8 as usize] {
            count += 1;
            q = q.add(1);
        }
    }
    count
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(s1: *const c_char, s2: *const c_char) {
    let n = unsafe { strcspn(s1, s2) };
    println!("{}", n);
}
