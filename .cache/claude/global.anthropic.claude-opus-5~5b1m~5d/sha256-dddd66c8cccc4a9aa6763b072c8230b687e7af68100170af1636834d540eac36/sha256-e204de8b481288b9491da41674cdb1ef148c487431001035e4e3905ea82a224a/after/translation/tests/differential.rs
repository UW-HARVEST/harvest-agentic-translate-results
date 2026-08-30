//! Differential tests: run the C `driver` and the Rust `driver` as subprocesses
//! with identical argv and require byte-identical stdout, byte-identical stderr
//! and identical exit status.
//!
//! The Rust code is NEVER used as a library here; only the built binary is
//! driven, exactly the way a shell would drive it.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

/// Path to the Rust binary under test (provided by Cargo).
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Repository root: the parent of the crate directory.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate dir has a parent")
        .to_path_buf()
}

/// Path to the C binary, building it with CMake on first use if necessary.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = repo_root().join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");
        if exe.exists() {
            return exe;
        }

        std::fs::create_dir_all(&build).expect("create c_src/build");
        let conf = Command::new("cmake")
            .arg("..")
            .current_dir(&build)
            .output()
            .expect("run `cmake ..` (is cmake installed?)");
        assert!(
            conf.status.success(),
            "cmake configure failed:\n{}\n{}",
            String::from_utf8_lossy(&conf.stdout),
            String::from_utf8_lossy(&conf.stderr)
        );
        let bld = Command::new("cmake")
            .args(["--build", "."])
            .current_dir(&build)
            .output()
            .expect("run `cmake --build .`");
        assert!(
            bld.status.success(),
            "cmake build failed:\n{}\n{}",
            String::from_utf8_lossy(&bld.stdout),
            String::from_utf8_lossy(&bld.stderr)
        );
        assert!(exe.exists(), "C binary missing after build: {}", exe.display());
        exe
    })
}

/// Build an `OsString` argument from raw bytes, so that non-UTF-8 argv values
/// (which the C program happily accepts) can be exercised too.
fn os_arg(bytes: &[u8]) -> OsString {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        OsStr::from_bytes(bytes).to_os_string()
    }
    #[cfg(not(unix))]
    {
        OsString::from(String::from_utf8_lossy(bytes).into_owned())
    }
}

fn run(exe: &Path, args: &[OsString]) -> Output {
    Command::new(exe)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()))
}

/// Core assertion: same argv => same stdout, same stderr, same exit status.
fn assert_same(args: &[&[u8]]) {
    let argv: Vec<OsString> = args.iter().map(|a| os_arg(a)).collect();
    let c = run(c_bin(), &argv);
    let r = run(&rust_bin(), &argv);

    let shown: Vec<String> = args
        .iter()
        .map(|a| String::from_utf8_lossy(a).into_owned())
        .collect();
    let label = format!("argv[1..] = {shown:?}");

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch for {label}\n C stdout: {:?}\n R stdout: {:?}",
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch for {label}\n C stderr: {:?}\n R stderr: {:?}",
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "exit status mismatch for {label}: C={:?} R={:?}",
        c.status.code(),
        r.status.code()
    );
}

fn assert_same_str(arg: &str) {
    assert_same(&[arg.as_bytes()]);
}

// ---------------------------------------------------------------------------
// argc branch: `if (argc != 2)`
// ---------------------------------------------------------------------------

#[test]
fn no_arguments_is_an_error() {
    assert_same(&[]);
}

#[test]
fn two_arguments_is_an_error() {
    assert_same(&[b"1", b"2"]);
}

#[test]
fn many_arguments_is_an_error() {
    assert_same(&[b"1", b"2", b"3"]);
    assert_same(&[b"", b"", b"", b"", b""]);
}

