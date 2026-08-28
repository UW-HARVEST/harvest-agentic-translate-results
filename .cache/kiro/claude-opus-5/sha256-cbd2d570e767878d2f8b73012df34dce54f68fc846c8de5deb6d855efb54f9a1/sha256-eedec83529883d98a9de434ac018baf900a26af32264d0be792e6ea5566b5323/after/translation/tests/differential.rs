//! Differential tests: run the C binary and the Rust binary as subprocesses and
//! compare stdout (byte for byte), stderr (byte for byte) and exit status.
//!
//! The Rust code is never called as a library here. Both programs are driven the
//! way a shell drives them, because that is how they are compared.
//!
//! The C binary must be built first:
//!     cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .

use std::io::Write;
use std::os::unix::process::CommandExt;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// The Rust binary under test, as built by cargo for this test run.
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// The reference C binary, built by CMake into `c_src/build/driver`.
fn c_bin() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.push("c_src");
    p.push("build");
    p.push("driver");
    assert!(
        p.is_file(),
        "reference C binary not found at {}\nbuild it with:\n  cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .",
        p.display()
    );
    p
}

/// Everything observable about one run of a program.
#[derive(Debug, PartialEq, Eq)]
struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// Normal exit code, if the process exited normally.
    code: Option<i32>,
    /// Terminating signal, if the process was killed by one.
    signal: Option<i32>,
}

