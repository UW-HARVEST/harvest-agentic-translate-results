//! Differential tests: run the original C program and the Rust translation as
//! subprocesses, feed both the same bytes on stdin, and require that stdout,
//! stderr and the exit status match byte for byte.
//!
//! The Rust code is never called as a library — only the built binary is
//! driven, exactly the way a shell would drive it.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// Repository root (the directory holding `c_src/` and `translation/`).
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the Rust binary under test. Cargo builds this before running the
/// integration test and hands us the path, so it is never stale.
fn rust_bin() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// Build `c_src` with CMake once per test binary, and return the C executable.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = repo_root().join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");

        std::fs::create_dir_all(&build).expect("could not create c_src/build");

        let configure = Command::new("cmake")
            .arg("..")
            .current_dir(&build)
            .output()
            .expect("failed to run `cmake ..` — is cmake installed?");
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

        assert!(exe.is_file(), "C driver not found at {}", exe.display());
        exe
    })
}

/// What one program produced for one input.
#[derive(PartialEq, Eq)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Ok(code)` for a normal exit, `Err(signal)` if killed by a signal.
    status: Result<i32, i32>,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "status={:?}\n  stdout={:?}\n  stderr={:?}",
            self.status,
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr)
        )
    }
}

fn spawn(exe: &Path, input: &[u8]) -> Outcome {
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));

    {
        let mut stdin = child.stdin.take().expect("piped stdin");
        // The child may exit without draining stdin; a broken pipe here is not
        // a test failure.
        let _ = stdin.write_all(input);
        let _ = stdin.flush();
    }

    let out = child.wait_with_output().expect("failed to collect output");

    #[cfg(unix)]
    let status = {
        use std::os::unix::process::ExitStatusExt;
        match out.status.code() {
            Some(c) => Ok(c),
            None => Err(out.status.signal().unwrap_or(-1)),
        }
    };
    #[cfg(not(unix))]
    let status = Ok(out.status.code().unwrap_or(-1));

    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        status,
    }
}

/// Assert the C program and the Rust program agree on stdout, stderr and exit
/// status for `input`.
#[track_caller]
fn assert_same(label: &str, input: &[u8]) {
    let c = spawn(c_bin(), input);
    let r = spawn(rust_bin(), input);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch for {label} (input {:?})\n  C:   {:?}\n  Rust:{:?}",
        Trunc(input),
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch for {label} (input {:?})\n  C:   {:?}\n  Rust:{:?}",
        Trunc(input),
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
    assert_eq!(
        c.status, r.status,
        "exit status mismatch for {label} (input {:?}): C={:?} Rust={:?}",
        Trunc(input),
        c.status,
        r.status
    );
}

/// Keeps failure messages readable when an input is a 200-byte blob.
struct Trunc<'a>(&'a [u8]);
impl std::fmt::Debug for Trunc<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.len() <= 64 {
            write!(f, "{:?}", String::from_utf8_lossy(self.0))
        } else {
            write!(
                f,
                "{:?}… ({} bytes)",
                String::from_utf8_lossy(&self.0[..64]),
                self.0.len()
            )
        }
    }
}

fn check_all(cases: &[(&str, &[u8])]) {
    for (label, input) in cases {
        assert_same(label, input);
    }
}

// ---------------------------------------------------------------------------
// fgets: the read itself
//
// `fgets(in, 100, stdin)` reads at most 99 bytes, stops after a newline (which
// it keeps), and on immediate EOF returns NULL leaving `in` as the "" it was
// initialised to. It does NOT read across newlines.
// ---------------------------------------------------------------------------

#[test]
fn empty_and_eof_input() {
    check_all(&[
        // fgets returns NULL, buffer stays "", strtol converts nothing.
        ("empty stdin", b""),
        ("single NUL", b"\0"),
    ]);
}

#[test]
fn fgets_stops_at_first_newline() {
    check_all(&[
        // Only the first line is ever read; the rest of stdin is ignored.
        ("two numeric lines", b"3\n7\n"),
        ("number then junk line", b"4\nabc\n"),
        ("junk then number line", b"abc\n4\n"),
        ("blank first line", b"\n5\n"),
        ("many blank lines", b"\n\n\n"),
    ]);
}

