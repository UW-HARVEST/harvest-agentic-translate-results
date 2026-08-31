//! Differential tests: run the C binary and the Rust binary as subprocesses,
//! feed both the exact same bytes on stdin, and require that stdout, stderr and
//! the exit status match byte for byte.
//!
//! Nothing here links the Rust code as a library; both programs are driven the
//! way a shell drives them, because that is how they are compared.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// Path to the Rust binary. Cargo builds it before the test runs.
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Path to the C binary, building it with CMake on first use if necessary.
fn c_bin() -> &'static PathBuf {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_src = manifest
            .parent()
            .expect("translation/ must have a parent directory")
            .join("c_src");
        assert!(
            c_src.join("CMakeLists.txt").is_file(),
            "cannot find c_src/CMakeLists.txt next to the Rust crate (looked in {})",
            c_src.display()
        );

        let build = c_src.join("build");
        let exe = build.join("driver");
        if exe.is_file() {
            return exe;
        }

        std::fs::create_dir_all(&build).expect("create c_src/build");
        run_tool(Command::new("cmake").arg("..").current_dir(&build), "cmake configure");
        run_tool(
            Command::new("cmake").args(["--build", "."]).current_dir(&build),
            "cmake build",
        );
        assert!(
            exe.is_file(),
            "C build finished but {} does not exist",
            exe.display()
        );
        exe
    })
}

fn run_tool(cmd: &mut Command, what: &str) {
    let out = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn {what}: {e}"));
    assert!(
        out.status.success(),
        "{what} failed ({})\n--- stdout ---\n{}\n--- stderr ---\n{}",
        out.status,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

/// What one program produced for one input.
struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    status: Option<i32>,
}

fn run(bin: &Path, stdin_bytes: &[u8]) -> Run {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    // Write on a helper thread so a large input cannot deadlock against the
    // child filling its stdout pipe.
    let mut sink = child.stdin.take().expect("piped stdin");
    let payload = stdin_bytes.to_vec();
    let writer = std::thread::spawn(move || {
        let _ = sink.write_all(&payload);
        let _ = sink.flush();
        drop(sink);
    });

    let out = child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("failed to wait on {}: {e}", bin.display()));
    writer.join().expect("stdin writer thread panicked");

    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        status: out.status.code(),
    }
}

fn show(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            0x0b => s.push_str("\\v"),
            0x0c => s.push_str("\\f"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    s
}

fn label(input: &[u8]) -> String {
    if input.len() <= 80 {
        format!("\"{}\"", show(input))
    } else {
        format!(
            "\"{}\"... ({} bytes total)",
            show(&input[..60]),
            input.len()
        )
    }
}

/// Assert the C and Rust programs agree on stdout, stderr and exit status.
fn assert_same(input: &[u8]) {
    let c = run(c_bin(), input);
    let r = run(&rust_bin(), input);

    let desc = label(input);
    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch for input {desc}\n  C   : \"{}\"\n  Rust: \"{}\"",
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch for input {desc}\n  C   : \"{}\"\n  Rust: \"{}\"",
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        c.status, r.status,
        "exit status mismatch for input {desc}\n  C   : {:?}\n  Rust: {:?}",
        c.status, r.status
    );
}

fn check(inputs: &[&str]) {
    for s in inputs {
        assert_same(s.as_bytes());
    }
}

// ---------------------------------------------------------------------------
// Phase A: both binaries exist and are runnable.
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_run() {
    let c = run(c_bin(), b"5 3\n");
    let r = run(&rust_bin(), b"5 3\n");
    // 5 | ~3 == 5 | -4 == -3
    assert_eq!(c.stdout, b"-3\n", "C reference output changed unexpectedly");
    assert_eq!(r.stdout, c.stdout);
    assert_eq!(c.status, Some(0));
    assert_eq!(r.status, Some(0));
}

// ---------------------------------------------------------------------------
// Phase B: the input classes the C program branches on.
//
// main() is: x = y = 0; scanf("%d",&x); scanf("%d",&y); print x | ~y.
// The branching lives inside the two %d conversions, so the input classes are
// the states a %d conversion can end in:
//   - input failure at EOF before any character  (variable keeps its 0)
//   - matching failure on a non-numeric character (variable keeps its 0)
//   - success, with/without sign, with/without overflow of long, then
//     truncation of long -> int
// plus the whitespace-skipping that lets a conversion cross newlines.
// ---------------------------------------------------------------------------

#[test]
fn empty_input_both_conversions_fail() {
    // Both scanfs see EOF immediately: x = y = 0, so 0 | ~0 == -1.
    assert_same(b"");
}

#[test]
fn whitespace_only_input() {
    check(&[
        "\n",
        "\n\n\n",
        " ",
        "   \t  ",
        "\t\n\x0b\x0c\r ",
        "\n\n\n\n\n\n\n\n\n\n",
    ]);
}

#[test]
fn single_item_second_conversion_hits_eof() {
    check(&["5", "5\n", "  5  ", "-5", "+5", "0", "-0", "2147483647", "-2147483648"]);
}

