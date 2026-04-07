use libloading::{Library, Symbol};
use std::ffi::CString;
use std::io::Read;
use std::os::raw::c_char;
use std::os::unix::io::FromRawFd;

/// Call `driver` from the given library, capturing its stdout output.
fn call_driver(lib: &Library, s1: &str, s2: &str) -> String {
    let s1 = CString::new(s1).unwrap();
    let s2 = CString::new(s2).unwrap();

    // Create a pipe to capture stdout
    let mut fds = [0i32; 2];
    unsafe { libc_pipe(fds.as_mut_ptr()) };
    let old_stdout = unsafe { libc_dup(1) };
    unsafe { libc_dup2(fds[1], 1) };

    unsafe {
        let func: Symbol<unsafe extern "C" fn(*const c_char, *const c_char)> =
            lib.get(b"driver").unwrap();
        func(s1.as_ptr(), s2.as_ptr());
    }

    // Flush and restore stdout
    unsafe {
        libc_fflush(std::ptr::null_mut());
        libc_dup2(old_stdout, 1);
        libc_close(old_stdout);
        libc_close(fds[1]);
    }

    let mut buf = String::new();
    let mut reader = unsafe { std::fs::File::from_raw_fd(fds[0]) };
    reader.read_to_string(&mut buf).unwrap();
    buf
}

extern "C" {
    #[link_name = "pipe"]
    fn libc_pipe(fds: *mut i32) -> i32;
    #[link_name = "dup"]
    fn libc_dup(fd: i32) -> i32;
    #[link_name = "dup2"]
    fn libc_dup2(old: i32, new: i32) -> i32;
    #[link_name = "close"]
    fn libc_close(fd: i32) -> i32;
    #[link_name = "fflush"]
    fn libc_fflush(stream: *mut std::ffi::c_void) -> i32;
}

fn c_lib() -> Library {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libdriver.so");
    unsafe { Library::new(path).expect("failed to load C .so") }
}

fn rust_lib() -> Library {
    // Find the Rust cdylib in target/debug/
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/debug");
    for entry in std::fs::read_dir(&dir).unwrap() {
        let p = entry.unwrap().path();
        if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
            if name.starts_with("libdriver") && name.ends_with(".so") {
                return unsafe { Library::new(&p).expect("failed to load Rust .so") };
            }
        }
    }
    panic!("Rust .so not found in {:?}", dir);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(s1: &str, s2: &str) {
        let c = call_driver(&c_lib(), s1, s2);
        let r = call_driver(&rust_lib(), s1, s2);
        assert_eq!(c, r, "mismatch for s1={:?} s2={:?}: C={:?} Rust={:?}", s1, s2, c, r);
    }

    #[test]
    fn basic_cases() {
        check("hello", "world");
        check("abcdef", "dc");
        check("abcdef", "xyz");
        check("", "abc");
        check("abc", "");
        check("", "");
        check("aaa", "a");
        check("abcabc", "c");
        check("hello world", " ");
        check("test123", "0123456789");
    }
}
