//! Differential integration tests: run the original C binary and the Rust
//! binary as subprocesses with identical stdin, then compare stdout, stderr and
//! exit status byte-for-byte / value-for-value.
//!
//! The Rust code is never called as a library here; only the built executable is
//! driven, because that is how the two programs are compared.

use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// `translation/`
fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The Rust executable under test.
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// The reference C executable, built with cmake if it is not there yet.
fn c_bin() -> PathBuf {
    let c_src = manifest_dir().parent().unwrap().join("c_src");
    let build = c_src.join("build");
    let exe = build.join("driver");
    if !exe.exists() {
        std::fs::create_dir_all(&build).expect("create c_src/build");
        let conf = Command::new("cmake")
            .arg("..")
            .current_dir(&build)
            .output()
            .expect("run cmake (is cmake installed?)");
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
            .expect("run cmake --build");
        assert!(
            built.status.success(),
            "cmake build failed:\n{}\n{}",
            String::from_utf8_lossy(&built.stdout),
            String::from_utf8_lossy(&built.stderr)
        );
    }
    assert!(exe.exists(), "C binary missing at {}", exe.display());
    exe
}

#[derive(PartialEq, Eq)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    code: Option<i32>,
    signal: Option<i32>,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "stdout={:?} stderr={:?} code={:?} signal={:?}",
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr),
            self.code,
            self.signal
        )
    }
}

fn run(exe: &Path, stdin_bytes: &[u8]) -> Outcome {
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", exe.display()));

    {
        let mut sin = child.stdin.take().unwrap();
        let data = stdin_bytes.to_vec();
        // Write on a helper thread so a program that never drains stdin cannot
        // deadlock the test on a full pipe buffer.
        std::thread::spawn(move || {
            let _ = sin.write_all(&data);
            let _ = sin.flush();
        });
    }

    let out = child.wait_with_output().expect("wait_with_output");
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

/// Asserts the two programs are indistinguishable for `input`.
#[track_caller]
fn assert_same(input: &[u8]) {
    let c = run(&c_bin(), input);
    let r = run(&rust_bin(), input);
    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout differs for input {:?}\n  C: {:?}\n  R: {:?}",
        String::from_utf8_lossy(input),
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr differs for input {:?}",
        String::from_utf8_lossy(input)
    );
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "exit status differs for input {:?} (C {:?} vs Rust {:?})",
        String::from_utf8_lossy(input),
        (c.code, c.signal),
        (r.code, r.signal)
    );
}

#[track_caller]
fn assert_all(inputs: &[&[u8]]) {
    for i in inputs {
        assert_same(i);
    }
}

// ---------------------------------------------------------------------------
// Phase A sanity: both binaries exist and are runnable.
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_run() {
    let c = run(&c_bin(), b"1 2 3 4");
    let r = run(&rust_bin(), b"1 2 3 4");
    assert_eq!(c.stdout, b"1 2 1 4\n".to_vec(), "C reference output changed");
    assert_eq!(r.stdout, c.stdout);
    assert_eq!(c.stderr, Vec::<u8>::new());
    assert_eq!(c.code, Some(0));
    assert_eq!((c.code, c.signal), (r.code, r.signal));
}

// ---------------------------------------------------------------------------
// Phase B: the input classes main() branches on.
// ---------------------------------------------------------------------------

/// No conversion happens at all: every `scanf` takes the input-failure path and
/// x/y/b/z keep their initialisers.
#[test]
fn empty_and_whitespace_only_input() {
    assert_all(&[
        b"",
        b"\n",
        b" ",
        b"\t\t\t",
        b"\n\n\n\n",
        b"   \n\t  ",
        b" \t\n\r\x0b\x0c",
    ]);
}

/// Partial input: the first N conversions succeed and the rest hit EOF.
#[test]
fn partial_input_leaves_later_fields_at_zero() {
    assert_all(&[b"1", b"1 2", b"1 2 3", b"1 2 3 4", b"1 2 3 4 5", b"1 2 3 4 5 6"]);
}

