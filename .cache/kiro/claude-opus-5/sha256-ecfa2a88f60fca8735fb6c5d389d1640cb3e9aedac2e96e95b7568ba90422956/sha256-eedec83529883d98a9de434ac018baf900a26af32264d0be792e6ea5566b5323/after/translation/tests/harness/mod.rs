#![allow(dead_code)]
//! Differential harness: runs the C binary and the Rust binary as subprocesses
//! and compares stdout, stderr and exit status byte for byte.
//!
//! Nothing here loads the Rust code as a library; both programs are driven
//! exactly the way a shell drives them (stdin redirected from a byte buffer,
//! stdout/stderr captured through pipes, or both redirected to one file for the
//! interleaving checks).

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// The Rust binary under test, as built by cargo for this test run.
pub const RUST_BIN: &str = env!("CARGO_BIN_EXE_driver");

/// Result of one run: raw stdout, raw stderr and the exit status as reported by
/// the shell (128 + signal number when the process died from a signal).
#[derive(PartialEq, Eq)]
pub struct Run {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub status: i32,
}

fn workspace_root() -> PathBuf {
    // tests/ live in the crate; the C sources are the crate's sibling.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate has a parent directory")
        .to_path_buf()
}

/// Path to the C executable, built on first use with the commands from the
/// task description (`cmake .. && cmake --build .` in `c_src/build`). Nothing
/// under `c_src/` other than the cmake build directory is touched.
pub fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = workspace_root().join("c_src");
        let build = c_src.join("build");
        let bin = build.join("driver");
        if !bin.exists() {
            std::fs::create_dir_all(&build).expect("create c_src/build");
            let configure = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("run cmake");
            assert!(
                configure.status.success(),
                "cmake configure failed:\n{}",
                String::from_utf8_lossy(&configure.stderr)
            );
            let compile = Command::new("cmake")
                .args(["--build", "."])
                .current_dir(&build)
                .output()
                .expect("run cmake --build");
            assert!(
                compile.status.success(),
                "cmake --build failed:\n{}",
                String::from_utf8_lossy(&compile.stderr)
            );
        }
        assert!(bin.exists(), "C binary missing at {}", bin.display());
        bin
    })
}

/// `true` when this machine can start a process with address space
/// randomisation turned off. Needed only by the tests whose output depends on
/// heap addresses: with ASLR on, the C program prints different bytes on every
/// run, so there is nothing stable to compare against.
pub fn aslr_can_be_disabled() -> bool {
    static OK: OnceLock<bool> = OnceLock::new();
    *OK.get_or_init(|| {
        let probe = Command::new("setarch")
            .args(["-R", RUST_BIN])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        matches!(probe, Ok(s) if s.success())
    })
}

fn status_code(status: std::process::ExitStatus) -> i32 {
    use std::os::unix::process::ExitStatusExt;
    match status.code() {
        Some(code) => code,
        // Killed by a signal: report it the way a shell does.
        None => 128 + status.signal().expect("exited without code or signal"),
    }
}

fn spawn(program: &Path, no_aslr: bool, input: &[u8]) -> Run {
    let mut cmd = if no_aslr {
        let mut c = Command::new("setarch");
        c.arg("-R").arg(program);
        c
    } else {
        Command::new(program)
    };

    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", program.display()));

    {
        let mut stdin = child.stdin.take().expect("piped stdin");
        let owned = input.to_vec();
        // The programs write while we write; feeding stdin from another thread
        // keeps a full pipe buffer from deadlocking either side.
        std::thread::spawn(move || {
            let _ = stdin.write_all(&owned);
        });
    }

    let out = child.wait_with_output().expect("wait for child");
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        status: status_code(out.status),
    }
}

/// Run both programs on the same input with separate pipes for stdout/stderr.
pub fn run_both(input: &[u8], no_aslr: bool) -> (Run, Run) {
    (
        spawn(c_bin(), no_aslr, input),
        spawn(Path::new(RUST_BIN), no_aslr, input),
    )
}

fn show(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\t' => s.push_str("\\t"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    s
}

fn first_difference(a: &[u8], b: &[u8]) -> String {
    let at = a
        .iter()
        .zip(b.iter())
        .position(|(x, y)| x != y)
        .unwrap_or(a.len().min(b.len()));
    let from = at.saturating_sub(60);
    format!(
        "first difference at byte {at} (C {} bytes, Rust {} bytes)\n  C   : ...{}\n  Rust: ...{}",
        a.len(),
        b.len(),
        show(&a[from..(at + 60).min(a.len())]),
        show(&b[from..(at + 60).min(b.len())]),
    )
}

