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
//! The C header performs all of its work with the preprocessor: the build-time
//! macros `OP` (one of `add`, `sub`, `mul`) and `REPEAT` (an integer literal
//! `0`..=`7`, since only `REP0`..`REP7` exist) select which operation function
//! and which unrolled accumulation sequence get compiled in.
//!
//! Here the same selection is made with Cargo features. Each CMake cache value
//! becomes a feature with the identical (lowercased) name:
//!   * `OP`     -> features `add`, `sub`, `mul`
//!   * `REPEAT` -> features `0`, `1`, `2`, `3`, `4`, `5`, `6`, `7`
//!                 (aliases `repeat_0` .. `repeat_7` are also accepted)

use std::ffi::c_int;

/// The operation family selected at build time (C: `OP`).
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Op {
    Add,
    Sub,
    Mul,
}

/// Selected operation. A feature naming a non-default value wins over the
/// default (`add`) so that `--features sub` behaves like `-DOP=sub`.
pub const OP: Op = if cfg!(feature = "mul") {
    Op::Mul
} else if cfg!(feature = "sub") {
    Op::Sub
} else {
    Op::Add
};

/// C: `STR(OP)` -- the stringized operation name (`G_OP_NAME` points here).
pub const OP_NAME: &str = match OP {
    Op::Add => "add",
    Op::Sub => "sub",
    Op::Mul => "mul",
};

/// NUL terminated form of [`OP_NAME`], used for the exported `G_OP_NAME`.
pub const OP_NAME_CSTR: &[u8] = match OP {
    Op::Add => b"add\0",
    Op::Sub => b"sub\0",
    Op::Mul => b"mul\0",
};

/// The unroll count selected at build time (C: `REPEAT`).
/// A feature naming a non-default value wins over the default (`5`).
pub const REPEAT: c_int = if cfg!(feature = "0") {
    0
} else if cfg!(feature = "1") {
    1
} else if cfg!(feature = "2") {
    2
} else if cfg!(feature = "3") {
    3
} else if cfg!(feature = "4") {
    4
} else if cfg!(feature = "6") {
    6
} else if cfg!(feature = "7") {
    7
} else {
    5
};

/// C: `INIT_FOR(OP)` -- `INIT_add`/`INIT_sub` are 0, `INIT_mul` is 1.
#[inline]
pub const fn init_for_op() -> c_int {
    match OP {
        Op::Add => 0,
        Op::Sub => 0,
        Op::Mul => 1,
    }
}

/// C: `STEP_OP(OP, acc, i)`:
///   `STEP_add(acc,i)` -> `acc += i`
///   `STEP_sub(acc,i)` -> `acc -= i`
///   `STEP_mul(acc,i)` -> `acc *= (i + 1)`
///
/// Signed overflow is UB in C but wraps on every mainstream compiler; the
/// wrapping operators reproduce the observed behaviour.
#[inline]
pub const fn step_op(acc: c_int, i: c_int) -> c_int {
    match OP {
        Op::Add => acc.wrapping_add(i),
        Op::Sub => acc.wrapping_sub(i),
        Op::Mul => acc.wrapping_mul(i.wrapping_add(1)),
    }
}

/// C: `REPn(OP, acc)` -- applies `STEP_OP` for `i = 0 .. n-1`.
#[inline]
pub const fn rep_n(mut acc: c_int, n: c_int) -> c_int {
    let mut i: c_int = 0;
    while i < n {
        acc = step_op(acc, i);
        i += 1;
    }
    acc
}

/// C: `RUN_LOOP(OP, acc, REPEAT)` -- expands to `REP<REPEAT>(OP, acc)`.
/// (Only `REP0`..`REP7` exist, so `REPEAT` outside 0..=7 fails to compile in C.)
#[inline]
pub const fn run_loop(acc: c_int) -> c_int {
    rep_n(acc, REPEAT)
}

/// C: `DO_LOOP` / `FOR_EACH` -- the runtime loop form of the same body.
/// Unused by the C program (kept for fidelity with the header).
#[allow(dead_code)]
#[inline]
pub const fn do_loop(acc: c_int, n: c_int) -> c_int {
    rep_n(acc, n)
}

/// C: `DISPATCH_REP(OP, acc, n)` -- a switch over `n` covering only cases
/// 0..=6; every other value (including 7 and negatives) hits `default: break;`
/// and leaves the accumulator untouched.
#[inline]
pub const fn dispatch_rep(acc: c_int, n: c_int) -> c_int {
    match n {
        0 | 1 | 2 | 3 | 4 | 5 | 6 => rep_n(acc, n),
        _ => acc,
    }
}
