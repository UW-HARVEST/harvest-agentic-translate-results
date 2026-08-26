//! Shared plumbing for the C-vs-Rust differential tests.
//!
//! Both implementations are loaded as shared objects through `libloading` and
//! driven through their exported C ABI symbols (`driver`, `main`) — the Rust
//! functions are never called directly, so the `#[no_mangle]` wrappers are part
//! of what is under test.
//!
//! * the C `.so` is built with `gcc -shared -fPIC c_src/src/main.c`
//! * the Rust `.so` is built with `rustc --crate-type cdylib src/lib.rs`
//!   (flags selected by the `RUST_SO_PROFILE` env var: `dev` or `release`)
#![allow(dead_code)]

use std::ffi::c_void;
use std::fs;
use std::io::{self, Read, Write};
use std::os::raw::c_int;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

/// Redirecting fd 1 and `fork()`ing are process-global operations, so every
/// capture has to be serialised even though libtest runs tests on several
/// threads.
fn capture_lock() -> MutexGuard<'static, ()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    match L.get_or_init(|| Mutex::new(())).lock() {
        Ok(g) => g,
        // A previous test panicked while holding the lock; fd 1 was restored by
        // then, so the guard is still usable.
        Err(poisoned) => poisoned.into_inner(),
    }
}

// ---------------------------------------------------------------------------
// libc bits we need.  Declared directly (std already links libc) so the test
// harness needs no dependency beyond libloading.
// ---------------------------------------------------------------------------
extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
    fn exit(code: c_int) -> !;
    fn fflush(stream: *mut c_void) -> c_int;
    fn signal(signum: c_int, handler: usize) -> usize;
    fn write(fd: c_int, buf: *const u8, count: usize) -> isize;
    fn kill(pid: c_int, sig: c_int) -> c_int;
    fn posix_openpt(flags: c_int) -> c_int;
    fn grantpt(fd: c_int) -> c_int;
    fn unlockpt(fd: c_int) -> c_int;
    fn ptsname(fd: c_int) -> *mut i8;
    fn open(path: *const i8, flags: c_int) -> c_int;
}

const O_RDWR: c_int = 2;
const O_NOCTTY: c_int = 0o400;

/// A freshly allocated pseudo-terminal, as `(master, slave)`.  Used to give a
/// child a stdout that really is a character device, which is the only case
/// where glibc line-buffers instead of fully buffering.
pub fn make_pty() -> (c_int, c_int) {
    unsafe {
        let master = posix_openpt(O_RDWR | O_NOCTTY);
        assert!(master >= 0, "posix_openpt failed");
        assert_eq!(grantpt(master), 0, "grantpt failed");
        assert_eq!(unlockpt(master), 0, "unlockpt failed");
        let name = ptsname(master);
        assert!(!name.is_null(), "ptsname failed");
        let slave = open(name, O_RDWR | O_NOCTTY);
        assert!(slave >= 0, "opening the pty slave failed");
        (master, slave)
    }
}

const SIGPIPE: c_int = 13;
const SIG_DFL: usize = 0;

// ---------------------------------------------------------------------------
// Paths / builds
// ---------------------------------------------------------------------------

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn tmp_dir() -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    fs::create_dir_all(&d).unwrap();
    d
}

fn c_source() -> PathBuf {
    manifest_dir().join("c_src/src/main.c")
}

fn unique(name: &str) -> PathBuf {
    tmp_dir().join(format!("{}.{}", name, std::process::id()))
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

/// `gcc -shared -fPIC -o libdriver_c.so c_src/src/main.c`
pub fn c_so_path() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let out = unique("libdriver_c.so");
        run_tool(
            "gcc (C shared object)",
            Command::new("gcc")
                .arg("-shared")
                .arg("-fPIC")
                .arg("-o")
                .arg(&out)
                .arg(c_source()),
        );
        out
    })
    .as_path()
}

/// The C executable.  Prefers the CMake build (`c_src/build/driver`, the build
/// the task prescribes) and falls back to a plain `gcc -o` build, which uses the
/// same flags CMake does without a `CMAKE_BUILD_TYPE`.
pub fn c_exe_path() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let cmake = manifest_dir().join("c_src/build/driver");
        if cmake.is_file() {
            return cmake;
        }
        let out = unique("driver_c");
        run_tool(
            "gcc (C executable)",
            Command::new("gcc").arg("-o").arg(&out).arg(c_source()),
        );
        out
    })
    .as_path()
}

