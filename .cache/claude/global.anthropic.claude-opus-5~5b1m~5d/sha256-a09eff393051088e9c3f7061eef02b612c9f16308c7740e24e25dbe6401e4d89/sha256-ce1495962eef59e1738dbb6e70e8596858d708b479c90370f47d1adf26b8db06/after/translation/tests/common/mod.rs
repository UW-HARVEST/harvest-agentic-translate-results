//! Shared differential-test harness.
//!
//! Both libraries are loaded as **shared objects** via `libloading` and called
//! only through their exported `extern "C"` symbols — the Rust functions are
//! never called directly, so the `#[no_mangle]` export wrappers are under test
//! too.
//!
//! `driver` communicates purely by writing to libc `stdout`, so the harness
//! captures file descriptor 1 at the OS level (dup/dup2 onto a temp file).
//! That works for both `.so`s because both funnel through the same libc
//! `printf` in this process.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn lseek(fd: c_int, offset: i64, whence: c_int) -> i64;
    fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
}

const O_RDWR: c_int = 0o2;
const O_CREAT: c_int = 0o100;
const O_TRUNC: c_int = 0o1000;
const SEEK_SET: c_int = 0;

/// Signature of the exported symbol under test: `void driver(float)`.
pub type DriverFn = unsafe extern "C" fn(f32);

pub struct Libs {
    _c: libloading::Library,
    _rust: libloading::Library,
    pub c_driver: DriverFn,
    pub rust_driver: DriverFn,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    let p = manifest_dir().join("../c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {p:?}.\nBuild it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    p
}

/// Locate the Rust `cdylib`. Prefers the `release` artifact (the real shipping
/// artifact, built with `panic = "abort"`), falling back to `debug`.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "RUST_DRIVER_SO points at a missing file: {p:?}");
        return p;
    }
    // Honour a custom target dir if one is in use.
    let target = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| manifest_dir().join("target"));
    let release = target.join("release/libdriver.so");
    let debug = target.join("debug/libdriver.so");
    if release.exists() {
        return release;
    }
    assert!(
        debug.exists(),
        "Rust cdylib not found at {release:?} nor {debug:?}. Build it with `cargo build --release`."
    );
    debug
}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| unsafe {
        let c_path = c_so_path();
        let rust_path = rust_so_path();
        let c = libloading::Library::new(&c_path).expect("failed to dlopen the C .so");
        let rust = libloading::Library::new(&rust_path).expect("failed to dlopen the Rust .so");
        let c_driver: libloading::Symbol<DriverFn> =
            c.get(b"driver\0").expect("symbol `driver` missing from the C .so");
        let rust_driver: libloading::Symbol<DriverFn> = rust
            .get(b"driver\0")
            .expect("symbol `driver` missing from the Rust .so (missing #[no_mangle] export?)");
        let c_driver = *c_driver;
        let rust_driver = *rust_driver;
        Libs { _c: c, _rust: rust, c_driver, rust_driver, c_path, rust_path }
    })
}

/// The capture below redirects the process-wide file descriptor 1, so the test
/// harness's own progress output would land inside a capture if tests ran
/// concurrently. `.cargo/config.toml` pins `RUST_TEST_THREADS=1`; verify it.
fn require_single_threaded() {
    let t = std::env::var("RUST_TEST_THREADS").unwrap_or_default();
    assert_eq!(
        t, "1",
        "these differential tests redirect the process-wide stdout and must run \
         single-threaded. Run them via `cargo test` from the crate root (which picks up \
         RUST_TEST_THREADS=1 from .cargo/config.toml) or pass `-- --test-threads=1`."
    );
}

/// Serialises stdout redirection across tests in the same binary.
fn stdout_lock() -> MutexGuard<'static, ()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    match L.get_or_init(|| Mutex::new(())).lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

/// Runs `f` with file descriptor 1 redirected to a temporary file and returns
/// everything written to it (by either `.so`, via libc `printf`).
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    require_single_threaded();
    let _guard = stdout_lock();
    unsafe {
        // Flush anything the harness itself has buffered so it is not captured:
        // libc streams *and* Rust's own line-buffered stdout (libtest writes
        // unterminated progress lines like "test foo ... " that would otherwise
        // be flushed into our capture file).
        use std::io::Write;
        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();
        fflush(std::ptr::null_mut());

        let mut path = std::env::temp_dir();
        path.push(format!(
            "driver_diff_{}_{:p}.out",
            std::process::id(),
            &path as *const _
        ));
        let cpath = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).unwrap();

        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        let fd = open(cpath.as_ptr(), O_RDWR | O_CREAT | O_TRUNC, 0o600 as c_int);
        assert!(fd >= 0, "open({path:?}) failed");
        assert!(dup2(fd, 1) >= 0, "dup2 onto stdout failed");

        f();

        // Force the library's buffered output out through the (redirected) fd 1.
        fflush(std::ptr::null_mut());

        assert!(dup2(saved, 1) >= 0, "restoring stdout failed");
        close(saved);

        // Read the whole capture file back.
        assert!(lseek(fd, 0, SEEK_SET) == 0, "lseek failed");
        let mut out = Vec::new();
        let mut buf = [0u8; 65536];
        loop {
            let n = read(fd, buf.as_mut_ptr() as *mut c_void, buf.len());
            assert!(n >= 0, "read from capture file failed");
            if n == 0 {
                break;
            }
            out.extend_from_slice(&buf[..n as usize]);
        }
        close(fd);
        let _ = std::fs::remove_file(&path);
        out
    }
}

