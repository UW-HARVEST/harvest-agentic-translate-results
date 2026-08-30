//! Differential tests: run the original C binary and the Rust binary as
//! subprocesses on identical stdin and require byte-identical stdout, stderr
//! and identical exit status (including death-by-signal).
//!
//! Nothing here loads the Rust code as a library — both programs are driven
//! exactly the way a shell drives them, because that is how they are compared.

use std::io::{Read as _, Write};
use std::os::unix::io::FromRawFd;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

extern "C" {
    fn pipe(fds: *mut i32) -> i32;
    fn close(fd: i32) -> i32;
    fn signal(signum: i32, handler: usize) -> usize;
}

const SIGPIPE: i32 = 13;
const SIG_DFL: usize = 0;
const SIG_IGN: usize = 1;

// ---------------------------------------------------------------------------
// Locating / building the two executables
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// Directory holding the built Rust binary (target/<profile>/).
fn rust_bin() -> PathBuf {
    // env!("CARGO_BIN_EXE_<name>") is resolved by cargo for integration tests.
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Configure + build the C program with CMake, out-of-tree, and return the
/// path to the resulting `driver` executable. `c_src/` is never written to.
fn c_bin() -> &'static PathBuf {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let root = repo_root();
        let c_src = root.join("c_src");
        assert!(
            c_src.join("CMakeLists.txt").is_file(),
            "cannot find {}",
            c_src.join("CMakeLists.txt").display()
        );

        // Build into target/ so that c_src/ stays pristine.
        let build_dir = rust_bin()
            .parent()
            .expect("binary must live in a directory")
            .join("c_reference_build");
        std::fs::create_dir_all(&build_dir).expect("cannot create C build dir");

        let exe = build_dir.join("driver");

        let cfg = Command::new("cmake")
            .arg("-S")
            .arg(&c_src)
            .arg("-B")
            .arg(&build_dir)
            .output()
            .expect("failed to run cmake (is cmake installed?)");
        assert!(
            cfg.status.success(),
            "cmake configure failed:\n{}\n{}",
            String::from_utf8_lossy(&cfg.stdout),
            String::from_utf8_lossy(&cfg.stderr)
        );

        let bld = Command::new("cmake")
            .arg("--build")
            .arg(&build_dir)
            .output()
            .expect("failed to run cmake --build");
        assert!(
            bld.status.success(),
            "cmake build failed:\n{}\n{}",
            String::from_utf8_lossy(&bld.stdout),
            String::from_utf8_lossy(&bld.stderr)
        );

        assert!(exe.is_file(), "C binary missing at {}", exe.display());
        exe
    })
}

// ---------------------------------------------------------------------------
// Running one program
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq)]
struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// Normal exit code, if the process exited normally.
    code: Option<i32>,
    /// Terminating signal, if the process was killed by one (e.g. SIGFPE = 8).
    signal: Option<i32>,
}

impl std::fmt::Debug for Run {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "code={:?} signal={:?}\n  stdout={:?}\n  stderr={:?}",
            self.code,
            self.signal,
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr),
        )
    }
}

fn run(exe: &Path, stdin_bytes: &[u8]) -> Run {
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));

    {
        let mut sink = child.stdin.take().expect("stdin was piped");
        let data = stdin_bytes.to_vec();
        // Write on a helper thread so a program that never reads stdin (or a
        // very large input) cannot deadlock the test.
        std::thread::spawn(move || {
            let _ = sink.write_all(&data);
            let _ = sink.flush();
        });
    }

    let out = child.wait_with_output().expect("failed to collect output");
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

/// The core assertion: C and Rust agree on stdout, stderr and exit status.
fn assert_same(label: &str, stdin_bytes: &[u8]) {
    let c = run(c_bin(), stdin_bytes);
    let r = run(&rust_bin(), stdin_bytes);

    assert_eq!(
        c.stdout,
        r.stdout,
        "[{label}] stdout differs for input {:?}\n  C: {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(stdin_bytes),
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout),
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "[{label}] stderr differs for input {:?}\n  C: {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(stdin_bytes),
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr),
    );
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "[{label}] exit status differs for input {:?}\n  C: {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(stdin_bytes),
        (c.code, c.signal),
        (r.code, r.signal),
    );
}

