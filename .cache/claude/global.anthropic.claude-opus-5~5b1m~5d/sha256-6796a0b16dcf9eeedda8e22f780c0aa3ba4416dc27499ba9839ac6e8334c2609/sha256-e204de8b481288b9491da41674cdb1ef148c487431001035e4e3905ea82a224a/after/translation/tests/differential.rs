//! Differential tests: run the original C program and the Rust translation as
//! subprocesses on identical stdin, and compare stdout, stderr and exit status
//! byte for byte / value for value.
//!
//! Nothing here links the Rust code as a library; both programs are driven
//! exactly the way a shell would drive them.

use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// locating / building the two binaries
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// The Rust binary under test, built by cargo for us.
fn rust_bin() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// The C binary. Built with cmake on first use if it is not already present.
/// `c_src/` is never modified -- only the out-of-tree `c_src/build/` dir is
/// written, which is what the documented build procedure does.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = repo_root().join("c_src");
        let build = c_src.join("build");
        let bin = build.join("driver");
        if bin.exists() {
            return bin;
        }

        std::fs::create_dir_all(&build).expect("cannot create c_src/build");
        let configure = Command::new("cmake")
            .arg("..")
            .current_dir(&build)
            .output()
            .expect("failed to run `cmake ..` -- is cmake installed?");
        assert!(
            configure.status.success(),
            "cmake configure failed:\n{}\n{}",
            String::from_utf8_lossy(&configure.stdout),
            String::from_utf8_lossy(&configure.stderr),
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
            String::from_utf8_lossy(&compile.stderr),
        );

        assert!(
            bin.exists(),
            "C binary still missing after building: {}",
            bin.display()
        );
        bin
    })
}

// ---------------------------------------------------------------------------
// running a program
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq)]
struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    code: Option<i32>,
    signal: Option<i32>,
}

impl std::fmt::Debug for Run {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Run")
            .field("stdout", &String::from_utf8_lossy(&self.stdout))
            .field("stderr", &String::from_utf8_lossy(&self.stderr))
            .field("code", &self.code)
            .field("signal", &self.signal)
            .finish()
    }
}

impl Run {
    fn died_from_signal(&self) -> bool {
        self.signal.is_some()
    }
}

/// How far the top of the process stack sits above `bad()`'s frame depends on
/// the combined size of argv and the environment, because the kernel copies
/// both onto the stack above `main`. That distance is what decides which
/// out-of-bounds indices are still on a mapped page.
///
/// Every run therefore gets the same generous padding variable, which pushes
/// the stack top far above `bad()`'s frame for BOTH programs alike. That is
/// what makes the "invisible" and "far past the stack" index ranges genuinely
/// deterministic instead of dependent on whatever environment the test happened
/// to be launched from.
const STACK_PAD_VAR: &str = "DIFFTEST_STACK_PAD";

fn stack_pad() -> String {
    "x".repeat(64 * 1024)
}

/// Run `bin` with `input` piped to its stdin, capturing everything.
fn run(bin: &Path, input: &[u8]) -> Run {
    let mut child = Command::new(bin)
        .env(STACK_PAD_VAR, stack_pad())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    {
        let mut stdin = child.stdin.take().expect("piped stdin");
        // The program may die (SIGSEGV) before draining stdin, so a broken pipe
        // here is an expected outcome rather than a test failure.
        let _ = stdin.write_all(input);
        let _ = stdin.flush();
    }

    let out = child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("failed to wait for {}: {e}", bin.display()));

    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

/// Run the program with stdin closed entirely (not merely empty), so the very
/// first `fgets` fails on a read error rather than on end-of-file.
fn run_closed_stdin(bin: &Path) -> Run {
    let out = Command::new(bin)
        .env(STACK_PAD_VAR, stack_pad())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

// ---------------------------------------------------------------------------
// assertions
// ---------------------------------------------------------------------------

/// The core assertion: for this input the C and the Rust must agree on all
/// three observable channels.
#[track_caller]
fn assert_identical(label: &str, input: &[u8]) {
    let c = run(c_bin(), input);
    let r = run(rust_bin(), input);

    assert_eq!(
        c.stdout,
        r.stdout,
        "[{label}] stdout differs for input {:?}\n C stdout: {:?}\n R stdout: {:?}",
        String::from_utf8_lossy(input),
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout),
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "[{label}] stderr differs for input {:?}\n C stderr: {:?}\n R stderr: {:?}",
        String::from_utf8_lossy(input),
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr),
    );
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "[{label}] exit status differs for input {:?} (code, signal)",
        String::from_utf8_lossy(input),
    );
}

