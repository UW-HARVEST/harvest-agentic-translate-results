//! Differential tests: run the original C executable and the Rust executable as
//! subprocesses with identical argument vectors and require byte-identical
//! stdout, byte-identical stderr and the same exit status.
//!
//! The Rust code is never linked as a library here -- both programs are driven
//! exactly the way a shell would drive them, because that is how the
//! translation is graded.
//!
//! Note on `argv[0]`: the C program prints `argv[0]` in its usage message, and
//! the two executables live at different paths.  Every invocation therefore
//! sets `argv[0]` explicitly (via `CommandExt::arg0`) to the same string for
//! both programs, so a difference in output can only come from a difference in
//! behaviour.

use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::OsStringExt;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Output};

/// The `argv[0]` handed to both programs.
const ARGV0: &str = "driver";

/// Path of the executable built from `c_src/`.
fn c_binary() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // -> working directory root
    path.push("c_src");
    path.push("build");
    path.push("driver");
    assert!(
        path.is_file(),
        "the C executable is missing at {}.\n\
         Build it first:\n  cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .",
        path.display()
    );
    path
}

/// Path of the executable built from `translation/`.
fn rust_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn run(program: &PathBuf, args: &[OsString]) -> Output {
    Command::new(program)
        .arg0(OsStr::new(ARGV0))
        .args(args)
        .env_clear()
        .output()
        .unwrap_or_else(|e| panic!("failed to run {}: {e}", program.display()))
}

fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Run both programs with `args` and assert stdout, stderr and exit status all
/// match byte for byte.  Returns the (identical) stdout for further checks.
fn assert_same(args: &[OsString]) -> Vec<u8> {
    let c = run(&c_binary(), args);
    let r = run(&rust_binary(), args);

    let pretty: Vec<String> = args.iter().map(|a| format!("{a:?}")).collect();
    let label = format!("argv = [{ARGV0:?}, {}]", pretty.join(", "));

    assert_eq!(
        c.status.code(),
        r.status.code(),
        "{label}: exit status differs (C {:?} vs Rust {:?})\n\
         C stdout: {:?}\nRust stdout: {:?}\nC stderr: {:?}\nRust stderr: {:?}",
        c.status.code(),
        r.status.code(),
        show(&c.stdout),
        show(&r.stdout),
        show(&c.stderr),
        show(&r.stderr),
    );
    assert_eq!(
        c.stdout,
        r.stdout,
        "{label}: stdout differs\nC:    {:?}\nRust: {:?}",
        show(&c.stdout),
        show(&r.stdout),
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "{label}: stderr differs\nC:    {:?}\nRust: {:?}",
        show(&c.stderr),
        show(&r.stderr),
    );

    c.stdout
}

fn osargs(args: &[&str]) -> Vec<OsString> {
    args.iter().map(OsString::from).collect()
}

/// Convenience wrapper for the common "exactly one argument" case.
fn assert_same_seed(arg: &str) -> Vec<u8> {
    assert_same(&osargs(&[arg]))
}

// ---------------------------------------------------------------------------
// argc validation:  if (argc != 2) -> usage on stderr, exit 1
// ---------------------------------------------------------------------------

#[test]
fn no_arguments_prints_usage() {
    let out = assert_same(&[]);
    assert!(out.is_empty(), "usage path must not write to stdout");

    // Pin the exact text/exit status so a regression cannot make both
    // programs wrong in the same way.
    let c = run(&c_binary(), &[]);
    assert_eq!(c.stderr, b"Usage: driver <seed>\n");
    assert_eq!(c.status.code(), Some(1));
}

#[test]
fn two_arguments_prints_usage() {
    assert_same(&osargs(&["42", "43"]));
}

#[test]
fn many_arguments_prints_usage() {
    assert_same(&osargs(&["1", "2", "3", "4", "5"]));
}

#[test]
fn empty_extra_argument_still_counts() {
    // argc == 3 even though the second argument is empty.
    assert_same(&osargs(&["42", ""]));
}

// ---------------------------------------------------------------------------
// Seed validation:  *endptr != '\0' || errno != 0 || temp_seed > UINT_MAX
// ---------------------------------------------------------------------------

#[test]
fn rejects_non_numeric_seeds() {
    // No conversion performed at all: endptr == nptr.
    for arg in [
        "abc", " ", "\t", "\n", "\r", "\x0b", "\x0c", "+", "-", "+-1", "--1", "x", "  ", " +",
        "+ 1", "- 1",
    ] {
        let out = assert_same_seed(arg);
        assert!(out.is_empty(), "{arg:?} must not print to stdout");
    }
}

