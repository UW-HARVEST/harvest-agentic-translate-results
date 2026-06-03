use core::ffi::c_int;

/// `wchar_t` on Linux/glibc platforms is a 32-bit signed integer.
#[allow(non_camel_case_types)]
pub type wchar_t = i32;

/// Translation of the C function:
///
/// ```c
/// int wcscat(wchar_t *dst, size_t numElem, const wchar_t *src) {
///     wchar_t *ptr = dst;
///     if (!dst || numElem == 0)
///         return 22;
///     if (!src) {
///         dst[0] = 0;
///         return 22;
///     }
///     while (ptr < dst + numElem && *ptr != 0)
///         ptr++;
///     while (ptr < dst + numElem) {
///         if ((*ptr++ = *src++) == 0)
///             return 0;
///     }
///     dst[0] = 0;
///     return 34;
/// }
/// ```
///
/// # Safety
///
/// The caller must ensure that:
/// - `dst` (if non-null) points to a writable buffer of at least `num_elem`
///   `wchar_t` elements.
/// - `src` (if non-null) points to a NUL-terminated wide string whose
///   readable length is sufficient for the copy.
#[no_mangle]
pub unsafe extern "C" fn wcscat(
    dst: *mut wchar_t,
    num_elem: usize,
    src: *const wchar_t,
) -> c_int {
    let mut ptr = dst;

    if dst.is_null() || num_elem == 0 {
        return 22;
    }
    if src.is_null() {
        *dst = 0;
        return 22;
    }

    let end = dst.add(num_elem);

    // Advance ptr to the current end (NUL terminator) of dst, or to `end`.
    while ptr < end && *ptr != 0 {
        ptr = ptr.add(1);
    }

    let mut s = src;
    while ptr < end {
        let v = *s;
        *ptr = v;
        ptr = ptr.add(1);
        s = s.add(1);
        if v == 0 {
            return 0;
        }
    }

    *dst = 0;
    34
}
