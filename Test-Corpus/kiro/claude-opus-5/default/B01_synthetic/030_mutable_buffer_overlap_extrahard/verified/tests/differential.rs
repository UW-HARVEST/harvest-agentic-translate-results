//! Differential tests: run the C reference binary and the Rust binary as
//! subprocesses over the same stdin, and require byte-identical stdout,
//! byte-identical stderr and an identical exit status.
//!
//! Nothing here links the Rust crate as a library. Both programs are driven
//! exactly the way a shell drives them, because that is how they are compared.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Locating / building the two executables
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// The Rust program under test, built by cargo in the same profile as this test.
fn rust_bin() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// The C reference program.
///
/// Prefers the tree the task instructions build (`c_src/build/driver`). If that
/// is absent, the C program is configured and built into cargo's `target/`
/// directory so that nothing inside `c_src/` is created or modified.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let root = workspace_root();

        let prebuilt = root.join("c_src").join("build").join("driver");
        if prebuilt.is_file() {
            return prebuilt;
        }

        // Fall back to an out-of-source build under target/.
        let src = root.join("c_src");
        let out = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("c_reference_build");
        std::fs::create_dir_all(&out).expect("create C build directory");

        let configure = Command::new("cmake")
            .arg("-S")
            .arg(&src)
            .arg("-B")
            .arg(&out)
            .output()
            .expect(
                "the C reference binary is missing and `cmake` could not be run; \
                 build it first with: cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .",
            );
        assert!(
            configure.status.success(),
            "cmake configure failed:\n{}\n{}",
            String::from_utf8_lossy(&configure.stdout),
            String::from_utf8_lossy(&configure.stderr)
        );

        let build = Command::new("cmake")
            .arg("--build")
            .arg(&out)
            .output()
            .expect("run cmake --build");
        assert!(
            build.status.success(),
            "cmake --build failed:\n{}\n{}",
            String::from_utf8_lossy(&build.stdout),
            String::from_utf8_lossy(&build.stderr)
        );

        let built = out.join("driver");
        assert!(built.is_file(), "C reference binary not produced at {built:?}");
        built
    })
}

