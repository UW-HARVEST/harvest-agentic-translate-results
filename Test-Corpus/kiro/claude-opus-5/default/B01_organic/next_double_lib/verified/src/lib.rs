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

/// Mirrors `cn_rnd_t` from `include/lib.h`: two `uint64_t` words.
/// `#[repr(C)]` keeps layout/alignment identical to the C struct.
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
/// The state words are accessed with `read_unaligned`/`write_unaligned` through
/// a raw pointer rather than through a `&mut cn_rnd_t`. The C compiler emits
/// plain 8-byte loads/stores that succeed on x86-64 regardless of the pointer's
/// alignment, and forming a Rust reference would instead trip the debug-build
/// misaligned-pointer check and abort. On an aligned pointer the generated code
/// and results are identical, so this only widens the set of inputs Rust
/// handles the same way C does.
#[inline]
unsafe fn cn_rnd_next(rnd: *mut cn_rnd_t) -> u64 {
    // `cn_rnd_t` is `#[repr(C)]` with a single `[u64; 2]` field, so the struct
    // pointer and a pointer to `state[0]` have the same address.
    let state = rnd as *mut u64;

    let mut x: u64 = unsafe { core::ptr::read_unaligned(state) };
    let y: u64 = unsafe { core::ptr::read_unaligned(state.add(1)) };
    unsafe { core::ptr::write_unaligned(state, y) };
    x ^= x << 23;
    x ^= x >> 17;
    x ^= y ^ (y >> 26);
    unsafe { core::ptr::write_unaligned(state.add(1), x) };
    x.wrapping_add(y)
}

/// `double next_double(cn_rnd_t *rnd)`
///
/// Builds an IEEE-754 double in [1.0, 2.0) from the top 52 bits of the
/// generated value and subtracts 1.0. The C code type-puns through
/// `*(double *)&result`; `f64::from_bits` is the exact equivalent bit
/// reinterpretation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn next_double(rnd: *mut cn_rnd_t) -> c_double {
    // The C function dereferences `rnd` unconditionally (no NULL check);
    // reproduce that behaviour rather than "fixing" it. A NULL pointer must
    // fault here exactly as it does in C.
    let value: u64 = unsafe { cn_rnd_next(rnd) };
    let exponent: u64 = 1023;
    let mantissa: u64 = value >> 12;
    let result: u64 = (exponent << 52) | mantissa;
    f64::from_bits(result) - 1.0
}
