//! Differential integration tests: run the C binary and the Rust binary as
//! subprocesses on identical stdin and require byte-identical stdout, stderr
//! and exit status.
//!
//! The Rust program is never used as a library; both are driven exactly the
//! way a shell would drive them.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// locating / building the two executables
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // .../<root>/translation  ->  .../<root>
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn rust_bin() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// Path of the compiled C reference program, building it if necessary.
///
/// Prefers an existing `c_src/build/driver`. Otherwise configures a fully
/// out-of-source CMake build under `target/`, so nothing inside `c_src/` is
/// created or modified by the test run.
fn c_bin() -> &'static Path {
    static C: OnceLock<PathBuf> = OnceLock::new();
    C.get_or_init(|| {
        let root = workspace_root();
        let existing = root.join("c_src/build/driver");
        if existing.is_file() {
            return existing;
        }

        let src = root.join("c_src");
        let build = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/c_reference_build");
        std::fs::create_dir_all(&build).expect("create out-of-source build dir");

        let cfg = Command::new("cmake")
            .arg("-S")
            .arg(&src)
            .arg("-B")
            .arg(&build)
            .output()
            .expect("cmake must be installed to run the differential tests");
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
            .expect("cmake --build");
        assert!(
            bld.status.success(),
            "cmake build failed:\n{}\n{}",
            String::from_utf8_lossy(&bld.stdout),
            String::from_utf8_lossy(&bld.stderr)
        );

        let out = build.join("driver");
        assert!(out.is_file(), "C reference binary missing at {:?}", out);
        out
    })
}

// ---------------------------------------------------------------------------
// running one program
// ---------------------------------------------------------------------------

fn run(exe: &Path, stdin_bytes: &[u8]) -> Output {
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {exe:?}: {e}"));

    {
        let mut sin = child.stdin.take().expect("stdin pipe");
        let bytes = stdin_bytes.to_vec();
        // Write on a helper thread so a large payload cannot deadlock against
        // a full stdout pipe.
        std::thread::spawn(move || {
            let _ = sin.write_all(&bytes);
            let _ = sin.flush();
        });
    }

    child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("failed to wait for {exe:?}: {e}"))
}

fn show(b: &[u8]) -> String {
    let mut s = String::new();
    for &c in b.iter().take(120) {
        match c {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            0x0b => s.push_str("\\v"),
            0x0c => s.push_str("\\f"),
            0x20..=0x7e => s.push(c as char),
            _ => s.push_str(&format!("\\x{c:02x}")),
        }
    }
    if b.len() > 120 {
        s.push_str(&format!("...(+{} bytes)", b.len() - 120));
    }
    s
}

/// Compare all three observable channels for one input.
#[track_caller]
fn assert_same(label: &str, stdin_bytes: &[u8]) {
    let c = run(c_bin(), stdin_bytes);
    let r = run(rust_bin(), stdin_bytes);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch [{label}] input={:?}\n  C   : {:?}\n  rust: {:?}",
        show(stdin_bytes),
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch [{label}] input={:?}\n  C   : {:?}\n  rust: {:?}",
        show(stdin_bytes),
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "exit status mismatch [{label}] input={:?} C={:?} rust={:?}",
        show(stdin_bytes),
        c.status,
        r.status
    );
}

fn check_all(label: &str, cases: &[&[u8]]) {
    for case in cases {
        assert_same(label, case);
    }
}

// ---------------------------------------------------------------------------
// Phase A — both programs are runnable
// ---------------------------------------------------------------------------

#[test]
fn phase_a_both_binaries_exist_and_run() {
    assert!(c_bin().is_file(), "C binary not built");
    assert!(rust_bin().is_file(), "Rust binary not built");
    // Both must terminate normally on trivial input.
    let c = run(c_bin(), b"1");
    let r = run(rust_bin(), b"1");
    assert_eq!(c.status.code(), Some(0));
    assert_eq!(r.status.code(), Some(0));
}

// ---------------------------------------------------------------------------
// Phase B — the input classes main() branches on
// ---------------------------------------------------------------------------

/// scanf hits EOF before any conversion: input failure, `x` keeps its 0.f
/// initialiser, so the program must still print the four zero bytes.
#[test]
fn empty_input_leaves_x_at_zero() {
    check_all("empty", &[b"", b"\n", b" ", b"\t", b"\r", b"\x0b", b"\x0c", b"   \n\t\n  "]);
}

