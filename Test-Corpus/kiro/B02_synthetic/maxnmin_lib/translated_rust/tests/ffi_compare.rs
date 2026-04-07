use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::os::raw::c_char;
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target/debug/libmaxnmin_lib.so");
    p
}

// ---- safe_double_to_int ----
#[test]
fn test_safe_double_to_int() {
    unsafe {
        let c = Library::new(c_lib_path()).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(f64) -> c_int> = c.get(b"safe_double_to_int").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(f64) -> c_int> = r.get(b"safe_double_to_int").unwrap();

        let cases: &[f64] = &[
            0.0, 1.0, -1.0, 42.7, -42.7,
            f64::MAX, f64::MIN, f64::NAN, f64::INFINITY, f64::NEG_INFINITY,
            i32::MAX as f64, i32::MIN as f64,
            i32::MAX as f64 + 1.0, i32::MIN as f64 - 1.0,
            0.5, -0.5, 999999.999,
        ];
        for &val in cases {
            let c_res = c_fn(val);
            let r_res = r_fn(val);
            assert_eq!(c_res, r_res, "safe_double_to_int({val}) mismatch: C={c_res}, Rust={r_res}");
        }
    }
}

// ---- process_string ----
#[test]
fn test_process_string() {
    unsafe {
        let c = Library::new(c_lib_path()).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(*const c_char) -> c_int> = c.get(b"process_string").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*const c_char) -> c_int> = r.get(b"process_string").unwrap();

        let cases: &[&[u8]] = &[
            b"\0", b"hello\0", b"root\0", b"child1\0", b"A\0", b"ABCDEFGHIJ\0",
        ];
        for s in cases {
            let c_res = c_fn(s.as_ptr() as *const c_char);
            let r_res = r_fn(s.as_ptr() as *const c_char);
            assert_eq!(c_res, r_res, "process_string({:?}) mismatch: C={c_res}, Rust={r_res}", s);
        }
    }
}

// ---- maxnmin (the main function, exercises all internal functions) ----
#[test]
fn test_maxnmin() {
    unsafe {
        let c = Library::new(c_lib_path()).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> = c.get(b"maxnmin").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> = r.get(b"maxnmin").unwrap();

        let cases: &[(c_int, c_int, c_int, c_int)] = &[
            (0, 0, 0, 0),
            (1, 1, 1, 1),
            (1, 2, 3, 4),
            (5, 5, 5, 5),
            (6, 6, 6, 6),
            (0, 5, 2, 1),
            (3, 1, 4, 2),
            (100, 200, 50, 25),
            (-1, -2, -3, -4),
            (-6, 3, 0, 2),
            (0, 0, 0, 1),
            (0, 0, 0, 2),
            (11, 7, 3, 0),
            (i32::MAX, 0, 1, 1),
            (0, i32::MAX, 1, 1),
            (0, 0, i32::MAX, 0),
        ];
        for &(a, b, c_val, d) in cases {
            let c_res = c_fn(a, b, c_val, d);
            let r_res = r_fn(a, b, c_val, d);
            assert_eq!(c_res, r_res, "maxnmin({a},{b},{c_val},{d}) mismatch: C={c_res}, Rust={r_res}");
        }
    }
}

// ---- add_node + find_node_by_id + get_children_count + calculate_subtree_sum ----
// These use global state, so we test them through a sequence that mirrors the C behavior.
#[test]
fn test_node_functions() {
    unsafe {
        let c = Library::new(c_lib_path()).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();

        // First call maxnmin to reset node_count to 0 and populate nodes
        let c_maxnmin: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> = c.get(b"maxnmin").unwrap();
        let r_maxnmin: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> = r.get(b"maxnmin").unwrap();
        c_maxnmin(1, 1, 1, 1);
        r_maxnmin(1, 1, 1, 1);

        // Now both have the same 6 nodes loaded. Test get_children_count.
        let c_gcc: Symbol<unsafe extern "C" fn(c_int) -> c_int> = c.get(b"get_children_count").unwrap();
        let r_gcc: Symbol<unsafe extern "C" fn(c_int) -> c_int> = r.get(b"get_children_count").unwrap();
        for parent_id in -1..=7 {
            let c_res = c_gcc(parent_id);
            let r_res = r_gcc(parent_id);
            assert_eq!(c_res, r_res, "get_children_count({parent_id}) mismatch: C={c_res}, Rust={r_res}");
        }

        // Test calculate_subtree_sum
        let c_css: Symbol<unsafe extern "C" fn(c_int) -> f64> = c.get(b"calculate_subtree_sum").unwrap();
        let r_css: Symbol<unsafe extern "C" fn(c_int) -> f64> = r.get(b"calculate_subtree_sum").unwrap();
        for node_id in 0..=7 {
            let c_res = c_css(node_id);
            let r_res = r_css(node_id);
            assert!(
                c_res.to_bits() == r_res.to_bits(),
                "calculate_subtree_sum({node_id}) mismatch: C={c_res}, Rust={r_res}"
            );
        }

        // Test find_node_by_id returns non-null for ids 1-6, null for 0 and 7
        let c_find: Symbol<unsafe extern "C" fn(c_int) -> *mut u8> = c.get(b"find_node_by_id").unwrap();
        let r_find: Symbol<unsafe extern "C" fn(c_int) -> *mut u8> = r.get(b"find_node_by_id").unwrap();
        for id in 0..=7 {
            let c_res = c_find(id);
            let r_res = r_find(id);
            assert_eq!(
                c_res.is_null(), r_res.is_null(),
                "find_node_by_id({id}) null mismatch: C_null={}, Rust_null={}", c_res.is_null(), r_res.is_null()
            );
        }

        // Test add_node
        let c_add: Symbol<unsafe extern "C" fn(c_int, c_int, *const c_char, f64) -> c_int> = c.get(b"add_node").unwrap();
        let r_add: Symbol<unsafe extern "C" fn(c_int, c_int, *const c_char, f64) -> c_int> = r.get(b"add_node").unwrap();
        let name = b"test_node\0";
        let c_res = c_add(10, 1, name.as_ptr() as *const c_char, 99.9);
        let r_res = r_add(10, 1, name.as_ptr() as *const c_char, 99.9);
        assert_eq!(c_res, r_res, "add_node return mismatch: C={c_res}, Rust={r_res}");
    }
}
