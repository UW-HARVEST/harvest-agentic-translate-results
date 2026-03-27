use libloading::{Library, Symbol};
use std::ffi::{c_int, CStr};
use std::os::raw::c_char;

fn c_lib() -> Library {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/c_src/build/libdriver.so"
    );
    unsafe { Library::new(path).expect("Failed to load C shared library") }
}

// ── Level 0: leaf operations ────────────────────────────────────────

#[test]
fn test_op_add() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int) -> c_int> =
        unsafe { lib.get(b"op_add").unwrap() };
    for (a, b) in [(0, 0), (3, 7), (-1, 1), (100, -200), (i32::MAX, 0)] {
        let c_res = unsafe { c_fn(a, b) };
        let r_res = macrodepth::op_add(a, b);
        assert_eq!(c_res, r_res, "op_add({a}, {b})");
    }
}

#[test]
fn test_op_sub() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int) -> c_int> =
        unsafe { lib.get(b"op_sub").unwrap() };
    for (a, b) in [(0, 0), (10, 3), (-1, -1), (0, 100)] {
        let c_res = unsafe { c_fn(a, b) };
        let r_res = macrodepth::op_sub(a, b);
        assert_eq!(c_res, r_res, "op_sub({a}, {b})");
    }
}

#[test]
fn test_op_mul() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int) -> c_int> =
        unsafe { lib.get(b"op_mul").unwrap() };
    for (a, b) in [(0, 0), (3, 7), (-2, 5), (1, 1)] {
        let c_res = unsafe { c_fn(a, b) };
        let r_res = macrodepth::op_mul(a, b);
        assert_eq!(c_res, r_res, "op_mul({a}, {b})");
    }
}

// ── Level 1: helper functions (return values only) ──────────────────

#[test]
fn test_helper_call_return() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int) -> c_int> =
        unsafe { lib.get(b"helper_call").unwrap() };
    macrodepth::init_globals();
    for (a, b) in [(3, 7), (0, 0), (-5, 10)] {
        let c_res = unsafe { c_fn(a, b) };
        let r_res = macrodepth::helper_call(a, b);
        assert_eq!(c_res, r_res, "helper_call({a}, {b})");
    }
}

#[test]
fn test_helper_ptr_return() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int) -> c_int> =
        unsafe { lib.get(b"helper_ptr").unwrap() };
    macrodepth::init_globals();
    for (a, b) in [(3, 7), (0, 0), (-5, 10)] {
        let c_res = unsafe { c_fn(a, b) };
        let r_res = macrodepth::helper_ptr(a, b);
        assert_eq!(c_res, r_res, "helper_ptr({a}, {b})");
    }
}

#[test]
fn test_use_generated_return() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
        unsafe { lib.get(b"use_generated").unwrap() };
    for n in 0..=6 {
        let c_res = unsafe { c_fn(n) };
        let r_res = macrodepth::use_generated(n);
        assert_eq!(c_res, r_res, "use_generated({n})");
    }
}

// ── Level 2: globals ────────────────────────────────────────────────

#[test]
fn test_g_op_name() {
    let lib = c_lib();
    let c_ptr: Symbol<*const *const c_char> =
        unsafe { lib.get(b"G_OP_NAME").unwrap() };
    let c_str = unsafe { CStr::from_ptr(**c_ptr) };
    macrodepth::init_globals();
    let r_str = unsafe { CStr::from_ptr(macrodepth::G_OP_NAME) };
    assert_eq!(c_str, r_str, "G_OP_NAME");
}

#[test]
fn test_g_op_fn_ptr() {
    let lib = c_lib();
    // G_OP in C is int(*)(int,int) — a function pointer stored as data
    let c_g_op: Symbol<*const Option<extern "C" fn(c_int, c_int) -> c_int>> =
        unsafe { lib.get(b"G_OP").unwrap() };
    let c_fn = unsafe { (**c_g_op).unwrap() };
    macrodepth::init_globals();
    let r_fn = unsafe { macrodepth::G_OP.unwrap() };
    for (a, b) in [(3, 7), (0, 0), (-1, 1)] {
        let c_res = unsafe { c_fn(a, b) };
        let r_res = r_fn(a, b);
        assert_eq!(c_res, r_res, "G_OP({a}, {b})");
    }
}