/// A single well-formed value: the happy path.
#[test]
fn single_value_decimal() {
    check_all(
        "decimal",
        &[
            b"0", b"1", b"-1", b"+1", b"2", b"-2", b"0.5", b"1.5", b"-1.5", b"3.14159",
            b"0.1", b"0.2", b"0.3", b"2.5", b"3.5", b"100", b"12345.6789", b"-0", b"-0.0",
            b"0.000", b"000000001", b".5", b"-.5", b"+.5", b"5.", b"-5.",
        ],
    );
}

/// Exponent forms, both signs of the exponent and both case spellings.
#[test]
fn decimal_exponent_forms() {
    check_all(
        "exponent",
        &[
            b"1e0", b"1E0", b"1e10", b"1E10", b"1e+10", b"1e-10", b"1.5e3", b"1.5E-3",
            b"-1.5e3", b".5e1", b"5.e1", b"0e0", b"0e999999999999999999", b"-0e5",
            b"0e-99999999999",
        ],
    );
}

/// The extremes of binary32: largest finite, smallest normal, smallest
/// subnormal, and the values just past each edge.
#[test]
fn magnitude_extremes() {
    check_all(
        "extremes",
        &[
            b"340282346638528859811704183484516925440",   // FLT_MAX exactly
            b"3.4028234663852886e38",                     // FLT_MAX
            b"3.4028235e38",
            b"3.4028236e38",
            b"340282356779733661637539395458142568447",   // just below overflow threshold
            b"340282356779733661637539395458142568448",   // overflow threshold -> inf
            b"-340282356779733661637539395458142568448",
            b"1e38",
            b"1e39",
            b"1e40",
            b"-1e40",
            b"1.1754943508222875e-38",                    // FLT_MIN (smallest normal)
            b"1.1754942106924411e-38",                    // largest subnormal
            b"5.877471754111438e-39",
            b"1.401298464324817e-45",                     // smallest subnormal
            b"7.006492321624085e-46",                     // half of it -> ties to zero
            b"7.0064923216240854721e-46",
            b"7.0064923216240854722e-46",
            b"1e-45",
            b"1e-46",
            b"1e-50",
            b"-1e-50",
        ],
    );
}

/// Every rounding boundary the binary32 significand has: 2^24 +/- 1 and the
/// classic ties-to-even midpoints.
#[test]
fn rounding_boundaries() {
    check_all(
        "rounding",
        &[
            b"16777215", b"16777216", b"16777217", b"16777218", b"16777219",
            b"16777215.5", b"16777216.5", b"16777218.5",
            b"16777217.0000000000000000000000000000000000000000000000000000000000000000001",
            b"8388609", b"8388608.5",
            b"1.000000059604644775390625",  // exact tie above 1.0
            b"1.000000059604644775390625",
            b"1.0000000596046447753906250000000000000000000001",
        ],
    );
}

/// Infinity spellings, including the truncated prefixes that make the
/// conversion fail (matching failure -> nothing stored -> 0.f is printed).
#[test]
fn infinity_and_prefixes() {
    check_all(
        "inf",
        &[
            b"inf", b"INF", b"Inf", b"iNf", b"-inf", b"+inf", b"-INF",
            b"infinity", b"INFINITY", b"iNfInItY", b"-infinity",
            b"i", b"in", b"infi", b"infin", b"infini", b"infinit",
            b"inf2", b"infx", b"infinityx", b"-infinit", b"inf inity", b"in\nf",
        ],
    );
}

/// NaN spellings, including the parenthesised n-char-sequence payload form.
#[test]
fn nan_forms() {
    check_all(
        "nan",
        &[
            b"nan", b"NAN", b"NaN", b"nAn", b"-nan", b"+nan",
            b"nan(", b"nan()", b"nan(1)", b"nan(0x1)", b"nan(abc_123)", b"nan(abc",
            b"-nan(0x7)", b"NAN(1)", b"nanx", b"na", b"n", b"nan inf",
        ],
    );
}

