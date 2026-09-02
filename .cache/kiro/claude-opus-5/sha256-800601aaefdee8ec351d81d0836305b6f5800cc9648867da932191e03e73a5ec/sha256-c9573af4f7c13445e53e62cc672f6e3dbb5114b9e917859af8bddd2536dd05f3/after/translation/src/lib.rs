//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (from `nm -D` on the C shared object):
//!   - `rgb_to_hsv`
//!
//! Semantics are reproduced literally, including the exact ternary-based
//! min/max expansions from the C source (which differ from `f32::min` /
//! `f32::max` in their NaN handling) and the original order of checks.

use std::ffi::c_float;
use std::ptr::{read_volatile, write_volatile};

/// Convert an RGB triple to HSV.
///
/// Direct translation of `void rgb_to_hsv(float *dest, const float *src)`.
///
/// `src` must point to at least 3 readable `float`s and `dest` to at least 3
/// writable `float`s; the C original performs no validation and neither does
/// this translation.
///
/// # Why volatile accesses
///
/// The three loads and three stores use `read_volatile` / `write_volatile`
/// rather than plain dereferences, for two reasons, both about matching the C
/// byte for byte:
///
/// 1. **Fault behaviour parity.** A plain `*ptr` deref is instrumented by rustc
///    when `debug_assertions` are enabled, so a null `src`/`dest` aborts with a
///    Rust panic (`SIGABRT`) instead of faulting like the C does (`SIGSEGV`).
///    The C has no null check (verified: there is no `if (!src)` anywhere in
///    `c_src/src/lib.c`), so the translated library must fault the same way.
///    Volatile accesses are not instrumented, so both libraries produce the
///    identical fatal signal in every build profile, not just release.
/// 2. **Access ordering parity.** The C reads all three of `src` into locals
///    before storing anything to `dest`, which is what makes `dest == src` and
///    partially overlapping `dest`/`src` well behaved. Volatile accesses pin
///    that order explicitly instead of leaving it to the optimiser.
///
/// The computed values are unaffected — only the memory-access behaviour is
/// constrained.
///
/// # Safety
///
/// `src` must be valid for 3 `f32` reads and `dest` valid for 3 `f32` writes.
/// This is the same (unchecked) contract as the C function.
#[unsafe(no_mangle)]
// The C declares `v` and `delta` up front and assigns them later. Keeping that
// shape makes the translation line-for-line comparable with `c_src/src/lib.c`,
// so the late initialisation is deliberate.
#[allow(clippy::needless_late_init)]
pub unsafe extern "C" fn rgb_to_hsv(dest: *mut c_float, src: *const c_float) {
    let r: f32 = read_volatile(src.add(0));
    let g: f32 = read_volatile(src.add(1));
    let b: f32 = read_volatile(src.add(2));

    let mut h: f32 = 0.0;
    let mut s: f32 = 0.0;
    let v: f32;

    let mut min: f32 = r;
    let mut max: f32 = r;
    let delta: f32;

    // min = (((min) < (g)) ? (min) : (g));
    min = if min < g { min } else { g };
    // min = (((min) < (b)) ? (min) : (b));
    min = if min < b { min } else { b };
    // max = (((max) > (g)) ? (max) : (g));
    max = if max > g { max } else { g };
    // max = (((max) > (b)) ? (max) : (b));
    max = if max > b { max } else { b };

    delta = max - min;
    v = max;

    if delta == 0.0 || max == 0.0 {
        write_volatile(dest.add(0), h);
        write_volatile(dest.add(1), s);
        write_volatile(dest.add(2), v);
        return;
    }

    s = delta / max;

    if r == max {
        h = (g - b) / delta;
    } else if g == max {
        h = 2.0 + (b - r) / delta;
    } else {
        h = 4.0 + (r - g) / delta;
    }

    h *= 60.0;
    if h < 0.0 {
        h += 360.0;
    }

    write_volatile(dest.add(0), h);
    write_volatile(dest.add(1), s);
    write_volatile(dest.add(2), v);
}
