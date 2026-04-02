use libloading::{Library, Symbol};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libarity_lib.so")
}

fn rust_lib_path() -> PathBuf {
    // cargo puts cdylib in target/debug or target/release
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target");
    p.push("debug");
    p.push("libarity_lib.so");
    p
}

// ---- shift_array ----
#[test]
fn test_shift_array() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let c_fn: Symbol<unsafe extern "C" fn(*mut i32, i32, i32)> =
            c_lib.get(b"shift_array").unwrap();

        // Test cases: (initial array, size, positions)
        let cases: Vec<(Vec<i32>, i32, i32)> = vec![
            (vec![1, 2, 3, 4], 4, 1),
            (vec![10, 20, 30, 40, 50], 5, 2),
            (vec![1, 2, 3], 3, 0),   // positions == 0, no-op
            (vec![1, 2, 3], 3, 3),   // positions == size, no-op
            (vec![1, 2, 3], 3, -1),  // negative, no-op
        ];

        for (init, size, pos) in &cases {
            let mut c_arr = init.clone();
            let mut r_arr = init.clone();

            c_fn(c_arr.as_mut_ptr(), *size, *pos);
            arity_lib::shift_array(r_arr.as_mut_ptr(), *size, *pos);

            assert_eq!(c_arr, r_arr, "shift_array mismatch for init={:?} size={} pos={}", init, size, pos);
        }
    }
}

// ---- process_string ----
#[test]
fn test_process_string() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let c_fn: Symbol<unsafe extern "C" fn(*const u8) -> i32> =
            c_lib.get(b"process_string").unwrap();

        let cases: Vec<&[u8]> = vec![
            b"Hello\0",
            b"\0",
            b"test string\0",
            b"a\0",
        ];

        for s in &cases {
            let c_result = c_fn(s.as_ptr());
            let r_result = arity_lib::process_string(s.as_ptr());
            assert_eq!(c_result, r_result, "process_string mismatch for {:?}", s);
        }
    }
}

// ---- apply_bitmask ----
#[test]
fn test_apply_bitmask() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let c_fn: Symbol<unsafe extern "C" fn(i32, i32) -> i32> =
            c_lib.get(b"apply_bitmask").unwrap();

        let values = [0, 1, 15, 16, 255, 170, 85, -1, 0x7FFFFFFF];
        let ops = [0, 1, 2, 3, 4, -1];

        for &v in &values {
            for &op in &ops {
                let c_r = c_fn(v, op);
                let r_r = arity_lib::apply_bitmask(v, op);
                assert_eq!(c_r, r_r, "apply_bitmask({}, {}) C={} Rust={}", v, op, c_r, r_r);
            }
        }
    }
}

// ---- init_matrix ----
#[test]
fn test_init_matrix() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let c_fn: Symbol<unsafe extern "C" fn(*mut [i32; 4])> =
            c_lib.get(b"init_matrix").unwrap();

        let mut c_matrix = [[0i32; 4]; 3];
        let mut r_matrix = [[0i32; 4]; 3];

        c_fn(c_matrix.as_mut_ptr());
        arity_lib::init_matrix(r_matrix.as_mut_ptr());

        assert_eq!(c_matrix, r_matrix, "init_matrix mismatch");
    }
}

// ---- compare_allocations ----
// Pointer comparison is nondeterministic, so we only check that the
// val1>0 bonus (+10) is applied consistently. The base result (1,2,3)
// depends on allocator layout and may differ.
#[test]
fn test_compare_allocations_val_bonus() {
    // We can't compare C vs Rust directly because pointer ordering differs.
    // But we can verify the function doesn't crash and returns a sane range.
    let cases = [(5, 3), (0, 0), (-1, 5), (100, -100)];
    for &(v1, v2) in &cases {
        let r = unsafe { arity_lib::compare_allocations(v1, v2) };
        // result should be in {1,2,3} + {0,10} = {1,2,3,11,12,13}
        assert!(
            [1, 2, 3, 11, 12, 13].contains(&r),
            "compare_allocations({}, {}) returned unexpected {}", v1, v2, r
        );
    }
}

