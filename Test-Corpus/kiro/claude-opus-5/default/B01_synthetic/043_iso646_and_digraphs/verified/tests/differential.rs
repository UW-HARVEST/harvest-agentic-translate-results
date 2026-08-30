//! Differential tests: run the original C program and the Rust translation as
//! subprocesses with identical stdin and compare stdout, stderr and the exit
//! status byte for byte.
//!
//! The Rust code is never called as a library here; only the built binary is
//! driven, exactly the way a shell would drive it.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;

/// Path of the Rust binary under test, provided by Cargo.
const RUST_BIN: &str = env!("CARGO_BIN_EXE_driver");

fn workspace_root() -> PathBuf {
    // translation/ -> repository root
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Configure and build `c_src` with CMake once per test binary, returning the
/// path of the resulting `driver` executable.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = workspace_root().join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");
        if exe.is_file() {
            return exe;
        }

        std::fs::create_dir_all(&build).expect("cannot create c_src/build");

        let configure = Command::new("cmake")
            .arg("..")
            .current_dir(&build)
            .output()
            .expect("failed to run cmake (is it installed?)");
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
            .expect("failed to run cmake --build");
        assert!(
            compile.status.success(),
            "cmake --build failed:\n{}\n{}",
            String::from_utf8_lossy(&compile.stdout),
            String::from_utf8_lossy(&compile.stderr)
        );

        assert!(exe.is_file(), "C driver was not produced at {:?}", exe);
        exe
    })
}

/// Run `exe` with `input` on stdin and capture everything.
fn run(exe: &Path, input: &[u8]) -> Output {
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {exe:?}: {e}"));

    {
        let mut stdin = child.stdin.take().expect("stdin was piped");
        // The program may exit before consuming all of stdin (it only reads two
        // integers), which shows up as EPIPE here; that is not a test failure.
        let _ = stdin.write_all(input);
        let _ = stdin.flush();
    }

    child.wait_with_output().expect("failed to collect output")
}

fn describe(status: std::process::ExitStatus) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(sig) = status.signal() {
            return format!("killed by signal {sig}");
        }
    }
    match status.code() {
        Some(c) => format!("exit code {c}"),
        None => "unknown termination".to_string(),
    }
}