/// Spawn `bin` with `args`, write `stdin_bytes` to its stdin, and collect
/// stdout, stderr and the exit status. `stdin_bytes == None` means stdin is
/// /dev/null.
fn run(bin: &PathBuf, args: &[&str], stdin_bytes: Option<&[u8]>) -> Run {
    let mut cmd = Command::new(bin);
    cmd.args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(match stdin_bytes {
            Some(_) => Stdio::piped(),
            None => Stdio::null(),
        });

    let mut child = cmd.spawn().expect("failed to spawn program");

    if let Some(bytes) = stdin_bytes {
        let mut sink = child.stdin.take().expect("stdin was piped");
        // The program under test may never read stdin, so a full pipe would
        // block us forever. Write from a helper thread and ignore EPIPE.
        let bytes = bytes.to_vec();
        std::thread::spawn(move || {
            let _ = sink.write_all(&bytes);
            let _ = sink.flush();
        });
    }

    let out = child.wait_with_output().expect("failed to wait for program");
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

/// Assert the C and Rust programs agree on stdout, stderr and exit status for
/// one input, and return the shared observation.
fn assert_same(label: &str, args: &[&str], stdin_bytes: Option<&[u8]>) -> Run {
    let c = run(&c_bin(), args, stdin_bytes);
    let r = run(&rust_bin(), args, stdin_bytes);

    assert_eq!(
        c.stdout,
        r.stdout,
        "[{label}] stdout differs\n  C:    {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "[{label}] stderr differs\n  C:    {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "[{label}] exit status differs (code, signal)"
    );
    c
}

/// Run a bash snippet twice, with `$BIN` bound to the C binary and then to the
/// Rust binary, and compare stdout, stderr and exit status of the whole snippet.
/// Used for conditions a plain spawn cannot express: closed descriptors,
/// pipelines, redirections to files.
fn assert_same_shell(label: &str, snippet: &str) {
    let run_snippet = |bin: &PathBuf, tag: &str| -> Run {
        let out = Command::new("bash")
            .arg("-c")
            .arg(snippet)
            .env("BIN", bin)
            .env("TAG", tag)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("failed to run bash");
        Run {
            stdout: out.stdout,
            stderr: out.stderr,
            code: out.status.code(),
            signal: out.status.signal(),
        }
    };

    let c = run_snippet(&c_bin(), "c");
    let r = run_snippet(&rust_bin(), "rust");

    assert_eq!(
        c.stdout,
        r.stdout,
        "[{label}] shell stdout differs\n  snippet: {snippet}\n  C:    {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "[{label}] shell stderr differs\n  snippet: {snippet}\n  C:    {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "[{label}] shell exit status differs\n  snippet: {snippet}"
    );
}

// ---------------------------------------------------------------------------
// The program's only behavior: printf("Hello World!\n"); return 0;
// main ignores argc/argv and never reads stdin, so the input classes are the
// ways the process can be invoked and the states its descriptors can be in.
// ---------------------------------------------------------------------------

#[test]
fn no_args_no_stdin() {
    let out = assert_same("no args, stdin=/dev/null", &[], None);
    // Pin the exact bytes so a change in either program is caught, not just
    // agreement between two equally-wrong programs.
    assert_eq!(out.stdout, b"Hello World!\n");
    assert_eq!(out.stdout.len(), 13, "no extra or missing trailing bytes");
    assert!(out.stderr.is_empty(), "nothing is written to stderr");
    assert_eq!(out.code, Some(0));
    assert_eq!(out.signal, None);
}

#[test]
fn empty_stdin() {
    assert_same("empty stdin", &[], Some(b""));
}

#[test]
fn single_line_on_stdin() {
    // stdin is never read; it must not change the output.
    assert_same("one line on stdin", &[], Some(b"1\n"));
}

#[test]
fn stdin_line_without_trailing_newline() {
    assert_same("stdin without trailing newline", &[], Some(b"hello"));
}

#[test]
fn stdin_many_lines() {
    let mut input = Vec::new();
    for i in 0..1000 {
        input.extend_from_slice(format!("{i}\n").as_bytes());
    }
    assert_same("1000 lines on stdin", &[], Some(&input));
}

#[test]
fn stdin_larger_than_a_pipe_buffer() {
    // 1 MiB: far more than the 64 KiB pipe buffer. Neither program reads it, so
    // both must still finish rather than deadlock, with identical output.
    let input = vec![b'x'; 1024 * 1024];
    assert_same("1 MiB on stdin", &[], Some(&input));
}

#[test]
fn stdin_non_utf8_bytes() {
    let input: Vec<u8> = vec![0x00, 0xff, 0xfe, 0x80, 0x0a, 0xc3, 0x28];
    assert_same("invalid UTF-8 on stdin", &[], Some(&input));
}

#[test]
fn one_argument() {
    assert_same("one argv entry", &["foo"], None);
}

#[test]
fn many_arguments() {
    let args: Vec<String> = (0..64).map(|i| format!("arg{i}")).collect();
    let args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    assert_same("64 argv entries", &args, None);
}

#[test]
fn arguments_that_look_like_flags_and_have_odd_bytes() {
    assert_same(
        "flag-ish and odd arguments",
        &["--help", "-n", "", " ", "with space", "tab\there", "ünïcødé"],
        None,
    );
}

#[test]
fn empty_environment() {
    let run_with_empty_env = |bin: PathBuf| -> Run {
        let out = Command::new(bin)
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("failed to run with empty environment");
        Run {
            stdout: out.stdout,
            stderr: out.stderr,
            code: out.status.code(),
            signal: out.status.signal(),
        }
    };
    assert_eq!(
        run_with_empty_env(c_bin()),
        run_with_empty_env(rust_bin()),
        "output differs when the environment is empty"
    );
}

#[test]
fn unusual_argv0() {
    // argv[0] is not used by the program; setting it to something odd (or to
    // the empty string) must not change anything.
    let run_with_arg0 = |bin: PathBuf, arg0: &str| -> Run {
        let out = Command::new(bin)
            .arg0(arg0)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("failed to run with custom argv[0]");
        Run {
            stdout: out.stdout,
            stderr: out.stderr,
            code: out.status.code(),
            signal: out.status.signal(),
        }
    };
    for arg0 in ["", "not-driver", "/nonexistent/path"] {
        assert_eq!(
            run_with_arg0(c_bin(), arg0),
            run_with_arg0(rust_bin(), arg0),
            "output differs for argv[0] = {arg0:?}"
        );
    }
}

#[test]
fn repeated_runs_are_deterministic() {
    let first = assert_same("run 1", &[], None);
    for i in 2..=5 {
        let next = assert_same(&format!("run {i}"), &[], None);
        assert_eq!(first, next, "run {i} differs from run 1");
    }
}

// --- descriptor states, expressed through the shell -----------------------

#[test]
fn stdout_redirected_to_a_file() {
    assert_same_shell(
        "stdout to a regular file",
        r#"f=$(mktemp); "$BIN" > "$f"; s=$?; cat "$f"; rm -f "$f"; echo "status=$s""#,
    );
}

#[test]
fn stdout_appended_to_a_nonempty_file() {
    assert_same_shell(
        "stdout appended to a non-empty file",
        r#"f=$(mktemp); printf 'PRE' > "$f"; "$BIN" >> "$f"; s=$?; cat "$f"; rm -f "$f"; echo "status=$s""#,
    );
}

#[test]
fn stdout_to_dev_null() {
    assert_same_shell(
        "stdout to /dev/null",
        r#""$BIN" > /dev/null; echo "status=$?""#,
    );
}

#[test]
fn stdout_closed() {
    // fd 1 is closed: the write fails. The C program ignores printf's return
    // value and still returns 0.
    assert_same_shell("stdout closed", r#""$BIN" >&-; echo "status=$?""#);
}

#[test]
fn stderr_closed() {
    assert_same_shell("stderr closed", r#""$BIN" 2>&-; echo "status=$?""#);
}

#[test]
fn both_stdout_and_stderr_closed() {
    assert_same_shell(
        "stdout and stderr closed",
        r#""$BIN" >&- 2>&-; echo "status=$?""#,
    );
}

#[test]
fn stdin_closed() {
    assert_same_shell("stdin closed", r#""$BIN" <&-; echo "status=$?""#);
}

#[test]
fn stdout_merged_into_stderr() {
    assert_same_shell(
        "stdout merged into stderr",
        r#""$BIN" 1>&2; echo "status=$?""#,
    );
}

#[test]
fn piped_to_a_reader_that_consumes_everything() {
    assert_same_shell(
        "piped into cat",
        r#""$BIN" | cat; echo "status=${PIPESTATUS[0]}/$?""#,
    );
}

#[test]
fn piped_to_a_reader_that_exits_without_reading() {
    // The read end closes before the write lands, so the write gets EPIPE.
    // C leaves SIGPIPE at its default and is killed by signal 13, which the
    // shell reports as 141. Rust's runtime ignores SIGPIPE unless the default
    // disposition is restored, which would make it exit 0 instead.
    assert_same_shell(
        "piped into a reader that never reads",
        r#""$BIN" | true; echo "status=${PIPESTATUS[0]}""#,
    );
}

#[test]
fn piped_to_a_reader_that_closes_after_a_delay() {
    assert_same_shell(
        "piped into a reader that sleeps then exits",
        r#""$BIN" | (exec sleep 0.2); echo "status=${PIPESTATUS[0]}""#,
    );
}

#[test]
fn run_from_a_different_working_directory() {
    assert_same_shell(
        "run from /tmp",
        r#"cd /tmp && "$BIN"; echo "status=$?""#,
    );
}
