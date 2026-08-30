//! Differential tests: run the original C program and the Rust translation as
//! subprocesses over the same stdin bytes and require byte-identical stdout,
//! byte-identical stderr and an identical exit status.
//!
//! Nothing here links the Rust code as a library; both programs are driven
//! exactly the way a shell would drive them, because that is how the
//! translation is graded.

use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::Once;

/// Path to the Rust binary that Cargo just built for this test.
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn c_src_dir() -> PathBuf {
    // CARGO_MANIFEST_DIR is <repo>/translation; the C tree is its sibling.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .join("c_src")
}

/// Builds the C program once per test binary invocation and returns its path.
fn c_bin() -> PathBuf {
    static BUILD: Once = Once::new();
    let src = c_src_dir();
    let build_dir = src.join("build");
    let exe = build_dir.join("driver");

    BUILD.call_once(|| {
        if exe.exists() {
            return;
        }
        fs::create_dir_all(&build_dir).expect("failed to create c_src/build");
        let configure = Command::new("cmake")
            .arg("..")
            .current_dir(&build_dir)
            .output()
            .expect("failed to run cmake; is it installed?");
        assert!(
            configure.status.success(),
            "cmake configure failed:\n{}",
            String::from_utf8_lossy(&configure.stderr)
        );
        let compile = Command::new("cmake")
            .args(["--build", "."])
            .current_dir(&build_dir)
            .output()
            .expect("failed to run cmake --build");
        assert!(
            compile.status.success(),
            "cmake --build failed:\n{}",
            String::from_utf8_lossy(&compile.stderr)
        );
    });

    assert!(
        exe.exists(),
        "the C executable was not produced at {}",
        exe.display()
    );
    exe
}

/// Runs one program, writing `input` to its stdin and capturing everything.
fn run(program: &Path, input: &[u8]) -> Output {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", program.display()));

    {
        let mut stdin = child.stdin.take().expect("stdin was piped");
        let owned = input.to_vec();
        // Write on a helper thread: the programs can emit far more output than a
        // pipe buffer holds, so writing and reading must not deadlock.
        std::thread::spawn(move || {
            let _ = stdin.write_all(&owned);
            let _ = stdin.flush();
        });
    }

    child
        .wait_with_output()
        .expect("failed to collect program output")
}

fn describe(bytes: &[u8]) -> String {
    let shown: String = bytes
        .iter()
        .take(80)
        .flat_map(|b| std::ascii::escape_default(*b).map(char::from))
        .collect();
    if bytes.len() > 80 {
        format!("\"{shown}\"... ({} bytes total)", bytes.len())
    } else {
        format!("\"{shown}\"")
    }
}

/// The core assertion: identical stdout, stderr and exit status.
fn assert_same(case: &str, input: &[u8]) {
    let c = run(&c_bin(), input);
    let r = run(&rust_bin(), input);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch for case `{case}` with input {}\n  C   ({} bytes): {}\n  Rust({} bytes): {}",
        describe(input),
        c.stdout.len(),
        describe(&c.stdout),
        r.stdout.len(),
        describe(&r.stdout),
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch for case `{case}` with input {}\n  C   : {}\n  Rust: {}",
        describe(input),
        describe(&c.stderr),
        describe(&r.stderr),
    );
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "exit status mismatch for case `{case}` with input {}: C {:?} vs Rust {:?}",
        describe(input),
        c.status,
        r.status,
    );
}

// ---------------------------------------------------------------------------
// The input classes the C program branches on.
//
//   main:    scanf("%d", &x) either converts (x set) or fails/EOFs (x stays 0)
//   driver:  the `i < x` loop runs zero times for x <= 0, otherwise x times
// ---------------------------------------------------------------------------

/// scanf hits EOF before any conversion: x keeps its initialiser of 0, so the
/// loop body never runs.
#[test]
fn empty_input() {
    assert_same("empty", b"");
}

/// x == 0 is the exact boundary of `i < x`: no iterations.
#[test]
fn zero() {
    assert_same("zero", b"0");
}

/// A single iteration, the smallest x that produces output.
#[test]
fn one_item() {
    assert_same("one", b"1");
}

#[test]
fn small_counts() {
    for n in 2..=12 {
        assert_same(&format!("count {n}"), n.to_string().as_bytes());
    }
}

