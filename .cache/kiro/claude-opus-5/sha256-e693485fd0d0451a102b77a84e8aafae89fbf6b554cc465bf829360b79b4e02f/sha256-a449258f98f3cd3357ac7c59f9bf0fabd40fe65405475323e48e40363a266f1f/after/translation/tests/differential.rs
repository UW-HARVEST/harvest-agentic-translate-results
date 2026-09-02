//! Differential tests: run the C `driver` and the Rust `driver` as *subprocesses*
//! and compare stdout, stderr and exit status byte-for-byte / value-for-value.
//!
//! The Rust code is never called as a library here — the binary is driven exactly
//! the way a shell would drive it, which is how the translation is graded.
//!
//! Input surface of the program (from `c_src/src/main.c`):
//!
//! ```c
//! int main() { int x = 0; scanf("%d", &x); driver(x); return 0; }
//! ```
//!
//! The single input is stdin, consumed by one `scanf("%d", &x)`. Every branch the
//! program can take is therefore a property of that conversion:
//!   * input failure (EOF before any non-whitespace)  -> x stays 0
//!   * matching failure (no digits after optional sign) -> x stays 0
//!   * successful conversion, incl. leading whitespace (C `isspace`: ' ' \t \n \v \f \r)
//!   * optional '+' / '-' sign
//!   * strtol saturation at LONG_MAX / LONG_MIN followed by truncation to `int`
//!   * plain truncation for values that fit in `long` but not `int`
//!
//! `driver()`/`print_hex()` have no data-dependent branches: the loop always runs
//! `sizeof(house_t)` == 16 times. What varies is the 4 bytes of `house.floors`.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Once;

// ---------------------------------------------------------------------------
// Locating and building the two executables
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the Rust executable under test. Cargo builds it for us.
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

static C_BUILD: Once = Once::new();

/// Path to the C executable, building it with CMake on first use.
fn c_bin() -> PathBuf {
    let root = repo_root();
    let c_src = root.join("c_src");
    let build_dir = c_src.join("build");
    let exe = build_dir.join("driver");

    C_BUILD.call_once(|| {
        if exe.is_file() {
            return;
        }
        std::fs::create_dir_all(&build_dir).expect("create c_src/build");

        let cfg = Command::new("cmake")
            .arg("..")
            .current_dir(&build_dir)
            .output()
            .expect("failed to spawn `cmake` — is CMake installed?");
        assert!(
            cfg.status.success(),
            "cmake configure failed:\n{}\n{}",
            String::from_utf8_lossy(&cfg.stdout),
            String::from_utf8_lossy(&cfg.stderr)
        );

        let bld = Command::new("cmake")
            .args(["--build", "."])
            .current_dir(&build_dir)
            .output()
            .expect("failed to spawn `cmake --build`");
        assert!(
            bld.status.success(),
            "cmake build failed:\n{}\n{}",
            String::from_utf8_lossy(&bld.stdout),
            String::from_utf8_lossy(&bld.stderr)
        );
    });

    assert!(
        exe.is_file(),
        "C executable not found at {} — build c_src first",
        exe.display()
    );
    exe
}

