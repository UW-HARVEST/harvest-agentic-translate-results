// Differential tests: run the C binary and the Rust binary as subprocesses,
// feed both the same bytes on stdin, and require byte-identical stdout,
// byte-identical stderr and an identical exit status.
//
// Nothing here links against the Rust crate as a library; both programs are
// driven exactly the way a shell drives them.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// `<repo>/` — the directory holding both `c_src/` and `translation/`.
fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is `<repo>/translation`.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// Path to the C executable, building it with CMake on first use.
fn c_binary() -> PathBuf {
    let root = repo_root();
    let c_src = root.join("c_src");
    let build = c_src.join("build");
    let exe = build.join("driver");
    if exe.exists() {
        return exe;
    }

    std::fs::create_dir_all(&build).expect("create c_src/build");
    let status = Command::new("cmake")
        .arg("..")
        .current_dir(&build)
        .stdout(Stdio::null())
        .status()
        .expect("run cmake (is cmake installed?)");
    assert!(status.success(), "cmake configure failed");

    let status = Command::new("cmake")
        .args(["--build", "."])
        .current_dir(&build)
        .stdout(Stdio::null())
        .status()
        .expect("run cmake --build");
    assert!(status.success(), "cmake build failed");

    assert!(exe.exists(), "C binary missing after build: {}", exe.display());
    exe
}

/// Path to the Rust executable under test (the integration-test harness sets
/// CARGO_BIN_EXE_<name> to the freshly built binary).
fn rust_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    status: Option<i32>,
}

fn run(exe: &Path, stdin_bytes: &[u8]) -> Run {
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));

    {
        let mut stdin = child.stdin.take().expect("stdin pipe");
        // The child may exit without draining stdin; a broken pipe is not a
        // test failure.
        let _ = stdin.write_all(stdin_bytes);
        let _ = stdin.flush();
    }

    let out = child.wait_with_output().expect("wait for child");
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        status: out.status.code(),
    }
}

fn show(bytes: &[u8]) -> String {
    format!("{:?} (hex {})", String::from_utf8_lossy(bytes), hex(bytes))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Core assertion: for `input`, the two programs agree on all three channels.
fn assert_same(label: &str, input: &[u8]) {
    let c = run(&c_binary(), input);
    let r = run(&rust_binary(), input);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout differs for {label}\n  input: {}\n  C:    {}\n  Rust: {}",
        show(input),
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr differs for {label}\n  input: {}\n  C:    {}\n  Rust: {}",
        show(input),
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        c.status, r.status,
        "exit status differs for {label}\n  input: {}\n  C: {:?}  Rust: {:?}",
        show(input),
        c.status,
        r.status
    );
}

fn check_all(cases: &[(&str, &[u8])]) {
    for (label, input) in cases {
        assert_same(label, input);
    }
}

// ---------------------------------------------------------------------------
// Phase A sanity: both binaries exist and are runnable.
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_run() {
    let c = c_binary();
    let r = rust_binary();
    assert!(c.exists(), "C binary not found: {}", c.display());
    assert!(r.exists(), "Rust binary not found: {}", r.display());
    // Trivially runnable with empty stdin.
    let cr = run(&c, b"");
    let rr = run(&r, b"");
    assert_eq!(cr.status, Some(0));
    assert_eq!(rr.status, Some(0));
}

// ---------------------------------------------------------------------------
// Phase B: the branches main() actually takes.
//
//   main: x = 0; scanf("%d", &x); if (x) good() else bad();
//
// Two output branches only, but three distinct scanf outcomes reach them:
//   * successful conversion, non-zero  -> good() -> "string\n"
//   * successful conversion, zero      -> bad()  -> "\n"
//   * input failure / matching failure -> x stays 0 -> bad() -> "\n"
// ---------------------------------------------------------------------------

#[test]
fn empty_input_is_input_failure() {
    // EOF immediately: scanf returns EOF, x keeps its initializer 0, bad() runs.
    check_all(&[("empty stdin", b"")]);
}

#[test]
fn single_item_zero_takes_bad_branch() {
    check_all(&[
        ("zero", b"0"),
        ("zero with newline", b"0\n"),
        ("double zero", b"00"),
        ("many zeros", b"0000000"),
        ("plus zero", b"+0"),
        ("minus zero", b"-0"),
        ("minus many zeros", b"-000"),
    ]);
}

#[test]
fn single_item_nonzero_takes_good_branch() {
    check_all(&[
        ("one", b"1"),
        ("one with newline", b"1\n"),
        ("negative one", b"-1"),
        ("plus one", b"+1"),
        ("leading zeros then 7", b"007"),
        ("large positive", b"123456"),
        ("large negative", b"-123456"),
    ]);
}

