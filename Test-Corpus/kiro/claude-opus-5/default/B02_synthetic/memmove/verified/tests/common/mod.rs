//! Shared harness for the differential tests.
//!
//! Both programs are driven exactly as a shell would drive them: spawn the
//! executable, write the test input to its stdin, then compare stdout, stderr
//! and the exit status byte for byte. Nothing here links against the Rust crate
//! as a library.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// Path to the Rust executable under test. Cargo builds the `driver` bin target
/// before running integration tests and hands us the path.
pub fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_source_dir() -> PathBuf {
    manifest_dir().join("..").join("c_src")
}

/// Path to the C executable, building it with CMake on first use if needed.
///
/// `c_src/` itself is never modified: when a build is required it is configured
/// out-of-source into `translation/target/c_build`.
pub fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let candidates = [
            c_source_dir().join("build").join("driver"),
            manifest_dir().join("target").join("c_build").join("driver"),
        ];
        for candidate in &candidates {
            if candidate.is_file() {
                return candidate.clone();
            }
        }

        let build_dir = manifest_dir().join("target").join("c_build");
        std::fs::create_dir_all(&build_dir).expect("failed to create C build directory");

        let configure = Command::new("cmake")
            .arg("-S")
            .arg(c_source_dir())
            .arg("-B")
            .arg(&build_dir)
            .output()
            .expect("failed to run `cmake` -- is CMake installed?");
        assert!(
            configure.status.success(),
            "cmake configure failed:\n{}\n{}",
            String::from_utf8_lossy(&configure.stdout),
            String::from_utf8_lossy(&configure.stderr),
        );

        let build = Command::new("cmake")
            .arg("--build")
            .arg(&build_dir)
            .output()
            .expect("failed to run `cmake --build`");
        assert!(
            build.status.success(),
            "cmake build failed:\n{}\n{}",
            String::from_utf8_lossy(&build.stdout),
            String::from_utf8_lossy(&build.stderr),
        );

        let built = build_dir.join("driver");
        assert!(built.is_file(), "C driver not found at {}", built.display());
        built
    })
    .as_path()
}

/// Everything an external observer can see from one run.
#[derive(PartialEq, Eq)]
pub struct Observed {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    /// `Ok(code)` for a normal exit, `Err(signal)` when killed by a signal.
    pub status: Result<i32, i32>,
}

impl std::fmt::Debug for Observed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Observed")
            .field("stdout", &String::from_utf8_lossy(&self.stdout))
            .field("stderr", &String::from_utf8_lossy(&self.stderr))
            .field("status", &self.status)
            .finish()
    }
}

/// Run `exe` with `input` on stdin and capture everything it produces.
pub fn run(exe: &Path, input: &[u8]) -> Observed {
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));

    {
        let mut stdin = child.stdin.take().expect("stdin was piped");
        let owned = input.to_vec();
        // Writing on a helper thread avoids deadlocking on inputs larger than a
        // pipe buffer while the child is still filling its own output pipes.
        std::thread::spawn(move || {
            let _ = stdin.write_all(&owned);
        });
    }

    let out = child.wait_with_output().expect("failed to wait for child");

    #[cfg(unix)]
    let status = {
        use std::os::unix::process::ExitStatusExt;
        match out.status.code() {
            Some(code) => Ok(code),
            None => Err(out.status.signal().expect("no exit code and no signal")),
        }
    };
    #[cfg(not(unix))]
    let status = Ok(out.status.code().unwrap_or(-1));

    Observed {
        stdout: out.stdout,
        stderr: out.stderr,
        status,
    }
}

fn render(input: &[u8]) -> String {
    let text = String::from_utf8_lossy(input);
    if text.len() <= 400 {
        format!("{text:?}")
    } else {
        format!("{:?} ... ({} bytes total)", &text[..400], input.len())
    }
}

/// Assert that both programs agree on stdout, stderr and exit status.
pub fn assert_same(label: &str, input: &[u8]) {
    let c = run(c_bin(), input);
    let r = run(&rust_bin(), input);

    assert_eq!(
        c.stdout,
        r.stdout,
        "[{label}] stdout differs\n  input: {}\n  C   : {:?}\n  Rust: {:?}",
        render(input),
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout),
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "[{label}] stderr differs\n  input: {}\n  C   : {:?}\n  Rust: {:?}",
        render(input),
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr),
    );
    assert_eq!(
        c.status,
        r.status,
        "[{label}] exit status differs\n  input: {}\n  C   : {:?}\n  Rust: {:?}",
        render(input),
        c.status,
        r.status,
    );
}

/// Build one stdin image: `flags param1 param2 length b0 b1 ...\n`.
pub fn input(flags: &str, param1: &str, param2: &str, length: usize, data: &[u8]) -> Vec<u8> {
    let mut s = format!("{flags} {param1} {param2} {length}");
    for b in data {
        s.push(' ');
        s.push_str(&b.to_string());
    }
    s.push('\n');
    s.into_bytes()
}

/// Same as [`input`] but with `length` taken from `data`.
pub fn case(flags: u32, param1: i64, param2: i64, data: &[u8]) -> Vec<u8> {
    input(
        &flags.to_string(),
        &param1.to_string(),
        &param2.to_string(),
        data.len(),
        data,
    )
}

/// `true` when the C program would write past the end of its `uint8_t
/// buffer[256]`, which only `compact_runs` with a threshold of 1 can do (each
/// single-element run is rewritten as the pair `value, 1`).
///
/// Those inputs are undefined behaviour in the C original: the crash boundary
/// moves with the compiler's stack layout (we measured a final length of 314 at
/// `-O0` and 282 at `-O3` for the same program), so they are excluded from the
/// differential comparison instead of being asserted. See `ERRORS.md`.
pub fn c_overflows(flags: u32, param1: i64, length: usize) -> bool {
    if length == 0 || flags & 0x02 == 0 {
        return false;
    }
    let threshold = if param1 > 0 && param1 <= 255 { param1 } else { 3 };
    // Only threshold 1 grows the logical length, and it can at most double it,
    // so staying at or below 128 input bytes keeps every write below index 256.
    threshold == 1 && length > 128
}
