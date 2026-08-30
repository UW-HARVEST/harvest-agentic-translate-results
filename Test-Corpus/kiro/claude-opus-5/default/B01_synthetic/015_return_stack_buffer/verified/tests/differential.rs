//! Differential tests: run the C reference binary and the Rust binary as
//! subprocesses on identical stdin and require byte-identical stdout, stderr
//! and identical exit status (including termination by signal).
//!
//! Nothing here links the Rust code as a library. Both programs are driven the
//! way a shell drives them, which is how this translation is graded.
//!
//! Every assertion is *differential*: the expected value is whatever the C
//! program did, never a hardcoded string. That keeps the suite honest even
//! where glibc's `scanf` behavior is surprising. `harness_sanity` is the one
//! exception, and exists only to prove the harness is really running two
//! different programs.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// Path of the Rust binary under test. Cargo builds this for us.
fn rust_bin() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Path of the compiled C reference binary.
///
/// Prefers the CMake build the instructions describe
/// (`c_src/build/driver`). If that is absent, compiles `c_src/src/main.c`
/// into this crate's `target/` directory instead, so the suite is
/// self-sufficient without ever writing inside `c_src/`.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let root = manifest_dir().parent().expect("crate has a parent dir").to_path_buf();
        let c_src = root.join("c_src");

        let prebuilt = c_src.join("build").join("driver");
        if prebuilt.is_file() {
            return prebuilt;
        }

        // Fallback: compile directly, leaving c_src untouched.
        let main_c = c_src.join("src").join("main.c");
        assert!(main_c.is_file(), "cannot find C source at {}", main_c.display());

        let out_dir = manifest_dir().join("target").join("c_reference");
        std::fs::create_dir_all(&out_dir).expect("create target/c_reference");
        let out = out_dir.join("driver");

        let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
        let status = Command::new(&cc)
            .arg("-o")
            .arg(&out)
            .arg(&main_c)
            .status()
            .unwrap_or_else(|e| panic!("failed to run C compiler {cc:?}: {e}"));
        assert!(status.success(), "compiling the C reference failed: {status:?}");
        out
    })
}

/// What a run of either program produced.
#[derive(PartialEq, Eq)]
struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// Normal exit code, if it exited normally.
    code: Option<i32>,
    /// Terminating signal, if it was killed.
    signal: Option<i32>,
}

impl std::fmt::Debug for Run {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Run")
            .field("stdout", &Escaped(&self.stdout))
            .field("stderr", &Escaped(&self.stderr))
            .field("code", &self.code)
            .field("signal", &self.signal)
            .finish()
    }
}

/// Renders bytes readably without assuming they are UTF-8.
struct Escaped<'a>(&'a [u8]);

impl std::fmt::Debug for Escaped<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"")?;
        for &b in self.0 {
            match b {
                b'\n' => write!(f, "\\n")?,
                b'\t' => write!(f, "\\t")?,
                b'\r' => write!(f, "\\r")?,
                b'"' => write!(f, "\\\"")?,
                b'\\' => write!(f, "\\\\")?,
                0x20..=0x7e => write!(f, "{}", b as char)?,
                _ => write!(f, "\\x{b:02x}")?,
            }
        }
        write!(f, "\" ({} bytes)", self.0.len())
    }
}

/// Run `bin` with `input` on stdin, capturing everything.
fn run(bin: &Path, input: &[u8]) -> Run {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    {
        let mut stdin = child.stdin.take().expect("stdin was piped");
        // Neither program reads all of stdin in every case, so a write here can
        // legitimately fail with EPIPE once the child exits. That is not a test
        // failure; it is the same thing a shell would see.
        let _ = stdin.write_all(input);
        let _ = stdin.flush();
    }

    let out = child.wait_with_output().expect("failed to collect output");

    #[cfg(unix)]
    let signal = {
        use std::os::unix::process::ExitStatusExt;
        out.status.signal()
    };
    #[cfg(not(unix))]
    let signal = None;

    Run { stdout: out.stdout, stderr: out.stderr, code: out.status.code(), signal }
}

