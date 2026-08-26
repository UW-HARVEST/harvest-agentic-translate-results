//! Shared harness for the C-vs-Rust differential test-suite.
//!
//! Two comparison channels are provided, both of which exercise the *built
//! artifacts* — the Rust side is never called as a Rust function:
//!
//! * **`E` (executable)** — spawns `c_src/build/driver` and `target/<p>/driver`
//!   as subprocesses and compares stdout, stderr, exit code *and* terminating
//!   signal. This is the artifact `c_src/CMakeLists.txt` builds.
//! * **`S` (shared library)** — `dlopen()`s `target/csrc/libcdriver.so` and
//!   `target/<p>/libdriver.so` via `libloading` and calls the exported C symbols
//!   (`printLine`, `good`, `bad`, `main`) directly, capturing fd 1.
//!
//! Run `./build_all.sh` first; it produces all four artifacts.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::io::{Read, Write};
use std::os::unix::io::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

extern "C" {
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    fn pipe2(fds: *mut c_int, flags: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(status: c_int) -> !;
}

// ---------------------------------------------------------------------------
// Artifact locations
// ---------------------------------------------------------------------------

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `target/debug` or `target/release`, derived from the running test binary
/// (`target/<profile>/deps/<test>-<hash>`).
pub fn profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(Path::parent)
        .expect("target/<profile>")
        .to_path_buf()
}

fn require(p: PathBuf, what: &str) -> PathBuf {
    assert!(
        p.exists(),
        "missing {what}: {}\n\
         Build every artifact first:  ./build_all.sh",
        p.display()
    );
    p
}

/// C executable built by `c_src/CMakeLists.txt`.
pub fn c_exe() -> PathBuf {
    require(manifest_dir().join("c_src/build/driver"), "C executable")
}

/// Rust executable (`[[bin]] driver`).
pub fn rust_exe() -> PathBuf {
    require(profile_dir().join("driver"), "Rust executable")
}

/// `c_src/src/main.c` built as a shared library.
pub fn c_so() -> PathBuf {
    require(
        manifest_dir().join("target/csrc/libcdriver.so"),
        "C shared library",
    )
}

/// Rust cdylib (`[lib] crate-type = ["cdylib"]`).
pub fn rust_so() -> PathBuf {
    require(profile_dir().join("libdriver.so"), "Rust shared library")
}

/// Helper that dlopen()s a `.so` in a fresh process and calls one symbol.
pub fn so_runner() -> PathBuf {
    require(
        profile_dir().join("examples/so_runner"),
        "so_runner example",
    )
}

fn tmp_dir() -> PathBuf {
    let d = manifest_dir().join("target/difftest-tmp");
    std::fs::create_dir_all(&d).expect("create tmp dir");
    d
}

fn unique_path(tag: &str) -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    tmp_dir().join(format!(
        "{tag}-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ))
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed => reproducible property-style tests)
// ---------------------------------------------------------------------------

/// SplitMix64. Small, dependency-free and reproducible.
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

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Uniform in `0..n` (`n > 0`).
    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0);
        self.next_u64() % n
    }

    /// Uniform in `lo..=hi`.
    pub fn range(&mut self, lo: u64, hi: u64) -> u64 {
        lo + self.below(hi - lo + 1)
    }

    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }

    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len() as u64) as usize]
    }

    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

// ---------------------------------------------------------------------------
// Byte-diff reporting
// ---------------------------------------------------------------------------

pub fn hex(bytes: &[u8]) -> String {
    let shown = &bytes[..bytes.len().min(256)];
    let mut s = String::new();
    for b in shown {
        s.push_str(&format!("{b:02x} "));
    }
    if bytes.len() > shown.len() {
        s.push_str(&format!("... (+{} bytes)", bytes.len() - shown.len()));
    }
    s.trim_end().to_string()
}

