//! Differential tests: run the C program and the Rust program as subprocesses,
//! feed both the same bytes on stdin, and compare stdout, stderr and exit
//! status byte for byte / value for value.
//!
//! Nothing here links against the translation as a library; the built binary is
//! driven exactly the way a shell drives it, because that is how the two
//! programs are compared.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Path to the Rust binary under test, supplied by Cargo.
fn rust_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Repository root (the directory holding `c_src/` and `translation/`).
fn repo_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the C binary, building it with CMake the first time if needed so
/// that `cargo test` works from a clean checkout.
fn c_binary() -> PathBuf {
    let c_src = repo_root().join("c_src");
    let build_dir = c_src.join("build");
    let exe = build_dir.join("driver");

    if exe.is_file() {
        return exe;
    }

    std::fs::create_dir_all(&build_dir).expect("cannot create c_src/build");

    let configure = Command::new("cmake")
        .arg("..")
        .current_dir(&build_dir)
        .output()
        .expect("cmake not found on PATH; cannot build the C reference program");
    assert!(
        configure.status.success(),
        "cmake configure failed:\n{}\n{}",
        String::from_utf8_lossy(&configure.stdout),
        String::from_utf8_lossy(&configure.stderr)
    );

    let build = Command::new("cmake")
        .args(["--build", "."])
        .current_dir(&build_dir)
        .output()
        .expect("failed to invoke cmake --build");
    assert!(
        build.status.success(),
        "cmake build failed:\n{}\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    assert!(
        exe.is_file(),
        "C reference binary missing after build: {}",
        exe.display()
    );
    exe
}

/// What a run of either program produced.
#[derive(PartialEq, Eq)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Ok(code)` for a normal exit, `Err(signal)` when killed by a signal.
    status: Result<i32, i32>,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status = match self.status {
            Ok(code) => format!("exit {}", code),
            Err(sig) => format!("signal {}", sig),
        };
        write!(
            f,
            "Outcome {{ status: {}, stdout: {:?}, stderr: {:?} }}",
            status,
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr)
        )
    }
}

/// Run `program`, writing `input` to its stdin, and collect everything.
///
/// The write side ignores `EPIPE`: these programs stop reading as soon as one
/// `%d` conversion finishes, so the remainder of a long input is often never
/// consumed. That is true of the C program too and must not fail the test.
fn run(program: &Path, input: &[u8]) -> Outcome {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", program.display()));

    {
        let mut stdin = child.stdin.take().expect("stdin was piped");
        let input = input.to_vec();
        // A separate thread avoids deadlocking on inputs larger than the pipe
        // buffer; write errors (EPIPE) are deliberately ignored.
        std::thread::spawn(move || {
            let _ = stdin.write_all(&input);
            let _ = stdin.flush();
        });
    }

    let out = child.wait_with_output().expect("failed to wait for child");

    let status = match out.status.code() {
        Some(code) => Ok(code),
        None => {
            #[cfg(unix)]
            {
                use std::os::unix::process::ExitStatusExt;
                Err(out.status.signal().expect("no code and no signal"))
            }
            #[cfg(not(unix))]
            {
                panic!("process ended without an exit code")
            }
        }
    };

    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        status,
    }
}

