//! Differential tests: run the C binary and the Rust binary as subprocesses
//! with identical argv and require byte-identical stdout, byte-identical
//! stderr, and an identical exit status.
//!
//! The Rust program is NEVER used as a library here. It is driven exactly the
//! way a shell drives it, because that is how it is graded against the C.

use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Locating / building the two binaries
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// Path to the compiled Rust binary. Cargo builds it for us and hands us the
/// path via CARGO_BIN_EXE_*, so this is always the freshly-built executable.
fn rust_bin() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// Path to the compiled C binary, building it with cmake on first use.
fn c_bin() -> &'static Path {
    static C: OnceLock<PathBuf> = OnceLock::new();
    C.get_or_init(|| {
        let src = repo_root().join("c_src");
        let build = src.join("build");
        let exe = build.join("driver");
        if !exe.exists() {
            std::fs::create_dir_all(&build).expect("create c_src/build");
            let cfg = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("run cmake (is cmake installed?)");
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
                .expect("run cmake --build");
            assert!(
                bld.status.success(),
                "cmake build failed:\n{}\n{}",
                String::from_utf8_lossy(&bld.stdout),
                String::from_utf8_lossy(&bld.stderr)
            );
        }
        assert!(exe.exists(), "C binary missing at {}", exe.display());
        exe
    })
}

// ---------------------------------------------------------------------------
// Running a program
// ---------------------------------------------------------------------------

