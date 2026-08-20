//! Shared differential-test harness.
//!
//! Both implementations are exercised **only** through their exported C symbols
//! in a freshly loaded shared object:
//!
//! * C:    `target/cbuild/libc_driver.so`  (built from `c_src/src/main.c`)
//! * Rust: `target/<profile>/libdriver.so` (the `cdylib` of this crate)
//!
//! `examples/so_runner.rs` does the `libloading` work; every call below spawns
//! that runner once per library so the two runs are completely independent
//! processes with identical environments.

#![allow(dead_code)]

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// paths & building
// ---------------------------------------------------------------------------

pub fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `target/debug` or `target/release`, derived from the running test binary.
pub fn profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test>-<hash>
    exe.parent()
        .and_then(|p| p.parent())
        .expect("profile dir")
        .to_path_buf()
}

fn is_release() -> bool {
    profile_dir().file_name().map(|s| s == "release").unwrap_or(false)
}

pub fn tmp_dir() -> PathBuf {
    let d = profile_dir().join("difftest-tmp");
    fs::create_dir_all(&d).expect("create tmp dir");
    d
}

fn newer(a: &Path, b: &Path) -> bool {
    let ma = fs::metadata(a).and_then(|m| m.modified());
    let mb = fs::metadata(b).and_then(|m| m.modified());
    match (ma, mb) {
        (Ok(x), Ok(y)) => x > y,
        _ => true,
    }
}

/// Build `c_src/src/main.c` into a shared object (nothing in `c_src/` is
/// modified; the artifact lands in `target/cbuild/`).
pub fn c_so() -> &'static PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let src = crate_root().join("c_src/src/main.c");
        let out_dir = crate_root().join("target/cbuild");
        fs::create_dir_all(&out_dir).expect("create target/cbuild");
        let so = out_dir.join("libc_driver.so");
        if !so.exists() || newer(&src, &so) {
            let st = Command::new("gcc")
                .args(["-shared", "-fPIC", "-o"])
                .arg(&so)
                .arg(&src)
                .status()
                .expect("run gcc");
            assert!(st.success(), "gcc failed to build the C shared library");
        }
        so
    })
}

/// The same C translation unit built with `-O2`, used to prove that the
/// program's out-of-bounds write (`s[strlen(s)-1]` with `strlen==0`) really is
/// unobservable and not an artifact of the unoptimized build.
pub fn c_so_opt() -> &'static PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let src = crate_root().join("c_src/src/main.c");
        let out_dir = crate_root().join("target/cbuild");
        fs::create_dir_all(&out_dir).expect("create target/cbuild");
        let so = out_dir.join("libc_driver_O2.so");
        if !so.exists() || newer(&src, &so) {
            let st = Command::new("gcc")
                .args(["-shared", "-fPIC", "-O2", "-o"])
                .arg(&so)
                .arg(&src)
                .status()
                .expect("run gcc");
            assert!(st.success(), "gcc -O2 failed to build the C shared library");
        }
        so
    })
}

/// The C executable built exactly as `c_src/CMakeLists.txt` describes.
pub fn c_exe() -> &'static PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let build = crate_root().join("c_src/build");
        let exe = build.join("driver");
        if !exe.exists() {
            fs::create_dir_all(&build).expect("create c_src/build");
            let st = Command::new("cmake")
                .current_dir(&build)
                .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
                .status()
                .expect("run cmake");
            assert!(st.success(), "cmake configure failed");
            let st = Command::new("cmake")
                .current_dir(&build)
                .args(["--build", "."])
                .status()
                .expect("run cmake --build");
            assert!(st.success(), "cmake build failed");
        }
        exe
    })
}

fn cargo_build(extra: &[&str]) {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut cmd = Command::new(cargo);
    cmd.current_dir(crate_root()).arg("build").arg("--offline");
    if is_release() {
        cmd.arg("--release");
    }
    cmd.args(extra);
    let st = cmd.status().expect("run cargo build");
    assert!(st.success(), "cargo build {extra:?} failed");
}

