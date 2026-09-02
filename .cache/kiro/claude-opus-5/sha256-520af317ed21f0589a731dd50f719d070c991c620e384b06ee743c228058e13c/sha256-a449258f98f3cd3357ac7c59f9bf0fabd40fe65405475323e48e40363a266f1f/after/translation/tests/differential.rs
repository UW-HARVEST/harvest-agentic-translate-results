//! Differential tests: run the C binary and the Rust binary as subprocesses and
//! require byte-identical stdout, byte-identical stderr, and an identical exit
//! status (including death by signal) for every input class.
//!
//! Nothing here loads the Rust code as a library. Both programs are driven
//! exactly the way a shell drives them, because that is how the translation is
//! graded.
//!
//! Input classes come from reading c_src/src/main.c. That program:
//!   * never reads stdin,
//!   * ignores `argc` / `argv`,
//!   * writes a fixed 74-byte sequence to stdout and nothing to stderr,
//!   * branches only in `printLine` on `line != NULL`, which every caller
//!     satisfies with a string literal.
//! So the behaviour that can actually diverge is in the I/O environment: how
//! stdout is buffered, and what happens when writing to it fails. Those classes
//! are enumerated below.

use std::fs;
use std::io::Write;
use std::os::unix::io::FromRawFd;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Once;

// Borrowed from the C runtime that Rust already links on Linux, so the tests
// need no external crates.
extern "C" {
    fn pipe(fds: *mut i32) -> i32;
    fn close(fd: i32) -> i32;
    fn posix_openpt(flags: i32) -> i32;
    fn grantpt(fd: i32) -> i32;
    fn unlockpt(fd: i32) -> i32;
    fn ptsname(fd: i32) -> *const std::os::raw::c_char;
}

