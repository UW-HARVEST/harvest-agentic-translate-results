//! Shared differential-testing harness.
//!
//! Everything here goes through the *shared objects*: the C `.so` built from
//! `c_src/src/main.c` and the Rust `cdylib` built from this crate are both
//! `dlopen`ed with `libloading` and their `driver` / `main` symbols are called
//! through the C ABI.  No Rust function of the crate under test is ever called
//! directly, so the `#[no_mangle]` export wrappers are part of what is verified.

#![allow(dead_code)]

use std::ffi::c_void;
use std::fs;
use std::io::{Read, Write};
use std::os::unix::io::AsRawFd;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use libloading::{Library, Symbol};

extern "C" {
    fn dup(oldfd: i32) -> i32;
    fn dup2(oldfd: i32, newfd: i32) -> i32;
    fn close(fd: i32) -> i32;
    fn pipe(fds: *mut i32) -> i32;
    fn fflush(stream: *mut c_void) -> i32;
}

// ---------------------------------------------------------------------------
// C ABI of the library under test (identical for both implementations)
// ---------------------------------------------------------------------------

/// `void driver(int x)`
pub type DriverFn = unsafe extern "C" fn(std::os::raw::c_int);
/// `int main(void)`
pub type MainFn = unsafe extern "C" fn() -> std::os::raw::c_int;

// ---------------------------------------------------------------------------
// Paths / artifact building
// ---------------------------------------------------------------------------

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `target/<profile>` — derived from the running test binary
/// (`target/<profile>/deps/<test>-<hash>`).
pub fn profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(Path::parent)
        .expect("target/<profile>")
        .to_path_buf()
}

/// Scratch directory for generated artifacts and temp files.
pub fn scratch_dir() -> PathBuf {
    let d = profile_dir().join("difftest");
    fs::create_dir_all(&d).expect("create scratch dir");
    d
}

/// Newest modification time across everything the Rust artifacts are built from.
fn newest_source_mtime() -> std::time::SystemTime {
    fn walk(dir: &Path, newest: &mut std::time::SystemTime) {
        if let Ok(rd) = fs::read_dir(dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, newest);
                } else if p.extension().map(|x| x == "rs").unwrap_or(false) {
                    if let Ok(m) = e.metadata().and_then(|m| m.modified()) {
                        if m > *newest {
                            *newest = m;
                        }
                    }
                }
            }
        }
    }
    let mut newest = std::time::SystemTime::UNIX_EPOCH;
    walk(&manifest_dir().join("src"), &mut newest);
    if let Ok(m) = fs::metadata(manifest_dir().join("Cargo.toml")).and_then(|m| m.modified()) {
        if m > newest {
            newest = m;
        }
    }
    newest
}

/// `cargo test` compiles the `rlib` and the binaries of the package, but it does
/// **not** produce the `cdylib` artifact — so a plain `cargo test` would happily
/// run every `.so` comparison against a stale `libdriver.so` left over from an
/// earlier build and report success.  Build the library targets explicitly (once
/// per test binary) and then refuse to continue if the artifacts are still older
/// than the sources.
fn rust_artifacts() -> &'static (PathBuf, PathBuf) {
    static ARTIFACTS: OnceLock<(PathBuf, PathBuf)> = OnceLock::new();
    ARTIFACTS.get_or_init(|| {
        let dir = profile_dir();
        let so = dir.join("libdriver.so");
        let exe = dir.join("driver");

        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
        let mut cmd = Command::new(cargo);
        // Captured, not inherited: cargo's progress lines go to stderr and would
        // otherwise interleave with the harness' own report.
        cmd.args(["build", "--offline"])
            .current_dir(manifest_dir())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if dir.file_name().map(|n| n == "release").unwrap_or(false) {
            cmd.arg("--release");
        }
        // Lets the runner script forward the feature selection under test.
        if let Ok(extra) = std::env::var("DIFFTEST_CARGO_ARGS") {
            for a in extra.split_whitespace() {
                cmd.arg(a);
            }
        }
        let out = cmd.output().expect("spawn cargo build for the cdylib");
        assert!(
            out.status.success(),
            "`cargo build` for the cdylib/bin failed:\n{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );

        let newest = newest_source_mtime();
        for p in [&so, &exe] {
            assert!(p.exists(), "Rust artifact missing: {p:?}");
            let m = fs::metadata(p)
                .and_then(|m| m.modified())
                .expect("artifact mtime");
            assert!(
                m >= newest,
                "STALE Rust artifact {p:?}: it is older than the sources in src/. \
                 Differential results against a stale artifact are meaningless. \
                 Run `cargo build` (or ./run_difftests.sh) first."
            );
        }
        (so, exe)
    })
}

