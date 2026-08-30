//! Differential tests: run the C reference binary and the Rust binary as
//! subprocesses on identical stdin and require byte-identical stdout, stderr
//! and exit status.
//!
//! Nothing here links against the translated code as a library; both programs
//! are driven exactly the way a shell would drive them.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;

/// Path to the Rust binary under test, supplied by Cargo.
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Path to the compiled C reference binary, building it with CMake on demand.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_src = manifest
            .parent()
            .expect("translation/ must have a parent directory")
            .join("c_src");
        let build = c_src.join("build");
        let bin = build.join("driver");
        if !bin.exists() {
            std::fs::create_dir_all(&build).expect("cannot create c_src/build");
            let conf = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("failed to run cmake (is it installed?)");
            assert!(
                conf.status.success(),
                "cmake configure failed:\n{}",
                String::from_utf8_lossy(&conf.stderr)
            );
            let built = Command::new("cmake")
                .args(["--build", "."])
                .current_dir(&build)
                .output()
                .expect("failed to run cmake --build");
            assert!(
                built.status.success(),
                "cmake --build failed:\n{}",
                String::from_utf8_lossy(&built.stderr)
            );
        }
        assert!(
            bin.exists(),
            "C reference binary missing at {}",
            bin.display()
        );
        bin
    })
    .as_path()
}

/// Feed `input` to `bin` on stdin and capture everything it produces.
fn run(bin: &Path, input: &[u8]) -> Output {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));
    {
        let mut stdin = child.stdin.take().expect("stdin was piped");
        // The programs may exit without draining stdin (both stop after 100
        // integers), so a broken pipe here is expected and not a failure.
        let _ = stdin.write_all(input);
        let _ = stdin.flush();
    }
    child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("failed to wait on {}: {e}", bin.display()))
}

/// Assert the two programs agree on stdout, stderr and exit status.
fn assert_same(label: &str, input: &[u8]) {
    let c = run(c_bin(), input);
    let r = run(&rust_bin(), input);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch for {label} (input {:?})\n  C   : {:?}\n  Rust: {:?}",
        Preview(input),
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch for {label} (input {:?})\n  C   : {:?}\n  Rust: {:?}",
        Preview(input),
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "exit status mismatch for {label} (input {:?}): C={:?} Rust={:?}",
        Preview(input),
        c.status,
        r.status
    );
}

/// Keeps assertion messages readable when an input is very large.
struct Preview<'a>(&'a [u8]);

impl std::fmt::Debug for Preview<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.len() <= 120 {
            write!(f, "{}", String::from_utf8_lossy(self.0).escape_debug())
        } else {
            write!(
                f,
                "{}... ({} bytes total)",
                String::from_utf8_lossy(&self.0[..120]).escape_debug(),
                self.0.len()
            )
        }
    }
}

fn joined(values: impl IntoIterator<Item = i64>) -> Vec<u8> {
    let mut s = String::new();
    for v in values {
        if !s.is_empty() {
            s.push(' ');
        }
        s.push_str(&v.to_string());
    }
    s.into_bytes()
}

// ---------------------------------------------------------------------------
// Item-count classes: `main` loops `i` from 0 to 99 and `call_fma` special
// cases `len == 0`, so 0, 1, 99, 100 and >100 items are distinct paths.
// ---------------------------------------------------------------------------

#[test]
fn empty_input() {
    assert_same("empty stdin", b"");
}

#[test]
fn whitespace_only_input() {
    // scanf skips whitespace then hits EOF: zero items read.
    assert_same("spaces only", b"   ");
    assert_same("all whitespace kinds", b" \t\n\r\x0b\x0c ");
    assert_same("newline only", b"\n");
    assert_same("many newlines", b"\n\n\n\n\n");
}

#[test]
fn single_item() {
    assert_same("single value, no newline", b"42");
    assert_same("single value, newline", b"42\n");
    assert_same("single zero", b"0");
    assert_same("single negative", b"-5");
    assert_same("single explicit plus", b"+7");
    assert_same("leading zeros", b"0000000012");
}

#[test]
fn a_few_items() {
    assert_same("three values", b"1 2 3");
    assert_same("three values, trailing newline", b"1 2 3\n");
    assert_same("last value negative", b"1 2 -3");
    assert_same("last value zero", b"7 8 0");
}

#[test]
fn ninety_nine_items() {
    assert_same("99 items", &joined(1..=99));
}

#[test]
fn exactly_one_hundred_items() {
    // The loop fills the array exactly and exits on `i == 100`.
    assert_same("100 items", &joined(1..=100));
    assert_same("100 items, trailing newline", &{
        let mut v = joined(1..=100);
        v.push(b'\n');
        v
    });
}

