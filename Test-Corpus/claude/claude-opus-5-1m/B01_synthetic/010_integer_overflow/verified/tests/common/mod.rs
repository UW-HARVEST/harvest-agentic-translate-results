//! Shared plumbing for the C-vs-Rust differential tests.
//!
//! Nothing in here calls a Rust function of the translation directly: the Rust
//! code under test is always reached through `dlopen`/`dlsym` on the built
//! `cdylib` (`libdriver.so`), exactly like the C shared library built from
//! `c_src/src/main.c`.

#![allow(dead_code)]

use std::ffi::OsString;
use std::fs;
use std::io::{Read, Write};
use std::os::raw::{c_int, c_void};
use std::os::unix::io::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    fn openpty(
        amaster: *mut c_int,
        aslave: *mut c_int,
        name: *mut i8,
        termp: *const c_void,
        winp: *const c_void,
    ) -> c_int;
}

/// Serialises every test that temporarily re-points file descriptor 1.
pub fn fd_lock() -> &'static Mutex<()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seed, reproducible across runs.
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub const DEFAULT_SEED: u64 = 0x2545_F491_4F6C_DD1D;

    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { Self::DEFAULT_SEED } else { seed })
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

    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }

    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    /// Uniform-ish in `[lo, hi]`.
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        assert!(lo <= hi);
        lo + (self.next_u64() % ((hi - lo + 1) as u64)) as usize
    }

    pub fn bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.next_u8()).collect()
    }
}

// ---------------------------------------------------------------------------
// Build / locate the artifacts under test.
// ---------------------------------------------------------------------------

pub struct Artifacts {
    /// C translation unit as a shared library, `-O0` (matches the CMake default).
    pub c_so: PathBuf,
    /// Same, compiled `-O2`.
    pub c_so_o2: PathBuf,
    /// Rust `cdylib` built by cargo (`libdriver.so`).
    pub rust_so: PathBuf,
    /// C executable, built exactly like `cmake --build .` does.
    pub c_exe: PathBuf,
    /// Rust executable built by cargo in the same profile as the tests.
    pub rust_exe: PathBuf,
    /// `examples/so_runner` — `dlopen`s a library and calls one symbol.
    pub runner: PathBuf,
    /// Scratch directory for temporary files.
    pub scratch: PathBuf,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `target/<profile>` of the currently running test binary
/// (`target/<profile>/deps/<test>-<hash>`).
fn profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(|p| p.parent())
        .expect("test binary lives in target/<profile>/deps/")
        .to_path_buf()
}

