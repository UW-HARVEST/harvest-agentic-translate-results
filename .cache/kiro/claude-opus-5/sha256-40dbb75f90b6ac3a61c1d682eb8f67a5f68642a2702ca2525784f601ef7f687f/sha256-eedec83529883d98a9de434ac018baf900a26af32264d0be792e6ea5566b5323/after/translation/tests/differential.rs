//! Differential tests: run the C `driver` and the Rust `driver` as
//! subprocesses, feed both the same bytes on stdin, and require that stdout,
//! stderr and the exit status are identical.
//!
//! The Rust program is never used as a library here; it is executed exactly the
//! way a shell would run it, because that is how the two implementations are
//! compared.
//!
//! Commands under test:
//!   C:    <workspace>/c_src/build/driver          (cmake, built on demand)
//!   Rust: $CARGO_BIN_EXE_driver                   (cargo's own build of src/main.rs)

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// locating / building the two executables
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the C executable, building it with cmake the first time if needed.
fn c_binary() -> &'static Path {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let c_src = workspace_root().join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");
        if !exe.is_file() {
            std::fs::create_dir_all(&build).expect("cannot create c_src/build");
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
        assert!(
            exe.is_file(),
            "C executable {} was not produced; build c_src first",
            exe.display()
        );
        exe
    })
    .as_path()
}

/// Path to the Rust executable that cargo built for this test run.
fn rust_binary() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

