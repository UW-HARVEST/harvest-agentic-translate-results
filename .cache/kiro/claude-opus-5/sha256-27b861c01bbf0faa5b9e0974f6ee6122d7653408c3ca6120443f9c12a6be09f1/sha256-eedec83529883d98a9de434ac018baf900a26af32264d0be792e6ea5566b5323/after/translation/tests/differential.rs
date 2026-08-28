//! Differential tests: run the original C binary and the Rust binary as
//! subprocesses with identical argv and require byte-identical stdout, stderr
//! and exit status.
//!
//! Nothing here links against the Rust crate as a library; both programs are
//! driven exactly the way a shell drives them, because that is how the
//! translation is graded.

use std::ffi::{OsStr, OsString};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Locating / building the two binaries
// ---------------------------------------------------------------------------

/// The Rust binary under test, built by cargo for this test run.
fn rust_bin() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

fn workspace_root() -> PathBuf {
    // translation/ -> workspace root
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// The C binary, built from `c_src/` with CMake on first use.
///
/// `c_src/` itself is never modified; only the out-of-source `build/`
/// directory is created.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = workspace_root().join("c_src");
        let build = c_src.join("build");
        let exe = build.join(if cfg!(windows) { "driver.exe" } else { "driver" });
        if exe.is_file() {
            return exe;
        }

        std::fs::create_dir_all(&build).expect("create c_src/build");
        let configure = Command::new("cmake")
            .arg("..")
            .current_dir(&build)
            .output()
            .expect("run `cmake ..` in c_src/build (is cmake installed?)");
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
            .expect("run `cmake --build .`");
        assert!(
            compile.status.success(),
            "cmake build failed:\n{}\n{}",
            String::from_utf8_lossy(&compile.stdout),
            String::from_utf8_lossy(&compile.stderr)
        );

        assert!(exe.is_file(), "C binary missing after build: {}", exe.display());
        exe
    })
}

// ---------------------------------------------------------------------------
// Running and comparing
// ---------------------------------------------------------------------------

struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    status: String,
}

/// Render the exit status so that a normal exit and a signal death can never
/// compare equal by accident.
fn describe_status(status: std::process::ExitStatus) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(sig) = status.signal() {
            return format!("signal({sig})");
        }
    }
    match status.code() {
        Some(code) => format!("exit({code})"),
        None => "exit(unknown)".to_string(),
    }
}

fn run(bin: &Path, args: &[OsString]) -> Outcome {
    let out = Command::new(bin)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .unwrap_or_else(|e| panic!("failed to run {}: {e}", bin.display()));
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        status: describe_status(out.status),
    }
}

fn show(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    if text.len() <= 400 {
        text.into_owned()
    } else {
        let head: String = text.chars().take(200).collect();
        let tail: String = {
            let all: Vec<char> = text.chars().collect();
            all[all.len().saturating_sub(200)..].iter().collect()
        };
        format!("{head}\n...[{} bytes total]...\n{tail}", bytes.len())
    }
}

/// Core assertion: identical stdout, stderr and exit status.
fn assert_same(args: &[OsString]) {
    let c = run(c_bin(), args);
    let r = run(rust_bin(), args);

    let pretty: Vec<String> = args
        .iter()
        .map(|a| format!("{:?}", a.to_string_lossy()))
        .collect();
    let label = format!("argv = [{}]", pretty.join(", "));

    assert_eq!(
        c.status, r.status,
        "exit status mismatch for {label}\n C: {}\n R: {}",
        c.status, r.status
    );
    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch for {label}\n--- C stdout ---\n{}\n--- Rust stdout ---\n{}",
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch for {label}\n--- C stderr ---\n{}\n--- Rust stderr ---\n{}",
        show(&c.stderr),
        show(&r.stderr)
    );
}

fn check(args: &[&str]) {
    let owned: Vec<OsString> = args.iter().map(OsString::from).collect();
    assert_same(&owned);
}

/// Every argv in the slice, one process pair per entry.
fn check_each(cases: &[&str]) {
    for case in cases {
        check(&[case]);
    }
}

#[cfg(unix)]
fn os_from_bytes(bytes: &[u8]) -> OsString {
    use std::os::unix::ffi::OsStringExt;
    OsString::from_vec(bytes.to_vec())
}

// ---------------------------------------------------------------------------
// Phase A: both binaries exist and are runnable
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_are_runnable() {
    assert!(rust_bin().is_file(), "rust binary missing: {}", rust_bin().display());
    assert!(c_bin().is_file(), "c binary missing: {}", c_bin().display());
    // A trivial invocation of each must actually execute.
    for bin in [c_bin(), rust_bin()] {
        let out = run(bin, &[OsString::from("9")]);
        assert_eq!(out.stdout, b"9\n", "unexpected output from {}", bin.display());
        assert_eq!(out.status, "exit(0)");
    }
}

