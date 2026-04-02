use libloading::{Library, Symbol};
use std::ffi::CStr;
use std::os::unix::io::FromRawFd;
use std::io::Read;

/// Capture stdout from a closure that prints via C printf / Rust printf.
/// We dup stdout to a pipe, call the closure, then read the pipe.
fn capture_stdout<F: FnOnce()>(f: F) -> String {
    unsafe {
        let mut pipefd = [0i32; 2];
        assert_eq!(libc::pipe(pipefd.as_mut_ptr()), 0);
        let old_stdout = libc::dup(1);
        assert!(old_stdout >= 0);
        libc::dup2(pipefd[1], 1);
        libc::close(pipefd[1]);

        f();

        libc::fflush(std::ptr::null_mut()); // flush C stdout
        libc::dup2(old_stdout, 1);
        libc::close(old_stdout);

        let mut buf = String::new();
        let mut reader = std::fs::File::from_raw_fd(pipefd[0]);
        // Set non-blocking read with a small buffer approach
        reader.read_to_string(&mut buf).unwrap();
        buf
    }
}

fn c_lib_path() -> String {
    let manifest = env!("CARGO_MANIFEST_DIR");
    format!("{}/c_src/build/libhelxo_lib.so", manifest)
}

#[test]
fn test_helxo_output_matches() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("Failed to load C .so") };

    for &letter in &[b'l', b'z', b'a', b'!'] {
        let c_output = capture_stdout(|| unsafe {
            let c_helxo: Symbol<unsafe extern "C" fn(i8)> =
                c_lib.get(b"helxo").expect("helxo not found in C lib");
            c_helxo(letter as i8);
        });

        let rust_output = capture_stdout(|| {
            helxo_lib::helxo(letter as i8);
        });

        assert_eq!(
            c_output, rust_output,
            "Output mismatch for letter='{}' (0x{:02x}).\nC output:\n{}\nRust output:\n{}",
            letter as char, letter, c_output, rust_output
        );
    }
}

#[test]
fn test_strkey_matches() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("Failed to load C .so") };

    for n in [0, 1, 42, 999, -1, 100000] {
        let c_result: String = unsafe {
            let c_strkey: Symbol<unsafe extern "C" fn(i32) -> *const i8> =
                c_lib.get(b"strkey").expect("strkey not found in C lib");
            let ptr = c_strkey(n);
            CStr::from_ptr(ptr).to_string_lossy().into_owned()
        };

        // strkey uses a static buffer in C; Rust must match
        let rust_result = format!("test_{}", n);

        assert_eq!(
            c_result, rust_result,
            "strkey mismatch for n={}. C='{}', Rust='{}'",
            n, c_result, rust_result
        );
    }
}
