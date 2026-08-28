//! Differential tests: run the C program and the Rust program as subprocesses
//! over the same inputs and require byte-identical stdout, byte-identical
//! stderr, and an identical exit status.
//!
//! # Input classes covered
//!
//! `c_src` is branch-free. `main()` takes no parameters and immediately returns
//! `helloworld()`, which performs a single `printf("Hello World!\n")` and
//! returns 0. There is no `scanf`/`fgets`, no `argc`/`argv` inspection, no
//! length check and no error path, so there is no input that can steer it down a
//! second path. The observable input classes are therefore:
//!
//! * stdin shape: absent (`/dev/null`), empty, one line, no trailing newline,
//!   many lines, NUL/high bytes, and larger-than-a-pipe-buffer - all ignored
//! * stdin *kind*: regular file redirect vs. pipe (different `fstat` type,
//!   which is what would change C's stdio buffering had it read anything)
//! * argv: none, one, many, empty string, flag-looking, non-UTF-8
//! * environment: empty, and a locale that would matter if output were localized
//! * output channel: pipe, regular file, closed fd (`>&-`), and a reader that
//!   hangs up early (SIGPIPE)
//! * repetition: output must be deterministic across runs

mod harness;

use harness::{assert_same, c_binary, run, rust_binary, Stdin, EXPECTED_STDOUT};

// ---------------------------------------------------------------------------
// Baseline
// ---------------------------------------------------------------------------

/// The happy path, and a check that both programs really do emit the exact
/// 13 bytes `Hello World!\n` with a trailing newline and nothing more.
#[test]
fn no_args_no_stdin_matches_and_is_exactly_hello_world() {
    let out = assert_same("baseline", &[], Stdin::Null, &[]);
    assert_eq!(
        out.stdout, EXPECTED_STDOUT,
        "expected exactly {:?}",
        String::from_utf8_lossy(EXPECTED_STDOUT)
    );
    assert!(out.stderr.is_empty(), "nothing should be written to stderr");
    assert_eq!(out.code, Some(0), "helloworld() returns 0");
}

// ---------------------------------------------------------------------------
// stdin shape - every one of these is ignored by the C, and must be by the Rust
// ---------------------------------------------------------------------------

#[test]
fn stdin_empty_file() {
    assert_same("empty file", &[], Stdin::File(Vec::new()), &[]);
}

#[test]
fn stdin_empty_pipe() {
    assert_same("empty pipe", &[], Stdin::Pipe(Vec::new()), &[]);
}

#[test]
fn stdin_single_item_no_newline() {
    // A single "item" with no terminating newline: the classic case where
    // `fgets` and `scanf` disagree. Neither program reads it, so both ignore it.
    assert_same("single item, no newline", &[], Stdin::File(b"42".to_vec()), &[]);
}

#[test]
fn stdin_single_line_with_newline() {
    assert_same("single line", &[], Stdin::File(b"hello\n".to_vec()), &[]);
}

#[test]
fn stdin_many_lines() {
    let mut input = Vec::new();
    for i in 0..1000 {
        input.extend_from_slice(format!("{i}\n").as_bytes());
    }
    assert_same("1000 lines", &[], Stdin::File(input), &[]);
}

#[test]
fn stdin_whitespace_and_blank_lines() {
    assert_same(
        "whitespace only",
        &[],
        Stdin::File(b"\n\n   \t\r\n  \n".to_vec()),
        &[],
    );
}

#[test]
fn stdin_nul_and_high_bytes() {
    // Not valid UTF-8, and contains an interior NUL: safe for the C (never read)
    // and must not make the Rust program stumble either.
    let input = vec![0u8, 1, 0xff, 0xfe, b'a', 0, 0x80, b'\n'];
    assert_same("NUL + high bytes", &[], Stdin::File(input), &[]);
}

