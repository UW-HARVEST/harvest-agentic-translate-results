//! Differential tests: run the C `driver` and the Rust `driver` as
//! subprocesses with identical argument vectors and require byte-identical
//! stdout, byte-identical stderr and an identical exit status.
//!
//! Nothing here links the Rust code as a library. Both programs are driven the
//! way a shell drives them, because that is how they are compared.
//!
//! One harness detail matters for correctness of the comparison itself: the
//! `argc != 3` path prints `argv[0]`, so the two binaries would "differ" purely
//! because they live at different paths. Both children therefore get their
//! `argv[0]` pinned to the same string via `CommandExt::arg0`.

use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// The `argv[0]` both children observe. Pinned so the usage message matches.
const ARG0: &str = "driver";

/// Path to the Rust binary under test, provided by Cargo.
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Workspace root: the directory holding both `c_src/` and `translation/`.
fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is `<root>/translation`.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

/// Path to the C binary, building it with CMake on first use if absent.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = repo_root().join("c_src");
        let build = c_src.join("build");
        let bin = build.join("driver");
        if bin.exists() {
            return bin;
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
        let built = Command::new("cmake")
            .args(["--build", "."])
            .current_dir(&build)
            .output()
            .expect("run `cmake --build .`");
        assert!(
            built.status.success(),
            "cmake build failed:\n{}\n{}",
            String::from_utf8_lossy(&built.stdout),
            String::from_utf8_lossy(&built.stderr)
        );
        assert!(bin.exists(), "C driver missing after build: {}", bin.display());
        bin
    })
}

