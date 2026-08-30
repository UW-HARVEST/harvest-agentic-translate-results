//! Differential tests: run the C program and the Rust program as subprocesses,
//! feed both the same bytes on stdin, and require byte-identical stdout,
//! byte-identical stderr and an identical exit status (including termination by
//! signal).
//!
//! The Rust binary is never linked as a library; it is driven exactly the way a
//! shell would drive it, because that is how the C program is being compared.

use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Path to the Rust binary under test. Cargo builds it for us.
const RUST_BIN: &str = env!("CARGO_BIN_EXE_driver");

fn repo_root() -> PathBuf {
    // tests/ lives in translation/, whose parent holds c_src/.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Builds `c_src` with CMake if the executable is not already present, and
/// returns the path to it.
fn c_bin() -> PathBuf {
    let c_src = repo_root().join("c_src");
    let build = c_src.join("build");
    let exe = build.join("driver");

    if exe.is_file() {
        return exe;
    }

    std::fs::create_dir_all(&build).expect("cannot create c_src/build");

    let configure = Command::new("cmake")
        .arg("..")
        .current_dir(&build)
        .output()
        .expect("failed to run `cmake` - is CMake installed?");
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

    assert!(exe.is_file(), "C build produced no executable at {exe:?}");
    exe
}

/// What one program run produced.
#[derive(PartialEq, Eq)]
struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Ok(code)` for a normal exit, `Err(signal)` when killed by a signal.
    status: Result<i32, i32>,
}

impl std::fmt::Debug for Run {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "status={}, stdout={} ({} bytes), stderr={}",
            match self.status {
                Ok(code) => format!("exit {code}"),
                Err(sig) => format!("signal {sig}"),
            },
            show(&self.stdout),
            self.stdout.len(),
            show(&self.stderr),
        )
    }
}

/// Renders bytes readably, collapsing long runs so failures stay legible.
fn show(bytes: &[u8]) -> String {
    let mut out = String::from("\"");
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        let run = bytes[i..].iter().take_while(|&&x| x == b).count();
        if run > 8 {
            out.push_str(&format!("{}*{}", escape(b), run));
            i += run;
        } else {
            out.push_str(&escape(b));
            i += 1;
        }
    }
    out.push('"');
    out
}

fn escape(b: u8) -> String {
    match b {
        b'\n' => "\\n".to_string(),
        b'\r' => "\\r".to_string(),
        b'\t' => "\\t".to_string(),
        0x20..=0x7e => (b as char).to_string(),
        _ => format!("\\x{b:02x}"),
    }
}

/// Runs `program`, writing `input` to its stdin, and captures everything.
fn run(program: &Path, input: &[u8]) -> Run {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {program:?}: {e}"));

    {
        let mut stdin = child.stdin.take().expect("stdin was piped");
        let input = input.to_vec();
        // Write on a helper thread: the program reads at most 13 bytes and then
        // exits, so a large input would otherwise deadlock on a full pipe.
        std::thread::spawn(move || {
            let _ = stdin.write_all(&input);
            let _ = stdin.flush();
        });
    }

    let out = child.wait_with_output().expect("failed to wait for child");
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        status: match out.status.code() {
            Some(code) => Ok(code),
            None => Err(out.status.signal().expect("no exit code and no signal")),
        },
    }
}

/// Runs a program with stdin redirected from `/dev/null` (immediate EOF) rather
/// than from a pipe.
fn run_with_stdin_at_eof(program: &Path) -> Run {
    let devnull = std::fs::File::open("/dev/null").expect("cannot open /dev/null");
    let out = Command::new(program)
        .stdin(Stdio::from(devnull))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn {program:?}: {e}"));
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        status: match out.status.code() {
            Some(code) => Ok(code),
            None => Err(out.status.signal().expect("no exit code and no signal")),
        },
    }
}

/// The core assertion: stdout, stderr and exit status must all match.
#[track_caller]
fn assert_same(desc: &str, input: &[u8]) {
    let c = run(&c_bin(), input);
    let r = run(Path::new(RUST_BIN), input);
    assert_eq!(
        c, r,
        "\ncase       : {desc}\ninput      : {}\nC   produced: {c:?}\nRust produced: {r:?}\n",
        show(input)
    );
}

// ---------------------------------------------------------------------------
// Phase A - both binaries exist and are runnable.
// ---------------------------------------------------------------------------

#[test]
fn both_programs_build_and_run() {
    let c = c_bin();
    assert!(c.is_file(), "C binary missing at {c:?}");
    assert!(Path::new(RUST_BIN).is_file(), "Rust binary missing");
    // A trivial input must drive both to completion.
    assert_same("smoke: \"0\\n\"", b"0\n");
}

// ---------------------------------------------------------------------------
// Phase B - the branches the C source actually takes.
//
// main() has exactly three decisions:
//   1. `fgets(inputBuffer, 14, stdin) != NULL`  -> success vs "fgets() failed."
//   2. `data < 100`                             -> strncpy+terminate vs skip
//   3. `printLine`'s `line != NULL`             -> always true from main()
// `data` comes from atoi() over at most 13 bytes, and is passed to strncpy as a
// size_t, so a negative `data` becomes a huge count and the copy runs off the
// 100 byte stack buffer.
// ---------------------------------------------------------------------------

#[test]
fn fgets_returns_null_on_empty_stdin() {
    // fgets fails, data stays -1, so strncpy gets (size_t)(-1).
    // The "fgets() failed." message sits in the fully buffered stdout and is
    // discarded when the process dies, so stdout must be empty.
    assert_same("empty stdin (EOF immediately)", b"");
}

#[test]
fn fgets_returns_null_when_stdin_is_dev_null() {
    let c = run_with_stdin_at_eof(&c_bin());
    let r = run_with_stdin_at_eof(Path::new(RUST_BIN));
    assert_eq!(c, r, "\nstdin=/dev/null\nC: {c:?}\nRust: {r:?}\n");
}

#[test]
fn single_item_smallest_positive() {
    assert_same("data = 1", b"1\n");
}

#[test]
fn zero_copies_nothing() {
    // strncpy(dest, source, 0) leaves dest untouched, dest[0] = '\0'.
    assert_same("data = 0", b"0\n");
}

#[test]
fn newline_only_parses_as_zero() {
    assert_same("newline only", b"\n");
}

#[test]
fn typical_middle_value() {
    assert_same("data = 5", b"5\n");
}

#[test]
fn maximum_the_code_handles() {
    // 99 is the largest value that both satisfies `data < 100` and stays inside
    // dest[100]; source holds exactly 99 'A's before its NUL.
    assert_same("data = 99 (maximum in range)", b"99\n");
}

#[test]
fn boundary_98_99_100_101() {
    for input in [&b"98\n"[..], b"99\n", b"100\n", b"101\n"] {
        assert_same("boundary around data < 100", input);
    }
}

#[test]
fn value_100_skips_the_copy_entirely() {
    // `data < 100` is false, so dest keeps its zero initializer and printLine
    // emits just a newline.
    assert_same("data = 100 (branch not taken)", b"100\n");
}

#[test]
fn large_value_skips_the_copy() {
    assert_same("data = 1000", b"1000\n");
}

#[test]
fn negative_one_overflows_strncpy() {
    // (size_t)(-1) - the out-of-bounds write kills the process.
    assert_same("data = -1", b"-1\n");
}

#[test]
fn negative_values_all_crash_identically() {
    for input in [&b"-1\n"[..], b"-2\n", b"-99\n", b"-100\n", b"-12345\n"] {
        assert_same("negative data", input);
    }
}

#[test]
fn int_min_overflows_strncpy() {
    assert_same("data = INT_MIN", b"-2147483648\n");
}

#[test]
fn int_max_skips_the_copy() {
    assert_same("data = INT_MAX", b"2147483647\n");
}

#[test]
fn every_value_from_zero_to_one_hundred_twenty() {
    // Walks the whole `data < 100` boundary plus the copy length arithmetic.
    for n in 0..=120 {
        assert_same("exhaustive small values", format!("{n}\n").as_bytes());
    }
}

// ---------------------------------------------------------------------------
// Phase C - paths not covered above: atoi parsing, fgets buffer limits and
// binary input.
// ---------------------------------------------------------------------------

#[test]
fn non_numeric_input_parses_as_zero() {
    for input in [&b"abc\n"[..], b"hello world\n", b"!!!\n", b"-\n", b"+\n"] {
        assert_same("atoi finds no digits", input);
    }
}

#[test]
fn digits_after_junk_are_ignored() {
    assert_same("\"abc50\" -> 0", b"abc50\n");
}

#[test]
fn junk_after_digits_is_ignored() {
    assert_same("\"50abc\" -> 50", b"50abc\n");
    assert_same("\"0x1F\" -> 0 (atoi is base 10)", b"0x1F\n");
    assert_same("\"0.9\" -> 0", b"0.9\n");
    assert_same("\"7e2\" -> 7", b"7e2\n");
}

#[test]
fn leading_whitespace_is_skipped_by_atoi() {
    assert_same("spaces then digits", b"   42\n");
    assert_same("tab then signed digits", b"\t+7\n");
    assert_same("whitespace only", b"   \n");
    assert_same("form feed and vtab", b"\x0b\x0c9\n");
    assert_same("spaces then negative", b"   -5\n");
}

#[test]
fn explicit_signs_are_accepted() {
    assert_same("+50", b"+50\n");
    assert_same("+0", b"+0\n");
    assert_same("-0", b"-0\n");
}

#[test]
fn leading_zeros_are_decimal_not_octal() {
    assert_same("0000000000005 (13 chars wide)", b"0000000000005\n");
    assert_same("010", b"010\n");
}

#[test]
fn double_sign_stops_conversion() {
    assert_same("--5", b"--5\n");
    assert_same("+-5", b"+-5\n");
}

#[test]
fn internal_space_stops_conversion() {
    assert_same("\"5 0\" -> 5", b"5 0\n");
    assert_same("\"1 -2\" -> 1", b"1 -2\n");
}

#[test]
fn fgets_reads_at_most_thirteen_payload_bytes() {
    // inputBuffer is char[14], so fgets stores 13 bytes plus the NUL. Anything
    // beyond the 13th byte of the first line is never seen.
    assert_same("exactly 13 digits, no newline", b"1234567890123");
    assert_same("exactly 13 digits then newline", b"1234567890123\n");
    // 15 nines truncate to 13 nines before atoi runs.
    assert_same("15 nines truncated by fgets", b"999999999999999\n");
    // 14th byte would flip the value; it must be dropped.
    assert_same("\"00000000000009\" truncated to 0000000000000", b"00000000000009\n");
}

#[test]
fn fgets_stops_at_the_first_newline() {
    // Unlike scanf, fgets does not read across newlines, so the second line is
    // never consumed and cannot affect the result.
    assert_same("two lines: 7 then 50", b"7\n50\n");
    assert_same("first line empty, second line 99", b"\n99\n");
    assert_same("first line negative, second line 5", b"-1\n5\n");
}

#[test]
fn no_trailing_newline_still_converts() {
    assert_same("\"3\" without newline", b"3");
    assert_same("\"-4\" without newline", b"-4");
    assert_same("\"100\" without newline", b"100");
}

#[test]
fn carriage_return_terminates_the_number() {
    assert_same("CRLF line ending", b"9\r\n");
    assert_same("lone CR", b"9\r");
}

#[test]
fn int_truncation_from_long_matches() {
    // atoi is (int)strtol(...), so values beyond INT_MAX wrap. 13 digits fit in
    // a long, so these exercise the narrowing cast, not strtol saturation.
    assert_same("2^32 -> 0", b"4294967296\n");
    assert_same("2^32 + 100 -> 100", b"4294967396\n");
    assert_same("2^32 + 5 -> 5", b"4294967301\n");
    assert_same("2^31 -> INT_MIN (negative)", b"2147483648\n");
    assert_same("13 nines", b"9999999999999\n");
    assert_same("negative 2^32 + 5", b"-4294967291\n");
}

#[test]
fn embedded_nul_bytes_terminate_the_string() {
    assert_same("NUL first", b"\x0050\n");
    assert_same("NUL after digits", b"5\x009\n");
}

#[test]
fn high_bytes_and_binary_input() {
    assert_same("all high bytes", b"\xff\xfe\xfd\n");
    assert_same("utf8 text", "héllo\n".as_bytes());
    assert_same("digits then high byte", b"12\xff\n");
}

#[test]
fn input_much_larger_than_the_buffer() {
    // The program reads 13 bytes and exits; the rest of stdin is discarded.
    let mut big = vec![b'7'];
    big.extend(std::iter::repeat(b'0').take(100_000));
    big.push(b'\n');
    assert_same("100k byte first line", &big);

    let mut lines = Vec::new();
    for _ in 0..10_000 {
        lines.extend_from_slice(b"42\n");
    }
    assert_same("10k lines", &lines);
}

#[test]
fn random_inputs_agree() {
    // Deterministic xorshift so failures reproduce.
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    for _ in 0..120 {
        let len = (next() % 24) as usize;
        let input: Vec<u8> = (0..len)
            .map(|_| match next() % 10 {
                // Bias towards characters atoi cares about.
                0..=5 => b'0' + (next() % 10) as u8,
                6 => b'-',
                7 => b' ',
                8 => b'\n',
                _ => (next() % 256) as u8,
            })
            .collect();
        assert_same("random input", &input);
    }
}

// ---------------------------------------------------------------------------
// Phase C - stdout buffering mode.
//
// glibc line buffers stdout on a terminal and fully buffers it otherwise. On
// the fgets-failure path the process dies right after printf, so the message
// survives only when stdout is a terminal. This drives both programs through a
// pty to confirm the Rust translation makes the same choice.
// ---------------------------------------------------------------------------

#[test]
fn terminal_stdout_is_line_buffered() {
    fn via_pty(program: &Path) -> Option<(Vec<u8>, Vec<u8>)> {
        let out = Command::new("script")
            .args(["-q", "-c"])
            .arg(program.to_str().expect("path is valid UTF-8"))
            .arg("/dev/null")
            .stdin(Stdio::from(
                std::fs::File::open("/dev/null").expect("cannot open /dev/null"),
            ))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .ok()?;
        Some((out.stdout, out.stderr))
    }

    let (Some(c), Some(r)) = (via_pty(&c_bin()), via_pty(Path::new(RUST_BIN))) else {
        // `script` is not available on this host; the pipe-based tests above
        // already cover the fully buffered case.
        return;
    };
    assert_eq!(
        show(&c.0),
        show(&r.0),
        "stdout differs when stdout is a terminal"
    );
    assert_eq!(
        show(&c.1),
        show(&r.1),
        "stderr differs when stdout is a terminal"
    );
}
