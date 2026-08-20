//! Shared plumbing for the C-vs-Rust differential tests.
//!
//! Both implementations are always reached through their **shared objects**,
//! loaded with `libloading`/`dlopen`; the Rust functions are never called
//! directly, so the `#[no_mangle] extern "C"` export wrappers are part of what
//! is under test.

#![allow(dead_code)]

use std::io::Write;
use std::os::raw::{c_int, c_void};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// Artifact discovery / building
// ---------------------------------------------------------------------------

pub fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `target/<profile>/`, derived from a known binary artifact path.
pub fn artifact_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_so_main_runner"))
        .parent()
        .expect("bin exe has a parent")
        .to_path_buf()
}

/// A scratch directory for artifacts this test suite builds itself.  The test
/// binary's own name is part of the path so that two test binaries running
/// concurrently never write the same file.
pub fn scratch_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let tag = exe
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let dir = artifact_dir().join("difftest").join(tag);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

pub fn runner_exe() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_so_main_runner"))
}

/// The `cdylib` produced from `src/lib.rs`.
///
/// `cargo test` does not build the library's `cdylib` crate type on its own
/// (the integration tests do not link against it), so build it on demand.  The
/// profile is inferred from the artifact directory name.
pub fn rust_so() -> PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let dir = artifact_dir();
        let so = dir.join("libdriver.so");
        let release = dir.file_name().map(|s| s == "release").unwrap_or(false);
        let mut cmd = Command::new(env!("CARGO"));
        cmd.arg("build").arg("--offline").arg("--lib");
        if release {
            cmd.arg("--release");
        }
        let out = cmd
            .current_dir(crate_root())
            .output()
            .expect("spawn cargo build --lib");
        assert!(
            out.status.success(),
            "cargo build --lib failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(so.is_file(), "expected the cdylib at {}", so.display());
        so
    })
    .clone()
}

/// The C shared object, compiled from the single translation unit named by
/// `c_src/CMakeLists.txt` (`add_executable(driver src/main.c)`).
pub fn c_so() -> PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let out = scratch_dir().join("libcdriver.so");
        let src = crate_root().join("c_src").join("src").join("main.c");
        let res = Command::new("gcc")
            .arg("-shared")
            .arg("-fPIC")
            .arg("-o")
            .arg(&out)
            .arg(&src)
            .output()
            .expect("spawn gcc");
        assert!(
            res.status.success(),
            "building the C shared object failed:\n{}",
            String::from_utf8_lossy(&res.stderr)
        );
        out
    })
    .clone()
}

/// Build the C shared object with extra compiler flags (e.g. `-O2`), so the
/// differential tests can confirm the observable contract does not depend on
/// gcc's optimization level.  `tag` names the artifact.
pub fn c_so_with_flags(tag: &str, flags: &[&str]) -> PathBuf {
    let out = scratch_dir().join(format!("libcdriver-{tag}.so"));
    let src = crate_root().join("c_src").join("src").join("main.c");
    let mut cmd = Command::new("gcc");
    cmd.arg("-shared").arg("-fPIC");
    for f in flags {
        cmd.arg(f);
    }
    let res = cmd
        .arg("-o")
        .arg(&out)
        .arg(&src)
        .output()
        .expect("spawn gcc");
    assert!(
        res.status.success(),
        "building the C shared object with {flags:?} failed:\n{}",
        String::from_utf8_lossy(&res.stderr)
    );
    out
}

/// The C executable.  Prefers the CMake build (`c_src/build/driver`, produced
/// by `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`) and
/// falls back to an equivalent plain `gcc` link, which is what CMake performs
/// for the project's default (empty) build type.
pub fn c_exe() -> PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let cmake_built = crate_root().join("c_src").join("build").join("driver");
        if cmake_built.is_file() {
            return cmake_built;
        }
        let out = scratch_dir().join("c_driver");
        let src = crate_root().join("c_src").join("src").join("main.c");
        let res = Command::new("gcc")
            .arg("-o")
            .arg(&out)
            .arg(&src)
            .output()
            .expect("spawn gcc");
        assert!(
            res.status.success(),
            "building the C executable failed:\n{}",
            String::from_utf8_lossy(&res.stderr)
        );
        out
    })
    .clone()
}

