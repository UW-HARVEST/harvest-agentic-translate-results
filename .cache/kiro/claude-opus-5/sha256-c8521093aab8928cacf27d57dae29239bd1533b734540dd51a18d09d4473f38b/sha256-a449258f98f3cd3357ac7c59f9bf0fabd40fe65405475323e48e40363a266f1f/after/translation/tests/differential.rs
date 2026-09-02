//! Differential integration tests: run the C `driver` and the Rust `driver`
//! as subprocesses on identical stdin and require byte-identical stdout,
//! byte-identical stderr, and identical exit status.
//!
//! The Rust program is never loaded as a library; it is driven exactly the way
//! a shell drives it, because that is how it is graded.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Path to the Rust binary produced for this test run.
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Path to the C binary built by `cmake` under `c_src/build`.
fn c_bin() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest
        .parent()
        .expect("translation/ must have a parent directory");
    let candidates = [
        root.join("c_src/build/driver"),
        root.join("c_src/build/Debug/driver"),
        root.join("c_src/build/Release/driver"),
    ];
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "C binary not found. Build it first:\n  \
         cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .\n\
         Looked in: {:?}",
        candidates
    );
}

struct Output {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    status: Option<i32>,
    signal: Option<i32>,
}

fn run(bin: &Path, stdin_bytes: &[u8]) -> Output {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    {
        let mut sin = child.stdin.take().expect("stdin pipe");
        // The child may exit without draining stdin; a broken pipe here is not
        // a test failure.
        let _ = sin.write_all(stdin_bytes);
        let _ = sin.flush();
    }

    let out = child.wait_with_output().expect("wait_with_output");

    #[cfg(unix)]
    let signal = {
        use std::os::unix::process::ExitStatusExt;
        out.status.signal()
    };
    #[cfg(not(unix))]
    let signal = None;

    Output {
        stdout: out.stdout,
        stderr: out.stderr,
        status: out.status.code(),
        signal,
    }
}

fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).escape_debug().to_string()
}

/// The core assertion: all three channels must be identical.
fn assert_same(name: &str, stdin_bytes: &[u8]) {
    let c = run(&c_bin(), stdin_bytes);
    let r = run(&rust_bin(), stdin_bytes);

    assert_eq!(
        c.stdout,
        r.stdout,
        "[{name}] stdout differs\n  input : {}\n  C   ({} bytes): {}\n  Rust({} bytes): {}",
        show(stdin_bytes),
        c.stdout.len(),
        show(&c.stdout),
        r.stdout.len(),
        show(&r.stdout),
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "[{name}] stderr differs\n  input: {}\n  C   : {}\n  Rust: {}",
        show(stdin_bytes),
        show(&c.stderr),
        show(&r.stderr),
    );
    assert_eq!(
        c.status,
        r.status,
        "[{name}] exit status differs\n  input: {}\n  C: {:?}  Rust: {:?}",
        show(stdin_bytes),
        c.status,
        r.status,
    );
    assert_eq!(
        c.signal,
        r.signal,
        "[{name}] terminating signal differs\n  input: {}\n  C: {:?}  Rust: {:?}",
        show(stdin_bytes),
        c.signal,
        r.signal,
    );
}

fn check_all(cases: &[(&str, &[u8])]) {
    for (name, input) in cases {
        assert_same(name, input);
    }
}

// ---------------------------------------------------------------------------
// Phase B: the input classes the C program actually branches on.
//
// main():           fgets over a 100-byte buffer, then parse_val -> run twice
//                   or the "An error occurred" path.
// parse_val():      endp != str  &&  errno == 0  &&  INT_MIN <= tmp <= INT_MAX
// ---------------------------------------------------------------------------

/// fgets hits EOF immediately: returns NULL and leaves `in` as the ""
/// initialiser, so strtol performs no conversion -> error path.
#[test]
fn empty_input() {
    assert_same("empty", b"");
}

/// A single item: the smallest well-formed input.
#[test]
fn single_item() {
    check_all(&[
        ("5\n", b"5\n"),
        ("5 without newline (EOF ends fgets)", b"5"),
        ("0", b"0\n"),
        ("1", b"1\n"),
        ("2", b"2\n"),
    ]);
}

