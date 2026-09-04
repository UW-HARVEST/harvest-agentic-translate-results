// tests/differential.rs
//
// Differential test harness: runs the original C `driver` and the translated
// Rust `driver` as SUBPROCESSES, feeds both the identical stdin bytes, and
// asserts that stdout, stderr and the exit status match byte for byte.
//
// The Rust code is never linked in as a library -- both programs are driven
// exactly the way a shell would drive them, because that is how the
// translation is graded.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;

// ============================================================================
// Locating / building the two executables
// ============================================================================

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the Rust executable under test. Cargo builds this for us before the
/// integration test runs.
fn rust_bin() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// Path to the C executable, building it with CMake on first use so that
/// `cargo test` works from a clean checkout.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = repo_root().join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");

        if !exe.exists() {
            std::fs::create_dir_all(&build).expect("cannot create c_src/build");

            let cfg = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("failed to run `cmake ..` -- is cmake installed?");
            assert!(
                cfg.status.success(),
                "cmake configure failed:\n{}\n{}",
                String::from_utf8_lossy(&cfg.stdout),
                String::from_utf8_lossy(&cfg.stderr)
            );

            let bld = Command::new("cmake")
                .args(["--build", "."])
                .current_dir(&build)
                .output()
                .expect("failed to run `cmake --build .`");
            assert!(
                bld.status.success(),
                "cmake build failed:\n{}\n{}",
                String::from_utf8_lossy(&bld.stdout),
                String::from_utf8_lossy(&bld.stderr)
            );
        }

        assert!(
            exe.exists(),
            "C executable missing after build: {}",
            exe.display()
        );
        exe
    })
}

// ============================================================================
// Running one program
// ============================================================================

fn run(exe: &Path, stdin_bytes: &[u8]) -> Output {
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // Keep locale-sensitive libc behaviour pinned so the comparison is
        // about the translation, not about the ambient environment.
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));

    {
        let mut sink = child.stdin.take().expect("stdin was piped");
        let bytes = stdin_bytes.to_vec();
        // Write on a helper thread: a program that stops reading (menu choice
        // 7 exits early) would otherwise deadlock a large write.
        let writer = std::thread::spawn(move || {
            let _ = sink.write_all(&bytes);
            let _ = sink.flush();
            drop(sink);
        });
        let out = child
            .wait_with_output()
            .unwrap_or_else(|e| panic!("failed to wait for {}: {e}", exe.display()));
        let _ = writer.join();
        return out;
    }
}

/// A printable, comparable rendering of the exit status: the exit code when the
/// process exited normally, or the terminating signal when it did not.
fn status_repr(out: &Output) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(sig) = out.status.signal() {
            return format!("signal({sig})");
        }
    }
    match out.status.code() {
        Some(c) => format!("exit({c})"),
        None => "unknown".to_string(),
    }
}

