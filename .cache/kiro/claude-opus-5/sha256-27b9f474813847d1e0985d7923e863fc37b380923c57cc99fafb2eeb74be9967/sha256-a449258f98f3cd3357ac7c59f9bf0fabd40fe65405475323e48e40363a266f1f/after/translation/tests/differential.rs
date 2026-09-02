//! Differential tests: run the original C binary and the Rust binary as
//! subprocesses over the same stdin bytes and require byte-identical stdout,
//! byte-identical stderr, and an identical exit status.
//!
//! The Rust code is never linked as a library here; both programs are driven
//! exactly as a shell would drive them, because that is how they are compared.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// Path to the Rust binary under test, supplied by cargo for integration tests.
const RUST_BIN: &str = env!("CARGO_BIN_EXE_driver");

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Build the C reference program once per test binary, then hand back its path.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = workspace_root().join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");

        if !exe.exists() {
            std::fs::create_dir_all(&build).expect("cannot create c_src/build");

            let configure = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("failed to invoke cmake (is cmake installed?)");
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
                .expect("failed to invoke cmake --build");
            assert!(
                compile.status.success(),
                "cmake build failed:\n{}\n{}",
                String::from_utf8_lossy(&compile.stdout),
                String::from_utf8_lossy(&compile.stderr),
            );
        }

        assert!(
            exe.exists(),
            "C reference binary missing at {}",
            exe.display()
        );
        exe
    })
    .as_path()
}

