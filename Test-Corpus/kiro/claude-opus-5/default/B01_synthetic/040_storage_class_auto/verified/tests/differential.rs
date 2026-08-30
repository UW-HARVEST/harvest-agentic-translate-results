// Differential tests: run the C binary and the Rust binary as subprocesses on
// identical stdin and require byte-identical stdout, byte-identical stderr and
// an identical exit status (including death by signal).
//
// The Rust program is never linked as a library here. Both programs are driven
// exactly the way a shell would drive them, because that is how the
// translation is graded.

use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

// ---------------------------------------------------------------------------
// Locating the two binaries
// ---------------------------------------------------------------------------

/// Workspace root: the directory holding both `c_src/` and `translation/`.
fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is `<root>/translation`.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the compiled C program, building it with CMake if it is absent.
///
/// Only the generated `c_src/build/` tree is touched; no source file in
/// `c_src/` is read-modify-written by the tests.
fn c_binary() -> PathBuf {
    let root = repo_root();
    let c_src = root.join("c_src");
    let build = c_src.join("build");
    let exe = build.join("driver");

    if exe.exists() {
        return exe;
    }

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

    assert!(exe.exists(), "C binary missing after build: {}", exe.display());
    exe
}

/// Path to the compiled Rust program. Cargo builds and rebuilds this for us.
fn rust_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// The `--release` binary, if it has already been built. The graded artifact is
/// the release build, so it is checked in addition to the one Cargo hands the
/// test harness. It is never built from here: invoking `cargo` inside `cargo
/// test` would block on the target-directory lock.
fn release_binary() -> Option<PathBuf> {
    let p = repo_root().join("translation/target/release/driver");
    if p.exists() {
        Some(p)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Running a program
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Ok(code)` for normal exit, `Err(signal)` when killed by a signal.
    status: Result<i32, i32>,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "stdout={:?} stderr={:?} status={}",
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr),
            match self.status {
                Ok(c) => format!("exit {c}"),
                Err(s) => format!("signal {s}"),
            }
        )
    }
}

/// Runs `exe` with `input` on stdin, capturing stdout and stderr.
fn run(exe: &Path, input: &[u8]) -> Outcome {
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));

    {
        let mut stdin = child.stdin.take().expect("stdin was piped");
        // A program may exit without draining stdin (the C program reads at most
        // one number). A short/failed write is expected in that case, not a
        // test failure.
        let _ = stdin.write_all(input);
        let _ = stdin.flush();
    }

    let out = child.wait_with_output().expect("failed to wait for child");
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        status: match out.status.code() {
            Some(c) => Ok(c),
            None => Err(out.status.signal().expect("no exit code and no signal")),
        },
    }
}

/// Runs `exe` with `input` on stdin and a stdout pipe whose read end is already
/// closed, so the first write fails. Used to compare broken-pipe behavior.
fn run_with_closed_stdout(exe: &Path, input: &[u8]) -> Outcome {
    use std::os::unix::io::FromRawFd;

    // `pipe2` with `O_CLOEXEC` is required, not `pipe`. A plain `pipe(2)`
    // returns fds without close-on-exec, so the child inherits the read end,
    // the pipe still has a reader, and the write succeeds — the broken pipe
    // would never happen and this test would silently measure nothing.
    // `dup2` onto the child's fd 1 clears CLOEXEC, so stdout still works.
    const O_CLOEXEC: i32 = 0o2000000;
    extern "C" {
        fn pipe2(fds: *mut i32, flags: i32) -> i32;
        fn close(fd: i32) -> i32;
    }

    let mut fds = [0i32; 2];
    let rc = unsafe { pipe2(fds.as_mut_ptr(), O_CLOEXEC) };
    assert_eq!(rc, 0, "pipe2() failed");
    let (read_end, write_end) = (fds[0], fds[1]);

    // SAFETY: `write_end` is a fresh fd owned by us; ownership moves to Stdio,
    // which closes the parent's copy once the Command is dropped after spawn.
    let child_stdout = unsafe { Stdio::from_raw_fd(write_end) };

    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(child_stdout)
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));

    // Drop every reader of the pipe so the child's write gets EPIPE/SIGPIPE.
    unsafe {
        close(read_end);
    }

    {
        let mut stdin = child.stdin.take().expect("stdin was piped");
        let _ = stdin.write_all(input);
        let _ = stdin.flush();
    }

    let out = child.wait_with_output().expect("failed to wait for child");
    Outcome {
        stdout: Vec::new(), // went to the broken pipe, not observable
        stderr: out.stderr,
        status: match out.status.code() {
            Some(c) => Ok(c),
            None => Err(out.status.signal().expect("no exit code and no signal")),
        },
    }
}

// ---------------------------------------------------------------------------
// Assertions
// ---------------------------------------------------------------------------

/// Asserts the C and Rust programs agree on stdout, stderr and exit status.
#[track_caller]
fn assert_same(label: &str, input: &[u8]) {
    let c = c_binary();
    let expected = run(&c, input);

    let actual = run(&rust_binary(), input);
    assert_eq!(
        expected,
        actual,
        "\nmismatch for {label}\n  input: {:?}\n  C   : {:?}\n  Rust: {:?}\n",
        String::from_utf8_lossy(input),
        expected,
        actual
    );

    if let Some(rel) = release_binary() {
        let actual = run(&rel, input);
        assert_eq!(
            expected,
            actual,
            "\nmismatch for {label} (release binary)\n  input: {:?}\n  C   : {:?}\n  Rust: {:?}\n",
            String::from_utf8_lossy(input),
            expected,
            actual
        );
    }
}

#[track_caller]
fn assert_same_str(label: &str, input: &str) {
    assert_same(label, input.as_bytes());
}

// ---------------------------------------------------------------------------
// Phase B — the inputs the C program branches on
//
// `main` is:      int x = 0; scanf("%d", &x); driver(x); return 0;
// `driver` is:    y = 2*x; y += 300; printf("%d\n", y);
//
// The only branch is inside `scanf("%d", &x)`: on input failure or matching
// failure it stores nothing and `x` keeps its initializer of 0. Every input
// class below is one arm of that conversion.
// ---------------------------------------------------------------------------

/// No input at all: `scanf` hits EOF before converting, `x` stays 0.
#[test]
fn empty_input() {
    assert_same_str("empty input", "");
}

/// stdin at immediate EOF via an empty pipe is the same class, but exercised
/// through a file-backed empty stdin as well.
#[test]
fn empty_input_from_devnull() {
    let c = c_binary();
    let expected = {
        let out = Command::new(&c)
            .stdin(Stdio::null())
            .output()
            .expect("run C with /dev/null stdin");
        (out.stdout, out.stderr, out.status.code())
    };
    let actual = {
        let out = Command::new(rust_binary())
            .stdin(Stdio::null())
            .output()
            .expect("run Rust with /dev/null stdin");
        (out.stdout, out.stderr, out.status.code())
    };
    assert_eq!(expected, actual, "mismatch with stdin = /dev/null");
}

/// A single item, with and without a trailing newline.
#[test]
fn single_value() {
    assert_same_str("single value, no newline", "5");
    assert_same_str("single value, trailing newline", "5\n");
    assert_same_str("zero", "0");
    assert_same_str("one", "1");
    assert_same_str("small positive", "100");
}

/// `%d` accepts an optional sign.
#[test]
fn signs() {
    assert_same_str("explicit plus", "+7");
    assert_same_str("negative", "-7");
    assert_same_str("negative small", "-1");
    assert_same_str("negative two", "-2");
    assert_same_str("negative hundred", "-100");
    // -150 makes 2*x + 300 exactly zero.
    assert_same_str("negative one fifty", "-150");
    assert_same_str("negative zero", "-0");
    assert_same_str("plus zero", "+0");
}

/// `%d` skips leading whitespace, and `scanf` reads across newlines — unlike
/// `fgets`. A number on the third line is still found.
#[test]
fn scanf_reads_across_newlines() {
    assert_same_str("newlines before value", "\n\n\n5");
    assert_same_str("mixed leading whitespace", "  \t\n  7\n");
    assert_same_str("carriage return", "\r\n5");
    assert_same_str("vertical tab and form feed", "\x0b\x0c5");
    assert_same_str("tab separated", "\t\t42\t");
    assert_same_str("newline between sign and digits", "-\n5");
}

/// Whitespace only: `scanf` consumes it, then hits EOF. Input failure, `x` = 0.
#[test]
fn whitespace_only_input() {
    assert_same_str("spaces only", "   ");
    assert_same_str("spaces and newlines only", "   \n\t ");
    assert_same_str("single newline", "\n");
    assert_same_str("all whitespace kinds", " \t\n\x0b\x0c\r");
}

/// Only the first conversion happens; the rest of stdin is ignored.
#[test]
fn trailing_input_is_ignored() {
    assert_same_str("two numbers", "5 10");
    assert_same_str("number then text", "5 hello world");
    assert_same_str("number then many lines", "5\n6\n7\n8\n");
    // `%d` stops at the '.', so this reads 1, not 1.5.
    assert_same_str("float-looking", "1.5");
    // Stops at 'e'.
    assert_same_str("exponent-looking", "5e3");
    // Stops at 'x', having read the leading 0 — so this is 0, not 16.
    assert_same_str("hex-looking", "0x10");
    assert_same_str("digits then letters", "12abc");
}

// ---------------------------------------------------------------------------
// Error paths: every input that makes the conversion fail leaves x == 0.
// ---------------------------------------------------------------------------

/// Matching failure: the first non-whitespace byte cannot start an integer.
#[test]
fn matching_failure_leaves_x_zero() {
    assert_same_str("letters", "abc");
    assert_same_str("underscore", "_5");
    assert_same_str("lone dot", ".");
    assert_same_str("dot then digits", ".5");
    assert_same_str("comma", ",5");
    assert_same_str("slash", "/5");
    assert_same_str("colon", ":5");
    assert_same_str("tilde", "~");
    assert_same_str("hash", "#5");
    assert_same_str("open paren", "(5)");
    assert_same_str("asterisk", "*5");
}

/// A sign that is not followed by a digit is also a matching failure.
#[test]
fn sign_without_digits() {
    assert_same_str("minus then EOF", "-");
    assert_same_str("plus then EOF", "+");
    assert_same_str("minus then letter", "-a");
    assert_same_str("plus then letter", "+a");
    assert_same_str("double minus", "--5");
    assert_same_str("double plus", "++5");
    assert_same_str("minus plus", "-+5");
    assert_same_str("minus space digit", "- 5");
    assert_same_str("minus dot", "-.");
    assert_same_str("whitespace then minus then EOF", "   -");
}

/// Non-text bytes: a NUL is not whitespace and not a digit, so it is a
/// matching failure even though a digit follows it.
#[test]
fn non_text_bytes() {
    assert_same("NUL then digit", b"\x005");
    assert_same("NUL only", b"\x00");
    assert_same("high byte then digit", b"\xff5");
    assert_same("invalid UTF-8", b"\x80\xfe\xfd");
    assert_same("digit then NUL", b"5\x00");
    assert_same("space then NUL then digit", b" \x007");
    assert_same("all byte values", &(0u8..=255).collect::<Vec<u8>>());
}

// ---------------------------------------------------------------------------
// Arithmetic: `y = 2*x; y += 300` on 32-bit int, and `%d` storing through an
// `int *`. Both overflow points are exercised.
// ---------------------------------------------------------------------------

/// `2*x + 300` overflowing 32-bit int.
#[test]
fn int_arithmetic_overflow_in_driver() {
    assert_same_str("INT_MAX", "2147483647");
    assert_same_str("INT_MAX - 1", "2147483646");
    assert_same_str("INT_MIN", "-2147483648");
    // 2*x overflows exactly at 2^30.
    assert_same_str("2^30", "1073741824");
    assert_same_str("2^30 - 1", "1073741823");
    assert_same_str("-2^30", "-1073741824");
    assert_same_str("-2^30 - 1", "-1073741825");
    // The largest x for which 2*x + 300 does not overflow, and the first that does.
    assert_same_str("1073741673", "1073741673");
    assert_same_str("1073741674", "1073741674");
    assert_same_str("1073741675", "1073741675");
}

/// Values that fit a 64-bit `long` but not an `int`: `scanf` converts with
/// `strtol` and stores through an `int *`, truncating to 32 bits.
#[test]
fn scanf_truncates_to_int() {
    assert_same_str("INT_MAX + 1", "2147483648");
    assert_same_str("INT_MIN - 1", "-2147483649");
    assert_same_str("2^32", "4294967296");
    assert_same_str("2^32 + 1", "4294967297");
    assert_same_str("2^32 - 1", "4294967295");
    assert_same_str("-(2^32)", "-4294967296");
    assert_same_str("-(2^32 - 1)", "-4294967295");
    assert_same_str("10^10", "10000000000");
    assert_same_str("-10^10", "-10000000000");
    assert_same_str("6442450941", "6442450941");
}

/// Beyond `long`: `strtol` saturates at `LONG_MAX`/`LONG_MIN`, and the
/// saturated value is then truncated to `int`.
#[test]
fn scanf_saturates_beyond_long() {
    assert_same_str("LONG_MAX", "9223372036854775807");
    assert_same_str("LONG_MAX - 1", "9223372036854775806");
    assert_same_str("LONG_MAX + 1", "9223372036854775808");
    assert_same_str("LONG_MIN", "-9223372036854775808");
    assert_same_str("LONG_MIN - 1", "-9223372036854775809");
    assert_same_str("twenty nines", "99999999999999999999");
    assert_same_str("negative twenty nines", "-99999999999999999999");
    assert_same_str("2^64", "18446744073709551616");
    assert_same_str("beyond 2^64", "922337203685477580800");
    assert_same_str("fifty nines", &"9".repeat(50));
    assert_same_str("negative fifty nines", &format!("-{}", "9".repeat(50)));
    assert_same_str("12345678901234567890", "12345678901234567890");
    assert_same_str("-12345678901234567890", "-12345678901234567890");
    // Leading zeros do not count toward magnitude.
    assert_same_str("zero padded LONG_MAX + 1", "0000000000009223372036854775808");
    assert_same_str("zero padded LONG_MIN - 1", "-0000000000009223372036854775809");
}

/// Leading zeros are consumed as part of the digit run.
#[test]
fn leading_zeros() {
    assert_same_str("padded five", "000000005");
    assert_same_str("many zeros", "0000000000000000000000000");
    assert_same_str("negative padded", "-000000007");
    assert_same_str("plus padded", "+000000007");
}

/// The maximum the code handles at the input end: digit runs and whitespace
/// runs far longer than any internal buffer.
#[test]
fn very_long_input() {
    assert_same_str("100k digits", &"9".repeat(100_000));
    assert_same_str("100k leading zeros then 7", &format!("{}7", "0".repeat(100_000)));
    assert_same_str("100k spaces then 7", &format!("{}7", " ".repeat(100_000)));
    assert_same_str("100k newlines then 7", &format!("{}7", "\n".repeat(100_000)));
    assert_same_str("value then 100k trailing bytes", &format!("42{}", "x".repeat(100_000)));
    assert_same_str("100k letters", &"a".repeat(100_000));
}

// ---------------------------------------------------------------------------
// Phase C — paths not reached by feeding stdin alone
// ---------------------------------------------------------------------------

/// stdout is a pipe with no reader. The C program has the default `SIGPIPE`
/// disposition and is killed by signal 13; the Rust program must not silently
/// exit 0 because its runtime ignored `SIGPIPE`.
#[test]
fn broken_stdout_pipe_matches() {
    let expected = run_with_closed_stdout(&c_binary(), b"5\n");
    let actual = run_with_closed_stdout(&rust_binary(), b"5\n");
    assert_eq!(
        expected, actual,
        "\nmismatch with a closed stdout pipe\n  C   : {expected:?}\n  Rust: {actual:?}\n"
    );

    if let Some(rel) = release_binary() {
        let actual = run_with_closed_stdout(&rel, b"5\n");
        assert_eq!(
            expected, actual,
            "\nmismatch with a closed stdout pipe (release binary)\n  C   : {expected:?}\n  Rust: {actual:?}\n"
        );
    }
}

/// stdin is a directory, so `read` fails with EISDIR rather than reaching EOF.
/// `scanf` reports input failure and `x` keeps its initializer.
#[test]
fn unreadable_stdin_matches() {
    let dir = std::fs::File::open(repo_root()).expect("open workspace root as a file");
    let dir2 = std::fs::File::open(repo_root()).expect("open workspace root as a file");

    let c_out = Command::new(c_binary())
        .stdin(Stdio::from(dir))
        .output()
        .expect("run C with a directory as stdin");
    let r_out = Command::new(rust_binary())
        .stdin(Stdio::from(dir2))
        .output()
        .expect("run Rust with a directory as stdin");

    assert_eq!(
        (c_out.stdout, c_out.stderr, c_out.status.code()),
        (r_out.stdout, r_out.stderr, r_out.status.code()),
        "mismatch with an unreadable stdin"
    );
}

/// Neither program should ever write to stderr on any of the input classes.
#[test]
fn stderr_is_always_empty() {
    for input in ["", "5", "abc", "-", &"9".repeat(40), "\x00"] {
        let c = run(&c_binary(), input.as_bytes());
        assert!(
            c.stderr.is_empty(),
            "C wrote to stderr for {input:?}: {:?}",
            String::from_utf8_lossy(&c.stderr)
        );
        let r = run(&rust_binary(), input.as_bytes());
        assert_eq!(c.stderr, r.stderr, "stderr differs for {input:?}");
    }
}

/// Output is always exactly one decimal integer and one trailing newline, with
/// no padding and no separators — matching `printf("%d\n", y)`.
#[test]
fn output_format_is_exactly_one_line() {
    for input in ["", "5", "-7", "2147483647", &"9".repeat(30)] {
        let out = run(&rust_binary(), input.as_bytes()).stdout;
        let s = String::from_utf8(out).expect("output must be ASCII digits and a newline");
        assert!(s.ends_with('\n'), "missing trailing newline for {input:?}: {s:?}");
        assert_eq!(s.matches('\n').count(), 1, "expected exactly one line for {input:?}");
        let body = s.trim_end_matches('\n');
        assert!(
            body.strip_prefix('-').unwrap_or(body).bytes().all(|b| b.is_ascii_digit()),
            "unexpected characters in output for {input:?}: {body:?}"
        );
        assert!(!body.is_empty(), "empty number for {input:?}");
    }
}

// ---------------------------------------------------------------------------
// Randomized differential sweep over the alphabet `%d` actually branches on.
// Deterministic: a fixed seed and a fixed generator, so a failure reproduces.
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn next_u32(&mut self) -> u32 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 32) as u32
    }

    fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
}

#[test]
fn randomized_differential_sweep() {
    // Bytes that reach the interesting arms of the conversion.
    const ALPHABET: &[u8] = b"0123456789+-. \t\n\r\x0b\x0cabxXeE_\x00\xff";

    let c = c_binary();
    let r = rust_binary();
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);

    for case in 0..400u32 {
        let len = rng.below(14) as usize;
        let input: Vec<u8> = (0..len)
            .map(|_| ALPHABET[rng.below(ALPHABET.len() as u32) as usize])
            .collect();

        let expected = run(&c, &input);
        let actual = run(&r, &input);
        assert_eq!(
            expected, actual,
            "\nrandomized case {case} differs\n  input: {input:?}\n  C   : {expected:?}\n  Rust: {actual:?}\n"
        );
    }
}

/// Randomized sweep over numeric magnitudes, including values that overflow
/// `int` in `driver`, overflow `int` in the `scanf` store, and overflow `long`.
#[test]
fn randomized_numeric_sweep() {
    let c = c_binary();
    let r = rust_binary();
    let mut rng = Rng(0xDEAD_BEEF_CAFE_F00D);

    for case in 0..400u32 {
        let digits = 1 + rng.below(25) as usize;
        let mut s = String::new();
        match rng.below(3) {
            0 => s.push('-'),
            1 => s.push('+'),
            _ => {}
        }
        for _ in 0..digits {
            s.push((b'0' + rng.below(10) as u8) as char);
        }

        let expected = run(&c, s.as_bytes());
        let actual = run(&r, s.as_bytes());
        assert_eq!(
            expected, actual,
            "\nnumeric case {case} differs\n  input: {s:?}\n  C   : {expected:?}\n  Rust: {actual:?}\n"
        );
    }
}

/// Exhaustive sweep over every small magnitude, both signs, so the wrap point
/// of `2*x + 300` is covered without relying on chance.
#[test]
fn exhaustive_small_values() {
    let c = c_binary();
    let r = rust_binary();

    let mut values: Vec<i64> = (-300..=300).collect();
    // Plus the neighborhoods of each overflow boundary.
    for base in [
        1_073_741_824i64,   // 2*x overflows
        1_073_741_674,      // 2*x + 300 overflows
        2_147_483_647,      // INT_MAX
        -2_147_483_648,     // INT_MIN
        4_294_967_296,      // 2^32
        9_223_372_036_854_775_807, // LONG_MAX
    ] {
        for d in -2..=2i64 {
            values.push(base.saturating_add(d));
        }
    }

    for v in values {
        let s = v.to_string();
        let expected = run(&c, s.as_bytes());
        let actual = run(&r, s.as_bytes());
        assert_eq!(
            expected, actual,
            "\nvalue {v} differs\n  C   : {expected:?}\n  Rust: {actual:?}\n"
        );
    }
}