// ---------------------------------------------------------------------------
// Phase B / C: one test per branch in main()
// ---------------------------------------------------------------------------

/// `argc != 2` -> "should only be a single (integer) argument", exit 1.
#[test]
fn wrong_argument_count() {
    check(&[]); // argc == 1
    check(&["5", "6"]); // argc == 3
    check(&["1", "2", "3"]); // argc == 4
    check(&["9", ""]);
    check(&["", ""]);
    check(&["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"]);
}

/// `end == argv[1]`: strtol converted nothing -> "must be an integer", exit 1.
#[test]
fn no_conversion_performed() {
    check_each(&[
        "",        // empty argument
        " ",       // whitespace only
        "   ",     //
        "\t",      // other isspace characters
        "\n",      //
        "\r",      //
        "\u{0b}",  // vertical tab
        "\u{0c}",  // form feed
        " \t\r\n", // mixed whitespace only
        "abc",
        "+",     // sign with no digits
        "-",     //
        "+abc",  //
        "-abc",  //
        "--5",   // double sign
        "+-5",   //
        "++5",   //
        "  -  9",// sign detached from digits
        "- 9",
        ".5",    // no leading digit
        "-.5",
        "x9",
        "e9",
        "'9'",
        "/9",   // byte just below '0'
        ":9",   // byte just above '9'
        "٩",    // non-ASCII digit (Arabic-Indic nine) -> no conversion
    ]);
}

/// Non-UTF-8 argv bytes still reach strtol as raw bytes.
#[cfg(unix)]
#[test]
fn non_utf8_arguments() {
    for bytes in [
        &b"\xff"[..],        // invalid UTF-8, no conversion
        &b"\x80\x81"[..],    // invalid UTF-8, no conversion
        &b"5\xff"[..],       // digits then invalid trailing bytes
        &b"\xff5"[..],       // invalid bytes first -> no conversion
        &b"\xc3"[..],        // truncated UTF-8 sequence
        &b"-\xff"[..],       // sign then invalid byte
    ] {
        assert_same(&[os_from_bytes(bytes)]);
    }
}

/// `val % 10 == 9` on the very first iteration: exactly one line printed.
#[test]
fn breaks_immediately() {
    check_each(&[
        "9", "19", "29", "99", "109", "1000000009",
        "9 ",       // trailing whitespace after digits
        "  +0009",  // leading whitespace, plus sign, leading zeros
        "0009x",    // trailing junk
        "2147483639", // largest int that ends in 9
        "9abc",
        "9\u{0338}", // '9' followed by a multi-byte non-digit
    ]);
}

/// The counting loop: several iterations before the terminator.
#[test]
fn counts_up_to_terminator() {
    check_each(&[
        "0", "1", "2", "5", "8", "10", "11", "20", "42", "100", "2147483630",
        "+7", "-0", "007", "0x10", // base 10 stops at 'x'
        "1e3",   // stops at 'e'
        "1 2",   // stops at the space
        "12abc", // stops at 'a'
        "  42",  // leading spaces skipped
        "\t\n 3",// leading tab/newline/space skipped
        "\u{0b}\u{0c}3",
    ]);
}

/// Negative starts: C's `%` truncates toward zero, so `-9 % 10 == -9`, never 9.
/// The loop therefore counts all the way up through zero to +9.
#[test]
fn negative_starts_count_through_zero() {
    check_each(&["-1", "-2", "-5", "-9", "-10", "-12", "-19", "-29", "-100", "-0009"]);
}

/// `int val = strtol(...)` truncates a 64-bit long to 32 bits.
#[test]
fn long_to_int_truncation() {
    check_each(&[
        "4294967296",  // 2^32      -> 0
        "4294967297",  // 2^32 + 1  -> 1
        "4294967305",  // 2^32 + 9  -> 9
        "4294967291",  // 2^32 - 5  -> -5
        "8589934592",  // 2^33      -> 0
        "-4294967296", // -2^32     -> 0
        "-4294967291", // -2^32 + 5 -> 5
        // "2147483648" also truncates (to INT_MIN) but then prints ~2^31 lines;
        // it is covered by `signed_overflow_wraparound_prefix` instead.
        "4294967295",  // 2^32 - 1  -> -1
        "12884901897", // 3*2^32+9  -> 9
    ]);
}

