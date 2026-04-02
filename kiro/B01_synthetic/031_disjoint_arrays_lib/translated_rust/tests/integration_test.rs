use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    // Try debug first, then release
    let debug_path = path.join("debug").join("libdriver.so");
    if debug_path.exists() {
        return debug_path;
    }
    path.join("release").join("libdriver.so")
}

// ---- Test 1: fma_array (lowest level) ----
#[test]
fn test_fma_array() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let rust_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    type FmaArrayFn = unsafe extern "C" fn(*mut c_int, *const c_int, *const c_int, *const c_int, c_int);

    let c_fma: Symbol<FmaArrayFn> = unsafe { c_lib.get(b"fma_array").expect("c fma_array") };
    let rs_fma: Symbol<FmaArrayFn> = unsafe { rust_lib.get(b"fma_array").expect("rust fma_array") };

    // Test case 1: basic
    let mul1 = [1, 2, 3, 4, 5];
    let mul2 = [5, 4, 3, 2, 1];
    let add = [10, 20, 30, 40, 50];
    let mut c_out = [0i32; 5];
    let mut rs_out = [0i32; 5];

    unsafe {
        c_fma(c_out.as_mut_ptr(), mul1.as_ptr(), mul2.as_ptr(), add.as_ptr(), 5);
        rs_fma(rs_out.as_mut_ptr(), mul1.as_ptr(), mul2.as_ptr(), add.as_ptr(), 5);
    }
    assert_eq!(c_out, rs_out, "fma_array basic case mismatch");

    // Test case 2: empty
    let mut c_out2 = [0i32; 0];
    let mut rs_out2 = [0i32; 0];
    unsafe {
        c_fma(c_out2.as_mut_ptr(), [].as_ptr(), [].as_ptr(), [].as_ptr(), 0);
        rs_fma(rs_out2.as_mut_ptr(), [].as_ptr(), [].as_ptr(), [].as_ptr(), 0);
    }
    assert_eq!(c_out2, rs_out2, "fma_array empty case mismatch");

    // Test case 3: negative values
    let mul1 = [-1, -2, 3];
    let mul2 = [4, -5, -6];
    let add = [7, 8, -9];
    let mut c_out3 = [0i32; 3];
    let mut rs_out3 = [0i32; 3];
    unsafe {
        c_fma(c_out3.as_mut_ptr(), mul1.as_ptr(), mul2.as_ptr(), add.as_ptr(), 3);
        rs_fma(rs_out3.as_mut_ptr(), mul1.as_ptr(), mul2.as_ptr(), add.as_ptr(), 3);
    }
    assert_eq!(c_out3, rs_out3, "fma_array negative case mismatch");

    // Test case 4: single element
    let mut c_out4 = [0i32; 1];
    let mut rs_out4 = [0i32; 1];
    unsafe {
        c_fma(c_out4.as_mut_ptr(), [7].as_ptr(), [3].as_ptr(), [2].as_ptr(), 1);
        rs_fma(rs_out4.as_mut_ptr(), [7].as_ptr(), [3].as_ptr(), [2].as_ptr(), 1);
    }
    assert_eq!(c_out4, rs_out4, "fma_array single element mismatch");
}

// ---- Test 2: call_fma (mid level) ----
#[test]
fn test_call_fma() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let rust_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    type CallFmaFn = unsafe extern "C" fn(*const c_int, c_int) -> c_int;

    let c_fn: Symbol<CallFmaFn> = unsafe { c_lib.get(b"call_fma").expect("c call_fma") };
    let rs_fn: Symbol<CallFmaFn> = unsafe { rust_lib.get(b"call_fma").expect("rust call_fma") };

    // Test case 1: basic
    let data = [10, 20, 30, 40, 50];
    let c_res = unsafe { c_fn(data.as_ptr(), 5) };
    let rs_res = unsafe { rs_fn(data.as_ptr(), 5) };
    assert_eq!(c_res, rs_res, "call_fma basic mismatch: C={c_res}, Rust={rs_res}");

    // Test case 2: empty
    let c_res = unsafe { c_fn([].as_ptr(), 0) };
    let rs_res = unsafe { rs_fn([].as_ptr(), 0) };
    assert_eq!(c_res, rs_res, "call_fma empty mismatch");

    // Test case 3: single element
    let data = [42];
    let c_res = unsafe { c_fn(data.as_ptr(), 1) };
    let rs_res = unsafe { rs_fn(data.as_ptr(), 1) };
    assert_eq!(c_res, rs_res, "call_fma single mismatch");

    // Test case 4: negative values
    let data = [-5, -10, 15];
    let c_res = unsafe { c_fn(data.as_ptr(), 3) };
    let rs_res = unsafe { rs_fn(data.as_ptr(), 3) };
    assert_eq!(c_res, rs_res, "call_fma negative mismatch");
}

// ---- Test 3: driver (top level) - compare stdout ----
#[test]
fn test_driver_output() {

    let c_lib_p = c_lib_path();
    let rust_lib_p = rust_lib_path();

    let test_inputs = [
        "1 2 3 4 5",
        "42",
        "10 20 30",
        "-1 -2 -3",
        "0",
        "",
        "100 200",
    ];

    for input in &test_inputs {
        // Run C version via a small helper that dlopens and calls driver, capturing stdout
        let c_output = capture_driver_output(&c_lib_p, input);
        let rs_output = capture_driver_output(&rust_lib_p, input);
        assert_eq!(
            c_output, rs_output,
            "driver output mismatch for input {:?}: C={:?}, Rust={:?}",
            input, c_output, rs_output
        );
    }
}

/// Fork a child process, redirect stdout to a pipe, dlopen the lib and call driver.
/// This isolates printf/println output capture.
fn capture_driver_output(lib_path: &std::path::Path, input: &str) -> String {
    use std::process::Command;

    // We'll use a small inline C program approach via a subprocess that loads the lib
    // Actually, simpler: write a small Rust binary that loads and calls driver
    // But simplest: use LD_PRELOAD-like approach... 
    // Actually let's just fork with pipes.

    let lib_path_str = lib_path.to_str().unwrap();
    let input_escaped = input.replace('\\', "\\\\").replace('"', "\\\"");

    // Use a python one-liner to dlopen and call
    let script = format!(
        r#"
import ctypes, sys
lib = ctypes.CDLL("{}")
lib.driver.argtypes = [ctypes.c_char_p]
lib.driver.restype = None
lib.driver(b"{}")
sys.stdout.flush()
"#,
        lib_path_str, input_escaped
    );

    let output = Command::new("python3")
        .arg("-c")
        .arg(&script)
        .env("LD_LIBRARY_PATH", lib_path.parent().unwrap())
        .output()
        .expect("failed to run python3");

    String::from_utf8(output.stdout).expect("non-utf8 output")
}
