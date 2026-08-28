//! Rust translation of `c_src/src/lib.c`.
//!
//! The behaviour of the original C is reproduced exactly, including its
//! branch structure and the order of its checks. Arithmetic is expressed with
//! the `wrapping_*` helpers so that the Rust build never panics where the C
//! would silently wrap; the reachable inputs of this function never actually
//! overflow, so the results are identical either way.

use std::ffi::c_int;

/// `INT_MIN`, spelled the way the C source spells it (`-0x7fffffff - 1`).
const INT_MIN: c_int = -0x7fff_ffff - 1;

/// Euclidean division of `v1` by `v2`.
///
/// Mirrors the C `div_euclid` from `c_src/src/lib.c`, dangling-`else`
/// associations included. Division by zero yields `0`.
#[unsafe(no_mangle)]
pub extern "C" fn div_euclid(v1: c_int, v2: c_int) -> c_int {
    if v2 == 0 {
        return 0;
    }

    // `q` and `r` are only ever read after every branch below has assigned
    // both of them; the sole branch that leaves them unset returns early.
    let q: c_int;
    let r: c_int;

    if v1 >= 0 {
        if v2 >= 0 {
            return v1 / v2;
        } else if v2 != INT_MIN {
            q = (v1 / v2.wrapping_neg()).wrapping_neg();
            r = v1 % v2.wrapping_neg();
        } else {
            q = 0;
            r = v1;
        }
    } else if v1 != INT_MIN {
        if v2 >= 0 {
            q = (v1.wrapping_neg() / v2).wrapping_neg();
            r = (v1.wrapping_neg() % v2).wrapping_neg();
        } else if v2 != INT_MIN {
            q = v1.wrapping_neg() / v2.wrapping_neg();
            r = (v1.wrapping_neg() % v2.wrapping_neg()).wrapping_neg();
        } else {
            q = 1;
            r = v1.wrapping_sub(q.wrapping_mul(v2));
        }
    } else if v2 >= 0 {
        let n = v1.wrapping_add(v2).wrapping_neg();
        q = (n / v2).wrapping_neg().wrapping_sub(1);
        r = (n % v2).wrapping_neg();
    } else if v2 != INT_MIN {
        let n = v1.wrapping_sub(v2).wrapping_neg();
        q = (n / v2.wrapping_neg()).wrapping_add(1);
        r = (n % v2.wrapping_neg()).wrapping_neg();
    } else {
        q = 1;
        r = 0;
    }

    if r >= 0 {
        q
    } else {
        q + if v2 > 0 { -1 } else { 1 }
    }
}
