use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    // The cdylib is built in the deps dir or directly in target/debug
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug");
    dir.join("libhatch_lib.so")
}

#[repr(C)]
struct DataRecord {
    id: c_int,
    value: c_int,
    timestamp: i64, // time_t on linux x86_64
    name: [u8; 32],
}

/// Load both libraries fresh (each gets its own global state initialized to 0).
fn load_libs() -> (Library, Library) {
    unsafe {
        let c = Library::new(c_lib_path()).expect("load C lib");
        let r = Library::new(rust_lib_path()).expect("load Rust lib");
        (c, r)
    }
}

#[test]
fn test_add_three() {
    let (c, r) = load_libs();
    unsafe {
        let c_fn: Symbol<extern "C" fn(c_int, c_int, c_int) -> c_int> = c.get(b"add_three").unwrap();
        let r_fn: Symbol<extern "C" fn(c_int, c_int, c_int) -> c_int> = r.get(b"add_three").unwrap();
        for &(a, b, c_val) in &[(1,2,3), (0,0,0), (-1,5,10), (i32::MAX,0,0), (100,-200,300)] {
            assert_eq!(c_fn(a, b, c_val), r_fn(a, b, c_val), "add_three({a},{b},{c_val})");
        }
    }
}

#[test]
fn test_multiply_add() {
    let (c, r) = load_libs();
    unsafe {
        let c_fn: Symbol<extern "C" fn(c_int, c_int, c_int) -> c_int> = c.get(b"multiply_add").unwrap();
        let r_fn: Symbol<extern "C" fn(c_int, c_int, c_int) -> c_int> = r.get(b"multiply_add").unwrap();
        for &(a, b, c_val) in &[(2,3,4), (0,5,1), (-3,7,2), (100000,100000,1)] {
            assert_eq!(c_fn(a, b, c_val), r_fn(a, b, c_val), "multiply_add({a},{b},{c_val})");
        }
    }
}

#[test]
fn test_compute_with_dynamic_memory() {
    let (c, r) = load_libs();
    unsafe {
        let c_fn: Symbol<extern "C" fn(c_int, c_int) -> c_int> = c.get(b"compute_with_dynamic_memory").unwrap();
        let r_fn: Symbol<extern "C" fn(c_int, c_int) -> c_int> = r.get(b"compute_with_dynamic_memory").unwrap();
        for &(base, count) in &[(1,8), (0,1), (10,5), (100,20)] {
            assert_eq!(c_fn(base, count), r_fn(base, count), "compute_with_dynamic_memory({base},{count})");
        }
    }
}

#[test]
fn test_shift_array_data() {
    let (c, r) = load_libs();
    unsafe {
        let c_fn: Symbol<extern "C" fn(*mut c_int, c_int, c_int)> = c.get(b"shift_array_data").unwrap();
        let r_fn: Symbol<extern "C" fn(*mut c_int, c_int, c_int)> = r.get(b"shift_array_data").unwrap();
        for shift in [0, 1, 3, 9] {
            let mut c_arr = [10,20,30,40,50,60,70,80,90,100i32];
            let mut r_arr = c_arr;
            c_fn(c_arr.as_mut_ptr(), 10, shift);
            r_fn(r_arr.as_mut_ptr(), 10, shift);
            assert_eq!(c_arr, r_arr, "shift_array_data shift={shift}");
        }
    }
}

#[test]
fn test_increment_counter_and_complex_calc() {
    // Tests increment_counter + complex_calc together since complex_calc reads global_counter
    let (c, r) = load_libs();
    unsafe {
        let c_inc: Symbol<extern "C" fn(c_int, c_int)> = c.get(b"increment_counter").unwrap();
        let r_inc: Symbol<extern "C" fn(c_int, c_int)> = r.get(b"increment_counter").unwrap();
        let c_cc: Symbol<extern "C" fn(c_int, c_int, c_int) -> c_int> = c.get(b"complex_calc").unwrap();
        let r_cc: Symbol<extern "C" fn(c_int, c_int, c_int) -> c_int> = r.get(b"complex_calc").unwrap();

        // Increment counter by 5
        c_inc(5, 999);
        r_inc(5, 999);

        for &(a, b, cv) in &[(10, 3, 2), (0, 0, 1), (-5, 5, 3)] {
            assert_eq!(c_cc(a, b, cv), r_cc(a, b, cv), "complex_calc({a},{b},{cv}) after inc(5)");
        }
    }
}

