//! Differential tests: run the C `driver` and the Rust `driver` as subprocesses
//! with identical argv and require byte-identical stdout, byte-identical stderr
//! and an identical exit status.
//!
//! The Rust program is never called as a library; both sides are driven exactly
//! the way a shell would drive them.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Path to the Rust binary under test, as built by cargo for this test run.
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Workspace root (the directory holding `c_src/` and `translation/`).
fn root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the compiled C binary, building it with cmake on first use.
fn c_bin() -> PathBuf {
    let c_src = root().join("c_src");
    let build = c_src.join("build");
    let exe = build.join("driver");
    if exe.is_file() {
        return exe;
    }

    std::fs::create_dir_all(&build).expect("failed to create c_src/build");

    let configure = Command::new("cmake")
        .arg("..")
        .current_dir(&build)
        .output()
        .expect("failed to run `cmake ..` -- is cmake installed?");
    assert!(
        configure.status.success(),
        "cmake configure failed:\n{}\n{}",
        String::from_utf8_lossy(&configure.stdout),
        String::from_utf8_lossy(&configure.stderr),
    );

    let compile = Command::new("cmake")
        .arg("--build")
        .arg(".")
        .current_dir(&build)
        .output()
        .expect("failed to run `cmake --build .`");
    assert!(
        compile.status.success(),
        "cmake build failed:\n{}\n{}",
        String::from_utf8_lossy(&compile.stdout),
        String::from_utf8_lossy(&compile.stderr),
    );

    assert!(
        exe.is_file(),
        "C binary missing after build: {}",
        exe.display()
    );
    exe
}

fn run(bin: &Path, args: &[OsString]) -> Output {
    Command::new(bin)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()))
}

fn show(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => format!("{s:?}"),
        Err(_) => format!("{bytes:?}"),
    }
}

fn show_args(args: &[OsString]) -> String {
    let parts: Vec<String> = args.iter().map(|a| show(os_bytes(a))).collect();
    format!("[{}]", parts.join(", "))
}

#[cfg(unix)]
fn os_bytes(s: &OsStr) -> &[u8] {
    use std::os::unix::ffi::OsStrExt;
    s.as_bytes()
}

#[cfg(not(unix))]
fn os_bytes(s: &OsStr) -> &[u8] {
    s.to_str().map(|v| v.as_bytes()).unwrap_or(b"<non-utf8>")
}

/// The core assertion: same argv => same stdout bytes, stderr bytes, exit status.
fn assert_same(args: &[OsString]) {
    let c = run(&c_bin(), args);
    let r = run(&rust_bin(), args);
    let label = show_args(args);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch for argv {label}\n  C:    {}\n  Rust: {}",
        show(&c.stdout),
        show(&r.stdout),
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch for argv {label}\n  C:    {}\n  Rust: {}",
        show(&c.stderr),
        show(&r.stderr),
    );
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "exit status mismatch for argv {label}\n  C:    {:?}\n  Rust: {:?}",
        c.status,
        r.status,
    );
}

fn check(args: &[&str]) {
    let owned: Vec<OsString> = args.iter().map(OsString::from).collect();
    assert_same(&owned);
}

// ---------------------------------------------------------------------------
// Branch: `argc != 2` -> "Error: should only be a single (integer) argument!"
// ---------------------------------------------------------------------------

#[test]
fn argc_zero_extra_args() {
    // argc == 1: no argument at all.
    check(&[]);
}

#[test]
fn argc_two_extra_args() {
    // argc == 3 with two parsable integers: still the arity error.
    check(&["1", "2"]);
}

#[test]
fn argc_many_extra_args() {
    check(&["1", "2", "3"]);
    check(&["a", "b", "c"]);
    // argc == 3 where the first arg alone would have been valid.
    check(&["5", ""]);
    // A long argv list.
    check(&["1", "1", "1", "1", "1", "1", "1", "1", "1", "1", "1", "1"]);
}

// ---------------------------------------------------------------------------
// Branch: `end == argv[1]` -> "Error: first argument must be an integer!"
// (strtol performed no conversion)
// ---------------------------------------------------------------------------

#[test]
fn no_conversion_empty_string() {
    check(&[""]);
}

#[test]
fn no_conversion_whitespace_only() {
    check(&["   "]);
    check(&["\t"]);
    check(&["\n"]);
    check(&["\r"]);
    check(&["\u{0b}"]); // vertical tab
    check(&["\u{0c}"]); // form feed
    check(&[" \t\n\u{0b}\u{0c}\r"]); // every isspace() char in the C locale
}

