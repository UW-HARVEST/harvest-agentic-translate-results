//! Shared differential-test harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading` and called
//! only through their exported symbols, exactly as an external consumer would
//! (this exercises the `#[no_mangle]` wrapper too). Nothing in the Rust crate
//! is called directly.
//!
//! `sieve` returns nothing and its only observable effect is bytes on
//! `stdout`, so every comparison here is a byte-for-byte comparison of
//! captured `stdout`.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void, CString};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

// ---------------------------------------------------------------------------
// libc bits we need for fd juggling / forking
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    /// glibc's `stdout` FILE*, shared by the test binary, the C `.so` and the
    /// Rust `.so` (all three link the same libc).
    static mut stdout: *mut libc::FILE;
}

fn stdout_file() -> *mut libc::FILE {
    unsafe { stdout }
}

/// Serializes every test that touches the process-wide fd 1 / `stdout` FILE.
static STDOUT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
static TMP_SEQ: AtomicU64 = AtomicU64::new(0);

pub fn lock_stdout() -> std::sync::MutexGuard<'static, ()> {
    STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

/// The one exported symbol, in the ABI shape the header declares.
pub type SieveFn = unsafe extern "C" fn(c_int);
/// The same symbol viewed as taking a 64-bit argument, used to prove the
/// callee only reads the low 32 bits (ERRORS.md row 10 / CONFIGS.md row 18).
pub type SieveFn64 = unsafe extern "C" fn(i64);

pub struct Lib {
    _lib: Library,
    pub sieve: SieveFn,
    pub sieve64: SieveFn64,
    pub name: &'static str,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    manifest_dir().join("../c_src/build/libSieve.so")
}

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let release = manifest_dir().join("target/release/libSieve.so");
    if release.exists() {
        return release;
    }
    manifest_dir().join("target/debug/libSieve.so")
}

fn load(path: PathBuf, name: &'static str) -> Lib {
    assert!(
        path.exists(),
        "{} shared object not found at {}: build it first \
         (C: cmake --build c_src/build ; Rust: cargo build --release)",
        name,
        path.display()
    );
    unsafe {
        let lib = Library::new(&path).unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display()));
        let s: Symbol<SieveFn> = lib
            .get(b"sieve\0")
            .unwrap_or_else(|e| panic!("{} does not export `sieve`: {e}", path.display()));
        let sieve = *s;
        let s64: Symbol<SieveFn64> = lib.get(b"sieve\0").unwrap();
        let sieve64 = *s64;
        drop(s);
        Lib {
            _lib: lib,
            sieve,
            sieve64,
            name,
        }
    }
}

pub fn c_lib() -> Lib {
    load(c_so_path(), "C")
}

pub fn rust_lib() -> Lib {
    load(rust_so_path(), "Rust")
}

/// Both libraries, loaded into the same process.
pub fn both() -> (Lib, Lib) {
    (c_lib(), rust_lib())
}

// ---------------------------------------------------------------------------
// In-process stdout capture (for calls that terminate)
// ---------------------------------------------------------------------------

/// Redirect fd 1 to a temp file, run `f`, flush every stdio stream, restore
/// fd 1 and return the bytes that were written.
///
/// The pre-flush drains anything the harness had pending so it cannot leak
/// into the captured bytes; the post-flush pushes the shared `stdout` FILE
/// buffer out before fd 1 is put back. Because the Rust `.so` calls the same
/// libc `printf` the C `.so` does, one `fflush(NULL)` covers both.
pub fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = lock_stdout();
    let seq = TMP_SEQ.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!(
        "sieve_cap_{}_{}.out",
        unsafe { libc::getpid() },
        seq
    ));
    let cpath = CString::new(path.to_str().unwrap()).unwrap();

    unsafe {
        libc::fflush(std::ptr::null_mut());
        let saved = libc::dup(1);
        assert!(saved >= 0, "dup(1) failed");
        let fd = libc::open(
            cpath.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC,
            0o600,
        );
        assert!(fd >= 0, "open({}) failed", path.display());
        assert!(libc::dup2(fd, 1) >= 0, "dup2 failed");
        libc::close(fd);

        f();

        libc::fflush(std::ptr::null_mut());
        assert!(libc::dup2(saved, 1) >= 0, "dup2 restore failed");
        libc::close(saved);
    }

    let bytes = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);
    bytes
}

