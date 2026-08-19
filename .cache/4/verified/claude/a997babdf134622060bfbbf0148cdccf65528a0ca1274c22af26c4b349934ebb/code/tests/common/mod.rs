//! Shared differential-test harness.
//!
//! The C target in `c_src/CMakeLists.txt` is `add_executable(driver src/main.c)`
//! — a program, not a shared library.  It exports zero dynamic symbols and its
//! only worker function is `static void foo(int, int)`, so there is no `.so` to
//! `dlopen` and no FFI entry point to call (see `SYMBOLS.md`).  The equivalent of
//! "load both artifacts and compare through the boundary" for an executable is
//! to launch both artifacts as processes across the *real* boundary they expose —
//! stdin, stdout, stderr and exit status — and compare byte for byte.  That is
//! what this module provides; no Rust function is ever called directly.

#![allow(dead_code)]

use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

/// Wall-clock cap applied to every child process, in seconds.
pub const DEFAULT_TIMEOUT_SECS: u32 = 60;

/// Largest stdout a full-output comparison will buffer.  `foo` emits on the order
/// of `x + y` lines, so operands near `INT_MAX` (and any run that hits the
/// signed-overflow wrap) must be compared over a prefix instead.
pub const STDOUT_CAPTURE_CAP: usize = 64 * 1024 * 1024;

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The Rust artifact under test, as produced by cargo for the active profile
/// and feature set.
pub fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// The C artifact, built with the project's own CMake build system.
///
/// Built on demand if absent.  Several test binaries may run concurrently, so
/// the build is serialised with an exclusive lock file and losers simply wait
/// for the executable to appear.
pub fn c_bin() -> PathBuf {
    let c_src = manifest_dir().join("c_src");
    let build = c_src.join("build");
    let exe = build.join("driver");
    if exe.exists() {
        return exe;
    }

    // The lock lives outside `c_src/` so that nothing is ever added to the
    // pristine C source tree (only `c_src/build/`, the CMake build directory the
    // project's own instructions create, is written).
    let lock = tmp_dir().join("c-build.lock");
    match OpenOptions::new().write(true).create_new(true).open(&lock) {
        Ok(_) => {
            let r = build_c(&c_src, &build);
            let _ = std::fs::remove_file(&lock);
            r.expect("failed to build the C reference implementation");
        }
        Err(_) => {
            // Another test binary is building it; wait for the result.
            for _ in 0..600 {
                if exe.exists() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
    }

    assert!(
        exe.exists(),
        "C reference binary missing at {}; build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        exe.display()
    );
    exe
}

fn build_c(_c_src: &Path, build: &Path) -> Result<(), String> {
    std::fs::create_dir_all(build).map_err(|e| e.to_string())?;
    let configure = Command::new("cmake")
        .current_dir(build)
        .arg("..")
        .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
        .output()
        .map_err(|e| format!("cmake not runnable: {e}"))?;
    if !configure.status.success() {
        return Err(format!(
            "cmake configure failed:\n{}",
            String::from_utf8_lossy(&configure.stderr)
        ));
    }
    let compile = Command::new("cmake")
        .current_dir(build)
        .args(["--build", "."])
        .output()
        .map_err(|e| format!("cmake build not runnable: {e}"))?;
    if !compile.status.success() {
        return Err(format!(
            "cmake build failed:\n{}",
            String::from_utf8_lossy(&compile.stderr)
        ));
    }
    Ok(())
}

/// Everything observable about one run of one artifact.
#[derive(PartialEq, Eq)]
pub struct Outcome {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    /// Exit status as reported through the `timeout(1)` wrapper: the process's
    /// own code, or `128 + signal` when it died from a signal, or `124` when the
    /// wall-clock cap fired.
    pub code: i32,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Outcome")
            .field("code", &self.code)
            .field("stdout_len", &self.stdout.len())
            .field("stdout", &Preview(&self.stdout))
            .field("stderr", &Preview(&self.stderr))
            .finish()
    }
}

struct Preview<'a>(&'a [u8]);