/// The Rust `cdylib`, rebuilt on demand (`cargo test` does not necessarily emit
/// the cdylib artifact by itself).
pub fn rust_so() -> &'static PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let so = profile_dir().join("libdriver.so");
        let srcs = ["src/lib.rs", "src/core.rs"];
        let stale = !so.exists()
            || srcs
                .iter()
                .any(|s| newer(&crate_root().join(s), &so));
        if stale {
            cargo_build(&["--lib"]);
        }
        assert!(so.exists(), "missing Rust cdylib at {}", so.display());
        so
    })
}

/// The Rust executable (mirrors the C `main`).
pub fn rust_exe() -> &'static PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let exe = profile_dir().join("driver");
        if !exe.exists() || newer(&crate_root().join("src/main.rs"), &exe) {
            cargo_build(&["--bin", "driver"]);
        }
        assert!(exe.exists(), "missing Rust binary at {}", exe.display());
        exe
    })
}

/// The `libloading` runner used for every FFI call.
pub fn runner() -> &'static PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let exe = profile_dir().join("examples/so_runner");
        if !exe.exists() || newer(&crate_root().join("examples/so_runner.rs"), &exe) {
            cargo_build(&["--example", "so_runner"]);
        }
        assert!(exe.exists(), "missing so_runner at {}", exe.display());
        exe
    })
}

// ---------------------------------------------------------------------------
// process outcome
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq)]
pub struct Out {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub code: Option<i32>,
    pub signal: Option<i32>,
}

impl std::fmt::Debug for Out {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Out {{ code: {:?}, signal: {:?}, stdout: {:?}, stderr: {:?} }}",
            self.code,
            self.signal,
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr)
        )
    }
}

impl Out {
    /// Everything an external observer can see, except stderr (which the C code
    /// never writes to and which therefore is compared separately).
    pub fn observable(&self) -> (Vec<u8>, Option<i32>, Option<i32>) {
        (self.stdout.clone(), self.code, self.signal)
    }
}