#[test]
fn stdin_numeric_tokens_across_newlines() {
    // If the C had used `scanf("%d")`, this would parse across the newlines.
    // It does not read at all, so this must be ignored.
    assert_same(
        "numbers across newlines",
        &[],
        Stdin::File(b"1 2\n3\t4\n  5  ".to_vec()),
        &[],
    );
}

#[test]
fn stdin_larger_than_pipe_buffer_via_pipe() {
    // 1 MiB down a pipe that the child never drains. Both programs exit without
    // reading, so the writer sees a broken pipe; that must not change either
    // program's stdout, stderr or exit status.
    let input = vec![b'x'; 1024 * 1024];
    assert_same("1 MiB pipe", &[], Stdin::Pipe(input), &[]);
}

#[test]
fn stdin_larger_than_pipe_buffer_via_file() {
    let input = vec![b'y'; 1024 * 1024];
    assert_same("1 MiB file", &[], Stdin::File(input), &[]);
}

#[test]
fn stdin_very_long_single_line() {
    // One line far longer than any plausible fixed-size `fgets` buffer.
    let mut input = vec![b'z'; 100_000];
    input.push(b'\n');
    assert_same("100k-char line", &[], Stdin::File(input), &[]);
}

// ---------------------------------------------------------------------------
// argv - `main()` declares no parameters, so all of these are ignored
// ---------------------------------------------------------------------------

#[test]
fn args_single() {
    assert_same("one arg", &["foo"], Stdin::Null, &[]);
}

#[test]
fn args_many() {
    let args: Vec<String> = (0..64).map(|i| format!("arg{i}")).collect();
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    assert_same("64 args", &refs, Stdin::Null, &[]);
}

#[test]
fn args_flag_like() {
    // `--help` / `-v` would be handled by an arg parser; there isn't one, so
    // these must print the greeting and exit 0 just like the bare invocation.
    for case in [
        vec!["--help"],
        vec!["-h"],
        vec!["--version"],
        vec!["-"],
        vec!["--"],
        vec!["--unknown-flag", "--another"],
    ] {
        assert_same("flag-like arg", &case, Stdin::Null, &[]);
    }
}

#[test]
fn args_empty_string_and_whitespace() {
    assert_same("empty + spaces", &["", " ", "\t", "\n"], Stdin::Null, &[]);
}

#[test]
fn args_non_utf8() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    use std::process::{Command, Stdio};

    // A lone 0xff byte is a valid argument to execve but is not valid UTF-8.
    let bad = OsStr::from_bytes(b"\xff\xfe-not-utf8");
    let mut outs = Vec::new();
    for bin in [c_binary(), rust_binary()] {
        let out = Command::new(bin)
            .arg(bad)
            .stdin(Stdio::null())
            .output()
            .expect("spawn with non-UTF-8 argv");
        outs.push(out);
    }
    assert_eq!(outs[0].stdout, outs[1].stdout, "stdout differs");
    assert_eq!(outs[0].stderr, outs[1].stderr, "stderr differs");
    assert_eq!(
        outs[0].status.code(),
        outs[1].status.code(),
        "exit code differs"
    );
}

#[test]
fn args_very_long() {
    let long = "a".repeat(100_000);
    assert_same("100k-char arg", &[&long], Stdin::Null, &[]);
}

// ---------------------------------------------------------------------------
// Environment
// ---------------------------------------------------------------------------

#[test]
fn env_empty() {
    assert_same("empty env", &[], Stdin::Null, &[]);
}

#[test]
fn env_locales_do_not_change_output() {
    // The greeting is a plain ASCII literal, so no locale may alter it. A Turkish
    // locale is the classic trap for case-mapping, and LC_ALL=C vs UTF-8 is the
    // classic trap for `printf` of non-ASCII.
    for locale in ["C", "C.UTF-8", "en_US.UTF-8", "tr_TR.UTF-8", "de_DE.UTF-8"] {
        assert_same(
            "locale",
            &[],
            Stdin::Null,
            &[("LC_ALL", locale), ("LANG", locale)],
        );
    }
}