/// stdio buffering mode to install on `stdout` before the call.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Buffering {
    /// Leave whatever mode the stream already has (fully buffered onto a file).
    Default,
    /// `setvbuf(stdout, NULL, _IOFBF, 0)`
    Full,
    /// `setvbuf(stdout, NULL, _IOLBF, 0)`
    Line,
    /// `setvbuf(stdout, NULL, _IONBF, 0)`
    None,
}

fn apply_buffering(mode: Buffering) {
    unsafe {
        let m = match mode {
            Buffering::Default => return,
            Buffering::Full => libc::_IOFBF,
            Buffering::Line => libc::_IOLBF,
            Buffering::None => libc::_IONBF,
        };
        libc::fflush(stdout_file());
        libc::setvbuf(stdout_file(), std::ptr::null_mut(), m, 0);
    }
}

fn reset_buffering() {
    unsafe {
        libc::fflush(stdout_file());
        libc::setvbuf(stdout_file(), std::ptr::null_mut(), libc::_IOFBF, 0);
    }
}

/// Capture the full stdout of `lib.sieve(val)` under a given buffering mode.
pub fn run_capture(lib: &Lib, val: i32, mode: Buffering) -> Vec<u8> {
    capture(|| {
        apply_buffering(mode);
        unsafe { (lib.sieve)(val) };
        if mode != Buffering::Default {
            reset_buffering();
        }
    })
}

/// Capture stdout of `lib.sieve(val)` called through the 64-bit-argument view.
pub fn run_capture_i64(lib: &Lib, raw: i64) -> Vec<u8> {
    capture(|| unsafe { (lib.sieve64)(raw) })
}

/// Capture stdout with `pre` bytes already sitting unflushed in the shared
/// `stdout` FILE buffer (CONFIGS.md row 16).
pub fn run_capture_with_pending(lib: &Lib, val: i32, pre: &str) -> Vec<u8> {
    let pre = CString::new(pre).unwrap();
    capture(|| unsafe {
        printf(b"%s\0".as_ptr() as *const c_char, pre.as_ptr());
        (lib.sieve)(val);
    })
}

// ---------------------------------------------------------------------------
// Forked bounded-prefix capture (for calls that would run for ~2e9 lines)
// ---------------------------------------------------------------------------

/// Make glibc allocate `stdout`'s buffer now, so the forked child never has to
/// call `malloc` (avoiding the classic fork-in-a-threaded-process hazard).
fn prewarm_stdout() {
    unsafe {
        libc::fflush(std::ptr::null_mut());
        let saved = libc::dup(1);
        let devnull = libc::open(b"/dev/null\0".as_ptr() as *const c_char, libc::O_WRONLY);
        if devnull >= 0 {
            libc::dup2(devnull, 1);
            printf(b" \0".as_ptr() as *const c_char);
            libc::fflush(std::ptr::null_mut());
            libc::dup2(saved, 1);
            libc::close(devnull);
        }
        libc::close(saved);
    }
}

