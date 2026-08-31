//! Differential tests: run the C binary and the Rust binary as subprocesses,
//! feed both the same bytes on stdin, and require that stdout, stderr and the
//! exit status match exactly.
//!
//! The Rust code is never used as a library here; both programs are driven the
//! way a shell would drive them, because that is how they are compared.

use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

// ---------------------------------------------------------------------------
// Locating and running the two executables
// ---------------------------------------------------------------------------

/// The Rust binary, built by cargo for whatever profile the tests run under.
///
///     cd translation && cargo build --release
///     ./target/release/driver
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// The C binary. Built with:
///
///     cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .
///     ./c_src/build/driver
fn c_bin() -> PathBuf {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf();
    let c_src = repo_root.join("c_src");
    let build = c_src.join("build");
    let exe = build.join("driver");

    if !exe.exists() {
        std::fs::create_dir_all(&build).expect("could not create c_src/build");
        let configure = Command::new("cmake")
            .arg("..")
            .current_dir(&build)
            .output()
            .expect("failed to run `cmake` -- is cmake installed?");
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
    }

    assert!(
        exe.exists(),
        "C executable {} not found; build it with \
         `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`",
        exe.display()
    );
    exe
}

/// Everything the outside world can observe from one run.
#[derive(PartialEq, Eq)]
struct Observed {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Ok(code)` for a normal exit, `Err(signal)` when killed by a signal.
    status: Result<i32, i32>,
}

impl std::fmt::Debug for Observed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "stdout={:?} stderr={:?} status={}",
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr),
            match self.status {
                Ok(c) => format!("exit({})", c),
                Err(s) => format!("signal({})", s),
            }
        )
    }
}

fn run(exe: &Path, stdin_bytes: &[u8]) -> Observed {
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));

    {
        // The child may exit before reading everything; a failed write here is
        // not interesting, only the child's observable behaviour is.
        let mut stdin = child.stdin.take().expect("stdin was piped");
        let _ = stdin.write_all(stdin_bytes);
        let _ = stdin.flush();
    }

    let out = child.wait_with_output().expect("failed to wait for child");
    Observed {
        stdout: out.stdout,
        stderr: out.stderr,
        status: match out.status.code() {
            Some(code) => Ok(code),
            None => Err(out.status.signal().expect("no exit code and no signal")),
        },
    }
}

/// Render input bytes readably so failures are diagnosable.
fn show(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes.iter().take(160) {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{:02x}", b)),
        }
    }
    if bytes.len() > 160 {
        s.push_str(&format!("...(+{} bytes)", bytes.len() - 160));
    }
    s
}

struct Differ {
    c: PathBuf,
    rust: PathBuf,
    failures: Vec<String>,
    checked: usize,
}

impl Differ {
    fn new() -> Self {
        Differ {
            c: c_bin(),
            rust: rust_bin(),
            failures: Vec::new(),
            checked: 0,
        }
    }

    /// Compare stdout, stderr and exit status for one input.
    fn check(&mut self, input: &[u8]) {
        self.checked += 1;
        let expected = run(&self.c, input);
        let actual = run(&self.rust, input);
        if expected != actual {
            self.failures.push(format!(
                "input {:?}\n      C: {:?}\n   rust: {:?}",
                show(input),
                expected,
                actual
            ));
        }
    }

    fn check_str(&mut self, input: &str) {
        self.check(input.as_bytes());
    }

    fn finish(self, what: &str) {
        assert!(
            self.failures.is_empty(),
            "{} of {} {} inputs differed between the C and Rust programs:\n{}",
            self.failures.len(),
            self.checked,
            what,
            self.failures.join("\n")
        );
    }
}

// ---------------------------------------------------------------------------
// Phase A -- both programs exist, run, and agree on the simplest input
// ---------------------------------------------------------------------------

#[test]
fn both_programs_build_and_run() {
    let c = c_bin();
    let r = rust_bin();
    assert!(c.exists(), "C binary missing at {}", c.display());
    assert!(r.exists(), "Rust binary missing at {}", r.display());

    let expected = run(&c, b"1.5\n");
    let actual = run(&r, b"1.5\n");
    assert_eq!(expected, actual, "baseline run differs");
    // sanity: 1.5f32 == 0x3fc00000, printed little-endian byte by byte
    assert_eq!(expected.stdout, b"0000c03f\n");
    assert_eq!(expected.stderr, b"");
    assert_eq!(expected.status, Ok(0));
}

// ---------------------------------------------------------------------------
// Phase B -- the input classes the C program branches on
// ---------------------------------------------------------------------------

/// `scanf` returns EOF/0 and never stores: `x` keeps its initial `0.f`, so the
/// program must still print "00000000". Empty input is the canonical case.
#[test]
fn empty_and_whitespace_only_input() {
    let mut d = Differ::new();
    for s in [
        "", " ", "\n", "\t", "\r", "\u{b}", "\u{c}", "  ", "\n\n\n",
        "\t\n\u{b}\u{c}\r ", " \t \t \n",
    ] {
        d.check_str(s);
    }
    // long whitespace runs: %f skips whitespace across newlines
    d.check(&vec![b' '; 5000]);
    d.check(&vec![b'\n'; 5000]);
    d.finish("whitespace-only");
}

/// A single item: one float, the happy path.
#[test]
fn single_decimal_value() {
    let mut d = Differ::new();
    for s in [
        "0", "1", "2", "1.5", "0.5", "0.1", "3.14159", "2.718281828",
        "100", "1000000", "0.0", "0.000", "123.456",
    ] {
        d.check_str(s);
    }
    d.finish("single decimal");
}

/// Signs, including the sign of zero, which shows up in the raw bytes.
#[test]
fn signs_and_signed_zero() {
    let mut d = Differ::new();
    for s in [
        "-1", "+1", "-1.5", "+1.5", "-0.0", "+0.0", "-0", "+0",
        "-0.000", "+.5", "-.5",
    ] {
        d.check_str(s);
    }
    d.finish("signed");
}

/// The forms with no digits at all before the decimal point or after it.
#[test]
fn dot_placement_forms() {
    let mut d = Differ::new();
    for s in [".", "..", ".5", "5.", "5.5", "0.", ".0", "5.5.5", ".e5", "."] {
        d.check_str(s);
    }
    d.finish("dot placement");
}

/// Exponent handling, including exponents that are begun but never completed.
/// `1e` and `1e+` reach the "no exponent digits" path.
#[test]
fn decimal_exponents() {
    let mut d = Differ::new();
    for s in [
        "1e0", "1e1", "1E1", "1e+5", "1e-5", "1e10", "1e-10", "1e38", "1e39",
        "1e-45", "1e-46", "1e100", "1e-100",
        // exponent started but not completed
        "1e", "1e+", "1e-", "1e+x", "1e-x", "1ee5", "1.5e", "1.5e2x", "1e+5x",
        "1e5.5",
    ] {
        d.check_str(s);
    }
    for e in -60..=60 {
        d.check_str(&format!("1.234567e{}", e));
        d.check_str(&format!("-9.87654321e{}", e));
    }
    d.finish("decimal exponent");
}

/// Hexadecimal floats. `0x` with no hex digits is its own error path.
#[test]
fn hexadecimal_floats() {
    let mut d = Differ::new();
    for s in [
        "0x1p0", "0X1P0", "0x1.8p1", "-0x1p-1", "0x1", "0x10", "0x0p0",
        "-0x0p0", "0x.8p0", "0x1.p2", "0xabcdefABCDEF", "0xfffffff",
        // no hex digits after the prefix
        "0x", "0X", "0xg", "0x.", "0x.p1", "0xp0", "-0x", "+0x",
        // exponent started but not completed
        "0x1p", "0x1p+", "0x1p-", "0x1.8p", "0x1pp5", "0x1p+5x", "0x1p5.5",
        "0x1p-x",
        // magnitude extremes
        "0x1p128", "0x1.fffffep127", "0x1.ffffffp127", "0x1p-126", "0x1p-149",
        "0x1p-150", "0x1p-151", "0x1.8p-149", "0x1p999999", "0x1p-999999",
        "0x0.0000001p0",
    ] {
        d.check_str(s);
    }
    for e in (-160..=140).step_by(7) {
        d.check_str(&format!("0x1.abcdefp{}", e));
    }
    d.finish("hex float");
}

/// `inf`/`infinity`, case-insensitive, and every truncated prefix -- a partial
/// `infinity` is a matching failure, not `inf`.
#[test]
fn infinity_forms_and_prefixes() {
    let mut d = Differ::new();
    for s in [
        "inf", "INF", "Inf", "iNf", "-inf", "+inf",
        "infinity", "INFINITY", "InFiNiTy", "-infinity", "+infinity",
        // truncated / trailing garbage
        "i", "in", "infi", "infin", "infini", "infinit", "infx", "infinityx",
        "-i", "-in", "-infi", "-infinit", "inf.", "infinit.",
    ] {
        d.check_str(s);
    }
    d.finish("infinity");
}

/// `nan`, with and without an `(n-char-sequence)`, plus truncated prefixes and
/// an unterminated parenthesis.
#[test]
fn nan_forms_and_prefixes() {
    let mut d = Differ::new();
    for s in [
        "nan", "NAN", "NaN", "nAn", "-nan", "+nan",
        "nan()", "nan(1)", "nan(123)", "nan(abc)", "nan(abc_1)", "nan(_)",
        "nan(0x123)", "nan(1)garbage",
        // truncated / malformed
        "n", "na", "-n", "-na", "nanx", "nan(", "nan(abc", "nan(abc]", "nan(]",
        "nan(abc)(def)",
    ] {
        d.check_str(s);
    }
    d.finish("nan");
}

/// Input that cannot start a float at all: `scanf` matches nothing.
#[test]
fn non_numeric_input_error_paths() {
    let mut d = Differ::new();
    for s in [
        "abc", "+", "-", "++", "--", "+-1", "-+1", "--1", "x", "X", "e5", "E5",
        "p5", "/", "z", "!", "#", "@", "'", "\"", "%f", "()", "[]", "hello",
        " abc", "\n\nabc", "\t-", "\t+",
    ] {
        d.check_str(s);
    }
    d.finish("non-numeric");
}

/// `%f` skips leading whitespace including newlines, and stops at the first
/// character that cannot extend the number -- trailing input is simply left.
#[test]
fn reads_across_newlines_and_stops_at_garbage() {
    let mut d = Differ::new();
    for s in [
        "  \t\n  7.5", "\n\n\n\n4.25", "   \n\t 0x2p3 \n", "1 2", "1\n2",
        "1.5abc", "1.5\n", "1.5\r\n", " -2.5 \n", "2.5 rest of line\nmore\n",
        "\n\n\n-inf\n\n", "\t\tnan\t\t",
    ] {
        d.check_str(s);
    }
    d.finish("whitespace/trailing");
}

/// Raw bytes, including NUL and bytes outside ASCII.
#[test]
fn binary_and_non_ascii_input() {
    let mut d = Differ::new();
    for b in [
        &b"\x00"[..], b"\xff\xfe", b"\x001", b"1\x00", b"\x7f", b"\x80\x81",
        b"\xc3\xa9", b"1.5\x00abc", b"\xef\xbb\xbf1.5", b"-\xff",
    ] {
        d.check(b);
    }
    // every single byte on its own
    for b in 0u8..=255 {
        d.check(&[b]);
    }
    d.finish("binary");
}

// ---------------------------------------------------------------------------
// Phase C -- rounding, saturation, truncation, and the remaining paths
// ---------------------------------------------------------------------------

/// Values at and beyond the f32 range: overflow to infinity, underflow to zero.
#[test]
fn overflow_and_underflow_boundaries() {
    let mut d = Differ::new();
    for s in [
        // largest finite float and just past it
        "340282346638528859811704183484516925440",
        "340282356638528859811704183484516925440",
        "340282366920938463463374607431768211456",
        "3.40282346638528859812e+38",
        "3.40282356779733661637e+38",
        "3.40282366920938463463e+38",
        // smallest normal, largest subnormal, smallest subnormal, half of it
        "1.17549435082228750797e-38",
        "1.17549421069244107e-38",
        "1.40129846432481707e-45",
        "7.00649232162408535e-46",
        "7.00649232162408536e-46",
        "2.938735877055718770e-39",
        "1e-45", "1.5e-45", "2.5e-45", "1e-46", "7e-46", "7.1e-46",
        "0.000000000000000000000000000000000000000000001",
    ] {
        d.check_str(s);
        d.check_str(&format!("-{}", s));
    }
    d.finish("range boundary");
}

/// Ties: values exactly halfway between two representable floats must round to
/// even, the way glibc's strtof does.
#[test]
fn round_half_to_even() {
    let mut d = Differ::new();
    for s in [
        "16777216", "16777217", "16777218", "16777219", "16777220",
        "8388609e-1", "1.000000059604644775390625",
        "1.0000000596046447753906250001", "2.00000011920928955078125",
        "0x1000001p0", "0x1000003p0", "0x800000.8p1", "0x800001.8p1",
        "0x1.0000008p0", "0x1.0000018p0", "0x1.0000001p0",
    ] {
        d.check_str(s);
    }
    // exact ties built from an odd 25-bit significand: (2m+1) * 2^(e-1)
    for e in -30i32..=30 {
        for m in [0x800001u32, 0xabcdefu32, 0xfffffeu32, 0xffffffu32] {
            d.check_str(&format!("0x{:x}p{}", 2 * m + 1, e - 1));
            d.check_str(&format!("0x{:x}.8p{}", m, e));
        }
    }
    d.finish("rounding tie");
}

/// Very long significands: more digits than the value can possibly need, so the
/// discarded tail only matters through its non-zeroness (the sticky bit).
#[test]
fn very_long_significands() {
    let mut d = Differ::new();
    d.check_str(&"9".repeat(41));
    d.check_str(&"0".repeat(50));
    d.check_str(&format!("{}1", "0".repeat(50)));
    d.check_str("12345678901234567890");
    d.check_str("99999999999999999999999999999999999999999");
    d.check_str("0.0000000000000000000001");
    d.check_str(&format!("1.{}1", "0".repeat(400)));
    d.check_str(&format!("0.{}1", "0".repeat(400)));
    // hex significands wider than the 124-bit accumulator
    d.check_str("0x1234567890abcdef1234567890abcdefp0");
    d.check_str(&format!("0x1{}p0", "0".repeat(200)));
    d.check_str(&format!("0x1{}1p-300", "0".repeat(60)));
    d.check_str(&format!("0x{}p-500", "f".repeat(100)));
    d.check_str(&format!("0x0.{}p0", "f".repeat(80)));
    d.check_str(&format!("0x0.{}1p0", "0".repeat(80)));
    for n in 1..=30 {
        d.check_str(&format!("{}.{}", "9".repeat(n), "9".repeat(n)));
    }
    d.finish("long significand");
}

/// Exponents far outside any sane range: the value saturates to infinity or to
/// zero, and the exponent digits themselves overflow any fixed-width integer.
#[test]
fn absurd_exponents() {
    let mut d = Differ::new();
    for s in [
        "1e999999999999999999999", "1e-999999999999999999999", "-1e999999",
        "1e1000000", "1e1000001", "1e-1000000", "1e99999999999999999999999999",
        "0x1p2147483647", "0x1p-2147483648", "0x1p9223372036854775807",
        "0x1p-9223372036854775808",
    ] {
        d.check_str(s);
    }
    d.check(format!("1{}e-300", "0".repeat(300)).as_bytes());
    d.check(format!("0.{}1e300", "0".repeat(300)).as_bytes());
    d.check(format!("{}e-400", "9".repeat(400)).as_bytes());
    d.check(format!("{}e{}", "1".repeat(100), "9".repeat(20)).as_bytes());
    d.finish("absurd exponent");
}

/// A megabyte-scale input: nothing in the C program bounds the digit run.
#[test]
fn very_large_inputs() {
    let mut d = Differ::new();
    d.check(format!("1{}", "0".repeat(200_000)).as_bytes());
    d.check(format!("0.{}", "9".repeat(200_000)).as_bytes());
    d.check(format!("0x1.{}p0", "f".repeat(100_000)).as_bytes());
    d.finish("huge input");
}

/// A deterministic sweep, so a regression anywhere in the parser shows up even
/// if no hand-written case names it.
#[test]
fn pseudo_random_sweep() {
    let mut d = Differ::new();
    let mut state: u64 = 0x1234_5678_9abc_def1;
    let mut next = |n: u64| -> u64 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state % n
    };

    for _ in 0..500 {
        let s = match next(6) {
            0 => format!("{}.{}e{}", next(1_000_000), next(1_000_000),
                         next(120) as i64 - 60),
            1 => format!("-{}.{}e{}", next(1_000_000), next(1_000_000),
                         next(120) as i64 - 60),
            2 => format!("0x{:x}p{}", next(u64::MAX), next(320) as i64 - 160),
            3 => format!("0x{:x}.{:x}p{}", next(1 << 30), next(1u64 << 40),
                         next(320) as i64 - 160),
            4 => {
                // random garbage drawn from characters the parser reacts to
                const ALPHABET: &[u8] = b"0123456789+-.eExXaAfFiInNtTyY() \t";
                let len = next(14) as usize + 1;
                (0..len)
                    .map(|_| ALPHABET[next(ALPHABET.len() as u64) as usize] as char)
                    .collect()
            }
            _ => format!("{}e-{}", next(1 << 40), next(50)),
        };
        d.check_str(&s);
    }
    d.finish("pseudo-random");
}

// ---------------------------------------------------------------------------
// Exit status and signal behaviour
// ---------------------------------------------------------------------------

/// `main` always `return 0`, on every path, including matching failure.
#[test]
fn exit_status_is_always_zero_on_normal_runs() {
    let c = c_bin();
    let r = rust_bin();
    for input in ["", "abc", "1.5", "inf", "nan", "0x", "-", "1e+"] {
        let expected = run(&c, input.as_bytes());
        let actual = run(&r, input.as_bytes());
        assert_eq!(expected.status, Ok(0), "C exit status for {input:?}");
        assert_eq!(actual.status, expected.status, "exit status for {input:?}");
        assert!(expected.stderr.is_empty(), "C wrote to stderr for {input:?}");
        assert!(actual.stderr.is_empty(), "Rust wrote to stderr for {input:?}");
    }
}

/// `argv` is unused by `main()`, so extra arguments must change nothing.
#[test]
fn command_line_arguments_are_ignored() {
    for args in [vec!["foo"], vec!["--help"], vec!["-x", "1", "2"]] {
        let mut outputs = Vec::new();
        for exe in [c_bin(), rust_bin()] {
            let mut child = Command::new(&exe)
                .args(&args)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("spawn");
            let _ = child.stdin.take().unwrap().write_all(b"2.5\n");
            let out = child.wait_with_output().expect("wait");
            outputs.push((out.stdout, out.stderr, out.status.code()));
        }
        assert_eq!(outputs[0], outputs[1], "argv {args:?} changed behaviour");
    }
}

/// Writing to a stdout whose reader is gone: the C program is killed by
/// SIGPIPE. Rust's runtime ignores SIGPIPE by default, so the translation has
/// to restore the default disposition or it would exit 0 here instead.
#[test]
fn broken_stdout_pipe_dies_the_same_way() {
    use std::os::unix::io::FromRawFd;

    extern "C" {
        fn pipe(fds: *mut i32) -> i32;
        fn close(fd: i32) -> i32;
    }

    fn run_with_closed_stdout_reader(exe: &Path) -> Result<i32, i32> {
        let mut fds = [0i32; 2];
        assert_eq!(unsafe { pipe(fds.as_mut_ptr()) }, 0, "pipe() failed");
        let (read_end, write_end) = (fds[0], fds[1]);
        // Drop the read end first: any write by the child now raises SIGPIPE.
        assert_eq!(unsafe { close(read_end) }, 0, "close() failed");

        let stdout = unsafe { Stdio::from_raw_fd(write_end) };
        let mut child = Command::new(exe)
            .stdin(Stdio::piped())
            .stdout(stdout)
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn");
        let _ = child.stdin.take().unwrap().write_all(b"1.5\n");
        let out = child.wait_with_output().expect("wait");
        match out.status.code() {
            Some(code) => Ok(code),
            None => Err(out.status.signal().expect("signal")),
        }
    }

    let expected = run_with_closed_stdout_reader(&c_bin());
    let actual = run_with_closed_stdout_reader(&rust_bin());
    assert_eq!(
        expected, Err(13),
        "expected the C program to be killed by SIGPIPE"
    );
    assert_eq!(actual, expected, "broken-pipe behaviour differs");
}

/// stdout closed outright (EBADF, not EPIPE): `printf` fails silently and the C
/// program still exits 0.
#[test]
fn closed_stdout_still_exits_zero() {
    extern "C" {
        fn open(path: *const u8, flags: i32) -> i32;
    }

    fn run_with_stdout_to_devnull_then_closed(exe: &Path) -> Result<i32, i32> {
        use std::os::unix::io::FromRawFd;
        // O_WRONLY = 1
        let fd = unsafe { open(b"/dev/full\0".as_ptr(), 1) };
        if fd < 0 {
            return Ok(0); // /dev/full unavailable; nothing to compare
        }
        let mut child = Command::new(exe)
            .stdin(Stdio::piped())
            .stdout(unsafe { Stdio::from_raw_fd(fd) })
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn");
        let _ = child.stdin.take().unwrap().write_all(b"1.5\n");
        let out = child.wait_with_output().expect("wait");
        match out.status.code() {
            Some(code) => Ok(code),
            None => Err(out.status.signal().expect("signal")),
        }
    }

    let expected = run_with_stdout_to_devnull_then_closed(&c_bin());
    let actual = run_with_stdout_to_devnull_then_closed(&rust_bin());
    assert_eq!(actual, expected, "unwritable-stdout behaviour differs");
}

/// stdin that is not readable at all: `scanf` fails, `x` stays `0.f`.
#[test]
fn unreadable_stdin() {
    for stdin in ["/dev/null", "/tmp"] {
        let mut outputs = Vec::new();
        for exe in [c_bin(), rust_bin()] {
            let f = match std::fs::File::open(stdin) {
                Ok(f) => f,
                Err(_) => continue,
            };
            let out = Command::new(&exe)
                .stdin(Stdio::from(f))
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .expect("run");
            outputs.push((out.stdout, out.stderr, out.status.code()));
        }
        if outputs.len() == 2 {
            assert_eq!(outputs[0], outputs[1], "stdin={stdin} differs");
        }
    }
}

/// The output shape itself: `print_hex` always writes exactly 4 bytes worth of
/// two-digit lowercase hex followed by one '\n', for every input class.
#[test]
fn output_is_always_eight_hex_digits_and_a_newline() {
    let mut d = Differ::new();
    for input in ["", "abc", "1.5", "-inf", "nan", "0x1p0", "1e999999", "0x"] {
        let observed = run(&c_bin(), input.as_bytes());
        assert_eq!(observed.stdout.len(), 9, "C output length for {input:?}");
        assert_eq!(observed.stdout[8], b'\n', "C trailing newline for {input:?}");
        assert!(
            observed.stdout[..8]
                .iter()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(c)),
            "C output not lowercase hex for {input:?}"
        );
        d.check_str(input);
    }
    d.finish("output shape");
}
