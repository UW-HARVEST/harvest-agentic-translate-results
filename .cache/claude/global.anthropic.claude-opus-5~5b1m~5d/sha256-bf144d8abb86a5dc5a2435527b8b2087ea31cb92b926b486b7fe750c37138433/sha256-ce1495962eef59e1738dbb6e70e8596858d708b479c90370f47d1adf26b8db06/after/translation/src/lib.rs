//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (must match `nm -D` on the C `libdriver.so` exactly):
//!   * `tool_basename`
//!
//! Source of truth: `c_src/src/lib.c`, `c_src/include/lib.h`.
//!
//! ```c
//! char *tool_basename(char *path)
//! {
//!   char *s1;
//!   char *s2;
//!
//!   s1 = strrchr(path, '/');
//!   s2 = strrchr(path, '\\');
//!
//!   if(s1 && s2) {
//!     path = (s1 > s2) ? s1 + 1 : s2 + 1;
//!   }
//!   else if(s1)
//!     path = s1 + 1;
//!   else if(s2)
//!     path = s2 + 1;
//!
//!   return path;
//! }
//! ```

use core::ffi::{c_char, CStr};

/// Index of the last occurrence of `needle` in the NUL-terminated string at
/// `path`, mirroring `strrchr()` for a non-NUL `needle`.
///
/// Returns `None` when the byte does not occur, which stands in for the
/// `NULL` that `strrchr()` yields in that case.
///
/// # Safety
///
/// `path` must point to a valid NUL-terminated C string, exactly as the C
/// original requires of the argument it hands to `strrchr()`.
unsafe fn strrchr_index(path: *const c_char, needle: u8) -> Option<usize> {
    // `CStr::from_ptr` performs the same traversal to the NUL terminator that
    // `strrchr` does, letting the actual search run over a safe byte slice.
    let bytes = unsafe { CStr::from_ptr(path) }.to_bytes();
    bytes.iter().rposition(|&b| b == needle)
}

/// Return a pointer to the final path component of `path`.
///
/// Byte-for-byte equivalent of the C original, including its behaviour of
/// returning `path` unchanged when the string holds neither separator, and of
/// preferring whichever of `'/'` or `'\\'` occurs later in the string.
///
/// The returned pointer aliases into the caller's buffer, just as in C; no
/// copy is made and the string is not modified.
///
/// # Safety
///
/// `path` must point to a valid NUL-terminated C string. A NULL or otherwise
/// invalid pointer is undefined behaviour here just as it is in the C version,
/// which passes the pointer straight to `strrchr()` without a NULL check.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tool_basename(path: *mut c_char) -> *mut c_char {
    let s1 = unsafe { strrchr_index(path, b'/') };
    let s2 = unsafe { strrchr_index(path, b'\\') };

    // Preserve the C branch order and the `s1 > s2` pointer comparison, which
    // for two positions in one buffer is an index comparison.
    match (s1, s2) {
        (Some(i1), Some(i2)) => {
            let last = if i1 > i2 { i1 } else { i2 };
            unsafe { path.add(last + 1) }
        }
        (Some(i1), None) => unsafe { path.add(i1 + 1) },
        (None, Some(i2)) => unsafe { path.add(i2 + 1) },
        (None, None) => path,
    }
}