fn run(cmd: &mut Command) {
    let rendered = format!("{cmd:?}");
    let out = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn {rendered}: {e}"));
    assert!(
        out.status.success(),
        "command failed: {rendered}\nstatus: {:?}\nstdout:\n{}\nstderr:\n{}",
        out.status,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

pub fn artifacts() -> &'static Artifacts {
    static A: OnceLock<Artifacts> = OnceLock::new();
    A.get_or_init(|| {
        let manifest = manifest_dir();
        let profile = profile_dir();
        let out = profile.join("difftest");
        fs::create_dir_all(&out).expect("create target/<profile>/difftest");
        let scratch = out.join("scratch");
        let _ = fs::remove_dir_all(&scratch);
        fs::create_dir_all(&scratch).expect("create scratch dir");

        let c_main = manifest.join("c_src/src/main.c");
        assert!(c_main.is_file(), "missing {}", c_main.display());

        // ---- C artifacts (c_src itself is never modified) ----------------
        let c_so = out.join("libdriver_c.so");
        run(Command::new("gcc")
            .arg("-shared")
            .arg("-fPIC")
            .arg("-o")
            .arg(&c_so)
            .arg(&c_main));

        let c_so_o2 = out.join("libdriver_c_O2.so");
        run(Command::new("gcc")
            .arg("-shared")
            .arg("-fPIC")
            .arg("-O2")
            .arg("-o")
            .arg(&c_so_o2)
            .arg(&c_main));

        // `cmake --build .` runs `gcc -o driver src/main.c` (no extra flags).
        let c_exe = out.join("driver_c");
        run(Command::new("gcc").arg("-o").arg(&c_exe).arg(&c_main));

        // ---- Rust artifacts ---------------------------------------------
        // Build the cdylib, the bin and the dlopen runner into the same
        // target/<profile> the test binary itself lives in, honouring the
        // feature selection the test run was started with.
        let mut cargo = Command::new(env!("CARGO"));
        cargo
            .current_dir(&manifest)
            .arg("build")
            .arg("--offline")
            .arg("--lib")
            .arg("--bin")
            .arg("driver")
            .arg("--example")
            .arg("so_runner");
        if profile.file_name().map(|n| n == "release").unwrap_or(false) {
            cargo.arg("--release");
        }
        for extra in feature_args() {
            cargo.arg(extra);
        }
        run(&mut cargo);

        let rust_so = profile.join("libdriver.so");
        let rust_exe = profile.join("driver");
        let runner = profile.join("examples").join("so_runner");
        for p in [&rust_so, &rust_exe, &runner] {
            assert!(p.is_file(), "cargo did not produce {}", p.display());
        }

        Artifacts {
            c_so,
            c_so_o2,
            rust_so,
            c_exe,
            rust_exe,
            runner,
            scratch,
        }
    })
}

/// Extra `cargo` flags describing the feature set under test.
///
/// `verify.sh` exports `DIFFTEST_CARGO_FEATURE_ARGS` so that the `cdylib` is
/// rebuilt with the very same feature selection as the test binary.
fn feature_args() -> Vec<OsString> {
    match std::env::var("DIFFTEST_CARGO_FEATURE_ARGS") {
        Ok(s) => s
            .split_whitespace()
            .map(OsString::from)
            .collect::<Vec<_>>(),
        Err(_) => Vec::new(),
    }
}

static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub fn scratch_path(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    artifacts()
        .scratch
        .join(format!("{tag}-{}-{n}", std::process::id()))
}

pub fn write_scratch(tag: &str, bytes: &[u8]) -> PathBuf {
    let p = scratch_path(tag);
    fs::write(&p, bytes).expect("write scratch file");
    p
}

// ---------------------------------------------------------------------------
// In-process capture of file descriptor 1.
// ---------------------------------------------------------------------------

/// Points fd 1 at a fresh regular file (or a pipe) for the duration of `f`,
/// flushes C's `stdout` and returns everything that was written.
pub fn capture_fd1_file<R>(f: impl FnOnce() -> R) -> (Vec<u8>, R) {
    let _guard = fd_lock().lock().unwrap_or_else(|e| e.into_inner());
    let path = scratch_path("fd1");
    let file = fs::File::create(&path).expect("create capture file");
    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 failed");
    let r = f();
    unsafe {
        // Flush *all* C output streams (the C library's stdout is fully
        // buffered because fd 1 is a regular file).
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "restore dup2 failed");
        close(saved);
    }
    drop(file);
    let bytes = fs::read(&path).expect("read capture file");
    let _ = fs::remove_file(&path);
    (bytes, r)
}

/// Same, but fd 1 is the write end of a pipe (still fully buffered in C, but a
/// different `fstat` shape, and `LineWriter` in Rust).
pub fn capture_fd1_pipe<R>(f: impl FnOnce() -> R) -> (Vec<u8>, R) {
    let _guard = fd_lock().lock().unwrap_or_else(|e| e.into_inner());
    let mut fds = [0 as c_int; 2];
    assert!(unsafe { pipe(fds.as_mut_ptr()) } == 0, "pipe() failed");
    let read_end = unsafe { OwnedFd::from_raw_fd(fds[0]) };
    let write_end = unsafe { OwnedFd::from_raw_fd(fds[1]) };

    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(write_end.as_raw_fd(), 1) } >= 0, "dup2 failed");
    let r = f();
    unsafe {
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "restore dup2 failed");
        close(saved);
    }
    drop(write_end); // EOF for the reader

    let mut buf = Vec::new();
    let mut reader = fs::File::from(read_end);
    reader.read_to_end(&mut buf).expect("read pipe");
    (buf, r)
}