#[test]
fn no_conversion_sign_only() {
    check(&["+"]);
    check(&["-"]);
    check(&["  +"]);
    check(&["  -"]);
    check(&["++5"]);
    check(&["--5"]);
    check(&["+-5"]);
    check(&["-+5"]);
}

#[test]
fn no_conversion_non_numeric() {
    check(&["abc"]);
    check(&["x5"]);
    check(&["."]);
    check(&[".5"]);
    check(&["-.5"]);
    check(&["e5"]);
    check(&["#"]);
    check(&["  -  5"]); // space between sign and digits: no conversion
    // NOTE: a NUL byte cannot appear in argv at all (execve terminates each
    // argument at the first NUL), so there is no such input class to test.
    check(&["nan"]);
    check(&["inf"]);
    check(&["null"]);
    check(&["/"]); // byte just below '0'
    check(&[":"]); // byte just above '9'
}

#[test]
fn no_conversion_non_ascii_digits() {
    // Unicode digits are not isdigit() in the C locale.
    check(&["٥"]); // ARABIC-INDIC DIGIT FIVE
    check(&["５"]); // FULLWIDTH DIGIT FIVE
}

// ---------------------------------------------------------------------------
// Happy path: the ten-iteration running-total loop
// ---------------------------------------------------------------------------

#[test]
fn happy_path_small_values() {
    for a in [
        "0", "1", "2", "3", "-1", "-2", "5", "-3", "7", "10", "-10", "100", "-100",
    ] {
        check(&[a]);
    }
}

#[test]
fn happy_path_signs_and_padding() {
    check(&["+5"]);
    check(&["-0"]);
    check(&["+0"]);
    check(&["007"]);
    check(&["-007"]);
    check(&["+007"]);
    check(&["0000000000000000000000000000000000000000007"]);
    check(&["-0000000000000000000000000000000000000000007"]);
}

#[test]
fn happy_path_leading_whitespace_is_skipped() {
    check(&[" 7"]);
    check(&["\t9"]);
    check(&["\n5"]);
    check(&["\r5"]);
    check(&["\u{0b}5"]);
    check(&["\u{0c}5"]);
    check(&["   \t\n  -12"]);
}

#[test]
fn happy_path_trailing_garbage_is_ignored() {
    // strtol stops at the first non-digit; `end != argv[1]`, so no error.
    check(&["5abc"]);
    check(&["5.9"]);
    check(&["5 6"]);
    check(&["  -42xyz"]);
    check(&["0x10"]); // base 10: parses "0", stops at 'x'
    check(&["0X10"]);
    check(&["1e9"]);
    check(&["3_000"]);
    check(&["12,345"]);
    check(&["7\n"]);
    check(&["7 "]);
    check(&["-0trailing"]);
}

// ---------------------------------------------------------------------------
// long -> int truncation of strtol's result
// ---------------------------------------------------------------------------

#[test]
fn long_to_int_truncation() {
    check(&["2147483647"]); // INT_MAX
    check(&["-2147483648"]); // INT_MIN
    check(&["2147483648"]); // INT_MAX + 1 -> truncates
    check(&["-2147483649"]); // INT_MIN - 1 -> truncates
    check(&["4294967296"]); // 2^32 -> 0
    check(&["4294967295"]); // 2^32 - 1 -> -1
    check(&["-4294967296"]);
    check(&["8589934592"]); // 2^33 -> 0
    check(&["4294967301"]); // 2^32 + 5 -> 5
    check(&["-4294967301"]);
    check(&["1099511627776"]);
}

// ---------------------------------------------------------------------------
// strtol range clamping (ERANGE) followed by truncation
// ---------------------------------------------------------------------------

#[test]
fn strtol_range_clamping() {
    check(&["9223372036854775807"]); // LONG_MAX exactly
    check(&["-9223372036854775808"]); // LONG_MIN exactly
    check(&["9223372036854775808"]); // LONG_MAX + 1 -> clamps to LONG_MAX
    check(&["-9223372036854775809"]); // LONG_MIN - 1 -> clamps to LONG_MIN
    check(&["99999999999999999999"]);
    check(&["-99999999999999999999"]);
    check(&["18446744073709551616"]); // 2^64
    check(&["+9223372036854775808garbage"]);
}

