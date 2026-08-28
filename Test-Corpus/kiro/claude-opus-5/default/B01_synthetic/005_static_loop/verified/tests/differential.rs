//! Differential tests: run the C `driver` and the Rust `driver` as
//! subprocesses with identical argv and compare stdout, stderr and exit
//! status byte for byte / code for code.
//!
//! The Rust program is never called as a library; `CARGO_BIN_EXE_driver`
//! gives the path to the compiled binary, which is spawned exactly the way a
//! shell would spawn it.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

/// Path to the Rust binary under test (built by cargo for this test target).
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Workspace root: the directory holding both `c_src/` and `translation/`.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the C binary, configuring/building it once per test run if needed.
///
/// Nothing under `c_src/` is modified; cmake only writes into `c_src/build/`.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = repo_root().join("c_src");
        let build = c_src.join("build");
        let bin = build.join("driver");
        if bin.is_file() {
            return bin;
        }

        std::fs::create_dir_all(&build).expect("could not create c_src/build");

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
            "C binary missing after build: {}",
            bin.display()
        );
        bin
    })
    .as_path()
}

fn run(program: &Path, args: &[OsString]) -> Output {
    Command::new(program)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", program.display()))
}

/// Exit status rendered so that a normal exit and a signal death never
/// compare equal by accident.
fn status_repr(out: &Output) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        match (out.status.code(), out.status.signal()) {
            (Some(c), _) => format!("exit={c}"),
            (None, Some(s)) => format!("signal={s}"),
            (None, None) => "unknown".to_string(),
        }
    }
    #[cfg(not(unix))]
    {
        format!("exit={:?}", out.status.code())
    }
}

fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).escape_debug().to_string()
}

