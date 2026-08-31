//! Differential tests: run the C binary and the Rust binary as subprocesses on
//! identical stdin and require byte-identical stdout, byte-identical stderr and
//! an identical exit status (including termination-by-signal).
//!
//! The Rust code is never linked in as a library; only the built executable is
//! driven, because that is how the two programs are compared.

use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// Path to the Rust executable under test, provided by Cargo.
const RUST_BIN: &str = env!("CARGO_BIN_EXE_driver");

fn repo_root() -> PathBuf {
    // .../translation/  ->  .../
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Build `c_src` with CMake once per test binary and return the executable path.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = repo_root().join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");

        if !exe.is_file() {
            std::fs::create_dir_all(&build).expect("create c_src/build");

            let configure = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("failed to run `cmake ..` (is cmake installed?)");
            assert!(
                configure.status.success(),
                "cmake configure failed:\n{}\n{}",
                String::from_utf8_lossy(&configure.stdout),
                String::from_utf8_lossy(&configure.stderr)
            );

            let compile = Command::new("cmake")
                .args(["--build", "."])
                .current_dir(&build)
                .output()
                .expect("failed to run `cmake --build .`");
            assert!(
                compile.status.success(),
                "cmake build failed:\n{}\n{}",
                String::from_utf8_lossy(&compile.stdout),
                String::from_utf8_lossy(&compile.stderr)
            );
        }

        assert!(exe.is_file(), "C executable missing at {}", exe.display());
        exe
    })
}

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
            "status={}, stdout={:?}, stderr={:?}",
            match self.status {
                Ok(c) => format!("exit {c}"),
                Err(s) => format!("signal {s}"),
            },
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr),
        )
    }
}

/// Run `exe` with `input` on stdin, capturing everything.
fn run(exe: &Path, input: &[u8]) -> Outcome {
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));

    {
        let mut stdin = child.stdin.take().expect("piped stdin");
        // The child may die on SIGFPE before draining stdin; a broken pipe here
        // is expected and must not fail the test.
        let _ = stdin.write_all(input);
        let _ = stdin.flush();
    }

    let out = child.wait_with_output().expect("wait for child");
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        status: match (out.status.code(), out.status.signal()) {
            (Some(code), _) => Ok(code),
            (None, Some(sig)) => Err(sig),
            (None, None) => panic!("child neither exited nor signalled"),
        },
    }
}

/// Assert the two programs agree on stdout, stderr and exit status.
#[track_caller]
fn assert_same(label: &str, input: &[u8]) {
    let c = run(c_bin(), input);
    let r = run(Path::new(RUST_BIN), input);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch for {label} (input {:?})\n  C: {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(input),
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout),
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch for {label} (input {:?})\n  C: {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(input),
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr),
    );
    assert_eq!(
        c.status, r.status,
        "exit status mismatch for {label} (input {:?})\n  C: {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(input),
        c.status,
        r.status,
    );
}

#[track_caller]
fn check_all(cases: &[(&str, &[u8])]) {
    for (label, input) in cases {
        assert_same(label, input);
    }
}

// ---------------------------------------------------------------------------
// Phase A sanity: both executables exist and run.
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_run() {
    let input = b"42 5";
    let c = run(c_bin(), input);
    let r = run(Path::new(RUST_BIN), input);
    assert_eq!(c.stdout, b"quotient: 8, remainder: 2\n");
    assert_eq!(r.stdout, c.stdout);
    assert_eq!(c.status, Ok(0));
    assert_eq!(r.status, Ok(0));
}

// ---------------------------------------------------------------------------
// Happy path and sign combinations. `div` truncates toward zero, so the
// remainder carries the sign of the dividend.
// ---------------------------------------------------------------------------

