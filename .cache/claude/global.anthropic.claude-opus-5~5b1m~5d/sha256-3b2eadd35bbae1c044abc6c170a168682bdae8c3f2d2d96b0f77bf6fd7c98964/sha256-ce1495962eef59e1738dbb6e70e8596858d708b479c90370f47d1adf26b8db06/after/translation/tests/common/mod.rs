//! Shared differential-test harness.
//!
//! Both implementations are loaded as shared objects through `libloading` and
//! invoked only through their exported `driver` symbol — the Rust crate is
//! never linked or called directly, so the `#[no_mangle] extern "C"` wrapper is
//! part of what gets tested.
//!
//! `driver` communicates exclusively through `stdout` (it is `void` and takes no
//! pointers), so the harness captures the raw bytes written to file descriptor 1
//! around each call and compares them byte-for-byte.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

pub type DriverFn = unsafe extern "C" fn(c_int, c_int);
/// Same symbol, deliberately mis-declared with 64-bit arguments so a test can
/// put garbage in the upper halves of the argument registers (see ERRORS.md E8).
pub type DriverFnU64 = unsafe extern "C" fn(u64, u64);

pub struct Impls {
    pub c: DriverFn,
    pub rust: DriverFn,
    pub c_u64: DriverFnU64,
    pub rust_u64: DriverFnU64,
}

// ---------------------------------------------------------------------------
// locating the two shared objects
// ---------------------------------------------------------------------------

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

/// Path to the Rust `cdylib` under test.
///
/// `cargo test` does **not** emit the `cdylib` artifact (a `cdylib` is not
/// linkable by an integration test), so simply picking up
/// `target/<profile>/libdriver.so` silently loads whatever `.so` was left behind
/// by an earlier `cargo build` — a stale artifact that makes every differential
/// test vacuously pass. This function therefore *builds* the `cdylib` itself,
/// into a dedicated target directory so it cannot contend with the parent
/// cargo's build lock, and returns the freshly produced artifact.
///
/// Override with `DRIVER_RUST_SO=/path/to/libdriver.so` to test a prebuilt
/// object (used by the feature-combination sweep script).
pub fn rust_so_path() -> PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
            let p = PathBuf::from(p);
            assert!(p.exists(), "DRIVER_RUST_SO points at a missing file: {p:?}");
            return p;
        }

        let target_dir = manifest_dir().join("target").join("ffi-so");
        let features = std::env::var("DRIVER_TEST_FEATURES").unwrap_or_default();
        let no_default = std::env::var("DRIVER_TEST_NO_DEFAULT_FEATURES").is_ok();

        let mut cmd = std::process::Command::new(env!("CARGO"));
        cmd.current_dir(manifest_dir())
            .arg("build")
            .arg("--release")
            .arg("--offline")
            .arg("--target-dir")
            .arg(&target_dir);
        if no_default {
            cmd.arg("--no-default-features");
        }
        if !features.is_empty() {
            cmd.arg("--features").arg(&features);
        }
        // Do not inherit the parent cargo's per-invocation environment.
        for k in ["CARGO_MAKEFLAGS", "RUSTC_WORKSPACE_WRAPPER", "CARGO_BUILD_TARGET_DIR"] {
            cmd.env_remove(k);
        }
        let out = cmd.output().expect("spawn cargo build for the cdylib");
        assert!(
            out.status.success(),
            "failed to build the Rust cdylib:\n{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );

        let p = target_dir.join("release").join("libdriver.so");
        assert!(p.exists(), "cargo build succeeded but {p:?} is missing");
        assert_fresh(&p);
        p
    })
    .clone()
}

/// Guard against ever testing an artifact older than the source it came from.
fn assert_fresh(so: &Path) {
    let src = manifest_dir().join("src/lib.rs");
    let mt = |p: &Path| std::fs::metadata(p).and_then(|m| m.modified()).ok();
    if let (Some(a), Some(b)) = (mt(so), mt(&src)) {
        assert!(
            a >= b,
            "STALE ARTIFACT: {so:?} is older than {src:?}; the differential tests would be \
             comparing the C library against an out-of-date Rust build"
        );
    }
}

