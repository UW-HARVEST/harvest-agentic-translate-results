
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

// ---------------------------------------------------------------------------
// Global variables shared with C via FFI (G_OP, G_OP_NAME).
// The C side owns the definitions; we only declare and provide safe accessors.
// ---------------------------------------------------------------------------
type OpFn = unsafe extern "C" fn(c_int, c_int) -> c_int;

unsafe extern "C" {
    static mut G_OP: Option<OpFn>;
    static mut G_OP_NAME: *const c_char;
}

pub fn rust_get_G_OP() -> Option<OpFn> {
    unsafe { G_OP }
}

pub fn rust_set_G_OP(val: Option<OpFn>) {
    unsafe { G_OP = val; }
}

pub fn rust_get_G_OP_NAME() -> *const c_char {
    unsafe { G_OP_NAME }
}

pub fn rust_set_G_OP_NAME(val: *const c_char) {
    unsafe { G_OP_NAME = val; }
}

// ---------------------------------------------------------------------------
// Pure-Rust reimplementation of the mul-family of operations.
// The macro-family in C selects `mul`, INIT is 1, REPEAT is 4.
// STEP_mul(acc, i)  =>  acc *= (i + 1)
// ---------------------------------------------------------------------------

/// C-side REPEAT value (compiled with -DREPEAT=4).
const REPEAT: i32 = 4;

/// Rust equivalent of C `op_mul`.
fn rust_op_mul(a: i32, b: i32) -> i32 {
    a.wrapping_mul(b)
}

/// Apply STEP_mul repeatedly for i in 0..n, using the fixed initial value INIT_mul.
fn rust_run_loop_mul(n: i32) -> i32 {
    let mut acc: i32 = INIT_mul as i32;
    for i in 0..n {
        acc = acc.wrapping_mul(i.wrapping_add(1));
    }
    acc
}

/// Rust equivalent of the macro-generated `accum_mul(n)`.
///
/// The C version dispatches on `n` via a switch (0..=6), doing nothing for
/// out-of-range values. We reproduce that semantics exactly.
fn rust_accum_mul(n: i32) -> i32 {
    match n {
        0..=6 => rust_run_loop_mul(n),
        _ => INIT_mul as i32,
    }
}

/// Rust equivalent of C `helper_call`.
fn rust_helper_call(a: i32, b: i32) -> i32 {
    let r = rust_op_mul(a, b);
    let acc = rust_run_loop_mul(REPEAT);
    println!("helper.call={} helper.acc={}", r, acc);
    r.wrapping_add(acc)
}

/// Rust equivalent of C `helper_ptr`.
fn rust_helper_ptr(a: i32, b: i32) -> i32 {
    let fp: fn(i32, i32) -> i32 = rust_op_mul;
    let r = fp(a, b);
    println!("helper.ptr={}", r);
    r
}

/// Rust equivalent of C `use_generated`.
fn rust_use_generated(n: i32) -> i32 {
    let r = rust_accum_mul(n);
    println!("gen.acc={}", r);
    r
}

// ---------------------------------------------------------------------------
// FFI boundary: replacement for C `main`, exposed as `mdmain_main`.
// The unsafe blocks below are limited to reading the C-provided `argv` array
// and the shared globals `G_OP` / `G_OP_NAME`, which is the minimum required
// at the FFI boundary.
// ---------------------------------------------------------------------------

/// Safely collect the program arguments from a C-style `argv`.
fn rust_collect_args(argc: c_int, argv: *mut *mut c_char) -> Vec<String> {
    if argv.is_null() || argc <= 0 {
        return Vec::new();
    }
    (0..argc as isize)
        .map(|i| unsafe {
            let p = *argv.offset(i);
            if p.is_null() {
                String::new()
            } else {
                CStr::from_ptr(p).to_string_lossy().into_owned()
            }
        })
        .collect()
}

#[unsafe(no_mangle)]
pub extern "C" fn mdmain_main(argc: c_int, argv: *mut *mut c_char) -> c_int {
    let args = rust_collect_args(argc, argv);

    if argc < 3 {
        let prog = args.first().map(String::as_str).unwrap_or("");
        eprintln!("usage: {} A B", prog);
        return 2;
    }

    let a: i32 = args[1].trim().parse().unwrap_or(0);
    let b: i32 = args[2].trim().parse().unwrap_or(0);

    let r_call = rust_op_mul(a, b);
    let acc = rust_run_loop_mul(REPEAT);

    let x1 = rust_helper_call(a, b);
    let x2 = rust_helper_ptr(a, b);
    let x3 = rust_use_generated(REPEAT);

    // Invoke the C-provided G_OP through its (safe) accessor. G_OP is set by
    // the C initializer to op_mul, so calling it must be done through an
    // unsafe extern "C" call.
    let g: i32 = match rust_get_G_OP() {
        Some(f) => unsafe { f(a, b) },
        None => 0,
    };

    // Read G_OP_NAME (a C string) through its safe accessor.
    let name_ptr = rust_get_G_OP_NAME();
    let g_op_name: String = if name_ptr.is_null() {
        String::from("")
    } else {
        unsafe { CStr::from_ptr(name_ptr) }
            .to_string_lossy()
            .into_owned()
    };

    println!(
        "op={} call={} acc={} g.call={}",
        g_op_name, r_call, acc, g
    );
    println!(
        "summary={}",
        r_call
            .wrapping_add(acc)
            .wrapping_add(x1)
            .wrapping_add(x2)
            .wrapping_add(x3)
            .wrapping_add(g)
    );
    0
}

