//! Differential tests: run the C binary and the Rust binary as subprocesses on
//! the same stdin and require byte-identical stdout, byte-identical stderr and
//! an identical exit status.
//!
//! The Rust code is never called as a library. Both programs are driven exactly
//! the way a shell drives them, because that is how they are compared.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The Rust binary under test, as built by cargo for this test run.
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn workspace_root() -> PathBuf {
    // tests/ live in translation/, whose parent holds c_src/.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// The C binary, built via CMake on first use so `cargo test` works standalone.
fn c_bin() -> PathBuf {
    let c_src = workspace_root().join("c_src");
    let build = c_src.join("build");
    let exe = build.join("driver");
    if exe.is_file() {
        return exe;
    }

    std::fs::create_dir_all(&build).expect("cannot create c_src/build");
    run_build(Command::new("cmake").arg("..").current_dir(&build), "cmake");
    run_build(
        Command::new("cmake")
            .args(["--build", "."])
            .current_dir(&build),
        "cmake --build",
    );
    assert!(
        exe.is_file(),
        "C driver was not produced at {}",
        exe.display()
    );
    exe
}

fn run_build(cmd: &mut Command, what: &str) {
    let out = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn {what}: {e}"));
    assert!(
        out.status.success(),
        "{what} failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Everything observable about one run.
#[derive(PartialEq, Eq)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Ok(code)` for a normal exit, `Err(signal)` when killed by a signal.
    status: Result<i32, i32>,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "status={:?} stdout={} stderr={}",
            self.status,
            escape(&self.stdout),
            escape(&self.stderr)
        )
    }
}

fn escape(bytes: &[u8]) -> String {
    let mut s = String::from("\"");
    for &b in bytes {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            b'\\' => s.push_str("\\\\"),
            b'"' => s.push_str("\\\""),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    s.push('"');
    s
}

fn status_of(status: std::process::ExitStatus) -> Result<i32, i32> {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(sig) = status.signal() {
            return Err(sig);
        }
    }
    Ok(status.code().unwrap_or(-1))
}

/// Feed `input` to `prog` on stdin and capture everything it produces.
fn run(prog: &Path, input: &[u8]) -> Outcome {
    let mut child = Command::new(prog)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", prog.display()));

    {
        let mut stdin = child.stdin.take().expect("stdin was piped");
        // The peer may exit without draining stdin; a write error is not a test
        // failure, so it is deliberately ignored here.
        let _ = stdin.write_all(input);
        let _ = stdin.flush();
    }

    let out = child.wait_with_output().expect("failed to wait for child");
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        status: status_of(out.status),
    }
}

/// The core assertion: identical stdout, stderr and exit status.
#[track_caller]
fn assert_same(label: &str, input: &[u8]) {
    let c = run(&c_bin(), input);
    let r = run(&rust_bin(), input);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout differs for {label} (input {})\n  C: {}\n  Rust: {}",
        escape(input),
        escape(&c.stdout),
        escape(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr differs for {label} (input {})\n  C: {}\n  Rust: {}",
        escape(input),
        escape(&c.stderr),
        escape(&r.stderr)
    );
    assert_eq!(
        c.status,
        r.status,
        "exit status differs for {label} (input {})\n  C: {c:?}\n  Rust: {r:?}",
        escape(input)
    );
}

#[track_caller]
fn assert_all_same(cases: &[(&str, &[u8])]) {
    for (label, input) in cases {
        assert_same(label, input);
    }
}

// ---------------------------------------------------------------------------
// The two branches of `main`
//
//   scanf("%d", &x);  if (x) good(); else bad();
//
// `good()` prints "string\n". `bad()` reads an uninitialized `char *` and hands
// it to printLine; in the reference build the resulting output is a bare "\n".
// Both are asserted against the C binary rather than against a hardcoded
// expectation, so whatever the C does is what is required.
// ---------------------------------------------------------------------------

#[test]
fn empty_input() {
    // scanf hits EOF immediately, converts nothing, and leaves x == 0 -> bad().
    assert_same("empty input", b"");
}

#[test]
fn single_item_zero_takes_bad_branch() {
    assert_all_same(&[
        ("bare zero", b"0"),
        ("zero with newline", b"0\n"),
        ("negative zero", b"-0"),
        ("plus zero", b"+0"),
        ("padded zero", b"0000000"),
        ("zero then junk", b"0abc"),
        ("leading space then zero", b"   0"),
    ]);
}

