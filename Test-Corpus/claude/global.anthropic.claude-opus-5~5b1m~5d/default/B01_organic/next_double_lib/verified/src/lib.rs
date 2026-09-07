//! Rust translation of the C library in `c_src/`.
//!
//! The C library consists of a single translation unit (`src/lib.c`) exposing a
//! xorshift128+ style pseudo-random number generator through the public header
//! `include/lib.h`. The complete exported ABI is the single function
//! `next_double`.

#![allow(non_camel_case_types)]

/// Mirrors:
/// ```c
/// typedef struct cn_rnd_t {
///     uint64_t state[2];
/// } cn_rnd_t;
/// ```
#[repr(C)]
pub struct cn_rnd_t {
    pub state: [u64; 2],
}

/// Translation of the `static` helper:
///
/// ```c
/// static uint64_t cn_rnd_next(cn_rnd_t *rnd) {
///     uint64_t x = rnd->state[0];
///     uint64_t y = rnd->state[1];
///     rnd->state[0] = y;
///     x ^= x << 23;
///     x ^= x >> 17;
///     x ^= y ^ (y >> 26);
///     rnd->state[1] = x;
///     return x + y;
/// }
/// ```
///
/// Kept private, exactly like the C `static` function (it is not part of the
/// exported ABI). All arithmetic uses wrapping semantics to match C's unsigned
/// integer overflow behaviour.
///
/// The state is accessed through `read_unaligned` / `write_unaligned` on a raw
/// pointer rather than through a `&mut cn_rnd_t`. This is deliberate and is
/// required for C parity: the C compiler emits plain `mov`s that tolerate a
/// misaligned `cn_rnd_t *` on x86-64, whereas forming a Rust reference from a
/// misaligned pointer trips the `debug_assertions` alignment check and aborts
/// the process (`SIGABRT`) — a divergence from C, which returns a value. Using
/// unaligned accesses keeps the behaviour identical in every build profile and
/// also removes the reference-formation UB.
///
/// Reads and writes touch only `state[0]` and `state[1]`, i.e. never outside
/// the 16 bytes of the object, exactly like the C.
unsafe fn cn_rnd_next(rnd: *mut cn_rnd_t) -> u64 {
    // `addr_of_mut!` computes the field address without creating a reference.
    let state: *mut u64 = unsafe { core::ptr::addr_of_mut!((*rnd).state) }.cast::<u64>();

    let mut x: u64 = unsafe { core::ptr::read_unaligned(state) };
    let y: u64 = unsafe { core::ptr::read_unaligned(state.add(1)) };
    unsafe { core::ptr::write_unaligned(state, y) };
    x ^= x << 23;
    x ^= x >> 17;
    x ^= y ^ (y >> 26);
    unsafe { core::ptr::write_unaligned(state.add(1), x) };
    x.wrapping_add(y)
}

/// Translation of:
///
/// ```c
/// double next_double(cn_rnd_t *rnd) {
///     uint64_t value = cn_rnd_next(rnd);
///     uint64_t exponent = 1023;
///     uint64_t mantissa = value >> 12;
///     uint64_t result = (exponent << 52) | mantissa;
///     return *(double *)&result - 1.0;
/// }
/// ```
///
/// The type-punning `*(double *)&result` is reproduced bit-for-bit with
/// `f64::from_bits`.
/// The pointer is forwarded to `cn_rnd_next` as a raw pointer (no `&mut` is
/// formed), so a `NULL` argument faults with `SIGSEGV` on first access exactly
/// like the C, and a misaligned argument is handled rather than aborting.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn next_double(rnd: *mut cn_rnd_t) -> f64 {
    let value: u64 = unsafe { cn_rnd_next(rnd) };
    let exponent: u64 = 1023;
    let mantissa: u64 = value >> 12;
    let result: u64 = (exponent << 52) | mantissa;
    f64::from_bits(result) - 1.0
}
