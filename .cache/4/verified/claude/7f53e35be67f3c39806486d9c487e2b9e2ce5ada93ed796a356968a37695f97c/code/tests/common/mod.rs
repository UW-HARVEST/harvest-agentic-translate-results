//! Shared differential-test harness.
//!
//! Both implementations are exercised **only** through their shared libraries:
//! the C one built from `c_src/src/main.c` with `gcc -fPIC -shared`, the Rust
//! one being this crate's `cdylib`. Symbols are resolved with `libloading`, so
//! the `#[no_mangle]`/`extern "C"` export wrappers are part of what is tested.
//!
//! Every call happens in a freshly forked child process with fd 0 / fd 1
//! redirected, because
//!
//! * the captured stdout must not be polluted by the test harness's own output
//!   (other `#[test]`s run concurrently in the same process),
//! * `main` consumes stdin through process-wide stdio state (glibc's
//!   `FILE *stdin`, Rust's global `Stdin`) that cannot be reset between calls,
//! * a callee that dies from a signal must not take the test runner with it.

#![allow(dead_code)]

use std::fs;
use std::io::Write;
use std::os::raw::c_int;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// Paths / building
// ---------------------------------------------------------------------------

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn newer(src: &Path, dst: &Path) -> bool {
    match (fs::metadata(src), fs::metadata(dst)) {
        (Ok(s), Ok(d)) => match (s.modified(), d.modified()) {
            (Ok(s), Ok(d)) => s > d,
            _ => true,
        },
        _ => true,
    }
}

/// Absolute path to the C shared library, building it if necessary.
///
/// `c_src/CMakeLists.txt` builds an *executable*; the very same translation
/// unit is compiled here as a shared object with the same (default) flags so
/// that its two exported functions can be called through `dlopen`/`dlsym`.
pub fn c_so() -> PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let root = manifest_dir();
        let src = root.join("c_src/src/main.c");
        let out_dir = root.join("c_src/build");
        let out = out_dir.join("libcdriver.so");
        fs::create_dir_all(&out_dir).expect("create c_src/build");
        if !out.exists() || newer(&src, &out) {
            // Compile to a unique temporary name and rename, so parallel test
            // binaries never observe a half-written library.
            let tmp = out_dir.join(format!("libcdriver.{}.so.tmp", std::process::id()));
            let status = Command::new("gcc")
                .args(["-fPIC", "-shared", "-o"])
                .arg(&tmp)
                .arg(&src)
                .status()
                .expect("run gcc");
            assert!(status.success(), "gcc failed to build the C shared library");
            fs::rename(&tmp, &out).expect("install libcdriver.so");
        }
        out
    })
    .clone()
}

/// Absolute path to the C executable built exactly as `CMakeLists.txt` says.
pub fn c_bin() -> PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let root = manifest_dir();
        let build = root.join("c_src/build");
        let bin = build.join("driver");
        let src = root.join("c_src/src/main.c");
        if !bin.exists() || newer(&src, &bin) {
            fs::create_dir_all(&build).expect("create c_src/build");
            let cmake = Command::new("cmake")
                .arg("..")
                .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
                .current_dir(&build)
                .output()
                .expect("run cmake");
            assert!(cmake.status.success(), "cmake configure failed");
            let built = Command::new("cmake")
                .args(["--build", "."])
                .current_dir(&build)
                .output()
                .expect("run cmake --build");
            assert!(built.status.success(), "cmake --build failed");
        }
        bin
    })
    .clone()
}

/// Absolute path to this crate's `cdylib`, building it if necessary.
///
/// `cargo test` does not produce the `cdylib` artifact, so it is built here
/// with a *separate* `CARGO_TARGET_DIR` to avoid contending on the build lock
/// held by the outer `cargo test` invocation. The crate has no cargo features,
/// so there is nothing to forward.
pub fn rust_so() -> PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let root = manifest_dir();
        let target = root.join("target/so-build");
        let out = target.join("debug/libdriver.so");
        let stale = !out.exists()
            || ["src/lib.rs", "src/driver_impl.rs", "src/main.rs", "Cargo.toml"]
                .iter()
                .any(|f| newer(&root.join(f), &out));
        if stale {
            let output = Command::new(env!("CARGO"))
                .args(["build", "--offline", "--lib"])
                .current_dir(&root)
                .env("CARGO_TARGET_DIR", &target)
                .output()
                .expect("run cargo build --lib");
            assert!(
                output.status.success(),
                "failed to build the Rust cdylib:\n{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        assert!(out.exists(), "missing Rust cdylib at {}", out.display());
        out
    })
    .clone()
}

/// Absolute path to this crate's `driver` binary (built by `cargo test`).
pub fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

// ---------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------

pub struct Impls {
    pub c: Library,
    pub rust: Library,
}

/// Loads both shared libraries once per test process.
pub fn impls() -> &'static Impls {
    static IMPLS: OnceLock<Impls> = OnceLock::new();
    IMPLS.get_or_init(|| unsafe {
        let c = Library::new(c_so()).expect("dlopen C shared library");
        let rust = Library::new(rust_so()).expect("dlopen Rust shared library");
        Impls { c, rust }
    })
}