/// Normalised result of one run, used for comparison.
#[derive(PartialEq, Eq, Debug)]
struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Ok(code)` for normal exit, `Err(signal)` when killed by a signal.
    status: Result<i32, i32>,
}

impl Run {
    fn describe(&self) -> String {
        format!(
            "status={:?} stdout={:?} stderr={:?}",
            self.status,
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr)
        )
    }
}

fn status_of(s: std::process::ExitStatus) -> Result<i32, i32> {
    match s.code() {
        Some(c) => Ok(c),
        None => Err(s.signal().expect("process neither exited nor signalled")),
    }
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// Path to the Rust binary under test, provided by Cargo for integration tests.
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Path to the C binary, building it with CMake on first use if necessary.
/// A missing C binary is a hard failure: comparing against a program that did
/// not build measures nothing.
fn c_bin() -> PathBuf {
    static BUILD: Once = Once::new();
    let c_src = repo_root().join("c_src");
    let bin = c_src.join("build").join("driver");

    BUILD.call_once(|| {
        if bin.exists() {
            return;
        }
        let build_dir = c_src.join("build");
        fs::create_dir_all(&build_dir).expect("create c_src/build");
        let cfg = Command::new("cmake")
            .arg("..")
            .current_dir(&build_dir)
            .output()
            .expect("run cmake (is cmake installed?)");
        assert!(
            cfg.status.success(),
            "cmake configure failed:\n{}",
            String::from_utf8_lossy(&cfg.stderr)
        );
        let out = Command::new("cmake")
            .args(["--build", "."])
            .current_dir(&build_dir)
            .output()
            .expect("run cmake --build");
        assert!(
            out.status.success(),
            "cmake build failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
    });

    assert!(bin.exists(), "C binary missing at {}", bin.display());
    bin
}

/// Run one program with the given argv and stdin bytes, capturing everything.
fn run(exe: &Path, args: &[&str], stdin: Option<&[u8]>, cwd: &Path, env: &[(&str, &str)]) -> Run {
    let mut cmd = Command::new(exe);
    cmd.args(args)
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(match stdin {
            Some(_) => Stdio::piped(),
            None => Stdio::null(),
        });
    for (k, v) in env {
        cmd.env(k, v);
    }
    let mut child = cmd.spawn().unwrap_or_else(|e| panic!("spawn {:?}: {e}", exe));
    if let Some(bytes) = stdin {
        child
            .stdin
            .as_mut()
            .expect("stdin piped")
            .write_all(bytes)
            .ok(); // the program never reads stdin, so a broken pipe is expected
    }
    let out = child.wait_with_output().expect("wait for child");
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        status: status_of(out.status),
    }
}

/// Assert the two programs agree on all three observables for one input.
fn assert_same(label: &str, args: &[&str], stdin: Option<&[u8]>) {
    let root = repo_root();
    let c = run(&c_bin(), args, stdin, &root, &[]);
    let r = run(&rust_bin(), args, stdin, &root, &[]);
    assert_eq!(
        c, r,
        "\n[{label}] mismatch\n  args={args:?}\n  C   : {}\n  Rust: {}\n",
        c.describe(),
        r.describe()
    );
}

// ---------------------------------------------------------------------------
// Baseline: the only control-flow path the program has.
// ---------------------------------------------------------------------------

#[test]
fn no_arguments() {
    assert_same("no arguments", &[], None);
}

/// Locks in the exact bytes, so a regression cannot hide behind both programs
/// changing together. Also pins the `bad()` defect: it must print 0 twice,
/// because C discards `intOne + intTwo` instead of assigning it to `intSum`.
#[test]
fn exact_expected_bytes_including_bad_defect() {
    let expected = "Calling good()...\n0\n2\nFinished good()\nCalling bad()...\n0\n0\nFinished bad()\n";
    let root = repo_root();
    for exe in [c_bin(), rust_bin()] {
        let got = run(&exe, &[], None, &root, &[]);
        assert_eq!(
            String::from_utf8_lossy(&got.stdout),
            expected,
            "stdout for {}",
            exe.display()
        );
        assert!(got.stderr.is_empty(), "stderr must be empty for {}", exe.display());
        assert_eq!(got.status, Ok(0), "exit status for {}", exe.display());
    }
}

/// The 74 bytes are fixed; nothing is time- or environment-dependent.
#[test]
fn output_is_deterministic_across_runs() {
    let root = repo_root();
    let first = run(&rust_bin(), &[], None, &root, &[]);
    for _ in 0..10 {
        assert_eq!(first, run(&rust_bin(), &[], None, &root, &[]));
    }
    assert_eq!(first.stdout.len(), 74, "output length");
}

// ---------------------------------------------------------------------------
// argv: `main` takes argc/argv but never inspects them.
// ---------------------------------------------------------------------------

#[test]
fn arguments_are_ignored() {
    assert_same("single arg", &["one"], None);
    assert_same("several args", &["a", "b", "c"], None);
    assert_same("empty string arg", &[""], None);
    assert_same("flag-like args", &["-h", "--help", "--version"], None);
    assert_same("numeric args", &["0", "-1", "2147483647", "-2147483648"], None);
    assert_same("whitespace/newline arg", &["a b\tc\nd"], None);
    assert_same("non-ascii arg", &["café-\u{2014}-\u{1f600}"], None);
    assert_same("path-like arg", &["../../etc/passwd"], None);
    assert_same("format-specifier arg", &["%s %d %n %x"], None);
}

/// The maximum argv the harness can reasonably hand over: many long arguments.
#[test]
fn many_and_long_arguments() {
    let long = "x".repeat(4096);
    let many: Vec<String> = (0..256).map(|i| format!("arg{i}")).collect();
    let many_refs: Vec<&str> = many.iter().map(|s| s.as_str()).collect();
    assert_same("long single arg", &[long.as_str()], None);
    assert_same("many args", &many_refs, None);
}

// ---------------------------------------------------------------------------
// stdin: the C program never reads it, so every shape must be ignored
// identically and must be left unconsumed.
// ---------------------------------------------------------------------------

#[test]
fn stdin_variants_are_ignored() {
    assert_same("stdin from /dev/null", &[], None);
    assert_same("empty stdin", &[], Some(b""));
    assert_same("single line stdin", &[], Some(b"1\n"));
    assert_same("single item no newline", &[], Some(b"42"));
    assert_same("multi line stdin", &[], Some(b"1\n2\n3\n"));
    assert_same("non numeric stdin", &[], Some(b"not a number\n"));
    assert_same("binary stdin", &[], Some(&[0u8, 1, 2, 255, b'\n']));
    assert_same("whitespace only stdin", &[], Some(b"   \t\n  \n"));
}

#[test]
fn large_stdin_is_ignored() {
    let big = vec![b'7'; 1 << 20]; // 1 MiB, larger than any pipe buffer
    assert_same("1MiB stdin", &[], Some(&big));
}

/// stdin closed outright rather than redirected, so fd 0 is not even open.
#[test]
fn stdin_closed() {
    let root = repo_root();
    let mut results = Vec::new();
    for exe in [c_bin(), rust_bin()] {
        let out = Command::new("sh")
            .arg("-c")
            .arg(format!("exec {} <&-", shell_quote(&exe)))
            .current_dir(&root)
            .output()
            .expect("run via sh with stdin closed");
        results.push(Run {
            stdout: out.stdout,
            stderr: out.stderr,
            status: status_of(out.status),
        });
    }
    assert_eq!(
        results[0], results[1],
        "stdin-closed mismatch\n  C   : {}\n  Rust: {}",
        results[0].describe(),
        results[1].describe()
    );
}

// ---------------------------------------------------------------------------
// Environment and working directory are irrelevant to this program.
// ---------------------------------------------------------------------------

#[test]
fn environment_and_cwd_do_not_matter() {
    let root = repo_root();
    let env = [("LC_ALL", "C"), ("LANG", "C"), ("TERM", "dumb"), ("COLUMNS", "1")];
    let c = run(&c_bin(), &[], None, &root, &env);
    let r = run(&rust_bin(), &[], None, &root, &env);
    assert_eq!(c, r, "env mismatch\n  C: {}\n  Rust: {}", c.describe(), r.describe());

    let tmp = Path::new("/tmp");
    let c2 = run(&c_bin(), &[], None, tmp, &[]);
    let r2 = run(&rust_bin(), &[], None, tmp, &[]);
    assert_eq!(c2, r2, "cwd mismatch");
    assert_eq!(c, c2, "C output must not depend on cwd/env");
}

// ---------------------------------------------------------------------------
// stdout error paths. `printf` and the implicit `fflush` at exit can fail;
// the C program checks neither return value, so a failed write must not change
// the exit status.
// ---------------------------------------------------------------------------

/// Writing to /dev/full always fails with ENOSPC. C ignores that and exits 0.
#[test]
fn stdout_write_failure_on_dev_full() {
    if !Path::new("/dev/full").exists() {
        panic!("/dev/full is required for this test");
    }
    let root = repo_root();
    let mut results = Vec::new();
    for exe in [c_bin(), rust_bin()] {
        let out = Command::new("sh")
            .arg("-c")
            .arg(format!("exec {} >/dev/full", shell_quote(&exe)))
            .current_dir(&root)
            .output()
            .expect("run with stdout on /dev/full");
        results.push(Run {
            stdout: out.stdout,
            stderr: out.stderr,
            status: status_of(out.status),
        });
    }
    assert_eq!(
        results[0], results[1],
        "/dev/full mismatch\n  C   : {}\n  Rust: {}",
        results[0].describe(),
        results[1].describe()
    );
    assert_eq!(results[0].status, Ok(0), "C must still exit 0 on ENOSPC");
}

/// stdout closed entirely: every write fails with EBADF, still exit 0.
#[test]
fn stdout_closed() {
    let root = repo_root();
    let mut results = Vec::new();
    for exe in [c_bin(), rust_bin()] {
        let out = Command::new("sh")
            .arg("-c")
            .arg(format!("exec {} >&-", shell_quote(&exe)))
            .current_dir(&root)
            .output()
            .expect("run with stdout closed");
        results.push(Run {
            stdout: out.stdout,
            stderr: out.stderr,
            status: status_of(out.status),
        });
    }
    assert_eq!(
        results[0], results[1],
        "stdout-closed mismatch\n  C   : {}\n  Rust: {}",
        results[0].describe(),
        results[1].describe()
    );
    assert_eq!(results[0].status, Ok(0), "C must still exit 0 on EBADF");
}

/// Run `exe` with stdout on a pipe whose read end is already closed, so the
/// very first write hits a broken pipe. The C program inherits the default
/// SIGPIPE disposition and dies from signal 13; the Rust runtime sets SIGPIPE
/// to SIG_IGN before main, so the translation must restore SIG_DFL to match.
fn run_with_broken_stdout_pipe(exe: &Path) -> Run {
    let mut fds = [0i32; 2];
    let rc = unsafe { pipe(fds.as_mut_ptr()) };
    assert_eq!(rc, 0, "pipe() failed");
    let (read_end, write_end) = (fds[0], fds[1]);
    unsafe { close(read_end) };

    let stdout = unsafe { Stdio::from_raw_fd(write_end) }; // takes ownership of write_end
    let out = Command::new(exe)
        .stdin(Stdio::null())
        .stdout(stdout)
        .stderr(Stdio::piped())
        .current_dir(repo_root())
        .spawn()
        .expect("spawn with broken stdout pipe")
        .wait_with_output()
        .expect("wait");

    Run {
        stdout: Vec::new(), // unreadable by construction
        stderr: out.stderr,
        status: status_of(out.status),
    }
}

#[test]
fn broken_stdout_pipe_kills_both_with_sigpipe() {
    let c = run_with_broken_stdout_pipe(&c_bin());
    let r = run_with_broken_stdout_pipe(&rust_bin());
    assert_eq!(
        c, r,
        "broken-pipe mismatch\n  C   : {}\n  Rust: {}",
        c.describe(),
        r.describe()
    );
    assert_eq!(
        c.status,
        Err(13),
        "C is expected to die from SIGPIPE; got {}",
        c.describe()
    );
}

/// A reader that consumes only a few bytes and then goes away.
///
/// Note on what this does and does not prove: because the whole output is 74
/// bytes and fits in the pipe buffer, both programs finish writing long before
/// the reader closes, so this case does NOT discriminate line buffering from
/// full buffering. That was confirmed by negative control (forcing the
/// translation to line-buffer leaves the entire suite green). It is kept
/// because it still pins agreement on a reader that disappears.
fn run_with_early_closing_reader(exe: &Path) -> Run {
    let mut child = Command::new(exe)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .current_dir(repo_root())
        .spawn()
        .expect("spawn with piped stdout");

    // Give the program time to produce its output, then drop the read end.
    std::thread::sleep(std::time::Duration::from_millis(250));
    drop(child.stdout.take());

    let mut stderr = Vec::new();
    if let Some(mut e) = child.stderr.take() {
        use std::io::Read;
        let _ = e.read_to_end(&mut stderr);
    }
    let status = child.wait().expect("wait");
    Run {
        stdout: Vec::new(),
        stderr,
        status: status_of(status),
    }
}

#[test]
fn reader_closing_early_matches() {
    let c = run_with_early_closing_reader(&c_bin());
    let r = run_with_early_closing_reader(&rust_bin());
    assert_eq!(
        c, r,
        "early-closing-reader mismatch\n  C   : {}\n  Rust: {}",
        c.describe(),
        r.describe()
    );
}

// ---------------------------------------------------------------------------
// stdout to a regular file: the fully buffered path, byte-compared on disk.
// ---------------------------------------------------------------------------

#[test]
fn stdout_redirected_to_file_matches() {
    let root = repo_root();
    let dir = std::env::temp_dir();
    let c_path = dir.join("driver_diff_c.out");
    let r_path = dir.join("driver_diff_r.out");

    for (exe, path) in [(c_bin(), &c_path), (rust_bin(), &r_path)] {
        let f = fs::File::create(path).expect("create output file");
        let status = Command::new(&exe)
            .stdin(Stdio::null())
            .stdout(Stdio::from(f))
            .stderr(Stdio::piped())
            .current_dir(&root)
            .spawn()
            .expect("spawn with file stdout")
            .wait_with_output()
            .expect("wait");
        assert_eq!(status_of(status.status), Ok(0));
        assert!(status.stderr.is_empty());
    }

    let c_bytes = fs::read(&c_path).expect("read C output");
    let r_bytes = fs::read(&r_path).expect("read Rust output");
    assert_eq!(c_bytes, r_bytes, "file-redirected stdout differs");
    let _ = fs::remove_file(&c_path);
    let _ = fs::remove_file(&r_path);
}

/// Minimal single-quote shell quoting for embedding a path in `sh -c`.
fn shell_quote(p: &Path) -> String {
    format!("'{}'", p.to_string_lossy().replace('\'', r"'\''"))
}

// ---------------------------------------------------------------------------
// stdout on a real terminal. This is the one case that exercises the other side
// of glibc's buffering decision (line buffered rather than fully buffered), and
// it is the branch the translation selects with isatty(1). The TTY also turns
// each \n into \r\n, so the byte comparison covers that too.
// ---------------------------------------------------------------------------

/// Run `exe` with stdout attached to a pseudo-terminal, returning what the
/// master side observes.
fn run_on_pty(exe: &Path) -> Run {
    const O_RDWR: i32 = 2;
    let master = unsafe { posix_openpt(O_RDWR) };
    assert!(master >= 0, "posix_openpt failed");
    assert_eq!(unsafe { grantpt(master) }, 0, "grantpt failed");
    assert_eq!(unsafe { unlockpt(master) }, 0, "unlockpt failed");

    let slave_name = unsafe {
        let p = ptsname(master);
        assert!(!p.is_null(), "ptsname failed");
        std::ffi::CStr::from_ptr(p)
            .to_str()
            .expect("pts name is utf8")
            .to_owned()
    };
    let slave = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&slave_name)
        .expect("open pty slave");

    let out = Command::new(exe)
        .stdin(Stdio::null())
        .stdout(Stdio::from(slave))
        .stderr(Stdio::piped())
        .current_dir(repo_root())
        .spawn()
        .expect("spawn on pty")
        .wait_with_output()
        .expect("wait");

    // Drain the master side. Once the slave is closed, reads fail with EIO,
    // which marks the end of the output rather than an error.
    let mut stdout = Vec::new();
    {
        use std::io::Read;
        let mut m = std::mem::ManuallyDrop::new(unsafe { fs::File::from_raw_fd(master) });
        let mut buf = [0u8; 4096];
        loop {
            match m.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => stdout.extend_from_slice(&buf[..n]),
                Err(_) => break, // EIO: slave side is gone
            }
        }
    }
    unsafe { close(master) };

    Run {
        stdout,
        stderr: out.stderr,
        status: status_of(out.status),
    }
}

#[test]
fn stdout_on_a_tty_matches() {
    let c = run_on_pty(&c_bin());
    let r = run_on_pty(&rust_bin());
    assert_eq!(
        c, r,
        "pty mismatch\n  C   : {}\n  Rust: {}",
        c.describe(),
        r.describe()
    );
    // Sanity: a TTY performs ONLCR, so the bytes must differ from the pipe case.
    assert!(
        c.stdout.windows(2).any(|w| w == b"\r\n"),
        "expected CRLF on a tty, got {:?}",
        String::from_utf8_lossy(&c.stdout)
    );
}