/// Every `driver` call must emit exactly `%02x`×4 + `\n`. Anything else in a
/// capture means foreign bytes leaked into the redirected fd (a harness bug, not
/// a translation bug) — fail loudly instead of reporting a bogus mismatch.
fn assert_capture_uncontaminated(label: &str, who: &str, out: &[u8], calls: usize) {
    let contaminated = out.len() != calls * 9
        || out.chunks(9).any(|rec| {
            rec.len() != 9
                || rec[8] != b'\n'
                || !rec[..8].iter().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        });
    if contaminated {
        // Find the first offending record for the message.
        let bad = out
            .chunks(9)
            .find(|rec| {
                rec.len() != 9
                    || rec[8] != b'\n'
                    || !rec[..8].iter().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
            })
            .map(|r| String::from_utf8_lossy(r).to_string())
            .unwrap_or_default();
        panic!(
            "[{label}] {who} capture is malformed: expected {} bytes ({calls} calls x 9), got {}. \
             First bad record: {bad:?}. If this looks like harness text, the tests are running \
             multi-threaded (see RUST_TEST_THREADS).",
            calls * 9,
            out.len()
        );
    }
}

/// Calls the exported `driver` of **both** libraries over the same input list
/// and returns `(c_output, rust_output)`.
pub fn run_both(values: &[f32]) -> (Vec<u8>, Vec<u8>) {
    let l = libs();
    let c = capture_stdout(|| {
        for &v in values {
            unsafe { (l.c_driver)(v) }
        }
    });
    let r = capture_stdout(|| {
        for &v in values {
            unsafe { (l.rust_driver)(v) }
        }
    });
    (c, r)
}

fn render(bits: &[u32], out: &[u8]) -> String {
    let text = String::from_utf8_lossy(out);
    let lines: Vec<&str> = text.lines().take(8).collect();
    format!("first lines {lines:?} (len {}), inputs {:08x?}", out.len(), &bits[..bits.len().min(8)])
}

/// Asserts byte-identical stdout from both `.so`s for `values`.
pub fn assert_same(label: &str, values: &[f32]) {
    let (c, r) = run_both(values);
    assert_capture_uncontaminated(label, "C", &c, values.len());
    assert_capture_uncontaminated(label, "Rust", &r, values.len());
    if c != r {
        let bits: Vec<u32> = values.iter().map(|v| v.to_bits()).collect();
        // Pinpoint the first differing line for a useful message.
        let cl: Vec<&[u8]> = c.split(|&b| b == b'\n').collect();
        let rl: Vec<&[u8]> = r.split(|&b| b == b'\n').collect();
        let mut detail = String::new();
        for i in 0..cl.len().max(rl.len()) {
            let a = cl.get(i).copied().unwrap_or(b"<missing>");
            let b = rl.get(i).copied().unwrap_or(b"<missing>");
            if a != b {
                detail = format!(
                    "first divergence at line {i}: C={:?} RUST={:?} input=0x{:08x}",
                    String::from_utf8_lossy(a),
                    String::from_utf8_lossy(b),
                    bits.get(i).copied().unwrap_or(0)
                );
                break;
            }
        }
        panic!(
            "[{label}] C and Rust stdout differ.\n{detail}\nC   : {}\nRUST: {}",
            render(&bits, &c),
            render(&bits, &r)
        );
    }
    // Sanity: the output must actually be the expected shape (9 bytes/call),
    // otherwise "equal" could mean "both captured nothing".
    assert_eq!(
        c.len(),
        values.len() * 9,
        "[{label}] expected 9 bytes per call ({} calls), got {} bytes",
        values.len(),
        c.len()
    );
}

/// Deterministic PRNG (xorshift64*) so every randomized row is reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Random value in `0..n` (n > 0).
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
}

pub const SEED: u64 = 0x5EED_1234;

/// Number of randomized samples per property-style row.
pub const N: usize = 4096;