#[test]
fn rejects_trailing_garbage() {
    // Partial conversion: endptr stops before the end of the string.
    // NOTE: an argument containing an embedded NUL is impossible -- execve()
    // cannot pass one -- so there is nothing to compare for that case.
    for arg in [
        "12abc", "42 ", "1.5", "0x10", "7\n", "1,000", "42\t", "9-9", "\x0b42x", "0b1", "1e3",
    ] {
        assert_same_seed(arg);
    }
}

#[test]
fn rejects_negative_seeds() {
    // strtoul() negates modulo 2^64, so these become huge values that fail the
    // `> UINT_MAX` check rather than the endptr check.
    for arg in ["-1", "-42", "-4294967295", "-4294967296", "-2147483648"] {
        assert_same_seed(arg);
    }
}

#[test]
fn rejects_values_above_uint_max() {
    for arg in [
        "4294967296",
        "4294967297",
        "5000000000",
        "18446744073709551614",
        "18446744073709551615", // ULONG_MAX: converts fine, no ERANGE
    ] {
        assert_same_seed(arg);
    }
}

#[test]
fn rejects_erange_overflow() {
    // Overflow of unsigned long: strtoul sets ERANGE and returns ULONG_MAX.
    for arg in [
        "18446744073709551616",
        "99999999999999999999999999",
        "-18446744073709551616",
        "-99999999999999999999999999",
    ] {
        assert_same_seed(arg);
    }
}

#[test]
fn rejects_very_long_digit_string() {
    let arg = "9".repeat(4096);
    assert_same_seed(&arg);
}

#[test]
fn error_message_echoes_raw_bytes() {
    // The C program prints the argument with %s, so invalid UTF-8 must be
    // reproduced verbatim.
    let cases: Vec<Vec<u8>> = vec![
        vec![0xff, 0xfe],
        vec![b'4', b'2', 0x80],
        vec![0xc3],
        vec![0xf0, 0x9f, 0x92, 0xa9],
    ];
    for bytes in cases {
        assert_same(&[OsString::from_vec(bytes)]);
    }
}

// ---------------------------------------------------------------------------
// Accepted seeds.  These run the full 2000 x 100 x 256Ki arithmetic kernel in
// both programs, so each one takes a while -- that is inherent to the C code.
// ---------------------------------------------------------------------------

/// Each accepted seed is exercised by exactly one test with exactly one
/// invocation per program, to keep the (unavoidably long) runtime down.
/// `expected` pins the value the C program produces, so the tests also catch a
/// regression that would break both programs in the same way.
fn assert_accepted(arg: &str, expected: &str) {
    let out = assert_same_seed(arg);
    assert_eq!(
        String::from_utf8_lossy(&out),
        expected,
        "stdout for seed argument {arg:?}"
    );
}

/// Values produced by the C executable (`c_src/build/driver <seed>`).
/// srand(0): glibc substitutes 1 for a zero seed, so seeds 0 and 1 agree.
const XOR_SEED_0: &str = "42032659\n";
const XOR_SEED_42: &str = "430392287\n";
const XOR_SEED_UINT_MAX: &str = "494145113\n";
const XOR_SEED_2147483648: &str = "269448949\n";

#[test]
fn accepts_zero_seed() {
    assert_accepted("0", XOR_SEED_0);
}

#[test]
fn accepts_empty_string_as_zero_seed() {
    // Quirk of the C code: strtoul("") performs no conversion, leaves endptr
    // pointing at the start of the string (which is the terminating NUL) and
    // does not touch errno, so `*endptr == '\0'` holds and the empty string is
    // accepted as seed 0.
    assert_accepted("", XOR_SEED_0);
}

#[test]
fn accepts_negative_zero_seed() {
    assert_accepted("-0", XOR_SEED_0);
}

#[test]
fn accepts_negative_wraparound_seed() {
    // strtoul("-18446744073709551615") == 1 (negation modulo 2^64), which
    // passes every check -- and srand(1) is what srand(0) does too.
    assert_accepted("-18446744073709551615", XOR_SEED_0);
}

#[test]
fn accepts_plain_seed() {
    assert_accepted("42", XOR_SEED_42);
}

#[test]
fn accepts_leading_whitespace_plus_and_zero_padding() {
    // Every character isspace() accepts in the C locale, then '+', then zero
    // padding: strtoul() skips all of it and yields 42.
    assert_accepted("\t\n\x0b\x0c\r +0000000042", XOR_SEED_42);
}

#[test]
fn accepts_uint_max_seed() {
    assert_accepted("4294967295", XOR_SEED_UINT_MAX);
}

#[test]
fn accepts_seed_above_int_max() {
    // 2147483648 > INT_MAX but <= UINT_MAX: exercises the unsigned cast.
    assert_accepted("2147483648", XOR_SEED_2147483648);
}
