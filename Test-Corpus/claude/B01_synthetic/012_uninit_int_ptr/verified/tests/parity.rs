// Integration tests that compare the C shared library against the Rust
// shared library through their FFI exports.
//
// We test the lowest-level function (`printIntPtrLine`) first by calling
// it directly via libloading, then we test the higher-level `good()`
// function the same way. The `bad()` function and full `main()` flow are
// tested by spawning the binary equivalents (and the dlcall example
// program) so a SIGSEGV from undefined behavior cannot kill the test
// runner. In all cases we compare byte-for-byte stdout output between
// the C and Rust libraries.

use libloading::{Library, Symbol};
use std::ffi::OsStr;
use std::io::Write;
use std::os::raw::c_int;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so() -> PathBuf {
    workspace_root().join("c_src/build/libdriver_c.so")
}

fn rust_so() -> PathBuf {
    // libtest builds tests in the same profile as the lib; the Rust .so
    // for the lib lives next to the test binary's deps directory under
    // target/<profile>/.
    //
    // CARGO_MANIFEST_DIR / target / {debug,release} / libdriver.so
    // We try both debug and release.
    let root = workspace_root();
    for profile in &["debug", "release"] {
        let p = root.join("target").join(profile).join("libdriver.so");
        if p.exists() {
            return p;
        }
    }
    panic!("Rust libdriver.so not found in target/{{debug,release}}");
}

fn dlcall_bin() -> PathBuf {
    let root = workspace_root();
    for profile in &["debug", "release"] {
        let p = root
            .join("target")
            .join(profile)
            .join("examples")
            .join("dlcall");
        if p.exists() {
            return p;
        }
    }
    panic!("dlcall example binary not found; run `cargo build --example dlcall`");
}

fn ensure_built() {
    // Ensure the C shared library exists.
    if !c_so().exists() {
        let cs = workspace_root().join("c_src");
        let build = cs.join("build");
        std::fs::create_dir_all(&build).unwrap();
        let status = Command::new("gcc")
            .args([
                "-shared",
                "-fPIC",
                "-o",
            ])
            .arg(c_so())
            .arg(cs.join("src").join("main.c"))
            .status()
            .expect("failed to invoke gcc");
        assert!(status.success(), "gcc failed building C shared lib");
    }
    // Ensure the Rust shared library exists.
    if !rust_so().exists() {
        let status = Command::new(env!("CARGO"))
            .args(["build", "--lib"])
            .current_dir(workspace_root())
            .status()
            .expect("cargo build --lib failed to start");
        assert!(status.success(), "cargo build --lib failed");
    }
    if !dlcall_bin().exists() {
        let status = Command::new(env!("CARGO"))
            .args(["build", "--example", "dlcall"])
            .current_dir(workspace_root())
            .status()
            .expect("cargo build --example failed to start");
        assert!(status.success(), "cargo build --example dlcall failed");
    }
}

/// Run the dlcall helper to call `symbol` in `so_path` and capture stdout.
fn dlcall_capture(so_path: &OsStr, symbol: &str) -> (Vec<u8>, Option<i32>) {
    let out = Command::new(dlcall_bin())
        .arg(so_path)
        .arg(symbol)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .expect("failed to spawn dlcall");
    (out.stdout, out.status.code())
}

/// Call printIntPtrLine in-process (safe for non-crashing pointers).
fn call_print_int_ptr_line(so_path: &OsStr, value: c_int) -> String {
    // Use a pipe to capture the C library's stdout writes. dup2 stdout
    // to the write end of a pipe, call the function, restore stdout.
    use std::os::fd::AsRawFd;

    // Create a pipe.
    let mut fds: [libc::c_int; 2] = [0; 2];
    let r = unsafe { libc::pipe(fds.as_mut_ptr()) };
    assert_eq!(r, 0, "pipe() failed");
    let read_fd = fds[0];
    let write_fd = fds[1];

    let stdout = std::io::stdout();
    let saved = unsafe { libc::dup(stdout.as_raw_fd()) };
    assert!(saved >= 0);

    // Make sure libc stdout is flushed before swapping.
    unsafe {
        libc::fflush(std::ptr::null_mut());
    }
    let _ = std::io::stdout().flush();

    let r = unsafe { libc::dup2(write_fd, stdout.as_raw_fd()) };
    assert!(r >= 0);
    unsafe { libc::close(write_fd) };

    // Load library and call function.
    let result_str = {
        let lib = unsafe { Library::new(so_path) }.expect("dlopen failed");
        unsafe {
            type PrintFn = unsafe extern "C" fn(*const c_int);
            let f: Symbol<PrintFn> = lib.get(b"printIntPtrLine").expect("symbol");
            f(&value as *const c_int);
        }
        // Flush any libc-buffered stdout writes from the C library.
        unsafe {
            libc::fflush(std::ptr::null_mut());
        }
        let _ = std::io::stdout().flush();

        // Restore stdout.
        unsafe {
            libc::dup2(saved, stdout.as_raw_fd());
            libc::close(saved);
        }

        // Read the captured bytes.
        let mut buf = Vec::new();
        unsafe {
            let mut tmp = [0u8; 4096];
            // Set read end to nonblocking-ish: just read until empty.
            // Use libc::read in a loop with a short read; the writer end
            // was closed, so EOF will return 0.
            loop {
                let n = libc::read(
                    read_fd,
                    tmp.as_mut_ptr() as *mut libc::c_void,
                    tmp.len(),
                );
                if n <= 0 {
                    break;
                }
                buf.extend_from_slice(&tmp[..n as usize]);
            }
            libc::close(read_fd);
        }
        String::from_utf8(buf).expect("non-utf8 stdout")
    };

    result_str
}

