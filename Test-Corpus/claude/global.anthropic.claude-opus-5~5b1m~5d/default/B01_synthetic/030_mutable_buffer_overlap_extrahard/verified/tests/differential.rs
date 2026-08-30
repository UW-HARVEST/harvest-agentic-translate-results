//! Differential tests: run the original C program and the Rust translation as
//! subprocesses on identical stdin, and require byte-identical stdout, stderr
//! and exit status.
//!
//! The Rust code is NEVER called as a library here -- both sides are driven
//! exactly the way a shell would drive them, because that is how the
//! translation is graded.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;

/// Path to the Rust binary under test, provided by Cargo.
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Build the C reference program and return its path.
///
/// Cargo runs the tests in this file concurrently on many threads, and every
/// one of them needs the C binary. Two `cmake` processes writing the same build
/// directory corrupt each other's cache (it shows up as "The C compiler
/// identification is unknown"), so the build is funnelled through a `OnceLock`
/// and happens exactly once per test process.
///
/// `c_src/` is treated as read-only ground truth; we only ever create the
/// out-of-source `c_src/build` directory that CMake owns.
fn c_bin() -> PathBuf {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(build_c_bin).clone()
}

fn build_c_bin() -> PathBuf {
    let root = repo_root();
    let c_src = root.join("c_src");
    let build = c_src.join("build");
    let bin = build.join("driver");

    if bin.exists() {
        return bin;
    }

    std::fs::create_dir_all(&build).expect("could not create c_src/build");

    let conf = Command::new("cmake")
        .arg("..")
        .current_dir(&build)
        .output()
        .expect("failed to run `cmake ..` -- is cmake installed?");
    assert!(
        conf.status.success(),
        "cmake configure failed:\n{}\n{}",
        String::from_utf8_lossy(&conf.stdout),
        String::from_utf8_lossy(&conf.stderr)
    );

    let built = Command::new("cmake")
        .args(["--build", "."])
        .current_dir(&build)
        .output()
        .expect("failed to run `cmake --build .`");
    assert!(
        built.status.success(),
        "cmake build failed:\n{}\n{}",
        String::from_utf8_lossy(&built.stdout),
        String::from_utf8_lossy(&built.stderr)
    );

    assert!(bin.exists(), "C driver binary missing after build: {:?}", bin);
    bin
}

/// Feed `input` to `prog` on stdin and capture everything it produces.
fn run(prog: &Path, input: &[u8]) -> Output {
    let mut child = Command::new(prog)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {:?}: {e}", prog));

    // Write on a helper thread so a program that never drains stdin (or that
    // exits early) cannot deadlock us on a full pipe.
    let mut stdin = child.stdin.take().expect("piped stdin");
    let data = input.to_vec();
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&data);
        let _ = stdin.flush();
        // dropping `stdin` closes the pipe, signalling EOF
    });

    let out = child.wait_with_output().expect("failed to collect output");
    writer.join().expect("stdin writer thread panicked");
    out
}

fn describe(input: &[u8]) -> String {
    let shown: String = input
        .iter()
        .take(120)
        .map(|&b| match b {
            b'\n' => "\\n".to_string(),
            b'\r' => "\\r".to_string(),
            b'\t' => "\\t".to_string(),
            0x0b => "\\v".to_string(),
            0x0c => "\\f".to_string(),
            0 => "\\0".to_string(),
            0x20..=0x7e => (b as char).to_string(),
            other => format!("\\x{other:02x}"),
        })
        .collect();
    if input.len() > 120 {
        format!("{shown}... ({} bytes total)", input.len())
    } else {
        shown
    }
}

/// The core assertion: all three observable channels must match exactly.
#[track_caller]
fn assert_same(name: &str, input: &[u8]) {
    let c = run(&c_bin(), input);
    let r = run(&rust_bin(), input);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch for case `{name}`\n  input: {}\n  C   stdout: {:?}\n  Rust stdout: {:?}",
        describe(input),
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout),
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch for case `{name}`\n  input: {}\n  C   stderr: {:?}\n  Rust stderr: {:?}",
        describe(input),
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr),
    );
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "exit status mismatch for case `{name}`\n  input: {}\n  C: {:?}  Rust: {:?}",
        describe(input),
        c.status,
        r.status,
    );
}

