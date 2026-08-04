// FFI conformance tests: generic dynamic-array container.

mod common;

use common::*;
use libloading::Library;
use std::os::raw::{c_double, c_int};

// -- function pointer type aliases for each generic instantiation ------------

type ArrIntCreate = unsafe extern "C" fn(size_t) -> *mut array_int_t;
type ArrIntDestroy = unsafe extern "C" fn(*mut array_int_t);
type ArrIntPush = unsafe extern "C" fn(*mut array_int_t, c_int) -> c_int;
type ArrIntGet = unsafe extern "C" fn(*mut array_int_t, size_t) -> c_int;
type ArrIntSize = unsafe extern "C" fn(*mut array_int_t) -> size_t;
type ArrIntClear = unsafe extern "C" fn(*mut array_int_t);

type ArrDoubleCreate = unsafe extern "C" fn(size_t) -> *mut array_double_t;
type ArrDoubleDestroy = unsafe extern "C" fn(*mut array_double_t);
type ArrDoublePush = unsafe extern "C" fn(*mut array_double_t, c_double) -> c_int;
type ArrDoubleGet = unsafe extern "C" fn(*mut array_double_t, size_t) -> c_double;
type ArrDoubleSize = unsafe extern "C" fn(*mut array_double_t) -> size_t;
type ArrDoubleClear = unsafe extern "C" fn(*mut array_double_t);

// Run an exercise on both libs and compare struct state.
fn check_array_int(lib: &Library, values: &[c_int]) -> (size_t, size_t, Vec<c_int>) {
    unsafe {
        let create = sym::<ArrIntCreate>(lib, b"array_int_create");
        let destroy = sym::<ArrIntDestroy>(lib, b"array_int_destroy");
        let push = sym::<ArrIntPush>(lib, b"array_int_push");
        let get = sym::<ArrIntGet>(lib, b"array_int_get");
        let size = sym::<ArrIntSize>(lib, b"array_int_size");
        let clear = sym::<ArrIntClear>(lib, b"array_int_clear");

        // Use a small initial capacity to force growth.
        let arr = create(2);
        assert!(!arr.is_null());

        for v in values {
            let rc = push(arr, *v);
            assert_eq!(rc, 0);
        }

        let n = size(arr);
        let cap = (*arr).capacity;

        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            out.push(get(arr, i));
        }

        // Exercise clear and verify size goes to 0 but data buffer remains
        // (capacity preserved).
        clear(arr);
        assert_eq!(size(arr), 0);
        assert_eq!((*arr).capacity, cap);

        destroy(arr);

        // Exercise size() with NULL
        let nsize = size(std::ptr::null_mut());
        assert_eq!(nsize, 0);

        (n, cap, out)
    }
}

#[test]
fn array_int_matches_c() {
    let c_lib = load_c();
    let r_lib = load_rust();

    // A range that triggers multiple growth events from initial cap = 2.
    let values: Vec<c_int> = (0..20).map(|i| i * 7 - 13).collect();

    let c_result = check_array_int(&c_lib, &values);
    let r_result = check_array_int(&r_lib, &values);

    assert_eq!(c_result, r_result);
}

#[test]
fn array_int_initial_capacity_zero_defaults_to_16() {
    let c_lib = load_c();
    let r_lib = load_rust();

    unsafe {
        for lib in &[&c_lib, &r_lib] {
            let create = sym::<ArrIntCreate>(lib, b"array_int_create");
            let destroy = sym::<ArrIntDestroy>(lib, b"array_int_destroy");
            let arr = create(0);
            assert!(!arr.is_null());
            assert_eq!((*arr).capacity, 16);
            assert_eq!((*arr).size, 0);
            destroy(arr);
        }
    }
}

#[test]
fn array_int_push_null_returns_negative() {
    let c_lib = load_c();
    let r_lib = load_rust();
    unsafe {
        for lib in &[&c_lib, &r_lib] {
            let push = sym::<ArrIntPush>(lib, b"array_int_push");
            assert_eq!(push(std::ptr::null_mut(), 7), -1);
        }
    }
}

