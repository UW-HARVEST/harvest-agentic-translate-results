// Integration tests that load both C and Rust shared libraries via libloading
// and compare their outputs through the FFI boundary.

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::PathBuf;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Result {
    pub value: c_int,
    pub scaled: f64,
    pub rank: c_int,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ResultArray {
    pub data: [Result; 10],
    pub count: c_int,
}

pub type OperationFunc =
    extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

fn c_so_path() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .join("c_src")
        .join("build")
        .join("libtranslated_rust.so")
}

fn rust_so_path() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Try release first (since we built it with --release), fallback to debug.
    let release = manifest.join("target").join("release").join("libarrayfunc_lib.so");
    if release.exists() {
        release
    } else {
        manifest.join("target").join("debug").join("libarrayfunc_lib.so")
    }
}

fn load_libs() -> (Library, Library) {
    let c = unsafe { Library::new(c_so_path()) }.expect("load C .so");
    let r = unsafe { Library::new(rust_so_path()) }.expect("load Rust .so");
    (c, r)
}

fn empty_array() -> ResultArray {
    ResultArray {
        data: [Result {
            value: 0,
            scaled: 0.0,
            rank: 0,
        }; 10],
        count: 0,
    }
}

fn array_eq(a: &ResultArray, b: &ResultArray) -> bool {
    if a.count != b.count {
        return false;
    }
    for i in 0..(a.count.max(0) as usize) {
        if a.data[i].value != b.data[i].value
            || a.data[i].rank != b.data[i].rank
            || a.data[i].scaled.to_bits() != b.data[i].scaled.to_bits()
        {
            return false;
        }
    }
    true
}

#[test]
fn test_add_operation() {
    let (c, r) = load_libs();
    let cf: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
        unsafe { c.get(b"add_operation").unwrap() };
    let rf: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
        unsafe { r.get(b"add_operation").unwrap() };

    let cases: [(c_int, c_int); 7] = [
        (0, 0),
        (1, 2),
        (-5, 5),
        (i32::MAX, 0),
        (i32::MIN, 0),
        (i32::MAX, 1),  // wrap
        (i32::MIN, -1), // wrap
    ];
    for (a, b) in cases {
        let cv = unsafe { cf(a, b, 0, 0) };
        let rv = unsafe { rf(a, b, 0, 0) };
        assert_eq!(cv, rv, "add_operation({},{})", a, b);
    }
}

#[test]
fn test_multiply_operation() {
    let (c, r) = load_libs();
    let cf: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
        unsafe { c.get(b"multiply_operation").unwrap() };
    let rf: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
        unsafe { r.get(b"multiply_operation").unwrap() };

    let cases: [(c_int, c_int); 7] = [
        (0, 0),
        (3, 4),
        (-3, 4),
        (-3, -4),
        (i32::MAX, 1),
        (i32::MAX, 2),
        (i32::MIN, -1),
    ];
    for (a, b) in cases {
        let cv = unsafe { cf(a, b, 0, 0) };
        let rv = unsafe { rf(a, b, 0, 0) };
        assert_eq!(cv, rv, "multiply_operation({},{})", a, b);
    }
}

#[test]
fn test_subtract_operation() {
    let (c, r) = load_libs();
    let cf: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
        unsafe { c.get(b"subtract_operation").unwrap() };
    let rf: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
        unsafe { r.get(b"subtract_operation").unwrap() };

    let cases: [(c_int, c_int); 6] = [
        (0, 0),
        (10, 3),
        (-10, 3),
        (i32::MIN, 1),
        (i32::MIN, -1),
        (i32::MAX, -1),
    ];
    for (a, b) in cases {
        let cv = unsafe { cf(a, b, 0, 0) };
        let rv = unsafe { rf(a, b, 0, 0) };
        assert_eq!(cv, rv, "subtract_operation({},{})", a, b);
    }
}

#[test]
fn test_modulo_operation() {
    let (c, r) = load_libs();
    let cf: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
        unsafe { c.get(b"modulo_operation").unwrap() };
    let rf: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
        unsafe { r.get(b"modulo_operation").unwrap() };

    // Skip INT_MIN % -1 (UB in C, our Rust translates to 0). Test other cases.
    let cases: [(c_int, c_int); 7] = [
        (10, 3),
        (10, -3),
        (-10, 3),
        (-10, -3),
        (5, 0),     // returns 0
        (i32::MAX, 7),
        (0, 5),
    ];
    for (a, b) in cases {
        let cv = unsafe { cf(a, b, 0, 0) };
        let rv = unsafe { rf(a, b, 0, 0) };
        assert_eq!(cv, rv, "modulo_operation({},{})", a, b);
    }
}