/// What one program did with one input.
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Ok(code)` for a normal exit, `Err(signal)` when killed by a signal.
    status: Result<i32, i32>,
}

fn run(program: &Path, stdin_bytes: &[u8]) -> Outcome {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", program.display()));

    {
        let mut sink = child.stdin.take().expect("stdin was piped");
        // A short-lived program may exit before consuming everything; a broken
        // pipe here is not a test failure.
        let _ = sink.write_all(stdin_bytes);
        let _ = sink.flush();
    }

    let out = child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("failed to wait on {}: {e}", program.display()));

    let status = match out.status.code() {
        Some(code) => Ok(code),
        None => {
            #[cfg(unix)]
            {
                use std::os::unix::process::ExitStatusExt;
                Err(out.status.signal().unwrap_or(-1))
            }
            #[cfg(not(unix))]
            {
                Err(-1)
            }
        }
    };

    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        status,
    }
}

fn describe(bytes: &[u8]) -> String {
    let shown: String = bytes
        .iter()
        .take(80)
        .map(|&b| match b {
            b'\n' => "\\n".to_string(),
            b'\r' => "\\r".to_string(),
            b'\t' => "\\t".to_string(),
            0x20..=0x7e => (b as char).to_string(),
            other => format!("\\x{other:02x}"),
        })
        .collect();
    if bytes.len() > 80 {
        format!("{shown}... ({} bytes total)", bytes.len())
    } else {
        format!("{shown} ({} bytes)", bytes.len())
    }
}

/// Assert the two programs agree on all three observable channels.
fn assert_same(label: &str, stdin_bytes: &[u8]) {
    let c = run(c_bin(), stdin_bytes);
    let r = run(Path::new(RUST_BIN), stdin_bytes);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch for {label}\n  stdin: {}\n  C:    {}\n  Rust: {}",
        describe(stdin_bytes),
        describe(&c.stdout),
        describe(&r.stdout),
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch for {label}\n  stdin: {}\n  C:    {}\n  Rust: {}",
        describe(stdin_bytes),
        describe(&c.stderr),
        describe(&r.stderr),
    );
    assert_eq!(
        c.status,
        r.status,
        "exit status mismatch for {label}\n  stdin: {}\n  C: {:?}\n  Rust: {:?}",
        describe(stdin_bytes),
        c.status,
        r.status,
    );
}

// ---------------------------------------------------------------------------
// Phase A sanity: both binaries exist and run.
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_run() {
    let c = run(c_bin(), b"1");
    let r = run(Path::new(RUST_BIN), b"1");
    assert_eq!(c.stdout, b"01000000\n", "C reference output changed");
    assert_eq!(r.stdout, b"01000000\n", "Rust output for input `1`");
    assert_eq!(c.status, Ok(0));
    assert_eq!(r.status, Ok(0));
    assert!(c.stderr.is_empty());
    assert!(r.stderr.is_empty());
}

// ---------------------------------------------------------------------------
// Phase B: the input classes the C actually branches on.
//
// `main` has exactly one branch point, inside `scanf("%d", &x)`:
//   * conversion succeeds  -> x takes the converted value
//   * EOF before any input -> scanf returns EOF, x keeps its initialiser 0
//   * matching failure     -> scanf returns 0,   x keeps its initialiser 0
// and `print_hex` loops `sizeof(int)` times, so the loop bound is fixed.
// The remaining variation is in glibc's %d conversion: whitespace skipping
// across newlines, the optional sign, the digit run, strtol's saturation at
// LONG_MIN/LONG_MAX, and the truncating store through an `int *`.
// ---------------------------------------------------------------------------

#[test]
fn empty_input_leaves_x_at_zero() {
    // scanf returns EOF without touching x.
    assert_same("empty stdin", b"");
}

#[test]
fn single_item_happy_path() {
    for s in ["0", "1", "7", "42", "1000000", "123456789"] {
        assert_same(s, s.as_bytes());
    }
}

#[test]
fn negative_values() {
    for s in ["-0", "-1", "-7", "-42", "-123456789"] {
        assert_same(s, s.as_bytes());
    }
}

#[test]
fn explicit_plus_sign() {
    for s in ["+0", "+1", "+5", "+2147483647"] {
        assert_same(s, s.as_bytes());
    }
}

#[test]
fn int_boundaries() {
    // The maximum and minimum an `int` holds, and one step past each. Past the
    // boundary glibc still converts with strtol (64-bit long) and then
    // truncates on the store, so these wrap rather than fail.
    for s in [
        "2147483647",
        "-2147483648",
        "2147483648",
        "-2147483649",
        "2147483649",
        "-2147483650",
    ] {
        assert_same(s, s.as_bytes());
    }
}

#[test]
fn truncation_of_values_above_32_bits() {
    for s in [
        "4294967295",
        "4294967296",
        "4294967297",
        "-4294967296",
        "8589934592",
        "2147483648999",
    ] {
        assert_same(s, s.as_bytes());
    }
}

#[test]
fn strtol_saturation_at_long_bounds() {
    // At and past LONG_MIN/LONG_MAX strtol clamps; the clamped value is then
    // truncated to int, so LONG_MAX -> -1 and LONG_MIN -> 0.
    for s in [
        "9223372036854775807",
        "9223372036854775808",
        "9223372036854775809",
        "-9223372036854775808",
        "-9223372036854775809",
        "-9223372036854775810",
        "99999999999999999999",
        "-99999999999999999999",
        "999999999999999999999999999999",
        "-999999999999999999999999999999",
    ] {
        assert_same(s, s.as_bytes());
    }
}

#[test]
fn matching_failure_leaves_x_at_zero() {
    // scanf returns 0 without storing: no digit is available at the conversion
    // point. x keeps its initialiser, so the output is still eight zeros.
    for s in [
        "abc", "x", "-", "+", "--5", "++5", "+-5", "-+5", ".5", "-.5", "e5", "/", ":", "!", "-abc",
        "- 5", "+ 5", "-\n5",
    ] {
        assert_same(s, s.as_bytes());
    }
}

#[test]
fn whitespace_is_skipped_across_newlines() {
    // %d skips leading whitespace, and that skip crosses newlines -- unlike
    // fgets, which would stop at the first one.
    for s in [
        " 42",
        "   42",
        "\n42",
        "\n\n\n42",
        "\t42",
        "\r\n42",
        "\r\n\r\n-8",
        " \t\n\r\x0b\x0c 77",
        "\n \t -13",
        "   ",
        "\n",
        "\n\n\n",
        "\t \t",
        "\x0b\x0c",
        " \n \n ",
    ] {
        assert_same(&format!("ws {s:?}"), s.as_bytes());
    }
}

#[test]
fn trailing_bytes_after_the_digit_run_are_ignored() {
    // The conversion stops at the first non-digit; nothing else is read.
    for s in [
        "5abc",
        "5 abc",
        "12 34",
        "12\n34",
        "007",
        "0x10",
        "5.5",
        "-5.5",
        "1,000",
        "9-9",
        "42\n",
        "42\n\n",
        "42 ",
    ] {
        assert_same(&format!("trailing {s:?}"), s.as_bytes());
    }
}

#[test]
fn no_trailing_newline_on_input() {
    // The C never requires a terminating newline; verify both agree.
    assert_same("no newline", b"99");
    assert_same("with newline", b"99\n");
}

// ---------------------------------------------------------------------------
// Phase C: paths not covered above -- non-UTF-8 bytes, embedded NULs, and
// inputs far larger than any internal buffer.
// ---------------------------------------------------------------------------

#[test]
fn embedded_nul_bytes() {
    assert_same("NUL then digits", b"\x00123");
    assert_same("digits then NUL", b"12\x003");
    assert_same("NUL only", b"\x00");
    assert_same("NUL after sign", b"-\x005");
    assert_same("space NUL digits", b" \x009");
}

#[test]
fn non_ascii_and_high_bytes() {
    // 0x80..=0xff are not whitespace and not digits in the C locale, so each of
    // these is a matching failure.
    assert_same("0xff 0xfe", b"\xff\xfe abc");
    assert_same("0x80 then digits", b"\x80 5");
    assert_same("0xa0 (nbsp byte)", b"\xa0 5");
    assert_same("utf8 minus sign", "\u{2212}5".as_bytes());
    assert_same("utf8 digits", "１２３".as_bytes());
    assert_same("digits then high byte", b"5\xff");
}

#[test]
fn fixed_binary_garbage() {
    // A fixed byte pattern (not random, so failures reproduce).
    let mut buf = Vec::new();
    for i in 0u16..256 {
        buf.push(i as u8);
    }
    assert_same("all 256 byte values", &buf);
}

#[test]
fn very_long_inputs() {
    let zeros = format!("{}5", "0".repeat(10_000));
    assert_same("10k leading zeros then 5", zeros.as_bytes());

    let nines = "9".repeat(10_000);
    assert_same("10k nines", nines.as_bytes());

    let neg_nines = format!("-{}", "9".repeat(10_000));
    assert_same("negative 10k nines", neg_nines.as_bytes());

    let spaces = format!("{}42", " ".repeat(100_000));
    assert_same("100k spaces then 42", spaces.as_bytes());

    let newlines = format!("{}-7", "\n".repeat(50_000));
    assert_same("50k newlines then -7", newlines.as_bytes());

    let long_junk = format!("5{}", "z".repeat(100_000));
    assert_same("5 then 100k junk bytes", long_junk.as_bytes());

    let all_junk = "z".repeat(100_000);
    assert_same("100k junk bytes", all_junk.as_bytes());
}

#[test]
fn digit_run_lengths_around_the_conversion_limits() {
    // One extra digit at a time across the 32-bit and 64-bit boundaries.
    for len in 1..=25 {
        let s = "1".repeat(len);
        assert_same(&format!("{len} ones"), s.as_bytes());
        let neg = format!("-{s}");
        assert_same(&format!("negative {len} ones"), neg.as_bytes());
    }
}

#[test]
fn every_leading_digit_at_each_boundary() {
    for d in b'0'..=b'9' {
        let s = format!("{}{}", d as char, "0".repeat(18));
        assert_same(&format!("{s} (19 digits)"), s.as_bytes());
        let neg = format!("-{s}");
        assert_same(&format!("{neg} (19 digits)"), neg.as_bytes());
    }
}

/// The alphabet that drives every decision inside glibc's `%d`: digits, the
/// two signs, all six C-locale whitespace characters, non-digit letters, a
/// decimal point, NUL and a high byte.
const SCANF_ALPHABET: &[u8] = b"0123456789+- \t\n\r\x0b\x0cxa.\x00\xff";

#[test]
fn exhaustive_single_bytes() {
    // Every possible one-byte stdin. Each is either whitespace (skipped, then
    // EOF), a digit (converted), a sign (then EOF -> matching failure) or a
    // matching failure outright.
    for i in 0u16..256 {
        let b = [i as u8];
        assert_same(&format!("single byte {i:#04x}"), &b);
    }
}

#[test]
fn exhaustive_byte_pairs_over_scanf_alphabet() {
    for &a in SCANF_ALPHABET {
        for &b in SCANF_ALPHABET {
            let pair = [a, b];
            assert_same(&format!("pair {a:#04x}{b:#04x}"), &pair);
        }
    }
}

#[test]
fn exhaustive_byte_triples_over_reduced_alphabet() {
    const REDUCED: &[u8] = b"0189+- \n\x00a";
    for &a in REDUCED {
        for &b in REDUCED {
            for &c in REDUCED {
                let t = [a, b, c];
                assert_same(&format!("triple {a:#04x}{b:#04x}{c:#04x}"), &t);
            }
        }
    }
}

#[test]
fn exhaustive_small_integers() {
    for v in -300i32..=300 {
        for form in [
            format!("{v}"),
            format!("{v:+}"),
            format!(" {v}"),
            format!("{v}\n"),
            format!("0{v}"),
            format!("{v} abc"),
        ] {
            assert_same(&format!("small int {form:?}"), form.as_bytes());
        }
    }
}

#[test]
fn powers_of_two_and_neighbours_past_64_bits() {
    for k in 0u32..72 {
        let base: i128 = 1i128 << k;
        for delta in [-2i128, -1, 0, 1, 2] {
            let v = base + delta;
            for s in [format!("{v}"), format!("-{v}"), format!(" {v}\n")] {
                assert_same(&format!("pow2 {s:?}"), s.as_bytes());
            }
        }
    }
}

#[test]
fn buffer_boundary_sizes() {
    // Sizes straddling the block sizes glibc's stdio and our reader use, so a
    // refill bug at a boundary shows up.
    for size in [
        1usize, 2, 127, 128, 129, 255, 256, 257, 1023, 1024, 1025, 4095, 4096, 4097, 8191, 8192,
        8193, 65535, 65536, 65537,
    ] {
        for (filler, tail) in [
            (b' ', &b"42"[..]),
            (b'\n', &b"-42"[..]),
            (b'0', &b"7"[..]),
            (b'9', &b""[..]),
            (b'z', &b""[..]),
        ] {
            let mut data = vec![filler; size];
            data.extend_from_slice(tail);
            assert_same(
                &format!("boundary {size} x {:?} + {:?}", filler as char, tail),
                &data,
            );
        }
    }
}

#[test]
fn output_is_exactly_eight_hex_digits_and_one_newline() {
    // print_hex loops sizeof(int) == 4 times with "%02x", then one "\n".
    // Assert the shape directly as well as against C, so a change in either
    // program's formatting is caught even if both changed together.
    for s in ["0", "-1", "2147483647", "-2147483648", "255", "65536"] {
        let c = run(c_bin(), s.as_bytes());
        assert_eq!(c.stdout.len(), 9, "C output length for {s:?}");
        assert_eq!(c.stdout[8], b'\n', "C trailing newline for {s:?}");
        assert!(
            c.stdout[..8]
                .iter()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b)),
            "C output is lowercase hex for {s:?}"
        );
        assert_same(&format!("shape {s}"), s.as_bytes());
    }
}
