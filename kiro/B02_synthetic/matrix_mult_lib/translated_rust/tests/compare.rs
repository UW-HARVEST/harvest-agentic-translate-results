use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, CStr, CString};
use std::path::PathBuf;

#[repr(C)]
struct matrix_t {
    matrix: *mut *mut c_int,
    width: c_int,
    height: c_int,
}

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    // cargo puts cdylib in target/debug or target/release
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target");
    p.push("debug");
    p.push("libdriver.so");
    p
}

/// Read matrix values into a Vec<Vec<i32>> for comparison
unsafe fn read_matrix(mat: *mut matrix_t) -> Vec<Vec<i32>> {
    let h = (*mat).height as usize;
    let w = (*mat).width as usize;
    let mut rows = Vec::with_capacity(h);
    for i in 0..h {
        let mut row = Vec::with_capacity(w);
        for j in 0..w {
            row.push(*(*(*mat).matrix.add(i)).add(j));
        }
        rows.push(row);
    }
    rows
}

// ── Test 1: allocate_matrix ─────────────────────────────────────────────────

#[test]
fn test_allocate_matrix() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let rust_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_alloc: Symbol<unsafe extern "C" fn(c_int, c_int) -> *mut matrix_t> =
            c_lib.get(b"allocate_matrix").expect("c allocate_matrix");
        let r_alloc: Symbol<unsafe extern "C" fn(c_int, c_int) -> *mut matrix_t> =
            rust_lib.get(b"allocate_matrix").expect("rust allocate_matrix");

        let c_free: Symbol<unsafe extern "C" fn(*mut matrix_t)> =
            c_lib.get(b"free_matrix").expect("c free_matrix");
        let r_free: Symbol<unsafe extern "C" fn(*mut matrix_t)> =
            rust_lib.get(b"free_matrix").expect("rust free_matrix");

        for (w, h) in [(2, 3), (1, 1), (4, 5)] {
            let cm = c_alloc(w, h);
            let rm = r_alloc(w, h);
            assert!(!cm.is_null());
            assert!(!rm.is_null());
            assert_eq!((*cm).width, (*rm).width, "width mismatch for ({w},{h})");
            assert_eq!((*cm).height, (*rm).height, "height mismatch for ({w},{h})");
            c_free(cm);
            r_free(rm);
        }
    }
}

// ── Test 2: initialize_matrix_from_string ───────────────────────────────────

#[test]
fn test_initialize_matrix_from_string() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let rust_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_init: Symbol<unsafe extern "C" fn(*const c_char, c_int, c_int) -> *mut matrix_t> =
            c_lib.get(b"initialize_matrix_from_string").unwrap();
        let r_init: Symbol<unsafe extern "C" fn(*const c_char, c_int, c_int) -> *mut matrix_t> =
            rust_lib.get(b"initialize_matrix_from_string").unwrap();

        let c_free: Symbol<unsafe extern "C" fn(*mut matrix_t)> =
            c_lib.get(b"free_matrix").unwrap();
        let r_free: Symbol<unsafe extern "C" fn(*mut matrix_t)> =
            rust_lib.get(b"free_matrix").unwrap();

        let input = CString::new("1 2 3\n4 5 6\n").unwrap();
        let cm = c_init(input.as_ptr(), 3, 2);
        let rm = r_init(input.as_ptr(), 3, 2);
        assert!(!cm.is_null());
        assert!(!rm.is_null());

        let c_vals = read_matrix(cm);
        let r_vals = read_matrix(rm);
        assert_eq!(c_vals, r_vals, "matrix values differ");

        c_free(cm);
        r_free(rm);
    }
}

// ── Test 3: multiply_matrices ───────────────────────────────────────────────

#[test]
fn test_multiply_matrices() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let rust_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        type InitFn = unsafe extern "C" fn(*const c_char, c_int, c_int) -> *mut matrix_t;
        type MultFn = unsafe extern "C" fn(*mut matrix_t, *mut matrix_t) -> *mut matrix_t;
        type FreeFn = unsafe extern "C" fn(*mut matrix_t);

        let c_init: Symbol<InitFn> = c_lib.get(b"initialize_matrix_from_string").unwrap();
        let r_init: Symbol<InitFn> = rust_lib.get(b"initialize_matrix_from_string").unwrap();
        let c_mult: Symbol<MultFn> = c_lib.get(b"multiply_matrices").unwrap();
        let r_mult: Symbol<MultFn> = rust_lib.get(b"multiply_matrices").unwrap();
        let c_free: Symbol<FreeFn> = c_lib.get(b"free_matrix").unwrap();
        let r_free: Symbol<FreeFn> = rust_lib.get(b"free_matrix").unwrap();

        // A = 2x3, B = 3x2 => result = 2x2
        let a_str = CString::new("1 2 3\n4 5 6\n").unwrap();
        let b_str = CString::new("7 8\n9 10\n11 12\n").unwrap();

        let c_a = c_init(a_str.as_ptr(), 3, 2);
        let c_b = c_init(b_str.as_ptr(), 2, 3);
        let c_res = c_mult(c_a, c_b);
        assert!(!c_res.is_null());

        let r_a = r_init(a_str.as_ptr(), 3, 2);
        let r_b = r_init(b_str.as_ptr(), 2, 3);
        let r_res = r_mult(r_a, r_b);
        assert!(!r_res.is_null());

        let c_vals = read_matrix(c_res);
        let r_vals = read_matrix(r_res);
        assert_eq!(c_vals, r_vals, "multiply result differs");

        c_free(c_a); c_free(c_b); c_free(c_res);
        r_free(r_a); r_free(r_b); r_free(r_res);
    }
}