// ---------------------------------------------------------------------------
// Lowest-level: printIntPtrLine
// ---------------------------------------------------------------------------

#[test]
fn print_int_ptr_line_via_dlcall_zero() {
    ensure_built();
    let c = dlcall_capture(c_so().as_os_str(), "printIntPtrLine_zero");
    let r = dlcall_capture(rust_so().as_os_str(), "printIntPtrLine_zero");
    assert_eq!(c.1, Some(0), "C exit code");
    assert_eq!(r.1, Some(0), "R exit code");
    assert_eq!(c.0, r.0, "stdout mismatch for value 0");
    assert_eq!(c.0, b"0\n");
}

#[test]
fn print_int_ptr_line_via_dlcall_42() {
    ensure_built();
    let c = dlcall_capture(c_so().as_os_str(), "printIntPtrLine_42");
    let r = dlcall_capture(rust_so().as_os_str(), "printIntPtrLine_42");
    assert_eq!(c.1, Some(0));
    assert_eq!(r.1, Some(0));
    assert_eq!(c.0, r.0);
    assert_eq!(c.0, b"42\n");
}

#[test]
fn print_int_ptr_line_via_dlcall_neg7() {
    ensure_built();
    let c = dlcall_capture(c_so().as_os_str(), "printIntPtrLine_neg7");
    let r = dlcall_capture(rust_so().as_os_str(), "printIntPtrLine_neg7");
    assert_eq!(c.1, Some(0));
    assert_eq!(r.1, Some(0));
    assert_eq!(c.0, r.0);
    assert_eq!(c.0, b"-7\n");
}

#[test]
fn print_int_ptr_line_via_dlcall_imax() {
    ensure_built();
    let c = dlcall_capture(c_so().as_os_str(), "printIntPtrLine_imax");
    let r = dlcall_capture(rust_so().as_os_str(), "printIntPtrLine_imax");
    assert_eq!(c.1, Some(0));
    assert_eq!(r.1, Some(0));
    assert_eq!(c.0, r.0);
    assert_eq!(c.0, b"2147483647\n");
}

#[test]
fn print_int_ptr_line_via_dlcall_imin() {
    ensure_built();
    let c = dlcall_capture(c_so().as_os_str(), "printIntPtrLine_imin");
    let r = dlcall_capture(rust_so().as_os_str(), "printIntPtrLine_imin");
    assert_eq!(c.1, Some(0));
    assert_eq!(r.1, Some(0));
    assert_eq!(c.0, r.0);
    assert_eq!(c.0, b"-2147483648\n");
}

#[test]
fn print_int_ptr_line_in_process_various() {
    // In-process libloading test: load each .so, call printIntPtrLine
    // directly through the FFI, and compare stdout.
    ensure_built();
    let c_path = c_so();
    let r_path = rust_so();

    for v in [0i32, 1, -1, 42, -42, 12345, -12345, i32::MAX, i32::MIN] {
        let c_out = call_print_int_ptr_line(c_path.as_os_str(), v);
        let r_out = call_print_int_ptr_line(r_path.as_os_str(), v);
        assert_eq!(c_out, r_out, "mismatch for value {}", v);
        assert_eq!(c_out, format!("{}\n", v));
    }
}

// ---------------------------------------------------------------------------
// Mid-level: good()
// ---------------------------------------------------------------------------

#[test]
fn good_via_dlcall() {
    ensure_built();
    let c = dlcall_capture(c_so().as_os_str(), "good");
    let r = dlcall_capture(rust_so().as_os_str(), "good");
    assert_eq!(c.1, Some(0));
    assert_eq!(r.1, Some(0));
    assert_eq!(c.0, r.0);
    assert_eq!(c.0, b"5\n");
}

