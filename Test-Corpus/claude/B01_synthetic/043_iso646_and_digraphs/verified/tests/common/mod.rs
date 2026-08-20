// Copyright 2025 MIT Lincoln Laboratory
// SPDX-License-Identifier: MIT
//
// Shared plumbing for the C-vs-Rust differential tests.
//
// Two comparison boundaries are used, and *neither* of them calls a Rust
// function directly:
//
//   * process boundary  - the C executable and the Rust executable are spawned
//     with identical stdin bytes; stdout, stderr, exit code and terminating
//     signal are compared byte for byte.
//   * FFI boundary      - the C `.so` and the Rust `.so` are both `dlopen`ed and
//     their exported `driver` / `main` symbols are called through function
//     pointers, so the `#[no_mangle]` wrappers are under test too.

#![allow(dead_code)]

use std::ffi::c_void;
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::raw::c_int;
use std::os::unix::io::AsRawFd;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// Artifact locations
// ---------------------------------------------------------------------------

/// The C executable, built from the unmodified `c_src/src/main.c` by `build.rs`.
pub fn c_exe() -> &'static Path {
    Path::new(env!("C_DRIVER_EXE"))
}

/// The same C source built without optimisation, for confirming that the
/// reference behaviour does not depend on the optimisation level.
pub fn c_exe_o0() -> &'static Path {
    Path::new(env!("C_DRIVER_EXE_O0"))
}

/// The C shared object, built from the same translation unit with `-shared -fPIC`.
pub fn c_so() -> &'static Path {
    Path::new(env!("C_DRIVER_SO"))
}

/// Fail loudly when a build artifact predates the sources it is built from.
///
/// A stale artifact is the worst possible failure mode for a differential test:
/// the suite compares against code that is no longer in the tree and reports
/// success. This was observed for real — a `target/release/driver` left over from
/// an earlier build made the `SIGPIPE` row pass while the fix was absent — so it
/// is a hard error rather than a warning.
fn assert_fresh(artifact: &Path, sources: &[&str]) {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let artifact_mtime = std::fs::metadata(artifact)
        .and_then(|m| m.modified())
        .unwrap_or_else(|e| panic!("mtime of {artifact:?}: {e}"));
    for rel in sources {
        let src = manifest_dir.join(rel);
        let src_mtime = std::fs::metadata(&src)
            .and_then(|m| m.modified())
            .unwrap_or_else(|e| panic!("mtime of {src:?}: {e}"));
        assert!(
            artifact_mtime >= src_mtime,
            "STALE BUILD ARTIFACT: {artifact:?} is older than {src:?}.\n\
             The differential tests would have compared against out-of-date code.\n\
             Rebuild with plain `cargo test` (which also refreshes example targets)."
        );
    }
}

/// The Rust executable for whichever profile the tests were built in.
pub fn rust_exe() -> &'static Path {
    static CHECKED: OnceLock<()> = OnceLock::new();
    let p = Path::new(env!("CARGO_BIN_EXE_driver"));
    CHECKED.get_or_init(|| assert_fresh(p, &["src/lib.rs", "src/main.rs", "Cargo.toml"]));
    p
}

/// The Rust cdylib (`examples/driver_ffi.rs`).
///
/// The test binary lives in `target/<profile>/deps/`, so the sibling
/// `target/<profile>/examples/` directory holds the shared object.
///
/// # Staleness guard
///
/// `cargo test --test <name>` builds only the lib and that one integration test
/// target — it does **not** rebuild example targets. Without a check, the tests
/// would happily `dlopen` a shared object left over from an earlier build and
/// report success against code that is no longer there. That is a silent
/// false-pass, so a `.so` older than any source file it is built from is a hard
/// error.
pub fn rust_so() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(Path::parent)
        .expect("target/<profile>/deps/<test>");
    let so = profile_dir.join("examples").join("libdriver_ffi.so");
    assert!(
        so.is_file(),
        "missing Rust cdylib at {so:?}\n\
         build it first with: cargo build --examples"
    );

    assert_fresh(
        &so,
        &["src/lib.rs", "examples/driver_ffi.rs", "Cargo.toml"],
    );
    so
}

// ---------------------------------------------------------------------------
// Deterministic RNG (xorshift64*), so every failure is reproducible
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x2545_F491_4F6C_DD1D;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
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

    /// Uniform in `0..n`.
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }

    pub fn i32v(&mut self) -> i32 {
        self.next_u32() as i32
    }

    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }

    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

