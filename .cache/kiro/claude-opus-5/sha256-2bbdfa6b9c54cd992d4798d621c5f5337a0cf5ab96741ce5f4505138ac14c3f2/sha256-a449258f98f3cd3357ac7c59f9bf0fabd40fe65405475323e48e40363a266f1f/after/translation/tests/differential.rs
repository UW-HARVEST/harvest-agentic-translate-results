// Differential integration tests: run the C binary and the Rust binary as
// subprocesses on the same stdin and require byte-identical stdout, byte-identical
// stderr, and the same exit status.
//
// The Rust program is never linked as a library here; it is driven exactly the way
// a shell drives the C program, because that is what the two are compared on.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Path to the Rust binary under test, provided by Cargo.
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Path to the C binary, building it with CMake on first use if necessary.
fn c_bin() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_src = manifest
        .parent()
        .expect("translation/ must have a parent directory")
        .join("c_src");
    let build = c_src.join("build");
    let bin = build.join("driver");
    if !bin.exists() {
        build_c(&c_src, &build);
    }
    assert!(
        bin.exists(),
        "C binary missing at {}; build it with: cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .",
        bin.display()
    );
    bin
}

fn build_c(c_src: &Path, build: &Path) {
    std::fs::create_dir_all(build).expect("could not create c_src/build");
    let configure = Command::new("cmake")
        .arg("..")
        .current_dir(build)
        .output()
        .expect("failed to invoke cmake; is it installed?");
    assert!(
        configure.status.success(),
        "cmake configure failed:\n{}\n{}",
        String::from_utf8_lossy(&configure.stdout),
        String::from_utf8_lossy(&configure.stderr)
    );
    let compile = Command::new("cmake")
        .args(["--build", "."])
        .current_dir(build)
        .output()
        .expect("failed to invoke cmake --build");
    assert!(
        compile.status.success(),
        "cmake --build failed:\n{}\n{}",
        String::from_utf8_lossy(&compile.stdout),
        String::from_utf8_lossy(&compile.stderr)
    );
    let _ = c_src; // only used for the error message above
}

struct Output {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    status: Option<i32>,
}

fn run(bin: &Path, input: &[u8]) -> Output {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));
    child
        .stdin
        .take()
        .expect("stdin was piped")
        .write_all(input)
        .or_else(|e| {
            // A program that exits before draining stdin gives us EPIPE; that is
            // not a test failure, the child's output is still what matters.
            if e.kind() == std::io::ErrorKind::BrokenPipe {
                Ok(())
            } else {
                Err(e)
            }
        })
        .expect("failed writing to child stdin");
    let out = child.wait_with_output().expect("failed waiting for child");
    Output {
        stdout: out.stdout,
        stderr: out.stderr,
        status: out.status.code(),
    }
}

/// Runs both binaries on `input` and asserts stdout, stderr and exit status all match.
fn assert_same(label: &str, input: &[u8]) {
    let c = run(&c_bin(), input);
    let r = run(&rust_bin(), input);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch for {label} (input {:?})\n  C: {:?}\n  Rust: {:?}",
        Escaped(input),
        Escaped(&c.stdout),
        Escaped(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch for {label} (input {:?})\n  C: {:?}\n  Rust: {:?}",
        Escaped(input),
        Escaped(&c.stderr),
        Escaped(&r.stderr)
    );
    assert_eq!(
        c.status, r.status,
        "exit status mismatch for {label} (input {:?}): C {:?} vs Rust {:?}",
        Escaped(input),
        c.status,
        r.status
    );
}

/// Byte-accurate rendering for assertion messages.
struct Escaped<'a>(&'a [u8]);

impl std::fmt::Debug for Escaped<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("b\"")?;
        for &byte in self.0 {
            match byte {
                b'\n' => f.write_str("\\n")?,
                b'\r' => f.write_str("\\r")?,
                b'\t' => f.write_str("\\t")?,
                b'"' => f.write_str("\\\"")?,
                b'\\' => f.write_str("\\\\")?,
                0x20..=0x7e => f.write_str(std::str::from_utf8(&[byte]).unwrap())?,
                _ => write!(f, "\\x{byte:02x}")?,
            }
        }
        f.write_str("\"")
    }
}

