//! Shared differential-test harness.
//!
//! Both implementations are loaded as *shared objects* through `libloading`
//! (the C one built by CMake, the Rust one being this crate's `cdylib`), so
//! every call goes through the real `extern "C"` export exactly as an external
//! consumer would call it. Rust functions are never called directly.
//!
//! `driver` returns `void` and communicates only through `stdout`, so the
//! comparison point is the exact byte stream each library writes to fd 1.

#![allow(dead_code)]

use libloading::Library;
use std::ffi::{c_char, c_int, c_void};
use std::fs::File;
use std::io::Write;
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

/// Signature of the single public entry point: `void driver(int x, int y);`
pub type DriverFn = unsafe extern "C" fn(c_int, c_int);

unsafe extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    fn fork() -> c_int;
    fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
    fn kill(pid: c_int, sig: c_int) -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(status: c_int);
    fn setvbuf(stream: *mut c_void, buf: *mut c_char, mode: c_int, size: usize) -> c_int;
    fn socketpair(domain: c_int, ty: c_int, protocol: c_int, sv: *mut c_int) -> c_int;
    fn alarm(seconds: u32) -> u32;
    fn setrlimit(resource: c_int, rlim: *const RLimit) -> c_int;
    static mut stdout: *mut c_void;
}

#[repr(C)]
struct RLimit {
    rlim_cur: u64,
    rlim_max: u64,
}

const RLIMIT_FSIZE: c_int = 1;
/// Largest output any legitimate test configuration produces is ≈3 MiB
/// (`driver(400_000, 0)`); cap well above that so a *divergent* implementation
/// that loops forever cannot fill the filesystem.
const MAX_CAPTURE_BYTES: u64 = 16 << 20;
/// Every legitimate configuration finishes in well under a second.
const CHILD_TIMEOUT_SECS: u32 = 30;
/// Cap on the number of `write(2)` frames recorded per call.
const MAX_FRAMES: usize = 100_000;

/// Guards installed in every capture child: a file-size limit (`SIGXFSZ`) and a
/// wall-clock limit (`SIGALRM`). Both show up as a distinct `waitpid` status, so
/// "one library finished, the other ran away" is reported as a difference
/// instead of hanging the test run.
unsafe fn child_guards() {
    let rl = RLimit {
        rlim_cur: MAX_CAPTURE_BYTES,
        rlim_max: MAX_CAPTURE_BYTES,
    };
    unsafe {
        setrlimit(RLIMIT_FSIZE, &rl);
        alarm(CHILD_TIMEOUT_SECS);
    }
}

const IOFBF: c_int = 0; // _IOFBF
const IOLBF: c_int = 1; // _IOLBF
const IONBF: c_int = 2; // _IONBF
const AF_UNIX: c_int = 1;
const SOCK_SEQPACKET: c_int = 5;

/// `stdout` buffering mode a caller may have configured before calling into the
/// library. The C library inherits whatever the caller set, so the Rust one
/// must behave identically in every mode.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BufMode {
    /// Whatever the process default is for the given fd.
    Inherit,
    /// `setvbuf(stdout, NULL, _IOFBF, 4096)`
    Full,
    /// `setvbuf(stdout, NULL, _IOLBF, 0)`
    Line,
    /// `setvbuf(stdout, NULL, _IONBF, 0)`
    Unbuffered,
}

impl BufMode {
    pub const ALL: [BufMode; 4] = [
        BufMode::Inherit,
        BufMode::Full,
        BufMode::Line,
        BufMode::Unbuffered,
    ];

