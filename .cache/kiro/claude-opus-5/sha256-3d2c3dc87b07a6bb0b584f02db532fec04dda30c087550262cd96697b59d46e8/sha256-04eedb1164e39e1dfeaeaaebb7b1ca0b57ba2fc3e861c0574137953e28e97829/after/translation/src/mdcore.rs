//! Translation of `c_src/src/mdcore.c`.
//!
//! Every function that has external linkage in the C file keeps its exact
//! linker symbol (`op_add`, `op_sub`, `op_mul`, `helper_call`, `helper_ptr`,
//! `use_generated`) and the C ABI. `mdmacros.h` declares no namespace/renaming
//! macros, so the source-level names are already the final symbols.
//!
//! `accum_<OP>` is `static` in C and therefore stays private here.

use core::ffi::{c_char, c_int};

use crate::mdmacros::{INIT, OP_NAME, dispatch_rep, run_loop};

/* ---------- Define operations ----------
 * int op_add(int a,int b){ return a + b; }
 * int op_sub(int a,int b){ return a - b; }
 * int op_mul(int a,int b){ return a * b; }
 */

#[unsafe(no_mangle)]
pub extern "C" fn op_add(a: c_int, b: c_int) -> c_int {
    a.wrapping_add(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn op_sub(a: c_int, b: c_int) -> c_int {
    a.wrapping_sub(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn op_mul(a: c_int, b: c_int) -> c_int {
    a.wrapping_mul(b)
}

/// `OP_FN(OP)` — the operation picked by the build configuration.
#[cfg(feature = "mul")]
pub const OP_FN: extern "C" fn(c_int, c_int) -> c_int = op_mul;
#[cfg(all(not(feature = "mul"), feature = "sub"))]
pub const OP_FN: extern "C" fn(c_int, c_int) -> c_int = op_sub;
#[cfg(all(not(feature = "mul"), not(feature = "sub")))]
pub const OP_FN: extern "C" fn(c_int, c_int) -> c_int = op_add;

/* ---------- DEFINE_ACCUM(OP) ----------
 * static int accum_<OP>(int n) {
 *   int acc = INIT_FOR(op);
 *   DISPATCH_REP(op, acc, n);
 *   return acc;
 * }
 */

/// `accum_<OP>` — `static` in the C translation unit, so private here.
fn accum_op(n: c_int) -> c_int {
    let acc = INIT;
    dispatch_rep(acc, n)
}

/* ---------- Global macro uses at file scope ----------
 * int (*G_OP)(int,int) = OP_FN(OP);
 * const char *G_OP_NAME = STR(OP);
 */

#[unsafe(no_mangle)]
pub static G_OP: extern "C" fn(c_int, c_int) -> c_int = OP_FN;

/// Newtype so a `*const c_char` can live in a `static`; `repr(transparent)`
/// keeps the exported symbol a plain pointer, matching `const char *G_OP_NAME`.
#[repr(transparent)]
pub struct OpNamePtr(pub *const c_char);

// The pointer targets a `'static` string constant and is never written.
unsafe impl Sync for OpNamePtr {}

#[unsafe(no_mangle)]
pub static G_OP_NAME: OpNamePtr = OpNamePtr(OP_NAME.as_ptr());

/* ---------- Helpers ---------- */

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
    let r = OP_FN(a, b);
    let acc = run_loop(INIT);
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
#[unsafe(no_mangle)]
pub extern "C" fn helper_ptr(a: c_int, b: c_int) -> c_int {
    let fp: extern "C" fn(c_int, c_int) -> c_int = OP_FN;
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
    let r = accum_op(n);
    println!("gen.acc={}", r);
    r
}
