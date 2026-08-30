//! Differential tests: run the original C program and the Rust translation as
//! subprocesses with identical stdin, and require byte-identical stdout,
//! byte-identical stderr and an identical exit status.
//!
//! The Rust code is NEVER called as a library here — only the built binary is
//! driven, exactly the way a shell would drive it, because that is how the two
//! programs are compared.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// Repository root (the directory holding both `c_src/` and `translation/`).
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the Rust binary under test, as produced by cargo.
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Path to the compiled C binary, building it with CMake on first use.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = repo_root().join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");
        if exe.is_file() {
            return exe;
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
        assert!(exe.is_file(), "C binary missing after build: {}", exe.display());
        exe
    })
    .as_path()
}

/// Full termination status: the normal exit code *and* the terminating signal.
/// Comparing only `code()` would silently equate "killed by SIGPIPE" (None) on
/// one side with "killed by SIGSEGV" (also None) on the other.
#[derive(Debug, PartialEq, Eq)]
struct Status {
    code: Option<i32>,
    signal: Option<i32>,
}

struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    status: Status,
}

fn invoke(program: &Path, stdin_bytes: &[u8]) -> Outcome {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", program.display()));

    {
        let mut sin = child.stdin.take().expect("stdin pipe");
        // Both programs read at most one short line; the write may fail with
        // EPIPE if the child exits first, which is not an error for the test.
        let _ = sin.write_all(stdin_bytes);
        let _ = sin.flush();
    }

    let out = child.wait_with_output().expect("wait for child");
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        status: to_status(&out.status),
    }
}

fn to_status(s: &std::process::ExitStatus) -> Status {
    use std::os::unix::process::ExitStatusExt;
    Status {
        code: s.code(),
        signal: s.signal(),
    }
}

fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).escape_debug().to_string()
}