/// The cargo-built Rust executable.
pub fn rust_exe_path() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// Which flags to build the Rust cdylib with.  `dev` mirrors the default cargo
/// profile, `release` mirrors `[profile.release]` (`panic = "abort"`, optimised).
pub fn rust_so_profile() -> String {
    std::env::var("RUST_SO_PROFILE").unwrap_or_else(|_| "dev".to_string())
}

/// `rustc --crate-type cdylib -o libdriver_rs.so src/lib.rs`
pub fn rust_so_path() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let profile = rust_so_profile();
        let out = unique(&format!("libdriver_rs_{profile}.so"));
        let mut cmd = Command::new("rustc");
        cmd.arg("--edition")
            .arg("2021")
            .arg("--crate-type")
            .arg("cdylib")
            .arg("--crate-name")
            .arg("driver")
            .arg("-A")
            .arg("warnings");
        match profile.as_str() {
            "dev" => {
                cmd.arg("-C").arg("opt-level=0");
                cmd.arg("-C").arg("debug-assertions=on");
                cmd.arg("-C").arg("overflow-checks=on");
            }
            "release" => {
                cmd.arg("-C").arg("opt-level=3");
                cmd.arg("-C").arg("debug-assertions=off");
                cmd.arg("-C").arg("overflow-checks=off");
                cmd.arg("-C").arg("panic=abort");
            }
            other => panic!("unknown RUST_SO_PROFILE {other:?} (expected dev|release)"),
        }
        cmd.arg("-o").arg(&out).arg(manifest_dir().join("src/lib.rs"));
        run_tool("rustc (Rust cdylib)", &mut cmd);
        out
    })
    .as_path()
}

// ---------------------------------------------------------------------------
// The two implementations, loaded through libloading
// ---------------------------------------------------------------------------

pub type DriverFn = unsafe extern "C" fn(c_int);
pub type DriverWideFn = unsafe extern "C" fn(i64);
pub type MainFn = unsafe extern "C" fn() -> c_int;

pub struct Impl {
    pub name: &'static str,
    _lib: libloading::Library,
    pub driver: DriverFn,
    /// The very same `driver` symbol, but declared as taking a 64-bit argument
    /// so the tests can push out-of-range values across the ABI boundary.
    pub driver_wide: DriverWideFn,
    pub main: MainFn,
}

impl Impl {
    fn load(name: &'static str, path: &Path) -> Impl {
        unsafe {
            let lib = libloading::Library::new(path)
                .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
            // Inner scope so the `Symbol` borrows of `lib` end before `lib` is
            // moved into the struct; the extracted values are plain fn pointers.
            let (driver, driver_wide, main) = {
                let d: libloading::Symbol<DriverFn> = lib
                    .get(b"driver\0")
                    .unwrap_or_else(|e| panic!("{name}: no `driver` symbol: {e}"));
                let w: libloading::Symbol<DriverWideFn> = lib.get(b"driver\0").unwrap();
                let m: libloading::Symbol<MainFn> = lib
                    .get(b"main\0")
                    .unwrap_or_else(|e| panic!("{name}: no `main` symbol: {e}"));
                (*d, *w, *m)
            };
            Impl {
                name,
                _lib: lib,
                driver,
                driver_wide,
                main,
            }
        }
    }
}

/// Both implementations, loaded once per test process.
pub struct Pair {
    pub c: Impl,
    pub rs: Impl,
}

pub fn pair() -> &'static Pair {
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(|| {
        let p = Pair {
            c: Impl::load("C", c_so_path()),
            rs: Impl::load("Rust", rust_so_path()),
        };
        // Warm both libraries up (lazy statics, stdio buffers) before any
        // fork(), so the forked children inherit fully initialised state.
        let _ = capture_in_process(|| unsafe { (p.c.driver)(0) });
        let _ = capture_in_process(|| unsafe { (p.rs.driver)(0) });
        p
    })
}

