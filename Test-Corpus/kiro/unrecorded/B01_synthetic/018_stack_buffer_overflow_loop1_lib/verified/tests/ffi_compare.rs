use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, CString};
use std::io::Read;
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    // The cdylib is placed next to the test binary's deps dir
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target/debug/libdriver.so");
    p
}

/// Capture stdout from a closure by redirecting fd 1 to a pipe.
fn capture_stdout<F: FnOnce()>(f: F) -> String {
    // flush rust stdout first
    use std::io::Write;
    std::io::stdout().flush().unwrap();

    let (read_fd, write_fd) = nix_pipe();
    let saved = unsafe { libc::dup(1) };
    assert!(saved >= 0);
    unsafe { libc::dup2(write_fd, 1) };
    unsafe { libc::close(write_fd) };

    f();

    // flush libc stdout so printf output lands in the pipe
    unsafe { libc::fflush(std::ptr::null_mut()) };
    unsafe { libc::dup2(saved, 1) };
    unsafe { libc::close(saved) };

    let mut buf = String::new();
    let mut file = unsafe { std::fs::File::from_raw_fd(read_fd) };
    // set non-blocking so we don't hang if empty
    unsafe {
        let flags = libc::fcntl(read_fd, libc::F_GETFL);
        libc::fcntl(read_fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
    }
    let _ = file.read_to_string(&mut buf);
    buf
}

fn nix_pipe() -> (i32, i32) {
    let mut fds = [0i32; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    (fds[0], fds[1])
}

use std::os::unix::io::FromRawFd;

// We must serialize tests that capture stdout
use std::sync::Mutex;
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn test_print_int_line() {
    let _lock = STDOUT_LOCK.lock().unwrap();
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };

    for val in [0i32, 1, -1, 42, i32::MAX, i32::MIN] {
        let c_out = capture_stdout(|| unsafe {
            let f: Symbol<unsafe extern "C" fn(c_int)> =
                c_lib.get(b"printIntLine").unwrap();
            f(val);
        });
        let r_out = capture_stdout(|| unsafe {
            let f: Symbol<unsafe extern "C" fn(c_int)> =
                r_lib.get(b"printIntLine").unwrap();
            f(val);
        });
        assert_eq!(c_out, r_out, "printIntLine mismatch for {val}");
    }
}

#[test]
fn test_print_line() {
    let _lock = STDOUT_LOCK.lock().unwrap();
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };

    let cases = ["hello", "", "test 123"];
    for s in cases {
        let cs = CString::new(s).unwrap();
        let c_out = capture_stdout(|| unsafe {
            let f: Symbol<unsafe extern "C" fn(*const c_char)> =
                c_lib.get(b"printLine").unwrap();
            f(cs.as_ptr());
        });
        let r_out = capture_stdout(|| unsafe {
            let f: Symbol<unsafe extern "C" fn(*const c_char)> =
                r_lib.get(b"printLine").unwrap();
            f(cs.as_ptr());
        });
        assert_eq!(c_out, r_out, "printLine mismatch for {s:?}");
    }

    // Test NULL
    let c_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn(*const c_char)> =
            c_lib.get(b"printLine").unwrap();
        f(std::ptr::null());
    });
    let r_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn(*const c_char)> =
            r_lib.get(b"printLine").unwrap();
        f(std::ptr::null());
    });
    assert_eq!(c_out, r_out, "printLine mismatch for NULL");
}

#[test]
fn test_good() {
    let _lock = STDOUT_LOCK.lock().unwrap();
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };

    let c_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn()> = c_lib.get(b"good").unwrap();
        f();
    });
    let r_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn()> = r_lib.get(b"good").unwrap();
        f();
    });
    assert_eq!(c_out, r_out, "good() output mismatch");
}

#[test]
fn test_driver_good() {
    let _lock = STDOUT_LOCK.lock().unwrap();
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };

    for arg in [1i32, 2, 100] {
        let c_out = capture_stdout(|| unsafe {
            let f: Symbol<unsafe extern "C" fn(c_int)> =
                c_lib.get(b"driver").unwrap();
            f(arg);
        });
        let r_out = capture_stdout(|| unsafe {
            let f: Symbol<unsafe extern "C" fn(c_int)> =
                r_lib.get(b"driver").unwrap();
            f(arg);
        });
        assert_eq!(c_out, r_out, "driver({arg}) output mismatch");
    }
}

#[test]
fn test_driver_bad() {
    let _lock = STDOUT_LOCK.lock().unwrap();
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };

    let c_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn(c_int)> =
            c_lib.get(b"driver").unwrap();
        f(0);
    });
    let r_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn(c_int)> =
            r_lib.get(b"driver").unwrap();
        f(0);
    });
    assert_eq!(c_out, r_out, "driver(0) output mismatch");
}

#[test]
fn test_symbol_parity() {
    // Verify both .so files export the same set of public symbols
    let c_syms = get_dynamic_symbols(&c_lib_path());
    let r_syms = get_dynamic_symbols(&rust_lib_path());
    for sym in &c_syms {
        assert!(
            r_syms.contains(sym),
            "C exports {sym:?} but Rust .so does not"
        );
    }
}

fn get_dynamic_symbols(path: &std::path::Path) -> Vec<String> {
    let out = std::process::Command::new("nm")
        .arg("-D")
        .arg(path)
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines()
        .filter_map(|l| {
            let parts: Vec<&str> = l.split_whitespace().collect();
            if parts.len() == 3 && parts[1] == "T" {
                let name = parts[2];
                // skip linker-generated symbols
                if !name.starts_with('_') {
                    return Some(name.to_string());
                }
            }
            None
        })
        .collect()
}