/// Convenience: build the two-line stdin the program actually consumes.
/// The first line feeds `goodB2G()`, the second feeds `bad()`.
fn two_lines(first: &str, second: &str) -> Vec<u8> {
    format!("{first}\n{second}\n").into_bytes()
}

// ---------------------------------------------------------------------------
// Phase B / C: the input classes the C branches on
// ---------------------------------------------------------------------------

/// `fgets` returns NULL in BOTH goodB2G and bad: the "fgets() failed." branch
/// twice, then the two negative/out-of-bounds error branches.
#[test]
fn empty_input() {
    assert_identical("empty", b"");
}

/// stdin closed rather than empty: still the NULL-return path.
#[test]
fn closed_stdin() {
    let c = run_closed_stdin(c_bin());
    let r = run_closed_stdin(rust_bin());
    assert_eq!(c.stdout, r.stdout, "closed stdin: stdout differs");
    assert_eq!(c.stderr, r.stderr, "closed stdin: stderr differs");
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "closed stdin: exit status differs"
    );
}

/// Exactly one line available: goodB2G consumes it, bad's fgets hits EOF.
#[test]
fn single_line_only() {
    for input in [
        &b"3"[..],       // no trailing newline
        &b"3\n"[..],     // with trailing newline
        &b"\n"[..],      // just a newline -> atoi("\n") == 0
        &b"-1"[..],      // negative, no newline
        &b"abc\n"[..],   // non-numeric -> atoi == 0
    ] {
        assert_identical("single-line", input);
    }
}

/// A lone newline for each of the two reads.
#[test]
fn blank_lines() {
    assert_identical("two-blank-lines", b"\n\n");
    assert_identical("blank-then-value", b"\n5\n");
}

/// Every in-bounds index, exercising `buffer[data] = 1` at each position and
/// the `data >= 0 && data < 10` accept branch of goodB2G.
#[test]
fn in_bounds_indices() {
    for i in 0..10 {
        let s = i.to_string();
        assert_identical("in-bounds", &two_lines(&s, &s));
    }
}

/// The maximum in-bounds index and the first out-of-bounds one, which is where
/// goodB2G's guard flips to the "out-of-bounds" error branch.
#[test]
fn guard_boundary_in_good_b2g() {
    assert_identical("b2g-accept-9", &two_lines("9", "0"));
    assert_identical("b2g-reject-10", &two_lines("10", "0"));
    assert_identical("b2g-reject-neg1", &two_lines("-1", "0"));
    assert_identical("b2g-reject-huge", &two_lines("2147483647", "0"));
}

/// Negative values: the `data >= 0` reject branch of bad() ("Array index is
/// negative.") and the reject branch of goodB2G.
#[test]
fn negative_indices() {
    for v in ["-1", "-2", "-9", "-10", "-2147483648", "-99999999999999"] {
        assert_identical("negative", &two_lines(v, v));
    }
}

/// `atoi` corner cases: leading whitespace, signs, trailing junk, no digits.
/// All of these must parse to the same int the C's atoi produces.
#[test]
fn atoi_parsing_variants() {
    for v in [
        "   4",   // leading spaces
        "\t\t5",  // leading tabs
        "+6",     // explicit plus
        "-0",     // negative zero -> 0, takes the accept branch
        "0x10",   // base 10 only -> 0
        "7abc",   // trailing junk ignored
        "- 3",    // space after sign -> 0
        "abc",    // no digits -> 0
        "     ",  // whitespace only -> 0
        "0000008",
        ".5",
        "+",
        "-",
    ] {
        assert_identical("atoi", &two_lines(v, v));
    }
}