fn compare(label: &str, input: &[u8], c: &Run, r: &Run) {
    if c.stdout != r.stdout {
        panic!(
            "[{label}] stdout differs\ninput: {}\n{}",
            show(input),
            first_difference(&c.stdout, &r.stdout)
        );
    }
    if c.stderr != r.stderr {
        panic!(
            "[{label}] stderr differs\ninput: {}\n{}",
            show(input),
            first_difference(&c.stderr, &r.stderr)
        );
    }
    if c.status != r.status {
        panic!(
            "[{label}] exit status differs: C={} Rust={}\ninput: {}",
            c.status,
            r.status,
            show(input)
        );
    }
}

/// stdout, stderr and exit status must match byte for byte.
#[track_caller]
pub fn same(label: &str, input: &[u8]) {
    let (c, r) = run_both(input, false);
    compare(label, input, &c, &r);
}

/// Same as [`same`], for inputs whose output includes bytes read out of a freed
/// heap chunk. Those bytes are pieces of heap addresses, so the C program's own
/// output for them is only reproducible with ASLR disabled.
///
/// * If this machine can disable ASLR, the comparison is exact.
/// * Otherwise the C program is run twice and only the part it reproduces (the
///   common prefix of the two runs) is required of the Rust program. Exit status
///   and stderr are always compared exactly.
#[track_caller]
pub fn same_freed_memory(label: &str, input: &[u8]) {
    if aslr_can_be_disabled() {
        let (c, r) = run_both(input, true);
        compare(label, input, &c, &r);
        return;
    }

    let (c1, r) = run_both(input, false);
    let c2 = spawn(c_bin(), false, input);
    if c1.stdout == c2.stdout {
        compare(label, input, &c1, &r);
        return;
    }

    let stable = c1
        .stdout
        .iter()
        .zip(c2.stdout.iter())
        .take_while(|(a, b)| a == b)
        .count();
    assert!(
        r.stdout.len() >= stable && r.stdout[..stable] == c1.stdout[..stable],
        "[{label}] stdout differs over the {stable} bytes the C program reproduces\ninput: {}\n{}",
        show(input),
        first_difference(&c1.stdout[..stable], &r.stdout)
    );
    let mut c = c1;
    c.stdout.truncate(stable);
    let mut r = r;
    r.stdout.truncate(stable);
    compare(label, input, &c, &r);
}

/// Run both programs with stdout and stderr pointing at the *same* file, so the
/// comparison also covers where the block-buffered stdout stream is flushed
/// relative to the unbuffered stderr writes.
#[track_caller]
pub fn same_merged(label: &str, input: &[u8]) {
    let dir = std::env::temp_dir();
    let unique = format!(
        "dagdiff-{}-{}-{:?}",
        std::process::id(),
        label,
        std::thread::current().id()
    );

    let mut outputs = Vec::new();
    let mut statuses = Vec::new();
    for program in [c_bin().to_path_buf(), PathBuf::from(RUST_BIN)] {
        let path = dir.join(format!("{unique}-{}", outputs.len()));
        let file = std::fs::File::create(&path).expect("create merged output file");
        let err = file.try_clone().expect("clone fd for stderr");
        let mut child = Command::new(&program)
            .stdin(Stdio::piped())
            .stdout(Stdio::from(file))
            .stderr(Stdio::from(err))
            .spawn()
            .unwrap_or_else(|e| panic!("spawn {}: {e}", program.display()));
        {
            let mut stdin = child.stdin.take().expect("piped stdin");
            let owned = input.to_vec();
            std::thread::spawn(move || {
                let _ = stdin.write_all(&owned);
            });
        }
        let status = child.wait().expect("wait for child");
        statuses.push(status_code(status));
        outputs.push(std::fs::read(&path).expect("read merged output"));
        let _ = std::fs::remove_file(&path);
    }

    assert_eq!(
        outputs[0].len(),
        outputs[1].len(),
        "[{label}] merged output length differs\ninput: {}\n{}",
        show(input),
        first_difference(&outputs[0], &outputs[1])
    );
    if outputs[0] != outputs[1] {
        panic!(
            "[{label}] merged stdout+stderr differs\ninput: {}\n{}",
            show(input),
            first_difference(&outputs[0], &outputs[1])
        );
    }
    assert_eq!(
        statuses[0], statuses[1],
        "[{label}] exit status differs: C={} Rust={}",
        statuses[0], statuses[1]
    );
}
