use libloading::{Library, Symbol};
use std::os::unix::io::FromRawFd;
use std::io::{Read, Write};

/// Capture stdout from a closure by redirecting fd 1 to a pipe.
fn capture_stdout<F: FnOnce()>(f: F) -> String {
    // Flush any pending Rust stdout before redirecting
    let _ = std::io::stdout().flush();
    unsafe {
        let mut fds = [0i32; 2];
        assert_eq!(libc::pipe(fds.as_mut_ptr()), 0);
        let old_stdout = libc::dup(1);
        libc::dup2(fds[1], 1);
        libc::close(fds[1]);

        f();

        // Flush both C and Rust stdout
        libc::fflush(std::ptr::null_mut());
        let _ = std::io::stdout().flush();

        libc::dup2(old_stdout, 1);
        libc::close(old_stdout);

        let mut buf = String::new();
        let mut read_end = std::fs::File::from_raw_fd(fds[0]);
        libc::fcntl(fds[0], libc::F_SETFL, libc::O_NONBLOCK);
        let _ = read_end.read_to_string(&mut buf);
        buf
    }
}

fn c_lib_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("libdriver.so")
}

fn rust_lib_path() -> std::path::PathBuf {
    // The cdylib is built in target/debug/ (or release/)
    let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target");
    p.push("debug");
    p.push("libdriver.so");
    p
}

#[test]
fn test_driver_outputs_match() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C .so") };
    let rust_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust .so") };

    let test_values: &[i32] = &[0, 1, -1, 42, 100, i32::MAX, i32::MIN];

    for &val in test_values {
        let c_output = {
            let func: Symbol<unsafe extern "C" fn(i32)> =
                unsafe { c_lib.get(b"driver").expect("C driver symbol") };
            capture_stdout(|| unsafe { func(val) })
        };

        let rust_output = {
            let func: Symbol<unsafe extern "C" fn(i32)> =
                unsafe { rust_lib.get(b"driver").expect("Rust driver symbol") };
            capture_stdout(|| unsafe { func(val) })
        };

        assert_eq!(
            c_output, rust_output,
            "driver({}) mismatch:\n  C:    {:?}\n  Rust: {:?}",
            val, c_output, rust_output
        );
    }
}

#[test]
fn test_symbol_exports_match() {
    // Verify both libraries export the same key symbols
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C .so") };
    let rust_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust .so") };

    for sym_name in &[b"driver" as &[u8], b"main" as &[u8]] {
        let c_sym: Result<Symbol<*const ()>, _> = unsafe { c_lib.get(sym_name) };
        let rust_sym: Result<Symbol<*const ()>, _> = unsafe { rust_lib.get(sym_name) };
        assert!(
            c_sym.is_ok(),
            "C .so missing symbol: {}",
            std::str::from_utf8(sym_name).unwrap()
        );
        assert!(
            rust_sym.is_ok(),
            "Rust .so missing symbol: {}",
            std::str::from_utf8(sym_name).unwrap()
        );
    }
}
