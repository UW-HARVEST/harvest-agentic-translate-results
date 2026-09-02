//! Differential tests: run the original C `driver` and the Rust `driver` as
//! subprocesses over the same stdin bytes and require byte-identical stdout,
//! byte-identical stderr and an identical exit status.
//!
//! The Rust program is never linked as a library here; both sides are driven
//! exactly the way a shell would drive them.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Locating / building the two executables
// ---------------------------------------------------------------------------

/// Root of the checkout that contains both `c_src/` and `translation/`.
fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is `<root>/translation`.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the Rust binary under test, provided by cargo for integration tests.
fn rust_bin() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// Path to the C binary, building it with CMake on first use if needed.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = repo_root().join("c_src");
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
            .expect("failed to run `cmake ..` (is cmake installed?)");
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
            .expect("failed to run `cmake --build .`");
        assert!(
            compile.status.success(),
            "cmake build failed:\n{}\n{}",
            String::from_utf8_lossy(&compile.stdout),
            String::from_utf8_lossy(&compile.stderr)
        );

        assert!(
            exe.is_file(),
            "expected the C executable at {} after building",
            exe.display()
        );
        exe
    })
}

// ---------------------------------------------------------------------------
// Running one program
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Ok(code)` for a normal exit, `Err(signal)` when killed by a signal.
    status: Result<i32, i32>,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Outcome")
            .field("status", &self.status)
            .field("stdout", &Preview(&self.stdout))
            .field("stderr", &Preview(&self.stderr))
            .finish()
    }
}

/// Renders at most a few hundred bytes so a failure message stays readable.
struct Preview<'a>(&'a [u8]);

impl std::fmt::Debug for Preview<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let shown = &self.0[..self.0.len().min(400)];
        write!(f, "{:?}", String::from_utf8_lossy(shown))?;
        if self.0.len() > shown.len() {
            write!(f, " ...(+{} bytes, {} total)", self.0.len() - shown.len(), self.0.len())?;
        }
        Ok(())
    }
}

fn run(exe: &Path, input: &[u8]) -> Outcome {
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));

    // Feed stdin on a helper thread: the programs may emit more output than a
    // pipe buffer holds, so writing and reading must not deadlock.
    let mut stdin = child.stdin.take().expect("stdin was piped");
    let owned = input.to_vec();
    let writer = std::thread::spawn(move || {
        // A broken pipe here is legitimate (the child may exit without
        // draining stdin), so the error is intentionally ignored.
        let _ = stdin.write_all(&owned);
        let _ = stdin.flush();
        drop(stdin);
    });

    let out = child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("failed to wait for {}: {e}", exe.display()));
    writer.join().expect("stdin writer thread panicked");

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

/// Asserts the C and Rust programs agree on stdout, stderr and exit status.
#[track_caller]
fn assert_same(label: &str, input: &[u8]) {
    let c = run(c_bin(), input);
    let r = run(rust_bin(), input);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch for case `{label}`\n  input : {:?}\n  C     : {:?}\n  Rust  : {:?}",
        Preview(input),
        Preview(&c.stdout),
        Preview(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch for case `{label}`\n  input : {:?}\n  C     : {:?}\n  Rust  : {:?}",
        Preview(input),
        Preview(&c.stderr),
        Preview(&r.stderr)
    );
    assert_eq!(
        c.status, r.status,
        "exit status mismatch for case `{label}`\n  input : {:?}\n  C     : {:?}\n  Rust  : {:?}",
        Preview(input),
        c.status,
        r.status
    );
}

fn joined(values: &[i64], sep: &str) -> Vec<u8> {
    values
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(sep)
        .into_bytes()
}

fn ints(range: std::ops::RangeInclusive<i64>) -> Vec<i64> {
    range.collect()
}

// ---------------------------------------------------------------------------
// Phase A sanity: both executables exist and run
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_are_runnable() {
    for exe in [c_bin(), rust_bin()] {
        assert!(exe.is_file(), "missing executable: {}", exe.display());
        let out = run(exe, b"");
        assert_eq!(
            out.status,
            Ok(0),
            "{} did not exit 0 on empty input",
            exe.display()
        );
    }
}

// ---------------------------------------------------------------------------
// Phase B: the read loop in `main`
// ---------------------------------------------------------------------------
//
// `for (i = 0; i < 100; i++) if (scanf("%d", &data[i]) != 1) break;`
//
// Branches: immediate EOF (len 0), a successful conversion, a matching
// failure part way through, and running the loop guard out at i == 100.

#[test]
fn empty_input_produces_no_output() {
    assert_same("empty", b"");
}

#[test]
fn whitespace_only_input_is_an_immediate_eof() {
    assert_same("spaces only", b"     ");
    assert_same("newlines only", b"\n\n\n\n");
    assert_same("mixed whitespace only", b" \t\n\x0b\x0c\r");
    assert_same("many newlines", &b"\n".repeat(1000));
}

#[test]
fn single_item() {
    assert_same("single 0", b"0");
    assert_same("single 0 with newline", b"0\n");
    assert_same("single 1", b"1");
    assert_same("single 3", b"3");
    assert_same("single negative", b"-4");
}

#[test]
fn a_few_items() {
    assert_same("three space separated", b"1 2 3");
    assert_same("three with trailing newline", b"1 2 3\n");
    assert_same("three with trailing whitespace", b"1 2 3   \n\n\n");
    assert_same("negatives", b"-1 -2 -3");
    assert_same("mixed signs", b"2 3 -1 -2");
}

/// `%d` skips leading whitespace of every kind, so a conversion crosses line
/// boundaries -- unlike `fgets`, a newline is not a record terminator here.
#[test]
fn scanf_reads_across_newlines_and_all_whitespace() {
    assert_same("one per line", b"1\n2\n3\n");
    assert_same("tab and crlf separated", b"1\t2\r\n3");
    assert_same("vertical tab and form feed", b"1\x0b2\x0c3");
    assert_same("leading whitespace before first item", b"   \n\n  5");
    assert_same("blank lines between items", b"1\n\n\n2\n\n\n3");
    assert_same("every whitespace byte then a digit", b" \t\n\x0b\x0c\r7");
    assert_same(
        "one hundred newline separated",
        &joined(&ints(1..=100), "\n"),
    );
}

#[test]
fn exactly_the_maximum_the_code_handles() {
    assert_same("99 items", &joined(&ints(1..=99), " "));
    assert_same("100 items, the array capacity", &joined(&ints(1..=100), " "));
}

/// The loop stops at `i == 100` without consuming the rest of stdin. Anything
/// after the hundredth item -- more numbers or garbage -- is simply never read.
#[test]
fn input_longer_than_the_array_is_truncated_at_100() {
    assert_same("101 items", &joined(&ints(1..=101), " "));
    assert_same("150 items", &joined(&ints(1..=150), " "));
    assert_same("500 items", &joined(&ints(1..=500), " "));

    let mut with_junk = joined(&ints(1..=100), " ");
    with_junk.extend_from_slice(b" not-a-number");
    assert_same("100 items then junk", &with_junk);
}

// ---------------------------------------------------------------------------
// Phase B: matching failures -- the `!= 1` branch
// ---------------------------------------------------------------------------

#[test]
fn matching_failure_on_the_first_item() {
    assert_same("letters", b"abc");
    assert_same("period", b".");
    assert_same("comma", b",");
    assert_same("underscore", b"_1");
    assert_same("byte 0x0e is not whitespace", b"\x0e1");
    assert_same("high bytes", b"\xff\xfe 2");
}

#[test]
fn matching_failure_part_way_through() {
    assert_same("junk in the middle", b"1 2 abc 3");
    assert_same("junk after two items", b"7 8 ?? 9 10");
    assert_same("stops at the letter suffix", b"5x");
    assert_same("stops at the dash suffix", b"5-");
    assert_same("float stops at the period", b"3.14 2");
    assert_same("exponent stops at the e", b"1e5");
    assert_same("underscore inside", b"1_2");
    assert_same("hex prefix stops at x", b"0x10");
    assert_same("hex prefix mid stream", b"1 0x1f 2");
}

/// A sign with no digit after it is a matching failure, not a zero.
#[test]
fn sign_without_digits_is_a_matching_failure() {
    assert_same("minus at eof", b"-");
    assert_same("plus at eof", b"+");
    assert_same("minus then letter", b"-a");
    assert_same("minus then space then digit", b"- 5");
    assert_same("double minus", b"--1");
    assert_same("plus then minus", b"+-1");
    assert_same("item then bare minus", b"1 - 2");
}

#[test]
fn nul_bytes_terminate_the_read_loop() {
    assert_same("nul only", b"\x00");
    assert_same("nul first", b"\x001");
    assert_same("nul between items", b"1\x002");
    assert_same("nul after digits", b"12\x0034");
    assert_same("sign then nul", b"-\x00");
}

// ---------------------------------------------------------------------------
// Phase B: `%d` conversion details, including overflow of the conversion
// ---------------------------------------------------------------------------

#[test]
fn signs_and_leading_zeros() {
    assert_same("explicit plus", b"+7");
    assert_same("plus zero", b"+0");
    assert_same("negative zero", b"-0");
    assert_same("leading zeros", b"0000000005");
    assert_same("many zeros", &b"0".repeat(500));
    assert_same("padded items", b"007 -008 +009");
}

#[test]
fn int_boundaries() {
    assert_same("INT_MAX", b"2147483647");
    assert_same("INT_MIN", b"-2147483648");
    assert_same("INT_MIN + 1", b"-2147483647");
    assert_same("both extremes", b"2147483647 -2147483648");
    assert_same("INT_MIN twice", b"-2147483648 -2147483648");
}

/// glibc converts `%d` with `strtol` and then narrows to `int`, so values
/// beyond the `int` range wrap, and values beyond the `long` range saturate at
/// `LONG_MAX`/`LONG_MIN` before being narrowed. Both effects are reproduced.
#[test]
fn conversion_overflow_truncates_and_saturates_like_the_c_does() {
    assert_same("INT_MAX + 1", b"2147483648");
    assert_same("INT_MIN - 1", b"-2147483649");
    assert_same("2^32", b"4294967296");
    assert_same("2^32 + 1", b"4294967297");
    assert_same("LONG_MAX", b"9223372036854775807");
    assert_same("LONG_MIN", b"-9223372036854775808");
    assert_same("LONG_MAX + 1", b"9223372036854775808");
    assert_same("LONG_MIN - 1", b"-9223372036854775809");
    assert_same("2^64", b"18446744073709551616");
    assert_same("twenty nines", b"99999999999999999999");
    assert_same("negative twenty nines", b"-99999999999999999999");
    assert_same("one thousand nines", &b"9".repeat(1000));

    let mut neg_thousand = vec![b'-'];
    neg_thousand.extend_from_slice(&b"9".repeat(1000));
    assert_same("negative one thousand nines", &neg_thousand);

    assert_same("LONG_MIN twice", b"-9223372036854775808 -9223372036854775808");
}

// ---------------------------------------------------------------------------
// Phase B: the arithmetic in `fma_array`
// ---------------------------------------------------------------------------
//
// The single call site is `fma_array(out, out, out, out, len)`, so all four
// pointers alias and each element becomes `v * v + v`. Signed overflow wraps
// the way the compiled C does.

#[test]
fn arithmetic_around_the_overflow_boundary() {
    // 46340^2 fits in an int; 46341^2 does not.
    assert_same("just below the square limit", b"46340");
    assert_same("just above the square limit", b"46341");
    assert_same("negative of each", b"-46340 -46341");
    assert_same("powers of two around 2^16", b"65535 65536 65537");
    assert_same("large round numbers", b"100000 1000000 1000000000");
    assert_same("extremes squared", b"2147483647 -2147483648");
    assert_same(
        "assorted magnitudes",
        b"0 1 -1 2 -2 3 -3 10 -10 32768 -32768 123456 -123456",
    );
}

#[test]
fn zero_and_one_are_fixed_points_and_output_formatting_matches() {
    // `printf("%d\n", ...)` -- one value per line, trailing newline, no padding.
    assert_same("zero", b"0");
    assert_same("one", b"1");
    assert_same("zeros and ones", b"0 1 0 1 0");
    assert_same("minus one", b"-1");
}

// ---------------------------------------------------------------------------
// Phase C: breadth over lengths, and randomized differential coverage
// ---------------------------------------------------------------------------

/// Every possible `len` from 0 through 100, plus one over capacity.
#[test]
fn every_length_from_zero_through_the_maximum() {
    for n in 0..=101usize {
        let values: Vec<i64> = (0..n).map(|k| k as i64 - 50).collect();
        assert_same(&format!("length {n}"), &joined(&values, " "));
    }
}

/// The same lengths, but where the loop ends on a matching failure rather than
/// on EOF or on the `i < 100` guard.
#[test]
fn every_length_terminated_by_a_matching_failure() {
    for n in 0..=101usize {
        let mut input = joined(&(1..=n as i64).collect::<Vec<_>>(), " ");
        if n > 0 {
            input.push(b' ');
        }
        input.extend_from_slice(b"STOP 999");
        assert_same(&format!("length {n} then junk"), &input);
    }
}

/// Deterministic pseudo-random inputs mixing valid numbers, whitespace forms
/// and tokens that break the conversion. Seeded so failures are reproducible.
#[test]
fn randomized_differential_sweep() {
    // Small xorshift PRNG: no external dependencies, fully deterministic.
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            self.0 = x;
            x
        }
        fn below(&mut self, n: usize) -> usize {
            (self.next() % n as u64) as usize
        }
    }

    const TOKENS: &[&[u8]] = &[
        b"0",
        b"1",
        b"-1",
        b"7",
        b"46341",
        b"65536",
        b"100000",
        b"2147483647",
        b"-2147483648",
        b"+5",
        b"00012",
        b"-0",
        b"9223372036854775807",
        b"99999999999999999999",
        b"-99999999999999999999",
        b"abc",
        b"-",
        b"+",
        b".",
        b"0x1f",
        b"1.5",
        b"1e9",
        b"\x00",
    ];
    const SPACERS: &[&[u8]] = &[b" ", b"\n", b"\t", b"\r", b"\x0b", b"\x0c", b"  ", b"\n\n"];

    let mut rng = Rng(0x9E3779B97F4A7C15);
    for trial in 0..300 {
        let n = rng.below(140);
        let mut input = Vec::new();
        for _ in 0..n {
            input.extend_from_slice(TOKENS[rng.below(TOKENS.len())]);
            input.extend_from_slice(SPACERS[rng.below(SPACERS.len())]);
        }
        // Half the time drop the final separator so the input ends at EOF
        // immediately after a conversion.
        if rng.below(2) == 0 && !input.is_empty() {
            while input.last().is_some_and(|b| b" \n\t\r\x0b\x0c".contains(b)) {
                input.pop();
            }
        }
        assert_same(&format!("random trial {trial}"), &input);
    }
}

/// Arbitrary byte soup: exercises the conversion with bytes that are neither
/// digits nor whitespace in unpredictable positions.
#[test]
fn randomized_raw_byte_sweep() {
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            self.0 = x;
            x
        }
    }

    let mut rng = Rng(0xDEADBEEFCAFEF00D);
    for trial in 0..200 {
        let len = (rng.next() % 80) as usize;
        let input: Vec<u8> = (0..len).map(|_| (rng.next() % 256) as u8).collect();
        assert_same(&format!("raw bytes trial {trial}"), &input);
    }
}

/// Byte soup drawn only from characters `%d` cares about, so conversions
/// actually succeed often and the boundary logic is hit hard.
#[test]
fn randomized_numeric_alphabet_sweep() {
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            self.0 = x;
            x
        }
    }

    const ALPHABET: &[u8] = b"0123456789 \n\t+-";
    let mut rng = Rng(0x0123456789ABCDEF);
    for trial in 0..200 {
        let len = (rng.next() % 400) as usize;
        let input: Vec<u8> = (0..len)
            .map(|_| ALPHABET[(rng.next() % ALPHABET.len() as u64) as usize])
            .collect();
        assert_same(&format!("numeric alphabet trial {trial}"), &input);
    }
}

// ---------------------------------------------------------------------------
// Phase C: stream conditions rather than stream contents
// ---------------------------------------------------------------------------

/// stdin closed outright, and stdin as a large single-token stream. Both must
/// behave identically, including the exit status.
#[test]
fn unusual_stream_conditions() {
    // A single enormous digit run: the conversion consumes it all and saturates.
    assert_same("one hundred thousand digits", &b"7".repeat(100_000));

    // Maximum items with the largest textual representations.
    let big: Vec<i64> = std::iter::repeat(-2147483648i64).take(100).collect();
    assert_same("100 INT_MIN values", &joined(&big, "\n"));

    // Lots of leading whitespace before any data.
    let mut padded = b" \t\r\n\x0b\x0c".repeat(2000);
    padded.extend_from_slice(b"42");
    assert_same("heavy whitespace padding", &padded);
}
