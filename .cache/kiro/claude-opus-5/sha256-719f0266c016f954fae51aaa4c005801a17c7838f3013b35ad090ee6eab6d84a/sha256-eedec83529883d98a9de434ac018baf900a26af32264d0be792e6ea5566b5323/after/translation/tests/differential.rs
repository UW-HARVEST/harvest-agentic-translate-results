//! Differential tests: run the original C `driver` and the Rust `driver` as
//! subprocesses with identical `argv` and require byte-identical stdout,
//! byte-identical stderr and an identical exit status.
//!
//! The Rust code is never linked in as a library — only the built executable is
//! driven, exactly the way the graders (and a shell) do it.
//!
//! `argv[0]` is part of the observable output (the usage message prints it), so
//! both executables are copied into sibling directories under the same name
//! `driver` and invoked as `./driver` with the working directory set. That makes
//! `argv[0]` literally `./driver` for both programs.

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Output, Stdio};
use std::sync::OnceLock;

/// Root of the repository (parent of the `translation` crate).
fn repo_root() -> PathBuf {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .parent()
        .expect("crate dir has a parent")
        .to_path_buf()
}

fn target_dir() -> PathBuf {
    // `CARGO_BIN_EXE_driver` is <target>/<profile>/driver
    let exe = PathBuf::from(env!("CARGO_BIN_EXE_driver"));
    exe.parent()
        .and_then(|p| p.parent())
        .expect("bin path has target dir ancestors")
        .to_path_buf()
}

/// Configure and build the C program out-of-tree so that nothing inside
/// `c_src/` is written to. Returns the path of the built executable.
fn build_c_binary() -> PathBuf {
    let src = repo_root().join("c_src");
    assert!(
        src.join("CMakeLists.txt").is_file(),
        "cannot find c_src/CMakeLists.txt at {}",
        src.display()
    );

    // Prefer an already-built binary from the canonical in-tree build dir.
    let prebuilt = src.join("build").join("driver");
    if prebuilt.is_file() {
        return prebuilt;
    }

    let build = target_dir().join("c_build");
    std::fs::create_dir_all(&build).expect("create c build dir");

    let cfg = Command::new("cmake")
        .arg("-S")
        .arg(&src)
        .arg("-B")
        .arg(&build)
        .output()
        .expect("failed to run cmake (is cmake installed?)");
    assert!(
        cfg.status.success(),
        "cmake configure failed:\n{}\n{}",
        String::from_utf8_lossy(&cfg.stdout),
        String::from_utf8_lossy(&cfg.stderr)
    );

    let bld = Command::new("cmake")
        .arg("--build")
        .arg(&build)
        .output()
        .expect("failed to run cmake --build");
    assert!(
        bld.status.success(),
        "cmake build failed:\n{}\n{}",
        String::from_utf8_lossy(&bld.stdout),
        String::from_utf8_lossy(&bld.stderr)
    );

    let out = build.join("driver");
    assert!(out.is_file(), "C build produced no {}", out.display());
    out
}

struct Sandbox {
    c_dir: PathBuf,
    rust_dir: PathBuf,
}

fn sandbox() -> &'static Sandbox {
    static SANDBOX: OnceLock<Sandbox> = OnceLock::new();
    SANDBOX.get_or_init(|| {
        let base = target_dir().join("difftest");
        let c_dir = base.join("c");
        let rust_dir = base.join("rust");
        std::fs::create_dir_all(&c_dir).expect("create c sandbox");
        std::fs::create_dir_all(&rust_dir).expect("create rust sandbox");

        let c_bin = build_c_binary();
        let rust_bin = PathBuf::from(env!("CARGO_BIN_EXE_driver"));

        copy_exe(&c_bin, &c_dir.join("driver"));
        copy_exe(&rust_bin, &rust_dir.join("driver"));

        Sandbox { c_dir, rust_dir }
    })
}