/// `scanf("%u")` / `scanf("%d")` skip whitespace including newlines, so a
/// conversion happily crosses line boundaries (unlike `fgets`).
#[test]
fn conversions_read_across_newlines() {
    assert_all(&[
        b"1\n2\n3\n4\n",
        b"1\n2\n3\n4",
        b"1\r\n2\r\n3\r\n4\r\n",
        b"  1  2  3  4  ",
        b"\n\n1\n\n2\n\n3\n\n4\n\n",
        b"1\t2\x0b3\x0c4",
        b"  \t\n 1 \r\n 2 \x0b 3 \x0c 4 ",
        b"1 2 3 4\n\n\n",
    ]);
}

/// `unsigned int x : 2` keeps only the low 2 bits of the assigned value.
#[test]
fn x_field_truncates_to_two_bits() {
    for x in [
        0u64, 1, 2, 3, 4, 5, 6, 7, 8, 9, 15, 16, 17, 255, 256, 1023, 65535, 2147483647,
        2147483648, 4294967294, 4294967295,
    ] {
        assert_same(format!("{x} 0 0 0").as_bytes());
    }
}

/// `unsigned int y : 3` keeps only the low 3 bits.
#[test]
fn y_field_truncates_to_three_bits() {
    for y in [
        0u64, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 15, 16, 17, 23, 24, 255, 256, 4294967295,
    ] {
        assert_same(format!("0 {y} 0 0").as_bytes());
    }
}

/// `driver(..., !!b, ...)` normalises any non-zero to 1 before the 1-bit
/// `bool` field stores it, so even values whose low bit is 0 print as 1.
#[test]
fn bool_field_normalises_with_double_negation() {
    for b in [
        0i64, 1, 2, 3, -1, -2, 256, 512, 1024, 2147483647, -2147483648, 4096,
    ] {
        assert_same(format!("0 0 {b} 0").as_bytes());
    }
}

/// `int z` is printed with `%d`, full width, sign preserved, no truncation.
#[test]
fn z_field_is_a_full_signed_int() {
    for z in [
        0i64,
        1,
        -1,
        7,
        -7,
        42,
        2147483647,
        -2147483648,
        1000000,
        -1000000,
    ] {
        assert_same(format!("0 0 0 {z}").as_bytes());
    }
}