fn check_all(cases: &[(&str, &[u8])]) {
    for (label, input) in cases {
        assert_same(label, input);
    }
}

// ---------------------------------------------------------------------------
// Phase A sanity: both binaries exist, are runnable, and the C output is the
// shape we think it is.
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_run() {
    let c = run(&c_bin(), b"1 2 1 42");
    let r = run(&rust_bin(), b"1 2 1 42");
    assert_eq!(c.status, Some(0), "C program should exit 0");
    assert_eq!(r.status, Some(0), "Rust program should exit 0");
    // Pins the observed C behaviour so a regression in either direction is loud:
    // x = 1 & 0x3, y = 2 & 0x7, b = !!1, z = 42.
    assert_eq!(c.stdout, b"1 2 1 42\n");
    assert_eq!(r.stdout, c.stdout);
    assert!(c.stderr.is_empty() && r.stderr.is_empty());
}

// ---------------------------------------------------------------------------
// Phase B: the input classes the C program branches on.
//
// main() performs four unchecked scanf() conversions and then calls
// driver(x, y, !!b, z). Every conversion has three outcomes -- success, matching
// failure, input failure -- and on either failure the destination keeps its
// initial 0 and every later conversion fails too (the offending byte is never
// consumed by %u/%d). Those are the branches.
// ---------------------------------------------------------------------------

#[test]
fn happy_path_all_four_values() {
    check_all(&[
        ("all four", b"1 2 1 42"),
        ("all zero", b"0 0 0 0"),
        ("trailing newline", b"1 2 1 42\n"),
        ("trailing junk after last value", b"1 2 1 42 ignored rest"),
        ("newline separated", b"1\n2\n1\n42\n"),
        ("tab separated", b"1\t2\t1\t42"),
        ("crlf separated", b"1\r\n2\r\n1\r\n42\r\n"),
        ("vertical tab and form feed", b"\x0b1\x0c2\x0b1\x0c9"),
        ("extra leading whitespace", b"   1    2    1    9"),
        ("blank lines between values", b"\n\n\n1\n\n2\n\n1\n\n7\n"),
    ]);
}

#[test]
fn empty_and_whitespace_only_input() {
    // Input failure on the very first scanf: all four variables stay 0.
    check_all(&[
        ("empty stdin", b""),
        ("single newline", b"\n"),
        ("spaces only", b"   "),
        ("tab only", b"\t"),
        ("mixed whitespace only", b" \t\r\n\x0b\x0c "),
    ]);
}

#[test]
fn fewer_values_than_conversions() {
    // Each prefix leaves the remaining variables at their initial 0.
    check_all(&[
        ("one value", b"1"),
        ("one value with newline", b"1\n"),
        ("two values", b"1 2"),
        ("three values", b"1 2 3"),
        ("three values then whitespace", b"1 2 3   \n"),
    ]);
}

#[test]
fn bitfield_truncation_x_two_bits() {
    // x is `unsigned int x : 2`, so the store keeps the low 2 bits.
    check_all(&[
        ("x = 0", b"0 0 0 0"),
        ("x = 3 (max in field)", b"3 0 0 0"),
        ("x = 4 (wraps to 0)", b"4 0 0 0"),
        ("x = 5 (wraps to 1)", b"5 0 0 0"),
        ("x = 6", b"6 0 0 0"),
        ("x = 7 (wraps to 3)", b"7 0 0 0"),
        ("x = 8 (wraps to 0)", b"8 0 0 0"),
        ("x = 255", b"255 0 0 0"),
    ]);
}

#[test]
fn bitfield_truncation_y_three_bits() {
    // y is `unsigned int y : 3`, so the store keeps the low 3 bits.
    check_all(&[
        ("y = 7 (max in field)", b"0 7 0 0"),
        ("y = 8 (wraps to 0)", b"0 8 0 0"),
        ("y = 9 (wraps to 1)", b"0 9 0 0"),
        ("y = 15 (wraps to 7)", b"0 15 0 0"),
        ("y = 16 (wraps to 0)", b"0 16 0 0"),
        ("y = 255", b"0 255 0 0"),
    ]);
}