#[test]
fn matching_failure_leaves_x_zero() {
    // scanf finds a non-numeric character: matching failure, x untouched.
    check_all(&[
        ("letters", b"abc"),
        ("single letter", b"x"),
        ("dot five", b".5"),
        ("exponent only", b"e5"),
        ("plus letters", b"+abc"),
        ("minus letters", b"-abc"),
        ("double minus", b"--5"),
        ("sign then space then digit", b"-  5"),
        ("hash", b"#"),
        ("comma", b","),
    ]);
}

#[test]
fn sign_then_eof_is_not_a_conversion() {
    check_all(&[
        ("lone minus", b"-"),
        ("lone plus", b"+"),
        ("lone minus newline", b"-\n"),
        ("lone plus newline", b"+\n"),
    ]);
}

#[test]
fn whitespace_only_input() {
    check_all(&[
        ("single space", b" "),
        ("two spaces", b"  "),
        ("tab", b"\t"),
        ("newline", b"\n"),
        ("crlf", b"\r\n"),
        ("vertical tab and form feed", b"\x0b\x0c"),
        ("mixed whitespace", b" \t\r\n\x0b\x0c "),
    ]);
}

#[test]
fn scanf_skips_leading_whitespace_across_newlines() {
    // %d skips *any* whitespace, newlines included -- unlike fgets.
    check_all(&[
        ("spaces then 42", b"  42"),
        ("newlines then 5", b"\n\n\t 5"),
        ("crlf then 0", b"\r\n0"),
        ("vt ff then 7", b"\x0b\x0c7"),
        ("newlines then 0", b"\n\n\n0"),
        ("newlines then letters", b"\n\n\nabc"),
    ]);
}

#[test]
fn only_the_first_item_is_read() {
    // The program calls scanf once; trailing data must not change the outcome.
    check_all(&[
        ("one space two", b"1 2"),
        ("zero space zero", b"0 0"),
        ("zero space one", b"0 1"),
        ("one space zero", b"1 0"),
        ("digits then letters", b"5abc"),
        ("zero then letters", b"0abc"),
        ("three lines", b"1\n2\n3"),
        ("zero then lines", b"0\n1\n1"),
        ("hex-looking: 0 then x10", b"0x10"),
        ("float-looking: 3 then .9", b"3.9"),
        ("float-looking zero", b"0.9"),
    ]);
}

// ---------------------------------------------------------------------------
// Phase C: overflow, truncation and signedness exactly as the C performs it.
//
// glibc's %d accumulates in a `long`, saturates at LONG_MIN/LONG_MAX on
// overflow, then stores the low 32 bits into the `int`. Whether the resulting
// int is zero decides the branch, so these inputs are behaviourally load-
// bearing, not cosmetic.
// ---------------------------------------------------------------------------

#[test]
fn int_boundaries() {
    check_all(&[
        ("INT_MAX", b"2147483647"),
        ("INT_MIN", b"-2147483648"),
        ("INT_MAX+1 truncates to INT_MIN", b"2147483648"),
        ("INT_MIN-1 truncates to INT_MAX", b"-2147483649"),
    ]);
}

#[test]
fn truncation_to_int_can_produce_zero() {
    check_all(&[
        // 2^32 -> low 32 bits are 0 -> bad() branch even though a conversion
        // succeeded with a non-zero value.
        ("2^32 truncates to 0", b"4294967296"),
        ("2^32+1 truncates to 1", b"4294967297"),
        ("2^33 truncates to 0", b"8589934592"),
        ("-2^32 truncates to 0", b"-4294967296"),
        ("10^10", b"10000000000"),
    ]);
}

#[test]
fn long_boundaries_and_saturation() {
    check_all(&[
        ("LONG_MAX", b"9223372036854775807"),
        ("LONG_MAX+1 saturates", b"9223372036854775808"),
        ("LONG_MIN", b"-9223372036854775808"),
        ("LONG_MIN-1 saturates", b"-9223372036854775809"),
        ("2^64 saturates to LONG_MAX -> -1", b"18446744073709551616"),
        ("2^64+1 saturates", b"18446744073709551617"),
        // Saturated LONG_MAX has low 32 bits 0xffffffff (non-zero) ...
        ("twenty nines", b"99999999999999999999"),
        // ... while saturated LONG_MIN has low 32 bits 0 (zero!).
        ("minus twenty nines", b"-99999999999999999999"),
        ("fifty digits", b"12345678901234567890123456789012345678901234567890"),
        (
            "minus fifty digits",
            b"-12345678901234567890123456789012345678901234567890",
        ),
    ]);
}

