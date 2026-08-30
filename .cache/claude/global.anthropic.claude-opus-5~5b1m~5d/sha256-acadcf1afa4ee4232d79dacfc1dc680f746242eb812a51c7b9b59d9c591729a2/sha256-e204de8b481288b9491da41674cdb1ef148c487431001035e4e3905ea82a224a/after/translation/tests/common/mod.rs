//! Shared helpers: locate/build both executables and run them as subprocesses.
//!
//! The C program is the ground truth. Every check in this suite drives BOTH
//! programs as child processes (exactly the way a shell would) and compares
//! stdout, stderr and exit status byte for byte. Nothing here loads the Rust
//! code as a library.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// Repository root: the directory that contains `c_src/` and `translation/`.
pub fn repo_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the Rust executable built by cargo for this integration test.
pub fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Path to the C executable, building it with CMake on first use if needed.
pub fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = repo_root().join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");
        if exe.is_file() {
            return exe;
        }
        std::fs::create_dir_all(&build).expect("create c_src/build");
        let configure = Command::new("cmake")
            .arg("..")
            .current_dir(&build)
            .output()
            .expect("failed to run `cmake ..` (is cmake installed?)");
        assert!(
            configure.status.success(),
            "cmake configure failed:\n{}\n{}",
            String::from_utf8_lossy(&configure.stdout),
            String::from_utf8_lossy(&configure.stderr)
        );
        let compile = Command::new("cmake")
            .args(["--build", "."])
            .current_dir(&build)
            .output()
            .expect("failed to run `cmake --build .`");
        assert!(
            compile.status.success(),
            "cmake build failed:\n{}\n{}",
            String::from_utf8_lossy(&compile.stdout),
            String::from_utf8_lossy(&compile.stderr)
        );
        assert!(exe.is_file(), "C executable missing after build: {:?}", exe);
        exe
    })
    .as_path()
}

/// Captured result of one subprocess run.
#[derive(PartialEq, Eq)]
pub struct Run {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    /// `Some(code)` for a normal exit, `None` if killed by a signal.
    pub code: Option<i32>,
}

impl std::fmt::Debug for Run {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Run {{ stdout: {:?}, stderr: {:?}, exit: {:?} }}",
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr),
            self.code
        )
    }
}

/// Run `exe`, feeding `input` on stdin, and capture everything it produces.
pub fn run(exe: &Path, input: &[u8]) -> Run {
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {:?}: {e}", exe));

    {
        let mut stdin = child.stdin.take().expect("piped stdin");
        let data = input.to_vec();
        // Write on a helper thread: a large input could otherwise fill the pipe
        // buffer and deadlock against a child that has stopped reading.
        std::thread::spawn(move || {
            let _ = stdin.write_all(&data);
            let _ = stdin.flush();
        });
    }

    let out = child.wait_with_output().expect("wait_with_output");
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
    }
}

/// Assert the C and Rust programs agree on stdout, stderr and exit status.
pub fn assert_same(input: &[u8]) {
    let c = run(c_bin(), input);
    let r = run(&rust_bin(), input);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout differs for input {}\n  C:    {:?}\n  Rust: {:?}",
        show(input),
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr differs for input {}\n  C:    {:?}\n  Rust: {:?}",
        show(input),
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
    assert_eq!(
        c.code,
        r.code,
        "exit status differs for input {}\n  C: {:?}  Rust: {:?}",
        show(input),
        c.code,
        r.code
    );
}

/// Check a whole batch, reporting every mismatch at once.
///
/// Inputs are spread over a small pool of worker threads: each case costs two
/// process spawns, and some batches below hold tens of thousands of cases.
pub fn assert_all_same<I, B>(inputs: I)
where
    I: IntoIterator<Item = B>,
    B: AsRef<[u8]>,
{
    let cases: Vec<Vec<u8>> = inputs.into_iter().map(|b| b.as_ref().to_vec()).collect();
    let n = cases.len();
    let workers = std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(4)
        .clamp(1, 16);

    let next = std::sync::atomic::AtomicUsize::new(0);
    let cases_ref = &cases;
    let next_ref = &next;

    let failures: Vec<String> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..workers)
            .map(|_| {
                scope.spawn(move || {
                    let rust = rust_bin();
                    let mut local: Vec<String> = Vec::new();
                    loop {
                        let i = next_ref.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let Some(input) = cases_ref.get(i) else { break };
                        let c = run(c_bin(), input);
                        let r = run(&rust, input);
                        if c != r {
                            local.push(format!(
                                "input {}\n     C: {:?}\n  Rust: {:?}",
                                show(input),
                                c,
                                r
                            ));
                        }
                    }
                    local
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().expect("worker thread panicked"))
            .collect()
    });

    assert!(
        failures.is_empty(),
        "{} of {} inputs mismatched:\n{}",
        failures.len(),
        n,
        // Keep the message bounded when a systemic bug fails everything.
        failures
            .iter()
            .take(25)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// Readable, escaped rendering of an input for failure messages.
pub fn show(input: &[u8]) -> String {
    let head: Vec<u8> = input.iter().copied().take(80).collect();
    let mut s = String::from("\"");
    for b in &head {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\t' => s.push_str("\\t"),
            b'\r' => s.push_str("\\r"),
            0x0b => s.push_str("\\v"),
            0x0c => s.push_str("\\f"),
            b'\\' => s.push_str("\\\\"),
            b'"' => s.push_str("\\\""),
            0x20..=0x7e => s.push(*b as char),
            other => s.push_str(&format!("\\x{other:02x}")),
        }
    }
    s.push('"');
    if input.len() > head.len() {
        s.push_str(&format!(" (+{} more bytes)", input.len() - head.len()));
    }
    s
}