#[test]
fn bool_bitfield_normalisation() {
    // main() passes `!!b`, and the field is `bool b : 1`, so any nonzero b prints 1.
    check_all(&[
        ("b = 0", b"1 2 0 9"),
        ("b = 1", b"1 2 1 9"),
        ("b = 2", b"1 2 2 9"),
        ("b = -1", b"1 2 -1 9"),
        ("b = -5", b"1 2 -5 9"),
        ("b = 256", b"1 2 256 9"),
        // 4294967296 truncates to int 0, so !!b is false even though the text is nonzero.
        ("b = 4294967296 truncates to 0", b"1 2 4294967296 9"),
        // 4294967297 truncates to int 1.
        ("b = 4294967297 truncates to 1", b"1 2 4294967297 9"),
        ("b = INT_MIN", b"1 2 -2147483648 9"),
        ("b = INT_MAX", b"1 2 2147483647 9"),
        ("b = -0", b"1 2 -0 9"),
    ]);
}

#[test]
fn signed_z_is_printed_with_percent_d() {
    check_all(&[
        ("z positive", b"1 2 1 42"),
        ("z negative", b"1 2 1 -42"),
        ("z = -1", b"1 2 1 -1"),
        ("z = INT_MAX", b"1 2 1 2147483647"),
        ("z = INT_MIN", b"1 2 1 -2147483648"),
        ("z = INT_MAX + 1 wraps", b"1 2 1 2147483648"),
        ("z = INT_MIN - 1 wraps", b"1 2 1 -2147483649"),
        ("z = 2^32 truncates to 0", b"1 2 1 4294967296"),
        ("z = 2^32 + 1 truncates to 1", b"1 2 1 4294967297"),
        ("z = UINT_MAX truncates to -1", b"1 2 1 4294967295"),
    ]);
}

#[test]
fn matching_failure_stops_every_later_conversion() {
    // %u / %d leave the offending byte in the stream, so once one conversion
    // fails on a non-numeric byte, all the following ones fail on the same byte
    // and their destinations stay 0.
    check_all(&[
        ("non-numeric first token", b"abc"),
        ("non-numeric first token with rest", b"abc 1 2 3"),
        ("fails on second", b"1 abc 3 4"),
        ("fails on third", b"1 2 abc 4"),
        ("fails on fourth", b"1 2 3 abc"),
        ("single letter", b"x"),
        ("whitespace then letter", b" x"),
        ("punctuation", b"."),
        ("comma separated", b"1,2,1,9"),
        ("underscore in number", b"1_2 3 4 5"),
        ("digits then letter", b"5x 1 2 3"),
    ]);
}

#[test]
fn sign_handling_and_sign_only_matching_failures() {
    check_all(&[
        ("explicit plus signs", b"+3 +7 +1 +5"),
        ("minus only", b"-"),
        ("plus only", b"+"),
        ("minus then space", b"- 1 2 3"),
        ("plus then space", b"+ 1 2 3"),
        ("double minus", b"--1 2 3 4"),
        ("plus minus", b"+-1 2 3 4"),
        ("minus then newline", b"-\n1 2 3"),
    ]);
}

#[test]
fn percent_u_accepts_a_negative_sign_and_wraps() {
    // %u goes through strtoul, which accepts '-' and negates modulo 2^64; the
    // result is then stored into `unsigned int` and finally into the bit-field.
    check_all(&[
        ("all negative", b"-1 -1 -1 -1"),
        ("x = -1", b"-1 0 0 0"),
        ("x = -2", b"-2 0 0 0"),
        ("x = -3", b"-3 0 0 0"),
        ("x = -4", b"-4 0 0 0"),
        ("y = -1", b"0 -1 0 0"),
        ("y = -9", b"0 -9 0 0"),
        ("negative 2^32", b"-4294967295 -4294967296 0 0"),
        ("negative zero", b"-0 -0 -0 -0"),
    ]);
}

