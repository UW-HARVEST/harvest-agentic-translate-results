//! Differential tests: run the original C program and the Rust translation as
//! *subprocesses*, feed both the same bytes on stdin, and require that stdout,
//! stderr and the exit status match byte for byte.
//!
//! Nothing here calls the Rust code as a library — the crate under test is an
//! executable and is graded by running it, so that is exactly what we do.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// `translation/`
fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The directory holding `c_src/` and `translation/`.
fn workspace_root() -> PathBuf {
    crate_dir().parent().expect("crate has a parent dir").to_path_buf()
}

/// Path to the Rust binary built by cargo for this integration test.
///
/// `CARGO_BIN_EXE_<name>` is set by cargo for every `[[bin]]` target, so the
/// binary is guaranteed to be freshly built when the test runs.
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Configure + build `c_src` with CMake (once) and return the C binary path.
fn c_bin() -> PathBuf {
    use std::sync::OnceLock;
    static CACHED: OnceLock<PathBuf> = OnceLock::new();

    CACHED
        .get_or_init(|| {
            let c_src = workspace_root().join("c_src");
            let build = c_src.join("build");

            let candidates = [build.join("driver"), build.join("driver.exe")];
            if let Some(p) = candidates.iter().find(|p| p.is_file()) {
                return p.clone();
            }

            std::fs::create_dir_all(&build).expect("create c_src/build");

            let configure = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("failed to spawn `cmake` — is CMake installed?");
            assert!(
                configure.status.success(),
                "cmake configure failed:\n{}\n{}",
                String::from_utf8_lossy(&configure.stdout),
                String::from_utf8_lossy(&configure.stderr),
            );

            let compile = Command::new("cmake")
                .args(["--build", "."])
                .current_dir(&build)
                .output()
                .expect("failed to spawn `cmake --build`");
            assert!(
                compile.status.success(),
                "cmake build failed:\n{}\n{}",
                String::from_utf8_lossy(&compile.stdout),
                String::from_utf8_lossy(&compile.stderr),
            );

            candidates
                .iter()
                .find(|p| p.is_file())
                .cloned()
                .unwrap_or_else(|| panic!("C binary not found in {}", build.display()))
        })
        .clone()
}

#[derive(PartialEq, Eq)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Some(code)` for a normal exit, `None` if killed by a signal.
    code: Option<i32>,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "exit={:?} stdout={:?} stderr={:?}",
            self.code,
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr),
        )
    }
}

/// Run `bin` with `input` on stdin, the way a shell would.
fn run(bin: &Path, input: &[u8]) -> Outcome {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    {
        let mut stdin = child.stdin.take().expect("piped stdin");
        // The child may legitimately exit before consuming everything; a
        // broken pipe here is not a test failure.
        let _ = stdin.write_all(input);
        let _ = stdin.flush();
    }

    let out = child.wait_with_output().expect("wait for child");
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
    }
}

/// The core assertion: for `input`, C and Rust agree on all three observables.
#[track_caller]
fn assert_same(label: &str, input: &[u8]) {
    let c = run(&c_bin(), input);
    let r = run(&rust_bin(), input);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch for {label} (input {input:?})\n  C: {c:?}\n  Rust: {r:?}"
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch for {label} (input {input:?})\n  C: {c:?}\n  Rust: {r:?}"
    );
    assert_eq!(
        c.code,
        r.code,
        "exit status mismatch for {label} (input {input:?})\n  C: {c:?}\n  Rust: {r:?}"
    );
}

// ---------------------------------------------------------------------------
// The C program is:
//
//     int main() { int x = 0; scanf("%d", &x); driver(x); return 0; }
//
// `driver` fills a `{int floors; int bedrooms; double bathrooms;}` (which is
// zeroed first), then hex-dumps its 16-byte object representation.
//
// The only input-dependent behaviour is therefore the `scanf("%d")` call, so the
// input classes are the branches *inside* `%d` conversion:
//
//   1. input failure before any conversion (EOF / empty)  -> x stays 0
//   2. matching failure (no digits after optional sign)   -> x stays 0
//   3. successful conversion                              -> x = value
//   4. leading-whitespace skipping (incl. across newlines)
//   5. optional '+' / '-' sign
//   6. conversion stops at the first non-digit ("pushback")
//   7. out-of-range: glibc saturates to LONG_MIN/LONG_MAX, then truncates to int
//
// `driver` and `print_hex` have no input-dependent branches other than the
// loop over the fixed 16-byte length, which every case exercises.
// ---------------------------------------------------------------------------

// --- Class 1: input failure / EOF -----------------------------------------

