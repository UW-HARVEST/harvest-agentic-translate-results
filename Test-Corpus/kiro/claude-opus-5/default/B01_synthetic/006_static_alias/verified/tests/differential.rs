//! Differential tests: run the C `driver` and the Rust `driver` as subprocesses
//! with identical `argv` and require byte-identical stdout, byte-identical
//! stderr and an identical exit status.
//!
//! The Rust code is never called as a library — both programs are driven
//! exactly the way a shell would drive them.

use std::ffi::{OsStr, OsString};
use std::io::Read;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Once;

/// Path to the Rust binary under test, provided by cargo.
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Repository root (the directory holding `c_src/` and `translation/`).
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the C binary, building it with cmake the first time if needed.
fn c_bin() -> PathBuf {
    static BUILD: Once = Once::new();
    let c_src = repo_root().join("c_src");
    let exe = c_src.join("build").join("driver");

    BUILD.call_once(|| {
        if exe.is_file() {
            return;
        }
        let build_dir = c_src.join("build");
        std::fs::create_dir_all(&build_dir).expect("cannot create c_src/build");
        let configure = Command::new("cmake")
            .arg("..")
            .current_dir(&build_dir)
            .output()
            .expect("failed to run `cmake ..` (is cmake installed?)");
        assert!(
            configure.status.success(),
            "cmake configure failed:\n{}",
            String::from_utf8_lossy(&configure.stderr)
        );
        let build = Command::new("cmake")
            .args(["--build", "."])
            .current_dir(&build_dir)
            .output()
            .expect("failed to run `cmake --build .`");
        assert!(
            build.status.success(),
            "cmake build failed:\n{}",
            String::from_utf8_lossy(&build.stderr)
        );
    });

    assert!(
        exe.is_file(),
        "the C reference binary is missing at {}",
        exe.display()
    );
    exe
}