#[test]
fn very_long_digit_runs() {
    // %d has no field width here, so glibc consumes an unbounded digit run.
    let mut zeros_then_one = vec![b'0'; 100_000];
    zeros_then_one.push(b'1');
    let nines = vec![b'9'; 100_000];
    let mut neg_nines = vec![b'-'];
    neg_nines.extend(std::iter::repeat(b'9').take(100_000));
    let all_zeros = vec![b'0'; 100_000];
    let many_spaces = vec![b' '; 100_000];

    check_all(&[
        ("100k zeros then 1", &zeros_then_one),
        ("100k nines", &nines),
        ("minus 100k nines", &neg_nines),
        ("100k zeros", &all_zeros),
        ("100k spaces then EOF", &many_spaces),
    ]);
}

#[test]
fn non_ascii_and_nul_bytes() {
    check_all(&[
        ("high bytes", b"\xff\xfe"),
        ("NUL first", b"\x00"),
        ("NULs then 42", b"\x00\x0042"),
        ("42 then NUL", b"42\x00"),
        ("0 then NUL", b"0\x00"),
        ("utf8 text", "café".as_bytes()),
        ("minus sign u2212 then 5", "\u{2212}5".as_bytes()),
    ]);
}

#[test]
fn closed_stdin_behaves_like_eof() {
    // Spawn both with stdin connected to /dev/null (immediate EOF) rather than
    // a pipe, to exercise the input-failure path without any write at all.
    let devnull = || Stdio::from(std::fs::File::open("/dev/null").expect("/dev/null"));

    let c = Command::new(c_binary())
        .stdin(devnull())
        .output()
        .expect("run C");
    let r = Command::new(rust_binary())
        .stdin(devnull())
        .output()
        .expect("run Rust");

    assert_eq!(c.stdout, r.stdout, "stdout differs with /dev/null stdin");
    assert_eq!(c.stderr, r.stderr, "stderr differs with /dev/null stdin");
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "exit status differs with /dev/null stdin"
    );
}

#[test]
fn stdout_write_failure_is_not_reported() {
    // stdout points at /dev/full: every write fails with ENOSPC. glibc swallows
    // the failed flush at exit and main has already returned 0, so the C exits
    // 0 with no diagnostic. The Rust must not surface an error either.
    let devfull = || match std::fs::OpenOptions::new().write(true).open("/dev/full") {
        Ok(f) => Some(Stdio::from(f)),
        Err(_) => None, // platform without /dev/full
    };

    for input in [&b"1"[..], &b"0"[..]] {
        let (Some(cf), Some(rf)) = (devfull(), devfull()) else {
            return;
        };

        let mut cc = Command::new(c_binary())
            .stdin(Stdio::piped())
            .stdout(cf)
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn C");
        let _ = cc.stdin.take().unwrap().write_all(input);
        let c = cc.wait_with_output().expect("wait C");

        let mut rc = Command::new(rust_binary())
            .stdin(Stdio::piped())
            .stdout(rf)
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn Rust");
        let _ = rc.stdin.take().unwrap().write_all(input);
        let r = rc.wait_with_output().expect("wait Rust");

        assert_eq!(c.stderr, r.stderr, "stderr differs on /dev/full stdout");
        assert_eq!(
            c.status.code(),
            r.status.code(),
            "exit status differs on /dev/full stdout"
        );
    }
}

