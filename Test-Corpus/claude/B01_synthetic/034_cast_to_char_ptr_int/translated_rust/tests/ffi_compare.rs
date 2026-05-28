// Integration test: compare outputs of C and Rust shared libraries via FFI.
// Both libraries are loaded with libloading, and `driver(int)` is called.
// Standard output is captured via a pipe + dup2 for byte-for-byte comparison.

use std::ffi::OsStr;
use std::fs::File;
use std::io::Read;
use std::os::unix::io::{FromRawFd, IntoRawFd};
use std::path::PathBuf;
use std::sync::Mutex;

use libloading::{Library, Symbol};

// Serialize stdout redirection across tests to avoid races.
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_lib_path() -> PathBuf {
    project_root().join("build_so").join("libdriver_c.so")
}

fn rust_lib_path() -> PathBuf {
    // The Rust cdylib sits in target/<profile>/libdriver.so.
    // Cargo sets `OUT_DIR` for build scripts, but for tests we can use
    // CARGO_TARGET_DIR or the standard target path. Try debug first.
    let root = project_root();
    let candidates = [
        root.join("target/debug/libdriver.so"),
        root.join("target/release/libdriver.so"),
    ];
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    candidates[0].clone()
}

/// Capture stdout produced by the closure `f` (which typically calls into
/// a C/Rust .so that writes to libc stdout). Returns the captured bytes.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    // Take the lock — only one capture at a time.
    let _guard = STDOUT_LOCK.lock().unwrap();

    unsafe {
        // Flush Rust + libc stdout before redirecting.
        libc::fflush(std::ptr::null_mut());

        // Save current stdout fd.
        let saved = libc::dup(1);
        assert!(saved >= 0, "dup failed");

        // Create a pipe.
        let mut fds = [0i32; 2];
        let r = libc::pipe(fds.as_mut_ptr());
        assert!(r == 0, "pipe failed");
        let read_fd = fds[0];
        let write_fd = fds[1];

        // Redirect stdout (fd 1) to the write end of the pipe.
        let r = libc::dup2(write_fd, 1);
        assert!(r >= 0, "dup2 failed");
        libc::close(write_fd);

        // Run the closure.
        f();

        // Flush the libc stdout into the pipe.
        libc::fflush(std::ptr::null_mut());

        // Restore stdout.
        libc::dup2(saved, 1);
        libc::close(saved);

        // Read everything from the read end of the pipe.
        let mut file = File::from_raw_fd(read_fd);
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).unwrap();
        // file dropping closes read_fd.
        let _ = file.into_raw_fd();
        libc::close(read_fd);

        buf
    }
}

fn load_lib<P: AsRef<OsStr>>(path: P) -> Library {
    unsafe { Library::new(path).expect("failed to load library") }
}

fn call_driver(lib: &Library, x: i32) -> Vec<u8> {
    let func: Symbol<unsafe extern "C" fn(std::os::raw::c_int)> =
        unsafe { lib.get(b"driver").expect("driver symbol not found") };
    capture_stdout(|| unsafe { func(x) })
}

// All comparisons run in a single test so that cargo's concurrent test runner
// doesn't interleave its own status output with our captured stdout.
#[test]
fn driver_outputs_match_across_inputs() {
    assert!(
        c_lib_path().exists(),
        "C library missing at {}; build with `gcc -shared -fPIC -o build_so/libdriver_c.so c_src/src/main.c`",
        c_lib_path().display()
    );
    assert!(
        rust_lib_path().exists(),
        "Rust cdylib missing at {}; run `cargo build` first",
        rust_lib_path().display()
    );

    let c_lib = load_lib(c_lib_path());
    let r_lib = load_lib(rust_lib_path());

    // Verify the FFI export is reachable in both libraries.
    unsafe {
        let _: Symbol<unsafe extern "C" fn(std::os::raw::c_int)> =
            c_lib.get(b"driver").expect("C driver symbol missing");
        let _: Symbol<unsafe extern "C" fn(std::os::raw::c_int)> =
            r_lib.get(b"driver").expect("Rust driver symbol missing");
    }

    let mut inputs: Vec<i32> = Vec::new();
    inputs.extend([0i32, 1, 2, 7, 10, 42, 100, 255, 256, 65535, 65536, 1_000_000, i32::MAX]);
    inputs.extend([-1i32, -2, -42, -255, -65536, -1_000_000, i32::MIN, i32::MIN + 1]);
    inputs.extend([
        0x01020304, 0x7F7E7D7C, -0x01020304, -0x7F7E7D7C,
        0x55AA55AA_u32 as i32, 0xDEADBEEF_u32 as i32,
        0xCAFEBABE_u32 as i32, 0xBAAD_F00D_u32 as i32,
    ]);

    for &x in &inputs {
        let c_out = call_driver(&c_lib, x);
        let r_out = call_driver(&r_lib, x);
        assert_eq!(
            c_out, r_out,
            "driver({}) (0x{:08x}) mismatch: C={:?} Rust={:?}",
            x,
            x as u32,
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
    }
}
