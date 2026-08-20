//! Shared plumbing for the C-vs-Rust differential tests.
//!
//! Nothing in `c_src/` is modified: the C executable produced by CMake is used
//! when present, and the C shared object is compiled out of tree into
//! `target/c_build/`.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Paths / artifacts
// ---------------------------------------------------------------------------

pub fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Directory the current profile's artifacts live in (`target/debug` or
/// `target/release`), derived from the path cargo hands us for the binary.
pub fn artifact_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
        .parent()
        .expect("bin path has a parent")
        .to_path_buf()
}

/// The Rust build of the program (this is what `main` parity is measured on).
pub fn rust_exe() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn c_build_dir() -> PathBuf {
    let dir = crate_root().join("target").join("c_build");
    std::fs::create_dir_all(&dir).expect("create target/c_build");
    dir
}

fn c_source() -> PathBuf {
    crate_root().join("c_src").join("src").join("main.c")
}

fn newer_than_source(out: &Path) -> bool {
    let src = match std::fs::metadata(c_source()).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return false,
    };
    match std::fs::metadata(out).and_then(|m| m.modified()) {
        Ok(t) => t >= src,
        Err(_) => false,
    }
}

/// Compile `main.c` with `gcc`, writing to a unique temp path first and then
/// renaming, so concurrently running test binaries cannot corrupt each other.
fn compile_c(out: &Path, extra: &[&str]) {
    if out.exists() && newer_than_source(out) {
        return;
    }
    let tmp = out.with_extension(format!("tmp{}", std::process::id()));
    let mut cmd = Command::new("gcc");
    cmd.args(extra)
        .arg("-o")
        .arg(&tmp)
        .arg(c_source())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let output = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn gcc: {e}"));
    assert!(
        output.status.success(),
        "gcc failed to build {}: {}",
        out.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    // Rename is atomic within the same filesystem.
    std::fs::rename(&tmp, out).unwrap_or_else(|e| {
        let _ = std::fs::remove_file(&tmp);
        if !out.exists() {
            panic!("failed to install {}: {e}", out.display());
        }
    });
}

/// The C executable. Prefers the artifact CMake produced (`c_src/build/driver`,
/// i.e. CMake's default flags), falling back to an equivalent plain `gcc`
/// build so the suite never fails merely because CMake was not run.
pub fn c_exe() -> PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let cmake_built = crate_root().join("c_src").join("build").join("driver");
        if cmake_built.exists() && newer_than_source(&cmake_built) {
            return cmake_built;
        }
        let out = c_build_dir().join("driver_c");
        compile_c(&out, &[]);
        out
    })
    .clone()
}

/// The C shared object, for `dlopen`-level comparison of `printLine`.
pub fn c_so() -> PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let out = c_build_dir().join("libdriver_c.so");
        compile_c(&out, &["-shared", "-fPIC"]);
        out
    })
    .clone()
}

/// Newest modification time across the Rust sources.
fn newest_rust_source() -> std::time::SystemTime {
    let mut newest = std::time::SystemTime::UNIX_EPOCH;
    let dir = crate_root().join("src");
    let entries = std::fs::read_dir(&dir).expect("read src/");
    for e in entries.flatten() {
        if e.path().extension().and_then(|s| s.to_str()) == Some("rs") {
            if let Ok(t) = e.metadata().and_then(|m| m.modified()) {
                if t > newest {
                    newest = t;
                }
            }
        }
    }
    newest
}

/// `cargo test --test <name>` only needs the `rlib`, so it may leave a **stale**
/// `cdylib` (or example) on disk from an earlier build. Comparing against a
/// stale artifact would make the FFI tests pass vacuously, so refuse to run.
fn assert_fresh(p: &Path, how_to_build: &str) {
    assert!(
        p.exists(),
        "missing {} — build it first ({how_to_build})",
        p.display()
    );
    let built = std::fs::metadata(p)
        .and_then(|m| m.modified())
        .expect("artifact mtime");
    assert!(
        built >= newest_rust_source(),
        "{} is STALE (older than src/*.rs) — rebuild with `{how_to_build}`; \
         comparing against a stale artifact would silently pass",
        p.display()
    );
}

/// The Rust shared object (`cdylib`), the counterpart of [`c_so`].
pub fn rust_so() -> PathBuf {
    let p = artifact_dir().join("libdriver.so");
    assert_fresh(&p, "cargo build --lib");
    p
}

/// The `dlopen` probe helper (an example target, so it may use dev-deps).
pub fn probe_exe() -> PathBuf {
    let p = artifact_dir().join("examples").join("ffi_probe");
    assert_fresh(&p, "cargo build --examples");
    p
}