#[track_caller]
fn check(name: &str, input: &str) {
    assert_same(name, input.as_bytes());
}

// ---------------------------------------------------------------------------
// Empty / minimal input -- `i` stays 0, so driver() prints nothing.
// ---------------------------------------------------------------------------

#[test]
fn empty_input_produces_no_output() {
    check("empty", "");
}

#[test]
fn whitespace_only_input() {
    // scanf skips whitespace and then hits EOF: zero conversions.
    check("spaces", "     ");
    check("newlines", "\n\n\n\n");
    check("all_c_whitespace", " \t\n\r\x0b\x0c");
    check("single_newline", "\n");
}

#[test]
fn single_item() {
    check("zero", "0");
    check("one", "1");
    check("five", "5");
    check("five_nl", "5\n");
    check("negative", "-3");
    check("explicit_plus", "+7");
    check("negative_one", "-1");
}

// ---------------------------------------------------------------------------
// scanf("%d") reads ACROSS newlines -- any whitespace mix separates items.
// ---------------------------------------------------------------------------

#[test]
fn scanf_crosses_newlines_and_mixed_whitespace() {
    check("mixed_ws", "  1 \n 2\t3\n");
    check("no_trailing_newline", "1 2 3");
    check("one_per_line", "1\n2\n3\n4\n5\n");
    check("crlf", "1\r\n2\r\n3\r\n");
    check("tabs", "\t10\t20\t30\t");
    check("vt_ff_separated", "1\x0b2\x0c3");
    check("leading_ws_then_values", "\n\n\n  \t 4 5\n");
}

// ---------------------------------------------------------------------------
// Matching failure paths: scanf != 1 -> break, then print what was read.
// ---------------------------------------------------------------------------

#[test]
fn non_numeric_input_stops_reading() {
    check("pure_alpha", "abc");
    check("alpha_then_number", "abc 5");
    check("number_then_alpha", "5 abc 6");
    check("punctuation", "!!!");
    check("comma_separated", "1,2,3");
    check("semicolons", "1;2;3");
}

#[test]
fn sign_without_digits_is_a_matching_failure() {
    check("lone_minus", "-");
    check("lone_plus", "+");
    check("minus_then_alpha", "-x");
    check("plus_then_alpha", "+x");
    check("minus_then_space", "- 5");
    check("double_minus", "--5");
    check("plus_minus", "+-5");
    check("minus_then_newline", "-\n5");
    check("value_then_lone_minus", "7 -");
    check("value_then_double_sign", "7 --8");
}

#[test]
fn partial_numeric_forms_stop_at_first_non_digit() {
    // "%d" consumes only an optional sign plus a digit run.
    check("hex_prefix", "0x10"); // reads 0, then 'x' fails
    check("float", "3.5"); // reads 3, then '.' fails
    check("exponent", "1e5"); // reads 1, then 'e' fails
    check("trailing_letter", "12abc");
    check("underscore", "1_000");
    check("negative_float", "-2.75");
    check("float_then_int", "1.5 9");
}

// ---------------------------------------------------------------------------
// The 100-element array bound in main().
// ---------------------------------------------------------------------------

fn seq(n: usize) -> String {
    let mut s = String::new();
    for i in 1..=n {
        s.push_str(&i.to_string());
        s.push('\n');
    }
    s
}

#[test]
fn exactly_ninety_nine_items() {
    check("99", &seq(99));
}

#[test]
fn exactly_one_hundred_items_is_the_maximum() {
    check("100", &seq(100));
}

#[test]
fn more_than_one_hundred_items_ignores_the_excess() {
    // The loop stops at i == 100; trailing input is never consumed.
    check("101", &seq(101));
    check("150", &seq(150));
    check("300", &seq(300));
}

