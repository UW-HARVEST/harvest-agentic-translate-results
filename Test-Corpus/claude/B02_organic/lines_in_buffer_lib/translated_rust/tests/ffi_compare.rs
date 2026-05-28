use libloading::{Library, Symbol};
use std::ffi::c_char;
use std::os::raw::c_void;

type CreateLinePointersFn = unsafe extern "C" fn(
    buffer: *mut c_char,
    num_lines: usize,
    buffer_size: usize,
) -> *mut *const c_char;

extern "C" {
    fn free(ptr: *mut c_void);
}

fn c_lib_path() -> &'static str {
    "c_src/build/libdriver.so"
}

fn rust_lib_path() -> &'static str {
    // Default to debug. Override via DRIVER_RUST_LIB env var for release.
    if std::path::Path::new("target/debug/libdriver.so").exists() {
        "target/debug/libdriver.so"
    } else {
        "target/release/libdriver.so"
    }
}

unsafe fn call_create(
    lib: &Library,
    buffer: &mut [u8],
    num_lines: usize,
) -> (*mut *const c_char, usize) {
    let buffer_size = buffer.len();
    let func: Symbol<CreateLinePointersFn> = lib.get(b"UTIL_createLinePointers").unwrap();
    let ptr = func(buffer.as_mut_ptr() as *mut c_char, num_lines, buffer_size);
    (ptr, buffer_size)
}

/// Compare returned pointer arrays from C and Rust as offsets relative to their
/// respective buffer base pointers (since absolute addresses differ).
unsafe fn assert_match_or_both_null(
    c_ptr: *mut *const c_char,
    c_base: *const c_char,
    rust_ptr: *mut *const c_char,
    rust_base: *const c_char,
    num_lines: usize,
) {
    if c_ptr.is_null() || rust_ptr.is_null() {
        assert!(
            c_ptr.is_null() && rust_ptr.is_null(),
            "Mismatch: C returned {:?}, Rust returned {:?}",
            c_ptr,
            rust_ptr
        );
        return;
    }
    for i in 0..num_lines {
        let c_off = (*c_ptr.add(i) as usize).wrapping_sub(c_base as usize);
        let r_off = (*rust_ptr.add(i) as usize).wrapping_sub(rust_base as usize);
        assert_eq!(c_off, r_off, "Pointer offset mismatch at index {}", i);
    }
}

fn run_case(buffer: &[u8], num_lines: usize) {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("failed to load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("failed to load Rust lib");

        let mut c_buf = buffer.to_vec();
        let mut r_buf = buffer.to_vec();

        let c_base = c_buf.as_mut_ptr() as *const c_char;
        let r_base = r_buf.as_mut_ptr() as *const c_char;

        let (c_result, _) = call_create(&c_lib, &mut c_buf, num_lines);
        let (r_result, _) = call_create(&r_lib, &mut r_buf, num_lines);

        assert_match_or_both_null(c_result, c_base, r_result, r_base, num_lines);

        if !c_result.is_null() {
            free(c_result as *mut c_void);
        }
        if !r_result.is_null() {
            free(r_result as *mut c_void);
        }
    }
}

#[test]
fn test_basic_three_lines() {
    // Three null-terminated strings packed in one buffer.
    let buf = b"hello\0world\0foo\0";
    run_case(buf, 3);
}

#[test]
fn test_single_line() {
    let buf = b"only_line\0";
    run_case(buf, 1);
}

#[test]
fn test_empty_lines() {
    // Empty strings (just null terminators)
    let buf = b"\0\0\0";
    run_case(buf, 3);
}

#[test]
fn test_more_lines_than_present_returns_null() {
    // Buffer has only 2 lines but request 5.
    let buf = b"a\0b\0";
    run_case(buf, 5);
}

#[test]
fn test_no_trailing_null() {
    // Buffer ends without a null terminator
    let buf = b"hello\0world";
    run_case(buf, 2);
}

#[test]
fn test_mixed_lengths() {
    let buf = b"a\0bb\0ccc\0dddd\0eeeee\0";
    run_case(buf, 5);
}

#[test]
fn test_zero_lines() {
    // Edge case: 0 lines requested
    let buf = b"hello\0";
    run_case(buf, 0);
}

#[test]
fn test_exact_buffer_with_nulls() {
    let buf = b"line1\0line2\0line3\0line4\0";
    run_case(buf, 4);
}

#[test]
fn test_partial_request() {
    // 4 lines available but only request 2
    let buf = b"a\0b\0c\0d\0";
    run_case(buf, 2);
}

#[test]
fn test_long_lines() {
    let mut buf = Vec::new();
    for i in 0..10 {
        let s = format!("line_number_{}_with_some_content", i);
        buf.extend_from_slice(s.as_bytes());
        buf.push(0);
    }
    run_case(&buf, 10);
}

#[test]
fn test_exactly_one_byte_per_line() {
    let buf = b"x\0y\0z\0";
    run_case(buf, 3);
}

#[test]
fn test_no_nulls_at_all() {
    // Buffer has zero null terminators - should consume everything as one "line"
    let buf = b"hellothere";
    run_case(buf, 1);
}

#[test]
fn test_no_nulls_request_two_lines() {
    // Buffer has zero null terminators but request 2 lines - should return null
    let buf = b"hellothere";
    run_case(buf, 2);
}
