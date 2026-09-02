//! Differential tests: run the original C executable and the Rust executable as
//! subprocesses with identical stdin and require byte-identical stdout, stderr
//! and exit status.
//!
//! Nothing here links against the Rust crate as a library. Both programs are
//! driven exactly the way a shell drives them, because that is how the
//! translation is graded.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;

/// Directory holding both subtrees (`c_src/` and `translation/`).
fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the Rust binary under test, provided by Cargo for integration tests.
fn rust_bin() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// Path to the compiled C reference binary, building it on first use.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = workspace_root().join("c_src");
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
            .expect("failed to spawn cmake (is cmake installed?)");
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
            .expect("failed to spawn cmake --build");
        assert!(
            compile.status.success(),
            "cmake --build failed:\n{}\n{}",
            String::from_utf8_lossy(&compile.stdout),
            String::from_utf8_lossy(&compile.stderr),
        );

        assert!(
            exe.is_file(),
            "expected the C reference executable at {}",
            exe.display()
        );
        exe
    })
}

/// Feed `stdin` to `program` and capture everything it produced.
fn run(program: &Path, stdin_bytes: &[u8]) -> Output {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", program.display()));

    {
        let mut sink = child.stdin.take().expect("stdin was piped");
        // A program is free to exit without draining stdin; a broken pipe here
        // is not a test failure.
        match sink.write_all(stdin_bytes) {
            Ok(()) => {
                let _ = sink.flush();
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::BrokenPipe => {}
            Err(e) => panic!("writing stdin to {}: {e}", program.display()),
        }
    }

    child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("waiting on {}: {e}", program.display()))
}

fn describe(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) if s.chars().all(|c| !c.is_control() || c == '\n' || c == '\t') => {
            format!("{s:?}")
        }
        _ => format!("{bytes:02x?}"),
    }
}

/// The single assertion every case funnels through: stdout, stderr and exit
/// status must all agree.
#[track_caller]
fn assert_same(label: &str, stdin_bytes: &[u8]) {
    let c = run(c_bin(), stdin_bytes);
    let r = run(rust_bin(), stdin_bytes);

    assert_eq!(
        c.stdout,
        r.stdout,
        "[{label}] stdout mismatch for stdin {}\n  C   : {}\n  Rust: {}",
        describe(stdin_bytes),
        describe(&c.stdout),
        describe(&r.stdout),
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "[{label}] stderr mismatch for stdin {}\n  C   : {}\n  Rust: {}",
        describe(stdin_bytes),
        describe(&c.stderr),
        describe(&r.stderr),
    );
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "[{label}] exit code mismatch for stdin {}\n  C   : {:?}\n  Rust: {:?}",
        describe(stdin_bytes),
        c.status,
        r.status,
    );
    assert_eq!(
        c.status.success(),
        r.status.success(),
        "[{label}] exit status mismatch for stdin {}",
        describe(stdin_bytes),
    );
}

#[track_caller]
fn assert_same_str(label: &str, stdin_text: &str) {
    assert_same(label, stdin_text.as_bytes());
}

// ---------------------------------------------------------------------------
// Phase A sanity: both binaries exist and are runnable.
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_run() {
    let c = run(c_bin(), b"1\n");
    let r = run(rust_bin(), b"1\n");
    assert!(c.status.success(), "C reference did not exit successfully");
    assert!(r.status.success(), "Rust binary did not exit successfully");
    // Guard against a vacuous suite: the good path must actually print.
    assert_eq!(c.stdout, b"helperGood1 string\n");
    assert_eq!(r.stdout, c.stdout);
}

// ---------------------------------------------------------------------------
// The two branches of `if (x)` in main().
// ---------------------------------------------------------------------------

/// `x == 0` selects `bad()`, whose `helperBad()` returns the address of a
/// stack array. The reference compiler substitutes a null pointer for that
/// return value, so `printLine`'s NULL guard suppresses all output.
#[test]
fn zero_takes_the_bad_branch() {
    assert_same_str("zero", "0");
    assert_same_str("zero_newline", "0\n");
    assert_same_str("negative_zero", "-0");
    assert_same_str("plus_zero", "+0");
    assert_same_str("many_zeros", "0000");
    assert_same_str("zero_then_more", "0 1");
    assert_same_str("zero_then_junk", "0abc");
}