fn copy_exe(from: &Path, to: &Path) {
    // Copying (rather than symlinking) keeps `argv[0]` identical and avoids any
    // dependency on how the platform resolves symlinks.
    std::fs::copy(from, to)
        .unwrap_or_else(|e| panic!("copy {} -> {}: {e}", from.display(), to.display()));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(to).expect("stat copied exe").permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(to, perms).expect("chmod copied exe");
    }
}

fn run_in(dir: &Path, args: &[&[u8]]) -> Output {
    let mut cmd = Command::new("./driver");
    cmd.current_dir(dir);
    for a in args {
        cmd.arg(OsStr::from_bytes(a));
    }
    // A deterministic, minimal environment: locale can influence C's printf
    // decimal point, and the graders run in the default C locale.
    cmd.env_remove("LC_ALL")
        .env_remove("LC_NUMERIC")
        .env_remove("LANG");
    cmd.output().expect("failed to spawn ./driver")
}

fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

fn quote(args: &[&[u8]]) -> String {
    args.iter()
        .map(|a| format!("{:?}", String::from_utf8_lossy(a)))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The core assertion: stdout, stderr and exit status must all match.
#[track_caller]
fn assert_same(args: &[&[u8]]) {
    let sb = sandbox();
    let c = run_in(&sb.c_dir, args);
    let r = run_in(&sb.rust_dir, args);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout differs for argv [{}]\n  C   : {}\n  Rust: {}",
        quote(args),
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr differs for argv [{}]\n  C   : {}\n  Rust: {}",
        quote(args),
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        c.status,
        r.status,
        "exit status differs for argv [{}]: C {:?} (code {:?}) vs Rust {:?} (code {:?})",
        quote(args),
        c.status,
        c.status.code(),
        r.status,
        r.status.code()
    );
}

fn assert_same_all(cases: &[&[&[u8]]]) {
    for c in cases {
        assert_same(c);
    }
}

/// Convenience: two string arguments.
#[track_caller]
fn same2(base: &str, exponent: &str) {
    assert_same(&[base.as_bytes(), exponent.as_bytes()]);
}

// ---------------------------------------------------------------------------
// argc: `if (argc != 3)` -> usage on stderr, exit 1
// ---------------------------------------------------------------------------

#[test]
fn argc_not_three_prints_usage() {
    // Zero extra arguments (empty input), one, three and four.
    assert_same(&[]);
    assert_same(&[b"2"]);
    assert_same(&[b"2", b"3", b"4"]);
    assert_same(&[b"2", b"3", b"4", b"5"]);
    // Empty-string arguments still count towards argc.
    assert_same(&[b""]);
    assert_same(&[b"", b"", b""]);
}

#[test]
fn argc_exactly_three_is_the_only_accepted_form() {
    // The single shape that reaches the conversion code.
    same2("2", "10");
    // Both arguments empty: strtod converts nothing, so endptr == nptr == "" and
    // *endptr == '\0'; the C accepts it as 0 and prints pow(0,0).
    same2("", "");
    same2("", "2");
    same2("2", "");
}

// ---------------------------------------------------------------------------
// base conversion: ERANGE branch, then the `*endptr1 != '\0'` branch
// ---------------------------------------------------------------------------

#[test]
fn base_invalid_numeric_input() {
    for s in [
        "abc", "x", "--3", "+-2", "++2", ".", "+", "-", "+.", "-.", "e5", "E", "1 2", "5 ", " 5 ",
        "3.5.5", "1,5", "1_000", "0x", "0X", "0x.", "0xp3", "0x1z", "0x1p", "0x1p+", "0x1pz",
        "1e5x", "na", "n", "i", "in", "nan(12", "nan(a-b)", "inf inity", "0x10x10", "00x10",
        "1e", "1e+", "1e-", "#", "\u{7f}",
    ] {
        same2(s, "2");
    }
}

#[test]
fn base_range_error_takes_precedence_over_trailing_garbage() {
    // strtod sets ERANGE *and* leaves trailing text; the C checks ERANGE first.
    same2("1e999abc", "2");
    same2("1e-999zzz", "2");
    same2("1e400 ", "2");
}

