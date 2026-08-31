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

//! Rust equivalent of `mdmacros.h`.
//!
//! The C header selects, at preprocessing time, an operation family (`OP`) and
//! an iteration count (`REPEAT`) via `-D` defines supplied by CMake. Here those
//! selections are resolved from Cargo features, using the exact same names in
//! lowercase (`add`/`sub`/`mul` for `OP`; `0`..`7` for `REPEAT`).
//!
//! Feature semantics:
//! * When no OP feature is enabled the default is `add`; when no REPEAT feature
//!   is enabled the default is `5` (matching the CMake cache defaults).
//! * Enabling several conflicting values still compiles ("all combinations must
//!   compile"): OP priority is `mul > sub > add`, REPEAT priority is the highest
//!   selected number.

use core::ffi::c_int;

use crate::mdcore::{op_add, op_mul, op_sub};

/// `STR(OP)` — the textual name of the selected operation.
///
/// Selection priority mirrors the cfg cascade used throughout this module:
/// `mul` > `sub` > `add` (with `add` as the default).
/// (Consumers read the exported `G_OP_NAME` global instead, mirroring the C, so
/// this constant is informational.)
#[allow(dead_code)]
pub const OP_NAME: &str = if cfg!(feature = "mul") {
    "mul"
} else if cfg!(feature = "sub") {
    "sub"
} else {
    "add"
};

/// `INIT_FOR(OP)` — the initial accumulator value for the selected op
/// (`INIT_add == 0`, `INIT_sub == 0`, `INIT_mul == 1`).
pub const INIT_FOR: c_int = if cfg!(feature = "mul") { 1 } else { 0 };

/// `REPEAT` — the iteration count baked in at build time.
///
/// The C macros define `REP0`..`REP7`, so valid values are `0..=7`; the CMake
/// default is `5`. If several REPEAT features are enabled the largest wins.
pub const REPEAT: c_int = if cfg!(feature = "7") {
    7
} else if cfg!(feature = "6") {
    6
} else if cfg!(feature = "5") {
    5
} else if cfg!(feature = "4") {
    4
} else if cfg!(feature = "3") {
    3
} else if cfg!(feature = "2") {
    2
} else if cfg!(feature = "1") {
    1
} else if cfg!(feature = "0") {
    0
} else {
    5
};

/// `STEP_OP(OP, acc, i)` for the selected operation.
///
/// * `add`: `acc += i`
/// * `sub`: `acc -= i`
/// * `mul`: `acc *= (i + 1)`
///
/// Arithmetic wraps on overflow to match the two's-complement behavior produced
/// by the C toolchain for these programs.
#[inline]
pub fn step(acc: c_int, i: c_int) -> c_int {
    if cfg!(feature = "mul") {
        acc.wrapping_mul(i.wrapping_add(1))
    } else if cfg!(feature = "sub") {
        acc.wrapping_sub(i)
    } else {
        acc.wrapping_add(i)
    }
}

/// `(OP_FN(OP))(a, b)` — invoke the selected operation directly.
#[inline]
pub fn op_fn(a: c_int, b: c_int) -> c_int {
    if cfg!(feature = "mul") {
        op_mul(a, b)
    } else if cfg!(feature = "sub") {
        op_sub(a, b)
    } else {
        op_add(a, b)
    }
}