pub fn pretty(bytes: &[u8]) -> String {
    let shown = &bytes[..bytes.len().min(160)];
    let mut s = String::new();
    for &b in shown {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    if bytes.len() > shown.len() {
        s.push_str("…");
    }
    s
}

pub fn assert_bytes_eq(label: &str, input: &[u8], c: &[u8], r: &[u8]) {
    if c != r {
        panic!(
            "\n[{label}] OUTPUT MISMATCH\n  input  ({} bytes): {}\n  C   ({} bytes): {}\n              hex: {}\n  Rust({} bytes): {}\n              hex: {}\n",
            input.len(),
            pretty(input),
            c.len(),
            pretty(c),
            hex(c),
            r.len(),
            pretty(r),
            hex(r)
        );
    }
}

// ---------------------------------------------------------------------------
// Channel E: executable vs executable
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Run {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub code: Option<i32>,
    pub signal: Option<i32>,
}

impl Run {
    fn describe(&self) -> String {
        format!(
            "code={:?} signal={:?} stdout={:?} ({}) stderr={:?}",
            self.code,
            self.signal,
            pretty(&self.stdout),
            hex(&self.stdout),
            pretty(&self.stderr)
        )
    }
}

/// How fd 0 is wired for a run.
#[derive(Copy, Clone, Debug)]
pub enum StdinKind {
    /// Anonymous pipe fed by the parent (the usual case).
    Pipe,
    /// A regular, seekable file.
    File,
    /// `/dev/null` — immediate EOF.
    DevNull,
    /// fd 0 closed before `exec` — every read fails with `EBADF`.
    Closed,
}

/// How fd 1 is wired for a run.
#[derive(Copy, Clone, Debug)]
pub enum StdoutKind {
    Pipe,
    File,
    DevNull,
    /// fd 1 closed before `exec` — every write fails with `EBADF`.
    Closed,
    /// A pipe whose read end is already closed — writes raise `SIGPIPE`.
    BrokenPipe,
}

/// Extra process-level knobs: `main.c`'s `main` takes no parameters and reads no
/// environment, so both must be provably irrelevant.
#[derive(Default, Clone)]
pub struct Extras<'a> {
    pub args: &'a [&'a str],
    pub envs: &'a [(&'a str, &'a str)],
}

pub fn run_exe(bin: &Path, input: &[u8], sin: StdinKind, sout: StdoutKind) -> Run {
    run_exe_extras(bin, input, sin, sout, &Extras::default())
}

