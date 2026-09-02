// Differential tests: run the C reference binary and the Rust binary as
// subprocesses with identical argv, and require byte-identical stdout, stderr
// and exit status for every input.
//
// The Rust code is never called as a library.  Both programs are driven exactly
// the way a shell would drive them, because that is how the translation is
// graded.

use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Locating and building the two binaries
// ---------------------------------------------------------------------------

/// Path to the Rust binary under test, built by Cargo for this test run.
fn rust_binary() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

fn repo_root() -> PathBuf {
    // translation/ -> repository root
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the C reference binary, building it with CMake on first use so that
/// a bare `cargo test` works from a clean checkout.
fn c_binary() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let root = repo_root();
        let c_src = root.join("c_src");
        let build = c_src.join("build");
        let bin = build.join("driver");

        if !bin.exists() {
            let configure = Command::new("cmake")
                .arg("-S")
                .arg(&c_src)
                .arg("-B")
                .arg(&build)
                .output()
                .expect("failed to invoke cmake; is CMake installed?");
            assert!(
                configure.status.success(),
                "cmake configure failed:\n{}\n{}",
                String::from_utf8_lossy(&configure.stdout),
                String::from_utf8_lossy(&configure.stderr),
            );

            let compile = Command::new("cmake")
                .arg("--build")
                .arg(&build)
                .output()
                .expect("failed to invoke cmake --build");
            assert!(
                compile.status.success(),
                "cmake build failed:\n{}\n{}",
                String::from_utf8_lossy(&compile.stdout),
                String::from_utf8_lossy(&compile.stderr),
            );
        }

        assert!(
            bin.exists(),
            "C reference binary missing at {}; build c_src first",
            bin.display()
        );
        bin
    })
}

// ---------------------------------------------------------------------------
// Running a program and capturing everything observable
// ---------------------------------------------------------------------------

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
            Ok(code) => format!("exit {code}"),
            Err(sig) => format!("signal {sig}"),
        };
        write!(
            f,
            "{{ status: {status}, stdout: {:?}, stderr: {:?} }}",
            Bytes(&self.stdout),
            Bytes(&self.stderr)
        )
    }
}

/// Byte-exact, human-readable rendering so assertion failures show the actual
/// bytes (including missing or extra trailing newlines).
struct Bytes<'a>(&'a [u8]);

impl std::fmt::Debug for Bytes<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"")?;
        for &b in self.0 {
            match b {
                b'\n' => write!(f, "\\n")?,
                b'\r' => write!(f, "\\r")?,
                b'\t' => write!(f, "\\t")?,
                b'"' => write!(f, "\\\"")?,
                b'\\' => write!(f, "\\\\")?,
                0x20..=0x7e => write!(f, "{}", b as char)?,
                _ => write!(f, "\\x{b:02x}")?,
            }
        }
        write!(f, "\"")
    }
}

fn os(bytes: &[u8]) -> OsString {
    OsStr::from_bytes(bytes).to_owned()
}

fn run(program: &Path, args: &[&[u8]]) -> Outcome {
    let output = Command::new(program)
        .args(args.iter().map(|a| os(a)))
        .stdin(Stdio::null())
        .output()
        .unwrap_or_else(|e| panic!("failed to run {}: {e}", program.display()));

    Outcome {
        stdout: output.stdout,
        stderr: output.stderr,
        status: exit_status(&output.status),
    }
}

fn exit_status(status: &std::process::ExitStatus) -> Result<i32, i32> {
    use std::os::unix::process::ExitStatusExt;
    match status.code() {
        Some(code) => Ok(code),
        None => Err(status.signal().expect("process neither exited nor signalled")),
    }
}

/// Assert that the C and Rust programs agree on stdout, stderr and exit status.
#[track_caller]
fn assert_same(args: &[&[u8]]) {
    let expected = run(c_binary(), args);
    let actual = run(rust_binary(), args);

    if expected != actual {
        let rendered: Vec<_> = args.iter().map(|a| Bytes(a)).collect();
        panic!(
            "output differs for argv {rendered:?}\n  C    (reference): {expected:?}\n  Rust (translation): {actual:?}"
        );
    }
}