#[test]
fn argc_error_message_goes_to_stdout_not_stderr() {
    // The C code uses printf, so the diagnostic lands on stdout and stderr
    // stays empty. Pin that down explicitly.
    let c = run(c_bin(), &[]);
    assert_eq!(
        c.stdout,
        b"Error: should only be a single (integer) argument!\n"
    );
    assert!(c.stderr.is_empty(), "C wrote to stderr: {:?}", c.stderr);
    let r = run(&rust_bin(), &[]);
    assert_eq!(c.stdout, r.stdout);
    assert_eq!(c.stderr, r.stderr);
    assert_eq!(c.status.code(), r.status.code());
    assert_eq!(r.status.code(), Some(1));
}

// ---------------------------------------------------------------------------
// strtol branch: `if (end == argv[1])` -- nothing at all was parsed
// ---------------------------------------------------------------------------

#[test]
fn empty_argument_parses_nothing() {
    assert_same_str("");
}

#[test]
fn non_numeric_arguments_parse_nothing() {
    for a in [
        "abc", "xyz", "hello", ".5", "e5", "-", "+", "--5", "++5", "+-5", "-+5", "/", ":", "#", "~",
    ] {
        assert_same_str(a);
    }
}

#[test]
fn whitespace_only_arguments_parse_nothing() {
    // strtol skips leading whitespace, then finds no digits.
    for a in [" ", "  ", "\t", "\n", "\r", "\u{b}", "\u{c}", " \t\n\r\u{b}\u{c}"] {
        assert_same_str(a);
    }
    // Whitespace followed by a sign but no digit.
    for a in ["  -", "  +", "\t-", " + "] {
        assert_same_str(a);
    }
}

#[test]
fn parse_error_message_and_status() {
    let c = run(c_bin(), &[os_arg(b"abc")]);
    assert_eq!(c.stdout, b"Error: first argument must be an integer!\n");
    assert!(c.stderr.is_empty());
    assert_eq!(c.status.code(), Some(1));
    assert_same_str("abc");
}

// ---------------------------------------------------------------------------
// Happy path: the running-total loop
// ---------------------------------------------------------------------------

#[test]
fn zero_stride() {
    assert_same_str("0");
    assert_same_str("-0");
    assert_same_str("+0");
    assert_same_str("000");
}

#[test]
fn single_small_strides() {
    for a in ["1", "2", "3", "7", "10", "99", "12345"] {
        assert_same_str(a);
    }
}

#[test]
fn negative_strides() {
    for a in ["-1", "-2", "-7", "-99", "-12345"] {
        assert_same_str(a);
    }
}

#[test]
fn explicit_plus_sign_and_leading_zeros() {
    for a in ["+1", "+7", "007", "-007", "+007", "0000000000000009"] {
        assert_same_str(a);
    }
}

#[test]
fn leading_whitespace_is_skipped() {
    for a in ["  5", "\t5", "\n5", "\r5", " \t\n\u{b}\u{c}\r-8", "   +42"] {
        assert_same_str(a);
    }
}

#[test]
fn trailing_garbage_is_accepted_because_end_advanced() {
    // `end != argv[1]`, so the C code proceeds with the partially parsed value.
    for a in [
        "5abc", "0x10", "1e5", "12.75", "-12xyz", "  -12xyz", "7 8", "3,4", "9-", "42+",
    ] {
        assert_same_str(a);
    }
}

// ---------------------------------------------------------------------------
// Truncation of `long` to `int`, and saturation inside strtol
// ---------------------------------------------------------------------------

#[test]
fn int_boundary_strides() {
    for a in ["2147483647", "-2147483647", "-2147483648"] {
        assert_same_str(a);
    }
}

#[test]
fn long_to_int_truncation() {
    // strtol succeeds as a long, then `int stride = ...` truncates.
    for a in [
        "2147483648",
        "-2147483649",
        "4294967296",
        "4294967297",
        "-4294967296",
        "9999999999",
        "68719476736",
    ] {
        assert_same_str(a);
    }
}

