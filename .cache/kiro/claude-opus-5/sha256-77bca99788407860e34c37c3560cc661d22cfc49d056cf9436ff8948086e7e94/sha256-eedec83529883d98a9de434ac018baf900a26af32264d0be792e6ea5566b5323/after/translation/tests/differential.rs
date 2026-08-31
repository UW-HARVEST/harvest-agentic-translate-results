//! Differential tests: run the original C binary and the Rust binary as
//! subprocesses with identical stdin, and require byte-identical stdout,
//! byte-identical stderr, and an identical exit status.
//!
//! The Rust code is never linked in as a library — the built executable is
//! driven exactly the way a shell would drive it, because that is how the two
//! programs are compared.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Once;

/// Path to the Rust executable under test, supplied by Cargo.
const RUST_BIN: &str = env!("CARGO_BIN_EXE_driver");

fn workspace_root() -> PathBuf {
    // .../<root>/translation -> .../<root>
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

fn c_src_dir() -> PathBuf {
    workspace_root().join("c_src")
}

fn c_bin() -> PathBuf {
    c_src_dir().join("build").join("driver")
}

/// Builds the C reference program with CMake if it is not already built.
///
/// Nothing under `c_src/` is modified other than the generated `build/`
/// directory that `CMakeLists.txt` expects to be created out-of-source.
fn ensure_c_binary() -> PathBuf {
    static BUILD: Once = Once::new();
    BUILD.call_once(|| {
        let bin = c_bin();
        if bin.exists() {
            return;
        }
        let build_dir = c_src_dir().join("build");
        std::fs::create_dir_all(&build_dir).expect("create c_src/build");

        let configure = Command::new("cmake")
            .arg("..")
            .current_dir(&build_dir)
            .output()
            .expect("failed to run `cmake ..` — is cmake installed?");
        assert!(
            configure.status.success(),
            "cmake configure failed:\n{}\n{}",
            String::from_utf8_lossy(&configure.stdout),
            String::from_utf8_lossy(&configure.stderr)
        );

        let build = Command::new("cmake")
            .args(["--build", "."])
            .current_dir(&build_dir)
            .output()
            .expect("failed to run `cmake --build .`");
        assert!(
            build.status.success(),
            "cmake build failed:\n{}\n{}",
            String::from_utf8_lossy(&build.stdout),
            String::from_utf8_lossy(&build.stderr)
        );
    });

    let bin = c_bin();
    assert!(
        bin.exists(),
        "C reference binary missing at {}",
        bin.display()
    );
    bin
}

struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Some(code)` for a normal exit, `None` if killed by a signal.
    code: Option<i32>,
}

fn run(program: &Path, stdin_bytes: &[u8]) -> Outcome {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", program.display()));

    {
        let mut sink = child.stdin.take().expect("stdin pipe");
        let bytes = stdin_bytes.to_vec();
        // Write on a helper thread so a program that never drains stdin cannot
        // deadlock the test against a full pipe buffer.
        std::thread::spawn(move || {
            let _ = sink.write_all(&bytes);
            let _ = sink.flush();
        });
    }

    let out = child.wait_with_output().expect("wait_with_output");
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
    }
}

fn show(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) if bytes.len() <= 200 => format!("{s:?}"),
        _ if bytes.len() <= 64 => format!("{bytes:02x?}"),
        _ => format!("<{} bytes: {:02x?}...>", bytes.len(), &bytes[..32]),
    }
}

/// Runs both programs on the same stdin and asserts full observable equality.
fn assert_same(name: &str, stdin_bytes: &[u8]) {
    let c = ensure_c_binary();
    let c_out = run(&c, stdin_bytes);
    let r_out = run(Path::new(RUST_BIN), stdin_bytes);

    assert_eq!(
        c_out.stdout,
        r_out.stdout,
        "stdout differs for case `{name}` (stdin = {})\n  C:    {}\n  Rust: {}",
        show(stdin_bytes),
        show(&c_out.stdout),
        show(&r_out.stdout)
    );
    assert_eq!(
        c_out.stderr,
        r_out.stderr,
        "stderr differs for case `{name}` (stdin = {})\n  C:    {}\n  Rust: {}",
        show(stdin_bytes),
        show(&c_out.stderr),
        show(&r_out.stderr)
    );
    assert_eq!(
        c_out.code,
        r_out.code,
        "exit status differs for case `{name}` (stdin = {}): C={:?} Rust={:?}",
        show(stdin_bytes),
        c_out.code,
        r_out.code
    );
}

fn assert_same_str(name: &str, stdin_text: &str) {
    assert_same(name, stdin_text.as_bytes());
}

// ---------------------------------------------------------------------------
// The C program has exactly one input-dependent decision: whether the single
// `scanf("%d", &x)` conversion succeeds. On success `x` becomes the converted
// value (truncated to `int`); on a matching failure or EOF, `x` keeps its
// initializer of 0. `driver()` then always prints the 16-byte object
// representation of `house_t` and returns 0. The cases below cover both sides
// of that decision plus the numeric edges of the conversion.
// ---------------------------------------------------------------------------

// --- EOF / no-input paths: scanf returns EOF, x stays 0 --------------------

#[test]
fn empty_input() {
    assert_same_str("empty", "");
}

#[test]
fn only_whitespace() {
    // %d skips whitespace, then hits EOF: matching never starts.
    assert_same_str("spaces", "   ");
    assert_same_str("newlines", "\n\n\n");
    assert_same_str("tabs", "\t\t");
    assert_same_str("mixed ws", " \t\n\r\x0b\x0c ");
}

#[test]
fn closed_and_null_stdin() {
    assert_same("dev-null equivalent", b"");
}

// --- matching-failure paths: x stays 0 ------------------------------------

#[test]
fn non_numeric_input() {
    assert_same_str("letters", "abc");
    assert_same_str("underscore", "_5");
    assert_same_str("punctuation", "!!!");
    assert_same_str("leading dot", ".5");
    assert_same_str("comma", ",7");
}

#[test]
fn sign_without_digits() {
    // A sign is consumed but the conversion still fails with no digit after it.
    assert_same_str("dash only", "-");
    assert_same_str("plus only", "+");
    assert_same_str("dash newline", "-\n");
    assert_same_str("dash space digit", "- 5");
    assert_same_str("double dash", "--5");
    assert_same_str("plus dash", "+-5");
    assert_same_str("dash letter", "-a");
}

// --- successful conversions ----------------------------------------------

#[test]
fn single_value() {
    assert_same_str("zero", "0");
    assert_same_str("one", "1");
    assert_same_str("three", "3");
    assert_same_str("ten", "10");
    assert_same_str("negative zero", "-0");
    assert_same_str("negative one", "-1");
    assert_same_str("explicit plus", "+5");
}

#[test]
fn trailing_newline_variants() {
    // The trailing bytes are irrelevant: only one conversion is performed.
    assert_same_str("no newline", "7");
    assert_same_str("lf", "7\n");
    assert_same_str("crlf", "7\r\n");
    assert_same_str("many newlines", "7\n\n\n");
}

#[test]
fn reads_across_newlines() {
    // scanf's %d skips leading newlines, unlike fgets which would stop at one.
    assert_same_str("newline then int", "\n42");
    assert_same_str("blank lines then int", "\n\n\n   42\n");
    assert_same_str("crlf then int", "\r\n42");
    assert_same_str("vt ff then int", "\x0b\x0c7");
}

#[test]
fn stops_at_first_non_digit() {
    assert_same_str("digits then letters", "42abc");
    assert_same_str("digits then dot", "3.9");
    assert_same_str("digits then dash", "12-34");
    assert_same_str("hex literal", "0x10");
    assert_same_str("two integers", "12 34");
    assert_same_str("scientific", "1e9");
    assert_same_str("thousands separators", "1,234");
}

#[test]
fn leading_zeros_are_decimal_not_octal() {
    assert_same_str("octal-looking", "010");
    assert_same_str("padded", "0000000000000005");
    assert_same_str("signed padded", "  +0000000001  ");
    assert_same_str("negative padded", "-000000012");
}

// --- numeric edges: int range, long range, truncation ---------------------

#[test]
fn int_boundaries() {
    assert_same_str("INT_MAX", "2147483647");
    assert_same_str("INT_MIN", "-2147483648");
    assert_same_str("INT_MAX+1", "2147483648");
    assert_same_str("INT_MIN-1", "-2147483649");
    assert_same_str("UINT_MAX", "4294967295");
}

#[test]
fn truncation_to_int() {
    // glibc converts with strtol (a 64-bit long here) and stores through an
    // `int *`, so the low 32 bits survive.
    assert_same_str("2^32", "4294967296");
    assert_same_str("2^32+1", "4294967297");
    assert_same_str("2^32-1 negated", "-4294967295");
    assert_same_str("2^33", "8589934592");
    assert_same_str("13 digits", "2147483647999");
    assert_same_str("negative 2^32", "-4294967296");
}

#[test]
fn long_boundaries_and_saturation() {
    assert_same_str("LONG_MAX", "9223372036854775807");
    assert_same_str("LONG_MIN", "-9223372036854775808");
    assert_same_str("LONG_MAX+1", "9223372036854775808");
    assert_same_str("LONG_MAX+2", "9223372036854775809");
    assert_same_str("LONG_MIN-1", "-9223372036854775809");
    assert_same_str("2^64", "18446744073709551616");
    assert_same_str("20 nines", "99999999999999999999");
    assert_same_str("26 nines", "99999999999999999999999999");
    assert_same_str("26 nines negative", "-99999999999999999999999999");
}

#[test]
fn very_long_digit_runs() {
    let mut zeros = "0".repeat(100_000);
    zeros.push('7');
    assert_same_str("100k leading zeros", &zeros);

    let nines = "9".repeat(100_000);
    assert_same_str("100k nines", &nines);
    assert_same_str("100k nines negative", &format!("-{nines}"));

    // Just over the width where accumulation overflows mid-multiply vs mid-add.
    assert_same_str("19 digits", "1234567890123456789");
    assert_same_str("20 digits", "12345678901234567890");
}

// --- non-text input -------------------------------------------------------

#[test]
fn binary_and_nul_bytes() {
    assert_same("nul prefix", b"\x00\x005");
    assert_same("nul suffix", b"5\x00");
    assert_same("nul only", b"\x00");
    assert_same("invalid utf8", b"\xff\xfe");
    assert_same("high bytes then digit", b"\x80\x819");
    assert_same("digit then invalid utf8", b"9\xff\xfe");
}

#[test]
fn large_unread_input() {
    // The program converts once and exits, leaving most of stdin unread; both
    // implementations must still agree (and neither may hang).
    let mut data = b"42\n".to_vec();
    data.extend(std::iter::repeat(b'x').take(300_000));
    assert_same("42 then 300k junk", &data);
}

// --- the shape of the output itself --------------------------------------

#[test]
fn output_shape_is_32_hex_digits_plus_newline() {
    let c = ensure_c_binary();
    for input in ["", "0", "1", "-1", "2147483647", "abc"] {
        let out = run(&c, input.as_bytes());
        assert_eq!(
            out.stdout.len(),
            33,
            "expected sizeof(house_t)==16 printed as %02x plus a newline, got {}",
            show(&out.stdout)
        );
        assert_eq!(*out.stdout.last().unwrap(), b'\n');
        assert!(out.stdout[..32]
            .iter()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b)));
        assert!(out.stderr.is_empty(), "C writes nothing to stderr");
        assert_eq!(out.code, Some(0), "C always returns 0");
    }
}