#[test]
fn good_in_process() {
    // Call good() directly via dlopen in the test process.
    ensure_built();
    use std::os::fd::AsRawFd;

    fn run(so: &OsStr) -> String {
        let mut fds: [libc::c_int; 2] = [0; 2];
        unsafe {
            assert_eq!(libc::pipe(fds.as_mut_ptr()), 0);
        }
        let read_fd = fds[0];
        let write_fd = fds[1];
        let stdout = std::io::stdout();
        let saved = unsafe { libc::dup(stdout.as_raw_fd()) };
        unsafe {
            libc::fflush(std::ptr::null_mut());
        }
        let _ = std::io::stdout().flush();
        unsafe {
            libc::dup2(write_fd, stdout.as_raw_fd());
            libc::close(write_fd);
        }

        {
            let lib = unsafe { Library::new(so) }.expect("dlopen failed");
            unsafe {
                type VoidFn = unsafe extern "C" fn();
                let f: Symbol<VoidFn> = lib.get(b"good").expect("good");
                f();
            }
            unsafe {
                libc::fflush(std::ptr::null_mut());
            }
            let _ = std::io::stdout().flush();
        }

        unsafe {
            libc::dup2(saved, stdout.as_raw_fd());
            libc::close(saved);
        }

        let mut buf = Vec::new();
        unsafe {
            let mut tmp = [0u8; 4096];
            loop {
                let n = libc::read(
                    read_fd,
                    tmp.as_mut_ptr() as *mut libc::c_void,
                    tmp.len(),
                );
                if n <= 0 {
                    break;
                }
                buf.extend_from_slice(&tmp[..n as usize]);
            }
            libc::close(read_fd);
        }
        String::from_utf8(buf).unwrap()
    }

    let c_out = run(c_so().as_os_str());
    let r_out = run(rust_so().as_os_str());
    assert_eq!(c_out, r_out);
    assert_eq!(c_out, "5\n");
}

// ---------------------------------------------------------------------------
// Top-level: main() via the standalone executable
// ---------------------------------------------------------------------------
//
// The original C code is intended to be a CWE-457 demonstration. Calling
// bad() through the in-process dlopen path observes uninitialized stack
// memory from the test runner's stack, which is *not* the same stack
// state as the standalone driver binary. To compare apples-to-apples,
// we run the standalone executables (C and Rust) and compare their
// stdout. The standalone configuration is what the project ships, and
// both should produce identical output.

fn run_binary_with_input(bin: &OsStr, input: &str) -> (Vec<u8>, Option<i32>) {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn");
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        stdin.write_all(input.as_bytes()).expect("write");
    }
    let out = child.wait_with_output().expect("wait_with_output");
    (out.stdout, out.status.code())
}

fn c_bin() -> PathBuf {
    workspace_root().join("c_src/build/driver")
}

fn rust_bin() -> PathBuf {
    let root = workspace_root();
    for profile in &["debug", "release"] {
        let p = root.join("target").join(profile).join("driver");
        if p.exists() {
            return p;
        }
    }
    panic!("Rust driver binary not found");
}

fn ensure_binaries() {
    if !c_bin().exists() {
        let status = Command::new("cmake")
            .args(["--build", "."])
            .current_dir(workspace_root().join("c_src/build"))
            .status()
            .expect("cmake --build failed to start");
        assert!(status.success());
    }
    if !rust_bin().exists() {
        let status = Command::new(env!("CARGO"))
            .args(["build", "--bin", "driver"])
            .current_dir(workspace_root())
            .status()
            .expect("cargo build --bin failed to start");
        assert!(status.success());
    }
}

#[test]
fn main_binary_input_1() {
    ensure_binaries();
    let c = run_binary_with_input(c_bin().as_os_str(), "1\n");
    let r = run_binary_with_input(rust_bin().as_os_str(), "1\n");
    assert_eq!(c.0, r.0, "stdout mismatch on input '1'");
    assert_eq!(c.0, b"5\n");
    // Both should exit cleanly.
    assert_eq!(c.1, Some(0));
    assert_eq!(r.1, Some(0));
}

#[test]
fn main_binary_input_0() {
    // The "bad" path. With the default unoptimized build the C binary
    // happens to print "0\n" and exit cleanly (the uninitialized
    // pointer reads a zero word). Our Rust translation reproduces this
    // observable standalone behavior.
    ensure_binaries();
    let c = run_binary_with_input(c_bin().as_os_str(), "0\n");
    let r = run_binary_with_input(rust_bin().as_os_str(), "0\n");
    assert_eq!(c.0, r.0, "stdout mismatch on input '0'");
    assert_eq!(c.1, r.1, "exit code mismatch on input '0'");
}

#[test]
fn main_binary_input_42() {
    ensure_binaries();
    let c = run_binary_with_input(c_bin().as_os_str(), "42\n");
    let r = run_binary_with_input(rust_bin().as_os_str(), "42\n");
    assert_eq!(c.0, r.0);
    assert_eq!(c.0, b"5\n");
    assert_eq!(c.1, r.1);
}

#[test]
fn main_binary_input_negative() {
    ensure_binaries();
    let c = run_binary_with_input(c_bin().as_os_str(), "-3\n");
    let r = run_binary_with_input(rust_bin().as_os_str(), "-3\n");
    assert_eq!(c.0, r.0);
    assert_eq!(c.0, b"5\n");
    assert_eq!(c.1, r.1);
}
