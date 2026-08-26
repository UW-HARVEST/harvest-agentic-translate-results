//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both implementations are always reached through a `dlopen`'d shared object
//! (`libloading`) — the Rust functions are never called directly, so the
//! `#[no_mangle] extern "C"` export wrappers in `src/lib.rs` are under test as
//! well.
//!
//! * C shared object:    `build_c/libcdriver.so`   (built on demand from the
//!                        untouched `c_src/src/main.c`, same flags CMake uses)
//! * Rust shared object: `target/<profile>/libdriver.so` (the cdylib)
//!
//! `driver` writes to stdout and `main` reads stdin, so every call is made in a
//! `fork`ed child with fd 0 / fd 1 set up there.  Doing the redirect in the
//! child (instead of in the test process) means
//!   * libtest's own progress output on fd 1 can never end up in the captured
//!     bytes, and tests may still run in parallel, and
//!   * no stdio state (glibc `FILE` buffers, sticky EOF flags, Rust's `Stdout`
//!     handle) leaks from one invocation to the next.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::CString;
use std::fs;
use std::os::raw::c_int;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// paths / building
// ---------------------------------------------------------------------------

pub fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `target/<profile>/` — derived from the location of the test executable.
pub fn target_profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test>-<hash>
    exe.parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .to_path_buf()
}

/// `target/<profile>/libdriver.so` — the cdylib produced by `cargo build`.
///
/// `cargo test` alone does not build cdylib targets, so if the artifact is
/// absent it is produced here with a direct `rustc` invocation from the very
/// same `src/lib.rs` (the crate has no dependencies, so this is exactly what
/// cargo would do).  `run_all.sh` runs `cargo build` first, in which case this
/// falls straight through to cargo's artifact.
pub fn rust_so_path() -> PathBuf {
    static ONCE: OnceLock<PathBuf> = OnceLock::new();
    ONCE.get_or_init(|| {
        let dir = target_profile_dir();
        let so = dir.join("libdriver.so");
        if !so.exists() {
            let release = dir.file_name().map(|n| n == "release").unwrap_or(false);
            let tmp = dir.join(format!("libdriver.{}.so", std::process::id()));
            let mut cmd = std::process::Command::new(
                std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()),
            );
            cmd.args([
                "--edition",
                "2021",
                "--crate-type",
                "cdylib",
                "--crate-name",
                "driver",
            ])
            .arg(crate_dir().join("src/lib.rs"))
            .arg("-o")
            .arg(&tmp);
            if release {
                cmd.args(["-O", "-C", "panic=abort"]);
            }
            let status = cmd.status().expect("spawn rustc");
            assert!(status.success(), "rustc failed to build the Rust cdylib");
            let _ = fs::rename(&tmp, &so);
        }
        assert!(
            so.exists(),
            "Rust cdylib not found at {}. Run `cargo build` (same profile) first.",
            so.display()
        );
        so
    })
    .clone()
}

/// Builds `build_c/libcdriver.so` from the untouched C source if necessary.
pub fn c_so_path() -> PathBuf {
    static ONCE: OnceLock<PathBuf> = OnceLock::new();
    ONCE.get_or_init(|| {
        let dir = crate_dir().join("build_c");
        let so = dir.join("libcdriver.so");
        if !so.exists() {
            fs::create_dir_all(&dir).expect("mkdir build_c");
            let tmp = dir.join(format!("libcdriver.{}.so", std::process::id()));
            let src = crate_dir().join("c_src/src/main.c");
            let status = std::process::Command::new("gcc")
                .args(["-shared", "-fPIC", "-fno-strict-aliasing", "-O0"])
                .arg(&src)
                .arg("-o")
                .arg(&tmp)
                .status()
                .expect("spawn gcc");
            assert!(status.success(), "gcc failed to build the C shared object");
            // rename is atomic, so parallel test binaries cannot see a partial file
            let _ = fs::rename(&tmp, &so);
        }
        so
    })
    .clone()
}

pub fn c_exe_path() -> PathBuf {
    let p = crate_dir().join("c_src/build/driver");
    assert!(
        p.exists(),
        "C executable not found at {}. Build it with cmake first \
         (cmake -S c_src -B c_src/build && cmake --build c_src/build).",
        p.display()
    );
    p
}

pub fn rust_exe_path() -> PathBuf {
    let p = target_profile_dir().join("driver");
    assert!(
        p.exists(),
        "Rust executable not found at {}. Run `cargo build` first.",
        p.display()
    );
    p
}