/// The Rust executable built from `src/main.rs`.
pub fn rust_exe() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

// ---------------------------------------------------------------------------
// Sub-process invocation
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunResult {
    pub stdout: Vec<u8>,
    pub code: Option<i32>,
}

fn unique_temp(name: &str) -> PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    scratch_dir().join(format!("{name}.{}.{n}", std::process::id()))
}

fn run_with_stdin(mut cmd: Command, stdin_bytes: &[u8]) -> RunResult {
    // A real file (rather than a pipe) removes any chance of a writer/reader
    // deadlock and gives both implementations a seekable, identical stream.
    let path = unique_temp("stdin");
    {
        let mut f = std::fs::File::create(&path).expect("create stdin file");
        f.write_all(stdin_bytes).expect("write stdin file");
        f.flush().expect("flush stdin file");
    }
    let f = std::fs::File::open(&path).expect("open stdin file");
    let out = cmd
        .stdin(Stdio::from(f))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn child");
    let _ = std::fs::remove_file(&path);
    RunResult {
        stdout: out.stdout,
        code: out.status.code(),
    }
}

/// `dlopen(so)` in a fresh process and call its exported `main`, feeding it
/// `stdin_bytes` on standard input.
pub fn so_main(so: &Path, stdin_bytes: &[u8]) -> RunResult {
    let mut cmd = Command::new(runner_exe());
    cmd.arg(so).arg("main");
    run_with_stdin(cmd, stdin_bytes)
}

/// `dlopen(so)` in a fresh process and call the named zero-argument export.
pub fn so_void(so: &Path, symbol: &str) -> RunResult {
    let mut cmd = Command::new(runner_exe());
    cmd.arg(so).arg(symbol);
    run_with_stdin(cmd, b"")
}

/// `dlopen(so)` in a fresh process and call `printLine` with `NULL`.
pub fn so_print_line_null(so: &Path) -> RunResult {
    let mut cmd = Command::new(runner_exe());
    cmd.arg(so).arg("printLine:@null");
    run_with_stdin(cmd, b"")
}

/// `dlopen(so)` in a fresh process and call `printLine` with `bytes` as a
/// NUL-terminated string.
pub fn so_print_line(so: &Path, bytes: &[u8]) -> RunResult {
    let path = unique_temp("arg");
    std::fs::write(&path, bytes).expect("write arg file");
    let mut cmd = Command::new(runner_exe());
    cmd.arg(so).arg(format!("printLine:@file:{}", path.display()));
    let r = run_with_stdin(cmd, b"");
    let _ = std::fs::remove_file(&path);
    r
}

/// Run a stand-alone executable with the given standard input.
pub fn exe_run(exe: &Path, stdin_bytes: &[u8]) -> RunResult {
    run_with_stdin(Command::new(exe), stdin_bytes)
}

// ---------------------------------------------------------------------------
// In-process capture of file descriptor 1
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

/// Serializes the file-descriptor juggling, which is process-global state.
fn fd_lock() -> &'static Mutex<()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
}

/// Run `f` with file descriptor 1 pointed at a temporary file and return the
/// bytes it produced.
///
/// Both implementations are flushed afterwards: `fflush(NULL)` covers glibc's
/// `stdout` (shared with the `dlopen`ed C object) and the Rust object's export
/// wrappers flush their own `std::io::stdout()` before returning.
pub fn capture_fd1<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = fd_lock().lock().unwrap_or_else(|e| e.into_inner());

    let path = unique_temp("fd1");
    let file = std::fs::File::create(&path).expect("create capture file");
    let target_fd = {
        use std::os::unix::io::AsRawFd;
        file.as_raw_fd()
    };

    unsafe {
        // Do not let anything already buffered leak into the capture.
        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();
        fflush(std::ptr::null_mut());

        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(target_fd, 1) >= 0, "dup2 onto fd 1 failed");

        // Restore fd 1 even if `f` unwinds, otherwise every later diagnostic
        // would silently disappear into the capture file.
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

        let _ = std::io::stdout().flush();
        fflush(std::ptr::null_mut());

        assert!(dup2(saved, 1) >= 0, "restoring fd 1 failed");
        close(saved);

        if let Err(payload) = outcome {
            std::panic::resume_unwind(payload);
        }
    }

    drop(file);
    let bytes = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);
    bytes
}