/// Core assertion: the C program and the Rust program agree on all three
/// observable channels for this input.
fn assert_same(label: &str, stdin_bytes: &[u8]) {
    let c = invoke(c_bin(), stdin_bytes);
    let r = invoke(&rust_bin(), stdin_bytes);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch for {label} (input {:?})\n  C: \"{}\"\n  R: \"{}\"",
        show(stdin_bytes),
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch for {label} (input {:?})\n  C: \"{}\"\n  R: \"{}\"",
        show(stdin_bytes),
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        c.status, r.status,
        "exit status mismatch for {label} (input {:?}): C={:?} R={:?}",
        show(stdin_bytes),
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
// Sanity: both binaries exist and the expected output shape is produced.
// ---------------------------------------------------------------------------

fn exited_zero() -> Status {
    Status {
        code: Some(0),
        signal: None,
    }
}

#[test]
fn both_binaries_run() {
    let c = invoke(c_bin(), b"1\n");
    let r = invoke(&rust_bin(), b"1\n");
    assert_eq!(c.status, exited_zero());
    assert_eq!(r.status, exited_zero());
    // run() prints 4 lines and is called twice.
    assert_eq!(c.stdout.iter().filter(|&&b| b == b'\n').count(), 8);
    assert_eq!(c.stdout, r.stdout);
}

/// The exact expected text for the success path, derived by hand from the C:
/// globals start at floors=2, bedrooms=5, bathrooms=2.5 and persist across the
/// two `run()` calls.
#[test]
fn golden_success_path_for_one_extra_bedroom() {
    let expected = "\
The house has 2 floors, 5 bedrooms, and 2.5 bathrooms
The house has 3 floors, 5 bedrooms, and 2.5 bathrooms
The house has 3 floors, 5 bedrooms, and 3.5 bathrooms
The house has 3 floors, 6 bedrooms, and 3.5 bathrooms
The house has 3 floors, 6 bedrooms, and 3.5 bathrooms
The house has 4 floors, 6 bedrooms, and 3.5 bathrooms
The house has 4 floors, 6 bedrooms, and 4.5 bathrooms
The house has 4 floors, 7 bedrooms, and 4.5 bathrooms
";
    let c = invoke(c_bin(), b"1\n");
    let r = invoke(&rust_bin(), b"1\n");
    assert_eq!(String::from_utf8_lossy(&c.stdout), expected);
    assert_eq!(String::from_utf8_lossy(&r.stdout), expected);
    assert!(c.stderr.is_empty() && r.stderr.is_empty());
}

#[test]
fn golden_error_path() {
    let c = invoke(c_bin(), b"nope\n");
    let r = invoke(&rust_bin(), b"nope\n");
    assert_eq!(String::from_utf8_lossy(&c.stdout), "An error occurred\n");
    assert_eq!(String::from_utf8_lossy(&r.stdout), "An error occurred\n");
    // Note: the C prints the error to *stdout*, not stderr, and still exits 0.
    assert!(c.stderr.is_empty() && r.stderr.is_empty());
    assert_eq!(c.status, exited_zero());
    assert_eq!(r.status, exited_zero());
}

// ---------------------------------------------------------------------------
// Phase B — the input classes the C actually branches on.
// ---------------------------------------------------------------------------

/// `fgets` returns NULL and leaves `in` as the empty string => strtol performs
/// no conversion => `endp == str` => error path.
#[test]
fn empty_and_whitespace_only_input() {
    check_all(&[
        ("empty stdin (EOF immediately)", b""),
        ("single newline", b"\n"),
        ("CR LF only", b"\r\n"),
        ("single space", b" "),
        ("spaces then newline", b"    \n"),
        ("tabs only", b"\t\t\t\n"),
        ("all isspace chars", b" \t\n\x0b\x0c\r"),
        ("vertical tab + form feed", b"\x0b\x0c\n"),
    ]);
}

/// A single valid item, positive / negative / zero / signed.
#[test]
fn single_valid_value() {
    check_all(&[
        ("zero", b"0\n"),
        ("one", b"1\n"),
        ("minus one", b"-1\n"),
        ("plus one", b"+1\n"),
        ("no trailing newline", b"7"),
        ("multi digit", b"12345\n"),
        ("negative multi digit", b"-12345\n"),
        ("leading zeros", b"000000000000000000000000042\n"),
        ("minus zero", b"-0\n"),
        ("plus zero", b"+0\n"),
    ]);
}

/// strtol skips leading whitespace, so these all succeed.
#[test]
fn leading_whitespace_is_skipped_by_strtol() {
    check_all(&[
        ("spaces then number", b"   12\n"),
        ("tab then number", b"\t12\n"),
        ("mixed whitespace then number", b"  \t \x0b\x0c 12\n"),
        ("leading newline then number", b"\n42\n"),
        ("whitespace before sign", b"  -9\n"),
        ("CR before number", b"\r5\n"),
    ]);
}

/// `endp != str` only requires that *some* digits were consumed, so trailing
/// garbage is accepted and silently ignored.
#[test]
fn trailing_garbage_is_accepted() {
    check_all(&[
        ("digits then letters", b"42abc\n"),
        ("digits then space then digits", b"1 2\n"),
        ("digits then dot digits", b"3.9\n"),
        ("hex literal parsed as base 10", b"0x10\n"),
        ("digits then punctuation", b"8!!!\n"),
        ("number then whitespace", b"6   \n"),
        ("digits then sign", b"5-3\n"),
        ("exponent notation", b"2e5\n"),
    ]);
}

/// No digits at all => `endp == str` => error path.
#[test]
fn no_conversion_performed() {
    check_all(&[
        ("letters", b"abc\n"),
        ("sign only, minus", b"-\n"),
        ("sign only, plus", b"+\n"),
        ("double minus", b"--5\n"),
        ("double plus", b"++5\n"),
        ("sign then letter", b"-a\n"),
        ("leading dot", b".5\n"),
        ("only punctuation", b"!!!\n"),
        ("x then digits", b"x1\n"),
        ("comma separated", b",1\n"),
        ("underscore", b"_1\n"),
        ("space between sign and digits", b"- 1\n"),
    ]);
}

/// The `tmp >= INT_MIN && tmp <= INT_MAX` range check (in-range boundaries and
/// the first out-of-range values on either side, which take the error path even
/// though strtol itself succeeded with errno == 0).
#[test]
fn int_range_boundaries() {
    check_all(&[
        ("INT_MAX", b"2147483647\n"),
        ("INT_MAX - 1", b"2147483646\n"),
        ("INT_MIN", b"-2147483648\n"),
        ("INT_MIN + 1", b"-2147483647\n"),
        ("INT_MAX + 1 -> rejected", b"2147483648\n"),
        ("INT_MIN - 1 -> rejected", b"-2147483649\n"),
        ("2^31 + big -> rejected", b"4294967296\n"),
        ("2^30", b"1073741824\n"),
        ("-2^30", b"-1073741824\n"),
    ]);
}

/// Signed overflow of `bedrooms += extra_bedrooms`, which happens twice because
/// `run()` is called twice. This is UB in C; the test pins whatever the compiled
/// C binary actually does.
#[test]
fn bedroom_addition_overflow() {
    check_all(&[
        ("INT_MAX overflows bedrooms", b"2147483647\n"),
        ("INT_MIN overflows bedrooms", b"-2147483648\n"),
        ("just below overflow", b"2147483642\n"),
        ("exactly at overflow edge", b"2147483643\n"),
        ("second run overflows", b"1073741824\n"),
        ("negative overflow on second run", b"-1073741824\n"),
    ]);
}

/// strtol sets errno == ERANGE, so the `errno == 0` check fails.
#[test]
fn strtol_erange() {
    check_all(&[
        ("LONG_MAX (fits long, exceeds int)", b"9223372036854775807\n"),
        ("LONG_MIN (fits long, exceeds int)", b"-9223372036854775808\n"),
        ("LONG_MAX + 1 -> ERANGE", b"9223372036854775808\n"),
        ("LONG_MIN - 1 -> ERANGE", b"-9223372036854775809\n"),
        ("20 nines -> ERANGE", b"99999999999999999999\n"),
        ("negative 20 nines -> ERANGE", b"-99999999999999999999\n"),
        ("huge with trailing junk", b"123456789012345678901234567890abc\n"),
    ]);
}

// ---------------------------------------------------------------------------
// Phase C — paths not covered above: buffer limits, embedded NULs, non-UTF-8,
// extra input after the first line, and closed/odd streams.
// ---------------------------------------------------------------------------

/// `fgets(in, sizeof(in), stdin)` with `char in[100]` reads at most 99 bytes and
/// leaves the rest of the line in the stream, so long lines are truncated.
#[test]
fn fgets_100_byte_buffer_limit() {
    let ninety_nine_ones = vec![b'1'; 99];
    let mut ninety_nine_ones_nl = ninety_nine_ones.clone();
    ninety_nine_ones_nl.push(b'\n');

    let hundred_ones = vec![b'1'; 100];

    // Truncation lands in the middle of the number: only " "*95 + "1234" is read.
    let mut split_number = vec![b' '; 95];
    split_number.extend_from_slice(b"1234567\n");

    // 99 spaces fill the buffer entirely; nothing but whitespace is seen.
    let mut spaces_99 = vec![b' '; 99];
    spaces_99.extend_from_slice(b"42\n");

    // 98 spaces + "4" fits a single digit into the buffer.
    let mut spaces_98 = vec![b' '; 98];
    spaces_98.extend_from_slice(b"42\n");

    let many_nines = vec![b'9'; 200];
    let many_letters = {
        let mut v = vec![b'a'; 150];
        v.push(b'\n');
        v
    };
    // Exactly 98 digits + newline fits with the newline inside the buffer.
    let exact_98 = {
        let mut v = vec![b'7'; 98];
        v.push(b'\n');
        v
    };

    check_all(&[
        ("99 ones (ERANGE after truncation)", &ninety_nine_ones),
        ("99 ones + newline", &ninety_nine_ones_nl),
        ("100 ones (buffer full, no newline read)", &hundred_ones),
        ("truncation splits the number", &split_number),
        ("99 spaces fill buffer, digits unreachable", &spaces_99),
        ("98 spaces then one digit reachable", &spaces_98),
        ("200 nines, no newline", &many_nines),
        ("150 letters", &many_letters),
        ("98 digits + newline", &exact_98),
    ]);
}

/// An embedded NUL terminates the C string early even though `fgets` copied the
/// bytes after it into the buffer.
#[test]
fn embedded_nul_bytes() {
    check_all(&[
        ("NUL first, digits after", b"\x0042\n"),
        ("digits then NUL then junk", b"42\x00abc\n"),
        ("NUL then newline", b"\x00\n"),
        ("spaces, NUL, digits", b"  \x0042\n"),
        ("sign, NUL, digits", b"-\x0042\n"),
        ("digit, NUL, digits", b"1\x0023\n"),
        ("only a NUL", b"\x00"),
    ]);
}

/// The program must not choke on bytes that are not valid UTF-8.
#[test]
fn non_utf8_bytes() {
    check_all(&[
        ("invalid UTF-8 prefix", b"\xff\xfe42\n"),
        ("invalid UTF-8 suffix", b"42\xff\xfe\n"),
        ("lone continuation byte", b"\x8042\n"),
        ("high bytes only", b"\xc3\x28\n"),
        ("latin-1 minus sign", b"\xad5\n"),
        ("all high bytes", b"\xff\xff\xff\xff"),
    ]);
}

/// Only the first line is ever read; everything after it is ignored.
#[test]
fn extra_input_after_first_line() {
    check_all(&[
        ("valid then another number", b"5\n99\n"),
        ("valid then garbage", b"5\nnot a number\n"),
        ("garbage then valid", b"nope\n5\n"),
        ("empty first line then valid", b"\n5\n"),
        ("many lines", b"3\n4\n5\n6\n7\n"),
        ("first line empty, huge rest", b"\n99999999999999999999\n"),
    ]);
}

/// Sweep every decimal digit as a first character plus a spread of magnitudes,
/// so the `%d` / `%.1f` formatting is compared across many values.
#[test]
fn many_values_sweep() {
    let mut owned: Vec<(String, Vec<u8>)> = Vec::new();
    for d in 0..10i64 {
        owned.push((format!("digit {d}"), format!("{d}\n").into_bytes()));
    }
    for &v in &[
        -2147483648i64,
        -2147483647,
        -1000000,
        -12345,
        -100,
        -7,
        -2,
        2,
        7,
        100,
        12345,
        1000000,
        2147483646,
        2147483647,
        -2147483649,
        2147483648,
        10000000000,
    ] {
        owned.push((format!("value {v}"), format!("{v}\n").into_bytes()));
    }
    let cases: Vec<(&str, &[u8])> = owned
        .iter()
        .map(|(l, b)| (l.as_str(), b.as_slice()))
        .collect();
    check_all(&cases);
}

/// Degenerate stream setups: stdin closed with no data at all, and stdin that is
/// not readable.
#[test]
fn stdin_closed_immediately() {
    // Stdio::null() gives an immediate EOF, the same as an empty pipe.
    for prog in [c_bin().to_path_buf(), rust_bin()] {
        let out = Command::new(&prog)
            .stdin(Stdio::null())
            .output()
            .expect("spawn with null stdin");
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "An error occurred\n",
            "{} with /dev/null stdin",
            prog.display()
        );
        assert!(out.stderr.is_empty());
        assert_eq!(to_status(&out.status), exited_zero());
    }
}

