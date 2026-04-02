use libloading::{Library, Symbol};
use std::os::raw::c_int;
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("debug")
        .join("libmatrixsum_lib.so");
    if !path.exists() {
        path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("release")
            .join("libmatrixsum_lib.so");
    }
    path
}

#[repr(C)]
struct DynamicArray {
    data: *mut c_int,
    size: usize,
    capacity: usize,
}

#[test]
fn test_process_flags() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("Failed to load C lib");
        let rust_lib = Library::new(rust_lib_path()).expect("Failed to load Rust lib");

        let c_fn: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            c_lib.get(b"process_flags").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            rust_lib.get(b"process_flags").unwrap();

        let test_inputs: &[c_int] = &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 15, 16, 0xFF, -1];
        for &input in test_inputs {
            let c_result = c_fn(input);
            let r_result = r_fn(input);
            assert_eq!(c_result, r_result, "process_flags({input}): C={c_result}, Rust={r_result}");
        }
    }
}

#[test]
fn test_calculate_matrix_checksum() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("Failed to load C lib");
        let rust_lib = Library::new(rust_lib_path()).expect("Failed to load Rust lib");

        let c_fn: Symbol<unsafe extern "C" fn() -> c_int> =
            c_lib.get(b"calculate_matrix_checksum").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn() -> c_int> =
            rust_lib.get(b"calculate_matrix_checksum").unwrap();

        let c_result = c_fn();
        let r_result = r_fn();
        assert_eq!(c_result, r_result, "calculate_matrix_checksum: C={c_result}, Rust={r_result}");
    }
}

#[test]
fn test_matrix_global() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("Failed to load C lib");
        let rust_lib = Library::new(rust_lib_path()).expect("Failed to load Rust lib");

        let c_matrix: Symbol<*const [[c_int; 4]; 3]> = c_lib.get(b"matrix").unwrap();
        let r_matrix: Symbol<*const [[c_int; 4]; 3]> = rust_lib.get(b"matrix").unwrap();

        let c_data = &**c_matrix;
        let r_data = &**r_matrix;

        for i in 0..3 {
            for j in 0..4 {
                assert_eq!(
                    c_data[i][j], r_data[i][j],
                    "matrix[{i}][{j}]: C={}, Rust={}", c_data[i][j], r_data[i][j]
                );
            }
        }
    }
}

#[test]
fn test_init_free_array() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("Failed to load C lib");
        let rust_lib = Library::new(rust_lib_path()).expect("Failed to load Rust lib");

        let c_init: Symbol<unsafe extern "C" fn(usize) -> *mut DynamicArray> =
            c_lib.get(b"init_array").unwrap();
        let c_free: Symbol<unsafe extern "C" fn(*mut DynamicArray)> =
            c_lib.get(b"free_array").unwrap();
        let r_init: Symbol<unsafe extern "C" fn(usize) -> *mut DynamicArray> =
            rust_lib.get(b"init_array").unwrap();
        let r_free: Symbol<unsafe extern "C" fn(*mut DynamicArray)> =
            rust_lib.get(b"free_array").unwrap();

        for &cap in &[1usize, 2, 4, 10, 100] {
            let c_arr = c_init(cap);
            let r_arr = r_init(cap);
            assert!(!c_arr.is_null(), "C init_array({cap}) returned null");
            assert!(!r_arr.is_null(), "Rust init_array({cap}) returned null");
            assert_eq!((*c_arr).size, (*r_arr).size, "init_array({cap}) size mismatch");
            assert_eq!((*c_arr).capacity, (*r_arr).capacity, "init_array({cap}) capacity mismatch");
            c_free(c_arr);
            r_free(r_arr);
        }
    }
}

#[test]
fn test_add_element_and_expand() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("Failed to load C lib");
        let rust_lib = Library::new(rust_lib_path()).expect("Failed to load Rust lib");

        let c_init: Symbol<unsafe extern "C" fn(usize) -> *mut DynamicArray> =
            c_lib.get(b"init_array").unwrap();
        let c_add: Symbol<unsafe extern "C" fn(*mut DynamicArray, c_int) -> c_int> =
            c_lib.get(b"add_element").unwrap();
        let c_free: Symbol<unsafe extern "C" fn(*mut DynamicArray)> =
            c_lib.get(b"free_array").unwrap();

        let r_init: Symbol<unsafe extern "C" fn(usize) -> *mut DynamicArray> =
            rust_lib.get(b"init_array").unwrap();
        let r_add: Symbol<unsafe extern "C" fn(*mut DynamicArray, c_int) -> c_int> =
            rust_lib.get(b"add_element").unwrap();
        let r_free: Symbol<unsafe extern "C" fn(*mut DynamicArray)> =
            rust_lib.get(b"free_array").unwrap();

        let c_arr = c_init(2);
        let r_arr = r_init(2);

        let values = [10, 20, 30, 40, 50];
        for &v in &values {
            let c_ret = c_add(c_arr, v);
            let r_ret = r_add(r_arr, v);
            assert_eq!(c_ret, r_ret, "add_element return mismatch for value {v}");
        }

        assert_eq!((*c_arr).size, (*r_arr).size, "size mismatch after adds");
        for i in 0..(*c_arr).size {
            let c_val = *(*c_arr).data.add(i);
            let r_val = *(*r_arr).data.add(i);
            assert_eq!(c_val, r_val, "data[{i}] mismatch: C={c_val}, Rust={r_val}");
        }

        c_free(c_arr);
        r_free(r_arr);
    }
}

#[test]
fn test_matrixsum() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("Failed to load C lib");
        let rust_lib = Library::new(rust_lib_path()).expect("Failed to load Rust lib");

        let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            c_lib.get(b"matrixsum").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            rust_lib.get(b"matrixsum").unwrap();

        let test_cases: &[(c_int, c_int, c_int, c_int)] = &[
            (0, 0, 0, 0),
            (1, 0, 0, 0),
            (0, 1, 0, 0),
            (0, 0, 1, 0),
            (0, 0, 0, 1),
            (1, 1, 1, 1),
            (1, 2, 3, 4),
            (10, 20, 30, 40),
            (100, 200, 300, 400),
            (-1, -2, -3, -4),
            (0x7FFFFFFF, 0, 0, 0),
            (-2147483648, 0, 0, 0),
            (0xFF, 0xFF, 0xFF, 0xFF),
        ];

        for &(a, b, c, d) in test_cases {
            let c_result = c_fn(a, b, c, d);
            let r_result = r_fn(a, b, c, d);
            assert_eq!(
                c_result, r_result,
                "matrixsum({a}, {b}, {c}, {d}): C={c_result}, Rust={r_result}"
            );
        }
    }
}