#[test]
fn more_than_one_hundred_items() {
    // Everything past the 100th integer is never read; the result is the 100th.
    assert_same("101 items", &joined(1..=101));
    assert_same("150 items", &joined(1..=150));
    assert_same("500 items", &joined(1..=500));
    assert_same("100 items then junk", &{
        let mut v = joined(1..=100);
        v.extend_from_slice(b" zzz");
        v
    });
}

// ---------------------------------------------------------------------------
// scanf failure paths: `scanf("%d", ...) != 1` breaks the loop, both for a
// matching failure (returns 0) and for EOF (returns EOF).
// ---------------------------------------------------------------------------

#[test]
fn matching_failure_on_first_item() {
    // Zero items read => call_fma's `len == 0` early return => prints 0.
    assert_same("letters", b"abc");
    assert_same("punctuation", b".");
    assert_same("comma", b",");
    assert_same("hash", b"#");
    assert_same("leading junk then number", b"abc 5");
}

#[test]
fn matching_failure_mid_stream() {
    assert_same("junk after two items", b"1 2 abc");
    assert_same("junk terminates early", b"1 2 x 3 4");
    assert_same("junk on the 100th", &{
        let mut v = joined(std::iter::repeat(7).take(99));
        v.extend_from_slice(b" x 8");
        v
    });
}

#[test]
fn sign_only_and_malformed_signs() {
    assert_same("minus then EOF", b"-");
    assert_same("plus then EOF", b"+");
    assert_same("minus space digit", b"- 5");
    assert_same("plus then minus", b"+-5");
    assert_same("double minus", b"--5");
    assert_same("value then bare minus", b"5 -");
    assert_same("digit immediately followed by minus", b"5-3");
}

#[test]
fn partial_numeric_forms() {
    // scanf("%d") stops at the first non-digit; the rest fails the next call.
    assert_same("hex literal", b"0x10");
    assert_same("binary literal", b"0b101");
    assert_same("decimal point", b"3.7");
    assert_same("exponent form", b"1e5");
    assert_same("comma separated", b"1,2,3");
    assert_same("unicode digit", "٣".as_bytes());
    assert_same("non-breaking space then digit", "\u{a0}5".as_bytes());
}

// ---------------------------------------------------------------------------
// Whitespace handling: `scanf` skips across newlines, so line structure is
// irrelevant to how items are grouped.
// ---------------------------------------------------------------------------

#[test]
fn separators_are_interchangeable() {
    assert_same("tabs", b"1\t2\t\t3");
    assert_same("newlines", b"1\n2\n3");
    assert_same("crlf", b"1\r\n2\r\n3\r\n");
    assert_same("vertical tab", b"1\x0b2");
    assert_same("form feed", b"1\x0c2");
    assert_same("mixed padding", b"   \n   5   \n  ");
    assert_same("leading newlines", b"\n\n\n9");
}

// ---------------------------------------------------------------------------
// Integer range: the value printed is the last item as converted by scanf and
// truncated to `int`, including out-of-range saturation.
// ---------------------------------------------------------------------------

#[test]
fn int_boundaries() {
    for v in [
        "0",
        "1",
        "-1",
        "2147483646",
        "2147483647",
        "-2147483647",
        "-2147483648",
    ] {
        assert_same("int boundary", v.as_bytes());
    }
}

#[test]
fn out_of_range_values() {
    for v in [
        "2147483648",
        "2147483649",
        "-2147483649",
        "4294967295",
        "4294967296",
        "4294967297",
        "9223372036854775806",
        "9223372036854775807",
        "9223372036854775808",
        "-9223372036854775808",
        "-9223372036854775809",
        "18446744073709551615",
        "18446744073709551616",
        "18446744073709551617",
        "99999999999999999999",
        "-99999999999999999999",
    ] {
        assert_same("out of range", v.as_bytes());
    }
}

#[test]
fn very_long_digit_runs() {
    for n in [1usize, 9, 10, 18, 19, 20, 21, 30, 40, 64, 200] {
        assert_same("nines", "9".repeat(n).as_bytes());
        assert_same("negative nines", format!("-{}", "9".repeat(n)).as_bytes());
        assert_same("power of ten", format!("1{}", "0".repeat(n)).as_bytes());
        assert_same(
            "negative power of ten",
            format!("-1{}", "0".repeat(n)).as_bytes(),
        );
    }
    assert_same("many leading zeros", &{
        let mut v = b"0".repeat(9000);
        v.push(b'5');
        v
    });
    assert_same("huge magnitude", "1".repeat(9000).as_bytes());
}

#[test]
fn mixed_overflow_then_normal() {
    assert_same("overflow then in-range", b"2147483648 -2147483649");
    assert_same("in-range then overflow", b"5 99999999999999999999");
}

// ---------------------------------------------------------------------------
// Bytes that are not valid text at all: the C reads bytes, so the Rust must
// not assume UTF-8.
// ---------------------------------------------------------------------------