pub fn impls() -> &'static Impls {
    static IMPLS: OnceLock<Impls> = OnceLock::new();
    IMPLS.get_or_init(|| unsafe {
        let clib = libloading::Library::new(c_so_path()).expect("dlopen C libdriver.so");
        let rlib = libloading::Library::new(rust_so_path()).expect("dlopen Rust libdriver.so");

        let c: DriverFn = *clib.get::<DriverFn>(b"driver\0").expect("C driver symbol");
        let rust: DriverFn = *rlib.get::<DriverFn>(b"driver\0").expect("Rust driver symbol");
        let c_u64: DriverFnU64 = *clib.get::<DriverFnU64>(b"driver\0").unwrap();
        let rust_u64: DriverFnU64 = *rlib.get::<DriverFnU64>(b"driver\0").unwrap();

        // Keep both objects resident for the lifetime of the process so the
        // extracted function pointers stay valid.
        std::mem::forget(clib);
        std::mem::forget(rlib);

        Impls { c, rust, c_u64, rust_u64 }
    })
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

fn capture_lock() -> &'static Mutex<()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
}

/// `stdio` buffering mode to force on `stdout` for the duration of a capture.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BufMode {
    /// Leave whatever mode glibc already chose.
    Inherit,
    Full,
    Line,
    None,
}

extern "C" {
    /// glibc's `stdout` is a macro over this global.
    static stdout: *mut c_void;
}

unsafe fn set_buf_mode(mode: BufMode) {
    let m = match mode {
        BufMode::Inherit => return,
        BufMode::Full => libc::_IOFBF,
        BufMode::Line => libc::_IOLBF,
        BufMode::None => libc::_IONBF,
    };
    libc::fflush(stdout as *mut libc::FILE);
    // Always pass a NULL buffer so glibc owns the allocation (never a buffer
    // that could outlive this scope).
    libc::setvbuf(stdout as *mut libc::FILE, std::ptr::null_mut(), m, 0);
}

/// Flush only the C streams. Safe to call *inside* a capture window.
unsafe fn flush_c() {
    libc::fflush(std::ptr::null_mut());
}

/// Flush everything. Must only be called *outside* a capture window, since it
/// also drains Rust-side buffers that belong to the real stdout.
unsafe fn flush_all() {
    // Flush every C stream (both .so's share this process's `stdout` FILE) ...
    flush_c();
    // ... and Rust's own `LineWriter`-buffered stdout, which otherwise holds
    // libtest's partial "test <name> ... " progress line and would flush it
    // *into* our capture file once fd 1 is redirected.
    flush_rust_stdout();
}

/// Push out anything buffered in `std::io::stdout()` (and in libtest's capture
/// machinery) so it cannot leak into a capture window.
pub fn flush_rust_stdout() {
    use std::io::Write;
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
}

/// Panics unless the test harness is running single-threaded, because the
/// capture windows below are process-global (fd 1 is shared by all threads).
/// `.cargo/config.toml` sets `RUST_TEST_THREADS=1` so `cargo test` is correct by
/// default; this catches an override.
fn assert_single_threaded() {
    static CHECKED: OnceLock<()> = OnceLock::new();
    CHECKED.get_or_init(|| {
        if let Ok(v) = std::env::var("RUST_TEST_THREADS") {
            assert_eq!(
                v.trim(),
                "1",
                "these differential tests redirect the process-wide fd 1 and must run \
                 single-threaded; use `cargo test` (which picks up RUST_TEST_THREADS=1 from \
                 .cargo/config.toml) or pass `-- --test-threads=1`"
            );
        } else {
            panic!(
                "RUST_TEST_THREADS is not set; these tests redirect the process-wide fd 1 and \
                 must run single-threaded. Run `cargo test` from the crate root (\
                 .cargo/config.toml sets RUST_TEST_THREADS=1) or set it explicitly."
            );
        }
    });
}

/// Run `f` with fd 1 pointed at a fresh temporary file; return everything the
/// callee wrote (after flushing the shared C `stdout` buffer).
pub fn capture_with(mode: BufMode, f: impl FnOnce()) -> Vec<u8> {
    assert_single_threaded();
    let _g = capture_lock().lock().unwrap();
    unsafe {
        let mut tmp = tempfile();
        flush_all();
        let saved = libc::dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(libc::dup2(as_fd(&tmp), 1) >= 0, "dup2 failed");
        set_buf_mode(mode);

        f();

        flush_c();
        set_buf_mode(BufMode::Full); // leave stdout in a sane state
        assert!(libc::dup2(saved, 1) >= 0, "dup2 restore failed");
        libc::close(saved);

        tmp.seek(SeekFrom::Start(0)).unwrap();
        let mut out = Vec::new();
        tmp.read_to_end(&mut out).unwrap();
        out
    }
}

pub fn capture(f: impl FnOnce()) -> Vec<u8> {
    capture_with(BufMode::Inherit, f)
}

