// Rust translation of c_src/ (MIT Lincoln Laboratory `driver` library).
//
// Public ABI (from `nm -D` on the C shared object):
//     T driver
//
// The C implementation is:
//     void driver(const char *s1, const char *s2) {
//         printf("%zu\n", strcspn(s1, s2));
//     }

use std::ffi::c_char;
use std::ffi::c_int;

extern "C" {
    // Use the platform's printf so that formatting *and* stdio buffering
    // semantics are byte-for-byte identical to the C library's output.
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// Faithful reimplementation of C's `strcspn`.
///
/// Returns the length of the initial segment of `s1` consisting of characters
/// that do not appear in `s2`.  The terminating NUL of `s2` is not considered
/// part of the reject set, so an empty `s2` yields `strlen(s1)`.
///
/// # Safety
/// `s1` and `s2` must be valid NUL-terminated C strings (the C code has the
/// same requirement and would likewise fault on NULL).
unsafe fn strcspn(s1: *const c_char, s2: *const c_char) -> usize {
    let mut p = s1;
    loop {
        let c = *p;
        if c == 0 {
            break;
        }
        let mut q = s2;
        loop {
            let d = *q;
            if d == 0 {
                break;
            }
            if d == c {
                return p.offset_from(s1) as usize;
            }
            q = q.add(1);
        }
        p = p.add(1);
    }
    p.offset_from(s1) as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(s1: *const c_char, s2: *const c_char) {
    printf(b"%zu\n\0".as_ptr() as *const c_char, strcspn(s1, s2));
}
