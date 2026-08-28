//! Differential tests: run the original C binary and the Rust binary as
//! subprocesses with identical `argv` and require byte-identical stdout,
//! byte-identical stderr and the same exit status.
//!
//! Nothing here links the Rust code as a library — the crate is an executable
//! and is driven exactly the way a shell would drive it.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(unix)]
use std::os::unix::process::CommandExt;

/// The Rust binary cargo just built for us.
const RUST_BIN: &str = env!("CARGO_BIN_EXE_driver");

/// Workspace root, i.e. the directory holding `c_src/` and `translation/`.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Configure and build `c_src` with CMake, out-of-tree so that nothing is
/// written inside `c_src/`. Returns the path of the resulting `driver`.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let root = workspace_root();
        let src = root.join("c_src");
        let build = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("c_build");

        let configure = Command::new("cmake")
            .arg("-S")
            .arg(&src)
            .arg("-B")
            .arg(&build)
            .output()
            .expect("failed to run `cmake` — is CMake installed?");
        assert!(
            configure.status.success(),
            "cmake configure failed:\n{}\n{}",
            String::from_utf8_lossy(&configure.stdout),
            String::from_utf8_lossy(&configure.stderr)
        );

        let compile = Command::new("cmake")
            .arg("--build")
            .arg(&build)
            .output()
            .expect("failed to run `cmake --build`");
        assert!(
            compile.status.success(),
            "cmake build failed:\n{}\n{}",
            String::from_utf8_lossy(&compile.stdout),
            String::from_utf8_lossy(&compile.stderr)
        );

        let bin = build.join("driver");
        assert!(bin.is_file(), "expected the C driver at {}", bin.display());
        bin
    })
}

/// Turn a raw byte string into an `OsStr`, so arguments that are not valid
/// UTF-8 can be passed through untouched, just as `execve` would.
fn os(bytes: &[u8]) -> &OsStr {
    #[cfg(unix)]
    {
        OsStr::from_bytes(bytes)
    }
    #[cfg(not(unix))]
    {
        std::str::from_utf8(bytes)
            .expect("non-UTF-8 arguments are only supported on unix")
            .as_ref()
    }
}

/// `argv[0]` handed to *both* programs. The C `main` prints `argv[0]` on the
/// error path, so the two processes have to agree on it for stderr to match
/// byte for byte.
const ARGV0: &str = "driver";

fn run(program: &Path, args: &[&[u8]]) -> Output {
    let mut cmd = Command::new(program);
    #[cfg(unix)]
    cmd.arg0(ARGV0);
    for a in args {
        cmd.arg(os(a));
    }
    cmd.output()
        .unwrap_or_else(|e| panic!("failed to run {}: {e}", program.display()))
}

fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).escape_debug().to_string()
}

fn show_args(args: &[&[u8]]) -> String {
    let joined: Vec<String> = args.iter().map(|a| format!("{:?}", show(a))).collect();
    format!("[{}]", joined.join(", "))
}

/// Assert stdout, stderr and the exit status all agree.
#[track_caller]
fn assert_same(args: &[&[u8]]) {
    let c = run(c_bin(), args);
    let r = run(Path::new(RUST_BIN), args);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout differs for argv {}\n  C:    \"{}\"\n  Rust: \"{}\"",
        show_args(args),
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr differs for argv {}\n  C:    \"{}\"\n  Rust: \"{}\"",
        show_args(args),
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "exit status differs for argv {}: C {:?} vs Rust {:?}",
        show_args(args),
        c.status.code(),
        r.status.code()
    );
}