/// Same as [`assert_same`], but also pins the C program's observed behaviour so
/// the test documents which branch of the C source is being exercised.
#[track_caller]
fn assert_same_and_expect(args: &[&[u8]], stdout: &[u8], code: i32) {
    assert_same(args);
    let observed = run(c_binary(), args);
    let rendered: Vec<_> = args.iter().map(|a| Bytes(a)).collect();
    assert_eq!(
        Bytes(&observed.stdout).to_string_repr(),
        Bytes(stdout).to_string_repr(),
        "unexpected C stdout for argv {rendered:?}"
    );
    assert_eq!(
        observed.status,
        Ok(code),
        "unexpected C exit status for argv {rendered:?}"
    );
    assert!(
        observed.stderr.is_empty(),
        "C wrote to stderr for argv {rendered:?}: {:?}",
        Bytes(&observed.stderr)
    );
}

impl Bytes<'_> {
    fn to_string_repr(&self) -> String {
        format!("{self:?}")
    }
}

// Messages the C program can print, copied verbatim from c_src/src/main.c.
const USAGE: &[u8] =
    b"Error: there should be one to three arguments passed:\n<string> [start] [stop]\n";
const BAD_SECOND: &[u8] = b"Second argument must be an integer!"; // no newline
const BAD_THIRD: &[u8] = b"Third argument must be an integer!"; // unreachable
const START_OFF_END: &[u8] = b"Error: start is off the end of the string!\n";
const STOP_OFF_END: &[u8] = b"Error: stop is off the end of the string!\n";
const STOP_BEFORE_START: &[u8] = b"Error: stop must come after start!\n";

// ---------------------------------------------------------------------------
// Phase A sanity: both binaries exist and are runnable
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_are_runnable() {
    for bin in [c_binary(), rust_binary()] {
        let out = run(bin, &[b"hello", b"1", b"3"]);
        assert_eq!(
            out.status,
            Ok(0),
            "{} did not exit 0 on a valid input: {out:?}",
            bin.display()
        );
        assert_eq!(out.stdout, b"el\n", "{} produced {out:?}", bin.display());
    }
}

// ---------------------------------------------------------------------------
// Argument-count branches: `if ((argc > 4) || (argc == 1))`
// ---------------------------------------------------------------------------

#[test]
fn argc_one_prints_usage() {
    assert_same_and_expect(&[], USAGE, 1);
}

#[test]
fn argc_above_four_prints_usage() {
    // argc == 5, 6, 7, 8 -- every count past the maximum the code handles.
    assert_same_and_expect(&[b"a", b"0", b"1", b"x"], USAGE, 1);
    assert_same_and_expect(&[b"a", b"0", b"1", b"x", b"y"], USAGE, 1);
    assert_same_and_expect(&[b"a", b"0", b"1", b"x", b"y", b"z"], USAGE, 1);
    // Even otherwise-valid arguments lose to the count check.
    assert_same_and_expect(&[b"hello", b"1", b"3", b""], USAGE, 1);
}

// ---------------------------------------------------------------------------
// argc == 2: start = 0, stop = len
// ---------------------------------------------------------------------------

#[test]
fn single_argument_prints_whole_string() {
    assert_same_and_expect(&[b"hello"], b"hello\n", 0);
    assert_same_and_expect(&[b"a"], b"a\n", 0);
    // Empty subject: len == 0, so only the trailing newline is printed.
    assert_same_and_expect(&[b""], b"\n", 0);
    assert_same_and_expect(&[b" "], b" \n", 0);
    assert_same_and_expect(&[b"hello world"], b"hello world\n", 0);
}

#[test]
fn single_argument_passes_bytes_through_unchanged() {
    // Non-UTF-8 argv must survive verbatim: the C code copies bytes.
    assert_same_and_expect(&[b"\xff\xfe\xfd"], b"\xff\xfe\xfd\n", 0);
    assert_same_and_expect(&[b"\xc3\xa9\xc3\xa9"], b"\xc3\xa9\xc3\xa9\n", 0);
    assert_same_and_expect(&[b"a\tb\nc"], b"a\tb\nc\n", 0);
    assert_same_and_expect(&[b"\x01\x02\x7f"], b"\x01\x02\x7f\n", 0);
    // A subject that looks like a flag is still just a string.
    assert_same_and_expect(&[b"-5"], b"-5\n", 0);
    assert_same_and_expect(&[b"--help"], b"--help\n", 0);
}

// ---------------------------------------------------------------------------
// `if (end == argv[2])`: the second argument is not an integer.
// Note: the C code prints this message WITHOUT a trailing newline.
// ---------------------------------------------------------------------------