/// The Rust cdylib (`libdriver.so`), guaranteed freshly built.
pub fn rust_so() -> PathBuf {
    rust_artifacts().0.clone()
}

/// The Rust executable (`driver`), guaranteed freshly built.
pub fn rust_exe() -> PathBuf {
    rust_artifacts().1.clone()
}

fn c_artifacts() -> &'static (PathBuf, PathBuf) {
    static ARTIFACTS: OnceLock<(PathBuf, PathBuf)> = OnceLock::new();
    ARTIFACTS.get_or_init(|| {
        let src = manifest_dir().join("c_src/src/main.c");
        assert!(src.exists(), "C source missing at {src:?}");
        // Built outside c_src/ so that nothing under c_src/ is touched.
        let out = profile_dir().join("c_ref");
        fs::create_dir_all(&out).expect("create c_ref dir");
        let so = out.join("libdriver_c.so");
        let exe = out.join("driver_c");

        let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());

        // Shared object: same single translation unit, position independent.
        let st = Command::new(&cc)
            .args(["-shared", "-fPIC", "-o"])
            .arg(&so)
            .arg(&src)
            .status()
            .expect("spawn cc for shared object");
        assert!(st.success(), "cc failed to build {so:?}");

        // Executable: exactly what `add_executable(driver src/main.c)` does
        // (CMAKE_BUILD_TYPE is unset in c_src/CMakeLists.txt → no -O flag).
        let st = Command::new(&cc)
            .arg("-o")
            .arg(&exe)
            .arg(&src)
            .status()
            .expect("spawn cc for executable");
        assert!(st.success(), "cc failed to build {exe:?}");

        (so, exe)
    })
}

/// The C shared object built from `c_src/src/main.c`.
pub fn c_so() -> PathBuf {
    c_artifacts().0.clone()
}

/// The C executable built from `c_src/src/main.c`.
pub fn c_exe() -> PathBuf {
    c_artifacts().1.clone()
}

// ---------------------------------------------------------------------------
// Loaded libraries (loaded once per process; never `dlclose`d)
// ---------------------------------------------------------------------------

fn load_once(slot: &'static OnceLock<Library>, path: PathBuf) -> &'static Library {
    slot.get_or_init(|| unsafe { Library::new(&path).expect("dlopen") })
}

pub fn c_lib() -> &'static Library {
    static L: OnceLock<Library> = OnceLock::new();
    load_once(&L, c_so())
}

pub fn rust_lib() -> &'static Library {
    static L: OnceLock<Library> = OnceLock::new();
    load_once(&L, rust_so())
}

pub fn driver_sym(lib: &'static Library) -> Symbol<'static, DriverFn> {
    unsafe { lib.get(b"driver\0").expect("`driver` symbol") }
}

pub fn main_sym(lib: &'static Library) -> Symbol<'static, MainFn> {
    unsafe { lib.get(b"main\0").expect("`main` symbol") }
}

// ---------------------------------------------------------------------------
// fd-1 capture (serialised: fd 1 is process-global state)
// ---------------------------------------------------------------------------

fn fd1_lock() -> &'static Mutex<()> {
    static M: OnceLock<Mutex<()>> = OnceLock::new();
    M.get_or_init(|| Mutex::new(()))
}

fn unique_name(tag: &str) -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    scratch_dir().join(format!("{tag}-{}-{}", std::process::id(), n))
}

/// Runs `f` with fd 1 pointing at a fresh regular file and returns everything
/// written to it (after `fflush(NULL)`, so C's `FILE*` buffer is included).
fn with_stdout_file<R>(f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    let guard = fd1_lock().lock().unwrap_or_else(|e| e.into_inner());
    let path = unique_name("stdout");
    let file = fs::File::create(&path).expect("create capture file");
    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 failed");
    let r = f();
    unsafe {
        fflush(std::ptr::null_mut());
        dup2(saved, 1);
        close(saved);
    }
    drop(file);
    drop(guard);
    let bytes = fs::read(&path).expect("read capture file");
    let _ = fs::remove_file(&path);
    (r, bytes)
}

