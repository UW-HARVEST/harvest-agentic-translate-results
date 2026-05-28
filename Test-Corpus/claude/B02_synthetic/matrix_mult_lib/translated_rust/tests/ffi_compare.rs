// Integration tests comparing C and Rust shared libraries via libloading.
//
// Each test loads BOTH the C .so and the Rust .so and compares outputs through
// the FFI boundary, exactly as an external caller would.

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, CString};
use std::os::raw::c_void;
use std::path::PathBuf;
use std::ptr;

// matrix_t layout-compatible with C's definition.
#[repr(C)]
pub struct MatrixT {
    pub matrix: *mut *mut c_int,
    pub width: c_int,
    pub height: c_int,
}

fn c_lib_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("c_src/build/libdriver.so");
    p
}

fn rust_lib_path() -> PathBuf {
    // Locate the cdylib produced by `cargo build` for this package.
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target/debug/libdriver.so");
    p
}

fn ensure_built() {
    // Guarantee the Rust cdylib exists. Cargo doesn't build it for tests by
    // default (only test binaries), so run `cargo build` once at startup.
    let path = rust_lib_path();
    if !path.exists() {
        let status = std::process::Command::new(env!("CARGO"))
            .args(["build"])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .status()
            .expect("cargo build failed to run");
        assert!(status.success(), "cargo build failed");
    }
    assert!(c_lib_path().exists(), "C library not built: {:?}", c_lib_path());
}

unsafe fn load_libs() -> (Library, Library) {
    ensure_built();
    let c = unsafe { Library::new(c_lib_path()).expect("failed to load C lib") };
    let r = unsafe { Library::new(rust_lib_path()).expect("failed to load Rust lib") };
    (c, r)
}

// Helper: read all matrix cells into a Vec<i32> from a *mut MatrixT
unsafe fn matrix_cells(mat: *mut MatrixT) -> (i32, i32, Vec<i32>) {
    if mat.is_null() {
        return (-1, -1, Vec::new());
    }
    unsafe {
        let w = (*mat).width;
        let h = (*mat).height;
        let mut v = Vec::with_capacity((w * h) as usize);
        for i in 0..h as isize {
            let row = *(*mat).matrix.offset(i);
            for j in 0..w as isize {
                v.push(*row.offset(j));
            }
        }
        (w, h, v)
    }
}

type AllocateMatrixFn = unsafe extern "C" fn(c_int, c_int) -> *mut MatrixT;
type FreeMatrixFn = unsafe extern "C" fn(*mut MatrixT);
type InitializeMatrixFn = unsafe extern "C" fn(*const c_char, c_int, c_int) -> *mut MatrixT;
type MultiplyMatricesFn = unsafe extern "C" fn(*mut MatrixT, *mut MatrixT) -> *mut MatrixT;
type MatrixToStringFn = unsafe extern "C" fn(*mut MatrixT) -> *mut c_char;
type WriteToFileFn = unsafe extern "C" fn(*const c_char, *const c_char) -> c_int;
type DriverFn = unsafe extern "C" fn(c_int, c_int, *const c_char, c_int, c_int, *const c_char) -> c_int;

#[test]
fn test_allocate_matrix() {
    unsafe {
        let (clib, rlib) = load_libs();
        let alloc_c: Symbol<AllocateMatrixFn> = clib.get(b"allocate_matrix").unwrap();
        let alloc_r: Symbol<AllocateMatrixFn> = rlib.get(b"allocate_matrix").unwrap();
        let free_c: Symbol<FreeMatrixFn> = clib.get(b"free_matrix").unwrap();
        let free_r: Symbol<FreeMatrixFn> = rlib.get(b"free_matrix").unwrap();

        for &(w, h) in &[(1, 1), (3, 4), (10, 5), (2, 2), (8, 8)] {
            let m_c = alloc_c(w, h);
            let m_r = alloc_r(w, h);
            assert!(!m_c.is_null());
            assert!(!m_r.is_null());
            assert_eq!((*m_c).width, (*m_r).width);
            assert_eq!((*m_c).height, (*m_r).height);
            assert_eq!((*m_c).width, w);
            assert_eq!((*m_c).height, h);
            free_c(m_c);
            free_r(m_r);
        }
    }
}

