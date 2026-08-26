use libloading::Library;
use std::ffi::c_int;
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/libarity_lib.so")
}

macro_rules! load {
    ($lib:expr, $name:expr, $ty:ty) => {
        unsafe { $lib.get::<$ty>($name).expect(concat!("symbol: ", stringify!($name))) }
    };
}

// compare_allocations uses malloc pointer comparison which is non-deterministic.
// The pointer ordering (1/2/3) depends on allocator state and differs between
// C and Rust .so when loaded in the same process. The +10 bonus (val1 > 0)
// is deterministic. We verify the deterministic part matches and that the
// non-deterministic part is in the valid range.

#[test]
fn test_apply_bitmask() {
    let c = unsafe { Library::new(c_lib_path()) }.unwrap();
    let r = unsafe { Library::new(rust_lib_path()) }.unwrap();
    type F = unsafe extern "C" fn(c_int, c_int) -> c_int;
    let c_fn = load!(c, b"apply_bitmask", F);
    let r_fn = load!(r, b"apply_bitmask", F);

    for value in [0, 1, 15, 16, 127, 255, -1, 0xFF, 0xAA, 0x55] {
        for op in 0..=5 {
            let cv = unsafe { c_fn(value, op) };
            let rv = unsafe { r_fn(value, op) };
            assert_eq!(cv, rv, "apply_bitmask({value}, {op})");
        }
    }
}

#[test]
fn test_process_string() {
    let c = unsafe { Library::new(c_lib_path()) }.unwrap();
    let r = unsafe { Library::new(rust_lib_path()) }.unwrap();
    type F = unsafe extern "C" fn(*const i8) -> c_int;
    let c_fn = load!(c, b"process_string", F);
    let r_fn = load!(r, b"process_string", F);

    for s in [&b"\0"[..], b"Hello\0", b"A\0", b"test string\0"] {
        let cv = unsafe { c_fn(s.as_ptr() as *const i8) };
        let rv = unsafe { r_fn(s.as_ptr() as *const i8) };
        assert_eq!(cv, rv, "process_string");
    }
}

#[test]
fn test_shift_array() {
    let c = unsafe { Library::new(c_lib_path()) }.unwrap();
    let r = unsafe { Library::new(rust_lib_path()) }.unwrap();
    type F = unsafe extern "C" fn(*mut c_int, c_int, c_int);
    let c_fn = load!(c, b"shift_array", F);
    let r_fn = load!(r, b"shift_array", F);

    for positions in [0, 1, 2, 3, 5] {
        let mut c_arr = [10, 20, 30, 40];
        let mut r_arr = [10, 20, 30, 40];
        unsafe {
            c_fn(c_arr.as_mut_ptr(), 4, positions);
            r_fn(r_arr.as_mut_ptr(), 4, positions);
        }
        assert_eq!(c_arr, r_arr, "shift_array positions={positions}");
    }
}

#[test]
fn test_init_matrix() {
    let c = unsafe { Library::new(c_lib_path()) }.unwrap();
    let r = unsafe { Library::new(rust_lib_path()) }.unwrap();
    type F = unsafe extern "C" fn(*mut [[c_int; 4]; 3]);
    let c_fn = load!(c, b"init_matrix", F);
    let r_fn = load!(r, b"init_matrix", F);

    let mut c_mat = [[0i32; 4]; 3];
    let mut r_mat = [[0i32; 4]; 3];
    unsafe {
        c_fn(&mut c_mat);
        r_fn(&mut r_mat);
    }
    assert_eq!(c_mat, r_mat, "init_matrix");
}

#[test]
fn test_compare_allocations() {
    let c = unsafe { Library::new(c_lib_path()) }.unwrap();
    let r = unsafe { Library::new(rust_lib_path()) }.unwrap();
    type F = unsafe extern "C" fn(c_int, c_int) -> c_int;
    let c_fn = load!(c, b"compare_allocations", F);
    let r_fn = load!(r, b"compare_allocations", F);

    // Pointer comparison (1/2/3) is allocator-dependent and non-deterministic.
    // The +10 bonus depends on val1 > 0 and IS deterministic.
    for (v1, v2) in [(5, 10), (0, 0), (-1, 5), (100, -100)] {
        let cv = unsafe { c_fn(v1, v2) };
        let rv = unsafe { r_fn(v1, v2) };
        let c_base = cv % 10;
        let r_base = rv % 10;
        assert!((1..=3).contains(&c_base), "C base out of range: {cv}");
        assert!((1..=3).contains(&r_base), "R base out of range: {rv}");
        // Both should agree on the +10 bonus
        assert_eq!(cv / 10, rv / 10, "compare_allocations({v1},{v2}) bonus mismatch: c={cv} r={rv}");
    }
}