// ── Test 4: matrix_to_string ────────────────────────────────────────────────

#[test]
fn test_matrix_to_string() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let rust_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        type InitFn = unsafe extern "C" fn(*const c_char, c_int, c_int) -> *mut matrix_t;
        type ToStrFn = unsafe extern "C" fn(*mut matrix_t) -> *mut c_char;
        type FreeFn = unsafe extern "C" fn(*mut matrix_t);

        let c_init: Symbol<InitFn> = c_lib.get(b"initialize_matrix_from_string").unwrap();
        let r_init: Symbol<InitFn> = rust_lib.get(b"initialize_matrix_from_string").unwrap();
        let c_tostr: Symbol<ToStrFn> = c_lib.get(b"matrix_to_string").unwrap();
        let r_tostr: Symbol<ToStrFn> = rust_lib.get(b"matrix_to_string").unwrap();
        let c_free: Symbol<FreeFn> = c_lib.get(b"free_matrix").unwrap();
        let r_free: Symbol<FreeFn> = rust_lib.get(b"free_matrix").unwrap();

        let input = CString::new("1 2 3\n4 5 6\n").unwrap();
        let cm = c_init(input.as_ptr(), 3, 2);
        let rm = r_init(input.as_ptr(), 3, 2);

        let c_s = c_tostr(cm);
        let r_s = r_tostr(rm);
        assert!(!c_s.is_null());
        assert!(!r_s.is_null());

        let c_str = CStr::from_ptr(c_s).to_bytes();
        let r_str = CStr::from_ptr(r_s).to_bytes();
        assert_eq!(c_str, r_str, "matrix_to_string output differs:\nC:    {:?}\nRust: {:?}",
            String::from_utf8_lossy(c_str), String::from_utf8_lossy(r_str));

        libc::free(c_s as *mut libc::c_void);
        libc::free(r_s as *mut libc::c_void);
        c_free(cm);
        r_free(rm);
    }
}

// ── Test 5: write_to_file ───────────────────────────────────────────────────

#[test]
fn test_write_to_file() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let rust_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        type WriteFn = unsafe extern "C" fn(*const c_char, *const c_char) -> c_int;

        let c_write: Symbol<WriteFn> = c_lib.get(b"write_to_file").unwrap();
        let r_write: Symbol<WriteFn> = rust_lib.get(b"write_to_file").unwrap();

        let content = CString::new("hello world\n").unwrap();
        let c_file = CString::new("/tmp/test_c_write.txt").unwrap();
        let r_file = CString::new("/tmp/test_r_write.txt").unwrap();

        let c_ret = c_write(c_file.as_ptr(), content.as_ptr());
        let r_ret = r_write(r_file.as_ptr(), content.as_ptr());
        assert_eq!(c_ret, r_ret, "write_to_file return code differs");

        let c_data = std::fs::read("/tmp/test_c_write.txt").unwrap();
        let r_data = std::fs::read("/tmp/test_r_write.txt").unwrap();
        assert_eq!(c_data, r_data, "written file contents differ");
    }
}

// ── Test 6: driver ──────────────────────────────────────────────────────────

#[test]
fn test_driver() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let rust_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        type DriverFn = unsafe extern "C" fn(c_int, c_int, *const c_char, c_int, c_int, *const c_char) -> c_int;

        let c_driver: Symbol<DriverFn> = c_lib.get(b"driver").unwrap();
        let r_driver: Symbol<DriverFn> = rust_lib.get(b"driver").unwrap();

        let a_str = CString::new("1 2\n3 4\n").unwrap();
        let b_str = CString::new("5 6\n7 8\n").unwrap();

        // Run C driver first, read output
        let c_ret = c_driver(2, 2, a_str.as_ptr(), 2, 2, b_str.as_ptr());
        let c_output = std::fs::read("matrix.txt").unwrap();

        // Run Rust driver, read output
        let r_ret = r_driver(2, 2, a_str.as_ptr(), 2, 2, b_str.as_ptr());
        let r_output = std::fs::read("matrix.txt").unwrap();

        assert_eq!(c_ret, r_ret, "driver return code differs: C={c_ret}, Rust={r_ret}");
        assert_eq!(c_output, r_output, "driver file output differs:\nC:    {:?}\nRust: {:?}",
            String::from_utf8_lossy(&c_output), String::from_utf8_lossy(&r_output));
    }
}
