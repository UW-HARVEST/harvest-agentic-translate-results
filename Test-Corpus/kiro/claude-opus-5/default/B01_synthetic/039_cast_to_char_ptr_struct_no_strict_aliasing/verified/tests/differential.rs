//! Differential tests: run the original C program and the Rust translation as
//! subprocesses with identical stdin, and require byte-identical stdout,
//! byte-identical stderr and an identical exit status.
//!
//! The Rust code is never linked in as a library; the built binary is driven
//! exactly the way a shell would drive it, because that is how the two programs
//! are compared.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;

/// The working directory that contains both `c_src/` and `translation/`.
fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is `<root>/translation`.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the Rust binary under test, as produced by cargo for this test run.
fn rust_bin() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// Path to the compiled C binary, building it with CMake on first use.
///
/// Nothing inside `c_src/` is modified: CMake only writes into `c_src/build/`,
/// which is the out-of-tree build directory the project's own instructions use.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = repo_root().join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");
        if !exe.exists() {
            std::fs::create_dir_all(&build).expect("could not create c_src/build");

            let configure = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("failed to run `cmake ..` (is cmake installed?)");
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
                .expect("failed to run `cmake --build .`");
            assert!(
                compile.status.success(),
                "cmake build failed:\n{}\n{}",
                String::from_utf8_lossy(&compile.stdout),
                String::from_utf8_lossy(&compile.stderr),
            );
        }
        assert!(
            exe.exists(),
            "the C binary was not produced at {}",
            exe.display()
        );
        exe
    })
}

/// Run one program with `stdin` piped in, capturing stdout and stderr.
fn run(program: &Path, stdin: &[u8]) -> Output {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", program.display()));

    {
        let mut sink = child.stdin.take().expect("stdin was piped");
        // A broken pipe is legitimate: the program may exit before draining
        // stdin. That is the same thing a shell would observe, so it is not a
        // test failure.
        let _ = sink.write_all(stdin);
        let _ = sink.flush();
    }

    child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("failed to wait for {}: {e}", program.display()))
}

/// Assert that both programs behave identically for one input.
fn assert_same(label: &str, stdin: &[u8]) {
    let c = run(c_bin(), stdin);
    let r = run(rust_bin(), stdin);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout differs for {label} (input {:?})\n  C   : {:?}\n  Rust: {:?}",
        Escaped(stdin),
        Escaped(&c.stdout),
        Escaped(&r.stdout),
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr differs for {label} (input {:?})\n  C   : {:?}\n  Rust: {:?}",
        Escaped(stdin),
        Escaped(&c.stderr),
        Escaped(&r.stderr),
    );
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "exit code differs for {label} (input {:?}): C {:?} vs Rust {:?}",
        Escaped(stdin),
        c.status,
        r.status,
    );
    assert_eq!(
        format!("{}", c.status),
        format!("{}", r.status),
        "exit status differs for {label} (input {:?})",
        Escaped(stdin),
    );
}

/// Debug-printable byte string, so failures stay readable for non-UTF-8 input.
struct Escaped<'a>(&'a [u8]);

impl std::fmt::Debug for Escaped<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("\"")?;
        for &b in self.0 {
            match b {
                b'\\' => f.write_str("\\\\")?,
                b'"' => f.write_str("\\\"")?,
                b'\n' => f.write_str("\\n")?,
                b'\r' => f.write_str("\\r")?,
                b'\t' => f.write_str("\\t")?,
                0x20..=0x7e => f.write_str(&(b as char).to_string())?,
                _ => write!(f, "\\x{b:02x}")?,
            }
        }
        f.write_str("\"")
    }
}

// ---------------------------------------------------------------------------
// Phase A sanity: the two programs exist and agree on the trivial input.
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_run() {
    let c = run(c_bin(), b"1\n");
    let r = run(rust_bin(), b"1\n");
    assert!(!c.stdout.is_empty(), "the C program produced no stdout");
    assert!(!r.stdout.is_empty(), "the Rust program produced no stdout");
    assert_same("baseline", b"1\n");
}

/// The one output shape the program can ever produce: 32 lowercase hex digits
/// and exactly one trailing newline. Pins down `print_hex`'s formatting.
#[test]
fn output_shape_is_32_hex_digits_and_one_newline() {
    for input in [&b""[..], b"1\n", b"-1\n", b"garbage\n"] {
        let c = run(c_bin(), input);
        assert_eq!(c.stdout.len(), 33, "C output length changed unexpectedly");
        assert_eq!(*c.stdout.last().unwrap(), b'\n');
        assert!(c.stdout[..32]
            .iter()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b)));
        assert_same("output shape", input);
    }
}

