// Translation of c_src/src/mdmacros.h
//
// The C header is pure preprocessor machinery: it selects an operation family
// from the `OP` token and unrolls `REPEAT` accumulator steps via token pasting.
// Rust has no textual token pasting, so the selection is done with `cfg`
// features (one per CMake cache value) and the unrolling collapses into a
// bounded loop that is semantically identical to the unrolled chain.
//
// C macro -> Rust equivalent:
//   OP                 -> `add` / `sub` / `mul` features, see `sel` below
//   REPEAT             -> `REPEAT` const, from the `repeat_N` features
//   OP_FN(OP)          -> `OP_FN` (fn pointer to op_add / op_sub / op_mul)
//   INIT_FOR(OP)       -> `INIT`
//   STEP_OP(OP,acc,i)  -> `step(acc, i)`
//   STR(OP)            -> `OP_NAME` (NUL-terminated, as the C string literal)
//   RUN_LOOP(op,acc,n) -> `run_loop(acc)`
//   DEFINE_ACCUM(OP)   -> `accum(n)`
//   DO_LOOP(op,acc,n)  -> `do_loop(acc, n)`

use std::ffi::c_int;

use crate::mdcore::OpFn;

/* ---------------------------------------------------------------------------
 * `#define REPEAT <value>` (default 5 when the macro is not predefined).
 *
 * Exactly one value wins. `repeat_5` is resolved last so that the default
 * feature set can be left enabled while overriding REPEAT on the command line.
 * ------------------------------------------------------------------------- */

#[cfg(feature = "repeat_0")]
pub const REPEAT: c_int = 0;

#[cfg(all(not(feature = "repeat_0"), feature = "repeat_1"))]
pub const REPEAT: c_int = 1;

#[cfg(all(
    not(feature = "repeat_0"),
    not(feature = "repeat_1"),
    feature = "repeat_2"
))]
pub const REPEAT: c_int = 2;

#[cfg(all(
    not(feature = "repeat_0"),
    not(feature = "repeat_1"),
    not(feature = "repeat_2"),
    feature = "repeat_3"
))]
pub const REPEAT: c_int = 3;

#[cfg(all(
    not(feature = "repeat_0"),
    not(feature = "repeat_1"),
    not(feature = "repeat_2"),
    not(feature = "repeat_3"),
    feature = "repeat_4"
))]
pub const REPEAT: c_int = 4;

#[cfg(all(
    not(feature = "repeat_0"),
    not(feature = "repeat_1"),
    not(feature = "repeat_2"),
    not(feature = "repeat_3"),
    not(feature = "repeat_4"),
    feature = "repeat_6"
))]
pub const REPEAT: c_int = 6;

#[cfg(all(
    not(feature = "repeat_0"),
    not(feature = "repeat_1"),
    not(feature = "repeat_2"),
    not(feature = "repeat_3"),
    not(feature = "repeat_4"),
    not(feature = "repeat_6"),
    feature = "repeat_7"
))]
pub const REPEAT: c_int = 7;

// `repeat_5`, and also the fallback when no REPEAT feature is selected at all
// (mirrors `#ifndef REPEAT / #define REPEAT 5`).
#[cfg(all(
    not(feature = "repeat_0"),
    not(feature = "repeat_1"),
    not(feature = "repeat_2"),
    not(feature = "repeat_3"),
    not(feature = "repeat_4"),
    not(feature = "repeat_6"),
    not(feature = "repeat_7")
))]
pub const REPEAT: c_int = 5;

/* ---------------------------------------------------------------------------
 * The `OP` family: INIT_<op>, STEP_<op>, op_<op> and STR(OP).
 *
 * `add` is resolved last so the default feature set can stay enabled while
 * selecting a different operation on the command line.
 * ------------------------------------------------------------------------- */

#[cfg(feature = "sub")]
mod sel {
    use super::OpFn;
    use std::ffi::c_int;

    /// `STR(OP)` -- the C string literal, NUL terminated.
    pub const OP_NAME: &[u8] = b"sub\0";
    /// `INIT_sub`
    pub const INIT: c_int = 0;
    /// `OP_FN(sub)` == `op_sub`
    pub const OP_FN: OpFn = crate::mdcore::op_sub;

    /// `STEP_sub(acc, i)` == `((acc) -= (i))`
    #[inline]
    pub fn step(acc: c_int, i: c_int) -> c_int {
        acc.wrapping_sub(i)
    }
}

