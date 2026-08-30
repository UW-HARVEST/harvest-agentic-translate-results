//! Differential tests: run the C binary and the Rust binary as subprocesses,
//! feed both the same bytes on stdin, and require byte-identical stdout,
//! byte-identical stderr and an identical exit status.
//!
//! Nothing here links against the Rust crate as a library; both programs are
//! driven exactly the way a shell would drive them, because that is how the
//! translation is graded.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

fn repo_root() -> PathBuf {
    // translation/ -> repo root
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// Build (once per test binary) and return the path to the C executable.
fn c_binary() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = repo_root().join("c_src");

        // Prefer an already-built binary so we never write into c_src/.
        for candidate in [
            c_src.join("build").join("driver"),
            c_src.join("build").join("Debug").join("driver.exe"),
        ] {
            if candidate.is_file() {
                return candidate;
            }
        }

        // Otherwise configure/build out-of-tree, under our own target dir.
        let build_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("c_build");
        std::fs::create_dir_all(&build_dir).expect("create c_build dir");

        let cfg = Command::new("cmake")
            .arg("-S")
            .arg(&c_src)
            .arg("-B")
            .arg(&build_dir)
            .output()
            .expect("cmake must be installed to run the differential tests");
        assert!(
            cfg.status.success(),
            "cmake configure failed:\n{}\n{}",
            String::from_utf8_lossy(&cfg.stdout),
            String::from_utf8_lossy(&cfg.stderr)
        );

        let build = Command::new("cmake")
            .arg("--build")
            .arg(&build_dir)
            .output()
            .expect("cmake --build must run");
        assert!(
            build.status.success(),
            "cmake --build failed:\n{}\n{}",
            String::from_utf8_lossy(&build.stdout),
            String::from_utf8_lossy(&build.stderr)
        );

        let bin = build_dir.join("driver");
        assert!(bin.is_file(), "C binary not found at {}", bin.display());
        bin
    })
}

/// Path to the Rust executable under test (the binary cargo built for us).
fn rust_binary() -> &'static Path {
    static R_BIN: OnceLock<PathBuf> = OnceLock::new();
    R_BIN.get_or_init(|| {
        // env!("CARGO_BIN_EXE_driver") points at the freshly built binary.
        PathBuf::from(env!("CARGO_BIN_EXE_driver"))
    })
}

struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    status: Option<i32>,
}

fn run(bin: &Path, args: &[&str], stdin_bytes: &[u8]) -> Outcome {
    let mut child = Command::new(bin)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    {
        let mut sin = child.stdin.take().expect("stdin piped");
        // The program may exit without draining stdin (e.g. a huge input);
        // a broken pipe here is expected and must not fail the test.
        let _ = sin.write_all(stdin_bytes);
        let _ = sin.flush();
    }

    let out = child.wait_with_output().expect("wait for child");
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        status: out.status.code(),
    }
}

/// Core assertion: for `stdin_bytes`, C and Rust must agree on all three
/// observable channels.
fn assert_same(label: &str, stdin_bytes: &[u8]) {
    assert_same_with_args(label, &[], stdin_bytes)
}

fn assert_same_with_args(label: &str, args: &[&str], stdin_bytes: &[u8]) {
    let c = run(c_binary(), args, stdin_bytes);
    let r = run(rust_binary(), args, stdin_bytes);

    assert_eq!(
        c.stdout,
        r.stdout,
        "[{label}] stdout mismatch\n  input : {:?}\n  C     : {:?}\n  Rust  : {:?}",
        Shown(stdin_bytes),
        Shown(&c.stdout),
        Shown(&r.stdout),
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "[{label}] stderr mismatch\n  input : {:?}\n  C     : {:?}\n  Rust  : {:?}",
        Shown(stdin_bytes),
        Shown(&c.stderr),
        Shown(&r.stderr),
    );
    assert_eq!(
        c.status, r.status,
        "[{label}] exit status mismatch\n  input : {:?}\n  C     : {:?}\n  Rust  : {:?}",
        Shown(stdin_bytes), c.status, r.status,
    );
}