fn escape(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes {
        match b {
            b'\n' => s.push_str("\\n\n"),
            b'\t' => s.push_str("\\t"),
            b'\r' => s.push_str("\\r"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    s
}

/// Report the first differing byte together with a little context.
fn first_diff(a: &[u8], b: &[u8]) -> String {
    let n = a.len().min(b.len());
    let at = (0..n).find(|&i| a[i] != b[i]).unwrap_or(n);
    let lo = at.saturating_sub(80);
    let hi_a = (at + 80).min(a.len());
    let hi_b = (at + 80).min(b.len());
    format!(
        "first difference at byte {at} (C len {}, Rust len {})\n\
         --- C   ---\n{}\n--- Rust ---\n{}\n",
        a.len(),
        b.len(),
        escape(&a[lo..hi_a]),
        escape(&b[lo..hi_b]),
    )
}

// ============================================================================
// The assertion every test funnels through
// ============================================================================

fn assert_same(name: &str, stdin_bytes: &[u8]) {
    let c = run(c_bin(), stdin_bytes);
    let r = run(rust_bin(), stdin_bytes);

    assert_eq!(
        status_repr(&c),
        status_repr(&r),
        "[{name}] EXIT STATUS mismatch (C={} Rust={})",
        status_repr(&c),
        status_repr(&r)
    );

    assert!(
        c.stdout == r.stdout,
        "[{name}] STDOUT mismatch\n{}",
        first_diff(&c.stdout, &r.stdout)
    );

    assert!(
        c.stderr == r.stderr,
        "[{name}] STDERR mismatch\n{}",
        first_diff(&c.stderr, &r.stderr)
    );
}

fn assert_same_str(name: &str, stdin_text: &str) {
    assert_same(name, stdin_text.as_bytes());
}

/// Deterministic pseudo-random byte source (SplitMix64) so the fuzz-style
/// cases are reproducible without adding a dependency.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

// ============================================================================
// Phase A -- both programs exist and are runnable
// ============================================================================

#[test]
fn both_executables_are_runnable() {
    assert!(c_bin().exists(), "C binary not found at {:?}", c_bin());
    assert!(rust_bin().exists(), "Rust binary not found at {:?}", rust_bin());

    // The banner is printed before any input is consumed, so an empty stdin
    // proves both programs actually start and produce output.
    let c = run(c_bin(), b"");
    let r = run(rust_bin(), b"");
    assert!(!c.stdout.is_empty(), "C produced no stdout");
    assert!(!r.stdout.is_empty(), "Rust produced no stdout");
}

// ============================================================================
// Phase B -- the input classes main() branches on
//
// main() reads a line with fgets(input, 256, stdin) and runs
// sscanf(input, "%d", &choice); the switch has cases 1..=7 plus a default,
// and there are two pre-switch outcomes: fgets returning NULL (loop breaks)
// and sscanf not returning 1 ("Invalid input").
// ============================================================================

#[test]
fn empty_input_prints_banner_and_menu_then_eof() {
    assert_same_str("empty", "");
}

#[test]
fn single_item_one_menu_choice() {
    // The smallest non-trivial input: exactly one selection.
    assert_same_str("single-choice-1", "1\n");
}

#[test]
fn menu_choice_1_integer_containers() {
    assert_same_str("choice-1", "1\n");
}

#[test]
fn menu_choice_2_double_containers() {
    // Exercises the array growth path: array_double_create(5) then 7 pushes.
    assert_same_str("choice-2", "2\n");
}

#[test]
fn menu_choice_3_inventory_array() {
    assert_same_str("choice-3", "3\n");
}

#[test]
fn menu_choice_4_order_list() {
    assert_same_str("choice-4", "4\n");
}

#[test]
fn menu_choice_5_mixed_operations() {
    assert_same_str("choice-5", "5\n");
}

#[test]
fn menu_choice_6_run_all_demos() {
    assert_same_str("choice-6", "6\n");
}

#[test]
fn menu_choice_7_exits_with_goodbye() {
    // `case 7:` is the only `return` before the end of main().
    assert_same_str("choice-7", "7\n");
}

#[test]
fn menu_choice_7_stops_reading_remaining_input() {
    assert_same_str("choice-7-then-more", "7\n1\n2\n3\n");
}

#[test]
fn default_case_invalid_choice() {
    for (name, input) in [
        ("zero", "0\n"),
        ("eight", "8\n"),
        ("nine", "9\n"),
        ("negative-one", "-1\n"),
        ("negative-seven", "-7\n"),
        ("negative-zero", "-0\n"),
        ("large", "123456\n"),
    ] {
        assert_same_str(name, input);
    }
}

#[test]
fn sscanf_matching_failure_invalid_input() {
    // sscanf() != 1 -> "Invalid input", then the loop continues.
    for (name, input) in [
        ("blank-line", "\n"),
        ("letters", "abc\n"),
        ("sign-only-plus", "+\n"),
        ("sign-only-minus", "-\n"),
        ("dot-five", ".5\n"),
        ("space-only", "   \n"),
        ("tabs-only", "\t\t\n"),
        ("crlf-only", "\r\n"),
        ("punctuation", "!@#$%^&*()\n"),
        ("hash", "#7\n"),
    ] {
        assert_same_str(name, input);
    }
}

#[test]
fn invalid_input_then_valid_choice_continues_loop() {
    assert_same_str("invalid-then-exit", "x\n7\n");
    assert_same_str("blank-then-demo", "\n3\n");
    assert_same_str("mixed", "9\n-1\n0\n8\nfoo\n\n7\n");
}

#[test]
fn every_choice_in_order() {
    assert_same_str("all-in-order", "1\n2\n3\n4\n5\n6\n7\n");
}

#[test]
fn repeated_choices_accumulate_identically() {
    assert_same_str("repeat-3", "3\n3\n3\n");
    assert_same_str("double-all-demos", "6\n6\n7\n");
}

// ============================================================================
// Phase B -- sscanf("%d") conversion details
// ============================================================================

#[test]
fn sscanf_skips_leading_whitespace() {
    assert_same_str("leading-spaces", "  3\n");
    assert_same_str("leading-tab", "\t7\n");
    assert_same_str("leading-mixed-ws", " \t \t6\n");
    assert_same_str("leading-vtab-ff", "\x0b\x0c4\n");
}

#[test]
fn sscanf_accepts_optional_sign_and_leading_zeros() {
    assert_same_str("plus-seven", "+7\n");
    assert_same_str("plus-six-padded", "   +6   \n");
    assert_same_str("leading-zeros", "007\n");
    assert_same_str("many-leading-zeros", &format!("{}7\n", "0".repeat(40)));
}

#[test]
fn sscanf_stops_at_first_non_digit() {
    assert_same_str("digits-then-junk", "3abc\n");
    assert_same_str("two-numbers-one-line", "1 2\n");
    assert_same_str("float-truncates", "6.9\n");
    assert_same_str("hex-like-reads-zero", "0x10\n");
    assert_same_str("digit-then-dash", "4-5\n");
}

#[test]
fn sscanf_int_truncation_and_overflow() {
    // %d assigns into an `int`, so the converted long is truncated.
    assert_same_str("int-max", "2147483647\n");
    assert_same_str("int-max-plus-one", "2147483648\n");
    assert_same_str("int-min", "-2147483648\n");
    assert_same_str("int-min-minus-one", "-2147483649\n");
    // 2^32 + 7 truncates to 7, which reaches `case 7:` and exits.
    assert_same_str("wraps-to-seven", "4294967303\n");
    // 2 * 2^32 + 6 truncates to 6, which runs every demo.
    assert_same_str("wraps-to-six", "8589934598\n");
    // Values that overflow `long` saturate before truncation.
    assert_same_str("i64-max", "9223372036854775807\n");
    assert_same_str("i64-max-plus-one", "9223372036854775808\n");
    assert_same_str("i64-min-minus-one", "-9223372036854775809\n");
    assert_same_str("two-pow-64-plus-one", "18446744073709551617\n");
    assert_same_str("two-pow-64-plus-five", "18446744073709551621\n");
    assert_same_str("absurdly-long", &format!("{}\n", "9".repeat(120)));
    assert_same_str("absurdly-long-negative", &format!("-{}\n", "9".repeat(120)));
}

// ============================================================================
// Phase C -- fgets(input, 256, stdin) boundary behaviour
//
// fgets stops at a newline OR after 255 bytes, whichever comes first, and does
// NOT skip the rest of an over-long line: the remainder becomes the next line.
// ============================================================================

#[test]
fn line_exactly_at_the_fgets_limit() {
    // 255 payload bytes fill the buffer, so the '\n' is left behind and read
    // as a second (empty) line -> one extra "Invalid input".
    let s = format!("1{}\n", " ".repeat(254));
    assert_eq!(s.len(), 256);
    assert_same_str("exactly-255-then-newline", &s);
}

#[test]
fn overlong_line_is_split_mid_number() {
    // 254 spaces + "77": fgets takes 254 spaces + the first '7' only, so the
    // choice is 7 and the program exits before seeing the second '7'.
    assert_same_str("split-mid-number", &format!("{}77\n", " ".repeat(254)));
    // 253 spaces + "77" fits: the choice is 77 -> "Invalid choice".
    assert_same_str("fits-as-77", &format!("{}77\n", " ".repeat(253)));
    // 255 spaces: no digits at all -> "Invalid input", then "7\n" exits.
    assert_same_str("255-spaces-then-7", &format!("{}7\n", " ".repeat(255)));
}

#[test]
fn overlong_non_numeric_lines() {
    assert_same_str("255-z", &format!("{}\n", "z".repeat(255)));
    assert_same_str("256-z", &format!("{}\n", "z".repeat(256)));
    assert_same_str("300-z", &format!("{}\n", "z".repeat(300)));
    assert_same_str("1000-z", &format!("{}\n", "z".repeat(1000)));
}

#[test]
fn overlong_numeric_lines() {
    assert_same_str("300-sevens", &format!("{}\n", "7".repeat(300)));
    // 255 zeros -> choice 0; the 45-zero remainder plus '7' -> choice 7.
    assert_same_str("300-zeros-then-7", &format!("{}7\n", "0".repeat(300)));
}

#[test]
fn missing_trailing_newline_at_eof() {
    // fgets returns the partial last line, then NULL on the next call.
    assert_same("no-newline-7", b"7");
    assert_same("no-newline-1", b"1");
    assert_same("no-newline-junk", b"abc");
    assert_same("no-newline-blank-after", b"1\n2");
}

#[test]
fn carriage_returns_and_crlf() {
    assert_same("crlf-7", b"7\r\n");
    assert_same("crlf-3", b"3\r\n");
    assert_same("cr-line-then-7", b"\r\n7\n");
    assert_same("lone-cr", b"7\r");
}

#[test]
fn embedded_nul_bytes_terminate_the_sscanf_string() {
    // fgets copies the NUL through; sscanf then sees an empty string.
    assert_same("nul-then-7", b"\x007\n");
    // A NUL after the digits does not stop the conversion.
    assert_same("7-then-nul", b"7\x00\n");
    assert_same("nul-line-then-7", b"\x00\n7\n");
    assert_same("nul-mid-number", b"1\x002\n");
}

#[test]
fn non_ascii_and_invalid_utf8_input() {
    assert_same("utf8-input", "é\n7\n".as_bytes());
    assert_same("invalid-utf8", b"\xff\xfe\n7\n");
    assert_same("high-bytes-only", b"\x80\x81\x82\n");
    assert_same("utf8-digits", "７\n7\n".as_bytes());
}

// ============================================================================
// Phase C -- volume, and deterministic fuzzing over the branch space
// ============================================================================

#[test]
fn many_blank_lines() {
    assert_same_str("1000-blank-lines", &"\n".repeat(1000));
}

#[test]
fn many_repeated_demos() {
    assert_same_str("200x-demo-1", &"1\n".repeat(200));
    assert_same_str("50x-demo-6", &"6\n".repeat(50));
}

#[test]
fn fuzz_over_menu_tokens() {
    const TOKENS: &[&str] = &[
        "1", "2", "3", "4", "5", "6", "0", "9", "-3", "x", "", " 4", "+2", "8.5", "  ", "007",
        "2147483648", "4294967303", "abc123", "\t5",
    ];
    for seed in 0..8u64 {
        let mut rng = Rng::new(seed.wrapping_mul(0x1234_5678).wrapping_add(1));
        let mut input = String::new();
        for _ in 0..150 {
            input.push_str(TOKENS[rng.below(TOKENS.len())]);
            input.push('\n');
        }
        assert_same_str(&format!("fuzz-tokens-seed-{seed}"), &input);
    }
}

#[test]
fn fuzz_over_raw_bytes() {
    for seed in 0..6u64 {
        let mut rng = Rng::new(0xDEAD_BEEF ^ seed);
        let bytes: Vec<u8> = (0..3000).map(|_| rng.below(256) as u8).collect();
        assert_same(&format!("fuzz-raw-seed-{seed}"), &bytes);
    }
}

#[test]
fn fuzz_over_number_like_bytes() {
    const ALPHABET: &[u8] = b"0123456789 \t\n\r+-abc\x00\xff";
    for seed in 0..6u64 {
        let mut rng = Rng::new(0x0BAD_F00D ^ seed);
        let bytes: Vec<u8> = (0..4000)
            .map(|_| ALPHABET[rng.below(ALPHABET.len())])
            .collect();
        assert_same(&format!("fuzz-numeric-seed-{seed}"), &bytes);
    }
}