struct Output {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Some(code)` on normal exit, `None` if killed by a signal.
    code: Option<i32>,
}

/// Generous bounds for a program whose largest legitimate output in these
/// tests is a couple of hundred bytes. Exceeding either bound means the
/// translation looped when the C did not, which we want reported as a crisp
/// failure rather than a hung test run.
const RUN_CAP_BYTES: usize = 1 << 20; // 1 MiB
const RUN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);

fn read_capped(mut r: impl Read, cap: usize) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 8192];
    while buf.len() < cap {
        match r.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
            Err(_) => break,
        }
    }
    buf
}

fn run(bin: &Path, args: &[OsString]) -> Output {
    let mut child = Command::new(bin)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    let so = child.stdout.take().expect("piped stdout");
    let se = child.stderr.take().expect("piped stderr");
    let ho = std::thread::spawn(move || read_capped(so, RUN_CAP_BYTES));
    let he = std::thread::spawn(move || read_capped(se, RUN_CAP_BYTES));

    let deadline = std::time::Instant::now() + RUN_TIMEOUT;
    let status = loop {
        match child.try_wait().expect("try_wait") {
            Some(s) => break Some(s),
            None if std::time::Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
            None => std::thread::sleep(std::time::Duration::from_millis(5)),
        }
    };

    let stdout = ho.join().expect("stdout reader");
    let stderr = he.join().expect("stderr reader");

    if status.is_none() {
        panic!(
            "{} did not terminate within {:?} for argv {} \
             (produced {} bytes; the C program terminates promptly here)\n  first bytes: {}",
            bin.display(),
            RUN_TIMEOUT,
            show_args(args),
            stdout.len(),
            show(&stdout[..stdout.len().min(120)])
        );
    }

    Output {
        stdout,
        stderr,
        code: status.and_then(|s| s.code()),
    }
}

/// Run a program that may emit an unbounded amount of output, capturing only
/// the first `limit` bytes of stdout and then killing it. Used for the inputs
/// where the C program loops billions of times (signed overflow wrap-around).
fn run_prefix(bin: &Path, args: &[OsString], limit: usize) -> Vec<u8> {
    let mut child = Command::new(bin)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    let mut stdout = child.stdout.take().expect("piped stdout");
    let mut buf = vec![0u8; limit];
    let mut filled = 0usize;
    while filled < limit {
        match stdout.read(&mut buf[filled..]) {
            Ok(0) => break, // process ended early
            Ok(n) => filled += n,
            Err(e) => panic!("read from {}: {e}", bin.display()),
        }
    }
    buf.truncate(filled);

    let _ = child.kill();
    let _ = child.wait();
    buf
}

// ---------------------------------------------------------------------------
// Assertions
// ---------------------------------------------------------------------------

fn show(b: &[u8]) -> String {
    // Readable, but never hides a byte-level difference.
    String::from_utf8_lossy(b).escape_debug().to_string()
}

fn show_args(args: &[OsString]) -> String {
    let parts: Vec<String> = args
        .iter()
        .map(|a| format!("{:?}", a.as_os_str().to_string_lossy()))
        .collect();
    format!("[{}]", parts.join(", "))
}

/// The core check: stdout, stderr and exit status must all match.
#[track_caller]
fn assert_same(args: &[OsString]) -> Output {
    let c = run(c_bin(), args);
    let r = run(rust_bin(), args);
    let a = show_args(args);

    assert_eq!(
        c.stdout,
        r.stdout,
        "STDOUT mismatch for argv {a}\n  C   : {}\n  Rust: {}",
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "STDERR mismatch for argv {a}\n  C   : {}\n  Rust: {}",
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        c.code, r.code,
        "EXIT STATUS mismatch for argv {a}\n  C   : {:?}\n  Rust: {:?}",
        c.code, r.code
    );
    c
}

#[track_caller]
fn assert_same_prefix(args: &[OsString], limit: usize) {
    let c = run_prefix(c_bin(), args, limit);
    let r = run_prefix(rust_bin(), args, limit);
    assert_eq!(
        c.len(),
        limit,
        "expected {} to produce at least {limit} bytes for argv {}",
        c_bin().display(),
        show_args(args)
    );
    assert_eq!(
        c,
        r,
        "STDOUT PREFIX mismatch for argv {}\n  C   : {}\n  Rust: {}",
        show_args(args),
        show(&c),
        show(&r)
    );
}

// Convenience constructors ---------------------------------------------------

fn a(items: &[&str]) -> Vec<OsString> {
    items.iter().map(OsString::from).collect()
}

fn one(s: &str) -> Vec<OsString> {
    vec![OsString::from(s)]
}

#[cfg(unix)]
fn one_bytes(b: &[u8]) -> Vec<OsString> {
    use std::os::unix::ffi::OsStringExt;
    vec![OsString::from_vec(b.to_vec())]
}

// Expected literals, straight out of the C source.
const ERR_ARGC: &[u8] = b"Error: should only be a single (integer) argument!\n";
const ERR_PARSE: &[u8] = b"Error: first argument must be an integer!\n";

// ===========================================================================
// Phase A sanity: both binaries exist and are runnable
// ===========================================================================

#[test]
fn both_binaries_are_runnable() {
    // Force the C build, then run each once.
    let c = run(c_bin(), &[]);
    let r = run(rust_bin(), &[]);
    assert_eq!(c.code, Some(1));
    assert_eq!(r.code, Some(1));
}

// ===========================================================================
// Branch: argc != 2
// ===========================================================================

#[test]
fn argc_zero_extra_args() {
    let out = assert_same(&[]);
    assert_eq!(out.stdout, ERR_ARGC);
    assert_eq!(out.stderr, b"");
    assert_eq!(out.code, Some(1), "C returns 1 from the argc branch");
}

#[test]
fn argc_two_extra_args() {
    let out = assert_same(&a(&["1", "2"]));
    assert_eq!(out.stdout, ERR_ARGC);
    assert_eq!(out.code, Some(1));
}

#[test]
fn argc_three_extra_args() {
    let out = assert_same(&a(&["1", "2", "3"]));
    assert_eq!(out.stdout, ERR_ARGC);
    assert_eq!(out.code, Some(1));
}

#[test]
fn argc_many_extra_args() {
    assert_same(&a(&["5", "5", "5", "5", "5", "5"]));
}

// An empty-string extra arg still counts toward argc.
#[test]
fn argc_two_args_second_empty() {
    let out = assert_same(&a(&["7", ""]));
    assert_eq!(out.stdout, ERR_ARGC);
}

// ===========================================================================
// Branch: end == argv[1]  (strtol converted nothing)
// ===========================================================================

#[test]
fn parse_error_empty_string() {
    let out = assert_same(&one(""));
    assert_eq!(out.stdout, ERR_PARSE);
    assert_eq!(out.stderr, b"");
    assert_eq!(out.code, Some(1));
}

#[test]
fn parse_error_letters() {
    let out = assert_same(&one("abc"));
    assert_eq!(out.stdout, ERR_PARSE);
    assert_eq!(out.code, Some(1));
}

#[test]
fn parse_error_whitespace_only() {
    // strtol skips whitespace, finds no digits, and resets end to nptr.
    for s in ["   ", "\t", "\n", "\r", "\u{0b}", "\u{0c}", " \t\n\r\u{0b}\u{0c} "] {
        let out = assert_same(&one(s));
        assert_eq!(out.stdout, ERR_PARSE, "for {s:?}");
        assert_eq!(out.code, Some(1));
    }
}

#[test]
fn parse_error_sign_only() {
    for s in ["+", "-", "++", "--", "+-", "-+", "--5", "++5", "+ 5", "- 5"] {
        let out = assert_same(&one(s));
        assert_eq!(out.stdout, ERR_PARSE, "for {s:?}");
        assert_eq!(out.code, Some(1));
    }
}

#[test]
fn parse_error_non_digit_leaders() {
    for s in [".5", "e5", "x10", "#3", "/9", ":9", " abc", "\t-x", "!", "_9", "'9"] {
        let out = assert_same(&one(s));
        assert_eq!(out.stdout, ERR_PARSE, "for {s:?}");
        assert_eq!(out.code, Some(1));
    }
}

#[test]
fn parse_error_unicode_digit_lookalikes() {
    // Non-ASCII "digits" are not digits to strtol.
    for s in ["٣", "٩", "２", "٣٩"] {
        let out = assert_same(&one(s));
        assert_eq!(out.stdout, ERR_PARSE, "for {s:?}");
    }
}

#[cfg(unix)]
#[test]
fn parse_error_invalid_utf8_arg() {
    // argv is bytes to C; the Rust side must not choke on non-UTF-8.
    for bytes in [&b"\xff\xfe"[..], &b"\x80"[..], &b"\xc3"[..]] {
        let out = assert_same(&one_bytes(bytes));
        assert_eq!(out.stdout, ERR_PARSE, "for {bytes:?}");
        assert_eq!(out.code, Some(1));
    }
}

// ===========================================================================
// Happy path: counting up to a value ending in 9
// ===========================================================================

#[test]
fn single_item_already_ends_in_nine() {
    // The loop body runs exactly once: print, then break.
    let out = assert_same(&one("9"));
    assert_eq!(out.stdout, b"9\n");
    assert_eq!(out.stderr, b"");
    assert_eq!(out.code, Some(0));
}

#[test]
fn counts_from_zero() {
    let out = assert_same(&one("0"));
    assert_eq!(out.stdout, b"0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n");
    assert_eq!(out.code, Some(0));
}

#[test]
fn counts_from_mid_decade() {
    let out = assert_same(&one("5"));
    assert_eq!(out.stdout, b"5\n6\n7\n8\n9\n");
}

#[test]
fn counts_within_second_decade() {
    let out = assert_same(&one("12"));
    assert_eq!(out.stdout, b"12\n13\n14\n15\n16\n17\n18\n19\n");
}

#[test]
fn counts_from_nineteen_stops_immediately() {
    let out = assert_same(&one("19"));
    assert_eq!(out.stdout, b"19\n");
}

#[test]
fn counts_from_hundred() {
    let out = assert_same(&one("100"));
    assert_eq!(
        out.stdout,
        b"100\n101\n102\n103\n104\n105\n106\n107\n108\n109\n"
    );
}

#[test]
fn every_last_digit_zero_through_nine() {
    for d in 0..10 {
        let s = format!("{}", 40 + d);
        let out = assert_same(&one(&s));
        assert_eq!(out.code, Some(0), "for {s}");
        let expected: Vec<u8> = (40 + d..=49)
            .flat_map(|v| format!("{v}\n").into_bytes())
            .collect();
        assert_eq!(out.stdout, expected, "for {s}");
    }
}

// ===========================================================================
// Negative starts: `val % 10` is NEVER 9 for negative val in C (truncated
// division gives a non-positive remainder), so the loop counts all the way
// up through zero to 9.
// ===========================================================================

#[test]
fn negative_start_counts_up_through_zero_to_nine() {
    let out = assert_same(&one("-3"));
    let expected: Vec<u8> = (-3..=9).flat_map(|v| format!("{v}\n").into_bytes()).collect();
    assert_eq!(out.stdout, expected);
    assert_eq!(out.code, Some(0));
}

#[test]
fn negative_nine_does_not_stop_early() {
    // -9 % 10 == -9 in C, not 9 -> keeps counting.
    let out = assert_same(&one("-9"));
    let expected: Vec<u8> = (-9..=9).flat_map(|v| format!("{v}\n").into_bytes()).collect();
    assert_eq!(out.stdout, expected);
}

#[test]
fn negative_nineteen_and_friends() {
    for start in [-19i32, -29, -10, -1, -100] {
        let s = start.to_string();
        let out = assert_same(&one(&s));
        let expected: Vec<u8> = (start..=9)
            .flat_map(|v| format!("{v}\n").into_bytes())
            .collect();
        assert_eq!(out.stdout, expected, "for {s}");
    }
}

#[test]
fn negative_zero_is_zero() {
    let out = assert_same(&one("-0"));
    assert_eq!(out.stdout, b"0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n");
}

// ===========================================================================
// strtol accepts partial input: sign, leading whitespace, trailing garbage
// ===========================================================================

#[test]
fn explicit_plus_sign() {
    let out = assert_same(&one("+7"));
    assert_eq!(out.stdout, b"7\n8\n9\n");
}

#[test]
fn leading_zeros_are_decimal_not_octal() {
    let out = assert_same(&one("007"));
    assert_eq!(out.stdout, b"7\n8\n9\n");
}

#[test]
fn leading_whitespace_and_trailing_garbage() {
    let out = assert_same(&one("  42abc"));
    assert_eq!(out.stdout, b"42\n43\n44\n45\n46\n47\n48\n49\n");
    assert_eq!(out.code, Some(0));
}

#[test]
fn hex_prefix_parses_as_zero_in_base_ten() {
    // strtol(.., 10) reads "0" and stops at 'x'.
    let out = assert_same(&one("0x1F"));
    assert_eq!(out.stdout, b"0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n");
}

#[test]
fn float_like_input_truncates_at_the_dot() {
    let out = assert_same(&one("3.7"));
    assert_eq!(out.stdout, b"3\n4\n5\n6\n7\n8\n9\n");
}

#[test]
fn exponent_notation_stops_at_e() {
    let out = assert_same(&one("1e3"));
    assert_eq!(out.stdout, b"1\n2\n3\n4\n5\n6\n7\n8\n9\n");
}

#[test]
fn trailing_whitespace_and_internal_space() {
    for (s, start) in [("5 ", 5), ("5\n", 5), ("5 6", 5), ("-2 ", -2)] {
        let out = assert_same(&one(s));
        let expected: Vec<u8> = (start..=9)
            .flat_map(|v| format!("{v}\n").into_bytes())
            .collect();
        assert_eq!(out.stdout, expected, "for {s:?}");
    }
}

#[cfg(unix)]
#[test]
fn digits_followed_by_invalid_utf8() {
    let out = assert_same(&one_bytes(b"12\xff"));
    assert_eq!(out.stdout, b"12\n13\n14\n15\n16\n17\n18\n19\n");
    assert_eq!(out.code, Some(0));
}

// ===========================================================================
// `int val = strtol(...)`: the long result is TRUNCATED to 32 bits.
// ===========================================================================

#[test]
fn truncation_two_to_the_thirty_two_becomes_zero() {
    let out = assert_same(&one("4294967296"));
    assert_eq!(out.stdout, b"0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n");
    assert_eq!(out.code, Some(0));
}

#[test]
fn truncation_yields_nine_and_stops_at_once() {
    let out = assert_same(&one("4294967305")); // 2^32 + 9
    assert_eq!(out.stdout, b"9\n");
}

#[test]
fn truncation_yields_negative_value() {
    let out = assert_same(&one("4294967286")); // 2^32 - 10 -> -10
    let expected: Vec<u8> = (-10..=9)
        .flat_map(|v| format!("{v}\n").into_bytes())
        .collect();
    assert_eq!(out.stdout, expected);
}

#[test]
fn long_max_truncates_to_minus_one() {
    let expected: Vec<u8> = (-1..=9).flat_map(|v| format!("{v}\n").into_bytes()).collect();
    // LONG_MAX itself, and values that make strtol clamp to LONG_MAX (ERANGE).
    for s in [
        "9223372036854775807",
        "9223372036854775808",
        "9999999999999999999999",
        "123456789012345678901234567890",
    ] {
        let out = assert_same(&one(s));
        assert_eq!(out.stdout, expected, "for {s}");
        assert_eq!(out.code, Some(0), "for {s}");
    }
}

#[test]
fn long_min_truncates_to_zero() {
    let expected: Vec<u8> = (0..=9).flat_map(|v| format!("{v}\n").into_bytes()).collect();
    for s in [
        "-9223372036854775808",
        "-9223372036854775809",
        "-9999999999999999999999",
        "-123456789012345678901234567890",
    ] {
        let out = assert_same(&one(s));
        assert_eq!(out.stdout, expected, "for {s}");
        assert_eq!(out.code, Some(0), "for {s}");
    }
}

#[test]
fn many_leading_zeros_do_not_trigger_overflow() {
    let s = format!("{}5", "0".repeat(300));
    let out = assert_same(&one(&s));
    assert_eq!(out.stdout, b"5\n6\n7\n8\n9\n");
}

// ===========================================================================
// The largest values the loop handles without overflowing.
// ===========================================================================

#[test]
fn int_max_ending_in_nine_stops_immediately() {
    // 2147483639 is the largest int that satisfies val % 10 == 9.
    let out = assert_same(&one("2147483639"));
    assert_eq!(out.stdout, b"2147483639\n");
    assert_eq!(out.code, Some(0));
}

#[test]
fn largest_bounded_run() {
    let out = assert_same(&one("2147483630"));
    let expected: Vec<u8> = (2147483630i32..=2147483639)
        .flat_map(|v| format!("{v}\n").into_bytes())
        .collect();
    assert_eq!(out.stdout, expected);
    assert_eq!(out.code, Some(0));
}

// ===========================================================================
// Signed overflow: past INT_MAX the C wraps to INT_MIN and keeps going for
// billions of iterations. Compare a bounded prefix of stdout instead of
// waiting for (or storing) the whole thing.
// ===========================================================================

#[test]
fn int_max_wraps_to_int_min() {
    assert_same_prefix(&one("2147483647"), 4096);
}

#[test]
fn overflow_crosses_int_max_boundary() {
    // 2147483640 % 10 == 0, so it counts 2147483640..2147483647 then wraps.
    assert_same_prefix(&one("2147483640"), 4096);
}

#[test]
fn int_min_start_runs_for_billions_of_iterations() {
    // -2147483648 % 10 == -8, never 9, so it counts all the way up to 9.
    assert_same_prefix(&one("-2147483648"), 4096);
}

// ===========================================================================
// SIGPIPE. A C program leaves SIGPIPE at SIG_DFL, so `driver N | head -1`
// kills it with signal 13. The Rust runtime ignores SIGPIPE before main, which
// would instead surface as a write error and a *normal* exit -- a visible
// difference in `$?` (141 vs 0/1). Assert both die by signal.
// ===========================================================================

#[cfg(unix)]
#[test]
fn closed_stdout_kills_both_with_sigpipe() {
    use std::os::unix::process::ExitStatusExt;

    fn signal_after_reader_hangs_up(bin: &Path) -> Option<i32> {
        let mut child = Command::new(bin)
            .arg("2147483647") // unbounded output, so it keeps writing
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn");

        {
            let mut stdout = child.stdout.take().expect("piped stdout");
            let mut buf = [0u8; 16];
            let _ = stdout.read(&mut buf);
            // `stdout` drops here, closing the read end of the pipe.
        }

        // Bounded wait: a program that IGNORES SIGPIPE just spins forever
        // writing to a broken pipe, so an unbounded wait() would hang the
        // whole suite instead of reporting a failure.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            match child.try_wait().expect("try_wait") {
                Some(status) => return status.signal(),
                None if std::time::Instant::now() >= deadline => {
                    let _ = child.kill();
                    let _ = child.wait();
                    panic!(
                        "{} did not die after its stdout reader hung up; \
                         it is ignoring SIGPIPE instead of taking SIG_DFL",
                        bin.display()
                    );
                }
                None => std::thread::sleep(std::time::Duration::from_millis(10)),
            }
        }
    }

    let c_sig = signal_after_reader_hangs_up(c_bin());
    let r_sig = signal_after_reader_hangs_up(rust_bin());
    assert_eq!(c_sig, Some(13), "C should be killed by SIGPIPE");
    assert_eq!(
        c_sig, r_sig,
        "SIGPIPE disposition differs: C={c_sig:?} Rust={r_sig:?}"
    );
}

#[test]
fn overflow_prefix_contains_the_wrap() {
    // Pin the actual wrap-around bytes, so the prefix test cannot pass
    // vacuously if both programs changed behaviour together.
    let bytes = run_prefix(c_bin(), &one("2147483647"), 64);
    let text = String::from_utf8_lossy(&bytes);
    assert!(
        text.starts_with("2147483647\n-2147483648\n-2147483647\n"),
        "C did not wrap as expected: {text:?}"
    );
}