/// What one program produced for one argument vector.
struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Some(code)` for a normal exit, `None` if killed by a signal.
    code: Option<i32>,
}

fn run(bin: &Path, args: &[OsString]) -> Run {
    run_with_env(bin, args, &[])
}

/// Runs `bin` with `args`, optionally overriding environment variables. The
/// ambient environment is otherwise inherited, which is how a shell - and the
/// grader - invokes these programs. Both children get the same environment, so
/// any divergence is the program's, not the harness's.
fn run_with_env(bin: &Path, args: &[OsString], env: &[(&str, &str)]) -> Run {
    let mut cmd = Command::new(bin);
    cmd.arg0(ARG0).args(args);
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to run {}: {e}", bin.display()));
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
    }
}

/// Renders bytes readably in failure messages without hiding anything: printable
/// ASCII as-is, everything else as `\xNN`.
fn show(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\t' => s.push_str("\\t"),
            b'\\' => s.push_str("\\\\"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    s
}

fn show_args(args: &[OsString]) -> String {
    args.iter()
        .map(|a| format!("'{}'", show(a.as_bytes())))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Asserts the C and Rust programs agree on stdout, stderr and exit status.
fn assert_same(args: &[OsString]) {
    let c = run(c_bin(), args);
    let r = run(&rust_bin(), args);

    let ctx = format!("argv = [{}]", show_args(args));

    assert_eq!(
        c.code, r.code,
        "exit status differs for {ctx}\n  C: {:?}\n  R: {:?}\n  C stderr: {}\n  R stderr: {}",
        c.code,
        r.code,
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        show(&c.stdout),
        show(&r.stdout),
        "stdout differs for {ctx}"
    );
    assert_eq!(
        show(&c.stderr),
        show(&r.stderr),
        "stderr differs for {ctx}"
    );
    // The `show` comparisons above are lossless, but assert on the raw bytes as
    // well so the test cannot pass on a rendering coincidence.
    assert_eq!(c.stdout, r.stdout, "stdout bytes differ for {ctx}");
    assert_eq!(c.stderr, r.stderr, "stderr bytes differ for {ctx}");
}

/// Convenience: build an argv from `&str`s.
fn argv(items: &[&str]) -> Vec<OsString> {
    items.iter().map(OsString::from).collect()
}

/// Convenience: build an argv from raw byte strings (may be invalid UTF-8).
fn argv_bytes(items: &[&[u8]]) -> Vec<OsString> {
    items
        .iter()
        .map(|b| OsStr::from_bytes(b).to_os_string())
        .collect()
}

/// Same as [`assert_same`], with environment overrides applied to both children.
fn assert_same_env(args: &[OsString], env: &[(&str, &str)]) {
    let c = run_with_env(c_bin(), args, env);
    let r = run_with_env(&rust_bin(), args, env);
    let ctx = format!("argv = [{}], env = {env:?}", show_args(args));
    assert_eq!(c.code, r.code, "exit status differs for {ctx}");
    assert_eq!(show(&c.stdout), show(&r.stdout), "stdout differs for {ctx}");
    assert_eq!(show(&c.stderr), show(&r.stderr), "stderr differs for {ctx}");
    assert_eq!(c.stdout, r.stdout, "stdout bytes differ for {ctx}");
    assert_eq!(c.stderr, r.stderr, "stderr bytes differ for {ctx}");
}

fn check_all(cases: &[&[&str]]) {
    for case in cases {
        assert_same(&argv(case));
    }
}

// ---------------------------------------------------------------------------
// Phase A: both binaries exist and are runnable
// ---------------------------------------------------------------------------

/// The two commands under comparison, for the record:
///   C:    c_src/build/driver <base> <exponent>
///   Rust: translation/target/{debug,release}/driver <base> <exponent>
/// Both are ordinary executables driven through argv; nothing is loaded as a
/// library.
#[test]
fn both_binaries_are_runnable() {
    let c = c_bin();
    let r = rust_bin();
    assert!(c.is_file(), "C binary not found at {}", c.display());
    assert!(r.is_file(), "Rust binary not found at {}", r.display());

    // A trivial invocation must succeed for both, otherwise every other
    // comparison in this file would be measuring nothing.
    for bin in [c, r.as_path()] {
        let out = run(bin, &argv(&["2", "3"]));
        assert_eq!(
            out.code,
            Some(0),
            "{} did not exit 0 on `2 3`; stderr: {}",
            bin.display(),
            show(&out.stderr)
        );
        assert_eq!(out.stdout, b"Result: 8.00\n", "{}", bin.display());
        assert!(out.stderr.is_empty(), "{}", bin.display());
    }
}

/// Neither program calls `setlocale`, so both stay in the "C" locale and the
/// decimal point stays `.` even under a locale that would use `,`. Asserted
/// rather than assumed, since `strtod` and `printf` are locale-sensitive
/// functions and a divergence here would be invisible in the default locale.
#[test]
fn locale_does_not_change_behavior() {
    for locale in ["C", "de_DE.UTF-8", "fr_FR.UTF-8", "en_US.UTF-8", "invalid.locale"] {
        let env = [("LC_ALL", locale), ("LC_NUMERIC", locale), ("LANG", locale)];
        for case in [
            ["1.5", "2"],
            ["1,5", "2"],
            ["2", "1.5"],
            ["2", "1,5"],
            ["0.125", "1"],
            ["1e308", "2"],
            ["abc", "2"],
        ] {
            assert_same_env(&argv(&case), &env);
        }
    }
}

// ---------------------------------------------------------------------------
// argc != 3  ->  "Usage: %s base exponent", exit 1
// ---------------------------------------------------------------------------

#[test]
fn wrong_argument_count() {
    check_all(&[
        // Zero arguments: the "empty input" case for a program whose whole
        // input is argv.
        &[],
        // One argument.
        &["2"],
        // Four and more: the check is `!= 3`, not `< 3`.
        &["2", "3", "4"],
        &["2", "3", "4", "5"],
        // An empty argument still counts toward argc.
        &[""],
        &["", "", ""],
    ]);
}

// ---------------------------------------------------------------------------
// The happy path, and the shapes strtod accepts
// ---------------------------------------------------------------------------

#[test]
fn happy_path() {
    check_all(&[
        &["2", "3"],
        &["2", "10"],
        &["3", "0"],
        &["0", "0"],
        &["1", "1"],
        &["2", "-1"],
        &["2", "0.5"],
        &["9", "0.5"],
        &["10", "3"],
        &["-2", "3"],
        &["-2", "2"],
        &["-2", "-3"],
    ]);
}

#[test]
fn strtod_accepted_forms() {
    check_all(&[
        // Leading whitespace is skipped by strtod.
        &[" 5", "2"],
        &["\t5", "2"],
        &["\n5", "2"],
        &["  \t\n 5", "2"],
        // Explicit signs.
        &["+3", "+2"],
        &["-3", "+2"],
        &["+3", "-2"],
        // Missing integer or fraction part.
        &["5.", "2."],
        &[".5", ".5"],
        &["-.5", "3"],
        // Exponent forms.
        &["1e2", "2"],
        &["1E2", "2"],
        &["1e+2", "2"],
        &["1e-2", "2"],
        // Hex floats - accepted by strtod, rejected by Rust's own parser.
        &["0x10", "2"],
        &["0X1P4", "2"],
        &["0x1.8p1", "2"],
        &["  -0x1.8p1", "2"],
        &["0x0", "2"],
        // inf / nan spellings, in several cases.
        &["inf", "2"],
        &["INF", "2"],
        &["Inf", "2"],
        &["infinity", "2"],
        &["INFINITY", "2"],
        &["-inf", "3"],
        &["-inf", "2"],
        &["nan", "2"],
        &["NAN", "2"],
        &["nan(123)", "2"],
        &["nan(0x1)", "2"],
        &["-nan", "2"],
        &["2", "inf"],
        &["2", "-inf"],
        &["2", "nan"],
    ]);
}

/// The C code never checks whether `strtod` converted anything: on total
/// failure `endptr == nptr`, so `*endptr` is the terminator and an empty string
/// is silently accepted as `0.0`.
#[test]
fn empty_argument_is_accepted_as_zero() {
    check_all(&[
        &["", "2"],
        &["2", ""],
        &["", ""],
        // Whitespace only: strtod consumes nothing, endptr points at the space,
        // so this one DOES reach the "invalid" branch.
        &[" ", "2"],
        &["2", " "],
        &["\t", "\t"],
    ]);
}

// ---------------------------------------------------------------------------
// base: errno == ERANGE  ->  "Range error while converting base '%s'"
// ---------------------------------------------------------------------------

#[test]
fn base_range_error() {
    check_all(&[
        // Overflow.
        &["1e999", "2"],
        &["-1e999", "2"],
        &["1e310", "2"],
        &["0x1p9999", "2"],
        // Just past DBL_MAX.
        &["1.7976931348623159e308", "1"],
        // Underflow: glibc sets ERANGE for subnormal results too.
        &["1e-999", "2"],
        &["1e-320", "2"],
        &["1e-308", "1"],
        &["5e-324", "1"],
        &["2.2250738585072011e-308", "1"],
        &["-1e-999", "2"],
    ]);
}

/// ERANGE is tested *before* the trailing-character check, so an overflowing
/// value with garbage after it reports the range error, not the invalid-input
/// error.
#[test]
fn base_range_error_wins_over_trailing_garbage() {
    check_all(&[
        &["1e999xyz", "2"],
        &["1e-999xyz", "2"],
        &["1e999 ", "2"],
    ]);
}

// ---------------------------------------------------------------------------
// base: *endptr != '\0'  ->  "Invalid numeric input for base: '%s'"
// ---------------------------------------------------------------------------

#[test]
fn base_invalid_input() {
    check_all(&[
        &["abc", "2"],
        &["2abc", "2"],
        &["2.5.5", "2"],
        &["--2", "2"],
        &["+", "2"],
        &["-", "2"],
        &[".", "2"],
        &["e5", "2"],
        &["5e", "2"],
        &["5e+", "2"],
        &["0x", "2"],
        &["0xg", "2"],
        &["1,5", "2"],
        &["2 ", "2"],
        &[" 2 ", "2"],
        &["1 2", "2"],
        &["2\n", "2"],
        &["2\t", "2"],
        &["inf inity", "2"],
        &["infin", "2"],
        &["nan(", "2"],
        &["nan()x", "2"],
        &["#", "2"],
        &["0b101", "2"],
        &["1p5", "2"],
    ]);
}

// ---------------------------------------------------------------------------
// exponent: the same two checks, reached only after base succeeds
// ---------------------------------------------------------------------------

#[test]
fn exponent_range_error() {
    check_all(&[
        &["2", "1e999"],
        &["2", "-1e999"],
        &["2", "1e-999"],
        &["2", "1e-320"],
        &["2", "0x1p9999"],
        &["2", "1.7976931348623159e308"],
        // ERANGE before trailing-garbage here too.
        &["2", "1e999xyz"],
    ]);
}

#[test]
fn exponent_invalid_input() {
    check_all(&[
        &["2", "abc"],
        &["2", "3abc"],
        &["2", "3.5.5"],
        &["2", "--3"],
        &["2", "+"],
        &["2", "e5"],
        &["2", "5e"],
        &["2", "0x"],
        &["2", "3,5"],
        &["2", "3 "],
        &["2", "3\n"],
        &["2", "#"],
    ]);
}

/// Base is validated first, so a bad base masks a bad exponent entirely - and
/// the message names the base.
#[test]
fn base_is_validated_before_exponent() {
    check_all(&[
        &["abc", "def"],
        &["1e999", "1e999"],
        &["abc", "1e999"],
        &["1e999", "abc"],
        &["2abc", "3abc"],
    ]);
}

// ---------------------------------------------------------------------------
// pow: errno == EDOM  ->  "Domain error: pow(%.2f, %.2f) ..."
// ---------------------------------------------------------------------------

#[test]
fn pow_domain_error() {
    check_all(&[
        // Negative finite base to a non-integer exponent.
        &["-1", "0.5"],
        &["-2", "0.5"],
        &["-2", "1.5"],
        &["-2", "-0.5"],
        &["-8", "0.3333333333333333"],
        &["-1", "1e-1"],
        // The message re-prints base and exponent with %.2f, so values that
        // round exercise the formatting as well.
        &["-0.125", "0.5"],
        &["-1.005", "0.5"],
        &["-123.456", "0.5"],
    ]);
}

// ---------------------------------------------------------------------------
// pow: errno == ERANGE  ->  "Range error: pow(%.2f, %.2f) ..."
// ---------------------------------------------------------------------------

#[test]
fn pow_range_error() {
    check_all(&[
        // Overflow.
        &["10", "400"],
        &["2", "1024"],
        &["1e308", "2"],
        &["-10", "401"],
        &["0.5", "1e308"],
        // Underflow.
        &["10", "-400"],
        &["2", "-1075"],
        &["0.5", "1075"],
        // Pole error: pow(0, negative) raises divide-by-zero; glibc reports it
        // through ERANGE, so this lands in the range branch, not the domain one.
        &["0", "-1"],
        &["0", "-2"],
        &["-0", "-1"],
        &["-0", "-3"],
        &["0", "-0.5"],
        &["0", "-1e308"],
    ]);
}

// ---------------------------------------------------------------------------
// printf("%.2f") formatting, on the success path
// ---------------------------------------------------------------------------

#[test]
fn formatting_rounding_halfway() {
    // Exact binary halfway values: glibc rounds them to even. Any Rust
    // formatter that rounds half away from zero would print 0.13 here.
    check_all(&[
        &["0.125", "1"],
        &["0.375", "1"],
        &["0.625", "1"],
        &["0.875", "1"],
        &["-0.125", "1"],
        &["2.5", "1"],
        &["0.5", "3"],
        // Decimals that only look like halfway cases.
        &["0.005", "1"],
        &["0.015", "1"],
        &["1.005", "1"],
        &["2.675", "1"],
        &["1.045", "1"],
        &["8.835", "1"],
    ]);
}

#[test]
fn formatting_signed_zero() {
    check_all(&[
        &["0", "1"],
        &["-0", "1"],
        &["-0", "3"],
        &["-0.0", "1"],
        &["-0", "2"],
        &["-0x0", "1"],
        // pow(x, huge negative) underflows to a *signed* zero, but that path
        // sets ERANGE; a plain signed zero base is the reachable case.
        &["inf", "-1"],
        &["-inf", "-3"],
        &["-inf", "-2"],
    ]);
}

#[test]
fn formatting_infinity_and_nan() {
    // pow can return inf or nan without setting errno, so these reach the
    // success printf. glibc prints inf/-inf/nan/-nan, not Rust's default `NaN`.
    check_all(&[
        &["inf", "2"],
        &["inf", "3"],
        &["-inf", "3"],
        &["-inf", "2"],
        &["-inf", "0.5"],
        &["inf", "inf"],
        &["nan", "2"],
        &["-nan", "2"],
        &["2", "nan"],
        // pow(1, nan) == 1 and pow(nan, 0) == 1 by definition, no error.
        &["1", "nan"],
        &["nan", "0"],
        &["1", "inf"],
        &["-1", "inf"],
        &["nan", "nan"],
    ]);
}

#[test]
fn formatting_very_large_values() {
    // %.2f on a value near DBL_MAX expands to ~300 integer digits; the digits
    // must match exactly, not just the magnitude.
    check_all(&[
        &["1.7976931348623157e308", "1"],
        &["1e300", "1"],
        &["1e308", "1"],
        &["-1e300", "1"],
        &["1e100", "3"],
        &["12345678901234567890", "1"],
        &["2", "1023"],
        &["1e150", "2"],
    ]);
}

#[test]
fn formatting_precision_sensitive_values() {
    check_all(&[
        &["3.9999999999999999", "1"],
        &["0.9999999999999999", "1"],
        &["1e-20", "1"],
        &["1e-3", "1"],
        &["0.001", "1"],
        &["0.0049999999", "1"],
        &["1.4142135623730951", "2"],
        &["2", "0.5"],
        &["3", "0.5"],
        &["7", "-2"],
        &["1e-300", "1"],
    ]);
}

// ---------------------------------------------------------------------------
// Argument bytes that are not valid UTF-8 or are very long
// ---------------------------------------------------------------------------

/// The error messages echo the argument back through `%s`, i.e. raw bytes. A
/// translation that round-tripped argv through `String` would mangle these.
#[test]
fn non_utf8_arguments() {
    let cases: &[&[&[u8]]] = &[
        &[b"\xff\xfe", b"2"],
        &[b"2", b"\x80abc"],
        &[b"\xc3", b"2"],
        &[b"5\xff", b"2"],
        &[b"2", b"3\xff"],
        &[b"\xf0\x9f", b"\xf0\x9f"],
        // Valid UTF-8 but non-ASCII, echoed verbatim.
        &["é".as_bytes(), b"2"],
        &[b"2", "é".as_bytes()],
        &["2°".as_bytes(), b"2"],
    ];
    for case in cases {
        assert_same(&argv_bytes(case));
    }
}

/// Both error messages interpolate the argument with `%s`, so the bytes must
/// come back out unaltered - same case, same whitespace, same everything. These
/// use letters whose case survives `strtod` unchanged, so only the echo differs
/// if a translation normalises the argument.
#[test]
fn arguments_are_echoed_verbatim() {
    check_all(&[
        // Invalid-input branch, uppercase.
        &["ABC", "2"],
        &["2ABC", "2"],
        &["2", "ABC"],
        &["2", "3ABC"],
        &["MiXeDcAsE", "2"],
        &["2", "MiXeDcAsE"],
        &["NaNx", "2"],
        &["InFiNiTyZ", "2"],
        &["0XG", "2"],
        // Range-error branch, uppercase exponent marker: the message prints the
        // original spelling, not a normalised one.
        &["1E999", "2"],
        &["1E999XYZ", "2"],
        &["2", "1E999"],
        &["2", "1E-999"],
        &["0X1P9999", "2"],
        // Interior whitespace and punctuation, preserved exactly.
        &["1 2 3", "2"],
        &["\t A B \n", "2"],
        &["2", "1 2 3"],
        &["it's", "2"],
        &["a'b'c", "2"],
        &["%s%d%n", "2"],
        &["2", "%s%d%n"],
    ]);
}

#[test]
fn long_arguments() {
    let long_a = "a".repeat(5000);
    let long_digits = "9".repeat(400);
    let leading_zeros = format!("{}1", "0".repeat(400));
    let long_frac = format!("0.{}5", "0".repeat(300));

    assert_same(&argv(&[&long_a, "2"]));
    assert_same(&argv(&["2", &long_a]));
    assert_same(&argv(&[&long_digits, "1"]));
    assert_same(&argv(&[&leading_zeros, "1"]));
    assert_same(&argv(&[&long_frac, "1"]));
    // Many digits, all significant, ending in a valid value.
    assert_same(&argv(&[&format!("1.{}", "0".repeat(2000)), "2"]));
}

// ---------------------------------------------------------------------------
// A broad sweep, to catch anything the hand-picked cases missed
// ---------------------------------------------------------------------------

#[test]
fn integer_grid_sweep() {
    for base in -12i32..=12 {
        for exp in -6i32..=6 {
            assert_same(&argv(&[&base.to_string(), &exp.to_string()]));
        }
    }
}

#[test]
fn fractional_sweep() {
    let vals = [
        "-3.75", "-2.5", "-1.25", "-0.5", "-0.25", "0", "0.25", "0.5", "1.25", "2.5", "3.75",
        "0.1", "0.3", "1.1", "7.7", "100.5",
    ];
    for b in vals {
        for e in vals {
            assert_same(&argv(&[b, e]));
        }
    }
}

/// Deterministic pseudo-random argument pairs mixing well-formed numbers with
/// junk, so both the success and the error paths get hit with values nobody
/// chose by hand.
#[test]
fn pseudorandom_sweep() {
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    let shapes = [
        "{}", " {}", "{} ", "{}x", "0x{}", "{}e{}", "{}.{}", "-{}", "+{}", "{}e-{}", ".{}",
        "{},{}", "1e{}", "{}e999",
    ];

    for _ in 0..400 {
        let mut arg = || {
            let shape = shapes[(next() % shapes.len() as u64) as usize];
            let a = next() % 1000;
            let b = next() % 100;
            shape.replacen("{}", &a.to_string(), 1).replacen("{}", &b.to_string(), 1)
        };
        let (x, y) = (arg(), arg());
        assert_same(&argv(&[&x, &y]));
    }
}