#[test]
fn base_range_error_overflow_and_underflow() {
    for s in [
        "1e309",
        "-1e309",
        "1e400",
        "1e999999999999999999999",
        "1e+999999999999999999999",
        "-1e999999999999999999999",
        "1e-320",
        "5e-324",
        "2e-324",
        "1e-400",
        "1e-999999999999999999999",
        "0x1p-1075",
        "0x1.8p-1074",
        "0x1p1024",
        "0x1.fffffffffffffp-1023",
    ] {
        same2(s, "1");
    }
}

// ---------------------------------------------------------------------------
// exponent conversion: the same two branches, on argv[2]
// ---------------------------------------------------------------------------

#[test]
fn exponent_invalid_numeric_input() {
    for s in [
        "abc", "--3", ".", "+", "-", "e5", "3.5.5", "0x", "0xp3", "1e5x", "na", "nan(12", "1e",
        "1e+",
    ] {
        same2("2", s);
    }
}

#[test]
fn exponent_range_error() {
    for s in [
        "1e309",
        "-1e400",
        "1e-320",
        "5e-324",
        "1e999999999999999999999",
        "0x1p-1075",
        "1e-400abc",
    ] {
        same2("2", s);
    }
}

#[test]
fn base_is_validated_before_exponent() {
    // Both arguments are bad: only the base's message must appear.
    same2("abc", "def");
    same2("1e999", "abc");
    same2("abc", "1e999");
    same2("1e999", "1e999");
    same2("1e-999", "1e999");
}

// ---------------------------------------------------------------------------
// accepted strtod forms (all reach pow)
// ---------------------------------------------------------------------------

#[test]
fn strtod_accepted_decimal_forms() {
    for s in [
        "0", "-0", "+0", "1", "-1", "007", "0.000", ".5", "5.", "+5", "-5", "1.5", "-1.5", "2.5",
        "0.1", "1e5", "1E5", "1e+5", "1e-5", "0e999999999999999999999", "1e308", "1e-308",
        "2.2250738585072014e-308", "1.7976931348623157e308", "9007199254740993",
        "18446744073709551616", "0.3333333333333333",
    ] {
        same2(s, "1");
    }
}

#[test]
fn strtod_leading_whitespace_is_skipped() {
    // The C-locale space set: ' ', \t, \n, \v, \f, \r.
    for s in [
        " 5", "\t5", "\n5", "\u{b}5", "\u{c}5", "\r5", " \t\n\u{b}\u{c}\r 3.5", "  +5", "  -5",
    ] {
        same2(s, "2");
    }
    // Whitespace only: no conversion, endptr == nptr, so *endptr != '\0'.
    for s in [" ", "\t", "\n", "\u{b}", "\u{c}", "\r", "   "] {
        same2(s, "2");
    }
    // A non-breaking space is not C whitespace.
    assert_same(&[b"\xa05", b"2"]);
}

#[test]
fn strtod_hexadecimal_forms() {
    for s in [
        "0x10", "0X10", "-0x10", "0x1p4", "0X1P+2", "0x1.8p1", "0x1.p3", "0x.8p1", "0x.8", "0x8.",
        "0xABCDEF", "0x0", "0x0p0", "0x1P3", "0x1p-1074", "0x1p-1023", "0x1p-1022",
        "0x1.fffffffffffffp1023", "0x1.FFFFFFFFFFFFF8p-1023", "0x1p1e3",
    ] {
        same2(s, "1");
    }
}

#[test]
fn strtod_infinity_and_nan_forms() {
    for s in [
        "inf",
        "INF",
        "Inf",
        "-inf",
        "+inf",
        "infinity",
        "INFINITY",
        "InFiNiTy",
        "infinityx",
        "nan",
        "NAN",
        "-nan",
        "nan(123)",
        "nan()",
        "nan(_a1)",
        "-nan(1)",
    ] {
        same2(s, "2");
        same2("2", s);
    }
}