#[test]
fn strtol_range_clamping_very_long_inputs() {
    let long_pos: String = "9".repeat(5000);
    let long_neg = format!("-{long_pos}");
    check(&[&long_pos]);
    check(&[&long_neg]);

    // Enough leading zeros to exceed any accumulator, but the value is tiny.
    let padded = format!("{}7", "0".repeat(5000));
    check(&[&padded]);
    check(&[&format!("-{padded}")]);
}

// ---------------------------------------------------------------------------
// `int` arithmetic in the loop: `i * stride` and the running `sum`
// ---------------------------------------------------------------------------

#[test]
fn int_multiply_overflow_in_loop() {
    // i * stride exceeds INT_MAX for the later iterations.
    check(&["2000000000"]);
    check(&["-2000000000"]);
    check(&["2147483647"]);
    check(&["-2147483648"]);
    check(&["1000000000"]);
    check(&["-1000000000"]);
    check(&["300000000"]);
    check(&["268435456"]); // 2^28
    check(&["1073741824"]); // 2^30
}

#[test]
fn running_sum_overflow_boundary() {
    // sum after 10 iterations is 45 * stride; probe right at the INT_MAX edge.
    check(&["47721858"]);
    check(&["47721859"]); // 45 * 47721859 > INT_MAX
    check(&["-47721858"]);
    check(&["-47721859"]);
    check(&["477218589"]);
    check(&["477218588"]);
    check(&["-477218589"]);
}

// ---------------------------------------------------------------------------
// argv bytes that are not valid UTF-8 (the C code only ever sees bytes)
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn non_utf8_arguments() {
    use std::os::unix::ffi::OsStringExt;

    let cases: Vec<Vec<u8>> = vec![
        vec![0xff, 0xfe],             // no conversion -> error path
        b"5\xff".to_vec(),            // digits then an invalid byte
        b"\xff5".to_vec(),            // invalid byte first -> error path
        b"  -7\x80\x81".to_vec(),     // whitespace, sign, digits, junk
        b"\xc3".to_vec(),             // truncated UTF-8 lead byte
        b"\x80".to_vec(),             // stray continuation byte
        vec![0xf4, 0x90, 0x80, 0x80], // beyond the Unicode range
    ];

    for bytes in cases {
        let arg = OsString::from_vec(bytes);
        assert_same(&[arg]);
    }
}

// ---------------------------------------------------------------------------
// Sanity checks on the harness itself
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_exist_and_are_executable() {
    let c = c_bin();
    let r = rust_bin();
    assert!(c.is_file(), "C binary not found at {}", c.display());
    assert!(r.is_file(), "Rust binary not found at {}", r.display());

    // A known-good invocation must actually produce output on both sides.
    let out_c = run(&c, &[OsString::from("1")]);
    let out_r = run(&r, &[OsString::from("1")]);
    assert_eq!(out_c.stdout, b"0\n1\n3\n6\n10\n15\n21\n28\n36\n45\n");
    assert_eq!(out_r.stdout, out_c.stdout);
    assert!(out_c.stderr.is_empty());
    assert!(out_r.stderr.is_empty());
    assert_eq!(out_c.status.code(), Some(0));
    assert_eq!(out_r.status.code(), Some(0));
}

#[test]
fn error_paths_write_to_stdout_not_stderr_and_exit_1() {
    // The C program prints its errors with printf, i.e. on stdout, and returns 1.
    for args in [vec![], vec!["abc"]] {
        let owned: Vec<OsString> = args.iter().map(OsString::from).collect();
        let c = run(&c_bin(), &owned);
        let r = run(&rust_bin(), &owned);
        assert!(c.stderr.is_empty(), "C unexpectedly wrote to stderr");
        assert_eq!(c.stderr, r.stderr);
        assert_eq!(c.stdout, r.stdout);
        assert_eq!(c.status.code(), Some(1));
        assert_eq!(r.status.code(), Some(1));
    }
}

#[test]
fn stdin_is_never_read() {
    // The C program takes no stdin; feeding it data must not change anything.
    use std::io::Write;
    use std::process::Stdio;

    let feed = |bin: &Path| -> Output {
        let mut child = Command::new(bin)
            .arg("2")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn failed");
        child
            .stdin
            .as_mut()
            .expect("stdin piped")
            .write_all(b"99\n99\n")
            .expect("write to child stdin");
        child.wait_with_output().expect("wait failed")
    };

    let c = feed(&c_bin());
    let r = feed(&rust_bin());
    assert_eq!(c.stdout, r.stdout);
    assert_eq!(c.stderr, r.stderr);
    assert_eq!(c.status.code(), r.status.code());
}