#[test]
fn single_item_nonzero_takes_good_branch() {
    assert_all_same(&[
        ("one", b"1"),
        ("one with newline", b"1\n"),
        ("negative one", b"-1"),
        ("explicit plus", b"+5"),
        ("large positive", b"123456"),
        ("large negative", b"-123456"),
        ("nonzero then junk", b"1abc"),
        ("leading zeros then nonzero", b"0009"),
    ]);
}

// ---------------------------------------------------------------------------
// scanf matching failures: no conversion happens, so x keeps its initial 0 and
// control reaches bad(). Each of these is a distinct early-out inside the
// conversion.
// ---------------------------------------------------------------------------

#[test]
fn scanf_matching_failures_leave_x_at_zero() {
    assert_all_same(&[
        ("letters only", b"abc"),
        ("single letter", b"z"),
        ("sign with no digits", b"-"),
        ("plus with no digits", b"+"),
        ("double sign", b"--1"),
        ("sign then letter", b"-a"),
        ("dot first", b".5"),
        ("hex prefix is not accepted by %d", b"0x10"),
        ("punctuation", b"!!!"),
        ("newline only", b"\n"),
        ("space only", b" "),
        ("all C whitespace, then EOF", b"\t\x0b\x0c\r \n"),
        ("non-ascii bytes", b"\xff\xfe"),
        ("nul byte first", b"\x00 1"),
    ]);
}

// ---------------------------------------------------------------------------
// scanf's %d skips whitespace across newlines (unlike fgets, which would stop
// at the first one). These inputs distinguish the two readers.
// ---------------------------------------------------------------------------

#[test]
fn percent_d_scans_across_newlines() {
    assert_all_same(&[
        ("blank lines before the number", b"\n\n\n\n5"),
        ("mixed leading whitespace", b"  \n\n 7"),
        ("tabs and newlines then negative", b"  \t\n  -42\nrest"),
        ("newline before zero", b"\n0"),
        ("carriage returns then nonzero", b"\r\r\r3"),
    ]);
}

#[test]
fn only_the_first_conversion_is_read() {
    // A single %d consumes one integer; trailing tokens never affect the branch.
    assert_all_same(&[
        ("zero then one", b"0 1"),
        ("leading space, zero, one", b" 0 1"),
        ("one then zero", b"1 0"),
        ("zero then many tokens", b"0 1 2 3 4 5\n6 7\n"),
        ("nonzero then many tokens", b"9 0 0 0\n"),
    ]);
}

// ---------------------------------------------------------------------------
// Range behavior. glibc converts %d with strtol (saturating at LONG_MIN/MAX on
// a 64-bit long) and then stores into an int, truncating to the low 32 bits.
// That makes several out-of-range inputs land on x == 0 and take bad(), notably
// -99999999999999999999, whose saturated value has zero low bits.
// ---------------------------------------------------------------------------

#[test]
fn int_boundaries() {
    assert_all_same(&[
        ("INT_MAX", b"2147483647"),
        ("INT_MAX + 1", b"2147483648"),
        ("INT_MIN", b"-2147483648"),
        ("INT_MIN - 1", b"-2147483649"),
        ("UINT_MAX", b"4294967295"),
        ("2^32 truncates to 0", b"4294967296"),
        ("2^32 + 1 truncates to 1", b"4294967297"),
        ("negative 2^32", b"-4294967296"),
    ]);
}

#[test]
fn long_boundaries_and_saturation() {
    assert_all_same(&[
        ("LONG_MAX", b"9223372036854775807"),
        ("LONG_MAX + 1 saturates", b"9223372036854775808"),
        ("LONG_MIN", b"-9223372036854775808"),
        ("LONG_MIN - 1 saturates", b"-9223372036854775809"),
        ("ULONG_MAX", b"18446744073709551615"),
        ("ULONG_MAX + 1", b"18446744073709551616"),
        ("far above range", b"99999999999999999999"),
        ("far below range", b"-99999999999999999999"),
        ("absurdly long positive", b"999999999999999999999999999999990"),
        ("absurdly long negative", b"-999999999999999999999999999999990"),
        (
            "many leading zeros then in-range value",
            b"00000000000000000000000000000001",
        ),
        ("many leading zeros then zero", b"00000000000000000000000000000000"),
    ]);
}

