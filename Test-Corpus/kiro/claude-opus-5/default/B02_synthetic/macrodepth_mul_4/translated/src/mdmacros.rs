// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

//! Rust translation of `src/mdmacros.h`.
//!
//! The C header does all of its work in the preprocessor: token pasting picks
//! an `op_<OP>` function, a `STEP_<OP>` expression, an `INIT_<OP>` seed, and a
//! `REP<REPEAT>` manual unrolling. None of that survives into the object file,
//! so the Rust equivalent resolves the same choices at compile time with
//! `#[cfg(feature = ...)]` and `const`.
//!
//! Feature -> CMake mapping:
//!   `add` / `sub` / `mul`  <=>  `-DOP=add|sub|mul`
//!   `0` ..= `7`            <=>  `-DREPEAT=0..7`
//!
//! `REPEAT` is bounded by 0..=7 in C as well: `RUN_LOOP` expands to
//! `CHOOSE_REP(REPEAT)`, i.e. `REP<REPEAT>`, and the header only defines
//! `REP0` through `REP7`.

use core::ffi::c_int;

// ---------------------------------------------------------------------------
// OP selection  (`#define OP add` / `-DOP=...`)
// ---------------------------------------------------------------------------
//
// Precedence is mul > sub > add so that enabling a non-default OP feature wins
// over the `add` that arrives with `default`. Absence of every OP feature falls
// back to `add`, matching the header's `#ifndef OP / #define OP add`.

/// `STR(OP)` -- the operation name as a NUL-terminated C string body.
///
/// All three names are three bytes, so one fixed-size array serves every
/// configuration.
#[cfg(feature = "mul")]
pub const OP_NAME_C: &[u8; 4] = b"mul\0";
#[cfg(all(feature = "sub", not(feature = "mul")))]
pub const OP_NAME_C: &[u8; 4] = b"sub\0";
#[cfg(all(not(feature = "sub"), not(feature = "mul")))]
pub const OP_NAME_C: &[u8; 4] = b"add\0";

/// `INIT_FOR(OP)`: `INIT_add`/`INIT_sub` are 0, `INIT_mul` is 1.
#[cfg(feature = "mul")]
pub const INIT: c_int = 1;
#[cfg(not(feature = "mul"))]
pub const INIT: c_int = 0;

/// `OP_FN(OP)` -- `CAT(op_, OP)`, the selected operation function.
///
/// Kept as an `extern "C" fn` value so that both the direct call in
/// `helper_call`, the local function pointer in `helper_ptr`, and the `G_OP`
/// global all refer to the very same exported symbol, as in C.
#[cfg(feature = "mul")]
pub const OP_FN: extern "C" fn(c_int, c_int) -> c_int = crate::mdcore::op_mul;
#[cfg(all(feature = "sub", not(feature = "mul")))]
pub const OP_FN: extern "C" fn(c_int, c_int) -> c_int = crate::mdcore::op_sub;
#[cfg(all(not(feature = "sub"), not(feature = "mul")))]
pub const OP_FN: extern "C" fn(c_int, c_int) -> c_int = crate::mdcore::op_add;

// ---------------------------------------------------------------------------
// STEP_OP(op, acc, i)
// ---------------------------------------------------------------------------
//
//   #define STEP_add(acc, i) ((acc) += (i))
//   #define STEP_sub(acc, i) ((acc) -= (i))
//   #define STEP_mul(acc, i) ((acc) *= ((i) + 1))
//
// Signed overflow is undefined in C but wraps in practice on the platforms this
// builds for (gcc/clang two's complement). `wrapping_*` reproduces that without
// panicking in debug builds.

/// One `STEP_mul` application.
#[cfg(feature = "mul")]
#[inline]
pub fn step(acc: c_int, i: c_int) -> c_int {
    acc.wrapping_mul(i.wrapping_add(1))
}

/// One `STEP_sub` application.
#[cfg(all(feature = "sub", not(feature = "mul")))]
#[inline]
pub fn step(acc: c_int, i: c_int) -> c_int {
    acc.wrapping_sub(i)
}

