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
/// The C globals `G_OP` and `G_OP_NAME` are mutable (the pointer itself is
/// non-const), so in the C `.so` they land in the writable `.data` section and
/// an external consumer can store through the `dlsym` address. Declaring them
/// with `static mut` makes rustc emit them into `.data` too. An immutable
/// `static` would instead go into `.data.rel.ro`, which the loader maps
/// read-only after relocation (PT_GNU_RELRO), causing a consumer store to
/// SIGSEGV -- a divergence from the C `.so`.
#[unsafe(no_mangle)]
pub static mut G_OP: OpFn = mdmacros::OP_FN;

/// `const char *G_OP_NAME = STR(OP);`
///
/// Mutable like the C global (see `G_OP` above), so `static mut` places it in
/// the writable `.data` section rather than the read-only `.data.rel.ro`.
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
