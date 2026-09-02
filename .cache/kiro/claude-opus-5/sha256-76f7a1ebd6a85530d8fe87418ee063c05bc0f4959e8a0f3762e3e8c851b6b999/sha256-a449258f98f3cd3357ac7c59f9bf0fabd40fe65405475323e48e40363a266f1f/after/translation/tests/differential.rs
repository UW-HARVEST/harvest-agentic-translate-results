//! Differential tests: run the C `driver` and the Rust `driver` as
//! subprocesses, feed both the exact same bytes on stdin, and require that
//! stdout, stderr and exit status match byte for byte.
//!
//! The Rust code is never called as a library here; both programs are driven
//! the way a shell would drive them, because that is how they are compared.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Once;

/// Repository root: the directory holding both `c_src/` and `translation/`.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

fn c_binary() -> PathBuf {
    repo_root().join("c_src/build/driver")
}

/// Path to the Rust binary under test. Cargo puts integration-test executables
/// in `target/<profile>/deps/`, so the sibling binary lives two levels up.
fn rust_binary() -> PathBuf {
    let mut p = std::env::current_exe().expect("current_exe");
    p.pop(); // deps/
    if p.file_name().map(|n| n == "deps").unwrap_or(false) {
        p.pop(); // <profile>/
    }
    let candidate = p.join("driver");
    if candidate.exists() {
        return candidate;
    }
    // Fall back to the conventional locations.
    for profile in ["release", "debug"] {
        let c = repo_root().join("translation/target").join(profile).join("driver");
        if c.exists() {
            return c;
        }
    }
    panic!(
        "could not locate the Rust `driver` binary; run `cargo build --release` in translation/"
    );
}

/// Build the C program once per test binary, if it is not already built.
fn ensure_c_built() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        if c_binary().exists() {
            return;
        }
        let build_dir = repo_root().join("c_src/build");
        std::fs::create_dir_all(&build_dir).expect("create c_src/build");
        let configure = Command::new("cmake")
            .arg("..")
            .current_dir(&build_dir)
            .output()
            .expect("cmake must be installed to build the C reference program");
        assert!(
            configure.status.success(),
            "cmake configure failed:\n{}",
            String::from_utf8_lossy(&configure.stderr)
        );
        let build = Command::new("cmake")
            .args(["--build", "."])
            .current_dir(&build_dir)
            .output()
            .expect("cmake --build");
        assert!(
            build.status.success(),
            "cmake --build failed:\n{}",
            String::from_utf8_lossy(&build.stderr)
        );
    });
    assert!(
        c_binary().exists(),
        "C reference binary missing at {}",
        c_binary().display()
    );
}