// ---------------------------------------------------------------------------
// Capturing fd 1
// ---------------------------------------------------------------------------

fn next_id() -> u64 {
    static N: AtomicU64 = AtomicU64::new(0);
    N.fetch_add(1, Ordering::SeqCst)
}

/// A unique scratch path (unique per process *and* per call, so tests running
/// on different threads never collide).
pub fn scratch_file(tag: &str) -> PathBuf {
    tmp_dir().join(format!(
        "{tag}.{}.{}.bin",
        std::process::id(),
        next_id()
    ))
}

/// Run `f` with fd 1 redirected to a fresh regular file and return everything
/// that was written (after flushing C stdio, which `printf` buffers).
///
/// NOTE: fd 1 is process-global and libtest also writes its progress lines
/// there from another thread, so this in-process variant is only used for the
/// one-off warm-up in [`pair`].  Everything that is compared goes through
/// [`capture_child`], which redirects fd 1 in a forked child and therefore
/// cannot pick up the harness's own output.
fn capture_in_process(f: impl FnOnce()) -> Vec<u8> {
    let _guard = capture_lock();
    let path = scratch_file("capture");
    let bytes = {
        let file = fs::File::create(&path).unwrap();
        let _ = io::stdout().flush();
        unsafe {
            fflush(std::ptr::null_mut());
        }
        let saved = unsafe { dup(1) };
        assert!(saved >= 0, "dup(1) failed");
        assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 failed");
        f();
        unsafe {
            fflush(std::ptr::null_mut());
        }
        let _ = io::stdout().flush();
        assert!(unsafe { dup2(saved, 1) } >= 0, "dup2 restore failed");
        unsafe {
            close(saved);
        }
        drop(file);
        fs::read(&path).unwrap()
    };
    let _ = fs::remove_file(&path);
    bytes
}

/// Run `f` in a forked child whose fd 1 is a fresh regular file, and return
/// what it wrote plus how the child terminated.
pub fn capture_child(f: impl FnOnce()) -> Run {
    run_child(Stdin::Inherit, Stdout::File, || {
        f();
        0
    })
}

/// Same, but the child's fd 1 is a pipe (stdio picks a different buffering mode
/// for non-seekable streams).
pub fn capture_child_piped(f: impl FnOnce()) -> Run {
    run_child(Stdin::Inherit, Stdout::Pipe, || {
        f();
        0
    })
}

/// `pipe(2)`, returning `(read_end, write_end)`.
pub fn make_pipe() -> (c_int, c_int) {
    let mut fds = [0 as c_int; 2];
    assert_eq!(unsafe { pipe(fds.as_mut_ptr()) }, 0, "pipe() failed");
    (fds[0], fds[1])
}

/// Put a descriptor into non-blocking mode, so draining a pty/pipe cannot block.
pub fn set_nonblocking(fd: c_int) {
    const F_GETFL: c_int = 3;
    const F_SETFL: c_int = 4;
    const O_NONBLOCK: c_int = 0o4000;
    extern "C" {
        fn fcntl(fd: c_int, cmd: c_int, arg: c_int) -> c_int;
    }
    unsafe {
        let flags = fcntl(fd, F_GETFL, 0);
        if flags >= 0 {
            fcntl(fd, F_SETFL, flags | O_NONBLOCK);
        }
    }
}

/// `close(2)`.
pub fn close_fd(fd: c_int) {
    unsafe {
        close(fd);
    }
}

// ---------------------------------------------------------------------------
// Running the exported `main` in a forked child
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Status {
    Exited(i32),
    Signaled(i32),
}