/// One `STEP_add` application.
#[cfg(all(not(feature = "sub"), not(feature = "mul")))]
#[inline]
pub fn step(acc: c_int, i: c_int) -> c_int {
    acc.wrapping_add(i)
}

// ---------------------------------------------------------------------------
// REPEAT selection  (`#define REPEAT 5` / `-DREPEAT=...`)
// ---------------------------------------------------------------------------
//
// As with OP, an explicitly requested value beats the `5` that comes in via
// `default`, and no REPEAT feature at all falls back to 5.

#[cfg(feature = "0")]
pub const REPEAT: c_int = 0;

#[cfg(all(feature = "1", not(feature = "0")))]
pub const REPEAT: c_int = 1;

#[cfg(all(feature = "2", not(any(feature = "0", feature = "1"))))]
pub const REPEAT: c_int = 2;

#[cfg(all(
    feature = "3",
    not(any(feature = "0", feature = "1", feature = "2"))
))]
pub const REPEAT: c_int = 3;

#[cfg(all(
    feature = "4",
    not(any(feature = "0", feature = "1", feature = "2", feature = "3"))
))]
pub const REPEAT: c_int = 4;

#[cfg(all(
    feature = "6",
    not(any(
        feature = "0",
        feature = "1",
        feature = "2",
        feature = "3",
        feature = "4"
    ))
))]
pub const REPEAT: c_int = 6;

#[cfg(all(
    feature = "7",
    not(any(
        feature = "0",
        feature = "1",
        feature = "2",
        feature = "3",
        feature = "4",
        feature = "6"
    ))
))]
pub const REPEAT: c_int = 7;

#[cfg(not(any(
    feature = "0",
    feature = "1",
    feature = "2",
    feature = "3",
    feature = "4",
    feature = "6",
    feature = "7"
)))]
pub const REPEAT: c_int = 5;

// ---------------------------------------------------------------------------
// REP0 .. REP7 unrolling
// ---------------------------------------------------------------------------

/// `REP<n>(op, acc)` for `n` in `0..=7`.
///
/// The C macros chain: `REP3` expands to `REP2` plus `STEP_OP(op, acc, 2)`, so
/// `REP<n>` applies `STEP_OP` once for each `i` in `0..n` with the literal index
/// baked in. A counted loop produces exactly the same sequence of accumulator
/// values; the C version is unrolled only because the preprocessor cannot
/// generate a loop bound from a token.
#[inline]
pub fn rep(mut acc: c_int, n: c_int) -> c_int {
    let mut i: c_int = 0;
    while i < n {
        acc = step(acc, i);
        i = i.wrapping_add(1);
    }
    acc
}

/// `RUN_LOOP(OP, acc, REPEAT)` -- `CHOOSE_REP(REPEAT)(OP, acc)`.
#[inline]
pub fn run_loop(acc: c_int) -> c_int {
    rep(acc, REPEAT)
}

/// `DISPATCH_REP(op, acc, n)`: a `switch (n)` covering only cases 0 through 6.
///
/// The `default:` arm is an empty `break`, so any other `n` -- including 7, and
/// including negatives -- leaves the accumulator at its initial value. This is a
/// real asymmetry in the C header (`REP7` exists but the switch stops at 6) and
/// is reproduced verbatim: with `-DREPEAT=7`, `use_generated(REPEAT)` reports
/// just `INIT_FOR(OP)`.
#[inline]
pub fn dispatch_rep(acc: c_int, n: c_int) -> c_int {
    match n {
        0 => rep(acc, 0),
        1 => rep(acc, 1),
        2 => rep(acc, 2),
        3 => rep(acc, 3),
        4 => rep(acc, 4),
        5 => rep(acc, 5),
        6 => rep(acc, 6),
        _ => acc,
    }
}

/// `FOR_EACH`/`DO_LOOP`: the runtime-bounded variant of the unrolling.
///
/// Defined in the header but never instantiated by `mdcore.c` or `mdmain.c`;
/// carried over for completeness so the translated header exposes the same
/// surface.
#[inline]
#[allow(dead_code)]
pub fn do_loop(acc: c_int, n: c_int) -> c_int {
    rep(acc, n)
}