// ---------------------------------------------------------------------------
// Running a program
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq)]
struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Some(code)` for a normal exit, `None` if killed by a signal.
    code: Option<i32>,
    /// Terminating signal, if any (Unix).
    signal: Option<i32>,
}

impl std::fmt::Debug for Run {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Run {{ exit: {}, stdout: {:?} ({}), stderr: {:?} ({}) }}",
            match (self.code, self.signal) {
                (Some(c), _) => format!("code {c}"),
                (None, Some(s)) => format!("signal {s}"),
                (None, None) => "unknown".to_string(),
            },
            String::from_utf8_lossy(&self.stdout),
            hex(&self.stdout),
            String::from_utf8_lossy(&self.stderr),
            hex(&self.stderr),
        )
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn run(exe: &Path, input: &[u8]) -> Run {
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));

    {
        let mut stdin = child.stdin.take().expect("piped stdin");
        // The child may exit without reading everything; a broken pipe is fine.
        let _ = stdin.write_all(input);
        let _ = stdin.flush();
    }

    let out = child.wait_with_output().expect("wait_with_output");

    #[cfg(unix)]
    let signal = {
        use std::os::unix::process::ExitStatusExt;
        out.status.signal()
    };
    #[cfg(not(unix))]
    let signal: Option<i32> = None;

    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal,
    }
}

/// The core assertion: stdout, stderr and exit status must all match.
#[track_caller]
fn assert_same(label: &str, input: &[u8]) {
    let c = run(&c_bin(), input);
    let r = run(&rust_bin(), input);

    assert_eq!(
        c.stdout,
        r.stdout,
        "STDOUT mismatch for {label}\n  input bytes: {}\n  C  : {:?}\n  Rust: {:?}",
        hex(input),
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "STDERR mismatch for {label}\n  input bytes: {}\n  C  : {:?}\n  Rust: {:?}",
        hex(input),
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "EXIT STATUS mismatch for {label}\n  input bytes: {}\n  C  : {:?}\n  Rust: {:?}",
        hex(input),
        c,
        r
    );
}

#[track_caller]
fn same(label: &str, input: &str) {
    assert_same(label, input.as_bytes());
}

// ---------------------------------------------------------------------------
// Phase A — both programs exist, run, and produce something
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_run() {
    let c = run(&c_bin(), b"1");
    let r = run(&rust_bin(), b"1");
    assert_eq!(c.code, Some(0), "C should exit 0: {c:?}");
    assert_eq!(r.code, Some(0), "Rust should exit 0: {r:?}");
    assert!(!c.stdout.is_empty(), "C produced no stdout");
    assert_eq!(c.stdout, r.stdout);
}

/// Pin the exact expected byte layout so a silent change in either program is
/// caught even if both drifted together.
///
/// `house_t{ int floors; int bedrooms; double bathrooms; }` on LP64 x86-64:
/// floors@0, bedrooms@4, bathrooms@8, size 16, no padding bytes.
/// bedrooms = 3 -> `03000000`; bathrooms = 2.0 -> `0000000000000040`.
#[test]
fn known_layout_for_input_one() {
    let c = run(&c_bin(), b"1");
    assert_eq!(
        c.stdout, b"01000000030000000000000000000040\n",
        "unexpected C layout (host ABI differs from the documented one?): {}",
        String::from_utf8_lossy(&c.stdout)
    );
    let r = run(&rust_bin(), b"1");
    assert_eq!(c.stdout, r.stdout);
    assert!(c.stdout.ends_with(b"\n"), "print_hex must end with a newline");
}

// ---------------------------------------------------------------------------
// Phase B — the input classes the C branches on
// ---------------------------------------------------------------------------

#[test]
fn empty_and_eof_input() {
    // scanf returns EOF (input failure); x keeps its initializer 0.
    assert_same("completely empty stdin", b"");
}

#[test]
fn single_item_minimal_input() {
    same("single digit", "0");
    same("single digit 1", "1");
    same("single digit 9", "9");
}

#[test]
fn plain_decimal_values() {
    for v in [
        "0", "1", "2", "3", "7", "10", "42", "99", "100", "12345", "65535", "65536", "1000000",
        "123456789", "1073741824",
    ] {
        same("plain decimal", v);
    }
}

#[test]
fn signed_values() {
    for v in [
        "-1", "-2", "-42", "-12345", "+1", "+42", "+0", "-0", "+2147483647", "-2147483647",
    ] {
        same("signed", v);
    }
}

#[test]
fn int_boundaries() {
    same("INT_MAX", "2147483647");
    same("INT_MAX+1 (truncates to INT_MIN)", "2147483648");
    same("INT_MIN", "-2147483648");
    same("INT_MIN-1 (truncates to INT_MAX)", "-2147483649");
    same("UINT_MAX", "4294967295");
    same("2^32 (truncates to 0)", "4294967296");
    same("2^32+1", "4294967297");
    same("-2^32", "-4294967296");
}

#[test]
fn long_boundaries_and_strtol_saturation() {
    same("LONG_MAX", "9223372036854775807");
    same("LONG_MAX+1 -> saturates to LONG_MAX", "9223372036854775808");
    same("LONG_MIN", "-9223372036854775808");
    same("LONG_MIN-1 -> saturates to LONG_MIN", "-9223372036854775809");
    same("far above LONG_MAX", "99999999999999999999999999999999");
    same("far below LONG_MIN", "-99999999999999999999999999999999");
    same("absurdly long digit run", &"9".repeat(400));
    same("absurdly long negative digit run", &format!("-{}", "9".repeat(400)));
}

#[test]
fn values_fitting_long_but_not_int() {
    same("5e9", "5000000000");
    same("-5e9", "-5000000000");
    same("1e18", "1000000000000000000");
    same("-1e18", "-1000000000000000000");
    same("1e19 (overflows long)", "10000000000000000000");
}

#[test]
fn matching_failure_leaves_x_zero() {
    same("letters", "abc");
    same("lone minus", "-");
    same("lone plus", "+");
    same("minus then newline", "-\n5");
    same("plus then letters", "+abc");
    same("two signs", "--5");
    same("sign flip flop", "-+5");
    same("plus minus", "+-5");
    same("leading dot", ".5");
    same("leading comma", ",5");
    same("leading x", "x5");
    same("punctuation", "!@#$%");
}

#[test]
fn whitespace_is_skipped_including_vertical_tab() {
    // C's isspace() in the "C" locale is exactly: ' ' \t \n \v \f \r.
    // Rust's u8::is_ascii_whitespace() omits \v (0x0B) — a real mismatch source.
    same("leading space", " 42");
    same("leading tab", "\t42");
    same("leading newline", "\n42");
    same("leading vertical tab", "\u{b}42");
    same("leading form feed", "\u{c}42");
    same("leading carriage return", "\r42");
    same("all six whitespace chars", " \t\n\u{b}\u{c}\r42");
    same("scanf reads across newlines", "\n\n\n\n99");
    same("whitespace then sign", "  \u{b}\t-8");
    same("lots of whitespace", &format!("{}123456", " ".repeat(200)));
}

#[test]
fn whitespace_only_input_is_input_failure() {
    same("single space", " ");
    same("single newline", "\n");
    same("single vertical tab", "\u{b}");
    same("single form feed", "\u{c}");
    same("single tab", "\t");
    same("single carriage return", "\r");
    same("mixed whitespace only", " \t\n\u{b}\u{c}\r \t\n");
    same("many spaces only", &" ".repeat(500));
}

#[test]
fn conversion_stops_at_first_non_digit() {
    same("digits then letters", "42abc");
    same("hex-looking", "0x10");
    same("hex-looking nonzero lead", "0X1F");
    same("e-notation", "1e5");
    same("float", "3.14");
    same("thousands separator", "1,234");
    same("underscore separator", "1_000");
    same("trailing L", "5L");
    same("digit space digit", "4 2");
    same("two numbers, only first read", "7 8");
    same("newline terminated", "42\n");
    same("crlf terminated", "42\r\n");
    same("trailing junk after newline", "42\nignored\n");
}

#[test]
fn leading_zeros_are_decimal_not_octal() {
    same("zeros", "000");
    same("many zeros", &"0".repeat(300));
    same("padded 42", "0000000000000000000042");
    same("octal-looking 010", "010");
    same("zeros then overflow", &format!("{}9223372036854775808", "0".repeat(50)));
    same("negative padded", "-0000042");
}

#[test]
fn embedded_nul_and_non_ascii_bytes() {
    assert_same("NUL first", b"\x005");
    assert_same("digit then NUL", b"5\x00abc");
    assert_same("NUL only", b"\x00");
    assert_same("high bytes", b"\xff\xfe\xfd");
    assert_same("utf8 then number", "é 5".as_bytes());
    assert_same("control bytes", b"\x01\x02\x03");
    assert_same("digits then high byte", b"12\xff34");
}

// ---------------------------------------------------------------------------
// Phase C — systematic sweeps over paths not covered above
// ---------------------------------------------------------------------------

/// Every possible single byte as the entire input. Covers the whitespace set,
/// the sign characters, each digit, and every rejected byte, exhaustively.
#[test]
fn exhaustive_single_byte_inputs() {
    for b in 0u8..=255 {
        assert_same(&format!("single byte 0x{b:02x}"), &[b]);
    }
}

/// Two-byte inputs drawn from the alphabet that actually matters to `%d`,
/// exercising sign/digit/whitespace/terminator combinations.
#[test]
fn exhaustive_two_byte_inputs_over_interesting_alphabet() {
    let alphabet: &[u8] = b"0123456789+- \t\n\x0b\x0c\r.xeE,\x00a";
    for &a in alphabet {
        for &b in alphabet {
            assert_same(&format!("bytes 0x{a:02x}{b:02x}"), &[a, b]);
        }
    }
}

/// Every bit position of the resulting `int`, plus its negation, so each byte of
/// `house.floors` in the hex dump takes on non-zero values.
#[test]
fn every_bit_of_floors() {
    for shift in 0..32u32 {
        let v = 1u32 << shift;
        same(&format!("1<<{shift}"), &v.to_string());
        same(&format!("-(1<<{shift})"), &format!("-{v}"));
        same(&format!("(1<<{shift})-1"), &(v.wrapping_sub(1)).to_string());
    }
    for v in [
        i32::MIN,
        i32::MIN + 1,
        -1,
        0,
        1,
        i32::MAX - 1,
        i32::MAX,
        0x0f0f0f0fu32 as i32,
        0x7f7f7f7fu32 as i32,
        0x0000_00ffu32 as i32,
        0x00ff_0000u32 as i32,
    ] {
        same(&format!("value {v}"), &v.to_string());
    }
}

/// Digit-count sweep: 1..=25 digits, positive and negative, crossing the int and
/// long boundaries and entering strtol's saturation range.
#[test]
fn digit_length_sweep() {
    for n in 1..=25usize {
        let s: String = (0..n).map(|i| char::from(b'1' + (i % 9) as u8)).collect();
        same(&format!("{n} digits"), &s);
        same(&format!("{n} digits negative"), &format!("-{s}"));
    }
}

/// Deterministic pseudo-random inputs (xorshift, fixed seed) over both raw bytes
/// and numeric-looking text. Deterministic so the suite never flakes.
#[test]
fn deterministic_fuzz() {
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    const NUMERIC_ALPHABET: &[u8] = b"0123456789+-  \t\n\x0b\x0c\r.,xXeEabcdefABCDEF_L";

    for i in 0..400 {
        // Raw arbitrary bytes.
        let len = (next() % 24) as usize;
        let bytes: Vec<u8> = (0..len).map(|_| (next() & 0xff) as u8).collect();
        assert_same(&format!("fuzz raw #{i}"), &bytes);

        // Numeric-ish text.
        let len = (next() % 30) as usize;
        let text: Vec<u8> = (0..len)
            .map(|_| NUMERIC_ALPHABET[(next() % NUMERIC_ALPHABET.len() as u64) as usize])
            .collect();
        assert_same(&format!("fuzz numeric #{i}"), &text);

        // Pure digit runs of random length, sometimes signed: hammers strtol
        // overflow/truncation.
        let len = (next() % 28) as usize + 1;
        let mut digits: Vec<u8> = Vec::with_capacity(len + 1);
        if next() % 2 == 0 {
            digits.push(b'-');
        }
        for _ in 0..len {
            digits.push(b'0' + (next() % 10) as u8);
        }
        assert_same(&format!("fuzz digits #{i}"), &digits);
    }
}

/// Large input: the program reads only the first conversion, so a big payload
/// must not change the output (and must not deadlock on the write side either).
#[test]
fn large_input_only_first_conversion_is_read() {
    let mut big = Vec::new();
    big.extend_from_slice(b"77\n");
    big.extend(std::iter::repeat(b'x').take(200_000));
    assert_same("77 followed by 200k junk bytes", &big);

    let mut big2 = Vec::new();
    big2.extend(std::iter::repeat(b' ').take(100_000));
    big2.extend_from_slice(b"-1234567\n");
    big2.extend(std::iter::repeat(b'9').take(100_000));
    assert_same("100k spaces, number, 100k digits", &big2);
}

/// stdin closed immediately vs. stdin that is an empty-but-open pipe: both are
/// input failures for scanf and must agree.
#[test]
fn stdin_closed_and_binary_zero_fill() {
    assert_same("no bytes at all", b"");
    assert_same("1024 NUL bytes", &vec![0u8; 1024]);
    assert_same("1024 0xff bytes", &vec![0xffu8; 1024]);
}

/// Output must be exactly 33 bytes (32 hex digits + '\n') with no stderr,
/// for every input class — a structural invariant of print_hex.
#[test]
fn output_shape_invariant() {
    for input in [
        "".to_string(),
        "0".to_string(),
        "-1".to_string(),
        "abc".to_string(),
        "2147483647".to_string(),
        "9".repeat(100),
    ] {
        let c = run(&c_bin(), input.as_bytes());
        let r = run(&rust_bin(), input.as_bytes());
        assert_eq!(c.stdout.len(), 33, "C stdout length for {input:?}: {c:?}");
        assert_eq!(r.stdout.len(), 33, "Rust stdout length for {input:?}: {r:?}");
        assert!(c.stderr.is_empty(), "C wrote to stderr: {c:?}");
        assert!(r.stderr.is_empty(), "Rust wrote to stderr: {r:?}");
        assert_eq!(c.stdout, r.stdout);
        assert_eq!(c.code, r.code);
    }
}
