//! Shared differential-test harness.
//!
//! Both implementations are loaded as shared objects with `libloading` and
//! driven **only** through their exported C-ABI symbols, exactly as an external
//! consumer would — the Rust functions are never called directly, so the
//! `#[no_mangle]` export wrappers are covered too.
//!
//! * C side:    `target/cdiff/libcdriver.so`, compiled on demand from the
//!              unmodified `c_src/src/main.c` (`cc -shared -fPIC`).
//! * Rust side: `target/<profile>/libdriver.so` (the crate's `cdylib`).
//!
//! Because the library's only observable effect is bytes written to fd 1, the
//! harness redirects fd 1 to a temporary file around each call, flushes both
//! the C stdio buffer (`fflush(NULL)`) and the Rust side, and compares the
//! captured bytes exactly.

#![allow(dead_code)]

use std::ffi::CString;
use std::io::Write;
use std::os::raw::{c_char, c_int};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, Once, OnceLock};

// ---------------------------------------------------------------------------
// Function signatures of the exported C ABI
// ---------------------------------------------------------------------------

pub type PrintLineFn = unsafe extern "C" fn(*const c_char);
pub type VoidFn = unsafe extern "C" fn();
pub type MainFn = unsafe extern "C" fn(c_int, *mut *mut c_char) -> c_int;

/// One loaded implementation (C or Rust), used only via `dlsym`.
pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    lib: libloading::Library,
}