/// Fork, point the child's stdout at a pipe, call `sieve(val)` there, and read
/// the first `want` bytes it produces. The child is then killed, so this works
/// for inputs whose full run would emit gigabytes.
///
/// Identical treatment for both libraries makes the prefixes comparable.
pub fn run_prefix(lib: &Lib, val: i32, want: usize) -> Vec<u8> {
    let _guard = lock_stdout();
    prewarm_stdout();
    let mut fds = [0i32; 2];
    unsafe {
        assert_eq!(libc::pipe(fds.as_mut_ptr()), 0, "pipe failed");
        libc::fflush(std::ptr::null_mut());
        let pid = libc::fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // ---- child ----
            libc::close(fds[0]);
            libc::dup2(fds[1], 1);
            libc::close(fds[1]);
            libc::alarm(120); // hard stop if the callee never returns
            (lib.sieve)(val);
            libc::fflush(std::ptr::null_mut());
            libc::_exit(0);
        }
        // ---- parent ----
        libc::close(fds[1]);
        let mut buf = vec![0u8; want];
        let mut got = 0usize;
        while got < want {
            let n = libc::read(
                fds[0],
                buf[got..].as_mut_ptr() as *mut c_void,
                want - got,
            );
            if n <= 0 {
                break;
            }
            got += n as usize;
        }
        buf.truncate(got);
        libc::kill(pid, libc::SIGKILL);
        let mut status = 0i32;
        libc::waitpid(pid, &mut status, 0);
        libc::close(fds[0]);
        buf
    }
}

// ---------------------------------------------------------------------------
// Forked exit-status probe (for hostile stdout states)
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug)]
pub enum StdoutState {
    /// fd 1 points at /dev/null — writes succeed and are discarded.
    DevNull,
    /// fd 1 is closed — every `write(2)` fails with EBADF.
    Closed,
    /// fd 1 points at /dev/full — every `write(2)` fails with ENOSPC.
    DevFull,
    /// fd 1 is a read-only fd — every `write(2)` fails with EBADF.
    ReadOnly,
}

/// Result of a forked call: the child exits 0 only if `sieve` returned
/// normally. A signal (crash / abort / SIGALRM timeout) is reported instead.
#[derive(Debug, PartialEq, Eq)]
pub enum ChildOutcome {
    Exited(i32),
    Signaled(i32),
}

/// Call `sieve(val)` in a child process whose stdout is in the given state and
/// report how the child terminated. Used to show the C ignores `printf`
/// failures and still returns — and that the Rust does too.
pub fn run_child_outcome(lib: &Lib, val: i32, state: StdoutState) -> ChildOutcome {
    let _guard = lock_stdout();
    prewarm_stdout();
    unsafe {
        libc::fflush(std::ptr::null_mut());
        let pid = libc::fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // ---- child ----
            libc::alarm(10); // hard stop for inputs whose run is ~2e9 lines
            match state {
                StdoutState::DevNull => {
                    let fd = libc::open(b"/dev/null\0".as_ptr() as *const c_char, libc::O_WRONLY);
                    libc::dup2(fd, 1);
                }
                StdoutState::Closed => {
                    libc::close(1);
                }
                StdoutState::DevFull => {
                    let fd = libc::open(b"/dev/full\0".as_ptr() as *const c_char, libc::O_WRONLY);
                    if fd < 0 {
                        libc::close(1);
                    } else {
                        libc::dup2(fd, 1);
                    }
                }
                StdoutState::ReadOnly => {
                    let fd = libc::open(b"/dev/null\0".as_ptr() as *const c_char, libc::O_RDONLY);
                    libc::dup2(fd, 1);
                }
            }
            (lib.sieve)(val);
            libc::fflush(std::ptr::null_mut());
            libc::_exit(0);
        }
        let mut status = 0i32;
        libc::waitpid(pid, &mut status, 0);
        if libc::WIFEXITED(status) {
            ChildOutcome::Exited(libc::WEXITSTATUS(status))
        } else {
            ChildOutcome::Signaled(libc::WTERMSIG(status))
        }
    }
}

// ---------------------------------------------------------------------------
// Expected-cost model, so tests pick full vs prefix comparison mechanically
// ---------------------------------------------------------------------------