/// Integer overflow and truncation, exactly as C's `(int) strtol` does it.
///
/// The overflowing text goes on the FIRST line, which feeds `goodB2G()`. That
/// sink is range-checked, so any parsed value is safe to observe there: a
/// mis-parse changes which of the two goodB2G branches is taken, or which of
/// the ten slots is set, and is therefore still caught. Feeding these to
/// `bad()` instead would index gigabytes off the stack, which is exactly the
/// non-deterministic region documented in ERRORS.md.
#[test]
fn atoi_overflow_and_truncation() {
    for v in [
        "2147483647",           // INT_MAX
        "2147483648",           // INT_MAX + 1 -> truncates to INT_MIN
        "4294967296",           // 2^32 -> truncates to 0
        "4294967297",           // 2^32 + 1 -> truncates to 1
        "4294967305",           // 2^32 + 9 -> truncates to 9, the last in-bounds index
        "-2147483648",          // INT_MIN
        "-2147483649",          // truncates to INT_MAX
        "-4294967296",          // truncates to 0
        "1234567890123",        // 13 chars, fits the fgets window exactly
        "9223372036854775807",  // LONG_MAX (split by fgets)
        "99999999999999999999", // beyond LONG_MAX -> saturates, then truncates
    ] {
        assert_identical("overflow-first-line", &two_lines(v, "0"));
    }

    // Values whose truncated result is a safe index for bad() as well, so the
    // truncation is checked through the unguarded sink too. (INT_MAX-valued
    // results are deliberately excluded here: as a bad() index they reach the
    // non-deterministic SIGSEGV/SIGBUS region.)
    for v in ["4294967296", "4294967297", "4294967305", "2147483648", "-4294967296"] {
        assert_identical("overflow-both-lines", &two_lines("0", v));
    }
}

/// `fgets` reads at most 13 bytes, so a longer line is SPLIT between the
/// goodB2G read and the bad() read. This is the buffer-size branch.
#[test]
fn fgets_13_byte_limit_splits_long_lines() {
    for input in [
        &b"1234567890123\n"[..],        // exactly 13 chars + newline
        &b"12345678901234\n"[..],       // 14 chars: split across both reads
        &b"1111111111111\n"[..],        // 13 chars, no split
        &b"00000000000009\n"[..],       // split: "0000000000000" then "9\n"
        &b"aaaaaaaaaaaaaaaaaaaaaaaa\n"[..], // long non-numeric, split
        &b"             1\n"[..],       // 13 spaces then a digit -> split
    ] {
        assert_identical("fgets-limit", input);
    }
}

/// Only the first two reads matter; any extra input is ignored.
#[test]
fn extra_input_is_ignored() {
    assert_identical("three-lines", b"1\n2\n3\n");
    assert_identical("many-lines", b"5\n5\n5\n5\n5\n");
}

/// Carriage returns and embedded NUL bytes inside the 13-byte window.
#[test]
fn odd_bytes_in_input() {
    assert_identical("crlf", b"5\r\n5\r\n");
    assert_identical("cr-only", b"5\r5\r");
    assert_identical("embedded-nul", b"5\x00 9\n7\n");
    assert_identical("nul-first", b"\x005\n\x007\n");
    assert_identical("vtab-ff", b"\x0b\x0c8\n\x0b\x0c8\n");
}

/// Out-of-bounds writes in bad() that land in DEAD stack padding: the store has
/// no visible effect, the ten zeros are printed and the process exits 0.
/// Verified deterministic over 100 runs of the C for each value.
#[test]
fn oob_writes_that_are_invisible() {
    let mut values: Vec<i64> = vec![10, 11, 12, 13, 14, 15, 20, 21, 22, 23, 24, 25];
    values.extend(28..=60);
    values.extend([100, 200, 300, 500, 800, 1000, 1500, 2000]);
    // With the environment padding the harness installs, the stack top sits far
    // above bad()'s frame, so these much larger indices are still mapped memory
    // and still invisible. Verified 0 faults in 25 runs per value.
    values.extend([4000, 8000, 12_000, 15_000, 16_000, 17_000]);
    for v in values {
        assert_identical("oob-invisible", &two_lines("0", &v.to_string()));
    }
}

/// Out-of-bounds writes in bad() that clobber the LIVE control slots of the
/// frame (saved frame pointer / return address). The C dies from a fatal signal
/// with both streams empty, because the block-buffered stdout is never flushed.
/// Measured 100/100 reproducible for each index.
#[test]
fn oob_writes_that_smash_the_frame() {
    for v in [16, 17, 18, 19, 26, 27] {
        assert_identical("oob-fatal-slot", &two_lines("0", &v.to_string()));
    }
}

