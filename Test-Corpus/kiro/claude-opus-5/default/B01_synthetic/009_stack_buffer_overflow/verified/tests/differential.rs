//! Differential tests: run the C program and the Rust program as subprocesses
//! with identical stdin and require byte-identical stdout, byte-identical
//! stderr, and the same exit status (including termination by signal).
//!
//! Nothing here loads the Rust code as a library; both sides are driven as
//! executables, the way the translation is graded.
//!
//! The C binary is built on demand with the project's own CMakeLists, so
//! `cargo test` is self-contained.

use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// locating and building the two binaries
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// Builds `c_src` with cmake the first time it is needed and returns the
/// resulting executable's path.
fn c_binary() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = repo_root().join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");

        if !exe.exists() {
            std::fs::create_dir_all(&build).expect("create c_src/build");

            let configure = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("run cmake (is cmake installed?)");
            assert!(
                configure.status.success(),
                "cmake configure failed:\n{}",
                String::from_utf8_lossy(&configure.stderr)
            );

            let compile = Command::new("cmake")
                .args(["--build", "."])
                .current_dir(&build)
                .output()
                .expect("run cmake --build");
            assert!(
                compile.status.success(),
                "cmake --build failed:\n{}",
                String::from_utf8_lossy(&compile.stderr)
            );
        }

        assert!(exe.exists(), "C binary missing at {}", exe.display());
        exe
    })
}

/// The Rust binary under test. Cargo builds and points at this for us.
fn rust_binary() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

// ---------------------------------------------------------------------------
// running one program
// ---------------------------------------------------------------------------

struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    code: Option<i32>,
    signal: Option<i32>,
}

fn run(program: &Path, args: &[&str], stdin_bytes: &[u8]) -> Run {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", program.display()));

    // Write on a helper thread: a program that dies mid-run (or never reads)
    // would otherwise deadlock us on a full pipe or an EPIPE.
    let mut stdin = child.stdin.take().expect("piped stdin");
    let payload = stdin_bytes.to_vec();
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&payload);
        let _ = stdin.flush();
        // dropping `stdin` closes the pipe, giving the child EOF
    });

    let output = child.wait_with_output().expect("wait for child");
    let _ = writer.join();

    Run {
        stdout: output.stdout,
        stderr: output.stderr,
        code: output.status.code(),
        signal: output.status.signal(),
    }
}

// ---------------------------------------------------------------------------
// the assertion
// ---------------------------------------------------------------------------

fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).escape_debug().to_string()
}

/// Asserts the two programs agree on stdout, stderr and exit status.
fn assert_same_with_args(case: &str, args: &[&str], stdin_bytes: &[u8]) {
    let c = run(c_binary(), args, stdin_bytes);
    let r = run(rust_binary(), args, stdin_bytes);

    let context = || {
        format!(
            "case {case:?}\n  stdin  = {:?}\n  args   = {args:?}",
            show(stdin_bytes)
        )
    };

    assert_eq!(
        show(&c.stdout),
        show(&r.stdout),
        "stdout differs\n{}",
        context()
    );
    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout bytes differ\n{}",
        context()
    );
    assert_eq!(
        show(&c.stderr),
        show(&r.stderr),
        "stderr differs\n{}",
        context()
    );
    assert_eq!(c.stderr, r.stderr, "stderr bytes differ\n{}", context());
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "exit status differs (code, signal)\n{}",
        context()
    );
}

fn assert_same(case: &str, stdin_bytes: &[u8]) {
    assert_same_with_args(case, &[], stdin_bytes);
}

/// Convenience for the common shape: `goodB2G` consumes the first line and
/// `bad()` the second.
fn assert_same_lines(case: &str, first: &str, second: &str) {
    assert_same(case, format!("{first}\n{second}\n").as_bytes());
}

// ===========================================================================
// Phase B/C: the input classes the C source branches on
// ===========================================================================

// --- `fgets() != NULL` branch: EOF with nothing read --------------------

#[test]
fn empty_input_fails_both_fgets() {
    // Both goodB2G() and bad() take the `fgets() == NULL` branch, so
    // "fgets() failed." is printed twice and `data` stays -1 in both.
    assert_same("empty", b"");
}