#[test]
fn test_safe_double_to_int() {
    let (c, r) = load_libs();
    let cf: Symbol<unsafe extern "C" fn(f64) -> c_int> =
        unsafe { c.get(b"safe_double_to_int").unwrap() };
    let rf: Symbol<unsafe extern "C" fn(f64) -> c_int> =
        unsafe { r.get(b"safe_double_to_int").unwrap() };

    let cases: [f64; 12] = [
        0.0,
        1.0,
        -1.0,
        1.5,
        -1.5,
        1e10,
        -1e10,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        i32::MAX as f64,
        i32::MIN as f64,
    ];
    for d in cases {
        let cv = unsafe { cf(d) };
        let rv = unsafe { rf(d) };
        assert_eq!(cv, rv, "safe_double_to_int({})", d);
    }
}

#[test]
fn test_compute_scaled_value() {
    let (c, r) = load_libs();
    let cf: Symbol<unsafe extern "C" fn(c_int, f64) -> c_int> =
        unsafe { c.get(b"compute_scaled_value").unwrap() };
    let rf: Symbol<unsafe extern "C" fn(c_int, f64) -> c_int> =
        unsafe { r.get(b"compute_scaled_value").unwrap() };

    let cases: [(c_int, f64); 7] = [
        (0, 1.0),
        (10, 1.5),
        (-10, 1.5),
        (100, 0.0),
        (i32::MAX, 1.0),
        (i32::MAX, 2.0),
        (i32::MIN, 2.0),
    ];
    for (b, s) in cases {
        let cv = unsafe { cf(b, s) };
        let rv = unsafe { rf(b, s) };
        assert_eq!(cv, rv, "compute_scaled_value({},{})", b, s);
    }
}

#[test]
fn test_init_result_array() {
    let (c, r) = load_libs();
    let cf: Symbol<unsafe extern "C" fn(*mut ResultArray, *mut c_int, c_int)> =
        unsafe { c.get(b"init_result_array").unwrap() };
    let rf: Symbol<unsafe extern "C" fn(*mut ResultArray, *mut c_int, c_int)> =
        unsafe { r.get(b"init_result_array").unwrap() };

    for &count in &[0i32, 1, 4, 8, 10, 15] {
        let mut values: Vec<c_int> = (0..count.max(0)).map(|i| (i + 1) * 7 - 3).collect();
        if values.is_empty() {
            values.push(0);
        }

        let mut arr_c = empty_array();
        let mut arr_r = empty_array();

        unsafe {
            cf(&mut arr_c, values.as_mut_ptr(), count);
            rf(&mut arr_r, values.as_mut_ptr(), count);
        }

        assert_eq!(arr_c.count, arr_r.count, "init count for count={}", count);
        assert!(array_eq(&arr_c, &arr_r), "init mismatch for count={}", count);
    }
}

fn build_array(libc: &Library, values: &mut [c_int]) -> ResultArray {
    let cf: Symbol<unsafe extern "C" fn(*mut ResultArray, *mut c_int, c_int)> =
        unsafe { libc.get(b"init_result_array").unwrap() };
    let mut arr = empty_array();
    unsafe { cf(&mut arr, values.as_mut_ptr(), values.len() as c_int) };
    arr
}

#[test]
fn test_compare_results_in_array() {
    let (c, r) = load_libs();
    let cf: Symbol<unsafe extern "C" fn(*mut ResultArray, c_int, c_int) -> c_int> =
        unsafe { c.get(b"compare_results_in_array").unwrap() };
    let rf: Symbol<unsafe extern "C" fn(*mut ResultArray, c_int, c_int) -> c_int> =
        unsafe { r.get(b"compare_results_in_array").unwrap() };

    let mut values: [c_int; 5] = [1, 2, 3, 4, 5];
    let mut arr_c = build_array(&c, &mut values);
    let mut arr_r = build_array(&r, &mut values);

    for &(i, j) in &[(0i32, 0i32), (0, 1), (1, 0), (2, 4), (4, 4), (5, 0), (0, 5), (3, 4)] {
        let cv = unsafe { cf(&mut arr_c, i, j) };
        let rv = unsafe { rf(&mut arr_r, i, j) };
        assert_eq!(cv, rv, "compare({},{})", i, j);
    }
}