// arity4 calls compare_allocations internally, so its result includes a
// non-deterministic component (1 or 2 from pointer comparison).
// We verify the difference between C and Rust is at most 1 (the pointer
// comparison component) and that the deterministic parts match.
#[test]
fn test_arity4() {
    let c = unsafe { Library::new(c_lib_path()) }.unwrap();
    let r = unsafe { Library::new(rust_lib_path()) }.unwrap();
    type F = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
    let c_fn = load!(c, b"arity4", F);
    let r_fn = load!(r, b"arity4", F);

    let cases = [
        (0, 0, 0, 0),
        (1, 2, 3, 4),
        (0, 0, 1, 0),
        (0, 0, 0, 5),
        (2, 3, 50, 10),
        (3, 1, 100, 0),
        (-1, -2, 0, 0),
    ];
    for (p1, p2, p3, p4) in cases {
        let cv = unsafe { c_fn(p1, p2, p3, p4) };
        let rv = unsafe { r_fn(p1, p2, p3, p4) };
        // The non-deterministic compare_allocations contributes at most ±1
        // to the result (before any param3/param4 scaling).
        // For param3!=0, the difference gets scaled by param3/100.
        let max_diff = if p3 != 0 { (p3.abs() / 100).max(1) } else { 1 };
        assert!(
            (cv - rv).abs() <= max_diff,
            "arity4({p1},{p2},{p3},{p4}): c={cv} r={rv} diff={} max_diff={max_diff}",
            (cv - rv).abs()
        );
    }
}

#[test]
fn test_arity2() {
    let c = unsafe { Library::new(c_lib_path()) }.unwrap();
    let r = unsafe { Library::new(rust_lib_path()) }.unwrap();
    type F = unsafe extern "C" fn(c_int, c_int) -> c_int;
    let c_fn = load!(c, b"arity2", F);
    let r_fn = load!(r, b"arity2", F);

    for (p1, p2) in [(0, 0), (1, 2), (5, 10), (-1, 3)] {
        let cv = unsafe { c_fn(p1, p2) };
        let rv = unsafe { r_fn(p1, p2) };
        assert!((cv - rv).abs() <= 1, "arity2({p1},{p2}): c={cv} r={rv}");
    }
}

#[test]
fn test_arity3() {
    let c = unsafe { Library::new(c_lib_path()) }.unwrap();
    let r = unsafe { Library::new(rust_lib_path()) }.unwrap();
    type F = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;
    let c_fn = load!(c, b"arity3", F);
    let r_fn = load!(r, b"arity3", F);

    for (p1, p2, p3) in [(0, 0, 0), (1, 2, 3), (2, 5, 50)] {
        let cv = unsafe { c_fn(p1, p2, p3) };
        let rv = unsafe { r_fn(p1, p2, p3) };
        let max_diff = if p3 != 0 { (p3.abs() / 100).max(1) } else { 1 };
        assert!((cv - rv).abs() <= max_diff, "arity3({p1},{p2},{p3}): c={cv} r={rv}");
    }
}

#[test]
fn test_arity() {
    let c = unsafe { Library::new(c_lib_path()) }.unwrap();
    let r = unsafe { Library::new(rust_lib_path()) }.unwrap();
    type F = unsafe extern "C" fn(c_int, *mut c_int) -> c_int;
    let c_fn = load!(c, b"arity", F);
    let r_fn = load!(r, b"arity", F);

    // len < 2 => -1 (deterministic, no allocations)
    let mut params = [0i32; 4];
    for len in [0, 1] {
        let cv = unsafe { c_fn(len, params.as_mut_ptr()) };
        let rv = unsafe { r_fn(len, params.as_mut_ptr()) };
        assert_eq!(cv, rv, "arity(len={len}) short");
    }

    // len >= 2 calls arity2/3/4 which include non-deterministic compare_allocations
    let mut p = [3, 7, 0, 0];
    let cv = unsafe { c_fn(2, p.as_mut_ptr()) };
    let rv = unsafe { r_fn(2, p.as_mut_ptr()) };
    assert!((cv - rv).abs() <= 1, "arity(2, [3,7]): c={cv} r={rv}");

    let mut p = [1, 2, 50, 0];
    let cv = unsafe { c_fn(3, p.as_mut_ptr()) };
    let rv = unsafe { r_fn(3, p.as_mut_ptr()) };
    assert!((cv - rv).abs() <= 1, "arity(3, [1,2,50]): c={cv} r={rv}");

    let mut p = [2, 3, 50, 10];
    let cv = unsafe { c_fn(4, p.as_mut_ptr()) };
    let rv = unsafe { r_fn(4, p.as_mut_ptr()) };
    assert!((cv - rv).abs() <= 1, "arity(4, [2,3,50,10]): c={cv} r={rv}");
}