// ---------------------------------------------------------------------------
// Subprocess driving: `so_runner <library> <symbol> [arg]`.
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub enum StdinSpec {
    /// A regular, seekable file containing the given bytes.
    File(Vec<u8>),
    /// A pipe carrying the given bytes (not seekable).
    Pipe(Vec<u8>),
    /// `/dev/null`.
    DevNull,
    /// fd 0 closed before `exec` (reads fail with `EBADF`).
    Closed,
    /// fd 0 is a directory (reads fail with `EISDIR`).
    Directory,
    /// fd 0 is the *write* end of a pipe (reads fail with `EBADF`).
    WriteOnlyPipe,
    /// Inherit whatever the parent has.
    Inherit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StdoutSpec {
    /// A regular file (fully buffered in C).
    File,
    /// A pipe (still fully buffered in C, line buffered in Rust).
    Pipe,
    /// `/dev/null`.
    DevNull,
    /// fd 1 closed before `exec` (`printf` fails with `EBADF`).
    Closed,
    /// `/dev/full` (writes fail with `ENOSPC`).
    DevFull,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Outcome {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub status: Option<i32>,
}

fn dir_for_stdin() -> PathBuf {
    let p = artifacts().scratch.join("a_directory");
    let _ = fs::create_dir_all(&p);
    p
}

/// Runs `so_runner <lib> <symbol> [arg]` with the requested stdio shape.
pub fn run_symbol(
    lib: &Path,
    symbol: &str,
    arg: Option<i32>,
    stdin: &StdinSpec,
    stdout: StdoutSpec,
) -> Outcome {
    let art = artifacts();
    let mut cmd = Command::new(&art.runner);
    cmd.arg(lib).arg(symbol);
    if let Some(a) = arg {
        cmd.arg(a.to_string());
    }
    run_command(cmd, stdin, stdout)
}

/// Runs a standalone executable (the CMake-built C binary or the cargo-built
/// Rust binary) with the requested stdio shape.
pub fn run_exe(exe: &Path, stdin: &StdinSpec, stdout: StdoutSpec) -> Outcome {
    run_command(Command::new(exe), stdin, stdout)
}

/// Same as [`run_exe`], with extra command line arguments (C's `int main()`
/// declares no parameters, so they must be ignored).
pub fn run_exe_with_args(
    exe: &Path,
    args: &[&str],
    stdin: &StdinSpec,
    stdout: StdoutSpec,
) -> Outcome {
    let mut cmd = Command::new(exe);
    cmd.args(args);
    run_command(cmd, stdin, stdout)
}

fn run_command(mut cmd: Command, stdin: &StdinSpec, stdout: StdoutSpec) -> Outcome {
    // ---- stdin ----
    let mut pipe_payload: Option<Vec<u8>> = None;
    let mut close_stdin = false;
    match stdin {
        StdinSpec::File(bytes) => {
            let p = write_scratch("stdin", bytes);
            cmd.stdin(Stdio::from(
                fs::File::open(&p).expect("reopen stdin scratch file"),
            ));
        }
        StdinSpec::Pipe(bytes) => {
            pipe_payload = Some(bytes.clone());
            cmd.stdin(Stdio::piped());
        }
        StdinSpec::DevNull => {
            cmd.stdin(Stdio::null());
        }
        StdinSpec::Closed => {
            close_stdin = true;
            cmd.stdin(Stdio::null()); // replaced by the pre_exec close below
        }
        StdinSpec::Directory => {
            let d = dir_for_stdin();
            cmd.stdin(Stdio::from(
                fs::File::open(&d).expect("open directory as stdin"),
            ));
        }
        StdinSpec::WriteOnlyPipe => {
            let mut fds = [0 as c_int; 2];
            assert!(unsafe { pipe(fds.as_mut_ptr()) } == 0, "pipe() failed");
            let read_end = unsafe { OwnedFd::from_raw_fd(fds[0]) };
            let write_end = unsafe { OwnedFd::from_raw_fd(fds[1]) };
            drop(read_end);
            cmd.stdin(Stdio::from(write_end));
        }
        StdinSpec::Inherit => {
            cmd.stdin(Stdio::inherit());
        }
    }

    // ---- stdout ----
    let mut out_file: Option<PathBuf> = None;
    let mut close_stdout = false;
    match stdout {
        StdoutSpec::File => {
            let p = scratch_path("stdout");
            cmd.stdout(Stdio::from(
                fs::File::create(&p).expect("create stdout file"),
            ));
            out_file = Some(p);
        }
        StdoutSpec::Pipe => {
            cmd.stdout(Stdio::piped());
        }
        StdoutSpec::DevNull => {
            cmd.stdout(Stdio::null());
        }
        StdoutSpec::Closed => {
            close_stdout = true;
            cmd.stdout(Stdio::null()); // replaced by the pre_exec close below
        }
        StdoutSpec::DevFull => {
            cmd.stdout(Stdio::from(
                fs::OpenOptions::new()
                    .write(true)
                    .open("/dev/full")
                    .expect("open /dev/full"),
            ));
        }
    }
    cmd.stderr(Stdio::piped());

    if close_stdin || close_stdout {
        unsafe {
            cmd.pre_exec(move || {
                if close_stdin {
                    close(0);
                }
                if close_stdout {
                    close(1);
                }
                Ok(())
            });
        }
    }

    let mut child = cmd.spawn().expect("spawn child");
    if let Some(payload) = pipe_payload {
        let mut sink = child.stdin.take().expect("piped stdin");
        // The child may exit before consuming everything -> ignore EPIPE.
        let _ = sink.write_all(&payload);
        let _ = sink.flush();
        drop(sink);
    }
    let out = child.wait_with_output().expect("wait_with_output");

    let stdout_bytes = match out_file {
        Some(p) => {
            let b = fs::read(&p).expect("read stdout file");
            let _ = fs::remove_file(&p);
            b
        }
        None => out.stdout,
    };

    Outcome {
        stdout: stdout_bytes,
        stderr: out.stderr,
        status: out.status.code(),
    }
}

/// Runs a program (`so_runner <lib> <symbol>` or an executable) with **stdin
/// connected to a pseudo terminal** that already holds `input`.
///
/// A terminal makes glibc pick *line* buffering for `stdin` (`isatty(0)`), which
/// is a different code path inside `fscanf` than the fully buffered file/pipe
/// case. `input` must end with a newline, otherwise the canonical-mode line
/// discipline would never hand the bytes to the reader.
pub fn run_on_pty(program: &Path, args: &[&std::ffi::OsStr], input: &[u8]) -> Outcome {
    assert!(
        input.ends_with(b"\n"),
        "pty input must be newline terminated, else the reader blocks"
    );
    let mut master: c_int = -1;
    let mut slave: c_int = -1;
    let rc = unsafe {
        openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null(),
        )
    };
    assert_eq!(rc, 0, "openpty failed");
    let master_owned = unsafe { OwnedFd::from_raw_fd(master) };
    let slave_owned = unsafe { OwnedFd::from_raw_fd(slave) };

    // Queue the input in the terminal *before* the child starts, so it can never
    // block waiting for us.
    {
        let mut w = fs::File::from(master_owned.try_clone().expect("dup master"));
        w.write_all(input).expect("write to pty master");
        w.flush().expect("flush pty master");
    }

    let out_path = scratch_path("pty-stdout");
    let mut cmd = Command::new(program);
    cmd.args(args)
        .stdin(Stdio::from(slave_owned))
        .stdout(Stdio::from(
            fs::File::create(&out_path).expect("create pty stdout file"),
        ))
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn child on pty");

    // Bounded wait so a misbehaving implementation cannot hang the suite.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    let status = loop {
        match child.try_wait().expect("try_wait") {
            Some(s) => break s,
            None => {
                if std::time::Instant::now() > deadline {
                    let _ = child.kill();
                    panic!(
                        "{} blocked for more than 20s on a pty stdin",
                        program.display()
                    );
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        }
    };
    let mut stderr = Vec::new();
    if let Some(mut e) = child.stderr.take() {
        let _ = e.read_to_end(&mut stderr);
    }
    drop(master_owned);
    let stdout = fs::read(&out_path).expect("read pty stdout file");
    let _ = fs::remove_file(&out_path);
    Outcome {
        stdout,
        stderr,
        status: status.code(),
    }
}

/// Asserts two [`Outcome`]s are byte-identical (stdout + exit status).
pub fn assert_same(context: &str, c: &Outcome, r: &Outcome) {
    assert_eq!(
        c.stdout,
        r.stdout,
        "{context}: stdout differs\n  C   : {:?} ({})\n  Rust: {:?} ({})\n  C stderr: {}\n  Rust stderr: {}",
        String::from_utf8_lossy(&c.stdout),
        hex(&c.stdout),
        String::from_utf8_lossy(&r.stdout),
        hex(&r.stdout),
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr),
    );
    assert_eq!(
        c.status, r.status,
        "{context}: exit status differs (C {:?} vs Rust {:?})\n  C stderr: {}\n  Rust stderr: {}",
        c.status,
        r.status,
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr),
    );
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// ---------------------------------------------------------------------------
// In-process `dlopen` handles.
// ---------------------------------------------------------------------------

pub struct Libs {
    pub c: libloading::Library,
    pub c_o2: libloading::Library,
    pub rust: libloading::Library,
}

pub fn libs() -> &'static Libs {
    static L: OnceLock<Libs> = OnceLock::new();
    L.get_or_init(|| {
        let a = artifacts();
        unsafe {
            Libs {
                c: libloading::Library::new(&a.c_so).expect("dlopen C .so"),
                c_o2: libloading::Library::new(&a.c_so_o2).expect("dlopen C -O2 .so"),
                rust: libloading::Library::new(&a.rust_so).expect("dlopen Rust .so"),
            }
        }
    })
}

/// `void printHexCharLine(char)` — the argument is passed as a full `int`
/// register so that out-of-`char`-range values can be exercised too.
pub type PrintHexCharLine = unsafe extern "C" fn(c_int);

pub fn print_hex_char_line(lib: &libloading::Library) -> libloading::Symbol<'_, PrintHexCharLine> {
    unsafe { lib.get(b"printHexCharLine\0") }.expect("dlsym printHexCharLine")
}