#[test]
fn test_update_accumulator_and_process_pointer_data() {
    // Tests update_accumulator + process_pointer_data since it reads global_accumulator
    let (c, r) = load_libs();
    unsafe {
        let c_upd: Symbol<extern "C" fn(c_int, c_int)> = c.get(b"update_accumulator").unwrap();
        let r_upd: Symbol<extern "C" fn(c_int, c_int)> = r.get(b"update_accumulator").unwrap();
        let c_ppd: Symbol<extern "C" fn(*const c_int, c_int) -> c_int> = c.get(b"process_pointer_data").unwrap();
        let r_ppd: Symbol<extern "C" fn(*const c_int, c_int) -> c_int> = r.get(b"process_pointer_data").unwrap();

        c_upd(7, 888);
        r_upd(7, 888);

        let val: c_int = 42;
        for &mult in &[1, 2, -3, 0] {
            assert_eq!(c_ppd(&val, mult), r_ppd(&val, mult), "process_pointer_data(42, {mult}) after upd(7)");
        }
    }
}

#[test]
fn test_apply_operation() {
    let (c, r) = load_libs();
    unsafe {
        type OpFn = extern "C" fn(c_int, c_int, c_int) -> c_int;
        let c_apply: Symbol<extern "C" fn(Option<OpFn>, c_int, c_int, c_int) -> c_int> = c.get(b"apply_operation").unwrap();
        let r_apply: Symbol<extern "C" fn(Option<OpFn>, c_int, c_int, c_int) -> c_int> = r.get(b"apply_operation").unwrap();
        let c_add: Symbol<OpFn> = c.get(b"add_three").unwrap();
        let r_add: Symbol<OpFn> = r.get(b"add_three").unwrap();

        // Use each lib's own add_three as the function pointer
        let c_res = c_apply(Some(*c_add), 1, 2, 3);
        let r_res = r_apply(Some(*r_add), 1, 2, 3);
        assert_eq!(c_res, r_res, "apply_operation(add_three, 1, 2, 3)");
    }
}

#[test]
fn test_get_time_based_value() {
    let (c, r) = load_libs();
    unsafe {
        let c_fn: Symbol<extern "C" fn(c_int) -> c_int> = c.get(b"get_time_based_value").unwrap();
        let r_fn: Symbol<extern "C" fn(c_int) -> c_int> = r.get(b"get_time_based_value").unwrap();
        // seed * 3600 / 100 + seed = seed * 37, so result is deterministic for given seed
        for &seed in &[0, 1, 5, 100] {
            let cv = c_fn(seed);
            let rv = r_fn(seed);
            assert_eq!(cv, rv, "get_time_based_value({seed})");
        }
    }
}

#[test]
fn test_manipulate_records() {
    let (c, r) = load_libs();
    unsafe {
        let c_fn: Symbol<extern "C" fn(*mut DataRecord, c_int, c_int) -> c_int> = c.get(b"manipulate_records").unwrap();
        let r_fn: Symbol<extern "C" fn(*mut DataRecord, c_int, c_int) -> c_int> = r.get(b"manipulate_records").unwrap();

        let make_records = || -> Vec<DataRecord> {
            (0..5).map(|i| DataRecord {
                id: i,
                value: 10 + i * 10,
                timestamp: 0,
                name: [0u8; 32],
            }).collect()
        };

        for &shift in &[0, 1, 2, 4] {
            let mut c_recs = make_records();
            let mut r_recs = make_records();
            let cv = c_fn(c_recs.as_mut_ptr(), 5, shift);
            let rv = r_fn(r_recs.as_mut_ptr(), 5, shift);
            assert_eq!(cv, rv, "manipulate_records(5, {shift})");
        }
    }
}

#[test]
fn test_hatch() {
    // hatch modifies global state, so we need fresh library loads
    let (c, r) = load_libs();
    unsafe {
        let c_fn: Symbol<extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> = c.get(b"hatch").unwrap();
        let r_fn: Symbol<extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> = r.get(b"hatch").unwrap();

        // Call both back-to-back to minimize time difference
        let cv = c_fn(1, 2, 3, 4);
        let rv = r_fn(1, 2, 3, 4);
        assert_eq!(cv, rv, "hatch(1,2,3,4)");
    }
}

#[test]
fn test_hatch_multiple_calls() {
    // Test that global state accumulates identically across multiple calls
    let (c, r) = load_libs();
    unsafe {
        let c_fn: Symbol<extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> = c.get(b"hatch").unwrap();
        let r_fn: Symbol<extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> = r.get(b"hatch").unwrap();

        for &(a, b, cv, d) in &[(1,2,3,4), (5,6,7,8), (0,0,0,0), (10,-5,3,100)] {
            let c_res = c_fn(a, b, cv, d);
            let r_res = r_fn(a, b, cv, d);
            assert_eq!(c_res, r_res, "hatch({a},{b},{cv},{d})");
        }
    }
}
