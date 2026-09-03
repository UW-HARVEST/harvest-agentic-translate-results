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
//!   compile"). A single build of the C code can only have one `OP` and one
//!   `REPEAT`, so a conflicting feature set has no C counterpart; it is resolved
//!   with a fixed priority: **OP `add` > `sub` > `mul`**, and **REPEAT: the
//!   lowest selected number wins**.
//!
//! # Relationship to [`crate::mdconfig`]
//!
//! [`crate::mdconfig`] is the module the exported `#[no_mangle]` symbols in
//! [`crate::mdcore`] actually use, and it is what the differential test suite
//! pins down. This module is a thin, self-contained restatement of the same
//! macros for readers comparing against `mdmacros.h` side by side. It therefore
//! **must** resolve `OP` / `REPEAT` identically to `mdconfig`; the
//! `agrees_with_mdconfig` tests below fail the build if the two ever drift.

use core::ffi::c_int;

use crate::mdcore::{op_add, op_mul, op_sub};

/// `STR(OP)` — the textual name of the selected operation.
///
/// Selection priority: `add` > `sub` > `mul`, with `add` as the default
/// (`#ifndef OP / #define OP add`). Identical to [`crate::mdconfig::OP_NAME`].
pub const OP_NAME: &str = if cfg!(feature = "add") {
    "add"
} else if cfg!(feature = "sub") {
    "sub"
} else if cfg!(feature = "mul") {
    "mul"
} else {
    "add"
};

/// `INIT_FOR(OP)` — the initial accumulator value for the selected op
/// (`INIT_add == 0`, `INIT_sub == 0`, `INIT_mul == 1`).
pub const INIT_FOR: c_int = if matches!(OP_NAME.as_bytes(), b"mul") { 1 } else { 0 };

/// `REPEAT` — the iteration count baked in at build time.
///
/// The C macros define `REP0`..`REP7`, so valid values are `0..=7`; the CMake
/// default is `5`. If several REPEAT features are enabled the *lowest* wins,
/// matching [`crate::mdconfig::REPEAT`].
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
} else if cfg!(feature = "5") {
    5
} else if cfg!(feature = "6") {
    6
} else if cfg!(feature = "7") {
    7
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
    match OP_NAME.as_bytes() {
        b"mul" => acc.wrapping_mul(i.wrapping_add(1)),
        b"sub" => acc.wrapping_sub(i),
        _ => acc.wrapping_add(i),
    }
}

/// `(OP_FN(OP))(a, b)` — invoke the selected operation directly.
#[inline]
pub fn op_fn(a: c_int, b: c_int) -> c_int {
    match OP_NAME.as_bytes() {
        b"mul" => op_mul(a, b),
        b"sub" => op_sub(a, b),
        _ => op_add(a, b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mdconfig;

    /// This module and `mdconfig` must never disagree about the resolved
    /// configuration, for any feature combination.
    #[test]
    fn agrees_with_mdconfig() {
        assert_eq!(OP_NAME, mdconfig::OP_NAME);
        assert_eq!(INIT_FOR, mdconfig::INIT);
        assert_eq!(REPEAT, mdconfig::REPEAT);

        let mut name = OP_NAME.as_bytes().to_vec();
        name.push(0);
        assert_eq!(name, mdconfig::OP_NAME_C);

        for a in [0i32, 1, -1, 7, -7, i32::MAX, i32::MIN, 46341] {
            for b in [0i32, 1, -1, 7, -7, i32::MAX, i32::MIN, 46341] {
                assert_eq!(op_fn(a, b), (mdconfig::op_fn())(a, b), "op_fn({a}, {b})");
            }
            for i in -8..=8 {
                assert_eq!(step(a, i), mdconfig::step(a, i), "step({a}, {i})");
            }
        }
    }
}
