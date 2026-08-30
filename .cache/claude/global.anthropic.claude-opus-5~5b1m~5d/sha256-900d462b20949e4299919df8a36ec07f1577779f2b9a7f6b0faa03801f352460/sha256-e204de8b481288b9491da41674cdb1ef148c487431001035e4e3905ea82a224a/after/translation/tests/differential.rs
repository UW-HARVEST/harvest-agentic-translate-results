//! Differential tests: run the original C executable and the Rust executable
//! as subprocesses on the same stdin and require byte-identical stdout,
//! byte-identical stderr and an identical exit status.
//!
//! The Rust code is never used as a library here; only the built binary is
//! driven, exactly the way a shell would drive it.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Locating / building the two executables
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Builds `c_src` with CMake (once per test binary) and returns the path to the
/// resulting `driver` executable.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = repo_root().join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");
        let exe_win = build.join("Debug").join("driver.exe");

        if !exe.exists() && !exe_win.exists() {
            std::fs::create_dir_all(&build).expect("cannot create c_src/build");

            let cfg = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("failed to spawn `cmake` - is CMake installed?");
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
                .expect("failed to spawn `cmake --build`");
            assert!(
                bld.status.success(),
                "cmake build failed:\n{}\n{}",
                String::from_utf8_lossy(&bld.stdout),
                String::from_utf8_lossy(&bld.stderr)
            );
        }

        if exe.exists() {
            exe
        } else {
            exe_win
        }
    })
}

// ---------------------------------------------------------------------------
// Running one program
// ---------------------------------------------------------------------------

fn run(program: &Path, stdin_bytes: &[u8]) -> Output {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", program.display()));

    {
        let mut si = child.stdin.take().expect("piped stdin");
        // The child may exit before consuming all of stdin (it only performs a
        // single scanf), so a broken pipe here is expected and not an error.
        let _ = si.write_all(stdin_bytes);
        let _ = si.flush();
    }

    child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("failed to wait for {}: {e}", program.display()))
}

fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

/// Asserts stdout, stderr and exit status all match for one input.
fn assert_same(input: &[u8]) {
    let c = run(c_bin(), input);
    let r = run(&rust_bin(), input);

    let label = format!("input = {:?}", String::from_utf8_lossy(input));

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch for {label}\n  C:    {:?}\n  Rust: {:?}",
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch for {label}\n  C:    {:?}\n  Rust: {:?}",
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "exit-status mismatch for {label}: C {:?} vs Rust {:?}",
        c.status,
        r.status
    );
}

fn assert_all(inputs: &[&str]) {
    for s in inputs {
        assert_same(s.as_bytes());
    }
}

// ---------------------------------------------------------------------------
// Phase A - both programs are runnable and produce something
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_run() {
    let c = run(c_bin(), b"1.5\n");
    let r = run(&rust_bin(), b"1.5\n");
    assert!(!c.stdout.is_empty(), "C program produced no stdout");
    assert_eq!(c.stdout, r.stdout);
    assert_eq!(c.stderr, r.stderr);
    assert_eq!(c.status.code(), r.status.code());
    // Sanity: the documented output shape "<hexbits> <%a> <%.4f>\n".
    assert_eq!(show(&c.stdout), "3ff8000000000000 0x1.8p+0 1.5000\n");
}

// ---------------------------------------------------------------------------
// Phase B - the input classes the C code branches on
// ---------------------------------------------------------------------------

/// `scanf` returns EOF: `f` keeps its initialiser 0.0 and `driver` still runs.
#[test]
fn empty_and_whitespace_only_input() {
    assert_all(&[
        "", " ", "\n", "\t", "\r", "\x0b", "\x0c", "   \n\n\t\r\x0b\x0c   ", "\n\n\n",
    ]);
}

