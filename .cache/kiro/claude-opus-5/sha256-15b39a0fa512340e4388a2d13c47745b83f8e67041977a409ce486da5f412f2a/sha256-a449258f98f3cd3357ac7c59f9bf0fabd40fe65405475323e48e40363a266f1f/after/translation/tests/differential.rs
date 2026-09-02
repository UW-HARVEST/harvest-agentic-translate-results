//! Differential tests: run the C `driver` and the Rust `driver` as
//! subprocesses and compare stdout, stderr and exit status byte for byte.
//!
//! Nothing here links against the translation as a library. Both programs are
//! driven exactly the way a shell drives them, because that is how they are
//! compared.
//!
//! The C reference is built on demand from `../c_src` with CMake, into a
//! throwaway build directory so `c_src` itself is never modified.

use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Locating and building the two binaries
// ---------------------------------------------------------------------------

/// Path to the Rust binary under test. Cargo builds it for us and hands over
/// the path, so this is always the binary matching the current sources.
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is `<root>/translation`.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Build the C reference program once per test binary run and return its path.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();

    C_BIN.get_or_init(|| {
        let root = workspace_root();
        let c_src = root.join("c_src");
        assert!(
            c_src.join("CMakeLists.txt").is_file(),
            "expected the C sources at {}",
            c_src.display()
        );

        // A dedicated build directory keeps this out of the way of any build
        // the developer already has in `c_src/build`, and guarantees we never
        // write into a tracked location.
        let build_dir = root.join("target-c-reference");
        std::fs::create_dir_all(&build_dir).expect("failed to create the C build directory");

        let configure = Command::new("cmake")
            .arg("-S")
            .arg(&c_src)
            .arg("-B")
            .arg(&build_dir)
            .output()
            .expect("failed to run `cmake` — is CMake installed?");
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

        let bin = build_dir.join("driver");
        assert!(
            bin.is_file(),
            "the C build did not produce {}",
            bin.display()
        );
        bin
    })
}

