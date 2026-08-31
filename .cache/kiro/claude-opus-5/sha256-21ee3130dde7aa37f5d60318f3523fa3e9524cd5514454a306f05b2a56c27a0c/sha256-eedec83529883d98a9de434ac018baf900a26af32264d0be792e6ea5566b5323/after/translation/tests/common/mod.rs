//! Shared harness: locates and builds both executables, runs each of them as a
//! subprocess with the same stdin, and compares stdout, stderr and exit status.
//!
//! The Rust code is never linked as a library here. Both programs are driven the
//! way a shell drives them, because that is how the translation is graded.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::Once;

/// Absolute path to the crate root (`translation/`).
fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Absolute path to the C sources (`c_src/`), which sit next to `translation/`.
fn c_src_dir() -> PathBuf {
    manifest_dir()
        .parent()
        .expect("translation/ must have a parent directory")
        .join("c_src")
}

/// Path of the compiled Rust binary, supplied by Cargo for integration tests.
pub fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Path of the compiled C binary, building it on first use if necessary.
///
/// `c_src/` is read-only ground truth; only its out-of-tree `build/` directory
/// is created here, and only when the executable is not already present.
pub fn c_bin() -> PathBuf {
    static BUILD: Once = Once::new();
    let build_dir = c_src_dir().join("build");
    let exe = build_dir.join("driver");

    BUILD.call_once(|| {
        if exe.is_file() {
            return;
        }
        std::fs::create_dir_all(&build_dir).expect("cannot create c_src/build");

        let configure = Command::new("cmake")
            .arg("..")
            .current_dir(&build_dir)
            .output()
            .expect("failed to run `cmake` -- is it installed?");
        assert!(
            configure.status.success(),
            "cmake configure failed:\n{}\n{}",
            String::from_utf8_lossy(&configure.stdout),
            String::from_utf8_lossy(&configure.stderr)
        );

        let build = Command::new("cmake")
            .args(["--build", "."])
            .current_dir(&build_dir)
            .output()
            .expect("failed to run `cmake --build .`");
        assert!(
            build.status.success(),
            "cmake build failed:\n{}\n{}",
            String::from_utf8_lossy(&build.stdout),
            String::from_utf8_lossy(&build.stderr)
        );
    });

    assert!(
        exe.is_file(),
        "the C executable was not produced at {}",
        exe.display()
    );
    exe
}

/// Everything observable about one run of a program.
#[derive(PartialEq, Eq)]
pub struct Run {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    /// `Ok(code)` for a normal exit, `Err(signal)` when killed by a signal.
    pub status: Result<i32, i32>,
}

fn describe_status(status: &Result<i32, i32>) -> String {
    match status {
        Ok(code) => format!("exit {code}"),
        Err(sig) => format!("killed by signal {sig}"),
    }
}

impl std::fmt::Debug for Run {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{ stdout: {:?}, stderr: {:?}, status: {} }}",
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr),
            describe_status(&self.status)
        )
    }
}

fn classify(output: &Output) -> Result<i32, i32> {
    use std::os::unix::process::ExitStatusExt;
    match output.status.code() {
        Some(code) => Ok(code),
        None => Err(output.status.signal().unwrap_or(-1)),
    }
}

/// Runs `program` with `input` on stdin and captures stdout, stderr and status.
pub fn run(program: &Path, input: &[u8]) -> Run {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("cannot spawn {}: {e}", program.display()));

    {
        let mut stdin = child.stdin.take().expect("stdin was piped");
        // A program may stop reading before consuming all of the input (or the
        // kernel may report EPIPE); that is not a harness failure.
        let _ = stdin.write_all(input);
        let _ = stdin.flush();
    }

    let output = child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("cannot wait for {}: {e}", program.display()));

    Run {
        stdout: output.stdout.clone(),
        stderr: output.stderr.clone(),
        status: classify(&output),
    }
}

/// Asserts the C and Rust programs are indistinguishable for one input.
///
/// All three observables are compared: stdout byte for byte, stderr byte for
/// byte, and the exit status (including death by signal).
#[track_caller]
pub fn assert_same(label: &str, input: &[u8]) {
    let c = run(&c_bin(), input);
    let r = run(&rust_bin(), input);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout differs for {label} (input {:?}):\n  C:    {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(input),
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr differs for {label} (input {:?}):\n  C:    {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(input),
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
    assert_eq!(
        describe_status(&c.status),
        describe_status(&r.status),
        "exit status differs for {label} (input {:?})",
        String::from_utf8_lossy(input)
    );
}

/// Convenience wrapper for textual inputs.
#[track_caller]
pub fn assert_same_str(label: &str, input: &str) {
    assert_same(label, input.as_bytes());
}

/// Deterministic xorshift64* generator, so the fuzz cases are reproducible
/// without pulling in a dependency.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
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
        (self.next_u64() % (n as u64)) as usize
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }
}
