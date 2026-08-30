// Differential tests: run the ORIGINAL C binary and the Rust binary as
// subprocesses with identical inputs/environments and require byte-identical
// stdout, byte-identical stderr, and an identical exit status (including death
// by signal).
//
// The Rust code is never linked as a library here; both programs are driven
// exactly the way a shell would drive them.

use std::ffi::OsStr;
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::io::{FromRawFd, OwnedFd};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

// ---------------------------------------------------------------------------
// Locating the two binaries
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn c_binary() -> PathBuf {
    let p = workspace_root().join("c_src/build/driver");
    assert!(
        p.is_file(),
        "C binary missing at {}. Build it first:\n  cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .",
        p.display()
    );
    p
}

fn rust_binary() -> PathBuf {
    // Prefer the binary next to the test executable so we test what cargo built.
    let mut dir = std::env::current_exe().expect("current_exe");
    dir.pop(); // deps/
    if dir.ends_with("deps") {
        dir.pop();
    }
    let candidate = dir.join("driver");
    if candidate.is_file() {
        return candidate;
    }
    let fallback = workspace_root().join("translation/target/release/driver");
    assert!(
        fallback.is_file(),
        "Rust binary missing; run `cargo build --release` in translation/"
    );
    fallback
}

// ---------------------------------------------------------------------------
// Running a program and capturing everything that is compared
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// Exit code, or None when the process was killed by a signal.
    code: Option<i32>,
    /// Terminating signal, if any (e.g. 13 for SIGPIPE).
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

/// How the child's stdin should be wired up.
enum In<'a> {
    Empty,
    Bytes(&'a [u8]),
}

/// Run `prog` with the given args/stdin, capturing stdout and stderr.
fn run(prog: &Path, args: &[&OsStr], stdin: &In, envs: &[(&str, &str)], cwd: &Path) -> Outcome {
    use std::os::unix::process::ExitStatusExt;

    let mut cmd = Command::new(prog);
    cmd.args(args)
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::piped());
    for (k, v) in envs {
        cmd.env(k, v);
    }

    let mut child = cmd.spawn().expect("spawn");
    {
        let mut si = child.stdin.take().expect("stdin");
        let data: &[u8] = match stdin {
            In::Empty => b"",
            In::Bytes(b) => b,
        };
        // The program never reads stdin, so it may exit before we finish
        // writing; a broken pipe here is expected and must not fail the test.
        let _ = si.write_all(data);
        let _ = si.flush();
    }
    let out = child.wait_with_output().expect("wait");

    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

