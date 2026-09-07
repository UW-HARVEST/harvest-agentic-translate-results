//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (as exported by the C shared object, per `nm -D`):
//!   * `div_euclid`
//!
//! The translation reproduces the original control flow exactly, including the
//! `if`/`else` chain ordering and the (buggy) special cases for `INT_MIN`.
//! All arithmetic uses wrapping semantics so that the behaviour matches what
//! the C compiler emits for the two's-complement target, and so that Rust
//! never panics where C would silently wrap.

#![allow(clippy::needless_return)]

use std::os::raw::c_int;

/// C `INT_MIN` as spelled in the original source: `(-0x7fffffff - 1)`.
const C_INT_MIN: c_int = -0x7fffffff - 1;

/// Faithful translation of `int div_euclid(int v1, int v2)` from `c_src/src/lib.c`.
///
/// The C source reads (dangling `else`s bind to the nearest `if`, which matches
/// the original indentation):
///
/// ```c
/// int div_euclid(int v1, int v2) {
///     if (v2 == 0) return 0;
///     int q, r;
///     if (v1 >= 0)
///         if (v2 >= 0)                       return v1 / v2;
///         else if (v2 != INT_MIN)            q = -(v1 / -v2),  r = v1 % -v2;
///         else                               q = 0, r = v1;
///     else if (v1 != INT_MIN)
///         if (v2 >= 0)                       q = -((-v1)/v2),    r = -((-v1)%v2);
///         else if (v2 != INT_MIN)            q = (-v1)/(-v2),    r = -((-v1)%(-v2));
///         else                               q = 1, r = v1 - q*v2;
///     else if (v2 >= 0)                      q = -((-(v1+v2))/v2) - 1, r = -((-(v1+v2))%v2);
///     else if (v2 != INT_MIN)                q = ((-(v1-v2))/(-v2)) + 1, r = -((-(v1-v2))%(-v2));
///     else                                   q = 1, r = 0;
///     if (r >= 0) return q; else return q + (v2 > 0 ? -1 : 1);
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn div_euclid(v1: c_int, v2: c_int) -> c_int {
    if v2 == 0 {
        return 0;
    }

    let q: c_int;
    let r: c_int;

    if v1 >= 0 {
        if v2 >= 0 {
            // Truncating division, same as C's `/` for i32.
            return v1.wrapping_div(v2);
        } else if v2 != C_INT_MIN {
            let nv2 = v2.wrapping_neg();
            q = v1.wrapping_div(nv2).wrapping_neg();
            r = v1.wrapping_rem(nv2);
        } else {
            q = 0;
            r = v1;
        }
    } else if v1 != C_INT_MIN {
        let nv1 = v1.wrapping_neg();
        if v2 >= 0 {
            q = nv1.wrapping_div(v2).wrapping_neg();
            r = nv1.wrapping_rem(v2).wrapping_neg();
        } else if v2 != C_INT_MIN {
            let nv2 = v2.wrapping_neg();
            q = nv1.wrapping_div(nv2);
            r = nv1.wrapping_rem(nv2).wrapping_neg();
        } else {
            // `r = v1 - q * v2` with `q` already assigned to 1 by the comma
            // operator's sequencing in the original C.
            q = 1;
            r = v1.wrapping_sub(q.wrapping_mul(v2));
        }
    } else if v2 >= 0 {
        let t = v1.wrapping_add(v2).wrapping_neg(); // -(v1 + v2)
        q = t.wrapping_div(v2).wrapping_neg().wrapping_sub(1);
        r = t.wrapping_rem(v2).wrapping_neg();
    } else if v2 != C_INT_MIN {
        let t = v1.wrapping_sub(v2).wrapping_neg(); // -(v1 - v2)
        let nv2 = v2.wrapping_neg();
        q = t.wrapping_div(nv2).wrapping_add(1);
        r = t.wrapping_rem(nv2).wrapping_neg();
    } else {
        q = 1;
        r = 0;
    }

    if r >= 0 {
        return q;
    } else {
        return q.wrapping_add(if v2 > 0 { -1 } else { 1 });
    }
}
