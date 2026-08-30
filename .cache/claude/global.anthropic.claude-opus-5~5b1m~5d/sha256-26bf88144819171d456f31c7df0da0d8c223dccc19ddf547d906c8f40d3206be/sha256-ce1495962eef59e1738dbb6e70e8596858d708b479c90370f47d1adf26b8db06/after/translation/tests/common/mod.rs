// Differential-test harness shared by every integration test.
//
// Design notes
// ------------
// The C library's ONLY observable effect is the byte stream it writes to
// stdout via `printf("%d\n", val)`. To compare that stream exactly we must
// own fd 1, which is impossible to do safely inside a multi-threaded libtest
// process. So every measurement runs in a *fresh child process*:
//
//   parent test  --spawn-->  current_exe --exact common::child_worker
//                            (env: which .so, which values, where to write)
//
// The child dlopen()s the requested shared object with `libloading`, resolves
// the `sieve` symbol from *that* object's handle, points fd 1 at a file (or a
// FIFO), calls the symbol, flushes and `_exit(0)`s.
//
// Both the C `.so` and the Rust `.so` are driven through the *identical* code
// path, so any difference in the captured bytes is a real behavioural
// divergence. The Rust implementation is never called directly -- always
// through `dlsym` on `libSieve.so`, exactly as an external C consumer would.

#![allow(dead_code)]

use std::ffi::{CString, c_char, c_int, c_void};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// libc bits we need (declared directly so the crate needs no `libc` dep)
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn mkfifo(path: *const c_char, mode: u32) -> c_int;
    fn _exit(code: c_int) -> !;
}

const O_WRONLY: c_int = 1;
const O_CREAT: c_int = 0o100;
const O_TRUNC: c_int = 0o1000;

// ---------------------------------------------------------------------------
// Child-process protocol
// ---------------------------------------------------------------------------

/// Fully-qualified libtest name of the worker entry point below. Every
/// integration test file does `mod common;`, so the name is stable.
pub const CHILD_TEST_NAME: &str = "common::child_worker";

const E_LIB: &str = "SIEVE_CHILD_LIB";
const E_VALS: &str = "SIEVE_CHILD_VALS";
const E_OUT: &str = "SIEVE_CHILD_OUT";
const E_WIDE: &str = "SIEVE_CHILD_WIDE";
const E_CLOSE: &str = "SIEVE_CHILD_CLOSE_STDOUT";
const E_THREADS: &str = "SIEVE_CHILD_THREADS";

