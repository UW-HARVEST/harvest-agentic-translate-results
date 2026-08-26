// Shared differential-test harness.
//
// Both the C shared library (`c_src/build/libSieve.so`) and the Rust shared
// library (a cdylib built from `src/lib.rs`) are loaded with `libloading`; the
// Rust side is NEVER called directly, always through its exported `sieve`
// symbol, so the `#[no_mangle] extern "C"` wrapper is under test too.
//
// `sieve` communicates only by writing to the process's `stdout` stream, so the
// harness captures output at the *file descriptor* level, which sees the libc
// `printf` bytes emitted from inside either `.so`.
//
// Two important properties of this harness:
//
//  1. FRESHNESS. `cargo test` does *not* rebuild a `crate-type = ["cdylib"]`
//     target (an integration test cannot link one), so `target/<profile>/
//     libSieve.so` may be stale — which would silently make every differential
//     test pass against an old artifact. `rust_so_path()` therefore uses the
//     cargo artifact only when it is newer than `src/lib.rs`, and otherwise
//     compiles a fresh cdylib with `rustc`.
//
//  2. BOUNDEDNESS. `sieve` can legitimately run for ~2^31 iterations, and a
//     *divergent* implementation can run forever. Comparisons therefore execute
//     in a forked child writing to a pipe, with a byte cap and a wall-clock
//     timeout, so a wrong implementation fails loudly instead of filling the
//     disk or hanging.
//
// All fd-1 manipulation and all `fork()` calls are serialized by one mutex, and
// libc's `stdout` buffer is pre-warmed before forking so the child allocates
// nothing.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::fs::{File, OpenOptions};
use std::io::Write as _;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

pub type SieveFn = unsafe extern "C" fn(i32);
pub type SieveWideFn = unsafe extern "C" fn(i64);

unsafe extern "C" {
    fn dup(oldfd: i32) -> i32;
    fn dup2(oldfd: i32, newfd: i32) -> i32;
    fn close(fd: i32) -> i32;
    fn fflush(stream: *mut c_void) -> i32;
    fn pipe(fds: *mut i32) -> i32;
    fn read(fd: i32, buf: *mut u8, count: usize) -> isize;
    fn fork() -> i32;
    fn _exit(code: i32) -> !;
    fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
    fn kill(pid: i32, sig: i32) -> i32;
    fn poll(fds: *mut PollFdC, nfds: u64, timeout: i32) -> i32;
}

#[repr(C)]
struct PollFdC {
    fd: i32,
    events: i16,
    revents: i16,
}

const POLLIN: i16 = 0x001;
const SIGKILL: i32 = 9;
const WNOHANG: i32 = 1;

/// Default byte cap for a single forked comparison run.
pub const CAP: usize = 32 << 20; // 32 MiB
/// Default wall-clock limit for a forked comparison run.
pub const TIMEOUT_MS: i32 = 30_000;

static IO_LOCK: Mutex<()> = Mutex::new(());

fn lock() -> std::sync::MutexGuard<'static, ()> {
    IO_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

// ---------------------------------------------------------------- library load

pub fn c_so_path() -> PathBuf {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libSieve.so");
    assert!(
        p.exists(),
        "C shared library not built: {}\nBuild it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

/// The cargo-produced cdylib for the profile the tests were built with, if any.
pub fn cargo_so_path() -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(profile_dir) = exe.parent().and_then(|deps| deps.parent()) {
            let p = profile_dir.join("libSieve.so");
            if p.exists() {
                return Some(p);
            }
        }
    }
    for profile in ["debug", "release"] {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join(profile)
            .join("libSieve.so");
        if p.exists() {
            return Some(p);
        }
    }
    None
}

fn mtime(p: &Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(p).ok()?.modified().ok()
}

fn src_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src").join("lib.rs")
}

/// True if `so` is at least as new as `src/lib.rs`.
pub fn cargo_so_is_fresh(so: &Path) -> bool {
    match (mtime(so), mtime(&src_path())) {
        (Some(a), Some(b)) => a >= b,
        _ => false,
    }
}

