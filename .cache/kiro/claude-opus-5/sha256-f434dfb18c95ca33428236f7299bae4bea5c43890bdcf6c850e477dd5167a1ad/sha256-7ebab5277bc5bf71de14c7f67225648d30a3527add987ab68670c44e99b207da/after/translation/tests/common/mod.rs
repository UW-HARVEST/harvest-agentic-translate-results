//! Shared harness: loads the C and Rust shared objects via `libloading` and
//! captures whatever each one writes to `stdout` so the raw bytes can be
//! compared.
//!
//! Included by more than one integration test binary, so not every helper is
//! used by every consumer.
#![allow(dead_code, unused_imports)]

use std::ffi::{c_char, c_int, c_void};
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::process::Command;

use libloading::{Library, Symbol};

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn tmpfile() -> *mut c_void;
    fn fileno(stream: *mut c_void) -> c_int;
    fn fclose(stream: *mut c_void) -> c_int;
}

const STDOUT_FD: c_int = 1;

/// `capture_stdout` mutates the process-wide file descriptor 1, so only one
/// capture may be in flight at a time even though the test harness runs test
/// functions on several threads.
static CAPTURE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Path to the C shared library produced by `c_src/build`.
pub fn c_so_path() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation dir has a parent")
        .to_path_buf();
    let p = root.join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {}. Build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

/// Path to the Rust `cdylib` built from this crate.
///
/// `Cargo.toml` declares `crate-type = ["cdylib"]` only, so the integration
/// test binaries have no dependency edge on the library target and `cargo test`
/// will *not* rebuild the `.so` when `src/` changes. Loading whatever `.so`
/// happens to be lying around would let a stale artifact pass the differential
/// tests, so the harness builds the library itself, into a dedicated target
/// directory (a separate directory avoids contending for the outer `cargo`
/// invocation's build lock).
pub fn rust_so_path() -> PathBuf {
    static BUILT: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    BUILT.get_or_init(build_rust_so).clone()
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// True when the currently running test binary came from a release build.
fn is_release_profile() -> bool {
    std::env::current_exe()
        .ok()
        .map(|exe| exe.components().any(|c| c.as_os_str() == "release"))
        .unwrap_or(false)
}

fn build_rust_so() -> PathBuf {
    let manifest = manifest_dir();
    let target_dir = manifest.join("target/so-under-test");
    let release = is_release_profile();

    let mut cmd = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
    cmd.current_dir(&manifest)
        .arg("build")
        .arg("--lib")
        .arg("--target-dir")
        .arg(&target_dir);
    if release {
        cmd.arg("--release");
    }

    // Mirror the feature selection of the outer `cargo test` invocation. The
    // package currently declares no features, so these are normally unset; the
    // feature-sweep script sets them so each combination is honoured.
    if std::env::var("DRIVER_TEST_NO_DEFAULT_FEATURES").is_ok() {
        cmd.arg("--no-default-features");
    }
    if let Ok(features) = std::env::var("DRIVER_TEST_FEATURES") {
        if !features.is_empty() {
            cmd.arg("--features").arg(features);
        }
    }

    // Cargo env vars inherited from the outer invocation would otherwise steer
    // this nested build back at the outer target directory.
    cmd.env_remove("CARGO_TARGET_DIR");
    cmd.env_remove("RUSTFLAGS");

    let out = cmd.output().expect("failed to run `cargo build --lib`");
    assert!(
        out.status.success(),
        "building the cdylib under test failed:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let so = target_dir
        .join(if release { "release" } else { "debug" })
        .join("libdriver.so");
    assert!(
        so.exists(),
        "expected the cdylib at {} after a successful build",
        so.display()
    );
    so
}

/// The signature of the single public entry point: `void driver(float x);`
pub type DriverFn = unsafe extern "C" fn(f32);

pub struct Libs {
    // Field order matters for drop order: symbols borrow from the libraries,
    // but we store owned function pointers instead, so the libraries are only
    // kept alive to keep the code mapped.
    pub c_driver: DriverFn,
    pub rust_driver: DriverFn,
    _c_lib: Library,
    _rust_lib: Library,
}

impl Libs {
    pub fn load() -> Libs {
        unsafe {
            let c_lib = Library::new(c_so_path()).expect("dlopen C libdriver.so");
            let rust_lib = Library::new(rust_so_path()).expect("dlopen Rust libdriver.so");

            let c_sym: Symbol<DriverFn> =
                c_lib.get(b"driver\0").expect("C .so must export `driver`");
            let rust_sym: Symbol<DriverFn> = rust_lib
                .get(b"driver\0")
                .expect("Rust .so must export `driver`");

            let c_driver = *c_sym;
            let rust_driver = *rust_sym;

            Libs {
                c_driver,
                rust_driver,
                _c_lib: c_lib,
                _rust_lib: rust_lib,
            }
        }
    }
}

/// Run `f` with file descriptor 1 redirected into a temporary file and return
/// every byte that was written.
///
/// Both shared objects call into the process-wide libc `stdout`, so flushing
/// before and after the redirect is enough to capture their output precisely.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    // Poisoning is irrelevant here: a panic inside a previous capture only
    // means that capture was abandoned, the lock itself still protects fd 1.
    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        // Rust's `Stdout` keeps its own buffer, independent of libc's; push any
        // pending harness output out before fd 1 is pointed elsewhere.
        let _ = std::io::Write::flush(&mut std::io::stdout());
        // Drain anything already buffered so it does not land in our capture.
        fflush(std::ptr::null_mut());

        let tmp = tmpfile();
        assert!(!tmp.is_null(), "tmpfile() failed");
        let tmp_fd = fileno(tmp);
        assert!(tmp_fd >= 0, "fileno() failed");

        let saved = dup(STDOUT_FD);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(tmp_fd, STDOUT_FD) >= 0, "dup2 onto stdout failed");

        f();

        // Flush the callee's buffered output while fd 1 still points at the file.
        fflush(std::ptr::null_mut());

        assert!(dup2(saved, STDOUT_FD) >= 0, "restoring stdout failed");
        close(saved);

        // Re-read the temporary file from the beginning.
        let mut file = {
            let dup_fd = dup(tmp_fd);
            assert!(dup_fd >= 0, "dup(tmp_fd) failed");
            use std::os::fd::FromRawFd;
            std::fs::File::from_raw_fd(dup_fd)
        };
        file.seek(SeekFrom::Start(0)).expect("seek temp file");
        let mut out = Vec::new();
        file.read_to_end(&mut out).expect("read temp file");
        drop(file);
        fclose(tmp);

        out
    }
}

