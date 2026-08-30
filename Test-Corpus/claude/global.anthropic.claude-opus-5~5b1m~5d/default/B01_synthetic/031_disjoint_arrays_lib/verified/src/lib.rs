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
//
// Rust translation of c_src/src/driver.c + c_src/include/driver.h
//
// Public ABI (as exported by the C shared library `libdriver.so`):
//   fma_array
//   call_fma
//   driver

use std::ffi::{c_char, c_int};

// The C code uses `sscanf` (with the `%d` and `%zn` conversions) and `printf`.
// We call straight through to the platform C library so that number parsing,
// formatting and stdout buffering behaviour is byte-for-byte identical to the
// original C implementation.
// glibc has two `sscanf` entry points that differ in their handling of a few
// conversions (`%a`, positional `%n$`): the legacy `sscanf` and the C99 one,
// `__isoc99_sscanf`.  A C compiler targeting glibc redirects `sscanf` to
// `__isoc99_sscanf` (confirmed by `nm -D -u c_src/build/libdriver.so`), so bind
// that exact symbol here in order to be byte-for-byte identical with the C
// build.  Other platforms only have plain `sscanf`.
#[cfg(all(target_os = "linux", target_env = "gnu"))]
unsafe extern "C" {
    #[link_name = "__isoc99_sscanf"]
    fn sscanf(s: *const c_char, format: *const c_char, ...) -> c_int;
}

#[cfg(not(all(target_os = "linux", target_env = "gnu")))]
unsafe extern "C" {
    fn sscanf(s: *const c_char, format: *const c_char, ...) -> c_int;
}

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

/// ```c
/// void fma_array(int *restrict out, const int *mul1, const int *mul2,
///                const int *add, int len);
/// ```
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
            // `wrapping_offset`, not `offset`: C's `mul1[i]` at -O0 is a plain
            // address computation with no in-bounds requirement, whereas
            // `<*const T>::offset` carries a safety precondition that a debug
            // build checks with `assert_unsafe_precondition!`.  With `offset`,
            // a caller passing a NULL or out-of-range pointer would make the
            // debug build *panic* where the C build faults, i.e. the two would
            // disagree on how they die (see ERRORS.md rows 19-20).
            // `wrapping_offset` lowers to the same `lea`/`add` and keeps the
            // observable behaviour identical to C in every cargo profile.
            //
            // Likewise `ptr::read`/`ptr::write` rather than `*p` / `*p = v`:
            // with `-C debug-assertions=on` rustc inserts a null-pointer check
            // around a raw-pointer *place* expression, which aborts (SIGABRT)
            // instead of faulting (SIGSEGV) the way the C build does.
            // `ptr::read`/`ptr::write` carry no such check, so `driver`'s and
            // `call_fma`'s NULL-pointer UB dies identically to C under both the
            // dev and the release profile.
            let m1 = core::ptr::read(mul1.wrapping_offset(idx));
            let m2 = core::ptr::read(mul2.wrapping_offset(idx));
            let a = core::ptr::read(add.wrapping_offset(idx));
            // Signed overflow is UB in C; gcc/clang emit wrapping arithmetic.
            core::ptr::write(
                out.wrapping_offset(idx),
                m1.wrapping_mul(m2).wrapping_add(a),
            );
        }
        i += 1;
    }
}

/// ```c
/// int call_fma(const int *data, int len);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn call_fma(data: *const c_int, len: c_int) -> c_int {
    if len == 0 {
        return 0;
    }
    // A negative `len` would declare variable length arrays with a negative
    // size, which is undefined behaviour in C.  There is no meaningful
    // behaviour to reproduce, so bail out without touching memory.
    if len < 0 {
        return 0;
    }

    let n = len as usize;
    // `int out[len]; int ones[len]; int zeros[len];`
    let mut out: Vec<c_int> = vec![0; n];
    let mut ones: Vec<c_int> = vec![0; n];
    let mut zeros: Vec<c_int> = vec![0; n];

    out[0] = 0;
    for i in 0..n {
        ones[i] = 1;
        zeros[i] = 0;
    }

    unsafe {
        fma_array(
            out.as_mut_ptr(),
            ones.as_ptr(),
            data,
            zeros.as_ptr(),
            len,
        );
    }
    out[n - 1]
}

/// ```c
/// void driver(const char *in);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(input: *const c_char) {
    let mut data = [0 as c_int; 100];
    let mut cursor = input;
    let mut i: usize = 0;

    while i < 100 {
        let mut nb: usize = 0;
        let matched = unsafe {
            sscanf(
                cursor,
                c"%d%zn".as_ptr(),
                &mut data[i] as *mut c_int,
                &mut nb as *mut usize,
            )
        };
        if matched != 1 {
            break;
        }
        // `in += nb` in C is plain pointer arithmetic; `wrapping_add` matches it
        // without `add`'s debug-checked in-bounds precondition.
        cursor = cursor.wrapping_add(nb);
        i += 1;
    }

    let result = unsafe { call_fma(data.as_ptr(), i as c_int) };
    unsafe {
        printf(c"%d\n".as_ptr(), result);
    }
}