// ---------------------------------------------------------------------------
// loaded libraries
// ---------------------------------------------------------------------------

pub type DriverFn = unsafe extern "C" fn(c_int);
pub type MainFn = unsafe extern "C" fn() -> c_int;

pub struct Impl {
    pub name: &'static str,
    lib: Library,
}

impl Impl {
    fn load(name: &'static str, path: &Path) -> Impl {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()));
        Impl { name, lib }
    }

    /// Resolves `driver` through the dynamic symbol table of the `.so`.
    pub fn driver_fn(&self) -> DriverFn {
        let s: Symbol<DriverFn> = unsafe { self.lib.get(b"driver\0") }
            .unwrap_or_else(|e| panic!("{}: symbol `driver` missing: {e}", self.name));
        *s
    }

    /// Resolves `main` through the dynamic symbol table of the `.so`.
    pub fn main_fn(&self) -> MainFn {
        let s: Symbol<MainFn> = unsafe { self.lib.get(b"main\0") }
            .unwrap_or_else(|e| panic!("{}: symbol `main` missing: {e}", self.name));
        *s
    }

    pub fn has_symbol(&self, name: &str) -> bool {
        let mut n = name.as_bytes().to_vec();
        n.push(0);
        unsafe { self.lib.get::<*const ()>(&n) }.is_ok()
    }
}

pub fn c_impl() -> &'static Impl {
    static L: OnceLock<Impl> = OnceLock::new();
    L.get_or_init(|| Impl::load("C", &c_so_path()))
}

pub fn rust_impl() -> &'static Impl {
    static L: OnceLock<Impl> = OnceLock::new();
    L.get_or_init(|| Impl::load("Rust", &rust_so_path()))
}

// ---------------------------------------------------------------------------
// fd plumbing
// ---------------------------------------------------------------------------

fn tmp_path(tag: &str) -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let base = std::env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"));
    base.join(format!(
        "cdiff-{}-{}-{}-{}",
        std::process::id(),
        tag,
        unsafe { libc::gettid() },
        n
    ))
}

fn open_ro(path: &Path) -> c_int {
    let c = CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
    let fd = unsafe { libc::open(c.as_ptr(), libc::O_RDONLY) };
    assert!(fd >= 0, "open {} for reading failed", path.display());
    fd
}

fn open_trunc_wo(path: &Path) -> c_int {
    let c = CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
    let fd = unsafe {
        libc::open(
            c.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC,
            0o600,
        )
    };
    assert!(fd >= 0, "open {} for writing failed", path.display());
    fd
}

/// How fd 0 is set up for an invocation.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stdin {
    /// fd 0 untouched (the callee is not supposed to read it)
    Inherit,
    /// seekable regular file holding the input
    File,
    /// non-seekable pipe carrying the input
    Pipe,
    /// `/dev/null` (input bytes ignored)
    DevNull,
    /// fd 0 closed before the call
    Closed,
    /// fd 0 is an open directory
    Directory,
}

pub struct RunResult {
    pub stdout: Vec<u8>,
    /// only used by the helpers that report extra information (e.g. the bytes
    /// left unconsumed on fd 0) out of band on fd 2
    pub stderr: Vec<u8>,
    pub status: i32,
}