/// Runs `f` with fd 1 pointing at the write end of a pipe and returns
/// everything written to it.  The caller must keep the produced output well
/// under the pipe capacity (64 KiB on Linux) to avoid blocking.
fn with_stdout_pipe<R>(f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    let guard = fd1_lock().lock().unwrap_or_else(|e| e.into_inner());
    let mut fds = [0i32; 2];
    assert_eq!(unsafe { pipe(fds.as_mut_ptr()) }, 0, "pipe() failed");
    let (rd, wr) = (fds[0], fds[1]);
    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(wr, 1) } >= 0, "dup2 failed");
    let r = f();
    unsafe {
        fflush(std::ptr::null_mut());
        dup2(saved, 1);
        close(saved);
        close(wr);
    }
    drop(guard);
    let mut out = Vec::new();
    {
        let mut rf = unsafe { <fs::File as std::os::unix::io::FromRawFd>::from_raw_fd(rd) };
        rf.read_to_end(&mut out).expect("read pipe");
    }
    (r, out)
}

// ---------------------------------------------------------------------------
// `driver` through the FFI boundary
// ---------------------------------------------------------------------------

/// Calls `driver(x)` for every `x` in `xs`, in order, in one process, capturing
/// the concatenated bytes written to a regular-file stdout.
pub fn driver_out_file(lib: &'static Library, xs: &[i32]) -> Vec<u8> {
    let f = driver_sym(lib);
    let (_, bytes) = with_stdout_file(|| {
        for &x in xs {
            unsafe { f(x) };
        }
    });
    bytes
}

/// Same, but stdout is a pipe.
pub fn driver_out_pipe(lib: &'static Library, xs: &[i32]) -> Vec<u8> {
    let f = driver_sym(lib);
    let (_, bytes) = with_stdout_pipe(|| {
        for &x in xs {
            unsafe { f(x) };
        }
    });
    bytes
}

/// Differential check for `driver`: identical bytes from both `.so`s.
#[track_caller]
pub fn assert_driver_eq(xs: &[i32]) {
    let c = driver_out_file(c_lib(), xs);
    let r = driver_out_file(rust_lib(), xs);
    assert_bytes_eq(&c, &r, &format!("driver(file) xs={:?}", Trunc(xs)));
}

#[track_caller]
pub fn assert_driver_eq_pipe(xs: &[i32]) {
    let c = driver_out_pipe(c_lib(), xs);
    let r = driver_out_pipe(rust_lib(), xs);
    assert_bytes_eq(&c, &r, &format!("driver(pipe) xs={:?}", Trunc(xs)));
}

// ---------------------------------------------------------------------------
// `main` through the FFI boundary (one invocation per child process, because
// both implementations keep global buffered-stdin state, exactly like C does)
// ---------------------------------------------------------------------------

pub const ENV_LIB: &str = "DIFFTEST_HARNESS_LIB";
pub const ENV_OUT: &str = "DIFFTEST_HARNESS_OUT";
pub const ENV_CLOSE_STDIN: &str = "DIFFTEST_HARNESS_CLOSE_STDIN";
pub const ENV_CLOSE_STDOUT: &str = "DIFFTEST_HARNESS_CLOSE_STDOUT";
pub const HARNESS_TEST: &str = "common::child_harness";

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum StdinKind {
    Pipe,
    File,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    pub stdout: Vec<u8>,
    pub code: Option<i32>,
    pub signal: Option<i32>,
}

