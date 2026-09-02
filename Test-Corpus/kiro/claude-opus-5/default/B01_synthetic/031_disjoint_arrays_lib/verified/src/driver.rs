//! Direct translation of `c_src/src/driver.c`.
//!
//! Every observable behaviour of the C original is preserved, including the
//! quirks: `fma_array` is a plain multiply-add loop with no bounds or null
//! checks, `call_fma` only special-cases `len == 0`, and `driver` stops parsing
//! after 100 integers (or at the first `sscanf` failure) and prints the last
//! value it managed to read.

use std::ffi::{c_char, c_int};

use crate::cstdio::{printf, sscanf};

/// `"%d%zn"` — the scanf format used by `driver`, NUL terminated.
const SCAN_FMT: &[u8] = b"%d%zn\0";

/// `"%d\n"` — the printf format used by `driver`, NUL terminated.
const PRINT_FMT: &[u8] = b"%d\n\0";

/// ```c
/// void fma_array(int *restrict out, const int *mul1, const int *mul2,
///                const int *add, int len);
/// ```
///
/// # Safety
///
/// Exactly as unsafe as the C original: `out` must be writable for `len`
/// `int`s and `mul1`, `mul2` and `add` must each be readable for `len` `int`s.
/// `out` must not alias the inputs (the C parameter is `restrict`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fma_array(
    out: *mut c_int,
    mul1: *const c_int,
    mul2: *const c_int,
    add: *const c_int,
    len: c_int,
) {
    // for (int i = 0; i < len; i++) out[i] = mul1[i] * mul2[i] + add[i];
    //
    // A negative `len` makes the C loop body execute zero times, which the
    // empty Rust range reproduces. Signed overflow is UB in C; gcc/clang wrap
    // on the two's-complement targets this library builds for, so `wrapping_*`
    // matches the generated code.
    let mut i: c_int = 0;
    while i < len {
        let idx = i as isize;
        let value = unsafe {
            (*mul1.offset(idx))
                .wrapping_mul(*mul2.offset(idx))
                .wrapping_add(*add.offset(idx))
        };
        unsafe { *out.offset(idx) = value };
        i += 1;
    }
}

/// ```c
/// int call_fma(const int *data, int len);
/// ```
///
/// Builds an all-ones and an all-zeros vector of length `len`, runs
/// `fma_array` over them, and returns the last element. Because `mul1` is all
/// ones and `add` is all zeros, the result is simply `data[len - 1]`, but the
/// computation is performed the same way the C code performs it so that any
/// overflow behaviour matches.
///
/// # Safety
///
/// `data` must be readable for `len` `int`s when `len > 0`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn call_fma(data: *const c_int, len: c_int) -> c_int {
    // if (len == 0) return 0;
    if len == 0 {
        return 0;
    }

    // The C code then declares `int out[len]` / `ones[len]` / `zeros[len]`.
    // For a negative `len` those are negative-size VLAs: undefined behaviour,
    // and in practice the C code goes on to read `out[len - 1]`, i.e. off the
    // front of the array, yielding whatever happens to be on the stack. There
    // is no observable behaviour to reproduce, so return a deterministic 0
    // rather than reading out of bounds.
    if len < 0 {
        return 0;
    }

    let n = len as usize;

    // int out[len]; int ones[len]; int zeros[len];
    // out[0] = 0;
    // for (int i = 0; i < len; i++) { ones[i] = 1; zeros[i] = 0; }
    //
    // `out` is uninitialised in C apart from `out[0] = 0`, but `fma_array`
    // writes every element before any is read, so zero-filling is equivalent.
    let mut out: Vec<c_int> = vec![0; n];
    let ones: Vec<c_int> = vec![1; n];
    let zeros: Vec<c_int> = vec![0; n];

    // fma_array(out, ones, data, zeros, len);
    unsafe { fma_array(out.as_mut_ptr(), ones.as_ptr(), data, zeros.as_ptr(), len) };

    // return out[len-1];
    out[n - 1]
}

/// ```c
/// void driver(const char *in);
/// ```
///
/// Reads up to 100 decimal integers out of `in` with `sscanf("%d%zn", ...)`,
/// advancing past each one, then prints `call_fma(data, i)` followed by a
/// newline. With no parsable integer at all, `i` is 0 and `call_fma` returns 0,
/// so `"0\n"` is printed.
///
/// # Safety
///
/// `in_` must be a valid NUL-terminated C string (the C original dereferences
/// it unconditionally, so a null pointer faults there too).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(in_: *const c_char) {
    // int data[100];
    //
    // Left uninitialised in C. Only the first `i` elements are ever read, and
    // those are exactly the ones `sscanf` filled in.
    let mut data: [c_int; 100] = [0; 100];

    // int i;
    // for (i = 0; i < 100; i++) { ... }
    let mut i: c_int = 0;
    let mut cursor = in_;
    while i < 100 {
        // size_t nb;
        // if (sscanf(in, "%d%zn", &data[i], &nb) != 1) break;
        let mut nb: usize = 0;
        let rc = unsafe {
            sscanf(
                cursor,
                SCAN_FMT.as_ptr() as *const c_char,
                data.as_mut_ptr().offset(i as isize),
                &mut nb as *mut usize,
            )
        };
        if rc != 1 {
            break;
        }

        // in += nb;
        cursor = unsafe { cursor.add(nb) };
        i += 1;
    }

    // int result = call_fma(data, i);
    let result = unsafe { call_fma(data.as_ptr(), i) };

    // printf("%d\n", result);
    unsafe { printf(PRINT_FMT.as_ptr() as *const c_char, result) };
}