// ---------------------------------------------------------------------------
// running one program
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Ok(code)` for a normal exit, `Err(signal)` when killed by a signal.
    status: Result<Option<i32>, Option<i32>>,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "stdout={:?} stderr={:?} status={:?}",
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr),
            self.status
        )
    }
}

fn run(program: &Path, input: &[u8]) -> Outcome {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("cannot spawn {}: {e}", program.display()));

    {
        let mut stdin = child.stdin.take().expect("piped stdin");
        // A closed reader is not interesting here, so a broken pipe is ignored.
        let _ = stdin.write_all(input);
        let _ = stdin.flush();
    }

    let out = child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("cannot wait for {}: {e}", program.display()));

    let signal = signal_of(&out.status);
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        status: match signal {
            Some(s) => Err(Some(s)),
            None => Ok(out.status.code()),
        },
    }
}

#[cfg(unix)]
fn signal_of(status: &std::process::ExitStatus) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt;
    status.signal()
}

#[cfg(not(unix))]
fn signal_of(_status: &std::process::ExitStatus) -> Option<i32> {
    None
}

// ---------------------------------------------------------------------------
// the assertion every test funnels through
// ---------------------------------------------------------------------------

#[track_caller]
fn assert_matches(input: &[u8]) {
    let c = run(c_binary(), input);
    let rust = run(rust_binary(), input);
    assert_eq!(
        c.stdout,
        rust.stdout,
        "stdout differs for input {:?}\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(input),
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&rust.stdout)
    );
    assert_eq!(
        c.stderr,
        rust.stderr,
        "stderr differs for input {:?}\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(input),
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&rust.stderr)
    );
    assert_eq!(
        c.status,
        rust.status,
        "exit status differs for input {:?}\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(input),
        c.status,
        rust.status
    );
}

#[track_caller]
fn assert_all(inputs: &[&str]) {
    for input in inputs {
        assert_matches(input.as_bytes());
    }
}

// ---------------------------------------------------------------------------
// Phase A -- the two programs exist and produce output at all
// ---------------------------------------------------------------------------

#[test]
fn both_programs_run_and_agree_on_a_trivial_input() {
    let c = run(c_binary(), b"1.5\n");
    let rust = run(rust_binary(), b"1.5\n");
    assert_eq!(c.stdout, b"3ff8000000000000 0x1.8p+0 1.5000\n".to_vec());
    assert_eq!(c, rust, "trivial input disagrees: {c:?} vs {rust:?}");
}

// ---------------------------------------------------------------------------
// Phase B -- the input classes the C code branches on
// ---------------------------------------------------------------------------

/// `scanf` finds nothing to convert, so `f` keeps its initial `0.0`.
#[test]
fn no_conversion_leaves_the_initial_zero() {
    assert_all(&[
        "",          // immediate EOF
        "\n",        // only a newline
        " ",         // only a blank
        " \t\n\r\u{0b}\u{0c}", // every isspace() character, then EOF
        "abc",       // not a number at all
        "+",         // sign then EOF
        "-",         //
        "--1",       // two signs
        "++1",       //
        ".",         // a lone decimal point
        ".e5",       // point, no digits
        "e1",        // exponent with no significand
        "E",         //
        "/3",        //
        ",5",        //
        "_1",        //
    ]);
}

/// Whitespace is skipped, and `scanf` crosses newlines while doing so.
#[test]
fn leading_whitespace_is_skipped_across_lines() {
    assert_all(&[
        "   42",
        "\t\n  7.25",
        "\n\n\n-1",
        "\r1.5",
        "\u{0b}\u{0c}1.5",
        "\n\n\n",
    ]);
}

/// Plain decimal forms, including the partial-exponent backtracking.
#[test]
fn decimal_forms() {
    assert_all(&[
        "0",
        "0.0",
        "1",
        "-1",
        "+3.5",
        "1.5",
        "3.14159",
        ".5",
        "5.",
        "0.1",
        "1e",     // 'e' with no exponent digits: only "1" is converted
        "1e+",    //
        "1e-",    //
        "1e+5",
        "1E5",
        "1.5e-3",
        "1 2",    // trailing input is left unread
        "1/3",
        "9007199254740993", // not representable: rounds to 2^53
        "1.7976931348623157e308",
        "0.000050000000000000001",
        "0.0000000000000000000000000000001",
    ]);
}

/// Signed zero has to survive, because `%llx` shows the sign bit.
#[test]
fn signed_zeros() {
    assert_all(&[
        "0", "-0", "+0", "-0.0", "-0e0", "-0.0e-999999999", "-0x0p0", "-0x0", "0e99999999999999999999",
    ]);
}

/// The `0x` prefix: accepted, and the two ways it can fail.
#[test]
fn hexadecimal_forms() {
    assert_all(&[
        "0x1p3",
        "0X1P-5",
        "0x1",
        "0x10",
        "0x1.8",
        "0x1.8p-2",
        "0x.8p1",
        "0x1p",     // 'p' with no digits: backtracks to the significand
        "0x1p+",
        "0x1.8p+q",
        "0x",       // matching failure: `f` stays +0.0
        "-0x",      // matching failure: `f` stays +0.0, *not* -0.0
        "0X",
        "0x.",      // succeeds via strtod backtracking to "0"
        "-0x.",     // ... and therefore yields -0.0
        "0x.p1",
        "0xp1",     // 'x' is not consumed, so this reads "0"
        "00x1",
        "0.0x1",
        "0x1.fffffffffffffp+1023",
        "0x1p+1023",
        "0x1.0000000000001p1023",
    ]);
}

/// `inf` / `infinity`, and the partial spellings that fail to match.
#[test]
fn infinity_spellings() {
    assert_all(&[
        "inf",
        "INF",
        "-inf",
        "+inf",
        "Infinity",
        "-infinity",
        "inFiNiTy",
        "infinity1",
        "infx",
        "inf.",
        "inf1",
        "inf 7",
        "i",
        "in",
        "infi",     // a fourth 'i' commits to "infinity": matching failure
        "infin",
        "infini",
        "infinit",
        "1e309",    // overflow to +inf
        "-1e309",
        "0x1p1024",
        "0x2p1023",
        "1e99999999999999999999",
        "0x1p99999999999999999999",
    ]);
}

/// `nan`, with and without a payload. This glibc discards the payload.
#[test]
fn nan_spellings() {
    assert_all(&[
        "nan",
        "NaN",
        "-nan",
        "+nan",
        "nan(1)",
        "nan(0x5)",
        "nan()",
        "nan(abc)",
        "nan(123", // unterminated payload
        "-nan(1)",
        "NAN(0X10)",
        "n",
        "na",
    ]);
}

/// Subnormals keep glibc's un-normalised `%a` spelling (`0x0.…p-1022`).
#[test]
fn subnormals_and_underflow() {
    assert_all(&[
        "5e-324",
        "2.5e-324",
        "4.9406564584124654e-324",
        "1e-320",
        "1e-308",
        "1.1125369292536007e-308",
        "0x1p-1074",
        "0x1p-1075",              // underflows to +0
        "0x0.0000000000001p-1022",
        "0x0.00000000000008p-1022",
        "0x1.00000000000008p-1022", // rounds up into the smallest normal
        "0x0.8p-1022",
        "1e-9999999999999999",
        "-1e-99999",
        "0x1p-99999999999999999",
        "0x0p99999999999999",
    ]);
}

/// `%.4f` has to round the exact value, ties to even.
#[test]
fn fixed_point_rounding() {
    assert_all(&[
        "0.03125",  // exact tie, rounds down to 0.0312
        "0.09375",  // exact tie, rounds up to 0.0938
        "0.15625",
        "-0.09375",
        "1.03125",
        "2.71875",
        "32.03125",
        "-32.03125",
        "1048575.03125",
        "0.00005",
        "0.99995",
        "1.00005",
        "9.99999",
        "-9.99999",
        "1e308",    // the full 309-digit expansion
        "0x1p1023",
        "0x1.fffffffffffffp+1023",
        "99999999999999999999999999999999999999999999999999999999999999999999999",
    ]);
}

// ---------------------------------------------------------------------------
// Phase C -- bytes and sizes the earlier cases do not reach
// ---------------------------------------------------------------------------

/// Raw bytes that are not valid UTF-8, plus embedded NULs.
#[test]
fn non_utf8_and_nul_bytes() {
    for input in [
        &b"\x00"[..],
        b"\x001",
        b"1\x002",
        b"\xff\xfe",
        b" \xc3\xa9",
        b"1.5\xff",
        b"\x1c 1",
        b"0x1p3\xff",
        b"\xef\xbb\xbf1.5", // a UTF-8 BOM is not whitespace
    ] {
        assert_matches(input);
    }
}

/// Digit strings far longer than any fixed-size buffer.
#[test]
fn very_long_inputs() {
    let long_decimal = format!("3.{}", "1234567890".repeat(2000));
    let long_hex = format!("0x{}p-100", "89abcdef".repeat(500));
    let many_zeros = format!("0.{}1", "0".repeat(400));
    let leading_zeros = format!("{}1.5", "0".repeat(1000));
    let long_exponent = format!("1e{}5", "0".repeat(500));
    let long_whitespace = format!("{}2.5", " ".repeat(5000));
    for input in [
        long_decimal,
        long_hex,
        many_zeros,
        leading_zeros,
        long_exponent,
        long_whitespace,
    ] {
        assert_matches(input.as_bytes());
    }
}

/// Every prefix/body combination of the accept-and-reject grammar, with and
/// without trailing bytes, so no `if` in the reader is left untried.
#[test]
fn grammar_cross_product() {
    const PREFIXES: [&str; 7] = ["", "+", "-", " ", "\t\n", "--", "+-"];
    const BODIES: [&str; 34] = [
        "", "0", "0x", "0X", "0x.", "0x.p", "0xp1", ".", ".e1", "e1", "E", "i", "in", "inf",
        "infi", "infin", "infini", "infinit", "infinity", "infinityx", "n", "na", "nan", "nan(",
        "nan()", "nan(x", "1", "1.", ".1", "1e", "1e+", "0x1", "0x1p", "0x1.8p-2",
    ];
    for prefix in PREFIXES {
        for body in BODIES {
            for suffix in ["", "\n", " 7"] {
                assert_matches(format!("{prefix}{body}{suffix}").as_bytes());
            }
        }
    }
}

/// Deterministic sweep over bit patterns: every power of two, the boundary
/// between subnormal and normal, the largest finite value, and a fixed
/// pseudo-random selection, each fed in decimal and in hexadecimal form.
#[test]
fn bit_pattern_sweep() {
    let mut patterns: Vec<u64> = Vec::new();
    for k in 0..64 {
        patterns.push(1u64 << k);
        patterns.push((1u64 << k) - 1);
    }
    patterns.extend([
        0x000f_ffff_ffff_ffff, // largest subnormal
        0x0010_0000_0000_0000, // smallest normal
        0x0010_0000_0000_0001,
        0x7fef_ffff_ffff_ffff, // largest finite
        0x7ff0_0000_0000_0000, // +inf
        0x7ff8_0000_0000_0000, // quiet NaN
        0x3ff0_0000_0000_0000, // 1.0
        0x4340_0000_0000_0000, // 2^53
    ]);
    // xorshift64*, so the case list is identical on every run.
    let mut state: u64 = 0x2545_f491_4f6c_dd1d;
    let mut next = move || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state.wrapping_mul(0x2545_f491_4f6c_dd1d)
    };
    for _ in 0..300 {
        patterns.push(next());
    }

    for bits in patterns {
        for signed in [bits & !(1u64 << 63), bits | (1u64 << 63)] {
            let value = f64::from_bits(signed);
            if !value.is_finite() {
                continue; // covered by the inf/nan tests
            }
            // Both spellings round-trip exactly, so the C reader sees this bit
            // pattern back and every formatting path is exercised on it.
            assert_matches(format!("{value:e}").as_bytes());
            assert_matches(hex_literal(value).as_bytes());
        }
    }
}

/// Writes `value` as a C99 hex float literal (exact, no rounding involved).
fn hex_literal(value: f64) -> String {
    let bits = value.to_bits();
    let sign = if bits >> 63 != 0 { "-" } else { "" };
    let exp_field = ((bits >> 52) & 0x7ff) as i32;
    let mantissa = bits & 0x000f_ffff_ffff_ffff;
    if exp_field == 0 {
        format!("{sign}0x0.{mantissa:013x}p-1022")
    } else {
        format!("{sign}0x1.{mantissa:013x}p{:+}", exp_field - 1023)
    }
}

/// stdout is a pipe whose reader is already gone, so the write must raise
/// `SIGPIPE`. The C program has the default disposition for it; a Rust program
/// has to undo the runtime's `SIG_IGN` to die the same way.
#[cfg(unix)]
#[test]
fn sigpipe_when_the_stdout_reader_is_gone() {
    use std::os::unix::io::FromRawFd;

    extern "C" {
        fn pipe(fds: *mut i32) -> i32;
        fn close(fd: i32) -> i32;
    }

    fn status_with_dead_reader(program: &Path) -> Result<Option<i32>, Option<i32>> {
        let mut fds = [-1i32; 2];
        assert_eq!(unsafe { pipe(fds.as_mut_ptr()) }, 0, "pipe() failed");
        // Drop the read end first: any write on the write end now gets SIGPIPE.
        assert_eq!(unsafe { close(fds[0]) }, 0, "close() failed");
        let stdout = unsafe { Stdio::from_raw_fd(fds[1]) };

        let mut child = Command::new(program)
            .stdin(Stdio::piped())
            .stdout(stdout)
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("cannot spawn {}: {e}", program.display()));
        {
            let mut stdin = child.stdin.take().expect("piped stdin");
            let _ = stdin.write_all(b"1.5\n");
        }
        let out = child.wait_with_output().expect("wait failed");
        assert!(out.stderr.is_empty(), "unexpected stderr: {:?}", out.stderr);
        match signal_of(&out.status) {
            Some(s) => Err(Some(s)),
            None => Ok(out.status.code()),
        }
    }

    let c = status_with_dead_reader(c_binary());
    let rust = status_with_dead_reader(rust_binary());
    assert_eq!(
        c, rust,
        "exit status differs when stdout has no reader: C {c:?} vs Rust {rust:?}"
    );
}