/// Core assertion: identical stdout, stderr and exit status for one input.
#[track_caller]
fn assert_same(label: &str, input: &[u8]) {
    let c = run(c_bin(), input);
    let r = run(rust_bin(), input);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout differs for {label}\n  input: {:?}\n  C:    {:?}\n  Rust: {:?}",
        Escaped(input),
        Escaped(&c.stdout),
        Escaped(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr differs for {label}\n  input: {:?}\n  C:    {:?}\n  Rust: {:?}",
        Escaped(input),
        Escaped(&c.stderr),
        Escaped(&r.stderr)
    );
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "exit status differs for {label}\n  input: {:?}\n  C:    {:?}\n  Rust: {:?}",
        Escaped(input),
        c,
        r
    );
}

#[track_caller]
fn assert_same_str(label: &str, input: &str) {
    assert_same(label, input.as_bytes());
}

// ---------------------------------------------------------------------------
// Harness self-check
// ---------------------------------------------------------------------------

/// Proves the harness runs two distinct programs and that the two branches of
/// `printLine` are actually reachable. If this ever fails, every differential
/// result below is meaningless, so it is worth pinning literally.
///
/// `helperBad` returns the address of a local array; GCC folds that return to a
/// null pointer, so `printLine`'s NULL check suppresses all output on the bad
/// path -- not even a newline.
#[test]
fn harness_sanity() {
    assert_ne!(c_bin(), rust_bin(), "harness must compare two different binaries");

    let c_good = run(c_bin(), b"1");
    assert_eq!(c_good.stdout, b"helperGood1 string\n");
    assert_eq!(c_good.code, Some(0));

    let c_bad = run(c_bin(), b"0");
    assert_eq!(c_bad.stdout, b"", "the bad path prints nothing at all");
    assert_eq!(c_bad.code, Some(0));

    let r_good = run(rust_bin(), b"1");
    assert_eq!(r_good.stdout, b"helperGood1 string\n");
    let r_bad = run(rust_bin(), b"0");
    assert_eq!(r_bad.stdout, b"");
}

// ---------------------------------------------------------------------------
// Phase B: the branches in main / printLine
// ---------------------------------------------------------------------------

/// No input at all: `scanf` input failure, `x` keeps its initializer 0, so the
/// `else` (bad) branch runs.
#[test]
fn empty_input() {
    assert_same_str("empty", "");
}

/// The single documented "item": one integer, both truthiness outcomes.
#[test]
fn single_integer_both_branches() {
    assert_same_str("zero", "0");
    assert_same_str("one", "1");
}

#[test]
fn nonzero_values_take_good_branch() {
    for s in ["1", "2", "5", "42", "999999", "-1", "-3", "+5", "007", "0007"] {
        assert_same_str(s, s);
    }
}

/// Every spelling of zero still reaches `bad()`.
#[test]
fn zero_spellings_take_bad_branch() {
    for s in ["0", "-0", "+0", "00", "0000", "000000000000000000000"] {
        assert_same_str(s, s);
    }
}

// ---------------------------------------------------------------------------
// Phase B: scanf reads across newlines and skips whitespace
// ---------------------------------------------------------------------------

/// `%d` skips leading whitespace of every kind, including newlines -- the
/// documented difference from `fgets`.
#[test]
fn leading_whitespace_is_skipped() {
    assert_same_str("spaces then 7", "   7");
    assert_same_str("tab then 7", "\t7");
    assert_same_str("newline then 7", "\n7");
    assert_same_str("many newlines then 7", "\n\n\n\n7");
    assert_same_str("crlf then 7", "\r\n7");
    assert_same_str("vtab then 7", "\x0b7");
    assert_same_str("formfeed then 7", "\x0c7");
    assert_same_str("all ws kinds then 7", " \t\n\x0b\x0c\r7");
    assert_same_str("mixed ws then 0", " \t\n 0");
    assert_same_str("long ws run then 3", &format!("{}3", " \n\t".repeat(400)));
}