#[test]
fn single_newline_leaves_bad_at_eof() {
    // goodB2G() gets "\n" (atoi -> 0); bad() hits EOF.
    assert_same("just a newline", b"\n");
}

#[test]
fn two_newlines_feed_both_reads() {
    assert_same("two newlines", b"\n\n");
}

#[test]
fn extra_lines_are_never_read() {
    // Only two fgets() calls happen, so lines 3+ are ignored.
    assert_same("five lines", b"1\n2\n3\n4\n5\n");
}

// --- fgets() without a trailing newline ---------------------------------

#[test]
fn no_trailing_newline_single_line() {
    assert_same("no trailing newline", b"5");
}

#[test]
fn no_trailing_newline_second_line() {
    assert_same("second line unterminated", b"3\n7");
}

// --- in-bounds indices for each sink ------------------------------------

#[test]
fn every_in_bounds_index_in_bad() {
    for i in 0..10 {
        assert_same_lines(&format!("bad index {i}"), "1", &i.to_string());
    }
}

#[test]
fn every_in_bounds_index_in_good_b2g() {
    for i in 0..10 {
        assert_same_lines(&format!("goodB2G index {i}"), &i.to_string(), "1");
    }
}

// --- the two error paths -----------------------------------------------

#[test]
fn negative_index_hits_bad_negative_message() {
    for v in ["-1", "-2", "-10", "-2147483648"] {
        assert_same_lines(&format!("negative {v}"), "1", v);
    }
}

#[test]
fn negative_index_hits_good_b2g_out_of_bounds_message() {
    for v in ["-1", "-2", "-10", "-2147483648"] {
        assert_same_lines(&format!("negative {v} in goodB2G"), v, "1");
    }
}

#[test]
fn good_b2g_upper_bound_check() {
    // goodB2G()'s sink also rejects `data >= 10`, unlike bad()'s.
    for v in ["10", "11", "99", "2147483647"] {
        assert_same_lines(&format!("goodB2G rejects {v}"), v, "1");
    }
}

// --- bad()'s missing upper-bound check: the out-of-bounds write ---------

#[test]
fn oob_write_into_dead_locals_is_unobservable() {
    // Indices 10..=15 land on the 2-byte gap, inputBuffer, `i` and `data`,
    // all of which are dead by then, so the C program prints ten zeros.
    for i in 10..=15 {
        assert_same_lines(&format!("bad index {i}"), "1", &i.to_string());
    }
}

#[test]
fn oob_write_over_bad_frame_linkage_is_fatal() {
    // 16..=17 overwrite bad()'s saved rbp, 18..=19 its return address.
    for i in 16..=19 {
        assert_same_lines(&format!("bad index {i}"), "1", &i.to_string());
    }
}

#[test]
fn oob_write_into_main_locals_is_unobservable() {
    // 20..=23 are main()'s argc/argv, 24..=25 its saved rbp, none read again.
    for i in 20..=25 {
        assert_same_lines(&format!("bad index {i}"), "1", &i.to_string());
    }
}

#[test]
fn oob_write_over_main_return_address_is_fatal() {
    for i in 26..=27 {
        assert_same_lines(&format!("bad index {i}"), "1", &i.to_string());
    }
}

#[test]
fn oob_write_above_main_frame_is_unobservable() {
    // Above main()'s frame the write lands in the argv/envp block, which the
    // program never reads again. These indices stay clear of the stack top even
    // under an empty environment, where the first faulting index drops to ~250
    // (measured); see ERRORS.md.
    for i in [28, 29, 31, 32, 40, 63, 64, 100, 128, 200] {
        assert_same_lines(&format!("bad index {i}"), "1", &i.to_string());
    }
}

#[test]
fn oob_write_far_past_stack_top_is_fatal() {
    // Far enough above the stack mapping that no argv/envp block can reach it
    // (ARG_MAX caps that block well under 4 MiB), so the fault is deterministic
    // for any environment size.
    for i in [1_000_000, 4_000_000, 16_777_216] {
        assert_same_lines(&format!("bad index {i}"), "1", &i.to_string());
    }
}

