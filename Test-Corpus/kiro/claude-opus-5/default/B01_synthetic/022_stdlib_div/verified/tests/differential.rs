//! Differential tests: run the original C program and the Rust translation as
//! subprocesses, feed both the same bytes on stdin, and require that stdout,
//! stderr and the exit status are identical.
//!
//! Nothing here links against the Rust source as a library; both programs are
//! driven exactly the way a shell drives them, because that is how the
//! translation is graded.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Locating / building the two executables
// ---------------------------------------------------------------------------

/// `translation/`
fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The workspace root that holds both `c_src/` and `translation/`.
fn project_root() -> PathBuf {
    manifest_dir()
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// The Rust executable under test, as built by cargo for this test run.
fn rust_binary() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// The C executable. Built on demand so `cargo test` works from a clean tree.
fn c_binary() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(build_c_binary).as_path()
}

fn build_c_binary() -> PathBuf {
    let c_src = project_root().join("c_src");
    assert!(
        c_src.join("src/main.c").is_file(),
        "cannot find c_src/src/main.c under {}",
        c_src.display()
    );

    let build_dir = c_src.join("build");
    let exe = build_dir.join("driver");
    if exe.is_file() {
        return exe;
    }

    std::fs::create_dir_all(&build_dir).expect("cannot create c_src/build");

    // Preferred path: the project's own build system.
    let configured = Command::new("cmake")
        .arg("..")
        .current_dir(&build_dir)
        .output();
    if let Ok(out) = configured {
        if out.status.success() {
            let built = Command::new("cmake")
                .args(["--build", "."])
                .current_dir(&build_dir)
                .output()
                .expect("failed to spawn cmake --build");
            if built.status.success() && exe.is_file() {
                return exe;
            }
            panic!(
                "cmake --build failed:\n{}\n{}",
                String::from_utf8_lossy(&built.stdout),
                String::from_utf8_lossy(&built.stderr)
            );
        }
    }

    // Fallback: compile directly with the system C compiler.
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
    let out = Command::new(&cc)
        .arg(c_src.join("src/main.c"))
        .arg("-o")
        .arg(&exe)
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn {cc}: {e}"));
    assert!(
        out.status.success() && exe.is_file(),
        "could not build the C program with {cc}:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    exe
}

// ---------------------------------------------------------------------------
// Running a program
// ---------------------------------------------------------------------------

/// The complete observable result of one run.
#[derive(PartialEq, Eq)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Ok(code)` for a normal exit, `Err(signal)` when killed by a signal.
    status: Result<i32, i32>,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "status={} stdout={:?} stderr={:?}",
            match self.status {
                Ok(c) => format!("exit {c}"),
                Err(s) => format!("signal {s}"),
            },
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr),
        )
    }
}

fn run(bin: &Path, args: &[&str], stdin_bytes: &[u8]) -> Outcome {
    let mut child = Command::new(bin)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    // Write on a helper thread: the child may die from SIGFPE before draining
    // stdin, which would otherwise deadlock or fail the write.
    let mut stdin = child.stdin.take().expect("piped stdin");
    let payload = stdin_bytes.to_vec();
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&payload);
        let _ = stdin.flush();
        drop(stdin);
    });

    let out = child.wait_with_output().expect("failed to wait for child");
    let _ = writer.join();

    let status = match out.status.code() {
        Some(code) => Ok(code),
        None => {
            #[cfg(unix)]
            {
                use std::os::unix::process::ExitStatusExt;
                Err(out.status.signal().expect("no exit code and no signal"))
            }
            #[cfg(not(unix))]
            {
                panic!("process ended with neither an exit code nor a signal");
            }
        }
    };

    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        status,
    }
}

/// Compare both programs on one input; returns a description on mismatch.
fn compare(label: &str, args: &[&str], input: &[u8]) -> Option<String> {
    let c = run(c_binary(), args, input);
    let r = run(rust_binary(), args, input);

    if c == r {
        return None;
    }

    let mut why = Vec::new();
    if c.stdout != r.stdout {
        why.push(format!(
            "stdout differs:\n      C: {:?}\n   Rust: {:?}",
            c.stdout, r.stdout
        ));
    }
    if c.stderr != r.stderr {
        why.push(format!(
            "stderr differs:\n      C: {:?}\n   Rust: {:?}",
            c.stderr, r.stderr
        ));
    }
    if c.status != r.status {
        why.push(format!(
            "exit status differs: C {:?} vs Rust {:?}",
            c.status, r.status
        ));
    }

    Some(format!(
        "case {label:?} (stdin = {:?}, args = {args:?})\n   {}",
        String::from_utf8_lossy(input),
        why.join("\n   ")
    ))
}

/// Run a batch of stdin-only cases and report every mismatch at once.
fn check_all(cases: &[(&str, &[u8])]) {
    let mut failures = Vec::new();
    for (label, input) in cases {
        if let Some(msg) = compare(label, &[], input) {
            failures.push(msg);
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} case(s) diverged:\n\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n\n")
    );
}

