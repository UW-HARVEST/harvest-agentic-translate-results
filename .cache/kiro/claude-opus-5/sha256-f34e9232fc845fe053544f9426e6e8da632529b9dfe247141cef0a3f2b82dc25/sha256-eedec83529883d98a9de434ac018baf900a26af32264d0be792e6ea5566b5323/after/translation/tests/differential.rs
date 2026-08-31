//! Differential tests: run the C binary and the Rust binary as subprocesses
//! with identical stdin, and require byte-identical stdout, byte-identical
//! stderr and an identical exit status.
//!
//! The Rust code is never linked as a library here; only the built executable
//! is driven, the same way a shell would drive it.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;

/// Path to the Rust executable under test, provided by cargo.
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// `translation/` (the crate root).
fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Path to the C executable, building it with CMake on first use.
///
/// `c_src/` is only ever read and configured out-of-tree into `c_src/build`;
/// no source file in `c_src/` is touched.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = crate_dir()
            .parent()
            .expect("translation/ must have a parent")
            .join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");

        if !exe.exists() {
            std::fs::create_dir_all(&build).expect("create c_src/build");

            let configure = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("failed to run `cmake` - is CMake installed?");
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
                .expect("failed to run `cmake --build`");
            assert!(
                compile.status.success(),
                "cmake build failed:\n{}\n{}",
                String::from_utf8_lossy(&compile.stdout),
                String::from_utf8_lossy(&compile.stderr),
            );
        }

        assert!(exe.exists(), "C binary missing at {}", exe.display());
        exe
    })
}

/// Run one program with `input` on stdin and collect the full result.
fn run(program: &Path, input: &[u8]) -> Output {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", program.display()));

    child
        .stdin
        .take()
        .expect("stdin piped")
        .write_all(input)
        .or_else(|e| match e.kind() {
            // The program is allowed to stop reading before we finish writing.
            std::io::ErrorKind::BrokenPipe => Ok(()),
            _ => Err(e),
        })
        .expect("write stdin");

    child.wait_with_output().expect("wait for child")
}

/// Core assertion: stdout, stderr and exit status must all match exactly.
#[track_caller]
fn assert_same(label: &str, input: &[u8]) {
    let c = run(c_bin(), input);
    let r = run(&rust_bin(), input);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch for {label}\n  input:  {:?}\n  C:      {:?}\n  Rust:   {:?}",
        Escaped(input),
        Escaped(&c.stdout),
        Escaped(&r.stdout),
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch for {label}\n  input:  {:?}\n  C:      {:?}\n  Rust:   {:?}",
        Escaped(input),
        Escaped(&c.stderr),
        Escaped(&r.stderr),
    );
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "exit status mismatch for {label}\n  input:  {:?}\n  C:      {:?}\n  Rust:   {:?}",
        Escaped(input),
        c.status,
        r.status,
    );
}

/// Readable rendering of arbitrary bytes in assertion messages.
struct Escaped<'a>(&'a [u8]);

impl std::fmt::Debug for Escaped<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"")?;
        for &b in self.0 {
            match b {
                b'\n' => write!(f, "\\n")?,
                b'\r' => write!(f, "\\r")?,
                b'\t' => write!(f, "\\t")?,
                0x0b => write!(f, "\\v")?,
                0x0c => write!(f, "\\f")?,
                b'"' => write!(f, "\\\"")?,
                b'\\' => write!(f, "\\\\")?,
                0x20..=0x7e => write!(f, "{}", b as char)?,
                _ => write!(f, "\\x{b:02x}")?,
            }
        }
        write!(f, "\"")
    }
}

// ---------------------------------------------------------------------------
// Phase A sanity: both binaries exist and are runnable.
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_run() {
    let c = run(c_bin(), b"1 2 3\n");
    let r = run(&rust_bin(), b"1 2 3\n");
    assert!(c.status.success(), "C binary did not exit 0");
    assert!(r.status.success(), "Rust binary did not exit 0");
    assert_eq!(c.stdout, b"3\n");
    assert_eq!(r.stdout, c.stdout);
}

// ---------------------------------------------------------------------------
// `main`: the read loop. `scanf("%d")` either assigns (loop continues) or
// fails (loop breaks), and the loop is additionally capped at 100 items.
// ---------------------------------------------------------------------------

