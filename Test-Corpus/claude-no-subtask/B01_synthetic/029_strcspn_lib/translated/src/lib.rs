use std::ffi::c_char;
use std::io::Write;

/// Computes the length of the initial segment of s1 that contains no
/// characters from s2 (mirrors the semantics of C's `strcspn`).
unsafe fn strcspn(s1: *const c_char, s2: *const c_char) -> usize {
    // Build a 256-entry lookup table for the bytes contained in s2.
    let mut reject = [false; 256];
    let mut p = s2;
    unsafe {
        while *p != 0 {
            reject[*p as u8 as usize] = true;
            p = p.add(1);
        }
    }

    let mut count: usize = 0;
    let mut q = s1;
    unsafe {
        while *q != 0 {
            if reject[*q as u8 as usize] {
                break;
            }
            count += 1;
            q = q.add(1);
        }
    }
    count
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(s1: *const c_char, s2: *const c_char) {
    let n = unsafe { strcspn(s1, s2) };
    // Mimic C's `printf("%zu\n", ...)`.
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    let _ = write!(handle, "{}\n", n);
    let _ = handle.flush();
}