// ---------------------------------------------------------------------------
// Phase A — both programs exist and run
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_are_runnable() {
    let c = run(c_binary(), &[], b"6 3\n");
    let r = run(rust_binary(), &[], b"6 3\n");
    assert_eq!(c.status, Ok(0), "C program did not exit cleanly: {c:?}");
    assert_eq!(r.status, Ok(0), "Rust program did not exit cleanly: {r:?}");
    assert_eq!(c, r);
}

/// Pin the exact output format, including the single trailing newline, so a
/// spacing or precision regression cannot hide behind a lenient comparison.
#[test]
fn output_format_is_byte_exact() {
    let c = run(c_binary(), &[], b"7 2\n");
    assert_eq!(c.stdout, b"quotient: 3, remainder: 1\n".to_vec());
    assert!(c.stderr.is_empty());
    let r = run(rust_binary(), &[], b"7 2\n");
    assert_eq!(r.stdout, b"quotient: 3, remainder: 1\n".to_vec());
    assert!(r.stderr.is_empty());
}

// ---------------------------------------------------------------------------
// Phase B — the input classes the C code branches on
// ---------------------------------------------------------------------------

/// No input at all: `scanf` suffers an input failure for both conversions, so
/// `x` and `y` keep their initializers of 1.
#[test]
fn empty_and_whitespace_only_input() {
    check_all(&[
        ("empty", b""),
        ("single newline", b"\n"),
        ("spaces only", b"     "),
        ("mixed whitespace only", b" \t\n\r\x0b\x0c "),
        ("many newlines", b"\n\n\n\n\n"),
    ]);
}

/// Exactly one item read: the second conversion never happens and `y` stays 1.
#[test]
fn single_item_input() {
    check_all(&[
        ("one int, no newline", b"5"),
        ("one int, newline", b"5\n"),
        ("one int, trailing space", b"5 "),
        ("one int, trailing junk", b"5abc"),
        ("one negative int", b"-5"),
        ("one int then sign only", b"5 -"),
        ("one int then sign+space", b"5 - 3"),
        ("one int then plus only", b"5 +"),
    ]);
}

/// Ordinary two-item input across every sign combination, exercising C's
/// truncating division and the sign of the remainder.
#[test]
fn two_item_happy_paths() {
    check_all(&[
        ("exact division", b"6 3\n"),
        ("with remainder", b"7 2\n"),
        ("negative numerator", b"-7 2\n"),
        ("negative denominator", b"7 -2\n"),
        ("both negative", b"-7 -2\n"),
        ("zero numerator", b"0 5\n"),
        ("numerator smaller", b"3 7\n"),
        ("equal operands", b"42 42\n"),
        ("explicit plus signs", b"+5 +3\n"),
        ("leading zeros", b"007 002\n"),
        ("many leading zeros", b"000000000000000000000005 3\n"),
        ("no space, negative second", b"5-3"),
        ("extra trailing items ignored", b"9 4 7 11\n"),
    ]);
}

/// `scanf` skips whitespace, including newlines, before each `%d`; unlike
/// `fgets` it does not stop at a line boundary.
#[test]
fn scanf_reads_across_newlines() {
    check_all(&[
        ("newline between", b"9\n4"),
        ("blank lines between", b"9\n\n\n4\n"),
        ("leading newlines", b"\n\n9 4"),
        ("tabs and newlines", b"  \t\n 8\t\n 3"),
        ("vertical tab and form feed", b"\x0b\x0c9\x0b\x0c4"),
        ("carriage returns", b"\r\n9\r\n4\r\n"),
        ("indented across lines", b"   \n   9\n   4  "),
    ]);
}

/// Matching failure on the first or second conversion leaves that variable at
/// its initializer of 1 — the `scanf` return value is never inspected.
#[test]
fn matching_failure_leaves_initializers() {
    check_all(&[
        ("letters only", b"abc"),
        ("letters then ints", b"abc 9 4"),
        ("hex-looking literal", b"0x10"),
        ("float-looking literal", b"1e3 2"),
        ("leading dot", b".5 2"),
        ("sign only", b"-"),
        ("plus only", b"+"),
        ("sign then space", b"- 5"),
        ("sign then sign", b"+-5 2"),
        ("first ok, second letters", b"9 abc"),
        ("first ok, second dot", b"9 .5"),
        ("punctuation", b",;:"),
        ("underscore", b"_9 4"),
    ]);
}

/// The two undefined-behavior divisions in the C program: x86 `idiv` traps and
/// the process dies from SIGFPE with nothing on stdout.
#[test]
fn division_faults_match() {
    check_all(&[
        ("zero over zero", b"0 0"),
        ("nonzero over zero", b"5 0"),
        ("negative over zero", b"-5 0"),
        ("zero denominator via newline", b"\n5\n0\n"),
        ("zero denominator with sign", b"5 -0"),
        ("zero denominator, leading zeros", b"5 0000"),
        ("INT_MIN over -1", b"-2147483648 -1"),
    ]);
}

/// A default `y` of 1 means a bare number never faults, while a failed *first*
/// conversion cannot leave a zero denominator behind.
#[test]
fn default_denominator_is_one() {
    check_all(&[
        ("first fails, default 1/1", b"zzz"),
        ("first fails then zero", b"zzz 0"),
        ("only numerator", b"123456"),
    ]);
}

