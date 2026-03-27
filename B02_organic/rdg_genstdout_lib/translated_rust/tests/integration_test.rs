use libloading::{Library, Symbol};
use std::ffi::CStr;
use std::os::raw::c_char;

const C_LIB_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libdriver.so");
const RUST_LIB_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/target/debug/libdriver.so");

extern "C" { fn free(ptr: *mut std::ffi::c_void); }

// ============ extractFilename tests ============

#[test]
fn test_extract_filename() {
    unsafe {
        let c_lib = Library::new(C_LIB_PATH).expect("load C lib");
        let rust_lib = Library::new(RUST_LIB_PATH).expect("load Rust lib");

        let c_fn: Symbol<unsafe extern "C" fn(*const c_char, c_char) -> *const c_char> =
            c_lib.get(b"extractFilename").expect("C extractFilename");
        let rust_fn: Symbol<unsafe extern "C" fn(*const c_char, c_char) -> *const c_char> =
            rust_lib.get(b"extractFilename").expect("Rust extractFilename");

        let cases: &[(&[u8], i8)] = &[
            (b"/home/user/file.txt\0", b'/' as i8),
            (b"file.txt\0", b'/' as i8),
            (b"/file.txt\0", b'/' as i8),
            (b"a/b/c/d\0", b'/' as i8),
            (b"nodelim\0", b'/' as i8),
            (b"/\0", b'/' as i8),
            (b"C:\\Users\\file.txt\0", b'\\' as i8),
        ];

        for (input, sep) in cases {
            let ptr = input.as_ptr() as *const c_char;
            let c_res = CStr::from_ptr(c_fn(ptr, *sep));
            let r_res = CStr::from_ptr(rust_fn(ptr, *sep));
            assert_eq!(c_res, r_res,
                "extractFilename mismatch for {:?} sep {:?}: C={:?} Rust={:?}",
                std::str::from_utf8(&input[..input.len()-1]).unwrap(),
                *sep as u8 as char, c_res, r_res);
        }
    }
}

// ============ FIO_createFilename_fromOutDir tests ============

#[test]
fn test_fio_create_filename_from_outdir() {
    unsafe {
        let c_lib = Library::new(C_LIB_PATH).expect("load C lib");
        let rust_lib = Library::new(RUST_LIB_PATH).expect("load Rust lib");

        type FioFn = unsafe extern "C" fn(*const c_char, *const c_char, usize) -> *mut c_char;
        let c_fn: Symbol<FioFn> = c_lib.get(b"FIO_createFilename_fromOutDir").expect("C FIO fn");
        let rust_fn: Symbol<FioFn> = rust_lib.get(b"FIO_createFilename_fromOutDir").expect("Rust FIO fn");

        let cases: &[(&[u8], &[u8], usize)] = &[
            (b"/home/user/file.zst\0", b"/tmp/out/\0", 0),
            (b"/home/user/file.zst\0", b"/tmp/out\0", 0),
            (b"file.zst\0", b"/tmp/out/\0", 0),
            (b"file.zst\0", b"/tmp/out\0", 0),
            (b"/home/user/file.zst\0", b"/tmp/out/\0", 4),
            (b"/home/user/file.zst\0", b"/tmp/out\0", 4),
            (b"just_a_file\0", b"outdir/\0", 0),
            (b"just_a_file\0", b"outdir\0", 0),
        ];

        for (path, outdir, suffix_len) in cases {
            let p = path.as_ptr() as *const c_char;
            let o = outdir.as_ptr() as *const c_char;

            let c_result = c_fn(p, o, *suffix_len);
            let rust_result = rust_fn(p, o, *suffix_len);

            let c_str = CStr::from_ptr(c_result);
            let r_str = CStr::from_ptr(rust_result);
            assert_eq!(c_str, r_str,
                "FIO mismatch: path={:?} outdir={:?} suffix={}: C={:?} Rust={:?}",
                std::str::from_utf8(&path[..path.len()-1]).unwrap(),
                std::str::from_utf8(&outdir[..outdir.len()-1]).unwrap(),
                suffix_len, c_str, r_str);

            free(c_result as *mut _);
            free(rust_result as *mut _);
        }
    }
}
