// Rust translation of c_src/src/driver.c (MIT Lincoln Laboratory, 2025).
//
// Public ABI reproduced (as exported by the C shared library):
//   void driver(const int *data, int len);
//   void fma_array(int *out, const int *mul1, const int *mul2, const int *add, int len);
//
// `inner` is `static` in the C source and therefore has no external linkage;
// it is translated as a private Rust function.

use core::ffi::{c_char, c_int, c_void};

// Use the platform C library directly so that stdout buffering, formatting and
// byte-level output are identical to the original C code.
unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
}

/// `%d\n` format string used by the C `printf` call in `inner`.
static FMT_D_NL: [c_char; 4] = [b'%' as c_char, b'd' as c_char, b'\n' as c_char, 0];

/// void fma_array(int *out, const int *mul1, const int *mul2, const int *add, int len)
///
/// out[i] = mul1[i] * mul2[i] + add[i] for i in [0, len).
/// Signed overflow is UB in C; the generated code wraps, so `wrapping_*` is used
/// here to reproduce the same values without panicking.
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
        unsafe {
            let m1 = *mul1.offset(idx);
            let m2 = *mul2.offset(idx);
            let a = *add.offset(idx);
            *out.offset(idx) = m1.wrapping_mul(m2).wrapping_add(a);
        }
        i += 1;
    }
}

/// static void inner(int *out, int len)
fn inner(out: *mut c_int, len: c_int) {
    unsafe {
        fma_array(out, out, out, out, len);
    }
    let mut i: c_int = 0;
    while i < len {
        unsafe {
            printf(FMT_D_NL.as_ptr(), *out.offset(i as isize));
        }
        i += 1;
    }
}

/// void driver(const int *data, int len)
///
/// The C version declares a variable-length array `int out[len]` and copies
/// `len * sizeof(int)` bytes into it. The byte count is computed exactly as C
/// does (the `int` `len` is converted to `size_t`, i.e. sign-extended, before
/// being multiplied), so non-positive lengths behave as they do in the original.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(data: *const c_int, len: c_int) {
    let elems = if len > 0 { len as usize } else { 0 };
    let mut out: Vec<c_int> = vec![0; elems];

    let n_bytes = (len as isize as usize).wrapping_mul(core::mem::size_of::<c_int>());
    unsafe {
        memcpy(
            out.as_mut_ptr() as *mut c_void,
            data as *const c_void,
            n_bytes,
        );
    }

    inner(out.as_mut_ptr(), len);
}
