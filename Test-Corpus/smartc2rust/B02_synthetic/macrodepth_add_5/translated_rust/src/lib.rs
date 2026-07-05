
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

// ---------------------------------------------------------------------------
// Global variables shared with the C side via FFI linkage.
// The C definitions live in mdcore.c:
//     int (*G_OP)(int,int)   = op_add;
//     const char *G_OP_NAME  = "add";
// ---------------------------------------------------------------------------
unsafe extern "C" {
    static mut G_OP: Option<extern "C" fn(c_int, c_int) -> c_int>;
    static mut G_OP_NAME: *const c_char;
}

pub fn rust_get_G_OP() -> Option<extern "C" fn(c_int, c_int) -> c_int> {
    unsafe { G_OP }
}

pub fn rust_set_G_OP(val: Option<extern "C" fn(c_int, c_int) -> c_int>) {
    unsafe { G_OP = val; }
}

pub fn rust_get_G_OP_NAME() -> *const c_char {
    unsafe { G_OP_NAME }
}

pub fn rust_set_G_OP_NAME(val: *const c_char) {
    unsafe { G_OP_NAME = val; }
}

// ---------------------------------------------------------------------------
// Operation family (OP = add). Only op_add is required for OP=add.
// ---------------------------------------------------------------------------
fn rust_op_add(a: i32, b: i32) -> i32 {
    a.wrapping_add(b)
}

// STEP_add(acc, i) => acc += i
fn rust_step_add(acc: &mut i32, i: i32) {
    *acc = acc.wrapping_add(i);
}

// RUN_LOOP(add, acc, REPEAT=5) expands to REP5(add, acc):
//   STEP_add(acc, 0..4)
fn rust_run_loop_add(acc: &mut i32) {
    for i in 0..5 {
        rust_step_add(acc, i);
    }
}

// DISPATCH_REP for OP=add. Corresponds to switch(n) { case k: REPk(add, acc); }
fn rust_dispatch_rep_add(acc: &mut i32, n: i32) {
    let count = match n {
        0..=6 => n,
        _ => return,
    };
    for i in 0..count {
        rust_step_add(acc, i);
    }
}

// DEFINE_ACCUM(add) => static int accum_add(int n) { int acc = INIT_add; DISPATCH_REP(add, acc, n); return acc; }
fn rust_accum_add(n: i32) -> i32 {
    // INIT_add is an independent constant macro provided by bindgen.
    let mut acc: i32 = INIT_add as i32;
    rust_dispatch_rep_add(&mut acc, n);
    acc
}

// ---------------------------------------------------------------------------
// Helpers translated from mdcore.c
// ---------------------------------------------------------------------------
fn rust_helper_call(a: i32, b: i32) -> i32 {
    let r = rust_op_add(a, b);
    let mut acc: i32 = INIT_add as i32;
    rust_run_loop_add(&mut acc);
    println!("helper.call={} helper.acc={}", r, acc);
    r.wrapping_add(acc)
}

fn rust_helper_ptr(a: i32, b: i32) -> i32 {
    let fp: fn(i32, i32) -> i32 = rust_op_add;
    let r = fp(a, b);
    println!("helper.ptr={}", r);
    r
}

fn rust_use_generated(n: i32) -> i32 {
    let r = rust_accum_add(n);
    println!("gen.acc={}", r);
    r
}

// ---------------------------------------------------------------------------
// FFI boundary: replacement for C's `main`.
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub extern "C" fn mdmain_main(argc: c_int, argv: *mut *mut c_char) -> c_int {
    // Safely materialize argv into Vec<String>.
    let args: Vec<String> = if argv.is_null() || argc <= 0 {
        Vec::new()
    } else {
        (0..argc as isize)
            .map(|i| {
                let p = unsafe { *argv.offset(i) };
                if p.is_null() {
                    String::new()
                } else {
                    unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
                }
            })
            .collect()
    };

    if argc < 3 {
        let prog = args.get(0).map(String::as_str).unwrap_or("");
        eprintln!("usage: {} A B", prog);
        return 2;
    }

    let a: i32 = args[1].trim().parse().unwrap_or(0);
    let b: i32 = args[2].trim().parse().unwrap_or(0);

    // r_call = (OP_FN(OP))(a, b);
    let r_call = rust_op_add(a, b);

    // int acc = INIT_FOR(OP); RUN_LOOP(OP, acc, REPEAT);
    let mut acc: i32 = INIT_add as i32;
    rust_run_loop_add(&mut acc);

    let x1 = rust_helper_call(a, b);
    let x2 = rust_helper_ptr(a, b);
    let x3 = rust_use_generated(5); // REPEAT = 5

    // g = G_OP(a, b);  — call through the shared function pointer.
    let g = match rust_get_G_OP() {
        Some(fp) => fp(a as c_int, b as c_int) as i32,
        None => 0,
    };

    // Read G_OP_NAME from the shared C string.
    let g_op_name: String = {
        let p = rust_get_G_OP_NAME();
        if p.is_null() {
            String::new()
        } else {
            unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
        }
    };

    println!("op={} call={} acc={} g.call={}", g_op_name, r_call, acc, g);
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

