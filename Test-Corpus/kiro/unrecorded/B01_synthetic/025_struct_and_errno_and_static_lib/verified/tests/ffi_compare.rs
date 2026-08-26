use libloading::{Library, Symbol};
use std::ffi::CString;
use std::io::Read;
use std::os::unix::io::FromRawFd;

const C_LIB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libdriver.so");

fn rust_lib_path() -> String {
    let dir = format!("{}/target/debug", env!("CARGO_MANIFEST_DIR"));
    for entry in std::fs::read_dir(&dir).unwrap() {
        let p = entry.unwrap().path();
        if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
            if name.starts_with("libdriver") && name.ends_with(".so") && !name.contains(".d") {
                return p.to_string_lossy().into_owned();
            }
        }
    }
    panic!("Rust .so not found in {}", dir);
}

/// Capture stdout produced by the closure by redirecting fd 1 to a pipe.
fn capture_stdout<F: FnOnce()>(f: F) -> String {
    unsafe {
        let mut fds = [0i32; 2];
        assert_eq!(libc::pipe(fds.as_mut_ptr()), 0);
        let old_stdout = libc::dup(1);
        libc::dup2(fds[1], 1);
        libc::close(fds[1]);
        f();
        libc::fflush(std::ptr::null_mut());
        libc::dup2(old_stdout, 1);
        libc::close(old_stdout);
        let mut buf = String::new();
        let mut reader = std::fs::File::from_raw_fd(fds[0]);
        reader.read_to_string(&mut buf).unwrap();
        buf
    }
}

/// Run a test in a forked subprocess so each library gets fresh global state.
/// Returns the captured stdout from the child.
fn run_in_subprocess<F: FnOnce() -> String>(f: F) -> String {
    unsafe {
        let mut fds = [0i32; 2];
        assert_eq!(libc::pipe(fds.as_mut_ptr()), 0);
        let pid = libc::fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // Child
            libc::close(fds[0]);
            let result = f();
            let bytes = result.as_bytes();
            let mut written = 0;
            while written < bytes.len() {
                let n = libc::write(fds[1], bytes[written..].as_ptr() as *const _, bytes.len() - written);
                if n <= 0 { break; }
                written += n as usize;
            }
            libc::close(fds[1]);
            libc::_exit(0);
        }
        // Parent
        libc::close(fds[1]);
        let mut buf = String::new();
        let mut reader = std::fs::File::from_raw_fd(fds[0]);
        reader.read_to_string(&mut buf).unwrap();
        let mut status = 0;
        libc::waitpid(pid, &mut status, 0);
        assert!(libc::WIFEXITED(status) && libc::WEXITSTATUS(status) == 0,
            "child process failed");
        buf
    }
}

fn call_driver_fresh(lib_path: &str, input: &str) -> String {
    let lib_path = lib_path.to_owned();
    let input = input.to_owned();
    run_in_subprocess(move || {
        capture_stdout(|| unsafe {
            let lib = Library::new(&lib_path).unwrap();
            let func: Symbol<unsafe extern "C" fn(*const i8)> = lib.get(b"driver").unwrap();
            let c_input = CString::new(input.as_str()).unwrap();
            func(c_input.as_ptr());
        })
    })
}

fn call_run_fresh(lib_path: &str, extra_bedrooms: i32) -> String {
    let lib_path = lib_path.to_owned();
    run_in_subprocess(move || {
        capture_stdout(|| unsafe {
            let lib = Library::new(&lib_path).unwrap();
            let func: Symbol<unsafe extern "C" fn(i32)> = lib.get(b"run").unwrap();
            func(extra_bedrooms);
        })
    })
}

#[test]
fn test_run_values() {
    let rust_so = rust_lib_path();
    for extra in [0, 1, 3, -1, 100, i32::MAX, i32::MIN] {
        let c_out = call_run_fresh(C_LIB, extra);
        let r_out = call_run_fresh(&rust_so, extra);
        assert_eq!(c_out, r_out, "run({}) mismatch", extra);
    }
}

#[test]
fn test_driver_valid() {
    let rust_so = rust_lib_path();
    for input in ["0", "1", "5", "-1", "100", "  42", "+7"] {
        let c_out = call_driver_fresh(C_LIB, input);
        let r_out = call_driver_fresh(&rust_so, input);
        assert_eq!(c_out, r_out, "driver({:?}) mismatch", input);
    }
}

#[test]
fn test_driver_invalid() {
    let rust_so = rust_lib_path();
    for input in ["", "abc", "  ", "++1", "--1"] {
        let c_out = call_driver_fresh(C_LIB, input);
        let r_out = call_driver_fresh(&rust_so, input);
        assert_eq!(c_out, r_out, "driver({:?}) mismatch", input);
    }
}

#[test]
fn test_driver_edge_cases() {
    let rust_so = rust_lib_path();
    for input in ["2147483647", "-2147483648", "2147483648", "-2147483649", "0x10", "10abc"] {
        let c_out = call_driver_fresh(C_LIB, input);
        let r_out = call_driver_fresh(&rust_so, input);
        assert_eq!(c_out, r_out, "driver({:?}) mismatch", input);
    }
}
