// Translation of c_src/src/lib.c to Rust.
//
// The original C source defines a single function `wcscat` that mirrors the
// semantics of Microsoft's `wcscat_s` style "safe" string concatenation. There
// is no `main` in the C package — it is built as a shared library — so the
// executable produced here exposes the same routine and a `main` that performs
// no I/O, yielding byte-identical (empty) output for any input.

/// `wchar_t` on the C side is treated as a 32-bit value here. The exact width
/// does not affect the byte-for-byte output because the executable produces no
/// output, but using `u32` matches the typical Linux `wchar_t` width used by
/// the original CMake build.
#[allow(non_camel_case_types)]
pub type wchar_t = u32;

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
/// The translation preserves the exact order of validation checks and the
/// error codes (22 for invalid arguments, 34 for an out-of-range result, and 0
/// for success). Bugs in the original C — such as resetting `dst[0]` to 0 when
/// the destination is too small (which truncates rather than restores the
/// original string) — are reproduced verbatim per the translation rules.
///
/// # Safety
///
/// `dst` must either be null or point to a writable buffer of at least
/// `num_elem` `wchar_t` elements. `src` must either be null or point to a
/// null-terminated `wchar_t` string. The function performs the same pointer
/// arithmetic the C code does and is therefore `unsafe`.
pub unsafe fn wcscat(
    dst: *mut wchar_t,
    num_elem: usize,
    src: *const wchar_t,
) -> i32 {
    let mut ptr = dst;
    if dst.is_null() || num_elem == 0 {
        return 22;
    }
    if src.is_null() {
        // SAFETY: dst is non-null and num_elem > 0, so dst[0] is valid.
        unsafe { *dst = 0 };
        return 22;
    }

    // SAFETY: dst points to a buffer of length num_elem.
    let end = unsafe { dst.add(num_elem) };

    // Advance ptr past the existing contents of dst, stopping either at the
    // end of the buffer or at the first zero element.
    while ptr < end && unsafe { *ptr } != 0 {
        ptr = unsafe { ptr.add(1) };
    }

    let mut s = src;
    while ptr < end {
        // SAFETY: ptr is in [dst, dst + num_elem) and s is advanced one element
        // at a time over a null-terminated source string.
        let value = unsafe { *s };
        unsafe { *ptr = value };
        ptr = unsafe { ptr.add(1) };
        s = unsafe { s.add(1) };
        if value == 0 {
            return 0;
        }
    }

    // Reproduce the C behavior verbatim: on overflow, the destination's first
    // element is zeroed and 34 is returned.
    unsafe { *dst = 0 };
    34
}

fn main() {
    // The original C package is a shared library and has no `main`, so the
    // executable produces no output for any input.
}