/// Pretty-print possibly-binary bytes in assertion messages.
struct Shown<'a>(&'a [u8]);
impl std::fmt::Debug for Shown<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", String::from_utf8_lossy(self.0).escape_debug())
    }
}

// ---------------------------------------------------------------------------
// Sanity: both binaries exist and run.
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_are_runnable() {
    assert!(c_binary().is_file(), "C binary missing");
    assert!(rust_binary().is_file(), "Rust binary missing");
    let c = run(c_binary(), &[], b"0");
    let r = run(rust_binary(), &[], b"0");
    assert_eq!(c.status, Some(0));
    assert_eq!(r.status, Some(0));
    assert!(!c.stdout.is_empty());
}

// ---------------------------------------------------------------------------
// The two top-level branches of main(): `if (x) good(); else bad();`
// ---------------------------------------------------------------------------

#[test]
fn zero_takes_the_bad_branch() {
    // bad(): data = CHAR_MAX (127) > 0, result = (char)(127*2) = -2,
    // printf("%02x") promotes to int and prints it as unsigned -> "fffffffe".
    assert_same("x == 0", b"0");
    let c = run(c_binary(), &[], b"0");
    assert_eq!(c.stdout, b"fffffffe\n", "C's own bad() output pinned");
}

#[test]
fn nonzero_takes_the_good_branch() {
    // goodG2B(): data = 2 -> result = 4 -> "04"
    // goodB2G(): data = CHAR_MAX; 127 < 127/2 (63) is false -> printLine(...)
    assert_same("x == 1", b"1");
    let c = run(c_binary(), &[], b"1");
    assert_eq!(
        c.stdout,
        b"04\ndata value is too large to perform arithmetic safely.\n" as &[u8],
        "C's own good() output pinned"
    );
}

#[test]
fn negative_is_nonzero_and_takes_good() {
    assert_same("x == -1", b"-1");
}

// ---------------------------------------------------------------------------
// scanf("%d") input-failure / matching-failure paths: x is left at 0, so the
// program falls into bad(). These are the "error paths" of this program.
// ---------------------------------------------------------------------------

#[test]
fn empty_input_leaves_x_untouched() {
    // scanf returns EOF, x stays 0 -> bad()
    assert_same("empty stdin", b"");
}

#[test]
fn whitespace_only_inputs() {
    assert_same("single newline", b"\n");
    assert_same("spaces only", b"   ");
    assert_same("every C space char", b" \t\n\x0b\x0c\r");
    assert_same("many newlines", b"\n\n\n\n");
}

#[test]
fn matching_failure_non_numeric() {
    assert_same("letters", b"abc");
    assert_same("punctuation", b"!!!");
    assert_same("lone plus", b"+");
    assert_same("lone minus", b"-");
    assert_same("sign then space", b"- 5");
    assert_same("sign then letter", b"+x");
    assert_same("dot first", b".5");
    assert_same("comma", b",");
}

#[test]
fn scanf_reads_across_newlines() {
    // Unlike fgets, %d skips leading whitespace including newlines and finds
    // the number on a later line.
    assert_same("newlines then 42", b"\n\n  \n  42");
    assert_same("leading blank lines then 8", b"\t\x0b\x0c\r\n 8");
    assert_same("newlines then zero", b"\n\n0\n");
}

#[test]
fn only_the_first_conversion_is_consumed() {
    assert_same("several tokens", b"1 2 3\n4 5\n");
    assert_same("first token zero", b"0 1 2\n");
    assert_same("trailing garbage", b"12abc");
    assert_same("zero then garbage", b"0abc");
    assert_same("float is read as int part", b"3.7");
    assert_same("zero point seven", b"0.7");
    assert_same("hex-looking input stops at x", b"0x10");
}

// ---------------------------------------------------------------------------
// Integer edge cases: signedness, truncation and glibc's strtol saturation.
// A long that saturates and is then stored through an `int *` can truncate
// to zero, flipping the branch. Those cases must match exactly.
// ---------------------------------------------------------------------------

