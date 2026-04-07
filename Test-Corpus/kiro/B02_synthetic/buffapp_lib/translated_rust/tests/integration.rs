use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, CStr};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target/debug/libbuffapp_lib.so");
    p
}

#[repr(C)]
struct StringBuffer {
    data: *mut c_char,
    capacity: c_int,
    length: c_int,
}

// ---- get_operation_name ----

#[test]
fn test_get_operation_name() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(c_int) -> *const c_char> =
            c_lib.get(b"get_operation_name").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int) -> *const c_char> =
            r_lib.get(b"get_operation_name").unwrap();

        for op in -2..=10 {
            let c_str = CStr::from_ptr(c_fn(op));
            let r_str = CStr::from_ptr(r_fn(op));
            assert_eq!(c_str, r_str, "get_operation_name({op}) mismatch");
        }
    }
}

// ---- perform_operation ----

#[test]
fn test_perform_operation() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, *const c_char) -> c_int> =
            c_lib.get(b"perform_operation").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int, c_int, *const c_char) -> c_int> =
            r_lib.get(b"perform_operation").unwrap();

        let ops: &[&CStr] = &[c"add", c"subtract", c"multiply", c"divide", c"unknown", c"nope"];
        // Note: (i32::MIN, -1) with "divide" is UB in C (SIGFPE), so we skip it
        let vals: &[(c_int, c_int)] = &[
            (0, 0), (1, 1), (10, 3), (-5, 3), (7, -2), (-10, -3),
            (100, 0), (0, 100), (i32::MAX, 1), (i32::MIN, 1),
        ];

        for op in ops {
            for &(a, b) in vals {
                let c_res = c_fn(a, b, op.as_ptr());
                let r_res = r_fn(a, b, op.as_ptr());
                assert_eq!(c_res, r_res, "perform_operation({a}, {b}, {:?}) mismatch", op);
            }
        }
    }
}

// ---- create_buffer / append_to_buffer / destroy_buffer ----

#[test]
fn test_buffer_operations() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_create: Symbol<unsafe extern "C" fn(c_int) -> *mut StringBuffer> =
            c_lib.get(b"create_buffer").unwrap();
        let c_append: Symbol<unsafe extern "C" fn(*mut StringBuffer, *const c_char) -> c_int> =
            c_lib.get(b"append_to_buffer").unwrap();
        let c_destroy: Symbol<unsafe extern "C" fn(*mut StringBuffer)> =
            c_lib.get(b"destroy_buffer").unwrap();

        let r_create: Symbol<unsafe extern "C" fn(c_int) -> *mut StringBuffer> =
            r_lib.get(b"create_buffer").unwrap();
        let r_append: Symbol<unsafe extern "C" fn(*mut StringBuffer, *const c_char) -> c_int> =
            r_lib.get(b"append_to_buffer").unwrap();
        let r_destroy: Symbol<unsafe extern "C" fn(*mut StringBuffer)> =
            r_lib.get(b"destroy_buffer").unwrap();

        // Test basic create + append + read back
        let c_buf = c_create(16);
        let r_buf = r_create(16);
        assert!(!c_buf.is_null());
        assert!(!r_buf.is_null());

        let strs = [c"hello", c" ", c"world", c"!", c"longer string to force realloc"];
        for s in &strs {
            let c_ret = c_append(c_buf, s.as_ptr());
            let r_ret = r_append(r_buf, s.as_ptr());
            assert_eq!(c_ret, r_ret, "append return mismatch for {:?}", s);

            let c_data = CStr::from_ptr((*c_buf).data);
            let r_data = CStr::from_ptr((*r_buf).data);
            assert_eq!(c_data, r_data, "buffer content mismatch after appending {:?}", s);
            assert_eq!((*c_buf).length, (*r_buf).length, "length mismatch");
        }

        c_destroy(c_buf);
        r_destroy(r_buf);

        // Test destroy with null (should not crash)
        c_destroy(std::ptr::null_mut());
        r_destroy(std::ptr::null_mut());
    }
}

// ---- buffapp (top-level) ----

#[test]
fn test_buffapp() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            c_lib.get(b"buffapp").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            r_lib.get(b"buffapp").unwrap();

        let cases: &[(c_int, c_int, c_int, c_int)] = &[
            (0, 0, 0, 0),
            (1, 2, 3, 4),
            (0, 1, 0, 1),
            (1, 1, 1, 1),
            (2, 3, 2, 3),
            (3, 7, 3, 2),
            (4, 5, 6, 7),
            (10, 20, 30, 40),
            (-1, 2, -3, 4),
            (0, 0, 1, 0),
            (100, 200, 300, 400),
            (7, 0, 3, 0),
            (1, 0, 2, 0),
        ];

        for &(a, b, c, d) in cases {
            let c_res = c_fn(a, b, c, d);
            let r_res = r_fn(a, b, c, d);
            assert_eq!(c_res, r_res, "buffapp({a}, {b}, {c}, {d}) mismatch: C={c_res}, Rust={r_res}");
        }
    }
}
