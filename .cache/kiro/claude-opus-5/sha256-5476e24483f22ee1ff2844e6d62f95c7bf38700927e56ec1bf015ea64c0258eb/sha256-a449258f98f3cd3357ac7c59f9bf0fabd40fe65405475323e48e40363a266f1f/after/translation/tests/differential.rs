//! Differential tests: run the C `driver` and the Rust `driver` as
//! subprocesses over the same stdin bytes and require byte-identical stdout,
//! byte-identical stderr, and an identical exit status.
//!
//! The Rust code is NEVER called as a library here; both programs are driven
//! exactly the way a shell would drive them, because that is how the
//! translation is graded.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// Path to the Rust binary under test. Cargo sets `CARGO_BIN_EXE_<name>` for
/// integration tests, so this is always the freshly built executable.
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Path to the C reference binary produced by
/// `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`
fn c_bin() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER_BIN") {
        return PathBuf::from(p);
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.parent().expect("translation/ must have a parent");
    // Single-config generators put it in build/, multi-config in build/<cfg>/.
    for cand in [
        root.join("c_src/build/driver"),
        root.join("c_src/build/Release/driver"),
        root.join("c_src/build/Debug/driver"),
        root.join("c_src/build/driver.exe"),
    ] {
        if cand.is_file() {
            return cand;
        }
    }
    panic!(
        "C reference binary not found. Build it first:\n  \
         cd {} && mkdir -p build && cd build && cmake .. && cmake --build .\n\
         (or set C_DRIVER_BIN to its path)",
        root.join("c_src").display()
    );
}

struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    status: Option<i32>,
    signal: Option<i32>,
}

fn run(bin: &PathBuf, input: &[u8]) -> Run {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    {
        let mut stdin = child.stdin.take().expect("stdin piped");
        let input = input.to_vec();
        // Write on a helper thread so a program that never drains stdin cannot
        // deadlock us on a full pipe buffer.
        std::thread::spawn(move || {
            let _ = stdin.write_all(&input);
            let _ = stdin.flush();
        });
    }

    let out = child.wait_with_output().expect("wait_with_output");

    #[cfg(unix)]
    let signal = {
        use std::os::unix::process::ExitStatusExt;
        out.status.signal()
    };
    #[cfg(not(unix))]
    let signal: Option<i32> = None;

    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        status: out.status.code(),
        signal,
    }
}

/// Variant of [`run`] that hands the child an arbitrary stdin (e.g. /dev/null
/// or a directory fd) instead of a pipe we write to.
fn run_with_stdin(bin: &PathBuf, stdin: Stdio) -> Run {
    let out = Command::new(bin)
        .stdin(stdin)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap_or_else(|e| panic!("failed to run {}: {e}", bin.display()));

    #[cfg(unix)]
    let signal = {
        use std::os::unix::process::ExitStatusExt;
        out.status.signal()
    };
    #[cfg(not(unix))]
    let signal: Option<i32> = None;

    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        status: out.status.code(),
        signal,
    }
}

