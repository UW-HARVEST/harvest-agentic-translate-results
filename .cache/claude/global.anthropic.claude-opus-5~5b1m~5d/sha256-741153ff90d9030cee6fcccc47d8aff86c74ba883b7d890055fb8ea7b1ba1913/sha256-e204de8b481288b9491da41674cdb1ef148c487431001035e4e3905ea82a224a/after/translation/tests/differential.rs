//! Differential tests: run the original C `driver` and the Rust `driver` as
//! subprocesses with identical argv and compare stdout, stderr and exit status
//! byte for byte.
//!
//! The Rust program is NEVER used as a library here; it is driven exactly the
//! way a shell would drive it, because that is how it is graded.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_src_dir() -> PathBuf {
    manifest_dir().parent().unwrap().join("c_src")
}

/// Directory used for the CMake build tree. Deliberately *outside* `c_src/`
/// so that running the tests never writes anything into the C source tree.
fn c_build_dir() -> PathBuf {
    manifest_dir().join("target").join("c_build")
}

/// Build the C program with CMake (once per test binary) and return the path to
/// the resulting `driver` executable.
fn c_binary() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let src = c_src_dir();
        let build = c_build_dir();
        assert!(
            src.join("CMakeLists.txt").is_file(),
            "cannot find c_src/CMakeLists.txt at {}",
            src.display()
        );
        std::fs::create_dir_all(&build).expect("create cmake build dir");

        let configure = Command::new("cmake")
            .arg("-S")
            .arg(&src)
            .arg("-B")
            .arg(&build)
            .output()
            .expect("failed to run `cmake` (is cmake installed?)");
        assert!(
            configure.status.success(),
            "cmake configure failed:\n{}\n{}",
            String::from_utf8_lossy(&configure.stdout),
            String::from_utf8_lossy(&configure.stderr)
        );

        let build_out = Command::new("cmake")
            .arg("--build")
            .arg(&build)
            .output()
            .expect("failed to run `cmake --build`");
        assert!(
            build_out.status.success(),
            "cmake build failed:\n{}\n{}",
            String::from_utf8_lossy(&build_out.stdout),
            String::from_utf8_lossy(&build_out.stderr)
        );

        let exe = build.join(if cfg!(windows) { "driver.exe" } else { "driver" });
        assert!(exe.is_file(), "C driver not found at {}", exe.display());
        exe
    })
    .as_path()
}

/// Path to the Rust executable produced by this crate.
fn rust_binary() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

fn run(bin: &Path, args: &[&str]) -> Output {
    Command::new(bin)
        .args(args)
        .env_clear()
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()))
}

fn show(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => format!("{s:?}"),
        Err(_) => format!("{bytes:?}"),
    }
}

/// Assert that the C and Rust programs agree on stdout, stderr and exit status.
#[track_caller]
fn assert_same(args: &[&str]) {
    let c = run(c_binary(), args);
    let r = run(rust_binary(), args);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch for args {args:?}\n  C: {}\n  R: {}",
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch for args {args:?}\n  C: {}\n  R: {}",
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "exit status mismatch for args {args:?} (C {:?} vs Rust {:?})",
        c.status,
        r.status
    );
}

// ---------------------------------------------------------------------------
// argc != 3  ->  "Error: should only be two (integer) arguments!" / exit 1
// ---------------------------------------------------------------------------

#[test]
fn argc_zero_extra_args() {
    assert_same(&[]);
}

#[test]
fn argc_one_extra_arg() {
    assert_same(&["1"]);
}

#[test]
fn argc_three_extra_args() {
    assert_same(&["1", "2", "3"]);
}

#[test]
fn argc_many_extra_args() {
    assert_same(&["1", "2", "3", "4", "5"]);
}

// ---------------------------------------------------------------------------
// first argument fails to parse -> error path 2
// ---------------------------------------------------------------------------

#[test]
fn first_arg_empty() {
    assert_same(&["", "5"]);
}

#[test]
fn first_arg_alpha() {
    assert_same(&["abc", "5"]);
}

#[test]
fn first_arg_only_whitespace() {
    assert_same(&["   ", "3"]);
}

#[test]
fn first_arg_only_sign() {
    for a in ["-", "+", "--5", "+-3", ".", "-.", "e", "$"] {
        assert_same(&[a, "3"]);
    }
}

#[test]
fn first_arg_bad_takes_precedence_over_second() {
    // Validation order: argv[1] is checked before argv[2].
    assert_same(&["abc", "def"]);
}

// ---------------------------------------------------------------------------
// second argument fails to parse -> error path 3
// ---------------------------------------------------------------------------