/// stdout is a pipe whose read end is already closed. The C program's stdio
/// flush at exit gets EPIPE and, because the C process has the default SIGPIPE
/// disposition, the process is *killed by signal 13* rather than exiting 0. The
/// Rust runtime ignores SIGPIPE by default, so the translation has to restore
/// SIG_DFL to reproduce this.
#[test]
fn writing_to_a_closed_stdout_is_killed_by_sigpipe() {
    use std::os::unix::io::FromRawFd;

    fn run_with_dead_reader(prog: &Path, stdin_bytes: &[u8]) -> Status {
        // Create a pipe, hand the write end to the child, and close both ends
        // here so that no reader remains.
        let mut fds = [0i32; 2];
        extern "C" {
            fn pipe2(fds: *mut i32, flags: i32) -> i32;
            fn close(fd: i32) -> i32;
        }
        // O_CLOEXEC, so the child does not inherit the read end and thereby keep
        // the pipe's read side alive inside itself.
        const O_CLOEXEC: i32 = 0o2000000;
        assert_eq!(
            unsafe { pipe2(fds.as_mut_ptr(), O_CLOEXEC) },
            0,
            "pipe2() failed"
        );
        let (read_end, write_end) = (fds[0], fds[1]);

        let stdout: Stdio = unsafe { std::fs::File::from_raw_fd(write_end) }.into();
        let mut child = Command::new(prog)
            .stdin(Stdio::piped())
            .stdout(stdout)
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn child with pipe stdout");

        // Drop every descriptor for the pipe on this side, including the reader.
        unsafe { close(read_end) };

        {
            let mut sin = child.stdin.take().expect("stdin pipe");
            let _ = sin.write_all(stdin_bytes);
        }
        let out = child.wait_with_output().expect("wait for child");
        to_status(&out.status)
    }

    for input in [&b"3\n"[..], &b"nope\n"[..], &b""[..]] {
        let c = run_with_dead_reader(c_bin(), input);
        let r = run_with_dead_reader(&rust_bin(), input);
        assert_eq!(
            c, r,
            "closed-stdout status mismatch for input {:?}: C={c:?} R={r:?}",
            show(input)
        );
        assert_eq!(
            c,
            Status {
                code: None,
                signal: Some(13),
            },
            "expected the C program to die of SIGPIPE for input {:?}",
            show(input)
        );
    }
}