/// A single well-formed item - the happy path.
#[test]
fn single_plain_values() {
    assert_all(&[
        "0", "1", "-1", "+1", "2.5", "3.14", "-3.14", "0.5", ".5", "5.", "-.5", "+.5",
        "00000000001", "1.0", "100", "-0", "-0.0", "+0",
    ]);
}

/// `scanf` skips arbitrary leading white space, including newlines, and stops
/// at the first character that cannot extend the number.
#[test]
fn scanf_reads_across_newlines_and_stops_early() {
    assert_all(&[
        "\n\n\n1.25", "   \t\n 2.5", "\x0b7", "\x0c8", "\r9", "12 34", "1\n2", "1.5xyz", "1.5\n\n\n",
        "42abc", "7,5",
    ]);
    // Trailing input after the converted item is simply never read.
    assert_same(b"3.5\nleftover text\n");
}

/// Matching failure: nothing is converted, `f` stays 0.0, exit status is still 0.
#[test]
fn matching_failure_leaves_value_zero() {
    assert_all(&[
        "abc", "+", "-", ".", "+.", "-.", "e5", "E", "x", "+x", "-abc", ",", "#", "/", "z9",
        " \t +", "..5",
    ]);
}

/// Special decimal exponent forms, including the backtracking cases where the
/// exponent marker is consumed and then pushed back.
#[test]
fn decimal_exponents_and_backtracking() {
    assert_all(&[
        "1e10", "1e-10", "1e+5", "1E5", "1E-5", "0e", "0e+", "0e-", "1.5e", "1.5e+", "1.5e-",
        "1.5ex", "1e", "2e0", "9e9", "1e00005", "1.5e+003",
    ]);
}

/// Infinity spellings, case insensitivity and the partial-match failures.
#[test]
fn infinity_forms() {
    assert_all(&[
        "inf",
        "INF",
        "Inf",
        "-inf",
        "+inf",
        "infinity",
        "INFINITY",
        "iNfInItY",
        "-infinity",
        "+infinity",
        "in",
        "i",
        "infi",
        "infin",
        "infini",
        "infinit",
        "-infinit",
        "inf inity",
        "iNfInItY9",
        "infx",
        "infinityz",
    ]);
}

/// NaN, its sign, and the optional parenthesised n-char-sequence.
#[test]
fn nan_forms() {
    assert_all(&[
        "nan",
        "NAN",
        "NaN",
        "-nan",
        "+nan",
        "nan()",
        "nan(123)",
        "nan(0x123abc_)",
        "nan(abc",
        "NAN(",
        "na",
        "n",
        "nax",
        "nan9",
        "-nan(1)",
    ]);
}

/// Hexadecimal floating literals, including the "0x" with no digits failure and
/// the "0x." special case.
#[test]
fn hex_float_forms() {
    assert_all(&[
        "0x1p3",
        "0X1P-3",
        "0x1p+3",
        "0x",
        "0X",
        "-0x",
        "0x.",
        "0x.8p1",
        "0xabcdef",
        "0xABCDEF",
        "0x0p0",
        "0X0P0",
        "0x0p999999",
        "-0x0p+3",
        "0x1",
        "0x1.",
        "0x1.8",
        "0x1p",
        "0x1p+",
        "0x1p-",
        "0x1px",
        "0x1.8pz",
        "0x10",
        "-0x1.921fb54442d18p+1",
        "0x.0000000000001p-1022",
    ]);
}

