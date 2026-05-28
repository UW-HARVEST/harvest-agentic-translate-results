// Integration tests: compare C .so vs Rust .so via libloading.
// Both libraries must export identical symbols and produce identical results.

use libloading::{Library, Symbol};
use std::os::raw::{c_int, c_void};

const C_LIB: &str = "c_src/build/libtranslated_rust.so";
const RUST_LIB: &str = "target/debug/libmatrixsum_lib.so";

#[repr(C)]
struct DynamicArray {
    data: *mut c_int,
    size: usize,
    capacity: usize,
}

unsafe fn load_libs() -> (Library, Library) {
    let c = Library::new(C_LIB).expect("failed to load C lib (build c_src first)");
    let r = Library::new(RUST_LIB).expect("failed to load Rust lib (cargo build first)");
    (c, r)
}

#[test]
fn test_process_flags() {
    unsafe {
        let (c, r) = load_libs();
        let c_fn: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            c.get(b"process_flags").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            r.get(b"process_flags").unwrap();

        for flags in 0..32 {
            let cv = c_fn(flags);
            let rv = r_fn(flags);
            assert_eq!(cv, rv, "process_flags({}) mismatch: c={} rust={}", flags, cv, rv);
        }

        // Edge cases: negative, large
        for flags in [-1, c_int::MIN, c_int::MAX, 0xFF, 0x10, 0xAB, 0b1111] {
            let cv = c_fn(flags);
            let rv = r_fn(flags);
            assert_eq!(cv, rv, "process_flags({}) mismatch", flags);
        }
    }
}

#[test]
fn test_calculate_matrix_checksum() {
    unsafe {
        let (c, r) = load_libs();
        let c_fn: Symbol<unsafe extern "C" fn() -> c_int> =
            c.get(b"calculate_matrix_checksum").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn() -> c_int> =
            r.get(b"calculate_matrix_checksum").unwrap();

        let cv = c_fn();
        let rv = r_fn();
        assert_eq!(cv, rv, "checksum mismatch");
    }
}

#[test]
fn test_matrix_global_value() {
    unsafe {
        let (c, r) = load_libs();
        let c_sym: Symbol<*mut [[c_int; 4]; 3]> = c.get(b"matrix").unwrap();
        let r_sym: Symbol<*mut [[c_int; 4]; 3]> = r.get(b"matrix").unwrap();
        let c_mat = &**c_sym;
        let r_mat = &**r_sym;
        assert_eq!(c_mat, r_mat, "matrix global mismatch");
    }
}

#[test]
fn test_init_array_and_add_and_free() {
    unsafe {
        let (c, r) = load_libs();

        let c_init: Symbol<unsafe extern "C" fn(usize) -> *mut DynamicArray> =
            c.get(b"init_array").unwrap();
        let c_add: Symbol<unsafe extern "C" fn(*mut DynamicArray, c_int) -> c_int> =
            c.get(b"add_element").unwrap();
        let c_free: Symbol<unsafe extern "C" fn(*mut DynamicArray)> =
            c.get(b"free_array").unwrap();

        let r_init: Symbol<unsafe extern "C" fn(usize) -> *mut DynamicArray> =
            r.get(b"init_array").unwrap();
        let r_add: Symbol<unsafe extern "C" fn(*mut DynamicArray, c_int) -> c_int> =
            r.get(b"add_element").unwrap();
        let r_free: Symbol<unsafe extern "C" fn(*mut DynamicArray)> =
            r.get(b"free_array").unwrap();

        for cap in [1usize, 2, 4, 8] {
            let ca = c_init(cap);
            let ra = r_init(cap);
            assert!(!ca.is_null());
            assert!(!ra.is_null());
            assert_eq!((*ca).size, (*ra).size);
            assert_eq!((*ca).capacity, (*ra).capacity);

            // Add 10 elements (will trigger expand)
            for v in 0..10 {
                let cv = c_add(ca, v);
                let rv = r_add(ra, v);
                assert_eq!(cv, rv, "add_element rv mismatch");
                assert_eq!((*ca).size, (*ra).size);
                assert_eq!((*ca).capacity, (*ra).capacity);
            }
            // Compare data
            let csz = (*ca).size;
            for i in 0..csz {
                let cd = *(*ca).data.add(i);
                let rd = *(*ra).data.add(i);
                assert_eq!(cd, rd, "data[{}] mismatch", i);
            }
            c_free(ca);
            r_free(ra);
        }
    }
}

#[test]
fn test_expand_array() {
    unsafe {
        let (c, r) = load_libs();

        let c_init: Symbol<unsafe extern "C" fn(usize) -> *mut DynamicArray> =
            c.get(b"init_array").unwrap();
        let c_expand: Symbol<unsafe extern "C" fn(*mut DynamicArray) -> c_int> =
            c.get(b"expand_array").unwrap();
        let c_free: Symbol<unsafe extern "C" fn(*mut DynamicArray)> =
            c.get(b"free_array").unwrap();

        let r_init: Symbol<unsafe extern "C" fn(usize) -> *mut DynamicArray> =
            r.get(b"init_array").unwrap();
        let r_expand: Symbol<unsafe extern "C" fn(*mut DynamicArray) -> c_int> =
            r.get(b"expand_array").unwrap();
        let r_free: Symbol<unsafe extern "C" fn(*mut DynamicArray)> =
            r.get(b"free_array").unwrap();

        let ca = c_init(4);
        let ra = r_init(4);
        let cv = c_expand(ca);
        let rv = r_expand(ra);
        assert_eq!(cv, rv);
        assert_eq!((*ca).capacity, (*ra).capacity);
        assert_eq!((*ca).size, (*ra).size);
        c_free(ca);
        r_free(ra);

        // null pointer expand
        let cv = c_expand(std::ptr::null_mut());
        let rv = r_expand(std::ptr::null_mut());
        assert_eq!(cv, rv);
    }
}

#[test]
fn test_matrixsum() {
    unsafe {
        let (c, r) = load_libs();
        let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            c.get(b"matrixsum").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            r.get(b"matrixsum").unwrap();

        let cases: &[(c_int, c_int, c_int, c_int)] = &[
            (0, 0, 0, 0),
            (1, 0, 0, 0),
            (0, 1, 0, 0),
            (0, 0, 1, 0),
            (0, 0, 0, 1),
            (1, 1, 1, 1),
            (10, 20, 30, 40),
            (-1, -2, -3, -4),
            (100, 200, 300, 400),
            (0xFF, 0x10, 0xA1, 0xD4),
            (-100, 50, -50, 100),
            (1, 2, 3, 4),
            (i32::MAX, 0, 0, 0),
            (i32::MIN, 0, 0, 0),
            (1, 1, 1, 0),
            (0, 0, 1, 1),
        ];

        for (a, b, cc, d) in cases.iter() {
            let cv = c_fn(*a, *b, *cc, *d);
            let rv = r_fn(*a, *b, *cc, *d);
            assert_eq!(cv, rv, "matrixsum({},{},{},{}) c={} rust={}", a, b, cc, d, cv, rv);
        }
    }
}

// Suppress unused warning
#[allow(dead_code)]
fn _unused(_: *mut c_void) {}
