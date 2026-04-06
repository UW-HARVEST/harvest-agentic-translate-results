use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::io::Read;
use std::os::unix::io::FromRawFd;

extern crate libc;

/// Capture stdout from a closure by redirecting fd 1 to a pipe.
fn capture_stdout(f: impl FnOnce()) -> Vec<u8> {
    use std::io::Write;
    std::io::stdout().flush().unwrap();
    unsafe { libc::fflush(std::ptr::null_mut()); }

    let mut pipe_fds = [0i32; 2];
    unsafe { libc::pipe(pipe_fds.as_mut_ptr()); }
    let (pipe_r, pipe_w) = (pipe_fds[0], pipe_fds[1]);

    let orig_stdout = unsafe { libc::dup(1) };
    unsafe { libc::dup2(pipe_w, 1); }
    unsafe { libc::close(pipe_w); }

    f();

    unsafe { libc::fflush(std::ptr::null_mut()); }
    std::io::stdout().flush().unwrap();

    unsafe { libc::dup2(orig_stdout, 1); }
    unsafe { libc::close(orig_stdout); }

    let mut buf = Vec::new();
    let mut reader = unsafe { std::fs::File::from_raw_fd(pipe_r) };
    unsafe {
        let flags = libc::fcntl(pipe_r, libc::F_GETFL);
        libc::fcntl(pipe_r, libc::F_SETFL, flags | libc::O_NONBLOCK);
    }
    let _ = reader.read_to_end(&mut buf);
    buf
}

#[test]
fn test_helloworld_output_matches() {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_lib_path = manifest.join("c_src/build/libhello.so");
    let rust_lib_path = manifest.join("target/debug/libhello.so");

    assert!(c_lib_path.exists(), "C lib not found: {:?}", c_lib_path);
    assert!(rust_lib_path.exists(), "Rust lib not found: {:?}. Run `cargo build` first.", rust_lib_path);

    let c_lib = unsafe { Library::new(&c_lib_path).expect("load C lib") };
    let rust_lib = unsafe { Library::new(&rust_lib_path).expect("load Rust lib") };

    let c_fn: Symbol<unsafe extern "C" fn() -> c_int> =
        unsafe { c_lib.get(b"helloworld").expect("C helloworld") };
    let rust_fn: Symbol<unsafe extern "C" fn() -> c_int> =
        unsafe { rust_lib.get(b"helloworld").expect("Rust helloworld") };

    // Capture C output
    let c_output = capture_stdout(|| { unsafe { c_fn(); } });
    // Capture Rust output
    let rust_output = capture_stdout(|| { unsafe { rust_fn(); } });

    // Compare return values
    let c_ret = unsafe { c_fn() };
    let rust_ret = unsafe { rust_fn() };
    assert_eq!(c_ret, rust_ret, "Return values differ: C={}, Rust={}", c_ret, rust_ret);

    // Compare stdout byte-for-byte
    assert_eq!(
        c_output, rust_output,
        "Stdout differs!\nC:    {:?}\nRust: {:?}",
        String::from_utf8_lossy(&c_output),
        String::from_utf8_lossy(&rust_output),
    );
}