/// Same as [`capture`] but fd 1 is a **pipe** (non-seekable). Keep the produced
/// volume well under the 64 KiB pipe capacity so the writer cannot block.
pub fn capture_via_pipe(f: impl FnOnce()) -> Vec<u8> {
    assert_single_threaded();
    let _g = capture_lock().lock().unwrap();
    unsafe {
        let mut fds = [0 as c_int; 2];
        assert_eq!(libc::pipe(fds.as_mut_ptr()), 0, "pipe() failed");
        let (rd, wr) = (fds[0], fds[1]);

        flush_all();
        let saved = libc::dup(1);
        assert!(saved >= 0);
        assert!(libc::dup2(wr, 1) >= 0);

        f();

        flush_c();
        assert!(libc::dup2(saved, 1) >= 0);
        libc::close(saved);
        libc::close(wr); // EOF for the reader

        let mut out = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            let n = libc::read(rd, buf.as_mut_ptr() as *mut c_void, buf.len());
            if n <= 0 {
                break;
            }
            out.extend_from_slice(&buf[..n as usize]);
        }
        libc::close(rd);
        out
    }
}

/// Run `f` with fd 1 pointed at `/dev/null` (a character device). Nothing can be
/// read back; the point is that the call completes normally.
pub fn run_to_dev_null(f: impl FnOnce()) {
    assert_single_threaded();
    let _g = capture_lock().lock().unwrap();
    unsafe {
        let devnull = libc::open(b"/dev/null\0".as_ptr() as *const c_char, libc::O_WRONLY);
        assert!(devnull >= 0);
        flush_all();
        let saved = libc::dup(1);
        assert!(libc::dup2(devnull, 1) >= 0);

        f();

        flush_c();
        libc::dup2(saved, 1);
        libc::close(saved);
        libc::close(devnull);
    }
}

/// Run `f` with fd 1 pointed at `/dev/write-always-fails`; returns
/// `(bytes that made it through == none, ferror(stdout) was set)`.
pub fn run_to_dev_full(f: impl FnOnce()) -> bool {
    assert_single_threaded();
    let _g = capture_lock().lock().unwrap();
    unsafe {
        let devfull = libc::open(b"/dev/full\0".as_ptr() as *const c_char, libc::O_WRONLY);
        assert!(devfull >= 0, "/dev/full not available");
        flush_all();
        let saved = libc::dup(1);
        assert!(libc::dup2(devfull, 1) >= 0);
        libc::clearerr(stdout as *mut libc::FILE);

        f();

        libc::fflush(stdout as *mut libc::FILE);
        let err = libc::ferror(stdout as *mut libc::FILE) != 0;
        libc::clearerr(stdout as *mut libc::FILE);

        libc::dup2(saved, 1);
        libc::close(saved);
        libc::close(devfull);
        libc::clearerr(stdout as *mut libc::FILE);
        err
    }
}

/// Run `f` with fd 1 **closed outright**; returns whether `ferror(stdout)` was
/// set afterwards.
pub fn run_with_stdout_closed(f: impl FnOnce()) -> bool {
    assert_single_threaded();
    let _g = capture_lock().lock().unwrap();
    unsafe {
        flush_all();
        let saved = libc::dup(1);
        assert!(saved >= 0);
        libc::close(1);
        libc::clearerr(stdout as *mut libc::FILE);

        f();

        libc::fflush(stdout as *mut libc::FILE);
        let err = libc::ferror(stdout as *mut libc::FILE) != 0;
        libc::clearerr(stdout as *mut libc::FILE);

        libc::dup2(saved, 1);
        libc::close(saved);
        libc::clearerr(stdout as *mut libc::FILE);
        err
    }
}

unsafe fn tempfile() -> std::fs::File {
    use std::os::unix::io::FromRawFd;
    let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".into());
    let mut tpl: Vec<u8> = dir.into_bytes();
    if tpl.last() == Some(&b'/') {
        tpl.pop();
    }
    tpl.extend_from_slice(b"/driver-diff-XXXXXX\0");
    let fd = libc::mkstemp(tpl.as_mut_ptr() as *mut c_char);
    assert!(fd >= 0, "mkstemp failed");
    // Unlink immediately; the fd keeps it alive.
    libc::unlink(tpl.as_ptr() as *const c_char);
    std::fs::File::from_raw_fd(fd)
}

fn as_fd(f: &std::fs::File) -> c_int {
    use std::os::unix::io::AsRawFd;
    f.as_raw_fd()
}

// ---------------------------------------------------------------------------
// differential drivers
// ---------------------------------------------------------------------------

