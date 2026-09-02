//! Differential tests: run the C binary and the Rust binary as subprocesses on
//! the same stdin and require byte-identical stdout, byte-identical stderr and
//! an identical exit status.
//!
//! The Rust code is never linked as a library here; only the built executable is
//! driven, because that is how the two programs are compared.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Locating and building the two executables
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    // .../translation -> ...
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the Rust executable under test, provided by Cargo.
fn rust_bin() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// Path to the C executable, building it with CMake on first use.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = repo_root().join("c_src");
        let build = c_src.join("build");
        let bin = build.join("driver");
        if bin.exists() {
            return bin;
        }

        std::fs::create_dir_all(&build).expect("create c_src/build");
        let configure = Command::new("cmake")
            .arg("..")
            .current_dir(&build)
            .output()
            .expect("run `cmake ..` (is cmake installed?)");
        assert!(
            configure.status.success(),
            "cmake configure failed:\n{}\n{}",
            String::from_utf8_lossy(&configure.stdout),
            String::from_utf8_lossy(&configure.stderr),
        );
        let compile = Command::new("cmake")
            .args(["--build", "."])
            .current_dir(&build)
            .output()
            .expect("run `cmake --build .`");
        assert!(
            compile.status.success(),
            "cmake build failed:\n{}\n{}",
            String::from_utf8_lossy(&compile.stdout),
            String::from_utf8_lossy(&compile.stderr),
        );
        assert!(bin.exists(), "C binary missing after build: {}", bin.display());
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
    /// Normal exit code, if the process was not killed by a signal.
    code: Option<i32>,
    /// Terminating signal, if any.
    signal: Option<i32>,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Outcome")
            .field("stdout", &String::from_utf8_lossy(&self.stdout))
            .field("stdout_hex", &hex(&self.stdout))
            .field("stderr", &String::from_utf8_lossy(&self.stderr))
            .field("code", &self.code)
            .field("signal", &self.signal)
            .finish()
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn outcome_of(status: std::process::ExitStatus, stdout: Vec<u8>, stderr: Vec<u8>) -> Outcome {
    #[cfg(unix)]
    let signal = {
        use std::os::unix::process::ExitStatusExt;
        status.signal()
    };
    #[cfg(not(unix))]
    let signal: Option<i32> = None;

    Outcome { stdout, stderr, code: status.code(), signal }
}

/// Runs `bin` with `input` on stdin and `args` as arguments.
fn run_with_args(bin: &Path, input: &[u8], args: &[&str]) -> Outcome {
    let mut child = Command::new(bin)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", bin.display()));

    // Write stdin from a thread so a program that never drains it cannot
    // deadlock against a full pipe buffer.
    let mut stdin = child.stdin.take().expect("piped stdin");
    let payload = input.to_vec();
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&payload);
        let _ = stdin.flush();
        drop(stdin);
    });

    let out = child.wait_with_output().expect("wait_with_output");
    writer.join().expect("stdin writer thread");
    outcome_of(out.status, out.stdout, out.stderr)
}

fn run(bin: &Path, input: &[u8]) -> Outcome {
    run_with_args(bin, input, &[])
}

/// Asserts the C and Rust programs agree on stdout, stderr and exit status.
fn assert_same(label: &str, input: &[u8]) -> Outcome {
    let c = run(c_bin(), input);
    let r = run(rust_bin(), input);
    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout differs for {label} (input {:?} / hex {})\n  C: {:?}\n  R: {:?}",
        String::from_utf8_lossy(input),
        hex(input),
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout),
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr differs for {label} (input {:?})\n  C: {:?}\n  R: {:?}",
        String::from_utf8_lossy(input),
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr),
    );
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "exit status differs for {label} (input {:?}): C {:?} vs Rust {:?}",
        String::from_utf8_lossy(input),
        (c.code, c.signal),
        (r.code, r.signal),
    );
    c
}

fn assert_same_cases(cases: &[(&str, &[u8])]) {
    for (label, input) in cases {
        assert_same(label, input);
    }
}

// ---------------------------------------------------------------------------
// The two reachable outputs, pinned to exact bytes
// ---------------------------------------------------------------------------

/// `bad()`: `data = CHAR_MAX` (127), `data * 2` computed as `int` (254) and
/// truncated to `char` (-2); `printf("%02x")` promotes -2 to `int` and prints
/// its unsigned bit pattern, so eight digits, not two.
const BAD_OUT: &[u8] = b"fffffffe\n";