fn build_fresh_cdylib() -> PathBuf {
    let src = src_path();
    let tag = std::env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|s| s.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "test".into());
    let out = tmp_dir().join(format!("libSieve_fresh_{}_{}.so", std::process::id(), tag));
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    let o = std::process::Command::new(&rustc)
        .args([
            "--edition",
            "2021",
            "--crate-type",
            "cdylib",
            "--crate-name",
            "Sieve",
            "-C",
            "debuginfo=0",
            "-o",
        ])
        .arg(&out)
        .arg(&src)
        .output()
        .unwrap_or_else(|e| panic!("cannot run `{rustc}` to build a fresh cdylib: {e}"));
    assert!(
        o.status.success(),
        "cannot build fresh Rust cdylib from {}:\n{}",
        src.display(),
        String::from_utf8_lossy(&o.stderr)
    );
    out
}

/// Path of the Rust `.so` under test: the cargo artifact when up to date,
/// otherwise a freshly compiled one (never a stale artifact).
pub fn rust_so_path() -> PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| match cargo_so_path() {
        Some(p) if cargo_so_is_fresh(&p) => p,
        other => {
            let fresh = build_fresh_cdylib();
            match other {
                Some(stale) => eprintln!(
                    "note: {} is stale (cargo does not rebuild cdylib targets during \
                     `cargo test`); testing freshly built {}",
                    stale.display(),
                    fresh.display()
                ),
                None => eprintln!(
                    "note: no cargo cdylib found; testing freshly built {}",
                    fresh.display()
                ),
            }
            fresh
        }
    })
    .clone()
}

/// The two `sieve` entry points, resolved once through `dlopen`/`dlsym`.
pub fn funcs() -> (SieveFn, SieveFn) {
    static F: OnceLock<(SieveFn, SieveFn)> = OnceLock::new();
    *F.get_or_init(|| unsafe {
        let c = Box::leak(Box::new(
            Library::new(c_so_path()).expect("dlopen C libSieve.so"),
        ));
        let r = Box::leak(Box::new(
            Library::new(rust_so_path()).expect("dlopen Rust libSieve.so"),
        ));
        let cf: Symbol<SieveFn> = c.get(b"sieve\0").expect("C .so must export `sieve`");
        let rf: Symbol<SieveFn> = r.get(b"sieve\0").expect("Rust .so must export `sieve`");
        (*cf, *rf)
    })
}

/// Same two symbols typed as taking a 64-bit argument, to probe what the callee
/// does with garbage in the upper half of the argument register.
pub fn funcs_wide() -> (SieveWideFn, SieveWideFn) {
    static F: OnceLock<(SieveWideFn, SieveWideFn)> = OnceLock::new();
    *F.get_or_init(|| unsafe {
        let c = Box::leak(Box::new(Library::new(c_so_path()).unwrap()));
        let r = Box::leak(Box::new(Library::new(rust_so_path()).unwrap()));
        let cf: Symbol<SieveWideFn> = c.get(b"sieve\0").unwrap();
        let rf: Symbol<SieveWideFn> = r.get(b"sieve\0").unwrap();
        (*cf, *rf)
    })
}

pub fn c_lib_exports_sieve() -> bool {
    unsafe {
        Library::new(c_so_path())
            .unwrap()
            .get::<SieveFn>(b"sieve\0")
            .is_ok()
    }
}

pub fn rust_lib_exports_sieve() -> bool {
    unsafe {
        Library::new(rust_so_path())
            .unwrap()
            .get::<SieveFn>(b"sieve\0")
            .is_ok()
    }
}

// ------------------------------------------------------------------ temp files

fn tmp_dir() -> PathBuf {
    std::env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}

fn tmp_file() -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    tmp_dir().join(format!(
        "sieve_diff_{}_{}.out",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ))
}

// ----------------------------------------------------- in-process fd capture

