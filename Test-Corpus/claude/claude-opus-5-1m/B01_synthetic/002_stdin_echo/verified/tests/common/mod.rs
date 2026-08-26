//! Shared helpers for the differential tests.
//!
//! Everything here exists to run the **C build** and the **Rust build** under
//! byte-identical conditions and compare (exit status, stdout bytes, stderr
//! bytes). The C code is the ground truth; any mismatch is a Rust bug.

#![allow(dead_code)]

use std::ffi::CString;
use std::fs;
use std::io::{Read, Write};
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// Locations
// ---------------------------------------------------------------------------

/// Crate root (`translated_rust/`).
pub fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The directory holding the built test targets, i.e. `target/<profile>/`.
fn target_profile_dir() -> PathBuf {
    // current_exe is target/<profile>/deps/<test>-<hash>
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .to_path_buf()
}

/// A scratch directory for generated artifacts and temp files.
pub fn scratch_dir() -> PathBuf {
    let d = target_profile_dir().join("difftest");
    fs::create_dir_all(&d).expect("create scratch dir");
    d
}

/// The translated Rust executable (`target/<profile>/driver`).
pub fn rust_exe() -> PathBuf {
    let p = target_profile_dir().join("driver");
    assert!(p.is_file(), "rust binary not built: {}", p.display());
    p
}

/// The Rust `cdylib` (`target/<profile>/libdriver.so`).
///
/// An integration test cannot *link* a `cdylib`-only library, so cargo has no
/// reason to build it as a dependency of the test binary; it is built on demand
/// here instead, into the same profile the test itself was built with (so
/// `cargo test --release` compares release artifacts, not stale debug ones).
pub fn rust_so() -> PathBuf {
    let dir = target_profile_dir();
    let p = dir.join("libdriver.so");
    let profile = dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("debug")
        .to_string();

    let mut args: Vec<String> = ["build", "--offline", "--lib"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    // the "debug" directory is produced by the built-in `dev` profile
    if profile != "debug" {
        args.push("--profile".into());
        args.push(profile.clone());
    }

    let st = Command::new(env!("CARGO"))
        .args(&args)
        .current_dir(crate_root())
        .output();
    assert!(
        p.is_file(),
        "rust cdylib not built: {}\n  cargo {:?} -> {:?}",
        p.display(),
        args,
        st.map(|o| String::from_utf8_lossy(&o.stderr).to_string())
    );
    p
}

// ---------------------------------------------------------------------------
// Building the C side (c_src/ is never modified, only read)
// ---------------------------------------------------------------------------

/// The C executable built by `c_src/CMakeLists.txt`.
pub fn c_exe() -> PathBuf {
    static ONCE: OnceLock<PathBuf> = OnceLock::new();
    ONCE.get_or_init(|| {
        let build = crate_root().join("c_src/build");
        let exe = build.join("driver");
        if !exe.is_file() {
            fs::create_dir_all(&build).expect("create c_src/build");
            let cfg = Command::new("cmake")
                .args([
                    "..".as_ref(),
                    "-DCMAKE_POSITION_INDEPENDENT_CODE=ON".as_ref(),
                ] as [&std::ffi::OsStr; 2])
                .current_dir(&build)
                .output()
                .expect("run cmake");
            assert!(cfg.status.success(), "cmake configure failed: {:?}", cfg);
            let b = Command::new("cmake")
                .args(["--build", "."])
                .current_dir(&build)
                .output()
                .expect("run cmake --build");
            assert!(b.status.success(), "cmake build failed: {:?}", b);
        }
        assert!(exe.is_file(), "C executable missing: {}", exe.display());
        exe
    })
    .clone()
}

/// The **same, unmodified** C source compiled as a shared object, so that its
/// `main` can be dlopen()ed and compared against the Rust `cdylib`'s `main`.
/// Nothing in `c_src/` is written to: the source is only read, and the output
/// goes to the cargo target directory.
pub fn c_so() -> PathBuf {
    static ONCE: OnceLock<PathBuf> = OnceLock::new();
    ONCE.get_or_init(|| {
        let src = crate_root().join("c_src/src/main.c");
        assert!(src.is_file(), "C source missing: {}", src.display());
        let out = scratch_dir().join("libcdriver.so");
        let o = Command::new("cc")
            .arg("-shared")
            .arg("-fPIC")
            .arg("-O2")
            .arg("-o")
            .arg(&out)
            .arg(&src)
            .output()
            .expect("run cc");
        assert!(o.status.success(), "building C .so failed: {:?}", o);
        out
    })
    .clone()
}

// ---------------------------------------------------------------------------
// Temp files
// ---------------------------------------------------------------------------

static COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn temp_path(tag: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    scratch_dir().join(format!("{}-{}-{}.bin", tag, std::process::id(), n))
}

pub fn write_temp(tag: &str, bytes: &[u8]) -> PathBuf {
    let p = temp_path(tag);
    let mut f = fs::File::create(&p).expect("create temp");
    f.write_all(bytes).expect("write temp");
    f.sync_all().ok();
    p
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed -> reproducible test inputs)
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    /// splitmix64
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform in `[0, n)`.
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 {
            0
        } else {
            self.next_u64() % n
        }
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
    pub fn bool_pct(&mut self, pct: u64) -> bool {
        self.below(100) < pct
    }
}