// ---------------------------------------------------------------------------
// Phase B: the input classes the C program actually branches on.
//
// `main` has a single branch point: whether `scanf("%d", &x)` converts a value
// or leaves `x` at its initial 0 (matching failure or EOF). `driver` and
// `print_hex` are branch-free apart from the per-byte loop, whose only input is
// `sizeof(house_t)`. So the interesting classes are all scanf classes plus the
// integer conversion/truncation behaviour feeding `house.floors`.
// ---------------------------------------------------------------------------

/// EOF before any conversion: `x` keeps its initial value.
#[test]
fn empty_input() {
    assert_same("empty stdin", b"");
}

/// Whitespace only, i.e. EOF is reached while skipping whitespace.
#[test]
fn whitespace_only_input() {
    for input in [
        &b"\n"[..],
        b" ",
        b"\t",
        b"\r",
        b"\x0b",
        b"\x0c",
        b"   \n\n  \t\r\x0b\x0c  ",
    ] {
        assert_same("whitespace only", input);
    }
}

/// A single ordinary value, with and without a trailing newline.
#[test]
fn single_value() {
    for input in [&b"1"[..], b"1\n", b"0", b"0\n", b"42\n", b"7"] {
        assert_same("single value", input);
    }
}

/// Signs: `%d` accepts an optional leading '+' or '-'.
#[test]
fn signs() {
    for input in [&b"+7\n"[..], b"-7\n", b"+0\n", b"-0\n", b"+2147483647\n"] {
        assert_same("signed value", input);
    }
}

/// `%d` skips leading whitespace of every kind in the C locale's `isspace`
/// set, including the vertical tab that Rust's `is_ascii_whitespace` omits.
#[test]
fn leading_whitespace_is_skipped() {
    for prefix in [
        &b" "[..],
        b"   ",
        b"\t",
        b"\n",
        b"\n\n\n",
        b"\r",
        b"\x0b",
        b"\x0c",
        b" \t\n\x0b\x0c\r ",
    ] {
        let mut input = prefix.to_vec();
        input.extend_from_slice(b"42\n");
        assert_same("leading whitespace", &input);
    }
}

/// Every single leading byte, so no character is misclassified as whitespace
/// (or as not-whitespace) relative to C.
#[test]
fn every_leading_byte_agrees() {
    for b in 0u16..=255 {
        let input = [b as u8, b'4', b'2', b'\n'];
        assert_same(&format!("leading byte 0x{b:02x}"), &input);
    }
}

/// Matching failure: the first non-whitespace character cannot start a number,
/// so `scanf` returns 0 and `x` stays 0.
#[test]
fn matching_failure() {
    for input in [
        &b"abc\n"[..],
        b"x",
        b".5\n",
        b"-\n",
        b"+\n",
        b"- 5\n",
        b"+-5\n",
        b"--5\n",
        b"-abc\n",
        b"/\n",
        b":\n",
        b"\0",
        b"\x0042\n",
        b"\xff\xfe\n", // not valid UTF-8
    ] {
        assert_same("matching failure", input);
    }
}

/// Only the first conversion happens; trailing text is never consumed.
#[test]
fn trailing_input_is_ignored() {
    for input in [
        &b"42abc\n"[..],
        b"1 2\n",
        b"1\n2\n3\n",
        b"3.9\n",
        b"1e5\n",
        b"0x10\n",
        b"5,6\n",
        b"7-8\n",
    ] {
        assert_same("trailing input", input);
    }
}

/// `%d` is base 10, so leading zeros are just zeros.
#[test]
fn leading_zeros() {
    for input in [
        &b"007\n"[..],
        b"0000000000000000000005\n",
        b"-0000000000000000000005\n",
        b"00000000000000000000000000000000000000000000000000\n",
    ] {
        assert_same("leading zeros", input);
    }
}

/// Exact `int` boundaries.
#[test]
fn int_boundaries() {
    for input in [
        &b"2147483647\n"[..],  // INT_MAX
        b"-2147483648\n",      // INT_MIN
        b"2147483648\n",       // INT_MAX + 1, truncates
        b"-2147483649\n",      // INT_MIN - 1, truncates
        b"4294967295\n",       // UINT_MAX -> -1
        b"4294967296\n",       // 2^32 -> 0
        b"-4294967296\n",      // -2^32 -> 0
        b"2147483647999\n",
    ] {
        assert_same("int boundary", input);
    }
}