/// Runs `f` in *this* process with fd 1 redirected to a fresh regular file
/// (fully buffered stdout) and returns every byte written.
///
/// Unbounded: only use with inputs whose run length is small.
pub fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    let _g = lock();
    let path = tmp_file();
    let data;
    unsafe {
        let _ = std::io::stdout().flush();
        fflush(std::ptr::null_mut());
        let file = File::create(&path).expect("create capture file");
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");
        f();
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "restore dup2 failed");
        close(saved);
        drop(file);
        data = std::fs::read(&path).expect("read capture file");
    }
    let _ = std::fs::remove_file(&path);
    data
}

/// Runs `f` in *this* process with fd 1 redirected to a pipe drained by a helper
/// thread, stopping after `cap` bytes (the reader closes its end, so a runaway
/// implementation cannot fill memory or disk). Returns `(bytes, capped)`.
pub fn capture_pipe_capped<F: FnOnce()>(f: F, cap: usize) -> (Vec<u8>, bool) {
    let _g = lock();
    unsafe {
        let _ = std::io::stdout().flush();
        fflush(std::ptr::null_mut());
        let mut fds = [0i32; 2];
        assert_eq!(pipe(fds.as_mut_ptr()), 0, "pipe() failed");
        let (rd, wr) = (fds[0], fds[1]);
        let reader = std::thread::spawn(move || {
            let mut out = Vec::new();
            let mut buf = [0u8; 8192];
            let mut capped = false;
            loop {
                if out.len() >= cap {
                    capped = true;
                    break;
                }
                let n = read(rd, buf.as_mut_ptr(), buf.len());
                if n <= 0 {
                    break;
                }
                out.extend_from_slice(&buf[..n as usize]);
            }
            close(rd);
            (out, capped)
        });
        let saved = dup(1);
        assert!(saved >= 0);
        assert!(dup2(wr, 1) >= 0);
        f();
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0);
        close(saved);
        close(wr); // last writer gone -> reader sees EOF
        reader.join().expect("reader thread")
    }
}

/// Convenience: in-process capture of one `sieve(val)` call (regular file).
pub fn run(f: SieveFn, val: i32) -> Vec<u8> {
    capture(|| unsafe { f(val) })
}

// ------------------------------------------------------- forked, bounded runs

/// Where the forked child points fd 1 before running the payload.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Dest {
    /// A pipe drained by the parent (output captured, byte-capped).
    Pipe,
    /// `/dev/full` — every `write` fails with `ENOSPC`. No output captured.
    DevFull,
    /// fd 1 closed — every `write` fails with `EBADF`. No output captured.
    Closed,
}

#[derive(Clone, PartialEq, Eq)]
pub struct RunOut {
    pub bytes: Vec<u8>,
    /// The byte cap was reached and the child was killed.
    pub capped: bool,
    /// The wall-clock limit was reached and the child was killed.
    pub timed_out: bool,
    /// Raw `waitpid` status (exit code / signal), compared verbatim.
    pub status: i32,
}

impl std::fmt::Debug for RunOut {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RunOut {{ {} bytes, capped: {}, timed_out: {}, status: {} }}",
            self.bytes.len(),
            self.capped,
            self.timed_out,
            self.status
        )
    }
}

fn wait_readable(fd: i32, timeout_ms: i32) -> bool {
    let mut pfd = PollFdC {
        fd,
        events: POLLIN,
        revents: 0,
    };
    unsafe { poll(&mut pfd, 1, timeout_ms) > 0 }
}

/// Pre-fault libc's `stdout` buffer and the library's code pages so the forked
/// child never allocates.
fn warm(f: SieveFn) {
    let out = capture(|| unsafe { f(9) });
    assert!(!out.is_empty(), "warm-up capture produced nothing");
}