/// Render bytes for assertion messages without drowning the log in output.
fn show(bytes: &[u8]) -> String {
    const LIMIT: usize = 400;
    let head = &bytes[..bytes.len().min(LIMIT)];
    let mut s = String::new();
    for &b in head {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\t' => s.push_str("\\t"),
            b'\r' => s.push_str("\\r"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    if bytes.len() > LIMIT {
        s.push_str(&format!("...<+{} bytes>", bytes.len() - LIMIT));
    }
    s
}

/// The single assertion used by every case: stdout, stderr and exit status
/// must all agree.
fn assert_same(case: &str, input: &[u8]) {
    let c = run(&c_bin(), input);
    let r = run(&rust_bin(), input);

    assert_eq!(
        c.stdout,
        r.stdout,
        "[{case}] stdout mismatch\n  input : {}\n  C     : {}\n  Rust  : {}\n  \
         C len={} Rust len={}",
        show(input),
        show(&c.stdout),
        show(&r.stdout),
        c.stdout.len(),
        r.stdout.len()
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "[{case}] stderr mismatch\n  input : {}\n  C     : {}\n  Rust  : {}",
        show(input),
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        (c.status, c.signal),
        (r.status, r.signal),
        "[{case}] exit status mismatch\n  input : {}\n  C     : code={:?} signal={:?}\n  \
         Rust  : code={:?} signal={:?}",
        show(input),
        c.status,
        c.signal,
        r.status,
        r.signal
    );
}

/// Spawn a program, read exactly the first `n` bytes of its stdout, then kill
/// it. Used for inputs whose output is far too large to buffer (x near
/// INT_MAX emits tens of gigabytes), where the whole-output comparison in
/// `assert_same` would exhaust memory.
fn stdout_prefix(bin: &PathBuf, input: &[u8], n: usize) -> Vec<u8> {
    use std::io::Read;

    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    {
        let mut stdin = child.stdin.take().expect("stdin piped");
        let input = input.to_vec();
        std::thread::spawn(move || {
            let _ = stdin.write_all(&input);
            let _ = stdin.flush();
        });
    }

    let mut out = child.stdout.take().expect("stdout piped");
    let mut buf = vec![0u8; n];
    let mut filled = 0;
    while filled < n {
        match out.read(&mut buf[filled..]) {
            Ok(0) => break, // program finished before producing n bytes
            Ok(k) => filled += k,
            Err(e) => panic!("read from {} failed: {e}", bin.display()),
        }
    }
    buf.truncate(filled);

    // The program may still be running; we have what we need.
    let _ = child.kill();
    let _ = child.wait();
    buf
}

/// Compare a bounded prefix of stdout for an input whose full output is not
/// buffer-able. Also asserts both sides actually produced the full prefix, so
/// the check cannot silently degenerate into comparing two empty vectors.
fn assert_same_stdout_prefix(case: &str, input: &[u8], n: usize) {
    let c = stdout_prefix(&c_bin(), input, n);
    let r = stdout_prefix(&rust_bin(), input, n);
    assert_eq!(
        c.len(),
        n,
        "[{case}] C produced only {} of the {n} expected prefix bytes",
        c.len()
    );
    assert_eq!(
        r.len(),
        n,
        "[{case}] Rust produced only {} of the {n} expected prefix bytes",
        r.len()
    );
    if c != r {
        let at = c.iter().zip(r.iter()).position(|(a, b)| a != b).unwrap_or(0);
        let lo = at.saturating_sub(80);
        let hi = (at + 80).min(c.len());
        panic!(
            "[{case}] stdout prefix mismatch at byte {at}\n  input : {}\n  C     : {}\n  Rust  : {}",
            show(input),
            show(&c[lo..hi]),
            show(&r[lo..hi])
        );
    }
}

// ---------------------------------------------------------------------------
// Phase A sanity: both binaries exist and are runnable.
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_are_runnable() {
    let c = c_bin();
    let r = rust_bin();
    assert!(c.is_file(), "C binary missing at {}", c.display());
    assert!(r.is_file(), "Rust binary missing at {}", r.display());
    // A trivial run must succeed for both, otherwise every comparison below
    // would be measuring nothing.
    assert_eq!(run(&c, b"1\n").status, Some(0), "C binary did not exit 0");
    assert_eq!(run(&r, b"1\n").status, Some(0), "Rust binary did not exit 0");
}

// ---------------------------------------------------------------------------
// Phase B: the input classes the C program branches on.
//
// The C is:
//     int x = 0; scanf("%d", &x); driver(x); return 0;
//     driver: for (i=0, j=0; i < x; i++, j+=2) printf("%d %d\n", i, j);
//
// So the branch space is (1) whether scanf assigns at all, (2) the value it
// assigns, and (3) the `i < x` loop guard: zero iterations vs one vs many.
// ---------------------------------------------------------------------------

/// scanf hits an input failure immediately: nothing is stored, x stays 0,
/// the loop body never runs. Zero-iteration path.
#[test]
fn empty_input() {
    assert_same("empty input", b"");
}

/// Whitespace-only input: `%d` skips all of it, then hits EOF -> input
/// failure, x stays 0.
#[test]
fn whitespace_only_input() {
    assert_same("spaces only", b"   ");
    assert_same("newlines only", b"\n\n\n");
    assert_same("mixed whitespace only", b" \t\n\r\x0b\x0c ");
}

/// x == 0 explicitly: loop guard false on the first check.
#[test]
fn zero() {
    assert_same("0", b"0");
    assert_same("0 with newline", b"0\n");
    assert_same("-0", b"-0\n");
    assert_same("+0", b"+0\n");
}

/// The single-iteration boundary of `i < x`.
#[test]
fn single_item() {
    assert_same("1", b"1\n");
}

/// A handful of iterations: exercises the `j += 2` stride and the two-column
/// printf format.
#[test]
fn small_counts() {
    for n in 2..=12u32 {
        let input = format!("{n}\n");
        assert_same(&format!("n={n}"), input.as_bytes());
    }
}

/// Negative x: the loop guard is false immediately, no output.
#[test]
fn negative_values() {
    assert_same("-1", b"-1\n");
    assert_same("-3", b"-3\n");
    assert_same("-2147483648 (INT_MIN)", b"-2147483648\n");
    assert_same("-2147483647", b"-2147483647\n");
}

/// Column widths change as i and j cross decimal digit boundaries (9->10,
/// 99->100, and j crossing them one step earlier than i).
#[test]
fn digit_width_transitions() {
    for n in [9u32, 10, 11, 50, 51, 99, 100, 101, 500, 501, 999, 1000, 1001] {
        let input = format!("{n}\n");
        assert_same(&format!("width n={n}"), input.as_bytes());
    }
}

/// A large count, to shake out any stdout buffering or flushing difference
/// between C's block-buffered stdout and Rust's BufWriter.
#[test]
fn large_count() {
    assert_same("n=100000", b"100000\n");
}

// ---------------------------------------------------------------------------
// Phase B/C: scanf("%d") parsing behavior. `%d` skips leading whitespace
// INCLUDING newlines, accepts an optional sign, consumes digits, and pushes
// back the first non-digit. On a matching failure nothing is stored, so `x`
// keeps its initializer 0.
// ---------------------------------------------------------------------------

#[test]
fn leading_whitespace_is_skipped_across_newlines() {
    assert_same("leading spaces", b"    4\n");
    assert_same("leading newlines", b"\n\n\n4\n");
    assert_same("leading tabs", b"\t\t4\n");
    assert_same("all whitespace kinds", b" \t\n\r\x0b\x0c4\n");
    assert_same("no trailing newline", b"4");
}

#[test]
fn explicit_plus_sign() {
    assert_same("+7", b"+7\n");
    assert_same("+0007", b"+0007\n");
}

#[test]
fn leading_zeros_are_decimal_not_octal() {
    assert_same("007", b"007\n");
    assert_same("0000000000000000000000000005", b"0000000000000000000000000005\n");
}

#[test]
fn trailing_junk_after_the_number_is_ignored() {
    // Only the first conversion runs; the rest of stdin is never read.
    assert_same("3 then another number", b"3 99\n");
    assert_same("3 then letters", b"3abc\n");
    assert_same("3 then newline then more", b"3\n7\n");
    assert_same("3 then punctuation", b"3,4\n");
    assert_same("3 then dot", b"3.9\n");
}

/// Matching failure paths: scanf returns 0 without assigning, so x stays 0.
#[test]
fn matching_failure_leaves_x_at_zero() {
    assert_same("letters", b"abc\n");
    assert_same("lone minus", b"-");
    assert_same("lone plus", b"+");
    assert_same("minus then letter", b"-a\n");
    assert_same("plus then letter", b"+a\n");
    assert_same("dot", b".\n");
    assert_same("dot five", b".5\n");
    assert_same("double minus", b"--5\n");
    assert_same("double plus", b"++5\n");
    assert_same("minus plus", b"-+5\n");
    assert_same("space between sign and digits", b"- 5\n");
    assert_same("newline between sign and digits", b"-\n5\n");
    assert_same("hex prefix is parsed as 0", b"0x10\n");
    assert_same("comma", b",5\n");
    assert_same("underscore", b"_5\n");
    assert_same("quote", b"\"5\"\n");
}

/// Non-UTF-8 and NUL bytes must not change the outcome or crash either side.
#[test]
fn non_utf8_and_nul_bytes() {
    assert_same("0xff then digit", b"\xff5\n");
    assert_same("NUL then digit", b"\x005\n");
    assert_same("digit then NUL", b"5\x00\n");
    assert_same("invalid utf8 sequence", b"\xc3\x28 5\n");
    assert_same("high bytes only", b"\x80\x81\x82");
}

// ---------------------------------------------------------------------------
// Phase C: integer overflow / truncation / signedness exactly as the C
// performs it. glibc's `%d` converts with a wide (long) accumulator that
// saturates at LONG_MAX / LONG_MIN, then stores the value truncated to `int`.
// That makes several out-of-range inputs land on small or negative x values.
// ---------------------------------------------------------------------------

#[test]
fn int_max_and_just_past_it() {
    // 2147483648 truncates to INT_MIN -> negative -> no output.
    assert_same("2147483648 (INT_MAX+1)", b"2147483648\n");
    assert_same("2147483649", b"2147483649\n");
}

/// -2147483649 truncates to +2147483647 == INT_MAX, i.e. the largest count the
/// loop can take. That emits ~30 GB, so this is the one case compared by a
/// bounded stdout prefix instead of the whole stream; the exit status is not
/// observable without letting it run to completion.
#[test]
fn int_min_minus_one_wraps_to_int_max_iterations() {
    assert_same_stdout_prefix("-2147483649 -> x=INT_MAX", b"-2147483649\n", 1 << 20);
    // Reached the same way, via a value that saturates nowhere near INT_MAX.
    assert_same_stdout_prefix("18446744071562067967 -> x=INT_MAX", b"-9223372034707292161\n", 1 << 20);
}

#[test]
fn truncation_to_int_wraps_into_small_positive_counts() {
    // 2^32 + k truncates to k, so these DO produce output.
    assert_same("4294967296 (2^32) -> 0", b"4294967296\n");
    assert_same("4294967297 (2^32+1) -> 1", b"4294967297\n");
    assert_same("4294967300 (2^32+4) -> 4", b"4294967300\n");
    assert_same("8589934597 (2^33+5) -> 5", b"8589934597\n");
    // -(2^32 - k) truncates to k.
    assert_same("-4294967293 -> 3", b"-4294967293\n");
    assert_same("-8589934586 -> 6", b"-8589934586\n");
}

#[test]
fn long_range_saturation() {
    assert_same("LONG_MAX", b"9223372036854775807\n");
    assert_same("LONG_MAX+1 saturates", b"9223372036854775808\n");
    assert_same("LONG_MIN", b"-9223372036854775808\n");
    assert_same("LONG_MIN-1 saturates", b"-9223372036854775809\n");
    assert_same("20 nines", b"99999999999999999999\n");
    assert_same("20 nines negative", b"-99999999999999999999\n");
    assert_same("UINT64_MAX", b"18446744073709551615\n");
    assert_same("UINT64_MAX+2", b"18446744073709551617\n");
}

#[test]
fn absurdly_long_digit_runs() {
    // Overflow detected mid-accumulation, then truncated.
    let mut input = vec![b'9'; 400];
    input.push(b'\n');
    assert_same("400 nines", &input);

    let mut input = Vec::new();
    input.push(b'-');
    input.extend(std::iter::repeat(b'1').take(400));
    input.push(b'\n');
    assert_same("400 ones negative", &input);

    // Enormous run of leading zeros followed by a real value.
    let mut input = vec![b'0'; 5000];
    input.extend_from_slice(b"7\n");
    assert_same("5000 zeros then 7", &input);
}

// ---------------------------------------------------------------------------
// Phase C: stdin shapes rather than stdin contents.
// ---------------------------------------------------------------------------

#[test]
fn value_split_by_the_pipe_is_still_one_number() {
    // scanf keeps consuming digits across read boundaries; the writer thread
    // hands over the bytes in one go but the parser must not stop early on a
    // short read.
    assert_same("digits then eof, no delimiter", b"12");
    assert_same("many bytes before the number", &{
        let mut v = vec![b' '; 8192];
        v.extend_from_slice(b"6\n");
        v
    });
}

#[test]
fn huge_trailing_payload_is_never_read() {
    // The program reads one number and exits; the unread remainder of stdin
    // must not change stdout, stderr or the exit status on either side.
    let mut input = Vec::from(&b"5\n"[..]);
    input.extend(std::iter::repeat(b'x').take(200_000));
    assert_same("5 then 200k junk bytes", &input);
}

/// stdin as /dev/null: the first read returns EOF, so `%d` sees an input
/// failure and x keeps its initializer.
#[test]
fn stdin_is_dev_null() {
    let c = run_with_stdin(&c_bin(), Stdio::null());
    let r = run_with_stdin(&rust_bin(), Stdio::null());
    assert_eq!(c.stdout, r.stdout, "/dev/null stdin: stdout mismatch");
    assert_eq!(c.stderr, r.stderr, "/dev/null stdin: stderr mismatch");
    assert_eq!(
        (c.status, c.signal),
        (r.status, r.signal),
        "/dev/null stdin: exit status mismatch"
    );
}

/// stdin as a directory fd: `read` fails with EISDIR rather than returning 0.
/// That is a read-error path distinct from EOF, and the C treats it the same
/// way (input failure, x stays 0, exit 0).
#[test]
#[cfg(unix)]
fn stdin_is_a_directory() {
    let dir = std::fs::File::open(env!("CARGO_MANIFEST_DIR")).expect("open manifest dir");
    let dir2 = dir.try_clone().expect("clone dir handle");
    let c = run_with_stdin(&c_bin(), Stdio::from(dir));
    let r = run_with_stdin(&rust_bin(), Stdio::from(dir2));
    assert_eq!(c.stdout, r.stdout, "directory stdin: stdout mismatch");
    assert_eq!(c.stderr, r.stderr, "directory stdin: stderr mismatch");
    assert_eq!(
        (c.status, c.signal),
        (r.status, r.signal),
        "directory stdin: exit status mismatch"
    );
}

/// Closed stdout: the C program has the default `SIGPIPE` disposition, so
/// `printf` to a pipe with no reader kills it with signal 13. The Rust runtime
/// ignores `SIGPIPE` by default, which would have made it exit 0 instead; the
/// translation resets the disposition so the two agree.
#[test]
#[cfg(unix)]
fn closed_stdout_kills_both_with_sigpipe() {
    fn run_with_closed_stdout(bin: &PathBuf) -> Run {
        let mut child = Command::new(bin)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

        // Close the read end immediately: every write to stdout now fails.
        drop(child.stdout.take());

        {
            let mut stdin = child.stdin.take().expect("stdin piped");
            std::thread::spawn(move || {
                let _ = stdin.write_all(b"100000000\n");
                let _ = stdin.flush();
            });
        }

        let out = child.wait_with_output().expect("wait_with_output");
        use std::os::unix::process::ExitStatusExt;
        Run {
            stdout: out.stdout,
            stderr: out.stderr,
            status: out.status.code(),
            signal: out.status.signal(),
        }
    }

    let c = run_with_closed_stdout(&c_bin());
    let r = run_with_closed_stdout(&rust_bin());
    assert_eq!(
        c.signal,
        Some(13),
        "expected the C program to die from SIGPIPE, got code={:?} signal={:?}",
        c.status,
        c.signal
    );
    assert_eq!(c.stderr, r.stderr, "closed stdout: stderr mismatch");
    assert_eq!(
        (c.status, c.signal),
        (r.status, r.signal),
        "closed stdout: exit status mismatch (C code={:?} signal={:?}, \
         Rust code={:?} signal={:?})",
        c.status,
        c.signal,
        r.status,
        r.signal
    );
}

/// Number of decimal digits `printf("%d")` emits for a non-negative value.
fn digits(v: u64) -> u64 {
    let mut n = 1;
    let mut t = v;
    while t >= 10 {
        t /= 10;
        n += 1;
    }
    n
}

/// Byte length of the output the C program emits for iterations `0..n`, i.e.
/// the byte offset at which the line for `i == n` begins.
///
/// Each line is `"%d %d\n"` over (i, 2*i). This is computed in closed form per
/// decimal bucket so the caller does not have to iterate a billion times.
/// Valid only while `2*i` has not yet overflowed, which is exactly the region
/// before the wrap we are trying to locate.
fn output_bytes_before(n: u64) -> u64 {
    let mut total = 0u64;
    let mut lo = 0u64;
    let mut pow = 1u64; // 10^k, the start of the current bucket
    while lo < n {
        // Bucket of i values sharing the same digit count.
        let (bucket_lo, bucket_hi) = if lo == 0 { (0, 1) } else { (pow, pow * 10) };
        let d_i = digits(bucket_lo);
        // Within the bucket, 2*i crosses a power of ten at 5 * 10^k, so d(2i)
        // is d_i below that split and d_i + 1 at or above it.
        let split = if lo == 0 { 1 } else { 5 * pow };
        for (seg_lo, seg_hi, d_j) in [
            (bucket_lo, split.min(bucket_hi), d_i),
            (split.min(bucket_hi), bucket_hi, d_i + 1),
        ] {
            let a = seg_lo.max(lo);
            let b = seg_hi.min(n);
            if b > a {
                // d_i digits + space + d_j digits + newline
                total += (b - a) * (d_i + 1 + d_j + 1);
            }
        }
        lo = bucket_hi;
        if bucket_lo != 0 {
            pow *= 10;
        }
    }
    total
}

/// The one branch region that only a billion iterations can reach: `j += 2`
/// overflows `int` at i == 1073741824, where j goes from 2147483646 to
/// -2147483648. The C is compiled without optimization (CMakeLists sets no
/// build type), so the increment is a plain 32-bit `addl` on a stack slot that
/// simply wraps; the Rust uses `wrapping_add` to match.
///
/// Both programs are streamed concurrently and compared byte for byte all the
/// way through the wrap. This takes a couple of minutes because there is no
/// shorter route to the overflow.
#[test]
fn j_overflows_int_at_one_billion_iterations() {
    use std::io::Read;

    // i at which j = 2*i would become 2^31.
    const WRAP_I: u64 = 1 << 30;
    const EXPECTED_LINE: &[u8] = b"1073741824 -2147483648\n";
    const BLOCK: usize = 1 << 20;

    // Guard the closed form against a brute-force count over a small prefix.
    {
        let mut brute = 0u64;
        for i in 0..20_000u64 {
            brute += digits(i) + 1 + digits(2 * i) + 1;
        }
        assert_eq!(
            output_bytes_before(20_000),
            brute,
            "output_bytes_before() disagrees with a brute-force count"
        );
    }

    let wrap_offset = output_bytes_before(WRAP_I);

    fn spawn(bin: &PathBuf) -> std::process::Child {
        let mut child = Command::new(bin)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));
        let mut stdin = child.stdin.take().expect("stdin piped");
        std::thread::spawn(move || {
            // -2147483649 truncates to INT_MAX, the largest loop count.
            let _ = stdin.write_all(b"-2147483649\n");
            let _ = stdin.flush();
        });
        child
    }

    /// Fill `buf` completely, or return the short count at EOF.
    fn fill(r: &mut impl Read, buf: &mut [u8]) -> usize {
        let mut n = 0;
        while n < buf.len() {
            match r.read(&mut buf[n..]) {
                Ok(0) => break,
                Ok(k) => n += k,
                Err(e) => panic!("read failed: {e}"),
            }
        }
        n
    }

    let mut cc = spawn(&c_bin());
    let mut rc = spawn(&rust_bin());
    let mut cout = cc.stdout.take().expect("C stdout");
    let mut rout = rc.stdout.take().expect("Rust stdout");

    let mut cbuf = vec![0u8; BLOCK];
    let mut rbuf = vec![0u8; BLOCK];
    let mut offset: u64 = 0;
    // Bytes captured from the C stream starting exactly at `wrap_offset`.
    let mut wrap_text: Vec<u8> = Vec::new();
    let want = EXPECTED_LINE.len() + 32;

    while wrap_text.len() < want {
        let cn = fill(&mut cout, &mut cbuf);
        let rn = fill(&mut rout, &mut rbuf);
        assert_eq!(
            cn, rn,
            "stream length diverged at byte offset {offset}: C gave {cn}, Rust gave {rn}"
        );
        if cn == 0 {
            break;
        }
        // Slice comparison bottoms out in memcmp, so this stays cheap even in
        // the unoptimized test profile.
        if cbuf[..cn] != rbuf[..rn] {
            let at = cbuf[..cn]
                .iter()
                .zip(rbuf[..rn].iter())
                .position(|(a, b)| a != b)
                .unwrap_or(0);
            let lo = at.saturating_sub(80);
            let hi = (at + 80).min(cn);
            panic!(
                "stdout diverged at byte offset {}\n  C    : {}\n  Rust : {}",
                offset + at as u64,
                show(&cbuf[lo..hi]),
                show(&rbuf[lo..hi])
            );
        }

        // Capture the bytes at and after the predicted wrap offset.
        let block_end = offset + cn as u64;
        if block_end > wrap_offset && wrap_text.len() < want {
            let from = wrap_offset.saturating_sub(offset) as usize;
            if from < cn {
                let take = (want - wrap_text.len()).min(cn - from);
                wrap_text.extend_from_slice(&cbuf[from..from + take]);
            }
        }
        offset += cn as u64;
    }

    let _ = cc.kill();
    let _ = cc.wait();
    let _ = rc.kill();
    let _ = rc.wait();

    assert!(
        wrap_text.starts_with(EXPECTED_LINE),
        "expected the line at byte offset {wrap_offset} to be {}, got {}\n\
         (streamed and byte-compared {offset} bytes)",
        show(EXPECTED_LINE),
        show(&wrap_text)
    );
}
