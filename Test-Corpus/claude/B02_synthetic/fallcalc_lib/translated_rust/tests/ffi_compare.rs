// Integration test: load BOTH the C .so and the Rust .so via libloading
// and compare their outputs through the FFI boundary for every public symbol.

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::PathBuf;

fn c_so_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("c_src/build/libtranslated_rust.so");
    p
}

fn rust_so_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target/release/libfallcalc_lib.so");
    p
}

fn load_libs() -> (Library, Library) {
    let c_lib = unsafe { Library::new(c_so_path()).expect("load C .so") };
    let r_lib = unsafe { Library::new(rust_so_path()).expect("load Rust .so") };
    (c_lib, r_lib)
}

// --- safe_double_to_int ---------------------------------------------------

type SafeDoubleToInt = unsafe extern "C" fn(f64) -> c_int;

#[test]
fn test_safe_double_to_int() {
    let (c_lib, r_lib) = load_libs();
    let c_fn: Symbol<SafeDoubleToInt> = unsafe { c_lib.get(b"safe_double_to_int").unwrap() };
    let r_fn: Symbol<SafeDoubleToInt> = unsafe { r_lib.get(b"safe_double_to_int").unwrap() };

    let inputs: Vec<f64> = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        2.5,
        -3.7,
        100.999,
        -100.999,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        i32::MAX as f64,
        i32::MIN as f64,
        (i32::MAX as f64) + 1.0,
        (i32::MIN as f64) - 1.0,
        (i32::MAX as f64) * 2.0,
        (i32::MIN as f64) * 2.0,
        1e10,
        -1e10,
        1e-10,
        -1e-10,
    ];

    for &d in &inputs {
        let c_out = unsafe { c_fn(d) };
        let r_out = unsafe { r_fn(d) };
        assert_eq!(c_out, r_out, "mismatch for input {:?}", d);
    }
}

// --- process_array_reverse ------------------------------------------------

type ProcessArrayReverse = unsafe extern "C" fn(*const c_int, c_int) -> c_int;

#[test]
fn test_process_array_reverse() {
    let (c_lib, r_lib) = load_libs();
    let c_fn: Symbol<ProcessArrayReverse> = unsafe { c_lib.get(b"process_array_reverse").unwrap() };
    let r_fn: Symbol<ProcessArrayReverse> = unsafe { r_lib.get(b"process_array_reverse").unwrap() };

    let arrays: Vec<Vec<c_int>> = vec![
        vec![1, 2, 3, 4, 5],
        vec![-1, -2, -3, -4, -5],
        vec![0, 0, 0, 0, 0],
        vec![100, 200, 300],
        vec![i32::MAX, 1, 1],
        vec![i32::MIN, -1, -1],
        vec![42],
        vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100],
    ];

    for arr in &arrays {
        for count in 0..=arr.len() as c_int {
            let end = unsafe { arr.as_ptr().add(arr.len() - 1) };
            let c_out = unsafe { c_fn(end, count) };
            let r_out = unsafe { r_fn(end, count) };
            assert_eq!(
                c_out, r_out,
                "mismatch for arr={:?}, count={}",
                arr, count
            );
        }
    }
}

// --- switch_fallthrough_calculator ----------------------------------------

type SwitchFallthrough = unsafe extern "C" fn(c_int, c_int) -> c_int;

#[test]
fn test_switch_fallthrough_calculator() {
    let (c_lib, r_lib) = load_libs();
    let c_fn: Symbol<SwitchFallthrough> =
        unsafe { c_lib.get(b"switch_fallthrough_calculator").unwrap() };
    let r_fn: Symbol<SwitchFallthrough> =
        unsafe { r_lib.get(b"switch_fallthrough_calculator").unwrap() };

    let values: Vec<c_int> = vec![
        0,
        1,
        -1,
        100,
        -100,
        1000,
        -1000,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        12345,
        -12345,
    ];
    let ops: Vec<c_int> = vec![-1, 0, 1, 2, 3, 4, 5, 6, 100];

    for &v in &values {
        for &op in &ops {
            let c_out = unsafe { c_fn(v, op) };
            let r_out = unsafe { r_fn(v, op) };
            assert_eq!(c_out, r_out, "mismatch for value={}, operation={}", v, op);
        }
    }
}

// --- allocate_and_compute -------------------------------------------------

type AllocateAndCompute = unsafe extern "C" fn(c_int, f64) -> c_int;

