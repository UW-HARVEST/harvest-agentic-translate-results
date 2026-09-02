//! Differential tests: run the C reference binary and the Rust binary as
//! subprocesses with identical stdin, then compare stdout, stderr and exit
//! status (including death-by-signal) byte for byte.
//!
//! The Rust program is never used as a library — only the built executable is
//! driven, exactly the way a shell would drive it.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

// ---------------------------------------------------------------------------
// Locating the two executables
// ---------------------------------------------------------------------------

/// Path to the Rust executable for the profile the tests were built with.
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn repo_root() -> PathBuf {
    // .../translation/  ->  .../
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the C reference executable, building it with CMake if necessary.
fn c_bin() -> PathBuf {
    let c_src = repo_root().join("c_src");
    let build = c_src.join("build");
    let exe = build.join("driver");
    if exe.is_file() {
        return exe;
    }

    std::fs::create_dir_all(&build).expect("could not create c_src/build");
    run_build(Command::new("cmake").arg("..").current_dir(&build), "cmake ..");
    run_build(
        Command::new("cmake")
            .args(["--build", "."])
            .current_dir(&build),
        "cmake --build .",
    );

    assert!(
        exe.is_file(),
        "C reference binary was not produced at {}",
        exe.display()
    );
    exe
}

fn run_build(cmd: &mut Command, what: &str) {
    let out = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn `{what}`: {e}"));
    assert!(
        out.status.success(),
        "`{what}` failed\n--- stdout ---\n{}\n--- stderr ---\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

// ---------------------------------------------------------------------------
// Running one program
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq)]
struct Outcome {
    /// `Some(n)` for normal exit, `None` when killed by a signal.
    code: Option<i32>,
    /// `Some(n)` when killed by signal `n`, else `None`.
    signal: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Outcome")
            .field("code", &self.code)
            .field("signal", &self.signal)
            .field("stdout", &Pretty(&self.stdout))
            .field("stderr", &Pretty(&self.stderr))
            .finish()
    }
}

/// Renders bytes readably: long runs are collapsed, non-printables escaped.
struct Pretty<'a>(&'a [u8]);

impl std::fmt::Debug for Pretty<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} bytes: \"", self.0.len())?;
        let mut i = 0;
        while i < self.0.len() {
            let b = self.0[i];
            let mut run = 1;
            while i + run < self.0.len() && self.0[i + run] == b {
                run += 1;
            }
            if run > 4 {
                write!(f, "{}x{}", escape(b), run)?;
            } else {
                for _ in 0..run {
                    write!(f, "{}", escape(b))?;
                }
            }
            i += run;
        }
        write!(f, "\"")
    }
}

fn escape(b: u8) -> String {
    match b {
        b'\n' => "\\n".to_string(),
        b'\r' => "\\r".to_string(),
        b'\t' => "\\t".to_string(),
        0x20..=0x7e => (b as char).to_string(),
        _ => format!("\\x{b:02x}"),
    }
}

/// Several inputs make both programs die from SIGSEGV. Writing a core file
/// each time dominates the runtime of this suite, so core dumps are disabled —
/// identically for both children, and without affecting the observed exit
/// status (`ExitStatus::signal` does not expose the core-dump flag).
#[cfg(unix)]
fn disable_core_dumps() {
    #[repr(C)]
    struct RLimit {
        cur: u64,
        max: u64,
    }
    const RLIMIT_CORE: i32 = 4;
    extern "C" {
        #[link_name = "setrlimit"]
        fn libc_setrlimit(resource: i32, rlim: *const RLimit) -> i32;
    }
    let zero = RLimit { cur: 0, max: 0 };
    unsafe {
        libc_setrlimit(RLIMIT_CORE, &zero);
    }
}

fn run(bin: &Path, stdin_bytes: &[u8]) -> Outcome {
    let mut cmd = Command::new(bin);
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    unsafe {
        use std::os::unix::process::CommandExt;
        cmd.pre_exec(|| {
            disable_core_dumps();
            Ok(())
        });
    }

    let mut child = cmd
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    // The child may die (SIGSEGV) before draining stdin; a failed write is not
    // an error for the purposes of this comparison.
    {
        let mut sink = child.stdin.take().expect("stdin was piped");
        let _ = sink.write_all(stdin_bytes);
        let _ = sink.flush();
    }

    let out = child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("failed to wait on {}: {e}", bin.display()));

    #[cfg(unix)]
    let signal = {
        use std::os::unix::process::ExitStatusExt;
        out.status.signal()
    };
    #[cfg(not(unix))]
    let signal: Option<i32> = None;

    Outcome {
        code: out.status.code(),
        signal,
        stdout: out.stdout,
        stderr: out.stderr,
    }
}