/// Whitespace only: `%d` consumes it, hits EOF, and fails without converting.
#[test]
fn whitespace_only_input() {
    for s in [" ", "   ", "\n", "\n\n\n", "\t", "\r", "\x0b", "\x0c", " \t\n\r\x0b\x0c"] {
        assert_same_str("whitespace only", s);
    }
    assert_same_str("long ws run only", &" \n\t".repeat(400));
}

/// Only the first conversion happens; trailing input is never read.
#[test]
fn only_first_token_is_converted() {
    assert_same_str("0 then 1", "0 1");
    assert_same_str("1 then 0", "1 0");
    assert_same_str("0 newline 1", "0\n1");
    assert_same_str("1 newline 0", "1\n0");
    assert_same_str("0 then junk", "0 abc");
    assert_same_str("1 with trailing newline", "1\n");
    assert_same_str("0 with trailing newline", "0\n");
    assert_same_str("many tokens", "0 1 2 3 4 5\n6 7 8\n");
}

/// `%d` stops at the first byte that cannot extend the number.
#[test]
fn conversion_stops_at_first_non_digit() {
    assert_same_str("1abc", "1abc");
    assert_same_str("0abc", "0abc");
    assert_same_str("hex-looking 0x10", "0x10"); // reads 0, stops at 'x'
    assert_same_str("hex-looking 0x0", "0x0");
    assert_same_str("float 3.9", "3.9"); // reads 3
    assert_same_str("float 0.9", "0.9"); // reads 0
    assert_same_str("float -0.5", "-0.5"); // reads -0 == 0
    assert_same_str("exponent 1e9", "1e9"); // reads 1
    assert_same_str("0e9", "0e9");
    assert_same_str("1,000", "1,000");
    assert_same_str("digits then NUL", "1\0002");
    assert_same_str("underscore", "1_0");
}

// ---------------------------------------------------------------------------
// Phase C: matching failures -- input that never yields a number
// ---------------------------------------------------------------------------

/// On a matching failure `scanf` leaves `x` alone, so `x == 0` and `bad()` runs.
#[test]
fn matching_failures_leave_x_zero() {
    for s in [
        "abc", "x", "z9", "-", "+", "--1", "++1", "-+1", "+-1", ".", ".5", "-.5", "+.5", "/", ":",
        "0x", "e", "E", "nan", "inf", "NULL", "#", "!", "~", "'", "\"", "?", "%", "%d", "*", "*1",
        " -", " +", "\n-", "- 1", "+ 1", "-\n1", "(1)", "[1]", "\\1",
    ] {
        assert_same_str(s, s);
    }
}

/// Bytes that are neither digits, signs nor whitespace, including non-UTF-8.
#[test]
fn non_ascii_and_control_bytes() {
    assert_same("NUL first", b"\x00");
    assert_same("NUL then digit", b"\x001");
    assert_same("high byte", b"\xff");
    assert_same("invalid utf8", b"\xff\xfe\x801");
    assert_same("utf8 minus sign", "\u{2212}5".as_bytes());
    assert_same("utf8 digit", "\u{ff11}".as_bytes()); // fullwidth 1
    assert_same("bom then 1", b"\xef\xbb\xbf1");
    assert_same("bell", b"\x071");
    assert_same("escape", b"\x1b[0m1");
    assert_same("del", b"\x7f1");
    assert_same("all non-ws control bytes", &(1u8..=8).collect::<Vec<u8>>());
}

// ---------------------------------------------------------------------------
// Phase C: integer range, truncation and signedness exactly as C does it
// ---------------------------------------------------------------------------