#[test]
fn empty_input() {
    assert_same("empty input (immediate EOF)", b"");
}

#[test]
fn whitespace_only_inputs() {
    // Whitespace is skipped, then EOF is hit: input failure, x stays 0.
    for input in [
        &b" "[..],
        b"\n",
        b"\t",
        b"\r",
        b"\x0b",
        b"\x0c",
        b"   \n\t\r\x0b\x0c   ",
        b"\n\n\n\n\n",
    ] {
        assert_same("whitespace only", input);
    }
}

#[test]
fn sign_then_eof() {
    // A sign with no following character: matching failure, x stays 0.
    assert_same("lone '-'", b"-");
    assert_same("lone '+'", b"+");
    assert_same("whitespace then lone '-'", b"   -");
}

// --- Class 2: matching failure --------------------------------------------

#[test]
fn non_numeric_inputs() {
    for input in [
        &b"abc"[..],
        b"x",
        b"hello world",
        b".5",
        b"e5",
        b"--5",   // second '-' is not a digit
        b"++5",   // second '+' is not a digit
        b"- 5",   // space after the sign is not a digit
        b"-\n5",  // newline after the sign is not a digit either
        b"-x",
        b"+x",
        b"/",     // char just below '0'
        b":",     // char just above '9'
        b"\x7f",
    ] {
        assert_same("non-numeric", input);
    }
}

#[test]
fn non_utf8_and_nul_bytes() {
    // The C program reads bytes, not text; the Rust one must not choke.
    assert_same("NUL first", b"\x005");
    assert_same("digit then NUL", b"5\x006");
    assert_same("0xff bytes", b"\xff\xff5");
    assert_same("invalid utf-8 lead", b"\xc3\x285");
    assert_same("lone continuation byte", b"\x805");
    assert_same("truncated utf-8 at EOF", b"7\xe2\x82");
}

// --- Class 3 & 5 & 6: successful conversions ------------------------------

#[test]
fn single_values() {
    for input in [
        &b"0"[..], b"1", b"2", b"3", b"7", b"9", b"10", b"42", b"12345", b"1000000",
    ] {
        assert_same("single positive value", input);
    }
}

#[test]
fn signed_values() {
    for input in [
        &b"-0"[..], b"-1", b"-2", b"-42", b"+0", b"+1", b"+42", b"-1000000", b"+1000000",
    ] {
        assert_same("signed value", input);
    }
}

#[test]
fn leading_zeros_are_decimal_not_octal() {
    assert_same("leading zeros", b"0000000000000000000000005");
    assert_same("octal-looking", b"010");
    assert_same("hex-looking (stops at 'x')", b"0x10");
    assert_same("negative leading zeros", b"-0007");
    let mut many = vec![b'0'; 5000];
    many.push(b'9');
    assert_same("5000 leading zeros then 9", &many);
}

#[test]
fn scanf_skips_leading_whitespace_across_newlines() {
    assert_same("spaces then value", b"   42");
    assert_same("newlines then value", b"\n\n\n7");
    assert_same("mixed whitespace then value", b" \t\r\n\x0b\x0c 8");
    assert_same("tabs then negative", b"\t\t-9");
    // A large run of whitespace forces glibc to refill its input buffer.
    let mut big = vec![b' '; 100_000];
    big.extend_from_slice(b"-12345");
    assert_same("100k spaces then value", &big);
}

#[test]
fn conversion_stops_at_first_non_digit() {
    assert_same("digits then letters", b"12abc");
    assert_same("digits then space and more", b"3 4");
    assert_same("digits then newline and more", b"5\n6\n7\n");
    assert_same("digits then sign", b"1-2");
    assert_same("digits then dot", b"12.75");
    assert_same("digits then comma", b"1,000");
    assert_same("trailing newline only", b"11\n");
}

// --- Class 7: range edges, truncation and signedness ----------------------

#[test]
fn int_range_edges() {
    assert_same("INT_MAX", b"2147483647");
    assert_same("INT_MIN", b"-2147483648");
    assert_same("INT_MAX-1", b"2147483646");
    assert_same("INT_MIN+1", b"-2147483647");
    assert_same("2^31 (wraps to INT_MIN)", b"2147483648");
    assert_same("-(2^31)-1 (wraps to INT_MAX)", b"-2147483649");
    assert_same("2^32", b"4294967296");
    assert_same("2^32-1", b"4294967295");
    assert_same("2^31+1", b"2147483649");
}