    /// Must only be called in a freshly forked child, before any I/O.
    unsafe fn apply(self) {
        let s = unsafe { stdout };
        let rc = match self {
            BufMode::Inherit => 0,
            BufMode::Full => unsafe { setvbuf(s, std::ptr::null_mut(), IOFBF, 4096) },
            BufMode::Line => unsafe { setvbuf(s, std::ptr::null_mut(), IOLBF, 0) },
            BufMode::Unbuffered => unsafe { setvbuf(s, std::ptr::null_mut(), IONBF, 0) },
        };
        if rc != 0 {
            unsafe { _exit(97) };
        }
    }
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

struct Libs {
    c: Library,
    rust: Library,
}

static LIBS: OnceLock<Libs> = OnceLock::new();

fn c_so_path() -> PathBuf {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libdriver.so");
    assert!(
        p.is_file(),
        "C shared library not found at {}\nbuild it with:\n  cd c_src && mkdir -p build && cd build \\\n    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<testbin>-<hash>
    let mut dir = exe.parent().expect("test exe parent").to_path_buf();
    let mut found = None;
    for _ in 0..4 {
        let cand = dir.join("libdriver.so");
        if cand.is_file() {
            found = Some(cand);
            break;
        }
        match dir.parent() {
            Some(p) => dir = p.to_path_buf(),
            None => break,
        }
    }
    let so = found.unwrap_or_else(|| {
        panic!(
            "could not locate the Rust cdylib `libdriver.so` near {}\n\
             build it first with `cargo build`",
            exe.display()
        )
    });
    assert_fresh(&so);
    so
}

/// `cargo test` does **not** rebuild a `crate-type = ["cdylib"]` artifact, so a
/// stale `libdriver.so` would silently be tested. Refuse to run in that case.
fn assert_fresh(so: &PathBuf) {
    let so_mtime = std::fs::metadata(so)
        .and_then(|m| m.modified())
        .expect("stat cdylib");
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut newest: Option<(PathBuf, std::time::SystemTime)> = None;
    let mut stack = vec![src];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if let Ok(t) = e.metadata().and_then(|m| m.modified()) {
                if newest.as_ref().map(|(_, n)| t > *n).unwrap_or(true) {
                    newest = Some((p, t));
                }
            }
        }
    }
    if let Some((p, t)) = newest {
        assert!(
            t <= so_mtime,
            "STALE ARTIFACT: {} is newer than {}\n\
             `cargo test` does not rebuild a cdylib — run `cargo build` first \
             (or use ./check_all.sh)",
            p.display(),
            so.display()
        );
    }
}

fn libs() -> &'static Libs {
    LIBS.get_or_init(|| unsafe {
        let c = Library::new(c_so_path()).expect("dlopen C libdriver.so");
        let rust = Library::new(rust_so_path()).expect("dlopen Rust libdriver.so");
        Libs { c, rust }
    })
}

/// `driver` from the C shared object.
pub fn c_driver() -> DriverFn {
    unsafe {
        *libs()
            .c
            .get::<DriverFn>(b"driver\0")
            .expect("symbol `driver` missing from the C .so")
    }
}

/// `driver` from the Rust shared object (exercises the `#[no_mangle]` export).
pub fn rust_driver() -> DriverFn {
    unsafe {
        *libs()
            .rust
            .get::<DriverFn>(b"driver\0")
            .expect("symbol `driver` missing from the Rust .so")
    }
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn tmp_path(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "driver_diff_{}_{}_{}.bin",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed),
        tag
    ))
}

/// What a captured run produced.
pub struct Outcome {
    /// Exact bytes written to `stdout`.
    pub out: Vec<u8>,
    /// Raw `waitpid` status of the child that performed the call (so a crash /
    /// abort / panic counts as an observable difference too).
    pub status: c_int,
}

/// Run `f` in a forked child whose fd 1 is a temporary file, and return the
/// bytes it wrote plus the child's exit status.
///
/// A forked child is used (rather than redirecting fd 1 in-process) so that the
/// libtest harness — which keeps writing its own progress lines to fd 1 from
/// other threads — cannot contaminate the capture, and so that tests remain
/// safe to run in parallel.
pub fn capture_outcome(f: impl FnOnce()) -> Outcome {
    let path = tmp_path("cap");
    let file = File::create(&path).expect("create capture file");
    let fd = file.as_raw_fd();

    // Nothing may be pending in either stdout buffer, or the child would
    // duplicate it into the capture.
    let _ = std::io::stdout().flush();
    unsafe { fflush(std::ptr::null_mut()) };

    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork() failed");
    if pid == 0 {
        unsafe {
            child_guards();
            dup2(fd, 1);
            f();
            fflush(std::ptr::null_mut());
            _exit(0);
        }
        std::process::exit(0); // not reached
    }
    drop(file);

    let mut status: c_int = 0;
    let rc = unsafe { waitpid(pid, &mut status, 0) };
    assert_eq!(rc, pid, "waitpid failed");

    let out = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);
    Outcome { out, status }
}

/// Same as [`capture_outcome`] but asserts the child terminated normally.
pub fn capture(f: impl FnOnce()) -> Vec<u8> {
    let o = capture_outcome(f);
    assert_eq!(o.status, 0, "captured call did not exit cleanly");
    o.out
}

/// [`capture_outcome`] with a caller-selected `stdout` buffering mode.
pub fn capture_outcome_mode(mode: BufMode, f: impl FnOnce()) -> Outcome {
    let path = tmp_path("cap");
    let file = File::create(&path).expect("create capture file");
    let fd = file.as_raw_fd();

    let _ = std::io::stdout().flush();
    unsafe { fflush(std::ptr::null_mut()) };

    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork() failed");
    if pid == 0 {
        unsafe {
            child_guards();
            dup2(fd, 1);
            mode.apply();
            f();
            fflush(std::ptr::null_mut());
            _exit(0);
        }
        std::process::exit(0); // not reached
    }
    drop(file);

    let mut status: c_int = 0;
    assert_eq!(unsafe { waitpid(pid, &mut status, 0) }, pid, "waitpid failed");

    let out = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);
    Outcome { out, status }
}