/// glibc's `%d` converts with `strtol` semantics into a `long`, then stores
/// through an `int *`. Out-of-range input therefore *saturates* at
/// `LONG_MAX`/`LONG_MIN` and is then *truncated* to 32 bits -- it does not clamp
/// to `INT_MAX`/`INT_MIN`. Truncation is observable because it decides which
/// branch runs: `4294967296` truncates to 0 and takes the bad path even though
/// the number is nonzero.
#[test]
fn int_range_edges() {
    for s in [
        "2147483647",  // INT_MAX
        "-2147483648", // INT_MIN
        "2147483648",  // INT_MAX + 1 -> 0x80000000 -> INT_MIN, nonzero
        "-2147483649",
        "4294967295", // 2^32 - 1 -> -1, nonzero
        "4294967296", // 2^32     -> 0, BAD branch despite nonzero input
        "4294967297", // 2^32 + 1 -> 1, nonzero
        "-4294967295",
        "-4294967296", // -2^32 -> 0, bad branch
        "-4294967297",
        "8589934592", // 2^33 -> 0
        "1099511627776",
    ] {
        assert_same_str(s, s);
    }
}

/// Values at and beyond the `long` range, where `strtol` saturates.
#[test]
fn long_range_saturation() {
    for s in [
        "9223372036854775807",  // LONG_MAX      -> low 32 bits 0xffffffff
        "9223372036854775808",  // LONG_MAX + 1  -> saturates to LONG_MAX
        "-9223372036854775808", // LONG_MIN      -> low 32 bits 0
        "-9223372036854775809", // beyond        -> saturates to LONG_MIN
        "10000000000000000000",
        "18446744073709551615", // 2^64 - 1
        "18446744073709551616", // 2^64
        "99999999999999999999999999",
        "-99999999999999999999999999",
    ] {
        assert_same_str(s, s);
    }
}

/// The sign of an out-of-range value is observable *only* under saturation, and
/// it flips which branch runs. `strtol` saturates to `LONG_MAX` for `+huge`
/// (low 32 bits `0xffffffff`, nonzero -> good) but to `LONG_MIN` for `-huge`
/// (low 32 bits `0`, zero -> bad). For in-range values the sign is invisible
/// here, because the low 32 bits of `-v` are zero exactly when those of `v` are.
#[test]
fn sign_is_observable_under_saturation() {
    for s in [
        "+9223372036854775807",
        "+9223372036854775808",
        "-9223372036854775808",
        "-9223372036854775809",
        "+18446744073709551616",
        "-18446744073709551616",
        "+99999999999999999999999999",
        "-99999999999999999999999999",
    ] {
        assert_same_str(s, s);
    }
    assert_same_str("plus 1000 nines", &format!("+{}", "9".repeat(1000)));
    assert_same_str("minus 1000 nines", &format!("-{}", "9".repeat(1000)));
    assert_same_str("ws then plus huge", &format!("  \n+{}", "9".repeat(40)));
    assert_same_str("ws then minus huge", &format!("  \n-{}", "9".repeat(40)));
}

/// Digit strings far past any integer width, and leading zeros that make a
/// short number look long.
#[test]
fn very_long_digit_strings() {
    assert_same_str("1000 nines", &"9".repeat(1000));
    assert_same_str("negative 1000 nines", &format!("-{}", "9".repeat(1000)));
    assert_same_str("1000 zeros", &"0".repeat(1000));
    assert_same_str("negative 1000 zeros", &format!("-{}", "0".repeat(1000)));
    assert_same_str("1000 zeros then 1", &format!("{}1", "0".repeat(1000)));
    assert_same_str("10000 nines", &"9".repeat(10_000));
    assert_same_str("1 then 1000 zeros", &format!("1{}", "0".repeat(1000)));
    assert_same_str("alternating", &"12".repeat(600));
}