pub fn run_exe_extras(
    bin: &Path,
    input: &[u8],
    sin: StdinKind,
    sout: StdoutKind,
    extras: &Extras<'_>,
) -> Run {
    let mut cmd = Command::new(bin);
    cmd.stderr(Stdio::piped());
    cmd.args(extras.args);
    for (k, v) in extras.envs {
        cmd.env(k, v);
    }

    // ---- stdin ----
    let mut feed_pipe = false;
    match sin {
        StdinKind::Pipe => {
            feed_pipe = true;
            cmd.stdin(Stdio::piped());
        }
        StdinKind::File => {
            let p = unique_path("stdin");
            std::fs::write(&p, input).expect("write stdin file");
            cmd.stdin(Stdio::from(
                std::fs::File::open(&p).expect("open stdin file"),
            ));
        }
        StdinKind::DevNull => {
            cmd.stdin(Stdio::null());
        }
        StdinKind::Closed => {
            cmd.stdin(Stdio::null());
            // SAFETY: `close` is async-signal-safe and the only thing done
            // between fork and exec.
            unsafe {
                cmd.pre_exec(|| {
                    close(0);
                    Ok(())
                })
            };
        }
    }

    // ---- stdout ----
    let mut stdout_file: Option<PathBuf> = None;
    match sout {
        StdoutKind::Pipe => {
            cmd.stdout(Stdio::piped());
        }
        StdoutKind::File => {
            let p = unique_path("stdout");
            cmd.stdout(Stdio::from(
                std::fs::File::create(&p).expect("create stdout file"),
            ));
            stdout_file = Some(p);
        }
        StdoutKind::DevNull => {
            cmd.stdout(Stdio::null());
        }
        StdoutKind::Closed => {
            cmd.stdout(Stdio::null());
            // SAFETY: see above.
            unsafe {
                cmd.pre_exec(|| {
                    close(1);
                    Ok(())
                })
            };
        }
        StdoutKind::BrokenPipe => {
            let mut fds = [0 as c_int; 2];
            assert_eq!(unsafe { pipe(fds.as_mut_ptr()) }, 0, "pipe()");
            // Close the read end *now*: the child's first write gets SIGPIPE.
            unsafe { close(fds[0]) };
            // SAFETY: fds[1] is a fresh, owned descriptor.
            let w = unsafe { OwnedFd::from_raw_fd(fds[1]) };
            cmd.stdout(Stdio::from(w));
        }
    }

    let mut child = cmd.spawn().unwrap_or_else(|e| panic!("spawn {bin:?}: {e}"));

    if feed_pipe {
        let mut sink = child.stdin.take().expect("stdin pipe");
        // A short-lived child may exit before draining the pipe, which makes
        // this write fail with EPIPE. That is expected and not part of the
        // compared behaviour.
        let _ = sink.write_all(input);
        drop(sink);
    }

    let out = child.wait_with_output().expect("wait_with_output");

    let stdout = match &stdout_file {
        Some(p) => std::fs::read(p).expect("read stdout file"),
        None => out.stdout,
    };
    Run {
        stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

/// Default plumbing: a pipe for small inputs, a regular file for large ones
/// (a >64 KiB pipe write would block on a child that has already exited).
fn default_stdin_kind(input: &[u8]) -> StdinKind {
    if input.len() <= 512 {
        StdinKind::Pipe
    } else {
        StdinKind::File
    }
}

/// Core Phase-B/C assertion for the executables.
pub fn assert_exe_same_with(label: &str, input: &[u8], sin: StdinKind, sout: StdoutKind) {
    let c = run_exe(&c_exe(), input, sin, sout);
    let r = run_exe(&rust_exe(), input, sin, sout);
    if c != r {
        panic!(
            "\n[{label}] EXECUTABLE MISMATCH (stdin={sin:?}, stdout={sout:?})\n  input ({} bytes): {}\n  C   : {}\n  Rust: {}\n",
            input.len(),
            pretty(input),
            c.describe(),
            r.describe()
        );
    }
    // The C `main` has a single `return 0;` and no `exit()` call anywhere.
    if c.signal.is_none() {
        assert_eq!(c.code, Some(0), "[{label}] C exit status must be 0");
    }
}

pub fn assert_exe_same(label: &str, input: &[u8]) {
    assert_exe_same_with(
        label,
        input,
        default_stdin_kind(input),
        StdoutKind::Pipe,
    );
}

/// Same as [`assert_exe_same`] but also passes argv entries / environment
/// variables to both executables.
pub fn assert_exe_same_extras(label: &str, input: &[u8], extras: &Extras<'_>) {
    let sin = default_stdin_kind(input);
    let sout = StdoutKind::Pipe;
    let c = run_exe_extras(&c_exe(), input, sin, sout, extras);
    let r = run_exe_extras(&rust_exe(), input, sin, sout, extras);
    if c != r {
        panic!(
            "\n[{label}] EXECUTABLE MISMATCH (args={:?}, envs={:?})\n  input ({} bytes): {}\n  C   : {}\n  Rust: {}\n",
            extras.args,
            extras.envs,
            input.len(),
            pretty(input),
            c.describe(),
            r.describe()
        );
    }
    assert_eq!(c.code, Some(0), "[{label}] C exit status must be 0");
}

pub fn assert_exe_same_str(label: &str, input: &str) {
    assert_exe_same(label, input.as_bytes());
}

// ---------------------------------------------------------------------------
// Channel E, shared-fd view: what the program leaves behind on stdin
// ---------------------------------------------------------------------------

/// Runs `bin` with fd 0 wired to a **regular file** the parent also holds a
/// (dup'ed, offset-sharing) handle to, then reads whatever is left.
///
/// This is how libc's exit-time repositioning of a seekable input stream becomes
/// observable: `{ ./driver; cat; } < file`.
pub fn exe_stdin_leftover_file(bin: &Path, input: &[u8]) -> (Run, Vec<u8>) {
    let p = unique_path("leftover-in");
    std::fs::write(&p, input).expect("write input file");
    let mut parent = std::fs::File::open(&p).expect("open input file");
    let child_fd = parent.try_clone().expect("dup input fd"); // shares the offset

    let out = Command::new(bin)
        .stdin(Stdio::from(child_fd))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run exe");

    let mut leftover = Vec::new();
    parent.read_to_end(&mut leftover).expect("read leftover");
    (
        Run {
            stdout: out.stdout,
            stderr: out.stderr,
            code: out.status.code(),
            signal: out.status.signal(),
        },
        leftover,
    )
}

/// Same, but fd 0 is a **pipe** (not seekable, so libc cannot give anything
/// back — whatever it buffered is gone). `input` must fit in the pipe buffer.
pub fn exe_stdin_leftover_pipe(bin: &Path, input: &[u8]) -> (Run, Vec<u8>) {
    assert!(input.len() < 60_000, "must fit in one pipe buffer");
    let mut fds = [0 as c_int; 2];
    // O_CLOEXEC matters here: a plain pipe() leaks BOTH ends into the child, and
    // the child holding a write end open means the pipe never reaches EOF — on
    // an empty input `scanf` would then block forever waiting for one.
    const O_CLOEXEC: c_int = 0o2_000_000;
    assert_eq!(
        unsafe { pipe2(fds.as_mut_ptr(), O_CLOEXEC) },
        0,
        "pipe2(O_CLOEXEC)"
    );
    // SAFETY: both are fresh, owned descriptors.
    let read_end = unsafe { OwnedFd::from_raw_fd(fds[0]) };
    let write_end = unsafe { OwnedFd::from_raw_fd(fds[1]) };

    let child_fd = read_end.try_clone().expect("dup read end");
    let child = Command::new(bin)
        .stdin(Stdio::from(child_fd))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn exe");

    {
        let mut w = std::fs::File::from(write_end);
        let _ = w.write_all(input);
    } // closing the write end lets the final read see EOF

    let out = child.wait_with_output().expect("wait");
    let mut leftover = Vec::new();
    std::fs::File::from(read_end)
        .read_to_end(&mut leftover)
        .expect("read leftover");
    (
        Run {
            stdout: out.stdout,
            stderr: out.stderr,
            code: out.status.code(),
            signal: out.status.signal(),
        },
        leftover,
    )
}

/// Asserts both executables leave stdin in the same state (and produced the same
/// output) for a seekable and a non-seekable fd 0.
pub fn assert_exe_leftover_same(label: &str, input: &[u8]) {
    for (kind, f) in [
        ("file", exe_stdin_leftover_file as fn(&Path, &[u8]) -> (Run, Vec<u8>)),
        ("pipe", exe_stdin_leftover_pipe),
    ] {
        if std::env::var_os("DIFFTEST_TRACE").is_some() {
            eprintln!("    trace {label}/{kind} len={}", input.len());
        }
        let (c_run, c_left) = f(&c_exe(), input);
        let (r_run, r_left) = f(&rust_exe(), input);
        if c_run != r_run {
            panic!(
                "\n[{label}/{kind}] OUTPUT MISMATCH\n  input ({} bytes)\n  C   : {}\n  Rust: {}\n",
                input.len(),
                c_run.describe(),
                r_run.describe()
            );
        }
        if c_left != r_left {
            panic!(
                "\n[{label}/{kind}] LEFTOVER-ON-STDIN MISMATCH\n  input ({} bytes): {}\n  \
                 C   left {} bytes: {}\n  Rust left {} bytes: {}\n",
                input.len(),
                pretty(input),
                c_left.len(),
                pretty(&c_left),
                r_left.len(),
                pretty(&r_left)
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Channel S: shared library vs shared library, in-process via libloading
// ---------------------------------------------------------------------------

/// Which of the two libraries to call.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Side {
    C,
    Rust,
}

struct Libs {
    c: libloading::Library,
    r: libloading::Library,
}

// Loading both libraries is fine: libloading uses RTLD_LOCAL, so the two
// identically-named symbol sets do not collide.
fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        // SAFETY: the paths are the two libraries built by ./build_all.sh.
        unsafe {
            Libs {
                c: libloading::Library::new(c_so()).expect("dlopen C .so"),
                r: libloading::Library::new(rust_so()).expect("dlopen Rust .so"),
            }
        }
    })
}

fn lib_of(side: Side) -> &'static libloading::Library {
    match side {
        Side::C => &libs().c,
        Side::Rust => &libs().r,
    }
}

/// Serialises the `fork()`s so that libc's stdio locks are provably unheld.
fn capture_lock() -> MutexGuard<'static, ()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    let m = L.get_or_init(|| Mutex::new(()));
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// Runs `f` — which calls into one of the dlopen()ed libraries — with fd 1
/// pointing at a fresh temporary file, and returns everything that was written.
///
/// The call happens in a `fork()`ed child so that the redirection cannot be seen
/// by anybody else: fd 1 is process-global, and libtest writes its own progress
/// lines to it from another thread, which would otherwise land in the capture
/// (that is a harness artefact, not a translation difference — it was observed
/// as a stray leading `\n`). The child inherits the parent's already-loaded
/// libraries, so this is still an in-process `libloading` call: no `exec`, and
/// the symbol pointer was resolved by the parent.
///
/// `_exit` is used on purpose so the child never runs atexit handlers or flushes
/// buffers the parent owns; `fflush(NULL)` before it drains the C library's
/// buffered `printf` output.
pub fn capture_fd1<F: FnOnce()>(f: F) -> Vec<u8> {
    let (out, status) = capture_fd1_status(f);
    assert!(
        status.is_clean(),
        "the captured call did not exit cleanly ({}); captured so far: {}",
        status.describe(),
        pretty(&out)
    );
    out
}

/// How a captured child terminated.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ChildStatus(pub c_int);

impl ChildStatus {
    pub fn signal(self) -> Option<c_int> {
        let s = self.0 & 0x7f;
        if s != 0 && s != 0x7f {
            Some(s)
        } else {
            None
        }
    }
    pub fn code(self) -> Option<c_int> {
        if self.0 & 0x7f == 0 {
            Some((self.0 >> 8) & 0xff)
        } else {
            None
        }
    }
    pub fn is_clean(self) -> bool {
        self.code() == Some(0)
    }
    pub fn describe(self) -> String {
        match (self.code(), self.signal()) {
            (Some(c), _) => format!("exited with code {c}"),
            (_, Some(s)) => format!("killed by signal {s}"),
            _ => format!("raw wait status {:#x}", self.0),
        }
    }
}

/// Like [`capture_fd1`] but *returns* how the child terminated instead of
/// requiring a clean exit.
///
/// Needed because the C `bad()`, called in isolation, dereferences an
/// uninitialised pointer: depending on what the caller left on the stack it
/// prints a NUL-terminated run of whatever it points at — or dies from `SIGSEGV`.
/// Both outcomes have been observed from the very same `main.c` (see ERRORS.md
/// row 22), so no assertion may presume a clean exit there.
pub fn capture_fd1_status<F: FnOnce()>(f: F) -> (Vec<u8>, ChildStatus) {
    let _guard = capture_lock();
    let path = unique_path("fd1");
    let file = std::fs::File::create(&path).expect("create capture file");
    let file_fd = file.as_raw_fd();

    // Nothing must be sitting in a stdio buffer when we fork.
    unsafe { fflush(std::ptr::null_mut()) };

    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork() failed");
    if pid == 0 {
        // ---- child ----
        if unsafe { dup2(file_fd, 1) } < 0 {
            unsafe { _exit(101) };
        }
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        unsafe { fflush(std::ptr::null_mut()) };
        unsafe { _exit(if res.is_ok() { 0 } else { 102 }) };
    }

    // ---- parent ----
    let mut status: c_int = 0;
    let w = unsafe { waitpid(pid, &mut status, 0) };
    assert_eq!(w, pid, "waitpid");
    drop(file);
    let out = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);
    (out, ChildStatus(status))
}

