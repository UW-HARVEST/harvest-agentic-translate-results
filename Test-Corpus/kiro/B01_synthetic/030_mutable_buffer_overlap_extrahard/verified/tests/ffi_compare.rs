use libloading::{Library, Symbol};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libdriver_c.so")
}

fn rust_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/libdriver.so")
}

type FmaArrayFn = unsafe extern "C" fn(*mut i32, *const i32, *const i32, *const i32, i32);
type DriverFn = unsafe extern "C" fn(*mut i32, i32);

// ---- fma_array tests (lowest level) ----

fn test_fma_array_case(input: &[i32]) {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C .so");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust .so");

        let c_fma: Symbol<FmaArrayFn> = c_lib.get(b"fma_array").unwrap();
        let r_fma: Symbol<FmaArrayFn> = r_lib.get(b"fma_array").unwrap();

        let len = input.len() as i32;

        // Test with separate arrays (no aliasing)
        let mul1: Vec<i32> = input.to_vec();
        let mul2: Vec<i32> = input.to_vec();
        let add: Vec<i32> = input.to_vec();

        let mut c_out = vec![0i32; input.len()];
        let mut r_out = vec![0i32; input.len()];

        c_fma(c_out.as_mut_ptr(), mul1.as_ptr(), mul2.as_ptr(), add.as_ptr(), len);
        r_fma(r_out.as_mut_ptr(), mul1.as_ptr(), mul2.as_ptr(), add.as_ptr(), len);

        assert_eq!(c_out, r_out, "fma_array mismatch (separate) for {:?}", input);

        // Test with aliased pointers (same as driver does)
        let mut c_data: Vec<i32> = input.to_vec();
        let mut r_data: Vec<i32> = input.to_vec();

        let cp = c_data.as_mut_ptr();
        c_fma(cp, cp, cp, cp, len);

        let rp = r_data.as_mut_ptr();
        r_fma(rp, rp, rp, rp, len);

        assert_eq!(c_data, r_data, "fma_array mismatch (aliased) for {:?}", input);
    }
}

#[test]
fn test_fma_array_basic() {
    test_fma_array_case(&[1, 2, 3, 4, 5]);
}

#[test]
fn test_fma_array_negative() {
    test_fma_array_case(&[-1, -2, -3]);
}

#[test]
fn test_fma_array_zeros() {
    test_fma_array_case(&[0, 0, 0]);
}

#[test]
fn test_fma_array_single() {
    test_fma_array_case(&[42]);
}

#[test]
fn test_fma_array_empty() {
    test_fma_array_case(&[]);
}

#[test]
fn test_fma_array_mixed() {
    test_fma_array_case(&[10, -5, 0, 7, -100]);
}

// ---- driver tests (higher level, calls fma_array + prints) ----

#[test]
fn test_driver_output() {
    // driver prints to stdout, so we compare via the executable outputs
    // But for the .so test, we just verify the array mutation matches
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C .so");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust .so");

        let c_driver: Symbol<DriverFn> = c_lib.get(b"driver").unwrap();
        let r_driver: Symbol<DriverFn> = r_lib.get(b"driver").unwrap();

        for input in &[
            vec![1, 2, 3, 4, 5],
            vec![-1, -2, -3],
            vec![0],
            vec![10, -5, 0, 7],
            vec![],
        ] {
            let mut c_data: Vec<i32> = input.clone();
            let mut r_data: Vec<i32> = input.clone();

            c_driver(c_data.as_mut_ptr(), c_data.len() as i32);
            r_driver(r_data.as_mut_ptr(), r_data.len() as i32);

            assert_eq!(c_data, r_data, "driver mismatch for {:?}", input);
        }
    }
}

// ---- symbol export check ----

#[test]
fn test_symbol_exports() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C .so");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust .so");

        for sym in &[b"fma_array" as &[u8], b"driver", b"main"] {
            let c_sym: Result<Symbol<*const ()>, _> = c_lib.get(sym);
            let r_sym: Result<Symbol<*const ()>, _> = r_lib.get(sym);
            assert!(c_sym.is_ok(), "C .so missing symbol: {}", std::str::from_utf8(sym).unwrap());
            assert!(r_sym.is_ok(), "Rust .so missing symbol: {}", std::str::from_utf8(sym).unwrap());
        }
    }
}