/// `good()`: `goodG2B` prints 4 as `04`, then `goodB2G` takes the
/// `data < CHAR_MAX/2` false branch (127 < 63 is false) and prints the message.
const GOOD_OUT: &[u8] = b"04\ndata value is too large to perform arithmetic safely.\n";

#[test]
fn golden_output_bytes_are_what_the_c_produces() {
    // Guards against both programs drifting together: these are the literal
    // bytes the C source is able to emit.
    let zero = run(c_bin(), b"0\n");
    assert_eq!(zero.stdout, BAD_OUT, "C bad() output changed");
    assert_eq!(zero.stderr, b"");
    assert_eq!(zero.code, Some(0));

    let one = run(c_bin(), b"1\n");
    assert_eq!(one.stdout, GOOD_OUT, "C good() output changed");
    assert_eq!(one.stderr, b"");
    assert_eq!(one.code, Some(0));

    // And the Rust binary reproduces them.
    assert_eq!(run(rust_bin(), b"0\n").stdout, BAD_OUT);
    assert_eq!(run(rust_bin(), b"1\n").stdout, GOOD_OUT);
}

// ---------------------------------------------------------------------------
// Phase B: the branches the C actually has
// ---------------------------------------------------------------------------

/// `main`: no input at all, so `scanf` returns EOF and leaves `x == 0`, taking
/// the `else` branch into `bad()`.
#[test]
fn empty_input_reaches_bad() {
    let out = assert_same("empty input", b"");
    assert_eq!(out.stdout, BAD_OUT);
    assert_eq!(out.code, Some(0));
}

/// A single item on stdin: the smallest well-formed input for each branch.
#[test]
fn single_value_selects_the_branch() {
    let zero = assert_same("single 0", b"0");
    assert_eq!(zero.stdout, BAD_OUT);
    let one = assert_same("single 1", b"1");
    assert_eq!(one.stdout, GOOD_OUT);
}

#[test]
fn zero_variants_all_reach_bad() {
    assert_same_cases(&[
        ("0", b"0"),
        ("0 with newline", b"0\n"),
        ("-0", b"-0"),
        ("+0", b"+0"),
        ("many zeros", b"00000"),
        ("0 then junk", b"0abc"),
        // `%d` stops at 'x'; it does not accept a hex prefix.
        ("0x10", b"0x10"),
        ("0 then more numbers", b"0 1 2"),
    ]);
}

#[test]
fn nonzero_variants_all_reach_good() {
    assert_same_cases(&[
        ("1", b"1"),
        ("-1", b"-1"),
        ("+5", b"+5"),
        ("-5", b"-5"),
        ("42 with newline", b"42\n"),
        ("leading zeros then 1", b"0001"),
        // `%d` stops at '.', converting just the 3.
        ("3.9", b"3.9"),
        ("1 then letters", b"1foo"),
        ("two numbers, first wins", b"  12  34"),
    ]);
}

/// `scanf` skips leading whitespace and therefore reads across newlines --
/// unlike `fgets`, which would stop at the first one.
#[test]
fn scanf_skips_whitespace_across_lines() {
    assert_same_cases(&[
        ("blank lines then value", b"\n\n\n  7\n"),
        ("leading spaces", b"      7"),
        ("tab then value", b"\t7"),
        ("crlf then value", b"\r\n7"),
        ("all C whitespace then value", b" \t\n\x0b\x0c\r 42"),
        ("value on the third line", b"\n\n5"),
    ]);
}

/// Inputs where no digit is converted: `scanf` reports a matching failure and
/// leaves `x` at its initialiser 0, so `bad()` runs.
#[test]
fn matching_failure_leaves_x_zero() {
    let cases: &[(&str, &[u8])] = &[
        ("letters", b"abc"),
        ("lone minus", b"-"),
        ("lone plus", b"+"),
        ("double minus", b"--1"),
        ("sign then space", b"- 1"),
        ("sign then letter", b"-a"),
        ("dot first", b".5"),
        ("only whitespace", b"   \n\t\n"),
        ("lone newline", b"\n"),
        ("punctuation", b"!!!"),
        ("NUL byte first", b"\0 5"),
        ("invalid utf-8", b"\xff\xfe"),
        ("utf-8 multibyte first", b"\xc3\xa9 7"),
    ];
    for (label, input) in cases {
        let out = assert_same(label, input);
        assert_eq!(out.stdout, BAD_OUT, "{label} should reach bad()");
        assert_eq!(out.code, Some(0));
    }
}