pub fn have_program(name: &str) -> bool {
    Command::new(name)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Running the programs
// ---------------------------------------------------------------------------

/// How the child's stdin is set up — each variant is a distinct configuration
/// axis the C `fgets` call reacts to.
#[derive(Clone, Debug)]
pub enum In {
    /// A pipe carrying these bytes, then EOF.
    Pipe(Vec<u8>),
    /// A regular file holding these bytes.
    File(Vec<u8>),
    /// `/dev/null` — immediate EOF, `fgets` returns NULL.
    DevNull,
    /// File descriptor 0 closed outright — `read` fails with `EBADF`.
    ClosedFd,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Outcome {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub code: Option<i32>,
    pub signal: Option<i32>,
}

impl Outcome {
    pub fn describe(&self) -> String {
        format!(
            "stdout={:?} stderr={:?} code={:?} signal={:?}",
            String::from_utf8_lossy(&summarize(&self.stdout)),
            String::from_utf8_lossy(&summarize(&self.stderr)),
            self.code,
            self.signal
        )
    }
}

fn summarize(bytes: &[u8]) -> Vec<u8> {
    if bytes.len() <= 120 {
        return bytes.to_vec();
    }
    let mut v = bytes[..60].to_vec();
    v.extend_from_slice(format!("...<{} bytes total>...", bytes.len()).as_bytes());
    v.extend_from_slice(&bytes[bytes.len() - 20..]);
    v
}

fn temp_stdin_file(bytes: &[u8]) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let dir = c_build_dir();
    let path = dir.join(format!(
        "stdin_{}_{}.bin",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(&path, bytes).expect("write temp stdin file");
    path
}

/// Run `exe` with the given stdin configuration and capture everything
/// observable: stdout bytes, stderr bytes, exit code and terminating signal.
pub fn run(exe: &Path, input: &In) -> Outcome {
    use std::io::Write;
    use std::os::unix::process::ExitStatusExt;

    let mut cmd = Command::new(exe);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut tmp_file = None;
    match input {
        In::Pipe(_) => {
            cmd.stdin(Stdio::piped());
        }
        In::DevNull => {
            cmd.stdin(Stdio::null());
        }
        In::File(bytes) => {
            let path = temp_stdin_file(bytes);
            let f = std::fs::File::open(&path).expect("open temp stdin file");
            tmp_file = Some(path);
            cmd.stdin(Stdio::from(f));
        }
        In::ClosedFd => {
            // Close descriptor 0 in the child so `read` fails with EBADF.
            cmd.stdin(Stdio::null());
            unsafe {
                use std::os::unix::process::CommandExt;
                cmd.pre_exec(|| {
                    extern "C" {
                        fn close(fd: i32) -> i32;
                    }
                    close(0);
                    Ok(())
                });
            }
        }
    }

    let mut child = cmd
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));

    if let In::Pipe(bytes) = input {
        let mut stdin = child.stdin.take().expect("piped stdin");
        // The program reads at most 13 bytes and then exits, so the write can
        // legitimately fail with EPIPE; that is not a test failure.
        let _ = stdin.write_all(bytes);
        drop(stdin);
    }

    let out = child.wait_with_output().expect("wait for child");
    if let Some(path) = tmp_file {
        let _ = std::fs::remove_file(path);
    }

    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

/// Run `exe` with **stdout attached to a pty**, which switches C's `stdout`
/// from fully buffered to line buffered. `stdin_path` is redirected inside the
/// shell command so the pty does not echo input back into the captured output.
pub fn run_tty(exe: &Path, stdin_path: &str) -> Outcome {
    use std::os::unix::process::ExitStatusExt;

    let script_cmd = format!("{} < {}", exe.display(), stdin_path);
    let out = Command::new("script")
        .arg("-qec")
        .arg(&script_cmd)
        .arg("/dev/null")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn script(1)");

    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

// ---------------------------------------------------------------------------
// Differential assertions
// ---------------------------------------------------------------------------

pub struct Diff {
    label: String,
    failures: Vec<String>,
    cases: usize,
    crashes: usize,
}

impl Diff {
    pub fn new(label: &str) -> Self {
        Diff {
            label: label.to_string(),
            failures: Vec::new(),
            cases: 0,
            crashes: 0,
        }
    }

    /// Compare one configuration end to end. Collects (rather than panics on)
    /// mismatches so a run reports every divergence at once.
    pub fn check(&mut self, case: &str, c: &Outcome, r: &Outcome) {
        self.cases += 1;
        if c.signal.is_some() {
            self.crashes += 1;
        }
        if c != r {
            self.failures.push(format!(
                "  case {case}\n    C   : {}\n    Rust: {}",
                c.describe(),
                r.describe()
            ));
        }
    }

    pub fn check_run(&mut self, case: &str, input: &In) {
        let c = run(&c_exe(), input);
        let r = run(&rust_exe(), input);
        self.check(case, &c, &r);
    }

    /// Feed `line` on a pipe (the ordinary way this program is driven).
    pub fn check_line(&mut self, line: &[u8]) {
        let case = format!("stdin={:?}", String::from_utf8_lossy(line));
        self.check_run(&case, &In::Pipe(line.to_vec()));
    }

    pub fn finish(self) {
        let Diff {
            label,
            failures,
            cases,
            crashes,
        } = self;
        assert!(cases > 0, "{label}: no cases ran");
        if !failures.is_empty() {
            panic!(
                "{label}: {} of {cases} cases diverged ({crashes} of them crash-path):\n{}",
                failures.len(),
                failures.join("\n")
            );
        }
        eprintln!("{label}: {cases} cases matched ({crashes} on the fault path)");
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed, so failures reproduce exactly)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new() -> Self {
        Rng(0x5eed_1234)
    }

    pub fn with_seed(seed: u64) -> Self {
        Rng(seed | 1)
    }

    pub fn next_u64(&mut self) -> u64 {
        // xorshift64*
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
    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0);
        self.next_u64() % n
    }

    /// Uniform in `lo..=hi`.
    pub fn range_i64(&mut self, lo: i64, hi: i64) -> i64 {
        assert!(hi >= lo);
        let span = (hi - lo) as u64 + 1;
        lo + (self.below(span) as i64)
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len() as u64) as usize]
    }

    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
}