fn decode_status(st: c_int) -> Status {
    if st & 0x7f == 0 {
        Status::Exited((st >> 8) & 0xff)
    } else {
        Status::Signaled(st & 0x7f)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Run {
    pub out: Vec<u8>,
    pub status: Status,
}

/// How the child's stdin is provided.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stdin<'a> {
    /// A regular (seekable) file holding these bytes.
    File(&'a [u8]),
    /// A pipe pre-filled with these bytes (must fit in the pipe buffer).
    Pipe(&'a [u8]),
    /// A pipe that stays empty until the parent writes these bytes two seconds
    /// after the child started, so the child blocks in `read`.
    SlowPipe(&'a [u8]),
    /// An already-open descriptor supplied by the caller (a pre-positioned file,
    /// a pty slave, a character device, a write-only fd …).
    Raw(c_int),
    /// fd 0 closed.
    Closed,
    /// fd 0 open on a directory.
    Directory,
    /// Leave the parent's fd 0 in place.
    Inherit,
}

/// How the child's stdout is provided.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stdout {
    /// A regular file whose contents are returned.
    File,
    /// A pipe drained by the parent.
    Pipe,
    /// A pipe whose read end is closed before the call (EPIPE / SIGPIPE).
    ClosedPipe,
    /// fd 1 closed (EBADF).
    Closed,
    /// `/dev/full` (ENOSPC on flush).
    DevFull,
    /// A pseudo-terminal, drained by the parent (glibc line-buffers this one).
    Tty,
}

/// Fork, wire up fd 0 / fd 1 as requested, run `f` in the child, and collect
/// its output and exit status.  `f` only calls into a `dlopen`ed library, so the
/// child does essentially no work of its own; its return value becomes the
/// child's exit code.
pub fn run_child(stdin: Stdin<'_>, stdout: Stdout, f: impl FnOnce() -> c_int) -> Run {
    // Serialised together with the fd-1 captures: `fork()` must not race with
    // another thread that is temporarily pointing fd 1 somewhere else.
    let _guard = capture_lock();
    // ---- set up stdin ----
    let mut in_file: Option<fs::File> = None;
    let mut in_pipe: Option<(c_int, c_int)> = None;
    let in_fd: Option<c_int> = match stdin {
        Stdin::File(data) => {
            let p = scratch_file("stdin");
            fs::write(&p, data).unwrap();
            let f = fs::File::open(&p).unwrap();
            let fd = f.as_raw_fd();
            in_file = Some(f);
            let _ = fs::remove_file(&p);
            Some(fd)
        }
        Stdin::Pipe(data) => {
            assert!(
                data.len() < 60_000,
                "pipe-backed stdin must fit in the pipe buffer ({} bytes)",
                data.len()
            );
            let mut fds = [0 as c_int; 2];
            assert_eq!(unsafe { pipe(fds.as_mut_ptr()) }, 0);
            let mut off = 0usize;
            while off < data.len() {
                let n = unsafe { write(fds[1], data[off..].as_ptr(), data.len() - off) };
                assert!(n > 0, "short write to pipe");
                off += n as usize;
            }
            unsafe {
                close(fds[1]);
            }
            in_pipe = Some((fds[0], -1));
            Some(fds[0])
        }
        Stdin::SlowPipe(_) => {
            let (rd, wr) = make_pipe();
            in_pipe = Some((rd, wr));
            Some(rd)
        }
        Stdin::Raw(fd) => Some(fd),
        Stdin::Closed => Some(-2), // sentinel: close fd 0 in the child
        Stdin::Directory => {
            let f = fs::File::open(manifest_dir()).unwrap();
            let fd = f.as_raw_fd();
            in_file = Some(f);
            Some(fd)
        }
        Stdin::Inherit => None,
    };

    // ---- set up stdout ----
    let out_path = scratch_file("stdout");
    let mut out_file: Option<fs::File> = None;
    let mut out_pipe: Option<(c_int, c_int)> = None;
    let mut out_pty: Option<(c_int, c_int)> = None;
    let out_fd: c_int = match stdout {
        Stdout::File => {
            let f = fs::File::create(&out_path).unwrap();
            let fd = f.as_raw_fd();
            out_file = Some(f);
            fd
        }
        Stdout::Pipe | Stdout::ClosedPipe => {
            let mut fds = [0 as c_int; 2];
            assert_eq!(unsafe { pipe(fds.as_mut_ptr()) }, 0);
            if stdout == Stdout::ClosedPipe {
                unsafe {
                    close(fds[0]);
                }
                out_pipe = Some((-1, fds[1]));
            } else {
                out_pipe = Some((fds[0], fds[1]));
            }
            fds[1]
        }
        Stdout::Closed => -2, // sentinel: close fd 1 in the child
        Stdout::DevFull => {
            let f = fs::OpenOptions::new().write(true).open("/dev/full").unwrap();
            let fd = f.as_raw_fd();
            out_file = Some(f);
            fd
        }
        Stdout::Tty => {
            let (master, slave) = make_pty();
            out_pty = Some((master, slave));
            slave
        }
    };

    let _ = io::stdout().flush();
    unsafe {
        fflush(std::ptr::null_mut());
    }

    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork() failed");
    if pid == 0 {
        // ---------------- child ----------------
        unsafe {
            match in_fd {
                Some(-2) => {
                    close(0);
                }
                Some(fd) => {
                    dup2(fd, 0);
                }
                None => {}
            }
            if out_fd == -2 {
                close(1);
            } else {
                dup2(out_fd, 1);
            }
            // Our inherited copy of a pipe's write end would keep the reader
            // from ever seeing EOF.
            if let Some((_, wr)) = in_pipe {
                if wr >= 0 {
                    close(wr);
                }
            }
            // A C program starts with SIGPIPE at SIG_DFL; the Rust test harness
            // set it to SIG_IGN, so restore the C default for both sides.
            signal(SIGPIPE, SIG_DFL);
            let rc = f();
            fflush(std::ptr::null_mut());
            // Terminate through `exit(3)`, not `_exit(2)`: a real program's exit
            // is what runs glibc's `_IO_cleanup` (flush stdout, rewind the
            // read-ahead on stdin) and the translation's equivalent `atexit`
            // hook, so both sides get their exit-time behaviour.
            exit(rc & 0xff);
        }
    }

    // ---------------- parent ----------------
    if let Stdin::SlowPipe(data) = stdin {
        // Let the child block in `read` first, then deliver the bytes.  Writing
        // may fail with EPIPE if the child already gave up; that is fine.
        let wr = in_pipe.map(|(_, w)| w).unwrap_or(-1);
        std::thread::sleep(std::time::Duration::from_secs(2));
        let mut off = 0usize;
        while off < data.len() {
            let n = unsafe { write(wr, data[off..].as_ptr(), data.len() - off) };
            if n <= 0 {
                break;
            }
            off += n as usize;
        }
        unsafe {
            close(wr);
        }
    }
    // Close our copies of the write ends so reads see EOF.
    if let Some((_, wr)) = out_pipe {
        if wr >= 0 {
            unsafe {
                close(wr);
            }
        }
    }
    let pty_out = out_pty.map(|(master, slave)| {
        // Our copy of the slave must go, otherwise the master never sees EOF.
        unsafe {
            close(slave);
        }
        let mut buf = Vec::new();
        let mut f = unsafe { <fs::File as std::os::unix::io::FromRawFd>::from_raw_fd(master) };
        // A pty master reports EIO (not EOF) once the last slave closes.
        let mut chunk = [0u8; 256];
        loop {
            match f.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => buf.extend_from_slice(&chunk[..n]),
                Err(_) => break,
            }
        }
        buf
    });
    let piped_out = out_pipe.and_then(|(rd, _)| {
        if rd < 0 {
            None
        } else {
            let mut buf = Vec::new();
            let mut f = unsafe { <fs::File as std::os::unix::io::FromRawFd>::from_raw_fd(rd) };
            f.read_to_end(&mut buf).unwrap();
            Some(buf)
        }
    });

    let mut st: c_int = 0;
    let status = {
        const WNOHANG: c_int = 1;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        let mut killed = false;
        loop {
            let w = unsafe { waitpid(pid, &mut st, WNOHANG) };
            if w == pid {
                break decode_status(st);
            }
            assert!(w >= 0, "waitpid failed");
            if !killed && std::time::Instant::now() > deadline {
                // A child that never finishes is a dead-lock: kill it and let the
                // comparison fail loudly rather than hanging the test run.
                killed = true;
                unsafe {
                    kill(pid, 9);
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    };

    let out = match stdout {
        Stdout::File => fs::read(&out_path).unwrap_or_default(),
        Stdout::Pipe => piped_out.unwrap_or_default(),
        Stdout::Tty => pty_out.unwrap_or_default(),
        _ => Vec::new(),
    };

    // cleanup
    drop(out_file);
    drop(in_file);
    if let Some((rd, wr)) = in_pipe {
        if rd >= 0 {
            unsafe {
                close(rd);
            }
        }
        if wr >= 0 && !matches!(stdin, Stdin::SlowPipe(_)) {
            unsafe {
                close(wr);
            }
        }
    }
    let _ = fs::remove_file(&out_path);

    Run { out, status }
}

/// Run the exported `main` of both implementations on the same stdin/stdout
/// wiring and assert the results are identical.
#[track_caller]
pub fn diff_main(stdin: Stdin<'_>, stdout: Stdout, label: &str) -> Run {
    let p = pair();
    let c = run_child(stdin, stdout, || unsafe { (p.c.main)() });
    let r = run_child(stdin, stdout, || unsafe { (p.rs.main)() });
    assert_eq!(
        (as_text(&c.out), c.status),
        (as_text(&r.out), r.status),
        "`main` diverged for {label} (stdin={:?}, stdout={:?})",
        Preview(stdin),
        stdout
    );
    c
}

/// Run the exported `main` of both implementations against the same input bytes
/// (regular-file stdin, regular-file stdout) and assert equality.
#[track_caller]
pub fn diff_main_input(input: &[u8]) -> Run {
    diff_main(Stdin::File(input), Stdout::File, &preview(input))
}

// ---------------------------------------------------------------------------
// driver differential helpers
// ---------------------------------------------------------------------------

/// Call `driver` once per value in one forked child per implementation and
/// compare the two output streams.
#[track_caller]
pub fn diff_driver_batch(values: &[i32], label: &str) {
    let p = pair();
    let c = capture_child(|| {
        for &v in values {
            unsafe { (p.c.driver)(v) }
        }
    });
    let r = capture_child(|| {
        for &v in values {
            unsafe { (p.rs.driver)(v) }
        }
    });
    assert_eq!(
        (c.status, r.status),
        (Status::Exited(0), Status::Exited(0)),
        "`driver` child terminated abnormally for {label}"
    );
    assert_batch_eq(&c.out, &r.out, values, label);
}

/// Same, through a pipe rather than a regular file.
#[track_caller]
pub fn diff_driver_batch_piped(values: &[i32], label: &str) {
    let p = pair();
    let c = capture_child_piped(|| {
        for &v in values {
            unsafe { (p.c.driver)(v) }
        }
    });
    let r = capture_child_piped(|| {
        for &v in values {
            unsafe { (p.rs.driver)(v) }
        }
    });
    assert_eq!(
        (c.status, r.status),
        (Status::Exited(0), Status::Exited(0)),
        "`driver` child terminated abnormally for {label}"
    );
    assert_batch_eq(&c.out, &r.out, values, label);
}

/// The same `driver` symbol called with a 64-bit argument (out-of-range input
/// across the FFI boundary).
#[track_caller]
pub fn diff_driver_batch_wide(values: &[i64], label: &str) {
    let p = pair();
    let c = capture_child(|| {
        for &v in values {
            unsafe { (p.c.driver_wide)(v) }
        }
    });
    let r = capture_child(|| {
        for &v in values {
            unsafe { (p.rs.driver_wide)(v) }
        }
    });
    assert_eq!(
        (c.status, r.status),
        (Status::Exited(0), Status::Exited(0)),
        "`driver` (wide) child terminated abnormally for {label}"
    );
    let (c, r) = (c.out, r.out);
    assert_eq!(
        as_text(&c),
        as_text(&r),
        "`driver` (wide argument) diverged for {label}; first values {:?}",
        &values[..values.len().min(8)]
    );
    assert_eq!(c.len(), values.len() * 9, "unexpected C output length");
}

#[track_caller]
fn assert_batch_eq(c: &[u8], r: &[u8], values: &[i32], label: &str) {
    if c != r {
        // Pin-point the first differing record for a useful message.
        let mut detail = String::new();
        for (i, v) in values.iter().enumerate() {
            let (a, b) = (
                c.get(i * 9..i * 9 + 9).unwrap_or(b"<missing>"),
                r.get(i * 9..i * 9 + 9).unwrap_or(b"<missing>"),
            );
            if a != b {
                detail = format!(
                    "first divergence at index {i} (x = {v} / {:#010x}): C {:?} vs Rust {:?}",
                    *v as u32,
                    String::from_utf8_lossy(a),
                    String::from_utf8_lossy(b)
                );
                break;
            }
        }
        panic!("`driver` diverged for {label}: {detail}");
    }
    assert_eq!(
        c.len(),
        values.len() * 9,
        "unexpected output length for {label}"
    );
}

// ---------------------------------------------------------------------------
// Process-level (executable) differential helper
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
pub struct ProcOut {
    pub out: Vec<u8>,
    pub status: Option<i32>,
    pub signal: Option<i32>,
}

pub fn run_exe_with_file_stdin(exe: &Path, input: &[u8]) -> ProcOut {
    let p = scratch_file("exe-stdin");
    fs::write(&p, input).unwrap();
    let f = fs::File::open(&p).unwrap();
    let out = Command::new(exe)
        .stdin(std::process::Stdio::from(f))
        .output()
        .unwrap();
    let _ = fs::remove_file(&p);
    use std::os::unix::process::ExitStatusExt;
    ProcOut {
        out: out.stdout,
        status: out.status.code(),
        signal: out.status.signal(),
    }
}

pub fn run_exe_with_pipe_stdin(exe: &Path, input: &[u8]) -> ProcOut {
    use std::os::unix::process::ExitStatusExt;
    let mut child = Command::new(exe)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    {
        let mut si = child.stdin.take().unwrap();
        // A separate thread keeps large inputs from dead-locking on the pipe.
        let data = input.to_vec();
        std::thread::spawn(move || {
            let _ = si.write_all(&data);
        });
    }
    let out = child.wait_with_output().unwrap();
    ProcOut {
        out: out.stdout,
        status: out.status.code(),
        signal: out.status.signal(),
    }
}

#[track_caller]
pub fn diff_exe_file_stdin(input: &[u8]) {
    let c = run_exe_with_file_stdin(c_exe_path(), input);
    let r = run_exe_with_file_stdin(rust_exe_path(), input);
    assert_eq!(
        (as_text(&c.out), c.status, c.signal),
        (as_text(&r.out), r.status, r.signal),
        "executables diverged for input {}",
        preview(input)
    );
}

#[track_caller]
pub fn diff_exe_pipe_stdin(input: &[u8]) {
    let c = run_exe_with_pipe_stdin(c_exe_path(), input);
    let r = run_exe_with_pipe_stdin(rust_exe_path(), input);
    assert_eq!(
        (as_text(&c.out), c.status, c.signal),
        (as_text(&r.out), r.status, r.signal),
        "executables diverged (pipe stdin) for input {}",
        preview(input)
    );
}

// ---------------------------------------------------------------------------
// misc
// ---------------------------------------------------------------------------

pub fn as_text(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

pub fn preview(input: &[u8]) -> String {
    let shown: Vec<u8> = input.iter().copied().take(64).collect();
    let mut s = String::new();
    for b in shown {
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
    format!("\"{s}\"{}", if input.len() > 64 {
        format!(" (+{} more bytes, {} total)", input.len() - 64, input.len())
    } else {
        String::new()
    })
}

struct Preview<'a>(Stdin<'a>);
impl std::fmt::Debug for Preview<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Stdin::File(d) => write!(f, "File({})", preview(d)),
            Stdin::Pipe(d) => write!(f, "Pipe({})", preview(d)),
            Stdin::SlowPipe(d) => write!(f, "SlowPipe({})", preview(d)),
            Stdin::Raw(fd) => write!(f, "Raw(fd {fd})"),
            other => write!(f, "{other:?}"),
        }
    }
}

/// SplitMix64 — fixed seed, so every "randomised" row is reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
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
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0);
        self.next_u64() % n
    }
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[self.below(xs.len() as u64) as usize]
    }
    pub fn digits(&mut self, n: usize) -> String {
        (0..n)
            .map(|i| {
                // Avoid a leading zero unless the caller wants one.
                let lo: u64 = if i == 0 && n > 1 { 1 } else { 0 };
                (b'0' + (lo + self.below(10 - lo)) as u8) as char
            })
            .collect()
    }
}
