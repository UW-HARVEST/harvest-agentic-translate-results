// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust. Behavior is preserved exactly.

use std::ffi::c_char;
use std::os::raw::c_int;

// ----- Operation family (always defined) -----

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

// ----- Determine selected OP via cargo features -----

#[cfg(feature = "add")]
const OP_NAME_BYTES: &[u8] = b"add\0";
#[cfg(feature = "sub")]
const OP_NAME_BYTES: &[u8] = b"sub\0";
#[cfg(feature = "mul")]
const OP_NAME_BYTES: &[u8] = b"mul\0";

#[cfg(feature = "add")]
fn selected_op_fn() -> extern "C" fn(c_int, c_int) -> c_int {
    op_add
}
#[cfg(feature = "sub")]
fn selected_op_fn() -> extern "C" fn(c_int, c_int) -> c_int {
    op_sub
}
#[cfg(feature = "mul")]
fn selected_op_fn() -> extern "C" fn(c_int, c_int) -> c_int {
    op_mul
}

// ----- Determine REPEAT value via cargo features -----

#[cfg(feature = "0")]
pub const REPEAT: c_int = 0;
#[cfg(feature = "1")]
pub const REPEAT: c_int = 1;
#[cfg(feature = "2")]
pub const REPEAT: c_int = 2;
#[cfg(feature = "3")]
pub const REPEAT: c_int = 3;
#[cfg(feature = "4")]
pub const REPEAT: c_int = 4;
#[cfg(feature = "5")]
pub const REPEAT: c_int = 5;
#[cfg(feature = "6")]
pub const REPEAT: c_int = 6;
#[cfg(feature = "7")]
pub const REPEAT: c_int = 7;

// ----- Step / init operations corresponding to STEP_<op> and INIT_<op> -----

#[inline(always)]
fn step(acc: &mut c_int, i: c_int) {
    #[cfg(feature = "add")]
    {
        *acc = acc.wrapping_add(i);
    }
    #[cfg(feature = "sub")]
    {
        *acc = acc.wrapping_sub(i);
    }
    #[cfg(feature = "mul")]
    {
        *acc = acc.wrapping_mul(i.wrapping_add(1));
    }
}

#[inline(always)]
const fn init_for() -> c_int {
    #[cfg(feature = "add")]
    {
        0
    }
    #[cfg(feature = "sub")]
    {
        0
    }
    #[cfg(feature = "mul")]
    {
        1
    }
}

/// Equivalent to RUN_LOOP(OP, acc, REPEAT) — manually unrolled REPEAT iterations.
#[inline(always)]
fn run_loop(acc: &mut c_int) {
    // The CHOOSE_REP(REPEAT) macro expands to REP<REPEAT>(op, acc).
    // REPn(op, acc) = STEP_OP(op, acc, 0); STEP_OP(op, acc, 1); ... STEP_OP(op, acc, n-1);
    let n: c_int = REPEAT;
    let mut i: c_int = 0;
    while i < n {
        step(acc, i);
        i += 1;
    }
}

/// Equivalent to the macro-generated `static int accum_<OP>(int n)`.
/// DISPATCH_REP performs a switch on n where each case n=k expands REPk(op, acc).
/// The default branch does nothing, leaving acc at its initial value.
fn accum_op(n: c_int) -> c_int {
    let mut acc: c_int = init_for();
    // The C switch only has cases 0..=6; default does nothing.
    if (0..=6).contains(&n) {
        let mut i: c_int = 0;
        while i < n {
            step(&mut acc, i);
            i += 1;
        }
    }
    acc
}

// ----- Globals (G_OP and G_OP_NAME) -----
//
// In C: `int (*G_OP)(int,int) = OP_FN(OP);` — a mutable global function pointer.
// `const char *G_OP_NAME = STR(OP);` — a mutable global pointer to a static string.
//
// Both end up at file scope, exported with their unmangled names. We mirror
// that with mutable statics initialized at startup. Because Rust does not
// allow casting a `fn` to a function pointer in a `static` initializer, we
// use a constructor to populate them once at library/program load.

pub type OpFnPtr = extern "C" fn(c_int, c_int) -> c_int;

#[unsafe(no_mangle)]
pub static mut G_OP: Option<OpFnPtr> = None;

#[unsafe(no_mangle)]
pub static mut G_OP_NAME: *const c_char = std::ptr::null();

/// Initializer that runs before `main` (mirrors the C global initializers).
#[used]
#[cfg_attr(any(target_os = "linux", target_os = "android"), link_section = ".init_array")]
#[cfg_attr(target_os = "macos", link_section = "__DATA,__mod_init_func")]
#[cfg_attr(target_os = "windows", link_section = ".CRT$XCU")]
static INIT_GLOBALS: extern "C" fn() = {
    extern "C" fn init() {
        unsafe {
            G_OP = Some(selected_op_fn());
            G_OP_NAME = OP_NAME_BYTES.as_ptr() as *const c_char;
        }
    }
    init
};

// ----- Helper functions exported from the library -----

#[unsafe(no_mangle)]
pub extern "C" fn helper_call(a: c_int, b: c_int) -> c_int {
    let r: c_int = (selected_op_fn())(a, b);
    let mut acc: c_int = init_for();
    run_loop(&mut acc);
    // printf("helper.call=%d helper.acc=%d\n", r, acc);
    println!("helper.call={} helper.acc={}", r, acc);
    r.wrapping_add(acc)
}

#[unsafe(no_mangle)]
pub extern "C" fn helper_ptr(a: c_int, b: c_int) -> c_int {
    let fp: OpFnPtr = selected_op_fn();
    let r: c_int = fp(a, b);
    println!("helper.ptr={}", r);
    r
}

#[unsafe(no_mangle)]
pub extern "C" fn use_generated(n: c_int) -> c_int {
    let r: c_int = accum_op(n);
    println!("gen.acc={}", r);
    r
}

// ----- Re-exports for the binary driver -----

pub fn driver_run_loop(acc: &mut c_int) {
    run_loop(acc);
}

pub fn driver_init_for() -> c_int {
    init_for()
}

pub fn driver_selected_op() -> OpFnPtr {
    selected_op_fn()
}
