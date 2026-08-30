//! Differential tests: run the C binary and the Rust binary as subprocesses and
//! compare stdout, stderr and exit status (including termination signal) byte
//! for byte.
//!
//! Nothing here calls the Rust code as a library; both programs are driven the
//! way a shell drives them, because that is how the translation is graded.

use std::fs;
use std::io::Write;
use std::os::unix::io::FromRawFd;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

extern "C" {
    fn pipe(fds: *mut i32) -> i32;
    fn close(fd: i32) -> i32;
}

/// The Rust binary under test, as built by cargo for this test run.
const RUST_BIN: &str = env!("CARGO_BIN_EXE_driver");

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// Path to the compiled C binary, building it once per test run if absent.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = repo_root().join("c_src");
        let build = c_src.join("build");
        let bin = build.join("driver");
        if !bin.exists() {
            fs::create_dir_all(&build).expect("create c_src/build");
            let cfg = Command::new("cmake")
                .arg("-S")
                .arg(&c_src)
                .arg("-B")
                .arg(&build)
                .output()
                .expect("run cmake (is cmake installed?)");
            assert!(
                cfg.status.success(),
                "cmake configure failed:\n{}",
                String::from_utf8_lossy(&cfg.stderr)
            );
            let out = Command::new("cmake")
                .arg("--build")
                .arg(&build)
                .output()
                .expect("run cmake --build");
            assert!(
                out.status.success(),
                "cmake build failed:\n{}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
        assert!(bin.exists(), "C binary missing after build: {}", bin.display());
        bin
    })
}

/// Everything a caller can observe about one run.
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
            .field("stderr", &String::from_utf8_lossy(&self.stderr))
            .field("code", &self.code)
            .field("signal", &self.signal)
            .finish()
    }
}