/// Calls `bad()` in the given library, tolerating the UB crash.
pub fn so_call_bad_tolerant(side: Side, times: usize) -> (Vec<u8>, ChildStatus) {
    let lib = lib_of(side);
    // SAFETY: `bad` is `void (*)(void)` in both libraries.
    let f: libloading::Symbol<unsafe extern "C" fn()> =
        unsafe { lib.get(b"bad\0") }.expect("symbol bad");
    capture_fd1_status(|| {
        for _ in 0..times {
            unsafe { f() }
        }
    })
}

/// Calls `printLine(payload)` in the given library. `None` passes a NULL pointer.
pub fn so_print_line(side: Side, payload: Option<&[u8]>) -> Vec<u8> {
    let lib = lib_of(side);
    // SAFETY: the symbol exists in both libraries with this signature.
    let f: libloading::Symbol<unsafe extern "C" fn(*const c_char)> =
        unsafe { lib.get(b"printLine\0") }.expect("printLine");
    let owned: Option<Vec<u8>> = payload.map(|p| {
        let mut v = p.to_vec();
        v.push(0); // NUL terminator
        v
    });
    capture_fd1(|| unsafe {
        match &owned {
            None => f(std::ptr::null()),
            Some(v) => f(v.as_ptr() as *const c_char),
        }
    })
}