#[test]
fn value_truncation_to_thirty_two_bits() {
    check_all(&[
        ("UINT_MAX", b"4294967295 4294967295 2147483647 2147483647"),
        ("2^32", b"4294967296 4294967296 2147483648 2147483648"),
        ("2^32 + 3", b"4294967299 4294967299 0 0"),
        ("2^33", b"8589934592 8589934592 0 0"),
    ]);
}

#[test]
fn strtol_and_strtoul_saturation_on_overflow() {
    // Overflow saturates at LONG_MAX / LONG_MIN / ULONG_MAX inside glibc before
    // the narrowing store, which is observable in the printed value.
    check_all(&[
        (
            "LONG_MAX",
            b"9223372036854775807 9223372036854775807 9223372036854775807 9223372036854775807",
        ),
        (
            "LONG_MAX + 1",
            b"9223372036854775808 9223372036854775808 9223372036854775808 9223372036854775808",
        ),
        (
            "LONG_MIN",
            b"-9223372036854775808 -9223372036854775808 -9223372036854775808 -9223372036854775808",
        ),
        (
            "LONG_MIN - 1",
            b"-9223372036854775809 -9223372036854775809 -9223372036854775809 -9223372036854775809",
        ),
        ("ULONG_MAX", b"18446744073709551615 18446744073709551615 0 0"),
        ("ULONG_MAX + 1", b"18446744073709551616 18446744073709551616 0 0"),
        (
            "twenty nines",
            b"99999999999999999999 99999999999999999999 99999999999999999999 99999999999999999999",
        ),
        (
            "negative twenty nines",
            b"-99999999999999999999 -99999999999999999999 -99999999999999999999 -99999999999999999999",
        ),
        (
            "forty digits",
            b"1234567890123456789012345678901234567890 1234567890123456789012345678901234567890 1234567890123456789012345678901234567890 1234567890123456789012345678901234567890",
        ),
        ("signed huge positive", b"1 2 1 +99999999999999999999999999"),
        ("signed huge negative", b"1 2 1 -99999999999999999999999999"),
    ]);
}

#[test]
fn leading_zeros_are_decimal_not_octal() {
    // %u / %d are base 10, so "007" is seven and a long run of zeros is zero.
    check_all(&[
        ("octal-looking", b"007 007 007 007"),
        ("many leading zeros", b"00000000000000000000000000005 2 1 3"),
        ("all zeros padded", b"000000000000000000000000000000 0 0 0"),
        ("zero padded max", b"0000000000004294967295 0 0 0"),
    ]);
}

#[test]
fn base_prefixes_and_float_syntax_stop_the_scan() {
    // %d/%u stop at the first byte that cannot extend a decimal integer, so
    // "0x10" reads 0 and leaves "x10" to break the next conversion.
    check_all(&[
        ("hex prefix", b"0x10 2 3 4"),
        ("hex prefix uppercase", b"0X10 2 3 4"),
        ("decimal point", b"1.5 2.5 3.5 4.5"),
        ("exponent notation", b"1e5 2 3 4"),
        ("infinity", b"inf 2 3 4"),
        ("nan", b"nan 2 3 4"),
    ]);
}

// ---------------------------------------------------------------------------
// Phase C: input classes not reached above -- non-ASCII and non-text stdin,
// long input, and single-write-per-line output framing.
// ---------------------------------------------------------------------------

#[test]
fn non_text_and_high_bytes_on_stdin() {
    check_all(&[
        ("NUL after values", b"1 2 1 42\x00 9"),
        ("NUL first", b"\x00 1 2 3 4"),
        ("high bytes", b"\xff\xfe 1 2 3"),
        ("utf8 text", "é 1 2 3".as_bytes()),
        ("utf8 digits after text", "1 é 3 4".as_bytes()),
        ("all bytes 0x80..0x88", b"\x80\x81\x82\x83\x84\x85\x86\x87\x88"),
        (
            "invalid utf8 between numbers",
            b"1 \xc3\x28 2 3",
        ),
    ]);
}

