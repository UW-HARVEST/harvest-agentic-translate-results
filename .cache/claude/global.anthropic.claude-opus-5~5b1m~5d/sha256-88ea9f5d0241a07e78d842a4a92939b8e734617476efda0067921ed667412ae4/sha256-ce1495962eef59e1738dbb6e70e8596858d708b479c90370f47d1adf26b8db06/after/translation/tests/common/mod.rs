// Shared support code for the differential tests.
//
// Both the C `.so` and the Rust `.so` are loaded with `libloading` and driven
// exclusively through their exported C symbols, so the `#[no_mangle]` export
// wrappers are part of what is under test.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::fs;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use libloading::{Library, Symbol};

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

// ---------------------------------------------------------------------------
// Library discovery
// ---------------------------------------------------------------------------

/// `<repo>/translation`
fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Directory holding the cargo build artifacts for the profile the test runs in
/// (`.../target/debug` or `.../target/release`).
pub fn artifact_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test-binary>
    exe.parent()
        .and_then(Path::parent)
        .expect("artifact dir")
        .to_path_buf()
}

pub fn c_lib_path() -> PathBuf {
    let p = manifest_dir()
        .parent()
        .expect("repo root")
        .join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {p:?}.\nBuild it with:\n  cd c_src && mkdir -p build && \
         cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    p
}

pub fn rust_lib_path() -> PathBuf {
    let dir = artifact_dir();
    let mut found = None;
    for cand in [dir.join("libdriver.so"), dir.join("deps/libdriver.so")] {
        if cand.exists() {
            found = Some(cand);
            break;
        }
    }
    let path = found.unwrap_or_else(|| {
        // Fall back to the release artifact, which `run_all.sh` always builds.
        let rel = manifest_dir().join("target/release/libdriver.so");
        assert!(
            rel.exists(),
            "Rust cdylib not found in {dir:?} nor at {rel:?}; run `cargo build` first"
        );
        rel
    });
    assert_not_stale(&path);
    path
}

/// `cargo test` does **not** rebuild the `cdylib` artifact (nothing in the test
/// graph links against it), so a stale `.so` would silently be tested and the
/// whole suite could pass while the current sources are broken. Refuse to run
/// in that case.
fn assert_not_stale(so: &Path) {
    let so_time = fs::metadata(so).and_then(|m| m.modified()).ok();
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    let src = manifest_dir().join("src");
    let mut stack = vec![src];
    while let Some(d) = stack.pop() {
        let Ok(rd) = fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "rs") {
                if let Ok(t) = e.metadata().and_then(|m| m.modified()) {
                    if newest.as_ref().is_none_or(|(bt, _)| t > *bt) {
                        newest = Some((t, p));
                    }
                }
            }
        }
    }
    if let (Some(so_time), Some((src_time, src_path))) = (so_time, newest) {
        assert!(
            so_time >= src_time,
            "STALE ARTIFACT: {so:?} is older than {src_path:?}.\n\
             `cargo test` does not rebuild the cdylib; run `cargo build` (same profile) \
             or use ./run_all.sh"
        );
    }
}

pub fn runner_path() -> Option<PathBuf> {
    let p = artifact_dir().join("examples/runner");
    if p.exists() { Some(p) } else { None }
}

pub struct Libs {
    pub c: Library,
    pub rs: Library,
}

impl Libs {
    fn load() -> Self {
        let c = unsafe { Library::new(c_lib_path()) }.expect("dlopen C lib");
        let rs = unsafe { Library::new(rust_lib_path()) }.expect("dlopen Rust lib");
        Libs { c, rs }
    }

    pub fn driver(&self, which: Which) -> Symbol<'_, unsafe extern "C" fn(c_int)> {
        let lib = match which {
            Which::C => &self.c,
            Which::Rust => &self.rs,
        };
        unsafe { lib.get(b"driver\0") }.expect("`driver` symbol")
    }

    pub fn print_line(&self, which: Which) -> Symbol<'_, unsafe extern "C" fn(*const c_char)> {
        let lib = match which {
            Which::C => &self.c,
            Which::Rust => &self.rs,
        };
        unsafe { lib.get(b"printLine\0") }.expect("`printLine` symbol")
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Which {
    C,
    Rust,
}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(Libs::load)
}

// ---------------------------------------------------------------------------
// stdout capture (both libraries write through libc `printf`)
// ---------------------------------------------------------------------------

static CAPTURE_LOCK: Mutex<()> = Mutex::new(());
static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Runs `f` with file descriptor 1 redirected into a temporary file and returns
/// the raw bytes that were written to it.
pub fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    let guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!("driver-diff-{}-{}.out", std::process::id(), n));

    let bytes = {
        let file = fs::File::create(&path).expect("create temp file");
        // Flush Rust's own line-buffered stdout (the libtest harness leaves a
        // partial `test <name> ... ` line in it) so that it cannot be flushed
        // into our capture file later on.
        {
            use std::io::Write;
            let _ = std::io::stdout().flush();
            let _ = std::io::stderr().flush();
        }
        unsafe {
            // Flush anything already pending so it does not land in our file.
            fflush(std::ptr::null_mut());
            let saved = dup(1);
            assert!(saved >= 0, "dup(1) failed");
            assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");

            f();

            // Flush the C runtime streams *before* restoring fd 1: stdout is a
            // regular file here, hence fully buffered.
            fflush(std::ptr::null_mut());
            assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
            close(saved);
        }
        fs::read(&path).expect("read temp file")
    };

    let _ = fs::remove_file(&path);
    drop(guard);
    bytes
}

