//! Shared harness for the differential tests.
//!
//! Both programs are driven as *subprocesses*, exactly the way a shell would
//! run them.  Nothing here loads the Rust crate as a library.

#![allow(dead_code)]

use std::ffi::OsStr;
use std::fs::File;
use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// Repository root: the directory holding both `c_src/` and `translation/`.
pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// The Rust program under test. `CARGO_BIN_EXE_driver` is set by cargo for
/// integration tests and points at the freshly built `driver` executable.
pub fn rust_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// The C program, built out-of-source with CMake exactly as documented.
pub fn c_binary() -> PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let c_src = workspace_root().join("c_src");
        let build = c_src.join("build");
        std::fs::create_dir_all(&build).expect("create c_src/build");

        let cfg = Command::new("cmake")
            .arg("..")
            .current_dir(&build)
            .output()
            .expect("run `cmake ..` (is cmake installed?)");
        assert!(
            cfg.status.success(),
            "cmake configure failed:\n{}\n{}",
            String::from_utf8_lossy(&cfg.stdout),
            String::from_utf8_lossy(&cfg.stderr)
        );

        let bld = Command::new("cmake")
            .args(["--build", "."])
            .current_dir(&build)
            .output()
            .expect("run `cmake --build .`");
        assert!(
            bld.status.success(),
            "cmake build failed:\n{}\n{}",
            String::from_utf8_lossy(&bld.stdout),
            String::from_utf8_lossy(&bld.stderr)
        );

        let bin = build.join("driver");
        assert!(bin.is_file(), "C binary missing at {}", bin.display());
        bin
    })
    .clone()
}

/// Everything observable about one run of a program.
#[derive(PartialEq, Eq)]
pub struct RunResult {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    /// Raw `wait(2)` status, so an exit code and a fatal signal are told apart.
    pub raw_status: i32,
}

impl RunResult {
    pub fn describe_status(&self) -> String {
        let st = std::process::ExitStatus::from_raw(self.raw_status);
        match (st.code(), st.signal()) {
            (Some(c), _) => format!("exited with code {c}"),
            (None, Some(s)) => format!("killed by signal {s}"),
            _ => format!("raw wait status {}", self.raw_status),
        }
    }
}

/// How a run should be set up.
pub struct Spec<'a> {
    pub args: Vec<&'a str>,
    pub stdin: StdinMode<'a>,
    /// Extra/overriding environment variables.
    pub env: Vec<(&'a str, &'a str)>,
    /// Start from a completely empty environment.
    pub clear_env: bool,
    pub cwd: Option<PathBuf>,
}

pub enum StdinMode<'a> {
    /// `< /dev/null`
    Empty,
    /// stdin closed outright (the child inherits nothing readable).
    Closed,
    /// Bytes fed on stdin. The C program never reads stdin, so these must be
    /// ignored; feeding them proves nothing is consumed.
    Bytes(&'a [u8]),
}

impl<'a> Default for Spec<'a> {
    fn default() -> Self {
        Spec {
            args: Vec::new(),
            stdin: StdinMode::Empty,
            env: Vec::new(),
            clear_env: false,
            cwd: None,
        }
    }
}

impl<'a> Spec<'a> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn args(mut self, a: &[&'a str]) -> Self {
        self.args = a.to_vec();
        self
    }
    pub fn stdin(mut self, s: StdinMode<'a>) -> Self {
        self.stdin = s;
        self
    }
    pub fn env(mut self, k: &'a str, v: &'a str) -> Self {
        self.env.push((k, v));
        self
    }
    pub fn clear_env(mut self) -> Self {
        self.clear_env = true;
        self
    }
    pub fn cwd<P: Into<PathBuf>>(mut self, p: P) -> Self {
        self.cwd = Some(p.into());
        self
    }
}

