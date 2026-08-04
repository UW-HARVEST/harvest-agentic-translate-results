use libloading::{Library, Symbol};
use std::ffi::c_char;
use std::path::PathBuf;

type CreateLinePointersFn =
    unsafe extern "C" fn(*mut c_char, usize, usize) -> *const *const c_char;

fn lib_paths() -> (PathBuf, PathBuf) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_lib = manifest.join("c_src/build/libdriver.so");
    let rust_lib = manifest.join("target/debug/libdriver.so");
    assert!(c_lib.exists(), "C .so not found at {c_lib:?}");
    assert!(rust_lib.exists(), "Rust .so not found at {rust_lib:?}");
    (c_lib, rust_lib)
}

/// Call UTIL_createLinePointers via a loaded library, return the pointer array as a Vec of byte slices.
/// We compare the offsets from buffer start and the string contents.
unsafe fn call_fn(
    lib: &Library,
    buffer: &mut [u8],
    num_lines: usize,
) -> Option<Vec<(usize, Vec<u8>)>> {
    let func: Symbol<CreateLinePointersFn> =
        lib.get(b"UTIL_createLinePointers").unwrap();
    let buf_ptr = buffer.as_mut_ptr() as *mut c_char;
    let buf_size = buffer.len();
    let result = func(buf_ptr, num_lines, buf_size);
    if result.is_null() {
        return None;
    }
    let mut entries = Vec::new();
    for i in 0..num_lines {
        let line_ptr = *result.add(i);
        let offset = (line_ptr as usize) - (buf_ptr as usize);
        // Read until null or end of buffer
        let max_len = buf_size.saturating_sub(offset);
        let mut bytes = Vec::new();
        for j in 0..max_len {
            let b = *line_ptr.add(j) as u8;
            if b == 0 {
                break;
            }
            bytes.push(b);
        }
        entries.push((offset, bytes));
    }
    // Free the returned pointer (it was malloc'd in C, alloc'd in Rust)
    libc_free(result as *mut _);
    Some(entries)
}

/// We need to free the returned pointer. For C lib it was malloc'd, for Rust it was alloc'd.
/// Since both are cdylib using system allocator, libc free should work for both.
unsafe fn libc_free(ptr: *mut std::ffi::c_void) {
    extern "C" {
        fn free(ptr: *mut std::ffi::c_void);
    }
    free(ptr);
}

fn compare(
    label: &str,
    buffer: &[u8],
    num_lines: usize,
    c_lib: &Library,
    rust_lib: &Library,
) {
    let mut c_buf = buffer.to_vec();
    let mut r_buf = buffer.to_vec();
    let c_result = unsafe { call_fn(c_lib, &mut c_buf, num_lines) };
    let r_result = unsafe { call_fn(rust_lib, &mut r_buf, num_lines) };
    assert_eq!(
        c_result.is_some(),
        r_result.is_some(),
        "{label}: NULL mismatch: C={}, Rust={}",
        c_result.is_some(),
        r_result.is_some()
    );
    if let (Some(c), Some(r)) = (c_result, r_result) {
        assert_eq!(c.len(), r.len(), "{label}: line count mismatch");
        for (i, (c_entry, r_entry)) in c.iter().zip(r.iter()).enumerate() {
            assert_eq!(
                c_entry.0, r_entry.0,
                "{label}: line {i} offset mismatch: C={}, Rust={}",
                c_entry.0, r_entry.0
            );
            assert_eq!(
                c_entry.1, r_entry.1,
                "{label}: line {i} content mismatch"
            );
        }
    }
}

#[test]
fn test_basic_two_lines() {
    let (c_path, r_path) = lib_paths();
    let c_lib = unsafe { Library::new(&c_path).unwrap() };
    let rust_lib = unsafe { Library::new(&r_path).unwrap() };
    // "hello\0world\0"
    let buf = b"hello\0world\0";
    compare("basic_two_lines", buf, 2, &c_lib, &rust_lib);
}

#[test]
fn test_single_line() {
    let (c_path, r_path) = lib_paths();
    let c_lib = unsafe { Library::new(&c_path).unwrap() };
    let rust_lib = unsafe { Library::new(&r_path).unwrap() };
    let buf = b"only_line\0";
    compare("single_line", buf, 1, &c_lib, &rust_lib);
}

#[test]
fn test_empty_strings() {
    let (c_path, r_path) = lib_paths();
    let c_lib = unsafe { Library::new(&c_path).unwrap() };
    let rust_lib = unsafe { Library::new(&r_path).unwrap() };
    // Three empty strings
    let buf = b"\0\0\0";
    compare("empty_strings", buf, 3, &c_lib, &rust_lib);
}

#[test]
fn test_num_lines_exceeds_actual() {
    let (c_path, r_path) = lib_paths();
    let c_lib = unsafe { Library::new(&c_path).unwrap() };
    let rust_lib = unsafe { Library::new(&r_path).unwrap() };
    // Only 1 line but asking for 3 — should return NULL
    let buf = b"one\0";
    compare("num_lines_exceeds", buf, 3, &c_lib, &rust_lib);
}

#[test]
fn test_no_null_terminator() {
    let (c_path, r_path) = lib_paths();
    let c_lib = unsafe { Library::new(&c_path).unwrap() };
    let rust_lib = unsafe { Library::new(&r_path).unwrap() };
    // Buffer with no null terminator, asking for 1 line
    let buf = b"abcdef";
    compare("no_null_term", buf, 1, &c_lib, &rust_lib);
}

#[test]
fn test_zero_lines() {
    let (c_path, r_path) = lib_paths();
    let c_lib = unsafe { Library::new(&c_path).unwrap() };
    let rust_lib = unsafe { Library::new(&r_path).unwrap() };
    let buf = b"data\0";
    compare("zero_lines", buf, 0, &c_lib, &rust_lib);
}

#[test]
fn test_buffer_size_zero() {
    let (c_path, r_path) = lib_paths();
    let c_lib = unsafe { Library::new(&c_path).unwrap() };
    let rust_lib = unsafe { Library::new(&r_path).unwrap() };
    // Empty buffer, asking for 1 line — should return NULL
    let buf: &[u8] = b"";
    compare("buffer_size_zero", buf, 1, &c_lib, &rust_lib);
}

#[test]
fn test_multiple_lines() {
    let (c_path, r_path) = lib_paths();
    let c_lib = unsafe { Library::new(&c_path).unwrap() };
    let rust_lib = unsafe { Library::new(&r_path).unwrap() };
    let buf = b"alpha\0beta\0gamma\0delta\0epsilon\0";
    compare("multiple_lines", buf, 5, &c_lib, &rust_lib);
}

#[test]
fn test_binary_content() {
    let (c_path, r_path) = lib_paths();
    let c_lib = unsafe { Library::new(&c_path).unwrap() };
    let rust_lib = unsafe { Library::new(&r_path).unwrap() };
    // Binary data with embedded nulls as separators
    let buf: &[u8] = &[1, 2, 3, 0, 255, 254, 253, 0];
    compare("binary_content", buf, 2, &c_lib, &rust_lib);
}

#[test]
fn test_exact_fit_no_trailing_null() {
    let (c_path, r_path) = lib_paths();
    let c_lib = unsafe { Library::new(&c_path).unwrap() };
    let rust_lib = unsafe { Library::new(&r_path).unwrap() };
    // Two strings, second has no trailing null, buffer ends exactly
    let buf = b"aa\0bb";
    compare("exact_fit_no_trailing", buf, 2, &c_lib, &rust_lib);
}