/// Number of lines `sieve(val)` prints, or `None` if `val++` signed-overflows
/// on the way (in which case the run is ~2^31 lines long).
pub fn line_count(val: i32) -> Option<i64> {
    if val >= 0 {
        let r = val % 10;
        let target = val as i64 + (9 - r) as i64;
        if target <= i32::MAX as i64 {
            Some(target - val as i64 + 1)
        } else {
            None // wraps through INT_MIN first
        }
    } else {
        // Truncating `%` makes negative residues, so a negative `val` never
        // matches `== 9`; it counts up to +9.
        Some(10i64 - val as i64)
    }
}

/// Lines we are willing to materialize in a full byte-for-byte comparison.
pub const FULL_COMPARE_LIMIT: i64 = 40_000;

pub fn is_cheap(val: i32) -> bool {
    matches!(line_count(val), Some(n) if n <= FULL_COMPARE_LIMIT)
}

// ---------------------------------------------------------------------------
// Differential assertions
// ---------------------------------------------------------------------------

fn show(bytes: &[u8]) -> String {
    let s = String::from_utf8_lossy(bytes);
    if s.len() <= 400 {
        s.into_owned()
    } else {
        format!("{}…[{} bytes total]", &s[..400], bytes.len())
    }
}

fn diff_note(val: i32, c: &[u8], r: &[u8]) -> String {
    let pos = c
        .iter()
        .zip(r.iter())
        .position(|(a, b)| a != b)
        .unwrap_or_else(|| c.len().min(r.len()));
    format!(
        "sieve({val}) diverged.\n  first differing byte offset: {pos}\n  \
         C   ({} bytes): {}\n  Rust({} bytes): {}",
        c.len(),
        show(c),
        r.len(),
        show(r)
    )
}

/// Full byte-for-byte comparison of both `.so`s for one input.
pub fn assert_same(c: &Lib, r: &Lib, val: i32, mode: Buffering) {
    // Guard: a full comparison of an overflow-region input would materialize
    // ~2e9 lines (~23 GB). Fail fast instead of filling the disk.
    assert!(
        is_cheap(val),
        "assert_same({val}) would emit {:?} lines; use assert_same_prefix",
        line_count(val)
    );
    let out_c = run_capture(c, val, mode);
    let out_r = run_capture(r, val, mode);
    assert_eq!(out_c, out_r, "{}", diff_note(val, &out_c, &out_r));
}

/// Bounded-prefix comparison, for inputs whose full run is astronomically long.
pub fn assert_same_prefix(c: &Lib, r: &Lib, val: i32, want: usize) {
    let out_c = run_prefix(c, val, want);
    let out_r = run_prefix(r, val, want);
    assert!(
        !out_c.is_empty(),
        "C produced no output for sieve({val}); capture harness is broken"
    );
    assert_eq!(
        out_c.len(),
        want,
        "C prefix for sieve({val}) is short ({} of {want} bytes) — the run \
         ended sooner than the cost model predicted",
        out_c.len()
    );
    assert_eq!(out_c, out_r, "{}", diff_note(val, &out_c, &out_r));
}

/// Pick the cheaper comparison automatically.
pub fn assert_same_auto(c: &Lib, r: &Lib, val: i32) {
    if is_cheap(val) {
        assert_same(c, r, val, Buffering::Default);
    } else {
        assert_same_prefix(c, r, val, 8192);
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed, so failures reproduce exactly)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

pub const SEED: u64 = 0x5EED_1234_ABCD_EF01;

impl Rng {
    pub fn new() -> Self {
        Rng(SEED)
    }
    pub fn with_seed(s: u64) -> Self {
        Rng(if s == 0 { SEED } else { s })
    }
    pub fn next_u64(&mut self) -> u64 {
        // xorshift64*
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
    /// Any 32-bit pattern, reinterpreted as `int` — this is exactly what a
    /// hostile caller can push across the FFI boundary.
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform in `[lo, hi]` inclusive.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
}