#[test]
fn empty_input() {
    // i == 0, so call_fma takes its `len == 0` early return.
    assert_same("empty", b"");
}

#[test]
fn whitespace_only_input() {
    // scanf skips whitespace and then hits EOF: still i == 0.
    for (label, input) in [
        ("single space", &b" "[..]),
        ("single newline", b"\n"),
        ("mixed whitespace", b" \t\n \r\n \x0b\x0c "),
        ("many newlines", b"\n\n\n\n\n"),
    ] {
        assert_same(label, input);
    }
}

#[test]
fn single_item() {
    for (label, input) in [
        ("no trailing newline", &b"5"[..]),
        ("trailing newline", b"5\n"),
        ("leading whitespace", b"   7"),
        ("surrounded by whitespace", b"  \n\t 7 \n "),
        ("zero", b"0"),
    ] {
        assert_same(label, input);
    }
}

#[test]
fn multiple_items_various_separators() {
    for (label, input) in [
        ("spaces", &b"1 2 3"[..]),
        ("newlines", b"1\n2\n3\n"),
        ("tabs", b"1\t2\t3"),
        ("crlf", b"1\r\n2\r\n3\r\n"),
        ("runs of whitespace", b"1   \n\n  2\t\t3   \n"),
        ("vertical tab / form feed", b"1\x0b2\x0c3"),
        ("two items", b"10 20"),
    ] {
        assert_same(label, input);
    }
}

#[test]
fn scanf_reads_across_newlines() {
    // Unlike fgets, one scanf("%d") call happily consumes the newline before
    // the number, so line structure is irrelevant to the result.
    assert_same("number on its own line", b"1\n\n\n\n2\n\n\n\n3");
    assert_same("leading blank lines", b"\n\n\n\n41");
}

// ---------------------------------------------------------------------------
// The loop cap: `for (i = 0; i < 100; i++)`. 100 is the maximum the code
// handles; anything beyond it is left unread in the stream.
// ---------------------------------------------------------------------------

fn seq(n: usize) -> Vec<u8> {
    let mut v = Vec::new();
    for k in 1..=n {
        if k > 1 {
            v.push(b' ');
        }
        v.extend_from_slice(k.to_string().as_bytes());
    }
    v
}

#[test]
fn item_counts_around_the_limit() {
    for n in [0usize, 1, 2, 3, 98, 99, 100, 101, 102, 150, 200] {
        assert_same(&format!("{n} items"), &seq(n));
        let mut with_newline = seq(n);
        with_newline.push(b'\n');
        assert_same(&format!("{n} items + newline"), &with_newline);
    }
}

#[test]
fn exactly_one_hundred_items() {
    // The last stored element is data[99], which is what gets printed.
    assert_same("exactly 100", &seq(100));
}

#[test]
fn beyond_the_limit_ignores_the_tail() {
    // The loop stops at 100 regardless of what follows, including inputs that
    // would themselves be error paths if they were read.
    let mut base = seq(100);
    for tail in [&b" 101"[..], b" abc", b" -", b" 2147483648", b" \x00"] {
        let mut input = base.clone();
        input.extend_from_slice(tail);
        assert_same("100 items + unread tail", &input);
    }
    base.push(b' ');
    assert_same("100 items + trailing space", &base);
}

// ---------------------------------------------------------------------------
// Error paths: every way `scanf("%d")` can return something other than 1,
// which is the `break` in main.
// ---------------------------------------------------------------------------

#[test]
fn matching_failure_on_first_item() {
    // scanf returns 0, i stays 0, call_fma returns 0 via the len == 0 branch.
    for (label, input) in [
        ("letters", &b"abc"[..]),
        ("single letter", b"x"),
        ("punctuation", b"."),
        ("comma", b","),
        ("underscore", b"_"),
        ("leading whitespace then letter", b"   \n zzz"),
    ] {
        assert_same(label, input);
    }
}

#[test]
fn matching_failure_mid_stream() {
    // The loop breaks early; the printed value is the last item read.
    for (label, input) in [
        ("letters after numbers", &b"1 2 abc 4 5"[..]),
        ("stop at x", b"1 2 3 x 4 5"),
        ("hex-looking literal", b"0x10"),
        ("float truncated at the dot", b"1.5"),
        ("exponent form", b"1e5"),
        ("comma separated", b"5,6"),
        ("digits then letters, no space", b"12abc"),
        ("second item bad", b"7 !"),
    ] {
        assert_same(label, input);
    }
}