/// Which implementation to drive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    C,
    Rust,
}

impl Side {
    fn lib(self) -> &'static Library {
        match self {
            Side::C => &impls().c,
            Side::Rust => &impls().rust,
        }
    }
}

type DriverFn = unsafe extern "C" fn(c_int);
type MainFn = unsafe extern "C" fn() -> c_int;

/// Resolves `driver` **before** forking (`dlsym` allocates).
fn driver_sym(side: Side) -> DriverFn {
    unsafe {
        let s: Symbol<DriverFn> = side.lib().get(b"driver\0").expect("dlsym driver");
        *s
    }
}

/// Resolves `main` **before** forking.
fn main_sym(side: Side) -> MainFn {
    unsafe {
        let s: Symbol<MainFn> = side.lib().get(b"main\0").expect("dlsym main");
        *s
    }
}

fn fork_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    match LOCK.get_or_init(|| Mutex::new(())).lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

fn temp_path(tag: &str) -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    dir.join(format!(
        "driver-difftest-{}-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst),
        tag
    ))
}

// ---------------------------------------------------------------------------
// Forked execution
// ---------------------------------------------------------------------------

/// How the child's stdin is set up before the call.
#[derive(Clone, Copy, Debug)]
pub enum In<'a> {
    /// fd 0 reads from a regular file pre-filled with these bytes.
    Bytes(&'a [u8]),
    /// fd 0 is closed.
    Closed,
    /// fd 0 is left untouched (only valid when the callee does not read it).
    Inherit,
}

/// How the child's stdout is set up before the call.
#[derive(Clone, Copy, Debug)]
pub enum Out {
    /// fd 1 writes to a temporary file that is read back afterwards.
    File,
    /// fd 1 is closed.
    Closed,
    /// fd 1 is a pipe whose read end is closed immediately (broken pipe).
    BrokenPipe,
}

/// Result of one forked call.
#[derive(Debug, PartialEq, Eq)]
pub struct Run {
    pub stdout: Vec<u8>,
    /// Exit status of the child (the value returned by `main`), or `None` when
    /// the child was killed by a signal.
    pub status: Option<i32>,
    pub signal: Option<i32>,
}

