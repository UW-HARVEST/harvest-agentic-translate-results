// Differential tests: run the C `driver` and the Rust `driver` as subprocesses
// with identical argv and require byte-identical stdout, byte-identical stderr
// and an identical exit status.
//
// Nothing here links against the Rust crate as a library -- both programs are
// driven exactly the way a shell drives them, because that is how the
// translation is graded.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

/// Path to the Rust executable under test. Cargo builds this before running
/// integration tests, so it always exists.
const RUST_BIN: &str = env!("CARGO_BIN_EXE_driver");

/// `<workspace>/c_src`
fn c_src_dir() -> PathBuf {
    // CARGO_MANIFEST_DIR is <workspace>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .join("c_src")
}

/// Configure + build the C program with CMake if it is not already built, and
/// return the path to the resulting `driver` executable.
///
/// Integration tests run in parallel threads within one process, so the build
/// is funnelled through a `OnceLock`: without it, 29 tests would invoke CMake
/// concurrently in the same directory and trip over each other's temporary
/// files.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(build_c_bin).as_path()
}

fn build_c_bin() -> PathBuf {
    let c_src = c_src_dir();
    let build = c_src.join("build");
    let exe = if cfg!(windows) {
        build.join("driver.exe")
    } else {
        build.join("driver")
    };
    if exe.is_file() {
        return exe;
    }

    std::fs::create_dir_all(&build).expect("could not create c_src/build");

    let cfg = Command::new("cmake")
        .arg("..")
        .current_dir(&build)
        .output()
        .expect("failed to run `cmake` -- is CMake installed?");
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

    assert!(
        exe.is_file(),
        "C build reported success but {} does not exist",
        exe.display()
    );
    exe
}

/// Run one program with raw (possibly non-UTF-8) argv bytes.
fn run(program: &Path, args: &[&[u8]]) -> Output {
    let mut cmd = Command::new(program);
    for a in args {
        #[cfg(unix)]
        {
            use std::ffi::OsStr;
            use std::os::unix::ffi::OsStrExt;
            cmd.arg(OsStr::from_bytes(a));
        }
        #[cfg(not(unix))]
        {
            cmd.arg(String::from_utf8_lossy(a).into_owned());
        }
    }
    cmd.stdin(std::process::Stdio::null());
    cmd.output()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", program.display()))
}

fn show(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes.iter().take(400) {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\t' => s.push_str("\\t"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    if bytes.len() > 400 {
        s.push_str("...<truncated>");
    }
    s
}

