//! Differential tests: run the C reference binary and the Rust binary as
//! subprocesses on identical stdin, and require byte-identical stdout,
//! byte-identical stderr and an identical exit status.
//!
//! Nothing here links the Rust code as a library; both programs are driven
//! exactly the way a shell would drive them, because that is how they are
//! compared.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// Path to the Rust binary under test, provided by Cargo.
fn rust_bin() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

fn repo_root() -> PathBuf {
    // translation/ -> repo root
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// Path to the C reference binary, building it with CMake if it is absent.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = repo_root().join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");
        if !exe.exists() {
            std::fs::create_dir_all(&build).expect("create c_src/build");
            let cfg = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("run `cmake ..` (is cmake installed?)");
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
                .expect("run `cmake --build .`");
            assert!(
                bld.status.success(),
                "cmake build failed:\n{}\n{}",
                String::from_utf8_lossy(&bld.stdout),
                String::from_utf8_lossy(&bld.stderr)
            );
        }
        assert!(exe.exists(), "C reference binary missing at {}", exe.display());
        exe
    })
}

#[derive(PartialEq, Eq)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Some(code)` for a normal exit, `None` if killed by a signal.
    code: Option<i32>,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "exit={:?} stdout={:?} stderr={:?}",
            self.code,
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr)
        )
    }
}

/// Spawn `prog`, write `input` to its stdin, and collect the full outcome.
fn run(prog: &Path, input: &[u8]) -> Outcome {
    let mut child = Command::new(prog)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", prog.display()));

    // Write on a helper thread so a program that never drains stdin (e.g. one
    // that stops reading after the first conversion) cannot deadlock us on a
    // large input.
    let mut stdin = child.stdin.take().expect("piped stdin");
    let data = input.to_vec();
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&data);
        let _ = stdin.flush();
        // Dropping `stdin` here closes the pipe, signalling EOF.
    });

    let out = child.wait_with_output().expect("wait_with_output");
    let _ = writer.join();

    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
    }
}

