//! Shared helpers for the differential tests.
//!
//! Everything here talks to the C reference and to the Rust translation through
//! the same external interfaces: the compiled executables (stdin -> stdout) and
//! the two shared objects loaded with `libloading`. The Rust code is never
//! called directly as a Rust function.

#![allow(dead_code)]

use std::io::{Read, Write};
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

extern "C" {
    fn close(fd: i32) -> i32;
    fn pipe(fds: *mut i32) -> i32;
}

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Directory holding the test binary, i.e. `target/{debug,release}`.
pub fn artifact_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
        .parent()
        .unwrap()
        .to_path_buf()
}

pub fn rust_exe() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

pub fn rust_so() -> PathBuf {
    artifact_dir().join("libdriver.so")
}

pub fn c_exe() -> PathBuf {
    manifest_dir().join("c_src/build/driver")
}

pub fn c_so() -> PathBuf {
    manifest_dir().join("build_c/libcdriver.so")
}

/// Builds the C executable (via CMake) and the C shared library (via gcc) if
/// they are not present yet.
pub fn ensure_c_artifacts() {
    let root = manifest_dir();
    let exe = c_exe();
    if !exe.exists() {
        let build = root.join("c_src/build");
        std::fs::create_dir_all(&build).expect("create c_src/build");
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
    assert!(exe.exists(), "missing C executable {}", exe.display());

    let so = c_so();
    if !so.exists() {
        std::fs::create_dir_all(so.parent().unwrap()).expect("create build_c");
        let st = Command::new("gcc")
            .current_dir(&root)
            .args(["-shared", "-fPIC", "-O2", "-o"])
            .arg(&so)
            .arg(root.join("c_src/src/main.c"))
            .status()
            .expect("run gcc");
        assert!(st.success(), "gcc -shared failed");
    }
    assert!(so.exists(), "missing C shared library {}", so.display());

    assert!(
        rust_exe().exists(),
        "missing Rust executable {}",
        rust_exe().display()
    );
    ensure_rust_so();
}

/// `cargo test` builds the bin and the integration tests but not the `cdylib`
/// artifact, so build it straight with `rustc` when it is missing. Same source
/// (`src/lib.rs` -> `src/imp.rs`), same edition, optimisation level matching the
/// profile the tests are running in.
fn ensure_rust_so() {
    let so = rust_so();
    if so.exists() {
        return;
    }
    let root = manifest_dir();
    let release = artifact_dir()
        .file_name()
        .map(|n| n == "release")
        .unwrap_or(false);
    let mut cmd = Command::new(std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string()));
    cmd.current_dir(&root)
        .args(["--edition", "2021"])
        .args(["--crate-type", "cdylib"])
        .args(["--crate-name", "driver"]);
    if release {
        cmd.arg("-O");
    }
    let st = cmd
        .arg("-o")
        .arg(&so)
        .arg(root.join("src/lib.rs"))
        .status()
        .expect("run rustc to build the cdylib");
    assert!(st.success(), "rustc --crate-type cdylib failed");
    assert!(
        so.exists(),
        "missing Rust shared library {} (run `cargo build` first)",
        so.display()
    );
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
            "code={:?} signal={:?}\nstdout({} bytes)={:?}\nstderr({} bytes)={:?}",
            self.code,
            self.signal,
            self.stdout.len(),
            String::from_utf8_lossy(&self.stdout),
            self.stderr.len(),
            String::from_utf8_lossy(&self.stderr)
        )
    }
}

/// Runs `prog` with `input` on stdin and `args` on the command line.
pub fn run_prog(prog: &Path, input: &[u8], args: &[&str]) -> Outcome {
    let mut child = Command::new(prog)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", prog.display()));

    let mut stdin = child.stdin.take().unwrap();
    let owned: Vec<u8> = input.to_vec();
    // A separate thread: the program may exit before draining large inputs, in
    // which case the write fails with EPIPE, which we ignore (as a shell would
    // report SIGPIPE for the writer, not for the program under test).
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&owned);
        let _ = stdin.flush();
    });

    let mut out = Vec::new();
    let mut err = Vec::new();
    child.stdout.take().unwrap().read_to_end(&mut out).unwrap();
    child.stderr.take().unwrap().read_to_end(&mut err).unwrap();
    let status = child.wait().expect("wait");
    let _ = writer.join();

    Outcome {
        stdout: out,
        stderr: err,
        code: status.code(),
        signal: status.signal(),
    }
}