/// Values that fit a `long` but not an `int`: glibc converts into a `long` and
/// the result is stored through an `int *`, so the high half is discarded.
#[test]
fn long_to_int_truncation() {
    for input in [
        &b"9223372036854775807\n"[..], // LONG_MAX
        b"-9223372036854775808\n",     // LONG_MIN
        b"9223372036854775806\n",
        b"1099511627776\n", // 2^40
        b"-1099511627776\n",
        b"8589934593\n", // 2^33 + 1
    ] {
        assert_same("long truncation", input);
    }
}

/// Past `LONG_MAX`/`LONG_MIN` the conversion saturates before truncation, so
/// huge positives become -1 and huge negatives become 0.
#[test]
fn overflow_saturates_then_truncates() {
    for input in [
        &b"9223372036854775808\n"[..], // LONG_MAX + 1
        b"-9223372036854775809\n",     // LONG_MIN - 1
        b"18446744073709551616\n",     // 2^64
        b"99999999999999999999\n",
        b"-99999999999999999999\n",
    ] {
        assert_same("overflow", input);
    }
}

/// Long digit runs, including ones far past any integer width.
#[test]
fn long_digit_runs() {
    for n in [18usize, 19, 20, 21, 30, 64, 100, 1000, 5000] {
        let nines: Vec<u8> = std::iter::repeat(b'9').take(n).collect();
        assert_same(&format!("{n} nines"), &nines);

        let mut negative = vec![b'-'];
        negative.extend_from_slice(&nines);
        assert_same(&format!("negative {n} nines"), &negative);

        // Leading zeros then a small value: long input, small result.
        let mut padded: Vec<u8> = std::iter::repeat(b'0').take(n).collect();
        padded.push(b'5');
        assert_same(&format!("{n} zeros then 5"), &padded);
    }
}

// ---------------------------------------------------------------------------
// Phase C: paths not covered above.
// ---------------------------------------------------------------------------

/// No newline at all anywhere: EOF terminates the digit run rather than a
/// delimiter. Also covers `scanf` hitting EOF mid-number.
#[test]
fn no_trailing_newline() {
    for input in [&b"42"[..], b"-42", b"+42", b"2147483647", b"9"] {
        assert_same("no trailing newline", input);
    }
}

/// `scanf` reads across newlines while skipping whitespace, so a value on a
/// later line is still found. This is the `scanf`-vs-`fgets` distinction.
#[test]
fn scanf_reads_across_newlines() {
    for input in [
        &b"\n\n\n\n42\n"[..],
        b"\n \n\t\n 42",
        b"\n\n\n\n\n\n\n\n\n\n7\n",
    ] {
        assert_same("across newlines", input);
    }
}

/// Every byte value appearing immediately after a valid number, to make sure
/// the digit run terminates identically on all delimiters.
#[test]
fn every_terminating_byte_agrees() {
    for b in 0u16..=255 {
        let input = [b'4', b'2', b as u8, b'7'];
        assert_same(&format!("terminator 0x{b:02x}"), &input);
    }
}

/// Every byte value directly after a sign, where only digits may follow.
#[test]
fn every_byte_after_sign_agrees() {
    for b in 0u16..=255 {
        let input = [b'-', b as u8, b'1'];
        assert_same(&format!("after sign 0x{b:02x}"), &input);
    }
}

/// Binary garbage, including embedded NULs and non-UTF-8 sequences, so the
/// Rust program cannot rely on the input being valid text.
#[test]
fn binary_input() {
    for input in [
        &b"\x00\x01\x02\x03"[..],
        b"\xff\xff\xff\xff",
        b"\x80\x81 42",
        b"4\x002",
        b"\xc3\x28 9",
        b" \xef\xbb\xbf7", // UTF-8 BOM before a digit
    ] {
        assert_same("binary input", input);
    }
}

/// A large input the program will not fully consume. The C program stops
/// reading after the first conversion; the Rust program must produce the same
/// output (and the same exit status) rather than being disturbed by the rest.
#[test]
fn very_large_unconsumed_input() {
    let mut input = b"42\n".to_vec();
    input.extend(std::iter::repeat(b'x').take(64 * 1024));
    assert_same("large unconsumed input", &input);
}

/// The output must not depend on stdout being a terminal or on buffering mode:
/// both programs are piped here, and the byte count is fixed.
#[test]
fn output_is_fully_flushed() {
    for input in [&b""[..], b"1\n", b"9999999999999999999999\n"] {
        let r = run(rust_bin(), input);
        assert_eq!(
            r.stdout.len(),
            33,
            "Rust output was truncated or unflushed for {:?}",
            Escaped(input)
        );
        assert_same("flush", input);
    }
}