/// A cross product of the field boundaries, exercising the printf format
/// `"%u %u %d %d\n"` (spacing and trailing newline included).
#[test]
fn field_boundary_cross_product() {
    for x in [0u64, 1, 3, 4, 7, 8] {
        for y in [0u64, 1, 7, 8, 15, 16] {
            for b in [0i64, 1, 2, -1] {
                for z in [0i64, -1, 2147483647, -2147483648] {
                    assert_same(format!("{x} {y} {b} {z}").as_bytes());
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Phase B/C: matching-failure and conversion-failure paths.
// ---------------------------------------------------------------------------

/// A non-numeric character is a matching failure: the variable is untouched and
/// the offending character stays in the stream, so every later conversion fails
/// on it too.
#[test]
fn matching_failure_on_non_numeric_input() {
    assert_all(&[
        b"abc",
        b"x",
        b"x 1 2 3",
        b"1 x 2 3",
        b"1 2 x 4",
        b"1 2 3 x",
        b"1,2,3,4",
        b",",
        b"/",
        b":",
        b"1 2 3 4extra",
        b"..",
        b"e",
        b"1e5 2 3 4",
    ]);
}

/// A sign with no digits behind it is also a matching failure.
#[test]
fn sign_without_digits() {
    assert_all(&[
        b"-", b"+", b"--", b"++", b"-+", b"+-", b"- 1 2 3", b"+ 1 2 3", b"-a 1 2 3", b"+a 1 2 3",
        b" -   1 2 3 4", b"-\n1 2 3 4", b"1 - 2 3", b"1 2 - 3", b"1 2 3 -",
    ]);
}

/// `%u` accepts a leading sign; the value is negated modulo 2^64 by strtoul and
/// then truncated to `unsigned int`.
#[test]
fn unsigned_conversion_accepts_a_minus_sign() {
    assert_all(&[
        b"-1 -1 -1 -1",
        b"-2 -3 -4 -5",
        b"-0 -0 -0 -0",
        b"-4 -8 0 0",
        b"-4294967295 1 2 3",
        b"-4294967296 1 2 3",
        b"-18446744073709551615 1 2 3",
        b"-18446744073709551616 1 2 3",
        b"+1 +2 +3 +4",
        b"+0 +0 +0 +0",
        b"1 2 -0 -0",
        b"1 2 +0 +0",
    ]);
}

/// Base 10 only: `%u` stops at the `x` of `0x10`, and a leading `0` is not
/// octal.
#[test]
fn conversions_are_decimal_only() {
    assert_all(&[
        b"0x10 1 2 3",
        b"1 0x10 2 3",
        b"010 011 012 013",
        b"00000001 2 3 4",
        b"0000000000000000000000000000000000005 1 2 3",
        b"0 0 0 0",
    ]);
}

/// A `.` ends the conversion, so `1.5` yields 1 and then a matching failure.
#[test]
fn decimal_point_stops_the_conversion() {
    assert_all(&[b"1.5 2.5 3.5 4.5", b"1. 2 3 4", b".5 1 2 3", b"1 2 3 4.5"]);
}

/// Values past the 64-bit range saturate inside strtoul/strtol before being
/// truncated into the 32-bit destination.
#[test]
fn out_of_range_values_saturate_then_truncate() {
    assert_all(&[
        b"4294967295 4294967295 4294967295 4294967295",
        b"4294967296 4294967296 4294967296 4294967296",
        b"4294967297 1 2 3",
        b"8589934592 1 2 3",
        b"2147483648 1 2 3",
        b"18446744073709551615 1 2 3",
        b"18446744073709551616 1 2 3",
        b"18446744073709551617 1 2 3",
        b"99999999999999999999999999 1 2 3",
        b"12345678901234567890 1 2 3",
        b"1 2 2147483647 2147483647",
        b"1 2 2147483648 2147483648",
        b"1 2 -2147483648 -2147483648",
        b"1 2 -2147483649 -2147483649",
        b"1 2 9223372036854775807 9223372036854775807",
        b"1 2 9223372036854775808 9223372036854775808",
        b"1 2 9223372036854775809 9223372036854775809",
        b"1 2 -9223372036854775808 -9223372036854775808",
        b"1 2 -9223372036854775809 -9223372036854775809",
        b"1 2 12345678901234567890 12345678901234567890",
        b"1 2 -12345678901234567890 -12345678901234567890",
        b"1 2 -99999999999999999999999 -99999999999999999999999",
    ]);
}

/// Absurdly long digit runs: the accumulator must not misbehave and the result
/// must still be the saturated-then-truncated value.
#[test]
fn very_long_digit_runs() {
    let long9 = "9".repeat(400);
    let long1 = "1".repeat(300);
    let zeros_then_one = format!("{}1", "0".repeat(500));
    let cases: Vec<Vec<u8>> = vec![
        format!("{long9} 1 2 3").into_bytes(),
        format!("-{long9} 1 2 3").into_bytes(),
        format!("1 2 {long9} {long9}").into_bytes(),
        format!("1 2 -{long9} -{long9}").into_bytes(),
        format!("{long1} {long1} {long1} {long1}").into_bytes(),
        format!("{zeros_then_one} 2 3 4").into_bytes(),
        format!("{} {} {} {}", "9".repeat(19), "9".repeat(19), "9".repeat(19), "9".repeat(19))
            .into_bytes(),
        format!("{} 1 2 3", "9".repeat(20)).into_bytes(),
    ];
    for c in &cases {
        assert_same(c);
    }
}

/// Bytes that are not valid UTF-8 and embedded NULs must be handled the same
/// way; they are ordinary non-numeric characters to `scanf`.
#[test]
fn non_utf8_and_nul_bytes() {
    assert_all(&[
        b"\x00",
        b"\x001 2 3 4",
        b"1 2 3 4\x00",
        b"1\x002 3 4",
        b"\xff\xfe 1 2 3",
        b"1 2 3 4\xff",
        b"\xc3\x28 1 2 3",
        b"1 \xff 2 3",
    ]);
}

// ---------------------------------------------------------------------------
// Phase C: environmental paths.
// ---------------------------------------------------------------------------

/// A closed stdin is an input failure for every conversion, not a crash.
#[test]
fn closed_stdin_behaves_like_eof() {
    let c = Command::new(c_bin())
        .stdin(Stdio::null())
        .output()
        .expect("run C");
    let r = Command::new(rust_bin())
        .stdin(Stdio::null())
        .output()
        .expect("run Rust");
    assert_eq!(c.stdout, r.stdout);
    assert_eq!(c.stderr, r.stderr);
    assert_eq!(c.status.code(), r.status.code());
    assert_eq!(c.status.signal(), r.status.signal());
    assert_eq!(c.stdout, b"0 0 0 0\n".to_vec());
}

/// Command-line arguments are ignored by `main()`.
#[test]
fn arguments_are_ignored() {
    for args in [vec![], vec!["foo"], vec!["-h", "--version"]] {
        let c = Command::new(c_bin())
            .args(&args)
            .stdin(Stdio::null())
            .output()
            .unwrap();
        let r = Command::new(rust_bin())
            .args(&args)
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert_eq!(c.stdout, r.stdout, "stdout differs for args {args:?}");
        assert_eq!(c.stderr, r.stderr, "stderr differs for args {args:?}");
        assert_eq!(c.status.code(), r.status.code());
        assert_eq!(c.status.signal(), r.status.signal());
    }
}

/// Writing to a closed stdout must end the same way in both programs. A C
/// program launched from a shell has the default `SIGPIPE` disposition, so it
/// dies from the signal; the Rust runtime ignores `SIGPIPE` unless the program
/// restores the default.
#[test]
fn broken_pipe_exit_status_matches() {
    fn run_into_closed_pipe(exe: &Path) -> (Option<i32>, Option<i32>) {
        let mut child = Command::new(exe)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        {
            let mut sin = child.stdin.take().unwrap();
            let _ = sin.write_all(b"1 2 3 4\n");
        }
        // Drop the read end so the child's next write gets EPIPE / SIGPIPE.
        drop(child.stdout.take());
        let st = child.wait().unwrap();
        (st.code(), st.signal())
    }
    // Give the child a moment to attempt its write after the pipe is closed.
    let c = run_into_closed_pipe(&c_bin());
    let r = run_into_closed_pipe(&rust_bin());
    assert_eq!(c, r, "broken-pipe exit status differs (C {c:?} vs Rust {r:?})");
}

/// Large input: only the first four conversions matter, the rest is left
/// unread, and neither program may choke on the volume.
#[test]
fn very_large_input() {
    let big = "7 ".repeat(200_000);
    assert_same(big.as_bytes());
    let big_newlines = "13\n".repeat(100_000);
    assert_same(big_newlines.as_bytes());
    let junk = "z".repeat(100_000);
    assert_same(junk.as_bytes());
}

/// `scanf` converts lazily: once the fourth number has been delimited the C
/// program prints and exits without waiting for end-of-input. A translation that
/// slurps stdin first would hang here while stdin stays open.
#[test]
fn stdin_held_open_does_not_block() {
    /// Feeds `chunks` with a pause between each, then leaves stdin open, and
    /// reports what the program did within `deadline`.
    fn drive_streaming(exe: &Path, chunks: &[&[u8]], deadline: std::time::Duration) -> Outcome {
        let mut child = Command::new(exe)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut sin = child.stdin.take().unwrap();
        let owned: Vec<Vec<u8>> = chunks.iter().map(|c| c.to_vec()).collect();
        // Keep the write end alive (no EOF) for longer than the deadline.
        let writer = std::thread::spawn(move || {
            for c in owned {
                if sin.write_all(&c).is_err() || sin.flush().is_err() {
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            std::thread::sleep(std::time::Duration::from_secs(10));
            drop(sin);
        });

        let start = std::time::Instant::now();
        loop {
            match child.try_wait().unwrap() {
                Some(status) => {
                    let mut stdout = Vec::new();
                    let mut stderr = Vec::new();
                    use std::io::Read as _;
                    let _ = child.stdout.take().unwrap().read_to_end(&mut stdout);
                    let _ = child.stderr.take().unwrap().read_to_end(&mut stderr);
                    drop(writer);
                    return Outcome {
                        stdout,
                        stderr,
                        code: status.code(),
                        signal: status.signal(),
                    };
                }
                None if start.elapsed() > deadline => {
                    let _ = child.kill();
                    let _ = child.wait();
                    drop(writer);
                    panic!(
                        "{} did not finish within {deadline:?} while stdin stayed open",
                        exe.display()
                    );
                }
                None => std::thread::sleep(std::time::Duration::from_millis(10)),
            }
        }
    }

    let deadline = std::time::Duration::from_secs(5);
    for chunks in [
        vec![&b"1 2 3 4\n"[..]],
        vec![&b"1 "[..], &b"2 "[..], &b"3 "[..], &b"4\n"[..]],
        vec![&b"9"[..], &b"9 8 7 6 "[..]],
        vec![&b"x "[..]],
    ] {
        let c = drive_streaming(&c_bin(), &chunks, deadline);
        let r = drive_streaming(&rust_bin(), &chunks, deadline);
        assert_eq!(c.stdout, r.stdout, "streaming stdout differs for {chunks:?}");
        assert_eq!(c.stderr, r.stderr, "streaming stderr differs for {chunks:?}");
        assert_eq!(
            (c.code, c.signal),
            (r.code, r.signal),
            "streaming exit status differs for {chunks:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Phase C: deterministic randomised differential sweep (no external deps).
// ---------------------------------------------------------------------------

struct Lcg(u64);

impl Lcg {
    fn next_u32(&mut self) -> u32 {
        // Numerical Recipes constants; deterministic across runs.
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.0 >> 33) as u32
    }
    fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
}

#[test]
fn randomised_byte_soup_matches() {
    const ALPHABET: &[u8] = b"0123456789 \n\t\r+-.,exX\x00\xffab/";
    let mut rng = Lcg(0x5DEECE66D);
    for _ in 0..600 {
        let len = rng.below(18) as usize;
        let input: Vec<u8> = (0..len)
            .map(|_| ALPHABET[rng.below(ALPHABET.len() as u32) as usize])
            .collect();
        assert_same(&input);
    }
}

#[test]
fn randomised_numeric_tokens_match() {
    const MAGNITUDES: [&str; 7] = [
        "9",
        "255",
        "65535",
        "2147483647",
        "4294967295",
        "18446744073709551615",
        "184467440737095516150000",
    ];
    const SIGNS: [&str; 3] = ["", "-", "+"];
    const SEPS: [&str; 5] = [" ", "\n", "\t", "\r\n", "  "];
    let mut rng = Lcg(12345);
    for _ in 0..600 {
        let n = rng.below(6) as usize;
        let mut toks = Vec::new();
        for _ in 0..n {
            let mag = MAGNITUDES[rng.below(MAGNITUDES.len() as u32) as usize];
            // Random value with the same digit count as `mag`, plus the bounds.
            let digits: String = mag
                .chars()
                .map(|_| char::from(b'0' + rng.below(10) as u8))
                .collect();
            let body = if rng.below(2) == 0 { mag.to_string() } else { digits };
            let sign = SIGNS[rng.below(3) as usize];
            let pad = "0".repeat(rng.below(3) as usize);
            toks.push(format!("{sign}{pad}{body}"));
        }
        let sep = SEPS[rng.below(SEPS.len() as u32) as usize];
        let input = toks.join(sep);
        assert_same(input.as_bytes());
    }
}
