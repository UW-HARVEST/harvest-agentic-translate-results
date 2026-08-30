//! Differential tests: run the C binary and the Rust binary as subprocesses with
//! identical stdin and require byte-identical stdout, byte-identical stderr and
//! an identical exit status.
//!
//! Nothing here links against the Rust code as a library; both programs are
//! driven exactly the way a shell would drive them.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

/// Path to the Rust binary produced by this crate.
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn c_src_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .join("c_src")
}

/// Path to the C binary, building it with CMake on first use if necessary.
fn c_bin() -> PathBuf {
    let src = c_src_dir();
    let build = src.join("build");
    let bin = build.join("driver");
    if bin.is_file() {
        return bin;
    }

    std::fs::create_dir_all(&build).expect("cannot create c_src/build");
    let configure = Command::new("cmake")
        .arg("..")
        .current_dir(&build)
        .output()
        .expect("failed to run `cmake ..` - is cmake installed?");
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
    assert!(bin.is_file(), "C binary missing after build: {}", bin.display());
    bin
}

/// Runs `program`, writing `stdin_bytes` to its standard input.
fn run(program: &Path, stdin_bytes: &[u8]) -> Output {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", program.display()));

    {
        let mut sink = child.stdin.take().expect("piped stdin");
        // The child may exit before consuming all of stdin; a broken pipe is not
        // a test failure, it is the same thing a shell would observe.
        let _ = sink.write_all(stdin_bytes);
        let _ = sink.flush();
    }

    child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("failed to wait for {}: {e}", program.display()))
}

/// Asserts the two programs agree on stdout, stderr and exit status for `input`.
fn assert_same(label: &str, input: &[u8]) {
    let c = run(&c_bin(), input);
    let r = run(&rust_bin(), input);

    assert_eq!(
        c.stdout,
        r.stdout,
        "[{label}] stdout differs for input {:?}\n  C   : {:?}\n  Rust: {:?}",
        Show(input),
        Show(&c.stdout),
        Show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "[{label}] stderr differs for input {:?}\n  C   : {:?}\n  Rust: {:?}",
        Show(input),
        Show(&c.stderr),
        Show(&r.stderr)
    );
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "[{label}] exit status differs for input {:?}: C {:?} vs Rust {:?}",
        Show(input),
        c.status,
        r.status
    );
}

/// Escapes bytes for readable assertion messages, truncating very long inputs.
struct Show<'a>(&'a [u8]);