/// C99 hexadecimal floating literals, which glibc's scanf accepts.
#[test]
fn hex_float_forms() {
    check_all(
        "hex",
        &[
            b"0x1p0", b"0X1p0", b"0x1P0", b"0x1p3", b"0x1P3", b"-0x1p-3", b"0x1.8p1",
            b"0xAp0", b"0xap0", b"0xffp0", b"0x1.8", b"0x.8p1", b"0x0p0", b"-0x0p0",
            b"0x1p-149", b"0x1p-150", b"0x1.0000001p-150", b"0x3p-150", b"0x1p128",
            b"0x1.fffffep127", b"0X1.FFFFFEP+127", b"0x1.ffffffp+127", b"0X1.FFFFFFp+127",
            b"0x0.0000000000001p0", b"0x1p1000", b"0x1p-1000",
            b"0x1p2147483647", b"0x1p-2147483648", b"0x1p2147483648",
            b"0x1p99999999999999999999",
            b"0x10000000000000000000000000000000000000000",
            b"0xffffffffffffffffffffffffffffffffffffffffp-200",
        ],
    );
}

/// Inputs where the conversion fails outright. `x` is never assigned, so the
/// program still exits 0 and prints the representation of 0.f.
#[test]
fn matching_failure_paths() {
    check_all(
        "match-fail",
        &[
            b"abc", b"xyz", b"e5", b"E", b"E5", b".", b"-.", b"+.", b".e5", b"-.e5",
            b"+", b"-", b"++1", b"--5", b"+-5", b"- 1", b"+ 1",
            b"0x", b"0X", b"0x.", b"0xg", b"0xp1", b"0x p1",
            b"/", b":", b"@", b"\x7f", b"#1.5", b"(1.5)",
        ],
    );
}

/// Partially formed exponents: the scanner must not consume an `e`/`p` that is
/// not followed by digits.
#[test]
fn incomplete_exponent_is_pushed_back() {
    check_all(
        "partial-exponent",
        &[
            b"1e", b"1e+", b"1e-", b"1E", b"1.5e", b"1.5e+", b".5e", b"5.e-",
            b"0x1p", b"0x1p+", b"0x1p-", b"0x1.8p", b"0x1P+",
        ],
    );
}

/// scanf skips arbitrary leading whitespace, including newlines, and stops at
/// the first character that cannot extend the number.
#[test]
fn whitespace_skipping_and_early_stop() {
    check_all(
        "whitespace",
        &[
            b"   42",
            b"\t\n  7.25",
            b"\n\n\n\t\x0b\x0c\r  -2.5",
            b"\r\n1.5",
            b"5abc",
            b"1.5.5",
            b"1.5 2.5",
            b"1.5\n2.5",
            b"1.5,2.5",
            b"1 2 3 4 5",
        ],
    );
}

// ---------------------------------------------------------------------------
// Phase C — paths not covered above
// ---------------------------------------------------------------------------

/// Exponents far outside any representable range, including exponents whose
/// digit string overflows every integer width.
#[test]
fn absurd_exponents() {
    check_all(
        "absurd-exponent",
        &[
            b"1e999999999",
            b"1e-999999999",
            b"1e2147483647",
            b"1e-2147483648",
            b"1e9223372036854775807",
            b"1e-9223372036854775808",
            b"1e+9999999999999999999999",
            b"1e100000000000000000000",
            b"1e-100000000000000000000",
            b"1e99999999999999999999999999999999999999999999999999999999999999999999",
        ],
    );
}

/// Very long significands, past the point where the translation truncates and
/// keeps a sticky digit.
#[test]
fn very_long_significands() {
    let mut owned: Vec<Vec<u8>> = Vec::new();
    for len in [40usize, 100, 800, 801, 900, 2000, 5000] {
        owned.push([b"1.".as_ref(), &b"0".repeat(len), b"5"].concat());
        owned.push([b"0.".as_ref(), &b"0".repeat(len), b"1"].concat());
        owned.push(b"9".repeat(len));
        owned.push([&b"1".repeat(len)[..], b"e-", len.to_string().as_bytes()].concat());
        owned.push([b"1".as_ref(), &b"0".repeat(len), b"e-", len.to_string().as_bytes()].concat());
        owned.push([b"0x1.".as_ref(), &b"f".repeat(len), b"p0"].concat());
    }
    // 100 KB of digits.
    owned.push([b"1.".as_ref(), &b"5".repeat(100_000)].concat());
    owned.push([b"0.".as_ref(), &b"0".repeat(100_000), b"1"].concat());

    let refs: Vec<&[u8]> = owned.iter().map(|v| v.as_slice()).collect();
    check_all("long", &refs);
}