/// Runs `body` in a forked child with fd 0 set up from `input`/`kind` and fd 1
/// either redirected to a temp file (`capture_stdout`) or closed.
///
/// The redirect happens in the child, so the test process' own descriptors are
/// never touched.
fn fork_capture(
    input: &[u8],
    kind: Stdin,
    capture_stdout: bool,
    body: impl FnOnce() -> i32,
) -> RunResult {
    let in_path = tmp_path("in");
    let out_path = tmp_path("out");
    if kind == Stdin::File {
        fs::write(&in_path, input).expect("write stdin file");
    }
    let err_path = tmp_path("err");
    fs::write(&out_path, b"").expect("create stdout file");
    fs::write(&err_path, b"").expect("create stderr file");
    let out_fd = open_trunc_wo(&out_path);
    let err_fd = open_trunc_wo(&err_path);

    let mut pipe_fds = [-1i32; 2];
    let in_fd: c_int = match kind {
        Stdin::Inherit => -2,
        Stdin::File => open_ro(&in_path),
        Stdin::DevNull => open_ro(Path::new("/dev/null")),
        Stdin::Directory => open_ro(&std::env::temp_dir()),
        Stdin::Closed => -1,
        Stdin::Pipe => {
            assert_eq!(unsafe { libc::pipe(pipe_fds.as_mut_ptr()) }, 0, "pipe()");
            pipe_fds[0]
        }
    };

    // Never let pending glibc stdio bytes of the test process be duplicated into
    // the child's buffers.
    unsafe { libc::fflush(std::ptr::null_mut()) };

    let pid = unsafe { libc::fork() };
    assert!(pid >= 0, "fork failed");

    if pid == 0 {
        // ---- child ------------------------------------------------------
        unsafe {
            if kind == Stdin::Pipe {
                libc::close(pipe_fds[1]);
            }
            if in_fd >= 0 {
                libc::dup2(in_fd, 0);
                if in_fd != 0 {
                    libc::close(in_fd);
                }
            } else if in_fd == -1 {
                libc::close(0);
            }
            if capture_stdout {
                libc::dup2(out_fd, 1);
            } else {
                libc::close(1);
            }
            libc::dup2(err_fd, 2);
            if out_fd > 2 {
                libc::close(out_fd);
            }
            if err_fd > 2 {
                libc::close(err_fd);
            }
            let rc = body();
            // flush the C implementation's stdio buffers (the Rust one flushes
            // explicitly inside `driver`)
            libc::fflush(std::ptr::null_mut());
            libc::_exit(rc);
        }
    }

    // ---- parent ---------------------------------------------------------
    if kind == Stdin::Pipe {
        unsafe { libc::close(pipe_fds[0]) };
        let mut off = 0usize;
        while off < input.len() {
            let n = unsafe {
                libc::write(
                    pipe_fds[1],
                    input[off..].as_ptr() as *const libc::c_void,
                    input.len() - off,
                )
            };
            if n <= 0 {
                break; // child exited without draining the pipe (EPIPE)
            }
            off += n as usize;
        }
        unsafe { libc::close(pipe_fds[1]) };
    } else if in_fd >= 0 {
        unsafe { libc::close(in_fd) };
    }

    let mut status: c_int = 0;
    let w = unsafe { libc::waitpid(pid, &mut status, 0) };
    assert_eq!(w, pid, "waitpid failed");
    unsafe {
        libc::close(out_fd);
        libc::close(err_fd);
    }

    let stdout = fs::read(&out_path).expect("read captured stdout");
    let stderr = fs::read(&err_path).expect("read captured stderr");
    let _ = fs::remove_file(&out_path);
    let _ = fs::remove_file(&err_path);
    let _ = fs::remove_file(&in_path);

    let code = if libc::WIFEXITED(status) {
        libc::WEXITSTATUS(status)
    } else if libc::WIFSIGNALED(status) {
        -libc::WTERMSIG(status)
    } else {
        i32::MIN
    };

    RunResult {
        stdout,
        stderr,
        status: code,
    }
}

/// Runs an arbitrary `body` (typically a raw call into one of the `.so`s) in a
/// forked child with fd 1 captured, and returns what it wrote.
pub fn fork_capture_stdout(body: impl FnOnce() -> i32) -> Vec<u8> {
    let r = fork_capture(&[], Stdin::Inherit, true, body);
    assert_eq!(r.status, 0, "captured child exited with {}", r.status);
    r.stdout
}

/// Calls `driver(x)` once through the `.so` export and returns its stdout.
pub fn call_driver(imp: &Impl, x: i32) -> Vec<u8> {
    call_driver_many(imp, &[x])
}

/// Calls `driver(x)` for every `x` in `xs` (in one child process, so repeated
/// invocations are covered too) and returns the concatenated stdout.
pub fn call_driver_many(imp: &Impl, xs: &[i32]) -> Vec<u8> {
    let f = imp.driver_fn();
    let r = fork_capture(&[], Stdin::Inherit, true, || {
        for &x in xs {
            unsafe { f(x) };
        }
        0
    });
    assert_eq!(
        r.status, 0,
        "{}: driver() child exited with {}",
        imp.name, r.status
    );
    r.stdout
}

/// Calls the exported `main()` with fd 0 fed from `input` according to `kind`.
pub fn call_main(imp: &Impl, input: &[u8], kind: Stdin) -> RunResult {
    let f = imp.main_fn();
    fork_capture(input, kind, true, || unsafe { f() })
}

