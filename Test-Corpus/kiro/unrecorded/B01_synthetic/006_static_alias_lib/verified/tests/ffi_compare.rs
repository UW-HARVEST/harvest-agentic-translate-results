use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::PathBuf;
use std::process::Command;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libStaticAlias.so")
}

fn rust_lib_path() -> PathBuf {
    // The cdylib is built alongside the test artifacts in the deps dir
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/debug/libStaticAlias.so");
    if !p.exists() {
        // fallback: search in target/debug/deps
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug");
        for entry in std::fs::read_dir(&dir).unwrap() {
            let entry = entry.unwrap();
            let name = entry.file_name();
            if name.to_str().unwrap().starts_with("libStaticAlias") && name.to_str().unwrap().ends_with(".so") {
                p = entry.path();
                break;
            }
        }
    }
    p
}

/// Test static_alias by calling both libraries in lockstep with the same inputs.
/// Each library is freshly loaded so the static variable starts at 1.
#[test]
fn test_static_alias_sequence() {
    // Test several sequences of calls with different starting values
    let test_sequences: Vec<Vec<c_int>> = vec![
        vec![5],
        vec![0],
        vec![1],
        vec![-1],
        vec![3, 3, 3],
        vec![1, 2, 3, 4, 5],
        vec![10, 1, 100, 1, 1],
        vec![0, 0, 0, 0],
        vec![-5, 10, -3, 7],
    ];

    for (seq_idx, sequence) in test_sequences.iter().enumerate() {
        // Fresh load for each sequence to reset static state
        let c_lib = unsafe { Library::new(c_lib_path()) }.expect("load C lib");
        let r_lib = unsafe { Library::new(rust_lib_path()) }.expect("load Rust lib");

        let c_fn: Symbol<unsafe extern "C" fn(*mut c_int) -> *mut c_int> =
            unsafe { c_lib.get(b"static_alias") }.expect("C static_alias");
        let r_fn: Symbol<unsafe extern "C" fn(*mut c_int) -> *mut c_int> =
            unsafe { r_lib.get(b"static_alias") }.expect("Rust static_alias");

        let mut c_val: c_int;
        let mut r_val: c_int;

        for (call_idx, &input) in sequence.iter().enumerate() {
            c_val = input;
            r_val = input;

            let c_ret = unsafe { c_fn(&mut c_val) };
            let r_ret = unsafe { r_fn(&mut r_val) };

            let c_ret_val = unsafe { *c_ret };
            let r_ret_val = unsafe { *r_ret };

            assert_eq!(
                c_val, r_val,
                "seq {seq_idx} call {call_idx}: outer mismatch (input={input}): C={c_val}, Rust={r_val}"
            );
            assert_eq!(
                c_ret_val, r_ret_val,
                "seq {seq_idx} call {call_idx}: return val mismatch (input={input}): C={c_ret_val}, Rust={r_ret_val}"
            );

            // Check whether return points to outer or to internal static
            let c_returned_outer = std::ptr::eq(c_ret, &c_val as *const c_int);
            let r_returned_outer = std::ptr::eq(r_ret, &r_val as *const c_int);
            assert_eq!(
                c_returned_outer, r_returned_outer,
                "seq {seq_idx} call {call_idx}: return-pointer-is-outer mismatch (input={input})"
            );
        }
    }
}

/// Test driver by capturing stdout from a subprocess that loads each .so and calls driver.
/// This avoids static state contamination between tests.
#[test]
fn test_driver_output() {
    let test_cases: Vec<(c_int, c_int)> = vec![
        (1, 5),
        (0, 3),
        (5, 1),
        (10, 10),
        (-1, 4),
        (0, 0),
        (1, 0),
    ];

    for &(initial, iters) in &test_cases {
        let c_out = run_driver_subprocess(&c_lib_path(), initial, iters);
        let r_out = run_driver_subprocess(&rust_lib_path(), initial, iters);
        assert_eq!(
            c_out, r_out,
            "driver({initial}, {iters}) output mismatch:\nC:    {c_out:?}\nRust: {r_out:?}"
        );
    }
}

fn run_driver_subprocess(lib_path: &std::path::Path, initial: c_int, iters: c_int) -> String {
    // Write a small C program that dlopen's the library and calls driver
    // Instead, use a Rust helper binary approach via LD_PRELOAD trick
    // Simpler: use a small inline python or shell script with ctypes
    let script = format!(
        r#"
import ctypes, sys
lib = ctypes.CDLL("{}")
lib.driver.argtypes = [ctypes.c_int, ctypes.c_int]
lib.driver.restype = None
lib.driver({}, {})
"#,
        lib_path.display(),
        initial,
        iters
    );

    let output = Command::new("python3")
        .args(["-c", &script])
        .output()
        .expect("failed to run python3");

    assert!(
        output.status.success(),
        "subprocess failed for {:?}: {}",
        lib_path,
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("non-utf8 stdout")
}