/// The body of the re-executed child: loads the requested `.so`, points fd 1 at
/// the requested file, calls the exported `main`, and exits with its result.
///
/// Returns immediately when the harness environment variables are absent, i.e.
/// when this is just a normal test run.
pub fn child_harness_body() {
    let lib = match std::env::var(ENV_LIB) {
        Ok(v) => v,
        Err(_) => return,
    };
    let out = std::env::var(ENV_OUT).expect("harness output path");
    {
        let file = fs::File::create(&out).expect("create harness output");
        assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 failed");
    }
    let lib = unsafe { Library::new(&lib).expect("dlopen") };
    let m: Symbol<MainFn> = unsafe { lib.get(b"main\0").expect("`main` symbol") };
    // Applied *after* `dlopen` so both implementations see exactly the same
    // descriptor state at the moment `main` is entered.
    if std::env::var(ENV_CLOSE_STDIN).is_ok() {
        unsafe { close(0) };
    }
    if std::env::var(ENV_CLOSE_STDOUT).is_ok() {
        unsafe { close(1) };
    }
    let code = unsafe { m() };
    unsafe { fflush(std::ptr::null_mut()) };
    std::process::exit(code);
}

/// Extra descriptor manipulations applied inside the child right before `main`
/// is entered.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct MainSoOpts {
    pub close_stdin: bool,
    pub close_stdout: bool,
}

/// Calls the `main` symbol exported by `lib` in a fresh child process, feeding
/// it `input` on stdin.
pub fn run_main_via_so(lib: &Path, input: &[u8], stdin_kind: StdinKind) -> Run {
    run_main_via_so_opts(lib, input, stdin_kind, MainSoOpts::default())
}

pub fn run_main_via_so_opts(
    lib: &Path,
    input: &[u8],
    stdin_kind: StdinKind,
    opts: MainSoOpts,
) -> Run {
    let out_path = unique_name("harness-out");
    let mut cmd = Command::new(std::env::current_exe().expect("current_exe"));
    cmd.env(ENV_LIB, lib)
        .env(ENV_OUT, &out_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if opts.close_stdin {
        cmd.env(ENV_CLOSE_STDIN, "1");
    }
    if opts.close_stdout {
        cmd.env(ENV_CLOSE_STDOUT, "1");
    }

    let in_file_path;
    match stdin_kind {
        StdinKind::Pipe => {
            cmd.stdin(Stdio::piped());
            in_file_path = None;
        }
        StdinKind::File => {
            let p = unique_name("harness-in");
            fs::write(&p, input).expect("write harness input");
            cmd.stdin(Stdio::from(fs::File::open(&p).expect("open harness input")));
            in_file_path = Some(p);
        }
    }

    let mut child = cmd.spawn().expect("spawn harness child");
    if stdin_kind == StdinKind::Pipe {
        let mut si = child.stdin.take().expect("child stdin");
        // The child may stop reading; a broken pipe here is not an error.
        let _ = si.write_all(input);
        drop(si);
    }
    let status = child.wait().expect("wait harness child");
    let stdout = fs::read(&out_path).unwrap_or_default();
    let _ = fs::remove_file(&out_path);
    if let Some(p) = in_file_path {
        let _ = fs::remove_file(p);
    }
    Run {
        stdout,
        code: status.code(),
        signal: status.signal(),
    }
}

#[track_caller]
pub fn assert_main_so_eq(input: &[u8], stdin_kind: StdinKind) {
    let c = run_main_via_so(&c_so(), input, stdin_kind);
    let r = run_main_via_so(&rust_so(), input, stdin_kind);
    assert_bytes_eq(
        &c.stdout,
        &r.stdout,
        &format!("main(.so) stdin={:?} kind={stdin_kind:?}", Show(input)),
    );
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "exit status mismatch for main(.so) stdin={:?}",
        Show(input)
    );
}

// ---------------------------------------------------------------------------
// End-to-end executable comparison
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ExeIo {
    /// stdin and stdout are pipes.
    Pipes,
    /// stdin and stdout are regular files.
    Files,
    /// stdin is a pipe, fd 1 is closed before `exec`.
    StdoutClosed,
    /// stdout is a pipe, fd 0 is closed before `exec`.
    StdinClosed,
}