/// Negative x: the loop is skipped entirely.
#[test]
fn negative_values() {
    for input in ["-1", "-3", "-0", "-2147483648", "-100000"] {
        assert_same(input, input.as_bytes());
    }
}

#[test]
fn explicit_plus_sign() {
    assert_same("+4", b"+4");
    assert_same("+0", b"+0");
}

#[test]
fn leading_zeros_are_decimal() {
    assert_same("007", b"007");
    // A long run of zeros must not be mistaken for an overflow.
    let mut input = vec![b'0'; 5000];
    input.push(b'7');
    assert_same("5000 zeros then 7", &input);
}

/// scanf's %d skips leading whitespace, including newlines, and \v \f \r.
#[test]
fn leading_whitespace_is_skipped() {
    assert_same("spaces then 3", b"   3");
    assert_same("newlines then 3", b"\n\n\n3");
    assert_same("mixed ws then 6", b"\x0b\x0c\r\n\t 6");
    let mut input = vec![b' '; 10_000];
    input.push(b'4');
    assert_same("10000 spaces then 4", &input);
}

/// Whitespace only: EOF is reached while skipping, so no conversion happens.
#[test]
fn whitespace_only() {
    assert_same("spaces only", b"    ");
    assert_same("newline only", b"\n");
    assert_same("all ws chars", b" \t\n\x0b\x0c\r");
}

/// Matching failure: the first non-whitespace byte cannot start an integer, so
/// x is left at 0.
#[test]
fn matching_failure_leaves_x_untouched() {
    for input in ["abc", ".5", "x", "-", "+", "-abc", "--3", "- 3", "+ 1", "/", ":"] {
        assert_same(input, input.as_bytes());
    }
}

/// Conversion stops at the first non-digit; the rest of stdin is never read.
#[test]
fn trailing_junk_after_number() {
    assert_same("3abc", b"3abc");
    assert_same("0x10", b"0x10"); // reads 0, stops at 'x'
    assert_same("1e3", b"1e3");
    assert_same("2.9", b"2.9");
    assert_same("4-5", b"4-5");
}

/// Only one number is ever read; a second one is ignored.
#[test]
fn only_first_number_is_read() {
    assert_same("3 5", b"3 5");
    assert_same("3 then newline 9", b"3\n9\n");
    assert_same("2 with trailing newline", b"2\n");
}

/// Bytes that are neither whitespace nor digits, including embedded NUL and
/// non-ASCII, are matching failures.
#[test]
fn non_ascii_and_nul_bytes() {
    assert_same("lone NUL", b"\0");
    assert_same("NUL before digits", b"\x005");
    assert_same("ws then NUL then digits", b"  \x004");
    assert_same("digits then NUL", b"4\0");
    assert_same("0xFF then 3", b"\xff3");
    assert_same("utf8 text", "é3".as_bytes());
}

/// Values above INT_MAX: glibc converts as a long and the assignment to `int`
/// truncates, so these become negative or zero and print nothing.
#[test]
fn values_beyond_int_range_truncate() {
    for input in [
        "2147483648",           // INT_MAX + 1 -> INT_MIN
        "4294967296",           // 2^32 -> 0
        "2147483647",           // INT_MAX itself: valid, but see loop_upper_bound
        "9223372036854775807",  // LONG_MAX -> -1
        "9223372036854775808",  // LONG_MAX + 1, saturates
        "18446744073709551616", // 2^64, overflows the accumulator
        "-9223372036854775808", // LONG_MIN
        "-9223372036854775809", // below LONG_MIN, saturates
    ] {
        // 2147483647 would loop for hours; it is covered by the dedicated
        // truncation cases below rather than by execution.
        if input == "2147483647" {
            continue;
        }
        assert_same(input, input.as_bytes());
    }
}

/// A digit string long enough to overflow even the u64 accumulator, both signs.
#[test]
fn absurdly_long_digit_strings() {
    let nines = vec![b'9'; 5000];
    assert_same("5000 nines", &nines);
    let mut neg = vec![b'-'];
    neg.extend_from_slice(&nines);
    assert_same("minus 5000 nines", &neg);
}

/// Truncation that lands back in positive territory still drives the loop.
#[test]
fn truncation_that_yields_a_positive_int() {
    assert_same("4294967299", b"4294967299"); // 2^32 + 3 -> 3
    assert_same("4294967306", b"4294967306"); // 2^32 + 10 -> 10
}

