//! Shared harness: loads both the C and the Rust shared libraries through
//! `libloading` and captures whatever each one writes to file descriptor 1.
//!
//! The Rust library is *never* called directly; it is always dlopen'ed and its
//! `#[no_mangle]` exports are invoked exactly as an external C caller would, so
//! the export wrappers themselves are part of what is under test.

use std::ffi::{c_int, c_void};
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use libloading::Library;

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes every open C stdio stream in the process, which is
    /// what the loaded libraries write through.
    fn fflush(stream: *mut c_void) -> c_int;
}

/// Path to the C shared library built from `c_src/`.
pub fn c_so_path() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let p = root.join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {}. Build it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

/// Path to the Rust `cdylib` for the profile the test itself was built with.
///
/// The test executable lives in `target/<profile>/deps/`, so the sibling
/// `target/<profile>/libdriver.so` is the matching artifact. `cargo test` does
/// not emit the `cdylib` on its own (it only builds the test harness), so if the
/// artifact is missing it is produced here, once per test process, with the same
/// profile and feature set. Cargo's build lock is already released by the time
/// tests run, so this nested invocation is safe.
pub fn rust_so_path() -> PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let exe = std::env::current_exe().expect("current_exe");
        let profile_dir = exe
            .parent()
            .and_then(|deps| deps.parent())
            .expect("target/<profile>")
            .to_path_buf();
        let p = profile_dir.join("libdriver.so");
        if !p.exists() {
            let profile = profile_dir
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("debug")
                .to_string();
            let mut cmd = std::process::Command::new(
                std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()),
            );
            cmd.current_dir(env!("CARGO_MANIFEST_DIR"))
                .arg("build")
                .arg("--lib");
            if profile != "debug" {
                cmd.arg("--profile").arg(&profile);
            }
            // Reproduce the feature selection this test was compiled with, so the
            // cdylib under test matches the harness's configuration.
            cmd.arg("--no-default-features");
            let feats = enabled_features().join(",");
            if !feats.is_empty() {
                cmd.arg("--features").arg(&feats);
            }
            let status = cmd.status().expect("spawn cargo build for cdylib");
            assert!(status.success(), "`cargo build --lib` for the cdylib failed");
        }
        assert!(
            p.exists(),
            "Rust shared library not found at {}",
            p.display()
        );
        p
    })
    .clone()
}

/// Cargo features active in this test build. The crate declares no `[features]`,
/// so this is empty; it is kept so the harness keeps rebuilding the cdylib with a
/// matching configuration if features are ever added.
fn enabled_features() -> Vec<&'static str> {
    // Each future feature would be added here as:
    //   #[cfg(feature = "foo")] v.push("foo");
    Vec::new()
}

/// Both libraries, loaded side by side. Each is opened `RTLD_LOCAL` (libloading's
/// default), so the two identically named `driver` symbols do not collide.
pub struct Pair {
    pub c: Library,
    pub rust: Library,
}

pub fn load_pair() -> Pair {
    unsafe {
        Pair {
            c: Library::new(c_so_path()).expect("dlopen C library"),
            rust: Library::new(rust_so_path()).expect("dlopen Rust library"),
        }
    }
}

/// Runs `f` with fd 1 redirected into a temporary file and returns the raw bytes
/// it produced. C stdio buffers are flushed on both sides of the redirect so no
/// output leaks into, or out of, the captured region.
///
/// Redirecting fd 1 is a process-global operation, so captures are serialised: two
/// concurrent captures would interleave each other's output.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    static LOCK: Mutex<()> = Mutex::new(());
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    assert_single_threaded();

    static SEQ: AtomicU64 = AtomicU64::new(0);
    let mut tmp = std::env::temp_dir();
    tmp.push(format!(
        "driver_capture_{}_{}.txt",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));

    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&tmp)
        .expect("open temp capture file");

    let mut out = Vec::new();
    unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 onto stdout failed");

        f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "restore stdout failed");
        close(saved);

        file.seek(SeekFrom::Start(0)).expect("rewind capture");
        file.read_to_end(&mut out).expect("read capture");
    }

    drop(file);
    let _ = std::fs::remove_file(&tmp);
    out
}

pub type DriverFn = unsafe extern "C" fn(f32);

/// The capture mechanism rewires fd 1 for the whole process, so libtest must not
/// be running other tests concurrently — its own "test ... ok" progress lines
/// would otherwise be written into an active capture and be reported as a
/// spurious mismatch. `translation/.cargo/config.toml` sets this for a plain
/// `cargo test`; fail loudly rather than produce misleading diffs if it is lost.
fn assert_single_threaded() {
    static CHECKED: OnceLock<()> = OnceLock::new();
    CHECKED.get_or_init(|| {
        let v = std::env::var("RUST_TEST_THREADS").unwrap_or_default();
        assert_eq!(
            v, "1",
            "these differential tests redirect fd 1 process-wide and must run one at \
             a time; set RUST_TEST_THREADS=1 (or pass `-- --test-threads=1`). \
             Current value: {v:?}"
        );
    });
}
/// Calls `driver(x)` in one library and returns its exact stdout bytes.
pub fn run_driver(lib: &Library, x: f32) -> Vec<u8> {
    let sym: libloading::Symbol<DriverFn> =
        unsafe { lib.get(b"driver\0").expect("`driver` symbol") };
    capture_stdout(|| unsafe { sym(x) })
}

/// Renders a float unambiguously (by bit pattern) for assertion messages.
pub fn describe(x: f32) -> String {
    format!("bits=0x{:08x} value={:?}", x.to_bits(), x)
}