/// strtol reports "no conversion" (endp == str): the error path.
#[test]
fn no_conversion_error_path() {
    check_all(&[
        ("newline only", b"\n"),
        ("spaces only", b"   \n"),
        ("CRLF", b"\r\n"),
        ("tabs only", b"\t\t\n"),
        ("all whitespace kinds", b" \t\n\x0b\x0c\r"),
        ("alpha", b"abc\n"),
        ("uppercase alpha", b"ABC\n"),
        ("leading letter then digit", b"x5\n"),
        ("double minus", b"--5\n"),
        ("plus then minus", b"+-5\n"),
        ("bare minus", b"-\n"),
        ("bare plus", b"+\n"),
        ("leading dot", b".5\n"),
        ("sign then space then digit", b" - 5\n"),
        ("sign then tab then digit", b"-\t5\n"),
        ("plus then space then digit", b"+ 5\n"),
        ("parenthesised", b"(5)\n"),
        ("comma grouped is fine actually", b",1\n"),
        ("literal inf", b"inf\n"),
        ("literal nan", b"nan\n"),
    ]);
}

/// strtol converts a prefix and stops; endp != str so this SUCCEEDS.
#[test]
fn partial_conversion_succeeds() {
    check_all(&[
        ("digits then letters", b"12abc\n"),
        ("hex-looking, base 10 stops at x", b"0x10\n"),
        ("leading zeros", b"007\n"),
        ("thousands comma stops at comma", b"1,000\n"),
        ("scientific notation stops at e", b"5e3\n"),
        ("two numbers, fgets keeps one line", b"5 10\n"),
        ("trailing whitespace", b"5   \n"),
        ("digits then long junk tail", b"5xxxxxxxxxxxxxxxxxxxx\n"),
    ]);
}

/// Signs, leading whitespace, and negative zero.
#[test]
fn signs_and_whitespace() {
    check_all(&[
        ("negative", b"-3\n"),
        ("explicit plus", b"+7\n"),
        ("negative zero", b"-0\n"),
        ("many minus zeros", b"-0000000000\n"),
        ("leading spaces", b"   42\n"),
        ("leading tabs", b"\t\t9\n"),
        ("vertical tab is whitespace to strtol", b"\x0b5\n"),
        ("form feed is whitespace to strtol", b"\x0c5\n"),
        ("mixed whitespace then signed number", b" \t\x0b\x0c\r-8\n"),
        ("minus one", b"-1\n"),
    ]);
}

/// The int range check in parse_val: the maximum the code handles and the
/// first value past it, on both ends.
#[test]
fn int_range_boundaries() {
    check_all(&[
        ("INT_MAX accepted", b"2147483647\n"),
        ("INT_MAX with leading zeros", b"0000002147483647\n"),
        ("INT_MAX with junk tail", b"  +2147483647abc\n"),
        ("INT_MAX-1", b"2147483646\n"),
        ("INT_MIN accepted", b"-2147483648\n"),
        ("INT_MIN+1", b"-2147483647\n"),
        ("INT_MAX+1 rejected", b"2147483648\n"),
        ("INT_MIN-1 rejected", b"-2147483649\n"),
        ("2^30", b"1073741824\n"),
        ("one million", b"1000000\n"),
    ]);
}

/// Values that fit a long but not an int, and values that make strtol set
/// ERANGE so `errno == 0` fails.
#[test]
fn long_range_and_erange() {
    check_all(&[
        ("LONG_MAX: no ERANGE but exceeds INT_MAX", b"9223372036854775807\n"),
        ("LONG_MIN: no ERANGE but below INT_MIN", b"-9223372036854775808\n"),
        ("LONG_MAX+1 sets ERANGE", b"9223372036854775808\n"),
        ("LONG_MIN-1 sets ERANGE", b"-9223372036854775809\n"),
        ("u64 max +1", b"18446744073709551616\n"),
        ("26 nines", b"99999999999999999999999999\n"),
        ("ERANGE with junk tail", b"99999999999999999999abc\n"),
    ]);
}

