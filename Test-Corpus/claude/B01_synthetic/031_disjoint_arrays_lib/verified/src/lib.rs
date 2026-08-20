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

//! Rust translation of `c_src/src/driver.c`.
//!
//! Exported ABI (matches `nm -D` of the C shared object):
//!   * `fma_array`
//!   * `call_fma`
//!   * `driver`
//!
//! The C original relies on `sscanf("%d%zn")` for parsing and `printf("%d\n")`
//! for output.  Both are called through the platform C library here so that the
//! parsing quirks (whitespace skipping, `strtol` saturation followed by
//! truncation to `int`, characters-consumed accounting) and the stdout
//! buffering behaviour are byte-for-byte identical to the C build.

use core::ffi::{c_char, c_int};

extern "C" {
    fn sscanf(s: *const c_char, format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
}

/// `void fma_array(int *restrict out, const int *mul1, const int *mul2, const int *add, int len)`
///
/// `out[i] = mul1[i] * mul2[i] + add[i]` for `i` in `[0, len)`.
/// Signed overflow (UB in C, wraps in practice with gcc) is reproduced with
/// wrapping arithmetic.  A non-positive `len` performs no iterations, exactly
/// like the C `for` loop.
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
        let a = *mul1.offset(idx);
        let b = *mul2.offset(idx);
        let c = *add.offset(idx);
        *out.offset(idx) = a.wrapping_mul(b).wrapping_add(c);
        i += 1;
    }
}

/// Reproduces the stack consumption of the C's three variable-length arrays.
///
/// `call_fma` declares `int out[len]; int ones[len]; int zeros[len];`, which gcc
/// lowers into a `3 * len * sizeof(int)` byte decrement of the stack pointer,
/// and the function then writes to every one of those bytes.  When that does not
/// fit in the thread's remaining stack the C build dies with `SIGSEGV`.
///
/// The Rust translation keeps the scratch arrays on the heap (so it stays
/// memory-safe), which would otherwise make it *succeed* — or abort on
/// allocation failure — where the C faults.  Touching the same stack depth here
/// makes the fault boundary identical to the C's.
///
/// Note that no probe happens at all until the C would need a full page
/// (`len >= 342`), so the sizes produced by `driver` (`len <= 100`) never reach
/// this code path.
#[inline(never)]
unsafe fn probe_vla_stack(bytes: usize) {
    const PAGE: usize = 4096;
    if bytes < PAGE {
        return;
    }
    let anchor: u8 = 0;
    let base = &anchor as *const u8 as usize;

    let mut off = PAGE;
    while off <= bytes {
        // Below the current frame: unused stack.  A write here grows the stack
        // exactly like the C's VLA does, and traps on the guard page exactly
        // where the C's VLA traps.
        match base.checked_sub(off) {
            Some(addr) => core::ptr::write_volatile(addr as *mut u8, 0),
            None => return,
        }
        off += PAGE;
    }
    if let Some(addr) = base.checked_sub(bytes) {
        core::ptr::write_volatile(addr as *mut u8, 0);
    }
}

/// Fallible allocation of `n` zeroed `int`s.  Returns `None` instead of aborting
/// the process, because the C has no allocation-failure path — it either fits on
/// the stack or faults, and `probe_vla_stack` already reproduced the fault.
fn try_zeroed(n: usize) -> Option<Vec<c_int>> {
    let mut v: Vec<c_int> = Vec::new();
    v.try_reserve_exact(n).ok()?;
    v.resize(n, 0);
    Some(v)
}

/// `int call_fma(const int *data, int len)`
///
/// Builds `ones` (all 1) and `zeros` (all 0) scratch arrays, runs
/// `fma_array(out, ones, data, zeros, len)` and returns `out[len - 1]`, i.e.
/// effectively `data[len - 1]`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn call_fma(data: *const c_int, len: c_int) -> c_int {
    if len == 0 {
        return 0;
    }

    // A negative `len` declares variable-length arrays of negative size and
    // then reads `out[len - 1]`: undefined behaviour in C (observed to return
    // uninitialised stack garbage that differs on every run, or to crash).
    // Nothing meaningful can be reproduced, so return without touching memory
    // out of bounds.
    if len < 0 {
        return 0;
    }

    let n = len as usize;

    // Same stack budget the C's `int out[len]; int ones[len]; int zeros[len];`
    // consumes, so an over-large `len` faults here just as it does in the C.
    probe_vla_stack(n * 3 * core::mem::size_of::<c_int>());

    match (try_zeroed(n), try_zeroed(n), try_zeroed(n)) {
        (Some(mut out), Some(mut ones), Some(mut zeros)) => {
            out[0] = 0;
            for i in 0..n {
                ones[i] = 1;
                zeros[i] = 0;
            }

            fma_array(out.as_mut_ptr(), ones.as_ptr(), data, zeros.as_ptr(), len);
            out[n - 1]
        }
        // The stack probe above already succeeded, so the C would have got its
        // VLAs and produced a result; only the Rust-specific heap allocation
        // failed.  Compute the identical observable behaviour without the
        // scratch buffers: `fma_array` reads `data[0..len]` (so a short `data`
        // still faults here) and stores `1 * data[i] + 0`, hence the returned
        // `out[len - 1]` is exactly `data[len - 1]`.
        _ => {
            let mut last: c_int = 0;
            let mut i: usize = 0;
            while i < n {
                last = *data.add(i);
                i += 1;
            }
            last
        }
    }
}

/// `void driver(const char *in)`
///
/// Scans up to 100 decimal integers out of `in` and prints the value of the
/// last successfully parsed one (0 when none could be parsed).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(input: *const c_char) {
    // The C code leaves `data` uninitialised; only the first `i` elements are
    // ever read, and those are always written by the scan loop below.
    let mut data: [c_int; 100] = [0; 100];
    let mut cursor = input;

    const SCAN_FMT: &[u8; 6] = b"%d%zn\0";

    let mut i: usize = 0;
    while i < 100 {
        let mut nb: usize = 0;
        if sscanf(
            cursor,
            SCAN_FMT.as_ptr() as *const c_char,
            &mut data[i] as *mut c_int,
            &mut nb as *mut usize,
        ) != 1
        {
            break;
        }
        cursor = cursor.add(nb);
        i += 1;
    }

    let result = call_fma(data.as_ptr(), i as c_int);

    const PRINT_FMT: &[u8; 4] = b"%d\n\0";
    printf(PRINT_FMT.as_ptr() as *const c_char, result);
}