/// Overflow to infinity, underflow to zero, subnormals and the exact
/// representable extremes.
#[test]
fn overflow_underflow_and_subnormals() {
    assert_all(&[
        "1e308",
        "1e309",
        "1e400",
        "-1e400",
        "1e-400",
        "1e-320",
        "5e-324",
        "2.5e-324",
        "2.4703282292062327e-324",
        "4.9406564584124654e-324",
        "1e-323",
        "1.7976931348623157e308",
        "1.7976931348623159e308",
        "-1.7976931348623159e308",
        "0x1.fffffffffffffp1023",
        "0x1.fffffffffffff8p1023",
        "0x1p1024",
        "0x1p-1074",
        "0x1p-1075",
        "0x0.8p-1073",
        "-0x1p-1075",
        "1e99999999999999999999",
        "1e-99999999999999999999",
    ]);
    // Absurdly long literals.
    assert_same(&[b"9".repeat(5000)].concat());
    assert_same(&[b"0.".to_vec(), b"0".repeat(400), b"1".to_vec()].concat());
    assert_same(&[b"1".to_vec(), b"0".repeat(400)].concat());
    assert_same(&[b"1e".to_vec(), b"9".repeat(40)].concat());
    assert_same(&[b"1e-".to_vec(), b"9".repeat(40)].concat());
    assert_same(&[b"0x1p".to_vec(), b"9".repeat(40)].concat());
    assert_same(&[b"0x1p-".to_vec(), b"9".repeat(40)].concat());
    assert_same(&[b" ".repeat(10000), b"42".to_vec()].concat());
}

/// `%.4f` rounding, including exact ties (dyadic rationals whose decimal
/// expansion ends with a 5 in the fifth fractional place).
#[test]
fn fixed_four_rounding_and_ties() {
    assert_all(&[
        "0.03125", "0.09375", "0.15625", "0.21875", "0.28125", "0.34375", "0.40625", "0.46875",
        "-0.03125", "-0.09375", "0.00005", "1.00005", "0.00004999", "0.00005001", "0.99995",
        "0.99999", "1.99995", "12345678901234567890", "1e10", "1e-10", "123456.789",
        "-123456.789",
    ]);
}

/// Non-UTF-8 and embedded NUL bytes on stdin.
#[test]
fn non_utf8_input() {
    for input in [
        &b"\x00"[..],
        &b"\xff\xfe"[..],
        &b"1.5\x00abc"[..],
        &b"\x801"[..],
        &b"1\xff"[..],
        &b"-\x00"[..],
        &b"\xc3\x28"[..],
    ] {
        assert_same(input);
    }
}

// ---------------------------------------------------------------------------
// Phase C - systematic sweeps over the numeric edge space
// ---------------------------------------------------------------------------

/// Deterministic xorshift64* so the sweep is reproducible.
struct Rng(u64);

impl Rng {
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}

/// Every f64 bit pattern is reachable: feed exact hex literals derived from
/// random bit patterns so the round trip through scanf must be exact.
#[test]
fn sweep_random_hex_bit_patterns() {
    let mut rng = Rng(0x1234_5678_9abc_def1);
    for _ in 0..1200 {
        let bits = rng.next_u64();
        let f = f64::from_bits(bits);
        if !f.is_finite() {
            continue;
        }
        let sign = if bits >> 63 != 0 { "-" } else { "" };
        let exp = ((bits >> 52) & 0x7ff) as i64;
        let mant = bits & 0x000f_ffff_ffff_ffff;
        let (lead, e) = if exp == 0 { ('0', -1022) } else { ('1', exp - 1023) };
        let s = format!("{sign}0x{lead}.{mant:013x}p{e}");
        assert_same(s.as_bytes());
    }
}

/// Every single-bit subnormal, both signs. `%a` renders these with a leading
/// `0x0.` digit and a fixed `p-1022` exponent, and the significand's trailing
/// zeros must be trimmed exactly the way glibc trims them.
#[test]
fn sweep_all_single_bit_subnormals() {
    for k in 0..52u32 {
        for sign in [0u64, 1u64 << 63] {
            let bits = sign | (1u64 << k);
            let m = bits & 0x000f_ffff_ffff_ffff;
            let s = format!(
                "{}0x0.{m:013x}p-1022",
                if sign != 0 { "-" } else { "" }
            );
            assert_same(s.as_bytes());
        }
    }
}

