//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI surface (from `nm -D` on the C shared object):
//!   - `next_double`
//!
//! `cn_rnd_next` is `static` in the C source, so it is not exported; it is
//! reproduced here as a private helper with identical semantics.

use std::ffi::c_double;

/// Mirror of the C `cn_rnd_t`:
///
/// ```c
/// typedef struct cn_rnd_t {
///     uint64_t state[2];
/// } cn_rnd_t;
/// ```
#[repr(C)]
pub struct cn_rnd_t {
    pub state: [u64; 2],
}

/// A plain 64-bit load, exactly like the C `uint64_t x = rnd->state[0];`.
///
/// Rust's own pointer primitives (`*p`, `ptr::read`, `read_unaligned`,
/// `read_volatile`, `copy_nonoverlapping`, reference creation, ...) all carry
/// compiler/`core`-inserted preconditions that are active whenever
/// `debug_assertions` / `-C ub-checks` are on:
///
///   * a NULL pointer raises `null pointer dereference occurred`, and
///   * a misaligned pointer raises `misaligned pointer dereference`,
///
/// both of which are *non-unwinding* panics that kill the process with
/// `SIGABRT`. The C compiler emits neither check: a misaligned `cn_rnd_t *`
/// simply performs an unaligned `mov`, and a NULL one takes a hardware fault
/// (`SIGSEGV`). Emitting the `mov` directly is what makes the Rust observably
/// identical to the C for those inputs, in debug *and* release builds.
#[cfg(target_arch = "x86_64")]
#[inline(always)]
unsafe fn load_u64(p: *const u64) -> u64 {
    let v: u64;
    unsafe {
        core::arch::asm!(
            "mov {v}, qword ptr [{p}]",
            p = in(reg) p,
            v = out(reg) v,
            // Not `nomem`/`pure`: the load must stay ordered with respect to the
            // stores below. `readonly` says it writes no memory.
            options(nostack, preserves_flags, readonly),
        );
    }
    v
}

/// A plain 64-bit store, exactly like the C `rnd->state[0] = y;`.
/// See [`load_u64`] for why inline `mov` is used instead of `ptr::write*`.
#[cfg(target_arch = "x86_64")]
#[inline(always)]
unsafe fn store_u64(p: *mut u64, v: u64) {
    unsafe {
        core::arch::asm!(
            "mov qword ptr [{p}], {v}",
            p = in(reg) p,
            v = in(reg) v,
            options(nostack, preserves_flags),
        );
    }
}

/// Portable fallback for non-x86-64 targets.
///
/// `read_unaligned`/`write_unaligned` reproduce the C's lack of an alignment
/// requirement. On such targets a NULL pointer is reported by Rust's own UB
/// check (abort) instead of a hardware fault when `ub_checks` are enabled;
/// x86-64 (the tested target) uses the `asm!` path above and has no such
/// difference.
#[cfg(not(target_arch = "x86_64"))]
#[inline(always)]
unsafe fn load_u64(p: *const u64) -> u64 {
    unsafe { p.read_unaligned() }
}

#[cfg(not(target_arch = "x86_64"))]
#[inline(always)]
unsafe fn store_u64(p: *mut u64, v: u64) {
    unsafe { p.write_unaligned(v) }
}

/// Translation of the `static` C helper:
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
/// The shifts and the final addition use wrapping arithmetic to match C's
/// unsigned 64-bit semantics exactly.
///
/// The state is accessed through the raw pointer with unaligned loads/stores
/// rather than through a `&mut cn_rnd_t`. The C code performs plain `uint64_t`
/// loads and stores with no alignment check, so a caller that passes a
/// misaligned `cn_rnd_t *` gets a normal (unaligned) access. Forming a Rust
/// reference would instead trip the `misaligned pointer dereference` check and
/// abort the process, which the C never does — so the reference is avoided.
#[inline]
unsafe fn cn_rnd_next(rnd: *mut cn_rnd_t) -> u64 {
    // `state[0]` is at byte offset 0 and `state[1]` at byte offset 8, matching
    // the C layout. The offset is computed in bytes so that no intermediate
    // typed-pointer arithmetic assumption is introduced.
    let base = rnd as *mut u8;
    let s0 = base as *mut u64;
    // `wrapping_add` carries no UB precondition check of its own, so a NULL
    // `rnd` reaches the `mov` and faults there, exactly as in C.
    let s1 = base.wrapping_add(8) as *mut u64;

    let mut x = unsafe { load_u64(s0) };
    let y = unsafe { load_u64(s1) };
    unsafe { store_u64(s0, y) };
    x ^= x << 23;
    x ^= x >> 17;
    x ^= y ^ (y >> 26);
    unsafe { store_u64(s1, x) };
    x.wrapping_add(y)
}

/// `double next_double(cn_rnd_t *rnd);`
///
/// Reproduces the C body verbatim, including the type-punning read of the
/// assembled bit pattern as an IEEE-754 double:
///
/// ```c
/// uint64_t value = cn_rnd_next(rnd);
/// uint64_t exponent = 1023;
/// uint64_t mantissa = value >> 12;
/// uint64_t result = (exponent << 52) | mantissa;
/// return *(double *)&result - 1.0;
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn next_double(rnd: *mut cn_rnd_t) -> c_double {
    // The C code dereferences `rnd` unconditionally with no NULL check and with
    // no alignment requirement enforced; we preserve that behaviour rather than
    // "fixing" it. A NULL pointer therefore faults exactly as the C does.
    let value: u64 = unsafe { cn_rnd_next(rnd) };
    let exponent: u64 = 1023;
    let mantissa: u64 = value >> 12;
    let result: u64 = (exponent << 52) | mantissa;
    f64::from_bits(result) - 1.0
}