#[test]
fn second_argument_not_an_integer() {
    for arg in [
        &b"abc"[..],
        b"",
        b" ",
        b"   ",
        b"\t",
        b"\n",
        b"+",
        b"-",
        b"++1",
        b"--1",
        b".9",
        b"x1",
        b",",
        b"\xff",
        b"\xa01", // non-ASCII byte: not isspace in the C locale
        b"e5",
    ] {
        assert_same_and_expect(&[b"hello", arg], BAD_SECOND, 1);
    }
}

#[test]
fn second_argument_check_precedes_third_argument_handling() {
    // Validation order: the argv[2] check runs before argv[3] is looked at, so
    // a bad second argument wins even when the third is also nonsense.
    assert_same_and_expect(&[b"hello", b"abc", b"zzz"], BAD_SECOND, 1);
    assert_same_and_expect(&[b"hello", b"", b"3"], BAD_SECOND, 1);
    assert_same_and_expect(&[b"hello", b"+", b"-1"], BAD_SECOND, 1);
}

// ---------------------------------------------------------------------------
// `if (start > len)`: an int/size_t comparison, so negatives become huge.
// ---------------------------------------------------------------------------

#[test]
fn start_off_the_end_of_the_string() {
    // Plainly too large.
    assert_same_and_expect(&[b"hello", b"6"], START_OFF_END, 1);
    assert_same_and_expect(&[b"hello", b"100"], START_OFF_END, 1);
    assert_same_and_expect(&[b"", b"1"], START_OFF_END, 1);
    // Negative: converted to size_t, so it compares greater than len.
    assert_same_and_expect(&[b"hello", b"-1"], START_OFF_END, 1);
    assert_same_and_expect(&[b"hello", b"-100"], START_OFF_END, 1);
    assert_same_and_expect(&[b"", b"-1"], START_OFF_END, 1);
    // strtol saturates at LONG_MAX; (int)LONG_MAX == -1, which is "off the end".
    assert_same_and_expect(&[b"hello", b"9223372036854775807"], START_OFF_END, 1);
    assert_same_and_expect(&[b"hello", b"99999999999999999999999999"], START_OFF_END, 1);
    // (int)2147483648L == INT_MIN, also negative.
    assert_same_and_expect(&[b"hello", b"2147483648"], START_OFF_END, 1);
    assert_same_and_expect(&[b"hello", b"-2147483649"], START_OFF_END, 1);
}

#[test]
fn start_at_the_boundaries_is_accepted() {
    // start == 0 and start == len are both in range; start == len prints nothing
    // but the newline, because stop - start == 0.
    assert_same_and_expect(&[b"hello", b"0"], b"hello\n", 0);
    assert_same_and_expect(&[b"hello", b"4"], b"o\n", 0);
    assert_same_and_expect(&[b"hello", b"5"], b"\n", 0);
    assert_same_and_expect(&[b"", b"0"], b"\n", 0);
    assert_same_and_expect(&[b"", b"-0"], b"\n", 0);
}

#[test]
fn start_uses_strtol_parsing_rules() {
    // Leading whitespace is skipped, a sign is honoured, and parsing stops at
    // the first non-digit without being an error.
    assert_same_and_expect(&[b"hello", b" 2"], b"llo\n", 0);
    assert_same_and_expect(&[b"hello", b"  2"], b"llo\n", 0);
    assert_same_and_expect(&[b"hello", b"\t2"], b"llo\n", 0);
    assert_same_and_expect(&[b"hello", b"\n2"], b"llo\n", 0);
    assert_same_and_expect(&[b"hello", b"\r2"], b"llo\n", 0);
    assert_same_and_expect(&[b"hello", b"\x0b2"], b"llo\n", 0);
    assert_same_and_expect(&[b"hello", b"\x0c2"], b"llo\n", 0);
    assert_same_and_expect(&[b"hello", b"+2"], b"llo\n", 0);
    assert_same_and_expect(&[b"hello", b"2abc"], b"llo\n", 0);
    assert_same_and_expect(&[b"hello", b"2 "], b"llo\n", 0);
    assert_same_and_expect(&[b"hello", b"2.9"], b"llo\n", 0);
    assert_same_and_expect(&[b"hello", b"002"], b"llo\n", 0);
    // Base 10 only: "0x2" parses as 0, stopping at 'x'.
    assert_same_and_expect(&[b"hello", b"0x2"], b"hello\n", 0);
    // long -> int truncation: 2^32 + 2 keeps only the low 32 bits.
    assert_same_and_expect(&[b"hello", b"4294967298"], b"llo\n", 0);
    assert_same_and_expect(&[b"hello", b"4294967296"], b"hello\n", 0);
    // (int)LONG_MIN == 0, so this saturating-negative value acts as start 0.
    assert_same_and_expect(&[b"hello", b"-9223372036854775808"], b"hello\n", 0);
    assert_same_and_expect(&[b"hello", b"-4294967296"], b"hello\n", 0);
}