impl std::fmt::Debug for Preview<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let head = &self.0[..self.0.len().min(120)];
        write!(f, "{:?}", String::from_utf8_lossy(head))?;
        if self.0.len() > head.len() {
            write!(f, "...(+{} bytes)", self.0.len() - head.len())?;
        }
        Ok(())
    }
}

/// How the child's stdin should be wired up.
pub enum In<'a> {
    /// Feed these exact bytes through a pipe.
    Bytes(&'a [u8]),
    /// Feed these bytes from a regular file instead of a pipe.
    File(&'a [u8]),
    /// Open this path (`/dev/null`, `/dev/zero`, …) as stdin.
    Path(&'a str),
}

/// How the child's stdout should be wired up.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Out {
    /// Capture through a pipe (the default).
    Pipe,
    /// Write to a regular file, then read the file back as the captured stdout.
    File,
    /// Redirect to a path that discards or rejects writes (`/dev/null`,
    /// `/dev/full`); captured stdout is reported as empty.
    Path(&'static str),
    /// Close file descriptor 1 before exec.
    Closed,
}

/// Run one artifact and collect everything observable about it.
pub fn run_with(bin: &Path, stdin: In<'_>, stdout: Out, secs: u32) -> Outcome {
    // Keep temporaries alive for the whole call.
    let tmp = tmp_dir();
    let mut stdin_file_guard: Option<PathBuf> = None;
    let mut stdout_file_guard: Option<PathBuf> = None;

    let mut cmd;
    if stdout == Out::Closed {
        // `Command` cannot close fd 1, so borrow a shell to do it.  `$0` is the
        // artifact path, and `exec` keeps the process count identical to the
        // other variants.
        cmd = Command::new("bash");
        cmd.arg("-c")
            .arg(format!(
                "exec 1>&-; exec timeout {secs} \"$0\"",
            ))
            .arg(bin);
    } else {
        cmd = Command::new("timeout");
        cmd.arg(secs.to_string()).arg(bin);
    }

    // ---- stdin ----
    let mut pipe_payload: Option<Vec<u8>> = None;
    match stdin {
        In::Bytes(b) => {
            pipe_payload = Some(b.to_vec());
            cmd.stdin(Stdio::piped());
        }
        In::File(b) => {
            let p = unique(&tmp, "stdin");
            std::fs::write(&p, b).expect("write stdin temp file");
            cmd.stdin(Stdio::from(File::open(&p).expect("open stdin temp file")));
            stdin_file_guard = Some(p);
        }
        In::Path(path) => {
            cmd.stdin(Stdio::from(
                File::open(path).unwrap_or_else(|e| panic!("open {path}: {e}")),
            ));
        }
    }

    // ---- stdout ----
    match stdout {
        Out::Pipe => {
            cmd.stdout(Stdio::piped());
        }
        Out::File => {
            let p = unique(&tmp, "stdout");
            cmd.stdout(Stdio::from(File::create(&p).expect("create stdout temp file")));
            stdout_file_guard = Some(p);
        }
        Out::Path(path) => {
            cmd.stdout(Stdio::from(
                OpenOptions::new()
                    .write(true)
                    .open(path)
                    .unwrap_or_else(|e| panic!("open {path} for writing: {e}")),
            ));
        }
        Out::Closed => {}
    }
    cmd.stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", bin.display()));

    // Feed a piped stdin from a thread: the payload can exceed the pipe
    // capacity while the child is concurrently producing output.
    let writer = pipe_payload.map(|payload| {
        let mut sink = child.stdin.take().expect("piped stdin");
        std::thread::spawn(move || {
            let _ = sink.write_all(&payload);
            let _ = sink.flush();
        })
    });

    // Capture stdout with a hard cap so that accidentally comparing a
    // multi-gigabyte run in full fails with a clear message instead of exhausting
    // memory.  (This program's stderr is never written to, so draining stdout
    // first cannot deadlock on a full stderr pipe.)
    let mut piped_stdout = Vec::new();
    let mut overflowed = false;
    if let Some(mut src) = child.stdout.take() {
        let mut chunk = vec![0u8; 64 * 1024];
        loop {
            match src.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => {
                    piped_stdout.extend_from_slice(&chunk[..n]);
                    if piped_stdout.len() > STDOUT_CAPTURE_CAP {
                        overflowed = true;
                        break;
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(e) => panic!("read stdout of {}: {e}", bin.display()),
            }
        }
    }
    let mut piped_stderr = Vec::new();
    if let Some(mut src) = child.stderr.take() {
        let _ = src.read_to_end(&mut piped_stderr);
    }
    if overflowed {
        let _ = child.kill();
    }
    let status = child
        .wait()
        .unwrap_or_else(|e| panic!("wait {}: {e}", bin.display()));
    if let Some(w) = writer {
        let _ = w.join();
    }
    assert!(
        !overflowed,
        "{} produced more than {STDOUT_CAPTURE_CAP} bytes: this configuration is \
         too large to compare in full and must use prefix comparison instead",
        bin.display()
    );

    let captured = match (&stdout, &stdout_file_guard) {
        (Out::File, Some(p)) => std::fs::read(p).expect("read stdout temp file"),
        _ => piped_stdout,
    };
    if let Some(p) = stdin_file_guard {
        let _ = std::fs::remove_file(p);
    }
    if let Some(p) = stdout_file_guard {
        let _ = std::fs::remove_file(p);
    }

    Outcome {
        stdout: captured,
        stderr: piped_stderr,
        code: status.code().unwrap_or(-1),
    }
}

/// Run one artifact with the common wiring: stdin bytes through a pipe, stdout
/// captured through a pipe.
pub fn run(bin: &Path, input: &[u8]) -> Outcome {
    run_with(bin, In::Bytes(input), Out::Pipe, DEFAULT_TIMEOUT_SECS)
}

fn tmp_dir() -> PathBuf {
    let d = std::env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest_dir().join("target"));
    let d = d.join("difftest");
    std::fs::create_dir_all(&d).expect("create temp dir");
    d
}

fn unique(dir: &Path, tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    dir.join(format!(
        "{tag}-{}-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    ))
}

/// Read at most `n` bytes of stdout from an artifact, then kill it.
///
/// Used for inputs whose output is unbounded: `foo` decrements `y` whenever
/// `y != 0`, so a negative `y` (reachable with `x > 0 && y < 0`) runs for ~2^32
/// iterations while `y` wraps around through `INT_MIN`.  Comparing a fixed-length
/// prefix is the only terminating way to diff those runs, and it is exact: both
/// artifacts must agree on all `n` bytes.
pub fn stdout_prefix(bin: &Path, input: &[u8], n: usize) -> Vec<u8> {
    let mut child: Child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", bin.display()));

    let payload = input.to_vec();
    let mut sink = child.stdin.take().expect("piped stdin");
    let writer = std::thread::spawn(move || {
        let _ = sink.write_all(&payload);
        let _ = sink.flush();
    });

    let mut buf = vec![0u8; n];
    let mut filled = 0usize;
    {
        let src = child.stdout.as_mut().expect("piped stdout");
        while filled < n {
            match src.read(&mut buf[filled..]) {
                Ok(0) => break, // the program terminated on its own
                Ok(k) => filled += k,
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(e) => panic!("read from {}: {e}", bin.display()),
            }
        }
    }
    buf.truncate(filled);

    let _ = child.kill();
    let _ = child.wait();
    let _ = writer.join();
    buf
}

/// Run one artifact with a *seekable* stdin shared with the caller, and report
/// `(outcome, final shared file offset)`.
///
/// The child receives a `dup` of our descriptor, so it shares the file offset:
/// after the child exits we can observe exactly how many bytes of the stream it
/// left consumed.  glibc hands back the unused tail of its stdio buffer when the
/// stream is cleaned up at exit, so the C program leaves the offset at the last
/// byte a conversion actually needed — behaviour a naive "slurp all of stdin"
/// translation would not reproduce.
pub fn run_tracking_stdin_offset(bin: &Path, payload: &[u8], secs: u32) -> (Outcome, u64) {
    use std::io::{Seek, SeekFrom};

    let tmp = tmp_dir();
    let path = unique(&tmp, "shared-stdin");
    std::fs::write(&path, payload).expect("write shared stdin file");
    let mut ours = File::open(&path).expect("open shared stdin file");
    let theirs = ours.try_clone().expect("dup shared stdin file");

    let child = Command::new("timeout")
        .arg(secs.to_string())
        .arg(bin)
        .stdin(Stdio::from(theirs))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", bin.display()));
    let out = child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("wait {}: {e}", bin.display()));

    let offset = ours.seek(SeekFrom::Current(0)).expect("read shared file offset");
    let _ = std::fs::remove_file(&path);
    (
        Outcome {
            stdout: out.stdout,
            stderr: out.stderr,
            code: out.status.code().unwrap_or(-1),
        },
        offset,
    )
}

/// Run one artifact with extra environment variables set.
///
/// The C program never calls `setlocale`, so it stays in the "C" locale no matter
/// what `LC_ALL`/`LANG` say, and `%d` keeps its ASCII-only digit and whitespace
/// classification.  A translation that consulted the environment (or used
/// Unicode-aware classification) would diverge here.
pub fn run_with_env(bin: &Path, env: &[(&str, &str)], input: &[u8], secs: u32) -> Outcome {
    let mut cmd = Command::new("timeout");
    cmd.arg(secs.to_string()).arg(bin);
    for (k, v) in env {
        cmd.env(k, v);
    }
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", bin.display()));
    let payload = input.to_vec();
    let mut sink = child.stdin.take().expect("piped stdin");
    let writer = std::thread::spawn(move || {
        let _ = sink.write_all(&payload);
        let _ = sink.flush();
    });
    let out = child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("wait {}: {e}", bin.display()));
    let _ = writer.join();
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code().unwrap_or(-1),
    }
}

/// Run one artifact with extra `argv` entries appended.
pub fn run_with_args(bin: &Path, args: &[&str], input: &[u8], secs: u32) -> Outcome {
    let mut child = Command::new("timeout")
        .arg(secs.to_string())
        .arg(bin)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", bin.display()));
    let payload = input.to_vec();
    let mut sink = child.stdin.take().expect("piped stdin");
    let writer = std::thread::spawn(move || {
        let _ = sink.write_all(&payload);
        let _ = sink.flush();
    });
    let out = child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("wait {}: {e}", bin.display()));
    let _ = writer.join();
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code().unwrap_or(-1),
    }
}

