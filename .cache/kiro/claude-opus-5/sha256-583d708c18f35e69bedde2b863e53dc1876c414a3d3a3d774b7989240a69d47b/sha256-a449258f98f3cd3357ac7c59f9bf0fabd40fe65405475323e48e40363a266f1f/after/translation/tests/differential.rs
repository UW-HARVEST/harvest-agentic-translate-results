// Differential tests: run the C binary and the Rust binary as subprocesses on
// identical stdin and require byte-identical stdout, byte-identical stderr and
// an identical exit status (including termination by signal).
//
// The Rust program is never loaded as a library. It is driven exactly the way a
// shell drives it, because that is how it is compared against the C program.

use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Once;

/// Path to the Rust binary. Cargo builds the `driver` bin before running
/// integration tests and exposes its path through this environment variable.
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn c_src_dir() -> PathBuf {
    // tests/ lives in the crate root; c_src is its sibling.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate has a parent directory")
        .join("c_src")
}

static BUILD_C: Once = Once::new();

/// Path to the C binary, configuring and building it once per test process if
/// it is not already present.
fn c_bin() -> PathBuf {
    let src = c_src_dir();
    let build = src.join("build");
    let exe = build.join("driver");

    BUILD_C.call_once(|| {
        if exe.exists() {
            return;
        }
        std::fs::create_dir_all(&build).expect("create c_src/build");
        let cfg = Command::new("cmake")
            .arg("..")
            .current_dir(&build)
            .output()
            .expect("run cmake (is cmake installed?)");
        assert!(
            cfg.status.success(),
            "cmake configure failed:\n{}\n{}",
            String::from_utf8_lossy(&cfg.stdout),
            String::from_utf8_lossy(&cfg.stderr)
        );
        let bld = Command::new("cmake")
            .args(["--build", "."])
            .current_dir(&build)
            .output()
            .expect("run cmake --build");
        assert!(
            bld.status.success(),
            "cmake build failed:\n{}\n{}",
            String::from_utf8_lossy(&bld.stdout),
            String::from_utf8_lossy(&bld.stderr)
        );
    });

    assert!(exe.exists(), "C binary missing at {}", exe.display());
    exe
}