#[test]
fn test_initialize_matrix_from_string() {
    unsafe {
        let (clib, rlib) = load_libs();
        let init_c: Symbol<InitializeMatrixFn> = clib.get(b"initialize_matrix_from_string").unwrap();
        let init_r: Symbol<InitializeMatrixFn> = rlib.get(b"initialize_matrix_from_string").unwrap();
        let free_c: Symbol<FreeMatrixFn> = clib.get(b"free_matrix").unwrap();
        let free_r: Symbol<FreeMatrixFn> = rlib.get(b"free_matrix").unwrap();

        let cases: &[(&str, c_int, c_int)] = &[
            ("1 2 3\n4 5 6\n", 3, 2),
            ("0 0\n0 0\n", 2, 2),
            ("-1 -2 -3\n-4 -5 -6\n7 8 9\n", 3, 3),
            ("100 200 300 400 500\n", 5, 1),
            ("1\n2\n3\n4\n", 1, 4),
            ("  42 7\n3   9\n", 2, 2),
        ];

        for (s, w, h) in cases {
            let cs = CString::new(*s).unwrap();
            let m_c = init_c(cs.as_ptr(), *w, *h);
            let m_r = init_r(cs.as_ptr(), *w, *h);
            assert!(!m_c.is_null(), "C returned null for {:?}", s);
            assert!(!m_r.is_null(), "Rust returned null for {:?}", s);
            let (cw, ch, cv) = matrix_cells(m_c);
            let (rw, rh, rv) = matrix_cells(m_r);
            assert_eq!(cw, rw);
            assert_eq!(ch, rh);
            assert_eq!(cv, rv, "matrix values differ for {:?}", s);
            free_c(m_c);
            free_r(m_r);
        }
    }
}

#[test]
fn test_multiply_matrices() {
    unsafe {
        let (clib, rlib) = load_libs();
        let init_c: Symbol<InitializeMatrixFn> = clib.get(b"initialize_matrix_from_string").unwrap();
        let init_r: Symbol<InitializeMatrixFn> = rlib.get(b"initialize_matrix_from_string").unwrap();
        let mul_c: Symbol<MultiplyMatricesFn> = clib.get(b"multiply_matrices").unwrap();
        let mul_r: Symbol<MultiplyMatricesFn> = rlib.get(b"multiply_matrices").unwrap();
        let free_c: Symbol<FreeMatrixFn> = clib.get(b"free_matrix").unwrap();
        let free_r: Symbol<FreeMatrixFn> = rlib.get(b"free_matrix").unwrap();

        let cases: &[((&str, c_int, c_int), (&str, c_int, c_int))] = &[
            (("1 2\n3 4\n", 2, 2), ("5 6\n7 8\n", 2, 2)),
            (("1 0 0\n0 1 0\n0 0 1\n", 3, 3), ("9 8 7\n6 5 4\n3 2 1\n", 3, 3)),
            (("1 2 3\n4 5 6\n", 3, 2), ("7 8\n9 10\n11 12\n", 2, 3)),
            (("-1 2\n3 -4\n", 2, 2), ("5 -6\n-7 8\n", 2, 2)),
            (("1\n2\n3\n", 1, 3), ("10 20 30\n", 3, 1)),
        ];

        for ((sa, wa, ha), (sb, wb, hb)) in cases {
            let csa = CString::new(*sa).unwrap();
            let csb = CString::new(*sb).unwrap();
            let ma_c = init_c(csa.as_ptr(), *wa, *ha);
            let mb_c = init_c(csb.as_ptr(), *wb, *hb);
            let ma_r = init_r(csa.as_ptr(), *wa, *ha);
            let mb_r = init_r(csb.as_ptr(), *wb, *hb);

            let res_c = mul_c(ma_c, mb_c);
            let res_r = mul_r(ma_r, mb_r);
            assert!(!res_c.is_null());
            assert!(!res_r.is_null());

            let (cw, ch, cv) = matrix_cells(res_c);
            let (rw, rh, rv) = matrix_cells(res_r);
            assert_eq!((cw, ch, &cv), (rw, rh, &rv));

            free_c(res_c);
            free_r(res_r);
            free_c(ma_c);
            free_c(mb_c);
            free_r(ma_r);
            free_r(mb_r);
        }
    }
}