/// Bytes that are not valid UTF-8, and embedded NUL bytes. The C program is
/// byte-oriented, so the Rust one must not choke on them.
#[test]
fn non_utf8_and_nul_bytes() {
    check_all(
        "raw-bytes",
        &[
            b"\xff",
            b"\x80\x81",
            b"\xc3\x28",
            b"1.5\xff",
            b"\xff1.5",
            b"\x00",
            b"\x001.5",
            b"1\x005",
            b"\xef\xbb\xbf1.5", // UTF-8 BOM before the number
            b"\xe2\x80\x891.5", // U+2009 thin space is not C whitespace
        ],
    );
}

/// Exact midpoints between adjacent binary32 values across the whole exponent
/// range, written out in full decimal, plus a tail that breaks the tie in each
/// direction. This is where a naive decimal parser diverges from strtof.
#[test]
fn exact_ties_across_the_exponent_range() {
    // (midpoint decimal string) generated from mantissa/exponent pairs.
    let mut owned: Vec<Vec<u8>> = Vec::new();
    for &(m, e) in &[
        (0x800000u32, -149i32),
        (0x800001, -149),
        (0xffffff, -149),
        (0x800000, -126),
        (0x800001, -126),
        (0xffffff, -126),
        (0x800000, 0),
        (0x800001, 0),
        (0xabcdef, 0),
        (0xffffff, 0),
        (0x800000, 60),
        (0xffffff, 103),
        (0xffffff, 104),
    ] {
        // value = m * 2^e ; midpoint to the next representable = (2m+1) * 2^(e-1)
        let s = pow2_times_odd_decimal(2 * u64::from(m) + 1, e - 1);
        owned.push(s.clone().into_bytes());
        owned.push(format!("{s}1").into_bytes());
        owned.push(format!("{s}0000000000000000001").into_bytes());
        owned.push(format!("-{s}").into_bytes());
        owned.push(format!("-{s}1").into_bytes());
    }
    let refs: Vec<&[u8]> = owned.iter().map(|v| v.as_slice()).collect();
    check_all("ties", &refs);
}

/// Exact decimal expansion of `mant * 2^exp` using big-decimal arithmetic on
/// digit vectors (no floating point involved).
fn pow2_times_odd_decimal(mant: u64, exp: i32) -> String {
    // Represent the value as an integer digit string plus a decimal point
    // position: mant * 2^exp. For exp >= 0 multiply by 2 repeatedly; for
    // exp < 0 multiply by 5 repeatedly and shift the point (x/2 == x*5/10).
    let mut digits: Vec<u8> = mant.to_string().into_bytes().iter().map(|d| d - b'0').collect();
    let mut point_from_right = 0usize;

    let mul = |digits: &mut Vec<u8>, k: u32| {
        let mut carry = 0u32;
        for d in digits.iter_mut().rev() {
            let v = u32::from(*d) * k + carry;
            *d = (v % 10) as u8;
            carry = v / 10;
        }
        while carry > 0 {
            digits.insert(0, (carry % 10) as u8);
            carry /= 10;
        }
    };

    if exp >= 0 {
        for _ in 0..exp {
            mul(&mut digits, 2);
        }
    } else {
        for _ in 0..(-exp) {
            mul(&mut digits, 5);
            point_from_right += 1;
        }
    }

    let mut s: String = digits.iter().map(|d| (d + b'0') as char).collect();
    if point_from_right > 0 {
        while s.len() <= point_from_right {
            s.insert(0, '0');
        }
        s.insert(s.len() - point_from_right, '.');
    }
    s
}

/// A deterministic pseudo-random sweep over mixed-shape inputs, so the suite
/// exercises far more of the parser than the hand-written cases alone.
#[test]
fn deterministic_random_sweep() {
    let mut state = 0x2545_F491_4F6C_DD1Du64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    const ALPHA: &[u8] = b"0123456789.eE+-xXpPinfaNtY \t\n";

    let mut owned: Vec<Vec<u8>> = Vec::new();
    for _ in 0..400 {
        let len = (next() % 14) as usize;
        owned.push((0..len).map(|_| ALPHA[(next() % ALPHA.len() as u64) as usize]).collect());
    }
    for _ in 0..200 {
        let mantissa = next() % 1_000_000_000_000;
        let e = (next() % 121) as i64 - 60;
        owned.push(format!("{mantissa}e{e}").into_bytes());
        owned.push(format!("-{mantissa}.{mantissa}e{e}").into_bytes());
        owned.push(format!("0x{mantissa:x}p{e}").into_bytes());
    }
    let refs: Vec<&[u8]> = owned.iter().map(|v| v.as_slice()).collect();
    check_all("sweep", &refs);
}
