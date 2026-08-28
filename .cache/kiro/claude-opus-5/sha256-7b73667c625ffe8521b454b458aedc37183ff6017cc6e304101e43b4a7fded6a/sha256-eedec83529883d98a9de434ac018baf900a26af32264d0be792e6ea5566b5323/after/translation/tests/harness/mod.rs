#![allow(dead_code)]
//! Shared harness for the differential tests.
//!
//! Both programs are driven as subprocesses, exactly the way a shell would
//! run them: bytes go in on stdin, and stdout, stderr and the exit status
//! come back out. Nothing here calls the Rust crate as a library, because
//! that is not how the translation is graded.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// Everything a single program run produced.
#[derive(PartialEq, Eq, Clone)]
pub struct Output {
    pub status: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl std::fmt::Debug for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "status={:?} stdout={:?} stderr={:?}",
            self.status,
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr)
        )
    }
}

/// Absolute path to the repository root (the directory holding `c_src/`
/// and `translation/`).
fn repo_root() -> &'static Path {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest
            .parent()
            .expect("translation/ must have a parent directory")
            .to_path_buf()
    })
}

/// Build the C program with CMake and return the path to the executable.
///
/// The build tree is placed under the Rust crate's `target/` directory so
/// that nothing inside `c_src/` is created or modified by the test run.
pub fn c_binary() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let root = repo_root();
        let src = root.join("c_src");
        assert!(
            src.join("CMakeLists.txt").is_file(),
            "expected {} to contain CMakeLists.txt",
            src.display()
        );

        let build = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("c_build");
        std::fs::create_dir_all(&build).expect("could not create the C build directory");

        let configure = Command::new("cmake")
            .arg("-S")
            .arg(&src)
            .arg("-B")
            .arg(&build)
            .output()
            .expect("could not run `cmake` - is CMake installed and on PATH?");
        assert!(
            configure.status.success(),
            "cmake configure failed:\n{}\n{}",
            String::from_utf8_lossy(&configure.stdout),
            String::from_utf8_lossy(&configure.stderr)
        );

        let compile = Command::new("cmake")
            .arg("--build")
            .arg(&build)
            .output()
            .expect("could not run `cmake --build`");
        assert!(
            compile.status.success(),
            "cmake build failed:\n{}\n{}",
            String::from_utf8_lossy(&compile.stdout),
            String::from_utf8_lossy(&compile.stderr)
        );

        let exe = build.join("driver");
        assert!(
            exe.is_file(),
            "the C build did not produce {}",
            exe.display()
        );
        exe
    })
}

/// Path to the Rust executable under test. Cargo points this at the binary
/// built for whichever profile the tests are running under.
pub fn rust_binary() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// Run one program with `input` on stdin and collect everything it produced.
pub fn run(program: &Path, input: &[u8]) -> Output {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("could not spawn {}: {e}", program.display()));

    {
        let mut stdin = child.stdin.take().expect("stdin was piped");
        let input = input.to_vec();
        // Write on a helper thread: an input longer than the pipe buffer
        // would otherwise deadlock against a child that stops reading.
        std::thread::spawn(move || {
            let _ = stdin.write_all(&input);
            let _ = stdin.flush();
        });
    }

    let out = child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("could not wait for {}: {e}", program.display()));

    Output {
        status: out.status.code(),
        stdout: out.stdout,
        stderr: out.stderr,
    }
}

/// Compare both programs on one input. Returns `Err` with a rendered
/// report when stdout, stderr or the exit status differ.
pub fn compare(input: &[u8]) -> Result<(), String> {
    let c = run(c_binary(), input);
    let rust = run(rust_binary(), input);

    if c == rust {
        return Ok(());
    }

    let mut report = format!("input {}\n", describe(input));
    if c.stdout != rust.stdout {
        report += &format!(
            "  stdout differs:\n    C   : {:?}\n    Rust: {:?}\n",
            String::from_utf8_lossy(&c.stdout),
            String::from_utf8_lossy(&rust.stdout)
        );
    }
    if c.stderr != rust.stderr {
        report += &format!(
            "  stderr differs:\n    C   : {:?}\n    Rust: {:?}\n",
            String::from_utf8_lossy(&c.stderr),
            String::from_utf8_lossy(&rust.stderr)
        );
    }
    if c.status != rust.status {
        report += &format!(
            "  exit status differs: C = {:?}, Rust = {:?}\n",
            c.status, rust.status
        );
    }
    Err(report)
}

/// Human-readable rendering of an input, abbreviated when it is long.
pub fn describe(input: &[u8]) -> String {
    let shown: Vec<u8> = input.iter().copied().take(96).collect();
    let mut s = format!("{:?}", String::from_utf8_lossy(&shown));
    if input.len() > shown.len() {
        s += &format!(" ...(+{} bytes, {} total)", input.len() - shown.len(), input.len());
    }
    s
}

/// Compare every input, in parallel, and fail once with every mismatch
/// listed rather than stopping at the first one.
pub fn compare_all(label: &str, inputs: Vec<Vec<u8>>) {
    // Force the one-time C build before fanning out, so concurrent
    // workers never race to configure the same CMake build tree.
    let _ = c_binary();

    let total = inputs.len();
    let workers = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .clamp(1, 16);

    let inputs = std::sync::Arc::new(inputs);
    let next = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let failures = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));

    let mut handles = Vec::with_capacity(workers);
    for _ in 0..workers {
        let inputs = std::sync::Arc::clone(&inputs);
        let next = std::sync::Arc::clone(&next);
        let failures = std::sync::Arc::clone(&failures);
        handles.push(std::thread::spawn(move || loop {
            let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if i >= inputs.len() {
                break;
            }
            if let Err(report) = compare(&inputs[i]) {
                failures.lock().unwrap().push(report);
            }
        }));
    }
    for h in handles {
        h.join().expect("a comparison worker panicked");
    }

    let failures = failures.lock().unwrap();
    if !failures.is_empty() {
        let shown: Vec<&String> = failures.iter().take(25).collect();
        panic!(
            "{label}: {} of {total} inputs differed between the C and Rust programs\n\n{}{}",
            failures.len(),
            shown
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
            if failures.len() > shown.len() {
                format!("\n...and {} more\n", failures.len() - shown.len())
            } else {
                String::new()
            }
        );
    }
}

/// Build a three-line stdin image: operation, parameter, decision string.
pub fn case(operation: &str, param: &str, decisions: &str) -> Vec<u8> {
    format!("{operation}\n{param}\n{decisions}\n").into_bytes()
}

/// All `y`/`n` strings of exactly `len` characters.
pub fn all_patterns(len: u32) -> Vec<String> {
    (0..(1u64 << len))
        .map(|bits| {
            (0..len)
                .map(|i| if (bits >> i) & 1 == 1 { 'y' } else { 'n' })
                .collect()
        })
        .collect()
}
