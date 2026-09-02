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

// Both C globals are *mutable* objects with external linkage:
//
//     int (*G_OP)(int,int) = OP_FN(OP);   /* the pointer is mutable      */
//     const char *G_OP_NAME = STR(OP);    /* `const` binds to the pointee,
//                                            the pointer itself is mutable */
//
// They therefore live in a writable `.data` section that is *outside* the
// `PT_GNU_RELRO` segment, and an external consumer may assign to them.
//
// A plain Rust `static` is immutable, so rustc emits it into `.data.rel.ro`,
// which the loader `mprotect`s read-only once relocations are applied — a
// consumer store would then `SIGSEGV`. `static mut` reproduces the C storage
// class exactly and keeps the objects in writable `.data`.
// See tests/error_paths.rs::e11_g_op_is_writable_data.

/// `int (*G_OP)(int,int) = OP_FN(OP);`
#[unsafe(no_mangle)]
pub static mut G_OP: OpFn = mdmacros::OP_FN;

/// `const char *G_OP_NAME = STR(OP);`
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
