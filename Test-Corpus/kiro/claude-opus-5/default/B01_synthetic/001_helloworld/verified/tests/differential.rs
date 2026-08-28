// Differential tests: run the C program and the Rust program as subprocesses and
// require byte-identical stdout, byte-identical stderr, and an identical exit
// status (including termination by signal).
//
// The Rust code is never called as a library; only the built binary is driven,
// the same way a shell would drive it.
//
// The C program (c_src/src/main.c) is:
//
//     int main() {
//         printf("Hello World!\n");
//         return 0;
//     }
//
// It has no conditionals, no loops, no early returns, and it never reads stdin
// or argv. So the input classes it "branches on" are not textual inputs but
// process-level ones: stdin contents (all ignored), argv (all ignored), the
// environment, and the kind/state of the stdout and stderr descriptors, which is
// what decides whether the buffered `printf` flush at exit succeeds, fails, or
// kills the process.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

/// Path to the Rust binary under test, built by cargo for the current profile.
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the C binary, building it with CMake on first use if necessary.
/// `c_src/` is only read and built out-of-tree into `c_src/build/`; no source
/// file there is modified.
fn c_bin() -> &'static Path {
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
                .expect("run cmake (is cmake installed?)");
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
                .expect("run cmake --build");
            assert!(
                compile.status.success(),
                "cmake build failed:\n{}\n{}",
                String::from_utf8_lossy(&compile.stdout),
                String::from_utf8_lossy(&compile.stderr)
            );
        }
        assert!(bin.exists(), "C binary missing at {}", bin.display());
        bin
    })
    .as_path()
}

/// Everything observable about one run.
#[derive(PartialEq, Eq)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    code: Option<i32>,
    signal: Option<i32>,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Outcome")
            .field("stdout", &String::from_utf8_lossy(&self.stdout))
            .field("stdout_bytes", &self.stdout)
            .field("stderr", &String::from_utf8_lossy(&self.stderr))
            .field("stderr_bytes", &self.stderr)
            .field("code", &self.code)
            .field("signal", &self.signal)
            .finish()
    }
}

/// How the child's environment should be set up.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Env {
    Inherit,
    Cleared,
    /// Cleared, then these pairs added.
    Only(&'static [(&'static str, &'static str)]),
}

/// One process-level scenario, applied identically to both binaries.
struct Scenario {
    args: Vec<String>,
    stdin: StdinKind,
    env: Env,
}

#[derive(Clone)]
enum StdinKind {
    /// Feed these exact bytes on stdin, then close it.
    Bytes(Vec<u8>),
    /// stdin connected to /dev/null.
    Null,
    /// stdin is a pipe closed immediately without writing anything.
    ClosedPipe,
    /// stdin inherited from the test process.
    Inherit,
}

impl Scenario {
    fn new() -> Self {
        Scenario {
            args: Vec::new(),
            stdin: StdinKind::Bytes(Vec::new()),
            env: Env::Inherit,
        }
    }

    fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args = args.into_iter().map(Into::into).collect();
        self
    }

    fn stdin_bytes<B: Into<Vec<u8>>>(mut self, bytes: B) -> Self {
        self.stdin = StdinKind::Bytes(bytes.into());
        self
    }

    fn stdin(mut self, kind: StdinKind) -> Self {
        self.stdin = kind;
        self
    }

    fn env(mut self, env: Env) -> Self {
        self.env = env;
        self
    }

    fn command(&self, bin: &Path) -> Command {
        let mut cmd = Command::new(bin);
        cmd.args(&self.args);
        match self.env {
            Env::Inherit => {}
            Env::Cleared => {
                cmd.env_clear();
            }
            Env::Only(pairs) => {
                cmd.env_clear();
                for (k, v) in pairs {
                    cmd.env(k, v);
                }
            }
        }
        cmd
    }

    /// Run one binary with stdout and stderr on pipes.
    fn run(&self, bin: &Path) -> Outcome {
        let mut cmd = self.command(bin);
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        match &self.stdin {
            StdinKind::Bytes(_) | StdinKind::ClosedPipe => {
                cmd.stdin(Stdio::piped());
            }
            StdinKind::Null => {
                cmd.stdin(Stdio::null());
            }
            StdinKind::Inherit => {
                cmd.stdin(Stdio::inherit());
            }
        }

        let mut child = cmd.spawn().unwrap_or_else(|e| panic!("spawn {}: {e}", bin.display()));

        match &self.stdin {
            StdinKind::Bytes(bytes) => {
                let mut sink = child.stdin.take().expect("piped stdin");
                let bytes = bytes.clone();
                // Write on a helper thread so a child that never reads cannot
                // deadlock the test (this program never reads stdin at all).
                let writer = std::thread::spawn(move || {
                    let _ = sink.write_all(&bytes);
                    let _ = sink.flush();
                });
                let out = child.wait_with_output().expect("wait_with_output");
                let _ = writer.join();
                return outcome(out);
            }
            StdinKind::ClosedPipe => {
                drop(child.stdin.take());
            }
            StdinKind::Null | StdinKind::Inherit => {}
        }

        let out = child.wait_with_output().expect("wait_with_output");
        outcome(out)
    }

    /// Run one binary with stdout redirected to a regular file (this makes the C
    /// stdio stream fully buffered, like a pipe, but with a seekable target).
    fn run_stdout_to_file(&self, bin: &Path, path: &Path) -> Outcome {
        let file = std::fs::File::create(path).expect("create stdout file");
        let mut cmd = self.command(bin);
        cmd.stdin(Stdio::null())
            .stdout(Stdio::from(file))
            .stderr(Stdio::piped());
        let out = cmd.output().expect("run with stdout to file");
        let mut stdout = Vec::new();
        std::fs::File::open(path)
            .expect("reopen stdout file")
            .read_to_end(&mut stdout)
            .expect("read stdout file");
        Outcome {
            stdout,
            stderr: out.stderr,
            code: out.status.code(),
            signal: signal_of(&out.status),
        }
    }

    /// Run one binary whose stdout is a pipe whose read end is closed before the
    /// child writes: the flush at exit hits EPIPE / SIGPIPE.
    fn run_with_broken_stdout(&self, bin: &Path) -> Outcome {
        let mut child = self
            .command(bin)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("spawn {}: {e}", bin.display()));
        drop(child.stdout.take()); // close the read end
        let mut stderr = child.stderr.take().expect("piped stderr");
        let mut err = Vec::new();
        let _ = stderr.read_to_end(&mut err);
        let status = child.wait().expect("wait");
        Outcome {
            stdout: Vec::new(),
            stderr: err,
            code: status.code(),
            signal: signal_of(&status),
        }
    }
}

fn signal_of(status: &std::process::ExitStatus) -> Option<i32> {
    #[cfg(unix)]
    {
        status.signal()
    }
    #[cfg(not(unix))]
    {
        let _ = status;
        None
    }
}

fn outcome(out: std::process::Output) -> Outcome {
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: signal_of(&out.status),
    }
}

/// Assert that both binaries agree on stdout, stderr and exit status.
fn assert_same(label: &str, scenario: &Scenario) -> Outcome {
    let c = scenario.run(c_bin());
    let rust = scenario.run(&rust_bin());
    compare(label, &c, &rust);
    c
}

fn compare(label: &str, c: &Outcome, rust: &Outcome) {
    assert_eq!(
        c.stdout, rust.stdout,
        "[{label}] stdout differs\n  C:    {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&rust.stdout)
    );
    assert_eq!(
        c.stderr, rust.stderr,
        "[{label}] stderr differs\n  C:    {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&rust.stderr)
    );
    assert_eq!(
        (c.code, c.signal),
        (rust.code, rust.signal),
        "[{label}] exit status differs\n  C:    {c:?}\n  Rust: {rust:?}"
    );
}

const EXPECTED: &[u8] = b"Hello World!\n";

// ---------------------------------------------------------------------------
// Baseline: the single code path the C program has.
// ---------------------------------------------------------------------------

#[test]
fn no_args_empty_stdin() {
    let out = assert_same("no args, empty stdin", &Scenario::new());
    // Pin the literal so a regression in both directions cannot hide.
    assert_eq!(out.stdout, EXPECTED, "C stdout is not the expected literal");
    assert!(out.stderr.is_empty(), "C wrote to stderr: {:?}", out.stderr);
    assert_eq!(out.code, Some(0), "C exit code changed");
    assert_eq!(out.signal, None, "C was killed by a signal");
}

#[test]
fn output_is_deterministic_across_repeated_runs() {
    let scenario = Scenario::new();
    let first = assert_same("repeat #0", &scenario);
    for i in 1..10 {
        let again = assert_same(&format!("repeat #{i}"), &scenario);
        assert!(first == again, "run {i} differed from run 0");
    }
}

// ---------------------------------------------------------------------------
// stdin: the C program never reads it. Every shape must be ignored the same way,
// including one that a `scanf`/`fgets` translation would have consumed.
// ---------------------------------------------------------------------------