/// SIGPIPE: the reader of stdout is gone before the program writes.
///
/// The Rust runtime sets SIGPIPE to SIG_IGN before `main`, which C does not do.
/// Unless the translation restores the default disposition, the C is killed by
/// signal 13 while the Rust exits 0 -- a mismatch invisible to any test that
/// only inspects stdout.
#[cfg(unix)]
#[test]
fn broken_stdout_pipe_kills_both_with_sigpipe() {
    use std::os::unix::io::{FromRawFd, OwnedFd};
    use std::os::unix::process::ExitStatusExt;

    /// stdout = the write end of a pipe whose read end is already closed.
    fn dead_pipe_write_end() -> OwnedFd {
        let mut fds = [0i32; 2];
        // SAFETY: `pipe` fills two ints; both fds are then owned by us.
        let rc = unsafe {
            extern "C" {
                fn pipe(fds: *mut i32) -> i32;
                fn close(fd: i32) -> i32;
            }
            let rc = pipe(fds.as_mut_ptr());
            if rc == 0 {
                close(fds[0]); // drop the reader
            }
            rc
        };
        assert_eq!(rc, 0, "pipe() failed");
        // SAFETY: fds[1] is a fresh, owned descriptor.
        unsafe { OwnedFd::from_raw_fd(fds[1]) }
    }

    fn run_with_dead_reader(exe: &Path, input: &[u8]) -> (Option<i32>, Option<i32>, Vec<u8>) {
        let mut child = Command::new(exe)
            .stdin(Stdio::piped())
            .stdout(Stdio::from(dead_pipe_write_end()))
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn");
        let _ = child.stdin.take().unwrap().write_all(input);
        let out = child.wait_with_output().expect("wait");
        (out.status.code(), out.status.signal(), out.stderr)
    }

    for input in [&b"1"[..], &b"0"[..], &b""[..], &b"abc"[..]] {
        let c = run_with_dead_reader(&c_binary(), input);
        let r = run_with_dead_reader(&rust_binary(), input);
        assert_eq!(
            (c.0, c.1),
            (r.0, r.1),
            "exit status/signal differs for input {} with a dead stdout reader\
             \n  C: code={:?} signal={:?}\n  Rust: code={:?} signal={:?}",
            show(input),
            c.0,
            c.1,
            r.0,
            r.1
        );
        assert_eq!(c.2, r.2, "stderr differs with a dead stdout reader");
    }
}

// ---------------------------------------------------------------------------
// Deterministic sweep: every 1-, 2- and 3-byte string over an alphabet of the
// characters scanf's %d actually discriminates on, plus a seeded pseudo-random
// sweep over wider bytes. This mechanically reaches input classes that were
// not hand-enumerated above.
// ---------------------------------------------------------------------------

#[test]
fn exhaustive_short_inputs_over_interesting_alphabet() {
    const ALPHABET: &[u8] = b"0129+- \t\nx.\0";

    for a in ALPHABET {
        assert_same("len1", &[*a]);
    }
    for a in ALPHABET {
        for b in ALPHABET {
            assert_same("len2", &[*a, *b]);
        }
    }
    for a in ALPHABET {
        for b in ALPHABET {
            for c in ALPHABET {
                assert_same("len3", &[*a, *b, *c]);
            }
        }
    }
}

#[test]
fn seeded_random_inputs() {
    // xorshift64*, so the corpus is reproducible without a dev-dependency.
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = move || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state.wrapping_mul(0x2545_F491_4F6C_DD1D)
    };

    for _ in 0..400 {
        let len = (next() % 24) as usize;
        let input: Vec<u8> = (0..len)
            .map(|_| {
                let r = next();
                match r % 10 {
                    // Bias heavily toward digits, signs and whitespace so the
                    // conversion path is exercised, but keep arbitrary bytes in
                    // the mix.
                    0..=5 => b'0' + (r >> 8) as u8 % 10,
                    6 => *b" \t\n\r\x0b\x0c".get((r >> 8) as usize % 6).unwrap(),
                    7 => *b"+-".get((r >> 8) as usize % 2).unwrap(),
                    _ => (r >> 8) as u8,
                }
            })
            .collect();
        assert_same("random", &input);
    }
}

// ---------------------------------------------------------------------------
// Pin down the two observable outputs, so a regression that changes *both*
// programs' comparison basis (e.g. an accidentally silent binary) is caught.
// ---------------------------------------------------------------------------

#[test]
fn good_branch_prints_string_and_newline() {
    let c = run(&c_binary(), b"1");
    assert_eq!(c.stdout, b"string\n", "C good() output changed");
    let r = run(&rust_binary(), b"1");
    assert_eq!(r.stdout, b"string\n");
    assert!(c.stderr.is_empty() && r.stderr.is_empty());
    assert_eq!(c.status, Some(0));
    assert_eq!(r.status, Some(0));
}

#[test]
fn bad_branch_prints_only_a_newline() {
    // bad() reads an uninitialized `char *`. On this platform the leftover
    // stack slot is non-NULL and points at a zero byte, so printLine emits
    // just the "\n" from its format string. Asserted against the C directly.
    let c = run(&c_binary(), b"0");
    assert_eq!(c.stdout, b"\n", "C bad() output changed");
    let r = run(&rust_binary(), b"0");
    assert_eq!(r.stdout, b"\n");
    assert!(c.stderr.is_empty() && r.stderr.is_empty());
    assert_eq!(c.status, Some(0));
    assert_eq!(r.status, Some(0));
}