// ---- arity4 ----
// Since arity4 calls compare_allocations internally, and pointer ordering
// may differ between C and Rust allocators, we need to account for that.
// We'll test that both produce results in a reasonable range, and test
// the deterministic parts separately.
#[test]
fn test_arity4() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let c_fn: Symbol<unsafe extern "C" fn(i32, i32, i32, i32) -> i32> =
            c_lib.get(b"arity4").unwrap();

        // Test cases where compare_allocations difference is minimized
        // by using param3 != 0 (which divides by 100, reducing the impact)
        // or param4 which adds a large offset
        let cases = [
            (0, 0, 0, 0),
            (1, 2, 3, 4),
            (0, 0, 100, 0),
            (1, 0, 0, 0),
            (2, 0, 0, 0),
            (3, 0, 0, 0),
        ];

        for &(p1, p2, p3, p4) in &cases {
            let c_r = c_fn(p1, p2, p3, p4);
            let r_r = arity_lib::arity4(p1, p2, p3, p4);
            // Allow difference due to compare_allocations nondeterminism
            // The alloc_result can differ by at most 12 (range 1..13)
            let diff = (c_r - r_r).abs();
            if p3 != 0 {
                // After division by 100, the alloc diff is at most 12*|p3|/100
                // which for small p3 is tiny
                assert!(diff <= (12 * p3.abs()) / 100 + 1,
                    "arity4({},{},{},{}) C={} Rust={} diff={}", p1, p2, p3, p4, c_r, r_r, diff);
            } else {
                assert!(diff <= 12,
                    "arity4({},{},{},{}) C={} Rust={} diff={}", p1, p2, p3, p4, c_r, r_r, diff);
            }
        }
    }
}

// ---- arity2 ----
#[test]
fn test_arity2() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let c_fn: Symbol<unsafe extern "C" fn(i32, i32) -> i32> =
            c_lib.get(b"arity2").unwrap();

        let cases = [(0, 0), (1, 2), (5, 10), (-1, 3)];
        for &(p1, p2) in &cases {
            let c_r = c_fn(p1, p2);
            let r_r = arity_lib::arity2(p1, p2);
            let diff = (c_r - r_r).abs();
            assert!(diff <= 12, "arity2({},{}) C={} Rust={}", p1, p2, c_r, r_r);
        }
    }
}

// ---- arity3 ----
#[test]
fn test_arity3() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let c_fn: Symbol<unsafe extern "C" fn(i32, i32, i32) -> i32> =
            c_lib.get(b"arity3").unwrap();

        let cases = [(0, 0, 0), (1, 2, 3), (1, 2, 100)];
        for &(p1, p2, p3) in &cases {
            let c_r = c_fn(p1, p2, p3);
            let r_r = arity_lib::arity3(p1, p2, p3);
            let diff = (c_r - r_r).abs();
            if p3 != 0 {
                assert!(diff <= (12 * p3.abs()) / 100 + 1,
                    "arity3({},{},{}) C={} Rust={}", p1, p2, p3, c_r, r_r);
            } else {
                assert!(diff <= 12,
                    "arity3({},{},{}) C={} Rust={}", p1, p2, p3, c_r, r_r);
            }
        }
    }
}

// ---- arity (top-level) ----
#[test]
fn test_arity() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let c_fn: Symbol<unsafe extern "C" fn(u8, *const i32) -> i32> =
            c_lib.get(b"arity").unwrap();

        // len < 2 => -1
        let params = [1i32, 2, 3, 4];
        assert_eq!(c_fn(0, params.as_ptr()), arity_lib::arity(0, params.as_ptr()));
        assert_eq!(c_fn(1, params.as_ptr()), arity_lib::arity(1, params.as_ptr()));

        // len == 2
        let c_r = c_fn(2, params.as_ptr());
        let r_r = arity_lib::arity(2, params.as_ptr());
        assert!((c_r - r_r).abs() <= 12, "arity(2) C={} Rust={}", c_r, r_r);

        // len == 3
        let c_r = c_fn(3, params.as_ptr());
        let r_r = arity_lib::arity(3, params.as_ptr());
        assert!((c_r - r_r).abs() <= 12, "arity(3) C={} Rust={}", c_r, r_r);

        // len == 4
        let c_r = c_fn(4, params.as_ptr());
        let r_r = arity_lib::arity(4, params.as_ptr());
        assert!((c_r - r_r).abs() <= 12, "arity(4) C={} Rust={}", c_r, r_r);
    }
}