// ---------------------------------------------------------------------------
// The assertion every test funnels through
// ---------------------------------------------------------------------------

fn assert_matches(label: &str, stdin_bytes: &[u8]) {
    let c = run(&c_bin(), stdin_bytes);
    let r = run(&rust_bin(), stdin_bytes);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout differs for {label} (stdin = {:?})\n  C  : {:?}\n  Rust: {:?}",
        Pretty(stdin_bytes),
        Pretty(&c.stdout),
        Pretty(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr differs for {label} (stdin = {:?})\n  C  : {:?}\n  Rust: {:?}",
        Pretty(stdin_bytes),
        Pretty(&c.stderr),
        Pretty(&r.stderr)
    );
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "exit status differs for {label} (stdin = {:?})\n  C  : {:?}\n  Rust: {:?}",
        Pretty(stdin_bytes),
        c,
        r
    );
}

fn assert_all(cases: &[(&str, &[u8])]) {
    for (label, input) in cases {
        assert_matches(label, input);
    }
}

// ===========================================================================
// Phase A — both programs are runnable
// ===========================================================================

#[test]
fn both_binaries_exist_and_run() {
    let c = c_bin();
    let r = rust_bin();
    assert!(c.is_file(), "missing C binary at {}", c.display());
    assert!(r.is_file(), "missing Rust binary at {}", r.display());
    // A trivially valid input must succeed for both.
    let oc = run(&c, b"1\n");
    let or = run(&r, b"1\n");
    assert_eq!(oc.code, Some(0), "C exited unexpectedly: {oc:?}");
    assert_eq!(or.code, Some(0), "Rust exited unexpectedly: {or:?}");
    assert_eq!(oc.stdout, b"A\n".to_vec());
    assert_eq!(or.stdout, b"A\n".to_vec());
}

// ===========================================================================
// Phase B — the branches the C code actually takes
// ===========================================================================

/// `fgets` returns NULL only on immediate EOF. That takes the
/// `printLine("fgets() failed.")` branch, leaves `data == -1`, and then faults
/// inside `strncpy`. Because stdout is a pipe (fully buffered in C), the
/// message never reaches the pipe.
#[test]
fn fgets_failure_path_empty_stdin() {
    assert_matches("empty stdin", b"");
}

/// `data == 0`: `strncpy(dest, source, 0)` copies nothing, `dest[0] = '\0'`,
/// so `printLine` emits just the newline.
#[test]
fn zero_length_copy() {
    assert_all(&[
        ("\"0\"", b"0\n"),
        ("\"00\"", b"00\n"),
        ("\"-0\"", b"-0\n"),
        ("\"+0\"", b"+0\n"),
        ("bare newline", b"\n"),
        ("non-numeric", b"abc\n"),
        ("whitespace only", b"   \n"),
        ("lone minus", b"-\n"),
        ("lone plus", b"+\n"),
        ("leading dot", b".5\n"),
    ]);
}

/// A single item.
#[test]
fn single_item() {
    assert_matches("\"1\"", b"1\n");
}

/// Every in-range length the `data < 100` branch handles, 0..=99.
#[test]
fn every_in_range_length() {
    for n in 0..=99u32 {
        let input = format!("{n}\n");
        assert_matches(&format!("data = {n}"), input.as_bytes());
    }
}

/// The maximum the code copies, and the boundary just past it where the
/// `if (data < 100)` guard skips the copy entirely and `dest` stays empty.
#[test]
fn upper_boundary_of_copy() {
    assert_all(&[
        ("data = 98", b"98\n"),
        ("data = 99 (max copied)", b"99\n"),
        ("data = 100 (guard fails)", b"100\n"),
        ("data = 101", b"101\n"),
        ("data = 110", b"110\n"),
        ("data = 200", b"200\n"),
        ("data = 2147483647", b"2147483647\n"),
    ]);
}