// ---------------------------------------------------------------------------
// Process-level comparison
// ---------------------------------------------------------------------------

/// Everything a caller can observe about one run of the program.
#[derive(PartialEq, Eq, Clone)]
pub struct Outcome {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    /// `None` when the process was terminated by a signal.
    pub code: Option<i32>,
    /// `Some(n)` when the process was terminated by signal `n`.
    pub signal: Option<i32>,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Outcome {{ stdout: {:?}, stderr: {:?}, code: {:?}, signal: {:?} }}",
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr),
            self.code,
            self.signal
        )
    }
}

/// How stdin should be delivered to the child.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum StdinKind {
    /// A pipe, written in one go (the input must fit the pipe buffer) — this is
    /// the shape a shell pipeline produces.
    Pipe,
    /// A regular file, i.e. a seekable descriptor.
    File,
    /// A pipe fed in 64 KiB chunks from a background thread, so inputs larger
    /// than the pipe buffer cannot deadlock.
    PipeChunked,
    /// A pipe fed one byte at a time by a background thread, forcing short reads.
    Drip,
    /// Descriptor 0 not open at all.
    Closed,
}

/// Where the child's stdout should point.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum StdoutKind {
    Pipe,
    File,
    /// Descriptor 1 not open at all (`EBADF` on write).
    Closed,
    /// A pipe whose read end is already closed (`EPIPE`, and `SIGPIPE`).
    BrokenPipe,
}

static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn tmp_path(tag: &str) -> PathBuf {
    let n = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "driver_diff_{}_{}_{}_{}",
        std::process::id(),
        tag,
        n,
        SEED
    ))
}

/// Run `exe` with `input` on stdin, using the default shapes (pipe in, pipe out).
pub fn run(exe: &Path, input: &[u8]) -> Outcome {
    run_cfg(exe, input, StdinKind::Pipe, StdoutKind::Pipe, &[])
}

pub fn run_cfg(
    exe: &Path,
    input: &[u8],
    stdin_kind: StdinKind,
    stdout_kind: StdoutKind,
    env: &[(&str, &str)],
) -> Outcome {
    // A pipe holds 64 KiB by default; anything bigger would deadlock a
    // single-threaded write, so it is streamed from a helper thread instead.
    let stdin_kind = if stdin_kind == StdinKind::Pipe && input.len() > 32 * 1024 {
        StdinKind::PipeChunked
    } else {
        stdin_kind
    };

    let mut cmd = Command::new(exe);
    cmd.stderr(Stdio::piped());
    for (k, v) in env {
        cmd.env(k, v);
    }

    // ---- stdin ----
    let mut stdin_file_guard = None;
    match stdin_kind {
        StdinKind::Pipe | StdinKind::Drip | StdinKind::PipeChunked => {
            cmd.stdin(Stdio::piped());
        }
        StdinKind::File => {
            let p = tmp_path("in");
            std::fs::write(&p, input).expect("write stdin temp file");
            let f = std::fs::File::open(&p).expect("open stdin temp file");
            cmd.stdin(Stdio::from(f));
            stdin_file_guard = Some(p);
        }
        StdinKind::Closed => {
            cmd.stdin(Stdio::null());
        }
    }

    // ---- stdout ----
    let mut stdout_file_guard = None;
    let mut broken_pipe_write_end = None;
    match stdout_kind {
        StdoutKind::Pipe => {
            cmd.stdout(Stdio::piped());
        }
        StdoutKind::File => {
            let p = tmp_path("out");
            let f = std::fs::File::create(&p).expect("create stdout temp file");
            cmd.stdout(Stdio::from(f));
            stdout_file_guard = Some(p);
        }
        StdoutKind::Closed => {
            // `Stdio::null()` would still be a *valid* descriptor, so open
            // /dev/null and immediately hand over a closed fd is not possible
            // portably; instead reuse a pipe whose write end we close before the
            // child is spawned is also not possible. The child therefore gets a
            // descriptor that has been closed via `pre_exec`.
            cmd.stdout(Stdio::piped());
            unsafe {
                use std::os::unix::process::CommandExt;
                cmd.pre_exec(|| {
                    // Close descriptor 1 in the child, just before exec.
                    if close(1) != 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                    Ok(())
                });
            }
        }
        StdoutKind::BrokenPipe => {
            // Create a pipe, close the read end *before* spawning, and give the
            // child the write end. Every write then fails with EPIPE
            // deterministically (and raises SIGPIPE).
            let (r, w) = make_pipe();
            unsafe { close(r) };
            cmd.stdout(unsafe { stdio_from_raw(w) });
            broken_pipe_write_end = Some(w);
        }
    }

    let mut child = cmd.spawn().unwrap_or_else(|e| panic!("spawn {exe:?}: {e}"));
    if let Some(w) = broken_pipe_write_end {
        // The child owns it now.
        unsafe { close(w) };
    }

    // ---- feed stdin ----
    let writer = match stdin_kind {
        StdinKind::Pipe => {
            let mut sink = child.stdin.take().expect("piped stdin");
            // The child is free to exit without draining stdin, so EPIPE here is
            // expected and ignored.
            let _ = sink.write_all(input);
            drop(sink);
            None
        }
        StdinKind::Drip | StdinKind::PipeChunked => {
            let mut sink = child.stdin.take().expect("piped stdin");
            let data = input.to_vec();
            let chunk = if stdin_kind == StdinKind::Drip {
                1
            } else {
                64 * 1024
            };
            Some(std::thread::spawn(move || {
                for part in data.chunks(chunk) {
                    // The child may exit before draining stdin; EPIPE is
                    // expected and simply ends the feed.
                    if sink.write_all(part).is_err() || sink.flush().is_err() {
                        return;
                    }
                }
            }))
        }
        StdinKind::File | StdinKind::Closed => None,
    };

    let output = child.wait_with_output().expect("wait_with_output");
    if let Some(h) = writer {
        let _ = h.join();
    }

    let stdout = match stdout_kind {
        StdoutKind::File => {
            let p = stdout_file_guard.as_ref().unwrap();
            std::fs::read(p).expect("read stdout temp file")
        }
        _ => output.stdout,
    };

    if let Some(p) = stdin_file_guard {
        let _ = std::fs::remove_file(p);
    }
    if let Some(p) = stdout_file_guard {
        let _ = std::fs::remove_file(p);
    }

    Outcome {
        stdout,
        stderr: output.stderr,
        code: output.status.code(),
        signal: output.status.signal(),
    }
}