/// Calls `driver(data)` in `which` library and returns the captured stdout.
pub fn run_driver(which: Which, data: i32) -> Vec<u8> {
    let l = libs();
    let f = l.driver(which);
    capture(|| unsafe { f(data) })
}

/// Calls `printLine(ptr)` in `which` library and returns the captured stdout.
/// `buf` must contain its own NUL terminator.
pub fn run_print_line(which: Which, buf: &[u8]) -> Vec<u8> {
    let l = libs();
    let f = l.print_line(which);
    capture(|| unsafe { f(buf.as_ptr() as *const c_char) })
}

pub fn run_print_line_null(which: Which) -> Vec<u8> {
    let l = libs();
    let f = l.print_line(which);
    capture(|| unsafe { f(std::ptr::null()) })
}

// ---------------------------------------------------------------------------
// Assertions
// ---------------------------------------------------------------------------

pub fn show(b: &[u8]) -> String {
    let mut s = String::new();
    for &c in b.iter().take(160) {
        match c {
            b'\n' => s.push_str("\\n"),
            0x20..=0x7e => s.push(c as char),
            _ => s.push_str(&format!("\\x{c:02x}")),
        }
    }
    if b.len() > 160 {
        s.push_str("...");
    }
    format!("len={} \"{}\"", b.len(), s)
}

pub fn assert_same(ctx: &str, c: &[u8], rs: &[u8]) {
    assert!(
        c == rs,
        "output mismatch for {ctx}\n   C: {}\nRust: {}",
        show(c),
        show(rs)
    );
}

/// Differential check of `driver(data)`.
pub fn diff_driver(data: i32) {
    let c = run_driver(Which::C, data);
    let rs = run_driver(Which::Rust, data);
    assert_same(&format!("driver({data})"), &c, &rs);
}

/// Differential check of `printLine(buf)`; `buf` must be NUL-terminated.
pub fn diff_print_line(buf: &[u8]) {
    assert!(
        buf.contains(&0),
        "test bug: printLine input must be NUL terminated"
    );
    let c = run_print_line(Which::C, buf);
    let rs = run_print_line(Which::Rust, buf);
    assert_same(&format!("printLine({})", show(buf)), &c, &rs);
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*), fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234_ABCD_0001;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 1 } else { seed })
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
    /// Uniform in `[lo, hi]` (inclusive).
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
    /// A non-zero byte (so it never terminates a C string prematurely).
    pub fn nonzero_byte(&mut self) -> u8 {
        1u8.wrapping_add((self.next_u64() >> 33) as u8 % 255)
    }
}

// ---------------------------------------------------------------------------
// Subprocess helpers (for the UB / crashing inputs)
// ---------------------------------------------------------------------------

use std::process::{Command, Stdio};

#[derive(Debug, PartialEq, Eq)]
pub struct RunOutcome {
    pub code: Option<i32>,
    pub signal: Option<i32>,
    pub stdout: Vec<u8>,
}

fn outcome_from(out: std::process::Output) -> RunOutcome {
    use std::os::unix::process::ExitStatusExt;
    RunOutcome {
        code: out.status.code(),
        signal: out.status.signal(),
        stdout: out.stdout,
    }
}

/// Runs the `runner` example against `which` library, capturing stdout via a pipe.
pub fn run_subprocess(which: Which, op: &str, arg: &str) -> Option<RunOutcome> {
    let runner = runner_path()?;
    let lib = match which {
        Which::C => c_lib_path(),
        Which::Rust => rust_lib_path(),
    };
    let out = Command::new(runner)
        .arg(lib)
        .arg(op)
        .arg(arg)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn runner");
    let o = outcome_from(out);
    assert_ne!(
        o.code,
        Some(2),
        "runner rejected `{op}` as a usage error (stale example binary? run `cargo build --examples`)"
    );
    Some(o)
}

/// Same as `run_subprocess` but with stdout redirected to a *file* (fully
/// buffered stream) instead of a pipe.
pub fn run_subprocess_to_file(which: Which, op: &str, arg: &str) -> Option<RunOutcome> {
    let runner = runner_path()?;
    let lib = match which {
        Which::C => c_lib_path(),
        Which::Rust => rust_lib_path(),
    };
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let path =
        std::env::temp_dir().join(format!("driver-sub-{}-{}-{n}.out", std::process::id(), op));
    let file = fs::File::create(&path).expect("create temp file");
    let status = Command::new(runner)
        .arg(lib)
        .arg(op)
        .arg(arg)
        .stdin(Stdio::null())
        .stdout(Stdio::from(file))
        .stderr(Stdio::null())
        .status()
        .expect("spawn runner");
    let stdout = fs::read(&path).unwrap_or_default();
    let _ = fs::remove_file(&path);
    use std::os::unix::process::ExitStatusExt;
    Some(RunOutcome {
        code: status.code(),
        signal: status.signal(),
        stdout,
    })
}