/// Call `driver` in both libraries with `x` and return `(c_output, rust_output)`.
pub fn run_both(libs: &Libs, x: f32) -> (Vec<u8>, Vec<u8>) {
    let c_out = capture_stdout(|| unsafe { (libs.c_driver)(x) });
    let rust_out = capture_stdout(|| unsafe { (libs.rust_driver)(x) });
    (c_out, rust_out)
}

/// Call `driver` once per value inside a *single* stdout capture for each
/// library. This keeps large sweeps cheap (one temp file per library rather
/// than one per value) while still comparing the full byte stream.
pub fn run_batch(libs: &Libs, values: &[f32]) -> (Vec<u8>, Vec<u8>) {
    let c_out = capture_stdout(|| {
        for &x in values {
            unsafe { (libs.c_driver)(x) }
        }
    });
    let rust_out = capture_stdout(|| {
        for &x in values {
            unsafe { (libs.rust_driver)(x) }
        }
    });
    (c_out, rust_out)
}

/// Compare a batch and, on mismatch, report the first differing line together
/// with the input that produced it.
pub fn assert_same_batch(libs: &Libs, values: &[f32], label: &str) {
    let (c_out, rust_out) = run_batch(libs, values);
    if c_out == rust_out {
        return;
    }

    let c_lines: Vec<&[u8]> = c_out.split(|&b| b == b'\n').collect();
    let r_lines: Vec<&[u8]> = rust_out.split(|&b| b == b'\n').collect();
    for (i, (cl, rl)) in c_lines.iter().zip(r_lines.iter()).enumerate() {
        if cl != rl {
            let input = values.get(i).copied();
            panic!(
                "batch `{}` mismatch at line {} (input = {:?}, bits = {:?})\n  C   : {:?}\n  Rust: {:?}",
                label,
                i,
                input,
                input.map(|v| format!("{:#010x}", v.to_bits())),
                String::from_utf8_lossy(cl),
                String::from_utf8_lossy(rl),
            );
        }
    }
    panic!(
        "batch `{}` mismatch: C produced {} lines, Rust produced {} lines",
        label,
        c_lines.len(),
        r_lines.len()
    );
}

/// Assert that both implementations emit byte-identical output for `x`.
pub fn assert_same(libs: &Libs, x: f32, label: &str) {
    let (c_out, rust_out) = run_both(libs, x);
    assert_eq!(
        c_out,
        rust_out,
        "driver() output mismatch for {} (bits = {:#010x}, value = {:?})\n  C   : {:?}\n  Rust: {:?}",
        label,
        x.to_bits(),
        x,
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&rust_out)
    );
}

/// Tiny deterministic PRNG (SplitMix64) so the value set is reproducible.
pub struct SplitMix64(pub u64);

impl SplitMix64 {
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
}

/// Keep `c_char` referenced so the import list stays honest across platforms.
#[allow(dead_code)]
pub type CChar = c_char;