#[test]
fn oob_write_outcome_is_always_one_c_could_produce() {
    // Between those two regions sits a band where the C program is not a
    // function of its input: the write may or may not clear the top of the
    // stack mapping, and the same input alternates across runs. Exact agreement
    // there is impossible, but one property still holds for every index, and it
    // is the property that would break if the port printed partial output,
    // flushed before dying, or wrote a panic message to stderr.
    //
    // Every C execution ends in exactly one of two shapes:
    //   * exit 0, the full transcript, empty stderr
    //   * killed by a signal, *empty* stdout (nothing was ever flushed),
    //     empty stderr
    let good_g2b_prefix = b"Calling good()...\n".to_vec();

    let mut indices: Vec<u32> = vec![250, 256, 300, 512, 1000, 1200, 1500];
    indices.extend([2000, 3000, 4096, 8192, 65536, 500_000]);

    for i in indices {
        let input = format!("1\n{i}\n");
        let r = run(rust_binary(), &[], input.as_bytes());

        assert!(r.stderr.is_empty(), "index {i}: stderr must stay empty");

        match (r.code, r.signal) {
            (Some(0), None) => {
                // Completed: must be the full transcript with ten zeros.
                assert!(
                    r.stdout.starts_with(&good_g2b_prefix),
                    "index {i}: truncated transcript"
                );
                assert!(
                    r.stdout.ends_with(b"Finished bad()\n"),
                    "index {i}: transcript does not end cleanly"
                );
                let zeros = r.stdout.windows(2).filter(|w| *w == b"0\n").count();
                assert!(zeros >= 10, "index {i}: expected ten zero slots");
            }
            (None, Some(_)) => {
                // Killed: the C program's buffered stdout is lost entirely.
                assert!(
                    r.stdout.is_empty(),
                    "index {i}: stdout must be empty when killed, got {:?}",
                    show(&r.stdout)
                );
            }
            other => panic!("index {i}: unexpected exit status {other:?}"),
        }
    }
}

// --- atoi() behaviour ---------------------------------------------------

#[test]
fn atoi_non_numeric_yields_zero() {
    for v in ["abc", "x", "hello", ".", "/", ":", "-abc", "+abc"] {
        assert_same_lines(&format!("non-numeric {v:?}"), v, v);
    }
}

#[test]
fn atoi_stops_at_first_non_digit() {
    for v in ["5abc", "3.9", "1 2", "7-8", "0x5", "9e9", "4,5"] {
        assert_same_lines(&format!("trailing junk {v:?}"), v, v);
    }
}

#[test]
fn atoi_leading_whitespace_and_sign() {
    for v in [
        "  5", "\t5", "\r5", "\u{b}5", "\u{c}5", " \t 5", "+5", "-5", "  +5",
        "  -5", "+", "-", "++5", "--5", "+-5",
    ] {
        assert_same_lines(&format!("whitespace/sign {v:?}"), v, v);
    }
}

#[test]
fn atoi_leading_zeros_are_decimal_not_octal() {
    for v in ["007", "0000000005", "00000000000012"] {
        assert_same_lines(&format!("leading zeros {v:?}"), v, v);
    }
}

#[test]
fn atoi_whitespace_only_yields_zero() {
    assert_same("spaces only", b"     \n     \n");
    assert_same("tab only", b"\t\n\t\n");
}

#[test]
fn crlf_line_endings() {
    // The \r stays in the buffer; atoi() stops at it.
    assert_same("crlf", b"5\r\n3\r\n");
    assert_same("cr only", b"5\r3\r");
}

#[test]
fn int_range_boundaries() {
    // (int)strtol truncation: "2147483648" becomes INT_MIN, taking the
    // negative branch in both sinks.
    for v in [
        "2147483647",
        "2147483648",
        "-2147483648",
        "-2147483649",
        "4294967296",
        "4294967297",
    ] {
        assert_same_lines(&format!("boundary {v}"), v, "1");
    }
}

// --- the 14-byte inputBuffer: fgets() truncation and carry-over --------

#[test]
fn line_of_exactly_thirteen_characters_fits() {
    // 13 chars + NUL exactly fills inputBuffer, so the newline is left behind
    // and becomes bad()'s whole line.
    assert_same("13 chars", b"1234567890123\n");
    assert_same("13 zeros then 5", b"0000000000005\n");
}