// ---------------------------------------------------------------------------
// `if (stop > len)` and `if (stop <= start)`
// ---------------------------------------------------------------------------

#[test]
fn stop_off_the_end_of_the_string() {
    assert_same_and_expect(&[b"hello", b"0", b"6"], STOP_OFF_END, 1);
    assert_same_and_expect(&[b"hello", b"0", b"100"], STOP_OFF_END, 1);
    // Negative stop becomes huge under the int/size_t comparison.
    assert_same_and_expect(&[b"hello", b"0", b"-1"], STOP_OFF_END, 1);
    assert_same_and_expect(&[b"hello", b"2", b"-5"], STOP_OFF_END, 1);
    // Saturation and truncation, as for start.
    assert_same_and_expect(&[b"hello", b"0", b"9223372036854775807"], STOP_OFF_END, 1);
    assert_same_and_expect(&[b"hello", b"0", b"2147483648"], STOP_OFF_END, 1);
    assert_same_and_expect(&[b"", b"0", b"-1"], STOP_OFF_END, 1);
}

#[test]
fn stop_must_come_after_start() {
    assert_same_and_expect(&[b"hello", b"2", b"2"], STOP_BEFORE_START, 1);
    assert_same_and_expect(&[b"hello", b"3", b"2"], STOP_BEFORE_START, 1);
    assert_same_and_expect(&[b"hello", b"0", b"0"], STOP_BEFORE_START, 1);
    assert_same_and_expect(&[b"hello", b"5", b"5"], STOP_BEFORE_START, 1);
    // Empty subject: stop can only be 0, which never exceeds start.
    assert_same_and_expect(&[b"", b"0", b"0"], STOP_BEFORE_START, 1);
}

#[test]
fn non_integer_third_argument_falls_through_to_the_ordering_check() {
    // The `end == argv[3]` guard is dead (see the test below), so an unparsable
    // third argument yields stop == 0 and trips "stop must come after start!".
    for arg in [
        &b"abc"[..],
        b"",
        b" ",
        b"+",
        b"-",
        b"xyz",
        b"\xff",
        b".5",
        b"e2",
    ] {
        assert_same_and_expect(&[b"hello", b"0", arg], STOP_BEFORE_START, 1);
        assert_same_and_expect(&[b"hello", b"3", arg], STOP_BEFORE_START, 1);
    }
    // "0x3" parses as 0 for the same reason.
    assert_same_and_expect(&[b"hello", b"0", b"0x3"], STOP_BEFORE_START, 1);
    // (int)LONG_MIN == 0 -> stop 0.
    assert_same_and_expect(&[b"hello", b"0", b"-9223372036854775808"], STOP_BEFORE_START, 1);
    assert_same_and_expect(&[b"hello", b"0", b"4294967296"], STOP_BEFORE_START, 1);
}