fn finish(out: std::process::Output) -> Out {
    use std::os::unix::process::ExitStatusExt;
    Out {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

// ---------------------------------------------------------------------------
// invoking the libraries
// ---------------------------------------------------------------------------

pub fn hex(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return ".".to_string();
    }
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

fn write_cases(label: &str, cases: &[(Vec<u8>, Vec<u8>)]) -> PathBuf {
    let p = tmp_dir().join(format!("{label}.cases"));
    let mut f = File::create(&p).expect("create casefile");
    for (a, b) in cases {
        writeln!(f, "{} {}", hex(a), hex(b)).expect("write casefile");
    }
    f.flush().expect("flush casefile");
    p
}

/// `driver(s1, s2)` once per case, all in one process (also covers repeated
/// calls / output buffering).
pub fn run_driver_batch(so: &Path, label: &str, cases: &[(Vec<u8>, Vec<u8>)]) -> Out {
    let casefile = write_cases(label, cases);
    let out = Command::new(runner())
        .arg("driver")
        .arg(so)
        .arg(&casefile)
        .stdin(Stdio::null())
        .output()
        .expect("spawn runner");
    finish(out)
}

/// `driver` with a NULL pointer for `which` in {"s1","s2","both"}.
pub fn run_driver_null(so: &Path, which: &str, other: &[u8]) -> Out {
    let out = Command::new(runner())
        .arg("driver-null")
        .arg(so)
        .arg(which)
        .arg(hex(other))
        .stdin(Stdio::null())
        .output()
        .expect("spawn runner");
    finish(out)
}

#[derive(Clone, Copy)]
pub enum StdinKind<'a> {
    /// stdin is a regular (seekable) file holding these bytes
    File(&'a [u8]),
    /// stdin is a pipe (non-seekable) fed with these bytes
    Pipe(&'a [u8]),
    /// stdin is a pipe fed in small chunks with pauses in between, so that
    /// `read(2)` returns short reads in the middle of a line
    PipeChunked(&'a [u8], usize),
    /// stdin is /dev/null
    DevNull,
    /// stdin is an open **directory** (reads fail with EISDIR)
    Directory,
    /// file descriptor 0 is closed before exec
    Closed,
}

extern "C" {
    fn close(fd: i32) -> i32;
}

/// `driver` with a non-NULL but invalid pointer (address 1) for `which`.
pub fn run_driver_bogus(so: &Path, which: &str, other: &[u8]) -> Out {
    let out = Command::new(runner())
        .arg("driver-bogus")
        .arg(so)
        .arg(which)
        .arg(hex(other))
        .stdin(Stdio::null())
        .output()
        .expect("spawn runner");
    finish(out)
}

/// Call the library's exported `main()` with the given stdin.
pub fn run_main(so: &Path, label: &str, kind: StdinKind<'_>) -> Out {
    let mut cmd = Command::new(runner());
    cmd.arg("main").arg(so);
    run_with_stdin(cmd, label, kind)
}

fn run_with_stdin(cmd: Command, label: &str, kind: StdinKind<'_>) -> Out {
    run_with_stdin_inner(cmd, label, kind, true)
}

fn run_with_stdin_inner(
    mut cmd: Command,
    label: &str,
    kind: StdinKind<'_>,
    set_stdout: bool,
) -> Out {
    match kind {
        StdinKind::File(bytes) => {
            let p = tmp_dir().join(format!("{label}.in"));
            fs::write(&p, bytes).expect("write stdin file");
            cmd.stdin(Stdio::from(File::open(&p).expect("open stdin file")));
        }
        StdinKind::Pipe(_) | StdinKind::PipeChunked(_, _) => {
            cmd.stdin(Stdio::piped());
        }
        StdinKind::DevNull => {
            cmd.stdin(Stdio::null());
        }
        StdinKind::Directory => {
            let d = tmp_dir();
            cmd.stdin(Stdio::from(File::open(&d).expect("open directory")));
        }
        StdinKind::Closed => {
            cmd.stdin(Stdio::null());
            // SAFETY: only async-signal-safe work between fork and exec.
            unsafe {
                use std::os::unix::process::CommandExt;
                cmd.pre_exec(|| {
                    close(0);
                    Ok(())
                });
            }
        }
    }
    if set_stdout {
        cmd.stdout(Stdio::piped());
    }
    cmd.stderr(Stdio::piped());

    match kind {
        StdinKind::Pipe(bytes) => {
            let bytes = bytes.to_vec();
            let mut child = cmd.spawn().expect("spawn runner");
            {
                let mut si = child.stdin.take().expect("child stdin");
                std::thread::spawn(move || {
                    let _ = si.write_all(&bytes);
                });
            }
            finish(child.wait_with_output().expect("wait runner"))
        }
        StdinKind::PipeChunked(bytes, chunk) => {
            let bytes = bytes.to_vec();
            let chunk = chunk.max(1);
            let mut child = cmd.spawn().expect("spawn runner");
            {
                let mut si = child.stdin.take().expect("child stdin");
                std::thread::spawn(move || {
                    for part in bytes.chunks(chunk) {
                        if si.write_all(part).is_err() {
                            return;
                        }
                        let _ = si.flush();
                        std::thread::sleep(std::time::Duration::from_millis(2));
                    }
                });
            }
            finish(child.wait_with_output().expect("wait runner"))
        }
        _ => finish(cmd.output().expect("spawn runner")),
    }
}

/// What stdout is connected to.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StdoutKind {
    /// captured pipe (the normal case)
    Capture,
    /// pipe whose read end is already closed -> writes fail with EPIPE/SIGPIPE
    BrokenPipe,
    /// `/dev/full` -> writes fail with ENOSPC
    Full,
    /// file descriptor 1 closed before exec -> writes fail with EBADF
    Closed,
}

fn spawn_with_stdout(
    mut cmd: Command,
    label: &str,
    stdin: StdinKind<'_>,
    stdout: StdoutKind,
    reset_sigpipe: bool,
) -> Out {
    if reset_sigpipe {
        cmd.env("RUNNER_RESET_SIGPIPE", "1");
    }
    match stdout {
        StdoutKind::Capture => {
            cmd.stdout(Stdio::piped());
        }
        StdoutKind::BrokenPipe => {
            let (r, w) = std::io::pipe().expect("create pipe");
            drop(r); // no reader
            cmd.stdout(Stdio::from(w));
        }
        StdoutKind::Full => {
            cmd.stdout(Stdio::from(
                File::options()
                    .write(true)
                    .open("/dev/full")
                    .expect("open /dev/full"),
            ));
        }
        StdoutKind::Closed => {
            cmd.stdout(Stdio::null());
            // SAFETY: only async-signal-safe work between fork and exec.
            unsafe {
                use std::os::unix::process::CommandExt;
                cmd.pre_exec(|| {
                    close(1);
                    Ok(())
                });
            }
        }
    }
    run_with_stdin_inner(cmd, label, stdin, false)
}

/// Call the library's exported `main()` `n` times inside the same process.
pub fn run_main_repeat(so: &Path, label: &str, kind: StdinKind<'_>, n: usize) -> Out {
    let mut cmd = Command::new(runner());
    cmd.arg("main-repeat").arg(so).arg(n.to_string());
    run_with_stdin(cmd, label, kind)
}

/// Compare C vs Rust for `n` successive `main()` calls in one process.
pub fn assert_main_repeat(label: &str, bytes: &[u8], n: usize) {
    let c = run_main_repeat(c_so(), &format!("{label}.c"), StdinKind::File(bytes), n);
    let r = run_main_repeat(rust_so(), &format!("{label}.rs"), StdinKind::File(bytes), n);
    assert_eq!(
        c.observable(),
        r.observable(),
        "[{label}] {n} successive main() calls diverge for stdin {:?}\n  C   : {c:?}\n  Rust: {r:?}",
        hex(bytes)
    );
    let c2 = run_main_repeat(c_so(), &format!("{label}.pc"), StdinKind::Pipe(bytes), n);
    let r2 = run_main_repeat(rust_so(), &format!("{label}.pr"), StdinKind::Pipe(bytes), n);
    assert_eq!(
        c2.observable(),
        r2.observable(),
        "[{label}] {n} successive main() calls diverge (pipe stdin) for {:?}\n  C   : {c2:?}\n  Rust: {r2:?}",
        hex(bytes)
    );
}

/// Call the library's exported `main()` with a hostile stdout.
pub fn run_main_stdout(
    so: &Path,
    label: &str,
    stdin: StdinKind<'_>,
    stdout: StdoutKind,
    reset_sigpipe: bool,
) -> Out {
    let mut cmd = Command::new(runner());
    cmd.arg("main").arg(so);
    spawn_with_stdout(cmd, label, stdin, stdout, reset_sigpipe)
}

/// Run a standalone executable with a hostile stdout.
pub fn run_exe_stdout(exe: &Path, label: &str, stdin: StdinKind<'_>, stdout: StdoutKind) -> Out {
    let cmd = Command::new(exe);
    spawn_with_stdout(cmd, label, stdin, stdout, false)
}

/// Run a standalone executable (not a `.so`) with the given stdin bytes.
pub fn run_exe(exe: &Path, label: &str, bytes: &[u8]) -> Out {
    run_exe_kind(exe, label, StdinKind::File(bytes))
}

/// Run a standalone executable with an arbitrary stdin kind.
pub fn run_exe_kind(exe: &Path, label: &str, kind: StdinKind<'_>) -> Out {
    let cmd = Command::new(exe);
    run_with_stdin(cmd, &format!("{label}.exe"), kind)
}

// ---------------------------------------------------------------------------
// comparison helpers
// ---------------------------------------------------------------------------

fn describe(cases: &[(Vec<u8>, Vec<u8>)], i: usize) -> String {
    match cases.get(i) {
        Some((a, b)) => format!("case #{i}: s1={:?} s2={:?}", hex(a), hex(b)),
        None => format!("case #{i}: <out of range>"),
    }
}

/// Compare C vs Rust for a batch of `driver` calls.
pub fn assert_driver_batch(label: &str, cases: &[(Vec<u8>, Vec<u8>)]) {
    let c = run_driver_batch(c_so(), &format!("{label}.c"), cases);
    let r = run_driver_batch(rust_so(), &format!("{label}.rs"), cases);

    if c.observable() != r.observable() {
        // find the first differing output line to make the failure readable
        let cl: Vec<&[u8]> = c.stdout.split(|&b| b == b'\n').collect();
        let rl: Vec<&[u8]> = r.stdout.split(|&b| b == b'\n').collect();
        let mut first = usize::MAX;
        for i in 0..cl.len().max(rl.len()) {
            if cl.get(i) != rl.get(i) {
                first = i;
                break;
            }
        }
        panic!(
            "[{label}] driver divergence at output line {first} ({})\n  C   : {:?}\n  Rust: {:?}\n  C lines={} Rust lines={}\n  C   full status: code={:?} signal={:?}\n  Rust full status: code={:?} signal={:?}\n  Rust stderr: {:?}",
            describe(cases, first),
            cl.get(first).map(|s| String::from_utf8_lossy(s).to_string()),
            rl.get(first).map(|s| String::from_utf8_lossy(s).to_string()),
            cl.len(),
            rl.len(),
            c.code,
            c.signal,
            r.code,
            r.signal,
            String::from_utf8_lossy(&r.stderr),
        );
    }
    assert!(
        r.stderr.is_empty(),
        "[{label}] Rust wrote to stderr (C never does): {:?}",
        String::from_utf8_lossy(&r.stderr)
    );
    assert!(
        c.stderr.is_empty(),
        "[{label}] C wrote to stderr: {:?}",
        String::from_utf8_lossy(&c.stderr)
    );
    // sanity: one line of output per case
    assert_eq!(
        c.stdout.iter().filter(|&&b| b == b'\n').count(),
        cases.len(),
        "[{label}] C produced an unexpected number of output lines"
    );
}

/// Compare C vs Rust for one whole-program (`main`) run.
pub fn assert_main(label: &str, kind_for_c: StdinKind<'_>, kind_for_rust: StdinKind<'_>) {
    let c = run_main(c_so(), &format!("{label}.c"), kind_for_c);
    let r = run_main(rust_so(), &format!("{label}.rs"), kind_for_rust);
    assert_eq!(
        c.observable(),
        r.observable(),
        "[{label}] main() divergence\n  C   : {c:?}\n  Rust: {r:?}"
    );
    assert!(
        c.stderr.is_empty(),
        "[{label}] C wrote to stderr: {:?}",
        String::from_utf8_lossy(&c.stderr)
    );
    assert!(
        r.stderr.is_empty(),
        "[{label}] Rust wrote to stderr (C never does): {:?}",
        String::from_utf8_lossy(&r.stderr)
    );
}

/// Convenience: file-backed stdin for both sides.
pub fn assert_main_bytes(label: &str, bytes: &[u8]) {
    assert_main(label, StdinKind::File(bytes), StdinKind::File(bytes));
}

/// Convenience: pipe-backed stdin for both sides.
pub fn assert_main_bytes_pipe(label: &str, bytes: &[u8]) {
    assert_main(label, StdinKind::Pipe(bytes), StdinKind::Pipe(bytes));
}

/// Compare the two standalone executables end to end.
pub fn assert_exe_bytes(label: &str, bytes: &[u8]) {
    let c = run_exe(c_exe(), &format!("{label}.c"), bytes);
    let r = run_exe(rust_exe(), &format!("{label}.rs"), bytes);
    assert_eq!(
        c.observable(),
        r.observable(),
        "[{label}] executable divergence for stdin {:?}\n  C   : {c:?}\n  Rust: {r:?}",
        hex(bytes)
    );
}

// ---------------------------------------------------------------------------
// deterministic PRNG (xorshift64*), fixed seeds -> reproducible corpora
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
    /// Random bytes drawn from `alphabet`.
    pub fn bytes_from(&mut self, len: usize, alphabet: &[u8]) -> Vec<u8> {
        (0..len).map(|_| alphabet[self.below(alphabet.len())]).collect()
    }
    /// Random bytes in `0x01..=0xff` (no NUL: that would terminate the string).
    pub fn bytes_nonzero(&mut self, len: usize) -> Vec<u8> {
        (0..len)
            .map(|_| {
                let mut b = self.byte();
                if b == 0 {
                    b = 1;
                }
                b
            })
            .collect()
    }
    /// Random bytes over the *full* range, NUL included.
    pub fn bytes_any(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.byte()).collect()
    }
}

pub const ASCII: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 !\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~";
