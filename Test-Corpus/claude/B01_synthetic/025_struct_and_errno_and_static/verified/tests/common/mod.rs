//! Shared plumbing for the C-vs-Rust differential tests.
//!
//! The C target in `c_src/CMakeLists.txt` is `add_executable(driver src/main.c)`,
//! so the primary comparison is process-level: feed both executables the same
//! stdin bytes and require byte-identical stdout, byte-identical stderr and an
//! identical exit status (code *and* terminating signal).
//!
//! In addition both sides are built as shared objects (`gcc -shared -fPIC` on the
//! untouched C source, and the `ffi/` cdylib on the Rust side) so that the two
//! exported C-ABI symbols — `main` and `run` — can be compared directly through
//! `dlopen`, which is the only way to reach `run()` at call depths and with
//! argument values that the process entry point can never produce.
//!
//! Nothing in `c_src/` is modified: the C artifacts are compiled *out of tree*
//! into `target/difftest/`.

#![allow(dead_code)]

use std::ffi::c_void;
use std::fs::File;
use std::io::Write;
use std::os::raw::c_int;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// Paths and artifact building
// ---------------------------------------------------------------------------

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_source() -> PathBuf {
    manifest_dir().join("c_src/src/main.c")
}

/// The Rust executable under test, exactly as cargo built it for this profile.
pub fn rust_exe() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// A scratch directory under `target/difftest/`.
pub fn scratch(tag: &str) -> PathBuf {
    let dir = manifest_dir().join("target/difftest").join(tag);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

/// Scratch directory reserved for *this* test binary's compiled artifacts.
///
/// Cargo runs test binaries one at a time, but the tests *inside* one binary run
/// in parallel threads, so the compiled artifacts have to be shared (built once)
/// rather than rebuilt per test on the same output path — two concurrent `rustc`
/// invocations writing the same file fail with "failed to open object file".
fn artifact_dir() -> &'static PathBuf {
    static D: OnceLock<PathBuf> = OnceLock::new();
    D.get_or_init(|| {
        let exe = std::env::current_exe().expect("current_exe");
        let name = exe
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        scratch(&format!("artifacts/{name}"))
    })
}

/// The C executable built with the exact flags of the CMake target, built once.
pub fn c_exe() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| build_c_exe(artifact_dir(), ""))
}

/// `gcc -O2 -shared -fPIC c_src/src/main.c`, built once.
pub fn c_so() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| build_c_so(artifact_dir()))
}

/// The Rust `cdylib`, built once.
pub fn rust_so() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| build_rust_so(artifact_dir()))
}