/// Run one artifact with its stdout piped into `head -c n`, i.e. with a reader
/// that goes away mid-stream, and report the *writer's* wait status.
///
/// A C program keeps the inherited default `SIGPIPE` disposition and is killed by
/// signal 13 (status 141); a Rust program would silently ignore `EPIPE` unless the
/// default disposition is restored.
pub fn writer_status_with_early_reader(bin: &Path, input: &[u8], head_bytes: usize, secs: u32) -> i32 {
    let mut child = Command::new("bash")
        .arg("-c")
        .arg(format!(
            "timeout {secs} \"$0\" | head -c {head_bytes} > /dev/null; exit ${{PIPESTATUS[0]}}"
        ))
        .arg(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn bash for {}: {e}", bin.display()));
    let payload = input.to_vec();
    let mut sink = child.stdin.take().expect("piped stdin");
    let writer = std::thread::spawn(move || {
        let _ = sink.write_all(&payload);
        let _ = sink.flush();
    });
    let status = child.wait().expect("wait bash");
    let _ = writer.join();
    status.code().unwrap_or(-1)
}

/// Assert that both artifacts behave identically for `input`.
pub fn assert_same(label: &str, input: &[u8]) {
    assert_same_with(label, In::Bytes(input), Out::Pipe, DEFAULT_TIMEOUT_SECS);
}

/// Assert that both artifacts behave identically for arbitrary stdio wiring.
pub fn assert_same_with(label: &str, stdin: In<'_>, stdout: Out, secs: u32) {
    let (c_in, r_in) = match stdin {
        In::Bytes(b) => (In::Bytes(b), In::Bytes(b)),
        In::File(b) => (In::File(b), In::File(b)),
        In::Path(p) => (In::Path(p), In::Path(p)),
    };
    let c = run_with(&c_bin(), c_in, stdout, secs);
    let r = run_with(&rust_bin(), r_in, stdout, secs);

    assert_ne!(
        c.code, 124,
        "{label}: the C reference hit the {secs}s wall-clock cap; \
         this input has unbounded output and must use prefix comparison"
    );

    if c.stdout != r.stdout {
        panic!(
            "{label}: stdout differs\n  first difference at byte {}\n  \
             C  : len={} {:?}\n  Rust: len={} {:?}",
            first_diff(&c.stdout, &r.stdout),
            c.stdout.len(),
            Preview(&window(&c.stdout, first_diff(&c.stdout, &r.stdout))),
            r.stdout.len(),
            Preview(&window(&r.stdout, first_diff(&c.stdout, &r.stdout))),
        );
    }
    assert_eq!(
        c.stderr, r.stderr,
        "{label}: stderr differs\n  C  : {:?}\n  Rust: {:?}",
        Preview(&c.stderr),
        Preview(&r.stderr)
    );
    assert_eq!(
        c.code, r.code,
        "{label}: exit status differs (C={}, Rust={})",
        c.code, r.code
    );
}

/// Assert both artifacts behave identically **and** that the C reference produces
/// exactly `expected_stdout`.
///
/// The second half keeps error-path rows honest: it proves the input really did
/// trigger the documented rejection (e.g. that a saturating conversion yielded
/// `-1`, so the loop guard rejected the workload) instead of merely proving that
/// the two artifacts agree on something.
pub fn assert_same_expecting(label: &str, input: &[u8], expected_stdout: &[u8]) {
    let c = run(&c_bin(), input);
    assert_eq!(
        c.stdout,
        expected_stdout,
        "{label}: the C reference did not produce the documented output; \
         the row's trigger is wrong.\n  expected: {:?}\n  actual  : {:?}",
        Preview(expected_stdout),
        Preview(&c.stdout)
    );
    assert_same(label, input);
}

/// Assert both artifacts emit an identical `n`-byte stdout prefix.
pub fn assert_same_prefix(label: &str, input: &[u8], n: usize) {
    let c = stdout_prefix(&c_bin(), input, n);
    let r = stdout_prefix(&rust_bin(), input, n);
    assert_eq!(
        c.len(),
        n,
        "{label}: expected the C reference to produce at least {n} bytes, got {}",
        c.len()
    );
    if c != r {
        panic!(
            "{label}: stdout prefix differs at byte {}\n  C  : len={} {:?}\n  Rust: len={} {:?}",
            first_diff(&c, &r),
            c.len(),
            Preview(&window(&c, first_diff(&c, &r))),
            r.len(),
            Preview(&window(&r, first_diff(&c, &r))),
        );
    }
}

fn first_diff(a: &[u8], b: &[u8]) -> usize {
    a.iter().zip(b.iter()).position(|(x, y)| x != y).unwrap_or(a.len().min(b.len()))
}

fn window(v: &[u8], at: usize) -> Vec<u8> {
    let start = at.saturating_sub(40);
    let end = (at + 40).min(v.len());
    v[start..end].to_vec()
}

/// Deterministic xorshift64* PRNG, so every "randomized" row is reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
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

    /// Uniform-ish value in `[lo, hi]` inclusive.
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        assert!(lo <= hi);
        let span = (hi - lo) as u64 + 1;
        lo + (self.next_u64() % span) as i64
    }

    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[(self.next_u64() % xs.len() as u64) as usize]
    }
}

/// `"<x> <y>"`, the canonical input layout.
pub fn pair(x: i64, y: i64) -> Vec<u8> {
    format!("{x} {y}").into_bytes()
}