#[test]
fn happy_path_and_signs() {
    check_all(&[
        ("two positives", b"42 5"),
        ("exact division", b"42 6"),
        ("negative dividend", b"-42 5"),
        ("negative divisor", b"42 -5"),
        ("both negative", b"-42 -5"),
        ("neg dividend odd", b"-7 2"),
        ("neg divisor odd", b"7 -2"),
        ("divide by one", b"9 1"),
        ("divide by minus one", b"9 -1"),
        ("zero dividend", b"0 7"),
        ("zero over negative", b"0 -1"),
        ("minus one over minus one", b"-1 -1"),
        ("divisor larger than dividend", b"1 2147483647"),
    ]);
}

// ---------------------------------------------------------------------------
// Whitespace handling. `%d` skips leading whitespace of every kind, including
// newlines, so `scanf` reads straight across line boundaries.
// ---------------------------------------------------------------------------

#[test]
fn whitespace_variants() {
    check_all(&[
        ("newline separated", b"7\n3\n"),
        ("newline no trailing", b"7\n3"),
        ("blank lines everywhere", b"\n\n\n5\n\n\n2"),
        ("tabs", b"5\t2"),
        ("double space", b"5  2"),
        ("leading whitespace", b"  \t\n  8   \t 2  \n"),
        ("crlf", b"5\r\n2\r\n"),
        ("vertical tab and form feed", b"\x0b\x0c5\x0b2"),
        ("all C isspace bytes between", b"5 \t\n\x0b\x0c\r2"),
        ("trailing space after x", b"5 "),
        ("trailing tab after x", b"5\t"),
    ]);
}

#[test]
fn long_whitespace_prefix() {
    let mut input = vec![b' '; 5000];
    input.extend_from_slice(b"5 2");
    assert_same("5000 spaces then digits", &input);
}

// ---------------------------------------------------------------------------
// Inputs that make a conversion fail. `scanf`'s return value is ignored, so an
// unconverted variable keeps its initialiser of 1.
// ---------------------------------------------------------------------------

#[test]
fn empty_and_missing_operands() {
    check_all(&[
        ("empty input", b""),
        ("single space", b" "),
        ("spaces only", b"   "),
        ("newline only", b"\n"),
        ("tab only", b"\t"),
        ("single item, y defaults to 1", b"42"),
        ("single item with newline", b"42\n"),
        ("single zero, 0/1", b"0"),
        ("single negative", b"-42"),
    ]);
}

#[test]
fn matching_failures() {
    check_all(&[
        ("letters only, x and y both default", b"xyz"),
        ("letters before digits", b"abc 10 3"),
        ("lone minus", b"-"),
        ("lone plus", b"+"),
        ("minus then space then digits", b"- 5"),
        ("plus then space then digits", b"+ 5"),
        ("double plus", b"++5 2"),
        ("double minus", b"--5 2"),
        ("leading underscore", b"_5 2"),
        ("leading dot", b".5 2"),
        ("y is a letter", b"5 z"),
        ("y is a lone minus", b"5 -"),
        ("y is a lone plus", b"5 +"),
        ("high byte before digits", b"\xff5 2"),
        ("nbsp byte is not space in C locale", b"\xa05 2"),
    ]);
}

#[test]
fn trailing_and_embedded_junk() {
    check_all(&[
        ("digits then letters", b"10 3abc"),
        ("junk directly after x", b"5\xff2"),
        ("junk after y", b"5 2\xff"),
        ("comma separated", b"5,2"),
        ("semicolon separated", b"5;2"),
        ("hex literal is not accepted by %d", b"0x10 2"),
        ("float truncates at the dot", b"3.9 2"),
        ("exponent notation stops at e", b"5e2 3"),
        ("extra operands ignored", b"9 2 100 200"),
        ("nul byte terminates the number", b"5\x004"),
        ("nul byte first", b"\x005 2"),
        ("signs accepted", b"+9 +4"),
    ]);
}

#[test]
fn leading_zeros() {
    check_all(&[
        ("leading zeros both", b"0000012 0000005"),
        ("all zeros", b"00000000000000000000000000000000 5"),
        ("zeros then INT_MAX", b"0000000000000000000002147483647 1"),
        ("negative zero dividend", b"-0 5"),
    ]);
}