#[test]
fn powers_of_ten_truncate_the_same_way() {
    // Walks the accumulate-then-truncate path across the 32- and 64-bit edges.
    for k in 1..40usize {
        let pos = format!("1{}", "0".repeat(k));
        assert_same(&format!("10^{k}"), pos.as_bytes());
        let neg = format!("-1{}", "0".repeat(k));
        assert_same(&format!("-10^{k}"), neg.as_bytes());
        let nines = "9".repeat(k);
        assert_same(&format!("{k} nines"), nines.as_bytes());
        let neg_nines = format!("-{}", "9".repeat(k));
        assert_same(&format!("negative {k} nines"), neg_nines.as_bytes());
    }
}

// ---------------------------------------------------------------------------
// Environment-level behavior that is still observable in the exit status.
// ---------------------------------------------------------------------------

#[test]
fn empty_stdin_from_a_closed_stream() {
    // /dev/null-style: readable but immediately at EOF.
    assert_same("immediate EOF", b"");
}

#[test]
fn large_input_before_the_number() {
    let mut input = vec![b' '; 100_000];
    input.extend_from_slice(b"\n\n42\n");
    assert_same("100k spaces then a number", &input);

    let mut junk = vec![b'0'; 50_000];
    junk.push(b'1');
    assert_same("50k leading zeros then 1", &junk);
}

#[test]
fn sigpipe_kills_both_programs_alike() {
    // A C program inherits SIGPIPE = SIG_DFL, so writing to a closed pipe is
    // fatal (status 128 + 13). Rust installs SIG_IGN before main, which would
    // instead yield a quiet exit 0 unless the translation restores the default.
    //
    // Both programs block in their first read until stdin is written, so
    // dropping the stdout reader first makes the failing write deterministic.
    for (name, prog) in [("C", c_bin()), ("Rust", rust_bin())] {
        let outcome = run_with_closed_stdout(&prog, b"1\n");
        assert_eq!(
            outcome,
            Err(13),
            "{name} binary should die from SIGPIPE, got {outcome:?}"
        );
    }

    // And the same on the bad() branch, which writes a single newline.
    let c = run_with_closed_stdout(&c_bin(), b"0\n");
    let r = run_with_closed_stdout(&rust_bin(), b"0\n");
    assert_eq!(c, r, "bad() branch differs under a closed stdout");
}

fn run_with_closed_stdout(prog: &Path, input: &[u8]) -> Result<i32, i32> {
    let mut child = Command::new(prog)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", prog.display()));

    // Close the read end before the child has any reason to write.
    drop(child.stdout.take());

    {
        let mut stdin = child.stdin.take().expect("stdin was piped");
        let _ = stdin.write_all(input);
        let _ = stdin.flush();
    }

    status_of(child.wait().expect("failed to wait for child"))
}

// ---------------------------------------------------------------------------
// A broad sweep, so no input class depends on having been thought of by name.
// ---------------------------------------------------------------------------

#[test]
fn exhaustive_short_inputs_over_the_interesting_alphabet() {
    // Every string of length <= 2 over the bytes %d actually branches on:
    // digits, signs, whitespace (including the ones fgets would not cross),
    // and non-numeric bytes.
    let alphabet: &[u8] = b"0123456789+- \t\n\r\x0b\x0ca.\x00\xff";

    assert_same("len 0", b"");
    for &a in alphabet {
        assert_same("len 1", &[a]);
    }
    for &a in alphabet {
        for &b in alphabet {
            assert_same("len 2", &[a, b]);
        }
    }
}

#[test]
fn pseudorandom_inputs() {
    // Deterministic xorshift, so a failure is always reproducible.
    let alphabet: &[u8] = b"0123456789+- \t\n\r\x0b\x0cabcxz.\x00\xff";
    let mut state: u64 = 0x2545_f491_4f6c_dd1d;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    for _ in 0..400 {
        let len = (next() % 13) as usize;
        let input: Vec<u8> = (0..len)
            .map(|_| alphabet[(next() % alphabet.len() as u64) as usize])
            .collect();
        assert_same("pseudorandom", &input);
    }
}