fn check_all(cases: &[(&str, &str)]) {
    for (label, input) in cases {
        assert_same(label, input.as_bytes());
    }
}

// ---------------------------------------------------------------------------
// Phase A — both programs build and run
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_exist_and_run() {
    let c = run(c_bin(), b"10 3\n");
    let r = run(&rust_bin(), b"10 3\n");
    assert_eq!(c.stdout, b"quotient: 3, remainder: 1\n".to_vec());
    assert_eq!(r.stdout, c.stdout);
    assert_eq!(c.code, Some(0));
    assert_eq!(r.code, Some(0));
}

// ---------------------------------------------------------------------------
// Phase B — the branches the C program actually has
//
//   int x = 1, y = 1;
//   scanf("%d %d", &x, &y);       <- 0, 1 or 2 successful conversions
//   div_t result = div(x, y);     <- y == 0 or (INT_MIN, -1) -> SIGFPE
//   printf("quotient: %d, remainder: %d\n", ...);
// ---------------------------------------------------------------------------

/// Zero conversions: both x and y keep their initialiser 1 -> div(1, 1).
#[test]
fn zero_conversions_keep_initialisers() {
    check_all(&[
        ("empty", ""),
        ("only_spaces", "   "),
        ("only_newlines", "\n\n\n\n"),
        ("only_tabs", "\t\t"),
        ("all_c_whitespace", " \t\n\x0b\x0c\r"),
        ("leading_garbage", "abc"),
        ("matching_failure_sign_only", "-"),
        ("matching_failure_plus_only", "+"),
        ("sign_then_space", "- 7 2"),
        ("double_sign", "--7 2"),
        ("double_plus", "++5 2"),
        ("dot_first", ".5 2"),
        ("letter_e", "e3 2"),
        ("ws_then_garbage", "   \n  xyz"),
    ]);
}

/// Exactly one conversion: y keeps its initialiser 1 -> div(x, 1).
#[test]
fn one_conversion_leaves_y_as_one() {
    check_all(&[
        ("single_item", "7"),
        ("single_item_newline", "7\n"),
        ("single_negative", "-7"),
        ("single_plus", "+7"),
        ("single_zero", "0"),
        ("x_then_trailing_ws", "7   "),
        ("x_then_trailing_newlines", "7\n\n\n"),
        ("x_then_garbage", "7 abc"),
        ("x_then_sign_only", "7 -"),
        ("x_then_plus_only", "7 +"),
        ("x_then_dot", "7 .5"),
        ("hex_prefix_stops_at_x", "0x10 2"),
        ("decimal_point_stops", "7.5 2"),
        ("exponent_stops", "1e3 2"),
        ("nul_byte_separator", "7\x002"),
        ("x_ok_y_letter", "10 z"),
    ]);
}

/// Two conversions, ordinary values. Checks C's truncation-toward-zero
/// division and the sign of the remainder in all four sign combinations.
#[test]
fn two_conversions_sign_matrix() {
    check_all(&[
        ("pos_pos", "7 2"),
        ("neg_pos", "-7 2"),
        ("pos_neg", "7 -2"),
        ("neg_neg", "-7 -2"),
        ("exact_pos", "6 3"),
        ("exact_neg", "-6 3"),
        ("numerator_zero", "0 5"),
        ("numerator_zero_neg_den", "0 -1"),
        ("smaller_than_divisor", "1 5"),
        ("smaller_than_divisor_neg", "-1 5"),
        ("neg_one_neg_one", "-1 -1"),
        ("one_one", "1 1"),
        ("leading_zeros", "0007 0002"),
        ("explicit_plus_both", "+7 +2"),
    ]);
}

/// `scanf` skips arbitrary whitespace, including newlines, between and before
/// conversions — unlike `fgets`, it reads straight across line boundaries.
#[test]
fn scanf_reads_across_whitespace_and_newlines() {
    check_all(&[
        ("space_separated", "7 2"),
        ("newline_separated", "7\n2\n"),
        ("many_newlines_between", "7\n\n\n\n2"),
        ("tabs_and_newlines", "\t 7 \t\n\n  2  "),
        ("crlf", "7\r\n2\r\n"),
        ("vtab_formfeed", "\x0b7\x0c2"),
        ("leading_ws_before_first", "\n\n\t  7 2"),
        ("no_trailing_newline", "7 2"),
        ("extra_third_token_ignored", "7 2 99"),
        ("extra_tokens_ignored", "7 2 99 100 abc"),
    ]);
}