#[test]
fn third_argument_error_message_is_unreachable() {
    // `stop = strtol(argv[3], NULL, 10)` passes NULL for endptr, so the
    // following `end == argv[3]` test reads the pointer left behind by the
    // argv[2] conversion.  That pointer always lies inside argv[2], so the
    // comparison can never hold and the message can never be printed.
    // Sweep the inputs most likely to trigger it and confirm neither program
    // ever emits it.
    let subjects: &[&[u8]] = &[b"", b"a", b"hello", b"0123456789"];
    let nums: &[&[u8]] = &[
        b"", b"0", b"1", b"2", b"5", b"9", b"10", b"-1", b"abc", b"+", b"-",
        b" 3", b"3x", b"0x3", b"2147483648", b"4294967296",
        b"9223372036854775807", b"-9223372036854775808",
    ];
    for subject in subjects {
        for start in nums {
            for stop in nums {
                let args = [*subject, *start, *stop];
                let out = run(c_binary(), &args);
                assert!(
                    !out.stdout.starts_with(BAD_THIRD),
                    "C unexpectedly reached the third-argument message for {args:?}"
                );
                assert_same(&args);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// The success path: printf("%.*s\n", stop - start, argv[1] + start)
// ---------------------------------------------------------------------------

#[test]
fn valid_ranges_print_the_substring() {
    assert_same_and_expect(&[b"hello", b"0", b"5"], b"hello\n", 0);
    assert_same_and_expect(&[b"hello", b"0", b"1"], b"h\n", 0);
    assert_same_and_expect(&[b"hello", b"1", b"3"], b"el\n", 0);
    assert_same_and_expect(&[b"hello", b"4", b"5"], b"o\n", 0);
    assert_same_and_expect(&[b"a", b"0", b"1"], b"a\n", 0);
    assert_same_and_expect(&[b"0123456789", b"3", b"7"], b"3456\n", 0);
    // Byte transparency on the success path too.
    assert_same_and_expect(&[b"\xff\xfe\xfd", b"1", b"3"], b"\xfe\xfd\n", 0);
    assert_same_and_expect(&[b"a\tb\nc", b"1", b"4"], b"\tb\n\n", 0);
}

#[test]
fn every_valid_range_of_a_short_string() {
    // Exhaustive over the whole index space, a little past the ends, for
    // subjects of several lengths -- this covers each of the four outcomes
    // (print, start off end, stop off end, stop <= start) many times over.
    for subject in [&b""[..], b"a", b"ab", b"abcdefgh"] {
        let n = subject.len() as i64;
        for start in -2..=(n + 2) {
            let s = start.to_string();
            assert_same(&[subject, s.as_bytes()]);
            for stop in -2..=(n + 2) {
                let t = stop.to_string();
                assert_same(&[subject, s.as_bytes(), t.as_bytes()]);
            }
        }
    }
}

#[test]
fn long_subject_strings() {
    // Well past any stdio buffer boundary, and right at the ends of the string.
    let long = vec![b'x'; 100_000];
    let mut mixed = Vec::with_capacity(70_000);
    for i in 0..70_000u32 {
        mixed.push((i % 251) as u8 + 1); // never 0: argv cannot contain NUL
    }
    for subject in [&long[..], &mixed[..]] {
        let n = subject.len();
        assert_same(&[subject]);
        assert_same(&[subject, b"0"]);
        assert_same(&[subject, n.to_string().as_bytes()]);
        assert_same(&[subject, (n - 1).to_string().as_bytes()]);
        assert_same(&[subject, (n + 1).to_string().as_bytes()]);
        assert_same(&[subject, b"0", n.to_string().as_bytes()]);
        assert_same(&[
            subject,
            (n - 3).to_string().as_bytes(),
            n.to_string().as_bytes(),
        ]);
        assert_same(&[subject, b"0", (n + 1).to_string().as_bytes()]);
    }
}

// ---------------------------------------------------------------------------
// Observable process behaviour beyond the captured streams
// ---------------------------------------------------------------------------

#[test]
fn stdout_closed_early_kills_both_programs_the_same_way() {
    // The C program keeps the default SIGPIPE disposition and dies from signal
    // 13 when its reader goes away.  The Rust runtime ignores SIGPIPE unless
    // the program restores it, which would silently turn a signal death into a
    // successful exit.  Compare the raw wait status of both.
    let long = vec![b'x'; 100_000];
    let statuses: Vec<_> = [c_binary(), rust_binary()]
        .iter()
        .map(|bin| {
            let mut child = Command::new(bin)
                .arg(os(&long))
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("spawn");
            // Drop the read end immediately so the child's write hits EPIPE.
            drop(child.stdout.take());
            let out = child.wait_with_output().expect("wait");
            exit_status(&out.status)
        })
        .collect();

    assert_eq!(
        statuses[0], statuses[1],
        "C and Rust disagree on the status after stdout is closed early: {statuses:?}"
    );
}

#[test]
fn unwritable_stdout_is_ignored_identically() {
    // /dev/full accepts opens and fails every write with ENOSPC.  The C code
    // never checks printf's return value, so it still exits 0; the Rust code
    // must do the same rather than reporting an I/O error.
    let full = Path::new("/dev/full");
    assert!(
        full.exists(),
        "/dev/full is required to exercise the write-failure path"
    );
    let statuses: Vec<_> = [c_binary(), rust_binary()]
        .iter()
        .map(|bin| {
            let sink = std::fs::OpenOptions::new()
                .write(true)
                .open(full)
                .expect("open /dev/full");
            let out = Command::new(bin)
                .args([os(b"hello"), os(b"1"), os(b"3")])
                .stdout(Stdio::from(sink))
                .stderr(Stdio::piped())
                .output()
                .expect("run");
            (exit_status(&out.status), out.stderr)
        })
        .collect();

    assert_eq!(
        statuses[0], statuses[1],
        "C and Rust disagree when stdout cannot be written"
    );
}

// ---------------------------------------------------------------------------
// Broad sweeps
// ---------------------------------------------------------------------------

/// Subjects and numeric spellings used by the sweep tests.
const SWEEP_SUBJECTS: &[&[u8]] = &[
    b"",
    b"a",
    b"ab",
    b"hello",
    b"hello world",
    b"0123456789",
    b"\xff\xfe\xfd",
    b"\xc3\xa9\xc3\xa9",
    b"a\tb\nc",
    b" ",
    b"-5",
];

const SWEEP_NUMBERS: &[&[u8]] = &[
    b"",
    b"0",
    b"1",
    b"2",
    b"5",
    b"9",
    b"10",
    b"11",
    b"-0",
    b"-1",
    b"-2",
    b"+0",
    b"+3",
    b" 4",
    b"\t4",
    b"\n4",
    b"\r4",
    b"\x0b4",
    b"\x0c4",
    b"4 ",
    b"abc",
    b"4abc",
    b"abc4",
    b"0x4",
    b"4.9",
    b".9",
    b"+",
    b"-",
    b"++1",
    b"--1",
    b"007",
    b"0000000000000000000005",
    b"2147483647",
    b"2147483648",
    b"-2147483648",
    b"-2147483649",
    b"4294967296",
    b"4294967297",
    b"4294967298",
    b"9223372036854775807",
    b"9223372036854775808",
    b"-9223372036854775808",
    b"-9223372036854775809",
    b"99999999999999999999999999",
    b"-99999999999999999999999999",
    b"18446744073709551616",
    b"\xa04",
    b"\xff4",
    b"1\xff",
    b"1e3",
    b"1,000",
];

#[test]
fn sweep_two_argument_forms() {
    for subject in SWEEP_SUBJECTS {
        for start in SWEEP_NUMBERS {
            assert_same(&[subject, start]);
        }
    }
}

#[test]
fn sweep_three_argument_forms() {
    // Full cross product over a smaller, representative numeric set.
    let nums: &[&[u8]] = &[
        b"",
        b"0",
        b"1",
        b"2",
        b"5",
        b"9",
        b"10",
        b"-1",
        b"abc",
        b"4abc",
        b" 3",
        b"0x3",
        b"+",
        b"2147483648",
        b"4294967296",
        b"4294967298",
        b"9223372036854775807",
        b"-9223372036854775808",
    ];
    for subject in [&b""[..], b"a", b"hello", b"0123456789", b"\xff\xfe\xfd"] {
        for start in nums {
            for stop in nums {
                assert_same(&[subject, start, stop]);
            }
        }
    }
}

#[test]
fn sweep_pseudo_random_inputs() {
    // Deterministic xorshift so failures reproduce exactly.
    fn next(state: &mut u64) -> u64 {
        *state ^= *state << 13;
        *state ^= *state >> 7;
        *state ^= *state << 17;
        *state
    }

    const FRAGMENTS: &[&[u8]] = &[
        b" ", b"\t", b"\n", b"+", b"-", b"0", b"1", b"5", b"9", b".", b"x",
        b"\xff", b"e", b",", b"00", b"21474836", b"48",
    ];

    fn build(state: &mut u64) -> Vec<u8> {
        let n = 1 + (next(state) % 5) as usize;
        let mut s = Vec::new();
        for _ in 0..n {
            s.extend_from_slice(FRAGMENTS[(next(state) % FRAGMENTS.len() as u64) as usize]);
        }
        s
    }

    let mut state: u64 = 0x2545_F491_4F6C_DD1D;

    for _ in 0..1500 {
        let subject =
            SWEEP_SUBJECTS[(next(&mut state) % SWEEP_SUBJECTS.len() as u64) as usize];

        match next(&mut state) % 4 {
            0 => assert_same(&[subject]),
            1 => {
                let a = build(&mut state);
                assert_same(&[subject, &a]);
            }
            _ => {
                let a = build(&mut state);
                let b = build(&mut state);
                assert_same(&[subject, &a, &b]);
            }
        }
    }
}