/// Record the exact sequence of `write(2)` calls the library performs.
///
/// fd 1 is a `SOCK_SEQPACKET` socket, which preserves message boundaries, so
/// each element of the result is exactly one `write` the library issued. This
/// distinguishes e.g. `puts("loop")` (payload + `'\n'` as two writes when the
/// stream is unbuffered) from `printf("%s", "loop\n")` (a single write) — a
/// difference that is observable to any caller that shares fd 1 with another
/// writer.
pub fn capture_frames(f: DriverFn, x: c_int, y: c_int, mode: BufMode) -> Vec<Vec<u8>> {
    let mut sv = [0 as c_int; 2];
    assert_eq!(
        unsafe { socketpair(AF_UNIX, SOCK_SEQPACKET, 0, sv.as_mut_ptr()) },
        0,
        "socketpair() failed"
    );

    let _ = std::io::stdout().flush();
    unsafe { fflush(std::ptr::null_mut()) };

    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork() failed");
    if pid == 0 {
        unsafe {
            child_guards();
            close(sv[0]);
            dup2(sv[1], 1);
            close(sv[1]);
            mode.apply();
            f(x, y);
            fflush(std::ptr::null_mut());
            _exit(0);
        }
        std::process::exit(0); // not reached
    }
    unsafe { close(sv[1]) };

    let mut frames = Vec::new();
    let mut buf = vec![0u8; 1 << 16];
    let mut runaway = false;
    loop {
        let n = unsafe { read(sv[0], buf.as_mut_ptr() as *mut c_void, buf.len()) };
        if n <= 0 {
            break;
        }
        frames.push(buf[..n as usize].to_vec());
        if frames.len() > MAX_FRAMES {
            // A divergent implementation looping forever must not hang the run.
            runaway = true;
            unsafe { kill(pid, 9) };
            break;
        }
    }
    let mut status: c_int = 0;
    unsafe {
        waitpid(pid, &mut status, 0);
        close(sv[0]);
    }
    assert!(
        !runaway,
        "framing child produced more than {MAX_FRAMES} writes — runaway implementation?"
    );
    assert_eq!(status, 0, "framing child exited with status {status:#x}");
    frames
}

/// Call the library with fd 1 **closed** (a hostile but perfectly legal caller
/// state): every write must fail and the two libraries must react identically.
pub fn outcome_closed_stdout(f: DriverFn, x: c_int, y: c_int, mode: BufMode) -> Outcome {
    let _ = std::io::stdout().flush();
    unsafe { fflush(std::ptr::null_mut()) };
    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork() failed");
    if pid == 0 {
        unsafe {
            child_guards();
            close(1);
            mode.apply();
            f(x, y);
            let r = fflush(std::ptr::null_mut());
            _exit(if r == 0 { 0 } else { 1 });
        }
        std::process::exit(0); // not reached
    }
    let mut status: c_int = 0;
    assert_eq!(unsafe { waitpid(pid, &mut status, 0) }, pid, "waitpid failed");
    Outcome {
        out: Vec::new(),
        status,
    }
}

/// Call the library with fd 1 being a pipe whose read end is already closed
/// (`EPIPE` / `SIGPIPE` write-error surface).
pub fn outcome_broken_pipe(f: DriverFn, x: c_int, y: c_int, mode: BufMode) -> Outcome {
    let mut fds = [0 as c_int; 2];
    assert_eq!(unsafe { pipe(fds.as_mut_ptr()) }, 0, "pipe() failed");
    // Close the read end *before* forking so the child can never race ahead of
    // it: every write it performs is guaranteed to hit EPIPE/SIGPIPE.
    unsafe { close(fds[0]) };
    let _ = std::io::stdout().flush();
    unsafe { fflush(std::ptr::null_mut()) };
    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork() failed");
    if pid == 0 {
        unsafe {
            child_guards();
            dup2(fds[1], 1);
            close(fds[1]);
            mode.apply();
            f(x, y);
            let r = fflush(std::ptr::null_mut());
            _exit(if r == 0 { 0 } else { 1 });
        }
        std::process::exit(0); // not reached
    }
    unsafe {
        close(fds[1]);
    }
    let mut status: c_int = 0;
    assert_eq!(unsafe { waitpid(pid, &mut status, 0) }, pid, "waitpid failed");
    Outcome {
        out: Vec::new(),
        status,
    }
}

