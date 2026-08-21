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
//! The C header is driven by two build-time knobs coming from the CMake cache:
//!
//! ```text
//! set(OP     "add" CACHE STRING "operation leveraged")
//! set(REPEAT "5"   CACHE STRING "iterations tested")
//! set(CMAKE_C_FLAGS "-DOP=${OP} -DREPEAT=${REPEAT}")
//! ```
//!
//! Those values are mirrored here by Cargo features carrying the *exact same
//! value names in lowercase*: `add`, `sub`, `mul` for `OP` and `0` .. `7` for
//! `REPEAT`.  When no feature of a family is enabled the `#ifndef` fallbacks of
//! `mdmacros.h` apply (`OP add`, `REPEAT 5`).

#![allow(dead_code)]

use std::ffi::{c_char, c_int};

/// The pieces of `<stdio.h>` used by `mdcore.c` / `mdmain.c`.
///
/// Going through C's `printf` (instead of Rust's `println!`) keeps the exact
/// same stdio stream and buffering behaviour as the original program, which
/// matters when the produced shared library is loaded by a C program that also
/// writes to `stdout`.
pub mod cstdio {
    use std::ffi::{c_char, c_int};

    extern "C" {
        pub fn printf(format: *const c_char, ...) -> c_int;
    }
}

/* ------------------------------------------------------------------------- */
/* OP selection: `#define OP add` (or sub / mul)                             */
/* ------------------------------------------------------------------------- */

// `mul` wins over `sub`, which wins over `add`, so that any combination of
// features still compiles to exactly one deterministic configuration.
#[cfg(feature = "mul")]
mod op_sel {
    use std::ffi::c_int;

    /// `STR(OP)`
    pub const OP_NAME: &str = "mul";
    /// `INIT_mul 1`
    pub const INIT: c_int = 1;

    /// `OP_FN(OP)` -> `op_mul`
    #[inline]
    pub const fn apply(a: c_int, b: c_int) -> c_int {
        a.wrapping_mul(b)
    }

    /// `STEP_mul(acc, i) ((acc) *= ((i) + 1))`
    #[inline]
    pub const fn step(acc: c_int, i: c_int) -> c_int {
        acc.wrapping_mul(i.wrapping_add(1))
    }
}

#[cfg(all(feature = "sub", not(feature = "mul")))]
mod op_sel {
    use std::ffi::c_int;

    /// `STR(OP)`
    pub const OP_NAME: &str = "sub";
    /// `INIT_sub 0`
    pub const INIT: c_int = 0;

    /// `OP_FN(OP)` -> `op_sub`
    #[inline]
    pub const fn apply(a: c_int, b: c_int) -> c_int {
        a.wrapping_sub(b)
    }

    /// `STEP_sub(acc, i) ((acc) -= (i))`
    #[inline]
    pub const fn step(acc: c_int, i: c_int) -> c_int {
        acc.wrapping_sub(i)
    }
}

#[cfg(all(not(feature = "sub"), not(feature = "mul")))]
mod op_sel {
    use std::ffi::c_int;

    /// `STR(OP)`
    pub const OP_NAME: &str = "add";
    /// `INIT_add 0`
    pub const INIT: c_int = 0;

    /// `OP_FN(OP)` -> `op_add`
    #[inline]
    pub const fn apply(a: c_int, b: c_int) -> c_int {
        a.wrapping_add(b)
    }

    /// `STEP_add(acc, i) ((acc) += (i))`
    #[inline]
    pub const fn step(acc: c_int, i: c_int) -> c_int {
        acc.wrapping_add(i)
    }
}

/// `STR(OP)` as a Rust string.
pub const OP_NAME: &str = op_sel::OP_NAME;

/// NUL terminated spelling of `STR(OP)`, used for the `G_OP_NAME` global.
#[cfg(feature = "mul")]
pub const OP_NAME_C: &[u8] = b"mul\0";
#[cfg(all(feature = "sub", not(feature = "mul")))]
pub const OP_NAME_C: &[u8] = b"sub\0";
#[cfg(all(not(feature = "sub"), not(feature = "mul")))]
pub const OP_NAME_C: &[u8] = b"add\0";

/// Pointer to the static NUL terminated operation name.
pub const fn op_name_ptr() -> *const c_char {
    OP_NAME_C.as_ptr() as *const c_char
}