#[test]
fn test_process_with_foreach() {
    let (c, r) = load_libs();
    let cf: Symbol<unsafe extern "C" fn(*mut ResultArray, OperationFunc) -> c_int> =
        unsafe { c.get(b"process_with_foreach").unwrap() };
    let rf: Symbol<unsafe extern "C" fn(*mut ResultArray, OperationFunc) -> c_int> =
        unsafe { r.get(b"process_with_foreach").unwrap() };

    // Use the same op symbol from C for both — this gives a stable function pointer.
    // Both libs receive the same fn pointer; behaviour shouldn't differ.
    let c_op: Symbol<OperationFunc> = unsafe { c.get(b"add_operation").unwrap() };
    let op = *c_op;

    let mut values: [c_int; 6] = [1, 2, 3, 4, 5, 6];
    let mut arr_c = build_array(&c, &mut values);
    let mut arr_r = build_array(&r, &mut values);

    let cv = unsafe { cf(&mut arr_c, op) };
    let rv = unsafe { rf(&mut arr_r, op) };
    assert_eq!(cv, rv, "process_with_foreach total");
    assert!(array_eq(&arr_c, &arr_r), "process_with_foreach mutation");
}

#[test]
fn test_compute_weighted_sum() {
    let (c, r) = load_libs();
    let cf: Symbol<unsafe extern "C" fn(*mut ResultArray) -> c_int> =
        unsafe { c.get(b"compute_weighted_sum").unwrap() };
    let rf: Symbol<unsafe extern "C" fn(*mut ResultArray) -> c_int> =
        unsafe { r.get(b"compute_weighted_sum").unwrap() };

    let mut values: [c_int; 5] = [10, 20, 30, 40, 50];
    let mut arr_c = build_array(&c, &mut values);
    let mut arr_r = build_array(&r, &mut values);

    let cv = unsafe { cf(&mut arr_c) };
    let rv = unsafe { rf(&mut arr_r) };
    assert_eq!(cv, rv, "compute_weighted_sum");
}

#[test]
fn test_arrayfunc_basic() {
    let (c, r) = load_libs();
    let cf: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
        unsafe { c.get(b"arrayfunc").unwrap() };
    let rf: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
        unsafe { r.get(b"arrayfunc").unwrap() };

    let cases: [(c_int, c_int, c_int, c_int); 12] = [
        (0, 0, 0, 0),
        (1, 2, 3, 4),
        (-1, -2, -3, -4),
        (10, 20, 30, 40),
        (-10, 5, -3, 7),
        (100, 200, 300, 400),
        (1, 1, 1, 1),
        (5, -5, 5, -5),
        (1000, 2, 3, 5),
        (7, 11, 13, 17),
        (-100, 100, -100, 100),
        (50, 25, 5, 2),
    ];

    for (a, b, c_, d) in cases {
        let cv = unsafe { cf(a, b, c_, d) };
        let rv = unsafe { rf(a, b, c_, d) };
        assert_eq!(cv, rv, "arrayfunc({},{},{},{})", a, b, c_, d);
    }
}

#[test]
fn test_arrayfunc_random() {
    // Deterministic pseudo-random sweep to cover broader inputs.
    let (c, r) = load_libs();
    let cf: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
        unsafe { c.get(b"arrayfunc").unwrap() };
    let rf: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
        unsafe { r.get(b"arrayfunc").unwrap() };

    let mut state: u64 = 0xdeadbeefcafebabe;
    let mut next = || {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (state >> 32) as i32
    };

    for _ in 0..200 {
        // Use modest values to limit overflow effects.
        let a = next() % 1000;
        let b = next() % 1000;
        let c_ = next() % 1000;
        let d = next() % 1000;
        let cv = unsafe { cf(a, b, c_, d) };
        let rv = unsafe { rf(a, b, c_, d) };
        assert_eq!(cv, rv, "arrayfunc({},{},{},{})", a, b, c_, d);
    }
}

#[test]
fn test_symbols_match() {
    use std::process::Command;

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_so = manifest.join("c_src/build/libtranslated_rust.so");
    let r_so = if manifest.join("target/release/libarrayfunc_lib.so").exists() {
        manifest.join("target/release/libarrayfunc_lib.so")
    } else {
        manifest.join("target/debug/libarrayfunc_lib.so")
    };

    let extract = |path: &PathBuf| {
        let out = Command::new("nm")
            .arg("-D")
            .arg("--defined-only")
            .arg(path)
            .output()
            .expect("nm");
        let s = String::from_utf8_lossy(&out.stdout).to_string();
        let mut names: Vec<String> = s
            .lines()
            .filter_map(|l| {
                let parts: Vec<&str> = l.split_whitespace().collect();
                if parts.len() == 3 && parts[1] == "T" {
                    let name = parts[2];
                    // skip linker-emitted internals
                    if name.starts_with('_') {
                        None
                    } else {
                        Some(name.to_string())
                    }
                } else {
                    None
                }
            })
            .collect();
        names.sort();
        names.dedup();
        names
    };

    let c_syms = extract(&c_so);
    let r_syms = extract(&r_so);

    for s in &c_syms {
        assert!(
            r_syms.contains(s),
            "Rust .so missing symbol exported by C .so: {}\nC: {:?}\nRust: {:?}",
            s,
            c_syms,
            r_syms
        );
    }
}
