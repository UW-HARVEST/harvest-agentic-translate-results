//! Shared harness: load the C and Rust shared libraries via `libloading` and
//! compare what each one writes to stdout.
//!
//! Both libraries print through libc's `printf`, so both share the process's
//! `stdout` stream. To capture a call's output we temporarily redirect file
//! descriptor 1 to a scratch file, flush all C streams, and read the file back.

use std::ffi::{c_char, c_int};
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use libloading::{Library, Symbol};

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_char) -> c_int;
}

/// `fflush(NULL)` flushes every open C output stream, which is what forces the
/// buffered `printf` output of both libraries out to fd 1.
fn flush_all_c_streams() {
    unsafe {
        fflush(std::ptr::null_mut());
    }
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn c_library_path() -> PathBuf {
    workspace_root().join("c_src/build/libdriver.so")
}

/// Build the crate's `cdylib` and return the path to it.
///
/// `cargo test` does not produce the `cdylib` artifact on its own (integration
/// tests don't link against it), so the harness builds it explicitly. A
/// dedicated target directory is used so this nested `cargo` invocation does
/// not contend with the outer one's build lock.
///
/// The active feature set is forwarded so the loaded `.so` matches the
/// configuration the tests themselves were compiled under.
fn rust_library_path() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target_dir = manifest_dir.join("target/cdylib-under-test");

    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut cmd = std::process::Command::new(cargo);
    cmd.current_dir(&manifest_dir)
        .arg("build")
        .arg("--lib")
        .arg("--release")
        .arg("--target-dir")
        .arg(&target_dir)
        .arg("--no-default-features");
    let features = active_features();
    if !features.is_empty() {
        cmd.arg("--features").arg(features.join(","));
    }
    // Don't inherit the outer cargo's job-server / wrapper state.
    cmd.env_remove("CARGO_MAKEFLAGS");
    cmd.env_remove("RUSTC_WORKSPACE_WRAPPER");

    let status = cmd
        .status()
        .expect("failed to spawn `cargo build --lib` for the cdylib under test");
    assert!(status.success(), "building the cdylib under test failed");

    target_dir.join("release/libdriver.so")
}

/// Features enabled for this test binary. The crate declares no `[features]`
/// today; when some are added, list them here behind matching
/// `#[cfg(feature = "...")]` pushes so the nested build stays in sync.
fn active_features() -> Vec<&'static str> {
    #[allow(unused_mut)]
    let mut features: Vec<&'static str> = Vec::new();
    features
}

/// The two libraries under comparison, loaded once for the whole test binary.
pub struct Libs {
    pub c: Library,
    pub rust: Library,
}

pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let c_path = c_library_path();
        let rust_path = rust_library_path();
        assert!(
            c_path.exists(),
            "C shared library not found at {}. Build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            c_path.display()
        );
        assert!(
            rust_path.exists(),
            "Rust cdylib not found at {}",
            rust_path.display()
        );
        unsafe {
            Libs {
                c: Library::new(&c_path).expect("load C libdriver.so"),
                rust: Library::new(&rust_path).expect("load Rust libdriver.so"),
            }
        }
    })
}

/// Serializes stdout redirection, which is process-global state.
fn capture_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// Run `f` with fd 1 pointed at a scratch file and return the raw bytes it
/// wrote. Bytes are returned exactly as produced, with no decoding.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());

    let mut scratch = tempfile();

    // Don't let previously buffered output land in our capture.
    flush_all_c_streams();

    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");

    let scratch_fd = as_raw_fd(&scratch);
    assert!(unsafe { dup2(scratch_fd, 1) } >= 0, "dup2 onto stdout failed");

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

    // Flush whatever the callee buffered *before* restoring fd 1.
    flush_all_c_streams();
    assert!(unsafe { dup2(saved, 1) } >= 0, "restoring stdout failed");
    unsafe {
        close(saved);
    }

    if let Err(payload) = result {
        std::panic::resume_unwind(payload);
    }

    scratch.seek(SeekFrom::Start(0)).expect("seek scratch file");
    let mut bytes = Vec::new();
    scratch.read_to_end(&mut bytes).expect("read scratch file");
    bytes
}

fn tempfile() -> std::fs::File {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "driver-capture-{}-{}-{}.tmp",
        std::process::id(),
        n,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .expect("create scratch file");
    // Unlink now; the fd keeps it alive until dropped.
    let _ = std::fs::remove_file(&path);
    file
}

fn as_raw_fd(file: &std::fs::File) -> c_int {
    use std::os::fd::AsRawFd;
    file.as_raw_fd()
}

pub type DriverFn = unsafe extern "C" fn(c_int);

/// Look up `driver` in a library. This exercises the `#[no_mangle]` export
/// wrapper on the Rust side, exactly as an external C caller would.
pub fn driver_symbol(lib: &Library) -> Symbol<'_, DriverFn> {
    unsafe { lib.get(b"driver\0").expect("`driver` symbol must be exported") }
}

/// Call `driver(x)` in both libraries and return `(c_output, rust_output)`.
pub fn run_both(x: c_int) -> (Vec<u8>, Vec<u8>) {
    let libs = libs();
    let c_driver = driver_symbol(&libs.c);
    let rust_driver = driver_symbol(&libs.rust);

    let c_out = capture_stdout(|| unsafe { c_driver(x) });
    let rust_out = capture_stdout(|| unsafe { rust_driver(x) });
    (c_out, rust_out)
}

/// Assert byte-for-byte equality, reporting a hex dump on mismatch.
pub fn assert_same(x: c_int, c_out: &[u8], rust_out: &[u8]) {
    assert_eq!(
        c_out,
        rust_out,
        "driver({x}) output mismatch\n  C   ({} bytes): {}\n  Rust({} bytes): {}",
        c_out.len(),
        hex(c_out),
        rust_out.len(),
        hex(rust_out),
    );
}

pub fn hex(bytes: &[u8]) -> String {
    let mut s = String::new();
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}
