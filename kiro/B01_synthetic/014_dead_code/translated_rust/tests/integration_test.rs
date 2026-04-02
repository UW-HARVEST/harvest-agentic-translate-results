use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::c_char;
use std::process::Command;

const C_LIB: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/c_src/build/libdriver.so"
);

fn rust_lib_path() -> String {
    let dir = env!("CARGO_MANIFEST_DIR");
    // Find the built cdylib
    let debug = format!("{}/target/debug/libdriver.so", dir);
    if std::path::Path::new(&debug).exists() {
        return debug;
    }
    let release = format!("{}/target/release/libdriver.so", dir);
    if std::path::Path::new(&release).exists() {
        return release;
    }
    panic!("Cannot find libdriver.so in target/debug or target/release");
}

/// Capture stdout from a void C function by forking
fn capture_c_void(f: impl FnOnce()) -> Vec<u8> {
    use std::io::Read;
    let (read_fd, write_fd) = nix_pipe();
    let old_stdout = unsafe { libc::dup(1) };
    unsafe { libc::dup2(write_fd, 1); }
    unsafe { libc::close(write_fd); }

    f();

    unsafe { libc::fflush(std::ptr::null_mut()); }
    unsafe { libc::dup2(old_stdout, 1); }
    unsafe { libc::close(old_stdout); }

    let mut buf = Vec::new();
    let mut file = unsafe { std::fs::File::from_raw_fd(read_fd) };
    file.read_to_end(&mut buf).unwrap();
    buf
}

fn nix_pipe() -> (i32, i32) {
    let mut fds = [0i32; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    (fds[0], fds[1])
}

use std::os::unix::io::FromRawFd;

#[test]
fn test_print_line() {
    let c_lib = unsafe { Library::new(C_LIB).expect("load C lib") };
    let rust_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    let msg = CString::new("hello test").unwrap();

    let c_out = capture_c_void(|| {
        unsafe {
            let f: Symbol<unsafe extern "C" fn(*const c_char)> =
                c_lib.get(b"printLine").unwrap();
            f(msg.as_ptr());
        }
    });

    let r_out = capture_c_void(|| {
        unsafe {
            let f: Symbol<unsafe extern "C" fn(*const c_char)> =
                rust_lib.get(b"printLine").unwrap();
            f(msg.as_ptr());
        }
    });

    assert_eq!(c_out, r_out, "printLine output mismatch:\nC:    {:?}\nRust: {:?}",
        String::from_utf8_lossy(&c_out), String::from_utf8_lossy(&r_out));
}

#[test]
fn test_print_line_null() {
    let c_lib = unsafe { Library::new(C_LIB).expect("load C lib") };
    let rust_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    let c_out = capture_c_void(|| {
        unsafe {
            let f: Symbol<unsafe extern "C" fn(*const c_char)> =
                c_lib.get(b"printLine").unwrap();
            f(std::ptr::null());
        }
    });

    let r_out = capture_c_void(|| {
        unsafe {
            let f: Symbol<unsafe extern "C" fn(*const c_char)> =
                rust_lib.get(b"printLine").unwrap();
            f(std::ptr::null());
        }
    });

    assert_eq!(c_out, r_out, "printLine(NULL) output mismatch");
}

#[test]
fn test_bad() {
    let c_lib = unsafe { Library::new(C_LIB).expect("load C lib") };
    let rust_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    let c_out = capture_c_void(|| {
        unsafe {
            let f: Symbol<unsafe extern "C" fn()> = c_lib.get(b"bad").unwrap();
            f();
        }
    });

    let r_out = capture_c_void(|| {
        unsafe {
            let f: Symbol<unsafe extern "C" fn()> = rust_lib.get(b"bad").unwrap();
            f();
        }
    });

    assert_eq!(c_out, r_out, "bad() output mismatch:\nC:    {:?}\nRust: {:?}",
        String::from_utf8_lossy(&c_out), String::from_utf8_lossy(&r_out));
}

#[test]
fn test_good() {
    let c_lib = unsafe { Library::new(C_LIB).expect("load C lib") };
    let rust_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    let c_out = capture_c_void(|| {
        unsafe {
            let f: Symbol<unsafe extern "C" fn()> = c_lib.get(b"good").unwrap();
            f();
        }
    });

    let r_out = capture_c_void(|| {
        unsafe {
            let f: Symbol<unsafe extern "C" fn()> = rust_lib.get(b"good").unwrap();
            f();
        }
    });

    assert_eq!(c_out, r_out, "good() output mismatch:\nC:    {:?}\nRust: {:?}",
        String::from_utf8_lossy(&c_out), String::from_utf8_lossy(&r_out));
}

#[test]
fn test_binary_output() {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let c_bin = format!("{}/c_src/build/driver", manifest);

    let c_output = Command::new(&c_bin)
        .output()
        .expect("run C binary");

    let rust_bin = format!("{}/target/debug/driver", manifest);
    let r_output = Command::new(&rust_bin)
        .output()
        .expect("run Rust binary");

    assert_eq!(c_output.stdout, r_output.stdout,
        "Binary stdout mismatch:\nC:    {:?}\nRust: {:?}",
        String::from_utf8_lossy(&c_output.stdout),
        String::from_utf8_lossy(&r_output.stdout));
}
