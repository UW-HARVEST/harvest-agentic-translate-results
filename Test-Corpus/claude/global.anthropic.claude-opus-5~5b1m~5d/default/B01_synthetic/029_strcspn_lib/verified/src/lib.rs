// Rust translation of c_src/ (MIT Lincoln Laboratory `driver` library).
//
// Public ABI (from `nm -D` on the C shared object):
//     T driver
//
// The C implementation, in full, is:
//     void driver(const char *s1, const char *s2) {
//         printf("%zu\n", strcspn(s1, s2));
//     }
//
// Both `printf` and `strcspn` are C *standard library* functions, not part of the
// translated source. They are therefore bound to the platform's libc rather than
// reimplemented, which is what makes the translation byte-identical:
//
//   * `printf`  — identical formatting of `%zu` and identical stdio buffering, so
//                 the bytes reaching stdout (and *when* they reach it) match.
//   * `strcspn` — identical result AND identical observable failure behaviour.
//
// A hand-written Rust reimplementation of `strcspn` was tried first and rejected;
// see ERRORS.md ("Divergences found and fixed") for the two ways it diverged:
//
//   1. Access ORDER. A natural nested-loop implementation checks `s1` before ever
//      touching `s2`, so `driver("", NULL)` returned 0 where the C segfaults —
//      the C library consumes the whole reject set *before* looking at `s1`.
//   2. Fault SIGNAL. Since Rust 1.78 a debug build inserts null/alignment
//      assertions on every raw-pointer dereference, so `*p` on a null pointer
//      raises a non-unwinding panic (`SIGABRT`, 6) instead of a hardware fault
//      (`SIGSEGV`, 11). The C always gives `SIGSEGV`, so a pure-Rust pointer walk
//      could not match the C in a debug build without resorting to inline asm.
//
// Delegating to libc removes both classes of divergence by construction, in every
// build profile, and additionally makes the Rust `.so`'s dynamic imports match the
// C `.so`'s (`printf@GLIBC_*` and `strcspn@GLIBC_*`).

use std::ffi::c_char;
use std::ffi::c_int;
use std::ffi::c_ulong;

extern "C" {
    /// The platform `printf`. Variadic, so the `size_t` argument below is passed
    /// exactly as the C compiler would pass it for `%zu`.
    fn printf(fmt: *const c_char, ...) -> c_int;

    /// The platform `strcspn`: length of the initial segment of `s1` made up of
    /// bytes that do not occur in `s2`.
    ///
    /// Declared with `c_ulong` because that is `size_t` on every LP64/LLP64 target
    /// this crate builds for; `driver` re-widens it to `usize` for the `printf`
    /// call so the vararg has exactly `size_t` width.
    fn strcspn(s1: *const c_char, s2: *const c_char) -> c_ulong;
}

/// `void driver(const char *s1, const char *s2)`
///
/// Prints `strcspn(s1, s2)` as an unsigned decimal followed by a newline.
///
/// # Safety
///
/// `s1` and `s2` must both be valid, NUL-terminated C strings. This is the same
/// contract the C function has: it validates nothing (the C source contains no
/// null check, range check, or assertion of any kind), so passing a null or
/// unterminated pointer faults here exactly as it does there. Note in particular
/// that an empty `s1` does *not* excuse an invalid `s2` — the reject set is read
/// unconditionally.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(s1: *const c_char, s2: *const c_char) {
    let n: usize = strcspn(s1, s2) as usize;
    printf(b"%zu\n\0".as_ptr() as *const c_char, n);
}