/// int boundary values that still divide successfully.
#[test]
fn int_boundaries() {
    check_all(&[
        ("intmax_div_1", "2147483647 1"),
        ("intmin_div_1", "-2147483648 1"),
        ("intmax_div_neg1", "2147483647 -1"),
        ("intmax_as_divisor", "5 2147483647"),
        ("intmin_as_divisor", "1 -2147483648"),
        ("intmin_over_intmin", "-2147483648 -2147483648"),
        ("intmax_over_intmax", "2147483647 2147483647"),
        ("intmin_div_2", "-2147483648 2"),
        ("intmin_div_neg2", "-2147483648 -2"),
    ]);
}

/// glibc converts `%d` through a `long`, then truncates into the `int`.
/// Out-of-range input saturates at LONG_MAX / LONG_MIN first.
#[test]
fn out_of_int_range_truncates_like_glibc() {
    check_all(&[
        ("intmax_plus_1", "2147483648 1"),
        ("intmin_minus_1", "-2147483649 1"),
        ("u32_max", "4294967295 1"),
        ("two_pow_32", "4294967296 3"),
        ("two_pow_32_plus_7", "4294967303 4"),
        ("neg_two_pow_32", "-4294967296 1"),
        ("long_max", "9223372036854775807 5"),
        ("long_max_plus_1", "9223372036854775808 5"),
        ("long_min", "-9223372036854775808 5"),
        ("long_min_minus_1", "-9223372036854775809 5"),
        ("way_out_of_range", "99999999999999999999999 5"),
        ("way_out_of_range_neg", "-99999999999999999999999 5"),
        ("y_out_of_range", "100 4294967296"),
    ]);
}

/// Very long digit runs and very long whitespace runs.
#[test]
fn very_long_tokens() {
    let nines = "9".repeat(5000);
    let zeros = "0".repeat(5000);
    let spaces = " ".repeat(10000);

    let cases: Vec<(String, String)> = vec![
        ("five_thousand_nines".into(), format!("{nines} 7")),
        ("five_thousand_nines_neg".into(), format!("-{nines} 7")),
        ("leading_zero_flood".into(), format!("{zeros}7 3")),
        ("zero_flood_is_zero".into(), format!("{zeros} 3")),
        ("whitespace_flood".into(), format!("{spaces}7 2")),
        ("nines_both".into(), format!("{nines} {nines}")),
    ];
    for (label, input) in &cases {
        assert_same(label, input.as_bytes());
    }
}

// ---------------------------------------------------------------------------
// Phase C — the error / fault paths
// ---------------------------------------------------------------------------

/// `div(x, 0)` is undefined behaviour in C; on x86-64 the `idiv` instruction
/// raises SIGFPE, so the program dies by signal 8 with NO stdout at all.
#[test]
fn division_by_zero_dies_by_signal() {
    check_all(&[
        ("pos_div_zero", "5 0"),
        ("neg_div_zero", "-5 0"),
        ("zero_div_zero", "0 0"),
        ("intmax_div_zero", "2147483647 0"),
        ("intmin_div_zero", "-2147483648 0"),
        ("zero_with_plus", "5 +0"),
        ("zero_with_minus", "5 -0"),
        ("zero_leading_zeros", "5 0000"),
        ("truncates_to_zero", "5 4294967296"),
        ("truncates_to_zero_huge", "5 18446744073709551616"),
    ]);
}

/// INT_MIN / -1 overflows the quotient; `idiv` raises SIGFPE for that too.
#[test]
fn intmin_over_neg_one_dies_by_signal() {
    check_all(&[
        ("intmin_over_neg1", "-2147483648 -1"),
        ("intmin_over_neg1_newlines", "-2147483648\n-1\n"),
        // -2147483648 arrived at by truncation from a wider value.
        ("truncated_intmin_over_neg1", "2147483648 -1"),
        ("truncated_intmin_over_neg1b", "-4294967296 -1"),
        // -1 arrived at by truncation (LONG_MAX -> -1).
        ("intmin_over_truncated_neg1", "-2147483648 4294967295"),
        (
            "intmin_over_saturated_neg1",
            "-2147483648 99999999999999999999999",
        ),
    ]);
}