fn show_args(args: &[&[u8]]) -> String {
    args.iter()
        .map(|a| format!("{:?}", show(a)))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The core assertion: stdout, stderr and exit status must all match.
#[track_caller]
fn assert_same(args: &[&[u8]]) {
    let c = c_bin();
    let cout = run(c, args);
    let rout = run(Path::new(RUST_BIN), args);

    let label = show_args(args);

    assert_eq!(
        cout.status.code(),
        rout.status.code(),
        "exit status differs for argv [{label}]: C={:?} Rust={:?}",
        cout.status,
        rout.status
    );

    assert!(
        cout.stdout == rout.stdout,
        "stdout differs for argv [{label}]\n  C   ({} bytes): {}\n  Rust({} bytes): {}",
        cout.stdout.len(),
        show(&cout.stdout),
        rout.stdout.len(),
        show(&rout.stdout)
    );

    assert!(
        cout.stderr == rout.stderr,
        "stderr differs for argv [{label}]\n  C   ({} bytes): {}\n  Rust({} bytes): {}",
        cout.stderr.len(),
        show(&cout.stderr),
        rout.stderr.len(),
        show(&rout.stderr)
    );
}

/// Convenience for the (overwhelmingly common) all-UTF-8 case.
#[track_caller]
fn same(args: &[&str]) {
    let owned: Vec<&[u8]> = args.iter().map(|s| s.as_bytes()).collect();
    assert_same(&owned);
}

// ---------------------------------------------------------------------------
// Phase A sanity: both binaries exist and are runnable.
// ---------------------------------------------------------------------------

#[test]
fn both_programs_are_runnable() {
    let c = c_bin();
    assert!(c.is_file(), "C binary missing at {}", c.display());
    assert!(
        Path::new(RUST_BIN).is_file(),
        "Rust binary missing at {RUST_BIN}"
    );
    // Exercising them with no arguments must not fail to spawn.
    let _ = run(c, &[]);
    let _ = run(Path::new(RUST_BIN), &[]);
}

// ---------------------------------------------------------------------------
// Branch: `if (argc != 3)` -> "should only be two (integer) arguments!", exit 1
// ---------------------------------------------------------------------------

#[test]
fn argc_zero_extra_args() {
    same(&[]);
}

#[test]
fn argc_one_extra_arg() {
    same(&["5"]);
    same(&["abc"]);
    same(&[""]);
}

#[test]
fn argc_three_extra_args() {
    same(&["5", "3", "7"]);
    same(&["a", "b", "c"]);
}

#[test]
fn argc_many_extra_args() {
    same(&["a", "b", "c", "d"]);
    same(&["1", "2", "3", "4", "5"]);
    same(&["1", "2", "3", "4", "5", "6", "7", "8"]);
}

// ---------------------------------------------------------------------------
// Branch: `if (end == argv[1])` -> "first argument must be an integer!", exit 1
// strtol performs no conversion, so *endptr == nptr.
// ---------------------------------------------------------------------------

#[test]
fn first_arg_not_an_integer() {
    same(&["abc", "3"]);
    same(&["", "3"]);
    same(&[" ", "3"]);
    same(&["+", "3"]);
    same(&["-", "3"]);
    same(&["--5", "3"]);
    same(&["+-5", "3"]);
    same(&["-+5", "3"]);
    same(&[".5", "3"]);
    same(&["x10", "3"]);
    same(&["\t\n\r ", "3"]);
    // whitespace then a non-digit: still no conversion
    same(&["   zzz", "3"]);
}

#[test]
fn first_arg_check_happens_before_second() {
    // Both arguments are bad; the C code reports the FIRST one and returns
    // immediately, so the second message must never appear.
    same(&["abc", "def"]);
    same(&["", ""]);
}

// ---------------------------------------------------------------------------
// Branch: `if (end == argv[2])` -> "second argument must be an integer!", exit 1
// ---------------------------------------------------------------------------

#[test]
fn second_arg_not_an_integer() {
    same(&["5", "abc"]);
    same(&["5", ""]);
    same(&["5", " "]);
    same(&["5", "+"]);
    same(&["5", "-"]);
    same(&["5", "--3"]);
    same(&["5", "@"]);
    // A valid first argument that would otherwise loop forever must still be
    // rejected on the second argument before any output is produced.
    same(&["1", "junk"]);
}

// ---------------------------------------------------------------------------
// strtol quirks the C code deliberately does NOT guard against.
// ---------------------------------------------------------------------------

#[test]
fn trailing_garbage_is_accepted() {
    // end != argv[n], so the C accepts these and uses the prefix value.
    same(&["5abc", "3"]);
    same(&["5", "3x"]);
    same(&["5 5", "2"]);
    same(&["12.9", "2"]);
    same(&["7)", "2"]);
}

#[test]
fn base_ten_means_hex_prefix_is_a_zero() {
    // strtol(.., 10) parses "0" then stops at 'x'.
    same(&["0x10", "3"]);
    same(&["5", "0x10"]);
    same(&["0X7F", "4"]);
}

#[test]
fn leading_whitespace_is_skipped() {
    same(&["  7  ", "2"]);
    same(&["\t\n 12", "2"]);
    same(&["\u{b}\u{c}9", "2"]);
    same(&["\r5", "2"]);
    same(&[" +5", "2"]);
    same(&["5", "  3"]);
}

#[test]
fn sign_and_leading_zero_forms() {
    same(&["+5", "2"]);
    same(&["007", "2"]);
    same(&["-007", "4"]);
    same(&["+0", "3"]);
    same(&["-0", "3"]);
    same(&["5", "+3"]);
    same(&["5", "-0"]);
    same(&["0000000000000000005", "3"]);
}

#[test]
fn out_of_range_saturates_then_truncates_to_int() {
    // strtol clamps to LONG_MAX / LONG_MIN; the implicit long->int conversion
    // then truncates. e.g. LONG_MAX -> -1, LONG_MIN -> 0.
    same(&["99999999999999999999", "3"]);
    same(&["-99999999999999999999", "3"]);
    same(&["9223372036854775807", "3"]);
    same(&["9223372036854775808", "3"]);
    same(&["-9223372036854775808", "3"]);
    same(&["-9223372036854775809", "3"]);
    same(&["5", "99999999999999999999"]);
    same(&["5", "-99999999999999999999"]);
    same(&[
        "1111111111111111111111111111111111111111111111111",
        "4",
    ]);
}

#[test]
fn long_to_int_truncation_of_in_range_longs() {
    // Values that fit in a long but not an int: only the low 32 bits survive.
    same(&["4294967296", "3"]); // -> 0
    same(&["4294967297", "3"]); // -> 1
    same(&["4294967295", "3"]); // -> -1
    same(&["2147483648", "3"]); // -> INT_MIN
    same(&["-2147483649", "3"]); // -> INT_MAX
    same(&["5", "4294967298"]); // iterations -> 2
    same(&["5", "4294967296"]); // iterations -> 0
    same(&["5", "8589934594"]); // iterations -> 2
}

// ---------------------------------------------------------------------------
// The loop: `for (int i = 0; i < iterations; i++)`
// ---------------------------------------------------------------------------

#[test]
fn zero_iterations_produces_no_output() {
    same(&["5", "0"]);
    same(&["0", "0"]);
    same(&["-5", "0"]);
    same(&["5", "-0"]);
    same(&["2147483647", "0"]);
}

#[test]
fn negative_iterations_produces_no_output() {
    same(&["5", "-1"]);
    same(&["5", "-100"]);
    same(&["-5", "-7"]);
    same(&["5", "-2147483648"]);
}

#[test]
fn single_iteration() {
    same(&["5", "1"]);
    same(&["0", "1"]);
    same(&["1", "1"]);
    same(&["-1", "1"]);
    same(&["-5", "1"]);
    same(&["2147483647", "1"]);
    same(&["-2147483648", "1"]);
}

// ---------------------------------------------------------------------------
// static_alias: `if (*outer >= inner)` -- both branches, and the transition.
// ---------------------------------------------------------------------------

#[test]
fn then_branch_immediately_when_initial_ge_one() {
    // *outer >= inner (inner starts at 1): inner += *outer, return &inner.
    // Thereafter the pointer aliases `inner`, so the comparison is always true
    // and the value doubles every iteration.
    same(&["1", "5"]);
    same(&["2", "5"]);
    same(&["5", "3"]);
    same(&["5", "10"]);
    same(&["100", "8"]);
}

#[test]
fn else_branch_while_initial_below_inner() {
    // *outer < inner: *outer += inner, return outer (still &initial_value).
    // inner stays 1, so initial_value climbs by 1 each iteration until it
    // reaches 1 and the then-branch takes over.
    same(&["0", "1"]);
    same(&["0", "2"]);
    same(&["0", "3"]);
    same(&["-1", "1"]);
    same(&["-5", "3"]);
    same(&["-5", "10"]);
    same(&["-10", "20"]);
}

#[test]
fn else_to_then_transition_boundary() {
    // Exercise the exact iteration where the branch flips for several starts.
    for init in ["-3", "-2", "-1", "0"] {
        for iters in ["1", "2", "3", "4", "5", "6", "7"] {
            same(&[init, iters]);
        }
    }
}

// ---------------------------------------------------------------------------
// Signed overflow: the doubling of `inner` wraps (as gcc does at -O0).
// ---------------------------------------------------------------------------

#[test]
fn overflow_wraps_on_doubling() {
    same(&["1", "40"]);
    same(&["1", "70"]);
    same(&["0", "40"]);
    same(&["-1", "40"]);
    same(&["3", "40"]);
    same(&["1073741824", "6"]);
    same(&["715827883", "8"]);
    same(&["1431655765", "8"]);
}

#[test]
fn overflow_on_the_very_first_addition() {
    // inner += *outer overflows immediately for large initial values.
    same(&["2147483647", "5"]);
    same(&["2147483646", "5"]);
    same(&["1", "1"]);
    same(&["2147483647", "3"]);
}

#[test]
fn most_negative_initial_value() {
    // INT_MIN < 1, so the else branch runs and climbs by one each time.
    same(&["-2147483648", "5"]);
    same(&["-2147483648", "10"]);
}

#[test]
fn value_settles_at_zero_after_wrapping() {
    // Once inner becomes 0 the then-branch keeps it at 0 forever, so the
    // output is a long run of "0\n" lines -- check the exact byte count.
    same(&["2147483647", "20"]);
    same(&["1073741824", "20"]);
}

// ---------------------------------------------------------------------------
// Larger outputs: make sure formatting/newlines agree at volume too.
// ---------------------------------------------------------------------------

#[test]
fn many_iterations_full_byte_comparison() {
    same(&["1", "200"]);
    same(&["-100", "500"]);
    same(&["7", "1000"]);
}

#[test]
fn large_but_finite_iteration_count() {
    // A multi-megabyte stdout, compared byte for byte along with the exit
    // status, to catch any formatting or newline drift only visible at volume.
    same(&["7", "3000000"]);
    // Same iteration count arrived at through long->int truncation.
    same(&["7", "4297967296"]); // 4297967296 - 2^32 == 3000000
}

/// Read at most `limit` bytes of the program's stdout, then stop the child.
/// Used for inputs whose full output is billions of lines long.
fn stdout_prefix(program: &Path, args: &[&str], limit: usize) -> Vec<u8> {
    use std::io::Read;
    let mut child = Command::new(program)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", program.display()));

    let mut buf = vec![0u8; limit];
    let mut filled = 0usize;
    {
        let out = child.stdout.as_mut().expect("piped stdout");
        while filled < limit {
            match out.read(&mut buf[filled..]) {
                Ok(0) => break,
                Ok(n) => filled += n,
                Err(e) => panic!("read failed: {e}"),
            }
        }
    }
    buf.truncate(filled);
    let _ = child.kill();
    let _ = child.wait();
    buf
}

#[test]
fn iterations_truncating_to_int_max() {
    // strtol("-2147483649") == -2147483649 fits in a long; the implicit
    // long->int conversion turns it into INT_MAX, so the C loops ~2^31 times.
    // The full output cannot be materialised, so compare a bounded prefix.
    const LIMIT: usize = 1 << 20;
    for args in [
        ["5", "-2147483649"],
        ["1", "-2147483649"],
        ["-5", "2147483647"],
        ["0", "-2147483649"],
    ] {
        let c = stdout_prefix(c_bin(), &args, LIMIT);
        let r = stdout_prefix(Path::new(RUST_BIN), &args, LIMIT);
        assert_eq!(c.len(), LIMIT, "C produced a short prefix for {args:?}");
        assert!(
            c == r,
            "stdout prefix differs for {args:?}\n  C   : {}\n  Rust: {}",
            show(&c),
            show(&r)
        );
    }
}

// ---------------------------------------------------------------------------
// Non-UTF-8 argv bytes must be handled like C's `char **argv`.
// ---------------------------------------------------------------------------

#[test]
fn non_utf8_arguments() {
    assert_same(&[b"\xff\xfe", b"3"]);
    assert_same(&[b"3", b"\xff\xfe"]);
    assert_same(&[b"\xff5", b"3"]);
    assert_same(&[b"5\xff", b"3"]); // trailing garbage -> accepted as 5
    assert_same(&[b"5", b"3\xc3"]);
    assert_same(&[b"\x80\x80\x80", b"\x80"]);
    assert_same(&[b"\xe2\x82", b"2"]);
}

// ---------------------------------------------------------------------------
// stderr must be empty on every path (the C uses printf, not fprintf(stderr)).
// ---------------------------------------------------------------------------

#[test]
fn errors_go_to_stdout_not_stderr() {
    for args in [
        vec![],
        vec!["1"],
        vec!["1", "2", "3"],
        vec!["abc", "3"],
        vec!["5", "abc"],
        vec!["5", "3"],
    ] {
        let c = c_bin();
        let owned: Vec<&[u8]> = args.iter().map(|s| s.as_bytes()).collect();
        let cout = run(c, &owned);
        assert!(
            cout.stderr.is_empty(),
            "C wrote to stderr for {args:?}: {}",
            show(&cout.stderr)
        );
        same(&args);
    }
}
