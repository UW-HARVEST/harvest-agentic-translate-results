//! Differential tests: run the original C binary and the Rust binary as
//! subprocesses with identical stdin and require byte-identical stdout,
//! byte-identical stderr and identical termination status (including the
//! terminating signal, because `div(x, 0)` makes the C program die of SIGFPE).

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// Path to the C reference executable, building it with CMake if necessary.
fn c_binary() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = repo_root().join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");
        if !exe.exists() {
            std::fs::create_dir_all(&build).expect("mkdir c_src/build");
            let cfg = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("run cmake (is cmake installed?)");
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
                .expect("run cmake --build");
            assert!(
                bld.status.success(),
                "cmake --build failed:\n{}\n{}",
                String::from_utf8_lossy(&bld.stdout),
                String::from_utf8_lossy(&bld.stderr)
            );
        }
        assert!(exe.exists(), "C binary not found at {}", exe.display());
        exe
    })
    .as_path()
}

fn rust_binary() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Ok(exit code)` for a normal exit, `Err(signal)` when killed by a signal.
    status: Result<i32, i32>,
}

fn run(exe: &Path, input: &[u8]) -> Run {
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));

    {
        let mut stdin = child.stdin.take().expect("piped stdin");
        let owned = input.to_vec();
        // Write on a helper thread: the program may exit (or die of SIGFPE)
        // before consuming all of stdin, which would otherwise deadlock or
        // raise EPIPE in this thread.
        std::thread::spawn(move || {
            let _ = stdin.write_all(&owned);
            let _ = stdin.flush();
        });
    }

    let out = child.wait_with_output().expect("wait for child");

    #[cfg(unix)]
    let status = {
        use std::os::unix::process::ExitStatusExt;
        match out.status.code() {
            Some(code) => Ok(code),
            None => Err(out.status.signal().expect("signal or code")),
        }
    };
    #[cfg(not(unix))]
    let status = Ok(out.status.code().unwrap_or(-1));

    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        status,
    }
}

fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).escape_debug().to_string()
}

/// The core assertion: stdout, stderr and exit status must all match.
fn assert_same(label: &str, input: &[u8]) {
    let c = run(c_binary(), input);
    let r = run(rust_binary(), input);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch for {label} (input {:?})\n  C   : \"{}\"\n  Rust: \"{}\"",
        show(input),
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch for {label} (input {:?})\n  C   : \"{}\"\n  Rust: \"{}\"",
        show(input),
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        c.status, r.status,
        "exit status mismatch for {label} (input {:?}): C {:?} vs Rust {:?}",
        show(input),
        c.status,
        r.status
    );
}

fn check_all(cases: &[(&str, &str)]) {
    for (label, input) in cases {
        assert_same(label, input.as_bytes());
    }
}

// ---------------------------------------------------------------------------
// Happy path: both conversions succeed.
// ---------------------------------------------------------------------------

#[test]
fn happy_path_sign_combinations() {
    check_all(&[
        ("pos/pos", "7 3"),
        ("neg/pos", "-7 3"),
        ("pos/neg", "7 -3"),
        ("neg/neg", "-7 -3"),
        ("explicit plus", "+5 +2"),
        ("exact division", "8 4"),
        ("numerator smaller", "3 7"),
        ("zero numerator", "0 5"),
        ("negative zero numerator", "-0 3"),
        ("leading zeros", "007 002"),
        ("many leading zeros", "0000000000000000000005 2"),
        ("trailing newline", "9 4\n"),
        ("trailing space", "9 4 "),
    ]);
}

// ---------------------------------------------------------------------------
// Whitespace handling: scanf's %d skips arbitrary whitespace, newlines
// included, so a value may be split across lines.
// ---------------------------------------------------------------------------

#[test]
fn whitespace_is_skipped_across_lines() {
    check_all(&[
        ("newline separated", "1\n2"),
        ("leading whitespace", "  12\n   \n -4"),
        ("tabs and newlines", "\n\t\n 8\t\n2"),
        ("crlf", "6\r\n3\r\n"),
        ("vertical tab / form feed", "\x0b\x0c6 3"),
        ("lots of spaces", "        6         3        "),
    ]);
}

#[test]
fn very_long_whitespace_and_digit_runs() {
    let mut ws = " ".repeat(9000);
    ws.push_str("7 3");
    assert_same("9000 spaces then 7 3", ws.as_bytes());

    let mut zeros = "0".repeat(9000);
    zeros.push_str("7 3");
    assert_same("9000 leading zeros", zeros.as_bytes());

    let mut nines = "9".repeat(9000);
    nines.push_str(" 3");
    assert_same("9000 nines (huge overflow)", nines.as_bytes());

    let mut nines = String::from("-");
    nines.push_str(&"9".repeat(5000));
    nines.push_str(" 3");
    assert_same("negative 5000 nines", nines.as_bytes());
}

// ---------------------------------------------------------------------------
// Missing input: x and/or y keep their initial value of 1 because scanf
// stores nothing for a conversion that never happens.
// ---------------------------------------------------------------------------

#[test]
fn empty_and_partial_input_keeps_initial_values() {
    check_all(&[
        ("completely empty", ""),
        ("single space", " "),
        ("only whitespace", "  "),
        ("only newline", "\n"),
        ("only whitespace mix", " \t\n\r\x0b\x0c"),
        ("single item", "5"),
        ("single item + space", "5 "),
        ("single item + newline", "5\n"),
        ("single negative item", "-9"),
    ]);
}

#[test]
fn matching_failure_paths() {
    check_all(&[
        ("no digits at all", "abc"),
        ("first token non numeric", "x 5"),
        ("second token non numeric", "5 x"),
        ("sign then space", "- 5"),
        ("sign then spaces", "-  5 2"),
        ("lone plus and space", "+ "),
        ("lone minus", "-"),
        ("lone plus", "+"),
        ("two signs", "--5 2"),
        ("sign after digits", "5 -"),
        ("comma separator", "5,3"),
        ("digits glued to letters", "12abc34"),
        ("hex-looking", "0x10 2"),
        ("float", "1.5 2.5"),
        ("exponent", "1e3 2"),
        ("dot first", ".5 2"),
    ]);
}

// ---------------------------------------------------------------------------
// Extra input past the two conversions is ignored.
// ---------------------------------------------------------------------------

#[test]
fn extra_trailing_input_is_ignored() {
    check_all(&[
        ("third number", "9 4 7"),
        ("third number and word", "9 4 7 extra"),
        ("many extra lines", "9 4\n7\n8\n9\n"),
    ]);
}

// ---------------------------------------------------------------------------
// Range: glibc converts %d through a `long`, saturating at LONG_MAX/LONG_MIN
// and then truncating to `int`.
// ---------------------------------------------------------------------------

#[test]
fn int_range_edges_and_truncation() {
    check_all(&[
        ("INT_MAX", "2147483647 1"),
        ("INT_MIN", "-2147483648 1"),
        ("INT_MAX + 1", "2147483648 3"),
        ("INT_MIN - 1", "-2147483649 3"),
        ("2^32", "4294967296 5"),
        ("2^32 + 1", "4294967297 5"),
        ("LONG_MAX", "9223372036854775807 3"),
        ("LONG_MIN", "-9223372036854775808 3"),
        ("LONG_MAX + 1", "9223372036854775808 3"),
        ("2^64 + 1", "18446744073709551617 3"),
        ("way past LONG_MAX", "99999999999999999999999 5"),
        ("way past LONG_MIN", "-99999999999999999999999 5"),
        ("huge denominator", "7 99999999999999999999999"),
        ("both huge", "99999999999999999999999 99999999999999999999999"),
        ("INT_MAX denominator", "5 2147483647"),
        ("INT_MIN denominator", "5 -2147483648"),
    ]);
}

// ---------------------------------------------------------------------------
// Fatal arithmetic: div by zero and INT_MIN / -1 raise SIGFPE in the C
// program (no stdout, no stderr, killed by signal 8).
// ---------------------------------------------------------------------------

#[test]
fn division_faults_match() {
    check_all(&[
        ("x / 0", "10 0"),
        ("0 / 0", "0 0"),
        ("negative / 0", "-10 0"),
        ("zero via truncation", "5 4294967296"),
        ("INT_MIN / -1", "-2147483648 -1"),
        ("INT_MIN / -1 via truncation", "-2147483648 4294967295"),
        ("overflowed value / 0", "99999999999999999999999 0"),
    ]);
}

#[test]
fn division_fault_is_a_signal_not_an_exit_code() {
    // Guard against a Rust panic (exit 101 / 134) being mistaken for a match:
    // the C program is killed by SIGFPE, so both must report a signal.
    let c = run(c_binary(), b"10 0");
    let r = run(rust_binary(), b"10 0");
    assert!(
        c.status.is_err(),
        "expected the C program to die of a signal, got {:?}",
        c.status
    );
    assert_eq!(c.status, r.status);
    assert!(c.stdout.is_empty() && r.stdout.is_empty());
}

// ---------------------------------------------------------------------------
// Odd bytes: NULs, non-UTF-8, no stdin at all.
// ---------------------------------------------------------------------------

#[test]
fn non_text_bytes() {
    assert_same("embedded NUL", b"7\x003");
    assert_same("NUL first", b"\x007 3");
    assert_same("high byte first", b"\xff7 3");
    assert_same("invalid utf8 between", b"7 \xc3\x283");
    assert_same("all NULs", b"\x00\x00\x00");
    assert_same("binary junk", b"\x01\x02\x03\x04");
}

#[test]
fn closed_stdin() {
    // /dev/null: immediate EOF, both conversions fail, x = y = 1.
    for exe in [c_binary(), rust_binary()] {
        let out = Command::new(exe)
            .stdin(Stdio::null())
            .output()
            .expect("spawn with /dev/null stdin");
        assert_eq!(out.stdout, b"quotient: 1, remainder: 0\n");
        assert!(out.stderr.is_empty());
        assert_eq!(out.status.code(), Some(0));
    }
}

// ---------------------------------------------------------------------------
// Deterministic fuzzing over the alphabet the parser branches on.
// ---------------------------------------------------------------------------

#[test]
fn deterministic_fuzz() {
    const ALPHABET: &[u8] = b" \t\n\r+-0123456789abxz.,";
    // xorshift64* so the corpus is reproducible.
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = move || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state = state.wrapping_mul(0x2545_F491_4F6C_DD1D);
        state
    };

    for i in 0..400 {
        let len = (next() % 24) as usize;
        let mut input = Vec::with_capacity(len);
        for _ in 0..len {
            input.push(ALPHABET[(next() % ALPHABET.len() as u64) as usize]);
        }
        assert_same(&format!("fuzz #{i}"), &input);
    }
}