#[test]
fn int_boundaries() {
    assert_same("INT_MAX", b"2147483647");
    assert_same("INT_MIN", b"-2147483648");
    assert_same("INT_MAX + 1", b"2147483648");
    assert_same("INT_MIN - 1", b"-2147483649");
    assert_same("UINT_MAX", b"4294967295");
}

#[test]
fn values_that_truncate_to_zero_flip_the_branch() {
    assert_same("2^32", b"4294967296");
    assert_same("2^33", b"8589934592");
    assert_same("-2^32", b"-4294967296");
}

#[test]
fn long_boundaries_and_saturation() {
    assert_same("LONG_MAX", b"9223372036854775807");
    assert_same("LONG_MAX + 1", b"9223372036854775808");
    assert_same("ULONG_MAX + 1", b"18446744073709551616");
    assert_same("twenty nines", b"99999999999999999999");
    assert_same("negative twenty nines", b"-99999999999999999999");
    assert_same("LONG_MIN", b"-9223372036854775808");
    assert_same("LONG_MIN - 1", b"-9223372036854775809");
}

#[test]
fn signed_zero_and_leading_zeros() {
    assert_same("minus zero", b"-0");
    assert_same("plus zero", b"+0");
    assert_same("padded minus zero", b"  -0  ");
    assert_same("many zeros", b"000000");
    assert_same("leading zeros nonzero", b"007");
    assert_same("plus five", b"+5");
    assert_same("zero with trailing newline", b"0\n");
    assert_same("one with trailing newline", b"1\n");
}

#[test]
fn very_long_digit_runs() {
    // Far past any accumulator width; must still agree.
    assert_same("1000 nines", &vec![b'9'; 1000]);
    assert_same("1000 zeros", &vec![b'0'; 1000]);
    let mut minus_nines = vec![b'-'];
    minus_nines.extend(std::iter::repeat(b'9').take(1000));
    assert_same("minus 1000 nines", &minus_nines);
    // A huge run of zeros followed by a 1 is still the value 1.
    let mut zeros_then_one = vec![b'0'; 500];
    zeros_then_one.push(b'1');
    assert_same("500 zeros then 1", &zeros_then_one);
    assert_same("100k nines", &vec![b'9'; 100_000]);
}

// ---------------------------------------------------------------------------
// Non-text / binary stdin.
// ---------------------------------------------------------------------------

#[test]
fn binary_and_non_ascii_stdin() {
    assert_same("NUL bytes then digit", b"\x00\x00 5");
    assert_same("high bytes", b"\xff\xfe");
    assert_same("digit after NUL", b"\x001");
    assert_same("utf8 text", "héllo".as_bytes());
    assert_same("all byte values", &(0u8..=255).collect::<Vec<u8>>());
}

// ---------------------------------------------------------------------------
// main() takes no parameters, so argv must be ignored identically.
// ---------------------------------------------------------------------------

#[test]
fn command_line_arguments_are_ignored() {
    assert_same_with_args("args with empty stdin", &["foo", "bar"], b"");
    assert_same_with_args("args with 1", &["1"], b"1");
    assert_same_with_args("args with 0", &["--help"], b"0");
}

// ---------------------------------------------------------------------------
// Exhaustive-ish sweep so no reachable numeric branch is left untried.
// ---------------------------------------------------------------------------

#[test]
fn sweep_small_integers() {
    for v in -300i32..=300 {
        let s = v.to_string();
        assert_same(&format!("v={v}"), s.as_bytes());
    }
}

#[test]
fn sweep_char_boundary_values() {
    // Values around CHAR_MIN/CHAR_MAX and the CHAR_MAX/2 comparison in goodB2G.
    for v in [
        -129i64, -128, -127, -1, 0, 1, 62, 63, 64, 126, 127, 128, 129, 254, 255, 256, 32767, 32768,
        65535, 65536,
    ] {
        let s = v.to_string();
        assert_same(&format!("boundary v={v}"), s.as_bytes());
    }
}