/// Negative `data` makes the length argument of `strncpy` an enormous
/// `size_t`, which runs off the destination buffer and kills the process.
#[test]
fn negative_length_faults() {
    assert_all(&[
        ("data = -1", b"-1\n"),
        ("data = -5", b"-5\n"),
        ("data = -99", b"-99\n"),
        ("data = -100", b"-100\n"),
        ("data = -2147483648", b"-2147483648\n"),
        ("leading spaces then -1", b"  -1\n"),
        ("leading tab then -3", b"\t-3\n"),
    ]);
}

/// `atoi` truncates the `long` result to `int`, so values above INT_MAX wrap —
/// including into negative territory, which then faults.
#[test]
fn atoi_truncation_to_int() {
    assert_all(&[
        ("2147483648 wraps to INT_MIN -> fault", b"2147483648\n"),
        ("4294967296 wraps to 0", b"4294967296\n"),
        ("4294967396 wraps to 100", b"4294967396\n"),
        ("4294967297 wraps to 1", b"4294967297\n"),
        ("99999999999", b"99999999999\n"),
        ("1000000000000", b"1000000000000\n"),
        ("9999999999999 (13 digits)", b"9999999999999\n"),
        ("-100000000000", b"-100000000000\n"),
        ("-999999999999", b"-999999999999\n"),
    ]);
}

/// `atoi` accepts leading whitespace and an optional sign, and stops at the
/// first non-digit.
#[test]
fn atoi_prefix_parsing() {
    assert_all(&[
        ("two leading spaces", b"  12\n"),
        ("leading tab", b"\t12\n"),
        ("leading vertical tab", b"\x0b12\n"),
        ("leading form feed", b"\x0c12\n"),
        ("leading CR", b"\r12\n"),
        ("explicit plus", b"+7\n"),
        ("digits then letters", b"3abc\n"),
        ("digits then letters then digits", b"12abc34\n"),
        ("exponent-looking", b"1e2\n"),
        ("digit space digit", b"1 2\n"),
        ("double minus", b"--5\n"),
        ("plus then minus", b"+-5\n"),
        ("minus then plus", b"-+5\n"),
        ("zero padded 99", b"000000000099\n"),
        ("zero padded 100", b"000000000100\n"),
        ("hex-looking", b"0x10\n"),
        ("sign only then newline", b"-\n"),
        ("high-bit byte first", b"\xa05\n"),
        ("high-bit bytes only", b"\xff\xfe\n"),
    ]);
}

/// `fgets` stops at the newline and does not read past it, so trailing lines
/// are ignored entirely.
#[test]
fn fgets_stops_at_newline() {
    assert_all(&[
        ("two lines, first used", b"5\n7\n"),
        ("three empty lines", b"\n\n\n"),
        ("newline then number", b"\n5\n"),
        ("CRLF terminated", b"5\r\n"),
        ("second line is garbage", b"7\nnot a number\n"),
    ]);
}

/// `fgets(buf, 14, ...)` reads at most 13 bytes, so a longer first line is
/// truncated before `atoi` ever sees it.
#[test]
fn fgets_thirteen_byte_window() {
    assert_all(&[
        ("exactly 13 digits", b"1234567890123\n"),
        ("14 digits, 14th dropped", b"12345678901234\n"),
        ("16 digits", b"1234567890123456\n"),
        ("truncation changes value", b"123456789012399\n"),
        ("13 chars, no newline", b"9999999999999"),
        ("long unterminated run", b"9999999999999999"),
        ("\"5\" then 20 spaces", b"5                    "),
    ]);
}

/// No trailing newline at all — `fgets` still returns the bytes it read.
#[test]
fn input_without_trailing_newline() {
    assert_all(&[
        ("\"5\"", b"5"),
        ("\"0\"", b"0"),
        ("\"99\"", b"99"),
        ("\"100\"", b"100"),
        ("\"-1\"", b"-1"),
        ("single letter", b"z"),
    ]);
}

// ===========================================================================
// Phase C — paths not covered above
// ===========================================================================

/// Embedded NUL bytes: `fgets` copies them into the buffer, then `atoi`
/// treats the NUL as the end of the string.
#[test]
fn embedded_nul_bytes() {
    assert_all(&[
        ("NUL then digit", b"\x005\n"),
        ("digit then NUL then digit", b"5\x006\n"),
        ("NUL only, no newline", b"\x00"),
        ("space NUL digits", b" \x0012\n"),
        ("NUL after 99", b"99\x00\n"),
        ("all NULs", b"\x00\x00\x00\x00\n"),
    ]);
}