#[test]
fn stdin_empty_pipe() {
    assert_same("stdin: empty pipe", &Scenario::new().stdin_bytes(&b""[..]));
}

#[test]
fn stdin_dev_null() {
    assert_same("stdin: /dev/null", &Scenario::new().stdin(StdinKind::Null));
}

#[test]
fn stdin_closed_pipe() {
    assert_same("stdin: closed pipe", &Scenario::new().stdin(StdinKind::ClosedPipe));
}

#[test]
fn stdin_single_item() {
    assert_same("stdin: single line", &Scenario::new().stdin_bytes(&b"1\n"[..]));
}

#[test]
fn stdin_single_item_without_trailing_newline() {
    assert_same("stdin: no trailing newline", &Scenario::new().stdin_bytes(&b"1"[..]));
}

#[test]
fn stdin_only_newlines() {
    assert_same("stdin: only newlines", &Scenario::new().stdin_bytes(&b"\n\n\n\n"[..]));
}

#[test]
fn stdin_whitespace_and_crlf() {
    assert_same(
        "stdin: whitespace and CRLF",
        &Scenario::new().stdin_bytes(&b" \t\r\n  42\r\n\t\r\n"[..]),
    );
}

#[test]
fn stdin_non_numeric_text() {
    assert_same(
        "stdin: non-numeric",
        &Scenario::new().stdin_bytes(&b"not a number at all\n"[..]),
    );
}

#[test]
fn stdin_numbers_that_would_overflow_an_int() {
    // Values chosen to break any accidental integer parsing: INT_MIN/INT_MAX
    // edges, a value past u64, and a negative count.
    assert_same(
        "stdin: overflowing numbers",
        &Scenario::new().stdin_bytes(
            &b"2147483647\n2147483648\n-2147483648\n-2147483649\n18446744073709551616\n-1\n0\n"[..],
        ),
    );
}

#[test]
fn stdin_invalid_utf8_and_nul_bytes() {
    let mut bytes = vec![0u8, 1, 2, 0xff, 0xfe, 0x80, b'\n', 0x00];
    bytes.extend_from_slice(&[0xc3, 0x28, 0xa0, 0xa1, b'\n']);
    assert_same("stdin: invalid UTF-8 + NUL", &Scenario::new().stdin_bytes(bytes));
}

#[test]
fn stdin_one_mib_of_lines() {
    // "the maximum the code handles": far more input than the program could ever
    // consume, delivered while it exits without reading a byte.
    let mut bytes = Vec::with_capacity(1 << 20);
    let mut i = 0u32;
    while bytes.len() < (1 << 20) {
        bytes.extend_from_slice(format!("{i}\n").as_bytes());
        i += 1;
    }
    assert_same("stdin: 1 MiB", &Scenario::new().stdin_bytes(bytes));
}

#[test]
fn stdin_single_very_long_line() {
    let mut bytes = vec![b'x'; 300_000];
    bytes.push(b'\n');
    assert_same("stdin: 300k-char line", &Scenario::new().stdin_bytes(bytes));
}

// ---------------------------------------------------------------------------
// argv: `main()` takes no parameters, so every argument vector is ignored.
// ---------------------------------------------------------------------------

#[test]
fn args_single() {
    assert_same("argv: one arg", &Scenario::new().args(["one"]));
}

#[test]
fn args_several() {
    assert_same("argv: three args", &Scenario::new().args(["a", "b", "c"]));
}

#[test]
fn args_flag_like() {
    assert_same(
        "argv: flag-like args",
        &Scenario::new().args(["-h", "--help", "--version", "-", "--"]),
    );
}

#[test]
fn args_empty_and_odd_strings() {
    assert_same(
        "argv: empty and odd strings",
        &Scenario::new().args(["", " ", "\t", "new\nline", "quote\"s", "ünïcödé", "0", "-1"]),
    );
}

#[test]
fn args_many() {
    let many: Vec<String> = (0..1024).map(|i| format!("arg{i}")).collect();
    assert_same("argv: 1024 args", &Scenario::new().args(many));
}

// ---------------------------------------------------------------------------
// Environment: the message is plain ASCII from a literal, so no locale or env
// setting may change it.
// ---------------------------------------------------------------------------

#[test]
fn env_cleared() {
    assert_same("env: cleared", &Scenario::new().env(Env::Cleared));
}

#[test]
fn env_c_locale() {
    assert_same(
        "env: LC_ALL=C",
        &Scenario::new().env(Env::Only(&[("LC_ALL", "C"), ("LANG", "C")])),
    );
}