#[test]
fn test_multiply_matrices_dim_mismatch() {
    unsafe {
        let (clib, rlib) = load_libs();
        let init_c: Symbol<InitializeMatrixFn> = clib.get(b"initialize_matrix_from_string").unwrap();
        let init_r: Symbol<InitializeMatrixFn> = rlib.get(b"initialize_matrix_from_string").unwrap();
        let mul_c: Symbol<MultiplyMatricesFn> = clib.get(b"multiply_matrices").unwrap();
        let mul_r: Symbol<MultiplyMatricesFn> = rlib.get(b"multiply_matrices").unwrap();
        let free_c: Symbol<FreeMatrixFn> = clib.get(b"free_matrix").unwrap();
        let free_r: Symbol<FreeMatrixFn> = rlib.get(b"free_matrix").unwrap();

        // 2x2 times 3x3: a.width=2 != b.height=3 -> should produce NULL
        let csa = CString::new("1 2\n3 4\n").unwrap();
        let csb = CString::new("1 0 0\n0 1 0\n0 0 1\n").unwrap();
        let ma_c = init_c(csa.as_ptr(), 2, 2);
        let mb_c = init_c(csb.as_ptr(), 3, 3);
        let ma_r = init_r(csa.as_ptr(), 2, 2);
        let mb_r = init_r(csb.as_ptr(), 3, 3);

        let res_c = mul_c(ma_c, mb_c);
        let res_r = mul_r(ma_r, mb_r);
        assert!(res_c.is_null());
        assert!(res_r.is_null());

        free_c(ma_c);
        free_c(mb_c);
        free_r(ma_r);
        free_r(mb_r);
    }
}

#[test]
fn test_matrix_to_string() {
    unsafe {
        let (clib, rlib) = load_libs();
        let init_c: Symbol<InitializeMatrixFn> = clib.get(b"initialize_matrix_from_string").unwrap();
        let init_r: Symbol<InitializeMatrixFn> = rlib.get(b"initialize_matrix_from_string").unwrap();
        let to_str_c: Symbol<MatrixToStringFn> = clib.get(b"matrix_to_string").unwrap();
        let to_str_r: Symbol<MatrixToStringFn> = rlib.get(b"matrix_to_string").unwrap();
        let free_c: Symbol<FreeMatrixFn> = clib.get(b"free_matrix").unwrap();
        let free_r: Symbol<FreeMatrixFn> = rlib.get(b"free_matrix").unwrap();

        let cases: &[(&str, c_int, c_int)] = &[
            ("1 2 3\n4 5 6\n", 3, 2),
            ("0\n", 1, 1),
            ("-1 -2\n-3 -4\n", 2, 2),
            ("1000 2000 3000\n", 3, 1),
            ("1 2\n3 4\n5 6\n", 2, 3),
        ];

        for (s, w, h) in cases {
            let cs = CString::new(*s).unwrap();
            let m_c = init_c(cs.as_ptr(), *w, *h);
            let m_r = init_r(cs.as_ptr(), *w, *h);
            let s_c = to_str_c(m_c);
            let s_r = to_str_r(m_r);

            let bc = std::ffi::CStr::from_ptr(s_c).to_bytes().to_vec();
            let br = std::ffi::CStr::from_ptr(s_r).to_bytes().to_vec();
            assert_eq!(bc, br, "matrix_to_string differs for {:?}", s);

            libc::free(s_c as *mut c_void);
            libc::free(s_r as *mut c_void);
            free_c(m_c);
            free_r(m_r);
        }
    }
}

#[test]
fn test_matrix_to_string_null() {
    unsafe {
        let (clib, rlib) = load_libs();
        let to_str_c: Symbol<MatrixToStringFn> = clib.get(b"matrix_to_string").unwrap();
        let to_str_r: Symbol<MatrixToStringFn> = rlib.get(b"matrix_to_string").unwrap();
        let r1 = to_str_c(ptr::null_mut());
        let r2 = to_str_r(ptr::null_mut());
        assert!(r1.is_null());
        assert!(r2.is_null());
    }
}

#[test]
fn test_write_to_file() {
    unsafe {
        let (clib, rlib) = load_libs();
        let write_c: Symbol<WriteToFileFn> = clib.get(b"write_to_file").unwrap();
        let write_r: Symbol<WriteToFileFn> = rlib.get(b"write_to_file").unwrap();

        let tmp = std::env::temp_dir();
        let p_c = tmp.join("write_to_file_c.txt");
        let p_r = tmp.join("write_to_file_r.txt");
        let _ = std::fs::remove_file(&p_c);
        let _ = std::fs::remove_file(&p_r);
        let cs_c = CString::new(p_c.to_str().unwrap()).unwrap();
        let cs_r = CString::new(p_r.to_str().unwrap()).unwrap();

        let content = CString::new("hello world\nline 2\n").unwrap();
        let rc_c = write_c(cs_c.as_ptr(), content.as_ptr());
        let rc_r = write_r(cs_r.as_ptr(), content.as_ptr());
        assert_eq!(rc_c, 0);
        assert_eq!(rc_r, 0);

        let bc = std::fs::read(&p_c).unwrap();
        let br = std::fs::read(&p_r).unwrap();
        assert_eq!(bc, br);
        assert_eq!(bc, b"hello world\nline 2\n");

        let _ = std::fs::remove_file(&p_c);
        let _ = std::fs::remove_file(&p_r);
    }
}