// ---------------------------------------------------------------------------
// Output channel behaviour
// ---------------------------------------------------------------------------

#[test]
fn stdout_to_regular_file_matches() {
    // With stdout on a pipe, C stdio is block-buffered; on a regular file it is
    // also block-buffered; on a tty, line-buffered. Only one write happens, so
    // the byte stream must be identical - verify the file-redirect case, which
    // the piped harness does not otherwise exercise.
    use std::process::{Command, Stdio};

    let dir = std::path::Path::new(env!("CARGO_BIN_EXE_driver"))
        .parent()
        .unwrap()
        .join("difftest-scratch");
    std::fs::create_dir_all(&dir).unwrap();

    let mut results = Vec::new();
    for (tag, bin) in [("c", c_binary()), ("rust", rust_binary())] {
        let path = dir.join(format!("stdout-{tag}-{}", std::process::id()));
        let file = std::fs::File::create(&path).unwrap();
        let status = Command::new(bin)
            .stdin(Stdio::null())
            .stdout(Stdio::from(file))
            .stderr(Stdio::piped())
            .output()
            .expect("spawn with stdout redirected to a file");
        let written = std::fs::read(&path).unwrap();
        results.push((written, status.stderr, status.status.code()));
    }
    assert_eq!(results[0].0, results[1].0, "file-redirected stdout differs");
    assert_eq!(results[0].0, EXPECTED_STDOUT, "unexpected file contents");
    assert_eq!(results[0].1, results[1].1, "stderr differs");
    assert_eq!(results[0].2, results[1].2, "exit code differs");
}

