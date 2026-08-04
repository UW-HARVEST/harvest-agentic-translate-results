// Integration test: compares the Rust .so against the C .so.
//
// Both libraries are loaded via libloading and the same exported symbols
// are called with identical inputs. Outputs (and side-effects observable
// through subsequent calls) must match byte-for-byte.

use libloading::{Library, Symbol};
use std::os::raw::c_int;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

// Tests that mutate the libraries' global state must run serially because
// each library has only one copy of `global_counter` / `global_accumulator`,
// and `update_accumulator` is order-sensitive (acc = acc * 2 + v).
fn state_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

// --- DataRecord layout, matching the C struct -------------------------------
#[repr(C)]
#[derive(Clone, Copy)]
struct DataRecord {
    id: c_int,
    value: c_int,
    timestamp: i64,
    name: [i8; 32],
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    workspace_root().join("c_src/build/libtranslated_rust.so")
}

fn rust_so_path() -> PathBuf {
    // dev-profile cdylib lives under target/debug. Since `cargo test` builds
    // the dev profile cdylib, the .so will be present alongside the test
    // binary's deps directory, but we use the canonical target/debug path.
    let p = workspace_root().join("target/debug/libhatch_lib.so");
    if p.exists() {
        return p;
    }
    workspace_root().join("target/release/libhatch_lib.so")
}

fn open_libs() -> (Library, Library) {
    let c_path = c_so_path();
    let r_path = rust_so_path();
    assert!(
        c_path.exists(),
        "C .so missing at {:?} — run cmake build first",
        c_path
    );
    assert!(
        r_path.exists(),
        "Rust .so missing at {:?} — run `cargo build` first",
        r_path
    );
    unsafe {
        let c = Library::new(&c_path).expect("load C .so");
        let r = Library::new(&r_path).expect("load Rust .so");
        (c, r)
    }
}

// ---------- low-level pure helpers ----------

#[test]
fn test_add_three() {
    let (c, r) = open_libs();
    unsafe {
        let cf: Symbol<unsafe extern "C" fn(c_int, c_int, c_int) -> c_int> =
            c.get(b"add_three").unwrap();
        let rf: Symbol<unsafe extern "C" fn(c_int, c_int, c_int) -> c_int> =
            r.get(b"add_three").unwrap();
        for &(a, b, cc) in &[
            (0, 0, 0),
            (1, 2, 3),
            (-1, 1, 0),
            (i32::MAX, 1, 0),
            (i32::MIN, -1, 0),
            (1234, -5678, 9012),
        ] {
            assert_eq!(cf(a, b, cc), rf(a, b, cc), "add_three({a},{b},{cc})");
        }
    }
}

#[test]
fn test_multiply_add() {
    let (c, r) = open_libs();
    unsafe {
        let cf: Symbol<unsafe extern "C" fn(c_int, c_int, c_int) -> c_int> =
            c.get(b"multiply_add").unwrap();
        let rf: Symbol<unsafe extern "C" fn(c_int, c_int, c_int) -> c_int> =
            r.get(b"multiply_add").unwrap();
        for &(a, b, cc) in &[
            (0, 0, 0),
            (3, 5, 7),
            (-2, 4, 1),
            (i32::MAX, 2, 0),
            (i32::MIN, 2, 0),
            (1000, 1000, -50),
        ] {
            assert_eq!(cf(a, b, cc), rf(a, b, cc), "multiply_add({a},{b},{cc})");
        }
    }
}