/// glibc converts `%d` with `strtol` (saturating at `long`) and then stores the
/// result into an `int`, keeping the low 32 bits. Values whose low 32 bits are
/// zero therefore reach `bad()` even though the number written was non-zero.
#[test]
fn int_truncation_of_the_converted_value() {
    // Low 32 bits are zero -> x == 0 -> bad().
    for (label, input) in [
        ("2^32", &b"4294967296"[..]),
        ("2^36", &b"68719476736"[..]),
        ("2^40", &b"1099511627776"[..]),
        ("-2^32", &b"-4294967296"[..]),
        ("LONG_MIN", &b"-9223372036854775808"[..]),
        ("below LONG_MIN saturates to LONG_MIN", &b"-9223372036854775809"[..]),
        ("very negative saturates", &b"-99999999999999999999"[..]),
        ("zeros then 2^32", &b"0000000000000000000000004294967296"[..]),
    ] {
        let out = assert_same(label, input);
        assert_eq!(out.stdout, BAD_OUT, "{label} truncates to 0");
    }

    // Low 32 bits non-zero -> good().
    for (label, input) in [
        ("INT_MAX", &b"2147483647"[..]),
        ("INT_MAX+1", &b"2147483648"[..]),
        ("INT_MIN", &b"-2147483648"[..]),
        ("INT_MIN-1", &b"-2147483649"[..]),
        ("LONG_MAX", &b"9223372036854775807"[..]),
        ("above LONG_MAX saturates to LONG_MAX", &b"9223372036854775808"[..]),
        ("2^64", &b"18446744073709551616"[..]),
        ("2^64+1", &b"18446744073709551617"[..]),
        ("huge positive saturates", &b"99999999999999999999"[..]),
    ] {
        let out = assert_same(label, input);
        assert_eq!(out.stdout, GOOD_OUT, "{label} truncates to non-zero");
    }
}

/// The largest inputs the conversion has to chew through: far more digits and
/// far more leading whitespace than any buffer in the program.
#[test]
fn maximum_sized_inputs() {
    let mut nines = vec![b'9'; 100_000];
    assert_same("100k digits", &nines);

    nines.insert(0, b'-');
    assert_same("100k digits, negative", &nines);

    let mut spaces = vec![b' '; 100_000];
    spaces.push(b'3');
    assert_same("100k spaces then a digit", &spaces);

    let mut newlines = vec![b'\n'; 100_000];
    newlines.push(b'0');
    assert_same("100k newlines then a digit", &newlines);

    let mut zeros = vec![b'0'; 100_000];
    zeros.push(b'1');
    assert_same("100k leading zeros then 1", &zeros);

    // No digit anywhere in a large input: a matching failure after a long scan.
    assert_same("100k letters", &vec![b'x'; 100_000]);

    // A large input with no whitespace or digits at all.
    assert_same("100k NUL bytes", &vec![0u8; 100_000]);
}

/// Every byte value as the first character of input, which covers each
/// whitespace / sign / digit / other classification in the conversion.
#[test]
fn every_leading_byte_agrees() {
    for b in 0u8..=255 {
        let input = [b, b'7'];
        assert_same(&format!("leading byte {:#04x}", b), &input);
    }
}

