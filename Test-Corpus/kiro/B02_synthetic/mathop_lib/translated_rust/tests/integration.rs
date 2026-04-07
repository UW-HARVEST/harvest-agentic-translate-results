use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::os::raw::c_char;

const C_LIB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libtranslated_rust.so");

fn rust_lib_path() -> String {
    let dir = env!("CARGO_MANIFEST_DIR");
    format!("{}/target/debug/libmathop_lib.so", dir)
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct ComputationResult {
    value: c_int,
    timestamp: i64,
    status: c_int,
}

// ---- Lowest-level: arithmetic operations ----

#[test]
fn test_add_operation() {
    unsafe {
        let c = Library::new(C_LIB).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int) -> c_int> = c.get(b"add_operation").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int) -> c_int> = r.get(b"add_operation").unwrap();
        for &(a, b) in &[(0,0),(1,2),(-1,1),(i32::MAX,0),(i32::MIN,0),(100,-200)] {
            assert_eq!(c_fn(a, b, 0), r_fn(a, b, 0), "add_operation({a},{b},0)");
        }
    }
}

#[test]
fn test_subtract_operation() {
    unsafe {
        let c = Library::new(C_LIB).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int) -> c_int> = c.get(b"subtract_operation").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int) -> c_int> = r.get(b"subtract_operation").unwrap();
        for &(a, b) in &[(0,0),(5,3),(3,5),(-1,-1),(i32::MAX,1),(i32::MIN,-1)] {
            assert_eq!(c_fn(a, b, 0), r_fn(a, b, 0), "subtract_operation({a},{b},0)");
        }
    }
}

#[test]
fn test_multiply_operation() {
    unsafe {
        let c = Library::new(C_LIB).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int) -> c_int> = c.get(b"multiply_operation").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int) -> c_int> = r.get(b"multiply_operation").unwrap();
        for &(a, b) in &[(0,0),(1,1),(2,3),(-1,5),(0,i32::MAX)] {
            assert_eq!(c_fn(a, b, 0), r_fn(a, b, 0), "multiply_operation({a},{b},0)");
        }
    }
}

#[test]
fn test_divide_operation() {
    unsafe {
        let c = Library::new(C_LIB).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int) -> c_int> = c.get(b"divide_operation").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int) -> c_int> = r.get(b"divide_operation").unwrap();
        for &(a, b) in &[(0,1),(10,3),(10,-3),(-10,3),(7,2),(1,0),(0,0)] {
            assert_eq!(c_fn(a, b, 0), r_fn(a, b, 0), "divide_operation({a},{b},0)");
        }
    }
}

#[test]
fn test_modulo_operation() {
    unsafe {
        let c = Library::new(C_LIB).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int) -> c_int> = c.get(b"modulo_operation").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int) -> c_int> = r.get(b"modulo_operation").unwrap();
        for &(a, b) in &[(0,1),(10,3),(10,-3),(-10,3),(7,2),(1,0),(0,0)] {
            assert_eq!(c_fn(a, b, 0), r_fn(a, b, 0), "modulo_operation({a},{b},0)");
        }
    }
}

// ---- is_valid_operation ----

#[test]
fn test_is_valid_operation() {
    unsafe {
        let c = Library::new(C_LIB).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(c_char) -> bool> = c.get(b"is_valid_operation").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_char) -> bool> = r.get(b"is_valid_operation").unwrap();
        // Test all possible char values
        for i in -128i8..=127i8 {
            assert_eq!(c_fn(i), r_fn(i), "is_valid_operation({i})");
        }
    }
}

// ---- get_operation_priority ----

#[test]
fn test_get_operation_priority() {
    unsafe {
        let c = Library::new(C_LIB).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(c_int) -> c_int> = c.get(b"get_operation_priority").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int) -> c_int> = r.get(b"get_operation_priority").unwrap();
        for op in 0..=6 {
            assert_eq!(c_fn(op), r_fn(op), "get_operation_priority({op})");
        }
    }
}

// ---- get_computation_timestamp ----

#[test]
fn test_get_computation_timestamp() {
    unsafe {
        let c = Library::new(C_LIB).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn() -> i64> = c.get(b"get_computation_timestamp").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn() -> i64> = r.get(b"get_computation_timestamp").unwrap();
        let cv = c_fn();
        let rv = r_fn();
        assert_eq!(cv, rv, "get_computation_timestamp: C={cv} Rust={rv}");
    }
}