/// Assert the C and Rust programs agree on stdout, stderr and exit status.
#[track_caller]
fn assert_same(label: &str, input: &[u8]) {
    let c = run(c_bin(), input);
    let r = run(rust_bin(), input);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch for {label} (input {:?})\n  C:    {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(input),
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch for {label} (input {:?})\n  C:    {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(input),
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
    assert_eq!(
        c.code, r.code,
        "exit status mismatch for {label} (input {:?}): C {:?} vs Rust {:?}",
        String::from_utf8_lossy(input),
        c.code,
        r.code
    );
}

// ---------------------------------------------------------------------------
// The C program's control flow, and therefore its input classes:
//
//   int x = 0;
//   scanf("%d", &x);      // may leave x == 0 on input/matching failure
//   if (x) good(); else bad();
//
//   good() -> printIntPtrLine(&5)   -> prints "5\n"
//   bad()  -> printIntPtrLine(data) -> `data` is an uninitialized pointer
//
// So the branches to cover are:
//   1. scanf succeeds with a nonzero value            -> good()
//   2. scanf succeeds with a value of zero            -> bad()
//   3. scanf fails on input failure (EOF / all space) -> x stays 0 -> bad()
//   4. scanf fails on matching failure (non-numeric)  -> x stays 0 -> bad()
//   5. values whose 32-bit truncation lands on zero   -> bad()
//   6. values that overflow `long` and saturate       -> depends on truncation
// ---------------------------------------------------------------------------

/// Class 3: input failure — nothing at all on stdin.
#[test]
fn empty_input() {
    assert_same("empty input", b"");
}

/// Class 3: input failure — stdin is only whitespace, so scanf hits EOF while
/// still skipping leading space. Covers every C whitespace character.
#[test]
fn whitespace_only_input() {
    assert_same("single newline", b"\n");
    assert_same("single space", b" ");
    assert_same("all C whitespace", b" \t\n\x0b\x0c\r");
    assert_same("many newlines", b"\n\n\n\n\n");
    assert_same("long space run", &vec![b' '; 5000]);
}

/// Class 2: a single item that is exactly zero -> the bad() branch.
#[test]
fn single_zero() {
    assert_same("bare zero", b"0");
    assert_same("zero with newline", b"0\n");
    assert_same("negative zero", b"-0");
    assert_same("positive zero", b"+0");
    assert_same("padded negative zero", b" -0 ");
    assert_same("many leading zeros", b"0000000000000000000000000000");
}

/// Class 1: a single nonzero item -> the good() branch, which must print "5\n"
/// regardless of the value read.
#[test]
fn single_nonzero() {
    assert_same("one", b"1");
    assert_same("five", b"5");
    assert_same("negative three", b"-3");
    assert_same("explicit plus", b"+7");
    assert_same("large-ish", b"123456");
    assert_same("nonzero with newline", b"42\n");
}

/// scanf("%d") skips leading whitespace and reads *across* newlines, unlike
/// fgets. A value on the second line must still be found.
#[test]
fn scanf_reads_across_newlines() {
    assert_same("leading blank lines then value", b"\n\n  42");
    assert_same("spaces and newlines then value", b"   \n  42");
    assert_same("tab then value", b"\t3");
    assert_same("vertical tab then value", b"\x0b3");
    assert_same("form feed then value", b"\x0c3");
    assert_same("carriage return then value", b"\r3");
    assert_same("value on third line", b"\n\n7\n");
    assert_same("blank lines then zero", b"\n\n\n0");
}

/// Class 4: matching failure — the first non-space byte cannot start an
/// integer, so scanf returns 0 and `x` keeps its initial value of 0.
#[test]
fn matching_failure_leaves_x_zero() {
    assert_same("letters", b"abc");
    assert_same("leading dot", b".5");
    assert_same("comma", b",");
    assert_same("space then punctuation", b"     ,,,");
    assert_same("hex-ish", b"0x10"); // %d stops after the leading 0
    assert_same("word then number", b"abc 1");
    assert_same("newline then letters", b"\nzz");
}

/// Class 4: a sign that is not followed by a digit is a matching failure.
#[test]
fn sign_without_digits() {
    assert_same("minus only", b"-");
    assert_same("plus only", b"+");
    assert_same("minus then junk", b"-x");
    assert_same("plus then junk", b"+x");
    assert_same("minus then newline then digit", b"-\n5");
    assert_same("double sign", b"--5");
    assert_same("sign then space then digit", b"- 5");
}

/// The conversion stops at the first non-digit; trailing junk is simply left
/// unread, and nothing else in the program reads stdin.
#[test]
fn trailing_junk_after_number() {
    assert_same("nonzero then letters", b"7abc");
    assert_same("zero then letters", b"0abc");
    assert_same("zero then space then nonzero", b"0 1");
    assert_same("nonzero then space then zero", b"1 0");
    assert_same("decimal point", b"3.9");
    assert_same("zero point nine", b"0.9");
}

/// The extremes of `int`, and the values just past them where glibc's %d
/// saturates the internal `long` and then truncates into an `int`.
#[test]
fn int_boundaries_and_overflow() {
    assert_same("INT_MAX", b"2147483647");
    assert_same("INT_MIN", b"-2147483648");
    assert_same("INT_MAX+1", b"2147483648");
    assert_same("INT_MIN-1", b"-2147483649");
    assert_same("LONG_MAX", b"9223372036854775807");
    assert_same("LONG_MAX+1", b"9223372036854775808");
    assert_same("LONG_MIN", b"-9223372036854775808");
    assert_same("past LONG_MAX", b"18446744073709551616");
    assert_same("past LONG_MIN", b"-18446744073709551616");
    assert_same("twenty digits", b"100000000000000000000");
    assert_same("twenty five digits", b"9999999999999999999999999");
}

/// Truncation to 32 bits can turn a nonzero number into a zero `x`, flipping
/// the branch from good() to bad(). These are the inputs that catch a
/// translation that widened or saturated instead of truncating.
#[test]
fn truncation_to_zero_flips_the_branch() {
    assert_same("2^32", b"4294967296");
    assert_same("-2^32", b"-4294967296");
    assert_same("2^33", b"8589934592");
    assert_same("2^32+1", b"4294967297");
    assert_same("2^34", b"17179869184");
    assert_same("100 * 2^32", b"429496729600");
    assert_same("2^35", b"34359738368");
    assert_same("2^36", b"68719476736");
}

/// Powers of ten with growing exponents walk `x` through both truncated-zero
/// and truncated-nonzero results, and past the point where the internal
/// accumulator overflows.
#[test]
fn powers_of_ten() {
    for k in 0..25 {
        let pos = format!("1{}", "0".repeat(k));
        assert_same(&format!("10^{k}"), pos.as_bytes());
        let neg = format!("-1{}", "0".repeat(k));
        assert_same(&format!("-10^{k}"), neg.as_bytes());
    }
}

/// Bytes that are not valid UTF-8 must not make the Rust program behave
/// differently from the C one, which just sees bytes.
#[test]
fn non_utf8_and_nul_bytes() {
    assert_same("NUL then digit", b"\x001");
    assert_same("high bytes then digit", b"\xff\xff1");
    assert_same("UTF-8 BOM then digit", b"\xef\xbb\xbf1");
    assert_same("lone continuation byte", b"\x80");
    assert_same("invalid sequence then zero", b"\xc3\x280");
    assert_same("all high bytes", b"\xfe\xfd\xfc");
}

/// A digit stream far longer than any internal buffer, exercising the
/// accumulate-and-saturate loop as well as the "stop reading early" behavior.
#[test]
fn very_long_inputs() {
    let long_digits = "9".repeat(4096);
    assert_same("4096 nines", long_digits.as_bytes());

    let long_zeros = "0".repeat(4096);
    assert_same("4096 zeros", long_zeros.as_bytes());

    // A short number followed by a megabyte of unread junk: the program must
    // exit without waiting to consume it all.
    let mut tail = b"1\n".to_vec();
    tail.extend(std::iter::repeat(b'x').take(1 << 20));
    assert_same("number then 1MiB of junk", &tail);

    let mut zero_tail = b"0\n".to_vec();
    zero_tail.extend(std::iter::repeat(b'x').take(1 << 20));
    assert_same("zero then 1MiB of junk", &zero_tail);
}

/// The bad() branch dereferences an uninitialized pointer. Whatever the
/// reference build does there, it must be *stable*, and the Rust program must
/// reproduce it on every run.
#[test]
fn uninitialized_pointer_branch_is_stable() {
    for _ in 0..30 {
        assert_same("repeated bad() branch", b"0");
        assert_same("repeated bad() branch (EOF)", b"");
    }
}

/// A deterministic pseudo-random sweep over the alphabet the parser branches
/// on, to catch input classes the hand-written cases missed.
#[test]
fn randomized_sweep() {
    const ALPHABET: &[u8] = b" \t\n\r\x0b\x0c+-0123456789xabc.,\x00\xff";
    // xorshift64*, so the sweep is reproducible without an extra dependency.
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = move || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state.wrapping_mul(0x2545_F491_4F6C_DD1D)
    };

    for case in 0..400 {
        let len = (next() % 11) as usize;
        let input: Vec<u8> = (0..len)
            .map(|_| ALPHABET[(next() % ALPHABET.len() as u64) as usize])
            .collect();
        assert_same(&format!("random case {case}"), &input);
    }
}
