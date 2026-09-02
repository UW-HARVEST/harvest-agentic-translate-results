//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (from `nm -D` on the C shared object and `include/lib.h`):
//!   * `div_euclid`
//!
//! The header declares no namespace/renaming macros, so the source-level name
//! is also the final linker symbol name.

use std::ffi::c_int;

/// `INT_MIN`, written the way the C source spells it: `(-0x7fffffff - 1)`.
const C_INT_MIN: c_int = -0x7fffffff - 1;

/// Translation of `int div_euclid(int v1, int v2)` from `c_src/src/lib.c`.
///
/// The C body is a chain of nested `if`/`else` statements relying on the usual
/// dangling-`else` binding; the structure below reproduces that binding
/// exactly. Every arithmetic operation is expressed with `wrapping_*` so the
/// Rust build can never panic where the C would simply compute a value — the
/// operand ranges enforced by the C branches mean no wrap actually occurs, so
/// the results are identical to the C.
///
/// Behaviour is preserved verbatim, including the `v2 == 0` early return of `0`
/// and the exact order of the `v2 >= 0` / `v2 != INT_MIN` checks.
#[unsafe(no_mangle)]
pub extern "C" fn div_euclid(v1: c_int, v2: c_int) -> c_int {
    if v2 == 0 {
        return 0;
    }

    // `int q, r;` — every path below assigns both before they are read.
    let q: c_int;
    let r: c_int;

    if v1 >= 0 {
        if v2 >= 0 {
            // return ((v1) / (v2));
            return v1.wrapping_div(v2);
        } else if v2 != C_INT_MIN {
            // q = -((v1) / (-v2)), r = ((v1) % (-v2));
            let nv2 = v2.wrapping_neg();
            q = v1.wrapping_div(nv2).wrapping_neg();
            r = v1.wrapping_rem(nv2);
        } else {
            // q = 0, r = v1;
            q = 0;
            r = v1;
        }
    } else if v1 != C_INT_MIN {
        let nv1 = v1.wrapping_neg();
        if v2 >= 0 {
            // q = -((-v1) / (v2)), r = -((-v1) % (v2));
            q = nv1.wrapping_div(v2).wrapping_neg();
            r = nv1.wrapping_rem(v2).wrapping_neg();
        } else if v2 != C_INT_MIN {
            // q = ((-v1) / (-v2)), r = -((-v1) % (-v2));
            let nv2 = v2.wrapping_neg();
            q = nv1.wrapping_div(nv2);
            r = nv1.wrapping_rem(nv2).wrapping_neg();
        } else {
            // q = 1, r = v1 - q * v2;
            // The comma operator sequences these, so `q` is already 1 here.
            q = 1;
            r = v1.wrapping_sub(q.wrapping_mul(v2));
        }
    } else if v2 >= 0 {
        // q = -((-(v1 + v2)) / (v2)) - 1, r = -((-(v1 + v2)) % (v2));
        let t = v1.wrapping_add(v2).wrapping_neg();
        q = t.wrapping_div(v2).wrapping_neg().wrapping_sub(1);
        r = t.wrapping_rem(v2).wrapping_neg();
    } else if v2 != C_INT_MIN {
        // q = ((-(v1 - v2)) / (-v2)) + 1, r = -((-(v1 - v2)) % (-v2));
        let t = v1.wrapping_sub(v2).wrapping_neg();
        let nv2 = v2.wrapping_neg();
        q = t.wrapping_div(nv2).wrapping_add(1);
        r = t.wrapping_rem(nv2).wrapping_neg();
    } else {
        // q = 1, r = 0;
        q = 1;
        r = 0;
    }

    if r >= 0 {
        q
    } else {
        // return q + (v2 > 0 ? -1 : 1);
        q.wrapping_add(if v2 > 0 { -1 } else { 1 })
    }
}
