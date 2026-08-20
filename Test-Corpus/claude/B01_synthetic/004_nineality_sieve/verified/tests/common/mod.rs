// Differential-test harness: runs the ORIGINAL C binary and the TRANSLATED Rust
// binary through their real external boundary (process invocation) and compares
// stdout / stderr / exit status byte-for-byte.
//
// This crate builds an EXECUTABLE (c_src/CMakeLists.txt: add_executable), so
// there is no shared object to `dlopen`; the process contract *is* the exported
// surface. Rust code is never called in-process — always via the built binary,
// exactly as an external caller would.
#![allow(dead_code)]

use std::ffi::{CString, OsStr, OsString};
use std::io::Read;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

// ---------------------------------------------------------------------------
// raw libc bits we need (no libc crate available offline)
// ---------------------------------------------------------------------------
extern "C" {
    fn close(fd: i32) -> i32;
    fn fork() -> i32;
    fn execv(path: *const i8, argv: *const *const i8) -> i32;
    fn pipe(fds: *mut i32) -> i32;
    fn dup2(old: i32, new: i32) -> i32;
    fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
    fn read(fd: i32, buf: *mut u8, count: usize) -> isize;
    fn kill(pid: i32, sig: i32) -> i32;
    fn _exit(code: i32) -> !;
}

const SIGKILL: i32 = 9;

// ---------------------------------------------------------------------------
// binaries under test
// ---------------------------------------------------------------------------

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The translated Rust binary, as an external caller sees it.
pub fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// The original C binary (ground truth). Built with cmake on first use.
pub fn c_bin() -> PathBuf {
    static C: OnceLock<PathBuf> = OnceLock::new();
    C.get_or_init(|| {
        static LOCK: Mutex<()> = Mutex::new(());
        let _g = LOCK.lock().unwrap();
        let src = manifest_dir().join("c_src");
        let build = src.join("build");
        let bin = build.join("driver");
        if !bin.exists() {
            std::fs::create_dir_all(&build).expect("create c_src/build");
            let st = Command::new("cmake")
                .current_dir(&build)
                .arg("..")
                .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
                .stdout(Stdio::null())
                .stderr(Stdio::inherit())
                .status()
                .expect("run cmake (is cmake installed?)");
            assert!(st.success(), "cmake configure failed");
            let st = Command::new("cmake")
                .current_dir(&build)
                .args(["--build", "."])
                .stdout(Stdio::null())
                .stderr(Stdio::inherit())
                .status()
                .expect("run cmake --build");
            assert!(st.success(), "cmake build failed");
        }
        assert!(bin.exists(), "C binary missing at {}", bin.display());
        bin
    })
    .clone()
}