// ---------------------------------------------------------------------------
// Minimal sequential test harness
// ---------------------------------------------------------------------------

/// Run `cases` strictly one at a time, reporting on **stderr**.
///
/// The `capture_fd1` tests cannot use libtest's default harness: libtest runs
/// tests on several threads and writes its own progress lines to file
/// descriptor 1, which would land inside a capture belonging to another thread.
/// This harness is single-threaded and keeps fd 1 untouched.
pub fn run_cases(cases: &[(&str, fn())]) {
    let filters: Vec<String> = std::env::args()
        .skip(1)
        .filter(|a| !a.starts_with('-'))
        .collect();

    let selected: Vec<&(&str, fn())> = cases
        .iter()
        .filter(|(name, _)| filters.is_empty() || filters.iter().any(|f| name.contains(f.as_str())))
        .collect();

    eprintln!("\nrunning {} tests (sequential harness)", selected.len());
    let mut failures: Vec<String> = Vec::new();
    for (name, f) in &selected {
        eprint!("test {name} ... ");
        let outcome = std::panic::catch_unwind(*f);
        match outcome {
            Ok(()) => eprintln!("ok"),
            Err(_) => {
                eprintln!("FAILED");
                failures.push((*name).to_string());
            }
        }
    }
    if failures.is_empty() {
        eprintln!(
            "\ntest result: ok. {} passed; 0 failed; {} filtered out\n",
            selected.len(),
            cases.len() - selected.len()
        );
    } else {
        eprintln!("\nfailures:");
        for f in &failures {
            eprintln!("    {f}");
        }
        eprintln!(
            "\ntest result: FAILED. {} passed; {} failed\n",
            selected.len() - failures.len(),
            failures.len()
        );
        std::process::exit(101);
    }
}

// ---------------------------------------------------------------------------
// Deterministic pseudo-random generator (SplitMix64)
// ---------------------------------------------------------------------------

/// Fixed seed so every run of the suite exercises the same corpus.
pub const SEED: u64 = 0x5EED_1234_ABCD_EF01;

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

    /// Uniform in `0..n` (`n > 0`).
    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0);
        self.next_u64() % n
    }

    /// Uniform in `lo..=hi`.
    pub fn range(&mut self, lo: u64, hi: u64) -> u64 {
        assert!(hi >= lo);
        lo + self.below(hi - lo + 1)
    }

    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }

    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len() as u64) as usize]
    }

    pub fn bytes(&mut self, len: usize, alphabet: &[u8]) -> Vec<u8> {
        (0..len).map(|_| *self.pick(alphabet)).collect()
    }
}

// ---------------------------------------------------------------------------
// Assertions
// ---------------------------------------------------------------------------

pub fn show(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes.iter().take(96) {
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
    if bytes.len() > 96 {
        s.push_str(&format!("...(+{} bytes)", bytes.len() - 96));
    }
    s
}

#[track_caller]
pub fn assert_same(label: &str, input: &[u8], c: &RunResult, r: &RunResult) {
    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch for {label}\n  input : \"{}\"\n  C     : \"{}\"\n  Rust  : \"{}\"",
        show(input),
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.code,
        r.code,
        "exit-code mismatch for {label}\n  input : \"{}\"\n  C     : {:?}\n  Rust  : {:?}",
        show(input),
        c.code,
        r.code
    );
}

#[track_caller]
pub fn assert_same_bytes(label: &str, input: &[u8], c: &[u8], r: &[u8]) {
    assert_eq!(
        c,
        r,
        "output mismatch for {label}\n  input : \"{}\"\n  C     : \"{}\"\n  Rust  : \"{}\"",
        show(input),
        show(c),
        show(r)
    );
}
