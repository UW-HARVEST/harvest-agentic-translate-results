//! Shared harness: loads the C and Rust shared libraries and captures the
//! stdout that each of them produces via `printf`.

use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::Mutex;

pub type DriverFn = unsafe extern "C" fn(c_int, c_int, c_int);

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn lseek(fd: c_int, offset: i64, whence: c_int) -> i64;
    fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
    fn fflush(stream: *mut c_void) -> c_int;
}

// The C `stdout` / `stderr` FILE* are shared by both libraries (both link the
// same libc), so flushing them from the test process flushes their output too.
unsafe extern "C" {
    static mut stdout: *mut c_void;
    static mut stderr: *mut c_void;
}

const O_RDWR: c_int = 0o2;
const O_CREAT: c_int = 0o100;
const O_TRUNC: c_int = 0o1000;

/// Root of the repository (parent of the `translation` crate directory).
pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate dir has a parent")
        .to_path_buf()
}

pub fn c_lib_path() -> PathBuf {
    let p = repo_root().join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {p:?}; build it with cmake first"
    );
    p
}

/// Path to the Rust cdylib produced for the profile the tests run under.
///
/// A `cdylib`-only crate has no lib target that `cargo test` builds on its own,
/// so the harness builds it on demand. The nested `cargo build` is safe because
/// cargo releases the build lock before running test binaries.
pub fn rust_lib_path() -> PathBuf {
    static BUILD: Mutex<()> = Mutex::new(());
    let _guard = BUILD.lock().unwrap_or_else(|e| e.into_inner());

    // CARGO_MANIFEST_DIR/target/<profile>/libdriver.so — derive <profile> from
    // the test executable's own location so debug/release both work.
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test binary>
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("test exe is under target/<profile>/deps");
    let p = profile_dir.join("libdriver.so");

    // Always rebuild: `cargo test` never builds the lib target of a
    // cdylib-only crate, so a stale .so left over from an earlier run would
    // otherwise be silently tested instead of the current sources.
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut cmd = std::process::Command::new(cargo);
    cmd.arg("build").arg("--lib");
    if profile_dir.file_name().and_then(|s| s.to_str()) == Some("release") {
        cmd.arg("--release");
    }
    // Mirror the feature selection the test binary itself was built with.
    cmd.args(cargo_feature_args());
    cmd.current_dir(env!("CARGO_MANIFEST_DIR"));
    let status = cmd.status().expect("spawn cargo build --lib");
    assert!(status.success(), "cargo build --lib failed");

    assert!(p.exists(), "Rust shared library not found at {p:?}");
    p
}

/// Reconstruct `--features` / `--no-default-features` flags from the
/// `CARGO_FEATURE_*` variables cargo sets for the current compilation.
fn cargo_feature_args() -> Vec<String> {
    let mut features: Vec<String> = Vec::new();
    for (k, _) in std::env::vars() {
        if let Some(name) = k.strip_prefix("CARGO_FEATURE_") {
            features.push(name.to_ascii_lowercase().replace('_', "-"));
        }
    }
    let mut args = vec!["--no-default-features".to_string()];
    if !features.is_empty() {
        args.push("--features".to_string());
        args.push(features.join(","));
    }
    args
}

/// Run `f` with the process's stdout (fd 1) redirected to a temporary file,
/// returning the raw bytes written.
///
/// The redirected fds are process-wide, so captures are serialized across test
/// threads.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    capture_fds(&[1], f).into_iter().next().unwrap()
}

/// Same as [`capture_stdout`], but returns stderr (fd 2). Stdout is captured
/// and discarded so it does not leak into the test runner's own output.
pub fn capture_stderr<F: FnOnce()>(f: F) -> Vec<u8> {
    let mut out = capture_fds(&[1, 2], f);
    out.pop().unwrap()
}

/// Redirect each fd in `fds` to its own temporary file, run `f`, and return the
/// captured bytes in the same order.
fn capture_fds<F: FnOnce()>(fds: &[c_int], f: F) -> Vec<Vec<u8>> {
    static LOCK: Mutex<()> = Mutex::new(());
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        // Flush anything already pending so it lands in the real streams.
        fflush(stdout);
        fflush(stderr);

        let mut saved = Vec::new();
        let mut tmps = Vec::new();
        for (i, &fd) in fds.iter().enumerate() {
            let tmp = std::env::temp_dir().join(format!(
                "driver_capture_{}_{:?}_{}.txt",
                std::process::id(),
                std::thread::current().id(),
                i
            ));
            let mut cpath: Vec<u8> = tmp.to_str().unwrap().as_bytes().to_vec();
            cpath.push(0);

            let s = dup(fd);
            assert!(s >= 0, "dup({fd}) failed");
            let tmpfd = open(
                cpath.as_ptr() as *const c_char,
                O_RDWR | O_CREAT | O_TRUNC,
                0o600 as c_int,
            );
            assert!(tmpfd >= 0, "open({tmp:?}) failed");
            assert!(dup2(tmpfd, fd) >= 0, "dup2 failed");

            saved.push(s);
            tmps.push((tmpfd, tmp));
        }

        f();

        // Force the libc stream buffers out before restoring the fds.
        fflush(stdout);
        fflush(stderr);

        for (i, &fd) in fds.iter().enumerate() {
            assert!(dup2(saved[i], fd) >= 0, "dup2 restore failed");
            close(saved[i]);
        }

        // Read back what was written.
        let mut results = Vec::new();
        for (tmpfd, tmp) in tmps {
            lseek(tmpfd, 0, 0 /* SEEK_SET */);
            let mut out = Vec::new();
            let mut buf = [0u8; 4096];
            loop {
                let n = read(tmpfd, buf.as_mut_ptr() as *mut c_void, buf.len());
                if n <= 0 {
                    break;
                }
                out.extend_from_slice(&buf[..n as usize]);
            }
            close(tmpfd);
            let _ = std::fs::remove_file(&tmp);
            results.push(out);
        }
        results
    }
}