// ---------------------------------------------------------------------------
// Integer limits. glibc converts `%d` via strtol: out-of-range literals
// saturate to LONG_MIN/LONG_MAX and the result is then truncated into `int`.
// ---------------------------------------------------------------------------

#[test]
fn int_boundaries() {
    check_all(&[
        ("INT_MAX / 1", b"2147483647 1"),
        ("INT_MIN / 1", b"-2147483648 1"),
        ("INT_MIN / 2", b"-2147483648 2"),
        ("INT_MAX / -1", b"2147483647 -1"),
        ("INT_MAX / INT_MIN", b"2147483647 -2147483648"),
        ("INT_MIN / INT_MIN", b"-2147483648 -2147483648"),
        ("1 / INT_MIN", b"1 -2147483648"),
    ]);
}

#[test]
fn out_of_range_literals_truncate() {
    check_all(&[
        ("INT_MAX+1 wraps to INT_MIN", b"2147483648 1"),
        ("INT_MAX+2 wraps", b"2147483649 1"),
        ("UINT_MAX truncates to -1", b"4294967295 1"),
        ("-UINT_MAX truncates to 1", b"-4294967295 1"),
        ("2^32 truncates to 0, x only", b"4294967296 5"),
        ("2^32+1 truncates to 1", b"4294967297 1"),
        ("2^63-1 (LONG_MAX) truncates to -1", b"9223372036854775807 1"),
        ("2^63 saturates to LONG_MAX", b"9223372036854775808 1"),
        ("2^63+1 saturates to LONG_MAX", b"9223372036854775809 1"),
        ("-2^63 (LONG_MIN) truncates to 0", b"-9223372036854775808 1"),
        ("-2^63-1 saturates to LONG_MIN", b"-9223372036854775809 1"),
        ("2^64 saturates", b"18446744073709551616 1"),
        ("2^64+1 saturates", b"18446744073709551617 1"),
        ("2^64+2 saturates", b"18446744073709551618 1"),
        ("-2^64 saturates to LONG_MIN", b"-18446744073709551616 1"),
        ("both operands overflow to INT_MIN", b"2147483648 2147483648"),
        ("y overflows but stays nonzero", b"10 4294967299"),
    ]);
}

#[test]
fn very_long_digit_runs_saturate() {
    for len in [19usize, 20, 21, 100, 1000] {
        let nines = vec![b'9'; len];

        let mut pos = nines.clone();
        pos.extend_from_slice(b" 1");
        assert_same(&format!("{len} nines"), &pos);

        let mut neg = vec![b'-'];
        neg.extend_from_slice(&nines);
        neg.extend_from_slice(b" 1");
        assert_same(&format!("negative {len} nines"), &neg);
    }
}

#[test]
fn long_zero_run_then_digits() {
    let mut input = vec![b'0'; 2000];
    input.extend_from_slice(b"7 2");
    assert_same("2000 zeros then 7", &input);
}

// ---------------------------------------------------------------------------
// Undefined-behaviour paths. `div(x, 0)` and `div(INT_MIN, -1)` execute `idiv`
// with no valid result on x86-64, so the process dies of SIGFPE with no output
// at all -- not an exit code, and nothing on stdout or stderr.
// ---------------------------------------------------------------------------

#[test]
fn division_by_zero_traps() {
    check_all(&[
        ("5 / 0", b"5 0"),
        ("0 / 0", b"0 0"),
        ("1 / 0", b"1 0"),
        ("-5 / 0", b"-5 0"),
        ("negative zero divisor", b"5 -0"),
        ("plus zero divisor", b"5 +0"),
        ("divisor with leading zeros", b"5 000"),
        ("divisor 2^32 truncates to zero", b"5 4294967296"),
        ("divisor -2^32 truncates to zero", b"5 -4294967296"),
        ("divisor 2^63 truncates to zero", b"5 9223372036854775808"),
    ]);
}