impl std::fmt::Debug for Show<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (head, elided) = if self.0.len() > 96 {
            (&self.0[..96], self.0.len() - 96)
        } else {
            (self.0, 0)
        };
        for &b in head {
            match b {
                b'\n' => write!(f, "\\n")?,
                b'\r' => write!(f, "\\r")?,
                b'\t' => write!(f, "\\t")?,
                0x0b => write!(f, "\\v")?,
                0x0c => write!(f, "\\f")?,
                0x20..=0x7e => write!(f, "{}", b as char)?,
                _ => write!(f, "\\x{b:02x}")?,
            }
        }
        if elided > 0 {
            write!(f, "...(+{elided} bytes)")?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Input classes the C program branches on.
//
// main() is: `int x = 0; scanf("%d", &x); driver(x);`
// The branch points are therefore entirely inside the `%d` conversion:
//   1. leading-whitespace skip (any amount, including none, including newlines)
//   2. optional '+' / '-' sign, or none
//   3. at least one digit required -> otherwise matching failure and `x` keeps 0
//   4. EOF before any character -> input failure and `x` keeps 0
//   5. magnitude out of `long` range -> glibc clamps to LONG_MAX / LONG_MIN
//   6. the `long` result is stored through an `int *`, keeping the low 32 bits
// and inside driver(): `2*x + 300` as wrapping 32-bit signed arithmetic.
// ---------------------------------------------------------------------------

// --- no conversion happens: x stays 0, output is 300 ---

#[test]
fn empty_input() {
    assert_same("empty", b"");
}

#[test]
fn whitespace_only_inputs() {
    for (i, s) in [
        &b" "[..],
        b"   ",
        b"\n",
        b"\n\n\n",
        b"\t",
        b"\r",
        b"\x0b",
        b"\x0c",
        b" \t\n\r\x0b\x0c",
    ]
    .iter()
    .enumerate()
    {
        assert_same(&format!("ws_only#{i}"), s);
    }
}

#[test]
fn matching_failure_non_numeric() {
    for (i, s) in [
        &b"abc"[..],
        b"abc\n",
        b"x5",
        b".5",
        b"-.5",
        b"+.5",
        b"e5",
        b",5",
        b"/5",
        b":5",
        b"\x00",
        b"\x005",
        b"\xff\xfe5\n",
        b"'5'",
    ]
    .iter()
    .enumerate()
    {
        assert_same(&format!("nonnumeric#{i}"), s);
    }
}

#[test]
fn sign_without_digits_is_a_matching_failure() {
    for (i, s) in [
        &b"-"[..], b"+", b"-\n", b"+\n", b"- 5", b"+ 5", b"--5", b"++5", b"-+5", b"+-5", b"-abc",
    ]
    .iter()
    .enumerate()
    {
        assert_same(&format!("lonesign#{i}"), s);
    }
}

// --- successful conversions ---

#[test]
fn single_small_values() {
    for v in [0i64, 1, 2, 5, 7, 9, 10, 42, 99, 100, 149, 150, 151, 1000] {
        assert_same(&format!("small({v})"), format!("{v}\n").as_bytes());
        assert_same(&format!("small_neg({v})"), format!("-{v}\n").as_bytes());
        assert_same(&format!("small_plus({v})"), format!("+{v}\n").as_bytes());
    }
}

#[test]
fn no_trailing_newline() {
    assert_same("no_nl", b"5");
    assert_same("no_nl_neg", b"-5");
    assert_same("no_nl_zero", b"0");
}

#[test]
fn negative_zero() {
    assert_same("neg_zero", b"-0\n");
    assert_same("plus_zero", b"+0\n");
    assert_same("neg_zeros", b"-00000\n");
}

#[test]
fn leading_whitespace_is_skipped_across_newlines() {
    // scanf's %d skips *any* run of whitespace, unlike fgets which stops at \n.
    assert_same("nl_then_num", b"\n\n\t 7\n");
    assert_same("many_nl", b"\n\n\n\n\n\n\n\n42\n");
    assert_same("mixed_ws", b" \t\r\x0b\x0c\n-13\n");
    assert_same("ws_then_sign", b"   -5\n");
}

#[test]
fn conversion_stops_at_first_non_digit() {
    assert_same("digits_then_alpha", b"12abc\n");
    assert_same("digits_then_dot", b"12.75\n");
    assert_same("hex_like", b"0x10\n");
    assert_same("exp_like", b"1e3\n");
    assert_same("plus5xyz", b"  +5xyz\n");
    assert_same("digit_then_nul", b"5\x00\n");
    assert_same("digit_then_minus", b"5-3\n");
    assert_same("comma_thousands", b"1,000\n");
}

#[test]
fn only_the_first_number_is_read() {
    // There is exactly one scanf; trailing input is never consumed and cannot
    // change the output.
    assert_same("two_numbers", b"7 8\n");
    assert_same("many_numbers", b"1 2 3 4 5\n");
    assert_same("num_then_junk", b"7\nnot a number\n");
}

#[test]
fn leading_zeros_are_decimal_not_octal() {
    assert_same("octal_like_007", b"007\n");
    assert_same("octal_like_010", b"010\n");
    assert_same("zeros_then_5", b"00000000000000000005\n");
}

// --- 32-bit wrapping inside driver(): y = 2*x, y += 300 ---

#[test]
fn int_boundaries_and_wraparound() {
    for v in [
        2147483647i64,  // INT_MAX      -> 2*x wraps negative
        2147483646,
        -2147483648,    // INT_MIN      -> 2*x wraps to 0
        -2147483647,
        1073741824,     // 2^30         -> 2*x is exactly INT_MIN
        1073741823,
        1073741674,     // 2*x + 300 lands exactly on INT_MIN
        -1073741824,
        -150,           // 2*x + 300 == 0
        -151,
        149,
    ] {
        assert_same(&format!("wrap({v})"), format!("{v}\n").as_bytes());
    }
}

#[test]
fn values_beyond_int_are_truncated_to_the_low_32_bits() {
    for v in [
        2147483648i64, // INT_MAX + 1
        2147483649,
        4294967295, // 2^32 - 1
        4294967296, // 2^32      -> truncates to 0
        4294967297,
        3000000000,
        -2147483649, // INT_MIN - 1
        -4294967296,
        -4294967297,
        -3000000000,
        1234567890123,
        -1234567890123,
    ] {
        assert_same(&format!("trunc({v})"), format!("{v}\n").as_bytes());
    }
}

// --- the largest magnitudes the conversion handles, and past them ---

#[test]
fn long_range_boundaries() {
    for s in [
        "9223372036854775806",  // LONG_MAX - 1
        "9223372036854775807",  // LONG_MAX
        "9223372036854775808",  // LONG_MAX + 1 -> clamped to LONG_MAX
        "9223372036854775809",
        "-9223372036854775807",
        "-9223372036854775808", // LONG_MIN
        "-9223372036854775809", // LONG_MIN - 1 -> clamped to LONG_MIN
        "18446744073709551615", // 2^64 - 1
        "18446744073709551616",
    ] {
        assert_same(&format!("longedge({s})"), format!("{s}\n").as_bytes());
    }
}

#[test]
fn magnitudes_far_past_long_range() {
    for s in [
        "99999999999999999999",
        "-99999999999999999999",
        "10000000000000000000000000",
        "-10000000000000000000000000",
    ] {
        assert_same(&format!("huge({s})"), format!("{s}\n").as_bytes());
    }
    // Overflow must still be detected when the digits arrive after a long run of
    // insignificant zeros, and a padded in-range value must not be mistaken for
    // an overflow.
    let mut padded_overflow = vec![b'0'; 4096];
    padded_overflow.extend_from_slice(b"99999999999999999999\n");
    assert_same("padded_overflow", &padded_overflow);

    let mut padded_in_range = vec![b'0'; 4096];
    padded_in_range.extend_from_slice(b"123\n");
    assert_same("padded_in_range", &padded_in_range);
}

#[test]
fn very_long_digit_runs() {
    // The conversion has no field width, so it consumes every digit.
    let mut nines = vec![b'9'; 100_000];
    nines.push(b'\n');
    assert_same("100k_nines", &nines);

    let mut zeros = vec![b'0'; 100_000];
    zeros.extend_from_slice(b"7\n");
    assert_same("100k_zeros_then_7", &zeros);

    let mut signed = vec![b'-'];
    signed.extend(std::iter::repeat(b'9').take(100_000));
    signed.push(b'\n');
    assert_same("100k_nines_negative", &signed);
}

#[test]
fn large_whitespace_run_before_the_number() {
    let mut ws = vec![b'\n'; 70_000];
    ws.extend_from_slice(b"11\n");
    assert_same("70k_newlines", &ws);
}

// --- exhaustive-ish sweep, so a boundary that was missed above still shows up ---

#[test]
fn sweep_of_representative_values() {
    let mut cases: Vec<String> = Vec::new();
    for shift in 0..64 {
        let base: i128 = 1i128 << shift;
        for delta in [-1i128, 0, 1] {
            cases.push((base + delta).to_string());
            cases.push((-(base + delta)).to_string());
        }
    }
    for (i, s) in cases.iter().enumerate() {
        assert_same(&format!("sweep#{i}({s})"), format!("{s}\n").as_bytes());
    }
}

#[test]
fn every_byte_as_the_first_character() {
    // Each possible leading byte selects one of: whitespace skip, sign, digit,
    // or matching failure.
    for b in 0u16..=255 {
        let input = [b as u8, b'4', b'2', b'\n'];
        assert_same(&format!("firstbyte(0x{b:02x})"), &input);
    }
}