pub fn run_exe(exe: &Path, input: &[u8], io: ExeIo) -> Run {
    let mut cmd = Command::new(exe);
    cmd.stderr(Stdio::null());
    let mut out_file = None;
    let mut tmp_paths: Vec<PathBuf> = Vec::new();

    match io {
        ExeIo::Pipes | ExeIo::StdinClosed => {
            cmd.stdin(Stdio::piped()).stdout(Stdio::piped());
        }
        ExeIo::Files => {
            let ip = unique_name("exe-in");
            fs::write(&ip, input).expect("write exe input");
            cmd.stdin(Stdio::from(fs::File::open(&ip).expect("open exe input")));
            let op = unique_name("exe-out");
            cmd.stdout(Stdio::from(
                fs::File::create(&op).expect("create exe output"),
            ));
            out_file = Some(op.clone());
            tmp_paths.push(ip);
            tmp_paths.push(op);
        }
        ExeIo::StdoutClosed => {
            cmd.stdin(Stdio::piped()).stdout(Stdio::null());
        }
    }

    match io {
        ExeIo::StdoutClosed => unsafe {
            cmd.pre_exec(|| {
                close(1);
                Ok(())
            });
        },
        ExeIo::StdinClosed => unsafe {
            cmd.pre_exec(|| {
                close(0);
                Ok(())
            });
        },
        _ => {}
    }

    let mut child = cmd.spawn().expect("spawn exe");
    if let Some(mut si) = child.stdin.take() {
        let _ = si.write_all(input);
    }
    let mut stdout = Vec::new();
    if let Some(mut so) = child.stdout.take() {
        so.read_to_end(&mut stdout).expect("read exe stdout");
    }
    let status = child.wait().expect("wait exe");
    if let Some(p) = out_file {
        stdout = fs::read(&p).unwrap_or_default();
    }
    for p in tmp_paths {
        let _ = fs::remove_file(p);
    }
    Run {
        stdout,
        code: status.code(),
        signal: status.signal(),
    }
}

#[track_caller]
pub fn assert_exe_eq(input: &[u8], io: ExeIo) {
    let c = run_exe(&c_exe(), input, io);
    let r = run_exe(&rust_exe(), input, io);
    assert_bytes_eq(
        &c.stdout,
        &r.stdout,
        &format!("exe stdin={:?} io={io:?}", Show(input)),
    );
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "exit status mismatch for exe stdin={:?} io={io:?}",
        Show(input)
    );
}

/// Runs `exe` with stdout on a pipe, reads at most `limit` bytes, then kills the
/// child.  Used for configurations whose output is ~2^31 lines long.
pub fn run_exe_prefix(exe: &Path, input: &[u8], limit: usize) -> Vec<u8> {
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn exe");
    {
        let mut si = child.stdin.take().expect("child stdin");
        let _ = si.write_all(input);
    }
    let mut buf = vec![0u8; limit];
    let mut got = 0usize;
    {
        let so = child.stdout.as_mut().expect("child stdout");
        while got < limit {
            match so.read(&mut buf[got..]) {
                Ok(0) => break,
                Ok(n) => got += n,
                Err(_) => break,
            }
        }
    }
    let _ = child.kill();
    let _ = child.wait();
    buf.truncate(got);
    buf
}

#[track_caller]
pub fn assert_exe_prefix_eq(input: &[u8], limit: usize) {
    let c = run_exe_prefix(&c_exe(), input, limit);
    let r = run_exe_prefix(&rust_exe(), input, limit);
    assert_bytes_eq(&c, &r, &format!("exe prefix stdin={:?}", Show(input)));
}

/// Calls `driver(x)` in a fresh child through the `.so`, capturing at most
/// `limit` bytes.  The child is the `main` harness fed a decimal `x` on stdin,
/// which is how a caller reaches `driver` with an unbounded `x`.
#[track_caller]
pub fn assert_driver_prefix_eq_via_main(input: &[u8], limit: usize) {
    assert_exe_prefix_eq(input, limit);
}

// ---------------------------------------------------------------------------
// Reference model of the C output (used for sanity, never as ground truth)
// ---------------------------------------------------------------------------

pub fn expected_driver_output(x: i32) -> Vec<u8> {
    let mut v = Vec::new();
    let mut i: i32 = 0;
    let mut j: i32 = 0;
    while i < x {
        v.extend_from_slice(format!("{i} {j}\n").as_bytes());
        i = i.wrapping_add(1);
        j = j.wrapping_add(2);
    }
    v
}

/// Number of bytes `driver(x)` writes, for picking buffer-boundary values.
pub fn driver_output_len(x: i32) -> usize {
    if x <= 0 {
        return 0;
    }
    let mut n = 0usize;
    for i in 0..x {
        n += dec_len(i) + 1 + dec_len(2 * i) + 1;
    }
    n
}