/// What a run of either program produced.
#[derive(PartialEq, Eq)]
struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Ok(code)` for a normal exit, `Err(signal)` when killed by a signal.
    status: Result<i32, i32>,
}

impl std::fmt::Debug for Run {
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

fn run_with(exe: &Path, input: &[u8]) -> Run {
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", exe.display()));

    {
        let stdin = child.stdin.as_mut().expect("stdin piped");
        // A short write followed by closing the pipe: both programs read at
        // most one integer, so they may exit before consuming everything.
        // A broken pipe here is expected and not a failure.
        let _ = stdin.write_all(input);
        let _ = stdin.flush();
    }
    drop(child.stdin.take());

    let out = child.wait_with_output().expect("wait for child");
    let status = match out.status.code() {
        Some(c) => Ok(c),
        None => Err(out.status.signal().expect("exited by signal")),
    };
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        status,
    }
}

/// Asserts the two programs agree on stdout, stderr and exit status.
fn assert_same(label: &str, input: &[u8]) {
    let c = run_with(&c_bin(), input);
    let r = run_with(&rust_bin(), input);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout differs for {label} (input {:?})\n  C   : {:?}\n  Rust: {:?}",
        Preview(input),
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr differs for {label} (input {:?})\n  C   : {:?}\n  Rust: {:?}",
        Preview(input),
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
    assert_eq!(
        c.status, r.status,
        "exit status differs for {label} (input {:?})\n  C   : {c:?}\n  Rust: {r:?}",
        Preview(input)
    );
}

/// Truncating, escaped rendering of an input for assertion messages.
struct Preview<'a>(&'a [u8]);

impl std::fmt::Debug for Preview<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let shown = &self.0[..self.0.len().min(64)];
        write!(f, "{}", String::from_utf8_lossy(shown).escape_debug())?;
        if self.0.len() > shown.len() {
            write!(f, "...({} bytes)", self.0.len())?;
        }
        Ok(())
    }
}

fn check_all(cases: &[&str]) {
    for s in cases {
        assert_same(&format!("[{}]", s.escape_debug()), s.as_bytes());
    }
}

// ---------------------------------------------------------------------------
// Phase A: both programs build and are runnable.
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_exist_and_run() {
    let c = c_bin();
    let r = rust_bin();
    assert!(c.is_file(), "C binary not built: {}", c.display());
    assert!(r.is_file(), "Rust binary not built: {}", r.display());
    // A trivial run must succeed for both, otherwise every later comparison
    // would be measuring nothing.
    assert_eq!(run_with(&c, b"1").status, Ok(0));
    assert_eq!(run_with(&r, b"1").status, Ok(0));
}

// ---------------------------------------------------------------------------
// Phase B: the input classes the C actually branches on.
// ---------------------------------------------------------------------------

/// `scanf` returns EOF with no conversion; `x` keeps its initializer 0, so the
/// program prints 2*0 + 300.
#[test]
fn empty_and_whitespace_only_input() {
    check_all(&[
        "", " ", "  ", "\n", "\n\n\n", "\t", "\r", "\r\n", "\u{b}", "\u{c}",
        "\t\u{b}\u{c}\r\n ", "                    ",
    ]);
}

#[test]
fn single_item_happy_path() {
    check_all(&["0", "1", "5", "42", "7\n", "  3", "\n\n   3", "5\r\n"]);
}

#[test]
fn signs_and_leading_zeros() {
    check_all(&[
        "+42", "-5", "-0", "+0", "0000000042", "-0000000042", "+0000000042",
        "000000000000000000000000000000005",
    ]);
}

/// `%d` stops at the first non-digit, so trailing junk is simply left unread.
#[test]
fn number_followed_by_trailing_junk() {
    check_all(&[
        "12abc", "0x10", "0X10", "0b1", "5.5", "5e3", "1_000", "9a", "5 6 7",
        "1 2", "4\n5", "1\n2\n3", "42,", "42/", "42:",
    ]);
}

/// Matching failure: no digit is ever seen, so `scanf` assigns nothing and `x`
/// stays 0. This is a distinct branch from the EOF case above.
#[test]
fn matching_failure_leaves_x_at_zero() {
    check_all(&[
        "abc", "-abc", "+abc", "-", "+", "--5", "++5", "- 5", "+ 5", ".5",
        "e5", "INF", "nan", ",5", "/5", ":5", "a9", "x", "-.5",
    ]);
}

/// `scanf` whitespace skipping crosses newlines, unlike `fgets`. The number on
/// the second line is therefore still consumed.
#[test]
fn scanf_reads_across_newlines() {
    check_all(&["\n\n\n8", "   \n   \t\n 9", "\n\n\n\n\n\n\n\n\n\n11"]);
}

#[test]
fn long_leading_whitespace_runs() {
    for (label, pad) in [("spaces", ' '), ("newlines", '\n'), ("tabs", '\t')] {
        for n in [64usize, 1024, 5000] {
            let mut s: String = std::iter::repeat(pad).take(n).collect();
            s.push_str("11");
            assert_same(&format!("{label}x{n}+11"), s.as_bytes());
            let ws: String = std::iter::repeat(pad).take(n).collect();
            assert_same(&format!("{label}x{n}-eof"), ws.as_bytes());
        }
    }
}

// ---------------------------------------------------------------------------
// Phase B/C: integer overflow, truncation and signedness exactly as C does it.
// ---------------------------------------------------------------------------

/// `2*x + 300` overflows `int` for large `x`; the compiled C wraps, and the
/// Rust translation must wrap identically rather than panic.
#[test]
fn arithmetic_overflow_in_driver() {
    check_all(&[
            "2147483647",  // INT_MAX
            "2147483646",
            "-2147483648", // INT_MIN
            "-2147483647",
            "1073741824",  // 2*x is exactly INT_MAX+1
            "1073741823",
            "1073741825",
            "-1073741824",
            "-1073741825",
            "1073741974",  // 2*x + 300 lands exactly on the wrap boundary
            "-150",        // 2*x + 300 == 0
            "-151",
            "-149",
    ]);
}

/// Values outside `int` range: glibc's `%d` conversion saturates at `long` and
/// the result is then narrowed to `int`.
#[test]
fn values_exceeding_int_range() {
    check_all(&[
        "2147483648",
        "2147483649",
        "-2147483649",
        "-2147483650",
        "4294967295",
        "4294967296",
        "4294967297",
        "-4294967296",
        "-4294967297",
        "6442450944",
        "8589934592",
        "12884901888",
        "10000000000000000000",
    ]);
}

/// Values at and beyond `long` range, where the conversion saturates.
#[test]
fn values_exceeding_long_range() {
    check_all(&[
        "9223372036854775806",
        "9223372036854775807", // LONG_MAX
        "9223372036854775808", // LONG_MAX + 1
        "9223372036854775809",
        "-9223372036854775807",
        "-9223372036854775808", // LONG_MIN
        "-9223372036854775809",
        "18446744073709551615", // ULONG_MAX
        "18446744073709551616",
        "99999999999999999999",
        "-99999999999999999999",
        "99999999999999999999999999999999999999",
    ]);
}

/// Digit runs long enough to cross any internal buffering boundary.
#[test]
fn very_long_digit_runs() {
    for n in [19usize, 20, 21, 63, 64, 65, 127, 128, 129, 1000, 4096, 4097] {
        let nines: String = std::iter::repeat('9').take(n).collect();
        assert_same(&format!("9x{n}"), nines.as_bytes());
        assert_same(&format!("-9x{n}"), format!("-{nines}").as_bytes());
        let zeros: String = std::iter::repeat('0').take(n).collect();
        assert_same(&format!("0x{n}+7"), format!("{zeros}7").as_bytes());
    }
}

// ---------------------------------------------------------------------------
// Phase C: input classes not reached above.
// ---------------------------------------------------------------------------

/// Non-ASCII and control bytes, including embedded NULs, which are neither
/// whitespace nor digits and so end (or prevent) the conversion.
#[test]
fn non_ascii_and_control_bytes() {
    let cases: &[(&str, &[u8])] = &[
        ("nul-only", b"\x00"),
        ("nul-first", b"\x005"),
        ("nul-after-digit", b"5\x006"),
        ("nul-mid-whitespace", b"  \x00 5"),
        ("high-bytes", b"\xff\xfe9"),
        ("del-byte", b"\x7f3"),
        ("utf8-digit-lookalike", "５".as_bytes()),
        ("utf8-minus-sign", "−5".as_bytes()),
        ("bom-then-number", b"\xef\xbb\xbf5"),
        ("esc-then-number", b"\x1b5"),
        ("bell-then-number", b"\x075"),
        ("digit-then-nul-run", b"7\x00\x00\x00"),
        ("all-high-bytes", b"\x80\x81\x82\x83"),
    ];
    for (label, bytes) in cases {
        assert_same(label, bytes);
    }
}

/// Every small value, so the arithmetic and the `%d` formatting (including the
/// minus sign and the trailing newline) are compared over a dense range.
#[test]
fn dense_sweep_of_small_values() {
    for v in -400i32..=400 {
        assert_same(&format!("sweep {v}"), v.to_string().as_bytes());
    }
}

/// Deterministic pseudo-random 64-bit sweep, to catch narrowing or saturation
/// differences the hand-picked boundaries might miss.
#[test]
fn pseudo_random_wide_value_sweep() {
    // xorshift64* with a fixed seed keeps this reproducible.
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state.wrapping_mul(0x2545_F491_4F6C_DD1D)
    };
    for _ in 0..300 {
        let r = next();
        let magnitude = r >> 1;
        let s = if r & 1 == 0 {
            format!("{magnitude}")
        } else {
            format!("-{magnitude}")
        };
        assert_same(&format!("rand {s}"), s.as_bytes());
    }
}

/// Redirections that change how the programs' streams behave, rather than what
/// they read.
#[test]
fn stdin_from_dev_null_and_stdout_to_dev_null() {
    for exe_pair in [(c_bin(), rust_bin())] {
        let (c, r) = exe_pair;
        for path in ["/dev/null"] {
            let cs = Command::new(&c)
                .stdin(std::fs::File::open(path).unwrap())
                .output()
                .unwrap();
            let rs = Command::new(&r)
                .stdin(std::fs::File::open(path).unwrap())
                .output()
                .unwrap();
            assert_eq!(cs.stdout, rs.stdout, "stdout differs with stdin={path}");
            assert_eq!(cs.stderr, rs.stderr, "stderr differs with stdin={path}");
            assert_eq!(
                cs.status.code(),
                rs.status.code(),
                "exit status differs with stdin={path}"
            );
        }
    }
}

/// Extra command-line arguments: `main` takes none, so they must be ignored
/// identically by both.
#[test]
fn extra_arguments_are_ignored() {
    for exe in [c_bin(), rust_bin()] {
        let mut child = Command::new(&exe)
            .args(["alpha", "-9", "--help"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.as_mut().unwrap().write_all(b"5").unwrap();
        drop(child.stdin.take());
        let out = child.wait_with_output().unwrap();
        assert_eq!(out.stdout, b"310\n", "{} stdout", exe.display());
        assert_eq!(out.stderr, b"", "{} stderr", exe.display());
        assert_eq!(out.status.code(), Some(0), "{} status", exe.display());
    }
}

/// A write to a pipe whose reader is gone. The C program inherits the default
/// `SIGPIPE` disposition and is killed by the signal; Rust's runtime ignores
/// `SIGPIPE` unless the program restores the default, so this asserts the exit
/// status still matches.
#[test]
fn sigpipe_on_closed_stdout_matches() {
    use std::os::unix::io::FromRawFd;

    fn probe(exe: &Path) -> Result<i32, i32> {
        // A pipe whose read end is closed before the child writes anything.
        let mut fds = [0i32; 2];
        let rc = unsafe { libc_pipe(fds.as_mut_ptr()) };
        assert_eq!(rc, 0, "pipe() failed");
        let (read_fd, write_fd) = (fds[0], fds[1]);
        unsafe { libc_close(read_fd) };
        let stdout = unsafe { std::fs::File::from_raw_fd(write_fd) };

        let mut child = Command::new(exe)
            .stdin(Stdio::piped())
            .stdout(stdout) // the File is consumed and closed by Command
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let _ = child.stdin.as_mut().unwrap().write_all(b"5\n");
        drop(child.stdin.take());
        let out = child.wait_with_output().unwrap();
        match out.status.code() {
            Some(c) => Ok(c),
            None => Err(out.status.signal().unwrap()),
        }
    }

    extern "C" {
        #[link_name = "pipe"]
        fn libc_pipe(fds: *mut i32) -> i32;
        #[link_name = "close"]
        fn libc_close(fd: i32) -> i32;
    }

    let c = probe(&c_bin());
    let r = probe(&rust_bin());
    assert_eq!(
        c, r,
        "exit status differs when stdout has no reader: C={c:?} Rust={r:?}"
    );
}
