//! Differential tests: run the original C program and the Rust translation as
//! subprocesses with identical arguments and require byte-identical stdout,
//! byte-identical stderr and an identical exit status (including death by
//! signal).
//!
//! Nothing here links the Rust code as a library — both programs are driven
//! exactly the way a shell would drive them, because that is how the
//! translation is graded.

use std::ffi::{OsStr, OsString};
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// Build an `argv` tail (`&[&[u8]]`) out of byte-string literals, `Vec<u8>`s or
/// anything else indexable by a range.
macro_rules! argv {
    () => { &[] as &[&[u8]] };
    ($($a:expr),+ $(,)?) => { &[$( &$a[..] as &[u8] ),+] as &[&[u8]] };
}

// ---------------------------------------------------------------------------
// Locating the two executables
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The Rust executable under test.
///
/// `RUST_DRIVER_BIN` allows pointing the suite at another build (for example
/// `target/release/driver`); by default the binary cargo just built for this
/// test run is used.
fn rust_bin() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| match std::env::var_os("RUST_DRIVER_BIN") {
        Some(p) => PathBuf::from(p),
        None => PathBuf::from(env!("CARGO_BIN_EXE_driver")),
    })
    .as_path()
}

/// The C executable, built out of `c_src/` with CMake exactly as the project
/// documents. It is configured/built on demand so that a bare `cargo test`
/// works in a fresh checkout.
fn c_bin() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let c_src = manifest_dir()
            .parent()
            .expect("translation/ must have a parent directory")
            .join("c_src");
        let build_dir = c_src.join("build");
        let exe = build_dir.join("driver");

        if !exe.exists() {
            std::fs::create_dir_all(&build_dir).expect("cannot create c_src/build");

            let configure = Command::new("cmake")
                .arg("..")
                .current_dir(&build_dir)
                .output()
                .expect("failed to run `cmake ..` — is cmake installed?");
            assert!(
                configure.status.success(),
                "cmake configure failed:\n{}\n{}",
                String::from_utf8_lossy(&configure.stdout),
                String::from_utf8_lossy(&configure.stderr)
            );

            let build = Command::new("cmake")
                .args(["--build", "."])
                .current_dir(&build_dir)
                .output()
                .expect("failed to run `cmake --build .`");
            assert!(
                build.status.success(),
                "cmake build failed:\n{}\n{}",
                String::from_utf8_lossy(&build.stdout),
                String::from_utf8_lossy(&build.stderr)
            );
        }

        assert!(
            exe.exists(),
            "the C executable was not produced at {}",
            exe.display()
        );
        exe
    })
    .as_path()
}