#[test]
fn two_items_happy_path() {
    check(&[
        "5 3",
        "5 3\n",
        "0 0",
        "1 1",
        "-1 -1",
        "0 -1",
        "-1 0",
        "7 9",
        "999999999 999999999",
        "305419896 2018915346",
    ]);
}

#[test]
fn scanf_reads_across_newlines() {
    // %d skips *all* leading whitespace, newlines included, so these are the
    // same two numbers no matter how they are laid out. (fgets would not.)
    check(&[
        "5\n3\n",
        "5\n\n\n\n3",
        "\n\n5\n\n3\n\n",
        "5\n3",
        "\n \t\n 42\n \t\n 7\n",
        "\n\n\n\n\n\n\n\n7\n\n\n\n\n\n9",
    ]);
}

#[test]
fn every_c_whitespace_character_separates() {
    check(&[
        "5 3", "5\t3", "5\n3", "5\x0b3", "5\x0c3", "5\r3",
        " \t\n\x0b\x0c\r \t\n\x0b\x0c\r-12 \t\n\x0b\x0c\r-34 \t\n",
        "11\r\n22\r\n",
    ]);
}

#[test]
fn signs_are_accepted() {
    check(&["+5 +3", "-5 -3", "+5 -3", "-5 +3", "+0 -0", "-0 +0"]);
}

#[test]
fn leading_zeros_are_decimal_not_octal() {
    // %d is base 10, so "008" is 8 and not an invalid octal literal.
    check(&["007 008", "0000000000000000 0000000000000001", "00 00"]);
}

// --- matching-failure paths: the variable keeps its initial 0 ---------------

#[test]
fn non_numeric_input_is_a_matching_failure() {
    check(&[
        "abc",
        "abc def",
        "x",
        "hello world",
        ",",
        "1,2",
        ".",
        "/",
        ":",
        "@",
    ]);
}

#[test]
fn conversion_stops_at_first_non_digit() {
    // x converts, then the pushed-back character makes the second scanf fail.
    check(&[
        "12abc",
        "0x10",
        "5.7 3.2",
        "1e5 2",
        "1_000 2",
        "1 2junk",
        "42/7",
        "3-4",
    ]);
}

#[test]
fn bare_and_doubled_signs() {
    // A lone sign is a matching failure. glibc pushes back only the one
    // offending character, not the sign, which is why "--5" gives y = -5.
    check(&["-", "+", "-\n", "+\n", "- ", "-  ", "- 5", "+ 5", "5 -", "5 +",
            "5 - 3", "--5", "++5", "+-5", "-+5", "--", "-+"]);
}

#[test]
fn extra_input_after_the_second_number_is_ignored() {
    check(&["5 3 7", "5 3 7 9 11", "1 2\nignored text\n", "1 2 abc"]);
}

// --- long accumulation, saturation and truncation to int -------------------

#[test]
fn values_above_int_max_truncate() {
    check(&[
        "2147483648 0",       // INT_MAX + 1
        "2147483649 0",
        "4294967295 0",       // 2^32 - 1  -> -1
        "4294967296 0",       // 2^32      -> 0
        "4294967301 7",
        "8589934592 0",       // 2^33      -> 0
        "0 2147483648",
        "0 4294967296",
    ]);
}

#[test]
fn long_saturation_boundary() {
    // glibc converts with strtol, which saturates at LONG_MAX / LONG_MIN, and
    // the saturated long is then truncated when stored into an int.
    check(&[
        "9223372036854775806 0",  // LONG_MAX - 1
        "9223372036854775807 0",  // LONG_MAX
        "9223372036854775808 0",  // LONG_MAX + 1 -> saturates
        "92233720368547758070 0",
        "-9223372036854775807 0", // LONG_MIN + 1
        "-9223372036854775808 0", // LONG_MIN
        "-9223372036854775809 0", // LONG_MIN - 1 -> saturates
        "-92233720368547758080 0",
        "0000000000009223372036854775807 0",
        "0000000000009223372036854775808 0",
    ]);
}

#[test]
fn far_beyond_long_range() {
    check(&[
        "99999999999999999999999999 0",
        "-99999999999999999999999999 0",
        "0 9223372036854775808",
        "0 -99999999999999999999",
        "99999999999999999999 99999999999999999999",
        "-99999999999999999999 -99999999999999999999",
    ]);
}

#[test]
fn digit_length_staircase() {
    // Walk the truncation/saturation staircase one digit at a time for both
    // signs; this crosses INT_MAX, UINT_MAX and LONG_MAX.
    for n in 1..=40 {
        assert_same(format!("{} 0", "1".repeat(n)).as_bytes());
        assert_same(format!("{} 0", "9".repeat(n)).as_bytes());
        assert_same(format!("-{} 0", "7".repeat(n)).as_bytes());
        assert_same(format!("0 {}", "1".repeat(n)).as_bytes());
        assert_same(format!("0 -{}", "8".repeat(n)).as_bytes());
    }
}

// --- the printed value itself ---------------------------------------------