/// Assert the two programs behave identically for one input.
fn assert_same(label: &str, args: &[&OsStr], stdin: In, envs: &[(&str, &str)]) {
    let cwd = workspace_root();
    let c = run(&c_binary(), args, &stdin, envs, &cwd);
    let r = run(&rust_binary(), args, &stdin, envs, &cwd);

    assert_eq!(
        c.stdout, r.stdout,
        "[{label}] stdout differs\n C: {:?}\n R: {:?}",
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    assert_eq!(
        c.stderr, r.stderr,
        "[{label}] stderr differs\n C: {:?}\n R: {:?}",
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
    assert_eq!(c.code, r.code, "[{label}] exit code differs");
    assert_eq!(c.signal, r.signal, "[{label}] terminating signal differs");
}

fn no_args() -> Vec<&'static OsStr> {
    Vec::new()
}

// ---------------------------------------------------------------------------
// Phase B — the expected output, and the argv/stdin input classes
//
// main() takes no input: it ignores argc/argv and never reads stdin. The only
// branch in the program is `if (line != NULL)` inside printLine, and every one
// of the six call sites passes a string literal, so the NULL arm is
// unreachable from the executable. The observable input classes are therefore
// argv shapes, stdin contents, and the state of the std file descriptors.
// ---------------------------------------------------------------------------

/// The exact bytes the C program prints. Note that `bad()` does NOT call
/// helperBad() -- helperBad is defined but never referenced. Replicated as-is.
const EXPECTED: &[u8] = b"Calling good()...\n\
good()\n\
helperGood()\n\
Finished good()\n\
Calling bad()...\n\
bad()\n\
Finished bad()\n";

#[test]
fn c_output_is_the_documented_bytes() {
    let cwd = workspace_root();
    let c = run(&c_binary(), &no_args(), &In::Empty, &[], &cwd);
    assert_eq!(
        c.stdout,
        EXPECTED,
        "C reference output changed: {:?}",
        String::from_utf8_lossy(&c.stdout)
    );
    assert_eq!(c.stderr, b"", "C writes nothing to stderr");
    assert_eq!(c.code, Some(0));
    assert_eq!(c.signal, None);
}

#[test]
fn rust_matches_exact_expected_bytes() {
    let cwd = workspace_root();
    let r = run(&rust_binary(), &no_args(), &In::Empty, &[], &cwd);
    assert_eq!(r.stdout, EXPECTED);
    assert_eq!(r.stderr, b"");
    assert_eq!(r.code, Some(0));
    assert_eq!(r.signal, None);
}

#[test]
fn no_args_empty_stdin() {
    assert_same("no args, empty stdin", &no_args(), In::Empty, &[]);
}

#[test]
fn single_arg() {
    assert_same("single arg", &[OsStr::new("one")], In::Empty, &[]);
}

#[test]
fn empty_string_arg() {
    assert_same("empty string arg", &[OsStr::new("")], In::Empty, &[]);
}

#[test]
fn several_args_including_flag_lookalikes() {
    let args: Vec<&OsStr> = vec![
        OsStr::new("-h"),
        OsStr::new("--help"),
        OsStr::new("good"),
        OsStr::new("bad"),
        OsStr::new("arg with spaces"),
        OsStr::new("-"),
    ];
    assert_same("flag-lookalike args", &args, In::Empty, &[]);
}

#[test]
fn many_args() {
    // argc well past anything the program inspects.
        let owned: Vec<String> = (0..256).map(|i| format!("arg{i}")).collect();
    let args: Vec<&OsStr> = owned.iter().map(|s| OsStr::new(s.as_str())).collect();
    assert_same("256 args", &args, In::Empty, &[]);
}

#[test]
fn non_utf8_and_unicode_args() {
    let raw = OsStr::from_bytes(b"\xff\xfe\x80bad-utf8");
    let args: Vec<&OsStr> = vec![raw, OsStr::new("naïve-ünïcode-\u{1F600}")];
    assert_same("non-UTF8 + unicode args", &args, In::Empty, &[]);
}

#[test]
fn very_long_single_arg() {
    let long = "x".repeat(100_000);
    assert_same("100k-char arg", &[OsStr::new(long.as_str())], In::Empty, &[]);
}

#[test]
fn stdin_with_text_is_ignored() {
    // No scanf/fgets anywhere: stdin must be left untouched and unconsumed.
    assert_same(
        "stdin text ignored",
        &no_args(),
        In::Bytes(b"5\n1 2 3 4 5\nextra line\n"),
        &[],
    );
}

#[test]
fn stdin_without_trailing_newline() {
    assert_same("stdin no trailing newline", &no_args(), In::Bytes(b"42"), &[]);
}

#[test]
fn stdin_binary_and_nul_bytes() {
    assert_same(
        "stdin binary/NUL",
        &no_args(),
        In::Bytes(b"\x00\x01\x02\xff\n\x00"),
        &[],
    );
}

#[test]
fn stdin_large() {
    let big = vec![b'a'; 1 << 20]; // 1 MiB
    assert_same("1MiB stdin", &no_args(), In::Bytes(&big), &[]);
}

#[test]
fn locale_env_does_not_change_output() {
    for loc in ["C", "en_US.UTF-8", "tr_TR.UTF-8", "POSIX"] {
        assert_same(
            &format!("LC_ALL={loc}"),
            &no_args(),
            In::Empty,
            &[("LC_ALL", loc), ("LANG", loc)],
        );
    }
}

#[test]
fn empty_environment() {
    // Same inputs, but scrub the environment for both children equally.
    use std::os::unix::process::ExitStatusExt;
    let cwd = workspace_root();
    let run_bare = |p: PathBuf| {
        let out = Command::new(p)
            .current_dir(&cwd)
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("run");
        Outcome {
            stdout: out.stdout,
            stderr: out.stderr,
            code: out.status.code(),
            signal: out.status.signal(),
        }
    };
    let c = run_bare(c_binary());
    let r = run_bare(rust_binary());
    assert_eq!(c.stdout, r.stdout, "empty env: stdout differs");
    assert_eq!(c.stderr, r.stderr, "empty env: stderr differs");
    assert_eq!(c.code, r.code, "empty env: exit code differs");
    assert_eq!(c.signal, r.signal, "empty env: signal differs");
}

// ---------------------------------------------------------------------------
// Phase C — file-descriptor states: the paths argv/stdin tests never reach
// ---------------------------------------------------------------------------

extern "C" {
    fn pipe(fds: *mut i32) -> i32;
    fn close(fd: i32) -> i32;
    fn open(path: *const i8, flags: i32, ...) -> i32;
}

/// Run the program with stdout attached to a pipe whose READ END IS ALREADY
/// CLOSED before the child is spawned. This is deterministic (unlike racing a
/// reader that exits): the very first write() must fail with EPIPE, which
/// raises SIGPIPE.
///
/// A C program dies from SIGPIPE by default. The Rust runtime installs
/// SIG_IGN for SIGPIPE before main, so the translation must explicitly restore
/// SIG_DFL to match.
fn run_with_dead_stdout_pipe(prog: &Path) -> Outcome {
    use std::os::unix::process::ExitStatusExt;

    let mut fds = [0i32; 2];
    assert_eq!(unsafe { pipe(fds.as_mut_ptr()) }, 0, "pipe() failed");
    let (read_fd, write_fd) = (fds[0], fds[1]);

    // Hand the write end to the child as stdout, then close the read end here
    // so no reader exists at all.
    let child_stdout = unsafe { OwnedFd::from_raw_fd(write_fd) };
    unsafe { close(read_fd) };

    let out = Command::new(prog)
        .current_dir(workspace_root())
        .stdin(Stdio::null())
        .stdout(Stdio::from(child_stdout))
        .stderr(Stdio::piped())
        .output()
        .expect("spawn with dead stdout pipe");

    Outcome {
        stdout: out.stdout, // empty: stdout went to the doomed pipe
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

#[test]
fn broken_stdout_pipe_matches_including_sigpipe() {
    let c = run_with_dead_stdout_pipe(&c_binary());
    let r = run_with_dead_stdout_pipe(&rust_binary());
    assert_eq!(
        c.signal, r.signal,
        "broken pipe: terminating signal differs (C={:?}, Rust={:?}). \
         The Rust runtime ignores SIGPIPE unless SIG_DFL is restored.",
        c.signal, r.signal
    );
    assert_eq!(c.code, r.code, "broken pipe: exit code differs");
    assert_eq!(c.stderr, r.stderr, "broken pipe: stderr differs");
    // Pin the reference behavior so a regression cannot silently pass.
    assert_eq!(c.signal, Some(13), "C is expected to die from SIGPIPE");
}

/// Run the program with file descriptor 1 (or 2) fully CLOSED, not redirected.
/// printf then fails with EBADF; C ignores the return value and still exits 0.
fn run_with_closed_fd(prog: &Path, which_fd: i32) -> Outcome {
    use std::os::unix::process::CommandExt;
    use std::os::unix::process::ExitStatusExt;

    let mut cmd = Command::new(prog);
    cmd.current_dir(workspace_root()).stdin(Stdio::null());
    if which_fd == 1 {
        cmd.stdout(Stdio::null()).stderr(Stdio::piped());
    } else {
        cmd.stdout(Stdio::piped()).stderr(Stdio::null());
    }
    unsafe {
        cmd.pre_exec(move || {
            // Close the fd right before exec so the program starts without it.
            close(which_fd);
            Ok(())
        });
    }
    let out = cmd.output().expect("spawn with closed fd");
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

#[test]
fn closed_stdout_matches() {
    let c = run_with_closed_fd(&c_binary(), 1);
    let r = run_with_closed_fd(&rust_binary(), 1);
    assert_eq!(c.code, r.code, "closed stdout: exit code differs");
    assert_eq!(c.signal, r.signal, "closed stdout: signal differs");
    assert_eq!(c.stderr, r.stderr, "closed stdout: stderr differs");
}

#[test]
fn closed_stderr_matches() {
    let c = run_with_closed_fd(&c_binary(), 2);
    let r = run_with_closed_fd(&rust_binary(), 2);
    assert_eq!(c.code, r.code, "closed stderr: exit code differs");
    assert_eq!(c.signal, r.signal, "closed stderr: signal differs");
    assert_eq!(c.stdout, r.stdout, "closed stderr: stdout differs");
}

#[test]
fn stdin_closed_entirely() {
    use std::os::unix::process::CommandExt;
    use std::os::unix::process::ExitStatusExt;
    let go = |p: PathBuf| {
        let mut cmd = Command::new(p);
        cmd.current_dir(workspace_root())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        unsafe {
            cmd.pre_exec(|| {
                close(0);
                Ok(())
            });
        }
        let out = cmd.output().expect("run");
        Outcome {
            stdout: out.stdout,
            stderr: out.stderr,
            code: out.status.code(),
            signal: out.status.signal(),
        }
    };
    let c = go(c_binary());
    let r = go(rust_binary());
    assert_eq!(c.stdout, r.stdout, "closed stdin: stdout differs");
    assert_eq!(c.stderr, r.stderr, "closed stdin: stderr differs");
    assert_eq!(c.code, r.code, "closed stdin: exit code differs");
    assert_eq!(c.signal, r.signal, "closed stdin: signal differs");
}

/// stdout redirected to a regular FILE rather than a pipe. C stdio is
/// block-buffered on files and line-buffered on TTYs; the byte content must be
/// identical either way, and must match the piped run.
#[test]
fn stdout_to_regular_file_matches() {
    use std::os::unix::process::ExitStatusExt;

    let dir = std::env::temp_dir();
    let go = |p: PathBuf, tag: &str| -> (Vec<u8>, Option<i32>, Option<i32>) {
        let path = dir.join(format!("driver_out_{tag}_{}.txt", std::process::id()));
        let f = std::fs::File::create(&path).expect("create out file");
        let out = Command::new(p)
            .current_dir(workspace_root())
            .stdin(Stdio::null())
            .stdout(Stdio::from(f))
            .stderr(Stdio::piped())
            .output()
            .expect("run to file");
        let bytes = std::fs::read(&path).expect("read out file");
        let _ = std::fs::remove_file(&path);
        (bytes, out.status.code(), out.status.signal())
    };
    let (cb, cc, cs) = go(c_binary(), "c");
    let (rb, rc_, rs) = go(rust_binary(), "r");
    assert_eq!(cb, rb, "stdout->file: bytes differ");
    assert_eq!(cc, rc_, "stdout->file: exit code differs");
    assert_eq!(cs, rs, "stdout->file: signal differs");
    // Same bytes as when stdout is a pipe: buffering must not alter content.
    assert_eq!(cb, EXPECTED, "stdout->file: differs from piped output");
}

/// stdout pointed at /dev/full: every write fails with ENOSPC. C's printf
/// ignores the error and main still returns 0.
#[test]
fn stdout_to_dev_full_matches() {
    use std::os::unix::process::ExitStatusExt;
    if !Path::new("/dev/full").exists() {
        // Not an ignored test: the comparison simply has no meaning here, and
        // every other case still runs.
        return;
    }
    let go = |p: PathBuf| {
        let f = std::fs::OpenOptions::new()
            .write(true)
            .open("/dev/full")
            .expect("open /dev/full");
        let out = Command::new(p)
            .current_dir(workspace_root())
            .stdin(Stdio::null())
            .stdout(Stdio::from(f))
            .stderr(Stdio::piped())
            .output()
            .expect("run to /dev/full");
        Outcome {
            stdout: out.stdout,
            stderr: out.stderr,
            code: out.status.code(),
            signal: out.status.signal(),
        }
    };
    let c = go(c_binary());
    let r = go(rust_binary());
    assert_eq!(c.stderr, r.stderr, "/dev/full: stderr differs");
    assert_eq!(c.code, r.code, "/dev/full: exit code differs");
    assert_eq!(c.signal, r.signal, "/dev/full: signal differs");
}

/// Running from a different working directory must not matter.
#[test]
fn different_cwd_matches() {
    use std::os::unix::process::ExitStatusExt;
    let tmp = std::env::temp_dir();
    let go = |p: PathBuf| {
        let out = Command::new(p)
            .current_dir(&tmp)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("run");
        Outcome {
            stdout: out.stdout,
            stderr: out.stderr,
            code: out.status.code(),
            signal: out.status.signal(),
        }
    };
    let c = go(c_binary());
    let r = go(rust_binary());
    assert_eq!(c.stdout, r.stdout, "cwd change: stdout differs");
    assert_eq!(c.stderr, r.stderr, "cwd change: stderr differs");
    assert_eq!(c.code, r.code, "cwd change: exit code differs");
    assert_eq!(c.signal, r.signal, "cwd change: signal differs");
}

/// argv[0] is not used by the program, but vary it anyway: the C program never
/// prints a program name, so output must be unchanged.
#[test]
fn unusual_argv0_matches() {
    use std::os::unix::process::CommandExt;
    use std::os::unix::process::ExitStatusExt;
    let go = |p: PathBuf| {
        let out = Command::new(p)
            .arg0("totally-different-name")
            .current_dir(workspace_root())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("run");
        Outcome {
            stdout: out.stdout,
            stderr: out.stderr,
            code: out.status.code(),
            signal: out.status.signal(),
        }
    };
    let c = go(c_binary());
    let r = go(rust_binary());
    assert_eq!(c.stdout, r.stdout, "argv0: stdout differs");
    assert_eq!(c.stderr, r.stderr, "argv0: stderr differs");
    assert_eq!(c.code, r.code, "argv0: exit code differs");
    assert_eq!(c.signal, r.signal, "argv0: signal differs");
}

/// Repeated runs are deterministic and identical (no address/time dependence).
#[test]
fn repeated_runs_are_stable() {
    for i in 0..5 {
        assert_same(&format!("repeat #{i}"), &no_args(), In::Empty, &[]);
    }
}

/// Silence the unused-function warning for `open`, which is declared alongside
/// the other libc entry points for completeness.
#[test]
fn libc_decls_are_used() {
    let _ = open as *const () as usize;
}