/// Every 3-tuple drawn from `values`.
fn assert_same_triples(values: &[&[u8]]) {
    for a in values {
        for b in values {
            for c in values {
                assert_same(&[a, b, c]);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// `argc` validation — `if (argc != 4)` in main.c
// ---------------------------------------------------------------------------

/// Zero, one, two, four and five operands all take the error path: a message on
/// stderr that starts with `argv[0]`, nothing on stdout, and `exit(1)`.
#[test]
fn wrong_argument_count_is_an_error() {
    assert_same(&[]);
    assert_same(&[b"1"]);
    assert_same(&[b"1", b"2"]);
    assert_same(&[b"1", b"2", b"3", b"4"]);
    assert_same(&[b"1", b"2", b"3", b"4", b"5"]);
    assert_same(&[b"1", b"2", b"3", b"4", b"5", b"6"]);
    // Empty strings still count towards argc.
    assert_same(&[b"", b""]);
    assert_same(&[b"", b"", b"", b""]);
}

/// The one accepted shape: exactly three operands.
#[test]
fn three_arguments_is_the_happy_path() {
    assert_same(&[b"3", b"4", b"0"]);
    assert_same(&[b"1", b"2", b"3"]);
    assert_same(&[b"-1", b"-2", b"-3"]);
    assert_same(&[b"0.5", b"0.25", b"0.125"]);
}

// ---------------------------------------------------------------------------
// `atof` (glibc `strtod`) input classes
// ---------------------------------------------------------------------------

/// Text with no numeric prefix converts to `0.0`. Note the sign is *not*
/// applied when no conversion happens: `-.` yields `+0.0`, not `-0.0`, which
/// the `%f` output makes visible.
#[test]
fn unconvertible_text_becomes_zero() {
    let bad: &[&[u8]] = &[
        b"", b"x", b"abc", b".", b"-.", b"+.", b"--3", b"++1", b"-", b"+", b" ", b"\t", b"e5",
        b"E", b"-e1", b"junk", b"NULL", b"0x", b"-0x", b"+0x", b"0X", b"-0xg",
    ];
    for a in bad {
        assert_same(&[a, b"1", b"1"]);
        assert_same(&[b"1", a, b"1"]);
        assert_same(&[b"1", b"1", a]);
        assert_same(&[a, a, a]);
    }
}

/// Leading whitespace is skipped, trailing garbage stops the scan.
#[test]
fn whitespace_and_trailing_garbage() {
    let cases: &[&[u8]] = &[
        b" 1.5",
        b"\t2.5",
        b"\n3.5",
        b"\r4.5",
        b"\x0b5.5",
        b"\x0c6.5",
        b"   \t 7.5",
        b"2.5 ",
        b"3.5x",
        b"1.5e-5f",
        b"4.5junk",
        b"6.5.5",
        b"7,5",
        b"1 2 3",
    ];
    for a in cases {
        assert_same(&[a, b"1", b"1"]);
        assert_same(&[b"2", a, b"3"]);
        assert_same(&[b"2", b"3", a]);
    }
}

/// Signs, bare fractions, bare integers and every exponent shape, including the
/// incomplete exponents that `strtod` must not consume.
#[test]
fn decimal_syntax_variants() {
    let cases: &[&[u8]] = &[
        b"1",
        b"+1",
        b"-1",
        b".5",
        b"-.5",
        b"5.",
        b"-5.",
        b"0.0",
        b"-0.0",
        b"00012",
        b"1e2",
        b"1E2",
        b"1e+2",
        b"1e-2",
        b"+.5e1",
        b"-.5E-1",
        b"5e",
        b"5E",
        b"3e+",
        b"3e-",
        b"1.5e",
        b"7e08",
        b"2.5e0000001",
    ];
    for a in cases {
        assert_same(&[a, b"1", b"2"]);
        assert_same(&[b"1", a, b"2"]);
        assert_same(&[b"1", b"2", a]);
    }
}

/// C99 hexadecimal floating point literals, which glibc's `strtod` accepts.
#[test]
fn hexadecimal_float_syntax() {
    let cases: &[&[u8]] = &[
        b"0x10",
        b"0X10",
        b"-0x10",
        b"0x1.8p3",
        b"0X1P1",
        b"0x1p-4",
        b"0x.8p1",
        b"0x8.p-1",
        b"0xabcdefp0",
        b"0xABCDEFp0",
        b"0x1p",
        b"0x1p+",
        b"0x1.8",
        b"0x0",
        b"-0x0",
        b"0x1p1000",
        b"0x1p-1000",
        b"0xfffffffffffffffffffffp0",
    ];
    for a in cases {
        assert_same(&[a, b"1", b"1"]);
        assert_same(&[b"1", a, b"1"]);
        assert_same(&[b"1", b"1", a]);
    }
}

/// `inf`/`infinity` and `nan`, in every spelling and both signs.
#[test]
fn infinity_and_nan_spellings() {
    let cases: &[&[u8]] = &[
        b"inf",
        b"INF",
        b"Inf",
        b"-inf",
        b"+inf",
        b"infinity",
        b"INFINITY",
        b"-Infinity",
        b"infin",
        b"nan",
        b"NAN",
        b"NaN",
        b"-nan",
        b"+nan",
        b"nan(123)",
        b"-nan(abc)",
        b"nan(",
        b"na",
        b"in",
    ];
    for a in cases {
        assert_same(&[a, b"1", b"1"]);
        assert_same(&[b"1", a, b"1"]);
        assert_same(&[b"1", b"1", a]);
    }
}

// ---------------------------------------------------------------------------
// Range limits: `double` conversion followed by narrowing to `float`
// ---------------------------------------------------------------------------

/// The largest and smallest magnitudes the code can represent, plus the values
/// that overflow or underflow on the way in.
#[test]
fn magnitude_limits() {
    let cases: &[&[u8]] = &[
        // float limits
        b"3.4028234663852886e38",   // FLT_MAX
        b"-3.4028234663852886e38",  // -FLT_MAX
        b"3.4028236e38",            // just past FLT_MAX -> inf as float
        b"1.1754943508222875e-38",  // FLT_MIN (smallest normal)
        b"1.401298464324817e-45",   // smallest float subnormal
        b"7.006492321624085e-46",   // rounds to a subnormal / zero
        b"1e-46",                   // underflows the float type
        // double limits
        b"1.7976931348623157e308",  // DBL_MAX -> inf as float
        b"-1.7976931348623157e308",
        b"5e-324",                  // smallest double subnormal -> 0.0f
        b"1e400",                   // strtod overflow -> HUGE_VAL
        b"-1e400",
        b"1e-400",                  // strtod underflow -> 0
        b"-1e-400",
        // huge literals written out in full
        b"340282346638528859811704183484516925440",
        b"-340282346638528859811704183484516925440",
        b"0.000000000000000000000000000000000000000000001",
    ];
    for a in cases {
        assert_same(&[a, b"0", b"0"]);
        assert_same(&[b"0", a, b"0"]);
        assert_same(&[b"0", b"0", a]);
        assert_same(&[a, a, a]);
        assert_same(&[a, b"1", b"-1"]);
    }
}

// ---------------------------------------------------------------------------
// `Q_rsqrt` / `VectorNormalizeFast` numeric paths
// ---------------------------------------------------------------------------

/// A zero-length vector: the routine deliberately does not guard against it, so
/// `Q_rsqrt(0)` runs the bit hack on `0x00000000` and the result is whatever
/// falls out. Signed zeros must keep their sign through the multiply.
#[test]
fn zero_length_vector() {
    let zeros: &[&[u8]] = &[b"0", b"-0", b"0.0", b"-0.0", b"+0", b"1e-46", b"-1e-46"];
    assert_same_triples(zeros);
}

/// Squaring the components overflows to `+inf`, so `Q_rsqrt` returns `-inf` and
/// the components come back sign-flipped — or as NaN where `0 * inf` happens.
#[test]
fn dot_product_overflows_to_infinity() {
    let cases: &[&[u8]] = &[b"1e38", b"-1e38", b"3e38", b"inf", b"-inf", b"0", b"-0", b"1"];
    assert_same_triples(cases);
}

/// Subnormal and near-subnormal components, where the squares underflow to zero
/// and the dot product loses all information.
#[test]
fn subnormal_components() {
    let cases: &[&[u8]] = &[
        b"1e-45", b"-1e-45", b"1e-38", b"-1e-38", b"1e-20", b"1e-22", b"1e-23", b"0",
    ];
    assert_same_triples(cases);
}

/// NaN propagation. Mixing `+nan` and `-nan` is the interesting case: which NaN
/// survives depends on the operand order of each SSE instruction, so the sign
/// printed for each component is not simply "the sign of some input".
#[test]
fn nan_propagation_and_sign() {
    let cases: &[&[u8]] = &[b"nan", b"-nan", b"1", b"-1", b"0", b"-0", b"inf", b"-inf", b"1e38"];
    assert_same_triples(cases);
}

// ---------------------------------------------------------------------------
// `printf("%f %f %f\n", ...)` formatting
// ---------------------------------------------------------------------------

/// Six fractional digits, a single space between fields, one trailing newline,
/// and the C spellings `inf` / `-inf` / `nan` / `-nan`.
#[test]
fn output_formatting() {
    assert_same(&[b"1", b"0", b"0"]);
    assert_same(&[b"-1", b"0", b"0"]);
    assert_same(&[b"0", b"0", b"0"]);
    assert_same(&[b"-0", b"-0", b"-0"]);
    assert_same(&[b"inf", b"inf", b"inf"]);
    assert_same(&[b"-inf", b"-inf", b"-inf"]);
    assert_same(&[b"nan", b"nan", b"nan"]);
    assert_same(&[b"-nan", b"-nan", b"-nan"]);
    // Values whose exact expansion needs rounding at the 6th decimal.
    assert_same(&[b"1", b"1", b"1"]);
    assert_same(&[b"2", b"3", b"6"]);
    assert_same(&[b"1e-20", b"1", b"1e20"]);
    assert_same(&[b"0.1", b"0.2", b"0.3"]);
    assert_same(&[b"12345.678", b"-98765.4321", b"0.000001"]);
}

// ---------------------------------------------------------------------------
// Non-UTF-8 and oversized arguments
// ---------------------------------------------------------------------------

/// `argv` is a byte string, not text: invalid UTF-8 must reach `strtod`
/// unchanged (and convert to `0.0`), and a leading number must still be read
/// out of a byte string with garbage after it.
#[test]
fn non_utf8_and_long_arguments() {
    let long_digits = vec![b'9'; 400];
    let long_zeros = {
        let mut v = b"0.".to_vec();
        v.extend(std::iter::repeat(b'0').take(400));
        v.push(b'1');
        v
    };
    let cases: &[&[u8]] = &[
        b"\xff\xfe",
        b"\x80",
        b"1.5\xff",
        b"\xc3\x28",
        b"\xff1.5",
        &long_digits,
        &long_zeros,
    ];
    for a in cases {
        assert_same(&[a, b"1", b"1"]);
        assert_same(&[b"1", a, b"1"]);
        assert_same(&[b"1", b"1", a]);
    }
}

// ---------------------------------------------------------------------------
// Deterministic sweep
// ---------------------------------------------------------------------------

/// A reproducible pseudo-random sweep over ordinary, extreme and malformed
/// operands, to catch anything the hand-written classes above miss.
#[test]
fn deterministic_random_sweep() {
    // xorshift64*, so the sweep is identical on every run and every machine.
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = move || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state = state.wrapping_mul(0x2545_F491_4F6C_DD1D);
        state
    };

    let literals: [&[u8]; 12] = [
        b"inf", b"-inf", b"nan", b"-nan", b"0", b"-0", b"", b"x", b"1e400", b"1e-400", b"-.",
        b"0x1.8p3",
    ];

    for _ in 0..250 {
        let mut args: Vec<Vec<u8>> = Vec::with_capacity(3);
        for _ in 0..3 {
            let r = next();
            args.push(match r % 4 {
                // An arbitrary f32 bit pattern, printed with enough digits to
                // round-trip.
                0 => format!("{:e}", f32::from_bits((r >> 32) as u32)).into_bytes(),
                // A moderate decimal value.
                1 => format!(
                    "{}.{}",
                    (r >> 32) as i32 % 1000,
                    (r >> 8) as u16 % 10_000
                )
                .into_bytes(),
                // A power-of-ten scaled value.
                2 => format!("{}e{}", (r >> 40) as u32 % 100_000, (r as i8) % 45).into_bytes(),
                // One of the awkward literals.
                _ => literals[(r >> 16) as usize % literals.len()].to_vec(),
            });
        }
        let refs: Vec<&[u8]> = args.iter().map(|a| a.as_slice()).collect();
        assert_same(&refs);
    }
}