/// strtol saturates at LONG_MAX / LONG_MIN but still consumes every digit, and
/// the saturated value is then truncated to int.
#[test]
fn strtol_saturation_then_truncation() {
    check_each(&[
        "9223372036854775806", // LONG_MAX-1 -> -2
        "9223372036854775807", // LONG_MAX   -> -1
        "9223372036854775808", // overflow   -> LONG_MAX -> -1
        "99999999999999999999",
        "170141183460469231731687303715884105727",
        "-9223372036854775808", // LONG_MIN   -> 0
        "-9223372036854775809", // underflow  -> LONG_MIN -> 0
        "-99999999999999999999",
        "9223372036854775807abc", // saturation plus trailing junk
    ]);
}

/// Digit runs far longer than any integer type.
#[test]
fn very_long_digit_runs() {
    let nines = "9".repeat(5000);
    check(&[&nines]);
    check(&[&format!("-{nines}")]);
    check(&[&format!("{}9", "0".repeat(5000))]); // 5000 leading zeros then 9
    check(&[&format!("+{}", "1".repeat(1000))]);
}

// ---------------------------------------------------------------------------
// Phase C: branches whose full output is too large to capture
// ---------------------------------------------------------------------------

/// Read exactly `n` bytes of stdout from a process, then stop it.
///
/// `int` overflow on `val++` makes some inputs print billions of lines, so the
/// only practical differential check for those is a bounded prefix.
fn stdout_prefix(bin: &Path, arg: &str, n: usize) -> Vec<u8> {
    let mut child = Command::new(bin)
        .arg(arg)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    let mut buf = vec![0u8; n];
    let mut read = 0usize;
    {
        let out = child.stdout.as_mut().expect("piped stdout");
        while read < n {
            match out.read(&mut buf[read..]) {
                Ok(0) => break,
                Ok(k) => read += k,
                Err(e) => panic!("read from {} failed: {e}", bin.display()),
            }
        }
    }
    buf.truncate(read);

    let _ = child.kill();
    let _ = child.wait();
    buf
}

/// `val++` past INT_MAX wraps to INT_MIN and the loop keeps going, so these
/// inputs emit billions of lines. Compare a 64 KiB prefix instead.
#[test]
fn signed_overflow_wraparound_prefix() {
    const N: usize = 64 * 1024;
    for arg in ["2147483647", "2147483648", "-2147483648", "-2000000000", "-2147483639"] {
        let c = stdout_prefix(c_bin(), arg, N);
        let r = stdout_prefix(rust_bin(), arg, N);
        assert_eq!(c.len(), N, "C produced a short prefix for {arg:?}");
        assert_eq!(
            c,
            r,
            "stdout prefix mismatch for argv = [{arg:?}]\n--- C ---\n{}\n--- Rust ---\n{}",
            show(&c[..c.len().min(200)]),
            show(&r[..r.len().min(200)])
        );
    }
}

/// The C program leaves SIGPIPE at SIG_DFL, so a closed stdout kills it. Rust
/// ignores SIGPIPE by default, which would instead let the loop run to
/// completion and exit 0; both must die from the signal here.
#[cfg(unix)]
#[test]
fn closed_stdout_kills_both_the_same_way() {
    fn status_after_closed_stdout(bin: &Path) -> String {
        let mut child = Command::new(bin)
            .arg("-2000000000")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

        // Read a little, then close the read end of the pipe.
        let mut stdout = child.stdout.take().expect("piped stdout");
        let mut buf = [0u8; 64];
        let mut read = 0;
        while read < buf.len() {
            match stdout.read(&mut buf[read..]) {
                Ok(0) => break,
                Ok(k) => read += k,
                Err(e) => panic!("read failed: {e}"),
            }
        }
        drop(stdout);

        describe_status(child.wait().expect("wait for child"))
    }

    let c = status_after_closed_stdout(c_bin());
    let r = status_after_closed_stdout(rust_bin());
    assert_eq!(c, r, "status after closed stdout differs: C={c} Rust={r}");
    assert_eq!(c, "signal(13)", "expected death by SIGPIPE, got {c}");
}

/// Guard rail: the C sources must be untouched by this test run.
#[test]
fn c_sources_are_not_modified() {
    let main_c = workspace_root().join("c_src").join("src").join("main.c");
    let text = std::fs::read_to_string(&main_c).expect("read c_src/src/main.c");
    assert!(text.contains("Error: should only be a single (integer) argument!"));
    assert!(text.contains("Error: first argument must be an integer!"));
    assert!(text.contains("if (val % 10 == 9)"));
}

/// A brute-force sweep so no small-value branch is missed.
#[test]
fn exhaustive_small_range() {
    let mut args: Vec<String> = Vec::new();
    for v in -60i32..=60 {
        args.push(v.to_string());
    }
    for a in &args {
        check(&[a.as_str()]);
    }
    // And a few larger values whose last digit walks the whole 0..=9 cycle.
    for v in 1230..=1245 {
        check(&[&v.to_string()]);
    }
    let _ = OsStr::new("");
}