fn run_tool(what: &str, cmd: &mut Command) {
    let out = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn {what}: {e}"));
    if !out.status.success() {
        panic!(
            "{what} failed ({:?})\n--- stdout ---\n{}\n--- stderr ---\n{}",
            out.status,
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

/// Compile `c_src/src/main.c` to an executable with the given optimisation flag.
///
/// `""` reproduces the CMake target exactly: `c_src/CMakeLists.txt` sets no
/// `CMAKE_BUILD_TYPE` and no flags, so the default build is unoptimised.
pub fn build_c_exe(dir: &Path, opt: &str) -> PathBuf {
    let name = if opt.is_empty() {
        "c_driver_default".to_string()
    } else {
        format!("c_driver_{}", opt.trim_start_matches('-'))
    };
    let out = dir.join(name);
    let mut cmd = Command::new("cc");
    if !opt.is_empty() {
        cmd.arg(opt);
    }
    cmd.arg("-o").arg(&out).arg(c_source());
    run_tool("cc (C executable)", &mut cmd);
    out
}

/// `gcc -shared -fPIC c_src/src/main.c` — the C side of the FFI comparison.
pub fn build_c_so(dir: &Path) -> PathBuf {
    let out = dir.join("libc_driver.so");
    let mut cmd = Command::new("cc");
    cmd.arg("-O2")
        .arg("-shared")
        .arg("-fPIC")
        .arg("-o")
        .arg(&out)
        .arg(c_source());
    run_tool("cc (C shared object)", &mut cmd);
    out
}

/// Build the Rust `cdylib` (the `ffi/` crate) with plain `rustc`.
///
/// `rustc` is used rather than a nested `cargo` invocation so the test never
/// contends for cargo's target-directory lock. Both crates are dependency-free,
/// so two `rustc` calls are all it takes, and the resulting `.so` is compiled
/// from the very same sources cargo uses for `target/*/libdriver_ffi.so`.
pub fn build_rust_so(dir: &Path) -> PathBuf {
    let rlib = dir.join("libdriver.rlib");
    let mut cmd = Command::new("rustc");
    cmd.args(["--edition", "2021"])
        .args(["--crate-type", "rlib"])
        .args(["--crate-name", "driver"])
        .args(["-C", "opt-level=2"])
        .arg("-o")
        .arg(&rlib)
        .arg(manifest_dir().join("src/lib.rs"));
    run_tool("rustc (driver rlib)", &mut cmd);

    let out = dir.join("librust_driver.so");
    let mut cmd = Command::new("rustc");
    cmd.args(["--edition", "2021"])
        .args(["--crate-type", "cdylib"])
        .args(["--crate-name", "driver_ffi"])
        .args(["-C", "opt-level=2"])
        .arg("--extern")
        .arg(format!("driver={}", rlib.display()))
        .arg("-o")
        .arg(&out)
        .arg(manifest_dir().join("ffi/src/lib.rs"));
    run_tool("rustc (driver_ffi cdylib)", &mut cmd);
    out
}

// ---------------------------------------------------------------------------
// Process runner
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
pub struct RunResult {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub code: Option<i32>,
    pub signal: Option<i32>,
}

impl RunResult {
    fn from_output(out: std::process::Output) -> Self {
        use std::os::unix::process::ExitStatusExt;
        RunResult {
            stdout: out.stdout,
            stderr: out.stderr,
            code: out.status.code(),
            signal: out.status.signal(),
        }
    }
    fn from_status(status: std::process::ExitStatus, stdout: Vec<u8>, stderr: Vec<u8>) -> Self {
        use std::os::unix::process::ExitStatusExt;
        RunResult {
            stdout,
            stderr,
            code: status.code(),
            signal: status.signal(),
        }
    }
}

/// Writes `input` to a temp file and runs `exe` with stdin redirected from it.
/// stdout/stderr are collected through pipes.
pub fn run_stdin_file(exe: &Path, dir: &Path, input: &[u8]) -> RunResult {
    let tmp = dir.join("stdin.bin");
    std::fs::write(&tmp, input).expect("write stdin file");
    let f = File::open(&tmp).expect("open stdin file");
    let out = Command::new(exe)
        .stdin(Stdio::from(f))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn");
    RunResult::from_output(out)
}

/// Runs `exe` with stdin connected to a pipe; `chunks` are written in order,
/// then the pipe is closed.
pub fn run_stdin_pipe(exe: &Path, chunks: &[&[u8]]) -> RunResult {
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    {
        let mut si = child.stdin.take().expect("stdin");
        for c in chunks {
            // The child may already have exited (it reads at most 99 bytes and
            // never reads again), so a broken pipe here is expected.
            if si.write_all(c).is_err() {
                break;
            }
            let _ = si.flush();
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    }
    let out = child.wait_with_output().expect("wait");
    RunResult::from_output(out)
}

/// Runs `exe` with stdin from a file and stdout redirected to a regular file
/// (full buffering, different from the pipe case).
pub fn run_stdout_file(exe: &Path, dir: &Path, input: &[u8], tag: &str) -> RunResult {
    let tmp = dir.join(format!("stdin_{tag}.bin"));
    std::fs::write(&tmp, input).expect("write stdin file");
    let outpath = dir.join(format!("stdout_{tag}.bin"));
    let sin = File::open(&tmp).expect("open stdin file");
    let sout = File::create(&outpath).expect("create stdout file");
    let mut child = Command::new(exe)
        .stdin(Stdio::from(sin))
        .stdout(Stdio::from(sout))
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    let mut err = Vec::new();
    if let Some(mut e) = child.stderr.take() {
        use std::io::Read;
        let _ = e.read_to_end(&mut err);
    }
    let status = child.wait().expect("wait");
    let stdout = std::fs::read(&outpath).expect("read stdout file");
    RunResult::from_status(status, stdout, err)
}

/// Runs `exe` with stdout on a pipe that already has **no** reader, which is what
/// makes the C process die from `SIGPIPE`.
///
/// The read end is dropped *before* `spawn`, otherwise the child can finish and
/// exit 0 before the parent gets round to closing it — a race that makes the
/// observed exit status nondeterministic for both implementations.
pub fn run_stdout_closed_pipe(exe: &Path, dir: &Path, input: &[u8]) -> RunResult {
    let tmp = dir.join("stdin_sigpipe.bin");
    std::fs::write(&tmp, input).expect("write stdin file");
    let sin = File::open(&tmp).expect("open stdin file");
    let (reader, writer) = std::io::pipe().expect("pipe");
    drop(reader); // no reader left -> every write gets EPIPE / SIGPIPE
    let mut child = Command::new(exe)
        .stdin(Stdio::from(sin))
        .stdout(Stdio::from(writer))
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    let mut err = Vec::new();
    if let Some(mut e) = child.stderr.take() {
        use std::io::Read;
        let _ = e.read_to_end(&mut err);
    }
    let status = child.wait().expect("wait");
    RunResult::from_status(status, Vec::new(), err)
}

/// Runs `exe` with the given file descriptors closed before `exec`.
pub fn run_with_closed_fds(exe: &Path, dir: &Path, input: &[u8], close_fds: &[c_int]) -> RunResult {
    use std::os::unix::process::CommandExt;
    let tmp = dir.join("stdin_closed.bin");
    std::fs::write(&tmp, input).expect("write stdin file");
    let sin = File::open(&tmp).expect("open stdin file");
    let fds: Vec<c_int> = close_fds.to_vec();
    let mut cmd = Command::new(exe);
    cmd.stdin(Stdio::from(sin))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    unsafe {
        cmd.pre_exec(move || {
            for &fd in &fds {
                close(fd);
            }
            Ok(())
        });
    }
    let out = cmd.output().expect("spawn");
    RunResult::from_output(out)
}

/// Runs `exe` with stdin opened on a directory, so every `read(2)` fails with
/// `EISDIR` and `fgets` returns `NULL`.
pub fn run_stdin_directory(exe: &Path) -> RunResult {
    let d = File::open("/").expect("open /");
    let out = Command::new(exe)
        .stdin(Stdio::from(d))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn");
    RunResult::from_output(out)
}

/// Runs `exe` with extra command-line arguments (`int main()` ignores them).
pub fn run_with_args(exe: &Path, dir: &Path, input: &[u8], args: &[&str]) -> RunResult {
    let tmp = dir.join("stdin_args.bin");
    std::fs::write(&tmp, input).expect("write stdin file");
    let f = File::open(&tmp).expect("open stdin file");
    let out = Command::new(exe)
        .args(args)
        .stdin(Stdio::from(f))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn");
    RunResult::from_output(out)
}

// ---------------------------------------------------------------------------
// Comparison
// ---------------------------------------------------------------------------

pub fn describe(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes.iter().take(160) {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            0x0b => s.push_str("\\v"),
            0x0c => s.push_str("\\f"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    if bytes.len() > 160 {
        s.push_str(&format!("...(+{} bytes)", bytes.len() - 160));
    }
    s
}

/// Assert that the C and Rust results are indistinguishable.
pub fn assert_same(row: &str, case: &str, c: &RunResult, r: &RunResult) {
    if c == r {
        return;
    }
    panic!(
        "[{row}] divergence for case {case}\n\
         stdout C: {}\n\
         stdout R: {}\n\
         stderr C: {}\n\
         stderr R: {}\n\
         status C: code={:?} signal={:?}\n\
         status R: code={:?} signal={:?}",
        describe(&c.stdout),
        describe(&r.stdout),
        describe(&c.stderr),
        describe(&r.stderr),
        c.code,
        c.signal,
        r.code,
        r.signal
    );
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — no external crates, reproducible corpus
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
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in `[0, n)` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % (n as u64)) as usize
    }
    pub fn i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    pub fn i64(&mut self) -> i64 {
        self.next_u64() as i64
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
}

/// The interesting `int`/`long` boundary values the C code branches on.
pub const BOUNDARY_I64: &[i64] = &[
    0,
    1,
    -1,
    2,
    -2,
    9,
    10,
    -10,
    127,
    128,
    255,
    256,
    32767,
    32768,
    65535,
    65536,
    i32::MAX as i64 - 5,
    i32::MAX as i64 - 4,
    i32::MAX as i64 - 1,
    i32::MAX as i64,
    i32::MAX as i64 + 1,
    i32::MAX as i64 + 2,
    i32::MIN as i64 - 2,
    i32::MIN as i64 - 1,
    i32::MIN as i64,
    i32::MIN as i64 + 1,
    i32::MIN as i64 + 5,
    2147483643,
    -2147483643,
    4294967295,
    4294967296,
    i64::MAX - 1,
    i64::MAX,
    i64::MIN,
    i64::MIN + 1,
];

pub const BOUNDARY_I32: &[i32] = &[
    0,
    1,
    -1,
    2,
    -2,
    5,
    -5,
    9,
    10,
    100,
    -100,
    32767,
    -32768,
    65535,
    1 << 30,
    -(1 << 30),
    i32::MAX,
    i32::MAX - 1,
    i32::MAX - 4,
    i32::MAX - 5,
    i32::MAX / 2,
    i32::MIN,
    i32::MIN + 1,
    i32::MIN + 5,
    i32::MIN / 2,
    2147483643,
    -2147483643,
];

// ---------------------------------------------------------------------------
// fd 1 capture, for the dlopen-based comparison of `run()`
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

/// fd 1 is process-global, so captures must not run concurrently.
///
/// `capture_fd1` always restores fd 1 before returning, so a test that panics on
/// a divergence leaves the descriptor in a sane state — the lock is therefore
/// safe to keep using and poisoning is deliberately ignored. (Otherwise the
/// first genuine failure would turn every other test into a `PoisonError`, which
/// hides which rows actually diverged.)
pub fn fd_guard() -> std::sync::MutexGuard<'static, ()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// Redirect fd 1 to `path`, run `f`, restore fd 1, and return everything written.
///
/// Handles both writers involved: `libc`'s `stdout` FILE (used by the C `.so`'s
/// `printf`) is flushed with `fflush(NULL)`, and Rust's `Stdout` (used by the
/// Rust `.so`) is line-buffered and flushed by `run()` itself.
pub fn capture_fd1<F: FnOnce()>(path: &Path, f: F) -> Vec<u8> {
    let file = File::create(path).expect("create capture file");
    let fd = file.as_raw_fd();
    let _ = std::io::stdout().flush();
    unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(fd, 1) >= 0, "dup2 failed");
        f();
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "restore dup2 failed");
        close(saved);
    }
    drop(file);
    std::fs::read(path).expect("read capture file")
}

/// Copy a shared object to a unique path so `dlopen` gives a *fresh* instance
/// with freshly initialised globals (`static house_t the_house`).
pub fn fresh_copy(src: &Path, dir: &Path, tag: &str) -> PathBuf {
    let dst = dir.join(format!("fresh_{tag}.so"));
    std::fs::copy(src, &dst).expect("copy .so");
    dst
}