#[test]
fn strtod_very_long_inputs() {
    let long_int = format!("1{}", "0".repeat(400));
    let long_frac = format!("0.{}1", "0".repeat(400));
    let nines = "9".repeat(1000);
    let long_hex = format!("0x1.{}p0", "f".repeat(300));
    let huge_exp = format!("1e{}", "9".repeat(30));
    let tiny_exp = format!("1e-{}", "9".repeat(30));
    let zero_huge_exp = format!("0e{}", "9".repeat(30));
    let many_zeros = format!("1{}", "0".repeat(100_000));
    let cases: Vec<Vec<&[u8]>> = vec![
        vec![long_int.as_bytes(), b"1"],
        vec![long_frac.as_bytes(), b"1"],
        vec![nines.as_bytes(), b"1"],
        vec![long_hex.as_bytes(), b"1"],
        vec![huge_exp.as_bytes(), b"1"],
        vec![tiny_exp.as_bytes(), b"1"],
        vec![zero_huge_exp.as_bytes(), b"1"],
        vec![many_zeros.as_bytes(), b"1"],
    ];
    for c in &cases {
        assert_same(c);
    }
}

#[test]
fn strtod_subnormal_boundary() {
    // Tininess is detected after rounding and additionally requires the result
    // to be inexact, so some subnormal results set ERANGE and some do not.
    for s in [
        "2.2250738585072014e-308",
        "2.2250738585072013e-308",
        "2.2250738585072012e-308",
        "2.225073858507201e-308",
        "1e-310",
        "1e-323",
        "4.9e-324",
        "3e-324",
        "0x1p-1074",
        "0x1.8p-1074",
        "0x1.0000000000001p-1074",
        "0x1.fffffffffffffp-1023",
        "0x1.FFFFFFFFFFFFF7p-1023",
    ] {
        same2(s, "1");
    }
    for m in 1..40u32 {
        same2(&format!("{m}e-324"), "1");
        same2(&format!("{m}e-320"), "1");
    }
}

#[test]
fn strtod_non_utf8_arguments() {
    // argv is bytes, not text; the error messages echo the raw bytes.
    assert_same(&[b"\xff\xfe", b"2"]);
    assert_same(&[b"2", b"\x80\x81"]);
    assert_same(&[b"3\xff", b"2"]);
    assert_same(&[b"\xc3", b"\xc3"]);
}

// ---------------------------------------------------------------------------
// pow: the EDOM branch
// ---------------------------------------------------------------------------

#[test]
fn pow_domain_error_negative_base_fractional_exponent() {
    for (b, e) in [
        ("-2", "0.5"),
        ("-2", "2.5"),
        ("-2", "-0.5"),
        ("-3", "3.5"),
        ("-1", "0.5"),
        ("-0.5", "0.5"),
        ("-8", "0.3333333333333333"),
        ("-1.5", "2.5"),
        ("-1e300", "1.5"),
        ("-4.9e-324", "0.5"),
    ] {
        same2(b, e);
    }
}

#[test]
fn pow_negative_base_integral_exponent_is_fine() {
    for (b, e) in [
        ("-2", "2"),
        ("-2", "3"),
        ("-2", "-3"),
        ("-2", "2.0"),
        ("-2", "1e20"),
        ("-2", "4503599627370496"),
        ("-1", "inf"),
        ("-1", "-inf"),
        ("-0.0", "3"),
        ("-3", "-3"),
    ] {
        same2(b, e);
    }
}

// ---------------------------------------------------------------------------
// pow: the ERANGE branch
// ---------------------------------------------------------------------------

#[test]
fn pow_range_error_pole() {
    for (b, e) in [
        ("0", "-1"),
        ("0", "-0.5"),
        ("-0", "-1"),
        ("-0", "-3"),
        ("-0", "-0.5"),
        ("0.0", "-2"),
    ] {
        same2(b, e);
    }
}

#[test]
fn pow_range_error_overflow() {
    for (b, e) in [
        ("10", "400"),
        ("10", "309"),
        ("2", "1024"),
        ("1e300", "2"),
        ("-10", "401"),
        ("0.1", "-400"),
        ("1.7976931348623157e308", "1.0000000001"),
    ] {
        same2(b, e);
    }
}