impl Impl {
    fn load(name: &'static str, path: PathBuf) -> Impl {
        let lib = unsafe { libloading::Library::new(&path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {} ({}): {e}", name, path.display()));
        Impl { name, path, lib }
    }

    /// `void printLine(const char *line)`
    pub fn print_line(&self, line: *const c_char) {
        unsafe {
            let f = self
                .lib
                .get::<PrintLineFn>(b"printLine\0")
                .unwrap_or_else(|e| panic!("{}: printLine not exported: {e}", self.name));
            f(line)
        }
    }

    /// `void bad(void)`
    pub fn bad(&self) {
        unsafe {
            let f = self
                .lib
                .get::<VoidFn>(b"bad\0")
                .unwrap_or_else(|e| panic!("{}: bad not exported: {e}", self.name));
            f()
        }
    }

    /// `void good(void)`
    pub fn good(&self) {
        unsafe {
            let f = self
                .lib
                .get::<VoidFn>(b"good\0")
                .unwrap_or_else(|e| panic!("{}: good not exported: {e}", self.name));
            f()
        }
    }

    /// `int main(int argc, char *argv[])`
    pub fn main(&self, argc: c_int, argv: *mut *mut c_char) -> c_int {
        unsafe {
            let f = self
                .lib
                .get::<MainFn>(b"main\0")
                .unwrap_or_else(|e| panic!("{}: main not exported: {e}", self.name));
            f(argc, argv)
        }
    }

    /// Convenience: `printLine` with a NUL-terminated copy of `bytes`.
    ///
    /// `bytes` must not contain an interior NUL (a C string cannot carry one).
    pub fn print_bytes(&self, bytes: &[u8]) {
        let mut buf = Vec::with_capacity(bytes.len() + 1);
        buf.extend_from_slice(bytes);
        buf.push(0);
        self.print_line(buf.as_ptr() as *const c_char);
    }
}

// ---------------------------------------------------------------------------
// Locating / building the two shared objects
// ---------------------------------------------------------------------------

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Path to a **freshly built** Rust `cdylib`.
///
/// `cargo test` compiles the library only as an `rlib` for the test targets: it
/// does *not* refresh `target/<profile>/libdriver.so`, so loading that artifact
/// would silently test whatever code happened to be built last. The `.so` is
/// therefore rebuilt here, into a private target directory so that the nested
/// `cargo` never contends with the outer `cargo test`'s build lock.
///
/// Extra flags (e.g. `--no-default-features --features x`) can be forwarded
/// through the `CDIFF_CARGO_ARGS` environment variable so that the differential
/// tests really exercise the same feature combination as the test run.
pub fn rust_so_path() -> PathBuf {
    static SO: OnceLock<PathBuf> = OnceLock::new();
    SO.get_or_init(|| {
        let target_dir = manifest_dir().join("target/cdiff/rustlib");
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
        let mut cmd = Command::new(cargo);
        cmd.current_dir(manifest_dir())
            .args(["build", "--offline", "--lib", "--target-dir"])
            .arg(&target_dir);
        if let Ok(extra) = std::env::var("CDIFF_CARGO_ARGS") {
            for arg in extra.split_whitespace() {
                cmd.arg(arg);
            }
        }
        let out = cmd.output().expect("failed to run cargo to build the Rust cdylib");
        assert!(
            out.status.success(),
            "cargo build --lib failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let p = target_dir.join("debug/libdriver.so");
        assert!(
            p.exists(),
            "Rust cdylib not found at {} after building it",
            p.display()
        );
        p
    })
    .clone()
}

fn cdiff_dir() -> PathBuf {
    let d = manifest_dir().join("target/cdiff");
    std::fs::create_dir_all(&d).expect("create target/cdiff");
    d
}

pub fn c_source() -> PathBuf {
    manifest_dir().join("c_src/src/main.c")
}

/// Compiles the unmodified C translation unit as a shared object.
/// Nothing inside `c_src/` is written to; the artifact lands in `target/cdiff/`.
pub fn c_so_path() -> PathBuf {
    static ONCE: Once = Once::new();
    let out = cdiff_dir().join("libcdriver.so");
    ONCE.call_once(|| {
        let tmp = cdiff_dir().join(format!("libcdriver.{}.tmp.so", std::process::id()));
        let status = Command::new("cc")
            .args(["-shared", "-fPIC", "-o"])
            .arg(&tmp)
            .arg(c_source())
            .status()
            .expect("failed to run cc to build the C shared object");
        assert!(status.success(), "cc failed to build the C shared object");
        std::fs::rename(&tmp, &out).expect("rename C shared object into place");
    });
    assert!(out.exists(), "C shared object missing at {}", out.display());
    out
}

/// Compiles the unmodified C translation unit as an executable (same result as
/// `c_src/CMakeLists.txt`'s `add_executable`), for the end-to-end comparison.
pub fn c_exe_path() -> PathBuf {
    static ONCE: Once = Once::new();
    // Prefer the CMake-produced binary when it is present.
    let cmake_exe = manifest_dir().join("c_src/build/driver");
    if cmake_exe.exists() {
        return cmake_exe;
    }
    let out = cdiff_dir().join("cdriver");
    ONCE.call_once(|| {
        let tmp = cdiff_dir().join(format!("cdriver.{}.tmp", std::process::id()));
        let status = Command::new("cc")
            .args(["-fPIC", "-pie", "-o"])
            .arg(&tmp)
            .arg(c_source())
            .status()
            .expect("failed to run cc to build the C executable");
        assert!(status.success(), "cc failed to build the C executable");
        std::fs::rename(&tmp, &out).expect("rename C executable into place");
    });
    out
}

/// The two loaded implementations: `(c, rust)`.
pub fn impls() -> (&'static Impl, &'static Impl) {
    static C: OnceLock<Impl> = OnceLock::new();
    static R: OnceLock<Impl> = OnceLock::new();
    let c = C.get_or_init(|| Impl::load("C", c_so_path()));
    let r = R.get_or_init(|| Impl::load("Rust", rust_so_path()));
    (c, r)
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// Serialises fd-1 redirection: tests otherwise run in parallel threads.
fn capture_lock() -> &'static Mutex<()> {
    static LOCK: Mutex<()> = Mutex::new(());
    &LOCK
}

fn next_id() -> u64 {
    static N: AtomicU64 = AtomicU64::new(0);
    N.fetch_add(1, Ordering::SeqCst)
}

/// Runs `f` with fd 1 redirected to a temporary file and returns every byte
/// that was written to it.
///
/// Two writers other than the library under test could pollute the capture:
/// another test thread (excluded by `capture_lock`) and libtest's own progress
/// output, which the harness writes through the process-global `io::stdout()`.
/// Holding the `StdoutLock` for the whole redirect window blocks the latter
/// until fd 1 has been restored, so the capture stays parallelism-proof without
/// relying on `--test-threads=1`.
pub fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());

    let path = std::env::temp_dir().join(format!(
        "cdiff-capture-{}-{}.bin",
        std::process::id(),
        next_id()
    ));
    let file = std::fs::File::create(&path).expect("create capture file");

    let stdout = std::io::stdout();
    let mut stdout_lock = stdout.lock();

    let bytes = unsafe {
        // Nothing of ours (nor of libtest's) may still be sitting in a buffer
        // when fd 1 changes.
        let _ = stdout_lock.flush();
        libc::fflush(std::ptr::null_mut());

        let saved = libc::dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(libc::dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");

        f();

        // The C `.so` writes through the (fully buffered, since fd 1 is a file)
        // stdio stream; flush it before the descriptor is restored. The Rust
        // `.so` writes unbuffered, so this is a no-op for it.
        libc::fflush(std::ptr::null_mut());
        let _ = stdout_lock.flush();

        assert!(libc::dup2(saved, 1) >= 0, "dup2 restore failed");
        libc::close(saved);

        drop(file);
        std::fs::read(&path).expect("read capture file")
    };
    drop(stdout_lock);
    let _ = std::fs::remove_file(&path);
    bytes
}

// ---------------------------------------------------------------------------
// Differential assertions
// ---------------------------------------------------------------------------

fn render(bytes: &[u8]) -> String {
    const MAX: usize = 320;
    let shown = &bytes[..bytes.len().min(MAX)];
    let mut s = String::new();
    for &b in shown {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            b'\\' => s.push_str("\\\\"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    if bytes.len() > MAX {
        s.push_str(&format!("...(+{} bytes)", bytes.len() - MAX));
    }
    format!("len={} \"{}\"", bytes.len(), s)
}

/// Runs `body` against both implementations and asserts the captured stdout
/// bytes are identical.
pub fn assert_same_output<F: Fn(&Impl)>(desc: &str, body: F) {
    let (c, r) = impls();
    let out_c = capture(|| body(c));
    let out_r = capture(|| body(r));
    if out_c != out_r {
        panic!(
            "output mismatch [{desc}]\n  C   : {}\n  Rust: {}",
            render(&out_c),
            render(&out_r)
        );
    }
}

/// Same as [`assert_same_output`] but also compares the value(s) returned.
pub fn assert_same_output_and_ret<T, F>(desc: &str, body: F)
where
    T: PartialEq + std::fmt::Debug,
    F: Fn(&Impl) -> T,
{
    let (c, r) = impls();
    let mut ret_c = None;
    let out_c = capture(|| ret_c = Some(body(c)));
    let mut ret_r = None;
    let out_r = capture(|| ret_r = Some(body(r)));
    if out_c != out_r {
        panic!(
            "output mismatch [{desc}]\n  C   : {}\n  Rust: {}",
            render(&out_c),
            render(&out_r)
        );
    }
    let (ret_c, ret_r) = (ret_c.unwrap(), ret_r.unwrap());
    if ret_c != ret_r {
        panic!("return-value mismatch [{desc}]\n  C   : {ret_c:?}\n  Rust: {ret_r:?}");
    }
}

/// Asserts both implementations produce exactly `expected` bytes (used to pin
/// the absolute reference output on top of the C-vs-Rust comparison).
pub fn assert_output_is<F: Fn(&Impl)>(desc: &str, expected: &[u8], body: F) {
    assert_same_output(desc, &body);
    let (c, _) = impls();
    let out_c = capture(|| body(c));
    assert_eq!(
        out_c,
        expected,
        "C reference output changed [{desc}]\n  got     : {}\n  expected: {}",
        render(&out_c),
        render(expected)
    );
}

// ---------------------------------------------------------------------------
// Call-sequence driver (exercises the composed pipeline, not just wrappers)
// ---------------------------------------------------------------------------

/// One call to the public API.
#[derive(Clone, Debug)]
pub enum Op {
    /// `printLine(<bytes>)` with a NUL-terminated copy of the payload.
    PrintLine(Vec<u8>),
    /// `printLine(NULL)`
    PrintNull,
    /// `bad()`
    Bad,
    /// `good()`
    Good,
    /// `main(argc, argv)` with `argv` built from the given arguments.
    Main(c_int, Vec<Vec<u8>>),
    /// `main(argc, NULL)`
    MainNullArgv(c_int),
}

/// Runs a whole sequence against one implementation, returning every value
/// returned by `main`.
pub fn run_ops(im: &Impl, ops: &[Op]) -> Vec<c_int> {
    let mut rets = Vec::new();
    for op in ops {
        match op {
            Op::PrintLine(bytes) => im.print_bytes(bytes),
            Op::PrintNull => im.print_line(std::ptr::null()),
            Op::Bad => im.bad(),
            Op::Good => im.good(),
            Op::Main(argc, args) => {
                let owned: Vec<CString> = args
                    .iter()
                    .map(|a| CString::new(a.clone()).expect("argv element without interior NUL"))
                    .collect();
                let mut ptrs: Vec<*mut c_char> =
                    owned.iter().map(|s| s.as_ptr() as *mut c_char).collect();
                ptrs.push(std::ptr::null_mut());
                rets.push(im.main(*argc, ptrs.as_mut_ptr()));
            }
            Op::MainNullArgv(argc) => rets.push(im.main(*argc, std::ptr::null_mut())),
        }
    }
    rets
}

/// Differential check for a whole call sequence: stdout bytes and every `main`
/// return value must match.
pub fn assert_same_sequence(desc: &str, ops: &[Op]) {
    assert_same_output_and_ret(desc, |im| run_ops(im, ops));
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed -> reproducible property-style testing)
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x2026_0818;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }

    /// splitmix64
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `0..n` (`n > 0`).
    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0);
        self.next_u64() % n
    }

    pub fn range(&mut self, lo: usize, hi_inclusive: usize) -> usize {
        lo + self.below((hi_inclusive - lo + 1) as u64) as usize
    }

    /// A byte in `0x01..=0xFF` (never NUL: it would terminate the C string).
    pub fn nonzero_byte(&mut self) -> u8 {
        (self.below(255) + 1) as u8
    }

    /// Printable-ASCII byte (`0x20..=0x7E`).
    pub fn printable_byte(&mut self) -> u8 {
        (0x20 + self.below(0x7f - 0x20)) as u8
    }

    pub fn bytes_nonzero(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.nonzero_byte()).collect()
    }

    pub fn bytes_printable(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.printable_byte()).collect()
    }
}

/// Fails the test run early with a clear message if either artifact is absent.
pub fn require_artifacts() {
    let c = c_so_path();
    let r = rust_so_path();
    assert!(Path::new(&c).exists() && Path::new(&r).exists());
}
