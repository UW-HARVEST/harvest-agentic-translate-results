use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Find the built cdylib
    let target_dir = dir.join("target/debug");
    target_dir.join("libcheckshift_lib.so")
}

type BinOp = unsafe extern "C" fn(c_int, c_int) -> c_int;
type CheckshiftFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

#[repr(C)]
struct ComputeState {
    accumulator: c_int,
    operation_count: c_int,
    checksum: u32,
}

// Test inputs for arithmetic functions
const TEST_PAIRS: &[(c_int, c_int)] = &[
    (0, 0),
    (1, 1),
    (5, 3),
    (-1, 1),
    (100, 200),
    (-50, 50),
    (0x7FFF, 0x7FFF),
    (1, 0),
    (-1, -1),
    (42, 7),
];

macro_rules! test_binop {
    ($name:ident, $sym:expr) => {
        #[test]
        fn $name() {
            unsafe {
                let c_lib = Library::new(c_lib_path()).expect("load C lib");
                let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");
                let c_fn: Symbol<BinOp> = c_lib.get($sym).expect("C symbol");
                let r_fn: Symbol<BinOp> = r_lib.get($sym).expect("Rust symbol");
                for &(a, b) in TEST_PAIRS {
                    let c_val = c_fn(a, b);
                    let r_val = r_fn(a, b);
                    assert_eq!(c_val, r_val, "{}: C({},{})={} Rust={}", stringify!($name), a, b, c_val, r_val);
                }
            }
        }
    };
}

test_binop!(test_multiply_with_static, b"multiply_with_static\0");
test_binop!(test_add_with_static, b"add_with_static\0");
test_binop!(test_xor_operation, b"xor_operation\0");
test_binop!(test_shift_with_static, b"shift_with_static\0");

#[test]
fn test_compute_checksum() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        type ChecksumFn = unsafe extern "C" fn(*const c_int, c_int) -> u32;
        let c_fn: Symbol<ChecksumFn> = c_lib.get(b"compute_checksum\0").expect("C symbol");
        let r_fn: Symbol<ChecksumFn> = r_lib.get(b"compute_checksum\0").expect("Rust symbol");

        // Test with various arrays
        let test_arrays: &[&[c_int]] = &[
            &[1, 2, 3, 4],
            &[0, 0, 0, 0],
            &[-1, -1, -1, -1],
            &[0x7FFFFFFF, 0, -1, 42],
            &[1],
            &[1, 2],
            &[1, 2, 3],
            &[100, 200, 300, 400],
        ];

        for arr in test_arrays {
            let c_val = c_fn(arr.as_ptr(), arr.len() as c_int);
            let r_val = r_fn(arr.as_ptr(), arr.len() as c_int);
            assert_eq!(c_val, r_val, "compute_checksum({:?}): C=0x{:04X} Rust=0x{:04X}", arr, c_val, r_val);
        }

        // Test null pointer
        let c_null = c_fn(std::ptr::null(), 0);
        let r_null = r_fn(std::ptr::null(), 0);
        assert_eq!(c_null, r_null, "compute_checksum(null): C={} Rust={}", c_null, r_null);
    }
}

#[test]
fn test_checkshift() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");
        let c_fn: Symbol<CheckshiftFn> = c_lib.get(b"checkshift\0").expect("C symbol");
        let r_fn: Symbol<CheckshiftFn> = r_lib.get(b"checkshift\0").expect("Rust symbol");

        let test_cases: &[(c_int, c_int, c_int, c_int)] = &[
            (1, 2, 3, 4),
            (0, 0, 0, 0),
            (10, 20, 30, 40),
            (-1, -2, -3, -4),
            (100, 200, 300, 400),
            (1, 0, 0, 0),
            (0, 1, 0, 0),
            (0, 0, 1, 0),
            (0, 0, 0, 1),
            (42, 7, 13, 99),
        ];

        for &(a, b, c, d) in test_cases {
            let c_val = c_fn(a, b, c, d);
            let r_val = r_fn(a, b, c, d);
            assert_eq!(c_val, r_val, "checkshift({},{},{},{}): C={} Rust={}", a, b, c, d, c_val, r_val);
        }
    }
}
