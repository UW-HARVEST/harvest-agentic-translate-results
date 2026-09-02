//! Shared plumbing for the differential tests.
//!
//! Both programs are driven as subprocesses, exactly the way a shell would run them.
//! Nothing here links the translation as a library: the Rust side is the built
//! `driver` executable (`CARGO_BIN_EXE_driver`), and the C side is the executable
//! produced from `c_src/` by CMake.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;

/// What one program did for one input: the three things that are compared.
#[derive(Debug, PartialEq, Eq)]
pub struct Run {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    /// `Some(code)` for a normal exit, `None` if the process was killed by a signal.
    pub code: Option<i32>,
    /// Terminating signal number, when the process died from a signal.
    pub signal: Option<i32>,
}

impl Run {
    fn from_output(out: Output) -> Run {
        #[cfg(unix)]
        let signal = {
            use std::os::unix::process::ExitStatusExt;
            out.status.signal()
        };
        #[cfg(not(unix))]
        let signal = None;

        Run {
            stdout: out.stdout,
            stderr: out.stderr,
            code: out.status.code(),
            signal,
        }
    }
}

/// The repository root (parent of the `translation` crate directory).
fn repo_root() -> &'static Path {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("translation/ must have a parent directory")
            .to_path_buf()
    })
}

/// Path to the Rust executable under test.
pub fn rust_bin() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// Path to the C executable, building it with CMake on first use.
///
/// The build is configured out-of-tree (into this crate's `target/` directory) so that
/// running the tests never writes anything into `c_src/`. An already-built
/// `c_src/build/driver` is reused if present.
pub fn c_bin() -> &'static Path {
    static BIN: OnceLock<PathBuf> = OnceLock::new();
    BIN.get_or_init(|| {
        let prebuilt = repo_root().join("c_src").join("build").join("driver");
        if prebuilt.is_file() {
            return prebuilt;
        }

        let source = repo_root().join("c_src");
        let build = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("c_build");
        std::fs::create_dir_all(&build).expect("could not create the CMake build directory");

        let configure = Command::new("cmake")
            .arg("-S")
            .arg(&source)
            .arg("-B")
            .arg(&build)
            .output()
            .expect("could not run cmake; it is required to build the C reference program");
        assert!(
            configure.status.success(),
            "cmake configure failed:\n{}\n{}",
            String::from_utf8_lossy(&configure.stdout),
            String::from_utf8_lossy(&configure.stderr),
        );

        let compile = Command::new("cmake")
            .arg("--build")
            .arg(&build)
            .output()
            .expect("could not run cmake --build");
        assert!(
            compile.status.success(),
            "cmake build failed:\n{}\n{}",
            String::from_utf8_lossy(&compile.stdout),
            String::from_utf8_lossy(&compile.stderr),
        );

        let built = build.join("driver");
        assert!(built.is_file(), "expected the C driver at {}", built.display());
        built
    })
}

/// Run one program with `input` on stdin and collect all three observables.
fn run_with_stdin(program: &Path, input: &[u8]) -> Run {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("could not spawn {}: {e}", program.display()));

    {
        let mut stdin = child.stdin.take().expect("stdin was piped");
        // A broken pipe here just means the program exited without reading everything,
        // which is legitimate behavior and must not fail the test.
        let _ = stdin.write_all(input);
        let _ = stdin.flush();
    }

    Run::from_output(
        child
            .wait_with_output()
            .unwrap_or_else(|e| panic!("could not wait for {}: {e}", program.display())),
    )
}

/// Run one program with stdin redirected from `/dev/null`.
fn run_with_devnull(program: &Path) -> Run {
    let out = Command::new(program)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap_or_else(|e| panic!("could not run {}: {e}", program.display()));
    Run::from_output(out)
}

fn describe(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes {
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
    s
}

fn compare(case: &str, input_desc: &str, c: &Run, r: &Run) {
    let mut problems = Vec::new();
    if c.stdout != r.stdout {
        problems.push(format!(
            "stdout differs:\n  C   : {:?} (hex {})\n  Rust: {:?} (hex {})",
            String::from_utf8_lossy(&c.stdout),
            hex(&c.stdout),
            String::from_utf8_lossy(&r.stdout),
            hex(&r.stdout),
        ));
    }
    if c.stderr != r.stderr {
        problems.push(format!(
            "stderr differs:\n  C   : {:?} (hex {})\n  Rust: {:?} (hex {})",
            String::from_utf8_lossy(&c.stderr),
            hex(&c.stderr),
            String::from_utf8_lossy(&r.stderr),
            hex(&r.stderr),
        ));
    }
    if c.code != r.code || c.signal != r.signal {
        problems.push(format!(
            "exit status differs:\n  C   : code={:?} signal={:?}\n  Rust: code={:?} signal={:?}",
            c.code, c.signal, r.code, r.signal
        ));
    }
    assert!(
        problems.is_empty(),
        "case `{case}` with stdin \"{input_desc}\":\n{}",
        problems.join("\n")
    );
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Assert that both programs agree on stdout, stderr and exit status for `input`.
pub fn assert_same(case: &str, input: &[u8]) {
    let c = run_with_stdin(c_bin(), input);
    let r = run_with_stdin(rust_bin(), input);
    compare(case, &describe(input), &c, &r);
}

/// Assert agreement and, additionally, that stdout is exactly `expected_stdout`.
///
/// This pins down *which* branch of the C program ran, so a test cannot pass merely
/// because both programs are broken in the same visible way.
pub fn assert_same_and_stdout(case: &str, input: &[u8], expected_stdout: &[u8]) {
    let c = run_with_stdin(c_bin(), input);
    let r = run_with_stdin(rust_bin(), input);
    compare(case, &describe(input), &c, &r);
    assert_eq!(
        c.stdout,
        expected_stdout,
        "case `{case}`: the C program's stdout was {:?}, expected {:?}. \
         The expectation encoded in the test no longer matches the reference program.",
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(expected_stdout),
    );
}

/// Assert agreement when stdin is `/dev/null` (immediately at EOF, not a pipe).
pub fn assert_same_devnull(case: &str) {
    let c = run_with_devnull(c_bin());
    let r = run_with_devnull(rust_bin());
    compare(case, "<dev/null>", &c, &r);
}

/// stdout that the `good()` branch produces: `printIntPtrLine(&data)` with `data == 5`.
pub const GOOD_STDOUT: &[u8] = b"5\n";

/// stdout that the `bad()` branch produces.
///
/// `bad()` dereferences an uninitialized `int *`, which is undefined behavior. The
/// reference build produced by the shipped `CMakeLists.txt` reads `0` through that
/// stale stack slot, deterministically, so the translation reproduces `0\n`. Every
/// `bad()`-path case below also goes through `assert_same*`, so if the C program's
/// behavior ever changes on a different toolchain the comparison — not just this
/// constant — is what governs.
pub const BAD_STDOUT: &[u8] = b"0\n";