/// Signed int overflow inside add_bedrooms, performed exactly as the C does.
/// `run` is called twice, so extra_bedrooms is added twice to a global that
/// starts at 5.
#[test]
fn bedroom_overflow_wraps_like_c() {
    check_all(&[
        ("INT_MAX overflows on first add", b"2147483647\n"),
        ("INT_MIN underflows", b"-2147483648\n"),
        ("2147483643 wraps on second add", b"2147483643\n"),
        ("2147483642 boundary", b"2147483642\n"),
        ("-2147483643", b"-2147483643\n"),
        ("half of INT_MAX, wraps on second add", b"1073741823\n"),
        ("1073741822", b"1073741822\n"),
    ]);
}

// ---------------------------------------------------------------------------
// Phase C: the buffer geometry of `char in[100]` + fgets, and stray bytes.
// ---------------------------------------------------------------------------

/// fgets stops at the first newline and keeps it; it never reads a second
/// line (unlike scanf, which would read across newlines).
#[test]
fn fgets_reads_only_the_first_line() {
    check_all(&[
        ("two lines, second ignored", b"5\n10\n"),
        ("blank first line", b"\n10\n"),
        ("junk first line, number second", b"abc\n5\n"),
        ("number then many newlines", b"5\n\n\n\n\n"),
        ("many newlines only", b"\n\n\n\n\n"),
        ("three lines", b"7\n8\n9\n"),
    ]);
}

/// fgets reads at most sizeof(in)-1 == 99 bytes, so long input is truncated
/// and the truncation can change the parsed value.
#[test]
fn buffer_truncation_at_99_bytes() {
    let n98 = vec![b'9'; 98];
    let mut c98 = n98.clone();
    c98.push(b'\n');

    let n99 = vec![b'9'; 99];
    let mut c99 = n99.clone();
    c99.push(b'\n');

    let mut c200 = vec![b'9'; 200];
    c200.push(b'\n');

    // 95 spaces + "12345": fgets keeps 95 spaces + "1234", dropping the 5.
    let mut split = vec![b' '; 95];
    split.extend_from_slice(b"12345\n");

    // 99 spaces fill the buffer; the digit never arrives -> error path.
    let mut ws99 = vec![b' '; 99];
    ws99.extend_from_slice(b"7\n");

    // 99 tabs, same idea with a different whitespace byte.
    let mut tab99 = vec![b'\t'; 99];
    tab99.extend_from_slice(b"5\n");

    // 98 spaces then a digit: the digit is the 99th byte, so it IS read.
    let mut ws98 = vec![b' '; 98];
    ws98.extend_from_slice(b"5");

    // "1" followed by 98 zeros = 99 bytes, no newline -> ERANGE.
    let mut e98 = vec![b'1'];
    e98.extend(std::iter::repeat(b'0').take(98));

    let mut a150 = vec![b'a'; 150];
    a150.push(b'\n');

    let mut neg99 = vec![b'-'];
    neg99.extend(std::iter::repeat(b'9').take(99));

    // A number whose 99-byte prefix is a *different* valid number.
    let mut prefix_num = vec![b'1'];
    prefix_num.extend(std::iter::repeat(b'0').take(8)); // 1000000000, 10 bytes
    prefix_num.extend(std::iter::repeat(b'x').take(120));
    prefix_num.push(b'\n');

    check_all(&[
        ("98 nines", &n98),
        ("98 nines + newline", &c98),
        ("99 nines exactly fills buffer", &n99),
        ("99 nines + newline", &c99),
        ("200 nines truncated to 99", &c200),
        ("truncation splits the number", &split),
        ("99 spaces then digit is unreachable", &ws99),
        ("99 tabs then digit is unreachable", &tab99),
        ("98 spaces then digit is reachable", &ws98),
        ("1 followed by 98 zeros", &e98),
        ("150 letters", &a150),
        ("minus then 99 nines", &neg99),
        ("number then 120 letters", &prefix_num),
    ]);
}

/// Embedded NUL bytes: fgets copies them into the buffer, but strtol treats
/// the buffer as a C string and stops at the first NUL.
#[test]
fn embedded_nul_bytes() {
    check_all(&[
        ("leading NUL hides the digit", b"\x005\n"),
        ("NUL after the digit", b"5\x00abc\n"),
        ("NUL only", b"\x00\n"),
        ("many NULs", &[0u8; 150]),
        ("NUL between digits", b"1\x002\n"),
        ("whitespace then NUL", b"  \x005\n"),
    ]);
}