/// Call C `driver(x, y)` and Rust `driver(x, y)`, each in its own capture, and
/// assert the byte streams are identical. Returns the (shared) output.
#[track_caller]
pub fn diff_one(x: c_int, y: c_int) -> Vec<i8> {
    let f = impls();
    let cout = capture(|| unsafe { (f.c)(x, y) });
    let rout = capture(|| unsafe { (f.rust)(x, y) });
    assert_eq!(
        show(&cout),
        show(&rout),
        "output mismatch for driver({x}, {y})\n  C:    {:?}\n  Rust: {:?}",
        show(&cout),
        show(&rout)
    );
    assert_eq!(cout, rout, "byte mismatch for driver({x}, {y})");
    cout.into_iter().map(|b| b as i8).collect()
}

#[track_caller]
pub fn diff_one_expect(x: c_int, y: c_int, expected: &str) {
    let f = impls();
    let cout = capture(|| unsafe { (f.c)(x, y) });
    let rout = capture(|| unsafe { (f.rust)(x, y) });
    assert_eq!(cout, rout, "byte mismatch for driver({x}, {y})");
    assert_eq!(
        show(&cout),
        expected,
        "C itself produced unexpected text for driver({x}, {y}) — check the test's expectation"
    );
}

/// Differential over a whole batch inside a **single** capture each, so stdio
/// buffer accumulation and inter-call ordering are compared too.
#[track_caller]
pub fn diff_batch(pairs: &[(c_int, c_int)]) {
    let f = impls();
    let cout = capture(|| unsafe {
        for &(x, y) in pairs {
            (f.c)(x, y);
        }
    });
    let rout = capture(|| unsafe {
        for &(x, y) in pairs {
            (f.rust)(x, y);
        }
    });
    if cout != rout {
        report_batch_mismatch(pairs, &cout, &rout);
    }
}

#[track_caller]
pub fn diff_batch_mode(mode: BufMode, pairs: &[(c_int, c_int)]) {
    let f = impls();
    let cout = capture_with(mode, || unsafe {
        for &(x, y) in pairs {
            (f.c)(x, y);
        }
    });
    let rout = capture_with(mode, || unsafe {
        for &(x, y) in pairs {
            (f.rust)(x, y);
        }
    });
    if cout != rout {
        report_batch_mismatch(pairs, &cout, &rout);
    }
}

#[track_caller]
pub fn diff_batch_pipe(pairs: &[(c_int, c_int)]) {
    let f = impls();
    let cout = capture_via_pipe(|| unsafe {
        for &(x, y) in pairs {
            (f.c)(x, y);
        }
    });
    let rout = capture_via_pipe(|| unsafe {
        for &(x, y) in pairs {
            (f.rust)(x, y);
        }
    });
    if cout != rout {
        report_batch_mismatch(pairs, &cout, &rout);
    }
}

#[track_caller]
fn report_batch_mismatch(pairs: &[(c_int, c_int)], cout: &[u8], rout: &[u8]) {
    let cl: Vec<&str> = std::str::from_utf8(cout).unwrap_or("<non-utf8>").lines().collect();
    let rl: Vec<&str> = std::str::from_utf8(rout).unwrap_or("<non-utf8>").lines().collect();
    for (i, (a, b)) in cl.iter().zip(rl.iter()).enumerate() {
        if a != b {
            let (x, y) = pairs.get(i).copied().unwrap_or((0, 0));
            panic!("batch mismatch at call #{i} driver({x}, {y}): C={a:?} Rust={b:?}");
        }
    }
    panic!(
        "batch mismatch: C produced {} lines / {} bytes, Rust produced {} lines / {} bytes",
        cl.len(),
        cout.len(),
        rl.len(),
        rout.len()
    );
}

pub fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

/// The value the C computes: `x bitor compl y` == `x | ~y`.
pub fn expected_text(x: c_int, y: c_int) -> String {
    format!("{}\n", x | !y)
}

// ---------------------------------------------------------------------------
// deterministic RNG (SplitMix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x2545_F491_4F6C_DD1D;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_i32(&mut self) -> i32 {
        (self.next_u64() >> 32) as u32 as i32
    }
    /// Uniform in `1..=i32::MAX` (strictly positive).
    pub fn next_pos(&mut self) -> i32 {
        (self.next_i32() & i32::MAX).max(1)
    }
    /// Uniform in `i32::MIN..=-1` (strictly negative).
    pub fn next_neg(&mut self) -> i32 {
        let v = self.next_i32() | i32::MIN;
        if v == 0 {
            -1
        } else {
            v
        }
    }
}