/// Assert that both programs agree on stdout, stderr and exit status.
fn assert_same(description: &str, input: &[u8]) {
    let c = run(&c_binary(), input);
    let r = run(&rust_binary(), input);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout differs for {description} (input {:?})\n  C:    {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(input),
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr differs for {description} (input {:?})\n  C:    {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(input),
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
    assert_eq!(
        c.status, r.status,
        "exit status differs for {description} (input {:?})\n  C:    {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(input),
        c.status,
        r.status
    );
}

// ===========================================================================
// The branch structure of c_src/src/main.c
//
//   main:  scanf("%d", &x);  if (x) good(); else bad();
//
// So the input classes are exactly the classes of `scanf("%d")` outcome
// crossed with "was the stored value zero or non-zero":
//
//   1. conversion never happens (EOF, matching failure) -> x keeps its
//      initialiser 0 -> bad()
//   2. conversion succeeds and yields 0                 -> bad()
//   3. conversion succeeds and yields non-zero          -> good()
//
// bad() calls alloca(10) but stores ten 4-byte ints through the pointer, so it
// writes 30 bytes past the request. That out-of-bounds write is the original
// defect and both programs must survive it identically.
// ===========================================================================

// --- class 1: no conversion, x stays 0, bad() runs -------------------------

#[test]
fn empty_input() {
    assert_same("empty input (immediate EOF)", b"");
}

#[test]
fn whitespace_only() {
    // %d skips whitespace then hits EOF: scanf returns EOF, x untouched.
    assert_same("spaces and newlines only", b"   \n\t \n");
    assert_same("newlines only", b"\n\n");
    assert_same("single space", b" ");
    assert_same("vertical tab and form feed", b"\x0b\x0c");
}

#[test]
fn matching_failure_non_numeric() {
    assert_same("letters", b"abc");
    assert_same("leading spaces then letters", b"  abc");
    assert_same("leading dot", b".5");
    assert_same("comma", b",");
    assert_same("hash", b"#1");
}

#[test]
fn matching_failure_sign_without_digits() {
    assert_same("bare minus", b"-");
    assert_same("bare plus", b"+");
    assert_same("minus then letter", b"-a");
    assert_same("plus then newline", b"+\n");
    assert_same("double minus", b"--1");
}

// --- class 2: conversion yields zero, bad() runs ---------------------------

#[test]
fn converts_to_zero() {
    assert_same("plain zero", b"0");
    assert_same("negative zero", b"-0");
    assert_same("positive zero", b"+0");
    assert_same("zero with trailing newline", b"0\n");
    assert_same("many leading zeros", b"0000000000000000000");
    // 0x is not a %d prefix: reads "0", leaves "x10" unread.
    assert_same("hex-looking literal", b"0x10");
    assert_same("zero then another number", b"0\n1\n");
    // 2^32 truncated to a 32-bit int is 0.
    assert_same("2^32 truncates to zero", b"4294967296");
    assert_same("-2^32 truncates to zero", b"-4294967296");
    // strtol saturates at LONG_MIN; (int)LONG_MIN == 0.
    assert_same("negative overflow saturates", b"-99999999999999999999");
}

// --- class 3: conversion yields non-zero, good() runs ---------------------

#[test]
fn converts_to_nonzero() {
    assert_same("one", b"1");
    assert_same("minus one", b"-1");
    assert_same("plus seven", b"+7");
    assert_same("leading zeros then digit", b"007");
    assert_same("leading whitespace then digit", b"   \n 5");
    assert_same("tab then digit", b"\t9");
    assert_same("digit then letters", b"1abc");
    assert_same("digit then space and more", b"42 99");
}

#[test]
fn integer_limits_and_overflow() {
    assert_same("INT_MAX", b"2147483647");
    assert_same("INT_MIN", b"-2147483648");
    // Past INT_MAX glibc's strtol still fits in a long, then narrows to int.
    assert_same("INT_MAX + 1", b"2147483648");
    assert_same("INT_MIN - 1", b"-2147483649");
    assert_same("2^32 + 1", b"4294967297");
    // Past LONG_MAX strtol saturates; (int)LONG_MAX == -1.
    assert_same("positive overflow saturates", b"99999999999999999999");
    assert_same("LONG_MAX", b"9223372036854775807");
    assert_same("LONG_MAX + 1", b"9223372036854775808");
    assert_same("LONG_MIN", b"-9223372036854775808");
}

// --- byte-level and stream-level edges ------------------------------------

#[test]
fn embedded_and_binary_bytes() {
    assert_same("leading NUL", b"\x00");
    assert_same("NUL then digit", b"\x005");
    assert_same("high bytes", b"\xff\xfe");
    assert_same("mixed binary", b"\x00\x01\xff\xfe1");
    assert_same("CR LF then digit", b"\r\n3");
}

#[test]
fn no_trailing_newline_vs_trailing_newline() {
    assert_same("no trailing newline", b"7");
    assert_same("trailing newline", b"7\n");
    assert_same("trailing CRLF", b"7\r\n");
}

#[test]
fn input_larger_than_the_pipe_buffer() {
    // 100k digits: one long conversion that saturates, plus a payload well
    // past the 64 KiB pipe buffer so the writer must not deadlock.
    let long_digits = vec![b'9'; 100_000];
    assert_same("100k nines", &long_digits);

    let mut long_zeros = vec![b'0'; 100_000];
    long_zeros.push(b'\n');
    assert_same("100k zeros", &long_zeros);

    // Unread remainder: conversion stops at the first letter.
    let mut short_then_junk = b"1".to_vec();
    short_then_junk.extend(std::iter::repeat(b'x').take(200_000));
    assert_same("digit then 200k unread bytes", &short_then_junk);

    // Leading whitespace larger than any internal buffer.
    let mut lots_of_space = vec![b' '; 100_000];
    lots_of_space.push(b'4');
    assert_same("100k spaces then digit", &lots_of_space);
}

#[test]
fn stdin_at_eof_immediately_from_dev_null() {
    // /dev/null is a real EOF-at-once stream rather than a pipe we close.
    #[cfg(unix)]
    {
        let mut results = Vec::new();
        for program in [c_binary(), rust_binary()] {
            let devnull = std::fs::File::open("/dev/null").expect("open /dev/null");
            let out = Command::new(&program)
                .stdin(Stdio::from(devnull))
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .expect("spawn with /dev/null stdin");
            results.push((out.stdout, out.stderr, out.status.code()));
        }
        assert_eq!(results[0], results[1], "differed with stdin=/dev/null");
    }
}

/// Run both programs with a stdout that has no reader at all, and with SIGPIPE
/// set to `sigpipe_handler` in the child just before `exec`, so both start from
/// an identical inherited disposition. Asserts the two agree.
///
/// A private FIFO is used rather than `Stdio::piped()`: the read end is opened
/// and closed by this process only, and `open` sets `O_CLOEXEC`, so no
/// concurrently spawned child can inherit it and keep the pipe alive. With
/// `Stdio::piped()` that inheritance does happen occasionally, which makes the
/// child's write succeed and the test flaky.
#[cfg(unix)]
fn assert_same_with_readerless_stdout(description: &str, sigpipe_handler: usize) {
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::OpenOptionsExt;
    use std::os::unix::process::CommandExt;
    use std::os::unix::process::ExitStatusExt;
    use std::sync::atomic::{AtomicU32, Ordering};

    const SIGPIPE: i32 = 13;
    const O_NONBLOCK: i32 = 0o4000; // Linux/macOS value
    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
        fn mkfifo(path: *const std::os::raw::c_char, mode: u32) -> i32;
    }

    static SEQ: AtomicU32 = AtomicU32::new(0);

    let mut results = Vec::new();
    for program in [c_binary(), rust_binary()] {
        // Unique FIFO path per invocation.
        let fifo = std::env::temp_dir().join(format!(
            "driver-difftest-{}-{}.fifo",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = std::fs::remove_file(&fifo);
        let c_path = std::ffi::CString::new(fifo.as_os_str().as_bytes()).expect("path has no NUL");
        let rc = unsafe { mkfifo(c_path.as_ptr(), 0o600) };
        assert_eq!(rc, 0, "mkfifo failed for {}", fifo.display());

        // Open the read end non-blocking (otherwise it waits for a writer), then
        // the write end, then drop the reader: the FIFO now has a writer and no
        // readers, which is exactly a broken pipe.
        let reader = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(O_NONBLOCK)
            .open(&fifo)
            .expect("open FIFO read end");
        let writer = std::fs::OpenOptions::new()
            .write(true)
            .open(&fifo)
            .expect("open FIFO write end");
        drop(reader);

        let mut command = Command::new(&program);
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::from(writer))
            .stderr(Stdio::piped());

        // Applied in the forked child, after fork and before exec, so the C and
        // Rust programs start from an identical SIGPIPE disposition.
        unsafe {
            command.pre_exec(move || {
                signal(SIGPIPE, sigpipe_handler);
                Ok(())
            });
        }

        let mut child = command.spawn().expect("spawn");

        let mut stdin = child.stdin.take().expect("stdin piped");
        let writer_thread = std::thread::spawn(move || {
            let _ = stdin.write_all(b"1\n");
        });

        let out = child.wait_with_output().expect("wait");
        let _ = writer_thread.join();
        let _ = std::fs::remove_file(&fifo);

        results.push((out.status.code(), out.status.signal(), out.stderr));
    }

    assert_eq!(
        results[0], results[1],
        "differed writing to a readerless stdout ({description})\n  C:    {:?}\n  Rust: {:?}",
        results[0], results[1]
    );
}

#[test]
#[cfg(unix)]
fn readerless_stdout_with_default_sigpipe() {
    // SIG_DFL: the C program is killed by signal 13. The translation must be
    // killed the same way, which means undoing the Rust runtime's automatic
    // SIG_IGN and restoring what was inherited.
    assert_same_with_readerless_stdout("SIGPIPE inherited as SIG_DFL", 0);
}

#[test]
#[cfg(unix)]
fn readerless_stdout_with_ignored_sigpipe() {
    // SIG_IGN: the write fails with EPIPE, nothing checks printf's return value,
    // and the C program exits 0. The translation must too.
    assert_same_with_readerless_stdout("SIGPIPE inherited as SIG_IGN", 1);
}

/// One test that walks a broad sweep of values through both programs, so a
/// regression in the `%d` conversion shows up even for values no hand-written
/// case above happens to name.
#[test]
fn swept_decimal_values() {
    let mut cases: Vec<String> = Vec::new();
    for v in [
        -1000i64, -257, -256, -129, -128, -2, -1, 0, 1, 2, 127, 128, 255, 256, 32767, 32768, 65535,
        65536, 1_000_000, 2_147_483_646, 2_147_483_647,
    ] {
        cases.push(v.to_string());
        cases.push(format!("{}\n", v));
        cases.push(format!("  {}  ", v));
    }
    // Values whose low 32 bits are zero: these flip main's branch.
    for v in [4_294_967_296i64, 8_589_934_592, -4_294_967_296] {
        cases.push(v.to_string());
    }
    for case in cases {
        assert_same(&format!("swept value {:?}", case), case.as_bytes());
    }
}