/// A broad randomized sweep over the integer space, exercising the conversion
/// and truncation arithmetic far more densely than the hand-picked cases.
#[test]
fn randomized_integers() {    // Deterministic xorshift so failures are reproducible.
    let mut state: u64 = 0x9E3779B97F4A7C15;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    for _ in 0..300 {
        let raw = next();
        // Mix of widths: 32-bit, 64-bit, and oversized decimal strings.
        let candidates = [
            format!("{}\n", raw as i32),
            format!("{}\n", raw as i64),
            format!("{}\n", raw),
            format!("-{}\n", raw),
            format!("{}{}\n", raw, raw),
        ];
        for c in candidates {
            assert_same("random integer", c.as_bytes());
        }
    }
}

// ---------------------------------------------------------------------------
// Phase C, continued: stdin/stdout error paths.
//
// `main` never checks scanf's return value and always `return 0`, so even a
// read error must still print and exit 0. These cases are reached by giving the
// programs something other than a readable pipe.
// ---------------------------------------------------------------------------

/// Run a program with a caller-supplied stdin, capturing stdout and stderr.
fn run_with_stdin(program: &Path, stdin: Stdio) -> Output {
    Command::new(program)
        .stdin(stdin)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap_or_else(|e| panic!("failed to run {}: {e}", program.display()))
}

fn assert_same_output(label: &str, c: &Output, r: &Output) {
    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout differs for {label}\n  C   : {:?}\n  Rust: {:?}",
        Escaped(&c.stdout),
        Escaped(&r.stdout),
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr differs for {label}\n  C   : {:?}\n  Rust: {:?}",
        Escaped(&c.stderr),
        Escaped(&r.stderr),
    );
    assert_eq!(
        format!("{}", c.status),
        format!("{}", r.status),
        "exit status differs for {label}: C {} vs Rust {}",
        c.status,
        r.status,
    );
}

/// stdin is /dev/null: an immediate EOF from a non-pipe.
#[test]
fn stdin_is_dev_null() {
    let c = run_with_stdin(c_bin(), Stdio::null());
    let r = run_with_stdin(rust_bin(), Stdio::null());
    assert_same_output("stdin = /dev/null", &c, &r);
}

/// stdin is a directory, so the very first read fails with EISDIR. `scanf`
/// reports an input failure, `x` stays 0, and the program still exits 0.
#[test]
fn stdin_is_a_directory() {
    let dir = repo_root();
    let open = || {
        Stdio::from(
            std::fs::File::open(&dir).expect("could not open the working directory for reading"),
        )
    };
    let c = run_with_stdin(c_bin(), open());
    let r = run_with_stdin(rust_bin(), open());
    assert_same_output("stdin = directory", &c, &r);
}

/// stdout is closed, so the write itself fails. Neither program checks
/// `printf`'s result, so both must stay silent on stderr and exit 0.
#[test]
fn stdout_is_closed() {
    let via_shell = |program: &Path| {
        Command::new("sh")
            .arg("-c")
            .arg(format!("printf '42\\n' | '{}' >&-", program.display()))
            .output()
            .expect("failed to run the program through sh")
    };
    let c = via_shell(c_bin());
    let r = via_shell(rust_bin());
    assert_same_output("stdout closed", &c, &r);
}

/// stdout is a pipe whose reader exits immediately, the classic SIGPIPE case.
/// Both programs must end the same way (same signal or same code).
#[test]
fn stdout_reader_exits_early() {
    let via_shell = |program: &Path| {
        Command::new("sh")
            .arg("-c")
            .arg(format!(
                "printf '42\\n' | '{}' | head -c 0; exit \"$?\"",
                program.display()
            ))
            .output()
            .expect("failed to run the program through sh")
    };
    let c = via_shell(c_bin());
    let r = via_shell(rust_bin());
    assert_same_output("stdout reader exits early", &c, &r);
}

/// Command-line arguments are declared away by `int main()` and never read, so
/// passing some must change nothing.
#[test]
fn arguments_are_ignored() {
    let with_args = |program: &Path| {
        let mut child = Command::new(program)
            .args(["--help", "-1", "garbage"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to spawn with arguments");
        {
            let mut sink = child.stdin.take().expect("stdin was piped");
            let _ = sink.write_all(b"42\n");
        }
        child.wait_with_output().expect("failed to wait")
    };
    let c = with_args(c_bin());
    let r = with_args(rust_bin());
    assert_same_output("with arguments", &c, &r);
}