#[test]
fn sign_handling_and_lone_signs() {
    for (label, input) in [
        ("negative", &b"-5"[..]),
        ("explicit plus", b"+9"),
        ("negative zero", b"-0"),
        ("plus zero", b"+0"),
        ("lone minus at EOF", b"-"),
        ("lone plus at EOF", b"+"),
        ("minus then newline then digit", b"-\n5"),
        ("minus then space then digit", b"- 5"),
        ("double minus", b"--5"),
        ("plus then minus", b"+-5"),
        ("minus then plus", b"-+5"),
        ("sign then letter", b"-a"),
        ("mixed signs in a list", b"1 -2 +3 -4"),
        ("negative last", b"1 2 -3"),
        ("trailing lone minus", b"5 -"),
        ("digit then minus", b"5-"),
    ] {
        assert_same(label, input);
    }
}

#[test]
fn eof_immediately_after_a_valid_item() {
    // First call assigns, second returns EOF rather than 0.
    assert_same("one item then EOF", b"42");
    assert_same("items then EOF", b"1 2 3");
}

#[test]
fn embedded_nul_and_non_ascii_bytes() {
    // A NUL is not whitespace and not a digit, so it is a matching failure.
    for (label, input) in [
        ("NUL first", &b"\x005"[..]),
        ("digit NUL digit", b"5\x006"),
        ("whitespace then NUL", b"1 \x002"),
        ("high bytes", b"\x80\xc15"),
        ("utf8 text", "héllo".as_bytes()),
        ("digit then utf8", "5 é 6".as_bytes()),
    ] {
        assert_same(label, input);
    }
}

// ---------------------------------------------------------------------------
// Integer conversion: glibc's %d converts with strtol semantics (saturating
// at long bounds) and then assigns the long to an int, i.e. truncating to 32
// bits. call_fma then computes 1 * value + 0 with int arithmetic.
// ---------------------------------------------------------------------------

#[test]
fn int_boundaries() {
    for (label, input) in [
        ("INT_MAX", &b"2147483647"[..]),
        ("INT_MIN", b"-2147483648"),
        ("INT_MAX + 1", b"2147483648"),
        ("INT_MIN - 1", b"-2147483649"),
        ("2^32", b"4294967296"),
        ("2^32 - 1", b"4294967295"),
        ("2^32 + 1", b"4294967297"),
        ("10^10", b"10000000000"),
        ("-10^10", b"-10000000000"),
    ] {
        assert_same(label, input);
    }
}

#[test]
fn long_boundaries_and_saturation() {
    for (label, input) in [
        ("LONG_MAX", &b"9223372036854775807"[..]),
        ("LONG_MAX + 1", b"9223372036854775808"),
        ("LONG_MIN", b"-9223372036854775808"),
        ("LONG_MIN - 1", b"-9223372036854775809"),
        ("ULONG_MAX", b"18446744073709551615"),
        ("ULONG_MAX + 1", b"18446744073709551616"),
        ("far past LONG_MAX", b"99999999999999999999999999999999"),
        ("far past LONG_MIN", b"-99999999999999999999999999999999"),
    ] {
        assert_same(label, input);
    }
}

#[test]
fn very_long_digit_strings() {
    for n in [19usize, 20, 21, 39, 40, 100, 400] {
        let nines = vec![b'9'; n];
        assert_same(&format!("{n} nines"), &nines);

        let mut neg = vec![b'-'];
        neg.extend_from_slice(&nines);
        assert_same(&format!("negative {n} nines"), &neg);
    }
}

#[test]
fn leading_zeros_are_decimal_not_octal() {
    for (label, input) in [
        ("double zero", &b"00"[..]),
        ("octal-looking 010", b"010"),
        ("many zeros then digit", b"0000000000000000000005"),
        ("negative with leading zeros", b"-0000012"),
    ] {
        assert_same(label, input);
    }

    let mut many = vec![b'0'; 500];
    many.push(b'7');
    assert_same("500 zeros then 7", &many);
}