/// The faulting cases must produce empty stdout and empty stderr and be
/// reported as killed by signal 8 (not as a normal exit) by BOTH programs.
#[test]
fn fault_cases_have_no_output_and_signal_eight() {
    for input in ["5 0", "0 0", "-2147483648 -1"] {
        let c = run(c_bin(), input.as_bytes());
        let r = run(&rust_bin(), input.as_bytes());

        assert_eq!(c.signal, Some(8), "C should die of SIGFPE on {input:?}: {c:?}");
        assert_eq!(c.code, None, "C should not exit normally on {input:?}");
        assert!(c.stdout.is_empty(), "C stdout should be empty on {input:?}");

        assert_eq!(r.signal, c.signal, "signal mismatch on {input:?}");
        assert_eq!(r.code, c.code, "exit code mismatch on {input:?}");
        assert_eq!(r.stdout, c.stdout, "stdout mismatch on {input:?}");
        assert_eq!(r.stderr, c.stderr, "stderr mismatch on {input:?}");
    }
}

/// Non-UTF-8 / binary stdin must not change behaviour (the C code reads bytes).
#[test]
fn binary_and_non_ascii_input() {
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("raw_control_bytes", vec![0x01, 0x02, b' ', b'7']),
        ("invalid_utf8_lead", vec![0xff, 0xfe, b' ', b'9', b' ', b'2']),
        ("nul_first", vec![0x00, b'7', b' ', b'2']),
        ("fullwidth_digits", "７ ２".as_bytes().to_vec()),
        ("digit_then_invalid_utf8", vec![b'8', 0x80, b' ', b'2']),
        ("high_bytes_only", vec![0x80, 0x81, 0x82]),
        ("nul_padded_number", vec![b'4', b'2', 0x00, 0x00, b'2']),
    ];
    for (label, input) in &cases {
        assert_same(label, input);
    }
}

/// stdin closed immediately (EOF with no bytes at all).
#[test]
fn immediate_eof() {
    assert_same("immediate_eof", b"");
}

/// Randomised differential sweep over the alphabet the parser branches on.
/// Deterministic (fixed LCG seed) so failures are reproducible.
#[test]
fn randomised_differential_sweep() {
    const ALPHABET: &[u8] = b"0123456789 \t\n\r+-.abcxXeE\x0b\x0c";
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = |n: u64| -> u64 {
        // xorshift64*
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state.wrapping_mul(0x2545_F491_4F6C_DD1D) % n
    };

    for i in 0..400 {
        let len = next(26) as usize;
        let input: Vec<u8> = (0..len).map(|_| ALPHABET[next(ALPHABET.len() as u64) as usize]).collect();
        assert_same(&format!("random_{i}"), &input);
    }
}

/// Randomised sweep over pairs of well-formed integers spanning the int range,
/// including the divisors that fault.
#[test]
fn randomised_integer_pairs() {
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = || -> u64 {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state.wrapping_mul(0x2545_F491_4F6C_DD1D)
    };

    let interesting: [i64; 14] = [
        0,
        1,
        -1,
        2,
        -2,
        i32::MAX as i64,
        i32::MIN as i64,
        i32::MAX as i64 + 1,
        i32::MIN as i64 - 1,
        u32::MAX as i64,
        4294967296,
        i64::MAX,
        i64::MIN,
        123456789,
    ];

    for i in 0..250 {
        let pick = |v: u64| -> i64 {
            if v % 3 == 0 {
                interesting[(v / 3) as usize % interesting.len()]
            } else {
                (v as i32) as i64
            }
        };
        let x = pick(next());
        let y = pick(next());
        let sep = match next() % 4 {
            0 => " ",
            1 => "\n",
            2 => "\t\n  ",
            _ => "   ",
        };
        let input = format!("{x}{sep}{y}");
        assert_same(&format!("pair_{i}"), input.as_bytes());
    }
}