// ---------------------------------------------------------------------------
// Running a program
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Ok(code)` for a normal exit, `Err(signal)` when killed by a signal.
    status: Result<i32, i32>,
}

fn run(bin: &Path, input: &[u8]) -> Outcome {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {bin:?}: {e}"));

    {
        let mut stdin = child.stdin.take().expect("stdin pipe");
        // The child may stop reading (it stops after 100 integers), so a short
        // write failure here is expected and not an error.
        let _ = stdin.write_all(input);
        let _ = stdin.flush();
    }

    let out = child.wait_with_output().expect("wait for child");

    let status = match out.status.code() {
        Some(code) => Ok(code),
        None => {
            #[cfg(unix)]
            {
                use std::os::unix::process::ExitStatusExt;
                Err(out.status.signal().unwrap_or(-1))
            }
            #[cfg(not(unix))]
            {
                Err(-1)
            }
        }
    };

    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        status,
    }
}

fn show(bytes: &[u8]) -> String {
    // Escape so that whitespace differences are visible in failure output.
    let mut s = String::new();
    for &b in bytes.iter().take(4096) {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\t' => s.push_str("\\t"),
            b'\r' => s.push_str("\\r"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    if bytes.len() > 4096 {
        s.push_str("...<truncated>");
    }
    s
}

fn show_status(s: &Result<i32, i32>) -> String {
    match s {
        Ok(c) => format!("exit {c}"),
        Err(sig) => format!("killed by signal {sig}"),
    }
}

/// Assert that the C and Rust programs agree on stdout, stderr and exit status.
#[track_caller]
fn assert_identical(label: &str, input: &[u8]) {
    let c = run(c_bin(), input);
    let r = run(rust_bin(), input);

    let mut problems = Vec::new();
    if c.stdout != r.stdout {
        problems.push(format!(
            "stdout differs ({} vs {} bytes)\n  C   : \"{}\"\n  Rust: \"{}\"",
            c.stdout.len(),
            r.stdout.len(),
            show(&c.stdout),
            show(&r.stdout)
        ));
    }
    if c.stderr != r.stderr {
        problems.push(format!(
            "stderr differs\n  C   : \"{}\"\n  Rust: \"{}\"",
            show(&c.stderr),
            show(&r.stderr)
        ));
    }
    if c.status != r.status {
        problems.push(format!(
            "status differs\n  C   : {}\n  Rust: {}",
            show_status(&c.status),
            show_status(&r.status)
        ));
    }

    assert!(
        problems.is_empty(),
        "case `{label}` mismatched\ninput ({} bytes): \"{}\"\n{}",
        input.len(),
        show(input),
        problems.join("\n")
    );
}

#[track_caller]
fn check(label: &str, input: &str) {
    assert_identical(label, input.as_bytes());
}

/// Build a whitespace-separated input from a list of values.
fn joined(values: &[i64], sep: &str) -> String {
    values
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(sep)
}

// ---------------------------------------------------------------------------
// Phase A sanity: both programs exist and run
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_run() {
    let c = run(c_bin(), b"1\n");
    let r = run(rust_bin(), b"1\n");
    assert_eq!(c.status, Ok(0), "C reference should exit 0");
    assert_eq!(r.status, Ok(0), "Rust program should exit 0");
    assert_eq!(c.stdout, b"2\n".to_vec(), "C reference output for input `1`");
    assert_eq!(c.stdout, r.stdout);
    assert!(c.stderr.is_empty() && r.stderr.is_empty());
}

// ---------------------------------------------------------------------------
// Phase B: the input classes main()/driver()/fma_array() branch on
// ---------------------------------------------------------------------------

// `main`: the read loop terminates immediately -> driver(data, 0) -> no output.
#[test]
fn empty_input() {
    check("empty", "");
}

#[test]
fn whitespace_only_input() {
    // `%d` skips whitespace, then hits EOF: scanf != 1 on the first iteration.
    check("single_space", " ");
    check("single_newline", "\n");
    check("many_newlines", "\n\n\n\n");
    check("mixed_ws_only", " \t\r\n\x0b\x0c ");
    check("tabs_only", "\t\t\t");
}

// `driver`/`fma_array`: len == 1, the loop body runs exactly once.
#[test]
fn single_item() {
    check("single_no_newline", "5");
    check("single_with_newline", "5\n");
    check("single_leading_ws", "   \n\t 5\n");
    check("single_trailing_ws", "5   \n\n");
    check("zero", "0");
    check("one", "1");
    check("negative", "-3\n");
    check("explicit_plus", "+7\n");
    check("negative_zero", "-0\n");
    check("plus_zero", "+0\n");
}

// `%d` skips *all* whitespace, newlines included, so layout is irrelevant.
#[test]
fn scanf_reads_across_newlines() {
    check("one_per_line", "1\n2\n3\n4\n5\n");
    check("all_one_line", "1 2 3 4 5");
    check("all_one_line_nl", "1 2 3 4 5\n");
    check("crlf", "1\r\n2\r\n3\r\n");
    check("vt_ff_separated", "1\x0b2\x0c3");
    check("wild_spacing", "  1\t\t2\n\n\n   3\r\n\x0b4     5  \n");
    check("no_trailing_newline", "1 2 3");
    check("leading_blank_lines", "\n\n\n7\n8\n");
}

// `main`: `scanf` returns 0 (matching failure) -> break. Nothing was assigned,
// so only the items read before the failure are printed.
#[test]
fn matching_failure_paths() {
    check("garbage_only", "abc");
    check("garbage_only_nl", "abc\n");
    check("garbage_first", "x 1 2 3");
    check("garbage_mid", "1 2 x 3 4");
    check("garbage_last", "1 2 3 x");
    check("digits_then_letters", "12abc");
    check("digits_then_letters_more", "1 2 34xyz 5");
    check("hex_literal", "0x10");
    check("hex_literal_upper", "0X1F");
    check("lone_minus", "-");
    check("lone_plus", "+");
    check("minus_then_letter", "-x");
    check("plus_then_letter", "+x");
    check("double_minus", "--5");
    check("plus_minus", "+-5");
    check("minus_space_digits", "- 5");
    check("decimal_point", "1.5");
    check("decimal_point_only", ".5");
    check("comma_separated", "1,2,3");
    check("exponent_notation", "1e3");
    check("underscore", "1_000");
    check("nul_byte_mid", "1\u{0}2");
    check("nul_byte_first", "\u{0}1");
    check("high_byte", "1 \u{e9} 2");
    check("slash", "1/2");
    check("star", "*");
}

// `printf("%d\n", ...)` formatting at the extremes of `int`.
#[test]
fn int_extremes() {
    check("int_max", "2147483647");
    check("int_min", "-2147483648");
    check("int_max_minus_one", "2147483646");
    check("int_min_plus_one", "-2147483647");
    check("both_extremes", "2147483647 -2147483648 0 1 -1");
}

// glibc's `%d` converts through `long` and narrows to `int`: values that fit a
// long are truncated, values that do not are clamped to LONG_MAX/LONG_MIN first.
#[test]
fn scanf_out_of_int_range_truncates() {
    check("just_over_int_max", "2147483648");
    check("just_under_int_min", "-2147483649");
    check("two_to_32", "4294967296");
    check("two_to_32_plus_one", "4294967297");
    check("two_to_31_plus_one", "2147483649");
    check("long_max", "9223372036854775807");
    check("long_min", "-9223372036854775808");
    check("over_long_max", "9223372036854775808");
    check("under_long_min", "-9223372036854775809");
    check("far_over_long_max", "99999999999999999999");
    check("far_under_long_min", "-99999999999999999999");
    check("absurdly_large", &"9".repeat(400));
    check("absurdly_small", &format!("-{}", "9".repeat(400)));
    check("mixed_ranges", "1 4294967296 -1 9223372036854775807 2");
}

// Long digit runs, which is where an accumulating parser is most likely to
// diverge from glibc's convert-then-narrow behaviour.
#[test]
fn long_digit_runs() {
    for n in [1usize, 9, 10, 18, 19, 20, 21, 25, 50, 200, 1000, 5000] {
        check(&format!("nines_{n}"), &"9".repeat(n));
        check(&format!("neg_nines_{n}"), &format!("-{}", "9".repeat(n)));
        check(&format!("ones_{n}"), &"1".repeat(n));
        check(&format!("zeros_then_5_{n}"), &format!("{}5", "0".repeat(n)));
        check(&format!("neg_zeros_then_5_{n}"), &format!("-{}5", "0".repeat(n)));
        check(&format!("plus_zeros_then_7_{n}"), &format!("+{}7", "0".repeat(n)));
        check(&format!("only_zeros_{n}"), &"0".repeat(n));
    }
}

#[test]
fn leading_zeros_are_decimal_not_octal() {
    check("leading_zeros", "000005");
    check("leading_zeros_octalish", "010 011 017 08 09");
    check("neg_leading_zeros", "-000005");
}

// `fma_array`: out[i] = out[i]*out[i] + out[i] on `int`, so the product wraps
// for |x| >= 46341.
#[test]
fn multiplication_overflow_wraps() {
    let cases: &[i64] = &[
        46340,
        46341,
        46342,
        -46340,
        -46341,
        -46342,
        65535,
        65536,
        65537,
        100000,
        -100000,
        1000000,
        -1000000,
        2147483647,
        -2147483648,
        1073741824,
        -1073741824,
        2147483646,
        123456789,
        -123456789,
        50000,
        -50000,
        99999,
        -99999,
        46341 * 2,
    ];
    for v in cases {
        check(&format!("mul_overflow_{v}"), &v.to_string());
    }
    // All of them together, in one run of the loop.
    check("mul_overflow_all", &joined(cases, "\n"));
    check("mul_overflow_all_spaced", &joined(cases, " "));
}

#[test]
fn add_overflow_wraps() {
    // x*x wraps to a value near INT_MAX/INT_MIN and the +x then wraps again.
    let cases: &[i64] = &[
        46349, 46350, 46351, 92682, -92682, 189812, -189812, 2000000000, -2000000000,
        1518500249, -1518500249, 715827883, 1431655765,
    ];
    for v in cases {
        check(&format!("add_overflow_{v}"), &v.to_string());
    }
    check("add_overflow_all", &joined(cases, "\n"));
}

// `main`: the read loop bound `i < 100`.
#[test]
fn array_capacity_boundary() {
    let vals99: Vec<i64> = (1..=99).collect();
    let vals100: Vec<i64> = (1..=100).collect();
    let vals101: Vec<i64> = (1..=101).collect();

    check("ninety_eight", &joined(&(1..=98).collect::<Vec<i64>>(), "\n"));
    check("ninety_nine", &joined(&vals99, "\n"));
    check("exactly_100", &joined(&vals100, "\n"));
    check("exactly_100_trailing_nl", &format!("{}\n", joined(&vals100, "\n")));
    check("one_hundred_one", &joined(&vals101, "\n"));

    // Excess input past the 100th item is never read.
    check("one_hundred_fifty", &joined(&(1..=150).collect::<Vec<i64>>(), "\n"));
    check("one_thousand", &joined(&(1..=1000).collect::<Vec<i64>>(), " "));
    check(
        "hundred_then_garbage",
        &format!("{} xyz not a number", joined(&vals100, " ")),
    );
    check(
        "hundred_then_garbage_no_space",
        &format!("{}xyz", joined(&vals100, " ")),
    );
    check(
        "ninety_nine_then_garbage",
        &format!("{} xyz 101 102", joined(&vals99, " ")),
    );
    check(
        "hundred_then_lots_of_whitespace",
        &format!("{}\n\n\n   \t\n", joined(&vals100, "\n")),
    );

    // A full array of values that all overflow the multiply.
    check("hundred_overflowing", &joined(&vec![100000i64; 100], " "));
    check("hundred_int_max", &joined(&vec![2147483647i64; 100], "\n"));
    check("hundred_int_min", &joined(&vec![-2147483648i64; 100], "\n"));
}

#[test]
fn item_counts_one_through_hundred() {
    // Every possible value of `len` seen by driver()/fma_array().
    for n in 0..=100usize {
        let vals: Vec<i64> = (0..n as i64).map(|k| k * 37 - 500).collect();
        check(&format!("count_{n}"), &joined(&vals, "\n"));
    }
}

// ---------------------------------------------------------------------------
// Phase C: broader sweeps over inputs no single hand-written case reaches
// ---------------------------------------------------------------------------

/// Deterministic xorshift, so the sweep is reproducible without a dev-dependency.
struct Rng(u64);

impl Rng {
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}

#[test]
fn sweep_random_byte_soup() {
    // Random bytes drawn from the alphabet the parser actually branches on.
    const ALPHA: &[u8] = b"0123456789 \n\t\r+-.,xabeE*/\x0b\x0c\x00";
    let mut rng = Rng(0x2545F4914F6CDD1D);
    for case in 0..600 {
        let len = rng.below(64) as usize;
        let input: Vec<u8> = (0..len)
            .map(|_| ALPHA[rng.below(ALPHA.len() as u64) as usize])
            .collect();
        assert_identical(&format!("byte_soup_{case}"), &input);
    }
}

#[test]
fn sweep_random_integers() {
    let mut rng = Rng(0x9E3779B97F4A7C15);
    let seps = [" ", "\n", "\t", "  \n  ", "\r\n", "\n\n"];
    for case in 0..400 {
        let count = rng.below(121) as usize;
        let mut parts: Vec<String> = Vec::with_capacity(count);
        for _ in 0..count {
            let v: i64 = match rng.below(6) {
                0 => (rng.next_u64() as i64 % 4294967296) - 2147483648,
                1 => (rng.next_u64() as i64 % 200001) - 100000,
                2 => {
                    const EDGE: [i64; 12] = [
                        0, 1, -1, 2, -2, 46340, 46341, -46341, 65536, 2147483647, -2147483648,
                        1073741824,
                    ];
                    EDGE[rng.below(EDGE.len() as u64) as usize]
                }
                3 => rng.next_u64() as i64,
                4 => (rng.next_u64() as i64 % 2000000001) - 1000000000,
                _ => (rng.next_u64() as i64) / 3,
            };
            parts.push(v.to_string());
        }
        let sep = seps[rng.below(seps.len() as u64) as usize];
        assert_identical(&format!("int_sweep_{case}"), parts.join(sep).as_bytes());
    }
}

#[test]
fn sweep_valid_prefix_then_junk() {
    // Every prefix length crossed with a junk token: exercises `break` at each
    // possible value of `i`.
    let junk = ["x", "-", "+", ".", ",", "abc", "0x1", "1.5e3", "\u{0}"];
    let mut rng = Rng(0xDEADBEEFCAFEBABE);
    for n in [0usize, 1, 2, 3, 17, 50, 98, 99, 100, 101] {
        for j in junk {
            let vals: Vec<i64> = (0..n as i64).map(|k| (k * 991) % 70001 - 35000).collect();
            let mut s = joined(&vals, " ");
            if !s.is_empty() {
                s.push(' ');
            }
            s.push_str(j);
            // Trailing digits after the junk are unreachable, but include them.
            s.push_str(" 7 8 9");
            assert_identical(&format!("prefix_{n}_junk_{}", rng.next_u64()), s.as_bytes());
        }
    }
}

#[test]
fn sweep_single_values_dense() {
    // Dense scan around every wrap boundary of x*x + x.
    let mut inputs: Vec<i64> = Vec::new();
    for base in [
        0i64,
        46340,
        -46340,
        65535,
        -65535,
        1 << 15,
        1 << 16,
        1 << 20,
        1073741823,
        2147483640,
        -2147483640,
    ] {
        for d in -4i64..=4 {
            inputs.push(base + d);
        }
    }
    for v in &inputs {
        check(&format!("dense_{v}"), &v.to_string());
    }
    check("dense_all", &joined(&inputs, "\n"));
}