#[test]
fn longer_lines_are_split_across_the_two_reads() {
    // goodB2G() takes the first 13 bytes; bad() gets whatever is left, which
    // is why a single long line still feeds both sinks.
    assert_same("14 chars", b"12345678901234\n");
    assert_same("15 chars", b"123456789012345\n");
    assert_same("26 chars", b"00000000000000000000000005\n");
    assert_same("13 then 13", b"0000000000000000000000009\n");
}

#[test]
fn thirteen_digit_values_truncate_to_int() {
    // Widest value the buffer admits; (int)strtol wraps it.
    for v in ["9999999999999", "1111111111111", "-999999999999"] {
        assert_same_lines(&format!("13-digit {v}"), v, "1");
    }
}

// --- raw bytes ---------------------------------------------------------

#[test]
fn embedded_nul_bytes() {
    // fgets() copies the NUL through; atoi() stops there.
    assert_same("leading NUL", b"\x005\n\x003\n");
    assert_same("NUL after digit", b"5\x009\n3\x009\n");
    assert_same("NUL only", b"\x00\n\x00\n");
}

#[test]
fn non_utf8_bytes() {
    assert_same("high bytes", b"\xff\xfe\n\xff\xfe\n");
    assert_same("latin1 digits", b"\xb95\n\xb93\n");
}

#[test]
fn no_newline_anywhere_long_input() {
    assert_same("30 chars unterminated", b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    assert_same("30 digits unterminated", b"555555555555555555555555555555");
}

// --- argv is declared but never used -----------------------------------

#[test]
fn command_line_arguments_are_ignored() {
    assert_same_with_args("one arg", &["ignored"], b"1\n5\n");
    assert_same_with_args("several args", &["a", "b", "c"], b"3\n7\n");
}

// ===========================================================================
// Phase D: the same matrix against the release binary that ships
// ===========================================================================

#[test]
fn release_binary_matches_too() {
    // The graded artifact is the release binary, and the release profile is not
    // identical to the test profile (optimisation, `panic = "abort"`), so check
    // it as well as the debug binary the tests above drive.
    //
    // Built into its own target directory: `cargo test` already holds the lock
    // on the default one, so reusing it would deadlock.
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let target_dir = manifest.join("target/release-check");

    let build = Command::new(env!("CARGO"))
        .args(["build", "--release", "--target-dir"])
        .arg(&target_dir)
        .current_dir(manifest)
        .output()
        .expect("run cargo build --release");
    assert!(
        build.status.success(),
        "release build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let release = target_dir.join("release/driver");
    assert!(
        release.exists(),
        "release binary missing at {}",
        release.display()
    );

    let mut inputs: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"\n".to_vec(),
        b"\n\n".to_vec(),
        b"5".to_vec(),
        b"3\n7".to_vec(),
        b"10\n1\n".to_vec(),
        b"-1\n-1\n".to_vec(),
        b"abc\nabc\n".to_vec(),
        b"  +7 \n  +7 \n".to_vec(),
        b"12345678901234\n".to_vec(),
        b"9999999999999\n1\n".to_vec(),
        b"2147483648\n1\n".to_vec(),
        b"-2147483648\n1\n".to_vec(),
        b"\x005\n\x003\n".to_vec(),
        b"\xff\xfe\n\xff\xfe\n".to_vec(),
    ];
    // Every frame-layout case, plus both deterministic far regions.
    inputs.extend((0..=28).map(|i| format!("1\n{i}\n").into_bytes()));
    inputs.extend([100, 200, 1_000_000, 16_777_216].map(|i| format!("1\n{i}\n").into_bytes()));

    for input in inputs {
        let c = run(c_binary(), &[], &input);
        let r = run(&release, &[], &input);
        let ctx = format!("release binary, stdin = {:?}", show(&input));
        assert_eq!(show(&c.stdout), show(&r.stdout), "stdout differs\n{ctx}");
        assert_eq!(c.stdout, r.stdout, "stdout bytes differ\n{ctx}");
        assert_eq!(c.stderr, r.stderr, "stderr differs\n{ctx}");
        assert_eq!(
            (c.code, c.signal),
            (r.code, r.signal),
            "exit status differs\n{ctx}"
        );
    }
}
