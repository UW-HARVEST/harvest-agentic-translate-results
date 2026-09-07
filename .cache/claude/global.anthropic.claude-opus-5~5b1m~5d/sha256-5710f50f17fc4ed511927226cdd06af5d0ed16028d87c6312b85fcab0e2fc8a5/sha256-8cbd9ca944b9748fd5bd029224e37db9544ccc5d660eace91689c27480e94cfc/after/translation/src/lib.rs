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

    // Use the platform's `strcspn` rather than reimplementing it.
    //
    // This is deliberate, and it is required for behavioural equivalence — a
    // hand-rolled `strcspn` is NOT a drop-in substitute even though it agrees
    // on every well-formed input:
    //
    //   * The obvious portable implementation scans `s1` in the outer loop and
    //     `s2` in the inner loop, so it never touches `s2` when `s1` is empty.
    //     glibc instead materialises the reject set from `s2` up front, so
    //     `driver("", NULL)` faults in C but would quietly print `0` with a
    //     hand-rolled version. Argument-inspection order is observable here.
    //   * glibc's vectorised path over-reads within the current page and
    //     dispatches on the length of `s2` and on pointer alignment; the exact
    //     byte at which an unterminated buffer faults follows from that, not
    //     from a naive byte-at-a-time scan.
    //
    // Calling the same libc routine the C calls makes all of this identical by
    // construction instead of by imitation.
    fn strcspn(s1: *const c_char, s2: *const c_char) -> usize;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(s1: *const c_char, s2: *const c_char) {
    printf(b"%zu\n\0".as_ptr() as *const c_char, strcspn(s1, s2));
}