/// Capture at most `max_bytes` of output from a call that may run
/// (essentially) forever: the call happens in a forked child whose stdout is a
/// pipe; the child is killed once enough bytes have been observed.
pub fn capture_prefix(f: DriverFn, x: c_int, y: c_int, max_bytes: usize) -> Vec<u8> {
    let mut fds = [0 as c_int; 2];
    assert_eq!(unsafe { pipe(fds.as_mut_ptr()) }, 0, "pipe() failed");

    let _ = std::io::stdout().flush();
    unsafe { fflush(std::ptr::null_mut()) };

    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork() failed");
    if pid == 0 {
        // Child: only async-signal-safe-ish work plus the library call.
        unsafe {
            close(fds[0]);
            dup2(fds[1], 1);
            close(fds[1]);
            f(x, y);
            fflush(std::ptr::null_mut());
            _exit(0);
        }
        // Not reached.
        std::process::exit(0);
    }

    unsafe { close(fds[1]) };
    let mut buf = vec![0u8; max_bytes];
    let mut got = 0usize;
    while got < max_bytes {
        let n = unsafe {
            read(
                fds[0],
                buf[got..].as_mut_ptr() as *mut c_void,
                max_bytes - got,
            )
        };
        if n <= 0 {
            break;
        }
        got += n as usize;
    }
    unsafe {
        kill(pid, 9); // SIGKILL
        let mut status: c_int = 0;
        waitpid(pid, &mut status, 0);
        close(fds[0]);
    }

    buf.truncate(got);
    buf
}

// ---------------------------------------------------------------------------
// Differential assertions
// ---------------------------------------------------------------------------

fn preview(b: &[u8]) -> String {
    let n = b.len().min(160);
    String::from_utf8_lossy(&b[..n]).escape_debug().to_string()
}

fn report(label: &str, x: c_int, y: c_int, c_out: &[u8], r_out: &[u8]) -> String {
    let first_diff = c_out
        .iter()
        .zip(r_out.iter())
        .position(|(a, b)| a != b)
        .unwrap_or_else(|| c_out.len().min(r_out.len()));
    format!(
        "{label}: driver({x}, {y}) diverged\n  C   len={} \n  Rust len={}\n  first difference at byte {first_diff}\n  C   [..]: \"{}\"\n  Rust[..]: \"{}\"",
        c_out.len(),
        r_out.len(),
        preview(&c_out[first_diff.min(c_out.len())..]),
        preview(&r_out[first_diff.min(r_out.len())..]),
    )
}

/// Call both `.so` exports with `(x, y)` and assert byte-identical stdout.
pub fn assert_same(x: c_int, y: c_int) {
    assert_same_labelled("case", x, y);
}

pub fn assert_same_labelled(label: &str, x: c_int, y: c_int) {
    let cf = c_driver();
    let rf = rust_driver();
    let c = capture_outcome(|| unsafe { cf(x, y) });
    let r = capture_outcome(|| unsafe { rf(x, y) });
    if c.out != r.out {
        panic!("{}", report(label, x, y, &c.out, &r.out));
    }
    assert_eq!(
        c.status, r.status,
        "{label}: driver({x}, {y}) termination status differs (C={:#x}, Rust={:#x})",
        c.status, r.status
    );
    assert_eq!(
        c.status, 0,
        "{label}: driver({x}, {y}) did not terminate cleanly in either library"
    );
}

/// Compare only the first `max_bytes` of output (for non-terminating inputs).
pub fn assert_same_prefix(label: &str, x: c_int, y: c_int, max_bytes: usize) {
    let c_out = capture_prefix(c_driver(), x, y, max_bytes);
    let r_out = capture_prefix(rust_driver(), x, y, max_bytes);
    if c_out != r_out {
        panic!("{}", report(label, x, y, &c_out, &r_out));
    }
    assert_eq!(
        c_out.len(),
        max_bytes,
        "{label}: expected a full {max_bytes}-byte window from driver({x}, {y})"
    );
    assert_eq!(
        r_out.len(),
        max_bytes,
        "{label}: expected a full {max_bytes}-byte window from driver({x}, {y})"
    );
}

// ---------------------------------------------------------------------------
// Deterministic RNG (xorshift64*), fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    /// Uniform in `[lo, hi]` (inclusive).
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
}

pub const SEED: u64 = 0x2545F4914F6CDD1D;

/// `x > 0 && y < 0` makes the C library run ~2^31 iterations (and hit signed
/// overflow UB); such pairs are steered away from for the bounded valid-path
/// rows and covered separately by the capped-prefix error-path tests.
pub fn bounded_pair(x: c_int, y: c_int) -> (c_int, c_int) {
    if x > 0 && y < 0 { (x, 0) } else { (x, y) }
}
