//! Shared plumbing for the C-vs-Rust differential test suite.
//!
//! Nothing in here contains test assertions; it only
//!
//! * builds the artifacts under comparison (C executable + C `.so` in several
//!   `CMAKE_BUILD_TYPE` flavours, Rust executable + Rust `cdylib` in `dev` and
//!   `release` flavours),
//! * runs a program with a given `stdin` payload and captures the *complete*
//!   observable result (`stdout`, `stderr`, exit code, terminating signal),
//! * loads a `.so` with `libloading` and calls its exported symbols with
//!   `stdout` redirected to a file, so the emitted bytes can be compared,
//! * provides a deterministic RNG so every property-style row is reproducible.
//!
//! The Rust side is *never* called directly: every comparison goes through the
//! `.so`'s exported symbols or through the executable's process boundary.

#![allow(dead_code)]

use std::ffi::OsStr;
use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// Deterministic RNG (xorshift64*), so every randomised row is reproducible.
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    /// Fixed-seed constructor; the seed is part of the test, never time-based.
    pub fn new(seed: u64) -> Self {
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

    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    /// Uniform in `[0, n)`; `n == 0` yields `0`.
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 {
            0
        } else {
            self.next_u64() % n
        }
    }

    /// Uniform in `[lo, hi]` (inclusive).
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        debug_assert!(lo <= hi);
        let span = (hi - lo) as u64 + 1;
        lo + self.below(span) as i64
    }

    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len() as u64) as usize]
    }

    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
}

// ---------------------------------------------------------------------------
// Paths / artifact construction
// ---------------------------------------------------------------------------

pub fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn cbuild_dir() -> PathBuf {
    let d = crate_root().join("cbuild");
    fs::create_dir_all(&d).expect("create cbuild dir");
    d
}

fn rbuild_dir() -> PathBuf {
    let d = crate_root().join("rbuild");
    fs::create_dir_all(&d).expect("create rbuild dir");
    d
}

pub fn tmp_dir() -> PathBuf {
    let d = crate_root().join("target").join("difftmp");
    fs::create_dir_all(&d).expect("create tmp dir");
    d
}

fn newer_than(out: &Path, src: &Path) -> bool {
    let (o, s) = (fs::metadata(out), fs::metadata(src));
    match (o, s) {
        (Ok(o), Ok(s)) => match (o.modified(), s.modified()) {
            (Ok(o), Ok(s)) => o >= s,
            _ => false,
        },
        _ => false,
    }
}

/// Compiles `out` from `src` with `compiler` + `args` unless it is already up to
/// date. Writes to a unique temporary path and renames, so parallel test
/// binaries cannot observe a half-written artifact.
fn compile<S: AsRef<OsStr>>(compiler: &str, src: &Path, out: &Path, args: &[S]) -> PathBuf {
    if newer_than(out, src) {
        return out.to_path_buf();
    }
    let stage = out.with_extension(format!("stage{}", std::process::id()));
    let mut cmd = Command::new(compiler);
    for a in args {
        cmd.arg(a);
    }
    cmd.arg(src).arg("-o").arg(&stage);
    let res = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn {compiler}: {e}"));
    assert!(
        res.status.success(),
        "{compiler} failed for {}:\n{}\n{}",
        src.display(),
        String::from_utf8_lossy(&res.stdout),
        String::from_utf8_lossy(&res.stderr)
    );
    // Rename is atomic within the same directory.
    fs::rename(&stage, out).or_else(|_| {
        // Another test binary won the race; its artifact is equivalent.
        let _ = fs::remove_file(&stage);
        if out.exists() {
            Ok(())
        } else {
            Err(std::io::Error::other("rename failed and output missing"))
        }
    })
    .expect("install compiled artifact");
    out.to_path_buf()
}

pub fn c_source() -> PathBuf {
    crate_root().join("c_src").join("src").join("main.c")
}

fn rust_bin_source() -> PathBuf {
    crate_root().join("src").join("main.rs")
}

