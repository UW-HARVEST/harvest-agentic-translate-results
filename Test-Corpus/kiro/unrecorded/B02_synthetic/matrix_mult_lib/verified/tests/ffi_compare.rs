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
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // cargo test builds in target/debug/deps, but the cdylib is in target/debug/
    dir.join("target/debug/libdriver.so")
}

struct Lib {
    _lib: Library,
}

impl Lib {
    fn load(path: &PathBuf) -> Self {
        let lib = unsafe { Library::new(path).expect(&format!("Failed to load {:?}", path)) };
        Lib { _lib: lib }
    }

    fn allocate_matrix(&self, w: c_int, h: c_int) -> *mut matrix_t {
        unsafe {
            let f: Symbol<unsafe extern "C" fn(c_int, c_int) -> *mut matrix_t> =
                self._lib.get(b"allocate_matrix").unwrap();
            f(w, h)
        }
    }

    fn free_matrix(&self, mat: *mut matrix_t) {
        unsafe {
            let f: Symbol<unsafe extern "C" fn(*mut matrix_t)> =
                self._lib.get(b"free_matrix").unwrap();
            f(mat);
        }
    }

    fn initialize_matrix_from_string(
        &self,
        input: *const c_char,
        w: c_int,
        h: c_int,
    ) -> *mut matrix_t {
        unsafe {
            let f: Symbol<unsafe extern "C" fn(*const c_char, c_int, c_int) -> *mut matrix_t> =
                self._lib.get(b"initialize_matrix_from_string").unwrap();
            f(input, w, h)
        }
    }

    fn multiply_matrices(&self, a: *mut matrix_t, b: *mut matrix_t) -> *mut matrix_t {
        unsafe {
            let f: Symbol<unsafe extern "C" fn(*mut matrix_t, *mut matrix_t) -> *mut matrix_t> =
                self._lib.get(b"multiply_matrices").unwrap();
            f(a, b)
        }
    }

    fn matrix_to_string(&self, mat: *mut matrix_t) -> *mut c_char {
        unsafe {
            let f: Symbol<unsafe extern "C" fn(*mut matrix_t) -> *mut c_char> =
                self._lib.get(b"matrix_to_string").unwrap();
            f(mat)
        }
    }

    fn write_to_file(&self, filename: *const c_char, content: *const c_char) -> c_int {
        unsafe {
            let f: Symbol<unsafe extern "C" fn(*const c_char, *const c_char) -> c_int> =
                self._lib.get(b"write_to_file").unwrap();
            f(filename, content)
        }
    }

    fn driver(
        &self,
        wa: c_int,
        ha: c_int,
        ma: *const c_char,
        wb: c_int,
        hb: c_int,
        mb: *const c_char,
    ) -> c_int {
        unsafe {
            let f: Symbol<
                unsafe extern "C" fn(c_int, c_int, *const c_char, c_int, c_int, *const c_char) -> c_int,
            > = self._lib.get(b"driver").unwrap();
            f(wa, ha, ma, wb, hb, mb)
        }
    }

    fn free_ptr(&self, ptr: *mut c_char) {
        unsafe { libc::free(ptr as *mut libc::c_void) };
    }
}

fn read_matrix_values(mat: *mut matrix_t) -> Vec<Vec<c_int>> {
    unsafe {
        let h = (*mat).height as usize;
        let w = (*mat).width as usize;
        (0..h)
            .map(|i| (0..w).map(|j| *(*(*mat).matrix.add(i)).add(j)).collect())
            .collect()
    }
}

fn cstr(s: &str) -> CString {
    CString::new(s).unwrap()
}

