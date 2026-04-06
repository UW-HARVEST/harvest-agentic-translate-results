use libloading::{Library, Symbol};
use std::ffi::c_int;

/// Capture stdout from a closure that calls printf-based C/Rust functions.
/// We fork, redirect stdout to a pipe, call the function, and read the output.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    // Flush before forking
    unsafe { libc::fflush(std::ptr::null_mut()) };

    let mut pipefd = [0i32; 2];
    assert_eq!(unsafe { libc::pipe(pipefd.as_mut_ptr()) }, 0);

    let pid = unsafe { libc::fork() };
    assert!(pid >= 0, "fork failed");

    if pid == 0 {
        // Child: redirect stdout to pipe write end, call f, exit
        unsafe {
            libc::close(pipefd[0]);
            libc::dup2(pipefd[1], 1);
            libc::close(pipefd[1]);
        }
        f();
        unsafe {
            libc::fflush(std::ptr::null_mut());
            libc::_exit(0);
        }
    } else {
        // Parent: read from pipe read end
        unsafe { libc::close(pipefd[1]) };
        let mut buf = vec![0u8; 4096];
        let mut total = 0usize;
        loop {
            let n = unsafe {
                libc::read(pipefd[0], buf[total..].as_mut_ptr() as *mut libc::c_void, buf.len() - total)
            };
            if n <= 0 { break; }
            total += n as usize;
            if total >= buf.len() { buf.resize(buf.len() * 2, 0); }
        }
        unsafe { libc::close(pipefd[0]) };
        let mut status = 0i32;
        unsafe { libc::waitpid(pid, &mut status, 0) };
        buf.truncate(total);
        buf
    }
}

fn c_lib_path() -> std::path::PathBuf {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest.join("c_src/build/libdriver.so")
}

#[test]
fn test_printLine_matching() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let c_printLine: Symbol<unsafe extern "C" fn(*const u8)> =
        unsafe { c_lib.get(b"printLine").expect("find C printLine") };

    let test_strings: &[&[u8]] = &[
        b"hello\0",
        b"\0",
        b"AAAA\0",
    ];

    for s in test_strings {
        let c_out = capture_stdout(|| unsafe { c_printLine(s.as_ptr()) });
        let rust_out = capture_stdout(|| unsafe { driver::printLine(s.as_ptr()) });
        assert_eq!(c_out, rust_out, "printLine mismatch for {:?}", s);
    }

    // Test NULL
    let c_out = capture_stdout(|| unsafe { c_printLine(std::ptr::null()) });
    let rust_out = capture_stdout(|| unsafe { driver::printLine(std::ptr::null()) });
    assert_eq!(c_out, rust_out, "printLine mismatch for NULL");
}

#[test]
fn test_driver_safe_values() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let c_driver: Symbol<unsafe extern "C" fn(c_int)> =
        unsafe { c_lib.get(b"driver").expect("find C driver") };

    // Test safe values: 0, 1, 50, 99, and values >= 100 (which skip the copy)
    let test_values: &[c_int] = &[0, 1, 10, 50, 98, 99, 100, 200];

    for &val in test_values {
        let c_out = capture_stdout(|| unsafe { c_driver(val) });
        let rust_out = capture_stdout(|| driver::driver(val));
        assert_eq!(c_out, rust_out, "driver mismatch for data={}", val);
    }
}