/// Unique id so concurrently running tests never share a temp path.
pub fn unique_id() -> String {
    static N: AtomicU64 = AtomicU64::new(0);
    format!(
        "{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    )
}

pub fn tmp_dir() -> PathBuf {
    let d = manifest_dir().join("target").join("difftest-tmp");
    std::fs::create_dir_all(&d).ok();
    d
}

// ---------------------------------------------------------------------------
// run specification
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StdoutTarget {
    /// stdout is a pipe read by the harness (fully buffered stdio in C).
    Pipe,
    /// stdout is a regular file.
    File,
    /// stdout is /dev/null.
    DevNull,
    /// fd 1 is closed before exec: every write fails.
    Closed,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StdinTarget {
    Inherit,
    Null,
    Closed,
    /// a file containing some bytes; the program must not read them
    FileWithData,
}

#[derive(Clone)]
pub struct Spec {
    /// operands appended after argv[0]
    pub args: Vec<Vec<u8>>,
    /// argv[0] override (the program never reads it)
    pub arg0: Option<Vec<u8>>,
    /// env vars to set
    pub env_set: Vec<(String, String)>,
    /// env vars to remove
    pub env_remove: Vec<String>,
    /// wipe the environment first
    pub env_clear: bool,
    pub stdout: StdoutTarget,
    pub stdin: StdinTarget,
    /// stop reading stdout after this many bytes
    pub cap: usize,
    /// hard wall-clock limit before SIGKILL
    pub timeout: Duration,
}

impl Spec {
    pub fn new<I, S>(args: I) -> Spec
    where
        I: IntoIterator<Item = S>,
        S: AsRef<[u8]>,
    {
        Spec {
            args: args.into_iter().map(|a| a.as_ref().to_vec()).collect(),
            arg0: None,
            env_set: Vec::new(),
            env_remove: Vec::new(),
            env_clear: false,
            stdout: StdoutTarget::Pipe,
            stdin: StdinTarget::Inherit,
            cap: 1 << 22, // 4 MiB
            timeout: Duration::from_secs(60),
        }
    }

    pub fn one(arg: impl AsRef<[u8]>) -> Spec {
        Spec::new([arg.as_ref().to_vec()])
    }

    pub fn cap(mut self, cap: usize) -> Spec {
        self.cap = cap;
        self
    }
    pub fn stdout(mut self, t: StdoutTarget) -> Spec {
        self.stdout = t;
        self
    }
    pub fn stdin(mut self, t: StdinTarget) -> Spec {
        self.stdin = t;
        self
    }
    pub fn arg0(mut self, a: impl AsRef<[u8]>) -> Spec {
        self.arg0 = Some(a.as_ref().to_vec());
        self
    }
    pub fn env(mut self, k: &str, v: &str) -> Spec {
        self.env_set.push((k.to_string(), v.to_string()));
        self
    }
    pub fn env_remove(mut self, k: &str) -> Spec {
        self.env_remove.push(k.to_string());
        self
    }
    pub fn env_clear(mut self) -> Spec {
        self.env_clear = true;
        self
    }
    pub fn timeout(mut self, d: Duration) -> Spec {
        self.timeout = d;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Out {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    /// exit code, if the process exited normally
    pub code: Option<i32>,
    /// terminating signal, if any
    pub signal: Option<i32>,
    /// true when the harness stopped reading at `cap` and killed the process
    pub truncated: bool,
}

fn bytes_to_os(b: &[u8]) -> OsString {
    OsString::from_vec(b.to_vec())
}

fn watchdog(pid: i32, dur: Duration) -> Arc<AtomicBool> {
    let done = Arc::new(AtomicBool::new(false));
    let flag = done.clone();
    std::thread::spawn(move || {
        let step = Duration::from_millis(25);
        let mut waited = Duration::ZERO;
        while waited < dur {
            if flag.load(Ordering::SeqCst) {
                return;
            }
            std::thread::sleep(step);
            waited += step;
        }
        if !flag.load(Ordering::SeqCst) {
            unsafe { kill(pid, SIGKILL) };
        }
    });
    done
}

/// Run one binary under `spec`, reading at most `spec.cap` stdout bytes.
pub fn run(bin: &Path, spec: &Spec) -> Out {
    let mut cmd = Command::new(bin);
    for a in &spec.args {
        cmd.arg(bytes_to_os(a));
    }
    if let Some(a0) = &spec.arg0 {
        cmd.arg0(bytes_to_os(a0));
    }
    if spec.env_clear {
        cmd.env_clear();
    }
    for k in &spec.env_remove {
        cmd.env_remove(k);
    }
    for (k, v) in &spec.env_set {
        cmd.env(k, v);
    }

    // stdin
    let stdin_file = match spec.stdin {
        StdinTarget::Inherit => {
            cmd.stdin(Stdio::null());
            None
        }
        StdinTarget::Null => {
            cmd.stdin(Stdio::null());
            None
        }
        StdinTarget::Closed => {
            unsafe {
                cmd.pre_exec(|| {
                    close(0);
                    Ok(())
                });
            }
            cmd.stdin(Stdio::null());
            None
        }
        StdinTarget::FileWithData => {
            let p = tmp_dir().join(format!("stdin-{}.txt", unique_id()));
            std::fs::write(&p, b"42\n7\nhello\n").unwrap();
            let f = std::fs::File::open(&p).unwrap();
            cmd.stdin(Stdio::from(f));
            Some(p)
        }
    };
    let _ = stdin_file;

    // stdout
    let mut out_file_path: Option<PathBuf> = None;
    match spec.stdout {
        StdoutTarget::Pipe => {
            cmd.stdout(Stdio::piped());
        }
        StdoutTarget::File => {
            let name = format!(
                "stdout-{}-{}.bin",
                bin.file_name().unwrap().to_string_lossy(),
                unique_id()
            );
            let p = tmp_dir().join(name);
            let f = std::fs::File::create(&p).unwrap();
            cmd.stdout(Stdio::from(f));
            out_file_path = Some(p);
        }
        StdoutTarget::DevNull => {
            cmd.stdout(Stdio::null());
        }
        StdoutTarget::Closed => {
            unsafe {
                cmd.pre_exec(|| {
                    close(1);
                    Ok(())
                });
            }
            cmd.stdout(Stdio::null());
        }
    }
    cmd.stderr(Stdio::piped());

    let mut child: Child = cmd.spawn().unwrap_or_else(|e| {
        panic!("failed to spawn {}: {e}", bin.display());
    });
    let pid = child.id() as i32;
    let wd = watchdog(pid, spec.timeout);

    // drain stderr on a thread so it can never deadlock the pipe
    let mut err_handle = child.stderr.take().unwrap();
    let err_thread = std::thread::spawn(move || {
        let mut v = Vec::new();
        let _ = err_handle.read_to_end(&mut v);
        v
    });

    let mut stdout_bytes = Vec::new();
    let mut truncated = false;
    if let Some(mut h) = child.stdout.take() {
        let mut buf = vec![0u8; 64 * 1024];
        loop {
            if stdout_bytes.len() >= spec.cap {
                truncated = true;
                break;
            }
            let want = std::cmp::min(buf.len(), spec.cap - stdout_bytes.len());
            match h.read(&mut buf[..want]) {
                Ok(0) => break,
                Ok(n) => stdout_bytes.extend_from_slice(&buf[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
        if truncated {
            let _ = child.kill();
        }
        drop(h);
    }

    let status = child.wait().expect("wait");
    wd.store(true, Ordering::SeqCst);
    let stderr_bytes = err_thread.join().unwrap_or_default();

    if let Some(p) = &out_file_path {
        let data = std::fs::read(p).unwrap_or_default();
        if data.len() > spec.cap {
            truncated = true;
            stdout_bytes = data[..spec.cap].to_vec();
        } else {
            stdout_bytes = data;
        }
        let _ = std::fs::remove_file(p);
    }

    Out {
        stdout: stdout_bytes,
        stderr: stderr_bytes,
        code: status.code(),
        signal: status.signal(),
        truncated,
    }
}

// ---------------------------------------------------------------------------
// comparison helpers
// ---------------------------------------------------------------------------

fn show(b: &[u8]) -> String {
    let head: Vec<u8> = b.iter().copied().take(160).collect();
    let mut s = String::new();
    for c in head {
        match c {
            b'\n' => s.push_str("\\n"),
            b'\t' => s.push_str("\\t"),
            0x20..=0x7e => s.push(c as char),
            other => s.push_str(&format!("\\x{other:02x}")),
        }
    }
    if b.len() > 160 {
        s.push_str(&format!("…(+{} bytes)", b.len() - 160));
    }
    s
}

pub fn describe(spec: &Spec) -> String {
    let args: Vec<String> = spec.args.iter().map(|a| show(a)).collect();
    format!(
        "args={:?} arg0={:?} stdout={:?} stdin={:?} env_clear={} env_set={:?} cap={}",
        args,
        spec.arg0.as_ref().map(|a| show(a)),
        spec.stdout,
        spec.stdin,
        spec.env_clear,
        spec.env_set,
        spec.cap
    )
}

/// The core assertion: identical observable behaviour from both binaries.
///
/// When neither run hit the byte cap this is a complete comparison (stdout,
/// stderr, exit code, terminating signal). When both hit the cap, the compared
/// stdout prefix must still match byte-for-byte.
pub fn assert_same(spec: &Spec) {
    let c = run(&c_bin(), spec);
    let r = run(&rust_bin(), spec);

    let ctx = describe(spec);
    assert_eq!(
        c.truncated, r.truncated,
        "truncation flag differs ({ctx}); C len={} rust len={}",
        c.stdout.len(),
        r.stdout.len()
    );
    if c.stdout != r.stdout {
        let first = c
            .stdout
            .iter()
            .zip(r.stdout.iter())
            .position(|(a, b)| a != b)
            .unwrap_or_else(|| std::cmp::min(c.stdout.len(), r.stdout.len()));
        let from = first.saturating_sub(40);
        panic!(
            "stdout mismatch ({ctx})\n  first difference at byte {first}\n  C   len={} …{}\n  RUST len={} …{}",
            c.stdout.len(),
            show(&c.stdout[from..std::cmp::min(c.stdout.len(), from + 160)]),
            r.stdout.len(),
            show(&r.stdout[from..std::cmp::min(r.stdout.len(), from + 160)]),
        );
    }
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch ({ctx}): C={:?} RUST={:?}",
        show(&c.stderr),
        show(&r.stderr)
    );
    if !c.truncated {
        assert_eq!(c.code, r.code, "exit code mismatch ({ctx})");
        assert_eq!(c.signal, r.signal, "terminating signal mismatch ({ctx})");
    }
}

/// Convenience: single operand, full comparison.
pub fn same_arg(arg: impl AsRef<[u8]>) {
    assert_same(&Spec::one(arg));
}

/// Convenience: single operand, bounded comparison (for ~2^31-iteration runs).
pub fn same_arg_bounded(arg: impl AsRef<[u8]>, cap: usize) {
    assert_same(&Spec::one(arg).cap(cap));
}

/// Assert the observed C behaviour matches an expected literal, so the tests
/// pin the real C contract (and not merely "both agree").
pub fn assert_c_result(spec: &Spec, expect_stdout: &[u8], expect_code: i32) {
    let c = run(&c_bin(), spec);
    assert_eq!(
        c.stdout,
        expect_stdout,
        "C stdout not as documented ({}): got {:?}",
        describe(spec),
        show(&c.stdout)
    );
    assert_eq!(
        c.code,
        Some(expect_code),
        "C exit code not as documented ({})",
        describe(spec)
    );
}

pub const E_ARGC: &[u8] = b"Error: should only be a single (integer) argument!\n";
pub const E_INT: &[u8] = b"Error: first argument must be an integer!\n";

// ---------------------------------------------------------------------------
// argc == 0: needs a raw execve with argv = {NULL}
// ---------------------------------------------------------------------------

/// Execute `bin` with a completely empty argv (argc == 0) and capture stdout.
/// Returns (stdout, exit code or -signal).
pub fn run_argc_zero(bin: &Path) -> (Vec<u8>, i32) {
    let path = CString::new(bin.as_os_str().as_bytes()).unwrap();
    let argv: [*const i8; 1] = [std::ptr::null()];
    let mut fds = [0i32; 2];
    assert_eq!(unsafe { pipe(fds.as_mut_ptr()) }, 0, "pipe failed");
    let (rd, wr) = (fds[0], fds[1]);

    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        // child: only exec-safe calls from here on
        unsafe {
            close(rd);
            dup2(wr, 1);
            close(wr);
            execv(path.as_ptr(), argv.as_ptr());
            _exit(127);
        }
    }
    unsafe { close(wr) };
    let mut out = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        let n = unsafe { read(rd, buf.as_mut_ptr(), buf.len()) };
        if n <= 0 {
            break;
        }
        out.extend_from_slice(&buf[..n as usize]);
        if out.len() > 1 << 20 {
            unsafe { kill(pid, SIGKILL) };
            break;
        }
    }
    unsafe { close(rd) };
    let mut status = 0i32;
    unsafe { waitpid(pid, &mut status, 0) };
    let code = if status & 0x7f == 0 {
        (status >> 8) & 0xff
    } else {
        -(status & 0x7f)
    };
    (out, code)
}

// ---------------------------------------------------------------------------
// SIGPIPE parity: reader closes the pipe while the program is still writing
// ---------------------------------------------------------------------------

/// Spawn `bin arg`, read `read_bytes` from its stdout, close the pipe, and
/// report (bytes read, exit code, terminating signal).
pub fn run_then_close_pipe(
    bin: &Path,
    arg: &[u8],
    read_bytes: usize,
) -> (Vec<u8>, Option<i32>, Option<i32>) {
    let mut child = Command::new(bin)
        .arg(bytes_to_os(arg))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn");
    let pid = child.id() as i32;
    let wd = watchdog(pid, Duration::from_secs(60));
    let mut got = Vec::new();
    {
        let h = child.stdout.as_mut().unwrap();
        let mut buf = vec![0u8; 4096];
        while got.len() < read_bytes {
            match h.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => got.extend_from_slice(&buf[..n]),
                Err(_) => break,
            }
        }
    }
    // close the read end while the child is (very likely) still writing
    drop(child.stdout.take());
    let st = child.wait().expect("wait");
    wd.store(true, Ordering::SeqCst);
    (got, st.code(), st.signal())
}

// ---------------------------------------------------------------------------
// deep streaming comparison
// ---------------------------------------------------------------------------

struct Streamer {
    child: Child,
    out: std::process::ChildStdout,
    buf: Vec<u8>,
    pos: usize,
    eof: bool,
}

impl Streamer {
    fn spawn(bin: &Path, arg: &[u8]) -> Streamer {
        let mut child = Command::new(bin)
            .arg(bytes_to_os(arg))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn");
        let out = child.stdout.take().unwrap();
        Streamer {
            child,
            out,
            buf: Vec::new(),
            pos: 0,
            eof: false,
        }
    }
    fn avail(&self) -> usize {
        self.buf.len() - self.pos
    }
    /// Read more bytes into the buffer. Returns false at EOF.
    fn fill(&mut self) -> bool {
        if self.eof {
            return false;
        }
        if self.pos > 0 {
            self.buf.drain(..self.pos);
            self.pos = 0;
        }
        let mut chunk = [0u8; 256 * 1024];
        match self.out.read(&mut chunk) {
            Ok(0) => {
                self.eof = true;
                false
            }
            Ok(n) => {
                self.buf.extend_from_slice(&chunk[..n]);
                true
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => true,
            Err(_) => {
                self.eof = true;
                false
            }
        }
    }
    fn consume(&mut self, n: usize) {
        self.pos += n;
    }
    fn finish(mut self, kill: bool) -> (Option<i32>, Option<i32>) {
        if kill {
            let _ = self.child.kill();
        }
        drop(self.out);
        let st = self.child.wait().expect("wait");
        (st.code(), st.signal())
    }
}

/// Compare up to `max_bytes` of the two stdout streams *incrementally*, without
/// buffering the whole (potentially multi-gigabyte) output. When both streams
/// end before the limit the comparison is exact and includes the total length
/// and the exit status.
pub fn assert_same_streaming(arg: &[u8], max_bytes: u64) {
    let mut c = Streamer::spawn(&c_bin(), arg);
    let mut r = Streamer::spawn(&rust_bin(), arg);
    let mut compared: u64 = 0;
    let ctx = show(arg);

    loop {
        if compared >= max_bytes {
            let (cc, cs) = c.finish(true);
            let (rc, rs) = r.finish(true);
            let _ = (cc, cs, rc, rs);
            return;
        }
        while c.avail() == 0 && !c.eof {
            c.fill();
        }
        while r.avail() == 0 && !r.eof {
            r.fill();
        }
        let n = std::cmp::min(c.avail(), r.avail());
        if n == 0 {
            // at least one side is finished
            assert_eq!(
                c.eof && c.avail() == 0,
                r.eof && r.avail() == 0,
                "stream lengths differ for arg {ctx}: compared {compared} bytes, \
                 C leftover={} eof={} RUST leftover={} eof={}",
                c.avail(),
                c.eof,
                r.avail(),
                r.eof
            );
            break;
        }
        let (cs, rs) = (&c.buf[c.pos..c.pos + n], &r.buf[r.pos..r.pos + n]);
        if cs != rs {
            let off = cs.iter().zip(rs.iter()).position(|(a, b)| a != b).unwrap();
            let from = off.saturating_sub(32);
            panic!(
                "deep stream mismatch for arg {ctx} at byte {}\n  C   …{}\n  RUST…{}",
                compared + off as u64,
                show(&cs[from..std::cmp::min(cs.len(), from + 96)]),
                show(&rs[from..std::cmp::min(rs.len(), from + 96)])
            );
        }
        c.consume(n);
        r.consume(n);
        compared += n as u64;
    }

    let (c_code, c_sig) = c.finish(false);
    let (r_code, r_sig) = r.finish(false);
    assert_eq!(c_code, r_code, "exit code differs for arg {ctx}");
    assert_eq!(c_sig, r_sig, "signal differs for arg {ctx}");
    assert!(compared > 0, "no output compared for arg {ctx}");
}

// ---------------------------------------------------------------------------
// deterministic PRNG (SplitMix64) — fixed seeds keep failures reproducible
// ---------------------------------------------------------------------------

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
    /// uniform in [0, n)
    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0);
        self.next_u64() % n
    }
    pub fn range_i64(&mut self, lo: i64, hi: i64) -> i64 {
        // inclusive
        let span = (hi as i128 - lo as i128 + 1) as u128;
        (lo as i128 + (self.next_u64() as u128 % span) as i128) as i64
    }
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len() as u64) as usize]
    }
    pub fn digits(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| b'0' + self.below(10) as u8).collect()
    }
}