#[test]
fn fgets_99_byte_truncation() {
    // Only 99 bytes reach the buffer, so a number straddling that boundary is
    // silently cut in half.
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();

    cases.push(("98 spaces then 12345".into(), [b" ".repeat(98), b"12345\n".to_vec()].concat()));
    cases.push(("97 spaces then 12345".into(), [b" ".repeat(97), b"12345\n".to_vec()].concat()));
    cases.push(("95 spaces then 1234567890".into(), [b" ".repeat(95), b"1234567890\n".to_vec()].concat()));
    cases.push(("98 spaces then -1".into(), [b" ".repeat(98), b"-1\n".to_vec()].concat()));
    cases.push(("97 spaces then -1".into(), [b" ".repeat(97), b"-1\n".to_vec()].concat()));
    cases.push(("99 spaces then 1".into(), [b" ".repeat(99), b"1\n".to_vec()].concat()));
    cases.push(("100 spaces then 1".into(), [b" ".repeat(100), b"1\n".to_vec()].concat()));
    cases.push(("1 then 98 spaces then 9".into(), [b"1".to_vec(), b" ".repeat(98), b"9\n".to_vec()].concat()));

    // Newline exactly at, side of, and past the buffer edge.
    cases.push(("newline at byte 99".into(), [b"1".repeat(98), b"\n".to_vec()].concat()));
    cases.push(("newline at byte 100".into(), [b"1".repeat(99), b"\n".to_vec()].concat()));
    cases.push(("newline at byte 101".into(), [b"1".repeat(100), b"\n".to_vec()].concat()));

    // 99 digits truncated from 100 is still an overflow either way.
    cases.push(("99 digits".into(), [b"5".repeat(99), b"\n".to_vec()].concat()));
    cases.push(("100 digits, no newline".into(), b"5".repeat(100)));

    // No newline at all, longer than the buffer.
    cases.push(("7 then 200 x, no newline".into(), [b"7".to_vec(), b"x".repeat(200)].concat()));
    cases.push(("4096 bytes of digits".into(), b"1".repeat(4096)));

    for (label, input) in &cases {
        assert_same(label, input);
    }
}

#[test]
fn embedded_nul_terminates_the_c_string() {
    check_all(&[
        // The buffer is handed to strtol as a C string, so it ends at the NUL
        // even though fgets read past it.
        ("NUL before the digits", b"\x0012\n"),
        ("NUL after the digits", b"12\x0034\n"),
        ("space, NUL, then a number", b" \x00 5\n"),
        ("NUL then newline", b"\x00\n"),
        ("minus then NUL", b"-\x005\n"),
    ]);
}

// ---------------------------------------------------------------------------
// parse_val: the success path
//
// `strtol(str, &endp, 10)` succeeds as long as it converted at least one digit.
// Trailing garbage is accepted and ignored — that is what `endp != str` means.
// ---------------------------------------------------------------------------

#[test]
fn accepted_numbers() {
    check_all(&[
        ("zero, no newline", b"0"),
        ("zero", b"0\n"),
        ("one", b"1\n"),
        ("five", b"5\n"),
        ("negative", b"-3\n"),
        ("negative one", b"-1\n"),
        ("negative zero", b"-0\n"),
        ("explicit plus", b"+7\n"),
        ("plus zero", b"+0\n"),
        ("leading zeros stay base 10", b"007\n"),
        ("large positive", b"1000000\n"),
        ("large negative", b"-1000000\n"),
    ]);
}

#[test]
fn leading_whitespace_is_skipped() {
    check_all(&[
        ("two leading spaces", b"  12\n"),
        ("leading tab", b"\t9\n"),
        ("vertical tab and form feed", b"\x0b\x0c5\n"),
        ("leading CR", b"\r5\n"),
        ("leading newline then digits on line 2 is NOT read", b"\n5\n"),
        ("spaces around", b" 42 \n"),
    ]);
}

#[test]
fn trailing_garbage_is_accepted() {
    check_all(&[
        // strtol stops at the first non-digit; parse_val does not care.
        ("digits then letters", b"12abc\n"),
        ("decimal point truncates", b"2.9\n"),
        ("negative decimal truncates", b"-2.9\n"),
        ("0x prefix parses as 0", b"0x10\n"),
        ("exponent notation truncates", b"1e3\n"),
        ("underscore separator truncates", b"1_000\n"),
        ("comma separator truncates", b"1,000\n"),
        ("digits then space then digits", b"12 34\n"),
        ("digits then minus", b"12-34\n"),
    ]);
}

// ---------------------------------------------------------------------------
// parse_val: the error path -> "An error occurred"
// ---------------------------------------------------------------------------

