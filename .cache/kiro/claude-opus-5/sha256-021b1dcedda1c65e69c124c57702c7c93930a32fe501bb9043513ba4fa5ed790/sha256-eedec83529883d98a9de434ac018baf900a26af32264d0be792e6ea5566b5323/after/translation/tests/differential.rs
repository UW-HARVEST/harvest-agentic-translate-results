//! Differential tests: run the C `driver` and the Rust `driver` as
//! subprocesses on identical stdin and require byte-identical stdout, stderr
//! and exit status.
//!
//! The Rust code is never linked as a library here; only the built binary is
//! driven, exactly the way a shell would drive it.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::Once;

/// Workspace root, i.e. the directory holding `c_src/` and `translation/`.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

fn c_src_dir() -> PathBuf {
    repo_root().join("c_src")
}

fn c_binary() -> PathBuf {
    c_src_dir().join("build").join("driver")
}

/// Build the C reference with CMake, once per test binary invocation.
/// Nothing under `c_src/` is modified apart from the generated `build/` tree.
fn ensure_c_binary() -> PathBuf {
    static BUILD: Once = Once::new();
    BUILD.call_once(|| {
        let bin = c_binary();
        if bin.exists() {
            return;
        }
        let build_dir = c_src_dir().join("build");
        std::fs::create_dir_all(&build_dir).expect("cannot create c_src/build");

        let configure = Command::new("cmake")
            .arg("..")
            .current_dir(&build_dir)
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
            .current_dir(&build_dir)
            .output()
            .expect("failed to run `cmake --build .`");
        assert!(
            compile.status.success(),
            "cmake build failed:\n{}\n{}",
            String::from_utf8_lossy(&compile.stdout),
            String::from_utf8_lossy(&compile.stderr)
        );
    });

    let bin = c_binary();
    assert!(bin.exists(), "C reference binary missing at {}", bin.display());
    bin
}

fn rust_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Feed `input` to `program` on stdin and capture everything it produces.
fn run(program: &Path, input: &[u8]) -> Output {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", program.display()));

    {
        let mut stdin = child.stdin.take().expect("stdin was piped");
        // The child may legitimately stop reading (it never does here, but a
        // broken pipe must not turn into a spurious test failure).
        let _ = stdin.write_all(input);
        let _ = stdin.flush();
    }

    child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("failed to collect output of {}: {e}", program.display()))
}

fn show(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) if s.len() <= 400 => format!("{s:?}"),
        Ok(s) => format!("{:?}... ({} bytes)", &s[..400], s.len()),
        Err(_) => format!("{bytes:?}"),
    }
}

/// The single assertion used by every case: stdout, stderr and exit status
/// must all agree between the C program and the Rust program.
#[track_caller]
fn assert_same(name: &str, input: &[u8]) {
    let c = ensure_c_binary();
    let r = rust_binary();

    let expected = run(&c, input);
    let actual = run(&r, input);

    assert_eq!(
        expected.stdout,
        actual.stdout,
        "stdout mismatch for case `{name}` (input {})\n  C:    {}\n  Rust: {}",
        show(input),
        show(&expected.stdout),
        show(&actual.stdout)
    );
    assert_eq!(
        expected.stderr,
        actual.stderr,
        "stderr mismatch for case `{name}` (input {})\n  C:    {}\n  Rust: {}",
        show(input),
        show(&expected.stderr),
        show(&actual.stderr)
    );
    assert_eq!(
        expected.status.code(),
        actual.status.code(),
        "exit status mismatch for case `{name}` (input {}): C {:?} vs Rust {:?}",
        show(input),
        expected.status,
        actual.status
    );
}

#[track_caller]
fn assert_same_str(name: &str, input: &str) {
    assert_same(name, input.as_bytes());
}

// ---------------------------------------------------------------------------
// Phase A — both programs build and are runnable.
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_exist_and_run() {
    let c = ensure_c_binary();
    let r = rust_binary();
    let a = run(&c, b"1\n");
    let b = run(&r, b"1\n");
    assert_eq!(a.status.code(), Some(0), "C program did not exit 0");
    assert_eq!(b.status.code(), Some(0), "Rust program did not exit 0");
    assert_eq!(a.stdout, b"2\n");
    assert_eq!(a.stdout, b.stdout);
}