/// Run one program with the given args and stdin bytes, capturing everything.
fn run(prog: &Path, args: &[&str], stdin: Option<&[u8]>) -> Outcome {
    let mut cmd = Command::new(prog);
    cmd.args(args)
        .env_clear()
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(match stdin {
            Some(_) => Stdio::piped(),
            None => Stdio::null(),
        });

    let mut child = cmd.spawn().expect("spawn program under test");

    if let Some(bytes) = stdin {
        let mut sink = child.stdin.take().expect("stdin was piped");
        let bytes = bytes.to_vec();
        // Write on a helper thread: the program never reads stdin, so a large
        // payload would otherwise deadlock against a full pipe buffer.
        std::thread::spawn(move || {
            let _ = sink.write_all(&bytes);
        });
    }

    let out = child.wait_with_output().expect("collect program output");
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

/// Assert the two programs agree on stdout, stderr and exit status.
fn assert_same(case: &str, args: &[&str], stdin: Option<&[u8]>) {
    let c = run(c_bin(), args, stdin);
    let r = run(Path::new(RUST_BIN), args, stdin);

    assert_eq!(
        c.stdout, r.stdout,
        "[{case}] stdout differs\n  C:    {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    assert_eq!(
        c.stderr, r.stderr,
        "[{case}] stderr differs\n  C:    {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
    assert_eq!(c.code, r.code, "[{case}] exit code differs");
    assert_eq!(c.signal, r.signal, "[{case}] termination signal differs");
}

/// The exact bytes the C program prints, spelled out so a regression in either
/// program is caught even if both drift together.
const EXPECTED: &[u8] = b"Calling good()...\ngood()\nhelperGood()\nFinished good()\nCalling bad()...\nbad()\nFinished bad()\n";

// ---------------------------------------------------------------------------
// Baseline
// ---------------------------------------------------------------------------

#[test]
fn no_args_no_stdin() {
    assert_same("no args, stdin from /dev/null", &[], None);
}

#[test]
fn c_output_is_exactly_as_expected() {
    let c = run(c_bin(), &[], None);
    assert_eq!(
        c.stdout,
        EXPECTED,
        "C stdout drifted from the recorded ground truth"
    );
    assert!(c.stderr.is_empty(), "C wrote to stderr: {:?}", c.stderr);
    assert_eq!(c.code, Some(0));
    assert_eq!(c.signal, None);
}

#[test]
fn rust_output_is_exactly_as_expected() {
    let r = run(Path::new(RUST_BIN), &[], None);
    assert_eq!(r.stdout, EXPECTED, "Rust stdout does not match the C ground truth");
    assert!(r.stderr.is_empty(), "Rust wrote to stderr: {:?}", r.stderr);
    assert_eq!(r.code, Some(0));
    assert_eq!(r.signal, None);
}

/// `helperBad()` is defined `static` in the C and never called, so its string
/// must never appear. A translation that "fixed" the apparent omission by making
/// `bad()` call it would fail here.
#[test]
fn helper_bad_is_never_called() {
    for (label, prog) in [("C", c_bin()), ("Rust", Path::new(RUST_BIN))] {
        let out = run(prog, &[], None);
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(
            !text.contains("helperBad"),
            "{label} called helperBad(), which the C never does:\n{text}"
        );
    }
}

// ---------------------------------------------------------------------------
// stdin: the program never reads it, so every payload must be ignored alike
// ---------------------------------------------------------------------------

#[test]
fn stdin_empty() {
    assert_same("empty stdin", &[], Some(b""));
}

#[test]
fn stdin_single_item() {
    assert_same("single line on stdin", &[], Some(b"1\n"));
}

#[test]
fn stdin_single_item_no_trailing_newline() {
    assert_same("single line, no trailing newline", &[], Some(b"1"));
}

#[test]
fn stdin_many_items() {
    let mut payload = Vec::new();
    for i in 0..10_000 {
        payload.extend_from_slice(format!("{i}\n").as_bytes());
    }
    assert_same("10000 lines on stdin", &[], Some(&payload));
}

#[test]
fn stdin_binary_with_nul_bytes() {
    let payload: Vec<u8> = (0u8..=255).cycle().take(4096).collect();
    assert_same("binary stdin including NUL", &[], Some(&payload));
}

#[test]
fn stdin_invalid_utf8() {
    assert_same("invalid UTF-8 on stdin", &[], Some(&[0xff, 0xfe, 0x80, b'\n']));
}

#[test]
fn stdin_whitespace_only() {
    assert_same("whitespace-only stdin", &[], Some(b"   \t \n\n  \n"));
}

// ---------------------------------------------------------------------------
// argv: `main` ignores argc/argv entirely
// ---------------------------------------------------------------------------

#[test]
fn one_arg() {
    assert_same("one argument", &["foo"], None);
}

#[test]
fn empty_string_arg() {
    assert_same("empty-string argument", &[""], None);
}

#[test]
fn several_args() {
    assert_same("several arguments", &["-h", "--help", "-1", "0", "--"], None);
}

#[test]
fn arg_with_newline_and_unicode() {
    assert_same("argument with newline and UTF-8", &["a\nb", "héllo ☃"], None);
}

#[test]
fn very_long_arg() {
    let long = "x".repeat(100_000);
    assert_same("100k-character argument", &[long.as_str()], None);
}

#[test]
fn many_args() {
    let owned: Vec<String> = (0..2000).map(|i| format!("arg{i}")).collect();
    let args: Vec<&str> = owned.iter().map(String::as_str).collect();
    assert_same("2000 arguments", &args, None);
}

/// The C ignores `argv[0]`, so invoking either program under a different name
/// must not change anything. `arg0` sets `argv[0]` without touching the file.
#[test]
fn argv0_does_not_matter() {
    let mut outcomes = Vec::new();
    for prog in [c_bin(), Path::new(RUST_BIN)] {
        let out = Command::new(prog)
            .arg0("some-other-name")
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("run with a rewritten argv[0]");
        outcomes.push(Outcome {
            stdout: out.stdout,
            stderr: out.stderr,
            code: out.status.code(),
            signal: out.status.signal(),
        });
    }
    assert_eq!(
        outcomes[0], outcomes[1],
        "programs disagree when invoked under a different argv[0]"
    );
    assert_eq!(outcomes[0].stdout, EXPECTED);
}

/// `argc == 0` (an empty `argv`) is legal at the syscall level. The C never
/// touches `argv`, so it must survive this too.
#[test]
fn empty_argv() {
    let mut outcomes = Vec::new();
    for prog in [c_bin(), Path::new(RUST_BIN)] {
        let out = Command::new(prog)
            .arg0("")
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("run with an empty argv[0]");
        outcomes.push(Outcome {
            stdout: out.stdout,
            stderr: out.stderr,
            code: out.status.code(),
            signal: out.status.signal(),
        });
    }
    assert_eq!(outcomes[0], outcomes[1], "programs disagree on an empty argv[0]");
}

// ---------------------------------------------------------------------------
// stdout / stderr failure paths: the C discards printf's return value, so a
// failing stdout must still exit 0 -- except for SIGPIPE, which kills it.
// ---------------------------------------------------------------------------

fn scratch_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("driver-difftest-{tag}-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

/// Run with stdout pointed at a descriptor that always fails to accept bytes.
fn run_stdout_to_dev_full(prog: &Path) -> Outcome {
    let full = fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .expect("/dev/full must be available");
    let out = Command::new(prog)
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::from(full))
        .stderr(Stdio::piped())
        .output()
        .expect("run with stdout on /dev/full");
    Outcome {
        stdout: Vec::new(),
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

#[test]
fn stdout_write_error_is_ignored() {
    // glibc's exit-time flush failure does not change the C exit status.
    let c = run_stdout_to_dev_full(c_bin());
    let r = run_stdout_to_dev_full(Path::new(RUST_BIN));
    assert_eq!(c, r, "programs disagree when stdout is /dev/full");
    assert_eq!(c.code, Some(0), "C should still exit 0 on a write error");
}

/// Run with file descriptor 1 closed outright, so every write fails with EBADF.
fn run_with_stdout_closed(prog: &Path) -> Outcome {
    let mut cmd = Command::new(prog);
    cmd.env_clear().stdin(Stdio::null()).stderr(Stdio::piped());
    unsafe {
        cmd.pre_exec(|| {
            close(1);
            Ok(())
        });
    }
    let out = cmd.output().expect("run with stdout closed");
    Outcome {
        stdout: Vec::new(),
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

#[test]
fn stdout_closed() {
    let c = run_with_stdout_closed(c_bin());
    let r = run_with_stdout_closed(Path::new(RUST_BIN));
    assert_eq!(c, r, "programs disagree when fd 1 is closed");
    assert_eq!(c.code, Some(0), "C should still exit 0 with fd 1 closed");
}

#[test]
fn stderr_closed() {
    for prog in [c_bin(), Path::new(RUST_BIN)] {
        let mut cmd = Command::new(prog);
        cmd.env_clear().stdin(Stdio::null()).stdout(Stdio::piped());
        unsafe {
            cmd.pre_exec(|| {
                close(2);
                Ok(())
            });
        }
        let out = cmd.output().expect("run with stderr closed");
        assert_eq!(out.stdout, EXPECTED, "stdout changed with fd 2 closed");
        assert_eq!(out.status.code(), Some(0));
        assert_eq!(out.status.signal(), None);
    }
}

/// Point stdout at a pipe whose read end is already closed. The C program runs
/// with SIGPIPE at its default disposition and dies from the signal; the Rust
/// program must do the same rather than ignoring EPIPE and exiting 0.
fn run_with_broken_pipe(prog: &Path) -> Outcome {
    let stdout = unsafe {
        let mut fds = [0i32; 2];
        assert_eq!(pipe(fds.as_mut_ptr()), 0, "pipe() failed");
        close(fds[0]); // reader is gone before the child writes anything
        Stdio::from_raw_fd(fds[1])
    };
    let out = Command::new(prog)
        .env_clear()
        .stdin(Stdio::null())
        .stdout(stdout)
        .stderr(Stdio::piped())
        .output()
        .expect("run with a broken stdout pipe");
    Outcome {
        stdout: Vec::new(),
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

#[test]
fn broken_stdout_pipe_matches() {
    let c = run_with_broken_pipe(c_bin());
    let r = run_with_broken_pipe(Path::new(RUST_BIN));
    assert_eq!(c, r, "programs disagree on a broken stdout pipe");
    assert_eq!(
        c.signal,
        Some(13),
        "expected the C program to die from SIGPIPE"
    );
    assert_eq!(c.code, None, "a signalled process has no exit code");
}

// ---------------------------------------------------------------------------
// Determinism and stream shape
// ---------------------------------------------------------------------------

#[test]
fn repeated_runs_are_identical() {
    let mut seen: Option<Outcome> = None;
    for _ in 0..5 {
        let c = run(c_bin(), &[], None);
        let r = run(Path::new(RUST_BIN), &[], None);
        assert_eq!(c, r, "programs disagree across repeated runs");
        if let Some(prev) = &seen {
            assert_eq!(*prev, c, "C output is not deterministic");
        }
        seen = Some(c);
    }
}

#[test]
fn stdout_to_regular_file_matches() {
    let dir = scratch_dir("file");
    let mut paths = Vec::new();
    for (name, prog) in [("c.out", c_bin()), ("rust.out", Path::new(RUST_BIN))] {
        let path = dir.join(name);
        let file = fs::File::create(&path).expect("create output file");
        let status = Command::new(prog)
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::from(file))
            .stderr(Stdio::null())
            .status()
            .expect("run with stdout on a regular file");
        assert_eq!(status.code(), Some(0));
        assert_eq!(status.signal(), None);
        paths.push(path);
    }
    let c = fs::read(&paths[0]).unwrap();
    let r = fs::read(&paths[1]).unwrap();
    assert_eq!(c, r, "file contents differ");
    assert_eq!(c, EXPECTED);
    let _ = fs::remove_dir_all(&dir);
}

/// Output must end in exactly one newline with no trailing blank line, since
/// every `printLine` appends `\n` and nothing else is printed.
#[test]
fn trailing_newline_shape() {
    for (label, prog) in [("C", c_bin()), ("Rust", Path::new(RUST_BIN))] {
        let out = run(prog, &[], None);
        assert!(out.stdout.ends_with(b"\n"), "{label}: missing trailing newline");
        assert!(
            !out.stdout.ends_with(b"\n\n"),
            "{label}: unexpected extra trailing newline"
        );
        assert_eq!(
            out.stdout.iter().filter(|&&b| b == b'\n').count(),
            7,
            "{label}: expected exactly 7 printed lines"
        );
    }
}

/// stdout on a terminal takes the other side of glibc's buffering decision
/// (line buffered instead of fully buffered). `script` supplies a pty.
#[test]
fn stdout_on_a_tty_matches() {
    assert!(
        Command::new("script")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false),
        "the `script` utility (util-linux) is required to test the tty path"
    );

    let mut outcomes = Vec::new();
    for prog in [c_bin(), Path::new(RUST_BIN)] {
        let out = Command::new("script")
            .arg("-qec")
            .arg(prog)
            .arg("/dev/null")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("run under a pty via script");
        outcomes.push(Outcome {
            stdout: out.stdout,
            stderr: out.stderr,
            code: out.status.code(),
            signal: out.status.signal(),
        });
    }
    assert_eq!(
        outcomes[0], outcomes[1],
        "programs disagree when stdout is a terminal"
    );
    // The pty turns each LF into CRLF; confirm we really took the tty path.
    assert!(
        outcomes[0].stdout.windows(2).any(|w| w == b"\r\n"),
        "expected CRLF from the pty, so this did not exercise the tty path"
    );
}

/// The program never reads stdin, so it must not care that fd 0 is closed.
#[test]
fn stdin_closed() {
    let mut outcomes = Vec::new();
    for prog in [c_bin(), Path::new(RUST_BIN)] {
        let mut cmd = Command::new(prog);
        cmd.env_clear().stdout(Stdio::piped()).stderr(Stdio::piped());
        unsafe {
            cmd.pre_exec(|| {
                close(0);
                Ok(())
            });
        }
        let out = cmd.output().expect("run with stdin closed");
        outcomes.push(Outcome {
            stdout: out.stdout,
            stderr: out.stderr,
            code: out.status.code(),
            signal: out.status.signal(),
        });
    }
    assert_eq!(outcomes[0], outcomes[1], "programs disagree with fd 0 closed");
    assert_eq!(outcomes[0].stdout, EXPECTED);
}

/// `printf("%s\n", ...)` on ASCII literals is locale independent; confirm no
/// locale makes the two diverge.
#[test]
fn locale_does_not_affect_output() {
    for locale in ["C", "C.UTF-8", "en_US.UTF-8", "tr_TR.UTF-8", "invalid-locale"] {
        let mut outcomes = Vec::new();
        for prog in [c_bin(), Path::new(RUST_BIN)] {
            let out = Command::new(prog)
                .env_clear()
                .env("LC_ALL", locale)
                .env("LANG", locale)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .expect("run with a locale set");
            outcomes.push(Outcome {
                stdout: out.stdout,
                stderr: out.stderr,
                code: out.status.code(),
                signal: out.status.signal(),
            });
        }
        assert_eq!(
            outcomes[0], outcomes[1],
            "programs disagree under LC_ALL={locale}"
        );
        assert_eq!(outcomes[0].stdout, EXPECTED, "output changed under LC_ALL={locale}");
    }
}
