use libloading::{Library, Symbol};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libstatic_alias.so")
}

fn rust_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/debug/libstatic_alias.so")
}

type StaticAliasFn = unsafe extern "C" fn(*mut i32) -> *mut i32;

/// Replicate the C main() logic: start with initial_value, call static_alias
/// `iterations` times, collecting printed values.
unsafe fn run_main_sequence(lib: &Library, initial_value: i32, iterations: i32) -> Vec<i32> {
    let func: Symbol<StaticAliasFn> = lib.get(b"static_alias").unwrap();
    let mut outer: i32 = initial_value;
    let mut running_sum: *mut i32 = &mut outer;
    let mut results = Vec::new();
    for _ in 0..iterations {
        running_sum = func(running_sum);
        results.push(*running_sum);
    }
    results
}

#[test]
fn test_static_alias_sequence_5_3() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let rust_lib = Library::new(rust_lib_path()).unwrap();
        let c_results = run_main_sequence(&c_lib, 5, 3);
        let rust_results = run_main_sequence(&rust_lib, 5, 3);
        assert_eq!(c_results, rust_results, "Mismatch for (5, 3)");
    }
}

#[test]
fn test_static_alias_sequence_1_5() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let rust_lib = Library::new(rust_lib_path()).unwrap();
        let c_results = run_main_sequence(&c_lib, 1, 5);
        let rust_results = run_main_sequence(&rust_lib, 1, 5);
        assert_eq!(c_results, rust_results, "Mismatch for (1, 5)");
    }
}

#[test]
fn test_static_alias_sequence_0_4() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let rust_lib = Library::new(rust_lib_path()).unwrap();
        let c_results = run_main_sequence(&c_lib, 0, 4);
        let rust_results = run_main_sequence(&rust_lib, 0, 4);
        assert_eq!(c_results, rust_results, "Mismatch for (0, 4)");
    }
}

#[test]
fn test_static_alias_sequence_neg() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let rust_lib = Library::new(rust_lib_path()).unwrap();
        let c_results = run_main_sequence(&c_lib, -3, 6);
        let rust_results = run_main_sequence(&rust_lib, -3, 6);
        assert_eq!(c_results, rust_results, "Mismatch for (-3, 6)");
    }
}

#[test]
fn test_static_alias_sequence_large() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let rust_lib = Library::new(rust_lib_path()).unwrap();
        let c_results = run_main_sequence(&c_lib, 100, 10);
        let rust_results = run_main_sequence(&rust_lib, 100, 10);
        assert_eq!(c_results, rust_results, "Mismatch for (100, 10)");
    }
}

#[test]
fn test_static_alias_single_call() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let rust_lib = Library::new(rust_lib_path()).unwrap();
        let c_results = run_main_sequence(&c_lib, 1, 1);
        let rust_results = run_main_sequence(&rust_lib, 1, 1);
        assert_eq!(c_results, rust_results, "Mismatch for (1, 1)");
    }
}

#[test]
fn test_static_alias_zero_iterations() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let rust_lib = Library::new(rust_lib_path()).unwrap();
        let c_results = run_main_sequence(&c_lib, 5, 0);
        let rust_results = run_main_sequence(&rust_lib, 5, 0);
        assert_eq!(c_results, rust_results, "Mismatch for (5, 0)");
    }
}