/// Same, but fd 1 is closed in the child instead of being redirected.
pub fn call_main_stdout_closed(imp: &Impl, input: &[u8], kind: Stdin) -> RunResult {
    let f = imp.main_fn();
    fork_capture(input, kind, false, || unsafe { f() })
}

/// Calls the exported `main()` `n` times **in the same process**, which is where
/// the process-wide stdin buffer (glibc's `stdin` `FILE`) and its sticky EOF
/// flag become observable.
pub fn call_main_n(imp: &Impl, input: &[u8], kind: Stdin, n: usize) -> RunResult {
    let f = imp.main_fn();
    fork_capture(input, kind, true, move || {
        let mut rc = 0;
        for _ in 0..n {
            rc = unsafe { f() };
        }
        rc
    })
}

/// Calls the exported `main()` once and then drains whatever is still readable
/// from fd 0, reporting those bytes on fd 2.  This makes the *descriptor state*
/// the implementation leaves behind observable through the FFI boundary (how
/// much of stdin was consumed / buffered away).
///
/// The drain happens *before* the harness' `fflush`, because `fflush(NULL)` on
/// glibc also syncs *input* streams (seeking the descriptor back over the
/// unconsumed buffer) and would therefore measure the harness instead of the
/// implementation.  The leftover bytes go to fd 2 so they cannot interleave with
/// the buffered stdout of the C implementation.
pub fn call_main_and_drain(imp: &Impl, input: &[u8], kind: Stdin) -> RunResult {
    let f = imp.main_fn();
    fork_capture(input, kind, true, move || {
        let rc = unsafe { f() };
        unsafe {
            let mut buf = [0u8; 8192];
            loop {
                let n = libc::read(0, buf.as_mut_ptr() as *mut libc::c_void, buf.len());
                if n <= 0 {
                    break;
                }
                libc::write(2, buf.as_ptr() as *const libc::c_void, n as usize);
            }
        }
        rc
    })
}

/// Runs a real executable with `input` on stdin (a pipe) and returns its stdout
/// and exit status.
pub fn run_exe(path: &Path, input: &[u8]) -> RunResult {
    use std::io::Write as _;
    use std::process::{Command, Stdio};

    let mut child = Command::new(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {} failed: {e}", path.display()));
    {
        let mut si = child.stdin.take().unwrap();
        let _ = si.write_all(input);
        let _ = si.flush();
    }
    let out = child.wait_with_output().expect("wait_with_output");
    RunResult {
        stdout: out.stdout,
        stderr: out.stderr,
        status: out.status.code().unwrap_or(i32::MIN),
    }
}

// ---------------------------------------------------------------------------
// assertions
// ---------------------------------------------------------------------------

pub fn show(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            0x0b => s.push_str("\\v"),
            0x0c => s.push_str("\\f"),
            b'\\' => s.push_str("\\\\"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{:02x}", b)),
        }
    }
    s
}

fn abbrev(bytes: &[u8]) -> String {
    if bytes.len() <= 96 {
        show(bytes)
    } else {
        format!(
            "{}...{} ({} bytes)",
            show(&bytes[..48]),
            show(&bytes[bytes.len() - 24..]),
            bytes.len()
        )
    }
}

/// Differential check of the exported `driver` symbol for one value.
pub fn assert_driver_eq(x: i32) {
    assert_driver_batch_eq("driver", &[x]);
}

/// Differential check of the exported `driver` symbol over a batch of values.
/// Every call crosses the FFI boundary through the `.so` export.
pub fn assert_driver_batch_eq(label: &str, xs: &[i32]) {
    let c = call_driver_many(c_impl(), xs);
    let r = call_driver_many(rust_impl(), xs);
    if c == r {
        assert_eq!(
            c.len(),
            xs.len() * 9,
            "{label}: unexpected output length {} for {} values ({})",
            c.len(),
            xs.len(),
            abbrev(&c)
        );
        return;
    }
    // pinpoint the first differing value
    let cl: Vec<&[u8]> = c.split(|&b| b == b'\n').collect();
    let rl: Vec<&[u8]> = r.split(|&b| b == b'\n').collect();
    for (i, x) in xs.iter().enumerate() {
        let a = cl.get(i).copied().unwrap_or(b"<missing>");
        let b = rl.get(i).copied().unwrap_or(b"<missing>");
        if a != b {
            panic!(
                "{label}: driver({x}) [0x{x:08x}] (call #{i}) mismatch:\n  C   : {}\n  Rust: {}",
                show(a),
                show(b)
            );
        }
    }
    panic!(
        "{label}: outputs differ but no per-call difference found:\n  C   : {}\n  Rust: {}",
        abbrev(&c),
        abbrev(&r)
    );
}