/// stdout points at /dev/full, so the flush fails with ENOSPC. C's stdio does
/// not report a failed flush from `exit`, so the program still exits 0.
#[test]
fn write_error_other_than_epipe_is_ignored() {
    let dev_full = Path::new("/dev/full");
    if !dev_full.exists() {
        // /dev/full is Linux-specific; there is nothing to compare without it.
        // Fall back to comparing the two programs on a plain pipe instead.
        assert_same("fallback: plain pipe", b"3\n");
        return;
    }
    for prog in [c_bin().to_path_buf(), rust_bin()] {
        let out = Command::new(&prog)
            .stdin(Stdio::piped())
            .stdout(std::fs::OpenOptions::new().write(true).open(dev_full).unwrap())
            .stderr(Stdio::piped())
            .spawn()
            .and_then(|mut ch| {
                let _ = ch.stdin.take().unwrap().write_all(b"3\n");
                ch.wait_with_output()
            })
            .expect("run with /dev/full stdout");
        assert!(
            out.stderr.is_empty(),
            "{} printed to stderr on ENOSPC: {}",
            prog.display(),
            show(&out.stderr)
        );
        assert_eq!(
            to_status(&out.status),
            exited_zero(),
            "{} did not exit 0 on ENOSPC",
            prog.display()
        );
    }
}