#[test]
fn hundred_items_followed_by_garbage() {
    // Loop exits on the bound, not on the bad token, so the garbage is unread.
    let mut s = seq(100);
    s.push_str("garbage not a number\n");
    check("100_then_garbage", &s);
}

#[test]
fn ninety_nine_items_followed_by_garbage() {
    // Here the 100th scanf DOES run and fails, so i == 99.
    let mut s = seq(99);
    s.push_str("garbage\n");
    check("99_then_garbage", &s);
}

#[test]
fn one_hundred_identical_extremes() {
    check("100x_int_max", &"2147483647\n".repeat(100));
    check("100x_int_min", &"-2147483648\n".repeat(100));
    check("100x_zero", &"0\n".repeat(100));
}

// ---------------------------------------------------------------------------
// Arithmetic: out[i] = out[i]*out[i] + out[i], with C's signed wraparound.
// ---------------------------------------------------------------------------

#[test]
fn int_boundary_values() {
    check("int_max", "2147483647");
    check("int_min", "-2147483648");
    check("int_max_minus_one", "2147483646");
    check("int_min_plus_one", "-2147483647");
    check("pow2_30", "1073741824");
    check("neg_pow2_30", "-1073741824");
    check("sqrt_int_max", "46340"); // 46340^2 fits
    check("sqrt_int_max_plus_one", "46341"); // 46341^2 overflows
    check("neg_sqrt", "-46340");
    check("neg_sqrt_plus_one", "-46341");
    check("pow2_16", "65536");
    check("pow2_16_minus_one", "65535");
}

#[test]
fn overflow_wraps_the_way_c_does() {
    check(
        "overflow_mix",
        "100000 -100000 50000 -50000 2147483647 -2147483648 46341 -46341 65536 123456789",
    );
}

// ---------------------------------------------------------------------------
// strtol saturation: glibc's %d converts with long semantics, saturates an
// out-of-range digit run to LONG_MAX/LONG_MIN, then truncates to int.
// ---------------------------------------------------------------------------

#[test]
fn values_beyond_int_range_truncate() {
    check("int_max_plus_one", "2147483648");
    check("int_min_minus_one", "-2147483649");
    check("pow2_32_minus_one", "4294967295");
    check("pow2_32", "4294967296");
    check("pow2_32_plus_one", "4294967297");
    check("neg_pow2_32", "-4294967296");
    check("pow2_33", "8589934592");
}

#[test]
fn values_beyond_long_range_saturate_then_truncate() {
    check("long_max_minus_one", "9223372036854775806");
    check("long_max", "9223372036854775807");
    check("long_max_plus_one", "9223372036854775808");
    check("long_max_plus_two", "9223372036854775809");
    check("long_min_plus_one", "-9223372036854775807");
    check("long_min", "-9223372036854775808");
    check("long_min_minus_one", "-9223372036854775809");
    check("long_min_minus_two", "-9223372036854775810");
    check("ulong_max", "18446744073709551615");
    check("ulong_max_plus_one", "18446744073709551616");
    check("random_huge", "12345678901234567890");
    check("signed_huge", "+9223372036854775808");
}

#[test]
fn absurdly_long_digit_runs() {
    check("ten_thousand_nines", &"9".repeat(10_000));
    check("ten_thousand_nines_negative", &format!("-{}", "9".repeat(10_000)));
    check("leading_zeros_small", "0000000000000000005");
    check("many_leading_zeros_then_five", &format!("{}5", "0".repeat(10_000)));
    check("only_many_zeros", &"0".repeat(10_000));
    check(
        "padded_long_max_plus_one",
        &format!("{}9223372036854775808", "0".repeat(8)),
    );
}

// ---------------------------------------------------------------------------
// Stream-buffering edges: the translation refills stdin in 4096-byte chunks
// and relies on one byte of pushback, so tokens straddling a chunk boundary
// are a real risk.
// ---------------------------------------------------------------------------