#[test]
fn int_min_over_minus_one_traps() {
    check_all(&[
        ("INT_MIN / -1", b"-2147483648 -1"),
        ("INT_MIN / -1 across a newline", b"-2147483648\n-1\n"),
        ("overflowed INT_MIN / -1", b"2147483648 -1"),
    ]);
}

#[test]
fn division_by_zero_produces_no_output() {
    // Guards against a Rust build that prints a panic message or exits 0 where
    // the C process is killed outright.
    let c = run(c_bin(), b"5 0");
    let r = run(Path::new(RUST_BIN), b"5 0");
    assert_eq!(c.status, Err(libc_sigfpe()), "C should die of SIGFPE");
    assert_eq!(r.status, c.status);
    assert!(c.stdout.is_empty() && c.stderr.is_empty());
    assert!(r.stdout.is_empty() && r.stderr.is_empty());
}

fn libc_sigfpe() -> i32 {
    8
}

// ---------------------------------------------------------------------------
// Stdin shapes other than a pipe with data.
// ---------------------------------------------------------------------------

#[test]
fn empty_stdin_from_dev_null() {
    let dev_null = || std::fs::File::open("/dev/null").expect("open /dev/null");

    let mut outs = Vec::new();
    for exe in [c_bin(), Path::new(RUST_BIN)] {
        let out = Command::new(exe)
            .stdin(Stdio::from(dev_null()))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("spawn with /dev/null stdin");
        outs.push((
            out.stdout,
            out.stderr,
            out.status.code(),
            out.status.signal(),
        ));
    }
    assert_eq!(outs[0], outs[1], "mismatch with /dev/null as stdin");
    assert_eq!(outs[0].0, b"quotient: 1, remainder: 0\n");
}