#[test]
fn non_utf8_and_nul_bytes() {
    assert_same("invalid utf8 mid stream", b"5 \xff\xfe 9");
    assert_same("invalid utf8 first", b"\xc3\x28 5");
    assert_same("nul byte terminates", b"5 \x00 9");
    assert_same("nul byte first", b"\x00 5");
    assert_same("high bytes only", b"\x80\x81\x82");
}

// ---------------------------------------------------------------------------
// Large inputs, including offsets around any internal read-buffer boundary.
// ---------------------------------------------------------------------------

#[test]
fn read_buffer_boundaries() {
    for off in [4095usize, 4096, 8188, 8189, 8190, 8191, 8192, 8193, 8194, 16384] {
        let mut v = b" ".repeat(off);
        v.extend_from_slice(b"123456789 7");
        assert_same("padded number across buffer boundary", &v);

        let mut v = b" ".repeat(off);
        v.extend_from_slice(b"abc");
        assert_same("padded junk across buffer boundary", &v);
    }
    assert_same("whitespace only, 20000 bytes", &b" ".repeat(20000));
    assert_same("digit run past boundary", &b"9".repeat(9000));
    assert_same("digit run then junk", &{
        let mut v = b"9".repeat(8192);
        v.push(b'x');
        v
    });
}

#[test]
fn large_stream() {
    let big: Vec<u8> = joined((0..50_000).map(|i| i % 1000));
    assert_same("50k tokens", &big);
}

#[test]
fn stdin_is_dev_null() {
    fn run_null(bin: &Path) -> Output {
        Command::new(bin)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .unwrap_or_else(|e| panic!("failed to run {}: {e}", bin.display()))
    }
    let c = run_null(c_bin());
    let r = run_null(&rust_bin());
    assert_eq!(c.stdout, r.stdout, "stdout mismatch with /dev/null stdin");
    assert_eq!(c.stderr, r.stderr, "stderr mismatch with /dev/null stdin");
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "exit status mismatch with /dev/null stdin"
    );
}

// ---------------------------------------------------------------------------
// Signal disposition: the C program writes to stdout with the default SIGPIPE
// handling, so a vanished reader kills it. The Rust runtime ignores SIGPIPE by
// default, which would silently turn that into a clean exit.
// ---------------------------------------------------------------------------

#[test]
fn stdout_reader_gone_kills_both_the_same_way() {
    use std::os::unix::process::ExitStatusExt;

    fn run_with_dead_stdout(bin: &Path) -> (Option<i32>, Option<i32>, Vec<u8>) {
        let mut child = Command::new(bin)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

        // Close the read end before the program can print: it only writes
        // after stdin reaches EOF, which we cause afterwards.
        drop(child.stdout.take().expect("stdout was piped"));

        {
            let mut stdin = child.stdin.take().expect("stdin was piped");
            let _ = stdin.write_all(b"5");
            let _ = stdin.flush();
        }

        let mut stderr_buf = Vec::new();
        let mut stderr = child.stderr.take().expect("stderr was piped");
        let _ = std::io::Read::read_to_end(&mut stderr, &mut stderr_buf);

        let status = child.wait().expect("failed to wait for child");
        (status.code(), status.signal(), stderr_buf)
    }

    let c = run_with_dead_stdout(c_bin());
    let r = run_with_dead_stdout(&rust_bin());

    assert_eq!(c.1, r.1, "termination signal mismatch: C={:?} Rust={:?}", c.1, r.1);
    assert_eq!(c.0, r.0, "exit code mismatch: C={:?} Rust={:?}", c.0, r.0);
    assert_eq!(c.2, r.2, "stderr mismatch");
}

// ---------------------------------------------------------------------------
// Deterministic randomized sweep over token fragments, to catch input classes
// the hand-written cases above miss.
// ---------------------------------------------------------------------------

#[test]
fn randomized_token_soup() {
    const TOKENS: &[&[u8]] = &[
        b"0",
        b"1",
        b"-1",
        b"7",
        b" ",
        b"\t",
        b"\n",
        b"\r",
        b"\x0b",
        b"\x0c",
        b"+",
        b"-",
        b"abc",
        b"0x10",
        b"3.5",
        b"99999999999999999999",
        b"2147483648",
        b"-2147483648",
        b"1e5",
        b"",
        b".",
        b",",
        b"#",
        b"9999999999999999999999999",
        b"\xff",
        b"\x00",
    ];

    // xorshift64* so the case set is reproducible without a dependency.
    let mut state: u64 = 0x9E3779B97F4A7C15;
    let mut next = move || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state.wrapping_mul(0x2545_F491_4F6C_DD1D)
    };

    for case in 0..300 {
        let n = (next() % 13) as usize;
        let mut input = Vec::new();
        for _ in 0..n {
            input.extend_from_slice(TOKENS[(next() as usize) % TOKENS.len()]);
        }
        assert_same(&format!("random case {case}"), &input);
    }
}
