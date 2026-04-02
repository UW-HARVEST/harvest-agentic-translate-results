use libloading::{Library, Symbol};
use std::ffi::c_int;

fn c_lib() -> Library {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libmathop_lib.so");
    unsafe { Library::new(path).expect("Failed to load C .so") }
}

// ---- Leaf functions ----

#[test]
fn test_add_operation() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int) -> c_int> =
        unsafe { lib.get(b"add_operation").unwrap() };
    for (a, b, u) in [(0,0,0),(1,2,0),(-1,1,0),(i32::MAX,1,0),(i32::MIN,-1,0)] {
        assert_eq!(unsafe { c_fn(a,b,u) }, mathop_lib::add_operation(a,b,u), "add({a},{b},{u})");
    }
}

#[test]
fn test_multiply_operation() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int) -> c_int> =
        unsafe { lib.get(b"multiply_operation").unwrap() };
    for (a, b, u) in [(0,0,0),(3,4,0),(-2,3,0),(i32::MAX,2,0)] {
        assert_eq!(unsafe { c_fn(a,b,u) }, mathop_lib::multiply_operation(a,b,u), "mul({a},{b},{u})");
    }
}

#[test]
fn test_subtract_operation() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int) -> c_int> =
        unsafe { lib.get(b"subtract_operation").unwrap() };
    for (a, b, u) in [(0,0,0),(5,3,0),(-1,-1,0),(i32::MIN,1,0)] {
        assert_eq!(unsafe { c_fn(a,b,u) }, mathop_lib::subtract_operation(a,b,u), "sub({a},{b},{u})");
    }
}

#[test]
fn test_divide_operation() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int) -> c_int> =
        unsafe { lib.get(b"divide_operation").unwrap() };
    for (a, b, u) in [(10,3,0),(10,0,0),(-7,2,0),(0,5,0)] {
        assert_eq!(unsafe { c_fn(a,b,u) }, mathop_lib::divide_operation(a,b,u), "div({a},{b},{u})");
    }
}

#[test]
fn test_modulo_operation() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int) -> c_int> =
        unsafe { lib.get(b"modulo_operation").unwrap() };
    for (a, b, u) in [(10,3,0),(10,0,0),(-7,2,0),(7,-2,0),(0,5,0)] {
        assert_eq!(unsafe { c_fn(a,b,u) }, mathop_lib::modulo_operation(a,b,u), "mod({a},{b},{u})");
    }
}

#[test]
fn test_is_valid_operation() {
    let lib = c_lib();
    // C: bool is_valid_operation(char) where char is signed on Linux x86_64
    let c_fn: Symbol<unsafe extern "C" fn(i8) -> u8> =
        unsafe { lib.get(b"is_valid_operation").unwrap() };
    for v in [0i8, 1, 48, 49, 50, 51, 52, 53, 54, -1, -128, 127] {
        let c_res = unsafe { c_fn(v) } != 0;
        let r_res = mathop_lib::is_valid_operation(v);
        assert_eq!(c_res, r_res, "is_valid_operation({v})");
    }
}

#[test]
fn test_get_operation_priority() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
        unsafe { lib.get(b"get_operation_priority").unwrap() };
    for op in [1, 2, 3, 4, 5, 0, -1, 100] {
        assert_eq!(unsafe { c_fn(op) }, mathop_lib::get_operation_priority(op), "priority({op})");
    }
}

#[test]
fn test_get_computation_timestamp() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn() -> i64> =
        unsafe { lib.get(b"get_computation_timestamp").unwrap() };
    let c_res = unsafe { c_fn() };
    let r_res = mathop_lib::get_computation_timestamp();
    assert_eq!(c_res, r_res, "get_computation_timestamp");
}

// ---- Mid-level ----

#[test]
fn test_select_operation() {
    let lib = c_lib();
    let c_select: Symbol<unsafe extern "C" fn(c_int) -> unsafe extern "C" fn(c_int, c_int, c_int) -> c_int> =
        unsafe { lib.get(b"select_operation").unwrap() };
    for (op, a, b) in [(1,3,4),(2,3,4),(3,10,3),(4,10,3),(5,10,3),(0,3,4),(99,3,4)] {
        let c_res = unsafe { let f = c_select(op); f(a, b, 0) };
        let r_res = mathop_lib::select_operation(op)(a, b, 0);
        assert_eq!(c_res, r_res, "select_op({op})({a},{b},0)");
    }
}

// ---- Top-level ----

#[test]
fn test_mathop() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
        unsafe { lib.get(b"mathop").unwrap() };
    // Both use time() internally; timestamp >> 29 changes very slowly so results should match
    for (p1, p2, p3, p4) in [
        (1,2,3,4), (0,0,0,0), (10,20,30,40), (-1,5,2,3),
        (49,10,1,1), (100,200,4,5), (-128,1,0,0),
    ] {
        let c_res = unsafe { c_fn(p1, p2, p3, p4) };
        let r_res = mathop_lib::mathop(p1, p2, p3, p4);
        assert_eq!(c_res, r_res, "mathop({p1},{p2},{p3},{p4})");
    }
}