// ---------------------------------------------------------------------------
// Running a build with file-backed stdin/stdout (never deadlocks)
// ---------------------------------------------------------------------------

/// Outcome of one run, as an external caller observes it.
#[derive(PartialEq, Eq, Clone)]
pub struct Outcome {
    /// `Ok(code)` for a normal exit, `Err(signal)` if the process was killed.
    pub status: Result<i32, i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Outcome {{ status: {}, stdout: {} bytes {:?}, stderr: {:?} }}",
            match self.status {
                Ok(c) => format!("exit {}", c),
                Err(s) => format!("KILLED by signal {}", s),
            },
            self.stdout.len(),
            Preview(&self.stdout),
            String::from_utf8_lossy(&self.stderr),
        )
    }
}

/// Short, escaped preview of a byte buffer for assertion messages.
pub struct Preview<'a>(pub &'a [u8]);

impl std::fmt::Debug for Preview<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let head: String = self.0.iter().take(64).map(|b| esc(*b)).collect();
        if self.0.len() > 64 {
            write!(f, "\"{}\"...(+{} bytes)", head, self.0.len() - 64)
        } else {
            write!(f, "\"{}\"", head)
        }
    }
}

fn esc(b: u8) -> String {
    match b {
        b'\n' => "\\n".into(),
        b'\r' => "\\r".into(),
        0 => "\\0".into(),
        0x20..=0x7e => (b as char).to_string(),
        _ => format!("\\x{:02x}", b),
    }
}

fn status_of(st: std::process::ExitStatus) -> Result<i32, i32> {
    match st.code() {
        Some(c) => Ok(c),
        None => Err(st.signal().unwrap_or(-1)),
    }
}

/// Run `exe` with `input` on stdin, both stdin and stdout backed by regular
/// files (so no pipe can ever deadlock), and collect the outcome.
pub fn run_file_io(exe: &Path, input: &[u8], args: &[&str]) -> Outcome {
    let in_path = write_temp("in", input);
    let out_path = temp_path("out");

    let fin = fs::File::open(&in_path).expect("open stdin temp");
    let fout = fs::File::create(&out_path).expect("create stdout temp");

    let out = Command::new(exe)
        .args(args)
        .stdin(Stdio::from(fin))
        .stdout(Stdio::from(fout))
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn")
        .wait_with_output()
        .expect("wait");

    let stdout = fs::read(&out_path).expect("read stdout temp");
    let _ = fs::remove_file(&in_path);
    let _ = fs::remove_file(&out_path);

    Outcome {
        status: status_of(out.status),
        stdout,
        stderr: out.stderr,
    }
}

/// Run both builds on the same input and assert the outcomes are identical.
pub fn assert_same(label: &str, input: &[u8]) {
    assert_same_args(label, input, &[]);
}

