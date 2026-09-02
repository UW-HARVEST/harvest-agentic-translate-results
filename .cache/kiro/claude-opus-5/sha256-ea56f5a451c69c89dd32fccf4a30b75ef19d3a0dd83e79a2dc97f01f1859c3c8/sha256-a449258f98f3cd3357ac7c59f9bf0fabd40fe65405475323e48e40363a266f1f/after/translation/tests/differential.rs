//! Differential tests: run the C `driver` and the Rust `driver` as
//! subprocesses on identical stdin bytes and require byte-identical stdout,
//! byte-identical stderr, and an identical exit status.
//!
//! Nothing here links against the Rust crate as a library; both programs are
//! driven exactly the way a shell drives them, which is how the translation is
//! graded.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// Repository root: the directory containing both `c_src/` and `translation/`.
fn repo_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// The Rust binary under test, as built by cargo for this test run.
fn rust_bin() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// The C binary, built with cmake on first use.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = repo_root().join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");
        if exe.is_file() {
            return exe;
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

        assert!(exe.is_file(), "expected C binary at {}", exe.display());
        exe
    })
}

/// What one program produced for one input.
struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Ok(code)` for a normal exit, `Err(signal)` when killed by a signal.
    status: Result<i32, i32>,
}

fn run(program: &Path, stdin_bytes: &[u8]) -> Run {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", program.display()));

    let mut stdin = child.stdin.take().expect("piped stdin");
    let bytes = stdin_bytes.to_vec();
    // Write on a helper thread: the child may exit without draining stdin
    // (matching failure, or a huge input), and an unbuffered write from this
    // thread could otherwise block or fail on EPIPE.
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&bytes);
        let _ = stdin.flush();
        drop(stdin);
    });

    let out = child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("wait for {}: {e}", program.display()));
    writer.join().expect("stdin writer thread");

    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        status: exit_repr(&out.status),
    }
}

#[cfg(unix)]
fn exit_repr(status: &std::process::ExitStatus) -> Result<i32, i32> {
    use std::os::unix::process::ExitStatusExt;
    match status.code() {
        Some(code) => Ok(code),
        None => Err(status.signal().expect("terminated without code or signal")),
    }
}

#[cfg(not(unix))]
fn exit_repr(status: &std::process::ExitStatus) -> Result<i32, i32> {
    Ok(status.code().unwrap_or(-1))
}

fn show(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\t' => s.push_str("\\t"),
            b'\r' => s.push_str("\\r"),
            0x0b => s.push_str("\\v"),
            0x0c => s.push_str("\\f"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    s
}

fn status_str(s: &Result<i32, i32>) -> String {
    match s {
        Ok(code) => format!("exit {code}"),
        Err(sig) => format!("signal {sig}"),
    }
}

/// Assert the two programs agree on stdout, stderr and exit status.
#[track_caller]
fn assert_same(label: &str, stdin_bytes: &[u8]) {
    let c = run(c_bin(), stdin_bytes);
    let r = run(rust_bin(), stdin_bytes);

    let input = if stdin_bytes.len() > 80 {
        format!(
            "{}... ({} bytes)",
            show(&stdin_bytes[..80]),
            stdin_bytes.len()
        )
    } else {
        show(stdin_bytes)
    };

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout differs for {label} (stdin = \"{input}\")\n  C:    {}\n  Rust: {}",
        show(&c.stdout),
        show(&r.stdout),
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr differs for {label} (stdin = \"{input}\")\n  C:    {}\n  Rust: {}",
        show(&c.stderr),
        show(&r.stderr),
    );
    assert_eq!(
        c.status,
        r.status,
        "exit status differs for {label} (stdin = \"{input}\")\n  C:    {}\n  Rust: {}",
        status_str(&c.status),
        status_str(&r.status),
    );
}

#[track_caller]
fn assert_same_str(input: &str) {
    assert_same(&format!("input {:?}", input), input.as_bytes());
}

// ---------------------------------------------------------------------------
// The C program is:
//
//     int x = 0;  scanf("%d", &x);  driver(x);
//
// `driver` memcpy's the object representation of the int and prints its
// `sizeof(int)` bytes as `%02x` followed by a newline. So the only branching
// the C actually performs is inside `scanf("%d", ...)`:
//
//   * input failure (EOF before any non-whitespace) -> x stays 0
//   * matching failure (no digit after optional sign) -> x stays 0
//   * leading whitespace is skipped ( \t \n \v \f \r )
//   * an optional single '+' or '-'
//   * one or more digits, accumulated into a `long`, clamped to LONG_MAX /
//     LONG_MIN on range error, then truncated into the `int` destination
//   * conversion stops at the first non-digit
//
// plus the process-level branch of a write failure on stdout.
// ---------------------------------------------------------------------------

#[test]
fn empty_input_leaves_x_at_zero() {
    assert_same("empty stdin", b"");
}

#[test]
fn single_item() {
    assert_same_str("42");
}

#[test]
fn zero_and_signed_zero() {
    for s in ["0", "-0", "+0", "000", "-000"] {
        assert_same_str(s);
    }
}

#[test]
fn small_values_and_byte_boundaries() {
    for s in [
        "1", "7", "9", "10", "127", "128", "255", "256", "65535", "65536", "16777215", "16777216",
        "1000000", "1234567890",
    ] {
        assert_same_str(s);
    }
}

#[test]
fn negative_values() {
    for s in [
        "-1",
        "-7",
        "-128",
        "-255",
        "-256",
        "-65536",
        "-1000000",
        "-1234567890",
    ] {
        assert_same_str(s);
    }
}

#[test]
fn explicit_plus_sign() {
    for s in ["+7", "+0", "+2147483647", "+9223372036854775807"] {
        assert_same_str(s);
    }
}

#[test]
fn int_range_extremes() {
    for s in ["2147483647", "-2147483648", "-2147483647", "2147483646"] {
        assert_same_str(s);
    }
}

/// Beyond INT_MAX/INT_MIN but inside `long`: glibc converts into a `long` and
/// stores the low bytes, so the value truncates rather than clamping to int.
#[test]
fn beyond_int_range_truncates() {
    for s in [
        "2147483648",
        "-2147483649",
        "4294967295",
        "4294967296",
        "4294967297",
        "1234567890123",
        "-1234567890123",
        "68719476736",
    ] {
        assert_same_str(s);
    }
}

/// At and beyond LONG_MAX/LONG_MIN the conversion clamps, then truncates.
#[test]
fn long_range_clamping() {
    for s in [
        "9223372036854775806",
        "9223372036854775807",
        "9223372036854775808",
        "9223372036854775809",
        "-9223372036854775807",
        "-9223372036854775808",
        "-9223372036854775809",
        "18446744073709551615",
        "18446744073709551616",
        "18446744073709551617",
        "99999999999999999999999999999999",
        "-99999999999999999999999999999999",
    ] {
        assert_same_str(s);
    }
}

/// Leading zeros must not be mistaken for overflow.
#[test]
fn leading_zeros() {
    for s in [
        "007",
        "-0000000000000000001",
        "00000000009223372036854775808",
        "-000000000000000000000009223372036854775809",
    ] {
        assert_same_str(s);
    }
}

/// `%d` skips every character `isspace` accepts before the conversion.
#[test]
fn leading_whitespace_is_skipped() {
    for s in [
        " 42",
        "   \n\t 42",
        "\n\n\n\n7",
        "\t-5",
        "\u{0b}\u{0c}\r -42",
        "\r\n\r\n123",
    ] {
        assert_same_str(s);
    }
}

/// Whitespace only: EOF is reached before any conversion, so x stays 0.
#[test]
fn whitespace_only_is_input_failure() {
    for s in [" ", "   ", "\n", "\n\n", "\t", "\u{0b}", "\u{0c}", "\r", " \t\n\u{0b}\u{0c}\r"] {
        assert_same_str(s);
    }
}

/// Matching failure: the first non-whitespace byte cannot start a number.
#[test]
fn matching_failure_leaves_x_at_zero() {
    for s in [
        "abc", ".", ",", "x", "e42", "-", "+", "--5", "++5", "-+5", "+-5", "  -  5", "  +  ", "-a",
        "+a", "/", ":", "@", "#", "\u{7f}",
    ] {
        assert_same_str(s);
    }
}

/// Conversion stops at the first byte that is not a digit.
#[test]
fn conversion_stops_at_first_non_digit() {
    for s in [
        "12abc",
        "0x1f",
        "1e5",
        "42 99",
        "1\n2",
        "-3.5",
        "  9223372036854775807abc",
        "5,6",
        "7-8",
        "9+1",
    ] {
        assert_same_str(s);
    }
}

/// `scanf` reads across newlines, unlike `fgets`; only the first field is used.
#[test]
fn only_the_first_field_is_consumed() {
    for s in [
        "\n\n  \t\n  -17\n999\n888\n",
        "42\n\n\n",
        "1 2 3 4 5",
        "  \n 0 \n 1 \n",
    ] {
        assert_same_str(s);
    }
}

/// Non-text bytes: NUL is neither whitespace nor a digit, and invalid UTF-8
/// must not disturb either program.
#[test]
fn non_utf8_and_nul_bytes() {
    assert_same("NUL first", b"\x00 42");
    assert_same("trailing NUL", b"42\x00");
    assert_same("lone invalid utf8", b"\xff\xfe");
    assert_same("digits then invalid utf8", b"42\n\xff\xfe");
    assert_same("invalid utf8 after sign", b"-\xff");
    assert_same("utf8 space then digits", "\u{2009}42".as_bytes());
    assert_same("all high bytes", &[0x80u8; 64]);
}

/// The maximum the code handles at the input end: an arbitrarily long run of
/// digits. Both the clamping path and the harmless leading-zero path.
#[test]
fn very_long_digit_runs() {
    let nines = "9".repeat(10_000);
    assert_same("10k nines", nines.as_bytes());

    let neg_nines = format!("-{}", "9".repeat(10_000));
    assert_same("negative 10k nines", neg_nines.as_bytes());

    let zeros_then_five = format!("{}5", "0".repeat(10_000));
    assert_same("10k zeros then 5", zeros_then_five.as_bytes());

    let zeros_only = "0".repeat(10_000);
    assert_same("10k zeros", zeros_only.as_bytes());

    let ws_then_num = format!("{}12345", " ".repeat(10_000));
    assert_same("10k spaces then digits", ws_then_num.as_bytes());

    let junk = "z".repeat(10_000);
    assert_same("10k non-digits", junk.as_bytes());
}

/// Every representable single byte as the sole input, so no first-byte class is
/// left untested.
#[test]
fn exhaustive_single_byte_inputs() {
    for b in 0u8..=255 {
        assert_same(&format!("single byte {b:#04x}"), &[b]);
    }
}

/// Every two-byte input drawn from the classes `%d` distinguishes.
#[test]
fn two_byte_combinations() {
    let alphabet: &[u8] = b" \t\n\x0b\x0c\r+-0159abxz.\x00\xff";
    for &a in alphabet {
        for &b in alphabet {
            assert_same(&format!("bytes {a:#04x} {b:#04x}"), &[a, b]);
        }
    }
}

/// Every bit position of the printed object representation, so the `%02x`
/// formatting of `print_hex` is checked for all four bytes and both nibbles.
#[test]
fn all_bit_positions_of_the_printed_int() {
    for bit in 0..31 {
        let v: i32 = 1i32 << bit;
        assert_same_str(&v.to_string());
        assert_same_str(&(-v).to_string());
        assert_same_str(&(v - 1).to_string());
    }
    for v in [i32::MIN, i32::MAX, -1, 0, 0x0f0f_0f0f, 0x7071_7273] {
        assert_same_str(&v.to_string());
    }
}

/// stdin closed outright: the read fails, which `scanf` reports as EOF.
#[cfg(unix)]
#[test]
fn closed_stdin() {
    fn run_with_null_stdin(program: &Path) -> Run {
        let out = Command::new(program)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .unwrap_or_else(|e| panic!("spawn {}: {e}", program.display()));
        Run {
            stdout: out.stdout,
            stderr: out.stderr,
            status: exit_repr(&out.status),
        }
    }

    let c = run_with_null_stdin(c_bin());
    let r = run_with_null_stdin(rust_bin());
    assert_eq!(c.stdout, r.stdout, "stdout differs with /dev/null stdin");
    assert_eq!(c.stderr, r.stderr, "stderr differs with /dev/null stdin");
    assert_eq!(c.status, r.status, "exit status differs with /dev/null stdin");
}

/// A write to a broken stdout pipe: the C program keeps the default `SIGPIPE`
/// disposition and dies with signal 13, so the Rust program must too.
#[cfg(unix)]
#[test]
fn broken_stdout_pipe_matches() {
    fn status_on_broken_pipe(program: &Path) -> Result<i32, i32> {
        // `sh` reads the number only after its stdout reader has already gone
        // away, which guarantees the write hits a closed pipe.
        let script = format!(
            "{{ sleep 0.5; echo 42; }} | '{}' 2>/dev/null | true; exit ${{PIPESTATUS[1]}}",
            program.display()
        );
        let out = Command::new("bash")
            .arg("-c")
            .arg(&script)
            .output()
            .expect("run bash");
        Ok(out.status.code().unwrap_or(-1))
    }

    let c = status_on_broken_pipe(c_bin());
    let r = status_on_broken_pipe(rust_bin());
    assert_eq!(
        c, r,
        "broken-pipe exit status differs: C {c:?} vs Rust {r:?} \
         (141 means terminated by SIGPIPE)"
    );
}