fn dec_len(mut v: i32) -> usize {
    if v == 0 {
        return 1;
    }
    let mut n = 0;
    while v > 0 {
        n += 1;
        v /= 10;
    }
    n
}

// ---------------------------------------------------------------------------
// Diagnostics helpers
// ---------------------------------------------------------------------------

pub struct Show<'a>(pub &'a [u8]);

impl std::fmt::Debug for Show<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = &self.0[..self.0.len().min(64)];
        write!(f, "\"")?;
        for &b in s {
            match b {
                b'\n' => write!(f, "\\n")?,
                b'\t' => write!(f, "\\t")?,
                b'\r' => write!(f, "\\r")?,
                0x0b => write!(f, "\\v")?,
                0x0c => write!(f, "\\f")?,
                0x20..=0x7e => write!(f, "{}", b as char)?,
                _ => write!(f, "\\x{b:02x}")?,
            }
        }
        if self.0.len() > s.len() {
            write!(f, "...\" ({} bytes)", self.0.len())?;
        } else {
            write!(f, "\"")?;
        }
        Ok(())
    }
}

pub struct Trunc<'a>(pub &'a [i32]);

impl std::fmt::Debug for Trunc<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.len() <= 8 {
            write!(f, "{:?}", self.0)
        } else {
            write!(f, "{:?}.. ({} values)", &self.0[..8], self.0.len())
        }
    }
}

#[track_caller]
pub fn assert_bytes_eq(c: &[u8], r: &[u8], ctx: &str) {
    if c == r {
        return;
    }
    let first = c
        .iter()
        .zip(r.iter())
        .position(|(a, b)| a != b)
        .unwrap_or(c.len().min(r.len()));
    let lo = first.saturating_sub(40);
    panic!(
        "output mismatch [{ctx}]\n  C len={} rust len={} first diff at byte {}\n  C   ...{:?}\n  rust...{:?}",
        c.len(),
        r.len(),
        first,
        Show(&c[lo..(first + 40).min(c.len())]),
        Show(&r[lo..(first + 40).min(r.len())]),
    );
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*), fixed seeds for reproducibility
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed | 1)
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
    /// Uniform in `[lo, hi]`.
    pub fn range_i64(&mut self, lo: i64, hi: i64) -> i64 {
        assert!(lo <= hi);
        let span = (hi - lo) as u64 + 1;
        lo + (self.next_u64() % span) as i64
    }
    pub fn range_usize(&mut self, lo: usize, hi: usize) -> usize {
        self.range_i64(lo as i64, hi as i64) as usize
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.range_usize(0, xs.len() - 1)]
    }
}

// ---------------------------------------------------------------------------
// Re-executed child entry point
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Extra differential helpers used by the error-path tests
// ---------------------------------------------------------------------------

#[track_caller]
pub fn assert_main_so_eq_opts(input: &[u8], stdin_kind: StdinKind, opts: MainSoOpts) {
    let c = run_main_via_so_opts(&c_so(), input, stdin_kind, opts);
    let r = run_main_via_so_opts(&rust_so(), input, stdin_kind, opts);
    assert_bytes_eq(
        &c.stdout,
        &r.stdout,
        &format!("main(.so) stdin={:?} kind={stdin_kind:?} opts={opts:?}", Show(input)),
    );
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "exit status mismatch for main(.so) stdin={:?} opts={opts:?}",
        Show(input)
    );
}

/// Runs `exe`, reads a few bytes of its output, then closes the read end of the
/// pipe so the writer hits `SIGPIPE` / `EPIPE`.
pub fn run_exe_sigpipe(exe: &Path, input: &[u8]) -> Run {
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn exe");
    {
        let mut si = child.stdin.take().expect("child stdin");
        let _ = si.write_all(input);
    }
    let mut got = Vec::new();
    if let Some(mut so) = child.stdout.take() {
        let mut buf = [0u8; 16];
        if let Ok(n) = so.read(&mut buf) {
            got.extend_from_slice(&buf[..n]);
        }
        drop(so); // reader gone -> the next write(2) raises SIGPIPE
    }
    let status = child.wait().expect("wait exe");
    Run {
        stdout: got,
        code: status.code(),
        signal: status.signal(),
    }
}