#[cfg(all(not(feature = "sub"), feature = "mul"))]
mod sel {
    use super::OpFn;
    use std::ffi::c_int;

    /// `STR(OP)` -- the C string literal, NUL terminated.
    pub const OP_NAME: &[u8] = b"mul\0";
    /// `INIT_mul`
    pub const INIT: c_int = 1;
    /// `OP_FN(mul)` == `op_mul`
    pub const OP_FN: OpFn = crate::mdcore::op_mul;

    /// `STEP_mul(acc, i)` == `((acc) *= ((i) + 1))`
    #[inline]
    pub fn step(acc: c_int, i: c_int) -> c_int {
        acc.wrapping_mul(i.wrapping_add(1))
    }
}

// `add`, and also the fallback when no OP feature is selected at all
// (mirrors `#ifndef OP / #define OP add`).
#[cfg(all(not(feature = "sub"), not(feature = "mul")))]
mod sel {
    use super::OpFn;
    use std::ffi::c_int;

    /// `STR(OP)` -- the C string literal, NUL terminated.
    pub const OP_NAME: &[u8] = b"add\0";
    /// `INIT_add`
    pub const INIT: c_int = 0;
    /// `OP_FN(add)` == `op_add`
    pub const OP_FN: OpFn = crate::mdcore::op_add;

    /// `STEP_add(acc, i)` == `((acc) += (i))`
    #[inline]
    pub fn step(acc: c_int, i: c_int) -> c_int {
        acc.wrapping_add(i)
    }
}

pub use self::sel::{step, INIT, OP_FN, OP_NAME};

/// `STR(OP)` as a Rust string (for `printf("%s")`), without the NUL byte.
pub fn op_name_str() -> &'static str {
    let bytes = &OP_NAME[..OP_NAME.len() - 1];
    // Every candidate literal above is ASCII.
    std::str::from_utf8(bytes).unwrap()
}

/* ---------------------------------------------------------------------------
 * Unrolling helpers.
 * ------------------------------------------------------------------------- */

/// `RUN_LOOP(OP, acc, REPEAT)` == `CHOOSE_REP(REPEAT)(OP, acc)`.
///
/// `REPn` expands to `STEP_OP(op, acc, 0); ... STEP_OP(op, acc, n-1);`, i.e.
/// the step applied with the literal indices `0 .. n-1` in order.
#[inline]
pub fn run_loop(mut acc: c_int) -> c_int {
    let mut i: c_int = 0;
    while i < REPEAT {
        acc = step(acc, i);
        i += 1;
    }
    acc
}

/// `DO_LOOP(op, acc, n)` == `FOR_EACH((n), { STEP_OP(op, acc, i); })`.
///
/// Unused by the C sources, kept for parity with the header.
#[allow(dead_code)]
#[inline]
pub fn do_loop(mut acc: c_int, n: c_int) -> c_int {
    let mut i: c_int = 0;
    while i < n {
        acc = step(acc, i);
        i += 1;
    }
    acc
}

/// `DEFINE_ACCUM(OP)` -> `static int accum_<OP>(int n)`.
///
/// Note the faithful reproduction of `DISPATCH_REP`: it only has cases for
/// `0..=6`, so `n == 7` (or any other out-of-range value) hits `default:` and
/// leaves the accumulator at its initial value. This is deliberate; the C
/// behaviour is not "fixed" here.
pub fn accum(n: c_int) -> c_int {
    let mut acc: c_int = INIT;
    match n {
        0 => {}
        1 => {
            acc = step(acc, 0);
        }
        2 => {
            acc = step(acc, 0);
            acc = step(acc, 1);
        }
        3 => {
            acc = step(acc, 0);
            acc = step(acc, 1);
            acc = step(acc, 2);
        }
        4 => {
            acc = step(acc, 0);
            acc = step(acc, 1);
            acc = step(acc, 2);
            acc = step(acc, 3);
        }
        5 => {
            acc = step(acc, 0);
            acc = step(acc, 1);
            acc = step(acc, 2);
            acc = step(acc, 3);
            acc = step(acc, 4);
        }
        6 => {
            acc = step(acc, 0);
            acc = step(acc, 1);
            acc = step(acc, 2);
            acc = step(acc, 3);
            acc = step(acc, 4);
            acc = step(acc, 5);
        }
        _ => { /* default: break; */ }
    }
    acc
}
