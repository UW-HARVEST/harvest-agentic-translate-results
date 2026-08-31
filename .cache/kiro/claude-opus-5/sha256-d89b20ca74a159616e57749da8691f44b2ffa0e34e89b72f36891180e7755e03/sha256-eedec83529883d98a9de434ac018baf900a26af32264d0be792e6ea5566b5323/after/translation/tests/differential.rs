//! Differential tests: run the C binary and the Rust binary as *subprocesses*,
//! feed both the same bytes on stdin, and require byte-identical stdout,
//! byte-identical stderr and an identical exit status.
//!
//! Nothing here links against the Rust code as a library; both programs are
//! driven exactly the way a shell would drive them, because that is how the
//! translation is graded.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// Path to the Rust binary built by cargo for this integration test.
fn rust_bin() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the C binary, building it with CMake on first use if necessary.
/// `c_src/` is only ever *read* / built out-of-tree into `c_src/build`.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = workspace_root().join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");
        if !exe.exists() {
            std::fs::create_dir_all(&build).expect("create c_src/build");
            let configure = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("failed to run `cmake ..` (is cmake installed?)");
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
                .expect("failed to run `cmake --build .`");
            assert!(
                compile.status.success(),
                "cmake build failed:\n{}\n{}",
                String::from_utf8_lossy(&compile.stdout),
                String::from_utf8_lossy(&compile.stderr)
            );
        }
        assert!(
            exe.exists(),
            "C binary not found at {} after building",
            exe.display()
        );
        exe
    })
}

struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Some(code)` for a normal exit, `None` if killed by a signal.
    code: Option<i32>,
}

fn run(program: &Path, stdin_bytes: &[u8]) -> Run {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", program.display()));

    {
        let mut sin = child.stdin.take().expect("stdin piped");
        // The child may legitimately stop reading; a broken pipe here is not a
        // test failure, so the error is deliberately swallowed.
        let _ = sin.write_all(stdin_bytes);
        let _ = sin.flush();
    }

    let out = child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("failed to wait on {}: {e}", program.display()));

    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
    }
}

/// Lockstep, byte-for-byte comparison of the first `budget` bytes of stdout of
/// both programs, without buffering the whole stream.
///
/// This exists only for inputs where `x` ends up at `INT_MAX`: those make both
/// programs emit ~2.1 billion lines (~47 GiB), so a full capture is not a
/// practical test. The comparison is still byte-exact, just bounded, and it
/// additionally proves that neither program terminated early (both must still
/// be producing output when the budget runs out).
#[track_caller]
fn assert_same_stdout_prefix(label: &str, stdin_bytes: &[u8], budget: usize) {
    fn spawn(program: &Path, stdin_bytes: &[u8]) -> std::process::Child {
        let mut child = Command::new(program)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", program.display()));
        let mut sin = child.stdin.take().expect("stdin piped");
        let _ = sin.write_all(stdin_bytes);
        let _ = sin.flush();
        drop(sin); // signal EOF
        child
    }

    let mut c_child = spawn(c_bin(), stdin_bytes);
    let mut r_child = spawn(rust_bin(), stdin_bytes);
    let mut c_out = c_child.stdout.take().expect("stdout piped");
    let mut r_out = r_child.stdout.take().expect("stdout piped");

    let mut compared = 0usize;
    let mut c_buf = Vec::new();
    let mut r_buf = Vec::new();
    let mut chunk = vec![0u8; 64 * 1024];

    while compared < budget {
        // Top both buffers up so there is something to compare.
        while c_buf.is_empty() {
            match c_out.read(&mut chunk).expect("read C stdout") {
                0 => break,
                n => c_buf.extend_from_slice(&chunk[..n]),
            }
        }
        while r_buf.is_empty() {
            match r_out.read(&mut chunk).expect("read Rust stdout") {
                0 => break,
                n => r_buf.extend_from_slice(&chunk[..n]),
            }
        }
        assert!(
            !c_buf.is_empty() && !r_buf.is_empty(),
            "[{label}] one program stopped early at byte {compared}: \
             C exhausted={} Rust exhausted={}",
            c_buf.is_empty(),
            r_buf.is_empty()
        );

        let n = c_buf.len().min(r_buf.len()).min(budget - compared);
        assert!(
            c_buf[..n] == r_buf[..n],
            "[{label}] stdout differs within bytes {}..{}\nC   ={:?}\nRust={:?}",
            compared,
            compared + n,
            show(&c_buf[..n]),
            show(&r_buf[..n])
        );
        c_buf.drain(..n);
        r_buf.drain(..n);
        compared += n;
    }

    // Both were still streaming; stop them and confirm neither wrote to stderr.
    let _ = c_child.kill();
    let _ = r_child.kill();
    let mut c_err = Vec::new();
    let mut r_err = Vec::new();
    let _ = c_child
        .stderr
        .take()
        .expect("stderr piped")
        .read_to_end(&mut c_err);
    let _ = r_child
        .stderr
        .take()
        .expect("stderr piped")
        .read_to_end(&mut r_err);
    let _ = c_child.wait();
    let _ = r_child.wait();
    assert!(
        c_err == r_err,
        "[{label}] stderr differs\nC   ={:?}\nRust={:?}",
        show(&c_err),
        show(&r_err)
    );
    assert_eq!(compared, budget, "[{label}] budget not fully compared");
}

/// Render bytes for assertion messages without drowning the output.
fn show(bytes: &[u8]) -> String {
    const LIMIT: usize = 400;
    let head = &bytes[..bytes.len().min(LIMIT)];
    let mut s = String::from_utf8_lossy(head).into_owned();
    if bytes.len() > LIMIT {
        s.push_str(&format!("... <{} bytes total>", bytes.len()));
    }
    s
}

/// The core assertion: same stdin ⇒ same stdout, stderr and exit status.
#[track_caller]
fn assert_same(label: &str, stdin_bytes: &[u8]) {
    let c = run(c_bin(), stdin_bytes);
    let r = run(rust_bin(), stdin_bytes);

    assert_eq!(
        c.code, r.code,
        "[{label}] exit status differs: C={:?} Rust={:?}\ninput={:?}",
        c.code,
        r.code,
        show(stdin_bytes)
    );
    assert!(
        c.stderr == r.stderr,
        "[{label}] stderr differs\ninput={:?}\nC   ={:?}\nRust={:?}",
        show(stdin_bytes),
        show(&c.stderr),
        show(&r.stderr)
    );
    assert!(
        c.stdout == r.stdout,
        "[{label}] stdout differs ({} vs {} bytes)\ninput={:?}\nC   ={:?}\nRust={:?}",
        c.stdout.len(),
        r.stdout.len(),
        show(stdin_bytes),
        show(&c.stdout),
        show(&r.stdout)
    );
}

#[track_caller]
fn same(label: &str, stdin_text: &str) {
    assert_same(label, stdin_text.as_bytes());
}

// ---------------------------------------------------------------------------
// Phase A sanity: both binaries exist and are runnable.
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_run() {
    let c = run(c_bin(), b"2");
    let r = run(rust_bin(), b"2");
    assert_eq!(c.code, Some(0), "C binary should exit 0");
    assert_eq!(r.code, Some(0), "Rust binary should exit 0");
    assert_eq!(c.stdout, b"0 0\n1 2\n".to_vec(), "C reference output");
    assert_eq!(r.stdout, c.stdout, "Rust must match the C reference output");
}

// ---------------------------------------------------------------------------
// Phase B: the branches `main`/`driver` actually take.
//
// `main` has exactly one input-dependent step (`scanf("%d", &x)`) and `driver`
// has exactly one branch (`i < x`, tested before the first iteration). So the
// input classes are: whether the scanf conversion succeeds, and the sign /
// magnitude of the resulting `int`.
// ---------------------------------------------------------------------------

#[test]
fn empty_input_leaves_x_at_zero() {
    // scanf returns EOF, x keeps its initializer 0, driver prints nothing.
    same("empty", "");
}

#[test]
fn whitespace_only_input() {
    // scanf skips whitespace then hits EOF: still no assignment.
    same("spaces", "   ");
    same("newlines", "\n\n\n");
    same("mixed ws", " \t\n\r\x0b\x0c ");
}

#[test]
fn zero_prints_nothing() {
    same("0", "0");
    same("0 + newline", "0\n");
    same("-0", "-0");
    same("+0", "+0");
}

#[test]
fn single_item() {
    same("1", "1");
    same("1 + newline", "1\n");
}

#[test]
fn small_counts() {
    for x in 2..=12 {
        same(&format!("x={x}"), &format!("{x}\n"));
    }
}

#[test]
fn negative_counts_print_nothing() {
    for x in [-1, -2, -7, -100, -32768, -2147483647i64, -2147483648i64] {
        same(&format!("x={x}"), &format!("{x}\n"));
    }
}

#[test]
fn explicit_plus_sign_is_accepted() {
    same("+4", "+4");
    same("+1", "+1");
    same("+0007", "+0007");
}

#[test]
fn leading_whitespace_is_skipped_across_newlines() {
    // `scanf` whitespace skipping crosses newlines, unlike `fgets`.
    same("newlines then value", "\n\n  7");
    same("tabs then value", "\t\t6\n");
    same("vtab/formfeed then value", "\x0b\x0c3");
    same("crlf then value", "\r\n5");
}

#[test]
fn leading_zeros_are_decimal_not_octal() {
    same("007", "007");
    same("0000000010", "0000000010");
    same("many zeros then 5", "000000000000000000005");
}

#[test]
fn trailing_garbage_after_the_number_is_ignored() {
    // Only one conversion is performed; the rest of stdin is never read.
    same("3abc", "3abc");
    same("3 then second number", "3 99");
    same("3 then newline junk", "3\nnot-a-number\n");
    same("0x10 stops at x", "0x10");
    same("1e3 stops at e", "1e3");
    same("2.9 stops at dot", "2.9");
    same("4-5", "4-5");
}

// ---------------------------------------------------------------------------
// Phase B: every input that reaches the "conversion failed" path, where `x`
// keeps its initial 0 and nothing is printed.
// ---------------------------------------------------------------------------

#[test]
fn matching_failure_leaves_x_at_zero() {
    same("letters", "abc");
    same("leading dot", ".5");
    same("bare minus", "-");
    same("bare plus", "+");
    same("sign then letter", "-a");
    same("double sign", "+-3");
    same("minus space digits", "- 3");
    same("plus space digits", "+ 3");
    same("punctuation", "!!!");
    same("comma", ",5");
    same("hash", "#5");
    same("unicode digit", "٥"); // Arabic-Indic five: not an ASCII digit
    same("newline only after sign", "-\n5");
}

#[test]
fn non_utf8_bytes_on_stdin() {
    // The C program reads bytes, not UTF-8; so must the Rust one.
    assert_same("0xff first", b"\xff5");
    assert_same("value then 0xff", b"5\xff");
    assert_same("nul first", b"\x005");
    assert_same("value then nul", b"5\x00");
    assert_same("lone continuation byte", b"\x80\x81\x82");
    assert_same("truncated utf8 then value", b"\xe2\x82 4");
}

// ---------------------------------------------------------------------------
// Phase C: integer width, overflow, truncation and signedness, exactly as the
// C library performs them. glibc converts `%d` via a long, clamping at
// LONG_MAX / LONG_MIN, and only then narrows the result to `int`.
// ---------------------------------------------------------------------------

#[test]
fn values_at_the_int_boundaries() {
    // 2^31: narrows to INT_MIN, so nothing is printed.
    same("2147483648", "2147483648");
    same("-2147483648", "-2147483648");
    // 2^31+1: narrows to -(2^31-1), still negative, nothing printed.
    same("2147483649", "2147483649");
}

#[test]
fn int_max_is_the_maximum_the_code_handles() {
    // INT_MAX is the largest `x` that makes `driver` loop; it emits 2_147_483_647
    // lines (~47 GiB), so the streams are compared in lockstep over a bounded
    // prefix instead of being captured whole. `-2147483649` is included because
    // glibc converts it as a long and *then* narrows to int, which lands on
    // INT_MAX -- a large positive count from a negative-looking input.
    const BUDGET: usize = 8 * 1024 * 1024;
    assert_same_stdout_prefix("x=INT_MAX", b"2147483647", BUDGET);
    assert_same_stdout_prefix("-2147483649 narrows to INT_MAX", b"-2147483649", BUDGET);
}

#[test]
fn values_that_wrap_modulo_2_pow_32() {
    same("2^32 -> 0", "4294967296");
    same("2^32+1 -> 1", "4294967297");
    same("2^32+4 -> 4", "4294967300");
    same("-(2^32) -> 0", "-4294967296");
    same("-(2^32-1) -> 1", "-4294967295");
    same("-(2^32+3) -> -3", "-4294967299");
    same("2*2^32+2 -> 2", "8589934594");
}

#[test]
fn values_at_and_beyond_the_long_boundaries() {
    same("LONG_MAX", "9223372036854775807");
    same("LONG_MAX+1", "9223372036854775808");
    same("LONG_MIN", "-9223372036854775808");
    same("LONG_MIN-1", "-9223372036854775809");
    same("10^19", "10000000000000000000");
    same("2^64", "18446744073709551616");
    same("2^64+1", "18446744073709551617");
    same("far past 2^64", "99999999999999999999999999");
    same("negative far past 2^64", "-99999999999999999999999999");
}

#[test]
fn absurdly_long_digit_runs() {
    same("300 nines", &"9".repeat(300));
    same("negative 300 nines", &format!("-{}", "9".repeat(300)));
    same("1000 zeros", &"0".repeat(1000));
    same("1000 zeros then 3", &format!("{}3", "0".repeat(1000)));
    same("leading zeros then overflow", &format!("{}{}", "0".repeat(20), "9".repeat(40)));
}

// ---------------------------------------------------------------------------
// Phase C: output volume — `printf` formatting and the exact number of lines,
// including the two-digit / multi-digit column spacing.
// ---------------------------------------------------------------------------

#[test]
fn output_formatting_across_digit_widths() {
    // Crosses 1->2, 2->3, 3->4 and 4->5 digit widths for both i and j.
    same("x=6 (j crosses 10)", "6");
    same("x=51 (i crosses 10, j crosses 100)", "51");
    same("x=501", "501");
    same("x=5001", "5001");
}

#[test]
fn large_output_matches_byte_for_byte() {
    same("x=100000", "100000");
}

#[test]
fn exhaustive_small_range() {
    // Every x in a contiguous range around the interesting boundary values.
    for x in -5..=40 {
        same(&format!("sweep x={x}"), &format!("{x}"));
    }
}

#[test]
fn no_trailing_newline_on_input_is_fine() {
    same("no newline, 4", "4");
    same("no newline, empty-ish", " ");
}

// ---------------------------------------------------------------------------
// Phase C: a deterministic randomised sweep, to catch input shapes not
// enumerated by hand. Numeric literals are capped at four digits so that no
// single case can ask for gigabytes of output; the large/overflow magnitudes are
// covered explicitly by the tests above.
// ---------------------------------------------------------------------------

#[test]
fn deterministic_fuzz_sweep() {
    // Small xorshift so the corpus is reproducible without a dependency.
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    const ALPHABET: &[u8] = b"0123456789+- \t\n\r\x0b\x0cabxeE.,#\x00\xff";

    for case in 0..300 {
        let len = (next() % 12) as usize;
        let mut input = Vec::with_capacity(len + 4);
        let mut digit_run = 0usize;
        for _ in 0..len {
            let mut b = ALPHABET[(next() as usize) % ALPHABET.len()];
            if b.is_ascii_digit() {
                digit_run += 1;
                // Cap the magnitude of any single numeric literal.
                if digit_run > 4 {
                    b = b' ';
                    digit_run = 0;
                }
            } else {
                digit_run = 0;
            }
            input.push(b);
        }
        assert_same(&format!("fuzz case {case}"), &input);
    }
}


// ---------------------------------------------------------------------------
// Phase C: stdout closed by the reader mid-stream. The C program runs with the
// default SIGPIPE disposition, so it dies by signal 13; the Rust runtime
// installs SIG_IGN before `main`, which would otherwise make it exit 0.
// ---------------------------------------------------------------------------

#[test]
#[cfg(unix)]
fn closed_stdout_kills_both_the_same_way() {
    use std::os::unix::process::ExitStatusExt;

    /// Start the program, read a handful of bytes, then close the read end and
    /// report how the process terminated.
    fn terminate_reader_early(program: &Path) -> (Option<i32>, Option<i32>) {
        let mut child = Command::new(program)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", program.display()));
        {
            let mut sin = child.stdin.take().expect("stdin piped");
            // Large enough that the writer cannot fit everything in the pipe buffer.
            let _ = sin.write_all(b"2000000");
            let _ = sin.flush();
        }
        {
            let mut sout = child.stdout.take().expect("stdout piped");
            let mut buf = [0u8; 8];
            let _ = sout.read(&mut buf);
            // Dropping `sout` closes the read end of the pipe.
        }
        let status = child.wait().expect("wait on child");
        (status.code(), status.signal())
    }

    let c = terminate_reader_early(c_bin());
    let r = terminate_reader_early(rust_bin());
    assert_eq!(
        c, r,
        "closed stdout: C (code, signal) = {c:?} but Rust = {r:?}"
    );
}