/// Runs `payload` in a forked child with the chosen stdout, bounded by `cap`
/// bytes and `timeout_ms` milliseconds.
pub fn fork_capture<F: FnOnce()>(dest: Dest, cap: usize, timeout_ms: i32, payload: F) -> RunOut {
    let _g = lock();
    unsafe {
        let _ = std::io::stdout().flush();
        fflush(std::ptr::null_mut());

        let mut fds = [-1i32; 2];
        if dest == Dest::Pipe {
            assert_eq!(pipe(fds.as_mut_ptr()), 0, "pipe() failed");
        }
        let (rd, wr) = (fds[0], fds[1]);

        let pid = fork();
        assert!(pid >= 0, "fork() failed");
        if pid == 0 {
            match dest {
                Dest::Pipe => {
                    close(rd);
                    dup2(wr, 1);
                    close(wr);
                }
                Dest::DevFull => match OpenOptions::new().write(true).open("/dev/full") {
                    Ok(fh) => {
                        dup2(fh.as_raw_fd(), 1);
                    }
                    Err(_) => _exit(97),
                },
                Dest::Closed => {
                    close(1);
                }
            }
            payload();
            fflush(std::ptr::null_mut());
            _exit(0);
        }

        let mut out = Vec::new();
        let mut capped = false;
        let mut timed_out = false;
        let start = std::time::Instant::now();

        if dest == Dest::Pipe {
            close(wr);
            let mut buf = [0u8; 65536];
            loop {
                if out.len() >= cap {
                    capped = true;
                    break;
                }
                let left = timeout_ms as i128 - start.elapsed().as_millis() as i128;
                if left <= 0 {
                    timed_out = true;
                    break;
                }
                if !wait_readable(rd, left.min(i32::MAX as i128) as i32) {
                    timed_out = true;
                    break;
                }
                let n = read(rd, buf.as_mut_ptr(), buf.len());
                if n <= 0 {
                    break; // EOF: child finished
                }
                out.extend_from_slice(&buf[..n as usize]);
            }
            close(rd);
        }

        // Reap, killing the child if it is still running.
        let mut st = 0i32;
        let mut reaped = false;
        if !capped && !timed_out {
            while start.elapsed().as_millis() < timeout_ms as u128 {
                let r = waitpid(pid, &mut st, WNOHANG);
                if r == pid {
                    reaped = true;
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            if !reaped {
                timed_out = true;
            }
        }
        if !reaped {
            kill(pid, SIGKILL);
            waitpid(pid, &mut st, 0);
        }
        if capped {
            out.truncate(cap);
        }
        RunOut {
            bytes: out,
            capped,
            timed_out,
            status: st,
        }
    }
}

/// Runs `sieve` over `vals` (in order) inside one forked child, capturing the
/// concatenated output.
pub fn run_vals(f: SieveFn, vals: &[i32], cap: usize) -> RunOut {
    warm(f);
    let v = vals.to_vec();
    fork_capture(Dest::Pipe, cap, TIMEOUT_MS, move || unsafe {
        for x in &v {
            f(*x);
        }
    })
}

/// Runs a single `sieve(val)` in a forked child.
pub fn run_one(f: SieveFn, val: i32, cap: usize) -> RunOut {
    run_vals(f, &[val], cap)
}

/// Captures at most `want` bytes of an (effectively unbounded) run.
pub fn child_prefix(f: SieveFn, val: i32, want: usize) -> Vec<u8> {
    let out = run_one(f, val, want);
    let mut b = out.bytes;
    b.truncate(want);
    b
}

/// Runs `sieve(val)` to completion in a forked child whose stdout is broken.
pub fn child_run(f: SieveFn, val: i32, dest: Dest) -> RunOut {
    warm(f);
    fork_capture(dest, CAP, TIMEOUT_MS, move || unsafe { f(val) })
}

// ------------------------------------------------------------------ assertions

fn show(bytes: &[u8]) -> String {
    let s = String::from_utf8_lossy(bytes);
    let all: Vec<&str> = s.lines().collect();
    if all.len() > 10 {
        format!("{:?} ... {:?} ({} lines)", &all[..5], &all[all.len() - 2..], all.len())
    } else {
        format!("{all:?}")
    }
}

fn diff_msg(label: &str, c: &RunOut, r: &RunOut) -> String {
    let first = c
        .bytes
        .iter()
        .zip(r.bytes.iter())
        .position(|(a, b)| a != b);
    format!(
        "{label} diverged\n  C   : {:?}\n        {}\n  Rust: {:?}\n        {}\n  first differing byte offset: {:?}",
        c,
        show(&c.bytes),
        r,
        show(&r.bytes),
        first
    )
}

/// Conservative upper bound on the bytes `sieve(val)` emits (C semantics).
fn est_bytes(val: i32) -> u64 {
    let lines: u64 = if val % 10 == 9 {
        1
    } else if val > 9 {
        10
    } else {
        (9i64 - val as i64) as u64 + 1
    };
    lines * 13
}

/// Differential assertion for one value: identical bytes, cap/timeout flags and
/// exit status from both `.so`s (forked, bounded).
pub fn assert_same(val: i32) {
    assert_same_all("single", [val]);
}

/// Differential assertion over many values. Values are grouped into batches that
/// share one forked child (fast) and the whole concatenated stream is compared;
/// on divergence the failing value is isolated and reported individually.
pub fn assert_same_all<I: IntoIterator<Item = i32>>(label: &str, vals: I) {
    let vals: Vec<i32> = vals.into_iter().collect();
    assert!(!vals.is_empty(), "empty value list for {label}");
    for &v in &vals {
        assert!(
            v <= i32::MAX - 8,
            "[{label}] sieve({v}) overflows/never terminates in C — compare a \
             prefix with child_prefix() instead"
        );
        assert!(
            est_bytes(v) < CAP as u64 / 2,
            "[{label}] sieve({v}) emits too much output for a full comparison — \
             use child_prefix()"
        );
    }
    let (c, r) = funcs();

    // group into batches of at most ~4 MiB of expected output
    let mut batches: Vec<Vec<i32>> = Vec::new();
    let mut cur: Vec<i32> = Vec::new();
    let mut cur_bytes = 0u64;
    for v in vals {
        let e = est_bytes(v);
        if !cur.is_empty() && (cur_bytes + e > 4 << 20 || cur.len() >= 64) {
            batches.push(std::mem::take(&mut cur));
            cur_bytes = 0;
        }
        cur.push(v);
        cur_bytes += e;
    }
    if !cur.is_empty() {
        batches.push(cur);
    }

    for batch in batches {
        let co = run_vals(c, &batch, CAP);
        let ro = run_vals(r, &batch, CAP);
        assert!(
            !co.bytes.is_empty(),
            "[{label}] harness broken: C produced no output for {batch:?}"
        );
        assert!(
            !co.capped && !co.timed_out,
            "[{label}] C run was cut short ({co:?}) for {batch:?}"
        );
        if co != ro {
            // isolate the offending value for a precise message
            for v in &batch {
                let c1 = run_one(c, *v, CAP);
                let r1 = run_one(r, *v, CAP);
                if c1 != r1 {
                    panic!("{}", diff_msg(&format!("[{label}] sieve({v})"), &c1, &r1));
                }
            }
            panic!(
                "{}",
                diff_msg(
                    &format!("[{label}] batch {batch:?} (differs only when concatenated)"),
                    &co,
                    &ro
                )
            );
        }
    }
}

// ------------------------------------------------------------------------- rng

/// SplitMix64 — deterministic, fixed seed per row for reproducibility.
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
    /// Uniform-ish in `[lo, hi]` (inclusive).
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        assert!(hi >= lo);
        let span = (hi - lo) as u64 + 1;
        lo + (self.next_u64() % span) as i64
    }
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        self.range(lo as i64, hi as i64) as i32
    }
}

/// Independent model of the C loop, used only to prove the capture harness is
/// really observing output (never to "correct" the C).
pub fn model(val: i32, max_lines: usize) -> Vec<u8> {
    let mut out = Vec::new();
    let mut v = val;
    let mut n = 0;
    loop {
        out.extend_from_slice(format!("{v}\n").as_bytes());
        n += 1;
        if v % 10 == 9 || n >= max_lines {
            break;
        }
        v = v.wrapping_add(1);
    }
    out
}
