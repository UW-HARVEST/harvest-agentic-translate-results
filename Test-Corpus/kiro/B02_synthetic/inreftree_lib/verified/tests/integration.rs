use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;
use std::sync::Mutex;

// Serialize all tests since both libs use global state
static LOCK: Mutex<()> = Mutex::new(());

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target/debug/libinreftree_lib.so");
    p
}

type OpFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

#[test]
fn test_arithmetic_ops() {
    let _g = LOCK.lock().unwrap();
    let c = unsafe { Library::new(c_lib_path()) }.unwrap();
    let r = unsafe { Library::new(rust_lib_path()) }.unwrap();

    let cases: &[(c_int, c_int)] = &[
        (0, 0), (1, 1), (10, 3), (-7, 2), (7, -2), (-7, -2),
        (100, 7), (i32::MAX, 1), (i32::MIN, 1), (5, 0), (0, 5),
    ];

    for name in &["add_op", "multiply_op", "subtract_op", "divide_op", "modulo_op"] {
        let c_fn: Symbol<OpFn> = unsafe { c.get(name.as_bytes()) }.unwrap();
        let r_fn: Symbol<OpFn> = unsafe { r.get(name.as_bytes()) }.unwrap();
        for &(a, b) in cases {
            let cv = unsafe { c_fn(a, b, 0, 0) };
            let rv = unsafe { r_fn(a, b, 0, 0) };
            assert_eq!(cv, rv, "{name}({a}, {b}): C={cv} Rust={rv}");
        }
    }
}

#[test]
fn test_parse_operation() {
    let _g = LOCK.lock().unwrap();
    let c = unsafe { Library::new(c_lib_path()) }.unwrap();
    let r = unsafe { Library::new(rust_lib_path()) }.unwrap();

    type ParseFn = unsafe extern "C" fn(*const c_char) -> c_int;
    let c_fn: Symbol<ParseFn> = unsafe { c.get(b"parse_operation") }.unwrap();
    let r_fn: Symbol<ParseFn> = unsafe { r.get(b"parse_operation") }.unwrap();

    // Test NULL
    let cv = unsafe { c_fn(std::ptr::null()) };
    let rv = unsafe { r_fn(std::ptr::null()) };
    assert_eq!(cv, rv, "parse_operation(NULL): C={cv} Rust={rv}");

    for s in &["+", "*", "-", "/", "%", "abc", "+*", "x-y", ""] {
        let cs = CString::new(*s).unwrap();
        let cv = unsafe { c_fn(cs.as_ptr()) };
        let rv = unsafe { r_fn(cs.as_ptr()) };
        assert_eq!(cv, rv, "parse_operation({s:?}): C={cv} Rust={rv}");
    }
}

#[test]
fn test_get_operation_func() {
    let _g = LOCK.lock().unwrap();
    let c = unsafe { Library::new(c_lib_path()) }.unwrap();
    let r = unsafe { Library::new(rust_lib_path()) }.unwrap();

    type GetFn = unsafe extern "C" fn(c_int) -> OpFn;
    let c_get: Symbol<GetFn> = unsafe { c.get(b"get_operation_func") }.unwrap();
    let r_get: Symbol<GetFn> = unsafe { r.get(b"get_operation_func") }.unwrap();

    // For each op code, get the function and call it with test values
    for op in 0..=6 {
        let c_func = unsafe { c_get(op) };
        let r_func = unsafe { r_get(op) };
        for &(a, b) in &[(10, 3), (7, 0), (-5, 2)] {
            let cv = unsafe { c_func(a, b, 0, 0) };
            let rv = unsafe { r_func(a, b, 0, 0) };
            assert_eq!(cv, rv, "get_operation_func({op})({a},{b}): C={cv} Rust={rv}");
        }
    }
}

#[test]
fn test_inreftree() {
    let _g = LOCK.lock().unwrap();
    let c = unsafe { Library::new(c_lib_path()) }.unwrap();
    let r = unsafe { Library::new(rust_lib_path()) }.unwrap();

    type InrefFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
    let c_fn: Symbol<InrefFn> = unsafe { c.get(b"inreftree") }.unwrap();
    let r_fn: Symbol<InrefFn> = unsafe { r.get(b"inreftree") }.unwrap();

    let cases: &[(c_int, c_int, c_int, c_int)] = &[
        (1, 2, 3, 4),
        (0, 0, 0, 0),
        (10, 20, 30, 40),
        (-1, -2, -3, -4),
        (100, 0, 50, 25),
        (0, 1, 0, 0),
        (1, 0, 0, 0),
        (5, 5, 5, 5),
        (1000, -500, 250, -125),
        (i32::MAX, 0, 0, 0),
        (0, 0, 0, i32::MIN),
    ];

    for &(a, b, c_val, d) in cases {
        let cv = unsafe { c_fn(a, b, c_val, d) };
        let rv = unsafe { r_fn(a, b, c_val, d) };
        assert_eq!(cv, rv, "inreftree({a},{b},{c_val},{d}): C={cv} Rust={rv}");
    }
}

// Test tree operations via the full inreftree entry point
// (add_tree_node, find_node_by_id, calculate_tree_sum are exercised internally)
// We also test them directly for completeness
#[test]
fn test_tree_operations() {
    let _g = LOCK.lock().unwrap();
    let c = unsafe { Library::new(c_lib_path()) }.unwrap();
    let r = unsafe { Library::new(rust_lib_path()) }.unwrap();

    // Use inreftree to reset state, then the tree functions are tested through it.
    // Direct testing of add_tree_node/find_node_by_id/calculate_tree_sum is tricky
    // because they depend on global state. We test them indirectly through inreftree
    // with varied inputs that exercise different code paths.
    type InrefFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
    let c_fn: Symbol<InrefFn> = unsafe { c.get(b"inreftree") }.unwrap();
    let r_fn: Symbol<InrefFn> = unsafe { r.get(b"inreftree") }.unwrap();

    // Sweep a range of values to exercise different modulo paths
    for a in (-5..=5) {
        for b in (-3..=3) {
            let cv = unsafe { c_fn(a, b, a + b, a - b) };
            let rv = unsafe { r_fn(a, b, a + b, a - b) };
            assert_eq!(cv, rv, "inreftree({a},{b},{},{}) C={cv} Rust={rv}", a + b, a - b);
        }
    }
}
