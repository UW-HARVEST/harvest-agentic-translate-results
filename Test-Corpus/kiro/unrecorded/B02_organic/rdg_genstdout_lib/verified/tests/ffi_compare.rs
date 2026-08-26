use libloading::{Library, Symbol};
use std::ffi::{c_char, CStr};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // cdylib is always in deps for test builds; use the direct target path
    p.push("target/debug/libdriver.so");
    p
}

struct Libs {
    _c: Library,
    _r: Library,
    c_extract: Symbol<'static, unsafe extern "C" fn(*const c_char, c_char) -> *const c_char>,
    r_extract: Symbol<'static, unsafe extern "C" fn(*const c_char, c_char) -> *const c_char>,
    c_create: Symbol<'static, unsafe extern "C" fn(*const c_char, *const c_char, usize) -> *mut c_char>,
    r_create: Symbol<'static, unsafe extern "C" fn(*const c_char, *const c_char, usize) -> *mut c_char>,
}

impl Libs {
    fn load() -> Self {
        // We need to leak the libraries so symbols live long enough
        let c: &'static Library =
            Box::leak(Box::new(unsafe { Library::new(c_lib_path()).expect("load C lib") }));
        let r: &'static Library =
            Box::leak(Box::new(unsafe { Library::new(rust_lib_path()).expect("load Rust lib") }));

        unsafe {
            let c_extract: Symbol<'static, unsafe extern "C" fn(*const c_char, c_char) -> *const c_char> =
                c.get(b"extractFilename").expect("C extractFilename");
            let r_extract: Symbol<'static, unsafe extern "C" fn(*const c_char, c_char) -> *const c_char> =
                r.get(b"extractFilename").expect("Rust extractFilename");
            let c_create: Symbol<'static, unsafe extern "C" fn(*const c_char, *const c_char, usize) -> *mut c_char> =
                c.get(b"FIO_createFilename_fromOutDir").expect("C FIO_createFilename_fromOutDir");
            let r_create: Symbol<'static, unsafe extern "C" fn(*const c_char, *const c_char, usize) -> *mut c_char> =
                r.get(b"FIO_createFilename_fromOutDir").expect("Rust FIO_createFilename_fromOutDir");

            Libs {
                _c: std::ptr::read(c as *const Library),
                _r: std::ptr::read(r as *const Library),
                c_extract: std::mem::transmute(c_extract),
                r_extract: std::mem::transmute(r_extract),
                c_create: std::mem::transmute(c_create),
                r_create: std::mem::transmute(r_create),
            }
        }
    }
}

fn cmp_extract(libs: &Libs, path: &CStr, sep: c_char) {
    unsafe {
        let c_res = (libs.c_extract)(path.as_ptr(), sep);
        let r_res = (libs.r_extract)(path.as_ptr(), sep);
        let c_str = CStr::from_ptr(c_res);
        let r_str = CStr::from_ptr(r_res);
        assert_eq!(
            c_str, r_str,
            "extractFilename mismatch for path={:?} sep={:?}: C={:?} Rust={:?}",
            path, sep as u8 as char, c_str, r_str
        );
    }
}

fn cmp_create(libs: &Libs, path: &CStr, outdir: &CStr, suffix_len: usize) {
    unsafe {
        let c_res = (libs.c_create)(path.as_ptr(), outdir.as_ptr(), suffix_len);
        let r_res = (libs.r_create)(path.as_ptr(), outdir.as_ptr(), suffix_len);
        let c_str = CStr::from_ptr(c_res);
        let r_str = CStr::from_ptr(r_res);
        assert_eq!(
            c_str, r_str,
            "FIO_createFilename_fromOutDir mismatch for path={:?} outdir={:?} suffixLen={}: C={:?} Rust={:?}",
            path, outdir, suffix_len, c_str, r_str
        );
        // Free the C-allocated result
        libc_free(c_res as *mut _);
        libc_free(r_res as *mut _);
    }
}

extern "C" {
    fn free(ptr: *mut std::ffi::c_void);
}
unsafe fn libc_free(ptr: *mut std::ffi::c_void) {
    unsafe { free(ptr) }
}

#[test]
fn test_extract_filename() {
    let libs = Libs::load();
    let cases: Vec<(&[u8], u8)> = vec![
        (b"/home/user/file.txt\0", b'/'),
        (b"file.txt\0", b'/'),
        (b"/file.txt\0", b'/'),
        (b"a/b/c/d\0", b'/'),
        (b"/\0", b'/'),
        (b"noslash\0", b'/'),
        (b"C:\\Users\\file.txt\0", b'\\'),
        (b"C:\\Users\\file.txt\0", b'/'),
        (b"a/b\\c\0", b'/'),
        (b"a/b\\c\0", b'\\'),
        (b"/trailing/\0", b'/'),
    ];
    for (path_bytes, sep) in &cases {
        let path = unsafe { CStr::from_bytes_with_nul_unchecked(path_bytes) };
        cmp_extract(&libs, path, *sep as c_char);
    }
}

#[test]
fn test_fio_create_filename_from_outdir() {
    let libs = Libs::load();
    let cases: Vec<(&[u8], &[u8], usize)> = vec![
        // Basic: path with dir, outdir with trailing slash
        (b"/home/user/file.zst\0", b"/output/\0", 0),
        // outdir without trailing slash
        (b"/home/user/file.zst\0", b"/output\0", 0),
        // No directory in path
        (b"file.zst\0", b"/output/\0", 0),
        // suffix_len > 0
        (b"/home/user/file.zst\0", b"/output/\0", 4),
        (b"/home/user/file.zst\0", b"/output\0", 4),
        // Single char filename
        (b"/a\0", b"/out/\0", 0),
        (b"/a\0", b"/out\0", 0),
        // Deep path
        (b"/a/b/c/d/e/file.txt\0", b"/x/y/z/\0", 3),
        (b"/a/b/c/d/e/file.txt\0", b"/x/y/z\0", 3),
        // Filename only
        (b"myfile\0", b"/dest/\0", 0),
        (b"myfile\0", b"/dest\0", 0),
        // Large suffix
        (b"/path/to/file\0", b"/out/\0", 100),
    ];
    for (path_bytes, outdir_bytes, suffix_len) in &cases {
        let path = unsafe { CStr::from_bytes_with_nul_unchecked(path_bytes) };
        let outdir = unsafe { CStr::from_bytes_with_nul_unchecked(outdir_bytes) };
        cmp_create(&libs, path, outdir, *suffix_len);
    }
}
