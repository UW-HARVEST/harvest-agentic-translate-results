use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target/debug/libtranslated_rust.so");
    p
}

/// Capture stdout by redirecting fd 1 to a pipe, calling `f`, then restoring.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    use std::io::Read;
    use std::os::unix::io::FromRawFd;

    unsafe {
        let mut pipefd = [0 as c_int; 2];
        assert_eq!(libc::pipe(pipefd.as_mut_ptr()), 0);
        let saved = libc::dup(1);
        assert!(saved >= 0);
        libc::dup2(pipefd[1], 1);
        libc::close(pipefd[1]);

        f();
        libc::fflush(std::ptr::null_mut()); // flush C stdio
        libc::dup2(saved, 1);
        libc::close(saved);

        // Set read end non-blocking and read all
        let flags = libc::fcntl(pipefd[0], libc::F_GETFL);
        libc::fcntl(pipefd[0], libc::F_SETFL, flags | libc::O_NONBLOCK);
        let mut file = std::fs::File::from_raw_fd(pipefd[0]);
        let mut buf = Vec::new();
        let _ = file.read_to_end(&mut buf);
        buf
    }
}

#[test]
fn test_print_line_with_string() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };

    let msg = CString::new("hello").unwrap();

    let c_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn(*const c_char)> = c_lib.get(b"printLine").unwrap();
        f(msg.as_ptr());
    });
    let r_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn(*const c_char)> = r_lib.get(b"printLine").unwrap();
        f(msg.as_ptr());
    });
    assert_eq!(c_out, r_out, "printLine(\"hello\") mismatch:\n  C: {:?}\n  Rust: {:?}", 
        String::from_utf8_lossy(&c_out), String::from_utf8_lossy(&r_out));
}

#[test]
fn test_print_line_null() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };

    let c_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn(*const c_char)> = c_lib.get(b"printLine").unwrap();
        f(std::ptr::null());
    });
    let r_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn(*const c_char)> = r_lib.get(b"printLine").unwrap();
        f(std::ptr::null());
    });
    assert_eq!(c_out, r_out, "printLine(NULL) mismatch");
}

#[test]
fn test_good() {
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
    assert_eq!(c_out, r_out, "good() mismatch:\n  C: {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c_out), String::from_utf8_lossy(&r_out));
}

#[test]
fn test_bad() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };

    let c_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn()> = c_lib.get(b"bad").unwrap();
        f();
    });
    let r_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn()> = r_lib.get(b"bad").unwrap();
        f();
    });
    assert_eq!(c_out, r_out, "bad() mismatch:\n  C: {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c_out), String::from_utf8_lossy(&r_out));
}

#[test]
fn test_nm_exports_match() {
    let output = |path: &str| -> Vec<String> {
        let out = std::process::Command::new("nm")
            .args(["-D", path])
            .output()
            .unwrap();
        let text = String::from_utf8_lossy(&out.stdout);
        let mut syms: Vec<String> = text.lines()
            .filter_map(|l| {
                let parts: Vec<&str> = l.split_whitespace().collect();
                if parts.len() >= 3 && parts[1] == "T" 
                    && !parts[2].starts_with('_') {
                    Some(parts[2].to_string())
                } else { None }
            })
            .collect();
        syms.sort();
        syms
    };
    let c_syms = output(c_lib_path().to_str().unwrap());
    let r_syms = output(rust_lib_path().to_str().unwrap());
    assert_eq!(c_syms, r_syms, "Export mismatch:\n  C: {:?}\n  Rust: {:?}", c_syms, r_syms);
}