/// The worker. A no-op unless the parent set `SIEVE_CHILD_LIB`, so it is
/// harmless when libtest runs it as an ordinary test.
#[test]
fn child_worker() {
    let lib_path = match std::env::var(E_LIB) {
        Ok(v) => v,
        Err(_) => return, // ordinary test-suite run: nothing to do
    };

    let vals: Vec<i64> = std::fs::read_to_string(std::env::var(E_VALS).unwrap())
        .unwrap()
        .split_ascii_whitespace()
        .map(|t| t.parse::<i64>().unwrap())
        .collect();

    let wide = std::env::var(E_WIDE).is_ok();
    let close_stdout = std::env::var(E_CLOSE).is_ok();

    // dlopen BEFORE touching fd 1 so any loader diagnostics stay out of the
    // captured stream.
    let lib = unsafe { libloading::Library::new(&lib_path) }
        .unwrap_or_else(|e| panic!("dlopen({lib_path}) failed: {e}"));

    // Drain anything libtest already wrote to the real stdout.
    let _ = std::io::stdout().flush();
    unsafe { fflush(std::ptr::null_mut()) };

    if close_stdout {
        // ERRORS.md row 12: printf() failures are ignored by the C code.
        unsafe { close(1) };
    } else {
        let out = CString::new(std::env::var(E_OUT).unwrap()).unwrap();
        let fd = unsafe { open(out.as_ptr(), O_WRONLY | O_CREAT | O_TRUNC, 0o644 as c_int) };
        if fd < 0 {
            unsafe { _exit(91) };
        }
        if unsafe { dup2(fd, 1) } < 0 {
            unsafe { _exit(92) };
        }
        unsafe { close(fd) };
    }

    let threads: usize = std::env::var(E_THREADS)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    if threads > 0 {
        // CONFIGS.md row 22: concurrent callers. `sieve` has no static state,
        // so the *multiset* of emitted lines is deterministic (glibc holds the
        // stream lock for the whole printf, so lines are never torn).
        let sym: libloading::Symbol<unsafe extern "C" fn(c_int)> =
            unsafe { lib.get(b"sieve\0") }.expect("no `sieve` symbol");
        let fp: unsafe extern "C" fn(c_int) = *sym; // fn pointers are Send
        let chunks: Vec<Vec<i64>> = (0..threads)
            .map(|t| {
                vals.iter()
                    .enumerate()
                    .filter(|(i, _)| i % threads == t)
                    .map(|(_, &v)| v)
                    .collect()
            })
            .collect();
        let handles: Vec<_> = chunks
            .into_iter()
            .map(|chunk| {
                std::thread::spawn(move || {
                    for v in chunk {
                        unsafe { fp(v as i32 as c_int) };
                    }
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
    } else if wide {
        // ERRORS.md row 10: call through a 64-bit prototype so the upper half
        // of %rdi carries garbage the callee must ignore.
        let f: libloading::Symbol<unsafe extern "C" fn(i64)> =
            unsafe { lib.get(b"sieve\0") }.expect("no `sieve` symbol");
        for &v in &vals {
            unsafe { f(v) };
        }
    } else {
        let f: libloading::Symbol<unsafe extern "C" fn(c_int)> =
            unsafe { lib.get(b"sieve\0") }.expect("no `sieve` symbol");
        for &v in &vals {
            unsafe { f(v as i32 as c_int) };
        }
    }

    unsafe { fflush(std::ptr::null_mut()) };
    unsafe { _exit(0) };
}

// ---------------------------------------------------------------------------
// Library locations
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

/// The C ground-truth shared library, built on demand via CMake.
pub fn c_lib() -> PathBuf {
    let root = repo_root();
    let so = root.join("c_src/build/libSieve.so");
    if !so.exists() {
        let build = root.join("c_src/build");
        std::fs::create_dir_all(&build).unwrap();
        let ok = Command::new("cmake")
            .current_dir(&build)
            .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
            .status()
            .expect("cmake not available")
            .success()
            && Command::new("cmake")
                .current_dir(&build)
                .args(["--build", "."])
                .status()
                .unwrap()
                .success();
        assert!(ok, "failed to build the C library with cmake");
    }
    assert!(so.exists(), "missing C library at {}", so.display());
    so
}

/// The Rust cdylib under test.
///
/// IMPORTANT: `cargo test` does *not* rebuild a `crate-type = ["cdylib"]`
/// library, because integration tests cannot link against a cdylib -- so
/// `target/<profile>/libSieve.so` can be arbitrarily stale (and a stale
/// artifact makes every differential test silently vacuous: it would keep
/// comparing the C library against an old, possibly correct, binary while the
/// current sources are broken).
///
/// So we build the cdylib ourselves, from the current sources, into a
/// dedicated `CARGO_TARGET_DIR` (which has its own lock, so it cannot
/// deadlock against the outer `cargo test`), and then assert the artifact is
/// newer than every source input. Built once per test process.
pub fn rust_lib() -> PathBuf {
    static SO: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    SO.get_or_init(build_rust_lib).clone()
}

fn build_rust_lib() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let release = !cfg!(debug_assertions);
    let profile_dir = if release { "release" } else { "debug" };
    let target_dir = manifest.join("target/difftest");

    let mut cmd = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
    cmd.current_dir(&manifest)
        .arg("build")
        .arg("--offline")
        .arg("--lib")
        .env("CARGO_TARGET_DIR", &target_dir)
        // Do not inherit the outer test run's flags/profile settings.
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_BUILD_TARGET_DIR")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if release {
        cmd.arg("--release");
    }
    let out = cmd.output().expect("failed to run `cargo build` for the cdylib");
    assert!(
        out.status.success(),
        "rebuilding the Rust cdylib failed:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let so = target_dir.join(profile_dir).join("libSieve.so");
    assert!(
        so.exists(),
        "cargo build did not produce {} (crate-type/lib name changed?)",
        so.display()
    );

    // Freshness gate: the artifact must be at least as new as its sources.
    let so_time = std::fs::metadata(&so).unwrap().modified().unwrap();
    for src in ["src/lib.rs", "Cargo.toml"] {
        let p = manifest.join(src);
        let t = std::fs::metadata(&p).unwrap().modified().unwrap();
        assert!(
            so_time >= t,
            "{} is STALE: {} is newer. Differential tests would compare against \
             out-of-date code.",
            so.display(),
            p.display()
        );
    }
    so
}

// ---------------------------------------------------------------------------
// Spawning
// ---------------------------------------------------------------------------

fn tmp_dir() -> PathBuf {
    let d = std::env::temp_dir().join("sieve-difftest");
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn unique(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{}-{}-{}", tag, std::process::id(), n))
}

fn write_vals(vals: &[i64]) -> PathBuf {
    let p = unique("vals");
    let mut s = String::with_capacity(vals.len() * 8);
    for v in vals {
        s.push_str(&v.to_string());
        s.push('\n');
    }
    std::fs::write(&p, s).unwrap();
    p
}

fn spawn_child(lib: &Path, vals_file: &Path, out: Option<&Path>, wide: bool, closed: bool) -> Child {
    spawn_child_full(lib, vals_file, out, wide, closed, 0)
}

fn spawn_child_full(
    lib: &Path,
    vals_file: &Path,
    out: Option<&Path>,
    wide: bool,
    closed: bool,
    threads: usize,
) -> Child {
    let mut cmd = Command::new(std::env::current_exe().unwrap());
    cmd.args([CHILD_TEST_NAME, "--exact", "--quiet", "--test-threads=1"])
        .env(E_LIB, lib)
        .env(E_VALS, vals_file)
        .env_remove(E_OUT)
        .env_remove(E_WIDE)
        .env_remove(E_CLOSE)
        .env_remove(E_THREADS)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    if let Some(o) = out {
        cmd.env(E_OUT, o);
    }
    if wide {
        cmd.env(E_WIDE, "1");
    }
    if closed {
        cmd.env(E_CLOSE, "1");
    }
    if threads > 0 {
        cmd.env(E_THREADS, threads.to_string());
    }
    cmd.spawn().expect("failed to spawn child worker")
}

fn wait_bounded(child: &mut Child, secs: u64, ctx: &str) -> std::process::ExitStatus {
    let deadline = Instant::now() + Duration::from_secs(secs);
    loop {
        if let Some(st) = child.try_wait().unwrap() {
            return st;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("child timed out after {secs}s ({ctx})");
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn drain_stderr(child: &mut Child) -> String {
    let mut s = String::new();
    if let Some(mut e) = child.stderr.take() {
        let _ = e.read_to_string(&mut s);
    }
    s
}

// ---------------------------------------------------------------------------
// Public measurement API
// ---------------------------------------------------------------------------

/// Run `sieve` once per element of `vals`, in order, in a child process whose
/// stdout is a regular file. Returns the raw bytes produced.
pub fn run_batch(lib: &Path, vals: &[i64]) -> Vec<u8> {
    run_batch_inner(lib, vals, false)
}

/// Same, but calling through a 64-bit prototype (garbage in the upper half of
/// the argument register).
pub fn run_batch_wide(lib: &Path, vals: &[i64]) -> Vec<u8> {
    run_batch_inner(lib, vals, true)
}

fn run_batch_inner(lib: &Path, vals: &[i64], wide: bool) -> Vec<u8> {
    let vf = write_vals(vals);
    let out = unique("out");
    std::fs::write(&out, b"").unwrap();
    let mut child = spawn_child(lib, &vf, Some(&out), wide, false);
    let st = wait_bounded(&mut child, 300, &format!("batch on {}", lib.display()));
    let err = drain_stderr(&mut child);
    assert!(
        st.success(),
        "child for {} exited with {:?}\nstderr:\n{}",
        lib.display(),
        st.code(),
        err
    );
    let bytes = std::fs::read(&out).unwrap();
    let _ = std::fs::remove_file(&vf);
    let _ = std::fs::remove_file(&out);
    bytes
}

/// Run `sieve(val)` in a child whose stdout is a FIFO (non-seekable pipe),
/// draining it concurrently. Returns the bytes read until the child exits.
pub fn run_batch_through_fifo(lib: &Path, vals: &[i64]) -> Vec<u8> {
    let vf = write_vals(vals);
    let fifo = unique("fifo");
    let cpath = CString::new(fifo.to_str().unwrap()).unwrap();
    let _ = std::fs::remove_file(&fifo);
    assert_eq!(unsafe { mkfifo(cpath.as_ptr(), 0o600) }, 0, "mkfifo failed");

    // The reader must open concurrently with the child's writer open(2),
    // because both block until the other side appears.
    let reader_path = fifo.clone();
    let reader = std::thread::spawn(move || {
        let mut f = std::fs::File::open(&reader_path).unwrap();
        let mut buf = Vec::new();
        f.read_to_end(&mut buf).unwrap();
        buf
    });

    let mut child = spawn_child(lib, &vf, Some(&fifo), false, false);
    let st = wait_bounded(&mut child, 300, &format!("fifo on {}", lib.display()));
    let err = drain_stderr(&mut child);
    assert!(
        st.success(),
        "fifo child for {} exited {:?}\nstderr:\n{}",
        lib.display(),
        st.code(),
        err
    );
    let bytes = reader.join().unwrap();
    let _ = std::fs::remove_file(&vf);
    let _ = std::fs::remove_file(&fifo);
    bytes
}

/// For inputs whose loop runs ~2^31 times: start the child, wait until it has
/// written `n` bytes, kill it, and return exactly the first `n` bytes.
/// stdout ordering makes a prefix comparison sound.
pub fn run_prefix(lib: &Path, val: i64, n: usize) -> Vec<u8> {
    let vf = write_vals(&[val]);
    let out = unique("pfx");
    std::fs::write(&out, b"").unwrap();
    let mut child = spawn_child(lib, &vf, Some(&out), false, false);

    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        let len = std::fs::metadata(&out).map(|m| m.len() as usize).unwrap_or(0);
        if len >= n {
            break;
        }
        if let Some(st) = child.try_wait().unwrap() {
            // Terminated on its own before producing n bytes.
            let err = drain_stderr(&mut child);
            panic!(
                "child for {} exited early ({:?}) with only {} of {} bytes\nstderr:\n{}",
                lib.display(),
                st.code(),
                len,
                n,
                err
            );
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("prefix child for {} too slow", lib.display());
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    let _ = child.kill();
    let _ = child.wait();

    let mut bytes = std::fs::read(&out).unwrap();
    bytes.truncate(n);
    let _ = std::fs::remove_file(&vf);
    let _ = std::fs::remove_file(&out);
    bytes
}

/// Run the batch spread across `threads` concurrent callers. Line *order* is
/// nondeterministic, so callers must compare the sorted line multiset.
pub fn run_batch_threaded(lib: &Path, vals: &[i64], threads: usize) -> Vec<u8> {
    let vf = write_vals(vals);
    let out = unique("thr");
    std::fs::write(&out, b"").unwrap();
    let mut child = spawn_child_full(lib, &vf, Some(&out), false, false, threads);
    let st = wait_bounded(&mut child, 300, "threaded");
    let err = drain_stderr(&mut child);
    assert!(
        st.success(),
        "threaded child for {} exited {:?}\nstderr:\n{}",
        lib.display(),
        st.code(),
        err
    );
    let bytes = std::fs::read(&out).unwrap();
    let _ = std::fs::remove_file(&vf);
    let _ = std::fs::remove_file(&out);
    bytes
}

/// Run with fd 1 *closed*: every `printf` fails. Returns the exit status code.
pub fn run_with_closed_stdout(lib: &Path, vals: &[i64]) -> Option<i32> {
    let vf = write_vals(vals);
    let mut child = spawn_child(lib, &vf, None, false, true);
    let st = wait_bounded(&mut child, 120, "closed-stdout");
    let _ = drain_stderr(&mut child);
    let _ = std::fs::remove_file(&vf);
    st.code()
}

// ---------------------------------------------------------------------------
// Assertions
// ---------------------------------------------------------------------------

fn describe(vals: &[i64]) -> String {
    if vals.len() <= 12 {
        format!("{vals:?}")
    } else {
        format!("{:?}.. ({} values)", &vals[..12], vals.len())
    }
}

fn first_difference(a: &[u8], b: &[u8]) -> String {
    let at = a.iter().zip(b.iter()).position(|(x, y)| x != y);
    let idx = at.unwrap_or(a.len().min(b.len()));
    let lo = idx.saturating_sub(80);
    let hi_a = (idx + 80).min(a.len());
    let hi_b = (idx + 80).min(b.len());
    format!(
        "lens: C={} rust={}; first difference at byte {}\n  C   ...{:?}\n  rust...{:?}",
        a.len(),
        b.len(),
        idx,
        String::from_utf8_lossy(&a[lo..hi_a]),
        String::from_utf8_lossy(&b[lo..hi_b]),
    )
}

/// Core differential assertion: identical stdout bytes from C and Rust.
pub fn assert_same(vals: &[i64]) -> Vec<u8> {
    let c = run_batch(&c_lib(), vals);
    let r = run_batch(&rust_lib(), vals);
    assert!(
        c == r,
        "stdout mismatch for {}\n{}",
        describe(vals),
        first_difference(&c, &r)
    );
    c
}

pub fn assert_same_wide(vals: &[i64]) -> Vec<u8> {
    let c = run_batch_wide(&c_lib(), vals);
    let r = run_batch_wide(&rust_lib(), vals);
    assert!(
        c == r,
        "stdout mismatch (64-bit prototype) for {}\n{}",
        describe(vals),
        first_difference(&c, &r)
    );
    c
}

pub fn assert_same_fifo(vals: &[i64]) -> Vec<u8> {
    let c = run_batch_through_fifo(&c_lib(), vals);
    let r = run_batch_through_fifo(&rust_lib(), vals);
    assert!(
        c == r,
        "stdout-through-FIFO mismatch for {}\n{}",
        describe(vals),
        first_difference(&c, &r)
    );
    c
}

fn sorted_lines(b: &[u8]) -> Vec<&[u8]> {
    let mut v: Vec<&[u8]> = b.split(|&c| c == b'\n').filter(|l| !l.is_empty()).collect();
    v.sort_unstable();
    v
}

/// Concurrent-callers comparison: identical *multiset* of output lines
/// (interleaving order between threads is not deterministic, but the set of
/// lines is, because `sieve` keeps no state and glibc never tears a printf).
pub fn assert_same_multiset_threaded(vals: &[i64], threads: usize) {
    let c = run_batch_threaded(&c_lib(), vals, threads);
    let r = run_batch_threaded(&rust_lib(), vals, threads);
    let (cl, rl) = (sorted_lines(&c), sorted_lines(&r));
    assert_eq!(
        cl.len(),
        rl.len(),
        "line-count mismatch with {threads} threads: C={} rust={}",
        cl.len(),
        rl.len()
    );
    assert!(cl == rl, "line multiset mismatch with {threads} concurrent callers");

    // and both must equal the single-threaded reference multiset
    let model = expected(vals);
    assert!(
        sorted_lines(&model) == cl,
        "concurrent output differs from the sequential reference multiset"
    );
}

pub fn assert_same_prefix(val: i64, n: usize) -> Vec<u8> {
    let c = run_prefix(&c_lib(), val, n);
    let r = run_prefix(&rust_lib(), val, n);
    assert_eq!(c.len(), n);
    assert_eq!(r.len(), n);
    assert!(
        c == r,
        "stdout prefix mismatch for sieve({val})\n{}",
        first_difference(&c, &r)
    );
    c
}

/// Reference model of the C loop, used to sanity-check that the differential
/// tests are actually observing the documented behaviour (both sides agreeing
/// on nothing would otherwise pass).
pub fn expected(vals: &[i64]) -> Vec<u8> {
    let mut out = Vec::new();
    for &v in vals {
        let mut val = v as i32;
        loop {
            out.extend_from_slice(format!("{val}\n").as_bytes());
            if val % 10 == 9 {
                break;
            }
            val = val.wrapping_add(1);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Deterministic RNG (PCG32) -- fixed seeds keep every row reproducible.
// ---------------------------------------------------------------------------

pub struct Pcg32 {
    state: u64,
    inc: u64,
}

impl Pcg32 {
    pub fn new(seed: u64) -> Self {
        let mut r = Pcg32 {
            state: 0,
            inc: (seed << 1) | 1,
        };
        r.next_u32();
        r.state = r.state.wrapping_add(seed);
        r.next_u32();
        r
    }

    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old
            .wrapping_mul(6364136223846793005)
            .wrapping_add(self.inc);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    /// Uniform in `[lo, hi]` inclusive.
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        assert!(hi >= lo);
        let span = (hi - lo) as u64 + 1;
        if span == 0 {
            return self.next_u32() as i64;
        }
        lo + (self.next_u32() as u64 % span) as i64
    }
}