/// What one unbounded-stdin run observed.
pub struct UnboundedRun {
    pub outcome: Outcome,
    pub elapsed: std::time::Duration,
    /// Bytes the feeder managed to push into the child's stdin before it exited.
    ///
    /// This is the load-bearing measurement: `scanf` is lazy, so a faithful
    /// implementation accepts at most a couple of buffers (plus whatever the
    /// 64 KiB pipe holds), whereas one that reads to end-of-file first swallows
    /// gigabytes. The gap is about six orders of magnitude, which makes this a
    /// far sharper signal than wall-clock time.
    pub bytes_fed: u64,
}

/// Run `exe` against a stdin that **never reaches end-of-file**: a background
/// thread writes `pattern` in a loop until the child goes away.
///
/// Returns `Err(elapsed)` if the child had to be killed, so a hang cannot block
/// the suite forever.
pub fn run_unbounded(
    exe: &Path,
    pattern: &[u8],
    timeout: std::time::Duration,
) -> Result<UnboundedRun, std::time::Duration> {
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {exe:?}: {e}"));

    let mut sink = child.stdin.take().expect("piped stdin");
    let block: Vec<u8> = pattern.repeat(4096 / pattern.len().max(1) + 1);
    let fed = std::sync::Arc::new(AtomicU64::new(0));
    let fed_writer = std::sync::Arc::clone(&fed);
    let feeder = std::thread::spawn(move || {
        // Stops as soon as the reader is gone (EPIPE).
        while sink.write_all(&block).is_ok() {
            fed_writer.fetch_add(block.len() as u64, Ordering::Relaxed);
        }
    });

    let start = std::time::Instant::now();
    let status = loop {
        match child.try_wait().expect("try_wait") {
            Some(s) => break s,
            None => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = feeder.join();
                    return Err(start.elapsed());
                }
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
        }
    };
    let elapsed = start.elapsed();
    let bytes_fed = fed.load(Ordering::Relaxed);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    if let Some(mut s) = child.stdout.take() {
        let _ = s.read_to_end(&mut stdout);
    }
    if let Some(mut s) = child.stderr.take() {
        let _ = s.read_to_end(&mut stderr);
    }
    let _ = feeder.join();

    Ok(UnboundedRun {
        outcome: Outcome {
            stdout,
            stderr,
            code: status.code(),
            signal: status.signal(),
        },
        elapsed,
        bytes_fed,
    })
}