/// C-locale whitespace, i.e. exactly what `strtol` skips.
pub const C_SPACES: [u8; 6] = [b' ', b'\t', b'\n', 0x0b, 0x0c, b'\r'];

pub fn os(b: &[u8]) -> &OsStr {
    OsStr::from_bytes(b)
}

// ---------------------------------------------------------------------------
// stdout is a TTY: glibc switches to line buffering, Rust uses a BufWriter.
// The byte stream at EOF must still be identical (including the pty's ONLCR
// \n -> \r\n translation, which both processes are subject to).
// ---------------------------------------------------------------------------

extern "C" {
    fn posix_openpt(flags: i32) -> i32;
    fn grantpt(fd: i32) -> i32;
    fn unlockpt(fd: i32) -> i32;
    fn ptsname(fd: i32) -> *const i8;
    fn setrlimit(resource: i32, rlim: *const RLimit) -> i32;
}

#[repr(C)]
pub struct RLimit {
    pub cur: u64,
    pub max: u64,
}

const O_RDWR: i32 = 2;
const O_NOCTTY: i32 = 0o400;
const RLIMIT_FSIZE: i32 = 1;

/// Run `bin arg` with stdout connected to a pseudo-terminal; returns the bytes
/// the pty master sees plus the exit status.
pub fn run_on_pty(bin: &Path, arg: &[u8]) -> (Vec<u8>, Option<i32>, Option<i32>) {
    let master = unsafe { posix_openpt(O_RDWR | O_NOCTTY) };
    assert!(master >= 0, "posix_openpt failed");
    assert_eq!(unsafe { grantpt(master) }, 0, "grantpt failed");
    assert_eq!(unsafe { unlockpt(master) }, 0, "unlockpt failed");
    let name = unsafe {
        let p = ptsname(master);
        assert!(!p.is_null(), "ptsname failed");
        std::ffi::CStr::from_ptr(p).to_owned()
    };
    let slave = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(std::path::Path::new(
            std::str::from_utf8(name.to_bytes()).unwrap(),
        ))
        .expect("open pty slave");

    let mut child = Command::new(bin)
        .arg(bytes_to_os(arg))
        .stdin(Stdio::null())
        .stdout(Stdio::from(slave))
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn on pty");
    let pid = child.id() as i32;
    let wd = watchdog(pid, Duration::from_secs(30));

    // our copy of the slave fd is gone (moved into the child), so once the child
    // exits the master read returns EIO, which we treat as EOF.
    let st = child.wait().expect("wait");
    wd.store(true, Ordering::SeqCst);

    let mut out = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        let n = unsafe { read(master, buf.as_mut_ptr(), buf.len()) };
        if n <= 0 {
            break;
        }
        out.extend_from_slice(&buf[..n as usize]);
        if out.len() > 1 << 20 {
            break;
        }
    }
    unsafe { close(master) };
    (out, st.code(), st.signal())
}