#[test]
fn stdout_closed_is_silent_and_still_exits_zero() {
    // `prog >&-`: the write fails. The C ignores `printf`'s return value and
    // still returns 0, so the Rust must not report the error or panic. (A
    // `println!` translation would abort here with exit 101 and a message on
    // stderr - this test is what catches that.)
    use std::os::unix::process::CommandExt;
    use std::process::{Command, Stdio};

    let mut results = Vec::new();
    for bin in [c_binary(), rust_binary()] {
        // Expressing `>&-` via Command needs a pre_exec hook that closes fd 1 in
        // the child, after the fork and before the exec.
        let mut cmd = Command::new(bin);
        cmd.stdin(Stdio::null())
            .stderr(Stdio::piped())
            .stdout(Stdio::null());
        unsafe {
            cmd.pre_exec(|| {
                // Close stdout so the program's write(2) fails with EBADF.
                if libc_close(1) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let out = cmd.output().expect("spawn with stdout closed");
        results.push((out.stderr, out.status.code(), out.status.signal_of()));
    }
    assert_eq!(
        results[0].0, results[1].0,
        "stderr differs with stdout closed:\n  C   ={:?}\n  Rust={:?}",
        String::from_utf8_lossy(&results[0].0),
        String::from_utf8_lossy(&results[1].0)
    );
    assert_eq!(
        results[0].1, results[1].1,
        "exit code differs with stdout closed (C={:?}, Rust={:?})",
        results[0].1, results[1].1
    );
    assert_eq!(results[0].2, results[1].2, "signal differs");
}

/// Minimal `close(2)` binding so the test needs no external crate.
fn libc_close(fd: i32) -> i32 {
    extern "C" {
        fn close(fd: i32) -> i32;
    }
    unsafe { close(fd) }
}

/// Minimal `pipe(2)` binding so the test needs no external crate.
unsafe fn libc_pipe(fds: *mut i32) -> i32 {
    extern "C" {
        fn pipe(fds: *mut i32) -> i32;
    }
    pipe(fds)
}

/// Helper trait so the test can read a terminating signal without importing the
/// unix extension trait at every call site.
trait SignalOf {
    fn signal_of(&self) -> Option<i32>;
}

impl SignalOf for std::process::ExitStatus {
    fn signal_of(&self) -> Option<i32> {
        use std::os::unix::process::ExitStatusExt;
        self.signal()
    }
}

#[test]
fn reader_hangs_up_early_same_disposition() {
    // A reader that has already hung up. The C program runs with the default
    // SIGPIPE disposition and is killed by signal 13; the Rust runtime sets
    // SIGPIPE to SIG_IGN before main, which would make it exit 0 instead. Both
    // must agree.
    use std::fs::File;
    use std::os::unix::io::FromRawFd;
    use std::process::{Command, Stdio};

    let mut results = Vec::new();
    for bin in [c_binary(), rust_binary()] {
        // Build the pipe by hand and close the read end *before* spawning, so
        // the child's write is guaranteed to find no reader. Dropping the read
        // end after spawn would race against the child's write.
        let mut fds = [0i32; 2];
        assert_eq!(unsafe { libc_pipe(fds.as_mut_ptr()) }, 0, "pipe(2) failed");
        let read_end = unsafe { File::from_raw_fd(fds[0]) };
        let write_end = unsafe { File::from_raw_fd(fds[1]) };
        drop(read_end);

        let out = Command::new(bin)
            .stdin(Stdio::null())
            .stdout(Stdio::from(write_end))
            .stderr(Stdio::piped())
            .output()
            .expect("spawn with a hung-up reader");
        results.push((out.status.code(), out.status.signal_of()));
    }

    // Sanity-check that the scenario really did trigger SIGPIPE, so this test
    // cannot silently degrade into asserting "both exited 0".
    assert_eq!(
        results[0],
        (None, Some(13)),
        "expected the C program to be killed by SIGPIPE; got {:?}",
        results[0]
    );
    assert_eq!(
        results[0], results[1],
        "writer disposition differs when the reader hangs up (C={:?}, Rust={:?})",
        results[0], results[1]
    );
}

#[test]
fn stdout_write_error_enospc_is_ignored() {
    // `prog > /dev/full`: the write fails with ENOSPC. The C ignores `printf`'s
    // return value and returns 0 regardless; the Rust must do the same and must
    // not print a diagnostic.
    use std::process::{Command, Stdio};

    if !std::path::Path::new("/dev/full").exists() {
        // Nothing to compare against on a system without /dev/full; assert the
        // two programs still agree on the ordinary path so the test is not a
        // silent no-op.
        assert_same("no /dev/full", &[], Stdin::Null, &[]);
        return;
    }

    let mut results = Vec::new();
    for bin in [c_binary(), rust_binary()] {
        let full = std::fs::OpenOptions::new()
            .write(true)
            .open("/dev/full")
            .expect("open /dev/full");
        let out = Command::new(bin)
            .stdin(Stdio::null())
            .stdout(Stdio::from(full))
            .stderr(Stdio::piped())
            .output()
            .expect("spawn with stdout on /dev/full");
        results.push((out.stderr, out.status.code(), out.status.signal_of()));
    }
    assert_eq!(
        results[0].0, results[1].0,
        "stderr differs on ENOSPC:\n  C   ={:?}\n  Rust={:?}",
        String::from_utf8_lossy(&results[0].0),
        String::from_utf8_lossy(&results[1].0)
    );
    assert_eq!(
        results[0].1, results[1].1,
        "exit code differs on ENOSPC (C={:?}, Rust={:?})",
        results[0].1, results[1].1
    );
    assert_eq!(results[0].2, results[1].2, "signal differs on ENOSPC");
}

#[test]
fn stdin_closed_entirely() {
    // `prog <&-`: fd 0 is not open at all. Neither program reads stdin, so both
    // must still print the greeting and exit 0.
    use std::os::unix::process::CommandExt;
    use std::process::{Command, Stdio};

    let mut results = Vec::new();
    for bin in [c_binary(), rust_binary()] {
        let mut cmd = Command::new(bin);
        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        unsafe {
            cmd.pre_exec(|| {
                if libc_close(0) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let out = cmd.output().expect("spawn with stdin closed");
        results.push((out.stdout, out.stderr, out.status.code()));
    }
    assert_eq!(results[0].0, results[1].0, "stdout differs with stdin closed");
    assert_eq!(results[0].0, EXPECTED_STDOUT, "unexpected stdout");
    assert_eq!(results[0].1, results[1].1, "stderr differs with stdin closed");
    assert_eq!(results[0].2, results[1].2, "exit code differs");
}

#[test]
fn stdout_to_a_tty_matches() {
    // On a tty C's stdio is line-buffered rather than block-buffered. Only one
    // write happens either way, so the byte stream must still be identical.
    // `script` provides the pty; skip cleanly if it is unavailable.
    use std::process::{Command, Stdio};

    let script = std::path::Path::new("/usr/bin/script");
    if !script.exists() {
        assert_same("no script(1)", &[], Stdin::Null, &[]);
        return;
    }

    let mut outs = Vec::new();
    for bin in [c_binary(), rust_binary()] {
        let out = Command::new(script)
            .arg("-qec")
            .arg(bin.to_str().expect("binary path is UTF-8"))
            .arg("/dev/null")
            .stdin(Stdio::null())
            .output()
            .expect("run under a pty via script(1)");
        outs.push(out.stdout);
    }
    assert_eq!(
        outs[0],
        outs[1],
        "tty output differs:\n  C   ={:?}\n  Rust={:?}",
        String::from_utf8_lossy(&outs[0]),
        String::from_utf8_lossy(&outs[1])
    );
    // The pty translates NL to CRNL, so expect the greeting with a CR.
    assert_eq!(
        outs[0], b"Hello World!\r\n",
        "unexpected pty byte stream: {:?}",
        String::from_utf8_lossy(&outs[0])
    );
}

#[test]
fn working_directory_does_not_matter() {
    // Neither program touches the filesystem, so the cwd must be irrelevant.
    use std::process::{Command, Stdio};

    let mut outs = Vec::new();
    for bin in [c_binary(), rust_binary()] {
        let abs = std::fs::canonicalize(bin).expect("canonicalize binary path");
        let out = Command::new(&abs)
            .current_dir("/")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("spawn from /");
        outs.push((out.stdout, out.stderr, out.status.code()));
    }
    assert_eq!(outs[0].0, outs[1].0, "stdout differs when run from /");
    assert_eq!(outs[0].1, outs[1].1, "stderr differs when run from /");
    assert_eq!(outs[0].2, outs[1].2, "exit code differs when run from /");
    assert_eq!(outs[0].0, EXPECTED_STDOUT);
}

// ---------------------------------------------------------------------------
// Determinism
// ---------------------------------------------------------------------------

#[test]
fn output_is_deterministic_across_repeated_runs() {
    let first = run(rust_binary(), &[], &Stdin::Null, &[]);
    for i in 0..20 {
        let again = assert_same("repeat", &[], Stdin::Null, &[]);
        assert_eq!(again.stdout, first.stdout, "run {i} changed stdout");
        assert_eq!(again.stderr, first.stderr, "run {i} changed stderr");
        assert_eq!(again.code, first.code, "run {i} changed exit code");
    }
}

#[test]
fn stdout_has_single_trailing_newline_and_no_cr() {
    let out = run(c_binary(), &[], &Stdin::Null, &[]);
    let rust = run(rust_binary(), &[], &Stdin::Null, &[]);
    for (tag, o) in [("C", &out), ("Rust", &rust)] {
        assert!(
            o.stdout.ends_with(b"\n"),
            "{tag}: must end with exactly one newline"
        );
        assert!(
            !o.stdout.ends_with(b"\n\n"),
            "{tag}: must not end with a blank line"
        );
        assert!(
            !o.stdout.contains(&b'\r'),
            "{tag}: must not contain a carriage return"
        );
        assert_eq!(o.stdout.len(), 13, "{tag}: expected 13 bytes");
    }
}