// ---------------------------------------------------------------------------
// Phase C — boundaries, overflow, truncation, signedness
// ---------------------------------------------------------------------------

/// The extremes of `int`, which is the widest value the code handles.
#[test]
fn int_boundaries() {
    check_all(&[
        ("INT_MAX over 1", b"2147483647 1"),
        ("INT_MAX over INT_MAX", b"2147483647 2147483647"),
        ("INT_MAX over -1", b"2147483647 -1"),
        ("INT_MIN over 1", b"-2147483648 1"),
        ("INT_MIN over 2", b"-2147483648 2"),
        ("INT_MIN over INT_MIN", b"-2147483648 -2147483648"),
        ("1 over INT_MIN", b"1 -2147483648"),
        ("-1 over INT_MIN", b"-1 -2147483648"),
        ("INT_MAX and INT_MIN", b"2147483647 -2147483648"),
    ]);
}

/// glibc's `%d` converts through `strtol`, which saturates at LONG_MAX /
/// LONG_MIN, and the saturated value is then truncated to `int`.
#[test]
fn overflow_saturates_then_truncates() {
    check_all(&[
        ("just past INT_MAX", b"2147483648 1"),
        ("just past INT_MIN", b"-2147483649 1"),
        ("UINT_MAX", b"4294967295 1"),
        ("2^32", b"4294967296 1"),
        ("2^32 + 1", b"4294967297 1"),
        ("3 * 2^31 - 3", b"6442450941 2"),
        ("eleven nines", b"99999999999 3"),
        ("LONG_MAX", b"9223372036854775807 1"),
        ("LONG_MAX + 1", b"9223372036854775808 1"),
        ("LONG_MIN", b"-9223372036854775808 1"),
        ("LONG_MIN - 1", b"-9223372036854775809 1"),
        ("2^64 + 1", b"18446744073709551617 1"),
        ("twenty-three nines", b"99999999999999999999999 1"),
        ("negative twenty-three nines", b"-99999999999999999999999 1"),
        ("overflow in denominator", b"7 4294967296"),
        ("overflow both", b"4294967297 4294967298"),
        (
            "leading zeros then LONG_MAX",
            b"0000000000009223372036854775807 1",
        ),
    ]);
}

/// A digit run long enough to exercise the conversion's internal buffering.
#[test]
fn very_long_digit_runs() {
    let mut nines = vec![b'9'; 4096];
    nines.extend_from_slice(b" 1");
    let mut zeros = vec![b'0'; 4096];
    zeros.extend_from_slice(b"7 2");
    let mut neg = vec![b'-'];
    neg.extend(std::iter::repeat(b'8').take(1000));
    neg.extend_from_slice(b" 1");

    check_all(&[
        ("4096 nines", &nines),
        ("4096 leading zeros", &zeros),
        ("1000 eights, negative", &neg),
    ]);
}

/// Bytes that are neither digits nor whitespace, including NUL and values
/// outside ASCII, must not be treated specially.
#[test]
fn non_ascii_and_nul_bytes() {
    check_all(&[
        ("NUL first", b"\x009 4"),
        ("NUL between", b"9\x004"),
        ("high bytes first", b"\xff\xfe 9 4"),
        ("high byte after", b"9 4\xff"),
        ("utf8 first", "é 9 4".as_bytes()),
        ("utf8 between", "9 é 4".as_bytes()),
        ("lone continuation byte", b"\x80\x81"),
    ]);
}

/// A large amount of leading whitespace still resolves to the same two ints.
#[test]
fn large_input() {
    let mut spaces = vec![b' '; 1 << 16];
    spaces.extend_from_slice(b"9 4\n");
    let mut trailing = b"9 4\n".to_vec();
    trailing.extend(std::iter::repeat(b'x').take(1 << 16));

    check_all(&[
        ("64 KiB of leading spaces", &spaces),
        ("64 KiB of trailing junk", &trailing),
    ]);
}

/// `main` takes no parameters, so command-line arguments are ignored by both.
#[test]
fn command_line_arguments_are_ignored() {
    let mut failures = Vec::new();
    for (label, args) in [
        ("one arg", &["9"][..]),
        ("two args", &["9", "4"][..]),
        ("flag-looking arg", &["--help"][..]),
    ] {
        if let Some(msg) = compare(label, args, b"8 3\n") {
            failures.push(msg);
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

/// Systematic sweep over small operand pairs, skipping only the two inputs
/// whose C behavior is a fatal signal (covered separately above) so this test
/// stays fast and focused on arithmetic agreement.
#[test]
fn small_operand_sweep() {
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    for x in -6i32..=6 {
        for y in -6i32..=6 {
            if y == 0 {
                continue;
            }
            cases.push((format!("{x} / {y}"), format!("{x} {y}\n").into_bytes()));
        }
    }

    let mut failures = Vec::new();
    for (label, input) in &cases {
        if let Some(msg) = compare(label, &[], input) {
            failures.push(msg);
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} pair(s) diverged:\n\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n\n")
    );
}
