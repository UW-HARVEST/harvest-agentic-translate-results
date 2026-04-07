use libloading::{Library, Symbol};
use std::ffi::{c_int, CStr};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    let op = if cfg!(feature = "op_add") { "add" }
        else if cfg!(feature = "op_sub") { "sub" }
        else { "mul" };
    let rep = if cfg!(feature = "repeat_0") { "0" }
        else if cfg!(feature = "repeat_1") { "1" }
        else if cfg!(feature = "repeat_2") { "2" }
        else if cfg!(feature = "repeat_3") { "3" }
        else if cfg!(feature = "repeat_4") { "4" }
        else if cfg!(feature = "repeat_5") { "5" }
        else if cfg!(feature = "repeat_6") { "6" }
        else { "7" };
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.join(format!("c_libs/libdriver_{}_{}.so", op, rep))
}

fn rust_lib_path() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // cdylib output is in target/<profile>/libdriver.so
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    dir.join(format!("target/{}/libdriver.so", profile))
}

type BinOp = unsafe extern "C" fn(c_int, c_int) -> c_int;
type UnaryOp = unsafe extern "C" fn(c_int) -> c_int;

#[test]
fn test_op_add() {
    unsafe {
        let c = Library::new(c_lib_path()).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<BinOp> = c.get(b"op_add").unwrap();
        let r_fn: Symbol<BinOp> = r.get(b"op_add").unwrap();
        for &(a, b) in &[(0,0),(1,2),(10,3),(-5,5),(100,-200),(i32::MAX,0),(i32::MIN,0)] {
            assert_eq!(c_fn(a, b), r_fn(a, b), "op_add({},{})", a, b);
        }
    }
}

#[test]
fn test_op_sub() {
    unsafe {
        let c = Library::new(c_lib_path()).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<BinOp> = c.get(b"op_sub").unwrap();
        let r_fn: Symbol<BinOp> = r.get(b"op_sub").unwrap();
        for &(a, b) in &[(0,0),(1,2),(10,3),(-5,5),(100,-200)] {
            assert_eq!(c_fn(a, b), r_fn(a, b), "op_sub({},{})", a, b);
        }
    }
}

#[test]
fn test_op_mul() {
    unsafe {
        let c = Library::new(c_lib_path()).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<BinOp> = c.get(b"op_mul").unwrap();
        let r_fn: Symbol<BinOp> = r.get(b"op_mul").unwrap();
        for &(a, b) in &[(0,0),(1,2),(10,3),(-5,5),(100,-200)] {
            assert_eq!(c_fn(a, b), r_fn(a, b), "op_mul({},{})", a, b);
        }
    }
}

#[test]
fn test_helper_call() {
    unsafe {
        let c = Library::new(c_lib_path()).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<BinOp> = c.get(b"helper_call").unwrap();
        let r_fn: Symbol<BinOp> = r.get(b"helper_call").unwrap();
        for &(a, b) in &[(10,3),(0,0),(1,1),(-5,5)] {
            assert_eq!(c_fn(a, b), r_fn(a, b), "helper_call({},{})", a, b);
        }
    }
}

#[test]
fn test_helper_ptr() {
    unsafe {
        let c = Library::new(c_lib_path()).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<BinOp> = c.get(b"helper_ptr").unwrap();
        let r_fn: Symbol<BinOp> = r.get(b"helper_ptr").unwrap();
        for &(a, b) in &[(10,3),(0,0),(1,1),(-5,5)] {
            assert_eq!(c_fn(a, b), r_fn(a, b), "helper_ptr({},{})", a, b);
        }
    }
}

#[test]
fn test_use_generated() {
    unsafe {
        let c = Library::new(c_lib_path()).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<UnaryOp> = c.get(b"use_generated").unwrap();
        let r_fn: Symbol<UnaryOp> = r.get(b"use_generated").unwrap();
        // Test with values 0-7 and some edge cases
        for n in 0..=7 {
            assert_eq!(c_fn(n), r_fn(n), "use_generated({})", n);
        }
        // Also test default case (n > 6 in C's switch)
        assert_eq!(c_fn(10), r_fn(10), "use_generated(10)");
        assert_eq!(c_fn(-1), r_fn(-1), "use_generated(-1)");
    }
}

#[test]
fn test_g_op_data() {
    unsafe {
        let c = Library::new(c_lib_path()).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();

        // G_OP is a function pointer in C: int (*G_OP)(int,int)
        // In Rust it's a static fn ptr. Test that calling through it works.
        let c_gop: Symbol<*const BinOp> = c.get(b"G_OP").unwrap();
        let r_gop: Symbol<*const BinOp> = r.get(b"G_OP").unwrap();
        let c_fn = **c_gop;
        let r_fn = **r_gop;
        for &(a, b) in &[(10,3),(0,0),(-1,1)] {
            assert_eq!(c_fn(a, b), r_fn(a, b), "G_OP({},{})", a, b);
        }

        // G_OP_NAME is a const char* in C, a &[u8;4] in Rust
        // Both should point to the same string content
        let c_name: Symbol<*const *const u8> = c.get(b"G_OP_NAME").unwrap();
        let r_name: Symbol<*const *const u8> = r.get(b"G_OP_NAME").unwrap();
        let c_str = CStr::from_ptr(**c_name as *const i8);
        let r_str = CStr::from_ptr(**r_name as *const i8);
        assert_eq!(c_str, r_str, "G_OP_NAME mismatch");
    }
}

#[test]
fn test_nm_symbols() {
    // Verify all C exported symbols exist in Rust .so
    let c_path = c_lib_path();
    let r_path = rust_lib_path();
    let c_out = std::process::Command::new("nm").arg("-D").arg(&c_path).output().unwrap();
    let r_out = std::process::Command::new("nm").arg("-D").arg(&r_path).output().unwrap();
    let c_syms: Vec<&str> = std::str::from_utf8(&c_out.stdout).unwrap()
        .lines()
        .filter_map(|l| {
            let parts: Vec<&str> = l.split_whitespace().collect();
            if parts.len() == 3 && (parts[1] == "T" || parts[1] == "D" || parts[1] == "B") {
                let s = parts[2];
                if !s.starts_with('_') { Some(s) } else { None }
            } else { None }
        }).collect();
    let r_text = std::str::from_utf8(&r_out.stdout).unwrap();
    for sym in &c_syms {
        assert!(r_text.contains(sym), "Rust .so missing symbol: {}", sym);
    }
}