/// Any nonzero `x` selects `good()`, which returns a pointer to a `static`
/// buffer and therefore prints.
#[test]
fn nonzero_takes_the_good_branch() {
    assert_same_str("one", "1");
    assert_same_str("one_newline", "1\n");
    assert_same_str("minus_one", "-1");
    assert_same_str("plus_seven", "+7");
    assert_same_str("leading_zeros", "007");
    assert_same_str("int_max", "2147483647");
    assert_same_str("int_min", "-2147483648");
    assert_same_str("large_positive", "123456789");
}

// ---------------------------------------------------------------------------
// `scanf("%d", &x)` failure modes. On both input failure (EOF first) and
// matching failure (no digits) C leaves `x` at its initializer 0, so every
// case below must take the bad branch.
// ---------------------------------------------------------------------------

#[test]
fn empty_input_leaves_x_at_zero() {
    assert_same("empty", b"");
}

#[test]
fn whitespace_only_input_is_an_input_failure() {
    assert_same_str("single_space", " ");
    assert_same_str("single_newline", "\n");
    assert_same_str("single_tab", "\t");
    assert_same_str("vertical_tab", "\x0b");
    assert_same_str("form_feed", "\x0c");
    assert_same_str("carriage_return", "\r");
    assert_same_str("all_whitespace", " \t\n\x0b\x0c\r ");
    assert_same_str("many_newlines", &"\n".repeat(1000));
    assert_same_str("many_spaces", &" ".repeat(4096));
}

#[test]
fn non_numeric_input_is_a_matching_failure() {
    assert_same_str("letters", "abc");
    assert_same_str("single_letter", "z");
    assert_same_str("leading_ws_letter", "  x");
    assert_same_str("dot_five", ".5");
    assert_same_str("exponent_only", "e5");
    assert_same_str("comma", ",");
    assert_same_str("hash", "#42");
    assert_same_str("underscore", "_1");
    assert_same_str("hex_prefix_upper", "X10");
}

#[test]
fn sign_without_digits_is_a_matching_failure() {
    assert_same_str("minus_only", "-");
    assert_same_str("plus_only", "+");
    assert_same_str("minus_eof_after_ws", "   -");
    assert_same_str("minus_space_digit", "- 5");
    assert_same_str("plus_space_digit", "+ 9");
    assert_same_str("double_minus", "--9");
    assert_same_str("plus_minus", "+-3");
    assert_same_str("minus_newline_digit", "-\n5");
    assert_same_str("minus_letter", "-a");
}

// ---------------------------------------------------------------------------
// `scanf` skips whitespace across line boundaries -- unlike `fgets`, the
// newline is not a terminator.
// ---------------------------------------------------------------------------

#[test]
fn scanf_reads_across_newlines() {
    assert_same_str("newline_then_value", "\n5");
    assert_same_str("blank_lines_then_value", "\n\n\n\n7");
    assert_same_str("mixed_ws_then_value", "  \n\t\n 42  extra");
    assert_same_str("crlf_then_value", "\r\n3\r\n");
    assert_same_str("ws_then_zero", "\n\n   0\n\n");
    assert_same_str("all_ws_forms_then_value", "\t\x0b\x0c\r 8");
}

/// The conversion stops at the first character that cannot extend the number;
/// trailing input is simply never read.
#[test]
fn conversion_stops_at_first_non_digit() {
    assert_same_str("digit_then_letters", "1abc");
    assert_same_str("hex_literal", "0x10");
    assert_same_str("float", "3.14");
    assert_same_str("float_zero", "0.9");
    assert_same_str("digit_then_minus", "5-3");
    assert_same_str("two_numbers", "12 34");
    assert_same_str("digit_then_newline_junk", "6\nnot a number\n");
}

// ---------------------------------------------------------------------------
// Overflow, truncation and signedness, performed the way the C does: the
// conversion saturates at the `long` range and the assignment to `int x`
// truncates. Values whose low 32 bits are zero therefore take the bad branch
// even though they were written as nonzero.
// ---------------------------------------------------------------------------