/// A sign followed by many digits, and signs in positions `%d` rejects.
#[test]
fn sign_handling() {
    assert_same_str("+2147483647", "+2147483647");
    assert_same_str("+0000000001", "+0000000001");
    assert_same_str("-0000000000", "-0000000000");
    assert_same_str("-0000000001", "-0000000001");
    assert_same_str("sign at EOF", "-");
    assert_same_str("plus at EOF", "+");
    assert_same_str("sign then space", "- ");
    assert_same_str("sign then newline", "-\n");
    assert_same_str("1-2", "1-2");
    assert_same_str("1+2", "1+2");
}

// ---------------------------------------------------------------------------
// Phase C: stdin shapes other than a plain pipe write
// ---------------------------------------------------------------------------

/// stdin closed immediately: read returns EOF, same as empty input.
#[test]
fn stdin_closed_immediately() {
    let c = {
        let child = Command::new(c_bin())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn C");
        child.wait_with_output().expect("collect C")
    };
    let r = {
        let child = Command::new(rust_bin())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn Rust");
        child.wait_with_output().expect("collect Rust")
    };

    assert_eq!(Escaped(&c.stdout).to_debug(), Escaped(&r.stdout).to_debug(), "stdout with /dev/null stdin");
    assert_eq!(Escaped(&c.stderr).to_debug(), Escaped(&r.stderr).to_debug(), "stderr with /dev/null stdin");
    assert_eq!(c.status.code(), r.status.code(), "exit code with /dev/null stdin");
}

impl Escaped<'_> {
    fn to_debug(&self) -> String {
        format!("{self:?}")
    }
}

/// A large input where the number appears only after more bytes than any single
/// stdio buffer, so buffered and unbuffered readers must still agree.
#[test]
fn input_larger_than_a_stdio_buffer() {
    assert_same_str("64KiB of spaces then 1", &format!("{}1", " ".repeat(64 * 1024)));
    assert_same_str("64KiB of newlines then 0", &format!("{}0", "\n".repeat(64 * 1024)));
    assert_same_str("1 then 64KiB of junk", &format!("1{}", "x".repeat(64 * 1024)));
    assert_same_str("0 then 64KiB of junk", &format!("0{}", "x".repeat(64 * 1024)));
    assert_same_str("junk then number", &format!("{}1", "x".repeat(64 * 1024)));
}