/// Assert that both programs agree on stdout, stderr and exit status.
fn assert_same(label: &str, input: &[u8]) {
    let c = run(c_bin(), input);
    let r = run(Path::new(RUST_BIN), input);

    assert_eq!(
        c.stdout,
        r.stdout,
        "[{label}] stdout differs for input {input:?}\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "[{label}] stderr differs for input {input:?}\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
    assert_eq!(
        describe(c.status),
        describe(r.status),
        "[{label}] exit status differs for input {input:?}"
    );
}

fn assert_all(cases: &[(&str, &[u8])]) {
    for (label, input) in cases {
        assert_same(label, input);
    }
}

// ---------------------------------------------------------------------------
// Phase A sanity: both binaries exist, run, and produce the documented result.
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_run() {
    let c = run(c_bin(), b"5 7");
    let r = run(Path::new(RUST_BIN), b"5 7");
    // driver(5, 7) = 5 | ~7 = -3, printed with a trailing newline from puts("").
    assert_eq!(c.stdout, b"-3\n", "C reference output changed unexpectedly");
    assert_eq!(r.stdout, c.stdout);
    assert_eq!(r.stderr, c.stderr);
    assert_eq!(describe(r.status), describe(c.status));
}

// ---------------------------------------------------------------------------
// Phase B: the input classes the C program's behaviour actually depends on.
// The only control flow in the C is inside the two scanf("%d") calls, whose
// return values are discarded; every distinguishable class is therefore a
// scanf outcome (success / matching failure / input failure) crossed with the
// integer conversion rules.
// ---------------------------------------------------------------------------

#[test]
fn no_input_and_whitespace_only() {
    assert_all(&[
        ("empty", b""),
        ("single space", b" "),
        ("spaces", b"     "),
        ("newlines", b"\n\n\n"),
        ("all c whitespace", b" \t\n\r\x0b\x0c"),
        ("5000 spaces", &[b' '; 5000]),
    ]);
}

#[test]
fn one_integer_only_second_scanf_hits_eof() {
    assert_all(&[
        ("bare 5", b"5"),
        ("5 with newline", b"5\n"),
        ("5 with trailing spaces", b"5   "),
        ("0", b"0"),
        ("-1", b"-1"),
        ("leading ws then one int", b"\n\t  42"),
    ]);
}

#[test]
fn two_integers_various_separators() {
    assert_all(&[
        ("space", b"5 7"),
        ("newline", b"5\n7\n"),
        ("tabs", b"\t\t1\t\t2"),
        ("crlf", b"1\r\n2\r\n"),
        ("vertical tab and form feed", b"\x0b1\x0c2"),
        ("many blank lines", b"   \n\t 5 \n\n\n  7  \n"),
        ("no trailing newline", b"1 2"),
        ("zeros", b"0 0"),
        ("both negative", b"-5 -7"),
        ("both explicitly positive", b"+5 +7"),
        ("mixed signs", b"-5 +7"),
        ("leading zeros", b"007 0008"),
        ("28 leading zeros", b"00000000000000000000000000005 3"),
        ("negative zero", b"-0 -0"),
        ("negative zeros", b"-000000 1"),
    ]);
}

#[test]
fn scanf_reads_across_newlines_unlike_fgets() {
    // A line-oriented reader would stop at the first newline and leave y at 0;
    // scanf("%d") skips the newline and reads the second integer.
    assert_all(&[
        ("int newline int", b"11\n22"),
        ("int blank lines int", b"11\n\n\n22"),
        ("int crlf int", b"11\r\n22"),
        ("sign split across lines", b"-\n5"),
    ]);
}

#[test]
fn trailing_input_after_two_integers_is_ignored() {
    assert_all(&[
        ("five integers", b"1 2 3 4 5"),
        ("two then junk", b"1 2 junk\n"),
        ("two then huge tail", b"1 2 \x00\x00\xff\xfe rest of line\n"),
        ("two then nul", b"1 2\x00"),
    ]);
}

// ---------------------------------------------------------------------------
// Phase B/C: matching failures. scanf leaves the variable at its initial 0 and
// the discarded return value hides the failure.
// ---------------------------------------------------------------------------

#[test]
fn matching_failure_on_non_numeric_input() {
    assert_all(&[
        ("letters", b"abc"),
        ("two words", b"abc def"),
        ("comma separated", b"5,7"),
        ("hex literal", b"0x10 5"),
        ("digits then letters", b"12abc34"),
        ("decimal point first", b".5 .7"),
        ("scientific notation", b"1e5 2"),
        ("underscore separator", b"1_2 3"),
        ("punctuation", b"*/ */"),
        ("nul byte first", b"\x005 6"),
        ("nul between digits", b"5\x006"),
        ("nul as separator", b"1 \x00 2"),
        ("non-ascii digits", "\u{ff15} \u{ff17}".as_bytes()),
    ]);
}

#[test]
fn sign_without_digits() {
    // The sign is consumed, then the conversion fails. glibc restores enough of
    // the input that the *second* scanf can still see a pushed-back character,
    // which is why these cases are not all equivalent.
    assert_all(&[
        ("minus at eof", b"-"),
        ("plus at eof", b"+"),
        ("minus newline", b"-\n"),
        ("minus then letter", b"-a 5"),
        ("minus space five", b"- 5"),
        ("plus space five", b"+ 5"),
        ("double minus", b"--5 1"),
        ("double plus", b"++5 1"),
        ("plus minus", b"+-5 1"),
        ("minus plus", b"-+5 1"),
        ("five then minus", b"5-"),
        ("five space minus", b"5 -"),
        ("sign only twice", b"- -"),
    ]);
}

// ---------------------------------------------------------------------------
// Phase C: the integer conversion limits. glibc converts "%d" through strtol,
// saturating at long, and then truncates the low 32 bits into the int.
// ---------------------------------------------------------------------------

#[test]
fn int_boundaries() {
    assert_all(&[
        ("INT_MAX", b"2147483647 2147483647"),
        ("INT_MIN", b"-2147483648 -2147483648"),
        ("INT_MAX and INT_MIN", b"2147483647 -2147483648"),
        ("INT_MAX plus one", b"2147483648 2147483648"),
        ("INT_MIN minus one", b"-2147483649 -2147483649"),
        ("UINT_MAX", b"4294967295 4294967295"),
        ("UINT_MAX plus one", b"4294967296 4294967296"),
        ("one and INT_MIN", b"1 -2147483648"),
        ("INT_MIN and one", b"-2147483648 1"),
    ]);
}

#[test]
fn long_boundaries_and_overflow_truncation() {
    assert_all(&[
        ("LONG_MAX", b"9223372036854775807 9223372036854775807"),
        ("LONG_MAX plus one", b"9223372036854775808 9223372036854775808"),
        ("LONG_MIN", b"-9223372036854775808 -9223372036854775808"),
        ("LONG_MIN minus one", b"-9223372036854775809 -9223372036854775809"),
        ("19 digits", b"1234567890123456789 1"),
        ("20 digits", b"12345678901234567890 1"),
        ("30 nines", b"999999999999999999999999999999 1"),
        ("30 nines negative", b"-999999999999999999999999999999 1"),
        ("1 followed by 40 zeros", b"10000000000000000000000000000000000000000 1"),
        ("saturated then valid", b"99999999999999999999 -7"),
    ]);
}

#[test]
fn very_long_digit_runs() {
    let nines = vec![b'9'; 1000];
    let mut pos = nines.clone();
    pos.extend_from_slice(b" 1");
    let mut neg = vec![b'-'];
    neg.extend_from_slice(&nines);
    neg.extend_from_slice(b" 1");
    let mut zeros = vec![b'0'; 4096];
    zeros.extend_from_slice(b"7 8");

    assert_all(&[
        ("1000 nines", &pos),
        ("1000 nines negative", &neg),
        ("4096 leading zeros", &zeros),
    ]);
}

#[test]
fn every_bit_pattern_class_of_the_or_not_expression() {
    // driver computes x | ~y; exercise the sign and bit-pattern combinations.
    let cases: &[(&str, &[u8])] = &[
        ("both max", b"2147483647 2147483647"),
        ("x max y min", b"2147483647 -2147483648"),
        ("x min y max", b"-2147483648 2147483647"),
        ("both min", b"-2147483648 -2147483648"),
        ("x zero y max", b"0 2147483647"),
        ("x zero y min", b"0 -2147483648"),
        ("x zero y neg one", b"0 -1"),
        ("x neg one y zero", b"-1 0"),
        ("alternating bits", b"1431655765 -1431655766"),
        ("powers of two", b"65536 65536"),
        ("mid values", b"123456789 987654321"),
    ];
    assert_all(cases);
}

// ---------------------------------------------------------------------------
// Phase C: environment-driven paths that are not stdin bytes.
// ---------------------------------------------------------------------------

#[test]
fn command_line_arguments_are_ignored() {
    let c = Command::new(c_bin())
        .args(["a", "b", "c"])
        .stdin(Stdio::null())
        .output()
        .expect("C run failed");
    let r = Command::new(RUST_BIN)
        .args(["a", "b", "c"])
        .stdin(Stdio::null())
        .output()
        .expect("Rust run failed");
    assert_eq!(c.stdout, r.stdout);
    assert_eq!(c.stderr, r.stderr);
    assert_eq!(describe(c.status), describe(r.status));
}

#[test]
fn stdin_from_dev_null() {
    let out = |exe: &Path| {
        Command::new(exe)
            .stdin(Stdio::null())
            .output()
            .expect("run failed")
    };
    let c = out(c_bin());
    let r = out(Path::new(RUST_BIN));
    assert_eq!(c.stdout, r.stdout);
    assert_eq!(c.stderr, r.stderr);
    assert_eq!(describe(c.status), describe(r.status));
}

/// C ignores the return value of printf/puts, so a failing write must not
/// change the exit status or print anything on stderr.
#[cfg(unix)]
#[test]
fn write_error_on_full_device_is_ignored() {
    if !Path::new("/dev/full").exists() {
        return;
    }
    let run_full = |exe: &Path| {
        let sink = std::fs::OpenOptions::new()
            .write(true)
            .open("/dev/full")
            .expect("cannot open /dev/full");
        let mut child = Command::new(exe)
            .stdin(Stdio::piped())
            .stdout(Stdio::from(sink))
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn failed");
        {
            let mut stdin = child.stdin.take().unwrap();
            let _ = stdin.write_all(b"5 7");
        }
        child.wait_with_output().expect("wait failed")
    };
    let c = run_full(c_bin());
    let r = run_full(Path::new(RUST_BIN));
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr differs when stdout is /dev/full\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
    assert_eq!(
        describe(c.status),
        describe(r.status),
        "exit status differs when stdout is /dev/full"
    );
}

/// Writing to a pipe with no reader kills the C program with SIGPIPE. Rust
/// ignores SIGPIPE by default, so the translation has to restore it.
#[cfg(unix)]
#[test]
fn broken_pipe_terminates_the_same_way() {
    use std::os::unix::io::FromRawFd;

    extern "C" {
        fn pipe(fds: *mut i32) -> i32;
        fn close(fd: i32) -> i32;
    }

    let run_broken = |exe: &Path| {
        let mut fds = [0i32; 2];
        assert_eq!(unsafe { pipe(fds.as_mut_ptr()) }, 0, "pipe() failed");
        // Close the read end *before* spawning, so every write gets EPIPE.
        assert_eq!(unsafe { close(fds[0]) }, 0, "close() failed");
        let write_end = unsafe { Stdio::from_raw_fd(fds[1]) };

        let mut child = Command::new(exe)
            .stdin(Stdio::piped())
            .stdout(write_end)
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn failed");
        {
            let mut stdin = child.stdin.take().unwrap();
            let _ = stdin.write_all(b"5 7");
        }
        child.wait_with_output().expect("wait failed")
    };

    let c = run_broken(c_bin());
    let r = run_broken(Path::new(RUST_BIN));
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr differs on a broken stdout pipe\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
    assert_eq!(
        describe(c.status),
        describe(r.status),
        "termination differs on a broken stdout pipe"
    );
}

// ---------------------------------------------------------------------------
// Phase C: randomized differential sweep over the byte alphabet the parser
// branches on, plus a deterministic sweep over integer pairs.
// ---------------------------------------------------------------------------

#[test]
fn deterministic_integer_pair_sweep() {
    const VALUES: &[i64] = &[
        0,
        1,
        -1,
        2,
        -2,
        7,
        -7,
        255,
        256,
        -256,
        65535,
        65536,
        1_000_000,
        i32::MAX as i64,
        i32::MIN as i64,
        i32::MAX as i64 + 1,
        i32::MIN as i64 - 1,
        u32::MAX as i64,
        u32::MAX as i64 + 1,
        i64::MAX,
        i64::MIN,
    ];
    for &x in VALUES {
        for &y in VALUES {
            let input = format!("{x} {y}\n");
            assert_same(&format!("pair {x} {y}"), input.as_bytes());
        }
    }
}

#[test]
fn randomized_byte_sweep() {
    // Small xorshift PRNG: deterministic, no dev-dependencies.
    let mut state: u64 = 0x9E3779B97F4A7C15;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    const ALPHABET: &[u8] = b"0123456789 \t\n\r\x0b\x0c+-abcxXeE.,_/*\x00\xff";

    for i in 0..400 {
        let len = (next() % 15) as usize;
        let input: Vec<u8> = (0..len)
            .map(|_| ALPHABET[(next() % ALPHABET.len() as u64) as usize])
            .collect();
        assert_same(&format!("random #{i}"), &input);
    }
}