/// stdin closed / at EOF by other means still takes the `fgets` NULL path.
#[test]
fn stdin_immediately_at_eof() {
    assert_matches("zero-byte stdin", b"");
}

/// stdout closed before the program writes: the write fails but is not
/// checked, so both programs still exit 0.
#[test]
#[cfg(unix)]
fn stdout_closed() {
    let expect_same = |input: &[u8]| {
        let mut codes = Vec::new();
        for bin in [c_bin(), rust_bin()] {
            let mut child = Command::new(&bin)
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn()
                .expect("spawn");
            {
                let mut sink = child.stdin.take().unwrap();
                let _ = sink.write_all(input);
            }
            let out = child.wait_with_output().expect("wait");
            use std::os::unix::process::ExitStatusExt;
            codes.push((out.status.code(), out.status.signal(), out.stderr));
        }
        assert_eq!(codes[0], codes[1], "stdout-to-/dev/null differs");
    };
    expect_same(b"50\n");
    expect_same(b"");
}

/// A reader that goes away mid-run: C dies from SIGPIPE, so the Rust program
/// must not silently swallow EPIPE.
#[test]
#[cfg(unix)]
fn broken_pipe_kills_both() {
    use std::os::unix::io::FromRawFd;
    use std::os::unix::process::ExitStatusExt;

    let mut results = Vec::new();
    for bin in [c_bin(), rust_bin()] {
        // Build a pipe, hand the write end to the child, then drop the read
        // end so the very first write faces a broken pipe.
        let mut fds = [0i32; 2];
        let rc = unsafe { libc_pipe(fds.as_mut_ptr()) };
        assert_eq!(rc, 0, "pipe() failed");
        let (read_fd, write_fd) = (fds[0], fds[1]);

        let stdout = unsafe { Stdio::from_raw_fd(write_fd) };
        let mut child = Command::new(&bin)
            .stdin(Stdio::piped())
            .stdout(stdout)
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn");

        // Close our copy of the read end: nothing will ever read the pipe.
        unsafe { libc_close(read_fd) };

        {
            let mut sink = child.stdin.take().unwrap();
            let _ = sink.write_all(b"50\n");
        }
        let status = child.wait().expect("wait");
        results.push((status.code(), status.signal()));
    }
    assert_eq!(
        results[0], results[1],
        "broken-pipe exit status differs: C = {:?}, Rust = {:?}",
        results[0], results[1]
    );
}

extern "C" {
    #[link_name = "pipe"]
    fn libc_pipe(fds: *mut i32) -> i32;
    #[link_name = "close"]
    fn libc_close(fd: i32) -> i32;
}

/// Deterministic sweep over byte soup, exercising combinations no hand-written
/// case would reach.
#[test]
fn randomized_sweep() {
    const ALPHABET: &[u8] = b"0123456789 \t\n+-abcXY\x00\r\x0b\x0c\xff.eE";
    // Small xorshift so the sequence is fixed and reproducible.
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    for i in 0..150 {
        let len = (next() % 21) as usize;
        let input: Vec<u8> = (0..len)
            .map(|_| ALPHABET[(next() % ALPHABET.len() as u64) as usize])
            .collect();
        assert_matches(&format!("random case #{i}"), &input);
    }
}

/// Numeric sweep across the interesting neighbourhoods of the guard, of
/// INT_MAX and of the sign boundary.
#[test]
fn numeric_sweep() {
    let mut values: Vec<i64> = Vec::new();
    values.extend(-8..=8);
    values.extend(90..=110);
    values.extend([
        127, 128, 255, 256, 999, 1000,
        2147483646, 2147483647, 2147483648, 2147483649,
        4294967295, 4294967296, 4294967297,
        4294967395, 4294967396, 4294967397,
        -2147483647, -2147483648, -2147483649,
        999999999999, -99999999999,
    ]);
    for v in values {
        for text in [format!("{v}\n"), format!("{v}"), format!("  {v}\n")] {
            assert_matches(&format!("value {v} as {text:?}"), text.as_bytes());
        }
    }
}