/// `main` takes no arguments, so argv must not change anything.
#[test]
fn arguments_are_ignored_identically() {
    for args in [vec!["1"], vec!["0"], vec!["--help"], vec!["a", "b", "c"]] {
        let c = run_with_args(c_bin(), b"0\n", &args);
        let r = run_with_args(rust_bin(), b"0\n", &args);
        assert_eq!(c.stdout, r.stdout, "stdout differs with args {args:?}");
        assert_eq!(c.stderr, r.stderr, "stderr differs with args {args:?}");
        assert_eq!(
            (c.code, c.signal),
            (r.code, r.signal),
            "status differs with args {args:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Phase C: paths reached through the environment rather than through stdin
// ---------------------------------------------------------------------------

/// `printf` reports a write failure only through its return value, which the C
/// ignores, and a failed flush in `exit` does not change the status `main`
/// returned: the C exits 0. Writing to `/dev/full` always fails with ENOSPC.
#[cfg(unix)]
#[test]
fn stdout_write_error_is_ignored() {
    let dev_full = Path::new("/dev/full");
    assert!(dev_full.exists(), "/dev/full is required for this test");

    let run_to_dev_full = |bin: &Path| -> Outcome {
        let sink = std::fs::OpenOptions::new()
            .write(true)
            .open(dev_full)
            .expect("open /dev/full");
        let mut child = Command::new(bin)
            .stdin(Stdio::piped())
            .stdout(Stdio::from(sink))
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn");
        drop(child.stdin.take());
        let out = child.wait_with_output().expect("wait");
        outcome_of(out.status, Vec::new(), out.stderr)
    };

    let c = run_to_dev_full(c_bin());
    let r = run_to_dev_full(rust_bin());
    assert_eq!(c.stderr, r.stderr, "stderr differs when stdout is unwritable");
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "status differs when stdout is unwritable: C {:?} vs Rust {:?}",
        (c.code, c.signal),
        (r.code, r.signal)
    );
    assert_eq!(c.code, Some(0), "the C ignores the write failure");
}

/// A C program runs with `SIGPIPE` at its default disposition, so a write into
/// a pipe with no reader kills it with signal 13.
///
/// The reader is closed while the child is still blocked in `scanf` waiting for
/// stdin, and only then is stdin closed, so the child cannot write before the
/// pipe is broken -- no race.
#[cfg(unix)]
#[test]
fn broken_stdout_pipe_matches() {
    let run_into_broken_pipe = |bin: &Path| -> Outcome {
        let mut child: Child = Command::new(bin)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn");

        // Close the read end of stdout while the child is still waiting on stdin.
        drop(child.stdout.take().expect("piped stdout"));

        let mut stderr = child.stderr.take().expect("piped stderr");
        // Now let the child proceed: EOF on stdin -> it prints -> EPIPE.
        drop(child.stdin.take());

        let mut err = Vec::new();
        stderr.read_to_end(&mut err).expect("read stderr");
        let status = child.wait().expect("wait");
        outcome_of(status, Vec::new(), err)
    };

    let c = run_into_broken_pipe(c_bin());
    let r = run_into_broken_pipe(rust_bin());
    assert_eq!(c.stderr, r.stderr, "stderr differs on a broken stdout pipe");
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "status differs on a broken stdout pipe: C {:?} vs Rust {:?}",
        (c.code, c.signal),
        (r.code, r.signal)
    );
    assert_eq!(c.signal, Some(13), "the C dies from SIGPIPE");
}

/// stdin already at EOF because it is `/dev/null`, rather than an empty pipe.
#[cfg(unix)]
#[test]
fn stdin_from_dev_null_matches() {
    let run_from_dev_null = |bin: &Path| -> Outcome {
        let null = std::fs::File::open("/dev/null").expect("open /dev/null");
        let out = Command::new(bin)
            .stdin(Stdio::from(null))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("output");
        outcome_of(out.status, out.stdout, out.stderr)
    };

    let c = run_from_dev_null(c_bin());
    let r = run_from_dev_null(rust_bin());
    assert_eq!(c.stdout, r.stdout);
    assert_eq!(c.stderr, r.stderr);
    assert_eq!((c.code, c.signal), (r.code, r.signal));
    assert_eq!(c.stdout, BAD_OUT);
}

/// stdin redirected from a regular file, which is seekable and read in blocks --
/// a different stdio path from a pipe.
#[cfg(unix)]
#[test]
fn stdin_from_regular_file_matches() {
    let dir = std::env::temp_dir().join(format!("driver-difftest-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");

    let run_from_file = |bin: &Path, path: &Path| -> Outcome {
        let f = std::fs::File::open(path).expect("open input file");
        let out = Command::new(bin)
            .stdin(Stdio::from(f))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("output");
        outcome_of(out.status, out.stdout, out.stderr)
    };

    for (name, contents) in [
        ("empty", &b""[..]),
        ("zero", &b"0\n"[..]),
        ("one", &b"1\n"[..]),
        ("trailing junk", &b"7 rest of the line\nsecond line\n"[..]),
    ] {
        let path = dir.join(name);
        std::fs::write(&path, contents).expect("write input file");
        let c = run_from_file(c_bin(), &path);
        let r = run_from_file(rust_bin(), &path);
        assert_eq!(c.stdout, r.stdout, "stdout differs for file input {name}");
        assert_eq!(c.stderr, r.stderr, "stderr differs for file input {name}");
        assert_eq!(
            (c.code, c.signal),
            (r.code, r.signal),
            "status differs for file input {name}"
        );
    }

    let _ = std::fs::remove_dir_all(&dir);
}

/// Neither program writes to stderr on any input, and both always exit 0 when
/// stdout is writable.
#[test]
fn stderr_is_always_empty_and_status_always_zero() {
    for input in [&b""[..], b"0", b"1", b"abc", b"-9223372036854775809"] {
        let out = assert_same("stderr/status invariant", input);
        assert_eq!(out.stderr, b"", "unexpected stderr for {:?}", input);
        assert_eq!(out.code, Some(0));
        assert_eq!(out.signal, None);
    }
}