/// Runs `prog` with stdin taken from `stdin` and stdout sent to `stdout`
/// (both already-open `Stdio` handles), returning the outcome. `stdout_capture`
/// is read back after the child exits when the caller redirected stdout to a
/// file.
pub fn run_prog_with(prog: &Path, stdin: Stdio, stdout: Stdio, args: &[&str]) -> Outcome {
    let output = Command::new(prog)
        .args(args)
        .stdin(stdin)
        .stdout(stdout)
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", prog.display()));
    let out = output.wait_with_output().expect("wait_with_output");
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

/// Runs `prog` with the given file descriptor closed before `exec`.
pub fn run_prog_with_closed_fd(prog: &Path, fd: i32, input: &[u8]) -> Outcome {
    let mut cmd = Command::new(prog);
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    unsafe {
        cmd.pre_exec(move || {
            close(fd);
            Ok(())
        });
    }
    let mut child = cmd
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", prog.display()));

    if let Some(mut stdin) = child.stdin.take() {
        let owned = input.to_vec();
        std::thread::spawn(move || {
            let _ = stdin.write_all(&owned);
        });
    }
    let mut out = Vec::new();
    let mut err = Vec::new();
    if let Some(mut s) = child.stdout.take() {
        let _ = s.read_to_end(&mut out);
    }
    if let Some(mut s) = child.stderr.take() {
        let _ = s.read_to_end(&mut err);
    }
    let status = child.wait().expect("wait");
    Outcome {
        stdout: out,
        stderr: err,
        code: status.code(),
        signal: status.signal(),
    }
}

/// Runs `prog` with stdout connected to the write end of a pipe whose read end
/// is already closed, so the first successful write raises `SIGPIPE`.
pub fn run_prog_broken_stdout(prog: &Path, input: &[u8]) -> Outcome {
    let mut fds = [0i32; 2];
    assert_eq!(unsafe { pipe(fds.as_mut_ptr()) }, 0, "pipe()");
    let (rfd, wfd) = (fds[0], fds[1]);
    // No reader will ever exist.
    unsafe { close(rfd) };

    let stdout = unsafe {
        use std::os::fd::FromRawFd;
        Stdio::from_raw_fd(wfd)
    };

    let mut child = Command::new(prog)
        .stdin(Stdio::piped())
        .stdout(stdout)
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", prog.display()));

    if let Some(mut stdin) = child.stdin.take() {
        let owned = input.to_vec();
        std::thread::spawn(move || {
            let _ = stdin.write_all(&owned);
        });
    }
    let mut err = Vec::new();
    if let Some(mut s) = child.stderr.take() {
        let _ = s.read_to_end(&mut err);
    }
    let status = child.wait().expect("wait");
    Outcome {
        stdout: Vec::new(),
        stderr: err,
        code: status.code(),
        signal: status.signal(),
    }
}

/// Asserts the C and Rust executables behaved identically.
pub fn assert_same(label: &str, input: &[u8], c: &Outcome, r: &Outcome) {
    if c != r {
        panic!(
            "DIVERGENCE [{label}]\ninput({} bytes) = {:?}\n--- C ---\n{}\n--- Rust ---\n{}",
            input.len(),
            String::from_utf8_lossy(&input[..input.len().min(200)]),
            c.describe(),
            r.describe()
        );
    }
}

/// Convenience: run both executables on the same input and compare.
pub fn diff_input(label: &str, input: &[u8]) {
    let c = run_prog(&c_exe(), input, &[]);
    let r = run_prog(&rust_exe(), input, &[]);
    assert_same(label, input, &c, &r);
}

pub fn diff_input_args(label: &str, input: &[u8], args: &[&str]) {
    let c = run_prog(&c_exe(), input, args);
    let r = run_prog(&rust_exe(), input, args);
    assert_same(label, input, &c, &r);
}

/// Deterministic SplitMix64 PRNG so every randomized case is reproducible.
pub struct Rng(pub u64);

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
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[self.below(xs.len() as u64) as usize]
    }
}