// ---------------------------------------------------------------------------
// Phase C — stdout write failures and SIGPIPE disposition
//
// The C program never touches SIGPIPE, so it runs with the disposition it
// inherited, and its printf/fflush failures are non-fatal. The Rust runtime
// installs SIGPIPE=SIG_IGN before main and `print!` panics on write errors, so
// both of those had to be neutralised in the translation.
// ---------------------------------------------------------------------------

/// Spawn `exe` with stdout wired to a pipe that has NO reader (the read end is
/// closed before the child is even created), so the very first write fails with
/// EPIPE. `sigpipe` is the disposition installed in the child before exec.
fn run_with_broken_stdout(exe: &Path, stdin_bytes: &[u8], sigpipe: usize) -> Run {
    // A pipe whose read end is closed up-front => deterministic EPIPE, no race.
    let mut fds = [-1i32; 2];
    assert_eq!(unsafe { pipe(fds.as_mut_ptr()) }, 0, "pipe() failed");
    let (read_end, write_end) = (fds[0], fds[1]);
    assert_eq!(unsafe { close(read_end) }, 0, "close() failed");

    let stdout_sink = unsafe { Stdio::from_raw_fd(write_end) };

    let mut cmd = Command::new(exe);
    cmd.stdin(Stdio::piped()).stdout(stdout_sink).stderr(Stdio::piped());
    unsafe {
        // Runs in the child, between fork and exec. SIG_DFL and SIG_IGN both
        // survive execve, so this sets the disposition the program inherits.
        cmd.pre_exec(move || {
            signal(SIGPIPE, sigpipe);
            Ok(())
        });
    }

    let mut child = cmd.spawn().unwrap_or_else(|e| panic!("spawn {}: {e}", exe.display()));

    {
        let mut sink = child.stdin.take().expect("stdin piped");
        let data = stdin_bytes.to_vec();
        std::thread::spawn(move || {
            let _ = sink.write_all(&data);
        });
    }

    let mut stderr = Vec::new();
    child
        .stderr
        .take()
        .expect("stderr piped")
        .read_to_end(&mut stderr)
        .expect("read stderr");
    let status = child.wait().expect("wait");

    Run {
        stdout: Vec::new(), // unobservable: it went into the broken pipe
        stderr,
        code: status.code(),
        signal: status.signal(),
    }
}

/// With SIGPIPE at its default disposition, writing to the dead stdout pipe
/// kills the process with signal 13. Both programs must agree.
#[test]
fn broken_stdout_with_default_sigpipe() {
    for input in ["7 2", "", "abc"] {
        let c = run_with_broken_stdout(c_bin(), input.as_bytes(), SIG_DFL);
        let r = run_with_broken_stdout(&rust_bin(), input.as_bytes(), SIG_DFL);
        assert_eq!(
            c.signal, r.signal,
            "SIGPIPE=SIG_DFL, input {input:?}: signal mismatch\n  C: {c:?}\n  Rust: {r:?}"
        );
        assert_eq!(c.code, r.code, "SIGPIPE=SIG_DFL, input {input:?}: code mismatch");
        assert_eq!(
            c.stderr, r.stderr,
            "SIGPIPE=SIG_DFL, input {input:?}: stderr mismatch\n  C: {c:?}\n  Rust: {r:?}"
        );
        // Sanity check that this really is the fatal-SIGPIPE path.
        assert_eq!(c.signal, Some(13), "expected C to die of SIGPIPE: {c:?}");
    }
}

/// When SIGPIPE is inherited as SIG_IGN, the C program's fflush just fails and
/// main still returns 0 — with nothing on stderr. The Rust translation must not
/// panic ("failed printing to stdout") in that situation.
#[test]
fn broken_stdout_with_ignored_sigpipe() {
    for input in ["7 2", "", "abc", "-2147483648 1"] {
        let c = run_with_broken_stdout(c_bin(), input.as_bytes(), SIG_IGN);
        let r = run_with_broken_stdout(&rust_bin(), input.as_bytes(), SIG_IGN);
        assert_eq!(
            (c.code, c.signal),
            (r.code, r.signal),
            "SIGPIPE=SIG_IGN, input {input:?}: status mismatch\n  C: {c:?}\n  Rust: {r:?}"
        );
        assert_eq!(
            c.stderr, r.stderr,
            "SIGPIPE=SIG_IGN, input {input:?}: stderr mismatch\n  C: {c:?}\n  Rust: {r:?}"
        );
        // Sanity check: the C program survives and says nothing.
        assert_eq!(c.code, Some(0), "expected C to exit 0: {c:?}");
        assert!(c.stderr.is_empty(), "expected C stderr empty: {c:?}");
        assert!(r.stderr.is_empty(), "Rust must not print a panic message: {r:?}");
    }
}