fn describe(input: &[u8]) -> String {
    if input.len() > 160 {
        format!(
            "{:?}... ({} bytes total)",
            String::from_utf8_lossy(&input[..160]),
            input.len()
        )
    } else {
        format!("{:?}", String::from_utf8_lossy(input))
    }
}

/// Assert the C and Rust executables behave identically for `input`.
pub fn assert_same(row: &str, input: &[u8]) {
    assert_same_cfg(row, input, StdinKind::Pipe, StdoutKind::Pipe, &[]);
}

pub fn assert_same_cfg(
    row: &str,
    input: &[u8],
    stdin_kind: StdinKind,
    stdout_kind: StdoutKind,
    env: &[(&str, &str)],
) {
    let c = run_cfg(c_exe(), input, stdin_kind, stdout_kind, env);
    let r = run_cfg(rust_exe(), input, stdin_kind, stdout_kind, env);
    assert_eq!(
        c, r,
        "\n[{row}] C and Rust diverged\n  stdin  = {}\n  stdin  = {stdin_kind:?}, stdout = {stdout_kind:?}, env = {env:?}\n  C   -> {c:?}\n  Rust-> {r:?}\n",
        describe(input)
    );
}

// ---------------------------------------------------------------------------
// Raw libc bits used for descriptor juggling
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    /// `fflush(NULL)` flushes every C stream in the process, which is how the C
    /// `.so`'s buffered `printf`/`puts` output is forced out.
    fn fflush(stream: *mut c_void) -> c_int;
}

fn make_pipe() -> (c_int, c_int) {
    let mut fds = [0 as c_int; 2];
    let rc = unsafe { pipe(fds.as_mut_ptr()) };
    assert_eq!(rc, 0, "pipe(2) failed: {}", std::io::Error::last_os_error());
    (fds[0], fds[1])
}

unsafe fn stdio_from_raw(fd: c_int) -> Stdio {
    use std::os::unix::io::FromRawFd;
    // Duplicate so the caller keeps ownership of `fd`.
    let dupped = dup(fd);
    assert!(dupped >= 0, "dup failed");
    Stdio::from_raw_fd(dupped)
}

/// fd 1 (and optionally fd 0) are process-wide, so redirection has to be
/// serialised across the test harness's threads.
fn fd_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// Run `f` with stdin fed from `stdin_bytes` and stdout captured, then return
/// whatever was written to fd 1.
///
/// Both C and Rust code write through fd 1, but through different buffering
/// layers, so `fflush(NULL)` is issued for the C side and the Rust side flushes
/// itself inside `driver_impl`.
pub fn capture(stdin_bytes: Option<&[u8]>, f: impl FnOnce()) -> Vec<u8> {
    let _guard = fd_lock().lock().unwrap_or_else(|e| e.into_inner());

    let out_path = tmp_path("cap");
    let mut out_file = std::fs::File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&out_path)
        .expect("create capture file");

    let in_file = stdin_bytes.map(|bytes| {
        let p = tmp_path("capin");
        std::fs::write(&p, bytes).expect("write capture stdin");
        let f = std::fs::File::open(&p).expect("open capture stdin");
        (p, f)
    });

    // Make sure nothing of ours is still sitting in a buffer aimed at fd 1.
    let _ = std::io::stdout().flush();
    unsafe { fflush(std::ptr::null_mut()) };

    let saved_out = unsafe { dup(1) };
    assert!(saved_out >= 0, "dup(1) failed");
    let saved_in = in_file.as_ref().map(|_| {
        let s = unsafe { dup(0) };
        assert!(s >= 0, "dup(0) failed");
        s
    });

    unsafe {
        assert!(dup2(out_file.as_raw_fd(), 1) >= 0, "dup2 onto stdout failed");
        if let Some((_, f)) = in_file.as_ref() {
            assert!(dup2(f.as_raw_fd(), 0) >= 0, "dup2 onto stdin failed");
        }
    }

    f();

    // Flush the C library's buffers before putting the descriptors back.
    unsafe { fflush(std::ptr::null_mut()) };
    let _ = std::io::stdout().flush();

    unsafe {
        dup2(saved_out, 1);
        close(saved_out);
        if let (Some(s), Some(_)) = (saved_in, in_file.as_ref()) {
            dup2(s, 0);
            close(s);
        }
    }

    let mut buf = Vec::new();
    out_file.seek(SeekFrom::Start(0)).expect("seek capture file");
    out_file.read_to_end(&mut buf).expect("read capture file");

    let _ = std::fs::remove_file(&out_path);
    if let Some((p, _)) = in_file {
        let _ = std::fs::remove_file(p);
    }
    buf
}