/// Forks, wires up fd 0 / fd 1, runs `body` in the child and exits with the
/// value it returns.
fn fork_run<F>(stdin: In<'_>, stdout: Out, body: F) -> Run
where
    F: FnOnce() -> i32,
{
    let _guard = fork_lock();

    let out_path = temp_path("out");
    let in_path = temp_path("in");
    let out_file = match stdout {
        Out::File => Some(fs::File::create(&out_path).expect("create child stdout file")),
        _ => None,
    };
    let in_file = match stdin {
        In::Bytes(b) => {
            fs::write(&in_path, b).expect("write child stdin file");
            Some(fs::File::open(&in_path).expect("open child stdin file"))
        }
        _ => None,
    };
    let out_fd = out_file.as_ref().map(|f| f.as_raw_fd());
    let in_fd = in_file.as_ref().map(|f| f.as_raw_fd());

    // A broken pipe: the child inherits the write end, the read end is closed
    // in both processes before the child writes anything.
    let mut pipe_fds = [-1 as c_int; 2];
    if matches!(stdout, Out::BrokenPipe) {
        assert_eq!(unsafe { libc::pipe(pipe_fds.as_mut_ptr()) }, 0, "pipe failed");
    }

    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    unsafe { libc::fflush(std::ptr::null_mut()) };

    let pid = unsafe { libc::fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        // ---- child ----
        unsafe {
            match in_fd {
                Some(fd) => {
                    libc::dup2(fd, 0);
                }
                None => {
                    if matches!(stdin, In::Closed) {
                        libc::close(0);
                    }
                }
            }
            match stdout {
                Out::File => {
                    libc::dup2(out_fd.unwrap(), 1);
                }
                Out::Closed => {
                    libc::close(1);
                }
                Out::BrokenPipe => {
                    libc::dup2(pipe_fds[1], 1);
                    libc::close(pipe_fds[0]);
                    libc::close(pipe_fds[1]);
                }
            }
            let rc = body();
            // `_exit` skips libc's atexit flush, so flush explicitly.
            libc::fflush(std::ptr::null_mut());
            libc::_exit(rc);
        }
    }

    // ---- parent ----
    if matches!(stdout, Out::BrokenPipe) {
        unsafe {
            libc::close(pipe_fds[0]);
            libc::close(pipe_fds[1]);
        }
    }
    let mut wstatus: c_int = 0;
    let waited = unsafe { libc::waitpid(pid, &mut wstatus, 0) };
    assert_eq!(waited, pid, "waitpid failed");
    let (status, signal) = if libc::WIFEXITED(wstatus) {
        (Some(libc::WEXITSTATUS(wstatus)), None)
    } else if libc::WIFSIGNALED(wstatus) {
        (None, Some(libc::WTERMSIG(wstatus)))
    } else {
        (None, None)
    };
    let out = match stdout {
        Out::File => fs::read(&out_path).expect("read child stdout file"),
        _ => Vec::new(),
    };
    let _ = fs::remove_file(&out_path);
    let _ = fs::remove_file(&in_path);
    Run {
        stdout: out,
        status,
        signal,
    }
}

// ---------------------------------------------------------------------------
// `driver(int x)` — the lowest-level entry point
// ---------------------------------------------------------------------------

/// Calls `driver(x)` for every `x` in one forked child and returns the whole
/// stdout byte stream (one line per call, in order).
pub fn driver_batch(side: Side, xs: &[i32]) -> Vec<u8> {
    let f = driver_sym(side);
    fork_run(In::Inherit, Out::File, || {
        for &x in xs {
            unsafe { f(x as c_int) };
        }
        0
    })
    .stdout
}

/// Calls `driver(x)` once with a non-standard stdout setup.
pub fn driver_once(side: Side, x: i32, stdout: Out) -> Run {
    let f = driver_sym(side);
    fork_run(In::Inherit, stdout, || {
        unsafe { f(x as c_int) };
        0
    })
}

/// Asserts both `.so`s print byte-identical output for every `x`.
pub fn assert_driver_eq_all(xs: &[i32]) {
    let c = driver_batch(Side::C, xs);
    let r = driver_batch(Side::Rust, xs);
    if c != r {
        // Pin-point the first differing value for a useful failure message.
        let cl: Vec<&[u8]> = c.split(|&b| b == b'\n').collect();
        let rl: Vec<&[u8]> = r.split(|&b| b == b'\n').collect();
        for (i, x) in xs.iter().enumerate() {
            let a = cl.get(i).copied().unwrap_or(b"<missing>");
            let b = rl.get(i).copied().unwrap_or(b"<missing>");
            assert_eq!(
                String::from_utf8_lossy(a),
                String::from_utf8_lossy(b),
                "driver({x}) differs (call #{i})"
            );
        }
        panic!(
            "driver output differs but not line-wise:\nC   ={:?}\nRust={:?}",
            String::from_utf8_lossy(&c),
            String::from_utf8_lossy(&r)
        );
    }
}

/// Asserts both `.so`s print byte-identical output for `driver(x)`.
pub fn assert_driver_eq(x: i32) {
    assert_driver_eq_all(&[x]);
}

/// Asserts both `.so`s agree on `driver(x)` **and** that C printed exactly
/// `expected`, so a test pins the ground-truth bytes instead of only "equal".
pub fn assert_driver_eq_expect(x: i32, expected: &str) {
    let c = driver_batch(Side::C, &[x]);
    assert_eq!(
        String::from_utf8_lossy(&c),
        expected,
        "C ground truth changed for driver({x})"
    );
    assert_driver_eq(x);
}

// ---------------------------------------------------------------------------
// `main()` — the composed `scanf` -> `driver` -> `printf` pipeline
// ---------------------------------------------------------------------------

pub fn call_main(side: Side, stdin: In<'_>, stdout: Out) -> Run {
    let f = main_sym(side);
    fork_run(stdin, stdout, || unsafe { f() })
}

/// Asserts both `.so`s behave identically for `main()` on the given stdin.
pub fn assert_main_eq(input: &[u8]) {
    assert_main_eq_in(In::Bytes(input));
}

pub fn assert_main_eq_in(stdin: In<'_>) {
    let c = call_main(Side::C, stdin, Out::File);
    let r = call_main(Side::Rust, stdin, Out::File);
    assert_eq!(
        c,
        r,
        "main() with stdin {:?}: C={:?} Rust={:?}",
        describe(stdin),
        c,
        r
    );
}

/// Like [`assert_main_eq`] but also pins the exact expected C stdout.
pub fn assert_main_eq_expect(input: &[u8], expected_stdout: &str) {
    let c = call_main(Side::C, In::Bytes(input), Out::File);
    assert_eq!(
        String::from_utf8_lossy(&c.stdout),
        expected_stdout,
        "C ground truth changed for stdin {:?}",
        String::from_utf8_lossy(input)
    );
    assert_eq!(
        c.status,
        Some(0),
        "C exit status for stdin {:?}",
        String::from_utf8_lossy(input)
    );
    assert_main_eq(input);
}

fn describe(stdin: In<'_>) -> String {
    match stdin {
        In::Bytes(b) if b.len() <= 64 => format!("{:?}", String::from_utf8_lossy(b)),
        In::Bytes(b) => format!("<{} bytes>", b.len()),
        In::Closed => "<fd 0 closed>".to_string(),
        In::Inherit => "<inherited>".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seeds keep failures reproducible
// ---------------------------------------------------------------------------

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
        self.next_u64() as u32 as i32
    }

    pub fn next_i64(&mut self) -> i64 {
        self.next_u64() as i64
    }

    /// Uniform-ish value in `lo..=hi`.
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        debug_assert!(lo <= hi);
        let span = (hi as i128 - lo as i128 + 1) as u128;
        (lo as i128 + (self.next_u64() as u128 % span) as i128) as i64
    }
}