/// Larger loop counts exercise printf formatting across many lines, including
/// the widening field as i and j grow.
#[test]
fn large_output() {
    assert_same("1000", b"1000");
    assert_same("65535", b"65535");
}

/// The doubling of j is the other arithmetic path: j = 2*i must match for
/// values large enough to be several digits wider than i.
#[test]
fn j_stays_double_i() {
    let out = run(&c_bin(), b"70000").stdout;
    let text = String::from_utf8(out).expect("output is ASCII");
    let last = text.lines().last().expect("there is output");
    assert_eq!(last, "69999 139998", "C's final line for x=70000");
    assert_same("70000", b"70000");
}

/// stdout closed early: the C program inherits SIGPIPE's default disposition and
/// dies by signal. The Rust runtime ignores SIGPIPE by default, so this checks
/// the translation restored it.
#[test]
fn broken_stdout_pipe_kills_both() {
    fn status_after_closing_stdout(program: &Path) -> std::process::ExitStatus {
        let mut child = Command::new(program)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn");
        child
            .stdin
            .take()
            .expect("stdin")
            .write_all(b"65535")
            .expect("write stdin");
        {
            // Read a little, then drop the pipe so further writes see EPIPE.
            let mut out = child.stdout.take().expect("stdout");
            let mut buf = [0u8; 5];
            let _ = out.read(&mut buf);
        }
        child.wait().expect("wait")
    }

    let c = status_after_closing_stdout(&c_bin());
    let r = status_after_closing_stdout(&rust_bin());
    assert_eq!(
        c.code(),
        r.code(),
        "exit code on broken pipe: C {c:?} vs Rust {r:?}"
    );
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(
            c.signal(),
            r.signal(),
            "terminating signal on broken pipe: C {c:?} vs Rust {r:?}"
        );
    }
}

/// scanf consumes only the bytes its conversion used, pushing back the single
/// byte that terminated it. Because glibc repositions a seekable stream at exit,
/// a later reader of the same descriptor sees the remainder; the translation
/// must leave the shared file offset in the same place.
#[test]
fn stdin_is_consumed_to_the_same_offset() {
    let dir = std::env::temp_dir().join("driver_offset_cases");
    fs::create_dir_all(&dir).expect("create temp dir");

    let mut cases: Vec<(String, Vec<u8>)> = vec![
        ("number then space".into(), b"2 REMAINDER".to_vec()),
        ("two digits then junk".into(), b"25 X".to_vec()),
        ("exact number".into(), b"25".to_vec()),
        ("ws then letters".into(), b"   abcdef".to_vec()),
        ("sign then letters".into(), b"-abc".to_vec()),
        ("lone sign".into(), b"-".to_vec()),
        ("plus then letters".into(), b"+xy".to_vec()),
        ("digit then letters".into(), b"3abc".to_vec()),
        ("hex like".into(), b"0x10".to_vec()),
        ("leading dot".into(), b".5".to_vec()),
        ("newline separated".into(), b"3\n4".to_vec()),
        ("ws then number then rest".into(), b"  \n 7 rest".to_vec()),
        ("empty".into(), b"".to_vec()),
    ];
    // Larger than any plausible stdio buffer, to catch read-ahead differences.
    let mut big = b"2 R".to_vec();
    big.extend(std::iter::repeat(b'x').take(6000));
    cases.push(("6KB input".into(), big));

    for (i, (name, input)) in cases.iter().enumerate() {
        let path = dir.join(format!("case{i}.bin"));
        fs::write(&path, input).expect("write case file");

        let offset_after = |program: &Path| -> u64 {
            let mut file = fs::File::open(&path).expect("open case file");
            file.seek(SeekFrom::Start(0)).expect("rewind");
            // The child gets a dup of this descriptor, so they share one offset.
            let status = Command::new(program)
                .stdin(Stdio::from(file.try_clone().expect("dup")))
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .expect("run program");
            assert!(status.success(), "{} exited with {status:?}", program.display());
            file.stream_position().expect("read shared offset")
        };

        assert_eq!(
            offset_after(&c_bin()),
            offset_after(&rust_bin()),
            "stdin bytes consumed differ for case `{name}` (input {})",
            describe(input),
        );
    }
}