/// Run `bin arg` with stdout on a regular file and RLIMIT_FSIZE set to `limit`,
/// so writes past the limit raise SIGXFSZ. Returns (file bytes, code, signal).
pub fn run_with_fsize_limit(
    bin: &Path,
    arg: &[u8],
    limit: u64,
) -> (Vec<u8>, Option<i32>, Option<i32>) {
    let path = tmp_dir().join(format!("fsize-{}.bin", unique_id()));
    let f = std::fs::File::create(&path).unwrap();
    let mut cmd = Command::new(bin);
    cmd.arg(bytes_to_os(arg))
        .stdin(Stdio::null())
        .stdout(Stdio::from(f))
        .stderr(Stdio::null());
    unsafe {
        cmd.pre_exec(move || {
            let rl = RLimit {
                cur: limit,
                max: limit,
            };
            setrlimit(RLIMIT_FSIZE, &rl);
            Ok(())
        });
    }
    let mut child = cmd.spawn().expect("spawn with rlimit");
    let wd = watchdog(child.id() as i32, Duration::from_secs(30));
    let st = child.wait().expect("wait");
    wd.store(true, Ordering::SeqCst);
    let data = std::fs::read(&path).unwrap_or_default();
    let _ = std::fs::remove_file(&path);
    (data, st.code(), st.signal())
}