// ---- select_operation (test indirectly by calling returned fn ptr) ----

#[test]
fn test_select_operation() {
    type MathOp = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;
    unsafe {
        let c = Library::new(C_LIB).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();
        let c_sel: Symbol<unsafe extern "C" fn(c_int) -> MathOp> = c.get(b"select_operation").unwrap();
        let r_sel: Symbol<unsafe extern "C" fn(c_int) -> MathOp> = r.get(b"select_operation").unwrap();
        // ops 1-5 plus default (0, 6)
        for op in 0..=6 {
            let c_op = c_sel(op);
            let r_op = r_sel(op);
            for &(a, b) in &[(10, 3), (0, 0), (-5, 2), (7, 1)] {
                assert_eq!(c_op(a, b, 0), r_op(a, b, 0), "select_operation({op})({a},{b},0)");
            }
        }
    }
}

// ---- allocate_results ----

#[test]
fn test_allocate_results() {
    unsafe {
        let c = Library::new(C_LIB).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(c_int) -> *mut ComputationResult> = c.get(b"allocate_results").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int) -> *mut ComputationResult> = r.get(b"allocate_results").unwrap();
        let cp = c_fn(5);
        let rp = r_fn(5);
        assert!(!cp.is_null());
        assert!(!rp.is_null());
        // calloc zeroes memory — check first element
        assert_eq!((*cp).value, (*rp).value);
        assert_eq!((*cp).timestamp, (*rp).timestamp);
        assert_eq!((*cp).status, (*rp).status);
        libc_free(cp as *mut u8);
        libc_free(rp as *mut u8);
    }
}

unsafe fn libc_free(p: *mut u8) {
    extern "C" { fn free(p: *mut u8); }
    unsafe { free(p); }
}

// ---- perform_computation_with_history ----

#[test]
fn test_perform_computation_with_history() {
    unsafe {
        let c = Library::new(C_LIB).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, *mut *mut ComputationResult, *mut c_int) -> c_int> =
            c.get(b"perform_computation_with_history").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, *mut *mut ComputationResult, *mut c_int) -> c_int> =
            r.get(b"perform_computation_with_history").unwrap();

        let mut c_hist: *mut ComputationResult = std::ptr::null_mut();
        let mut c_count: c_int = 0;
        let mut r_hist: *mut ComputationResult = std::ptr::null_mut();
        let mut r_count: c_int = 0;

        let inputs = [(10, 3, 1), (20, 4, 2), (7, 2, 3), (15, 5, 4), (9, 3, 5)];
        for &(a, b, op) in &inputs {
            let cv = c_fn(a, b, op, &mut c_hist, &mut c_count);
            let rv = r_fn(a, b, op, &mut r_hist, &mut r_count);
            assert_eq!(cv, rv, "perform_computation_with_history({a},{b},{op}) result");
            assert_eq!(c_count, r_count, "history count after ({a},{b},{op})");
        }

        // Compare history entries
        for i in 0..c_count as isize {
            let ce = &*c_hist.offset(i);
            let re = &*r_hist.offset(i);
            assert_eq!(ce.value, re.value, "history[{i}].value");
            assert_eq!(ce.timestamp, re.timestamp, "history[{i}].timestamp");
            assert_eq!(ce.status, re.status, "history[{i}].status");
        }

        libc_free(c_hist as *mut u8);
        libc_free(r_hist as *mut u8);
    }
}

// ---- mathop (top-level) ----
// Note: mathop uses static state and time(), so we call both with same args
// and compare. The >>29 shift makes timestamp stable within a call pair.

#[test]
fn test_mathop() {
    unsafe {
        let c = Library::new(C_LIB).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> = c.get(b"mathop").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> = r.get(b"mathop").unwrap();

        let inputs = [
            (1, 2, 3, 4),
            (10, 20, 30, 40),
            (0, 0, 0, 0),
            (-1, -2, -3, -4),
            (100, 200, 1, 1),
            (49, 50, 2, 3),  // 49 % 128 = 49 = '1', valid
        ];
        for &(a, b, c_arg, d) in &inputs {
            let cv = c_fn(a, b, c_arg, d);
            let rv = r_fn(a, b, c_arg, d);
            assert_eq!(cv, rv, "mathop({a},{b},{c_arg},{d}): C={cv} Rust={rv}");
        }
    }
}