/// Significands with trailing zero nibbles at every position, for the smallest,
/// largest and mid-range exponents - the trailing-zero trimming in `%a`.
#[test]
fn sweep_trailing_zero_significands() {
    for exp in [0u64, 1, 1022, 1023, 1024, 2046] {
        for k in 0..53u32 {
            let m = (0x000f_ffff_ffff_ffffu64 >> k) << k;
            let bits = (exp << 52) | (m & 0x000f_ffff_ffff_ffff);
            let e: i64 = if exp == 0 { -1022 } else { exp as i64 - 1023 };
            let lead = if exp == 0 { '0' } else { '1' };
            let mm = bits & 0x000f_ffff_ffff_ffff;
            assert_same(format!("0x{lead}.{mm:013x}p{e}").as_bytes());
        }
    }
}

/// Random decimal literals across the whole exponent range.
#[test]
fn sweep_random_decimals() {
    let mut rng = Rng(0xdead_beef_cafe_babe);
    for _ in 0..1200 {
        let digits = 1 + rng.below(22);
        let mut m = String::new();
        for i in 0..digits {
            let d = rng.below(10) as u8;
            m.push((b'0' + d) as char);
            if i == 0 && digits > 1 && rng.below(3) == 0 {
                m.push('.');
            }
        }
        let e = rng.below(700) as i64 - 350;
        let sign = if rng.below(2) == 0 { "-" } else { "" };
        let s = format!("{sign}{m}e{e}");
        assert_same(s.as_bytes());
    }
}

/// Random hex literals with over-long significands, exercising the sticky-bit
/// and dropped-bits paths of the significand accumulation.
#[test]
fn sweep_random_long_hex() {
    const HEX: &[u8] = b"0123456789abcdefABCDEF";
    let mut rng = Rng(0x0bad_c0de_0bad_c0de);
    for _ in 0..1200 {
        let nint = 1 + rng.below(60);
        let nfrac = rng.below(40);
        let mut s = String::from("0x");
        for _ in 0..nint {
            s.push(HEX[rng.below(HEX.len() as u64) as usize] as char);
        }
        s.push('.');
        for _ in 0..nfrac {
            s.push(HEX[rng.below(16) as usize] as char);
        }
        let p = rng.below(2400) as i64 - 1200;
        s.push_str(&format!("p{p}"));
        assert_same(s.as_bytes());
    }
}

/// Random short garbage strings built from the alphabet the parser branches on;
/// most of these hit matching-failure and backtracking paths.
#[test]
fn sweep_random_garbage() {
    const ALPHA: &[u8] = b"0123456789+-.eEpPxXinfaNt() \n\t";
    let mut rng = Rng(0x5eed_5eed_5eed_5eed);
    for _ in 0..2000 {
        let len = rng.below(9);
        let s: Vec<u8> = (0..len)
            .map(|_| ALPHA[rng.below(ALPHA.len() as u64) as usize])
            .collect();
        assert_same(&s);
    }
}

/// Walk the decimal/binary exponent boundaries around the subnormal region and
/// around overflow, where rounding decisions are hardest.
#[test]
fn sweep_boundaries() {
    for e in -330i32..=-290 {
        for m in ["1", "2", "3", "4.9", "5", "5.1", "9.99"] {
            assert_same(format!("{m}e{e}").as_bytes());
        }
    }
    for e in 290i32..=320 {
        assert_same(format!("1.7976931348623157e{e}").as_bytes());
    }
    for p in -1090i32..=-1020 {
        for m in [
            "0x1",
            "0x1.8",
            "0x1.0000000000001",
            "0x1.fffffffffffff",
            "0x3",
            "0x1.7ffffffffffff",
        ] {
            assert_same(format!("{m}p{p}").as_bytes());
        }
    }
    for p in 1020i32..=1030 {
        for m in [
            "0x1",
            "0x1.fffffffffffff",
            "0x1.fffffffffffff8",
            "0x1.ffffffffffffffff",
        ] {
            assert_same(format!("{m}p{p}").as_bytes());
        }
    }
}
