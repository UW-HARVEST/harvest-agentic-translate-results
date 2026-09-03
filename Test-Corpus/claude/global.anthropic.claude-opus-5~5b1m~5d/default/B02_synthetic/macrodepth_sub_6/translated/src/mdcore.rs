// Translation of c_src/src/mdcore.c

use std::ffi::{c_char, c_int};
use std::io::Write;

use crate::mdconfig::{self, INIT, OP_NAME_C, REPEAT};

/* Define operations */

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

/* Define the macro-generated accumulator for the selected OP:
 *   DEFINE_ACCUM(OP) => static int accum_<OP>(int n)
 * It is `static` in C, so it stays private here. */
fn accum(n: c_int) -> c_int {
    let acc: c_int = INIT;
    mdconfig::dispatch_rep(acc, n)
}

/* Global macro uses at file scope (exercises expansion at global init) */

#[unsafe(no_mangle)]
pub static G_OP: extern "C" fn(c_int, c_int) -> c_int = mdconfig::op_fn();

#[repr(transparent)]
pub struct CStrPtr(pub *const c_char);
unsafe impl Sync for CStrPtr {}

#[unsafe(no_mangle)]
pub static G_OP_NAME: CStrPtr = CStrPtr(OP_NAME_C.as_ptr() as *const c_char);

/// stdout writer mirroring printf's behaviour closely enough for
/// byte-identical output.
fn out(s: &str) {
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    let _ = lock.write_all(s.as_bytes());
    let _ = lock.flush();
}

#[unsafe(no_mangle)]
pub extern "C" fn helper_call(a: c_int, b: c_int) -> c_int {
    let r = (mdconfig::op_fn())(a, b);
    let mut acc: c_int = INIT;
    acc = mdconfig::run_loop(acc);
    out(&format!("helper.call={} helper.acc={}\n", r, acc));
    r.wrapping_add(acc)
}

#[unsafe(no_mangle)]
pub extern "C" fn helper_ptr(a: c_int, b: c_int) -> c_int {
    let fp: extern "C" fn(c_int, c_int) -> c_int = mdconfig::op_fn();
    let r = fp(a, b);
    out(&format!("helper.ptr={}\n", r));
    r
}

#[unsafe(no_mangle)]
pub extern "C" fn use_generated(n: c_int) -> c_int {
    let r = accum(n);
    out(&format!("gen.acc={}\n", r));
    r
}

/// Re-export of the compile-time REPEAT value for the driver.
pub const REPEAT_VALUE: c_int = REPEAT;