pub fn assert_same_args(label: &str, input: &[u8], args: &[&str]) {
    let c = run_file_io(&c_exe(), input, args);
    let r = run_file_io(&rust_exe(), input, args);
    if c != r {
        panic!(
            "\n{} DIVERGED\n  input ({} bytes): {:?}\n  C   : {:?}\n  Rust: {:?}\n  first differing stdout byte: {:?}\n",
            label,
            input.len(),
            Preview(input),
            c,
            r,
            first_diff(&c.stdout, &r.stdout)
        );
    }
}

pub fn first_diff(a: &[u8], b: &[u8]) -> Option<(usize, Option<u8>, Option<u8>)> {
    let n = a.len().max(b.len());
    (0..n).find_map(|i| {
        let (x, y) = (a.get(i).copied(), b.get(i).copied());
        if x != y {
            Some((i, x, y))
        } else {
            None
        }
    })
}

// ---------------------------------------------------------------------------
// Reference model of the C program (cross-check for the C observations)
// ---------------------------------------------------------------------------

/// What `while (fgets(text,128,stdin)) fputs(text,stdout);` must produce:
/// split the input into chunks that end at a newline or at 127 bytes, and emit
/// each chunk truncated at its first NUL byte.
pub fn model(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < input.len() {
        let mut end = i;
        while end < input.len() && end - i < 127 {
            let b = input[end];
            end += 1;
            if b == b'\n' {
                break;
            }
        }
        let chunk = &input[i..end];
        let stop = chunk.iter().position(|&b| b == 0).unwrap_or(chunk.len());
        out.extend_from_slice(&chunk[..stop]);
        i = end;
    }
    out
}

// ---------------------------------------------------------------------------
// Low-level fd plumbing for the stream-kind / error-path rows
// ---------------------------------------------------------------------------

/// Serialises tests that fork or juggle process-wide fds.
pub fn fd_lock() -> &'static Mutex<()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
}

/// Spawn `exe` with stdin from a file and the given fd closed in the child
/// (`fd` = 0 or 1) to reproduce `EBADF` on that stream.
pub fn run_with_closed_fd(exe: &Path, input: &[u8], fd: i32) -> Outcome {
    let in_path = write_temp("in", input);
    let fin = fs::File::open(&in_path).expect("open stdin temp");
    let out_path = temp_path("out");
    let fout = fs::File::create(&out_path).expect("create stdout temp");

    let mut cmd = Command::new(exe);
    cmd.stdin(Stdio::from(fin))
        .stdout(Stdio::from(fout))
        .stderr(Stdio::piped());
    unsafe {
        cmd.pre_exec(move || {
            // async-signal-safe: a single close(2)
            libc::close(fd);
            Ok(())
        });
    }
    let out = cmd
        .spawn()
        .expect("spawn")
        .wait_with_output()
        .expect("wait");
    let stdout = fs::read(&out_path).unwrap_or_default();
    let _ = fs::remove_file(&in_path);
    let _ = fs::remove_file(&out_path);
    Outcome {
        status: status_of(out.status),
        stdout,
        stderr: out.stderr,
    }
}

/// Run `exe` with stdin taken from `path` (which may be a directory, /dev/null,
/// ...) and stdout to a temp file.
pub fn run_stdin_from_path(exe: &Path, path: &Path) -> Outcome {
    let fin = match fs::File::open(path) {
        Ok(f) => f,
        Err(e) => panic!("cannot open {} as stdin: {}", path.display(), e),
    };
    let out_path = temp_path("out");
    let fout = fs::File::create(&out_path).expect("create stdout temp");
    let out = Command::new(exe)
        .stdin(Stdio::from(fin))
        .stdout(Stdio::from(fout))
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn")
        .wait_with_output()
        .expect("wait");
    let stdout = fs::read(&out_path).unwrap_or_default();
    let _ = fs::remove_file(&out_path);
    Outcome {
        status: status_of(out.status),
        stdout,
        stderr: out.stderr,
    }
}