// ---------------------------------------------------------------------------
// Phase B — the input classes `main` branches on.
//
// `main` has exactly two loop exits: `i == 100`, and `scanf("%d", ..) != 1`
// (EOF or matching failure). `driver` then branches on `len == 0` vs `len > 0`
// for both the fma loop and the print loop.
// ---------------------------------------------------------------------------

/// `scanf` returns EOF immediately: `i == 0`, both loops in `driver` execute
/// zero times, so nothing at all is printed.
#[test]
fn empty_input_prints_nothing() {
    assert_same_str("empty", "");
    let out = run(&ensure_c_binary(), b"");
    assert!(out.stdout.is_empty(), "C prints nothing for empty input");
}

/// Whitespace-only input is still EOF as far as `%d` is concerned, because the
/// conversion skips leading whitespace before looking for a digit.
#[test]
fn whitespace_only_is_eof() {
    for (name, input) in [
        ("single newline", "\n"),
        ("many newlines", "\n\n\n\n"),
        ("spaces", "    "),
        ("tabs", "\t\t"),
        ("mixed", " \t\r\n \x0b\x0c "),
        ("vertical tab", "\x0b"),
        ("form feed", "\x0c"),
        ("carriage return", "\r"),
        ("long run", &" ".repeat(10_000)),
    ] {
        assert_same_str(name, input);
    }
}

/// One value: `x * x + x`.
#[test]
fn single_item() {
    for v in [0i64, 1, 2, 3, -1, -2, 7, 10, -10, 12345, -12345] {
        assert_same_str(&format!("single {v}"), &v.to_string());
    }
}

/// A trailing newline must not change anything, and its absence must not
/// truncate the last value.
#[test]
fn trailing_newline_is_irrelevant() {
    assert_same_str("no trailing newline", "1 2 3");
    assert_same_str("trailing newline", "1 2 3\n");
    assert_same_str("trailing spaces", "1 2 3   ");
    assert_same_str("many trailing newlines", "1 2 3\n\n\n");
}

/// `scanf` skips *any* whitespace, so the separator and line structure are
/// interchangeable — unlike `fgets`, which would stop at a newline.
#[test]
fn scanf_reads_across_newlines() {
    let values = [4i64, -5, 6, 0, 7];
    let joined: Vec<String> = values.iter().map(|v| v.to_string()).collect();
    for sep in [" ", "\n", "\t", "\r\n", "  \n\t ", "\n\n\n", "\x0b", "\x0c"] {
        assert_same_str(&format!("separator {sep:?}"), &joined.join(sep));
    }
    // Leading whitespace before the first value.
    assert_same_str("leading whitespace", "\n\n   \t4 -5 6 0 7\n");
}

/// Boundary of the `i < 100` guard: 99 fits, 100 exactly fills the array and
/// exits via the count check, 101+ leaves input unread.
#[test]
fn item_count_boundaries() {
    for n in [0usize, 1, 2, 98, 99, 100, 101, 102, 150, 500] {
        let space: Vec<String> = (1..=n).map(|v| v.to_string()).collect();
        assert_same_str(&format!("{n} items space separated"), &space.join(" "));

        let lines: String = (1..=n).map(|v| format!("{v}\n")).collect();
        assert_same_str(&format!("{n} items one per line"), &lines);
    }
}

/// Once 100 values are read the loop exits on the count, so trailing garbage
/// is never examined and cannot cause an error.
#[test]
fn garbage_after_hundred_items_is_never_read() {
    let head: Vec<String> = (1..=100).map(|v| v.to_string()).collect();
    let head = head.join(" ");
    assert_same_str("100 items then letters", &format!("{head} not-a-number"));
    assert_same_str("100 items then nul", &format!("{head} \0\0\0"));
    assert_same_str("100 items then more numbers", &format!("{head} 101 102 103"));
}