#[test]
fn strtol_saturates_then_truncates() {
    for a in [
        "9223372036854775807",  // LONG_MAX exactly
        "9223372036854775808",  // overflow -> LONG_MAX
        "99999999999999999999", // overflow -> LONG_MAX -> int -1
        "-9223372036854775808", // LONG_MIN exactly
        "-9223372036854775809", // overflow -> LONG_MIN
        "-99999999999999999999",
        "18446744073709551616",
        "18446744073709551617",
        "999999999999999999999999999999999999999",
    ] {
        assert_same_str(a);
    }
}

// ---------------------------------------------------------------------------
// Signed overflow inside the loop (`i * stride` and the accumulating sum)
// ---------------------------------------------------------------------------

#[test]
fn multiplication_and_sum_overflow() {
    for a in [
        "1000000000",
        "2000000000",
        "2147483647",
        "-2147483648",
        "238609294", // sum of 0..9 times this is right at the int boundary
        "238609295",
        "-238609294",
        "-238609295",
        "477218588",
        "715827883",
    ] {
        assert_same_str(a);
    }
}

// ---------------------------------------------------------------------------
// Non-UTF-8 argv bytes
// ---------------------------------------------------------------------------

#[test]
fn non_utf8_arguments() {
    for a in [
        &b"\xff\xfe"[..],
        &b"\xff5"[..],
        &b"5\xff"[..],
        &b"-\x80"[..],
        &b"\x80\x81\x82"[..],
        &b"  \xc3\x28 7"[..],
        &b"\xed\xa0\x807"[..],
    ] {
        assert_same(&[a]);
    }
}

// ---------------------------------------------------------------------------
// Broad sweep: every short string over an interesting alphabet, plus a
// deterministic pseudo-random numeric sweep.
// ---------------------------------------------------------------------------

#[test]
fn exhaustive_short_strings_over_interesting_alphabet() {
    const ALPHA: &[u8] = b"0159+- \tax.";
    // All strings of length 0, 1 and 2 over the alphabet.
    assert_same(&[b""]);
    for &a in ALPHA {
        assert_same(&[&[a]]);
        for &b in ALPHA {
            assert_same(&[&[a, b]]);
        }
    }
}

#[test]
fn pseudo_random_numeric_sweep() {
    // Deterministic xorshift64* so the suite is reproducible.
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state.wrapping_mul(0x2545_F491_4F6C_DD1D)
    };

    for _ in 0..300 {
        // Values spanning well past the int range, both signs.
        let magnitude = next() % 20_000_000_000;
        let negative = next() & 1 == 1;
        let s = if negative {
            format!("-{magnitude}")
        } else {
            format!("{magnitude}")
        };
        assert_same_str(&s);
    }
}

#[test]
fn pseudo_random_junk_sweep() {
    const ALPHA: &[u8] = b"0123456789+- \tabxX.\n\r";
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state.wrapping_mul(0x2545_F491_4F6C_DD1D)
    };

    for _ in 0..400 {
        let len = (next() % 11) as usize;
        let bytes: Vec<u8> = (0..len)
            .map(|_| ALPHA[(next() % ALPHA.len() as u64) as usize])
            .collect();
        assert_same(&[&bytes]);
    }
}

// ---------------------------------------------------------------------------
// Output shape: exactly 10 lines, each newline-terminated, no stderr.
// ---------------------------------------------------------------------------

#[test]
fn happy_path_prints_ten_newline_terminated_lines() {
    let c = run(c_bin(), &[os_arg(b"3")]);
    let r = run(&rust_bin(), &[os_arg(b"3")]);
    assert_eq!(c.stdout, r.stdout);
    assert_eq!(c.stderr, r.stderr);
    assert_eq!(c.status.code(), r.status.code());
    assert_eq!(c.status.code(), Some(0));
    assert!(c.stderr.is_empty());
    assert!(c.stdout.ends_with(b"\n"));
    assert_eq!(c.stdout.iter().filter(|&&b| b == b'\n').count(), 10);
    // 0, 3, 9, 18, 30, 45, 63, 84, 108, 135
    assert_eq!(
        c.stdout,
        b"0\n3\n9\n18\n30\n45\n63\n84\n108\n135\n".to_vec()
    );
}
