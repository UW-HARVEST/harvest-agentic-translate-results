//! Translation of `c_src/src/mdmacros.h`.
//!
//! The C header does all of its work in the preprocessor: `OP` and `REPEAT` are
//! `-D` macros supplied by CMake, and token pasting picks the operation
//! function, the accumulator seed, the per-iteration step and the unrolled
//! `REPn` chain. Rust has no token pasting across `cfg`, so the selection is
//! expressed as `#[cfg(feature = ...)]`-gated constants and functions; the
//! resulting code is the same as what the preprocessor produces for a given
//! configuration.
//!
//! Arithmetic uses the wrapping operators throughout. Signed overflow is
//! undefined in C, but gcc/clang emit two's-complement wraparound, so wrapping
//! reproduces the observable behaviour instead of panicking.

use core::ffi::{CStr, c_int};

/* ---------------------------------------------------------------- OP -------
 * #ifndef OP
 * #  define OP add
 * #endif
 *
 * When several OP features are enabled at once, the order below decides:
 * mul > sub > add. `add` is also the fallback when none is enabled.
 */

/// The operation selected at build time, i.e. the `OP` macro.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Op {
    Add,
    Sub,
    Mul,
}

#[cfg(feature = "mul")]
pub const OP: Op = Op::Mul;
#[cfg(all(not(feature = "mul"), feature = "sub"))]
pub const OP: Op = Op::Sub;
#[cfg(all(not(feature = "mul"), not(feature = "sub")))]
pub const OP: Op = Op::Add;

/// `STR(OP)` — the stringified operation name, NUL terminated for `G_OP_NAME`.
#[cfg(feature = "mul")]
pub const OP_NAME: &CStr = c"mul";
#[cfg(all(not(feature = "mul"), feature = "sub"))]
pub const OP_NAME: &CStr = c"sub";
#[cfg(all(not(feature = "mul"), not(feature = "sub")))]
pub const OP_NAME: &CStr = c"add";

/* ------------------------------------------------------------ REPEAT -------
 * #ifndef REPEAT
 * #  define REPEAT 5
 * #endif
 *
 * Only REP0 .. REP7 exist in the header, so 0..=7 are the buildable values.
 * The cfg cascade guarantees exactly one definition for any feature set: the
 * lowest enabled value wins, and 5 is used when none is enabled.
 */

#[cfg(feature = "0")]
pub const REPEAT: c_int = 0;
#[cfg(all(not(feature = "0"), feature = "1"))]
pub const REPEAT: c_int = 1;
#[cfg(all(not(feature = "0"), not(feature = "1"), feature = "2"))]
pub const REPEAT: c_int = 2;
#[cfg(all(
    not(feature = "0"),
    not(feature = "1"),
    not(feature = "2"),
    feature = "3"
))]
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
/// `#ifndef REPEAT -> 5`
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

/* --------------------------------------------------------- INIT_FOR --------
 * #define INIT_add 0
 * #define INIT_sub 0
 * #define INIT_mul 1
 * #define INIT_FOR(op) CAT(INIT_, op)
 */

/// `INIT_FOR(OP)` — the accumulator seed for the selected operation.
pub const INIT: c_int = match OP {
    Op::Add => 0,
    Op::Sub => 0,
    Op::Mul => 1,
};

/* ---------------------------------------------------------- STEP_OP --------
 * #define STEP_add(acc, i) ((acc) += (i))
 * #define STEP_sub(acc, i) ((acc) -= (i))
 * #define STEP_mul(acc, i) ((acc) *= ((i) + 1))
 */

/// `STEP_OP(OP, acc, i)`, returning the updated accumulator.
#[inline]
pub const fn step_op(acc: c_int, i: c_int) -> c_int {
    match OP {
        Op::Add => acc.wrapping_add(i),
        Op::Sub => acc.wrapping_sub(i),
        Op::Mul => acc.wrapping_mul(i.wrapping_add(1)),
    }
}

/* ------------------------------------------------------------- REPn --------
 * REP0 expands to nothing; REPn = REP(n-1) followed by STEP_OP(op, acc, n-1).
 * So REPn applies the step for i = 0 .. n-1, in that order.
 */

/// `REPn(OP, acc)` for `n` in `0..=7`.
#[inline]
pub const fn rep(acc: c_int, n: c_int) -> c_int {
    let mut acc = acc;
    let mut i = 0;
    while i < n {
        acc = step_op(acc, i);
        i += 1;
    }
    acc
}

/* --------------------------------------------------------- RUN_LOOP --------
 * #define RUN_LOOP(op, acc, n) CHOOSE_REP(n)(op, acc)
 *
 * `n` is always the REPEAT macro at the call sites, so this is the fully
 * unrolled REP<REPEAT> chain.
 */

/// `RUN_LOOP(OP, acc, REPEAT)`, returning the updated accumulator.
#[inline]
pub const fn run_loop(acc: c_int) -> c_int {
    rep(acc, REPEAT)
}

/* ------------------------------------------------------ DISPATCH_REP -------
 * switch (n) { case 0..6: REPn(op, acc); break; default: break; }
 *
 * Note the switch stops at 6 even though REP7 exists: n == 7 (or anything else
 * outside 0..=6) falls into `default` and leaves the accumulator untouched.
 * This is reproduced verbatim, not "fixed".
 */

/// `DISPATCH_REP(OP, acc, n)`, returning the resulting accumulator.
#[inline]
pub const fn dispatch_rep(acc: c_int, n: c_int) -> c_int {
    match n {
        0..=6 => rep(acc, n),
        _ => acc,
    }
}

/* --------------------------------------------------------- FOR_EACH --------
 * #define FOR_EACH(n, body) for (int i = 0; i < (n); ++i) body
 * #define DO_LOOP(op, acc, n) FOR_EACH((n), { STEP_OP(op, acc, i); })
 *
 * DO_LOOP is unused by the current sources but is part of the header's API, so
 * it is translated for completeness.
 */

/// `DO_LOOP(OP, acc, n)` — the rolled equivalent of `REPn`.
#[inline]
pub const fn do_loop(acc: c_int, n: c_int) -> c_int {
    let mut acc = acc;
    let mut i = 0;
    while i < n {
        acc = step_op(acc, i);
        i += 1;
    }
    acc
}
