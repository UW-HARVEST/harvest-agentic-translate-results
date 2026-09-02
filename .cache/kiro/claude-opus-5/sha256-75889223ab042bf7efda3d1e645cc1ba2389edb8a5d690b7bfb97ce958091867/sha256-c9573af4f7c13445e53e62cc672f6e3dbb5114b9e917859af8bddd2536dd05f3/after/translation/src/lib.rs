//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI surface (from `nm -D` on the C shared object):
//!   * `normalize`
//!
//! The header declares no namespace-renaming macros, so the linker symbol
//! matches the source-level name exactly.

use std::ffi::{c_int, c_void};

unsafe extern "C" {
    /// The C source calls `memset` directly, so this translation does too.
    ///
    /// `core::ptr::write_bytes` would be the idiomatic choice, but it carries an
    /// enabled `check_language_ub` precondition assert that aborts on a null
    /// destination — even for a zero length — which would diverge from the C's
    /// `SIGSEGV`/no-op behaviour. Calling libc's `memset` reproduces the C
    /// exactly, including `memset(NULL, 0, 0)` being harmless.
    fn memset(dest: *mut c_void, c: c_int, n: usize) -> *mut c_void;
}

/// Translation of:
///
/// ```c
/// void normalize(float *dest, const float *src, int size) {
///     float sum = 0.0f;
///     int i;
///     for (i = 0; i < size; i++)
///         sum += src[i] * src[i];
///     if (sum > 0.0f) {
///         sum = 1.0f / sqrtf(sum);
///         for (i = 0; i < size; i++)
///             dest[i] = src[i] * sum;
///     } else if (dest != src) {
///         memset(dest, 0, size * sizeof(float));
///     }
/// }
/// ```
///
/// Behaviour notes preserved verbatim from the C:
///   * The accumulation is performed in `f32` (single precision) in ascending
///     index order, so rounding matches the C compiled without fast-math.
///   * The reciprocal square root is computed as `1.0f / sqrtf(sum)` — a
///     division of the correctly-rounded square root, *not* an approximate
///     `rsqrt`.
///   * `sum > 0.0f` is false for `sum == 0.0` and for `NaN`, in which case the
///     `else if` branch is taken (zero-fill when `dest != src`).
///   * The `memset` length is `size * sizeof(float)` where `size` is an `int`
///     converted to `size_t` by the usual arithmetic conversions. A negative
///     `size` therefore wraps to a huge length, exactly as in C.
///   * When `sum <= 0.0f` and `dest == src`, nothing is written at all.
///
/// # Safety
///
/// Same contract as the C function: `src` must be readable for `size`
/// elements, and `dest` writable for `size` elements.
///
/// # Why `wrapping_add` + `read`/`write` instead of `*p.offset(i)`
///
/// A language-level raw dereference (`*p.offset(i)`) makes rustc insert
/// `-Cub-checks` assertions (enabled by default whenever `debug-assertions` is
/// on). Those turn C's *observable* behaviour on an invalid pointer — a
/// `SIGSEGV` from the faulting access — into a Rust `SIGABRT` with a
/// "null pointer dereference occurred" message. Since this function is a
/// drop-in ABI replacement for the C one, that difference is itself a
/// divergence: a caller passing `NULL` must get the same signal from both
/// libraries in every build profile. `wrapping_add` has no preconditions and
/// `ptr::read`/`ptr::write` carry no enabled UB check, so the faulting access
/// reaches the hardware exactly as the C compiler's does.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn normalize(dest: *mut f32, src: *const f32, size: c_int) {
    let mut sum: f32 = 0.0f32;

    let mut i: c_int = 0;
    while i < size {
        let v = unsafe { src.wrapping_add(i as usize).read() };
        sum += v * v;
        i += 1;
    }

    if sum > 0.0f32 {
        sum = 1.0f32 / sum.sqrt();
        i = 0;
        while i < size {
            let v = unsafe { src.wrapping_add(i as usize).read() };
            unsafe { dest.wrapping_add(i as usize).write(v * sum) };
            i += 1;
        }
    } else if dest as *const f32 != src {
        // Reproduce C's `size * sizeof(float)` size_t arithmetic, including
        // the wraparound for negative `size`.
        let len = (size as i64 as u64).wrapping_mul(std::mem::size_of::<f32>() as u64) as usize;
        unsafe { memset(dest as *mut c_void, 0, len) };
    }
}