/// Run `exe` with stdin from a file and stdout redirected to `path`
/// (e.g. `/dev/full`). Returns `None` if `path` cannot be opened here.
pub fn run_stdout_to_path(exe: &Path, input: &[u8], path: &Path) -> Option<Outcome> {
    let fout = fs::OpenOptions::new().write(true).open(path).ok()?;
    let in_path = write_temp("in", input);
    let fin = fs::File::open(&in_path).expect("open stdin temp");
    let out = Command::new(exe)
        .stdin(Stdio::from(fin))
        .stdout(Stdio::from(fout))
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn")
        .wait_with_output()
        .expect("wait");
    let _ = fs::remove_file(&in_path);
    Some(Outcome {
        status: status_of(out.status),
        stdout: Vec::new(),
        stderr: out.stderr,
    })
}

/// Result of the broken-pipe experiment.
pub fn run_broken_pipe(exe: &Path, input: &[u8]) -> Result<i32, i32> {
    let _g = fd_lock().lock().unwrap();
    let in_path = write_temp("in", input);
    let fin = fs::File::open(&in_path).expect("open stdin temp");

    let mut fds = [0i32; 2];
    assert_eq!(
        unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) },
        0,
        "pipe2()"
    );
    let (rd, wr) = (fds[0], fds[1]);

    let child_out = unsafe { Stdio::from_raw_fd_owned(wr) };
    let mut child = Command::new(exe)
        .stdin(Stdio::from(fin))
        .stdout(child_out)
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn");
    // Parent must drop its copy of the write end, otherwise the pipe never breaks.
    unsafe { libc::close(wr) };

    // Consume a little, then close the read end so the next write gets EPIPE.
    let mut buf = [0u8; 8];
    let _ = unsafe { libc::read(rd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
    unsafe { libc::close(rd) };

    let st = child.wait().expect("wait");
    let _ = fs::remove_file(&in_path);
    status_of(st)
}

/// `Stdio::from_raw_fd` helper: takes ownership of `fd` for the child.
trait FromRawFdOwned {
    unsafe fn from_raw_fd_owned(fd: i32) -> Stdio;
}
impl FromRawFdOwned for Stdio {
    unsafe fn from_raw_fd_owned(fd: i32) -> Stdio {
        use std::os::fd::FromRawFd;
        // dup so the caller keeps its own fd to close explicitly
        let d = libc::dup(fd);
        assert!(d >= 0, "dup");
        // dup2 onto fd 0/1 in the child clears FD_CLOEXEC there, so the child
        // still gets the descriptor - it just does not also inherit this copy.
        set_cloexec(d);
        Stdio::from_raw_fd(d)
    }
}

// ---------------------------------------------------------------------------
// Timing observations: when does output become visible?
// ---------------------------------------------------------------------------

/// Feed `input` to `exe`, whose stdout is a **pipe**, then report how many bytes
/// are readable within `timeout_ms` *before* stdin is closed. This is what makes
/// glibc's block-vs-line buffering choice observable.
pub fn visible_before_eof(exe: &Path, input: &[u8], timeout_ms: i32, tty_stdout: bool) -> usize {
    let _g = fd_lock().lock().unwrap();

    // `out_read` stays with us; `child_side` is handed to the child and is
    // registered so the parent drops its own copy right after spawn (otherwise
    // EOF would never be seen on `out_read`).
    let (out_read, child_side) = if tty_stdout { open_pty() } else { pipe_pair() };
    let child_stdout = unsafe { Stdio::from_raw_fd_owned(child_side) };

    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(child_stdout)
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn");

    close_extra();

    let mut stdin = child.stdin.take().expect("stdin");
    stdin.write_all(input).expect("write stdin");
    stdin.flush().ok();

    let n = poll_read_len(out_read, timeout_ms);

    drop(stdin);
    let _ = child.wait();
    unsafe { libc::close(out_read) };
    n
}

// The transient fds that must be closed in the parent after spawn.
static EXTRA_FDS: Mutex<Vec<i32>> = Mutex::new(Vec::new());

pub fn close_extra() {
    let mut v = EXTRA_FDS.lock().unwrap();
    for fd in v.drain(..) {
        unsafe { libc::close(fd) };
    }
}

pub fn remember_extra(fd: i32) {
    EXTRA_FDS.lock().unwrap().push(fd);
}

/// Mark `fd` close-on-exec.
///
/// Critical for the pipe/pty helpers: `pipe(2)` and `openpty(3)` hand back
/// *inheritable* descriptors, so without this the spawned child keeps a copy of
/// the pipe's **read** end. The pipe could then never break (the child is its
/// own reader), `SIGPIPE` would never be raised, and the child would block
/// forever once the pipe filled up. `dup2` clears FD_CLOEXEC on the descriptor
/// it creates, so the child still gets a working fd 0/1.
pub fn set_cloexec(fd: i32) {
    unsafe {
        let f = libc::fcntl(fd, libc::F_GETFD);
        assert!(f >= 0, "F_GETFD");
        assert!(libc::fcntl(fd, libc::F_SETFD, f | libc::FD_CLOEXEC) >= 0, "F_SETFD");
    }
}

/// Create a pipe; returns (read end, write end). The write end is remembered so
/// the parent closes its copy after spawning the child.
pub fn pipe_pair() -> (i32, i32) {
    let mut fds = [0i32; 2];
    assert_eq!(
        unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) },
        0,
        "pipe2()"
    );
    remember_extra(fds[1]);
    (fds[0], fds[1])
}