#[test]
fn tokens_straddling_buffer_boundaries() {
    for pad in [4090usize, 4093, 4094, 4095, 4096, 4097, 8190, 8191, 8192] {
        let ws = " ".repeat(pad);
        check(&format!("pad{pad}_then_values"), &format!("{ws}1234567 89"));
        check(&format!("pad{pad}_then_signed"), &format!("{ws}-5 6"));
        // A digit immediately followed by a non-digit across the boundary
        // exercises ungetc() right after a refill.
        check(&format!("pad{pad}_digit_then_junk"), &format!("{ws}7x 8"));
        check(&format!("pad{pad}_then_eof"), &format!("{ws}42"));
    }
}

#[test]
fn many_values_spanning_multiple_buffer_refills() {
    // ~100 wide values plus padding forces several refills mid-token.
    let mut s = String::new();
    for i in 0..100 {
        s.push_str(&format!("{:>80}\n", 1000000 + i * 7919));
    }
    check("wide_padded_values", &s);
}

// ---------------------------------------------------------------------------
// Raw bytes that a shell cannot easily express.
// ---------------------------------------------------------------------------

#[test]
fn embedded_nul_bytes_terminate_conversion() {
    assert_same("nul_between", b"1\x002");
    assert_same("leading_nul", b"\x001 2");
    assert_same("only_nul", b"\x00");
    assert_same("nul_after_values", b"1 2 3\x004 5");
}

#[test]
fn non_utf8_and_high_bytes() {
    assert_same("invalid_utf8", b"1 2 \xff\xfe 3");
    assert_same("high_byte_first", b"\x80\x81");
    assert_same("latin1_digits", b"5 \xb2\xb3 6");
    // Multi-byte UTF-8 "full width" digit is not an ASCII digit.
    assert_same("fullwidth_digit", "５".as_bytes());
    assert_same("minus_sign_u2212", "−5".as_bytes());
}

// ---------------------------------------------------------------------------
// A broad randomized sweep over the value space.
// ---------------------------------------------------------------------------

/// Tiny deterministic xorshift PRNG so the sweep needs no dev-dependencies.
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
    fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}

#[test]
fn randomized_value_sweep() {
    const INTERESTING: [i64; 17] = [
        0,
        1,
        -1,
        2,
        -2,
        46340,
        46341,
        -46340,
        -46341,
        65535,
        65536,
        i32::MAX as i64,
        i32::MIN as i64,
        1073741824,
        -1073741824,
        i32::MAX as i64 + 1,
        i32::MIN as i64 - 1,
    ];

    let mut rng = Rng(0x2545F4914F6CDD1D);
    for case in 0..80 {
        let n = 1 + rng.below(100) as usize;
        let mut s = String::new();
        for _ in 0..n {
            let v: i64 = if rng.below(100) < 40 {
                INTERESTING[rng.below(INTERESTING.len() as u64) as usize]
            } else {
                (rng.next_u64() as u32) as i32 as i64
            };
            s.push_str(&v.to_string());
            s.push(if rng.below(4) == 0 { ' ' } else { '\n' });
        }
        assert_same(&format!("random_case_{case}"), s.as_bytes());
    }
}

#[test]
fn randomized_token_soup() {
    // Mixes valid numbers with tokens that trigger the matching-failure path,
    // so the break can land at any index.
    const TOKENS: [&str; 14] = [
        "0", "1", "-1", "2147483647", "-2147483648", "9999999999999999999", "abc", "-", "+",
        "0x1f", "3.5", "--2", ".", "1e9",
    ];
    let mut rng = Rng(0xDEADBEEFCAFEF00D);
    for case in 0..60 {
        let n = 1 + rng.below(120) as usize;
        let mut s = String::new();
        for _ in 0..n {
            s.push_str(TOKENS[rng.below(TOKENS.len() as u64) as usize]);
            s.push(match rng.below(5) {
                0 => '\t',
                1 => ' ',
                _ => '\n',
            });
        }
        assert_same(&format!("soup_case_{case}"), s.as_bytes());
    }
}