#[test]
fn test_increment_counter_and_complex_calc() {
    let _g = state_lock().lock().unwrap();
    // increment_counter writes to a per-library global; we exercise it then
    // observe the side-effect via complex_calc which reads global_counter.
    let (c, r) = open_libs();
    unsafe {
        let c_inc: Symbol<unsafe extern "C" fn(c_int, c_int)> =
            c.get(b"increment_counter").unwrap();
        let r_inc: Symbol<unsafe extern "C" fn(c_int, c_int)> =
            r.get(b"increment_counter").unwrap();
        let c_cc: Symbol<unsafe extern "C" fn(c_int, c_int, c_int) -> c_int> =
            c.get(b"complex_calc").unwrap();
        let r_cc: Symbol<unsafe extern "C" fn(c_int, c_int, c_int) -> c_int> =
            r.get(b"complex_calc").unwrap();

        // Apply the same sequence of increments to both libs.
        for &v in &[5, -2, 100, 7] {
            c_inc(v, 999);
            r_inc(v, 999);
        }
        // Now the counters in both libs should be the same.
        for &(a, b, cc) in &[(10, 3, 4), (-1, -2, -3), (7, 8, 9)] {
            assert_eq!(c_cc(a, b, cc), r_cc(a, b, cc), "complex_calc({a},{b},{cc})");
        }
    }
}

#[test]
fn test_update_accumulator_and_process_pointer_data() {
    let _g = state_lock().lock().unwrap();
    let (c, r) = open_libs();
    unsafe {
        let c_upd: Symbol<unsafe extern "C" fn(c_int, c_int)> =
            c.get(b"update_accumulator").unwrap();
        let r_upd: Symbol<unsafe extern "C" fn(c_int, c_int)> =
            r.get(b"update_accumulator").unwrap();
        let c_ppd: Symbol<unsafe extern "C" fn(*const c_int, c_int) -> c_int> =
            c.get(b"process_pointer_data").unwrap();
        let r_ppd: Symbol<unsafe extern "C" fn(*const c_int, c_int) -> c_int> =
            r.get(b"process_pointer_data").unwrap();

        for &v in &[1, 3, -2, 7] {
            c_upd(v, 888);
            r_upd(v, 888);
        }
        for &(val, mult) in &[(10, 3), (-5, 4), (100, -1)] {
            let ptr = &val as *const c_int;
            assert_eq!(c_ppd(ptr, mult), r_ppd(ptr, mult));
        }
    }
}

#[test]
fn test_apply_operation() {
    let (c, r) = open_libs();
    unsafe {
        let c_ap: Symbol<
            unsafe extern "C" fn(
                unsafe extern "C" fn(c_int, c_int, c_int) -> c_int,
                c_int,
                c_int,
                c_int,
            ) -> c_int,
        > = c.get(b"apply_operation").unwrap();
        let r_ap: Symbol<
            unsafe extern "C" fn(
                unsafe extern "C" fn(c_int, c_int, c_int) -> c_int,
                c_int,
                c_int,
                c_int,
            ) -> c_int,
        > = r.get(b"apply_operation").unwrap();

        // Use the C library's own add_three / multiply_add for both calls,
        // since apply_operation just invokes the function pointer it's
        // given. We just need to confirm both wrappers do that correctly.
        let c_add: Symbol<unsafe extern "C" fn(c_int, c_int, c_int) -> c_int> =
            c.get(b"add_three").unwrap();
        let r_add: Symbol<unsafe extern "C" fn(c_int, c_int, c_int) -> c_int> =
            r.get(b"add_three").unwrap();

        let cv = c_ap(*c_add, 1, 2, 3);
        let rv = r_ap(*r_add, 1, 2, 3);
        assert_eq!(cv, rv);

        let c_mul: Symbol<unsafe extern "C" fn(c_int, c_int, c_int) -> c_int> =
            c.get(b"multiply_add").unwrap();
        let r_mul: Symbol<unsafe extern "C" fn(c_int, c_int, c_int) -> c_int> =
            r.get(b"multiply_add").unwrap();
        let cv = c_ap(*c_mul, 5, 6, 7);
        let rv = r_ap(*r_mul, 5, 6, 7);
        assert_eq!(cv, rv);
    }
}

#[test]
fn test_compute_with_dynamic_memory() {
    let (c, r) = open_libs();
    unsafe {
        let cf: Symbol<unsafe extern "C" fn(c_int, c_int) -> c_int> =
            c.get(b"compute_with_dynamic_memory").unwrap();
        let rf: Symbol<unsafe extern "C" fn(c_int, c_int) -> c_int> =
            r.get(b"compute_with_dynamic_memory").unwrap();
        for &(b, n) in &[(0, 1), (5, 8), (100, 4), (-3, 6)] {
            assert_eq!(cf(b, n), rf(b, n), "compute_with_dynamic_memory({b},{n})");
        }
    }
}