fn base_command(bin: &Path, spec: &Spec) -> Command {
    let mut cmd = Command::new(bin);
    for a in &spec.args {
        cmd.arg(OsStr::new(a));
    }
    if spec.clear_env {
        cmd.env_clear();
    }
    for (k, v) in &spec.env {
        cmd.env(k, v);
    }
    if let Some(dir) = &spec.cwd {
        cmd.current_dir(dir);
    }
    cmd
}

/// Run `bin` with stdout and stderr captured on separate pipes.
pub fn run(bin: &Path, spec: &Spec) -> RunResult {
    let mut cmd = base_command(bin, spec);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    match spec.stdin {
        StdinMode::Empty | StdinMode::Bytes(_) => {
            cmd.stdin(Stdio::piped());
        }
        StdinMode::Closed => {
            cmd.stdin(Stdio::null());
        }
    }

    let mut child = cmd.spawn().unwrap_or_else(|e| panic!("spawn {bin:?}: {e}"));

    if let Some(mut sink) = child.stdin.take() {
        let payload: Vec<u8> = match spec.stdin {
            StdinMode::Bytes(b) => b.to_vec(),
            _ => Vec::new(),
        };
        // The child never reads stdin, so a large payload would fill the pipe
        // buffer and block. Feed it from a helper thread and ignore EPIPE.
        std::thread::spawn(move || {
            let _ = sink.write_all(&payload);
            let _ = sink.flush();
            drop(sink);
        });
    }

    let out = child.wait_with_output().expect("wait_with_output");
    RunResult {
        stdout: out.stdout,
        stderr: out.stderr,
        raw_status: out.status.into_raw(),
    }
}

/// Run with stdout and stderr merged onto a single pipe, which exposes the
/// interleaving produced by C's stdio buffering rules.
pub fn run_merged(bin: &Path, spec: &Spec) -> RunResult {
    // `sh -c 'exec prog 2>&1'` is the most faithful way to put both streams on
    // one pipe, i.e. literally what `prog 2>&1 | ...` does in a shell.
    let mut sh = Command::new("sh");
    sh.arg("-c").arg(format!(
        "exec {} 2>&1 </dev/null",
        shell_quote(bin.to_str().unwrap())
    ));
    if spec.clear_env {
        sh.env_clear();
    }
    for (k, v) in &spec.env {
        sh.env(k, v);
    }
    if let Some(dir) = &spec.cwd {
        sh.current_dir(dir);
    }
    let out = sh
        .stdin(Stdio::null())
        .output()
        .expect("run merged via sh -c");
    RunResult {
        stdout: out.stdout,
        stderr: Vec::new(),
        raw_status: out.status.into_raw(),
    }
}

fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

/// Run with stdout redirected to `path` (opened for writing) and stderr piped.
pub fn run_stdout_to_file(bin: &Path, path: &str) -> RunResult {
    let f = File::options()
        .write(true)
        .open(path)
        .unwrap_or_else(|e| panic!("open {path} for writing: {e}"));
    let out = Command::new(bin)
        .stdin(Stdio::null())
        .stdout(Stdio::from(f))
        .stderr(Stdio::piped())
        .output()
        .expect("spawn with redirected stdout");
    RunResult {
        stdout: Vec::new(),
        stderr: out.stderr,
        raw_status: out.status.into_raw(),
    }
}

/// Run with stdout pointed at a *read-only* descriptor, so every write fails
/// with EBADF. Both programs ignore printf's return value, so this checks that
/// neither of them turns a write error into a different exit status.
pub fn run_stdout_unwritable(bin: &Path) -> RunResult {
    let f = File::open("/dev/null").expect("open /dev/null read-only");
    let out = Command::new(bin)
        .stdin(Stdio::null())
        .stdout(Stdio::from(f))
        .stderr(Stdio::piped())
        .output()
        .expect("spawn with unwritable stdout");
    RunResult {
        stdout: Vec::new(),
        stderr: out.stderr,
        raw_status: out.status.into_raw(),
    }
}