#[test]
fn second_arg_empty() {
    assert_same(&["5", ""]);
}

#[test]
fn second_arg_alpha() {
    assert_same(&["5", "abc"]);
}

#[test]
fn second_arg_only_whitespace() {
    assert_same(&["5", " \t "]);
}

#[test]
fn second_arg_only_sign() {
    for b in ["-", "+", "*", "0x", "--1"] {
        assert_same(&["5", b]);
    }
}

// ---------------------------------------------------------------------------
// zero-iteration / negative-iteration loops: no output, exit 0
// ---------------------------------------------------------------------------

#[test]
fn zero_iterations() {
    assert_same(&["0", "0"]);
    assert_same(&["1", "0"]);
    assert_same(&["-7", "0"]);
}

#[test]
fn negative_iterations() {
    assert_same(&["1", "-1"]);
    assert_same(&["1", "-1000"]);
    assert_same(&["-5", "-2147483648"]);
}

// ---------------------------------------------------------------------------
// single iteration and small happy paths
// ---------------------------------------------------------------------------

#[test]
fn single_iteration() {
    assert_same(&["1", "1"]);
    assert_same(&["0", "1"]);
    assert_same(&["-1", "1"]);
    assert_same(&["100", "1"]);
}

#[test]
fn happy_path_positive() {
    assert_same(&["1", "5"]);
    assert_same(&["5", "5"]);
    assert_same(&["12", "10"]);
}

/// `*outer < inner` branch: the pointer keeps aliasing `initial_value` until it
/// catches up with `inner`, then switches to `&inner` and doubles.
#[test]
fn else_branch_then_switch() {
    assert_same(&["-3", "5"]);
    assert_same(&["-5", "4"]);
    assert_same(&["-1", "10"]);
    assert_same(&["-100", "5"]);
    assert_same(&["0", "5"]);
}

// ---------------------------------------------------------------------------
// strtol acceptance details (leading space, sign, trailing junk, bases)
// ---------------------------------------------------------------------------

#[test]
fn strtol_leading_whitespace_and_sign() {
    assert_same(&[" 12", "3"]);
    assert_same(&["+7", "3"]);
    assert_same(&["-0", "3"]);
    assert_same(&["\t\n 42", "2"]);
    assert_same(&["  -5", "  4"]);
    assert_same(&["007", "3"]);
}

#[test]
fn strtol_trailing_junk_is_accepted() {
    // strtol stops at the first non-digit; `end != nptr`, so no error.
    assert_same(&["12abc", "3"]);
    assert_same(&["7", "3junk"]);
    assert_same(&["1e5", "3"]);
    assert_same(&["3 4", "2"]);
}

#[test]
fn strtol_base10_only() {
    // "0x10" parses as 0 in base 10 (end points at 'x').
    assert_same(&["0x10", "3"]);
    assert_same(&["010", "3"]);
    assert_same(&["0b11", "3"]);
}

// ---------------------------------------------------------------------------
// long -> int truncation and strtol range saturation
// ---------------------------------------------------------------------------

#[test]
fn long_to_int_truncation() {
    assert_same(&["4294967296", "5"]); // 2^32 -> 0
    assert_same(&["4294967297", "5"]); // 2^32+1 -> 1
    assert_same(&["2147483648", "5"]); // INT_MAX+1 -> INT_MIN
    assert_same(&["-2147483649", "5"]); // INT_MIN-1 -> INT_MAX
    assert_same(&["1", "4294967297"]); // iterations truncates to 1
    assert_same(&["1", "4294967296"]); // iterations truncates to 0
    assert_same(&["3", "2147483648"]); // iterations -> INT_MIN, loop skipped
}

#[test]
fn strtol_saturates_out_of_range() {
    assert_same(&["9223372036854775807", "4"]); // LONG_MAX
    assert_same(&["9223372036854775808", "4"]); // > LONG_MAX -> LONG_MAX
    assert_same(&["-9223372036854775808", "4"]); // LONG_MIN
    assert_same(&["-9223372036854775809", "4"]); // < LONG_MIN -> LONG_MIN
    assert_same(&["99999999999999999999", "3"]);
    assert_same(&["-99999999999999999999", "3"]);
    assert_same(&[
        "123456789012345678901234567890123456789012345678901234567890",
        "3",
    ]);
    assert_same(&["3", "99999999999999999999"]);
    assert_same(&["3", "-99999999999999999999"]);
}

// ---------------------------------------------------------------------------
// signed-integer wraparound inside the loop
// ---------------------------------------------------------------------------