/// A faulting divisor must still fault when stdout is broken and SIGPIPE is
/// ignored: SIGFPE happens before any printf, so the pipe is irrelevant.
#[test]
fn broken_stdout_still_faults_on_zero_divisor() {
    for input in ["5 0", "-2147483648 -1"] {
        let c = run_with_broken_stdout(c_bin(), input.as_bytes(), SIG_IGN);
        let r = run_with_broken_stdout(&rust_bin(), input.as_bytes(), SIG_IGN);
        assert_eq!(c.signal, Some(8), "expected C SIGFPE on {input:?}: {c:?}");
        assert_eq!((c.code, c.signal), (r.code, r.signal), "status mismatch on {input:?}");
        assert_eq!(c.stderr, r.stderr, "stderr mismatch on {input:?}");
    }
}

// ---------------------------------------------------------------------------
// Phase C — stdin is consumed lazily, exactly as far as scanf needs
// ---------------------------------------------------------------------------

/// Run `exe` with stdin as a pipe that is written to but deliberately LEFT
/// OPEN, so there is no EOF. Returns `(finished_within_timeout, stdout)`.
///
/// This distinguishes "reads only what scanf needs" from "slurps stdin until
/// EOF": the latter would block forever here, while the C program does not.
fn run_without_eof(exe: &Path, stdin_bytes: &[u8]) -> (bool, Vec<u8>) {
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", exe.display()));

    // NOTE: `stdin` is held for the whole function, so the pipe never sees EOF.
    let mut stdin = child.stdin.take().expect("stdin piped");
    stdin.write_all(stdin_bytes).expect("write stdin");
    stdin.flush().expect("flush stdin");

    let mut stdout_handle = child.stdout.take().expect("stdout piped");
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout_handle.read_to_end(&mut buf);
        let _ = tx.send(buf);
    });

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let mut finished = false;
    while std::time::Instant::now() < deadline {
        if child.try_wait().expect("try_wait").is_some() {
            finished = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    if !finished {
        let _ = child.kill();
        let _ = child.wait();
        return (false, Vec::new());
    }

    let out = rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap_or_default();
    (true, out)
}

/// When the number is terminated by a following character, scanf finishes and
/// the program exits without ever waiting for EOF. A translation that used
/// `read_to_end` on stdin would hang here while the C program does not.
#[test]
fn does_not_wait_for_eof_when_input_is_terminated() {
    for input in ["7 2 ", "7 2\n", "10 3\n\n", "7 2 rest of the line\n", "5 0 "] {
        let (c_done, c_out) = run_without_eof(c_bin(), input.as_bytes());
        let (r_done, r_out) = run_without_eof(&rust_bin(), input.as_bytes());
        assert!(c_done, "C should not block on {input:?}");
        assert_eq!(
            c_done, r_done,
            "completion mismatch on {input:?}: C finished={c_done}, Rust finished={r_done}"
        );
        assert_eq!(
            c_out,
            r_out,
            "stdout mismatch on {input:?}\n  C: {:?}\n  Rust: {:?}",
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out),
        );
    }
}

/// The flip side: when the trailing number is NOT terminated, the C program
/// itself blocks waiting for more input. The translation must block too rather
/// than "helpfully" treating the pipe as finished and printing early.
#[test]
fn blocks_exactly_where_c_blocks() {
    for input in ["7 2", "7", "", "  "] {
        let (c_done, _) = run_without_eof(c_bin(), input.as_bytes());
        let (r_done, _) = run_without_eof(&rust_bin(), input.as_bytes());
        assert!(!c_done, "expected C to block on unterminated input {input:?}");
        assert_eq!(
            c_done, r_done,
            "blocking behaviour mismatch on {input:?}: C finished={c_done}, Rust finished={r_done}"
        );
    }
}