/// What we compare: the full observable result of one run.
#[derive(PartialEq, Eq)]
struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Ok(code)` for a normal exit, `Err(signal)` when killed by a signal.
    status: Result<i32, i32>,
}

impl std::fmt::Debug for Run {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Run")
            .field("status", &self.status)
            .field("stdout", &Abbrev(&self.stdout))
            .field("stderr", &Abbrev(&self.stderr))
            .finish()
    }
}

/// Renders byte output readably, truncating very long streams.
struct Abbrev<'a>(&'a [u8]);
impl std::fmt::Debug for Abbrev<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let shown = &self.0[..self.0.len().min(400)];
        write!(f, "{:?}", String::from_utf8_lossy(shown))?;
        if self.0.len() > shown.len() {
            write!(f, " (+{} more bytes)", self.0.len() - shown.len())?;
        }
        Ok(())
    }
}

fn exec(exe: &Path, args: &[OsString]) -> Run {
    let out = Command::new(exe)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        status: match out.status.code() {
            Some(code) => Ok(code),
            None => Err(out.status.signal().unwrap_or(-1)),
        },
    }
}

/// Byte-string argument (allows argv that is not valid UTF-8).
fn arg(bytes: &[u8]) -> OsString {
    OsStr::from_bytes(bytes).to_os_string()
}

/// Core assertion: identical stdout, stderr and exit status.
fn assert_same(args: &[OsString]) {
    let c = exec(&c_bin(), args);
    let r = exec(&rust_bin(), args);

    let pretty: Vec<String> = args
        .iter()
        .map(|a| format!("{:?}", String::from_utf8_lossy(a.as_bytes())))
        .collect();
    let label = format!("argv = [{}]", pretty.join(", "));

    assert_eq!(
        c.status, r.status,
        "exit status differs for {label}\n  C: {:?}\n  R: {:?}",
        c, r
    );
    assert_eq!(
        c.stdout, r.stdout,
        "stdout differs for {label}\n  C: {:?}\n  R: {:?}",
        c, r
    );
    assert_eq!(
        c.stderr, r.stderr,
        "stderr differs for {label}\n  C: {:?}\n  R: {:?}",
        c, r
    );
}

/// Convenience for the common `<initial_value> <iterations>` shape.
fn same2(a: &str, n: &str) {
    assert_same(&[arg(a.as_bytes()), arg(n.as_bytes())]);
}

fn same_bytes(a: &[u8], b: &[u8]) {
    assert_same(&[arg(a), arg(b)]);
}

// ---------------------------------------------------------------------------
// argc validation: `if (argc != 3)`
// ---------------------------------------------------------------------------

#[test]
fn wrong_argument_count_is_rejected() {
    // Every arity except exactly two operands hits the first error path.
    assert_same(&[]);
    assert_same(&[arg(b"1")]);
    assert_same(&[arg(b"1"), arg(b"2"), arg(b"3")]);
    assert_same(&[arg(b"1"), arg(b"2"), arg(b"3"), arg(b"4")]);
    assert_same(&[arg(b""), arg(b""), arg(b"")]);
    // Even otherwise-invalid arguments never reach strtol when argc is wrong,
    // so the message must be the argc one, not the parse one.
    assert_same(&[arg(b"abc")]);
    assert_same(&[arg(b"abc"), arg(b"def"), arg(b"ghi")]);
}

// ---------------------------------------------------------------------------
// strtol validation: `if (end == argv[1])` / `if (end == argv[2])`
// ---------------------------------------------------------------------------

#[test]
fn first_argument_must_parse() {
    // Nothing here contains a digit that strtol could consume, so `end` stays
    // at the start of the string and the first error path is taken.
    for bad in [
        "", " ", "   ", "\t", "\n", "\u{b}", "\u{c}", "\r", " \t\n\u{b}\u{c}\r",
        "abc", "x", "X", "+", "-", "++", "--", "+-", "-+", "+ 1", "- 1",
        ".", ".5", ",", "_", "'", "\"", "/", ":", "e5", "E5", "x10", "X10",
        "  abc", "\tzz", "nan", "inf", "--5", "+-5", " + 5",
    ] {
        same2(bad, "3");
    }
}

#[test]
fn second_argument_must_parse() {
    for bad in [
        "", " ", "\t", "\n", "\r", "\u{b}", "\u{c}", "abc", "+", "-", "-+", "++",
        ".", ".5", ",", "_", "x", "e", "/", ":", "  ", " \t ",
    ] {
        same2("5", bad);
    }
}

#[test]
fn first_argument_checked_before_second() {
    // Both arguments are invalid: only the first message may be printed.
    same2("abc", "def");
    same2("", "");
    same2("+", "-");
    same2(" ", "\t");
}

#[test]
fn partial_parses_are_accepted() {
    // strtol succeeds as long as at least one digit was consumed; trailing
    // garbage is silently ignored because the C code only checks `end == arg`.
    for a in [
        "0x", "0x10", "0b101", "12abc", "3.9", "3,9", "7e2", "5 5", "5\t5", "  12",
        "\t\n\u{b}\u{c}\r 9", "+7", "-0", "  -3  ", "0000000000000000000000005",
        "-0000000000000000000000005", "1_000", "9/9", "4:4",
    ] {
        same2(a, "4");
    }
    for n in ["0x10", "3abc", "2.9", "  3", "+3", "-0", "0000000003", "5x"] {
        same2("6", n);
    }
}

#[test]
fn non_utf8_arguments() {
    same_bytes(b"\xff\xfe", b"3");
    same_bytes(b"3", b"\xff");
    same_bytes(b"5\xff", b"3");
    same_bytes(b"\xff5", b"3");
    same_bytes(b"\x80\x80", b"\x80");
    same_bytes(b"\xc3", b"4");
}

// ---------------------------------------------------------------------------
// Loop control: `for (int i = 0; i < iterations; i++)`
// ---------------------------------------------------------------------------

#[test]
fn zero_iterations_produces_no_output() {
    same2("0", "0");
    same2("5", "0");
    same2("-5", "0");
    same2("2147483647", "0");
    same2("-2147483648", "0");
    same2("0", "-0");
}

#[test]
fn negative_iterations_produce_no_output() {
    for n in ["-1", "-2", "-7", "-100", "-2147483648", "-9223372036854775808"] {
        same2("5", n);
        same2("-5", n);
    }
}

#[test]
fn single_iteration() {
    // Exercises both branches of static_alias exactly once.
    same2("1", "1"); // *outer >= inner  -> returns &inner
    same2("0", "1"); // *outer <  inner  -> returns outer
    same2("-1", "1");
    same2("2147483647", "1");
    same2("-2147483648", "1");
}

// ---------------------------------------------------------------------------
// static_alias: the taken branch, the untaken branch, and the aliased state
// ---------------------------------------------------------------------------

#[test]
fn then_branch_then_self_aliasing() {
    // initial_value >= 1 immediately, so the returned pointer aliases `inner`
    // from the first call on and the sum doubles forever.
    for a in ["1", "2", "3", "5", "7", "32", "1000"] {
        for n in ["1", "2", "3", "5", "10", "20", "31", "32", "33", "40"] {
            same2(a, n);
        }
    }
}

#[test]
fn else_branch_walks_up_to_zero_then_aliases() {
    // initial_value < 1: `*outer += inner` walks the automatic variable up by
    // one per iteration until it reaches 1, then control flips to `&inner`.
    for a in ["0", "-1", "-2", "-3", "-5", "-10", "-40"] {
        for n in ["1", "2", "3", "4", "5", "6", "10", "45", "60", "80"] {
            same2(a, n);
        }
    }
}

#[test]
fn boundary_initial_values_scan() {
    // A window across the sign change covers every walk length through the
    // else branch before the aliasing state is entered.
    for a in -40..=40 {
        same2(&a.to_string(), "50");
    }
}

// ---------------------------------------------------------------------------
// Overflow, truncation and signedness, exactly as the C performs them
// ---------------------------------------------------------------------------

#[test]
fn int_boundary_initial_values() {
    for a in [
        "2147483647",  // INT_MAX: inner += INT_MAX overflows on the first call
        "2147483646",
        "-2147483648", // INT_MIN
        "-2147483647",
        "1073741823",
        "1073741824",
        "1073741825",
        "65536",
        "-65536",
    ] {
        for n in ["1", "2", "3", "4", "8", "16", "34", "40", "70"] {
            same2(a, n);
        }
    }
}

#[test]
fn long_to_int_truncation_of_initial_value() {
    // strtol yields a long; assigning it to `int` truncates.
    for a in [
        "2147483648",           // -> INT_MIN
        "2147483649",
        "4294967296",           // -> 0
        "4294967297",           // -> 1
        "4294967295",           // -> -1
        "-2147483649",
        "-4294967296",
        "-4294967297",
        "8589934592",
        "123456789012345",
    ] {
        same2(a, "6");
    }
}

#[test]
fn long_to_int_truncation_of_iteration_count() {
    for n in [
        "4294967296", // -> 0 iterations
        "4294967300", // -> 4 iterations
        "8589934592", // -> 0 iterations
        "2147483648", // -> INT_MIN, negative, no iterations
        "-4294967296",
        "-4294967292", // -> 4 iterations
    ] {
        same2("5", n);
    }
}

#[test]
fn strtol_range_saturation() {
    // Out-of-range conversions saturate to LONG_MAX / LONG_MIN, which then
    // truncate to -1 / 0 respectively.
    for a in [
        "9223372036854775807",   // LONG_MAX exactly
        "9223372036854775808",   // overflow -> LONG_MAX
        "99999999999999999999999999",
        "-9223372036854775808",  // LONG_MIN exactly
        "-9223372036854775809",  // overflow -> LONG_MIN
        "-99999999999999999999999999",
        "+9223372036854775808",
    ] {
        same2(a, "5");
        same2("5", a);
    }
}

// ---------------------------------------------------------------------------
// Output formatting and stream behavior
// ---------------------------------------------------------------------------

#[test]
fn long_output_is_byte_identical() {
    // Enough iterations that stdio buffering boundaries are crossed.
    same2("1", "500");
    same2("-100", "1000");
    same2("123456", "2000");
}

#[test]
fn nothing_is_written_to_stderr() {
    // The C code uses printf for its error messages, so stderr stays empty on
    // every path; assert_same already compares it, this pins the expectation.
    for args in [
        vec![arg(b"5"), arg(b"5")],
        vec![arg(b"abc"), arg(b"5")],
        vec![arg(b"5"), arg(b"abc")],
        vec![arg(b"1")],
    ] {
        let c = exec(&c_bin(), &args);
        let r = exec(&rust_bin(), &args);
        assert!(c.stderr.is_empty(), "C wrote to stderr: {:?}", c);
        assert_eq!(c.stderr, r.stderr);
    }
}

#[test]
fn closed_stdout_pipe_terminates_the_same_way() {
    // The C program has the default SIGPIPE disposition; a Rust program does
    // not unless it restores it. With a reader that goes away mid-stream both
    // must end the same way.
    fn run_with_early_close(exe: &Path) -> Result<i32, i32> {
        let mut child = Command::new(exe)
            .args(["7", "5000000"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));

        {
            // Read a little, then drop the read end to break the pipe.
            let mut pipe = child.stdout.take().expect("stdout was piped");
            let mut buf = [0u8; 64];
            let _ = pipe.read(&mut buf);
        }

        let status = child.wait().expect("failed to wait for child");
        match status.code() {
            Some(code) => Ok(code),
            None => Err(status.signal().unwrap_or(-1)),
        }
    }

    let c = run_with_early_close(&c_bin());
    let r = run_with_early_close(&rust_bin());
    assert_eq!(
        c, r,
        "termination on a closed stdout pipe differs: C={c:?} Rust={r:?}"
    );
}

// ---------------------------------------------------------------------------
// A broad sweep, so the whole enumerated input space is covered in one place
// ---------------------------------------------------------------------------

#[test]
fn deterministic_sweep_over_all_input_classes() {
    let values = [
        "0", "1", "-1", "2", "-2", "3", "-3", "10", "-10", "63", "-63",
        "2147483647", "-2147483648", "2147483648", "4294967296",
        "9223372036854775808", "-9223372036854775809",
        "", " ", "abc", "+", "-", "12abc", "  -3  ", "0x10", ".5",
    ];
    let counts = ["0", "1", "2", "3", "5", "33", "64", "-1", "abc", "", "+"];
    for a in values {
        for n in counts {
            same2(a, n);
        }
    }
}
