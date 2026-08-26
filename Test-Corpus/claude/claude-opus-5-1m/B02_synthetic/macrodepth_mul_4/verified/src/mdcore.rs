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

//! Rust translation of `src/mdcore.c`.

#![allow(dead_code)]

use std::ffi::{c_char, c_int};

use crate::mdmacros::{
    cstdio::printf, dispatch_rep, op_apply, op_name_ptr, run_loop_from_init, INIT, OP_NAME, REPEAT,
};

/* ---------------------------- Define operations --------------------------- */

/// `int op_add(int a,int b){ return a + b; }`
#[unsafe(no_mangle)]
pub extern "C" fn op_add(a: c_int, b: c_int) -> c_int {
    a.wrapping_add(b)
}

/// `int op_sub(int a,int b){ return a - b; }`
#[unsafe(no_mangle)]
pub extern "C" fn op_sub(a: c_int, b: c_int) -> c_int {
    a.wrapping_sub(b)
}

/// `int op_mul(int a,int b){ return a * b; }`
#[unsafe(no_mangle)]
pub extern "C" fn op_mul(a: c_int, b: c_int) -> c_int {
    a.wrapping_mul(b)
}

/* ------ The macro-generated accumulator for the selected OP ---------------
 * DEFINE_ACCUM(OP):
 *   static int accum_<OP>(int n) {
 *       int acc = INIT_FOR(OP);
 *       DISPATCH_REP(OP, acc, n);
 *       return acc;
 *   }
 * ------------------------------------------------------------------------- */
fn accum_op(n: c_int) -> c_int {
    let acc = INIT;
    dispatch_rep(acc, n)
}

/* ------------- Global macro uses at file scope (global init) -------------- */

/// C type of the `G_OP` global: `int (*)(int, int)`.
pub type OpFn = extern "C" fn(c_int, c_int) -> c_int;

/// The `op_<OP>` function chosen by `OP_FN(OP)`, as an `extern "C"` pointer.
#[cfg(feature = "mul")]
const SELECTED_OP: OpFn = op_mul;
#[cfg(all(feature = "sub", not(feature = "mul")))]
const SELECTED_OP: OpFn = op_sub;
#[cfg(all(not(feature = "sub"), not(feature = "mul")))]
const SELECTED_OP: OpFn = op_add;

/// `int (*G_OP)(int,int) = OP_FN(OP);`
///
/// This is a *mutable* global in C: `mdmacros.h` declares it as
/// `extern int (*G_OP)(int, int);` with no `const`, so it lives in a writable
/// `.data` section and a consumer of the shared library may legitimately
/// reassign it.  It is therefore translated as a `static mut`: an immutable
/// `static` would be placed in `.data.rel.ro`, which RELRO maps read-only,
/// making such a store segfault instead of succeeding as it does in C.
#[unsafe(no_mangle)]
pub static mut G_OP: OpFn = SELECTED_OP;

/// `const char *G_OP_NAME = STR(OP);`
///
/// The `const` applies to the pointed-to characters, not to the pointer itself,
/// so like `G_OP` this is a writable global in `.data`.
#[unsafe(no_mangle)]
pub static mut G_OP_NAME: *const c_char = op_name_ptr();

/* -------------------------------- Helpers -------------------------------- */

/// ```c
/// int helper_call(int a, int b) {
///     int r = (OP_FN(OP))(a, b);
///     int acc = INIT_FOR(OP);
///     RUN_LOOP(OP, acc, REPEAT);
///     printf("helper.call=%d helper.acc=%d\n", r, acc);
///     return r + acc;
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn helper_call(a: c_int, b: c_int) -> c_int {
    let r = op_apply(a, b);
    let acc = run_loop_from_init();
    unsafe {
        printf(
            c"helper.call=%d helper.acc=%d\n".as_ptr(),
            r as c_int,
            acc as c_int,
        );
    }
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
#[unsafe(no_mangle)]
pub extern "C" fn helper_ptr(a: c_int, b: c_int) -> c_int {
    let fp: OpFn = SELECTED_OP;
    let r = fp(a, b);
    unsafe {
        printf(c"helper.ptr=%d\n".as_ptr(), r as c_int);
    }
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
    let r = accum_op(n);
    unsafe {
        printf(c"gen.acc=%d\n".as_ptr(), r as c_int);
    }
    r
}

/* -- Convenience accessors mirroring the header's `extern` declarations. --- */

/// The name of the selected operation (`G_OP_NAME` as a Rust `&str`).
pub const fn op_name() -> &'static str {
    OP_NAME
}

/// The compile-time `REPEAT` value.
pub const fn repeat() -> c_int {
    REPEAT
}