fn rust_ffi_source() -> PathBuf {
    crate_root().join("src").join("ffi.rs")
}

/// The `CMAKE_BUILD_TYPE` flavours from `c_src/build/CMakeCache.txt`.
/// The empty flavour ("default") is what `cmake ..` without arguments uses and
/// is the canonical reference build.
pub const C_BUILD_TYPES: &[(&str, &[&str])] = &[
    ("default", &[]),
    ("Debug", &["-g"]),
    ("Release", &["-O3", "-DNDEBUG"]),
    ("RelWithDebInfo", &["-O2", "-g", "-DNDEBUG"]),
    ("MinSizeRel", &["-Os", "-DNDEBUG"]),
];

/// C executable for a given `CMAKE_BUILD_TYPE` flavour.
pub fn c_exe_flavour(flavour: &str) -> PathBuf {
    let flags = C_BUILD_TYPES
        .iter()
        .find(|(n, _)| *n == flavour)
        .map(|(_, f)| *f)
        .unwrap_or_else(|| panic!("unknown C build flavour {flavour}"));
    let out = cbuild_dir().join(format!("cdriver_{flavour}"));
    compile("cc", &c_source(), &out, flags)
}

/// The canonical C executable: `cmake ..` + `cmake --build .` output if it is
/// present, otherwise an identically-flagged `cc` build.
pub fn c_exe() -> PathBuf {
    let cmake_out = crate_root().join("c_src").join("build").join("driver");
    if cmake_out.is_file() {
        return cmake_out;
    }
    c_exe_flavour("default")
}

/// The C translation unit compiled as a shared object (same flags as the CMake
/// default configuration).
pub fn c_so() -> PathBuf {
    let out = cbuild_dir().join("libcdriver.so");
    compile("cc", &c_source(), &out, &["-fPIC", "-shared"])
}

const RUSTC_RELEASE: &[&str] = &[
    "--edition",
    "2021",
    "-C",
    "opt-level=3",
    "-C",
    "panic=abort",
    "-C",
    "debug-assertions=off",
    "-C",
    "overflow-checks=off",
    "-A",
    "warnings",
];

const RUSTC_DEV: &[&str] = &[
    "--edition",
    "2021",
    "-C",
    "opt-level=0",
    "-C",
    "debug-assertions=on",
    "-C",
    "overflow-checks=on",
    "-A",
    "warnings",
];

/// Rust executable built like the `release` profile (`panic = "abort"`).
///
/// Prefers the artifact Cargo itself produced (`target/release/driver`, i.e. the
/// real deliverable) and falls back to an equivalent `rustc` invocation, because
/// `cargo test` does not build the `bin`/`cdylib` targets.
pub fn rust_exe_release() -> PathBuf {
    let cargo_out = crate_root().join("target").join("release").join("driver");
    if cargo_out.is_file() && newer_than(&cargo_out, &rust_bin_source()) {
        return cargo_out;
    }
    let out = rbuild_dir().join("driver_release");
    compile("rustc", &rust_bin_source(), &out, RUSTC_RELEASE)
}

/// Rust executable built like the `dev` profile: overflow checks and
/// debug assertions **on**, which is a genuinely different code path for the
/// wrapping arithmetic in `driver`.
pub fn rust_exe_dev() -> PathBuf {
    let out = rbuild_dir().join("driver_dev");
    compile("rustc", &rust_bin_source(), &out, RUSTC_DEV)
}

/// Rust `cdylib` exporting `driver` and `main`, release flavour.
pub fn rust_so() -> PathBuf {
    let cargo_out = crate_root()
        .join("target")
        .join("release")
        .join("libdriver.so");
    if cargo_out.is_file()
        && newer_than(&cargo_out, &rust_bin_source())
        && newer_than(&cargo_out, &rust_ffi_source())
    {
        return cargo_out;
    }
    let out = rbuild_dir().join("libdriver.so");
    let mut args: Vec<&str> = vec!["--crate-type=cdylib", "--crate-name=driver"];
    args.extend_from_slice(RUSTC_RELEASE);
    compile("rustc", &rust_ffi_source(), &out, &args)
}