#[test]
fn wraparound_on_doubling() {
    // inner doubles every iteration once the pointer aliases it, so 40
    // iterations from 1 overflow int and wrap to 0.
    assert_same(&["1", "40"]);
    assert_same(&["1", "64"]);
    assert_same(&["1073741824", "6"]);
    assert_same(&["2000000000", "5"]);
    assert_same(&["2147483647", "5"]);
}

#[test]
fn extremes_of_int_range() {
    assert_same(&["-2147483648", "5"]);
    assert_same(&["-2000000000", "5"]);
    assert_same(&["2147483646", "4"]);
}

// ---------------------------------------------------------------------------
// raw (non-UTF-8) argv bytes and the full C isspace() set
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[track_caller]
fn assert_same_bytes(args: &[&[u8]]) {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let os: Vec<&OsStr> = args.iter().map(|a| OsStr::from_bytes(a)).collect();
    let spawn = |bin: &Path| -> Output {
        Command::new(bin)
            .args(&os)
            .env_clear()
            .output()
            .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()))
    };
    let c = spawn(c_binary());
    let r = spawn(rust_binary());

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch for raw args {args:?}\n  C: {}\n  R: {}",
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(c.stderr, r.stderr, "stderr mismatch for raw args {args:?}");
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "exit status mismatch for raw args {args:?}"
    );
}

/// `strtol` skips every character `isspace()` accepts, including `\v` and `\f`.
#[cfg(unix)]
#[test]
fn all_c_whitespace_is_skipped() {
    for ws in [b" ", b"\t", b"\n", b"\x0b", b"\x0c", b"\r"] {
        let mut a = ws.to_vec();
        a.extend_from_slice(b"9");
        assert_same_bytes(&[a.as_slice(), b"3"]);
        assert_same_bytes(&[ws.as_slice(), b"3"]); // whitespace only -> error
        let mut b = ws.to_vec();
        b.extend_from_slice(b"3");
        assert_same_bytes(&[b"9", b.as_slice()]);
    }
    assert_same_bytes(&[b"\t\x0b\x0c\r\n -6", b" \t2"]);
}

/// Arguments need not be valid UTF-8; the C program only looks at bytes.
#[cfg(unix)]
#[test]
fn non_utf8_arguments() {
    assert_same_bytes(&[b"\xff9", b"3"]);
    assert_same_bytes(&[b"9\xff", b"3"]);
    assert_same_bytes(&[b"\xc3", b"\x80"]);
    assert_same_bytes(&[b"\xe2\x82\xac5", b"2"]);
}

// ---------------------------------------------------------------------------
// broad sweeps
// ---------------------------------------------------------------------------

#[test]
fn sweep_small_grid() {
    for init in -20i32..=20 {
        for iters in 0i32..=12 {
            let a = init.to_string();
            let b = iters.to_string();
            assert_same(&[a.as_str(), b.as_str()]);
        }
    }
}

#[test]
fn sweep_interesting_values() {
    let inits = [
        i32::MIN,
        i32::MIN + 1,
        -1_000_000_007,
        -65536,
        -3,
        -1,
        0,
        1,
        2,
        3,
        1023,
        65535,
        1_000_000_007,
        i32::MAX - 1,
        i32::MAX,
    ];
    let iters = [0, 1, 2, 3, 7, 31, 32, 33, 35, 40];
    for init in inits {
        for it in iters {
            let a = init.to_string();
            let b = it.to_string();
            assert_same(&[a.as_str(), b.as_str()]);
        }
    }
}

#[test]
fn sweep_pseudorandom() {
    // Deterministic xorshift so failures are reproducible.
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for _ in 0..400 {
        let init = next() as i32;
        let iters = (next() % 45) as i32;
        let a = init.to_string();
        let b = iters.to_string();
        assert_same(&[a.as_str(), b.as_str()]);
    }
}

#[test]
fn sweep_random_textual_arguments() {
    let mut state: u64 = 0xDEAD_BEEF_CAFE_1234;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let alphabet: &[u8] = b"0123456789+- \tabxX.e";
    for _ in 0..250 {
        let len = (next() % 8) as usize;
        let mk = |n: &mut dyn FnMut() -> u64, len: usize| -> String {
            (0..len)
                .map(|_| alphabet[(n() as usize) % alphabet.len()] as char)
                .collect()
        };
        let a = mk(&mut next, len);
        let len2 = (next() % 4) as usize;
        let b = mk(&mut next, len2);
        assert_same(&[a.as_str(), b.as_str()]);
    }
}