/// The fatal control-slot indices are stable, not a coin flip: repeat them.
#[test]
fn frame_smashing_is_reproducible() {
    for v in [16, 19, 26, 27] {
        let input = two_lines("0", &v.to_string());
        for attempt in 0..12 {
            let c = run(c_bin(), &input);
            let r = run(rust_bin(), &input);
            assert!(
                c.died_from_signal(),
                "index {v} attempt {attempt}: expected the C to die from a signal, got {c:?}"
            );
            assert_eq!(
                (c.code, c.signal),
                (r.code, r.signal),
                "index {v} attempt {attempt}: exit status differs"
            );
            assert_eq!(c.stdout, r.stdout, "index {v} attempt {attempt}: stdout differs");
            assert_eq!(c.stderr, r.stderr, "index {v} attempt {attempt}: stderr differs");
        }
    }
}

/// Indices far past the top of the stack: the C reliably faults with SIGSEGV
/// and no output. The values here are chosen with a wide margin above the
/// ASLR-sensitive band (which, with the harness's environment padding, ends
/// around index 21000), so the outcome does not depend on the per-exec random
/// stack offset.
#[test]
fn oob_writes_far_past_the_stack() {
    for v in [
        22_000, 24_000, 30_000, 50_000, 100_000, 200_000, 500_000, 1_000_000, 2_000_000,
        10_000_000, 20_000_000, 33_000_000,
    ] {
        assert_identical("oob-far", &two_lines("0", &v.to_string()));
    }
}

/// Long input lines whose SECOND fgets chunk lands in the reliably fatal range.
#[test]
fn split_line_producing_fatal_second_index() {
    // "1234567890123" then "4567890" -> bad() index 4567890, always SIGSEGV.
    assert_identical("split-fatal", b"12345678901234567890\n");
    // "9999999999999" then "9999999" -> bad() index 9999999, always SIGSEGV.
    assert_identical("split-fatal-nines", b"99999999999999999999\n");
}

// ---------------------------------------------------------------------------
// The genuinely non-deterministic region of the C
// ---------------------------------------------------------------------------
//
// For indices in roughly 2250..4250, and above ~33,000,000, the compiled C's
// outcome depends on the kernel's per-exec stack randomisation: the same input
// yields exit 0 on one run and SIGSEGV (or SIGBUS) on the next. No translation
// can be byte-identical to a program that disagrees with itself, so for these
// inputs the tests assert the strongest property the ground truth actually
// holds to, and the Rust matches C's majority outcome. See ERRORS.md.

/// Above ~33M the wild address can land in a file mapping, so the C dies from
/// SIGSEGV *or* SIGBUS depending on ASLR. Both programs must die from a fatal
/// signal, and both must produce no output at all.
#[test]
fn extreme_indices_always_die_silently() {
    for v in ["100000000", "536870912", "1073741824", "2147483647"] {
        let input = two_lines("0", v);
        let c = run(c_bin(), &input);
        let r = run(rust_bin(), &input);

        assert!(
            c.died_from_signal(),
            "index {v}: expected the C to die from a fatal signal, got {c:?}"
        );
        assert!(
            r.died_from_signal(),
            "index {v}: expected the Rust to die from a fatal signal, got {r:?}"
        );
        assert!(
            c.stdout.is_empty() && r.stdout.is_empty(),
            "index {v}: a fatal signal must discard the buffered stdout; C={:?} R={:?}",
            String::from_utf8_lossy(&c.stdout),
            String::from_utf8_lossy(&r.stdout),
        );
        assert_eq!(c.stderr, r.stderr, "index {v}: stderr differs");
        assert!(c.stderr.is_empty(), "index {v}: C stderr unexpectedly non-empty");
    }
}

