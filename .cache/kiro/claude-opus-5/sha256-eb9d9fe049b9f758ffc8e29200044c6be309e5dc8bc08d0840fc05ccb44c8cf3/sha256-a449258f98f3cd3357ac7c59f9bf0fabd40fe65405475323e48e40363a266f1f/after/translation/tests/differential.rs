//! Differential tests: run the original C program and the Rust translation as
//! subprocesses on the same stdin bytes and require byte-identical stdout,
//! byte-identical stderr, and the same exit status.
//!
//! Nothing here links against the Rust crate as a library. Both programs are
//! driven exactly the way a shell drives them.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

// ============================================================================
// Locating the two binaries
// ============================================================================

fn workspace_root() -> PathBuf {
    // tests/ live in translation/, whose parent holds c_src/ and translation/.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// The Rust program under test, built by cargo for this test binary.
fn rust_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// The C program, built with the CMake project in `c_src/`.
///
/// If it has not been built yet, build it here so the suite is self-contained.
fn c_binary() -> PathBuf {
    let c_src = workspace_root().join("c_src");
    let build = c_src.join("build");
    let bin = build.join("driver");
    if bin.is_file() {
        return bin;
    }

    std::fs::create_dir_all(&build).expect("cannot create c_src/build");

    let configure = Command::new("cmake")
        .arg("..")
        .current_dir(&build)
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
        .current_dir(&build)
        .output()
        .expect("failed to run `cmake --build .`");
    assert!(
        compile.status.success(),
        "cmake build failed:\n{}\n{}",
        String::from_utf8_lossy(&compile.stdout),
        String::from_utf8_lossy(&compile.stderr)
    );

    assert!(
        bin.is_file(),
        "the C build did not produce {}",
        bin.display()
    );
    bin
}

// ============================================================================
// Running a program
// ============================================================================

struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Ok(code)` for a normal exit, `Err(signal)` when killed by a signal.
    status: Result<i32, i32>,
}

fn run(bin: &Path, input: &[u8]) -> Run {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    // Feed stdin from a helper thread: the programs can emit far more than a
    // pipe buffer holds, so writing inline would deadlock.
    let mut stdin = child.stdin.take().expect("stdin was piped");
    let payload = input.to_vec();
    let writer = std::thread::spawn(move || {
        // A program that exits early (menu choice 7) closes the pipe while we
        // are still writing; EPIPE is expected and not a test failure.
        let _ = stdin.write_all(&payload);
        let _ = stdin.flush();
        drop(stdin);
    });

    let out = child.wait_with_output().expect("wait_with_output failed");
    writer.join().expect("stdin writer thread panicked");

    let status = match out.status.code() {
        Some(code) => Ok(code),
        None => {
            #[cfg(unix)]
            {
                use std::os::unix::process::ExitStatusExt;
                Err(out.status.signal().unwrap_or(-1))
            }
            #[cfg(not(unix))]
            {
                Err(-1)
            }
        }
    };

    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        status,
    }
}

// ============================================================================
// The assertion
// ============================================================================

fn show(bytes: &[u8]) -> String {
    // Readable rendering that still distinguishes non-UTF-8 and control bytes.
    let mut s = String::new();
    for &b in bytes {
        match b {
            b'\n' => s.push('\n'),
            b'\t' => s.push('\t'),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    s
}

fn first_diff(a: &[u8], b: &[u8]) -> String {
    let n = a.len().min(b.len());
    let at = (0..n).find(|&i| a[i] != b[i]).unwrap_or(n);
    let from = at.saturating_sub(60);
    let to_a = (at + 60).min(a.len());
    let to_b = (at + 60).min(b.len());
    format!(
        "first difference at byte {at} (C len {}, Rust len {})\n\
         C    ...{}\n\
         Rust ...{}",
        a.len(),
        b.len(),
        show(&a[from..to_a]),
        show(&b[from..to_b])
    )
}

/// Run both programs on `input` and require stdout, stderr and exit status to
/// match byte for byte.
#[track_caller]
fn assert_identical(case: &str, input: &[u8]) {
    let c = run(&c_binary(), input);
    let r = run(&rust_binary(), input);

    assert!(
        c.stdout == r.stdout,
        "[{case}] stdout differs\ninput = {:?}\n{}",
        show(input),
        first_diff(&c.stdout, &r.stdout)
    );
    assert!(
        c.stderr == r.stderr,
        "[{case}] stderr differs\ninput = {:?}\n{}",
        show(input),
        first_diff(&c.stderr, &r.stderr)
    );
    assert!(
        c.status == r.status,
        "[{case}] exit status differs: C {:?} vs Rust {:?}\ninput = {:?}",
        c.status,
        r.status,
        show(input)
    );
}

#[track_caller]
fn check(case: &str, input: &str) {
    assert_identical(case, input.as_bytes());
}

// ============================================================================
// Phase A — both programs are runnable and agree on the trivial input
// ============================================================================

#[test]
fn both_binaries_exist_and_run() {
    let c = c_binary();
    let r = rust_binary();
    assert!(c.is_file(), "C binary missing at {}", c.display());
    assert!(r.is_file(), "Rust binary missing at {}", r.display());

    // Sanity: the banner is produced even with no input at all.
    let out = run(&c, b"");
    assert!(!out.stdout.is_empty(), "C program produced no output");
    assert_identical("smoke", b"");
}

// ============================================================================
// Phase B — the branches main() takes
// ============================================================================

// `fgets` returns NULL immediately -> loop breaks -> return 0.
#[test]
fn empty_input_eof_immediately() {
    assert_identical("empty stdin (0 bytes)", b"");
}

#[test]
fn only_newlines_repeated_invalid_input() {
    // Each blank line is a sscanf matching failure: "Invalid input", loop again.
    check("single newline", "\n");
    check("two newlines", "\n\n");
    check("five newlines", "\n\n\n\n\n");
}

// One test per switch arm, i.e. per demo.
#[test]
fn each_menu_choice_alone() {
    check("choice 1 - integer containers", "1\n");
    check("choice 2 - double containers", "2\n");
    check("choice 3 - inventory array", "3\n");
    check("choice 4 - order list", "4\n");
    check("choice 5 - mixed operations", "5\n");
    check("choice 6 - run all demos", "6\n");
    check("choice 7 - exit", "7\n");
}

// `case 7` is the only `return` from main before EOF.
#[test]
fn choice_7_exits_and_ignores_the_rest_of_stdin() {
    check("7 then more input", "7\n1\n2\n3\n");
    check("7 with trailing garbage on the line", "7 exit now\n1\n");
}

// The `default:` arm.
#[test]
fn out_of_range_choices_hit_default_arm() {
    check("choice 0", "0\n");
    check("choice 8", "8\n");
    check("choice 9", "9\n");
    check("choice -1", "-1\n");
    check("choice 100", "100\n");
    check("choice -2147483648", "-2147483648\n");
}

// The `sscanf(...) != 1` arm.
#[test]
fn non_numeric_input_is_invalid_input() {
    check("letters", "abc\n");
    check("punctuation", "!!!\n");
    check("single space", " \n");
    check("tab only", "\t\n");
    check("lone minus", "-\n");
    check("lone plus", "+\n");
    check("dot", ".\n");
}

// ============================================================================
// Phase B — scanf conversion details
// ============================================================================

#[test]
fn leading_whitespace_is_skipped_by_scanf() {
    check("spaces then 3", "   3\n");
    check("tab then 4", "\t4\n");
    check("mixed C whitespace then 4", "\t\u{b}\u{c} 4\n");
    check("newline-only line then 7", "\n7\n");
}

#[test]
fn trailing_garbage_after_the_number_is_ignored() {
    check("3abc", "3abc\n");
    check("3 4 5", "3 4 5\n");
    check("spaces around 3", "  3  \n");
    check("1;drop", "1;drop\n");
}

#[test]
fn signs_are_accepted() {
    check("+5", "+5\n");
    check("+7", "+7\n");
    check("-0", "-0\n");
    check("leading zeros", "007\n");
}

#[test]
fn scanf_numeric_prefix_forms() {
    // "%d" is decimal only: 0x3 converts as 0, 3.9 as 3, 1e3 as 1.
    check("0x3", "0x3\n");
    check("3.9", "3.9\n");
    check(".5", ".5\n");
    check("1e3", "1e3\n");
    check("1_000", "1_000\n");
}

// Overflow: glibc converts as `long` (saturating) and truncates to `int`.
#[test]
fn integer_overflow_truncation_and_signedness() {
    check("int max", "2147483647\n");
    check("int max + 1", "2147483648\n");
    check("int min", "-2147483648\n");
    check("int min - 1", "-2147483649\n");
    check("2^32", "4294967296\n");
    check("2^32 + 7", "4294967303\n");
    check("long max", "9223372036854775807\n");
    check("long max + 1", "9223372036854775808\n");
    check("long min", "-9223372036854775808\n");
    check("far past long range", "99999999999999999999999999\n");
    check("400 nines", &format!("{}\n", "9".repeat(400)));
}

// ============================================================================
// Phase C — input classes no earlier test reaches
// ============================================================================

// `fgets` does not read across newlines, and it stops after 255 bytes; the
// remainder of an over-long line is handed to the *next* iteration.
#[test]
fn fgets_splits_lines_longer_than_the_255_byte_buffer() {
    // 254 filler + "17\n": the first read ends at '1', the second yields "7\n".
    assert_identical(
        "254 spaces then 17 - split makes a 1 then a 7",
        format!("{}17\n", " ".repeat(254)).as_bytes(),
    );
    // First chunk has no digits; the tail "7\n" then exits.
    assert_identical(
        "255 spaces then 7",
        format!("{}7\n", " ".repeat(255)).as_bytes(),
    );
    // First chunk is "x"*254 + "7"; the tail is just "\n".
    assert_identical(
        "254 x then 7",
        format!("{}7\n", "x".repeat(254)).as_bytes(),
    );
    // Exactly 255 bytes with a leading 6, then a separate "7" line.
    assert_identical(
        "exactly 255 bytes then 7",
        format!("6{}\n7\n", " ".repeat(254)).as_bytes(),
    );
    // A 300-byte line starting with '1'.
    assert_identical(
        "300-byte line starting with 1",
        format!("1{}\n", "a".repeat(299)).as_bytes(),
    );
    // A digit run split across the buffer boundary.
    assert_identical(
        "600 digits, no newline until the end",
        format!("{}\n", "1234567890".repeat(60)).as_bytes(),
    );
}

// EOF with bytes already in the buffer: fgets returns the partial line, then
// the following call returns NULL.
#[test]
fn input_without_a_trailing_newline() {
    check("bare 3, no newline", "3");
    check("bare 7, no newline", "7");
    check("bare 8, no newline", "8");
    check("bare abc, no newline", "abc");
    check("6 then bare 1", "6\n1");
}

// NUL bytes terminate the C string that sscanf sees, even though fgets copied
// the bytes after them.
#[test]
fn embedded_nul_bytes() {
    assert_identical("leading NUL then 5", b"\x005\n");
    assert_identical("digit then NUL then digit", b"5\x006\n");
    assert_identical("NUL only", b"\x00\n");
    assert_identical("7 after a NUL line", b"\x00abc\n7\n");
}

// The program never echoes input, so non-UTF-8 bytes must simply be tolerated.
#[test]
fn non_utf8_input_bytes() {
    assert_identical("lone continuation byte", b"\x80\n7\n");
    assert_identical("invalid utf8 pair", b"\xff\xfe\n7\n");
    assert_identical("utf8 digit lookalike", "３\n7\n".as_bytes());
    assert_identical("all high bytes", b"\xc3\x28\xa0\xa1\n8\n");
}

#[test]
fn carriage_returns_and_dos_line_endings() {
    check("crlf 3 then crlf 7", "3\r\n7\r\n");
    check("cr only", "3\r7\r");
    check("cr before digit", "\r3\n");
}

// Several iterations of the loop, mixing every arm.
#[test]
fn full_menu_walk_through_every_arm() {
    check("1..6 then exit", "1\n2\n3\n4\n5\n6\n7\n");
    check(
        "every arm including the invalid ones",
        "1\n2\n3\n4\n5\n0\n8\nabc\n\n-1\n6\n7\n",
    );
    check("no exit, run to EOF", "1\n2\n3\n4\n5\n6\n");
    check("repeated demo 6", "6\n6\n6\n6\n7\n");
}

// Enough output to cross stdio's block-buffer boundary many times over.
#[test]
fn large_output_crosses_stdio_buffer_boundaries() {
    let mut input = String::new();
    for _ in 0..25 {
        input.push_str("6\n");
    }
    input.push_str("7\n");
    assert_identical("25 runs of demo 6 then exit", input.as_bytes());
}

// Many loop iterations that produce almost no output.
#[test]
fn many_invalid_iterations() {
    let input = "z\n".repeat(500);
    assert_identical("500 invalid lines", input.as_bytes());

    let input = "0\n".repeat(500);
    assert_identical("500 invalid choices", input.as_bytes());
}

// A pathological mix, to pin the interleaving of prompt, demo and error text.
#[test]
fn interleaved_valid_and_invalid_input() {
    check(
        "alternating valid and invalid",
        "1\nbad\n2\n\n3\n0\n4\n-5\n5\nxyz\n6\n\n7\n",
    );
    check("invalid before exit", "8\n9\n10\n7\n");
    check("choice 7 last after everything", "5\n5\n5\n7\n");
}