/// Which stream gets a closed reader in [`run_with_closed_reader`].
pub enum Stream {
    Stdout,
    Stderr,
}

/// Spawn the program with a pipe on `stream`, then immediately close the read
/// end. The program's first write to that stream then fails with EPIPE, which
/// for a C program means death by SIGPIPE.
pub fn run_with_closed_reader(bin: &Path, stream: Stream) -> RunResult {
    let mut cmd = Command::new(bin);
    cmd.stdin(Stdio::null());
    match stream {
        Stream::Stdout => {
            cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        }
        Stream::Stderr => {
            cmd.stdout(Stdio::null()).stderr(Stdio::piped());
        }
    }
    let mut child = cmd.spawn().expect("spawn with closed reader");

    // Drop the read end(s) of the stream we are killing.
    match stream {
        Stream::Stdout => {
            drop(child.stdout.take());
        }
        Stream::Stderr => {
            drop(child.stderr.take());
        }
    }
    // Drain whatever is left so the child never blocks on the other stream.
    let mut leftover_out = Vec::new();
    let mut leftover_err = Vec::new();
    if let Some(mut s) = child.stdout.take() {
        use std::io::Read;
        let _ = s.read_to_end(&mut leftover_out);
    }
    if let Some(mut s) = child.stderr.take() {
        use std::io::Read;
        let _ = s.read_to_end(&mut leftover_err);
    }
    let status = child.wait().expect("wait");
    RunResult {
        stdout: leftover_out,
        stderr: leftover_err,
        raw_status: status.into_raw(),
    }
}

/// Run the program with stdout attached to a pseudo-terminal, which makes C's
/// stdout *line* buffered instead of fully buffered.
pub fn run_on_pty(bin: &Path) -> Option<RunResult> {
    if Command::new("sh")
        .args(["-c", "command -v script >/dev/null 2>&1"])
        .status()
        .map(|s| !s.success())
        .unwrap_or(true)
    {
        return None;
    }
    let out = Command::new("script")
        .arg("-qec")
        .arg(bin.to_str().unwrap())
        .arg("/dev/null")
        .stdin(Stdio::null())
        .output()
        .expect("run under script(1)");
    Some(RunResult {
        stdout: out.stdout,
        stderr: out.stderr,
        raw_status: out.status.into_raw(),
    })
}

fn render(label: &str, bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => format!("{label} ({} bytes):\n{s}", bytes.len()),
        Err(_) => format!("{label} ({} bytes, not UTF-8): {bytes:?}", bytes.len()),
    }
}

/// First differing byte offset, if any.
fn first_diff(a: &[u8], b: &[u8]) -> Option<usize> {
    let n = a.len().min(b.len());
    for i in 0..n {
        if a[i] != b[i] {
            return Some(i);
        }
    }
    if a.len() != b.len() {
        Some(n)
    } else {
        None
    }
}

/// Assert stdout, stderr and exit status all match, byte for byte.
pub fn assert_identical(case: &str, c: &RunResult, r: &RunResult) {
    if let Some(off) = first_diff(&c.stdout, &r.stdout) {
        panic!(
            "[{case}] stdout differs at byte {off}\n{}\n{}",
            render("C stdout", &c.stdout),
            render("Rust stdout", &r.stdout)
        );
    }
    if let Some(off) = first_diff(&c.stderr, &r.stderr) {
        panic!(
            "[{case}] stderr differs at byte {off}\n{}\n{}",
            render("C stderr", &c.stderr),
            render("Rust stderr", &r.stderr)
        );
    }
    assert_eq!(
        c.raw_status,
        r.raw_status,
        "[{case}] exit status differs: C {} vs Rust {}",
        c.describe_status(),
        r.describe_status()
    );
}

/// Convenience: run both programs with the same spec and compare.
pub fn compare(case: &str, spec: &Spec) {
    let c = run(&c_binary(), spec);
    let r = run(&rust_binary(), spec);
    assert_identical(case, &c, &r);
}
