// Rust translation of the C library in c_src/.
//
// Original copyright notice from the C sources:
//
// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use std::ffi::{c_char, c_int, c_void};

unsafe extern "C" {
    // Use the platform C library's printf so that output buffering and
    // formatting are byte-for-byte identical to the original C library
    // (and so output interleaves with any other C stdio in the process
    // exactly as it did before).
    fn printf(fmt: *const c_char, ...) -> c_int;

    // The C source calls `memcpy` from <string.h>; call the very same function
    // rather than `ptr::copy_nonoverlapping`. Besides being the literal
    // translation, this keeps the behaviour on out-of-contract inputs identical:
    // `copy_nonoverlapping` has a debug-only null/alignment precondition check
    // that aborts (SIGABRT), whereas `memcpy` with a bad pointer faults exactly
    // like the C does (SIGSEGV).
    fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
}

/// Format string `"%d\n"` used by `inner`.
static FMT_D_NL: [c_char; 4] = [b'%' as c_char, b'd' as c_char, b'\n' as c_char, 0];

/// C: `void fma_array(int *out, const int *mul1, const int *mul2, const int *add, int len)`
///
/// out[i] = mul1[i] * mul2[i] + add[i]
///
/// Signed overflow is UB in C; in practice the compiler emits wrapping
/// two's-complement arithmetic, which is what we reproduce here.
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
            let v = (*mul1.offset(idx))
                .wrapping_mul(*mul2.offset(idx))
                .wrapping_add(*add.offset(idx));
            *out.offset(idx) = v;
        }
        i += 1;
    }
}

/// C: `static void inner(int *out, int len)`
unsafe fn inner(out: *mut c_int, len: c_int) {
    unsafe {
        fma_array(out, out, out, out, len);
        let mut i: c_int = 0;
        while i < len {
            printf(FMT_D_NL.as_ptr(), *out.offset(i as isize));
            i += 1;
        }
    }
}

/// C: `void driver(const int *data, int len)`
///
/// Uses a VLA `int out[len]` plus `memcpy(out, data, len * sizeof(int))`,
/// then calls `inner`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(data: *const c_int, len: c_int) {
    // `int out[len]` — a VLA.
    //
    // A negative `len` is undefined behaviour in C: the VLA has a negative size
    // and, worse, `len * sizeof(int)` converts `len` to `size_t`, so `memcpy`
    // receives ~2^64 and the C process dies with SIGSEGV. That crash is not a
    // specified result and is not reproducible across compilers or stack
    // limits, so it is deliberately NOT replicated; `len` is clamped and the
    // function returns without output (see ERRORS.md row E11). C emits nothing
    // before trapping, so the two never produce *differing* output.
    let n = if len > 0 { len as usize } else { 0 };
    let mut out: Vec<c_int> = vec![0; n];
    unsafe {
        // `memcpy(out, data, len * sizeof(int))`, called unconditionally just as
        // the C does — including the `len == 0` case, where the C passes a size
        // of 0 and `data` is never dereferenced.
        memcpy(
            out.as_mut_ptr() as *mut c_void,
            data as *const c_void,
            n * std::mem::size_of::<c_int>(),
        );
        inner(out.as_mut_ptr(), len);
    }
}