/// High / non-ASCII bytes are not digits and are not whitespace.
#[test]
fn high_and_non_ascii_bytes() {
    let all_bytes: Vec<u8> = (1u8..=255).collect();
    check_all(&[
        ("0xFF then digit", b"\xff5\n"),
        ("digit then 0xFF", b"5\xff\n"),
        ("utf8 multibyte", "\u{00e9}5\n".as_bytes()),
        ("every byte 1..=255", &all_bytes),
        ("0x80", b"\x805\n"),
    ]);
}

/// Input larger than any pipe-friendly size: still only the first 99 bytes of
/// the first line matter.
#[test]
fn very_large_input() {
    let mut big = Vec::new();
    big.extend_from_slice(b"5\n");
    big.extend(std::iter::repeat(b'z').take(200_000));
    big.push(b'\n');
    assert_same("number then 200k bytes", &big);

    let mut big_junk = vec![b'q'; 200_000];
    big_junk.push(b'\n');
    assert_same("200k junk bytes", &big_junk);
}

/// Every value that reaches the accept path must produce eight lines: `run`
/// prints four times and is called twice. This pins the C's output shape
/// independently of the diff, so a translation that matched by printing
/// nothing at all would still be caught.
#[test]
fn accept_path_shape_is_eight_lines() {
    let c = run(&c_bin(), b"5\n");
    let r = run(&rust_bin(), b"5\n");
    let expected = b"The house has 2 floors, 5 bedrooms, and 2.5 bathrooms\n\
                     The house has 3 floors, 5 bedrooms, and 2.5 bathrooms\n\
                     The house has 3 floors, 5 bedrooms, and 3.5 bathrooms\n\
                     The house has 3 floors, 10 bedrooms, and 3.5 bathrooms\n\
                     The house has 3 floors, 10 bedrooms, and 3.5 bathrooms\n\
                     The house has 4 floors, 10 bedrooms, and 3.5 bathrooms\n\
                     The house has 4 floors, 10 bedrooms, and 4.5 bathrooms\n\
                     The house has 4 floors, 15 bedrooms, and 4.5 bathrooms\n";
    assert_eq!(c.stdout, expected.to_vec(), "C reference output changed");
    assert_eq!(r.stdout, expected.to_vec(), "Rust output does not match C");
    assert!(c.stderr.is_empty() && r.stderr.is_empty());
    assert_eq!(c.status, Some(0));
    assert_eq!(r.status, Some(0));
}

/// The reject path prints exactly one line and still exits 0.
#[test]
fn reject_path_shape() {
    let c = run(&c_bin(), b"abc\n");
    let r = run(&rust_bin(), b"abc\n");
    assert_eq!(c.stdout, b"An error occurred\n".to_vec());
    assert_eq!(r.stdout, b"An error occurred\n".to_vec());
    assert_eq!(c.status, Some(0));
    assert_eq!(r.status, Some(0));
    assert!(c.stderr.is_empty() && r.stderr.is_empty());
}

/// fgets fails (returns NULL) when stdin cannot be read at all. `in` keeps its
/// "" initialiser, so the error path runs and the exit status is still 0.
#[cfg(unix)]
#[test]
fn stdin_is_a_directory_read_error() {
    fn run_with_dir_stdin(bin: &Path) -> (Vec<u8>, Vec<u8>, Option<i32>) {
        let dir = std::fs::File::open("/").expect("open /");
        let out = Command::new(bin)
            .stdin(Stdio::from(dir))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("spawn");
        (out.stdout, out.stderr, out.status.code())
    }
    let c = run_with_dir_stdin(&c_bin());
    let r = run_with_dir_stdin(&rust_bin());
    assert_eq!(c.0, r.0, "stdout differs on unreadable stdin");
    assert_eq!(c.1, r.1, "stderr differs on unreadable stdin");
    assert_eq!(c.2, r.2, "exit status differs on unreadable stdin");
}