#[test]
fn pow_range_error_underflow() {
    for (b, e) in [
        ("10", "-400"),
        ("10", "-324"),
        ("2", "-1075"),
        ("2", "-1080"),
        ("0.1", "400"),
        ("1e-300", "2"),
        ("-10", "-401"),
    ] {
        same2(b, e);
    }
}

#[test]
fn pow_underflow_boundary_subnormal_results_do_not_set_errno() {
    // A merely subnormal (non-zero) result is not a range error for glibc.
    for e in -1080..-1060 {
        same2("2", &e.to_string());
    }
    for e in -330..-300 {
        same2("10", &e.to_string());
    }
    for e in ["-1074", "-1074.5", "-1073.5", "-1022.5", "-1076.0"] {
        same2("2", e);
        same2("0.5", e);
    }
}

#[test]
fn pow_overflow_boundary() {
    for e in 300..320 {
        same2("10", &e.to_string());
    }
    for e in 1020..1030 {
        same2("2", &e.to_string());
    }
}

// ---------------------------------------------------------------------------
// pow: the cases that set no errno at all
// ---------------------------------------------------------------------------

#[test]
fn pow_no_errno_special_cases() {
    for (b, e) in [
        ("nan", "2"),
        ("2", "nan"),
        ("nan", "0"),
        ("1", "nan"),
        ("-nan", "2"),
        ("2", "-nan"),
        ("nan", "nan"),
        ("inf", "2"),
        ("inf", "-2"),
        ("-inf", "2"),
        ("-inf", "3"),
        ("-inf", "-3"),
        ("2", "inf"),
        ("2", "-inf"),
        ("0.5", "-inf"),
        ("0", "inf"),
        ("0", "-inf"),
        ("inf", "inf"),
        ("-inf", "-inf"),
        ("1", "inf"),
        ("1", "1e999999"),
        ("100", "0"),
        ("0", "0"),
        ("0", "1"),
        ("3", "-0"),
        ("-0.0", "-3"),
    ] {
        same2(b, e);
    }
}

// ---------------------------------------------------------------------------
// printf("Result: %.2f\n", ...) formatting
// ---------------------------------------------------------------------------

#[test]
fn result_formatting_rounding_ties() {
    // Exactly representable ties for two fraction digits must round half to
    // even, the way glibc's exact decimal conversion does.
    for den in [8i32, 16, 32, 64, 128, 256] {
        for k in -40..=40i32 {
            let v = f64::from(k) / f64::from(den);
            same2(&format!("{v:?}"), "1");
        }
    }
}

#[test]
fn result_formatting_wide_and_small_magnitudes() {
    for s in [
        "1e308",
        "1.7976931348623157e308",
        "1e300",
        "1e17",
        "1e-30",
        "-1e-30",
        "0.999",
        "0.995",
        "-0.995",
        "0.005",
        "-0.005",
        "0.125",
        "0.375",
        "2.675",
        "0x1p-1074",
        "-0x1p-1074",
        "-0.0",
    ] {
        same2(s, "1");
    }
    // Very long %.2f expansions.
    for e in 300..309 {
        same2("10", &e.to_string());
    }
    same2("2", "1023");
}

#[test]
fn result_formatting_negative_zero_and_signed_zero_results() {
    for (b, e) in [
        ("-0.0", "3"),
        ("-0.0", "5"),
        ("-2", "-1075"),
        ("0", "5"),
        ("-0", "2"),
    ] {
        same2(b, e);
    }
}

// ---------------------------------------------------------------------------
// A broad cross product, to catch anything the enumerated classes miss.
// ---------------------------------------------------------------------------

#[test]
fn cross_product_of_interesting_tokens() {
    const TOKENS: &[&str] = &[
        "",
        "0",
        "-0",
        "1",
        "-1",
        "2",
        "-2",
        "0.5",
        "-0.5",
        "1e308",
        "1e309",
        "1e-308",
        "1e-320",
        "5e-324",
        "inf",
        "-inf",
        "nan",
        "-nan",
        "0x1p10",
        "0x1.8p-1074",
        "0X",
        "abc",
        " 3 ",
        "+.5",
        "1e",
        "--1",
        "3",
        "-3",
        "2.5",
        "-2.5",
        "1024",
        "0.1",
        "1.5",
    ];
    for a in TOKENS {
        for b in TOKENS {
            same2(a, b);
        }
    }
}