#[test]
fn test_allocate_and_compute() {
    let (c_lib, r_lib) = load_libs();
    let c_fn: Symbol<AllocateAndCompute> = unsafe { c_lib.get(b"allocate_and_compute").unwrap() };
    let r_fn: Symbol<AllocateAndCompute> = unsafe { r_lib.get(b"allocate_and_compute").unwrap() };

    // Avoid size 0 because malloc(0) is implementation-defined; the C code
    // handles a NULL return as -1 but POSIX malloc(0) may also return non-NULL,
    // so the result depends on libc. Skip that one.
    let sizes: Vec<c_int> = vec![1, 2, 3, 5, 10, 50, 100];
    let multipliers: Vec<f64> = vec![0.0, 1.0, 1.5, -1.5, 2.5, 0.1, -0.1, 100.0];

    for &s in &sizes {
        for &m in &multipliers {
            let c_out = unsafe { c_fn(s, m) };
            let r_out = unsafe { r_fn(s, m) };
            assert_eq!(c_out, r_out, "mismatch for size={}, multiplier={}", s, m);
        }
    }
}

// --- foreach_sum ----------------------------------------------------------

type ForeachSum = unsafe extern "C" fn(*const c_int, c_int) -> c_int;

#[test]
fn test_foreach_sum() {
    let (c_lib, r_lib) = load_libs();
    let c_fn: Symbol<ForeachSum> = unsafe { c_lib.get(b"foreach_sum").unwrap() };
    let r_fn: Symbol<ForeachSum> = unsafe { r_lib.get(b"foreach_sum").unwrap() };

    let arrays: Vec<Vec<c_int>> = vec![
        vec![1, 2, 3, 4, 5],
        vec![-1, -2, -3, -4, -5],
        vec![0],
        vec![i32::MAX, 1],
        vec![i32::MIN, -1],
        vec![100; 20],
    ];

    for arr in &arrays {
        for count in 0..=arr.len() as c_int {
            let c_out = unsafe { c_fn(arr.as_ptr(), count) };
            let r_out = unsafe { r_fn(arr.as_ptr(), count) };
            assert_eq!(
                c_out, r_out,
                "mismatch for arr={:?}, count={}",
                arr, count
            );
        }
    }
}

// --- fallcalc -------------------------------------------------------------

type Fallcalc = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

#[test]
fn test_fallcalc() {
    let (c_lib, r_lib) = load_libs();
    let c_fn: Symbol<Fallcalc> = unsafe { c_lib.get(b"fallcalc").unwrap() };
    let r_fn: Symbol<Fallcalc> = unsafe { r_lib.get(b"fallcalc").unwrap() };

    let test_vals: Vec<c_int> = vec![0, 1, -1, 2, -2, 5, 10, 100, 200, -100, 1000, -1000];

    for &p1 in &test_vals {
        for &p2 in &test_vals {
            for &p3 in &test_vals {
                for &p4 in &test_vals {
                    let c_out = unsafe { c_fn(p1, p2, p3, p4) };
                    let r_out = unsafe { r_fn(p1, p2, p3, p4) };
                    assert_eq!(
                        c_out, r_out,
                        "mismatch for fallcalc({}, {}, {}, {})",
                        p1, p2, p3, p4
                    );
                }
            }
        }
    }
}

#[test]
fn test_fallcalc_extreme() {
    let (c_lib, r_lib) = load_libs();
    let c_fn: Symbol<Fallcalc> = unsafe { c_lib.get(b"fallcalc").unwrap() };
    let r_fn: Symbol<Fallcalc> = unsafe { r_lib.get(b"fallcalc").unwrap() };

    // Use values that don't trigger malloc with extreme sizes (param4 % 10 + 1 is bounded).
    let extremes: Vec<c_int> = vec![
        0,
        1,
        -1,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        100000,
        -100000,
    ];

    for &p1 in &extremes {
        for &p2 in &extremes {
            for &p3 in &extremes {
                for &p4 in &extremes {
                    let c_out = unsafe { c_fn(p1, p2, p3, p4) };
                    let r_out = unsafe { r_fn(p1, p2, p3, p4) };
                    assert_eq!(
                        c_out, r_out,
                        "mismatch for fallcalc({}, {}, {}, {})",
                        p1, p2, p3, p4
                    );
                }
            }
        }
    }
}

// --- symbol export check --------------------------------------------------

#[test]
fn test_all_c_symbols_exported_by_rust() {
    let (c_lib, r_lib) = load_libs();
    // Functions exported by the C .so that the Rust .so must also expose.
    let names: Vec<&[u8]> = vec![
        b"safe_double_to_int",
        b"process_array_reverse",
        b"switch_fallthrough_calculator",
        b"allocate_and_compute",
        b"foreach_sum",
        b"fallcalc",
    ];
    for name in names {
        let _: Symbol<unsafe extern "C" fn()> =
            unsafe { c_lib.get(name).expect("C lib missing symbol") };
        let _: Symbol<unsafe extern "C" fn()> =
            unsafe { r_lib.get(name).expect("Rust lib missing symbol") };
    }
}