#[test]
fn overflow_values_in_the_middle_and_at_the_end() {
    for (label, input) in [
        ("overflow then normal", &b"2147483648 5"[..]),
        ("normal then overflow", b"5 2147483648"),
        ("two overflows", b"2147483648 4294967296"),
        ("overflow last of many", b"1 2 3 9223372036854775808"),
    ] {
        assert_same(label, input);
    }
}

// ---------------------------------------------------------------------------
// call_fma / fma_array: the printed value is out[len-1], and every element is
// overwritten by 1 * data[i] + 0. out[0] = 0 before the call is dead for
// len >= 1, but the len == 0 early return means data is never touched.
// ---------------------------------------------------------------------------

#[test]
fn printed_value_is_the_last_item_read() {
    for (label, input) in [
        ("ascending", &b"1 2 3 4 5"[..]),
        ("descending", b"5 4 3 2 1"),
        ("last is zero", b"1 2 3 0"),
        ("last is negative", b"1 2 3 -7"),
        ("last is negative zero", b"1 2 3 -0"),
        ("all zeros", b"0 0 0 0"),
        ("single zero", b"0"),
        ("len 1 exercises out[0]", b"-123"),
    ] {
        assert_same(label, input);
    }
}

#[test]
fn output_has_exactly_one_trailing_newline() {
    // printf("%d\n") - no padding, no extra whitespace, nothing on stderr.
    let out = run(c_bin(), b"1 2 3\n");
    assert_eq!(out.stdout, b"3\n");
    assert!(out.stderr.is_empty());
    assert_same("formatting", b"1 2 3\n");
    assert_same("formatting, negative", b"-2147483648");
}

// ---------------------------------------------------------------------------
// Broad sweep: deterministic pseudo-random inputs over the alphabet the
// scanner branches on, plus random numeric magnitudes.
// ---------------------------------------------------------------------------

/// Small deterministic xorshift PRNG so the corpus is reproducible.
struct Rng(u64);

impl Rng {
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

#[test]
fn fuzz_over_scanner_alphabet() {
    const ALPHABET: &[u8] = b" \n\t\r\x0b\x0c-+0123456789xX.,ae_\x00\xff";
    let mut rng = Rng(0x243f_6a88_85a3_08d3);

    for case in 0..400 {
        let len = rng.below(48);
        let input: Vec<u8> = (0..len).map(|_| ALPHABET[rng.below(ALPHABET.len())]).collect();
        assert_same(&format!("fuzz alphabet #{case}"), &input);
    }
}

#[test]
fn fuzz_over_numeric_magnitudes() {
    let mut rng = Rng(0x1319_8a2e_0370_7344);
    let separators: [&[u8]; 4] = [b" ", b"\n", b"\t", b"  \n "];

    for case in 0..300 {
        let count = rng.below(9);
        let mut parts: Vec<String> = Vec::new();
        for _ in 0..count {
            let raw = rng.next_u64();
            let text = match rng.below(5) {
                0 => (raw as i32).to_string(),
                1 => (raw as i64).to_string(),
                2 => {
                    const INTERESTING: [&str; 8] = [
                        "0",
                        "2147483647",
                        "-2147483648",
                        "2147483648",
                        "4294967296",
                        "9223372036854775807",
                        "-9223372036854775808",
                        "9223372036854775808",
                    ];
                    INTERESTING[rng.below(INTERESTING.len())].to_string()
                }
                3 => format!("{}", (raw % 21) as i64 - 10),
                _ => {
                    // A digit string of random length, sometimes negative.
                    let digits = 1 + rng.below(40);
                    let mut s = String::new();
                    if rng.below(2) == 0 {
                        s.push('-');
                    }
                    s.push((b'1' + rng.below(9) as u8) as char);
                    for _ in 1..digits {
                        s.push((b'0' + rng.below(10) as u8) as char);
                    }
                    s
                }
            };
            parts.push(text);
        }

        let sep = separators[rng.below(separators.len())];
        let mut input = Vec::new();
        for (idx, part) in parts.iter().enumerate() {
            if idx > 0 {
                input.extend_from_slice(sep);
            }
            input.extend_from_slice(part.as_bytes());
        }
        assert_same(&format!("fuzz numeric #{case}"), &input);
    }
}
