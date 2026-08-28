//! Shared harness for the differential tests.
//!
//! Both the C program and the Rust program are driven as *subprocesses*, the way
//! a shell would run them. Nothing here links the Rust code as a library.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// Path to the Rust binary under test, provided by Cargo.
pub fn rust_binary() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// Workspace root (the directory holding `c_src/` and `translation/`).
fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is `<root>/translation`.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// A directory we own for scratch files, inside Cargo's target dir.
fn scratch_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        // <target>/release/driver -> <target>/release
        let dir = rust_binary()
            .parent()
            .expect("binary must live in a directory")
            .join("difftest-scratch");
        std::fs::create_dir_all(&dir).expect("create scratch dir");
        dir
    })
}

/// Path to the compiled C binary, building it with CMake on first use.
///
/// The build is directed *out of tree* (into Cargo's target directory) so that
/// nothing inside `c_src/` is ever created or modified by the test suite.
pub fn c_binary() -> &'static Path {
    static BIN: OnceLock<PathBuf> = OnceLock::new();
    BIN.get_or_init(|| {
        let root = repo_root();
        let c_src = root.join("c_src");
        assert!(
            c_src.join("CMakeLists.txt").is_file(),
            "expected {}/CMakeLists.txt",
            c_src.display()
        );

        // Prefer a binary the developer already built in the canonical spot.
        let prebuilt = c_src.join("build").join("driver");
        if prebuilt.is_file() {
            return prebuilt;
        }

        let build_dir = rust_binary()
            .parent()
            .expect("binary must live in a directory")
            .join("c_build");
        std::fs::create_dir_all(&build_dir).expect("create c build dir");

        let configure = Command::new("cmake")
            .arg("-S")
            .arg(&c_src)
            .arg("-B")
            .arg(&build_dir)
            .output()
            .expect("failed to run `cmake` - is CMake installed?");
        assert!(
            configure.status.success(),
            "cmake configure failed:\n{}\n{}",
            String::from_utf8_lossy(&configure.stdout),
            String::from_utf8_lossy(&configure.stderr)
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
            String::from_utf8_lossy(&build.stderr)
        );

        let bin = build_dir.join("driver");
        assert!(bin.is_file(), "C binary not found at {}", bin.display());
        bin
    })
}

/// How the child's stdin should be wired up.
#[derive(Clone, Debug)]
pub enum Stdin {
    /// stdin is `/dev/null` (immediate EOF), like `prog < /dev/null`.
    Null,
    /// stdin is a regular file holding these bytes, like `prog < file`.
    File(Vec<u8>),
    /// stdin is a pipe fed these bytes, like `printf ... | prog`.
    Pipe(Vec<u8>),
}

/// Everything observable about one run of a program.
#[derive(PartialEq, Eq)]
pub struct Outcome {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    /// Normal exit code, or `None` if killed by a signal.
    pub code: Option<i32>,
    /// Terminating signal, or `None` if it exited normally.
    pub signal: Option<i32>,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Outcome")
            .field("stdout", &Bytes(&self.stdout))
            .field("stderr", &Bytes(&self.stderr))
            .field("code", &self.code)
            .field("signal", &self.signal)
            .finish()
    }
}

/// Byte-exact, readable rendering so failures show invisible differences
/// (trailing newlines, stray spaces) rather than hiding them.
struct Bytes<'a>(&'a [u8]);

impl std::fmt::Debug for Bytes<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:?} ({} bytes)",
            String::from_utf8_lossy(self.0),
            self.0.len()
        )
    }
}

/// Run one program to completion and capture stdout, stderr and exit status.
pub fn run(bin: &Path, args: &[&str], stdin: &Stdin, envs: &[(&str, &str)]) -> Outcome {
    use std::os::unix::process::ExitStatusExt;

    let mut cmd = Command::new(bin);
    cmd.args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_clear()
        // A minimal, deterministic environment, plus whatever the case asks for.
        .env("PATH", "/usr/bin:/bin")
        .envs(envs.iter().copied());

    let mut pipe_payload: Option<Vec<u8>> = None;
    match stdin {
        Stdin::Null => {
            cmd.stdin(Stdio::null());
        }
        Stdin::File(bytes) => {
            let path = scratch_dir().join(format!("stdin-{}", unique()));
            std::fs::write(&path, bytes).expect("write stdin file");
            let file = std::fs::File::open(&path).expect("open stdin file");
            cmd.stdin(Stdio::from(file));
        }
        Stdin::Pipe(bytes) => {
            cmd.stdin(Stdio::piped());
            pipe_payload = Some(bytes.clone());
        }
    }

    let mut child = cmd
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    if let Some(payload) = pipe_payload {
        let mut sink = child.stdin.take().expect("piped stdin");
        // Feed stdin from another thread: neither program reads stdin, so a large
        // payload would otherwise fill the pipe buffer and deadlock against our
        // own `wait`. A broken pipe here is expected and is not a failure - it
        // just means the child exited without reading, which both programs do.
        std::thread::spawn(move || {
            let _ = sink.write_all(&payload);
            let _ = sink.flush();
        });
    }

    let out = child.wait_with_output().expect("wait for child");
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

fn unique() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    // Mix in the pid so parallel test binaries cannot collide.
    (std::process::id() as u64) << 20 | n
}

/// The core differential assertion: for one input, the C and Rust programs must
/// agree on stdout, stderr and exit status.
#[track_caller]
pub fn assert_same(case: &str, args: &[&str], stdin: Stdin, envs: &[(&str, &str)]) -> Outcome {
    let c = run(c_binary(), args, &stdin, envs);
    let rust = run(rust_binary(), args, &stdin, envs);

    assert_eq!(
        c.stdout, rust.stdout,
        "case {case}: stdout differs byte-for-byte\n  args={args:?}\n  C   ={:?}\n  Rust={:?}",
        Bytes(&c.stdout),
        Bytes(&rust.stdout)
    );
    assert_eq!(
        c.stderr, rust.stderr,
        "case {case}: stderr differs byte-for-byte\n  C   ={:?}\n  Rust={:?}",
        Bytes(&c.stderr),
        Bytes(&rust.stderr)
    );
    assert_eq!(
        c.code, rust.code,
        "case {case}: exit code differs (C={:?}, Rust={:?})",
        c.code, rust.code
    );
    assert_eq!(
        c.signal, rust.signal,
        "case {case}: terminating signal differs (C={:?}, Rust={:?})",
        c.signal, rust.signal
    );

    rust
}

/// The one and only thing this program is supposed to print.
pub const EXPECTED_STDOUT: &[u8] = b"Hello World!\n";
