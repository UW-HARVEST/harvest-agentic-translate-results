// Rust translation of c_src/src/driver.c
//
// Public ABI (matches `nm -D` on the C shared library):
//   * fma_array
//   * driver
//
// `inner` is `static` in the C source and therefore is NOT exported here.

use core::ffi::{c_char, c_int, c_void};

unsafe extern "C" {
    /// C standard library `printf`, used so that stdout buffering behavior
    /// (and therefore the exact byte stream produced) matches the C library.
    fn printf(fmt: *const c_char, ...) -> c_int;

    /// C standard library `memcpy`, used so that `driver`'s copy has exactly
    /// the same semantics (including for degenerate lengths) as the C source.
    fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
}

/// Format string used by the `printf("%d\n", ...)` call in the C source.
const FMT_D_NL: &[u8; 4] = b"%d\n\0";

/// ```c
/// void fma_array(int *out, const int *mul1, const int *mul2, const int *add, int len) {
///     for (int i = 0; i < len; i++) {
///         out[i] = mul1[i] * mul2[i] + add[i];
///     }
/// }
/// ```
///
/// The C callers pass fully aliasing pointers (`out == mul1 == mul2 == add`),
/// so raw pointer accesses are used here instead of slices.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fma_array(
    out: *mut c_int,
    mul1: *const c_int,
    mul2: *const c_int,
    add: *const c_int,
    len: c_int,
) {
    let mut i: c_int = 0;
    while i < len {
        let idx = i as isize;
        let a = unsafe { *mul1.offset(idx) };
        let b = unsafe { *mul2.offset(idx) };
        let c = unsafe { *add.offset(idx) };
        // C signed overflow is UB; reproduce the two's-complement wrapping
        // behavior emitted by the C compiler on the target hardware.
        let v = a.wrapping_mul(b).wrapping_add(c);
        unsafe { *out.offset(idx) = v };
        i += 1;
    }
}

/// ```c
/// static void inner(int *out, int len) {
///     fma_array(out, out, out, out, len);
///     for (int i = 0; i < len; i++) {
///         printf("%d\n", out[i]);
///     }
/// }
/// ```
unsafe fn inner(out: *mut c_int, len: c_int) {
    unsafe { fma_array(out, out, out, out, len) };
    let mut i: c_int = 0;
    while i < len {
        let v = unsafe { *out.offset(i as isize) };
        unsafe { printf(FMT_D_NL.as_ptr() as *const c_char, v) };
        i += 1;
    }
}

/// ```c
/// void driver(const int *data, int len) {
///     int out[len];
///     memcpy(out, data, len * sizeof(int));
///     inner(out, len);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(data: *const c_int, len: c_int) {
    // `len * sizeof(int)` in C: `int` is converted to `size_t`, so a negative
    // length becomes an enormous unsigned byte count (the original UB is
    // reproduced rather than "fixed"). `black_box` keeps the optimizer from
    // reasoning about (and deleting) that out-of-range copy.
    let nbytes = core::hint::black_box((len as usize).wrapping_mul(core::mem::size_of::<c_int>()));

    if len >= 0 {
        // Stand-in for the C variable length array `int out[len]`.
        let mut out: Vec<c_int> = vec![0; len as usize];
        unsafe {
            memcpy(out.as_mut_ptr().cast::<c_void>(), data.cast::<c_void>(), nbytes);
            inner(out.as_mut_ptr(), len);
        }
    } else {
        // Negative length: the C code declares a bogus VLA and then performs a
        // `memcpy` of ~2^64 bytes. Reproduce that same out-of-bounds copy so the
        // observable behavior stays the same as the C library.
        let mut out: [c_int; 1] = [0; 1];
        // The destination is passed through `black_box` as well, otherwise the
        // optimizer proves the copy is out of bounds and deletes the whole
        // branch instead of performing the (faulting) copy the C code performs.
        let dst = core::hint::black_box(out.as_mut_ptr());
        unsafe {
            memcpy(dst.cast::<c_void>(), data.cast::<c_void>(), nbytes);
            inner(dst, len);
        }
    }
}