/// Create a pty; returns (master, slave). The slave is remembered so the parent
/// closes it after spawning the child.
pub fn open_pty() -> (i32, i32) {
    let mut m: i32 = -1;
    let mut s: i32 = -1;
    let rc = unsafe {
        libc::openpty(
            &mut m,
            &mut s,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    assert_eq!(rc, 0, "openpty failed");
    set_cloexec(m);
    set_cloexec(s);
    remember_extra(s);
    (m, s)
}

/// Read everything available on `fd` within `timeout_ms`, returning the count.
pub fn poll_read_len(fd: i32, timeout_ms: i32) -> usize {
    let mut total = 0usize;
    let mut buf = [0u8; 65536];
    loop {
        let mut p = libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        };
        let rc = unsafe { libc::poll(&mut p, 1, timeout_ms) };
        if rc <= 0 {
            break;
        }
        let n = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
        if n <= 0 {
            break;
        }
        total += n as usize;
    }
    total
}

/// Read up to `cap` bytes available on `fd` within `timeout_ms`.
pub fn poll_read_bytes(fd: i32, timeout_ms: i32, cap: usize) -> Vec<u8> {
    let mut out = Vec::new();
    let mut buf = vec![0u8; cap.max(1)];
    loop {
        let mut p = libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        };
        let rc = unsafe { libc::poll(&mut p, 1, timeout_ms) };
        if rc <= 0 {
            break;
        }
        let n = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
        if n <= 0 {
            break;
        }
        out.extend_from_slice(&buf[..n as usize]);
        if out.len() >= cap {
            break;
        }
    }
    out
}

/// Read `path` if it exists, else empty.
pub fn read_or_empty(p: &Path) -> Vec<u8> {
    fs::read(p).unwrap_or_default()
}

/// Helper for tests that need the whole input echoed through a pipe-fed stdin.
pub fn run_stdin_pipe_incremental(exe: &Path, pieces: &[&[u8]]) -> Outcome {
    let out_path = temp_path("out");
    let fout = fs::File::create(&out_path).expect("create stdout temp");
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::from(fout))
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    {
        let mut si = child.stdin.take().expect("stdin");
        for p in pieces {
            si.write_all(p).expect("write piece");
            si.flush().ok();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    } // dropping stdin closes it -> EOF
    let out = child.wait_with_output().expect("wait");
    let mut stdout = fs::File::open(&out_path).expect("open out");
    let mut bytes = Vec::new();
    stdout.read_to_end(&mut bytes).expect("read out");
    let _ = fs::remove_file(&out_path);
    Outcome {
        status: status_of(out.status),
        stdout: bytes,
        stderr: out.stderr,
    }
}

/// C strings for the fork-based FFI harness.
pub fn cstr(p: &Path) -> CString {
    CString::new(p.as_os_str().as_encoded_bytes()).expect("path has NUL")
}