#[test]
fn long_range_edges_and_saturation() {
    // glibc's %d funnels the digits through strtol: out-of-range saturates to
    // LONG_MAX / LONG_MIN, and the assignment to `int` then truncates
    // (LONG_MAX -> -1, LONG_MIN -> 0).
    assert_same("LONG_MAX", b"9223372036854775807");
    assert_same("LONG_MIN", b"-9223372036854775808");
    assert_same("LONG_MAX+1", b"9223372036854775808");
    assert_same("LONG_MIN-1", b"-9223372036854775809");
    assert_same("2^64", b"18446744073709551616");
    assert_same("huge positive", b"99999999999999999999999999");
    assert_same("huge negative", b"-99999999999999999999999999");
    assert_same("absurdly long positive", &[b'9'; 1000]);
    let mut neg = vec![b'-'];
    neg.extend_from_slice(&[b'9'; 1000]);
    assert_same("absurdly long negative", &neg);
    let mut one_then_zeros = vec![b'1'];
    one_then_zeros.extend_from_slice(&[b'0'; 1000]);
    assert_same("1 followed by 1000 zeros", &one_then_zeros);
}

#[test]
fn maximum_and_minimum_hex_digit_patterns() {
    // Values chosen so every byte of the `floors` field takes a non-trivial
    // value, checking `%02x` zero-padding and byte order.
    for input in [
        &b"1"[..],          // 01000000
        b"255",             // ff000000
        b"256",             // 00010000
        b"65535",           // ffff0000
        b"65536",           // 00000100
        b"16777215",        // ffffff00
        b"16777216",        // 00000001
        b"-1",              // ffffffff
        b"-256",            // 00ffffff
        b"305419896",       // 78563412
    ] {
        assert_same("hex pattern", input);
    }
}

// --- Redirections a shell can produce ------------------------------------

#[test]
fn stdin_from_dev_null() {
    let c = Command::new(c_bin())
        .stdin(Stdio::null())
        .output()
        .expect("run C with /dev/null stdin");
    let r = Command::new(rust_bin())
        .stdin(Stdio::null())
        .output()
        .expect("run Rust with /dev/null stdin");
    assert_eq!(c.stdout, r.stdout, "stdout mismatch with /dev/null stdin");
    assert_eq!(c.stderr, r.stderr, "stderr mismatch with /dev/null stdin");
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "exit status mismatch with /dev/null stdin"
    );
}

#[test]
fn extra_argv_is_ignored() {
    // `main()` takes no parameters, so arguments must make no difference.
    let c = Command::new(c_bin())
        .args(["some", "extra", "-args"])
        .stdin(Stdio::null())
        .output()
        .expect("run C with args");
    let r = Command::new(rust_bin())
        .args(["some", "extra", "-args"])
        .stdin(Stdio::null())
        .output()
        .expect("run Rust with args");
    assert_eq!(c.stdout, r.stdout, "stdout mismatch with extra argv");
    assert_eq!(c.stderr, r.stderr, "stderr mismatch with extra argv");
    assert_eq!(c.status.code(), r.status.code(), "exit status mismatch with extra argv");
}

// --- Shape of the output --------------------------------------------------

#[test]
fn output_shape_matches_print_hex_contract() {
    // 16 bytes as %02x plus one '\n' == 33 bytes, with no stderr output.
    let out = run(&c_bin(), b"5");
    assert_eq!(out.stdout.len(), 33, "C output should be 32 hex chars + newline");
    assert!(out.stderr.is_empty());
    assert_eq!(out.code, Some(0));
    // And Rust agrees, byte for byte.
    assert_same("output shape", b"5");
}

// --- Broad sweep ----------------------------------------------------------

#[test]
fn sweep_small_values() {
    for v in -300i64..=300 {
        assert_same("sweep", v.to_string().as_bytes());
    }
}

#[test]
fn sweep_powers_and_neighbours() {
    for bit in 0..70u32 {
        let base = 1i128 << bit;
        for delta in [-1i128, 0, 1] {
            let v = base + delta;
            assert_same("power of two", v.to_string().as_bytes());
            assert_same("negated power of two", (-v).to_string().as_bytes());
        }
    }
}

#[test]
fn sweep_generated_byte_soup() {
    // Deterministic pseudo-random inputs over the alphabet the parser branches
    // on, to catch parser states the hand-written cases miss.
    const ALPHABET: &[u8] = b" \t\n\r\x0b\x0c+-0123456789abxX.\x00\xff";
    let mut state: u64 = 0x243f_6a88_85a3_08d3;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for _ in 0..600 {
        let len = (next() % 13) as usize;
        let input: Vec<u8> = (0..len)
            .map(|_| ALPHABET[(next() % ALPHABET.len() as u64) as usize])
            .collect();
        assert_same("byte soup", &input);
    }
}