#[test]
fn output_covers_int_min_int_max_and_zero() {
    // printf("%d") must render the full int range, INT_MIN included, followed
    // by exactly one newline from puts("").
    check(&[
        "-2147483648 2147483647", // INT_MIN | ~INT_MAX == INT_MIN
        "0 -2147483648",          // 0 | ~INT_MIN       == INT_MAX
        "2147483647 0",
        "-2147483648 -2147483648",
        "2147483647 2147483647",
        "1 2147483647",
        "0 1",
        "1431655765 1431655765",
        "-1431655766 715827882",
    ]);
}

#[test]
fn output_is_exactly_the_number_and_one_newline() {
    let out = run(c_bin(), b"5 3").stdout;
    assert_eq!(out, b"-3\n", "C output shape changed");
    assert_eq!(run(&rust_bin(), b"5 3").stdout, out);
    // no trailing space, no second newline
    assert!(!out.ends_with(b"\n\n"));
    assert!(!out.contains(&b' '));
}

// ---------------------------------------------------------------------------
// Phase C: input classes not covered above -- raw bytes, NULs, non-ASCII,
// huge inputs, and non-regular stdin.
// ---------------------------------------------------------------------------

#[test]
fn nul_bytes_in_input() {
    // NUL is not whitespace and not a digit, so it is a matching failure.
    assert_same(b"\x00");
    assert_same(b"\x00\x00\x00");
    assert_same(b"\x005 3");
    assert_same(b"5\x003");
    assert_same(b"5 \x00 3");
    assert_same(b"5\x00");
    assert_same(&vec![0u8; 4096]);
}

#[test]
fn high_and_non_utf8_bytes() {
    // Bytes above 0x7f are neither isspace nor isdigit in the C locale; 0xa0
    // (non-breaking space in Latin-1) in particular must NOT be skipped.
    assert_same(b"\xff\xfe 5 3");
    assert_same(b"5 \xff 3");
    assert_same(b"\x80\x81\x82");
    assert_same(b"\xa05 3");
    assert_same(b"5\xa03");
    assert_same("€ 4 5".as_bytes());
    assert_same("5 € 3".as_bytes());
    assert_same(b"\xc3\x28 1 2"); // invalid UTF-8
}

#[test]
fn very_large_inputs() {
    // Long digit runs exercise glibc's growing conversion buffer against the
    // Rust accumulator's saturation.
    assert_same(format!("{} 1", "9".repeat(100_000)).as_bytes());
    assert_same(format!("-{} 1", "3".repeat(65_536)).as_bytes());
    assert_same(format!("{}7 1", "0".repeat(100_000)).as_bytes());
    assert_same(format!("{}7 1", " ".repeat(100_000)).as_bytes());
    assert_same(format!("{}7 1", "\n".repeat(100_000)).as_bytes());
    assert_same(format!("{} {}", "9".repeat(50_000), "8".repeat(50_000)).as_bytes());
}

#[test]
fn command_line_arguments_are_ignored() {
    // C's main() takes no parameters, so argv cannot change anything.
    let c = Command::new(c_bin())
        .args(["a", "b", "-3", "--help"])
        .stdin(Stdio::null())
        .output()
        .expect("run C with args");
    let r = Command::new(rust_bin())
        .args(["a", "b", "-3", "--help"])
        .stdin(Stdio::null())
        .output()
        .expect("run Rust with args");
    assert_eq!(c.stdout, r.stdout);
    assert_eq!(c.stderr, r.stderr);
    assert_eq!(c.status.code(), r.status.code());
}

#[test]
fn empty_stdin_from_dev_null() {
    let c = Command::new(c_bin())
        .stdin(Stdio::null())
        .output()
        .expect("run C");
    let r = Command::new(rust_bin())
        .stdin(Stdio::null())
        .output()
        .expect("run Rust");
    assert_eq!(c.stdout, r.stdout);
    assert_eq!(c.stderr, r.stderr);
    assert_eq!(c.status.code(), r.status.code());
    assert_eq!(c.stdout, b"-1\n");
}

#[test]
fn randomized_differential_fuzz() {
    // A deterministic xorshift walk over the interesting byte alphabet, so the
    // suite reproduces exactly while still mixing classes the hand cases keep
    // separate (signs, digits, whitespace, junk, NUL, high bytes).
    const ALPHABET: &[u8] = b"0123456789      \n\n\t-+-+.,axX\x00\xff\x0b\r*/";
    let mut state: u64 = 0x2545_f491_4f6c_dd1d;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    for case in 0..400u32 {
        let len = (next() % 24) as usize;
        let mut input = Vec::with_capacity(len);
        for _ in 0..len {
            input.push(ALPHABET[(next() % ALPHABET.len() as u64) as usize]);
        }
        // Occasionally splice in a long digit run to reach the overflow paths.
        if case % 37 == 0 {
            let run = (next() % 30) as usize + 1;
            for _ in 0..run {
                input.push(b'0' + (next() % 10) as u8);
            }
        }
        assert_same(&input);
    }
}