fn show_args(args: &[OsString]) -> String {
    args.iter()
        .map(|a| format!("{:?}", a.to_string_lossy()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The single assertion used by every case: stdout, stderr and exit status
/// must all agree between the C program and the Rust program.
fn assert_same(args: &[OsString]) {
    let c = run(c_bin(), args);
    let r = run(&rust_bin(), args);
    let label = show_args(args);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch for argv [{label}]\n  C  : \"{}\"\n  Rust: \"{}\"",
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch for argv [{label}]\n  C  : \"{}\"\n  Rust: \"{}\"",
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        status_repr(&c),
        status_repr(&r),
        "exit status mismatch for argv [{label}]"
    );
}

fn args(list: &[&str]) -> Vec<OsString> {
    list.iter().map(OsString::from).collect()
}

fn check(list: &[&str]) {
    assert_same(&args(list));
}

/// Build an argv element from raw bytes, so arguments that are not valid
/// UTF-8 can be passed through exactly as a `char *` would carry them.
#[cfg(unix)]
fn raw_arg(bytes: &[u8]) -> OsString {
    use std::os::unix::ffi::OsStrExt;
    OsStr::from_bytes(bytes).to_os_string()
}

// ---------------------------------------------------------------------------
// Branch 1: `if (argc != 2)` -> "should only be a single (integer) argument!"
// ---------------------------------------------------------------------------

#[test]
fn argc_zero_extra_args() {
    check(&[]);
}

#[test]
fn argc_two_extra_args() {
    check(&["1", "2"]);
}

#[test]
fn argc_many_extra_args() {
    check(&["3", "4", "5", "6", "7", "8", "9", "10"]);
}

// ---------------------------------------------------------------------------
// Branch 2: `if (end == argv[1])` -> "first argument must be an integer!"
// strtol consumed nothing, so `end` was reset to the start of the string.
// ---------------------------------------------------------------------------

#[test]
fn empty_argument() {
    check(&[""]);
}

#[test]
fn non_numeric_argument() {
    check(&["abc"]);
}

#[test]
fn whitespace_only_argument() {
    // strtol skips leading whitespace, then finds no digits.
    check(&[" "]);
    check(&["\t"]);
    check(&["\n"]);
    check(&[" \t\r\n\u{0b}\u{0c}"]);
}

#[test]
fn sign_without_digits() {
    check(&["-"]);
    check(&["+"]);
    check(&["--5"]);
    check(&["+-3"]);
    check(&["- 5"]);
}

#[test]
fn leading_punctuation() {
    check(&[".5"]);
    check(&["e10"]);
    check(&["x10"]);
    check(&["/1"]);
    check(&[":1"]);
}

#[test]
fn non_ascii_leading_bytes() {
    check(&["é1"]);
    check(&["∞"]);
}

#[cfg(unix)]
#[test]
fn invalid_utf8_leading_byte() {
    // 0xff is neither space, sign nor digit: nothing is parsed.
    assert_same(&[raw_arg(b"\xff5")]);
    assert_same(&[raw_arg(b"\xc3\x28")]);
    assert_same(&[raw_arg(b"\x80")]);
}

// ---------------------------------------------------------------------------
// Happy path: 10 iterations of the running static total.
// ---------------------------------------------------------------------------

#[test]
fn single_item_stride_zero() {
    check(&["0"]);
}

#[test]
fn small_positive_strides() {
    for s in ["1", "2", "3", "7", "10", "100"] {
        check(&[s]);
    }
}

#[test]
fn negative_strides() {
    for s in ["-1", "-2", "-7", "-100"] {
        check(&[s]);
    }
}

#[test]
fn explicit_plus_sign() {
    check(&["+5"]);
    check(&["+0"]);
    check(&["-0"]);
}

#[test]
fn leading_zeros_are_base_ten() {
    // strtol with base 10 ignores any octal-looking prefix.
    check(&["012"]);
    check(&["000000012"]);
    check(&["0000000000000000000000005"]);
    check(&["-08"]);
}

#[test]
fn leading_whitespace_then_digits() {
    check(&["  12"]);
    check(&["\t-7"]);
    check(&["\n3"]);
    check(&["\t\n5"]);
    check(&[" \t\r\n\u{0b}\u{0c}42"]);
}

#[test]
fn trailing_garbage_is_ignored() {
    // `end` advances past the digits, so the check passes and the parsed
    // prefix is used.
    check(&["12abc"]);
    check(&["0x10"]);
    check(&["5 "]);
    check(&["5\n"]);
    check(&["3.9"]);
    check(&["7e3"]);
    check(&["1,000"]);
}

#[cfg(unix)]
#[test]
fn invalid_utf8_trailing_bytes() {
    assert_same(&[raw_arg(b"5\xff")]);
    assert_same(&[raw_arg(b"-9\x80\x80")]);
}

// ---------------------------------------------------------------------------
// Width, truncation and overflow: strtol returns long, `stride` is int, and
// `i * stride` plus the running `sum` are int arithmetic.
// ---------------------------------------------------------------------------

#[test]
fn int_boundaries() {
    check(&["2147483647"]); // INT_MAX
    check(&["-2147483648"]); // INT_MIN
    check(&["2147483646"]);
    check(&["-2147483647"]);
}

#[test]
fn long_to_int_truncation() {
    check(&["2147483648"]); // INT_MAX + 1 -> truncates to INT_MIN
    check(&["4294967296"]); // 2^32 -> truncates to 0
    check(&["4294967297"]); // 2^32 + 1 -> truncates to 1
    check(&["-4294967296"]);
    check(&["18446744073709551616"]);
}

#[test]
fn long_boundaries_and_saturation() {
    check(&["9223372036854775806"]);
    check(&["9223372036854775807"]); // LONG_MAX
    check(&["9223372036854775808"]); // overflows -> LONG_MAX
    check(&["-9223372036854775807"]);
    check(&["-9223372036854775808"]); // LONG_MIN
    check(&["-9223372036854775809"]); // overflows -> LONG_MIN
    check(&["99999999999999999999999999999999"]);
    check(&["-99999999999999999999999999999999"]);
    check(&[&"9".repeat(400)]);
    check(&[&format!("-{}", "9".repeat(400))]);
}

#[test]
fn stride_multiplication_overflow() {
    // i * stride exceeds INT_MAX for the larger values of i.
    check(&["238609294"]); // 9 * s is just under INT_MAX
    check(&["238609295"]); // 9 * s just over INT_MAX
    check(&["300000000"]);
    check(&["-300000000"]);
    check(&["1431655765"]);
    check(&["1431655766"]);
}

#[test]
fn running_sum_overflow() {
    // sum of i*stride over i in 0..10 is 45*stride, which overflows int well
    // before i*stride does.
    check(&["47721859"]); // 45 * s just under INT_MAX
    check(&["47721860"]); // 45 * s just over INT_MAX
    check(&["-47721860"]);
    check(&["715827882"]);
    check(&["715827883"]);
}

// ---------------------------------------------------------------------------
// A broad sweep, so a regression in any single value class is caught even if
// no hand-picked case above happens to hit it.
// ---------------------------------------------------------------------------

#[test]
fn sweep_of_many_values() {
    let mut cases: Vec<String> = Vec::new();
    for v in -40i64..=40 {
        cases.push(v.to_string());
    }
    for shift in 0..63 {
        let v = 1i64 << shift;
        cases.push(v.to_string());
        cases.push((-v).to_string());
        cases.push((v - 1).to_string());
    }
    for c in &cases {
        check(&[c.as_str()]);
    }
}
