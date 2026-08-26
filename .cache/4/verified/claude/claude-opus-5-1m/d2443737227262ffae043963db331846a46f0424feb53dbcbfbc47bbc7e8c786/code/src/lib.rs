//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI surface (matches `nm -D` on the C shared library exactly):
//!   * `div_euclid`
//!
//! The translation is intentionally literal: the branch structure, the order of
//! the comparisons and every intermediate expression mirror `c_src/src/lib.c`
//! one-for-one. All arithmetic uses the `wrapping_*` family so that the
//! two's-complement behaviour produced by the C compiler (including the signed
//! overflow the original code performs for `v1 == INT_MIN, v2 == -1`) is
//! reproduced bit-for-bit instead of panicking or being "fixed".

#![allow(clippy::needless_return)]

use std::ffi::c_int;

/// `INT_MIN`, written the same way the C source writes it (`-0x7fffffff - 1`).
const C_INT_MIN: c_int = -0x7fffffff - 1;

/// Truncating division that never panics (mirrors the hardware `idiv` the C
/// compiler emits). The divisor is always non-zero at every call site, exactly
/// as in the C original.
#[inline]
fn cdiv(a: c_int, b: c_int) -> c_int {
    a.wrapping_div(b)
}

/// Truncating remainder that never panics (mirrors the C `%` operator).
#[inline]
fn crem(a: c_int, b: c_int) -> c_int {
    a.wrapping_rem(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn div_euclid(v1: c_int, v2: c_int) -> c_int {
    // if (v2 == 0) { return 0; }
    if v2 == 0 {
        return 0;
    }

    // int q, r;
    let q: c_int;
    let r: c_int;

    if v1 >= 0 {
        if v2 >= 0 {
            // return ((v1) / (v2));
            return cdiv(v1, v2);
        } else if v2 != C_INT_MIN {
            // q = -((v1) / (-v2)), r = ((v1) % (-v2));
            q = cdiv(v1, v2.wrapping_neg()).wrapping_neg();
            r = crem(v1, v2.wrapping_neg());
        } else {
            // q = 0, r = v1;
            q = 0;
            r = v1;
        }
    } else if v1 != C_INT_MIN {
        if v2 >= 0 {
            // q = -((-v1) / (v2)), r = -((-v1) % (v2));
            q = cdiv(v1.wrapping_neg(), v2).wrapping_neg();
            r = crem(v1.wrapping_neg(), v2).wrapping_neg();
        } else if v2 != C_INT_MIN {
            // q = ((-v1) / (-v2)), r = -((-v1) % (-v2));
            q = cdiv(v1.wrapping_neg(), v2.wrapping_neg());
            r = crem(v1.wrapping_neg(), v2.wrapping_neg()).wrapping_neg();
        } else {
            // q = 1, r = v1 - q * v2;
            // The comma operator sequences the assignment to `q` before `r` is
            // computed, so `r` is evaluated with q == 1.
            q = 1;
            r = v1.wrapping_sub(q.wrapping_mul(v2));
        }
    } else if v2 >= 0 {
        // q = -((-(v1 + v2)) / (v2)) - 1, r = -((-(v1 + v2)) % (v2));
        let t = v1.wrapping_add(v2).wrapping_neg();
        q = cdiv(t, v2).wrapping_neg().wrapping_sub(1);
        r = crem(t, v2).wrapping_neg();
    } else if v2 != C_INT_MIN {
        // q = ((-(v1 - v2)) / (-v2)) + 1, r = -((-(v1 - v2)) % (-v2));
        let t = v1.wrapping_sub(v2).wrapping_neg();
        q = cdiv(t, v2.wrapping_neg()).wrapping_add(1);
        r = crem(t, v2.wrapping_neg()).wrapping_neg();
    } else {
        // q = 1, r = 0;
        q = 1;
        r = 0;
    }

    // if (r >= 0) return q; else return q + (v2 > 0 ? -1 : 1);
    if r >= 0 {
        return q;
    } else {
        return q.wrapping_add(if v2 > 0 { -1 } else { 1 });
    }
}
