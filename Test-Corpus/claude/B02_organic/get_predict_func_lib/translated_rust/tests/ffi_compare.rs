use libloading::{Library, Symbol};
use std::os::raw::c_int;
use std::path::PathBuf;

type GetPredictFunc = unsafe extern "C" fn(c_int) -> c_int;

fn load_libs() -> (Library, Library) {
    // Locate the C lib
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_so = manifest_dir.join("c_src/build/libtranslated_rust.so");
    // Locate the Rust lib (built by cargo)
    let rust_so_release = manifest_dir.join("target/release/libget_predict_func_lib.so");
    let rust_so_debug = manifest_dir.join("target/debug/libget_predict_func_lib.so");

    let rust_so = if rust_so_release.exists() {
        rust_so_release
    } else {
        rust_so_debug
    };

    assert!(c_so.exists(), "C library not found at {:?}", c_so);
    assert!(rust_so.exists(), "Rust library not found at {:?}", rust_so);

    let c_lib = unsafe { Library::new(c_so).expect("failed to load C lib") };
    let rust_lib = unsafe { Library::new(rust_so).expect("failed to load Rust lib") };
    (c_lib, rust_lib)
}

#[test]
fn get_predict_func_matches_c() {
    let (c_lib, rust_lib) = load_libs();

    let c_fn: Symbol<GetPredictFunc> =
        unsafe { c_lib.get(b"get_predict_func").expect("get_predict_func missing in C") };
    let rust_fn: Symbol<GetPredictFunc> =
        unsafe { rust_lib.get(b"get_predict_func").expect("get_predict_func missing in Rust") };

    // Test valid range (0..=11) and outside range (defaults)
    let inputs: Vec<c_int> = (-10..=20).collect();
    for &pfcn in &inputs {
        let c_res = unsafe { c_fn(pfcn) };
        let r_res = unsafe { rust_fn(pfcn) };
        assert_eq!(
            c_res, r_res,
            "get_predict_func mismatch for pfcn={}: C={} Rust={}",
            pfcn, c_res, r_res
        );
    }
}

#[test]
fn get_predict_func_returns_one_for_valid_range() {
    // The C code's get_predict_func returns 1 when fcn matches the dispatch
    // (which happens for pfcn in 0..=11). For default branch, the C code does
    // NOT set result, so result remains 0. The Rust translation must mirror this.
    let (c_lib, rust_lib) = load_libs();

    let c_fn: Symbol<GetPredictFunc> = unsafe { c_lib.get(b"get_predict_func").unwrap() };
    let rust_fn: Symbol<GetPredictFunc> = unsafe { rust_lib.get(b"get_predict_func").unwrap() };

    for pfcn in 0..=11 {
        let c_res = unsafe { c_fn(pfcn) };
        let r_res = unsafe { rust_fn(pfcn) };
        assert_eq!(c_res, 1, "C should return 1 for pfcn={}", pfcn);
        assert_eq!(r_res, 1, "Rust should return 1 for pfcn={}", pfcn);
    }

    // Default branch -> result stays 0 in C
    for pfcn in [-1, 12, 13, 14, 15, 16, 100] {
        let c_res = unsafe { c_fn(pfcn) };
        let r_res = unsafe { rust_fn(pfcn) };
        assert_eq!(
            c_res, r_res,
            "default-branch mismatch for pfcn={}: C={} Rust={}",
            pfcn, c_res, r_res
        );
    }
}
