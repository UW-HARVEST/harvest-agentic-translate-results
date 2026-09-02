//! Shared harness for the differential tests.
//!
//! Both programs are driven as subprocesses exactly the way a shell would run
//! them: bytes on stdin, bytes off stdout/stderr, and an exit status. Nothing
//! here links against the Rust crate as a library.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Once;

/// Path to the Rust binary under test, as built by cargo.
pub fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Workspace root: the directory holding both `c_src/` and `translation/`.
fn repo_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

fn c_src_dir() -> PathBuf {
    repo_root().join("c_src")
}

fn c_build_dir() -> PathBuf {
    c_src_dir().join("build")
}

static BUILD_C: Once = Once::new();

/// Path to the C reference binary, building it on first use.
///
/// The C tree is treated as read-only input: we only ever add the cmake
/// `build/` directory, never touch a source file.
pub fn c_bin() -> PathBuf {
    let exe = c_build_dir().join("driver");

    BUILD_C.call_once(|| {
        if exe.is_file() {
            return;
        }
        let src = c_src_dir();
        assert!(
            src.join("CMakeLists.txt").is_file(),
            "missing {}",
            src.join("CMakeLists.txt").display()
        );
        std::fs::create_dir_all(c_build_dir()).expect("create c_src/build");

        let configure = Command::new("cmake")
            .arg("..")
            .current_dir(c_build_dir())
            .output()
            .expect("failed to run `cmake ..` (is cmake installed?)");
        assert!(
            configure.status.success(),
            "cmake configure failed:\n{}\n{}",
            String::from_utf8_lossy(&configure.stdout),
            String::from_utf8_lossy(&configure.stderr)
        );

        let build = Command::new("cmake")
            .args(["--build", "."])
            .current_dir(c_build_dir())
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
        "C reference binary not found at {}",
        exe.display()
    );
    exe
}

/// What one program produced for one input.
pub struct Run {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    /// `Some(code)` for a normal exit, `None` if killed by a signal.
    pub code: Option<i32>,
}

fn run_one(exe: &Path, stdin_bytes: &[u8], cwd: &Path) -> Run {
    let mut child = Command::new(exe)
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));

    {
        let mut sink = child.stdin.take().expect("piped stdin");
        // A short input may be fully consumed before we finish writing; the
        // program exiting early makes the write fail with EPIPE, which is not
        // a test failure.
        let _ = sink.write_all(stdin_bytes);
        let _ = sink.flush();
    }

    let out = child.wait_with_output().expect("wait_with_output");
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
    }
}

fn describe(label: &str, a: &[u8], b: &[u8]) -> String {
    let first = (0..a.len().min(b.len()))
        .find(|&i| a[i] != b[i])
        .unwrap_or_else(|| a.len().min(b.len()));
    let lo = first.saturating_sub(120);
    let hi_a = (first + 120).min(a.len());
    let hi_b = (first + 120).min(b.len());
    format!(
        "{label} differs at byte {first} (C len {}, Rust len {})\n\
         --- C   -> {:?}\n\
         --- Rust-> {:?}\n",
        a.len(),
        b.len(),
        String::from_utf8_lossy(&a[lo..hi_a]),
        String::from_utf8_lossy(&b[lo..hi_b]),
    )
}

/// Run both programs on `stdin_bytes` from `cwd` and assert that stdout,
/// stderr and the exit status are all byte-for-byte identical.
pub fn assert_same_in(name: &str, stdin_bytes: &[u8], cwd: &Path) {
    let c = run_one(&c_bin(), stdin_bytes, cwd);
    let r = run_one(&rust_bin(), stdin_bytes, cwd);

    let mut problems = String::new();
    if c.stdout != r.stdout {
        problems.push_str(&describe("stdout", &c.stdout, &r.stdout));
    }
    if c.stderr != r.stderr {
        problems.push_str(&describe("stderr", &c.stderr, &r.stderr));
    }
    if c.code != r.code {
        problems.push_str(&format!(
            "exit status differs: C {:?}, Rust {:?}\n",
            c.code, r.code
        ));
    }

    assert!(
        problems.is_empty(),
        "case `{name}`\ninput ({} bytes): {:?}\n{problems}",
        stdin_bytes.len(),
        String::from_utf8_lossy(&stdin_bytes[..stdin_bytes.len().min(400)]),
    );
}

/// `assert_same_in` with the repository root as the working directory.
pub fn assert_same(name: &str, stdin_bytes: &[u8]) {
    assert_same_in(name, stdin_bytes, &repo_root());
}

/// A scratch directory for fixture files, unique per caller-supplied tag.
pub fn scratch(tag: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(tag);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

/// Write a fixture file and return its path.
pub fn fixture(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, bytes).expect("write fixture");
    path
}