// ---------------------------------------------------------------------------
// Running one program and capturing everything observable
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Some(code)` for a normal exit, `None` when killed by a signal.
    code: Option<i32>,
    /// `Some(signal)` when killed by a signal, `None` otherwise.
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

/// Run `bin` with the given argv tail and stdin bytes, capturing everything.
fn run(bin: &Path, args: &[&str], stdin_bytes: &[u8]) -> Outcome {
    let mut child = Command::new(bin)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("LC_ALL", "C")
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    {
        let mut stdin = child.stdin.take().expect("stdin was piped");
        // The programs never read stdin, so the write may fail with EPIPE once
        // the child has already exited. That is not a test failure.
        let _ = stdin.write_all(stdin_bytes);
        let _ = stdin.flush();
    }

    let out = child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("failed to wait on {}: {e}", bin.display()));

    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

/// Assert the C and Rust programs are indistinguishable for one input.
fn assert_same(case: &str, args: &[&str], stdin_bytes: &[u8]) {
    let c = run(c_bin(), args, stdin_bytes);
    let r = run(&rust_bin(), args, stdin_bytes);

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
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "[{case}] exit status differs\n  C:    {c:?}\n  Rust: {r:?}"
    );
}

// ---------------------------------------------------------------------------
// Phase A — both programs exist and run
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_build_and_run() {
    let c = run(c_bin(), &[], b"");
    let r = run(&rust_bin(), &[], b"");
    assert_eq!(c.code, Some(0), "C reference did not exit 0: {c:?}");
    assert_eq!(r.code, Some(0), "Rust program did not exit 0: {r:?}");
}

// ---------------------------------------------------------------------------
// Phase B — the input classes the C program distinguishes
//
// `helloworld()` reads nothing: no scanf, no fgets, no getchar, no argv use.
// The only observable behaviour is the fixed 13-byte stdout write and the
// return value 0. The input classes are therefore about what the process is
// handed, and each must leave that behaviour unchanged in both programs.
// ---------------------------------------------------------------------------

#[test]
fn no_input_at_all() {
    assert_same("empty stdin, no args", &[], b"");
}

/// The exact bytes are pinned so a silent change to the format string in either
/// program is caught, not just a matching change in both.
#[test]
fn output_is_exactly_hello_world_newline() {
    let c = run(c_bin(), &[], b"");
    assert_eq!(
        c.stdout, b"Hello World!\n",
        "the C reference output changed; the expectation below needs revisiting"
    );

    let r = run(&rust_bin(), &[], b"");
    assert_eq!(r.stdout, b"Hello World!\n");
    assert_eq!(r.stdout.len(), 13);
    assert!(r.stderr.is_empty(), "nothing should be written to stderr");
    assert_eq!(r.code, Some(0));
}

#[test]
fn single_line_of_stdin_is_ignored() {
    assert_same("one line on stdin", &[], b"1\n");
}

#[test]
fn single_item_without_trailing_newline() {
    // `scanf` would consume this, `fgets` would too; neither is called, so the
    // absence of a trailing newline must change nothing.
    assert_same("no trailing newline", &[], b"42");
}

#[test]
fn many_lines_of_stdin_are_ignored() {
    let mut input = Vec::new();
    for i in 0..1000 {
        input.extend_from_slice(format!("{i}\n").as_bytes());
    }
    assert_same("1000 lines on stdin", &[], &input);
}

#[test]
fn stdin_larger_than_a_pipe_buffer() {
    // 256 KiB is past the usual 64 KiB pipe capacity, so the writing side
    // blocks and hits EPIPE once the child exits without reading. Both programs
    // must still exit cleanly rather than being affected by the unread input.
    let input = vec![b'x'; 256 * 1024];
    assert_same("256 KiB of unread stdin", &[], &input);
}

#[test]
fn binary_and_non_utf8_stdin() {
    let input: Vec<u8> = vec![
        0x00, 0xff, 0xfe, 0x80, 0x0a, 0x7f, 0x1b, 0x5b, 0x41, 0xc3, 0x28, 0x0a,
    ];
    assert_same("invalid UTF-8 and NUL bytes on stdin", &[], &input);
}

#[test]
fn stdin_is_a_closed_pipe_immediately() {
    // Spawn with stdin piped and drop the writer before waiting, so the child
    // sees stdin at EOF from the start.
    fn run_eof_stdin(bin: &Path) -> Outcome {
        let mut child = Command::new(bin)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn");
        drop(child.stdin.take());
        let out = child.wait_with_output().expect("wait");
        Outcome {
            stdout: out.stdout,
            stderr: out.stderr,
            code: out.status.code(),
            signal: out.status.signal(),
        }
    }

    let c = run_eof_stdin(c_bin());
    let r = run_eof_stdin(&rust_bin());
    assert_eq!(c.stdout, r.stdout, "stdout differs\n C: {c:?}\n R: {r:?}");
    assert_eq!(c.stderr, r.stderr, "stderr differs\n C: {c:?}\n R: {r:?}");
    assert_eq!((c.code, c.signal), (r.code, r.signal), "status differs");
}

// ---------------------------------------------------------------------------
// Phase C — paths not covered above
//
// `int main()` in C declares no parameters and never touches argv, so argv is
// an input class that must be inert. `printf`'s return value is discarded, so
// every way of making the write fail must still produce exit status 0 — except
// the signal case, which must produce a signal.
// ---------------------------------------------------------------------------

#[test]
fn arguments_are_ignored() {
    assert_same("one argument", &["ignored"], b"");
    assert_same("several arguments", &["-h", "--help", "12", "-1"], b"");
    assert_same("empty-string argument", &[""], b"");
    assert_same("argument that looks like a flag", &["--version"], b"");
}

#[test]
fn many_arguments_are_ignored() {
    let owned: Vec<String> = (0..500).map(|i| i.to_string()).collect();
    let args: Vec<&str> = owned.iter().map(String::as_str).collect();
    assert_same("500 arguments", &args, b"");
}

#[test]
fn arguments_with_non_ascii_and_spaces() {
    assert_same("odd arguments", &["a b\tc", "ünïcødé", "*", "$HOME", "\\n"], b"");
}

/// `printf` writing to a descriptor that is not open fails and sets the stream's
/// error flag, but the C code never checks it and still returns 0. The Rust
/// translation discards the `write_all`/`flush` results for the same reason, so
/// both must exit 0 with no output and nothing on stderr.
#[test]
fn stdout_closed_before_exec() {
    fn run_with_closed_stdout(bin: &Path) -> Outcome {
        let out = Command::new("sh")
            .arg("-c")
            .arg(format!("exec {} >&-", shell_quote(bin)))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("failed to run sh");
        Outcome {
            stdout: out.stdout,
            stderr: out.stderr,
            code: out.status.code(),
            signal: out.status.signal(),
        }
    }

    let c = run_with_closed_stdout(c_bin());
    let r = run_with_closed_stdout(&rust_bin());
    assert_eq!(c.stdout, r.stdout, "stdout differs\n C: {c:?}\n R: {r:?}");
    assert_eq!((c.code, c.signal), (r.code, r.signal), "status differs\n C: {c:?}\n R: {r:?}");
    assert_eq!(c.code, Some(0), "the write failure must not change the status");
}

/// stdout pointing at `/dev/full`: the write itself fails with ENOSPC. The C
/// code ignores that and returns 0.
#[test]
fn stdout_to_dev_full() {
    fn run_to_dev_full(bin: &Path) -> Outcome {
        let out = Command::new("sh")
            .arg("-c")
            .arg(format!("exec {} >/dev/full", shell_quote(bin)))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("failed to run sh");
        Outcome {
            stdout: out.stdout,
            stderr: out.stderr,
            code: out.status.code(),
            signal: out.status.signal(),
        }
    }

    // /dev/full exists on every Linux system; assert rather than skip, so this
    // case cannot quietly stop being exercised.
    assert!(
        Path::new("/dev/full").exists(),
        "/dev/full is missing, so the write-failure path cannot be tested"
    );

    let c = run_to_dev_full(c_bin());
    let r = run_to_dev_full(&rust_bin());
    assert_eq!(c.stdout, r.stdout, "stdout differs\n C: {c:?}\n R: {r:?}");
    assert_eq!(c.stderr, r.stderr, "stderr differs\n C: {c:?}\n R: {r:?}");
    assert_eq!((c.code, c.signal), (r.code, r.signal), "status differs\n C: {c:?}\n R: {r:?}");
}

/// stdout redirected to a regular file rather than a pipe. C stdio picks full
/// buffering here instead of the pipe's, so this checks the bytes still land
/// (i.e. the stream is flushed) in both programs.
#[test]
fn stdout_redirected_to_a_file() {
    fn run_to_file(bin: &Path, path: &Path) -> (Outcome, Vec<u8>) {
        let out = Command::new("sh")
            .arg("-c")
            .arg(format!(
                "exec {} >{}",
                shell_quote(bin),
                shell_quote(path)
            ))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("failed to run sh");
        let contents = std::fs::read(path).expect("failed to read the redirect target");
        (
            Outcome {
                stdout: out.stdout,
                stderr: out.stderr,
                code: out.status.code(),
                signal: out.status.signal(),
            },
            contents,
        )
    }

    let dir = std::env::temp_dir();
    let c_path = dir.join(format!("driver_c_{}.out", std::process::id()));
    let r_path = dir.join(format!("driver_r_{}.out", std::process::id()));

    let (c, c_file) = run_to_file(c_bin(), &c_path);
    let (r, r_file) = run_to_file(&rust_bin(), &r_path);

    let _ = std::fs::remove_file(&c_path);
    let _ = std::fs::remove_file(&r_path);

    assert_eq!(c_file, r_file, "file contents differ");
    assert_eq!(c_file, b"Hello World!\n");
    assert_eq!(c.stderr, r.stderr);
    assert_eq!((c.code, c.signal), (r.code, r.signal));
}

/// The reader closes the pipe before the program writes. A C program runs with
/// `SIGPIPE` at `SIG_DFL`, so it is killed by signal 13. The Rust runtime sets
/// `SIGPIPE` to `SIG_IGN` before `main`, which would turn this into exit 0
/// unless the translation restores the default — see `restore_default_sigpipe`
/// in `src/main.rs`.
#[test]
fn broken_pipe_kills_the_process_with_sigpipe() {
    fn run_with_broken_pipe(bin: &Path) -> Outcome {
        // `sleep` first so the read end is definitely closed before the program
        // writes; `exec` makes the program itself the process whose wait status
        // the shell reports.
        let mut child = Command::new("sh")
            .arg("-c")
            .arg(format!("sleep 1; exec {}", shell_quote(bin)))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to spawn sh");

        // Close the read end of the stdout pipe while the child is sleeping.
        drop(child.stdout.take());

        let out = child.wait_with_output().expect("failed to wait on sh");
        Outcome {
            stdout: Vec::new(),
            stderr: out.stderr,
            code: out.status.code(),
            signal: out.status.signal(),
        }
    }

    let c = run_with_broken_pipe(c_bin());
    let r = run_with_broken_pipe(&rust_bin());

    assert_eq!(
        c.signal,
        Some(13),
        "expected the C reference to die from SIGPIPE, got {c:?}"
    );
    assert_eq!(c.stderr, r.stderr, "stderr differs\n C: {c:?}\n R: {r:?}");
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "exit status differs on a broken pipe\n  C:    {c:?}\n  Rust: {r:?}"
    );
}

/// Running the program repeatedly must be deterministic: same bytes, same
/// status, every time, with no dependence on the environment.
#[test]
fn output_is_deterministic_across_runs() {
    let mut seen: Option<Outcome> = None;
    for i in 0..25 {
        let c = run(c_bin(), &[], b"");
        let r = run(&rust_bin(), &[], b"");
        assert_eq!(c.stdout, r.stdout, "run {i}: stdout differs");
        assert_eq!(c.stderr, r.stderr, "run {i}: stderr differs");
        assert_eq!((c.code, c.signal), (r.code, r.signal), "run {i}: status differs");
        match &seen {
            None => seen = Some(r),
            Some(first) => assert!(*first == r, "run {i} differs from the first run"),
        }
    }
}

/// With no environment variables at all the behaviour must be unchanged; the C
/// program never calls `getenv`.
#[test]
fn empty_environment() {
    fn run_bare_env(bin: &Path) -> Outcome {
        let out = Command::new(bin)
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("failed to run the program with an empty environment");
        Outcome {
            stdout: out.stdout,
            stderr: out.stderr,
            code: out.status.code(),
            signal: out.status.signal(),
        }
    }

    let c = run_bare_env(c_bin());
    let r = run_bare_env(&rust_bin());
    assert_eq!(c.stdout, r.stdout);
    assert_eq!(c.stderr, r.stderr);
    assert_eq!((c.code, c.signal), (r.code, r.signal));
}

/// A locale that would change numeric or message formatting in a C program must
/// not change this one, since the format string has no conversions.
#[test]
fn locale_does_not_change_output() {
    for locale in ["C", "POSIX", "en_US.UTF-8", "de_DE.UTF-8", "tr_TR.UTF-8", ""] {
        fn run_with_locale(bin: &Path, locale: &str) -> Outcome {
            let out = Command::new(bin)
                .env("LC_ALL", locale)
                .env("LANG", locale)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .expect("failed to run the program");
            Outcome {
                stdout: out.stdout,
                stderr: out.stderr,
                code: out.status.code(),
                signal: out.status.signal(),
            }
        }

        let c = run_with_locale(c_bin(), locale);
        let r = run_with_locale(&rust_bin(), locale);
        assert_eq!(c.stdout, r.stdout, "stdout differs under LC_ALL={locale:?}");
        assert_eq!(c.stderr, r.stderr, "stderr differs under LC_ALL={locale:?}");
        assert_eq!(
            (c.code, c.signal),
            (r.code, r.signal),
            "status differs under LC_ALL={locale:?}"
        );
    }
}

/// stdout and stderr sharing one descriptor (`2>&1`): the combined stream must
/// be identical, which pins down that nothing is written to stderr and that the
/// ordering of writes matches.
#[test]
fn stdout_and_stderr_merged() {
    fn run_merged(bin: &Path) -> Outcome {
        let out = Command::new("sh")
            .arg("-c")
            .arg(format!("exec {} 2>&1", shell_quote(bin)))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("failed to run sh");
        Outcome {
            stdout: out.stdout,
            stderr: out.stderr,
            code: out.status.code(),
            signal: out.status.signal(),
        }
    }

    let c = run_merged(c_bin());
    let r = run_merged(&rust_bin());
    assert_eq!(c.stdout, r.stdout, "merged output differs\n C: {c:?}\n R: {r:?}");
    assert_eq!(c.stdout, b"Hello World!\n");
    assert_eq!((c.code, c.signal), (r.code, r.signal));
}

/// Single-quote a path for `sh -c`, so a path containing spaces or shell
/// metacharacters cannot change the meaning of the command.
fn shell_quote(path: &Path) -> String {
    let s = path.to_string_lossy();
    format!("'{}'", s.replace('\'', r"'\''"))
}