// ---------------------------------------------------------------------------
// Running a program and capturing everything observable
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq)]
struct Observed {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Some(n)` when the process exited normally with status `n`.
    code: Option<i32>,
    /// `Some(sig)` when the process was killed by signal `sig`.
    signal: Option<i32>,
}

impl std::fmt::Debug for Observed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Observed")
            .field("stdout", &String::from_utf8_lossy(&self.stdout))
            .field("stdout_bytes", &self.stdout)
            .field("stderr", &String::from_utf8_lossy(&self.stderr))
            .field("stderr_bytes", &self.stderr)
            .field("code", &self.code)
            .field("signal", &self.signal)
            .finish()
    }
}

fn run(exe: &Path, args: &[OsString], stdin_data: &[u8]) -> Observed {
    let mut child = Command::new(exe)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));

    {
        let mut stdin = child.stdin.take().expect("piped stdin");
        // The programs never read stdin; ignore a broken pipe / dead child.
        let _ = stdin.write_all(stdin_data);
        let _ = stdin.flush();
    }

    let out = child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("failed to wait for {}: {e}", exe.display()));

    Observed {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

fn to_os_args(args: &[&[u8]]) -> Vec<OsString> {
    args.iter()
        .map(|a| OsStr::from_bytes(a).to_os_string())
        .collect()
}

/// Assert the C program and the Rust program are indistinguishable for `args`.
fn assert_same_with_stdin(label: &str, args: &[&[u8]], stdin_data: &[u8]) {
    let os_args = to_os_args(args);
    let c = run(c_bin(), &os_args, stdin_data);
    let r = run(rust_bin(), &os_args, stdin_data);

    let pretty = args
        .iter()
        .map(|a| format!("{:?}", String::from_utf8_lossy(a)))
        .collect::<Vec<_>>()
        .join(", ");

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch for {label} (argv = [{pretty}])\n C: {:?}\n R: {:?}",
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch for {label} (argv = [{pretty}])\n C: {:?}\n R: {:?}",
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "exit status mismatch for {label} (argv = [{pretty}])\n C: code={:?} signal={:?}\n R: code={:?} signal={:?}",
        c.code,
        c.signal,
        r.code,
        r.signal
    );

    // Everything observable, compared in one shot as a backstop.
    assert_eq!(c, r, "observable behaviour mismatch for {label}");
}

fn assert_same(label: &str, args: &[&[u8]]) {
    assert_same_with_stdin(label, args, b"");
}

const SIGSEGV: i32 = 11;

// ---------------------------------------------------------------------------
// Phase A — both programs exist and are runnable
// ---------------------------------------------------------------------------

#[test]
fn both_executables_are_runnable() {
    let c = c_bin();
    let r = rust_bin();
    assert!(c.exists(), "C executable missing: {}", c.display());
    assert!(r.exists(), "Rust executable missing: {}", r.display());

    // A trivially valid invocation must succeed on both sides.
    let args = to_os_args(argv!(b"1", b"2"));
    let cr = run(c, &args, b"");
    let rr = run(r, &args, b"");
    assert_eq!(cr.code, Some(0), "C did not exit 0 for `driver 1 2`: {cr:?}");
    assert_eq!(rr.code, Some(0), "Rust did not exit 0 for `driver 1 2`: {rr:?}");
    assert_eq!(cr.stdout, b"3\n", "unexpected C stdout: {cr:?}");
    assert_eq!(cr, rr);
}

// ---------------------------------------------------------------------------
// Phase B — the argument-count branches the C program has
// ---------------------------------------------------------------------------
//
// main() reads argv[1] and argv[2] with no argc check. With no arguments,
// argv[1] is NULL and atoi() dereferences it, so the process dies with SIGSEGV
// having produced no output. With exactly one argument the same happens on
// argv[2], *after* argv[1] parsed fine — still no output, because nothing is
// printed until both parses complete.

#[test]
fn no_arguments_null_derefs_in_atoi() {
    assert_same("no arguments", argv!());
}

#[test]
fn one_argument_null_derefs_on_argv2() {
    assert_same("one argument", argv!(b"1"));
    assert_same("one argument, garbage", argv!(b"not-a-number"));
    assert_same("one argument, empty", argv!(b""));
    assert_same("one argument, overflowing", argv!(b"99999999999999999999"));
}

#[test]
fn missing_arguments_die_by_signal_not_clean_exit() {
    // Guards against the Rust side "helpfully" exiting 0 or 1 where C faults.
    for args in [argv!(), argv!(b"1")] {
        let os_args = to_os_args(args);
        let c = run(c_bin(), &os_args, b"");
        let r = run(rust_bin(), &os_args, b"");
        assert_eq!(c.code, None, "C unexpectedly exited normally: {c:?}");
        assert_eq!(r.code, None, "Rust exited normally where C faulted: {r:?}");
        assert_eq!(c.signal, Some(SIGSEGV), "C did not die by SIGSEGV: {c:?}");
        assert_eq!(r.signal, c.signal, "signal mismatch: C {c:?} vs Rust {r:?}");
        assert!(c.stdout.is_empty() && r.stdout.is_empty(), "stdout not empty");
        assert!(c.stderr.is_empty() && r.stderr.is_empty(), "stderr not empty");
    }
}

#[test]
fn two_arguments_happy_path() {
    assert_same("1 2", argv!(b"1", b"2"));
    assert_same("0 0", argv!(b"0", b"0"));
    assert_same("negatives", argv!(b"-5", b"-7"));
    assert_same("mixed signs", argv!(b"-5", b"7"));
    assert_same("large-ish", argv!(b"123456", b"654321"));
}

#[test]
fn extra_arguments_are_ignored() {
    assert_same("three arguments", argv!(b"1", b"2", b"3"));
    assert_same("many arguments", argv!(b"1", b"2", b"3", b"4", b"5", b"6"));
    assert_same("extra garbage args", argv!(b"7", b"8", b"", b"-", b"zzz"));
}

// ---------------------------------------------------------------------------
// Phase B — printf("%d\n", ...) formatting and int arithmetic
// ---------------------------------------------------------------------------

#[test]
fn sum_wraps_like_c_int_arithmetic() {
    assert_same("INT_MAX + 1", argv!(b"2147483647", b"1"));
    assert_same("1 + INT_MAX", argv!(b"1", b"2147483647"));
    assert_same("INT_MIN + -1", argv!(b"-2147483648", b"-1"));
    assert_same("INT_MAX + INT_MAX", argv!(b"2147483647", b"2147483647"));
    assert_same("INT_MIN + INT_MIN", argv!(b"-2147483648", b"-2147483648"));
    assert_same("INT_MAX + INT_MIN", argv!(b"2147483647", b"-2147483648"));
    assert_same("INT_MIN + INT_MAX", argv!(b"-2147483648", b"2147483647"));
    assert_same("INT_MAX + 2", argv!(b"2147483647", b"2"));
    assert_same("cancels to zero", argv!(b"2147483647", b"-2147483647"));
}

#[test]
fn zero_and_signed_zero_printing() {
    assert_same("-0 and -0", argv!(b"-0", b"-0"));
    assert_same("+0 and -0", argv!(b"+0", b"-0"));
    assert_same("5 and -5", argv!(b"5", b"-5"));
}

// ---------------------------------------------------------------------------
// Phase C — every atoi()/strtol() parsing branch
// ---------------------------------------------------------------------------

#[test]
fn atoi_empty_and_non_numeric_yield_zero() {
    assert_same("both empty", argv!(b"", b""));
    assert_same("empty and number", argv!(b"", b"42"));
    assert_same("number and empty", argv!(b"42", b""));
    assert_same("alphabetic", argv!(b"abc", b"xyz"));
    assert_same("punctuation", argv!(b"!!!", b"???"));
    assert_same("sign only", argv!(b"-", b"-"));
    assert_same("plus only", argv!(b"+", b"+"));
    assert_same("double sign", argv!(b"--5", b"++6"));
    assert_same("sign then sign", argv!(b"+-5", b"-+6"));
    assert_same("sign then space", argv!(b"- 5", b"+ 6"));
}

#[test]
fn atoi_skips_leading_c_locale_whitespace() {
    assert_same("spaces", argv!(b"   42", b"  -8"));
    assert_same("tab", argv!(b"\t7", b"\t-7"));
    assert_same("newline", argv!(b"\n9", b"\n-9"));
    assert_same("vertical tab", argv!(b"\x0b5", b"\x0b6"));
    assert_same("form feed", argv!(b"\x0c5", b"\x0c6"));
    assert_same("carriage return", argv!(b"\r5", b"\r6"));
    assert_same("all whitespace kinds", argv!(b" \t\n\x0b\x0c\r-123", b" \t\r+4"));
    assert_same("whitespace only", argv!(b"    ", b"\t\n"));
    assert_same("whitespace after digits", argv!(b"5   ", b"6\t"));
    assert_same("whitespace inside digits", argv!(b"1 2", b"3\t4"));
    // Not whitespace in the C locale: U+00A0 encoded as UTF-8.
    assert_same("non-breaking space", argv!(b"\xc2\xa07", b"8"));
}

#[test]
fn atoi_stops_at_first_non_digit() {
    assert_same("digits then letters", argv!(b"12abc", b"34xyz"));
    assert_same("decimal point", argv!(b"3.9", b"2.9"));
    assert_same("exponent notation", argv!(b"1e3", b"2e1"));
    assert_same("thousands comma", argv!(b"1,000", b"2,000"));
    assert_same("underscore separator", argv!(b"1_000", b"2_000"));
    assert_same("hex literal", argv!(b"0x10", b"0X10"));
    assert_same("leading zeros are decimal", argv!(b"010", b"007"));
    assert_same("many leading zeros", argv!(b"0000000000000000000005", b"0"));
    assert_same("trailing sign", argv!(b"5-", b"6+"));
    assert_same("digits then whitespace junk", argv!(b"9\tjunk", b"1"));
}

#[test]
fn atoi_truncates_the_long_result_to_int() {
    // strtol returns long; atoi casts to int, so values above 2^31 wrap.
    assert_same("2^31", argv!(b"2147483648", b"0"));
    assert_same("-(2^31 + 1)", argv!(b"-2147483649", b"0"));
    assert_same("2^32 - 1", argv!(b"4294967295", b"0"));
    assert_same("2^32", argv!(b"4294967296", b"0"));
    assert_same("2^32 + 5", argv!(b"4294967301", b"0"));
    assert_same("-(2^32)", argv!(b"-4294967296", b"0"));
    assert_same("2^33 + 7", argv!(b"8589934599", b"0"));
    assert_same("LONG_MAX - 1", argv!(b"9223372036854775806", b"0"));
}

#[test]
fn atoi_saturates_long_on_overflow_then_truncates() {
    // strtol clamps to LONG_MAX / LONG_MIN; the int cast then yields -1 / 0.
    assert_same("LONG_MAX", argv!(b"9223372036854775807", b"0"));
    assert_same("LONG_MIN", argv!(b"-9223372036854775808", b"0"));
    assert_same("LONG_MAX + 1", argv!(b"9223372036854775808", b"0"));
    assert_same("LONG_MIN - 1", argv!(b"-9223372036854775809", b"0"));
    assert_same("2^64", argv!(b"18446744073709551616", b"0"));
    assert_same("20 nines", argv!(b"99999999999999999999", b"0"));
    assert_same("negative 20 nines", argv!(b"-99999999999999999999", b"0"));
    assert_same(
        "both overflow",
        argv!(b"99999999999999999999", b"99999999999999999999"),
    );
    // saturated positive truncates to -1, so -1 + 1 == 0
    assert_same("overflow plus one", argv!(b"99999999999999999999", b"1"));
    // saturated negative truncates to 0, so 0 + 1 == 1
    assert_same(
        "negative overflow plus one",
        argv!(b"-99999999999999999999", b"1"),
    );
    assert_same(
        "overflow with leading zeros",
        argv!(b"0000009223372036854775808", b"0"),
    );
    assert_same(
        "overflow after whitespace/sign",
        argv!(b"  +99999999999999999999", b"0"),
    );
}

#[test]
fn atoi_handles_very_long_digit_runs() {
    let nines_1k = vec![b'9'; 1000];
    let zeros_then_seven = {
        let mut v = vec![b'0'; 1000];
        v.push(b'7');
        v
    };
    let ones_100k = vec![b'1'; 100_000];
    assert_same("1000 nines", argv!(nines_1k, b"1"));
    assert_same("1000 zeros then 7", argv!(zeros_then_seven, b"1"));
    assert_same("100000 ones", argv!(ones_100k, b"1"));
    assert_same("100000 ones twice", argv!(ones_100k, ones_100k));
}

#[test]
fn arguments_need_not_be_valid_utf8() {
    assert_same("0xff prefix", argv!(b"\xff5", b"6\xfe"));
    assert_same("lone continuation byte", argv!(b"\x805", b"\x816"));
    assert_same("digits then raw bytes", argv!(b"12\xc3\x28", b"34\xed\xa0\x80"));
    assert_same("all high bytes", argv!(b"\xf0\xf1\xf2", b"\xf3\xf4\xf5"));
    assert_same("one invalid-utf8 argument only", argv!(b"\xff"));
}

#[test]
fn arguments_that_look_like_flags_are_parsed_as_numbers() {
    assert_same("double dash", argv!(b"--", b"5"));
    assert_same("dash h", argv!(b"-h", b"5"));
    assert_same("dash dash help", argv!(b"--help", b"5"));
    assert_same("dash 5 and dash dash 3", argv!(b"-5", b"--3"));
}

#[test]
fn stdin_is_never_read() {
    // The C program takes no input from stdin; feeding it data must change
    // nothing on either side.
    assert_same_with_stdin("stdin ignored, valid args", argv!(b"1", b"2"), b"99 99\n");
    assert_same_with_stdin("stdin ignored, no args", argv!(), b"1 2\n");
    assert_same_with_stdin("stdin ignored, one arg", argv!(b"4"), b"5\n");
    assert_same_with_stdin("large stdin ignored", argv!(b"7", b"8"), &vec![b'x'; 65_536]);
}

// ---------------------------------------------------------------------------
// Phase C — sweeps, so no arithmetic/formatting corner is left untried
// ---------------------------------------------------------------------------

#[test]
fn numeric_edge_matrix() {
    const EDGES: [&str; 17] = [
        "0",
        "1",
        "-1",
        "2",
        "-2",
        "32767",
        "-32768",
        "65535",
        "2147483646",
        "2147483647",
        "-2147483647",
        "-2147483648",
        "2147483648",
        "4294967295",
        "4294967296",
        "9223372036854775807",
        "-9223372036854775808",
    ];
    for a in EDGES {
        for b in EDGES {
            let label = format!("{a} + {b}");
            assert_same(&label, argv!(a.as_bytes(), b.as_bytes()));
        }
    }
}

#[test]
fn deterministic_pseudo_random_sweep() {
    // Small xorshift PRNG so the case list is stable across runs.
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    const ALPHABET: &[u8] = b"0123456789+- \t\n\r\x0b\x0cabcXZ.,_e\x80\xff";

    for _ in 0..300 {
        let argc = (next() % 4) as usize; // 0..=3 arguments
        let mut owned: Vec<Vec<u8>> = Vec::with_capacity(argc);
        for _ in 0..argc {
            let len = (next() % 15) as usize;
            let mut s = Vec::with_capacity(len);
            for _ in 0..len {
                s.push(ALPHABET[(next() % ALPHABET.len() as u64) as usize]);
            }
            owned.push(s);
        }
        let refs: Vec<&[u8]> = owned.iter().map(|v| v.as_slice()).collect();
        let label = format!(
            "random argv {:?}",
            owned
                .iter()
                .map(|v| String::from_utf8_lossy(v).into_owned())
                .collect::<Vec<_>>()
        );
        assert_same(&label, &refs);
    }
}