#[test]
fn test_get_time_based_value() {
    let (c, r) = open_libs();
    unsafe {
        let cf: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            c.get(b"get_time_based_value").unwrap();
        let rf: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            r.get(b"get_time_based_value").unwrap();
        for &s in &[0, 1, 2, 5, 100, -1, -7] {
            // The C semantics simplify to seed * 37 (with int overflow).
            assert_eq!(cf(s), rf(s), "get_time_based_value({s})");
        }
    }
}

#[test]
fn test_shift_array_data() {
    let (c, r) = open_libs();
    unsafe {
        let cf: Symbol<unsafe extern "C" fn(*mut c_int, c_int, c_int)> =
            c.get(b"shift_array_data").unwrap();
        let rf: Symbol<unsafe extern "C" fn(*mut c_int, c_int, c_int)> =
            r.get(b"shift_array_data").unwrap();

        let cases: &[(Vec<c_int>, c_int, c_int)] = &[
            (vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10], 10, 3),
            (vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10], 10, 0),     // no-op
            (vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10], 10, 10),    // no-op
            (vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10], 10, 11),    // no-op
            (vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10], 10, -2),    // no-op
            (vec![5; 6], 6, 1),
        ];

        for (orig, size, shift) in cases {
            let mut a = orig.clone();
            let mut b = orig.clone();
            cf(a.as_mut_ptr(), *size, *shift);
            rf(b.as_mut_ptr(), *size, *shift);
            assert_eq!(a, b, "shift_array_data shift={shift}");
        }
    }
}

#[test]
fn test_manipulate_records() {
    let (c, r) = open_libs();
    unsafe {
        let cf: Symbol<unsafe extern "C" fn(*mut DataRecord, c_int, c_int) -> c_int> =
            c.get(b"manipulate_records").unwrap();
        let rf: Symbol<unsafe extern "C" fn(*mut DataRecord, c_int, c_int) -> c_int> =
            r.get(b"manipulate_records").unwrap();

        fn make_records(n: usize) -> Vec<DataRecord> {
            (0..n)
                .map(|i| DataRecord {
                    id: i as c_int,
                    value: 100 + i as c_int * 10,
                    timestamp: 0,
                    name: [0i8; 32],
                })
                .collect()
        }

        for &(n, shift) in &[(5usize, 2), (5, 0), (5, 5), (3, 1), (8, 4)] {
            let mut a = make_records(n);
            let mut b = make_records(n);
            let cv = cf(a.as_mut_ptr(), n as c_int, shift);
            let rv = rf(b.as_mut_ptr(), n as c_int, shift);
            assert_eq!(cv, rv, "manipulate_records n={n} shift={shift}");
            // Compare value arrays (ignore name buffers since they are zero anyway).
            let a_vals: Vec<_> = a.iter().map(|x| x.value).collect();
            let b_vals: Vec<_> = b.iter().map(|x| x.value).collect();
            assert_eq!(a_vals, b_vals, "records' values differ for shift={shift}");
        }
    }
}

#[test]
fn test_hatch_top_level() {
    let _g = state_lock().lock().unwrap();
    // Top-level entry point: state in each library is independent (they have
    // their own copy of the static globals), so calling hatch with the same
    // inputs on a fresh load should produce the same output.
    let (c, r) = open_libs();
    unsafe {
        let cf: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            c.get(b"hatch").unwrap();
        let rf: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            r.get(b"hatch").unwrap();

        for &(a, b, cc, d) in &[
            (1, 2, 3, 4),
            (0, 0, 0, 0),
            (10, 20, 30, 40),
            (-1, -2, -3, -4),
            (7, -3, 11, -5),
        ] {
            let cv = cf(a, b, cc, d);
            let rv = rf(a, b, cc, d);
            assert_eq!(cv, rv, "hatch({a},{b},{cc},{d})");
        }
    }
}