#[test]
fn argv_is_ignored() {
    let mut outs = Vec::new();
    for exe in [c_bin(), Path::new(RUST_BIN)] {
        let mut child = Command::new(exe)
            .args(["alpha", "-42", "0"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn with argv");
        child
            .stdin
            .take()
            .unwrap()
            .write_all(b"9 2")
            .expect("write stdin");
        let out = child.wait_with_output().expect("wait");
        outs.push((
            out.stdout,
            out.stderr,
            out.status.code(),
            out.status.signal(),
        ));
    }
    assert_eq!(outs[0], outs[1], "mismatch when argv carries extra values");
    assert_eq!(outs[0].0, b"quotient: 4, remainder: 1\n");
}

// ---------------------------------------------------------------------------
// Exhaustive-ish sweep over small operands: every sign and magnitude pairing,
// including every divisor of zero, in one go.
// ---------------------------------------------------------------------------

#[test]
fn small_operand_grid() {
    for x in -6i64..=6 {
        for y in -6i64..=6 {
            let input = format!("{x} {y}");
            assert_same(&format!("grid {x} / {y}"), input.as_bytes());
        }
    }
}

#[test]
fn formatting_is_byte_identical() {
    // Widths, the ", " separator and the single trailing newline must match
    // exactly for multi-digit and negative results.
    for input in [
        "1000000 3",
        "-1000000 3",
        "2147483647 2",
        "-2147483647 2",
        "123456789 1000",
    ] {
        assert_same(input, input.as_bytes());
        let c = run(c_bin(), input.as_bytes());
        assert!(
            c.stdout.ends_with(b"\n") && !c.stdout.ends_with(b"\n\n"),
            "expected exactly one trailing newline"
        );
    }
}

// ---------------------------------------------------------------------------
// Phase C: input classes the earlier tests did not reach.
// ---------------------------------------------------------------------------

#[test]
fn mixed_sign_prefixes() {
    check_all(&[
        ("plus then minus", b"+-5 2"),
        ("minus then plus", b"-+5 2"),
        ("y has plus then minus", b"5 +-2"),
        ("sign between digits", b"5-2"),
        ("sign directly after digits", b"5+2"),
        ("minus glued to previous number", b"12-3"),
    ]);
}

#[test]
fn bulk_input_is_not_truncated_differently() {
    // A payload far larger than any stdio buffer: the digit run saturates and
    // the trailing operand is never reached, in both programs.
    let mut input = vec![b'7'; 1 << 20];
    input.extend_from_slice(b" 3");
    assert_same("1 MiB of digits", &input);

    // Same size, but as whitespace ahead of a real pair of operands.
    let mut ws = vec![b'\n'; 1 << 20];
    ws.extend_from_slice(b"9 4");
    assert_same("1 MiB of newlines then operands", &ws);
}

#[test]
fn stdin_delivered_in_slow_chunks() {
    // The C program reads through stdio, the Rust one a byte at a time; neither
    // may mistake a short read for end of input.
    for exe in [c_bin(), Path::new(RUST_BIN)] {
        let mut child = Command::new(exe)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn");
        {
            let mut stdin = child.stdin.take().expect("piped stdin");
            for chunk in [&b"-1"[..], b"2", b"3", b" ", b"1", b"0"] {
                stdin.write_all(chunk).expect("write chunk");
                stdin.flush().expect("flush chunk");
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
        }
        let out = child.wait_with_output().expect("wait");
        assert_eq!(
            out.stdout, b"quotient: -12, remainder: -3\n",
            "{} mishandled a chunked stdin",
            exe.display()
        );
        assert_eq!(out.status.code(), Some(0));
    }
}

#[test]
fn locale_environment_does_not_change_parsing() {
    // The C program never calls setlocale, so it stays in the "C" locale and
    // LC_ALL must not affect isspace or digit grouping.
    for locale in ["C", "en_US.UTF-8", "de_DE.UTF-8", "invalid.locale"] {
        for input in [&b"5 2"[..], b"\x855 2", b"\xa05 2", b"1,234 2"] {
            let mut outs = Vec::new();
            for exe in [c_bin(), Path::new(RUST_BIN)] {
                let mut child = Command::new(exe)
                    .env("LC_ALL", locale)
                    .env("LANG", locale)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .expect("spawn with locale env");
                let _ = child.stdin.take().unwrap().write_all(input);
                let out = child.wait_with_output().expect("wait");
                outs.push((out.stdout, out.stderr, out.status.code(), out.status.signal()));
            }
            assert_eq!(
                outs[0],
                outs[1],
                "mismatch under LC_ALL={locale} for input {:?}",
                String::from_utf8_lossy(input)
            );
        }
    }
}

#[test]
fn saturation_boundary_is_exact() {
    // The strtol saturation edge sits at LONG_MIN/LONG_MAX; each side of it
    // truncates to a different int, so an off-by-one in the accumulator shows
    // up here and nowhere else.
    let cases: [i128; 8] = [
        (i64::MAX as i128) - 1,
        i64::MAX as i128,
        (i64::MAX as i128) + 1,
        (i64::MAX as i128) + 2,
        (i64::MIN as i128) + 1,
        i64::MIN as i128,
        (i64::MIN as i128) - 1,
        (i64::MIN as i128) - 2,
    ];
    for v in cases {
        let input = format!("{v} 1");
        assert_same(&format!("saturation edge {v}"), input.as_bytes());
        // Same value with padding zeros, which strtol must ignore.
        let padded = if v < 0 {
            format!("-000000{} 1", -v)
        } else {
            format!("000000{v} 1")
        };
        assert_same(&format!("padded saturation edge {v}"), padded.as_bytes());
    }
}

#[test]
fn divisor_saturation_that_lands_on_zero_traps() {
    // Any literal whose low 32 bits are zero makes the divisor zero, including
    // ones that only get there through strtol saturation plus truncation.
    for divisor in [
        "4294967296",
        "8589934592",
        "-4294967296",
        "18446744073709551616",
        "0000000000",
        "-0000",
    ] {
        let input = format!("7 {divisor}");
        assert_same(&format!("divisor {divisor}"), input.as_bytes());
    }
}