#[test]
fn deterministic_pseudo_random_sweep() {
    // A small xorshift keeps this reproducible without extra dependencies.
    let mut state = 0x2545_F491_4F6C_DD1Du64;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let token = |r: u64| -> String {
        match r % 5 {
            0 => format!("{}", (r >> 8) as i32 % 2000),
            1 => format!("{:?}", (((r >> 8) % 40_000) as f64) / 1000.0 - 20.0),
            2 => format!("{}e{}", (r >> 8) % 10, ((r >> 20) % 700) as i64 - 350),
            3 => format!("0x{:x}p{}", (r >> 8) % 0xffff, ((r >> 32) % 2200) as i64 - 1100),
            _ => format!("{:?}", f64::from_bits(r).abs().min(1e300)),
        }
    };
    for _ in 0..250 {
        let a = token(next());
        let b = token(next());
        same2(&a, &b);
    }
}

#[test]
fn closed_stdout_pipe_kills_both_programs_with_sigpipe() {
    // A C program inherits the default SIGPIPE disposition; the Rust runtime
    // sets SIG_IGN before main. With the read end of the pipe already closed the
    // very first write fails with EPIPE, so the difference is deterministic:
    // the C program dies from SIGPIPE while an unpatched Rust program exits 0.
    let sb = sandbox();
    for (args, redirect) in [
        (vec![b"2".as_slice(), b"10".as_slice()], Stream::Stdout),
        (vec![b"x".as_slice(), b"y".as_slice()], Stream::Stderr),
    ] {
        let c = status_with_closed_pipe(&sb.c_dir, &args, redirect);
        let r = status_with_closed_pipe(&sb.rust_dir, &args, redirect);
        assert_eq!(
            c, r,
            "closed-{redirect:?} status differs for argv [{}]: C {:?} vs Rust {:?}",
            quote(&args),
            c,
            r
        );
        assert_eq!(
            c.signal(),
            Some(13),
            "expected the C program to die from SIGPIPE for argv [{}]",
            quote(&args)
        );
    }
}

#[derive(Copy, Clone, Debug)]
enum Stream {
    Stdout,
    Stderr,
}

fn status_with_closed_pipe(dir: &Path, args: &[&[u8]], which: Stream) -> ExitStatus {
    let (reader, writer) = std::io::pipe().expect("create pipe");
    drop(reader); // no reader: the child's first write gets EPIPE

    let mut cmd = Command::new("./driver");
    cmd.current_dir(dir);
    for a in args {
        cmd.arg(OsStr::from_bytes(a));
    }
    match which {
        Stream::Stdout => {
            cmd.stdout(Stdio::from(writer)).stderr(Stdio::null());
        }
        Stream::Stderr => {
            cmd.stderr(Stdio::from(writer)).stdout(Stdio::null());
        }
    }
    cmd.status().expect("spawn ./driver")
}

#[test]
fn suite_actually_compares_two_distinct_programs() {
    // Guard against the sandbox silently pointing both runs at one binary.
    let sb = sandbox();
    let c = std::fs::read(sb.c_dir.join("driver")).expect("read C driver");
    let r = std::fs::read(sb.rust_dir.join("driver")).expect("read Rust driver");
    assert_ne!(c, r, "the two sandboxes hold the same executable");
    // And that a known-good happy path really produces output.
    let out = run_in(&sb.c_dir, &[b"2", b"10"]);
    assert_eq!(out.stdout, b"Result: 1024.00\n");
    let out = run_in(&sb.rust_dir, &[b"2", b"10"]);
    assert_eq!(out.stdout, b"Result: 1024.00\n");
}

#[test]
fn unused_helper_is_exercised() {
    // Keep `assert_same_all` in use so the helper set stays warning-free.
    assert_same_all(&[&[b"2", b"3"], &[b"-2", b"0.5"], &[b"x"]]);
}
