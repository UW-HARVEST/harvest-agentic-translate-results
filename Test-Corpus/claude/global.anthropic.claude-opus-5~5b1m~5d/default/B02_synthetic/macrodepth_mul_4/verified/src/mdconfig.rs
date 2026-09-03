// Translation of the configurable parts of c_src/src/mdmacros.h
//
// The C code selects, at build time via `-DOP=<op> -DREPEAT=<n>`:
//   * which `op_<OP>` function is used for OP_FN(OP)
//   * the token-pasted STEP_<OP> accumulator step
//   * INIT_<OP>, the initial accumulator value
//   * the REP<REPEAT> manual unrolling depth
//
// Here that becomes Cargo features: `add` / `sub` / `mul` for OP and
// `"0"` .. `"7"` for REPEAT.  Feature combinations are resolved with a fixed
// priority so that every combination still compiles.

use std::ffi::c_int;

/* ------------------------------ OP selection ------------------------------ */

// Priority: add > sub > mul; if none is enabled fall back to the C default
// (`#ifndef OP / #define OP add`).
#[cfg(feature = "add")]
pub const OP_NAME: &str = "add";
#[cfg(all(not(feature = "add"), feature = "sub"))]
pub const OP_NAME: &str = "sub";
#[cfg(all(not(feature = "add"), not(feature = "sub"), feature = "mul"))]
pub const OP_NAME: &str = "mul";
#[cfg(not(any(feature = "add", feature = "sub", feature = "mul")))]
pub const OP_NAME: &str = "add";

/// NUL-terminated form of `STR(OP)` for the exported `G_OP_NAME` global.
#[cfg(feature = "add")]
pub const OP_NAME_C: &[u8] = b"add\0";
#[cfg(all(not(feature = "add"), feature = "sub"))]
pub const OP_NAME_C: &[u8] = b"sub\0";
#[cfg(all(not(feature = "add"), not(feature = "sub"), feature = "mul"))]
pub const OP_NAME_C: &[u8] = b"mul\0";
#[cfg(not(any(feature = "add", feature = "sub", feature = "mul")))]
pub const OP_NAME_C: &[u8] = b"add\0";

/// `INIT_FOR(OP)`: INIT_add 0, INIT_sub 0, INIT_mul 1.
#[cfg(all(not(feature = "add"), not(feature = "sub"), feature = "mul"))]
pub const INIT: c_int = 1;
#[cfg(not(all(not(feature = "add"), not(feature = "sub"), feature = "mul")))]
pub const INIT: c_int = 0;

/// `STEP_OP(OP, acc, i)`:
///   STEP_add(acc, i) => acc += i
///   STEP_sub(acc, i) => acc -= i
///   STEP_mul(acc, i) => acc *= (i + 1)
#[inline]
pub fn step(acc: c_int, i: c_int) -> c_int {
    #[cfg(all(not(feature = "add"), not(feature = "sub"), feature = "mul"))]
    {
        acc.wrapping_mul(i.wrapping_add(1))
    }
    #[cfg(all(not(feature = "add"), feature = "sub"))]
    {
        acc.wrapping_sub(i)
    }
    #[cfg(any(
        feature = "add",
        not(any(feature = "add", feature = "sub", feature = "mul"))
    ))]
    {
        acc.wrapping_add(i)
    }
}

/// `OP_FN(OP)`: the selected `op_<OP>` function.
#[inline]
pub const fn op_fn() -> extern "C" fn(c_int, c_int) -> c_int {
    #[cfg(all(not(feature = "add"), not(feature = "sub"), feature = "mul"))]
    {
        crate::mdcore::op_mul
    }
    #[cfg(all(not(feature = "add"), feature = "sub"))]
    {
        crate::mdcore::op_sub
    }
    #[cfg(any(
        feature = "add",
        not(any(feature = "add", feature = "sub", feature = "mul"))
    ))]
    {
        crate::mdcore::op_add
    }
}

/* ---------------------------- REPEAT selection ---------------------------- */

// Priority: the lowest explicitly requested value wins; the C default is 5.
#[cfg(feature = "0")]
pub const REPEAT: c_int = 0;
#[cfg(all(not(feature = "0"), feature = "1"))]
pub const REPEAT: c_int = 1;
#[cfg(all(not(feature = "0"), not(feature = "1"), feature = "2"))]
pub const REPEAT: c_int = 2;
#[cfg(all(not(feature = "0"), not(feature = "1"), not(feature = "2"), feature = "3"))]
pub const REPEAT: c_int = 3;
#[cfg(all(
    not(feature = "0"),
    not(feature = "1"),
    not(feature = "2"),
    not(feature = "3"),
    feature = "4"
))]
pub const REPEAT: c_int = 4;
#[cfg(all(
    not(feature = "0"),
    not(feature = "1"),
    not(feature = "2"),
    not(feature = "3"),
    not(feature = "4"),
    feature = "5"
))]
pub const REPEAT: c_int = 5;
#[cfg(all(
    not(feature = "0"),
    not(feature = "1"),
    not(feature = "2"),
    not(feature = "3"),
    not(feature = "4"),
    not(feature = "5"),
    feature = "6"
))]
pub const REPEAT: c_int = 6;
#[cfg(all(
    not(feature = "0"),
    not(feature = "1"),
    not(feature = "2"),
    not(feature = "3"),
    not(feature = "4"),
    not(feature = "5"),
    not(feature = "6"),
    feature = "7"
))]
pub const REPEAT: c_int = 7;
#[cfg(not(any(
    feature = "0",
    feature = "1",
    feature = "2",
    feature = "3",
    feature = "4",
    feature = "5",
    feature = "6",
    feature = "7"
)))]
pub const REPEAT: c_int = 5;

/// `RUN_LOOP(OP, acc, REPEAT)` == `REP<REPEAT>(OP, acc)`: the manually
/// unrolled sequence `STEP_OP(op, acc, 0); ... STEP_OP(op, acc, REPEAT-1);`
#[inline]
pub fn run_loop(mut acc: c_int) -> c_int {
    let mut i: c_int = 0;
    while i < REPEAT {
        acc = step(acc, i);
        i += 1;
    }
    acc
}

/// `DISPATCH_REP(OP, acc, n)`: switch over n with cases 0..=6 that perform
/// REPn; any other value leaves the accumulator untouched (`default: break`).
#[inline]
pub fn dispatch_rep(acc: c_int, n: c_int) -> c_int {
    match n {
        0..=6 => {
            let mut a = acc;
            let mut i: c_int = 0;
            while i < n {
                a = step(a, i);
                i += 1;
            }
            a
        }
        _ => acc,
    }
}
