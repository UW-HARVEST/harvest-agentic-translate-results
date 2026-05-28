// Integration tests that compare the C and Rust .so implementations
// through libloading, calling the exported FFI symbols directly.

use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::c_char;

const C_SO_PATH: &str = "c_src/build/libdriver.so";
const RUST_SO_PATH: &str = "target/release/libdriver.so";

type ExtractFilenameFn = unsafe extern "C" fn(*const c_char, c_char) -> *const c_char;
type CreateFilenameFromOutDirFn =
    unsafe extern "C" fn(*const c_char, *const c_char, libc::size_t) -> *mut c_char;

fn load_libs() -> (Library, Library) {
    unsafe {
        let c_lib = Library::new(C_SO_PATH).expect("failed to load C .so");
        let rust_lib = Library::new(RUST_SO_PATH).expect("failed to load Rust .so");
        (c_lib, rust_lib)
    }
}

unsafe fn cstr_to_bytes<'a>(p: *const c_char) -> &'a [u8] {
    if p.is_null() {
        return &[];
    }
    let len = libc::strlen(p);
    std::slice::from_raw_parts(p as *const u8, len)
}

fn extract_filename_compare(path: &str, sep: u8) {
    let (c_lib, rust_lib) = load_libs();
    let c_input = CString::new(path).unwrap();

    unsafe {
        let c_fn: Symbol<ExtractFilenameFn> = c_lib.get(b"extractFilename").unwrap();
        let r_fn: Symbol<ExtractFilenameFn> = rust_lib.get(b"extractFilename").unwrap();

        let c_out = c_fn(c_input.as_ptr(), sep as c_char);
        let r_out = r_fn(c_input.as_ptr(), sep as c_char);

        // Both should return pointers to (potentially the same) NUL-terminated string
        // We compare the resulting strings byte-for-byte.
        let c_bytes = cstr_to_bytes(c_out);
        let r_bytes = cstr_to_bytes(r_out);
        assert_eq!(
            c_bytes, r_bytes,
            "extractFilename mismatch for path={:?} sep={:?}",
            path, sep as char
        );

        // Also check whether either was equal to the input pointer (i.e., no separator found)
        let c_is_input = c_out == c_input.as_ptr();
        let r_is_input = r_out == c_input.as_ptr();
        assert_eq!(
            c_is_input, r_is_input,
            "extractFilename pointer-identity mismatch for path={:?} sep={:?}",
            path, sep as char
        );
    }
}

fn create_filename_compare(path: &str, out_dir: &str, suffix_len: usize) {
    let (c_lib, rust_lib) = load_libs();
    let c_path = CString::new(path).unwrap();
    let c_out_dir = CString::new(out_dir).unwrap();

    unsafe {
        let c_fn: Symbol<CreateFilenameFromOutDirFn> =
            c_lib.get(b"FIO_createFilename_fromOutDir").unwrap();
        let r_fn: Symbol<CreateFilenameFromOutDirFn> =
            rust_lib.get(b"FIO_createFilename_fromOutDir").unwrap();

        let c_result = c_fn(c_path.as_ptr(), c_out_dir.as_ptr(), suffix_len);
        let r_result = r_fn(c_path.as_ptr(), c_out_dir.as_ptr(), suffix_len);

        assert!(!c_result.is_null());
        assert!(!r_result.is_null());

        // The function calloc's strlen(outDir) + 1 + strlen(filenameStart) + suffixLen + 1 bytes.
        // We compare the resulting strings byte-for-byte (the visible NUL-terminated portion).
        let c_str = cstr_to_bytes(c_result);
        let r_str = cstr_to_bytes(r_result);
        assert_eq!(
            c_str, r_str,
            "FIO_createFilename_fromOutDir string mismatch path={:?} out_dir={:?} suffix_len={}",
            path, out_dir, suffix_len
        );

        // Also compare the entire allocated buffer: out_dir_len + 1 + filename_len + suffix_len + 1
        let out_dir_len = libc::strlen(c_out_dir.as_ptr());
        // Use the C function on the path to get the filename start length consistently.
        let extract_c: Symbol<ExtractFilenameFn> = c_lib.get(b"extractFilename").unwrap();
        let filename_start = extract_c(c_path.as_ptr(), b'/' as c_char);
        let filename_len = libc::strlen(filename_start);
        let total = out_dir_len + 1 + filename_len + suffix_len + 1;

        let c_buf = std::slice::from_raw_parts(c_result as *const u8, total);
        let r_buf = std::slice::from_raw_parts(r_result as *const u8, total);
        assert_eq!(
            c_buf, r_buf,
            "FIO_createFilename_fromOutDir buffer mismatch path={:?} out_dir={:?} suffix_len={}",
            path, out_dir, suffix_len
        );

        libc::free(c_result as *mut libc::c_void);
        libc::free(r_result as *mut libc::c_void);
    }
}

#[test]
fn test_extract_filename_basic() {
    extract_filename_compare("foo.txt", b'/');
    extract_filename_compare("/foo/bar/baz.txt", b'/');
    extract_filename_compare("/", b'/');
    extract_filename_compare("", b'/');
    extract_filename_compare("a/b/c", b'/');
    extract_filename_compare("a/b/c/", b'/');
    extract_filename_compare("noseparators", b'/');
    extract_filename_compare("a\\b\\c", b'\\');
    extract_filename_compare("hello,world,foo", b',');
}

#[test]
fn test_create_filename_simple() {
    create_filename_compare("foo.txt", "outdir", 0);
    create_filename_compare("foo.txt", "outdir/", 0);
    create_filename_compare("/path/to/foo.txt", "outdir", 0);
    create_filename_compare("/path/to/foo.txt", "outdir/", 0);
    create_filename_compare("/path/to/foo.txt", "/abs/out", 4);
    create_filename_compare("/path/to/foo.txt", "/abs/out/", 4);
    create_filename_compare("noslashes", "out", 0);
    create_filename_compare("noslashes", "out/", 0);
}

#[test]
fn test_create_filename_with_suffix() {
    create_filename_compare("file.zst", "tmp", 4);
    create_filename_compare("file.zst", "tmp/", 16);
    create_filename_compare("a/b/file", ".", 8);
    create_filename_compare("a/b/file", "./", 8);
}

#[test]
fn test_create_filename_edge_cases() {
    // single character outDirName
    create_filename_compare("foo", "/", 0);
    create_filename_compare("foo", "x", 0);
    // ends in separator vs not
    create_filename_compare("/a/b/c.bin", "out", 10);
    create_filename_compare("/a/b/c.bin", "out/", 10);
    // path is just a filename
    create_filename_compare("filename", "outputdir", 5);
}