/// `INIT_FOR(OP)`
pub const INIT: c_int = op_sel::INIT;

/// The operation selected by `OP_FN(OP)`.
#[inline]
pub const fn op_apply(a: c_int, b: c_int) -> c_int {
    op_sel::apply(a, b)
}

/// `STEP_OP(OP, acc, i)`
#[inline]
pub const fn step(acc: c_int, i: c_int) -> c_int {
    op_sel::step(acc, i)
}

/* ------------------------------------------------------------------------- */
/* REPEAT selection: `#define REPEAT 5` (0 .. 7 are supported by REPn)       */
/* ------------------------------------------------------------------------- */

#[cfg(feature = "0")]
pub const REPEAT: c_int = 0;

#[cfg(all(feature = "1", not(feature = "0")))]
pub const REPEAT: c_int = 1;

#[cfg(all(feature = "2", not(feature = "0"), not(feature = "1")))]
pub const REPEAT: c_int = 2;

#[cfg(all(
    feature = "3",
    not(feature = "0"),
    not(feature = "1"),
    not(feature = "2")
))]
pub const REPEAT: c_int = 3;

#[cfg(all(
    feature = "4",
    not(feature = "0"),
    not(feature = "1"),
    not(feature = "2"),
    not(feature = "3")
))]
pub const REPEAT: c_int = 4;

#[cfg(all(
    feature = "5",
    not(feature = "0"),
    not(feature = "1"),
    not(feature = "2"),
    not(feature = "3"),
    not(feature = "4")
))]
pub const REPEAT: c_int = 5;

#[cfg(all(
    feature = "6",
    not(feature = "0"),
    not(feature = "1"),
    not(feature = "2"),
    not(feature = "3"),
    not(feature = "4"),
    not(feature = "5")
))]
pub const REPEAT: c_int = 6;

#[cfg(all(
    feature = "7",
    not(feature = "0"),
    not(feature = "1"),
    not(feature = "2"),
    not(feature = "3"),
    not(feature = "4"),
    not(feature = "5"),
    not(feature = "6")
))]
pub const REPEAT: c_int = 7;

// `#ifndef REPEAT / # define REPEAT 5` fallback: no REPEAT feature selected.
#[cfg(all(
    not(feature = "0"),
    not(feature = "1"),
    not(feature = "2"),
    not(feature = "3"),
    not(feature = "4"),
    not(feature = "5"),
    not(feature = "6"),
    not(feature = "7")
))]
pub const REPEAT: c_int = 5;

/* ------------------------------------------------------------------------- */
/* REPEAT-based manual unrolling                                             */
/* ------------------------------------------------------------------------- */

/// `REPn(op, acc)` for a compile-time known `n`: performs `STEP_OP(op, acc, i)`
/// for `i` in `0 .. n`.
#[inline]
pub const fn rep_n(mut acc: c_int, n: c_int) -> c_int {
    let mut i: c_int = 0;
    while i < n {
        acc = step(acc, i);
        i += 1;
    }
    acc
}

/// `RUN_LOOP(OP, acc, REPEAT)` starting from `INIT_FOR(OP)`:
/// `CHOOSE_REP(REPEAT)(OP, acc)`.
#[inline]
pub const fn run_loop_from_init() -> c_int {
    rep_n(INIT, REPEAT)
}

/// `DO_LOOP(op, acc, n)` -> `for (int i = 0; i < n; ++i) STEP_OP(op, acc, i);`
#[inline]
pub const fn do_loop(acc: c_int, n: c_int) -> c_int {
    rep_n(acc, n)
}

/// `DISPATCH_REP(op, acc, n)`: a `switch (n)` over the hand written `REP0`
/// .. `REP6` expansions; every other value hits `default: break;` and therefore
/// leaves the accumulator untouched.
#[inline]
pub const fn dispatch_rep(acc: c_int, n: c_int) -> c_int {
    match n {
        0 => acc,               // REP0: nothing
        1 => rep_n(acc, 1),     // REP1
        2 => rep_n(acc, 2),     // REP2
        3 => rep_n(acc, 3),     // REP3
        4 => rep_n(acc, 4),     // REP4
        5 => rep_n(acc, 5),     // REP5
        6 => rep_n(acc, 6),     // REP6
        _ => acc,               // default: break
    }
}
