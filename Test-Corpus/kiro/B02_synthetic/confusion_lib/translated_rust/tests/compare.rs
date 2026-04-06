use libloading::{Library, Symbol};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    // Find the built Rust cdylib
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target_dir = dir.join("target/debug");
    // Try common names
    for name in &["libconfusion_lib.so", "libconfusion_lib.dylib"] {
        let p = target_dir.join(name);
        if p.exists() {
            return p;
        }
    }
    // Fallback: search
    panic!("Could not find Rust .so in {:?}", target_dir);
}

/// Call confusion() via the C shared library
fn call_c_confusion(a: i32, b: i32, c: i32, d: i32) -> i32 {
    unsafe {
        let lib = Library::new(c_lib_path()).expect("load C lib");
        let func: Symbol<unsafe extern "C" fn(i32, i32, i32, i32) -> i32> =
            lib.get(b"confusion").expect("find confusion symbol");
        func(a, b, c, d)
    }
}

/// Call confusion() via the Rust shared library
fn call_rust_confusion(a: i32, b: i32, c: i32, d: i32) -> i32 {
    unsafe {
        let lib = Library::new(rust_lib_path()).expect("load Rust lib");
        let func: Symbol<unsafe extern "C" fn(i32, i32, i32, i32) -> i32> =
            lib.get(b"confusion").expect("find confusion symbol");
        func(a, b, c, d)
    }
}

/// Test a set of inputs and assert C == Rust
fn check(a: i32, b: i32, c: i32, d: i32) {
    let c_result = call_c_confusion(a, b, c, d);
    let r_result = call_rust_confusion(a, b, c, d);
    assert_eq!(
        c_result, r_result,
        "MISMATCH for confusion({a}, {b}, {c}, {d}): C={c_result}, Rust={r_result}"
    );
}

#[test]
fn test_confusion_op0() {
    // param4 % 4 == 0 => confuse_types case 0 (set int)
    check(42, 7, 3, 0);
    check(100, 15, 0, 4); // 4%4==0
}

#[test]
fn test_confusion_op1() {
    // param4 % 4 == 1 => confuse_types case 1 (read as float)
    check(42, 7, 3, 1);
    check(100, 31, 5, 5); // 5%4==1
}

#[test]
fn test_confusion_op2() {
    // param4 % 4 == 2 => confuse_types case 2 (read as uint)
    check(42, 7, 3, 2);
    check(-1, 0, 9, 6); // 6%4==2
}

#[test]
fn test_confusion_op3() {
    // param4 % 4 == 3 => confuse_types case 3 (read as bytes)
    check(42, 7, 3, 3);
    check(0, 0, 0, 7); // 7%4==3
}

#[test]
fn test_confusion_various() {
    // Broader coverage
    for p1 in [0, 1, -1, 42, 255, 1000] {
        for p4 in 0..4 {
            check(p1, 7, 3, p4);
        }
    }
}
