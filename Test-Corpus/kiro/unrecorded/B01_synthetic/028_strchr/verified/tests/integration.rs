use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::{c_char, c_int};

fn c_lib() -> Library {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("c_build/libdriver.so");
    unsafe { Library::new(&path).expect("failed to load C .so") }
}

fn rust_lib() -> Library {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/debug/libdriver.so");
    unsafe { Library::new(&path).expect("failed to load Rust .so") }
}

// ---- foo tests ----

fn call_foo(lib: &Library, input: &CString, c: c_char) -> c_int {
    unsafe {
        let f: Symbol<unsafe extern "C" fn(*const c_char, c_char) -> c_int> =
            lib.get(b"foo").unwrap();
        f(input.as_ptr(), c)
    }
}

#[test]
fn test_foo_empty() {
    let c = c_lib();
    let r = rust_lib();
    let input = CString::new("").unwrap();
    for ch in [b'A', b'x'] {
        let cv = call_foo(&c, &input, ch as c_char);
        let rv = call_foo(&r, &input, ch as c_char);
        assert_eq!(cv, rv, "foo mismatch for empty string, char={}", ch);
    }
}

#[test]
fn test_foo_no_match() {
    let c = c_lib();
    let r = rust_lib();
    let input = CString::new("hello world").unwrap();
    let cv = call_foo(&c, &input, b'Z' as c_char);
    let rv = call_foo(&r, &input, b'Z' as c_char);
    assert_eq!(cv, rv, "foo mismatch for no-match case");
}

#[test]
fn test_foo_single_match() {
    let c = c_lib();
    let r = rust_lib();
    let input = CString::new("hAllo").unwrap();
    let cv = call_foo(&c, &input, b'A' as c_char);
    let rv = call_foo(&r, &input, b'A' as c_char);
    assert_eq!(cv, rv, "foo mismatch for single match");
}

#[test]
fn test_foo_multiple_matches() {
    let c = c_lib();
    let r = rust_lib();
    let input = CString::new("AAxAxxA").unwrap();
    for ch in [b'A', b'x'] {
        let cv = call_foo(&c, &input, ch as c_char);
        let rv = call_foo(&r, &input, ch as c_char);
        assert_eq!(cv, rv, "foo mismatch for multiple matches, char={}", ch);
    }
}

#[test]
fn test_foo_all_same() {
    let c = c_lib();
    let r = rust_lib();
    let input = CString::new("AAAA").unwrap();
    let cv = call_foo(&c, &input, b'A' as c_char);
    let rv = call_foo(&r, &input, b'A' as c_char);
    assert_eq!(cv, rv, "foo mismatch for all-same string");
}

#[test]
fn test_foo_special_chars() {
    let c = c_lib();
    let r = rust_lib();
    let input = CString::new("a\tb\nc\rd").unwrap();
    for ch in [b'\t', b'\n', b'\r', b'a'] {
        let cv = call_foo(&c, &input, ch as c_char);
        let rv = call_foo(&r, &input, ch as c_char);
        assert_eq!(cv, rv, "foo mismatch for special char={}", ch);
    }
}

// ---- driver tests (capture stdout) ----

fn call_driver_capture(lib: &Library, input: &CString) -> Vec<u8> {
    use std::io::Read;
    // Redirect stdout to a pipe
    let (read_fd, write_fd) = unsafe {
        let mut fds = [0i32; 2];
        assert_eq!(libc::pipe(fds.as_mut_ptr()), 0);
        (fds[0], fds[1])
    };
    let old_stdout = unsafe { libc::dup(1) };
    unsafe {
        libc::dup2(write_fd, 1);
    }

    // Call driver
    unsafe {
        let f: Symbol<unsafe extern "C" fn(*const c_char)> = lib.get(b"driver").unwrap();
        f(input.as_ptr());
        // Flush C stdout
        libc::fflush(std::ptr::null_mut());
    }

    // Restore stdout
    unsafe {
        libc::dup2(old_stdout, 1);
        libc::close(old_stdout);
        libc::close(write_fd);
    }

    // Read captured output
    let mut buf = Vec::new();
    let mut file = unsafe { std::fs::File::from_raw_fd(read_fd) };
    file.read_to_end(&mut buf).unwrap();
    buf
}

use std::os::unix::io::FromRawFd;

#[test]
fn test_driver_basic() {
    let c = c_lib();
    let r = rust_lib();
    let input = CString::new("AxAxx").unwrap();
    let c_out = call_driver_capture(&c, &input);
    let r_out = call_driver_capture(&r, &input);
    assert_eq!(c_out, r_out, "driver output mismatch:\nC:    {:?}\nRust: {:?}",
        String::from_utf8_lossy(&c_out), String::from_utf8_lossy(&r_out));
}

#[test]
fn test_driver_empty() {
    let c = c_lib();
    let r = rust_lib();
    let input = CString::new("").unwrap();
    let c_out = call_driver_capture(&c, &input);
    let r_out = call_driver_capture(&r, &input);
    assert_eq!(c_out, r_out, "driver output mismatch for empty:\nC:    {:?}\nRust: {:?}",
        String::from_utf8_lossy(&c_out), String::from_utf8_lossy(&r_out));
}

#[test]
fn test_driver_no_matches() {
    let c = c_lib();
    let r = rust_lib();
    let input = CString::new("hello world").unwrap();
    let c_out = call_driver_capture(&c, &input);
    let r_out = call_driver_capture(&r, &input);
    assert_eq!(c_out, r_out, "driver output mismatch for no-matches:\nC:    {:?}\nRust: {:?}",
        String::from_utf8_lossy(&c_out), String::from_utf8_lossy(&r_out));
}