/// In the ASLR-sensitive band the C is a coin flip. Assert the invariant that
/// does hold: whatever happens, the process either exits 0 having printed the
/// ten values, or dies from a signal having printed nothing -- and the Rust
/// produces one of those same two outcomes.
#[test]
fn aslr_band_outcomes_are_from_the_same_two_shapes() {
    let clean_exit_stdout = {
        // The exact bytes of a successful run, taken from an index known to be
        // invisible.
        run(c_bin(), &two_lines("0", "1000")).stdout
    };
    assert!(
        !clean_exit_stdout.is_empty(),
        "baseline run produced no output"
    );

    // Under the harness's padded environment the coin-flip band sits at roughly
    // 18500..21000.
    for v in ["18500", "19000", "19500", "20000", "20500", "21000"] {
        let input = two_lines("0", v);
        for prog in [c_bin(), rust_bin()] {
            let got = run(prog, &input);
            let ok_shape = if got.died_from_signal() {
                got.stdout.is_empty() && got.stderr.is_empty()
            } else {
                got.code == Some(0) && got.stdout == clean_exit_stdout && got.stderr.is_empty()
            };
            assert!(
                ok_shape,
                "index {v} from {}: unexpected outcome shape {got:?}",
                prog.display()
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Exact expected output, pinned so a regression in BOTH is still caught
// ---------------------------------------------------------------------------

/// Pin the full stdout for the canonical happy path, so that a change which
/// happens to break the C and the Rust identically is still visible.
#[test]
fn happy_path_exact_bytes() {
    let input = two_lines("3", "5");
    let c = run(c_bin(), &input);

    let mut expected = String::new();
    expected.push_str("Calling good()...\n");
    // goodG2B: data = 7
    for i in 0..10 {
        expected.push_str(if i == 7 { "1\n" } else { "0\n" });
    }
    // goodB2G: first line "3"
    for i in 0..10 {
        expected.push_str(if i == 3 { "1\n" } else { "0\n" });
    }
    expected.push_str("Finished good()\n");
    expected.push_str("Calling bad()...\n");
    // bad: second line "5"
    for i in 0..10 {
        expected.push_str(if i == 5 { "1\n" } else { "0\n" });
    }
    expected.push_str("Finished bad()\n");

    assert_eq!(
        String::from_utf8_lossy(&c.stdout),
        expected,
        "the C's own output drifted from the hand-computed expectation"
    );
    assert!(c.stderr.is_empty());
    assert_eq!(c.code, Some(0));

    assert_identical("happy-path", &input);
}

/// Pin the exact bytes of the all-error path (empty stdin).
#[test]
fn empty_input_exact_bytes() {
    let c = run(c_bin(), b"");
    let expected = concat!(
        "Calling good()...\n",
        // goodG2B always succeeds with data = 7
        "0\n0\n0\n0\n0\n0\n0\n1\n0\n0\n",
        // goodB2G: fgets fails, data stays -1 -> out-of-bounds branch
        "fgets() failed.\n",
        "ERROR: Array index is out-of-bounds\n",
        "Finished good()\n",
        "Calling bad()...\n",
        // bad: fgets fails, data stays -1 -> negative branch
        "fgets() failed.\n",
        "ERROR: Array index is negative.\n",
        "Finished bad()\n",
    );
    assert_eq!(String::from_utf8_lossy(&c.stdout), expected);
    assert!(c.stderr.is_empty());
    assert_eq!(c.code, Some(0));

    assert_identical("empty-exact", b"");
}

/// Command line arguments are ignored by the C (argc/argv unused), so passing
/// some must not change anything.
#[test]
fn arguments_are_ignored() {
    let input = two_lines("2", "4");
    let mut c = Command::new(c_bin());
    let mut r = Command::new(rust_bin());
    for cmd in [&mut c, &mut r] {
        cmd.args(["extra", "-1", "--flag"])
            .env(STACK_PAD_VAR, stack_pad())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
    }

    let run_with_args = |mut cmd: Command| -> Run {
        let mut child = cmd.spawn().expect("spawn");
        {
            let mut stdin = child.stdin.take().unwrap();
            let _ = stdin.write_all(&input);
        }
        let out = child.wait_with_output().expect("wait");
        Run {
            stdout: out.stdout,
            stderr: out.stderr,
            code: out.status.code(),
            signal: out.status.signal(),
        }
    };

    let cr = run_with_args(c);
    let rr = run_with_args(r);
    assert_eq!(cr.stdout, rr.stdout, "with args: stdout differs");
    assert_eq!(cr.stderr, rr.stderr, "with args: stderr differs");
    assert_eq!(
        (cr.code, cr.signal),
        (rr.code, rr.signal),
        "with args: exit status differs"
    );
}

/// A broad sweep over every small index, catching any single value where the
/// two implementations disagree.
#[test]
fn sweep_all_small_indices() {
    for v in -5..=120 {
        assert_identical("sweep", &two_lines("0", &v.to_string()));
    }
}

/// The same sweep, but varying the FIRST line (goodB2G's index) too.
#[test]
fn sweep_first_line_values() {
    for v in -3..=20 {
        assert_identical("sweep-first", &two_lines(&v.to_string(), "1"));
    }
}
