// Translation of c_src/src/mdcore.c

use std::ffi::{c_char, c_int};

use crate::mdmacros;

/// `int (*)(int, int)` -- the operation function pointer type.
pub type OpFn = extern "C" fn(c_int, c_int) -> c_int;

/* ---------- Define operations ---------- */

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

/* ---------- Global macro uses at file scope ---------- */

/// `int (*G_OP)(int,int) = OP_FN(OP);`
///
/// The C global is a plain (non-`const`) object in `.data`, so an external
/// caller may `dlsym("G_OP")` and overwrite it. `static mut` is used here (not
/// a plain `static`) precisely so the symbol also lands in a writable section
/// instead of `.data.rel.ro`; an immutable Rust `static` would be mapped
/// read-only by RELRO and such a store would fault, diverging from C.
#[unsafe(no_mangle)]
pub static mut G_OP: OpFn = mdmacros::OP_FN;

/// `const char *G_OP_NAME = STR(OP);`
///
/// The pointee is `const char`, but the pointer object itself is not `const`
/// in C, so it lives in writable `.data` as well -- same reasoning as `G_OP`.
#[unsafe(no_mangle)]
pub static mut G_OP_NAME: *const c_char = mdmacros::OP_NAME.as_ptr() as *const c_char;

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
    let r = (mdmacros::OP_FN)(a, b);
    let acc = mdmacros::run_loop(mdmacros::INIT);
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
    let fp: OpFn = mdmacros::OP_FN;
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
    let r = mdmacros::accum(n);
    println!("gen.acc={}", r);
    r
}