struct Output {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Some(code)` for a normal exit, `None` if killed by a signal.
    code: Option<i32>,
}

fn run(program: &Path, stdin_bytes: &[u8]) -> Output {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", program.display()));

    {
        let mut stdin = child.stdin.take().expect("stdin pipe");
        // The child may exit before draining stdin; a broken pipe is not a
        // test failure, it is part of the observed behavior.
        let _ = stdin.write_all(stdin_bytes);
        let _ = stdin.flush();
    }

    let out = child.wait_with_output().expect("wait_with_output");
    Output {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
    }
}

fn show(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => format!("{s:?}"),
        Err(_) => format!("{bytes:?}"),
    }
}

/// Core assertion: identical stdout, stderr and exit status for one input.
#[track_caller]
fn assert_same(label: &str, stdin_bytes: &[u8]) {
    ensure_c_built();
    let c = run(&c_binary(), stdin_bytes);
    let r = run(&rust_binary(), stdin_bytes);

    assert_eq!(
        c.stdout,
        r.stdout,
        "[{label}] stdout differs\n  input: {}\n  C:     {}\n  Rust:  {}",
        show(stdin_bytes),
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "[{label}] stderr differs\n  input: {}\n  C:     {}\n  Rust:  {}",
        show(stdin_bytes),
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        c.code,
        r.code,
        "[{label}] exit status differs\n  input: {}\n  C:     {:?}\n  Rust:  {:?}",
        show(stdin_bytes),
        c.code,
        r.code
    );
}

// ---------------------------------------------------------------------------
// Branch inventory of c_src/src/main.c
//
//   main:
//     - loop `for (i = 0; i < 100; i++)`   -> 0, 1, .., 99, 100, >100 items
//     - `if (scanf("%d", &data[i]) != 1) break;` -> EOF break, match-failure break
//   call_fma:
//     - `if (len == 0) return 0;`          -> the empty-input path
//     - otherwise returns out[len-1]       -> the last successfully read value
//   fma_array:
//     - out[i] = ones[i] * data[i] + zeros[i] = data[i]  (signed int arithmetic)
//
// Everything below is derived from that inventory.
// ---------------------------------------------------------------------------

// --- call_fma: len == 0 (early return) -------------------------------------

#[test]
fn empty_input() {
    assert_same("empty input", b"");
}

#[test]
fn whitespace_only_inputs() {
    // scanf hits EOF while skipping whitespace -> returns EOF -> i stays 0.
    assert_same("single space", b" ");
    assert_same("spaces", b"     ");
    assert_same("newlines only", b"\n\n\n");
    assert_same("tabs only", b"\t\t");
    assert_same("mixed C whitespace", b" \t\n\x0b\x0c\r");
    assert_same("trailing newline only", b"\n");
}

#[test]
fn non_numeric_first_token_yields_zero() {
    // scanf matching failure on the very first call -> i == 0 -> call_fma
    // takes the `len == 0` branch.
    assert_same("letters", b"abc");
    assert_same("letters newline", b"abc\n");
    assert_same("punctuation", b"!!!");
    assert_same("leading ws then letters", b"   \n  xyz");
    assert_same("hex prefix", b"0x10");
    assert_same("underscore", b"_1");
    assert_same("comma", b",1");
}

// --- scanf: sign handling --------------------------------------------------

#[test]
fn sign_only_and_malformed_signs() {
    assert_same("minus then EOF", b"-");
    assert_same("plus then EOF", b"+");
    assert_same("minus then newline", b"-\n");
    assert_same("minus then letter", b"-x");
    assert_same("plus then letter", b"+x");
    assert_same("double minus", b"--5");
    assert_same("double plus", b"++5");
    assert_same("plus minus", b"+-5");
    assert_same("sign after value", b"5-");
    assert_same("space between sign and digits", b"- 5");
}

#[test]
fn signed_values() {
    assert_same("negative", b"-7");
    assert_same("explicit positive", b"+7");
    assert_same("negative zero", b"-0");
    assert_same("positive zero", b"+0");
    assert_same("mixed signs", b"-1 +2 -3");
    assert_same("signs across newlines", b"-1\n+2\n-3\n");
}

// --- main: exactly one item, and the last-value semantics ------------------

#[test]
fn single_item() {
    assert_same("single, no newline", b"5");
    assert_same("single, newline", b"5\n");
    assert_same("single zero", b"0");
    assert_same("single leading zeros", b"0000000005");
    assert_same("single, trailing spaces", b"5   ");
    assert_same("single, leading spaces", b"   5");
}

#[test]
fn result_is_the_last_value_read() {
    assert_same("two items", b"1 2");
    assert_same("three items", b"1 2 3");
    assert_same("last is negative", b"1 2 -3");
    assert_same("last is zero", b"1 2 0");
    assert_same("descending", b"9 8 7 6 5 4 3 2 1");
}

// --- scanf reads across newlines (unlike fgets) ---------------------------

#[test]
fn scanf_crosses_newlines_and_arbitrary_whitespace() {
    assert_same("one per line", b"1\n2\n3\n");
    assert_same("no trailing newline", b"1\n2\n3");
    assert_same("blank lines between", b"1\n\n\n2\n\n3\n");
    assert_same("tab separated", b"1\t2\t3");
    assert_same("vertical tab / form feed / CR", b"1\x0b2\x0c3\r4 5");
    assert_same("crlf line endings", b"1\r\n2\r\n3\r\n");
    assert_same("wildly mixed whitespace", b"  \n\t 1 \r\n\x0b 2 \x0c\n 3  \n ");
}

// --- main: the break on a matching failure part-way through ----------------

#[test]
fn stops_at_first_non_numeric_token() {
    // The loop breaks, so the answer is the last value read *before* the junk.
    assert_same("junk in the middle", b"1 2 x 3");
    assert_same("junk at the end", b"1 2 3 x");
    assert_same("junk immediately after digits", b"1 2 3x4");
    assert_same("float is truncated at the dot", b"1.5");
    assert_same("float mid-stream", b"1 2 3.7 4");
    assert_same("exponent form", b"1e5");
    assert_same("comma separated", b"1,2,3");
    assert_same("semicolon separated", b"10;20");
}

// --- main: the 100-element bound -----------------------------------------

#[test]
fn item_count_boundaries() {
    let nums = |n: usize| -> Vec<u8> {
        let mut s = String::new();
        for i in 1..=n {
            s.push_str(&i.to_string());
            s.push('\n');
        }
        s.into_bytes()
    };
    assert_same("1 item", &nums(1));
    assert_same("2 items", &nums(2));
    assert_same("98 items", &nums(98));
    assert_same("99 items", &nums(99));
    // Exactly the maximum the array holds.
    assert_same("100 items", &nums(100));
    // One more than the maximum: the loop condition stops it, stdin is left
    // with unread bytes and the program still exits 0.
    assert_same("101 items", &nums(101));
    assert_same("150 items", &nums(150));
    assert_same("200 items", &nums(200));
}

#[test]
fn one_hundred_items_space_separated_without_trailing_newline() {
    let mut s = String::new();
    for i in 0..100 {
        if i > 0 {
            s.push(' ');
        }
        s.push_str(&(i * 3 - 50).to_string());
    }
    assert_same("100 space separated", s.as_bytes());
}

#[test]
fn hundredth_value_is_the_answer_even_with_junk_after() {
    let mut s = String::new();
    for i in 1..=100 {
        s.push_str(&i.to_string());
        s.push(' ');
    }
    s.push_str("garbage 999");
    assert_same("100 items then junk", s.as_bytes());
}

// --- int range, truncation and signedness --------------------------------

#[test]
fn int_extremes() {
    assert_same("INT_MAX", b"2147483647");
    assert_same("INT_MIN", b"-2147483648");
    assert_same("INT_MAX-1", b"2147483646");
    assert_same("INT_MIN+1", b"-2147483647");
}

#[test]
fn values_beyond_int_are_truncated_as_c_does() {
    assert_same("INT_MAX+1", b"2147483648");
    assert_same("INT_MIN-1", b"-2147483649");
    assert_same("2^32", b"4294967296");
    assert_same("2^32+1", b"4294967297");
    assert_same("UINT_MAX", b"4294967295");
    assert_same("2^33", b"8589934592");
    assert_same("large mixed", b"1 2 4294967296");
}

#[test]
fn values_beyond_long_saturate_then_truncate() {
    assert_same("LONG_MAX", b"9223372036854775807");
    assert_same("LONG_MAX+1", b"9223372036854775808");
    assert_same("LONG_MIN", b"-9223372036854775808");
    assert_same("LONG_MIN-1", b"-9223372036854775809");
    assert_same("way past LONG_MAX", b"99999999999999999999999999");
    assert_same("way past LONG_MIN", b"-99999999999999999999999999");
    assert_same("huge with leading zeros", b"000000000009223372036854775808");
}

#[test]
fn very_long_digit_runs() {
    let mut s = vec![b'7'; 5000];
    s.push(b'\n');
    assert_same("5000 digits", &s);

    let mut neg = vec![b'-'];
    neg.extend(std::iter::repeat(b'9').take(3000));
    assert_same("3000 negative digits", &neg);

    let mut zeros = vec![b'0'; 4096];
    zeros.push(b'1');
    assert_same("4096 zeros then a 1", &zeros);

    // Pure zeros: never overflows, however long.
    assert_same("many zeros", &vec![b'0'; 1000]);
}

// --- binary / non-UTF-8 stdin -------------------------------------------

#[test]
fn binary_and_high_bytes() {
    assert_same("NUL terminates the token", b"1 2 3\x004");
    assert_same("leading NUL", b"\x005");
    assert_same("high bytes", b"\xff\xfe");
    assert_same("digits then high byte", b"42\xff");
    assert_same("invalid utf8 mid-stream", b"1 2 \xc3\x28 3");
    assert_same("all bytes 0..=255", &(0u8..=255).collect::<Vec<u8>>());
}

// --- a deterministic differential sweep ---------------------------------

#[test]
fn deterministic_random_sweep() {
    // xorshift64* so the corpus is reproducible without a dependency.
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = move || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state.wrapping_mul(0x2545_F491_4F6C_DD1D)
    };

    const ALPHABET: &[&str] = &[
        "0", "1", "-1", "7", "-7", "2147483647", "-2147483648", "2147483648",
        "-2147483649", "4294967296", "9223372036854775807", "-9223372036854775808",
        "99999999999999999999", "+5", "-0", "007", "x", "abc", ".", "-", "+", "1.5",
        "0x1f", ",", "1e9",
    ];
    const SEPARATORS: &[&str] = &[" ", "\n", "\t", "  ", "\r\n", "\x0b", "\x0c", ""];

    for case in 0..300 {
        let count = (next() % 106) as usize; // spans the 100-item boundary
        let mut input = String::new();
        for _ in 0..count {
            input.push_str(SEPARATORS[(next() % SEPARATORS.len() as u64) as usize]);
            input.push_str(ALPHABET[(next() % ALPHABET.len() as u64) as usize]);
        }
        if next() % 2 == 0 {
            input.push('\n');
        }
        assert_same(&format!("random case {case}"), input.as_bytes());
    }
}

// --- output shape -------------------------------------------------------

#[test]
fn output_is_exactly_the_number_and_one_newline() {
    ensure_c_built();
    for input in [&b""[..], b"5", b"-3", b"1 2 3"] {
        let c = run(&c_binary(), input);
        let r = run(&rust_binary(), input);
        // Sanity-check the reference shape too, so a silently broken C build
        // cannot make this suite vacuously pass.
        assert!(
            c.stdout.ends_with(b"\n") && !c.stdout[..c.stdout.len() - 1].contains(&b'\n'),
            "C stdout should be one line: {}",
            show(&c.stdout)
        );
        assert!(c.stderr.is_empty(), "C stderr should be empty");
        assert_eq!(c.code, Some(0), "C should exit 0");
        assert_eq!(r.stdout, c.stdout);
        assert_eq!(r.stderr, c.stderr);
        assert_eq!(r.code, c.code);
    }
}