/// Calls `driver` through the `.so` with the argument register deliberately
/// carrying garbage in its upper 32 bits: a C `int` parameter only occupies the
/// low half, so both implementations must ignore the high half.
pub fn driver_out_wide_arg(lib: &'static Library, raw: u64) -> Vec<u8> {
    type WideDriverFn = unsafe extern "C" fn(u64);
    let f: Symbol<'static, WideDriverFn> = unsafe { lib.get(b"driver\0").expect("`driver`") };
    let (_, bytes) = with_stdout_file(|| unsafe { f(raw) });
    bytes
}

#[track_caller]
pub fn assert_driver_wide_arg_eq(raw: u64) {
    let c = driver_out_wide_arg(c_lib(), raw);
    let r = driver_out_wide_arg(rust_lib(), raw);
    assert_bytes_eq(&c, &r, &format!("driver(wide arg {raw:#018x})"));
}

// ---------------------------------------------------------------------------
// Minimal sequential test harness
// ---------------------------------------------------------------------------
//
// These test binaries are built with `harness = false` on purpose.  Both
// implementations write to file descriptor 1, so every comparison has to
// temporarily point fd 1 at a capture file — which is process-global state.
// libtest runs cases on several threads and writes its own progress lines to
// fd 1 while doing so, which would land inside the captured bytes.  Running the
// cases sequentially from our own `main` removes that interference entirely and
// makes the results reproducible.

pub struct Case {
    pub name: &'static str,
    pub f: fn(),
}

impl Case {
    pub const fn new(name: &'static str, f: fn()) -> Self {
        Case { name, f }
    }
}

#[macro_export]
macro_rules! case {
    ($f:ident) => {
        $crate::common::Case::new(stringify!($f), $f)
    };
}

/// Runs `cases` sequentially and exits with 0 only if all of them pass.
///
/// Also serves as the entry point of the re-executed `main`-symbol child: when
/// the harness environment variables are set, `child_harness_body` takes over
/// and never returns.
pub fn run_cases(cases: &[Case]) -> ! {
    child_harness_body();

    let mut filters: Vec<String> = Vec::new();
    let mut list = false;
    let mut args = std::env::args().skip(1).peekable();
    while let Some(a) = args.next() {
        match a.as_str() {
            "--list" => list = true,
            "--test-threads" | "--format" | "--logfile" | "--skip" => {
                let _ = args.next();
            }
            s if s.starts_with('-') => {}
            s => filters.push(s.to_string()),
        }
    }

    let selected: Vec<&Case> = cases
        .iter()
        .filter(|c| filters.is_empty() || filters.iter().any(|f| c.name.contains(f)))
        .collect();

    if list {
        for c in &selected {
            println!("{}: test", c.name);
        }
        std::process::exit(0);
    }

    println!("\nrunning {} tests", selected.len());
    let mut failed: Vec<(&str, String)> = Vec::new();
    for c in &selected {
        print!("test {} ... ", c.name);
        let _ = std::io::stdout().flush();
        let hook = std::panic::take_hook();
        let captured = std::sync::Arc::new(Mutex::new(String::new()));
        {
            let captured = std::sync::Arc::clone(&captured);
            std::panic::set_hook(Box::new(move |info| {
                *captured.lock().unwrap_or_else(|e| e.into_inner()) = format!("{info}");
            }));
        }
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(c.f));
        std::panic::set_hook(hook);
        match r {
            Ok(()) => println!("ok"),
            Err(_) => {
                println!("FAILED");
                let msg = captured.lock().unwrap_or_else(|e| e.into_inner()).clone();
                failed.push((c.name, msg));
            }
        }
        let _ = std::io::stdout().flush();
    }

    if failed.is_empty() {
        println!(
            "\ntest result: ok. {} passed; 0 failed\n",
            selected.len()
        );
        std::process::exit(0);
    }
    println!("\nfailures:\n");
    for (name, msg) in &failed {
        println!("---- {name} ----\n{msg}\n");
    }
    println!("failures:");
    for (name, _) in &failed {
        println!("    {name}");
    }
    println!(
        "\ntest result: FAILED. {} passed; {} failed\n",
        selected.len() - failed.len(),
        failed.len()
    );
    std::process::exit(1);
}
