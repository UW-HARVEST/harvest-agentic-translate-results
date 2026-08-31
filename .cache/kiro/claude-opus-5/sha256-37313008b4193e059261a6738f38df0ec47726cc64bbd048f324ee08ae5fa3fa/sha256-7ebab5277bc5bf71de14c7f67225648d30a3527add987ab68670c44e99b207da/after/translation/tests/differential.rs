//! Differential test: loads BOTH the C `libdriver.so` and the Rust
//! `libdriver.so` through `libloading` and compares their stdout byte-for-byte.
//!
//! The Rust implementation is never called directly — only through the
//! `#[no_mangle]` exported symbol of the built cdylib, exactly as an external
//! C caller would.

use libloading::{Library, Symbol};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

// ---------------------------------------------------------------------------
// libc bits needed to capture the C stdio stream (declared locally so the test
// does not need the `libc` crate).
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn dup(oldfd: i32) -> i32;
    fn dup2(oldfd: i32, newfd: i32) -> i32;
    fn close(fd: i32) -> i32;
    /// `fflush(NULL)` flushes *all* open output streams, including the `stdout`
    /// `FILE*` that both shared objects write through.
    fn fflush(stream: *mut std::ffi::c_void) -> i32;
}

/// stdout redirection is process-global, so serialize every capture.
static CAPTURE_LOCK: Mutex<()> = Mutex::new(());

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn c_so_path() -> PathBuf {
    workspace_root().join("c_src/build/libdriver.so")
}

fn rust_so_path() -> PathBuf {
    // The integration test binary lives in target/<profile>/deps/, so the
    // cdylib is two levels up.
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|deps| deps.parent())
        .expect("target/<profile>")
        .to_path_buf();
    let direct = profile_dir.join("libdriver.so");
    if direct.exists() {
        return direct;
    }
    // Fallbacks in case the harness ran from an unexpected location.
    for p in [
        workspace_root().join("translation/target/release/libdriver.so"),
        workspace_root().join("translation/target/debug/libdriver.so"),
    ] {
        if p.exists() {
            return p;
        }
    }
    direct
}

/// Runs `f` with fd 1 redirected to a temporary file and returns the raw bytes
/// that were written.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = CAPTURE_LOCK.lock().unwrap();

    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let path =
        std::env::temp_dir().join(format!("driver_capture_{}_{n}.bin", std::process::id()));

    // Flush anything already pending so it does not land in our capture file.
    unsafe {
        fflush(std::ptr::null_mut());
    }

    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");

    {
        let file = std::fs::File::create(&path).expect("create capture file");
        assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 failed");
    }

    f();

    // Both shared objects write through C `printf`; the redirected fd is a
    // regular file and therefore fully buffered, so an explicit flush is
    // required before restoring fd 1.
    unsafe {
        fflush(std::ptr::null_mut());
    }

    assert!(unsafe { dup2(saved, 1) } >= 0, "restore dup2 failed");
    unsafe {
        close(saved);
    }

    let data = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);
    data
}

type DriverFn = unsafe extern "C" fn(std::ffi::c_int);

struct Impls {
    _c_lib: Library,
    _rust_lib: Library,
    c_driver: DriverFn,
    rust_driver: DriverFn,
}

impl Impls {
    fn load() -> Impls {
        let c_path = c_so_path();
        let rust_path = rust_so_path();
        assert!(
            c_path.exists(),
            "C shared library not built at {c_path:?}. Build it with:\n  cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
        );
        assert!(rust_path.exists(), "Rust cdylib not found at {rust_path:?}");

        unsafe {
            let c_lib = Library::new(&c_path).expect("load C .so");
            let rust_lib = Library::new(&rust_path).expect("load Rust .so");
            let c_driver: Symbol<DriverFn> =
                c_lib.get(b"driver\0").expect("C .so exports `driver`");
            let rust_driver: Symbol<DriverFn> = rust_lib
                .get(b"driver\0")
                .expect("Rust .so exports `driver`");
            let c_driver = *c_driver;
            let rust_driver = *rust_driver;
            Impls {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c_driver,
                rust_driver,
            }
        }
    }

    fn compare(&self, x: i32) {
        let c_out = capture_stdout(|| unsafe { (self.c_driver)(x) });
        let rust_out = capture_stdout(|| unsafe { (self.rust_driver)(x) });
        assert_eq!(
            c_out,
            rust_out,
            "driver({x}) mismatch:\n  C   : {:?}\n  Rust: {:?}",
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&rust_out)
        );
        // Sanity: output must be non-empty (guards against a broken capture
        // silently making every comparison trivially pass).
        assert!(!c_out.is_empty(), "captured no output for driver({x})");
        // Independently derived expectation: the native-endian bytes of `x`
        // printed as lowercase %02x, then a newline. This proves the capture
        // machinery is really observing the library output.
        let expected: String = x
            .to_ne_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
            + "\n";
        assert_eq!(
            String::from_utf8_lossy(&c_out),
            expected,
            "capture sanity check failed for driver({x})"
        );
    }
}

/// The one and only public API function: `void driver(int x)`.
#[test]
fn driver_matches_c() {
    let impls = Impls::load();

    // Hand-picked edge cases.
    let mut inputs: Vec<i32> = vec![
        0,
        1,
        -1,
        2,
        -2,
        7,
        -7,
        0x7f,
        0x80,
        0xff,
        0x100,
        0x1234,
        0x12345678,
        -0x12345678,
        0x0000_00ff,
        0x0000_ff00,
        0x00ff_0000,
        -16_777_216, // 0xff000000
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        1 << 8,
        1 << 16,
        1 << 24,
        1 << 30,
        -(1 << 30),
        123_456_789,
        -123_456_789,
        1_000_000_000,
        -1_000_000_000,
    ];

    // Every single-bit pattern and its complement.
    for b in 0..32 {
        let v = 1i32 << b;
        inputs.push(v);
        inputs.push(!v);
        inputs.push(v.wrapping_neg());
    }

    // Deterministic pseudo-random sweep (xorshift32) for broad coverage.
    let mut state: u32 = 0x9E37_79B9;
    for _ in 0..512 {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        inputs.push(state as i32);
    }

    for x in inputs {
        impls.compare(x);
    }
}

/// Step 8: every symbol the C .so exports must also be exported by the Rust .so.
#[test]
fn exported_symbols_match() {
    fn exported(path: &Path) -> Vec<String> {
        let out = std::process::Command::new("nm")
            .args(["-D", "--defined-only", "--format=posix"])
            .arg(path)
            .output()
            .expect("run nm");
        assert!(
            out.status.success(),
            "nm failed on {path:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let mut syms: Vec<String> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.split_whitespace().next())
            .map(|s| s.to_string())
            .collect();
        syms.sort();
        syms.dedup();
        syms
    }

    let c_syms = exported(&c_so_path());
    let rust_syms = exported(&rust_so_path());

    assert!(
        c_syms.contains(&"driver".to_string()),
        "sanity: C .so should export `driver`, got {c_syms:?}"
    );

    let missing: Vec<&String> = c_syms.iter().filter(|s| !rust_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing symbols exported by the C .so: {missing:?}\n  C:    {c_syms:?}\n  Rust: {rust_syms:?}"
    );
}