fn ptr_to_string(p: *mut c_char) -> String {
    unsafe { CStr::from_ptr(p).to_string_lossy().into_owned() }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[test]
fn test_allocate_and_free_matrix() {
    let c = Lib::load(&c_lib_path());
    let r = Lib::load(&rust_lib_path());

    for (w, h) in [(2, 3), (1, 1), (5, 4)] {
        let cm = c.allocate_matrix(w, h);
        let rm = r.allocate_matrix(w, h);
        assert!(!cm.is_null());
        assert!(!rm.is_null());
        unsafe {
            assert_eq!((*cm).width, (*rm).width);
            assert_eq!((*cm).height, (*rm).height);
        }
        c.free_matrix(cm);
        r.free_matrix(rm);
    }
}

#[test]
fn test_free_matrix_null() {
    let c = Lib::load(&c_lib_path());
    let r = Lib::load(&rust_lib_path());
    // Should not crash
    c.free_matrix(std::ptr::null_mut());
    r.free_matrix(std::ptr::null_mut());
}

#[test]
fn test_initialize_matrix_from_string() {
    let c = Lib::load(&c_lib_path());
    let r = Lib::load(&rust_lib_path());

    let inputs = [
        ("1 2\n3 4", 2, 2),
        ("10 20 30\n40 50 60", 3, 2),
        ("7", 1, 1),
        ("-1 0\n0 -1", 2, 2),
    ];

    for (input, w, h) in inputs {
        let cs = cstr(input);
        let cm = c.initialize_matrix_from_string(cs.as_ptr(), w, h);
        let rm = r.initialize_matrix_from_string(cs.as_ptr(), w, h);
        assert!(!cm.is_null(), "C returned null for input: {}", input);
        assert!(!rm.is_null(), "Rust returned null for input: {}", input);

        let cv = read_matrix_values(cm);
        let rv = read_matrix_values(rm);
        assert_eq!(cv, rv, "Mismatch for input: {}", input);

        c.free_matrix(cm);
        r.free_matrix(rm);
    }
}

#[test]
fn test_multiply_matrices() {
    let c = Lib::load(&c_lib_path());
    let r = Lib::load(&rust_lib_path());

    // 2x2 * 2x2
    let a_str = cstr("1 2\n3 4");
    let b_str = cstr("5 6\n7 8");

    let ca = c.initialize_matrix_from_string(a_str.as_ptr(), 2, 2);
    let cb = c.initialize_matrix_from_string(b_str.as_ptr(), 2, 2);
    let ra = r.initialize_matrix_from_string(a_str.as_ptr(), 2, 2);
    let rb = r.initialize_matrix_from_string(b_str.as_ptr(), 2, 2);

    let c_res = c.multiply_matrices(ca, cb);
    let r_res = r.multiply_matrices(ra, rb);
    assert!(!c_res.is_null());
    assert!(!r_res.is_null());

    assert_eq!(read_matrix_values(c_res), read_matrix_values(r_res));

    c.free_matrix(ca);
    c.free_matrix(cb);
    c.free_matrix(c_res);
    r.free_matrix(ra);
    r.free_matrix(rb);
    r.free_matrix(r_res);

    // 2x3 * 3x2
    let a2 = cstr("1 2 3\n4 5 6");
    let b2 = cstr("7 8\n9 10\n11 12");

    let ca2 = c.initialize_matrix_from_string(a2.as_ptr(), 3, 2);
    let cb2 = c.initialize_matrix_from_string(b2.as_ptr(), 2, 3);
    let ra2 = r.initialize_matrix_from_string(a2.as_ptr(), 3, 2);
    let rb2 = r.initialize_matrix_from_string(b2.as_ptr(), 2, 3);

    let c_res2 = c.multiply_matrices(ca2, cb2);
    let r_res2 = r.multiply_matrices(ra2, rb2);
    assert!(!c_res2.is_null());
    assert!(!r_res2.is_null());

    assert_eq!(read_matrix_values(c_res2), read_matrix_values(r_res2));

    c.free_matrix(ca2);
    c.free_matrix(cb2);
    c.free_matrix(c_res2);
    r.free_matrix(ra2);
    r.free_matrix(rb2);
    r.free_matrix(r_res2);
}

#[test]
fn test_matrix_to_string() {
    let c = Lib::load(&c_lib_path());
    let r = Lib::load(&rust_lib_path());

    let inputs = [
        ("1 2\n3 4", 2, 2),
        ("10 20 30\n40 50 60", 3, 2),
        ("7", 1, 1),
        ("-1 0\n0 -1", 2, 2),
        ("100 200\n300 400\n500 600", 2, 3),
    ];

    for (input, w, h) in inputs {
        let cs = cstr(input);
        let cm = c.initialize_matrix_from_string(cs.as_ptr(), w, h);
        let rm = r.initialize_matrix_from_string(cs.as_ptr(), w, h);

        let c_str = c.matrix_to_string(cm);
        let r_str = r.matrix_to_string(rm);
        assert!(!c_str.is_null());
        assert!(!r_str.is_null());

        let c_out = ptr_to_string(c_str);
        let r_out = ptr_to_string(r_str);
        assert_eq!(
            c_out.as_bytes(),
            r_out.as_bytes(),
            "Byte mismatch for matrix_to_string with input: {}",
            input
        );

        c.free_ptr(c_str);
        r.free_ptr(r_str);
        c.free_matrix(cm);
        r.free_matrix(rm);
    }
}

#[test]
fn test_matrix_to_string_null() {
    let c = Lib::load(&c_lib_path());
    let r = Lib::load(&rust_lib_path());
    let c_str = c.matrix_to_string(std::ptr::null_mut());
    let r_str = r.matrix_to_string(std::ptr::null_mut());
    assert!(c_str.is_null());
    assert!(r_str.is_null());
}

#[test]
fn test_write_to_file() {
    let c = Lib::load(&c_lib_path());
    let r = Lib::load(&rust_lib_path());

    let content = cstr("hello world\n");
    let c_path = cstr("/tmp/test_c_write.txt");
    let r_path = cstr("/tmp/test_r_write.txt");

    let c_ret = c.write_to_file(c_path.as_ptr(), content.as_ptr());
    let r_ret = r.write_to_file(r_path.as_ptr(), content.as_ptr());
    assert_eq!(c_ret, r_ret, "Return codes differ");

    let c_content = std::fs::read("/tmp/test_c_write.txt").unwrap();
    let r_content = std::fs::read("/tmp/test_r_write.txt").unwrap();
    assert_eq!(c_content, r_content, "File contents differ");

    std::fs::remove_file("/tmp/test_c_write.txt").ok();
    std::fs::remove_file("/tmp/test_r_write.txt").ok();
}

#[test]
fn test_write_to_file_null_content() {
    let c = Lib::load(&c_lib_path());
    let r = Lib::load(&rust_lib_path());

    let path = cstr("/tmp/test_null_write.txt");
    let c_ret = c.write_to_file(path.as_ptr(), std::ptr::null());
    let r_ret = r.write_to_file(path.as_ptr(), std::ptr::null());
    assert_eq!(c_ret, r_ret, "Return codes for null content differ");
}

#[test]
fn test_driver() {
    let c = Lib::load(&c_lib_path());
    let r = Lib::load(&rust_lib_path());

    let a = cstr("1 2\n3 4");
    let b = cstr("5 6\n7 8");

    // Run C driver - writes to matrix.txt
    let c_ret = c.driver(2, 2, a.as_ptr(), 2, 2, b.as_ptr());
    let c_content = std::fs::read("matrix.txt").unwrap();

    // Run Rust driver - overwrites matrix.txt
    let r_ret = r.driver(2, 2, a.as_ptr(), 2, 2, b.as_ptr());
    let r_content = std::fs::read("matrix.txt").unwrap();

    assert_eq!(c_ret, r_ret, "Driver return codes differ");
    assert_eq!(c_content, r_content, "Driver output files differ");

    std::fs::remove_file("matrix.txt").ok();
}

#[test]
fn test_driver_non_square() {
    let c = Lib::load(&c_lib_path());
    let r = Lib::load(&rust_lib_path());

    let a = cstr("1 2 3\n4 5 6");
    let b = cstr("7 8\n9 10\n11 12");

    let c_ret = c.driver(3, 2, a.as_ptr(), 2, 3, b.as_ptr());
    let c_content = std::fs::read("matrix.txt").unwrap();

    let r_ret = r.driver(3, 2, a.as_ptr(), 2, 3, b.as_ptr());
    let r_content = std::fs::read("matrix.txt").unwrap();

    assert_eq!(c_ret, r_ret);
    assert_eq!(c_content, r_content);

    std::fs::remove_file("matrix.txt").ok();
}

#[test]
fn test_multiply_then_to_string_roundtrip() {
    let c = Lib::load(&c_lib_path());
    let r = Lib::load(&rust_lib_path());

    let a = cstr("1 0\n0 1");
    let b = cstr("9 8\n7 6");

    let ca = c.initialize_matrix_from_string(a.as_ptr(), 2, 2);
    let cb = c.initialize_matrix_from_string(b.as_ptr(), 2, 2);
    let ra = r.initialize_matrix_from_string(a.as_ptr(), 2, 2);
    let rb = r.initialize_matrix_from_string(b.as_ptr(), 2, 2);

    let c_res = c.multiply_matrices(ca, cb);
    let r_res = r.multiply_matrices(ra, rb);

    let c_str = c.matrix_to_string(c_res);
    let r_str = r.matrix_to_string(r_res);

    assert_eq!(
        ptr_to_string(c_str).as_bytes(),
        ptr_to_string(r_str).as_bytes()
    );

    c.free_ptr(c_str);
    r.free_ptr(r_str);
    c.free_matrix(ca);
    c.free_matrix(cb);
    c.free_matrix(c_res);
    r.free_matrix(ra);
    r.free_matrix(rb);
    r.free_matrix(r_res);
}