/// Matching failure: `scanf` returns 0, the loop breaks, and only the values
/// read *before* the bad token are processed.
#[test]
fn matching_failure_truncates_the_input() {
    assert_same_str("letters only", "abc");
    assert_same_str("letters first", "abc 1 2 3");
    assert_same_str("letters in the middle", "1 2 x 3 4");
    assert_same_str("letters at the end", "1 2 3 zzz");
    assert_same_str("dot", ".");
    assert_same_str("comma separated", "1,2,3");
    assert_same_str("semicolon separated", "1;2;3");
    assert_same_str("underscore", "1_2");
    assert_same_str("float", "3.5");
    assert_same_str("float list", "1.5 2.5");
    assert_same_str("exponent", "1e5");
    assert_same_str("inf", "inf");
    assert_same_str("nan", "nan");
    assert_same_str("hex prefix", "0x10");
    assert_same_str("hex prefix after value", "5 0x10");
    assert_same_str("digits then letters", "12abc");
    assert_same_str("digits then letters then digits", "12abc 34");
    assert_same_str("zero then junk", "0junk");
    assert_same_str("unicode sign", "\u{b1}5");
}

/// A sign with no digits after it is also a matching failure.
#[test]
fn sign_without_digits_is_a_matching_failure() {
    assert_same_str("minus at eof", "-");
    assert_same_str("plus at eof", "+");
    assert_same_str("minus then letter", "-a");
    assert_same_str("plus then letter", "+a");
    assert_same_str("minus then space then digit", "- 5");
    assert_same_str("double minus", "--5");
    assert_same_str("plus minus", "+-5");
    assert_same_str("minus then newline", "-\n5");
    assert_same_str("values then lone minus", "1 2 -");
    assert_same_str("minus dot", "-.");
}

/// `%d` accepts an explicit `+`, and leading zeros are decimal (not octal).
#[test]
fn accepted_integer_spellings() {
    assert_same_str("explicit plus", "+7");
    assert_same_str("plus zero", "+0");
    assert_same_str("minus zero", "-0");
    assert_same_str("leading zeros", "0000000012");
    assert_same_str("octal looking", "010 011 017 018 019");
    assert_same_str("huge zero run", &format!("{}7", "0".repeat(500)));
    assert_same_str("only zeros", "0000000000000000000000000");
    assert_same_str("mixed signs", "+1 -2 +3 -4");
}

/// Embedded NUL is an ordinary non-matching byte for `%d`.
#[test]
fn nul_bytes() {
    assert_same("lone nul", b"\0");
    assert_same("nul after value", b"5\0 6");
    assert_same("nul between values", b"1 \0 2");
    assert_same("nul terminated digits", b"12\x0034");
}

/// `out[i] = out[i] * out[i] + out[i]` overflows `int` for most inputs; the
/// wrap the compiled C performs must be reproduced.
#[test]
fn signed_overflow_in_the_fma() {
    let values: [i64; 26] = [
        0, 1, -1, 2, -2, 3, -3, 100, -100, 32768, -32768, 46339, 46340, 46341, -46340, -46341,
        65535, 65536, 100_000, 1_073_741_823, 1_073_741_824, -1_073_741_824, 2_147_483_646,
        2_147_483_647, -2_147_483_647, -2_147_483_648,
    ];
    for v in values {
        assert_same_str(&format!("overflow single {v}"), &v.to_string());
    }
    let all: Vec<String> = values.iter().map(|v| v.to_string()).collect();
    assert_same_str("overflow all one per line", &all.join("\n"));
    assert_same_str("overflow all space separated", &all.join(" "));
}

/// `%d` converts through `long` and then stores into `int`: out-of-range input
/// saturates at `LONG_MIN`/`LONG_MAX` and is then truncated.
#[test]
fn out_of_int_range_values() {
    for s in [
        "2147483647",
        "2147483648",
        "2147483649",
        "-2147483648",
        "-2147483649",
        "4294967295",
        "4294967296",
        "4294967297",
        "-4294967296",
        "9223372036854775806",
        "9223372036854775807",
        "9223372036854775808",
        "9223372036854775809",
        "-9223372036854775807",
        "-9223372036854775808",
        "-9223372036854775809",
        "18446744073709551615",
        "18446744073709551616",
        "99999999999999999999999999",
        "-99999999999999999999999999",
    ] {
        assert_same_str(&format!("range {s}"), s);
    }
}

/// Digit runs far longer than any integer type, including ones padded with
/// zeros so that the significant digits start late.
#[test]
fn absurdly_long_digit_runs() {
    for n in [19usize, 20, 21, 40, 100, 500, 5000] {
        assert_same_str(&format!("{n} nines"), &"9".repeat(n));
        assert_same_str(&format!("minus {n} nines"), &format!("-{}", "9".repeat(n)));
        assert_same_str(&format!("{n} ones"), &"1".repeat(n));
        assert_same_str(
            &format!("zero padded {n}"),
            &format!("{}{}", "0".repeat(n), "9".repeat(n)),
        );
    }
    assert_same_str("long run then valid", &format!("{} 5", "9".repeat(300)));
}