#[test]
fn no_conversion_is_an_error() {
    check_all(&[
        // endp == str, so parse_val returns false.
        ("newline only", b"\n"),
        ("spaces only", b"     "),
        ("spaces then newline", b"   \n"),
        ("tab CR newline", b"\t\r\n"),
        ("CR only", b"\r"),
        ("letters", b"abc\n"),
        ("dot only", b".\n"),
        ("minus only", b"-\n"),
        ("plus only", b"+\n"),
        ("minus space digit", b"- 5\n"),
        ("double plus", b"++5\n"),
        ("double minus", b"--5\n"),
        ("space plus letter", b"  +x\n"),
        ("high bytes are not whitespace in the C locale", b"\xff\xfe\n"),
        ("punctuation", b"!@#$\n"),
    ]);
}

#[test]
fn int_range_boundaries() {
    check_all(&[
        // Inside int: accepted.
        ("INT_MAX", b"2147483647\n"),
        ("INT_MAX - 1", b"2147483646\n"),
        ("INT_MIN", b"-2147483648\n"),
        ("INT_MIN + 1", b"-2147483647\n"),
        // Converts fine as a long, but fails the `tmp <= INT_MAX` guard.
        ("INT_MAX + 1", b"2147483648\n"),
        ("INT_MIN - 1", b"-2147483649\n"),
        ("well past INT_MAX", b"4000000000\n"),
        ("well past INT_MIN", b"-4000000000\n"),
    ]);
}

#[test]
fn long_range_boundaries_and_erange() {
    check_all(&[
        // Converts without ERANGE, still rejected for being outside int.
        ("LONG_MAX", b"9223372036854775807\n"),
        ("LONG_MIN", b"-9223372036854775808\n"),
        // strtol saturates and sets ERANGE, so `errno == 0` fails.
        ("LONG_MAX + 1", b"9223372036854775808\n"),
        ("LONG_MIN - 1", b"-9223372036854775809\n"),
        ("50 digits", b"11111111111111111111111111111111111111111111111111\n"),
        ("50 digits negative", b"-1111111111111111111111111111111111111111111111111\n"),
    ]);
}

#[test]
fn leading_zeros_do_not_trigger_overflow() {
    // Padding keeps the magnitude small, so these follow the value, not the
    // digit count — but the 99-byte fgets cut still applies.
    let pad80 = b"0".repeat(80);
    let cases: Vec<(String, Vec<u8>)> = vec![
        ("80 zeros then INT_MAX".into(), [pad80.clone(), b"2147483647\n".to_vec()].concat()),
        ("80 zeros then 5".into(), [pad80.clone(), b"5\n".to_vec()].concat()),
        ("80 zeros then a huge number".into(), [pad80, b"9999999999999999999999\n".to_vec()].concat()),
        ("18 zeros then 7".into(), [b"0".repeat(18), b"7\n".to_vec()].concat()),
    ];
    for (label, input) in &cases {
        assert_same(label, input);
    }
}

// ---------------------------------------------------------------------------
// run(): global state carried across the two calls
// ---------------------------------------------------------------------------

#[test]
fn global_state_persists_across_both_runs() {
    // `the_house` is a mutable global, so the second run() starts from where
    // the first left off: floors 2->4, bathrooms 2.5->4.5 over the 8 lines.
    check_all(&[
        ("bedrooms unchanged", b"0\n"),
        ("bedrooms grow twice", b"10\n"),
        ("bedrooms shrink twice", b"-10\n"),
        // 5 + INT_MAX wraps negative, then wraps back on the second run.
        ("int overflow on the first add", b"2147483647\n"),
        ("int underflow on the first add", b"-2147483648\n"),
        ("half of int range", b"1073741824\n"),
        ("just past the wrap point", b"2147483643\n"),
    ]);
}