#[test]
fn env_utf8_locale() {
    assert_same(
        "env: tr_TR.UTF-8",
        &Scenario::new().env(Env::Only(&[
            ("LC_ALL", "tr_TR.UTF-8"),
            ("LANG", "tr_TR.UTF-8"),
            ("LC_NUMERIC", "de_DE.UTF-8"),
        ])),
    );
}

// ---------------------------------------------------------------------------
// Output descriptors: these decide whether the buffered flush at exit succeeds,
// fails silently, or terminates the process.
// ---------------------------------------------------------------------------

#[test]
fn stdout_redirected_to_regular_file() {
    let dir = std::env::temp_dir();
    let unique = std::process::id();
    let c_path = dir.join(format!("driver_c_{unique}.out"));
    let rust_path = dir.join(format!("driver_rust_{unique}.out"));
    let scenario = Scenario::new();
    let c = scenario.run_stdout_to_file(c_bin(), &c_path);
    let rust = scenario.run_stdout_to_file(&rust_bin(), &rust_path);
    compare("stdout: regular file", &c, &rust);
    assert_eq!(c.stdout, EXPECTED);
    let _ = std::fs::remove_file(&c_path);
    let _ = std::fs::remove_file(&rust_path);
}

#[test]
fn stdout_pipe_read_end_closed_before_write() {
    // The C program dies from SIGPIPE here; the Rust runtime ignores SIGPIPE by
    // default, so main.rs restores the default disposition to match.
    let scenario = Scenario::new();
    let c = scenario.run_with_broken_stdout(c_bin());
    let rust = scenario.run_with_broken_stdout(&rust_bin());
    compare("stdout: broken pipe", &c, &rust);
}

#[cfg(unix)]
#[test]
fn stdout_closed_descriptor() {
    // fd 1 closed outright: `printf` fails, but `main` still returns 0.
    let run = |bin: &Path| -> Outcome {
        let out = Command::new("sh")
            .arg("-c")
            .arg(format!("exec {} >&-", shell_quote(bin)))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("run via sh with fd 1 closed");
        outcome(out)
    };
    let c = run(c_bin());
    let rust = run(&rust_bin());
    compare("stdout: closed fd 1", &c, &rust);
}

#[cfg(unix)]
#[test]
fn stderr_closed_descriptor() {
    let run = |bin: &Path| -> Outcome {
        let out = Command::new("sh")
            .arg("-c")
            .arg(format!("exec {} 2>&-", shell_quote(bin)))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("run via sh with fd 2 closed");
        outcome(out)
    };
    let c = run(c_bin());
    let rust = run(&rust_bin());
    compare("stderr: closed fd 2", &c, &rust);
    assert_eq!(c.stdout, EXPECTED);
}

#[cfg(unix)]
#[test]
fn stdout_to_dev_full_write_error() {
    // /dev/full accepts opens and fails writes with ENOSPC. The C program does
    // not check printf's return value, so it still exits 0.
    if !Path::new("/dev/full").exists() {
        return;
    }
    let run = |bin: &Path| -> Outcome {
        let out = Command::new("sh")
            .arg("-c")
            .arg(format!("exec {} >/dev/full", shell_quote(bin)))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("run via sh with stdout on /dev/full");
        outcome(out)
    };
    let c = run(c_bin());
    let rust = run(&rust_bin());
    compare("stdout: /dev/full", &c, &rust);
}

#[cfg(unix)]
fn shell_quote(path: &Path) -> String {
    let s = path.to_string_lossy();
    format!("'{}'", s.replace('\'', "'\\''"))
}

// ---------------------------------------------------------------------------
// Working directory and invocation form.
// ---------------------------------------------------------------------------

#[test]
fn runs_from_a_different_working_directory() {
    let dir = std::env::temp_dir();
    let run = |bin: &Path| -> Outcome {
        let out = Command::new(bin)
            .current_dir(&dir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("run from temp dir");
        outcome(out)
    };
    let c = run(c_bin());
    let rust = run(&rust_bin());
    compare("cwd: temp dir", &c, &rust);
}

#[cfg(unix)]
#[test]
fn invoked_through_a_shell_with_stdin_from_heredoc() {
    // Closest thing to how the graders drive it: a shell pipeline feeding stdin.
    let run = |bin: &Path| -> Outcome {
        let out = Command::new("sh")
            .arg("-c")
            .arg(format!("printf '3\\n1 2 3\\n' | {}", shell_quote(bin)))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("run via shell pipeline");
        outcome(out)
    };
    let c = run(c_bin());
    let rust = run(&rust_bin());
    compare("shell: piped stdin", &c, &rust);
    assert_eq!(c.stdout, EXPECTED);
}
