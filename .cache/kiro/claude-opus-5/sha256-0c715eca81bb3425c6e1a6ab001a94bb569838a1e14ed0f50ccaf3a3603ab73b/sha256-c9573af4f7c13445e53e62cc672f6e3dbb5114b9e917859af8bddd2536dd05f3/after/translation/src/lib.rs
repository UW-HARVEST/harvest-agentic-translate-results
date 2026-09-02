//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (from `nm -D` on the C shared object):
//!   * `next_double`
//!
//! Header (`include/lib.h`):
//! ```c
//! typedef struct cn_rnd_t { uint64_t state[2]; } cn_rnd_t;
//! double next_double(cn_rnd_t *rnd);
//! ```

use std::ffi::c_double;
use std::ptr;

/// Mirrors `cn_rnd_t` from `include/lib.h`: two `uint64_t` words.
/// `#[repr(C)]` keeps layout/alignment identical to the C struct
/// (16 bytes, 8-byte aligned, `state[0]` at offset 0, `state[1]` at offset 8).
#[repr(C)]
pub struct cn_rnd_t {
    pub state: [u64; 2],
}

/// Translation of the file-local `static uint64_t cn_rnd_next(cn_rnd_t *rnd)`.
///
/// xorshift128+ style step. All arithmetic in C is on `uint64_t`, i.e. modulo
/// 2^64, so the final addition uses `wrapping_add` to reproduce it exactly
/// (and to avoid a debug-build overflow panic).
///
/// The state is accessed through raw `ptr::read`/`ptr::write` rather than a
/// `&mut cn_rnd_t`. That is deliberate: the C code dereferences `rnd`
/// unconditionally, and forming a Rust reference from the pointer first would
/// trip a debug-build UB check and `abort()` (SIGABRT) where the C faults
/// (SIGSEGV). Raw pointer reads fault exactly like the C does, in every
/// profile. See `ERRORS.md` row B1.
///
/// # Safety
/// `rnd` must be a valid, aligned pointer to a `cn_rnd_t`, mirroring the
/// (unchecked) contract of the C function.
#[inline]
unsafe fn cn_rnd_next(rnd: *mut cn_rnd_t) -> u64 {
    // `state[0]` is at offset 0 and `state[1]` at offset 8 of the `#[repr(C)]`
    // struct, so a `*mut u64` walk is layout-identical to `rnd->state[i]`.
    let state = rnd as *mut u64;

    let mut x: u64 = unsafe { ptr::read(state) }; // uint64_t x = rnd->state[0];
    let y: u64 = unsafe { ptr::read(state.add(1)) }; // uint64_t y = rnd->state[1];
    unsafe { ptr::write(state, y) }; // rnd->state[0] = y;
    x ^= x << 23; // x ^= x << 23;
    x ^= x >> 17; // x ^= x >> 17;
    x ^= y ^ (y >> 26); // x ^= y ^ (y >> 26);
    unsafe { ptr::write(state.add(1), x) }; // rnd->state[1] = x;
    x.wrapping_add(y) // return x + y;
}

/// `double next_double(cn_rnd_t *rnd)`
///
/// Builds an IEEE-754 double in [1.0, 2.0) from the top 52 bits of the
/// generated value and subtracts 1.0. The C code type-puns through
/// `*(double *)&result`; `f64::from_bits` is the exact equivalent bit
/// reinterpretation.
///
/// # Safety
/// Same contract as the C function: `rnd` is dereferenced without any NULL or
/// validity check.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn next_double(rnd: *mut cn_rnd_t) -> c_double {
    let value: u64 = unsafe { cn_rnd_next(rnd) };
    let exponent: u64 = 1023;
    let mantissa: u64 = value >> 12;
    let result: u64 = (exponent << 52) | mantissa;
    f64::from_bits(result) - 1.0
}