#[test]
fn output_shape_is_exactly_what_the_c_prints() {
    // Pin the literal bytes so a formatting drift (`%.1f`, spacing, the
    // trailing newline) cannot pass by matching a Rust-side mistake twice.
    let out = spawn(c_bin(), b"5\n");
    let expected = "The house has 2 floors, 5 bedrooms, and 2.5 bathrooms\n\
                    The house has 3 floors, 5 bedrooms, and 2.5 bathrooms\n\
                    The house has 3 floors, 5 bedrooms, and 3.5 bathrooms\n\
                    The house has 3 floors, 10 bedrooms, and 3.5 bathrooms\n\
                    The house has 3 floors, 10 bedrooms, and 3.5 bathrooms\n\
                    The house has 4 floors, 10 bedrooms, and 3.5 bathrooms\n\
                    The house has 4 floors, 10 bedrooms, and 4.5 bathrooms\n\
                    The house has 4 floors, 15 bedrooms, and 4.5 bathrooms\n";
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        expected,
        "the C reference output changed shape; update this test deliberately"
    );
    assert_same("pinned output shape", b"5\n");

    let err = spawn(c_bin(), b"abc\n");
    assert_eq!(String::from_utf8_lossy(&err.stdout), "An error occurred\n");
    assert_eq!(err.status, Ok(0), "the C program always returns 0");
    assert_same("pinned error shape", b"abc\n");
}

#[test]
fn errors_go_to_stdout_not_stderr_and_exit_is_always_zero() {
    for input in [&b""[..], b"abc\n", b"9223372036854775808\n", b"5\n"] {
        let c = spawn(c_bin(), input);
        let r = spawn(rust_bin(), input);
        assert!(
            c.stderr.is_empty() && r.stderr.is_empty(),
            "neither program should write to stderr"
        );
        assert_eq!(c.status, Ok(0));
        assert_eq!(r.status, Ok(0));
    }
}

// ---------------------------------------------------------------------------
// Broad sweeps
// ---------------------------------------------------------------------------

/// Small deterministic LCG so the sweeps are reproducible without a dev-dependency.
struct Rng(u64);
impl Rng {
    fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.0 >> 33) as u32
    }
    fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
}

#[test]
fn fuzz_short_inputs_over_the_interesting_alphabet() {
    let alphabet = b"0123456789 \t\n+-abxX.\0\r";
    let mut rng = Rng(0x5EED_1234);
    for _ in 0..1500 {
        let len = rng.below(13) as usize;
        let input: Vec<u8> = (0..len)
            .map(|_| alphabet[rng.below(alphabet.len() as u32) as usize])
            .collect();
        assert_same("fuzz short", &input);
    }
}

#[test]
fn fuzz_numbers_around_every_boundary() {
    let mut rng = Rng(0xC0FF_EE01);
    let prefixes: [&[u8]; 6] = [b"", b" ", b"  ", b"\t", b"\n", b"+"];
    let suffixes: [&[u8]; 5] = [b"", b"\n", b"x\n", b" \n", b"\0"];

    for _ in 0..1200 {
        let magnitude = rng.below(3);
        let raw = rng.next_u32() as u64 | ((rng.next_u32() as u64) << 32);
        let value: i128 = match magnitude {
            // Straddle INT_MIN/INT_MAX.
            0 => (raw % (2u64.pow(32) + 11)) as i128 - 2i128.pow(31) - 5,
            // Straddle LONG_MIN/LONG_MAX.
            1 => (raw as i128) - 2i128.pow(63) + (rng.below(11) as i128) - 5,
            // Small values.
            _ => (raw % 201) as i128 - 100,
        };
        let mut input = prefixes[rng.below(prefixes.len() as u32) as usize].to_vec();
        input.extend_from_slice(value.to_string().as_bytes());
        input.extend_from_slice(suffixes[rng.below(suffixes.len() as u32) as usize]);
        assert_same("fuzz number", &input);
    }
}

#[test]
fn fuzz_lines_straddling_the_fgets_buffer() {
    let alphabet = b"0123456789 -+";
    let mut rng = Rng(0xABCD_0007);
    for _ in 0..400 {
        let len = 90 + rng.below(21) as usize;
        let mut input: Vec<u8> = (0..len)
            .map(|_| alphabet[rng.below(alphabet.len() as u32) as usize])
            .collect();
        if rng.below(2) == 0 {
            input.push(b'\n');
        }
        assert_same("fuzz buffer edge", &input);
    }
}

#[test]
fn every_int_boundary_value_exhaustively() {
    // Walk both sides of INT_MIN/INT_MAX one at a time.
    for delta in -3i64..=3 {
        for base in [i32::MAX as i64, i32::MIN as i64, 0] {
            let v = base + delta;
            assert_same("int boundary", format!("{v}\n").as_bytes());
            assert_same("int boundary, no newline", format!("{v}").as_bytes());
        }
    }
    for delta in -3i128..=3 {
        for base in [i64::MAX as i128, i64::MIN as i128] {
            let v = base + delta;
            assert_same("long boundary", format!("{v}\n").as_bytes());
        }
    }
}
