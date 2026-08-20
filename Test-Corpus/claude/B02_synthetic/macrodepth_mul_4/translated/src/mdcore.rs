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

use std::ffi::{c_char, c_int};

use crate::mdmacros::{dispatch_rep, init_for_op, run_loop, Op, OP, OP_NAME_CSTR};

/* ---------- Define operations (C: op_add / op_sub / op_mul) ---------- */

/// C: `int op_add(int a,int b){ return a + b; }`
#[unsafe(no_mangle)]
pub extern "C" fn op_add(a: c_int, b: c_int) -> c_int {
    a.wrapping_add(b)
}

/// C: `int op_sub(int a,int b){ return a - b; }`
#[unsafe(no_mangle)]
pub extern "C" fn op_sub(a: c_int, b: c_int) -> c_int {
    a.wrapping_sub(b)
}

/// C: `int op_mul(int a,int b){ return a * b; }`
#[unsafe(no_mangle)]
pub extern "C" fn op_mul(a: c_int, b: c_int) -> c_int {
    a.wrapping_mul(b)
}

/// C: `OP_FN(OP)` -- the `op_<OP>` function picked by the build configuration.
pub const OP_FN: extern "C" fn(c_int, c_int) -> c_int = match OP {
    Op::Add => op_add,
    Op::Sub => op_sub,
    Op::Mul => op_mul,
};

/* ---------- Macro-generated accumulator for the selected OP ----------
 * C: DEFINE_ACCUM(OP) expands to
 *      static int accum_<OP>(int n) {
 *          int acc = INIT_FOR(OP);
 *          DISPATCH_REP(OP, acc, n);
 *          return acc;
 *      }
 * Note that DISPATCH_REP only handles n in 0..=6; anything else (7, negatives)
 * falls through `default: break;` and returns the untouched initial value.
 */
fn accum_op(n: c_int) -> c_int {
    let acc = init_for_op();
    dispatch_rep(acc, n)
}

/* ---------- Global macro uses at file scope ---------- */

/// C: `int (*G_OP)(int,int) = OP_FN(OP);`
#[unsafe(no_mangle)]
pub static G_OP: extern "C" fn(c_int, c_int) -> c_int = OP_FN;

/// Wrapper making a raw `const char *` usable as an exported static.
#[repr(transparent)]
pub struct OpNamePtr(pub *const c_char);
// The pointer targets a `'static` read-only string, so sharing it is sound.
unsafe impl Sync for OpNamePtr {}

/// C: `const char *G_OP_NAME = STR(OP);`
#[unsafe(no_mangle)]
pub static G_OP_NAME: OpNamePtr = OpNamePtr(OP_NAME_CSTR.as_ptr() as *const c_char);

/* ---------- Helpers ---------- */

/// C:
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
    let r = (OP_FN)(a, b);
    let acc = run_loop(init_for_op());
    print!("helper.call={} helper.acc={}\n", r, acc);
    r.wrapping_add(acc)
}

/// C:
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
    let fp: extern "C" fn(c_int, c_int) -> c_int = OP_FN;
    let r = fp(a, b);
    print!("helper.ptr={}\n", r);
    r
}

/// C:
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
    print!("gen.acc={}\n", r);
    r
}
