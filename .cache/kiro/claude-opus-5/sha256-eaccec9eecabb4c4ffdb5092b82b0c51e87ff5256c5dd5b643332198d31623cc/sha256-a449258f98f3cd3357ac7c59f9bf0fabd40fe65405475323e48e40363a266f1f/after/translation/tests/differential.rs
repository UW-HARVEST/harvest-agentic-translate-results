//! Differential tests: run the C binary and the Rust binary as subprocesses on
//! identical stdin and require byte-identical stdout, byte-identical stderr and
//! an identical exit status.
//!
//! The Rust code is deliberately NOT called as a library. `c_src/src/main.c` is
//! the ground truth; these tests only ever compare two processes.
//!
//! ## Branches of the C program that must be covered
//!
//! `main`:
//!   M0  `scanf("%d %d %d", &x, &y, &z)` completes 0 conversions -> x=0, y=123, z=0
//!   M1  completes 1 conversion          -> x set,  y=123 (initial), z=0
//!   M2  completes 2 conversions         -> x, y set, z=0
//!   M3  completes 3 conversions
//!   M4  `printf("Result: %d\n", result)` and `return 0` on every path
//!
//! `multi_stage(x, z)` (reads the file-scope `y` directly, it is not a parameter):
//!   S1  x != 1                      -> "Error: x != 1",                          result 1, goto fail
//!   S2  x == 1, y != 2              -> "Error: x == 1 but y != 2",                result 2, goto fail
//!   S3  x == 1, y == 2, z != 3      -> "Error: x == 1 and y == 2, but z != 3",    result 3, goto fail
//!   S4  x == 1, y == 2, z == 3      -> "Ok!", returns *before* the fail label, so
//!                                      "Operation failed" is NOT printed
//!   S5  the `fail:` label           -> "Operation failed"
//!
//! Because `y` is only assigned by the *second* `%d`, a short input leaves it at
//! 123, which is what makes M1 observable as S2.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Locating / building the two executables
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the built C executable, building it with CMake on first use.
///
/// `get_or_init` serializes this across the parallel test threads, so CMake is
/// only ever invoked once.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = repo_root().join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");

        if !exe.exists() {
            std::fs::create_dir_all(&build).expect("create c_src/build");

            let cfg = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("failed to run `cmake ..` (is cmake installed?)");
            assert!(
                cfg.status.success(),
                "cmake configure failed:\n{}\n{}",
                String::from_utf8_lossy(&cfg.stdout),
                String::from_utf8_lossy(&cfg.stderr)
            );

            let bld = Command::new("cmake")
                .args(["--build", "."])
                .current_dir(&build)
                .output()
                .expect("failed to run `cmake --build .`");
            assert!(
                bld.status.success(),
                "cmake build failed:\n{}\n{}",
                String::from_utf8_lossy(&bld.stdout),
                String::from_utf8_lossy(&bld.stderr)
            );
        }

        assert!(
            exe.exists(),
            "C executable missing after build: {}",
            exe.display()
        );
        exe
    })
}

/// Path to the Rust executable under test. Cargo builds it for us and hands the
/// path to integration tests through this env var.
fn rust_bin() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

// ---------------------------------------------------------------------------
// Running one program
// ---------------------------------------------------------------------------

struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Ok(code)` for a normal exit, `Err(signal)` if killed by a signal.
    status: Result<i32, i32>,
}

fn run(exe: &Path, input: &[u8]) -> Run {
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));

    // Feed stdin from a helper thread. The program may exit without consuming
    // all of its input (scanf stops at the first failed conversion), so a write
    // error such as BrokenPipe is expected and deliberately ignored - that is
    // also what a shell redirect looks like from the program's point of view.
    let mut stdin = child.stdin.take().expect("stdin was piped");
    let owned = input.to_vec();
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&owned);
        let _ = stdin.flush();
        drop(stdin);
    });

    let out = child.wait_with_output().expect("wait_with_output");
    let _ = writer.join();

    let status = {
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            match out.status.code() {
                Some(c) => Ok(c),
                None => Err(out.status.signal().unwrap_or(-1)),
            }
        }
        #[cfg(not(unix))]
        {
            Ok(out.status.code().unwrap_or(-1))
        }
    };

    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        status,
    }
}

// ---------------------------------------------------------------------------
// The differential assertion
// ---------------------------------------------------------------------------

fn show(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => format!("{s:?}"),
        Err(_) => format!("<non-utf8: {}>", hex(bytes)),
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Runs both programs on `input` and asserts stdout, stderr and exit status all
/// match byte for byte.
#[track_caller]
fn assert_same(label: &str, input: &[u8]) {
    // Set DIFF_TRACE=1 to list every case that is actually compared, so the
    // coverage count can be verified rather than estimated.
    if std::env::var_os("DIFF_TRACE").is_some() {
        eprintln!("DIFFCASE {label}");
    }

    let c = run(c_bin(), input);
    let r = run(rust_bin(), input);

    let ctx = || {
        let shown = if input.len() > 200 {
            format!("<{} bytes, starts {}>", input.len(), show(&input[..80]))
        } else {
            show(input)
        };
        format!("case {label:?}\ninput = {shown}")
    };

    assert_eq!(
        c.stdout,
        r.stdout,
        "\n{}\nSTDOUT differs\n  C    = {}\n  Rust = {}\n  C hex    = {}\n  Rust hex = {}",
        ctx(),
        show(&c.stdout),
        show(&r.stdout),
        hex(&c.stdout),
        hex(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "\n{}\nSTDERR differs\n  C    = {}\n  Rust = {}",
        ctx(),
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        c.status,
        r.status,
        "\n{}\nEXIT STATUS differs (Ok = exit code, Err = signal)\n  C    = {:?}\n  Rust = {:?}",
        ctx(),
        c.status,
        r.status
    );
}

/// Convenience wrapper for the many textual cases.
#[track_caller]
fn same(label: &str, input: &str) {
    assert_same(label, input.as_bytes());
}

// ===========================================================================
// Phase A sanity: both executables exist and run
// ===========================================================================

#[test]
fn both_binaries_exist_and_run() {
    assert!(c_bin().exists(), "C binary not built: {}", c_bin().display());
    assert!(
        rust_bin().exists(),
        "Rust binary not built: {}",
        rust_bin().display()
    );
    // A trivial run must succeed for both, otherwise every other test is
    // measuring nothing.
    let c = run(c_bin(), b"1 2 3");
    let r = run(rust_bin(), b"1 2 3");
    assert_eq!(c.status, Ok(0), "C program did not exit 0 on the happy path");
    assert_eq!(r.status, Ok(0), "Rust program did not exit 0 on the happy path");
    assert_eq!(c.stdout, b"Ok!\nResult: 0\n");
    assert_eq!(r.stdout, c.stdout);
}

// ===========================================================================
// Phase B: the four multi_stage outcomes and the four scanf arities
// ===========================================================================

/// M0 + S1 + S5: nothing to read, so x stays 0.
#[test]
fn m0_empty_input() {
    same("empty", "");
    same("single newline", "\n");
    same("whitespace only", "   \t  \n  ");
    same("all C isspace bytes", " \t\n\u{0b}\u{0c}\r");
}

/// M1: only x is converted, so `y` keeps its initial 123 and S2 fires.
#[test]
fn m1_one_conversion() {
    same("x=1 only", "1");
    same("x=1 only, newline", "1\n");
    same("x=1 then trailing whitespace", "1 \t\n\u{0b}\u{0c}\r ");
    same("x=1 then non-numeric", "1 abc");
    same("x=1 then non-numeric then number", "1 abc 3");
    same("x=1 then lone minus", "1 -");
    same("x=1 then minus space", "1 - 2 3");
    same("x=1 then NUL", "1 \0 2 3");
}

/// M2: x and y converted, z stays 0 and S3 fires.
#[test]
fn m2_two_conversions() {
    same("x=1 y=2", "1 2");
    same("x=1 y=2 newline", "1 2\n");
    same("x=1 y=2 then junk", "1 2 abc");
    same("x=1 y=2 then dot", "1 2 .");
    same("x=1 y=2 then whitespace", "1 2 \t\n ");
}

/// M3 + S4: the only path that prints "Ok!" and skips "Operation failed".
#[test]
fn m3_s4_success_path() {
    same("happy space separated", "1 2 3");
    same("happy trailing newline", "1 2 3\n");
    same("happy newline separated", "1\n2\n3\n");
    same("happy blank lines between", "1\n\n\n2\n\n\n3\n");
    same("happy tab separated", "1\t2\t3");
    same("happy CR separated", "1\r2\r3");
    same("happy vtab/formfeed separated", "1\u{0b}2\u{0c}3");
    same("happy CRLF separated", "1\r\n2\r\n3\r\n");
    same("happy leading whitespace", "   \n\t 1 2 3 \n");
    same("happy explicit plus signs", "+1 +2 +3");
    same("happy leading zeros", "001 002 003");
    same("happy extra trailing tokens", "1 2 3 4 5 6");
    same("happy trailing junk", "1 2 3 abcdef");
    same("happy z followed by fraction", "1 2 3.5");
    same("happy second line ignored", "1 2 3\n1 2 3\n");
}

/// S1: x != 1 short-circuits before y or z are ever examined.
#[test]
fn s1_x_not_one() {
    same("x=0", "0");
    same("x=2", "2 2 3");
    same("x=-1", "-1 2 3");
    same("x all negative", "-1 -2 -3");
    same("x=7 via leading zeros", "007 2 3");
    same("x non-numeric", "abc");
    same("x non-numeric then valid numbers", "abc 1 2 3");
    same("x=123 as one token", "123");
    same("x=INT_MAX", "2147483647 2 3");
    same("x=INT_MIN", "-2147483648 2 3");
    // The whole-line values are wrong but y and z are still consumed; only the
    // x check is reported.
    same("x=99 y=2 z=3", "99 2 3");
}

/// S2: x == 1 but the global y is not 2.
#[test]
fn s2_y_not_two() {
    same("y=3", "1 3 3");
    same("y=0", "1 0 3");
    same("y=-2", "1 -2 3");
    same("y default 123", "1");
    same("y=2 but written as 2.x is fine; y=2.5 -> y=2", "1 2.5 3"); // consumes "2", then ".5" fails z
    same("y non-numeric", "1 x 3");
}

/// S3: x == 1, y == 2, z != 3.
#[test]
fn s3_z_not_three() {
    same("z=4", "1 2 4");
    same("z=-3", "1 2 -3");
    same("z=0 default", "1 2");
    same("z=33", "1 2 33");
    same("z=INT_MIN", "1 2 -2147483648");
}

// ===========================================================================
// Phase C: paths the happy-path tests do not reach
// ===========================================================================

/// `%d` uses strtol semantics: base 10 only, so `0x`, `e` and `_` all terminate
/// the token after the leading digits.
#[test]
fn strtol_token_termination() {
    same("hex in x", "0x10 2 3");
    same("hex in y", "1 0x2 3");
    same("hex in z", "1 2 0x3");
    same("scientific notation", "1e5 2 3");
    same("underscore separator", "1_2 2 3");
    same("comma separated", "1,2,3");
    same("semicolon separated", "1;2;3");
    same("fraction in x", "1.5 2 3");
    same("fraction in y", "1 2.5 3");
    same("octal-looking", "010 020 030");
}

/// A sign with no digits after it is a matching failure, not a zero.
#[test]
fn sign_without_digits() {
    same("lone minus", "-");
    same("lone plus", "+");
    same("lone minus newline", "-\n");
    same("minus space digit", "- 1 2 3");
    same("plus then minus", "+-1 2 3");
    same("double minus", "--1 2 3");
    same("minus at end after x", "1 2 -");
    same("plus at end after x and y", "1 2 +");
}

/// glibc's `%d` collects the digits and runs them through `strtol`, which
/// *saturates* at LONG_MAX / LONG_MIN on overflow; the saturated `long` is then
/// truncated when stored through the `int *`. That is observably different from
/// wrapping 64-bit arithmetic, so each of these pins the behavior down.
#[test]
fn overflow_saturation_and_truncation() {
    // Fits in a long, truncates to 1/2/3 -> success path.
    same("x = 2^32 + 1 truncates to 1", "4294967297 2 3");
    same("x = 2^33 + 1 truncates to 1", "8589934593 2 3");
    same("y = 2^32 + 2 truncates to 2", "1 4294967298 3");
    same("z = 2^32 + 3 truncates to 3", "1 2 4294967299");
    same("x negative wrap to 1", "-4294967295 2 3");

    // Exactly at the long boundaries.
    same("y = LONG_MAX", "1 9223372036854775807 3");
    same("y = LONG_MIN", "1 -9223372036854775808 3");
    same("x = LONG_MAX", "9223372036854775807 2 3");
    same("x = LONG_MIN", "-9223372036854775808 2 3");

    // Past the long boundaries: saturation, NOT wraparound. If the Rust side
    // wrapped instead of saturating, `2^64 + 2` would land on y == 2 and print
    // "Ok!" where the C prints the y error.
    same("y = 2^64 + 2 (saturates, does not wrap to 2)", "1 18446744073709551618 3");
    same("z = 2^64 + 3 (saturates, does not wrap to 3)", "1 2 18446744073709551619");
    same("x = 2^64 + 1 (saturates, does not wrap to 1)", "18446744073709551617 2 3");
    same("y = -(2^64 - 2) (saturates, does not wrap to 2)", "1 -18446744073709551614 3");
    same("y = LONG_MAX + 1", "1 9223372036854775808 3");
    same("y = LONG_MIN - 1", "1 -9223372036854775809 3");
    same("x = 20 nines", "99999999999999999999 2 3");
    same("y = 21 digits", "1 184467440737095516180 3");
    same("x = INT_MAX + 1", "2147483648 2 3");
    same("x = 2^63 - 2^31 + 2", "9223372034707292162 2 3");
}

/// Leading zeros must not be counted toward overflow, and the digit run may be
/// arbitrarily long.
#[test]
fn very_long_digit_runs() {
    same("23 leading zeros then 1", "00000000000000000000001 2 3");
    same("5001-digit x", &format!("1{} 2 3", "0".repeat(5000)));
    same("5000 leading zeros then 1", &format!("{}1 2 3", "0".repeat(5000)));
    // >100k digits, all significant: forces the widest possible accumulation.
    same("100k-digit y ending in 2", &format!("1 {}2 3", "0".repeat(100_000)));
    same("100k nines in x", &format!("{} 2 3", "9".repeat(100_000)));
}

/// Bytes that are neither digits, signs nor `isspace`.
#[test]
fn non_ascii_and_control_bytes() {
    assert_same("NUL first", b"\x001 2 3");
    assert_same("NUL after x", b"1\x00 2 3");
    assert_same("NUL between y digits", b"1 \x002 3");
    assert_same("0xFF 0xFE prefix", b"\xff\xfe 1 2 3");
    assert_same("DEL byte", b"1 \x7f2 3");
    assert_same("UTF-8 e-acute", b"1 \xc3\xa9 3");
    assert_same("invalid UTF-8 mid stream", b"1 2 \xc3\x28");
    assert_same("all high bytes", b"\x80\x81\x82\x83");
    assert_same("bell/backspace/escape", b"1\x072\x083\x1b");
    // 0x1c..0x1f are not isspace in the C locale even though some tables list
    // them as separators.
    assert_same("0x1c-0x1f separators", b"1\x1c2\x1d3");
}

/// Huge amounts of leading whitespace, and huge amounts of unread trailing
/// input (the program exits without draining stdin).
#[test]
fn large_and_unread_input() {
    same("10k leading spaces", &format!("{}1 2 3", " ".repeat(10_000)));
    same("10k newlines between tokens", &format!("1{}2{}3", "\n".repeat(10_000), "\n".repeat(10_000)));
    same("100k unread trailing bytes", &format!("1 2 3 {}", "x".repeat(100_000)));
    // Fails on x, then leaves ~1 MiB unread.
    same("fails early, 1MiB unread", &format!("q{}", "y".repeat(1_048_576)));
}

/// stdout must carry every message; stderr must stay empty on every path.
#[test]
fn stderr_is_always_empty_and_exit_is_always_zero() {
    for (label, input) in [
        ("x error", "9 9 9"),
        ("y error", "1 9 9"),
        ("z error", "1 2 9"),
        ("ok", "1 2 3"),
        ("empty", ""),
        ("garbage", "!!!"),
    ] {
        // The differential assertion is the real check; these extra assertions
        // document what the C program does so a regression is obvious.
        let c = run(c_bin(), input.as_bytes());
        assert!(
            c.stderr.is_empty(),
            "C wrote to stderr for {label:?}: {}",
            show(&c.stderr)
        );
        assert_eq!(c.status, Ok(0), "C did not exit 0 for {label:?}");
        assert_same(label, input.as_bytes());
    }
}

/// `main()` takes no arguments, so argv must have no effect.
#[test]
fn argv_is_ignored() {
    let input = b"1 2 3";
    for args in [vec!["foo"], vec!["--help"], vec!["-1", "-2", "-3"]] {
        let mut c = Command::new(c_bin());
        let mut r = Command::new(rust_bin());
        c.args(&args);
        r.args(&args);
        let out_c = with_args(c, input);
        let out_r = with_args(r, input);
        assert_eq!(
            out_c, out_r,
            "argv {args:?} produced different results\n  C    = {:?}\n  Rust = {:?}",
            out_c, out_r
        );
    }
}

type Captured = (Vec<u8>, Vec<u8>, Option<i32>);

fn with_args(mut cmd: Command, input: &[u8]) -> Captured {
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    let mut stdin = child.stdin.take().unwrap();
    let owned = input.to_vec();
    let w = std::thread::spawn(move || {
        let _ = stdin.write_all(&owned);
    });
    let out = child.wait_with_output().expect("wait");
    let _ = w.join();
    (out.stdout, out.stderr, out.status.code())
}

/// stdin that is not a readable stream of text: a closed descriptor and a
/// directory both make the first `%d` fail, giving the M0/S1 path.
#[test]
fn unreadable_stdin() {
    // stdin connected to /dev/null (immediate EOF).
    let devnull = std::fs::File::open("/dev/null").expect("open /dev/null");
    let c = Command::new(c_bin())
        .stdin(Stdio::from(devnull))
        .output()
        .expect("run C with /dev/null stdin");
    let devnull = std::fs::File::open("/dev/null").expect("open /dev/null");
    let r = Command::new(rust_bin())
        .stdin(Stdio::from(devnull))
        .output()
        .expect("run Rust with /dev/null stdin");
    assert_eq!(c.stdout, r.stdout, "/dev/null stdin: stdout differs");
    assert_eq!(c.stderr, r.stderr, "/dev/null stdin: stderr differs");
    assert_eq!(c.status.code(), r.status.code(), "/dev/null stdin: exit differs");

    // stdin connected to a directory: read(2) returns EISDIR.
    let dir = std::fs::File::open(repo_root()).expect("open repo root as file");
    let c = Command::new(c_bin())
        .stdin(Stdio::from(dir))
        .output()
        .expect("run C with directory stdin");
    let dir = std::fs::File::open(repo_root()).expect("open repo root as file");
    let r = Command::new(rust_bin())
        .stdin(Stdio::from(dir))
        .output()
        .expect("run Rust with directory stdin");
    assert_eq!(c.stdout, r.stdout, "directory stdin: stdout differs");
    assert_eq!(c.stderr, r.stderr, "directory stdin: stderr differs");
    assert_eq!(c.status.code(), r.status.code(), "directory stdin: exit differs");
}

/// A failing stdout (write to /dev/full returns ENOSPC). The C program ignores
/// every `printf` return value, so it must still exit 0; the Rust program has to
/// do the same rather than panicking on the write error.
#[test]
fn stdout_write_failure_is_ignored() {
    let full = match std::fs::OpenOptions::new().write(true).open("/dev/full") {
        Ok(f) => f,
        Err(e) => {
            // /dev/full is standard on Linux. Refuse to pass quietly there: a
            // missing device would mean this check never ran.
            if cfg!(target_os = "linux") {
                panic!("/dev/full is required to run this check on Linux: {e}");
            }
            return;
        }
    };
    let c = Command::new(c_bin())
        .stdin(Stdio::null())
        .stdout(Stdio::from(full))
        .stderr(Stdio::piped())
        .output()
        .expect("run C to /dev/full");
    let full = std::fs::OpenOptions::new().write(true).open("/dev/full").unwrap();
    let r = Command::new(rust_bin())
        .stdin(Stdio::null())
        .stdout(Stdio::from(full))
        .stderr(Stdio::piped())
        .output()
        .expect("run Rust to /dev/full");
    assert_eq!(c.stderr, r.stderr, "/dev/full: stderr differs");
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "/dev/full: exit differs (C={:?} Rust={:?})",
        c.status.code(),
        r.status.code()
    );
}

// ===========================================================================
// Broad table sweep + deterministic fuzz
// ===========================================================================

/// Every combination of x in {wrong, 1}, y in {wrong, 2} and z in {wrong, 3},
/// crossed with each separator style, so no ordering of the three checks in
/// `multi_stage` can hide behind another.
#[test]
fn full_truth_table_across_separators() {
    let xs = ["1", "2", "0", "-1"];
    let ys = ["2", "3", "0", "-2"];
    let zs = ["3", "4", "0", "-3"];
    let seps = [" ", "\n", "\t", "  \n\t ", "\r\n"];
    for x in xs {
        for y in ys {
            for z in zs {
                for s in seps {
                    let input = format!("{x}{s}{y}{s}{z}");
                    same(&format!("table x={x} y={y} z={z} sep={s:?}"), &input);
                }
            }
        }
    }
}

/// Truncated prefixes of a valid input: exercises every point at which the
/// input can end mid-token or between tokens.
#[test]
fn every_prefix_of_a_valid_input() {
    let full = "  12 -34 567  ";
    for n in 0..=full.len() {
        same(&format!("prefix len {n}"), &full[..n]);
    }
    let full2 = "1 2 3";
    for n in 0..=full2.len() {
        same(&format!("happy prefix len {n}"), &full2[..n]);
    }
}

/// Deterministic pseudo-random inputs built from an interesting token pool.
/// Fixed seed so a failure is reproducible.
#[test]
fn deterministic_fuzz() {
    const TOKENS: &[&str] = &[
        "0", "1", "2", "3", "4", "-1", "-2", "-3", "+1", "+2", "+3", "007", "001", "002", "003",
        "abc", "x", "-", "+", ".", "0x2", "1.5", "1e5", "2147483647", "-2147483648", "2147483648",
        "4294967297", "4294967298", "4294967299", "9223372036854775807", "9223372036854775808",
        "-9223372036854775808", "-9223372036854775809", "18446744073709551617",
        "18446744073709551618", "99999999999999999999", ",", ";", "#", "_", "\u{0}", "\u{7f}",
    ];
    const SEPS: &[&str] = &[" ", "\n", "\t", "\r", "\u{0b}", "\u{0c}", "  ", " \n ", "\n\n", ""];

    // xorshift64*, so the sequence is fixed and portable.
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state = state.wrapping_mul(0x2545_F491_4F6C_DD1D);
        state
    };

    for i in 0..600 {
        let ntok = (next() % 6) as usize;
        let mut input = String::new();
        if next() % 3 == 0 {
            input.push_str(SEPS[(next() % SEPS.len() as u64) as usize]);
        }
        for _ in 0..ntok {
            input.push_str(TOKENS[(next() % TOKENS.len() as u64) as usize]);
            input.push_str(SEPS[(next() % SEPS.len() as u64) as usize]);
        }
        same(&format!("fuzz #{i}"), &input);
    }
}

/// Deterministic pseudo-random raw byte strings, including bytes that are not
/// valid UTF-8.
#[test]
fn deterministic_byte_fuzz() {
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state = state.wrapping_mul(0x2545_F491_4F6C_DD1D);
        state
    };

    for i in 0..400 {
        let len = (next() % 32) as usize;
        let mut buf = Vec::with_capacity(len);
        for _ in 0..len {
            // Bias toward the interesting byte classes but allow anything.
            let r = next();
            let b = match r % 4 {
                0 => b'0' + (r >> 8) as u8 % 10,
                1 => *b" \t\n\r\x0b\x0c".get((r >> 8) as usize % 6).unwrap(),
                2 => *b"+-.,;xX\0\x7f\xff".get((r >> 8) as usize % 10).unwrap(),
                _ => (r >> 8) as u8,
            };
            buf.push(b);
        }
        assert_same(&format!("byte fuzz #{i}"), &buf);
    }
}

/// The store through `int *` truncates to exactly 32 bits - not 8, not 16, not
/// 64. These inputs separate those widths: `65537` has low 16 bits equal to 1
/// but is not 1 as an `int`, and `257` has low 8 bits equal to 1 but is not 1 as
/// an `int`. A translation that truncated too narrowly would print "Ok!" here
/// where the C prints the x error.
#[test]
fn truncation_width_is_exactly_32_bits() {
    // Low 8 bits are 1/2/3 but the int value is not.
    for k in 1u32..=8 {
        let x = k * 256 + 1;
        let y = k * 256 + 2;
        let z = k * 256 + 3;
        same(&format!("8-bit alias x={x}"), &format!("{x} 2 3"));
        same(&format!("8-bit alias y={y}"), &format!("1 {y} 3"));
        same(&format!("8-bit alias z={z}"), &format!("1 2 {z}"));
        same(&format!("8-bit alias all {x} {y} {z}"), &format!("{x} {y} {z}"));
    }
    // Low 16 bits are 1/2/3 but the int value is not.
    for k in 1u32..=8 {
        let x = k * 65536 + 1;
        let y = k * 65536 + 2;
        let z = k * 65536 + 3;
        same(&format!("16-bit alias x={x}"), &format!("{x} 2 3"));
        same(&format!("16-bit alias y={y}"), &format!("1 {y} 3"));
        same(&format!("16-bit alias z={z}"), &format!("1 2 {z}"));
        same(&format!("16-bit alias all {x} {y} {z}"), &format!("{x} {y} {z}"));
    }
    // Negative counterparts: -65535 has low 16 bits 0x0001.
    same("x=-65535 (low16 == 1)", "-65535 2 3");
    same("y=-65534 (low16 == 2)", "1 -65534 3");
    same("z=-65533 (low16 == 3)", "1 2 -65533");
    same("x=-255 (low8 == 1)", "-255 2 3");
    same("y=-254 (low8 == 2)", "1 -254 3");
    same("z=-253 (low8 == 3)", "1 2 -253");
    // Values that are 1/2/3 in 32 bits but NOT in 16 bits, to catch a widening
    // mistake in the other direction.
    same("x = 2^32+1 (int 1, but 1 in 16 bits too)", "4294967297 2 3");
    same("x = 2^32+65537", "4294967361 2 3");
    // Sign-extension boundaries of the narrower widths.
    for v in [127i64, 128, 129, 255, 256, 32767, 32768, 32769, 65535, 65536] {
        same(&format!("width boundary x={v}"), &format!("{v} 2 3"));
        same(&format!("width boundary y={v}"), &format!("1 {v} 3"));
        same(&format!("width boundary z={v}"), &format!("1 2 {v}"));
        same(&format!("width boundary x=-{v}"), &format!("-{v} 2 3"));
        same(&format!("width boundary y=-{v}"), &format!("1 -{v} 3"));
        same(&format!("width boundary z=-{v}"), &format!("1 2 -{v}"));
    }
}

/// Every long value whose low 32 bits are 1, 2 or 3, swept across the top of the
/// long range: catches any off-by-one in truncation or in the overflow cutoff.
#[test]
fn low_32_bits_sweep() {
    for k in 0u64..48 {
        for lo in 1u64..=3 {
            let v = k * (1u64 << 32) + lo;
            same(
                &format!("x = {k}*2^32 + {lo}"),
                &format!("{v} {} {}", k * (1u64 << 32) + 2, k * (1u64 << 32) + 3),
            );
        }
    }
    // Just below, at, and just above the LONG_MAX cutoff.
    for v in [
        u64::from(u32::MAX),
        (1u64 << 63) - 3,
        (1u64 << 63) - 2,
        (1u64 << 63) - 1,
        1u64 << 63,
        (1u64 << 63) + 1,
        (1u64 << 63) + 2,
        u64::MAX - 1,
        u64::MAX,
    ] {
        same(&format!("y = {v}"), &format!("1 {v} 3"));
        same(&format!("x = {v}"), &format!("{v} 2 3"));
        same(&format!("y = -{v}"), &format!("1 -{v} 3"));
    }
}