fn check_array_double(lib: &Library, values: &[c_double]) -> (size_t, size_t, Vec<u64>) {
    unsafe {
        let create = sym::<ArrDoubleCreate>(lib, b"array_double_create");
        let destroy = sym::<ArrDoubleDestroy>(lib, b"array_double_destroy");
        let push = sym::<ArrDoublePush>(lib, b"array_double_push");
        let get = sym::<ArrDoubleGet>(lib, b"array_double_get");
        let size = sym::<ArrDoubleSize>(lib, b"array_double_size");
        let clear = sym::<ArrDoubleClear>(lib, b"array_double_clear");

        let arr = create(3);
        assert!(!arr.is_null());

        for v in values {
            assert_eq!(push(arr, *v), 0);
        }

        let n = size(arr);
        let cap = (*arr).capacity;

        // Compare doubles bit-for-bit so that NaN payloads also match.
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            out.push(get(arr, i).to_bits());
        }

        clear(arr);
        assert_eq!(size(arr), 0);
        destroy(arr);

        (n, cap, out)
    }
}

#[test]
fn array_double_matches_c() {
    let c_lib = load_c();
    let r_lib = load_rust();

    let values = vec![
        23.5_f64, 25.0, 22.8, 26.3, 24.1, 21.9, 27.5, -1.0, 0.0, 1e308, 1e-308,
    ];

    let c_result = check_array_double(&c_lib, &values);
    let r_result = check_array_double(&r_lib, &values);

    assert_eq!(c_result, r_result);
}

// -- item_t array ------------------------------------------------------------

type ArrItemCreate = unsafe extern "C" fn(size_t) -> *mut array_item_t_t;
type ArrItemDestroy = unsafe extern "C" fn(*mut array_item_t_t);
type ArrItemPush = unsafe extern "C" fn(*mut array_item_t_t, item_t) -> c_int;
type ArrItemGet = unsafe extern "C" fn(*mut array_item_t_t, size_t) -> item_t;
type ArrItemSize = unsafe extern "C" fn(*mut array_item_t_t) -> size_t;
#[allow(dead_code)]
type ArrItemClear = unsafe extern "C" fn(*mut array_item_t_t);

type CreateItemFn =
    unsafe extern "C" fn(c_int, *const std::os::raw::c_char, *const std::os::raw::c_char,
        c_double, c_int) -> item_t;

// Build a "logical fingerprint" of an item_t that excludes structure padding
// bytes (which are uninitialized garbage in C and will not match between
// invocations). We hash: id, name (up to NUL), category (up to NUL), price
// bits, quantity.
fn item_fingerprint(it: &item_t) -> (c_int, Vec<u8>, Vec<u8>, u64, c_int) {
    let name = cstr_slice(&it.name).to_vec();
    let cat = cstr_slice(&it.category).to_vec();
    (it.id, name, cat, it.price.to_bits(), it.quantity)
}

fn run_array_item(lib: &Library) -> Vec<(c_int, Vec<u8>, Vec<u8>, u64, c_int)> {
    unsafe {
        let create = sym::<ArrItemCreate>(lib, b"array_item_t_create");
        let destroy = sym::<ArrItemDestroy>(lib, b"array_item_t_destroy");
        let push = sym::<ArrItemPush>(lib, b"array_item_t_push");
        let get = sym::<ArrItemGet>(lib, b"array_item_t_get");
        let size = sym::<ArrItemSize>(lib, b"array_item_t_size");
        let create_item = sym::<CreateItemFn>(lib, b"create_item");

        let arr = create(2);
        let names_cats: &[(&str, &str, c_double, c_int)] = &[
            ("Laptop", "Electronics", 899.99, 15),
            ("Mouse", "Electronics", 24.99, 50),
            ("Desk", "Furniture", 349.99, 8),
            ("Notebook", "Office", 4.99, 100),
            ("USB Cable", "Electronics", 9.99, 60),
        ];

        for (i, &(n, c, p, q)) in names_cats.iter().enumerate() {
            let cn = std::ffi::CString::new(n).unwrap();
            let cc = std::ffi::CString::new(c).unwrap();
            let it = create_item((i as c_int) + 1, cn.as_ptr(), cc.as_ptr(), p, q);
            assert_eq!(push(arr, it), 0);
        }

        let n = size(arr);
        let mut fps = Vec::with_capacity(n);
        for i in 0..n {
            let it = get(arr, i);
            fps.push(item_fingerprint(&it));
        }

        destroy(arr);
        fps
    }
}

#[test]
fn array_item_t_matches_c() {
    let c_lib = load_c();
    let r_lib = load_rust();
    let c_fps = run_array_item(&c_lib);
    let r_fps = run_array_item(&r_lib);
    assert_eq!(c_fps, r_fps);
}