/// Rust `cdylib`, `dev` flavour (overflow checks on).
pub fn rust_so_dev() -> PathBuf {
    let out = rbuild_dir().join("libdriver_dev.so");
    let mut args: Vec<&str> = vec!["--crate-type=cdylib", "--crate-name=driver"];
    args.extend_from_slice(RUSTC_DEV);
    compile("rustc", &rust_ffi_source(), &out, &args)
}

const LOADER_SRC: &str = r#"
/* Minimal external consumer: dlopen a shared object and call its exported
 * `main` (or `driver`) exactly as any other C program would. Used so the
 * shared object's `main` symbol can be exercised with a real, pristine
 * stdin/stdout instead of the test harness's. */
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>

int main(int argc, char **argv) {
    if (argc < 3) { fprintf(stderr, "usage: loader <so> main|driver [arg]\n"); return 2; }
    void *h = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!h) { fprintf(stderr, "dlopen: %s\n", dlerror()); return 3; }
    if (argv[2][0] == 'm') {
        int (*fn)(void) = (int (*)(void)) dlsym(h, "main");
        if (!fn) { fprintf(stderr, "dlsym main: %s\n", dlerror()); return 4; }
        int rc = fn();
        fflush(NULL);
        return rc;
    } else {
        void (*fn)(int) = (void (*)(int)) dlsym(h, "driver");
        if (!fn) { fprintf(stderr, "dlsym driver: %s\n", dlerror()); return 4; }
        fn(argc > 3 ? (int) strtol(argv[3], NULL, 10) : 0);
        fflush(NULL);
        return 0;
    }
}
"#;

/// Builds (once) the little C program that `dlopen`s a `.so` and calls the
/// requested export.
pub fn loader_exe() -> PathBuf {
    let src = cbuild_dir().join("loader.c");
    let need_write = match fs::read_to_string(&src) {
        Ok(s) => s != LOADER_SRC,
        Err(_) => true,
    };
    if need_write {
        fs::write(&src, LOADER_SRC).expect("write loader.c");
    }
    let out = cbuild_dir().join("loader");
    compile("cc", &src, &out, &["-ldl"])
}

// ---------------------------------------------------------------------------
// Running a program and capturing everything observable
// ---------------------------------------------------------------------------

/// The complete observable result of one run.
#[derive(Clone, PartialEq, Eq)]
pub struct Outcome {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub code: Option<i32>,
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

/// How the payload is handed to the child's `stdin`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum StdinKind {
    /// A regular file (seekable; glibc picks the file's `st_blksize`).
    File,
    /// An anonymous pipe (non-seekable).
    Pipe,
    /// `/dev/null` (immediate EOF).
    DevNull,
    /// fd 0 closed outright, so every `read` fails with `EBADF`.
    Closed,
    /// A directory, so every `read` fails with `EISDIR`.
    Directory,
}

/// How the child's `stdout` is captured.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum StdoutKind {
    /// An anonymous pipe (glibc: fully buffered).
    Pipe,
    /// A regular file (glibc: fully buffered; contents compared afterwards).
    File,
    /// fd 1 closed outright, so every `write` fails with `EBADF`.
    Closed,
    /// A pipe whose read end is closed before the child writes ⇒ `SIGPIPE`.
    ClosedPipe,
}