#[test]
fn test_write_to_file_null_content() {
    unsafe {
        let (clib, rlib) = load_libs();
        let write_c: Symbol<WriteToFileFn> = clib.get(b"write_to_file").unwrap();
        let write_r: Symbol<WriteToFileFn> = rlib.get(b"write_to_file").unwrap();

        let p = CString::new("/tmp/should_not_exist.txt").unwrap();
        let rc_c = write_c(p.as_ptr(), ptr::null());
        let rc_r = write_r(p.as_ptr(), ptr::null());
        assert_eq!(rc_c, rc_r);
        // EINVAL = 22
        assert_eq!(rc_c, 22);
    }
}

#[test]
fn test_write_to_file_bad_path() {
    unsafe {
        let (clib, rlib) = load_libs();
        let write_c: Symbol<WriteToFileFn> = clib.get(b"write_to_file").unwrap();
        let write_r: Symbol<WriteToFileFn> = rlib.get(b"write_to_file").unwrap();

        let p = CString::new("/no/such/dir/output.txt").unwrap();
        let content = CString::new("data").unwrap();
        let rc_c = write_c(p.as_ptr(), content.as_ptr());
        let rc_r = write_r(p.as_ptr(), content.as_ptr());
        assert_eq!(rc_c, rc_r);
        assert_ne!(rc_c, 0);
    }
}

#[test]
fn test_driver() {
    unsafe {
        let (clib, rlib) = load_libs();
        let driver_c: Symbol<DriverFn> = clib.get(b"driver").unwrap();
        let driver_r: Symbol<DriverFn> = rlib.get(b"driver").unwrap();

        // driver writes to "matrix.txt" in the current working directory.
        // We need to invoke each in a separate temp directory and capture
        // the written file.
        let tmp_root = std::env::temp_dir().join("ffi_driver_test");
        let _ = std::fs::remove_dir_all(&tmp_root);
        std::fs::create_dir_all(&tmp_root).unwrap();
        let dir_c = tmp_root.join("c");
        let dir_r = tmp_root.join("r");
        std::fs::create_dir_all(&dir_c).unwrap();
        std::fs::create_dir_all(&dir_r).unwrap();

        let cases: &[((&str, c_int, c_int), (&str, c_int, c_int))] = &[
            (("1 2\n3 4\n", 2, 2), ("5 6\n7 8\n", 2, 2)),
            (("1 2 3\n4 5 6\n", 3, 2), ("7 8\n9 10\n11 12\n", 2, 3)),
            (("-1 2\n3 -4\n", 2, 2), ("5 -6\n-7 8\n", 2, 2)),
        ];

        for ((sa, wa, ha), (sb, wb, hb)) in cases {
            let csa = CString::new(*sa).unwrap();
            let csb = CString::new(*sb).unwrap();

            // Run C driver
            let prev = std::env::current_dir().unwrap();
            std::env::set_current_dir(&dir_c).unwrap();
            let rc_c = driver_c(*wa, *ha, csa.as_ptr(), *wb, *hb, csb.as_ptr());
            std::env::set_current_dir(&prev).unwrap();

            std::env::set_current_dir(&dir_r).unwrap();
            let rc_r = driver_r(*wa, *ha, csa.as_ptr(), *wb, *hb, csb.as_ptr());
            std::env::set_current_dir(&prev).unwrap();

            assert_eq!(rc_c, rc_r, "driver return code differs");

            let bc = std::fs::read(dir_c.join("matrix.txt")).unwrap();
            let br = std::fs::read(dir_r.join("matrix.txt")).unwrap();
            assert_eq!(bc, br, "driver output differs for {:?} {:?}", sa, sb);
        }

        let _ = std::fs::remove_dir_all(&tmp_root);
    }
}