/// stdin closed outright (fd 0 not open): another fgets failure path.
#[cfg(unix)]
#[test]
fn stdin_closed() {
    fn run_no_stdin(bin: &Path) -> (Vec<u8>, Vec<u8>, Option<i32>) {
        let out = Command::new(bin)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("spawn");
        (out.stdout, out.stderr, out.status.code())
    }
    let c = run_no_stdin(&c_bin());
    let r = run_no_stdin(&rust_bin());
    assert_eq!(c.0, r.0);
    assert_eq!(c.1, r.1);
    assert_eq!(c.2, r.2);
}

/// stdout is a pipe whose read end is closed before the program flushes. The C
/// program inherits SIG_DFL for SIGPIPE and is killed by it; the Rust runtime
/// sets SIG_IGN, so `main` must restore SIG_DFL to match. Both programs buffer
/// their whole 437-byte output and emit it in one write at exit, so the write
/// always lands after the reader is gone.
#[cfg(unix)]
#[test]
fn sigpipe_on_closed_stdout_matches() {
    fn run_with_closed_stdout(bin: &Path, stdin_bytes: &[u8]) -> (Option<i32>, Option<i32>) {
        use std::os::unix::process::ExitStatusExt;
        let mut child = Command::new(bin)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn");
        {
            let mut sin = child.stdin.take().expect("stdin");
            let _ = sin.write_all(stdin_bytes);
        }
        // Close the read end of the stdout pipe immediately.
        drop(child.stdout.take());
        let st = child.wait().expect("wait");
        (st.code(), st.signal())
    }

    for input in [&b"5\n"[..], &b"abc\n"[..], &b""[..], &b"2147483647\n"[..]] {
        let c = run_with_closed_stdout(&c_bin(), input);
        let r = run_with_closed_stdout(&rust_bin(), input);
        assert_eq!(
            c,
            r,
            "closed-stdout (code, signal) differs for input {}: C={:?} Rust={:?}",
            show(input),
            c,
            r
        );
    }
}

/// stdout cannot absorb the output (ENOSPC). Both programs ignore the write
/// failure and still return 0 from `main`.
#[cfg(unix)]
#[test]
fn stdout_write_error_still_exits_zero() {
    fn run_to_dev_full(bin: &Path, stdin_bytes: &[u8]) -> Option<i32> {
        let full = match std::fs::OpenOptions::new().write(true).open("/dev/full") {
            Ok(f) => f,
            Err(_) => return None, // /dev/full unavailable: nothing to compare
        };
        let mut child = Command::new(bin)
            .stdin(Stdio::piped())
            .stdout(Stdio::from(full))
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn");
        {
            let mut sin = child.stdin.take().expect("stdin");
            let _ = sin.write_all(stdin_bytes);
        }
        child.wait().expect("wait").code()
    }
    for input in [&b"5\n"[..], &b"abc\n"[..]] {
        assert_eq!(
            run_to_dev_full(&c_bin(), input),
            run_to_dev_full(&rust_bin(), input),
            "exit status on unwritable stdout differs for input {}",
            show(input)
        );
    }
}

/// A dense sweep over every value near the int boundaries and a spread of
/// magnitudes, to catch any arithmetic or formatting divergence.
#[test]
fn numeric_sweep() {
    let mut cases: Vec<Vec<u8>> = Vec::new();
    for v in [
        i32::MIN as i64,
        i32::MIN as i64 + 1,
        i32::MIN as i64 + 2,
        -1_000_000_000,
        -100_000,
        -7,
        -2,
        -1,
        0,
        1,
        2,
        7,
        100_000,
        1_000_000_000,
        2_147_483_640,
        2_147_483_641,
        2_147_483_642,
        2_147_483_643,
        2_147_483_644,
        2_147_483_645,
        i32::MAX as i64 - 1,
        i32::MAX as i64,
        // just outside the accepted range
        i32::MAX as i64 + 1,
        i32::MIN as i64 - 1,
        i64::MAX,
        i64::MIN,
    ] {
        cases.push(format!("{v}\n").into_bytes());
    }
    for (i, input) in cases.iter().enumerate() {
        assert_same(&format!("sweep[{i}]"), input);
    }
}