fn unique_path(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    tmp_dir().join(format!(
        "{tag}.{}.{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ))
}

/// Runs `prog` with the given `stdin` payload; the default plumbing (payload in
/// a regular file, `stdout`/`stderr` on pipes).
pub fn run(prog: &Path, stdin: &[u8]) -> Outcome {
    run_cfg(prog, &[], stdin, StdinKind::File, StdoutKind::Pipe, &[])
}

pub fn run_args(prog: &Path, args: &[&str], stdin: &[u8]) -> Outcome {
    run_cfg(prog, args, stdin, StdinKind::File, StdoutKind::Pipe, &[])
}

/// Full control over the process plumbing and environment.
pub fn run_cfg(
    prog: &Path,
    args: &[&str],
    stdin: &[u8],
    sk: StdinKind,
    ok: StdoutKind,
    env: &[(&str, Option<&str>)],
) -> Outcome {
    use std::os::fd::{FromRawFd, OwnedFd};
    use std::os::unix::process::CommandExt;

    let mut cmd = Command::new(prog);
    cmd.args(args);
    for (k, v) in env {
        match v {
            Some(v) => cmd.env(k, v),
            None => cmd.env_remove(k),
        };
    }
    cmd.stderr(Stdio::piped());

    // ---- stdin ----
    let mut infile_path = None;
    match sk {
        StdinKind::File => {
            let p = unique_path("stdin");
            fs::write(&p, stdin).expect("write stdin payload");
            let f = fs::File::open(&p).expect("open stdin payload");
            cmd.stdin(Stdio::from(f));
            infile_path = Some(p);
        }
        StdinKind::Pipe => {
            cmd.stdin(Stdio::piped());
        }
        StdinKind::DevNull => {
            cmd.stdin(Stdio::null());
        }
        StdinKind::Closed => {
            // fd 0 is closed in the child after the stdio setup, so every
            // `read` fails with EBADF.
            cmd.stdin(Stdio::null());
        }
        StdinKind::Directory => {
            let f = fs::File::open(crate_root()).expect("open crate root as dir");
            cmd.stdin(Stdio::from(f));
        }
    }

    // ---- stdout ----
    let mut outfile_path = None;
    match ok {
        StdoutKind::Pipe | StdoutKind::Closed => {
            cmd.stdout(Stdio::piped());
        }
        StdoutKind::File => {
            let p = unique_path("stdout");
            let f = fs::File::create(&p).expect("create stdout file");
            cmd.stdout(Stdio::from(f));
            outfile_path = Some(p);
        }
        StdoutKind::ClosedPipe => {
            // A pipe whose read end is closed *before* the child is spawned:
            // the very first `write` is guaranteed to raise EPIPE/SIGPIPE, with
            // no race against a reader that might still be alive.
            let mut fds = [0i32; 2];
            assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0, "pipe() failed");
            let write_end = unsafe { OwnedFd::from_raw_fd(fds[1]) };
            assert_eq!(unsafe { libc::close(fds[0]) }, 0, "close(read end) failed");
            cmd.stdout(Stdio::from(write_end));
        }
    }

    // Close fd 0 / fd 1 in the child once the stdio plumbing is in place.
    let close_in = sk == StdinKind::Closed;
    let close_out = ok == StdoutKind::Closed;
    if close_in || close_out {
        unsafe {
            cmd.pre_exec(move || {
                if close_in {
                    libc::close(0);
                }
                if close_out {
                    libc::close(1);
                }
                Ok(())
            });
        }
    }

    let mut child = cmd.spawn().unwrap_or_else(|e| {
        panic!("failed to spawn {}: {e}", prog.display());
    });

    // Feed a piped stdin from a helper thread so large payloads cannot deadlock.
    let writer = if sk == StdinKind::Pipe {
        let mut sin = child.stdin.take().expect("piped stdin");
        let payload = stdin.to_vec();
        Some(std::thread::spawn(move || {
            // A closed reader (the child exits after the first number) is
            // expected; ignore the resulting EPIPE/EBADF.
            let _ = sin.write_all(&payload);
            let _ = sin.flush();
        }))
    } else {
        None
    };

    let out = child.wait_with_output().expect("wait for child");
    if let Some(w) = writer {
        let _ = w.join();
    }

    let stdout = match &outfile_path {
        Some(p) => fs::read(p).expect("read stdout file"),
        None => out.stdout.clone(),
    };
    if let Some(p) = outfile_path {
        let _ = fs::remove_file(p);
    }
    if let Some(p) = infile_path {
        let _ = fs::remove_file(p);
    }

    Outcome {
        stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

// ---------------------------------------------------------------------------
// What a *second* reader sharing file descriptor 0 sees afterwards
// ---------------------------------------------------------------------------

/// Runs `prog` with `payload` on a **seekable** `stdin` (a regular file) and
/// returns `(stdout, bytes still unread on the shared descriptor)`.
///
/// The child gets a `dup` of the parent's descriptor, so the two share one file
/// offset — exactly the situation `{ ./driver; cat; } < file` creates. glibc
/// rewinds a seekable `stdin` to the first unconsumed byte when it cleans the
/// stream up at exit, so this is directly observable.
pub fn leftover_via_file(prog: &Path, payload: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let p = unique_path("shared");
    fs::write(&p, payload).expect("write payload");
    let f = fs::File::open(&p).expect("open payload");
    let dup = f.try_clone().expect("dup payload fd");

    let out = Command::new(prog)
        .stdin(Stdio::from(dup))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", prog.display()));

    let mut rest = Vec::new();
    let mut f = f;
    f.read_to_end(&mut rest).expect("read leftovers");
    drop(f);
    let _ = fs::remove_file(&p);
    (out.stdout, rest)
}

/// Same, but `stdin` is a **pipe** (not seekable, so glibc cannot rewind and the
/// read-ahead really is lost — the amount lost is what must match).
///
/// `payload` must fit in the pipe buffer (64 KiB by default) because it is
/// written before the child starts.
pub fn leftover_via_pipe(prog: &Path, payload: &[u8]) -> (Vec<u8>, Vec<u8>) {
    use std::os::fd::{FromRawFd, OwnedFd};

    assert!(payload.len() < 60_000, "payload must fit the pipe buffer");
    let mut fds = [0i32; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0, "pipe() failed");
    let rd = unsafe { OwnedFd::from_raw_fd(fds[0]) };
    let mut wr = unsafe { fs::File::from_raw_fd(fds[1]) };
    wr.write_all(payload).expect("fill pipe");
    drop(wr); // EOF for the reader

    let child_rd = rd.try_clone().expect("dup pipe read end");
    let out = Command::new(prog)
        .stdin(Stdio::from(child_rd))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", prog.display()));

    let mut rest = Vec::new();
    let mut f = fs::File::from(rd);
    f.read_to_end(&mut rest).expect("read leftovers");
    (out.stdout, rest)
}

/// Runs `prog` with `stdout` connected to a pipe whose read end is already
/// closed, but with `SIGPIPE` set to `SIG_IGN` **before** the `exec`.
///
/// `SIG_IGN` is inherited across `execve` (a `fork`+`exec` daemon, or anything
/// spawned from CPython, leaves it that way), so the C program does *not* die
/// here — it just gets `EPIPE` and exits `0`. A translation that forces
/// `SIG_DFL` unconditionally would die from signal 13 instead.
pub fn run_closed_pipe_with_sigpipe_ignored(prog: &Path, stdin: &[u8]) -> Outcome {
    use std::os::fd::{FromRawFd, OwnedFd};
    use std::os::unix::process::CommandExt;

    let p = unique_path("stdin");
    fs::write(&p, stdin).expect("write payload");
    let f = fs::File::open(&p).expect("open payload");

    let mut fds = [0i32; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0, "pipe() failed");
    let write_end = unsafe { OwnedFd::from_raw_fd(fds[1]) };
    assert_eq!(unsafe { libc::close(fds[0]) }, 0, "close(read end) failed");

    let mut cmd = Command::new(prog);
    cmd.stdin(Stdio::from(f))
        .stdout(Stdio::from(write_end))
        .stderr(Stdio::piped());
    unsafe {
        cmd.pre_exec(|| {
            // Inherited across the following `execve`.
            libc::signal(libc::SIGPIPE, libc::SIG_IGN);
            Ok(())
        });
    }
    let out = cmd
        .output()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", prog.display()));
    let _ = fs::remove_file(&p);
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

/// Runs `prog` under a `RLIMIT_AS` (address-space) limit of `bytes`.
///
/// glibc degrades gracefully when an allocation fails (`printf` falls back to
/// the `FILE`'s `_shortbuf`), while a failed Rust allocation aborts the process
/// with a message on `stderr` — so this pins the translation's allocation
/// behaviour, not just its arithmetic.
pub fn run_with_address_space_limit(prog: &Path, stdin: &[u8], bytes: u64) -> Outcome {
    use std::os::unix::process::CommandExt;

    let p = unique_path("stdin");
    fs::write(&p, stdin).expect("write payload");
    let f = fs::File::open(&p).expect("open payload");

    let mut cmd = Command::new(prog);
    cmd.stdin(Stdio::from(f))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    unsafe {
        cmd.pre_exec(move || {
            let lim = libc::rlimit {
                rlim_cur: bytes,
                rlim_max: bytes,
            };
            libc::setrlimit(libc::RLIMIT_AS, &lim);
            Ok(())
        });
    }
    let out = cmd
        .output()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", prog.display()));
    let _ = fs::remove_file(&p);
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

/// Runs `prog` `n` times in a row, all sharing **one** file offset (each child
/// gets a `dup` of the same descriptor), starting at `start_offset`.
///
/// Returns one entry per run plus a final entry holding whatever is still
/// unread. This only produces the C's `384 498 314` for `"42 99 7"` if the
/// offset really is restored to the first unconsumed byte after every run.
pub fn shared_runs(prog: &Path, payload: &[u8], n: usize, start_offset: u64) -> Vec<Vec<u8>> {
    let p = unique_path("seq");
    fs::write(&p, payload).expect("write payload");
    let mut f = fs::File::open(&p).expect("open payload");
    if start_offset > 0 {
        f.seek(SeekFrom::Start(start_offset)).expect("seek");
    }
    let mut outs = Vec::new();
    for _ in 0..n {
        let dup = f.try_clone().expect("dup");
        let out = Command::new(prog)
            .stdin(Stdio::from(dup))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .unwrap_or_else(|e| panic!("spawn {}: {e}", prog.display()));
        outs.push(out.stdout);
    }
    let mut rest = Vec::new();
    f.read_to_end(&mut rest).expect("read leftovers");
    outs.push(rest);
    let _ = fs::remove_file(&p);
    outs
}

/// Runs `prog` with `stdin` connected to an arbitrary existing path (used for
/// character devices such as `/dev/zero`, which no payload can emulate).
pub fn run_stdin_path(prog: &Path, path: &str) -> Outcome {
    let f = fs::File::open(path).unwrap_or_else(|e| panic!("open {path}: {e}"));
    let out = Command::new(prog)
        .stdin(Stdio::from(f))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", prog.display()));
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

// ---------------------------------------------------------------------------
// Calling `.so` exports through libloading
// ---------------------------------------------------------------------------

/// fd juggling must not overlap between test threads.
fn fd_lock() -> &'static Mutex<()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
}


/// Runs `f` with fd 1 pointing at a fresh temporary file and returns everything
/// that was written to it. Flushes the C stdio buffers of any loaded `.so`
/// before restoring fd 1, so `printf`-based output is included.
pub fn capture_fd1<R>(f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    let _guard = fd_lock().lock().unwrap_or_else(|e| e.into_inner());
    let path = unique_path("fd1");
    let file = fs::File::create(&path).expect("create capture file");
    let ret;
    let bytes;
    unsafe {
        use std::os::fd::AsRawFd;
        let saved = libc::dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(libc::dup2(file.as_raw_fd(), 1) >= 0, "dup2 onto fd 1 failed");
        ret = f();
        // Flush glibc's `stdout` buffer (the C `.so` uses `printf`).
        libc::fflush(std::ptr::null_mut());
        assert!(libc::dup2(saved, 1) >= 0, "restore fd 1 failed");
        libc::close(saved);
    }
    drop(file);
    bytes = fs::read(&path).expect("read capture file");
    let _ = fs::remove_file(&path);
    (ret, bytes)
}

/// A `dlopen`ed shared object under test.
pub struct Lib {
    pub name: &'static str,
    lib: libloading::Library,
}

impl Lib {
    pub fn open(name: &'static str, path: &Path) -> Lib {
        let lib = unsafe { libloading::Library::new(path) }
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()));
        Lib { name, lib }
    }

    /// Calls the exported `void driver(int)` and returns the bytes it wrote.
    pub fn driver(&self, x: i32) -> Vec<u8> {
        let sym: libloading::Symbol<unsafe extern "C" fn(i32)> = unsafe {
            self.lib
                .get(b"driver\0")
                .unwrap_or_else(|e| panic!("{}: dlsym driver failed: {e}", self.name))
        };
        let (_, bytes) = capture_fd1(|| unsafe { sym(x) });
        bytes
    }

    /// Calls the exported `void driver(int)` once per element of `xs`, capturing
    /// all of the emitted bytes in one go (one temporary file for the whole
    /// batch instead of one per call — the calls are independent, so this is
    /// equivalent and far cheaper).
    pub fn driver_batch(&self, xs: &[i32]) -> Vec<u8> {
        let sym: libloading::Symbol<unsafe extern "C" fn(i32)> = unsafe {
            self.lib
                .get(b"driver\0")
                .unwrap_or_else(|e| panic!("{}: dlsym driver failed: {e}", self.name))
        };
        let (_, bytes) = capture_fd1(|| {
            for &x in xs {
                unsafe { sym(x) };
            }
        });
        bytes
    }

    pub fn has_symbol(&self, name: &[u8]) -> bool {
        // `name` must be NUL-terminated.
        unsafe {
            self.lib
                .get::<libloading::Symbol<*const ()>>(name)
                .is_ok()
        }
    }
}

/// The C and Rust shared objects, opened once per test binary.
pub fn c_lib() -> &'static Lib {
    static L: OnceLock<Lib> = OnceLock::new();
    L.get_or_init(|| Lib::open("C", &c_so()))
}

pub fn rust_lib() -> &'static Lib {
    static L: OnceLock<Lib> = OnceLock::new();
    L.get_or_init(|| Lib::open("Rust", &rust_so()))
}

pub fn rust_lib_dev() -> &'static Lib {
    static L: OnceLock<Lib> = OnceLock::new();
    L.get_or_init(|| Lib::open("Rust(dev)", &rust_so_dev()))
}