// ---------------------------------------------------------------------------
// Phase C — paths not covered above, plus a randomized sweep.
// ---------------------------------------------------------------------------

/// Full array of out-of-range values: exercises the saturating conversion and
/// the overflowing fma together, 100 times over.
#[test]
fn full_array_of_extremes() {
    let pool = [
        "0",
        "-0",
        "2147483647",
        "-2147483648",
        "9223372036854775807",
        "-9223372036854775808",
        "99999999999999999999",
        "-99999999999999999999",
        "46341",
        "-46341",
        "1",
        "-1",
    ];
    let items: Vec<&str> = (0..100).map(|i| pool[i % pool.len()]).collect();
    assert_same_str("100 extremes", &items.join("\n"));
    assert_same_str("100 extremes spaced", &items.join(" "));
}

/// The count boundary reached by a matching failure rather than by EOF, at
/// each interesting position.
#[test]
fn matching_failure_at_every_boundary() {
    for n in [0usize, 1, 2, 50, 98, 99, 100, 101] {
        let head: Vec<String> = (1..=n).map(|v| v.to_string()).collect();
        let mut s = head.join(" ");
        if n > 0 {
            s.push(' ');
        }
        s.push_str("STOP 1 2 3");
        assert_same_str(&format!("failure after {n} items"), &s);
    }
}

/// EOF reached mid-token in the middle of a long digit run.
#[test]
fn eof_immediately_after_digits() {
    assert_same_str("eof after one digit", "5");
    assert_same_str("eof after sign and digit", "-5");
    assert_same_str("eof after long run", &"1234567890".repeat(10));
}

/// Deterministic pseudo-random sweep over token kinds, separators and counts.
/// Uses a fixed seed so failures are reproducible.
#[test]
fn randomized_sweep() {
    // xorshift64*, so the test has no external dependency.
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = move || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state = state.wrapping_mul(0x2545_F491_4F6C_DD1D);
        state
    };

    let junk = [
        "abc", "-", "+", ".", "x", "0x1f", "1.5", "--3", "\u{b1}", "e", "_", ",",
    ];
    let seps = [" ", "\n", "\t", "\r\n", "  \n ", "\x0b", "\x0c", " \t "];
    let interesting: [i64; 12] = [
        0,
        1,
        -1,
        46340,
        46341,
        -46341,
        65536,
        2_147_483_647,
        -2_147_483_648,
        9_223_372_036_854_775_807,
        -9_223_372_036_854_775_808,
        4_294_967_296,
    ];

    for case in 0..400 {
        let count = (next() % 106) as usize;
        let mut tokens: Vec<String> = Vec::with_capacity(count);
        for _ in 0..count {
            match next() % 100 {
                0..=49 => {
                    let v = (next() as i64) % 4_294_967_296 - 2_147_483_648;
                    tokens.push(v.to_string());
                }
                50..=64 => {
                    tokens.push(interesting[(next() % interesting.len() as u64) as usize].to_string())
                }
                65..=74 => {
                    // Digit run longer than any integer type.
                    let len = 20 + (next() % 30) as usize;
                    let mut s = String::new();
                    if next() % 2 == 0 {
                        s.push('-');
                    }
                    for _ in 0..len {
                        s.push((b'0' + (next() % 10) as u8) as char);
                    }
                    tokens.push(s);
                }
                75..=84 => tokens.push(junk[(next() % junk.len() as u64) as usize].to_string()),
                _ => tokens.push(((next() % 201) as i64 - 100).to_string()),
            }
        }
        let sep = seps[(next() % seps.len() as u64) as usize];
        let input = tokens.join(sep);
        assert_same_str(&format!("random case {case}"), &input);
    }
}

/// Large input, to catch any buffering or flush difference between `printf`'s
/// stdio buffer and the Rust writer.
#[test]
fn large_input_output_is_fully_flushed() {
    // Values chosen so every printed line is a different width, including
    // negative results from the overflow.
    let items: Vec<String> = (0..100).map(|i| (i * 7_654_321 - 300_000_000).to_string()).collect();
    assert_same_str("wide value range", &items.join("\n"));
}