/// stdout is a pipe whose reader has already gone away by the time the program
/// writes. A C program inherits the default `SIGPIPE` disposition and dies with
/// signal 13 (status 141 through a shell); the Rust runtime ignores `SIGPIPE`
/// unless that is undone, which would make it exit 0 instead.
///
/// The `sleep` delays the program's only write until well after the reader has
/// exited, so the closed pipe is not a race. The assertion is differential, so
/// this test is meaningful on any platform without needing to be skipped.
#[test]
fn sigpipe_disposition_matches() {
    fn status_via_shell(bin: &Path) -> Option<String> {
        let script = format!(
            "{{ sleep 0.5; printf 1; }} | {} 2>/dev/null | true; printf '%s' \"${{PIPESTATUS[1]}}\"",
            shell_quote(bin)
        );
        let out = Command::new("bash").arg("-c").arg(&script).output().ok()?;
        Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    let c = status_via_shell(c_bin());
    let r = status_via_shell(rust_bin());
    assert_eq!(c, r, "exit status on SIGPIPE differs (C={c:?}, Rust={r:?})");
}

fn shell_quote(p: &Path) -> String {
    format!("'{}'", p.to_string_lossy().replace('\'', "'\\''"))
}

// ---------------------------------------------------------------------------
// Determinism and repeatability
// ---------------------------------------------------------------------------

/// stdout redirected to a regular file rather than a pipe. C's stdout is fully
/// buffered in both cases, but the flush path differs, so the file contents are
/// worth comparing directly.
#[test]
fn stdout_redirected_to_a_file() {
    let dir = std::env::temp_dir().join(format!("driver_diff_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");

    for (i, input) in ["1", "0", "", "  \n42", "abc", "4294967296"].iter().enumerate() {
        let mut captured = Vec::new();
        for (tag, bin) in [("c", c_bin()), ("rust", rust_bin())] {
            let path = dir.join(format!("{tag}_{i}.out"));
            let file = std::fs::File::create(&path).expect("create output file");
            let mut child = Command::new(bin)
                .stdin(Stdio::piped())
                .stdout(Stdio::from(file))
                .stderr(Stdio::piped())
                .spawn()
                .expect("spawn");
            {
                let mut stdin = child.stdin.take().expect("stdin piped");
                let _ = stdin.write_all(input.as_bytes());
            }
            let out = child.wait_with_output().expect("collect");
            let bytes = std::fs::read(&path).expect("read output file");
            captured.push((bytes, out.stderr, out.status.code()));
        }
        assert_eq!(
            captured[0].0, captured[1].0,
            "file stdout differs for {:?}: C={:?} Rust={:?}",
            input,
            Escaped(&captured[0].0),
            Escaped(&captured[1].0)
        );
        assert_eq!(captured[0].1, captured[1].1, "stderr differs for {input:?}");
        assert_eq!(captured[0].2, captured[1].2, "exit code differs for {input:?}");
    }

    let _ = std::fs::remove_dir_all(&dir);
}

/// `main` in the C takes no parameters and ignores argv entirely; extra
/// arguments must not change anything.
#[test]
fn command_line_arguments_are_ignored() {
    for args in [vec!["1"], vec!["0"], vec!["--help"], vec!["-x", "abc"], vec![""]] {
        let mut results = Vec::new();
        for bin in [c_bin(), rust_bin()] {
            let mut child = Command::new(bin)
                .args(&args)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("spawn");
            {
                let mut stdin = child.stdin.take().expect("stdin piped");
                let _ = stdin.write_all(b"1");
            }
            let out = child.wait_with_output().expect("collect");
            results.push((out.stdout, out.stderr, out.status.code()));
        }
        assert_eq!(results[0], results[1], "output differs with args {args:?}");
    }
}

/// Neither program should vary run to run, and repeated runs must not leak
/// state (the bad path in particular reads a dangling pointer in C).
#[test]
fn repeated_runs_are_stable() {
    for _ in 0..25 {
        assert_same_str("repeat zero", "0");
        assert_same_str("repeat one", "1");
        assert_same_str("repeat empty", "");
    }
}

/// A broad sweep of small integers, both signs, to be sure the truthiness test
/// lines up across the whole neighborhood of zero.
#[test]
fn sweep_small_integers() {
    for n in -300i32..=300 {
        let s = n.to_string();
        assert_same_str(&s, &s);
    }
}

/// Byte-level fuzz over short inputs drawn from the alphabet the parser
/// branches on. Deterministic (fixed seed) so failures are reproducible.
#[test]
fn deterministic_fuzz_short_inputs() {
    const ALPHABET: &[u8] = b"0123456789+- \t\n\rxX.eE\0\xff";
    // xorshift64*, so the suite pulls in no dependencies.
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state = state.wrapping_mul(0x2545_F491_4F6C_DD1D);
        state
    };

    for case in 0..600 {
        let len = (next() % 9) as usize;
        let input: Vec<u8> = (0..len).map(|_| ALPHABET[(next() % ALPHABET.len() as u64) as usize]).collect();
        assert_same(&format!("fuzz case {case}"), &input);
    }
}

/// Longer fuzz inputs, so that overflow and saturation interact with signs and
/// trailing junk. Short inputs cannot reach the `long` range at all, which is
/// exactly where the sign becomes observable.
#[test]
fn deterministic_fuzz_long_inputs() {
    const ALPHABET: &[u8] = b"0123456789999999+- \n";
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state = state.wrapping_mul(0x2545_F491_4F6C_DD1D);
        state
    };

    for case in 0..400 {
        let len = 10 + (next() % 26) as usize;
        let input: Vec<u8> = (0..len).map(|_| ALPHABET[(next() % ALPHABET.len() as u64) as usize]).collect();
        assert_same(&format!("long fuzz case {case}"), &input);
    }
}