// ---------------------------------------------------------------------------
// Assertions
// ---------------------------------------------------------------------------

pub fn show(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes.iter().take(120) {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\t' => s.push_str("\\t"),
            b'\r' => s.push_str("\\r"),
            0x0b => s.push_str("\\v"),
            0x0c => s.push_str("\\f"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    if bytes.len() > 120 {
        s.push_str(&format!("...(+{} bytes)", bytes.len() - 120));
    }
    s
}

/// Asserts the two executables behave identically for one payload.
#[track_caller]
pub fn assert_same_exe(row: &str, c: &Path, r: &Path, stdin: &[u8]) {
    let co = run(c, stdin);
    let ro = run(r, stdin);
    assert_eq!(
        co, ro,
        "[{row}] divergence for stdin={:?}\n  C   : {co:?}\n  Rust: {ro:?}",
        show(stdin)
    );
}

#[track_caller]
pub fn assert_same_outcome(row: &str, ctx: &str, c: &Outcome, r: &Outcome) {
    assert_eq!(
        c, r,
        "[{row}] divergence for {ctx}\n  C   : {c:?}\n  Rust: {r:?}"
    );
}

/// Asserts the exported `driver` symbols of both `.so`s emit the same bytes.
#[track_caller]
pub fn assert_same_driver(row: &str, x: i32) {
    let c = c_lib().driver(x);
    let r = rust_lib().driver(x);
    assert_eq!(
        c,
        r,
        "[{row}] driver({x}) divergence\n  C   : {}\n  Rust: {}",
        show(&c),
        show(&r)
    );
}

/// Asserts the exported `driver` symbols of both `.so`s emit the same bytes for
/// every element of `xs`, reporting the first offending argument.
#[track_caller]
pub fn assert_same_driver_batch(row: &str, xs: &[i32]) {
    let c = c_lib().driver_batch(xs);
    let r = rust_lib().driver_batch(xs);
    if c == r {
        return;
    }
    let cl: Vec<&[u8]> = c.split(|&b| b == b'\n').collect();
    let rl: Vec<&[u8]> = r.split(|&b| b == b'\n').collect();
    for (i, x) in xs.iter().enumerate() {
        let cline = cl.get(i).copied().unwrap_or(b"<missing>");
        let rline = rl.get(i).copied().unwrap_or(b"<missing>");
        assert_eq!(
            cline,
            rline,
            "[{row}] driver({x}) divergence (batch index {i})\n  C   : {}\n  Rust: {}",
            show(cline),
            show(rline)
        );
    }
    panic!(
        "[{row}] driver batch divergence with matching lines\n  C   : {}\n  Rust: {}",
        show(&c),
        show(&r)
    );
}

/// Asserts both executables agree for every payload in `payloads`.
#[track_caller]
pub fn assert_same_exe_all(row: &str, payloads: &[Vec<u8>]) {
    let c = c_exe();
    let r = rust_exe_release();
    for p in payloads {
        assert_same_exe(row, &c, &r, p);
    }
}

// ---------------------------------------------------------------------------
// Minimal runner for the `harness = false` test binaries
// ---------------------------------------------------------------------------

/// Collects the outcome of every case so one divergence does not hide the rest.
pub struct Runner {
    passed: Vec<String>,
    failed: Vec<(String, String)>,
}

impl Runner {
    pub fn new() -> Self {
        Runner {
            passed: Vec::new(),
            failed: Vec::new(),
        }
    }

    /// Runs one case. Nothing is printed while `f` executes, because `f` may
    /// have fd 1 redirected for output capture.
    pub fn case(&mut self, name: &str, f: impl FnOnce() + std::panic::UnwindSafe) {
        let prev = std::panic::take_hook();
        let msg = std::sync::Arc::new(Mutex::new(String::new()));
        let sink = std::sync::Arc::clone(&msg);
        std::panic::set_hook(Box::new(move |info| {
            *sink.lock().unwrap_or_else(|e| e.into_inner()) = format!("{info}");
        }));
        let res = std::panic::catch_unwind(f);
        std::panic::set_hook(prev);
        match res {
            Ok(()) => {
                eprintln!("  ok   {name}");
                self.passed.push(name.to_string());
            }
            Err(_) => {
                let m = msg.lock().unwrap_or_else(|e| e.into_inner()).clone();
                eprintln!("  FAIL {name}\n{m}");
                self.failed.push((name.to_string(), m));
            }
        }
    }

    /// Prints a summary to `stderr` and exits non-zero if anything failed.
    pub fn finish(self, suite: &str) {
        eprintln!(
            "\n{suite}: {} passed, {} failed",
            self.passed.len(),
            self.failed.len()
        );
        if !self.failed.is_empty() {
            for (n, m) in &self.failed {
                eprintln!("FAILED {n}: {m}");
            }
            std::process::exit(1);
        }
    }
}

/// Reads a whole file, panicking with context.
pub fn read_file(p: &Path) -> Vec<u8> {
    let mut v = Vec::new();
    fs::File::open(p)
        .unwrap_or_else(|e| panic!("open {}: {e}", p.display()))
        .read_to_end(&mut v)
        .expect("read");
    v
}