#[test]
fn values_truncating_to_zero_take_the_bad_branch() {
    assert_same_str("two_pow_32", "4294967296");
    assert_same_str("three_times_two_pow_32", "12884901888");
    assert_same_str("neg_two_pow_32", "-4294967296");
    assert_same_str("high_word_only", "9223372032559808512");
    assert_same_str("long_min_magnitude", "-9223372036854775808");
    // Saturation at LONG_MIN leaves low 32 bits zero.
    assert_same_str("negative_overflow", &format!("-{}", "9".repeat(40)));
}

#[test]
fn values_truncating_to_nonzero_take_the_good_branch() {
    assert_same_str("two_pow_31", "2147483648");
    assert_same_str("uint_max", "4294967295");
    assert_same_str("just_past_int_min", "-2147483649");
    assert_same_str("long_max", "9223372036854775807");
    assert_same_str("past_long_max", "9223372036854775808");
    assert_same_str("two_pow_64", "18446744073709551616");
    // Saturation at LONG_MAX truncates to -1.
    assert_same_str("positive_overflow", &"9".repeat(40));
}

#[test]
fn very_long_digit_runs() {
    assert_same_str("5000_zeros", &"0".repeat(5000));
    assert_same_str("5000_nines", &"9".repeat(5000));
    assert_same_str("neg_5000_nines", &format!("-{}", "9".repeat(5000)));
    assert_same_str("zeros_then_one", &format!("{}1", "0".repeat(5000)));
    assert_same_str("one_then_zeros", &format!("1{}", "0".repeat(5000)));
    assert_same_str("huge_padded", &format!("{}{}", " ".repeat(2000), "7".repeat(2000)));
}

// ---------------------------------------------------------------------------
// Arbitrary bytes: NUL and non-ASCII are ordinary non-digits to `scanf`.
// ---------------------------------------------------------------------------

#[test]
fn non_text_bytes() {
    assert_same("nul_only", b"\x00");
    assert_same("nul_then_digit", b"\x005");
    assert_same("digit_then_nul", b"5\x00");
    assert_same("nuls_then_digit", b"\x00\x005");
    assert_same("high_bytes", b"\xff\xfe");
    assert_same("utf8_e_acute", "é".as_bytes());
    assert_same("digit_after_high_byte", b"\xff1");
    assert_same("ws_nul_digit", b" \x00 1");
    assert_same("all_bytes", &(0u8..=255).collect::<Vec<u8>>());
}

/// Every single byte on its own, so no one-character input class is missed.
#[test]
fn every_single_byte_input() {
    for b in 0u8..=255 {
        assert_same(&format!("byte_{b:#04x}"), &[b]);
    }
}

// ---------------------------------------------------------------------------
// Broad sweeps.
// ---------------------------------------------------------------------------

#[test]
fn small_integers_both_signs() {
    for v in -40i64..=40 {
        assert_same_str(&format!("int_{v}"), &v.to_string());
        assert_same_str(&format!("int_{v}_nl"), &format!("{v}\n"));
    }
}

#[test]
fn powers_of_two_and_neighbours() {
    for shift in 0..66u32 {
        let base: i128 = 1i128 << shift;
        for delta in [-1i128, 0, 1] {
            let v = base + delta;
            assert_same_str(&format!("pow2_{shift}_{delta}"), &v.to_string());
            assert_same_str(&format!("pow2_{shift}_{delta}_neg"), &format!("-{v}"));
        }
    }
}

/// Deterministic pseudo-random byte soup, to catch anything the hand-written
/// classes missed. The generator is a fixed LCG so failures are reproducible.
#[test]
fn randomized_byte_soup() {
    // Alphabet weighted toward the characters the scanner actually branches on.
    const ALPHABET: &[u8] = b"0123456789   \n\n\t+-+-abcxX.,\0\x0b\x0c\r\xff";
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = move || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (state >> 33) as u32
    };

    for case in 0..400 {
        let len = (next() % 24) as usize;
        let input: Vec<u8> = (0..len)
            .map(|_| ALPHABET[(next() as usize) % ALPHABET.len()])
            .collect();
        assert_same(&format!("soup_{case}"), &input);
    }
}