#[test]
fn long_and_padded_input() {
    // A large amount of leading whitespace and trailing data must not change the
    // four values that are read.
    let mut long_ws = vec![b' '; 100_000];
    long_ws.extend_from_slice(b"1 2 1 42");
    long_ws.extend(std::iter::repeat(b'\n').take(10_000));
    assert_same("100k leading spaces", &long_ws);

    let mut long_digits = vec![b'0'; 100_000];
    long_digits.extend_from_slice(b"5 0 0 0");
    assert_same("100k leading zeros", &long_digits);

    let mut long_tail = Vec::from(&b"3 7 1 -5 "[..]);
    long_tail.extend(std::iter::repeat(b'9').take(200_000));
    assert_same("200k byte trailing tail", &long_tail);
}

#[test]
fn output_is_exactly_one_line_with_trailing_newline() {
    // Guards the printf format string itself: four space-separated fields and a
    // single trailing '\n', with nothing on stderr.
    for input in [
        &b""[..],
        &b"1 2 1 42"[..],
        &b"abc"[..],
        &b"-1 -1 -1 -1"[..],
        &b"4294967296 4294967296 4294967296 4294967296"[..],
    ] {
        let c = run(&c_bin(), input);
        let r = run(&rust_bin(), input);
        assert_eq!(c.stdout, r.stdout, "stdout mismatch for {:?}", Escaped(input));
        assert_eq!(c.stderr, r.stderr, "stderr mismatch for {:?}", Escaped(input));
        assert_eq!(c.status, r.status, "status mismatch for {:?}", Escaped(input));

        assert!(
            c.stdout.ends_with(b"\n"),
            "C stdout should end with a newline: {:?}",
            Escaped(&c.stdout)
        );
        assert_eq!(
            c.stdout.iter().filter(|&&b| b == b'\n').count(),
            1,
            "C stdout should be exactly one line: {:?}",
            Escaped(&c.stdout)
        );
        assert_eq!(
            c.stdout.iter().filter(|&&b| b == b' ').count(),
            3,
            "C stdout should have three separating spaces: {:?}",
            Escaped(&c.stdout)
        );
    }
}

// ---------------------------------------------------------------------------
// Exhaustive sweeps: cheap enough to run in-suite and they cover every reachable
// combination of the two narrow bit-fields.
// ---------------------------------------------------------------------------

#[test]
fn exhaustive_small_x_y_b_z() {
    for x in 0u32..=16 {
        for y in 0u32..=16 {
            let input = format!("{x} {y} {} {}", x % 3, y as i64 - 8);
            assert_same(&format!("x={x} y={y}"), input.as_bytes());
        }
    }
}

#[test]
fn exhaustive_powers_of_two_boundaries() {
    for bit in 0..64u32 {
        let v: u64 = 1u64 << bit;
        for form in [format!("{v}"), format!("-{v}"), format!("+{v}")] {
            let input = format!("{form} {form} {form} {form}");
            assert_same(&format!("2^{bit} as {form}"), input.as_bytes());
        }
        // And each boundary +/- 1, where truncation and saturation interact.
        for delta in [v.wrapping_sub(1), v.wrapping_add(1)] {
            let input = format!("{delta} {delta} {delta} {delta}");
            assert_same(&format!("near 2^{bit}"), input.as_bytes());
        }
    }
}

#[test]
fn separator_matrix() {
    // Every whitespace separator the C locale treats as space must be skipped by
    // %u/%d, and the empty separator must not be.
    let seps: [&[u8]; 9] = [
        b" ", b"\n", b"\t", b"\r", b"\x0b", b"\x0c", b"  \n\t ", b"\r\n", b"",
    ];
    for sep in seps {
        let mut input = Vec::new();
        for (i, tok) in [b"5".as_slice(), b"9", b"3", b"-7"].iter().enumerate() {
            if i > 0 {
                input.extend_from_slice(sep);
            }
            input.extend_from_slice(tok);
        }
        assert_same("separator matrix", &input);
        // Same tokens but with a leading separator too.
        let mut leading = Vec::from(sep);
        leading.extend_from_slice(&input);
        assert_same("leading separator", &leading);
    }
}
