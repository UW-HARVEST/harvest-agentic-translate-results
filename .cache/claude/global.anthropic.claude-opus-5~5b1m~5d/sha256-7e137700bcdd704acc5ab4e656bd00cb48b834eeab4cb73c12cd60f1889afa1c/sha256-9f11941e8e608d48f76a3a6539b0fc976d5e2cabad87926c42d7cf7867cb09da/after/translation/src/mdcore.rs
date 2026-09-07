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

//! Rust equivalent of `mdcore.c`.

use core::ffi::{c_char, c_int, CStr};

use crate::mdmacros::{op_fn, step, INIT_FOR, REPEAT};

/* Define operations */

/// `int op_add(int a, int b) { return a + b; }`
#[unsafe(no_mangle)]
pub extern "C" fn op_add(a: c_int, b: c_int) -> c_int {
    a.wrapping_add(b)
}

/// `int op_sub(int a, int b) { return a - b; }`
#[unsafe(no_mangle)]
pub extern "C" fn op_sub(a: c_int, b: c_int) -> c_int {
    a.wrapping_sub(b)
}

/// `int op_mul(int a, int b) { return a * b; }`
#[unsafe(no_mangle)]
pub extern "C" fn op_mul(a: c_int, b: c_int) -> c_int {
    a.wrapping_mul(b)
}

/// Macro-generated accumulator for the selected OP: `DEFINE_ACCUM(OP)`.
///
/// The C `DISPATCH_REP` expands to `switch (n) { case 0..6: REPk; default: }`,
/// where each `REPk` applies `step` for indices `0..k`. Only cases `0..=6` are
/// handled; any other `n` (including `>= 7` or negative) falls through
/// `default:` and leaves the accumulator at its initial value.
///
/// This is `static` in C (file-local), so it has no exported linker symbol.
fn accum(n: c_int) -> c_int {
    let mut acc = INIT_FOR;
    if (0..=6).contains(&n) {
        let mut i: c_int = 0;
        while i < n {
            acc = step(acc, i);
            i += 1;
        }
    }
    acc
}

/* Global macro uses at file scope (exercises expansion at global init) */

/// `int (*G_OP)(int,int) = OP_FN(OP);` — pointer to the selected op function.
///
/// The C object lives in writable `.data`: it is exported non-const, so any
/// consumer of the shared library may legally overwrite it. It is therefore
/// declared `static mut` here — an immutable Rust `static` holding a relocated
/// pointer is emitted into `.data.rel.ro`, which is mapped read-only after
/// relocation processing, and a caller's store would fault instead of taking
/// effect the way it does against the C `.so`.
#[cfg(feature = "mul")]
#[unsafe(no_mangle)]
pub static mut G_OP: extern "C" fn(c_int, c_int) -> c_int = op_mul;
#[cfg(all(feature = "sub", not(feature = "mul")))]
#[unsafe(no_mangle)]
pub static mut G_OP: extern "C" fn(c_int, c_int) -> c_int = op_sub;
#[cfg(all(not(feature = "mul"), not(feature = "sub")))]
#[unsafe(no_mangle)]
pub static mut G_OP: extern "C" fn(c_int, c_int) -> c_int = op_add;

/// `const char *G_OP_NAME = STR(OP);`
///
/// Note the C `const` binds to the *pointee*, not to the pointer: the pointer
/// object itself is mutable data in `.data`. Mirrored with `static mut` for the
/// same section-placement reason as `G_OP`.
const G_OP_NAME_CSTR: &CStr = if cfg!(feature = "mul") {
    c"mul"
} else if cfg!(feature = "sub") {
    c"sub"
} else {
    c"add"
};

#[unsafe(no_mangle)]
pub static mut G_OP_NAME: *const c_char = G_OP_NAME_CSTR.as_ptr();

/* Helpers provided by md_core.c */

/// ```c
/// int helper_call(int a, int b) {
///     int r = (OP_FN(OP))(a, b);
///     int acc = INIT_FOR(OP);
///     RUN_LOOP(OP, acc, REPEAT);
///     printf("helper.call=%d helper.acc=%d\n", r, acc);
///     return r + acc;
/// }
/// ```
///
/// `RUN_LOOP` statically unrolls exactly `REPEAT` steps over indices
/// `0..REPEAT` (the C `REP<REPEAT>` macro).
#[unsafe(no_mangle)]
pub extern "C" fn helper_call(a: c_int, b: c_int) -> c_int {
    let r = op_fn(a, b);
    let mut acc = INIT_FOR;
    let mut i: c_int = 0;
    while i < REPEAT {
        acc = step(acc, i);
        i += 1;
    }
    println!("helper.call={} helper.acc={}", r, acc);
    r.wrapping_add(acc)
}

/// ```c
/// int helper_ptr(int a, int b) {
///     int (*fp)(int,int) = OP_FN(OP);
///     int r = fp(a, b);
///     printf("helper.ptr=%d\n", r);
///     return r;
/// }
/// ```
///
/// `OP_FN(OP)` token-pastes to `op_<OP>` **directly**; it deliberately does not
/// read the mutable global `G_OP`, so overwriting `G_OP` from outside the
/// library must leave `helper_ptr`'s result unchanged.
#[unsafe(no_mangle)]
pub extern "C" fn helper_ptr(a: c_int, b: c_int) -> c_int {
    let fp: extern "C" fn(c_int, c_int) -> c_int = if cfg!(feature = "mul") {
        op_mul
    } else if cfg!(feature = "sub") {
        op_sub
    } else {
        op_add
    };
    let r = fp(a, b);
    println!("helper.ptr={}", r);
    r
}

/// ```c
/// int use_generated(int n) {
///     int r = (ACCUM_FN(OP))(n);
///     printf("gen.acc=%d\n", r);
///     return r;
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn use_generated(n: c_int) -> c_int {
    let r = accum(n);
    println!("gen.acc={}", r);
    r
}