/// Calls a `void (*)(void)` export (`good` / `bad`) `times` times.
pub fn so_call_void(side: Side, name: &str, times: usize) -> Vec<u8> {
    let lib = lib_of(side);
    let mut sym = name.as_bytes().to_vec();
    sym.push(0);
    // SAFETY: `good`/`bad` are `void (*)(void)` in both libraries.
    let f: libloading::Symbol<unsafe extern "C" fn()> =
        unsafe { lib.get(&sym) }.unwrap_or_else(|e| panic!("symbol {name}: {e}"));
    capture_fd1(|| {
        for _ in 0..times {
            unsafe { f() }
        }
    })
}

pub fn assert_so_print_line_same(label: &str, payload: Option<&[u8]>) {
    let c = so_print_line(Side::C, payload);
    let r = so_print_line(Side::Rust, payload);
    assert_bytes_eq(label, payload.unwrap_or(b"<NULL>"), &c, &r);
}

// ---------------------------------------------------------------------------
// Channel S, hermetic: `main` through the .so, in a fresh subprocess
// ---------------------------------------------------------------------------

/// Runs `so_runner <so> <symbol> [payload-hex|--null]` with `input` on stdin.
pub fn run_so_subprocess(so: &Path, symbol: &str, arg: Option<&str>, input: &[u8]) -> Run {
    let mut cmd = Command::new(so_runner());
    cmd.arg(so).arg(symbol);
    if let Some(a) = arg {
        cmd.arg(a);
    }
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn so_runner");
    {
        let mut sink = child.stdin.take().expect("stdin");
        let _ = sink.write_all(input);
    }
    let out = child.wait_with_output().expect("wait so_runner");
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

/// Runs `so_runner <so> main <times>` with fd 0 on a regular file the parent also
/// holds an offset-sharing handle to, and returns what is left unread.
///
/// The leftover *is* the stream position `scanf` left behind, so this measures
/// the push-back and buffering semantics **without observing `bad()`'s output**
/// — which matters, because `bad()`'s undefined value can be anything (it was
/// caught printing `"string"`, the pointer a preceding `good()` left in the same
/// stack slot). Stream position is fully defined; stdout on a `bad()` path is not.
pub fn so_main_repeat_leftover(so: &Path, input: &[u8], times: usize) -> (Run, Vec<u8>) {
    let p = unique_path("so-left-in");
    std::fs::write(&p, input).expect("write input file");
    let mut parent = std::fs::File::open(&p).expect("open input file");
    let child_fd = parent.try_clone().expect("dup input fd");

    let out = Command::new(so_runner())
        .arg(so)
        .arg("main")
        .arg(times.to_string())
        .stdin(Stdio::from(child_fd))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run so_runner");

    let mut leftover = Vec::new();
    parent.read_to_end(&mut leftover).expect("read leftover");
    (
        Run {
            stdout: out.stdout,
            stderr: out.stderr,
            code: out.status.code(),
            signal: out.status.signal(),
        },
        leftover,
    )
}

/// Asserts both libraries leave the shared stdin at the same position after
/// `times` calls to the exported `main`. UB-free (see above).
pub fn assert_so_main_repeat_leftover_same(label: &str, input: &[u8], times: usize) {
    let (c_run, c_left) = so_main_repeat_leftover(&c_so(), input, times);
    let (r_run, r_left) = so_main_repeat_leftover(&rust_so(), input, times);
    if c_left != r_left || c_run.code != r_run.code || c_run.signal != r_run.signal {
        panic!(
            "\n[{label}] .so main() x{times} STREAM-POSITION MISMATCH\n  input ({} bytes): {}\n  \
             C   left {} bytes: {}  (code={:?} signal={:?})\n  \
             Rust left {} bytes: {}  (code={:?} signal={:?})\n",
            input.len(),
            pretty(input),
            c_left.len(),
            pretty(&c_left),
            c_run.code,
            c_run.signal,
            r_left.len(),
            pretty(&r_left),
            r_run.code,
            r_run.signal
        );
    }
}

/// Compares stdout byte-for-byte for repeated `main` calls, restricted to inputs
/// where **every** call converts a non-zero value.
///
/// That restriction is what makes the comparison meaningful: `bad()` never runs,
/// so no undefined value is ever printed and the whole output is specified.
pub fn assert_so_main_repeat_all_good(label: &str, input: &[u8], times: usize) {
    let n = times.to_string();
    let c = run_so_subprocess(&c_so(), "main", Some(&n), input);
    let r = run_so_subprocess(&rust_so(), "main", Some(&n), input);
    let want: Vec<u8> = b"string\n".repeat(times);
    assert_eq!(
        c.stdout,
        want,
        "[{label}] this row requires every one of the {times} calls to convert a \
         non-zero value (so that bad() never runs); input {:?} does not",
        pretty(input)
    );
    assert_bytes_eq(label, input, &c.stdout, &r.stdout);
    assert_eq!(c.code, r.code, "[{label}] exit code");
    assert_eq!(c.signal, r.signal, "[{label}] signal");
}

/// Compares the `.so`-exported `main` of both libraries on the same stdin.
pub fn assert_so_main_same(label: &str, input: &[u8]) {
    let c = run_so_subprocess(&c_so(), "main", None, input);
    let r = run_so_subprocess(&rust_so(), "main", None, input);
    if c.stdout != r.stdout || c.code != r.code || c.signal != r.signal {
        panic!(
            "\n[{label}] .so main() MISMATCH\n  input ({} bytes): {}\n  C   : {}\n  Rust: {}\n",
            input.len(),
            pretty(input),
            c.describe(),
            r.describe()
        );
    }
    assert_eq!(c.code, Some(0), "[{label}] .so main must return 0");
}

pub fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