/// Differential check of the exported `main` symbol.
pub fn assert_main_eq(input: &[u8], kind: Stdin) {
    let c = call_main(c_impl(), input, kind);
    let r = call_main(rust_impl(), input, kind);
    assert_eq!(
        c.stdout,
        r.stdout,
        "main() stdout mismatch for stdin={kind:?} input {}:\n  C   : {}\n  Rust: {}",
        abbrev(input),
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.status,
        r.status,
        "main() exit status mismatch for stdin={kind:?} input {}: C={} Rust={}",
        abbrev(input),
        c.status,
        r.status
    );
}

/// Differential check of `main` with fd 1 closed.
pub fn assert_main_eq_stdout_closed(input: &[u8], kind: Stdin) {
    let c = call_main_stdout_closed(c_impl(), input, kind);
    let r = call_main_stdout_closed(rust_impl(), input, kind);
    assert_eq!(
        c.stdout,
        r.stdout,
        "main() [stdout closed] output mismatch for input {}",
        abbrev(input)
    );
    assert_eq!(
        c.status,
        r.status,
        "main() [stdout closed] exit status mismatch for input {}: C={} Rust={}",
        abbrev(input),
        c.status,
        r.status
    );
}

/// Differential check of `n` consecutive `main()` calls in one process.
pub fn assert_main_n_eq(input: &[u8], kind: Stdin, n: usize) {
    let c = call_main_n(c_impl(), input, kind, n);
    let r = call_main_n(rust_impl(), input, kind, n);
    assert_eq!(
        c.stdout,
        r.stdout,
        "{n} consecutive main() calls, stdin={kind:?}, input {}:\n  C   : {}\n  Rust: {}",
        abbrev(input),
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(c.status, r.status, "exit status after {n} main() calls");
}

/// Differential check of the descriptor state `main()` leaves on fd 0.
pub fn assert_main_drain_eq(input: &[u8], kind: Stdin) {
    let c = call_main_and_drain(c_impl(), input, kind);
    let r = call_main_and_drain(rust_impl(), input, kind);
    assert_eq!(
        c.stdout,
        r.stdout,
        "main() stdout mismatch, stdin={kind:?}, input {}:\n  C   : {}\n  Rust: {}",
        abbrev(input),
        abbrev(&c.stdout),
        abbrev(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "bytes left unconsumed on fd 0 differ, stdin={kind:?}, input {}:\n  C   : {}\n  Rust: {}",
        abbrev(input),
        abbrev(&c.stderr),
        abbrev(&r.stderr)
    );
    assert_eq!(c.status, r.status, "exit status");
}

/// Runs an executable with stdin bound to a **seekable file** holding `input`,
/// then returns its stdout together with the bytes still unread on the shared
/// descriptor afterwards (the file offset the program left behind).
pub fn run_exe_file_stdin(path: &Path, input: &[u8]) -> (RunResult, Vec<u8>) {
    use std::io::Read as _;
    use std::process::{Command, Stdio};

    let in_path = tmp_path("exein");
    fs::write(&in_path, input).expect("write stdin file");
    let mut f = fs::File::open(&in_path).expect("open stdin file");
    // `Stdio::from(File)` dups the descriptor, so parent and child share the
    // file offset -- exactly like a shell redirect shared with another reader.
    let dup = f.try_clone().expect("dup stdin file");
    let out = Command::new(path)
        .stdin(Stdio::from(dup))
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .unwrap_or_else(|e| panic!("run {} failed: {e}", path.display()));
    let mut leftover = Vec::new();
    f.read_to_end(&mut leftover).expect("read leftover");
    let _ = fs::remove_file(&in_path);
    (
        RunResult {
            stdout: out.stdout,
            stderr: out.stderr,
            status: out.status.code().unwrap_or(i32::MIN),
        },
        leftover,
    )
}

/// Runs an executable with stdin bound to a **pipe** that already contains all
/// of `input` (write end closed before the spawn, so the reads are
/// deterministic), and returns its stdout plus the bytes left in the pipe.
pub fn run_exe_pipe_stdin(path: &Path, input: &[u8]) -> (RunResult, Vec<u8>) {
    use std::os::fd::FromRawFd;
    use std::process::{Command, Stdio};

    assert!(
        input.len() <= 60_000,
        "input must fit the pipe capacity for this helper"
    );
    let mut fds = [-1i32; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0, "pipe()");
    let (rd, wr) = (fds[0], fds[1]);
    let mut off = 0usize;
    while off < input.len() {
        let n = unsafe {
            libc::write(
                wr,
                input[off..].as_ptr() as *const libc::c_void,
                input.len() - off,
            )
        };
        assert!(n > 0, "filling the pipe failed");
        off += n as usize;
    }
    unsafe { libc::close(wr) };

    let child_rd = unsafe { libc::dup(rd) };
    assert!(child_rd >= 0);
    let out = Command::new(path)
        .stdin(unsafe { Stdio::from_raw_fd(child_rd) })
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .unwrap_or_else(|e| panic!("run {} failed: {e}", path.display()));

    let mut leftover = Vec::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = unsafe { libc::read(rd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
        if n <= 0 {
            break;
        }
        leftover.extend_from_slice(&buf[..n as usize]);
    }
    unsafe { libc::close(rd) };
    (
        RunResult {
            stdout: out.stdout,
            stderr: out.stderr,
            status: out.status.code().unwrap_or(i32::MIN),
        },
        leftover,
    )
}

/// Differential check of stdout **and** of the stdin bytes the program leaves
/// unconsumed, for a seekable stdin (where libc's exit-time seek-back applies).
pub fn assert_exe_file_stdin_eq(input: &[u8]) {
    let (cr, cl) = run_exe_file_stdin(&c_exe_path(), input);
    let (rr, rl) = run_exe_file_stdin(&rust_exe_path(), input);
    assert_eq!(
        cr.stdout,
        rr.stdout,
        "program stdout mismatch (file stdin) for input {}:\n  C   : {}\n  Rust: {}",
        abbrev(input),
        show(&cr.stdout),
        show(&rr.stdout)
    );
    assert_eq!(cr.status, rr.status, "exit status (file stdin)");
    assert_eq!(
        cl,
        rl,
        "unconsumed stdin mismatch (file stdin) for input {}:\n  C   : {}\n  Rust: {}",
        abbrev(input),
        abbrev(&cl),
        abbrev(&rl)
    );
}

/// Same for a pre-filled pipe (non-seekable: libc cannot seek back, so the
/// whole buffered block stays consumed).
pub fn assert_exe_pipe_stdin_eq(input: &[u8]) {
    let (cr, cl) = run_exe_pipe_stdin(&c_exe_path(), input);
    let (rr, rl) = run_exe_pipe_stdin(&rust_exe_path(), input);
    assert_eq!(
        cr.stdout,
        rr.stdout,
        "program stdout mismatch (pipe stdin) for input {}:\n  C   : {}\n  Rust: {}",
        abbrev(input),
        show(&cr.stdout),
        show(&rr.stdout)
    );
    assert_eq!(cr.status, rr.status, "exit status (pipe stdin)");
    assert_eq!(
        cl,
        rl,
        "unconsumed stdin mismatch (pipe stdin) for input {}:\n  C   : {}\n  Rust: {}",
        abbrev(input),
        abbrev(&cl),
        abbrev(&rl)
    );
}

/// Differential check of the two real executables (end-to-end pipeline).
pub fn assert_exe_eq(input: &[u8]) {
    let c = run_exe(&c_exe_path(), input);
    let r = run_exe(&rust_exe_path(), input);
    assert_eq!(
        c.stdout,
        r.stdout,
        "program stdout mismatch for input {}:\n  C   : {}\n  Rust: {}",
        abbrev(input),
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.status,
        r.status,
        "program exit status mismatch for input {}: C={} Rust={}",
        abbrev(input),
        c.status,
        r.status
    );
}

// ---------------------------------------------------------------------------
// deterministic RNG (SplitMix64) — no external crate, fully reproducible
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub const SEED: u64 = 0x243F_6A88_85A3_08D3;

    pub fn new() -> Rng {
        Rng(Self::SEED)
    }

    pub fn with_seed(seed: u64) -> Rng {
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

    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    /// uniform in `0..n`
    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0);
        self.next_u64() % n
    }

    pub fn range_i64(&mut self, lo: i64, hi: i64) -> i64 {
        assert!(lo <= hi);
        let span = (hi as i128 - lo as i128 + 1) as u128;
        (lo as i128 + (self.next_u64() as u128 % span) as i128) as i64
    }

    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len() as u64) as usize]
    }
}
