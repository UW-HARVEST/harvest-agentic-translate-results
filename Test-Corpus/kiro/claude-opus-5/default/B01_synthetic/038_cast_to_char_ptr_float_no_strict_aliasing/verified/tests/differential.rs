//! Differential tests: run the C program and the Rust program as subprocesses,
//! feed both the same bytes on stdin, and require byte-identical stdout,
//! byte-identical stderr and an identical exit status.
//!
//! The Rust code is never linked as a library here; only the built binary is
//! driven, exactly the way a shell would drive it.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Locating / building the two executables
// ---------------------------------------------------------------------------

/// `translation/` -> the working directory that holds `c_src/` and `translation/`.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the Rust binary under test (built by cargo for integration tests).
fn rust_bin() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// Path to the C binary, building it with CMake on first use.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = workspace_root().join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");
        if !exe.exists() {
            std::fs::create_dir_all(&build).expect("create c_src/build");
            let cfg = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("failed to run `cmake ..` (is cmake installed?)");
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
                .expect("failed to run `cmake --build .`");
            assert!(
                bld.status.success(),
                "cmake build failed:\n{}\n{}",
                String::from_utf8_lossy(&bld.stdout),
                String::from_utf8_lossy(&bld.stderr)
            );
        }
        assert!(exe.exists(), "C executable not found at {}", exe.display());
        exe
    })
}

// ---------------------------------------------------------------------------
// Running one program
// ---------------------------------------------------------------------------

struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Ok(code)` for a normal exit, `Err(signal)` when killed by a signal.
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
        // The child may exit before consuming everything (a broken pipe is not
        // a test failure), so write errors are tolerated here.
        let _ = stdin.write_all(input);
        let _ = stdin.flush();
    }

    let out = child.wait_with_output().expect("wait_with_output");

    #[cfg(unix)]
    let status = {
        use std::os::unix::process::ExitStatusExt;
        match out.status.code() {
            Some(c) => Ok(c),
            None => Err(out.status.signal().unwrap_or(-1)),
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

fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).escape_debug().to_string()
}

/// Asserts stdout, stderr and exit status all agree for one input.
#[track_caller]
fn assert_same(input: &[u8]) {
    let c = run(c_bin(), input);
    let r = run(rust_bin(), input);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout differs for input {:?}\n  C   : {}\n  Rust: {}",
        show(input),
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr differs for input {:?}\n  C   : {}\n  Rust: {}",
        show(input),
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        c.status,
        r.status,
        "exit status differs for input {:?}: C {:?} vs Rust {:?}",
        show(input),
        c.status,
        r.status
    );
}

fn assert_all(cases: &[&str]) {
    for case in cases {
        assert_same(case.as_bytes());
    }
}

// ---------------------------------------------------------------------------
// Phase A — both programs build and run
// ---------------------------------------------------------------------------

#[test]
fn both_programs_run() {
    let c = run(c_bin(), b"1.5");
    let r = run(rust_bin(), b"1.5");
    assert_eq!(c.status, Ok(0), "C program should exit 0");
    assert_eq!(r.status, Ok(0), "Rust program should exit 0");
    assert_eq!(c.stdout, b"0000c03f\n".to_vec(), "C reference output for 1.5");
    assert_eq!(c.stdout, r.stdout);
}

// ---------------------------------------------------------------------------
// Phase B — the input classes the C branches on
// ---------------------------------------------------------------------------

/// `scanf` gets EOF before any conversion: `x` keeps its initialiser, `0.f`.
#[test]
fn empty_and_whitespace_only_input() {
    assert_all(&[
        "",
        "\n",
        " ",
        "   ",
        "\t",
        "\r",
        "\n\n\n",
        " \t\r\n\x0b\x0c",
    ]);
}

/// A single well formed value, the happy path through `driver`/`print_hex`.
#[test]
fn single_plain_values() {
    assert_all(&[
        "0", "1", "2", "1.5", "-1.5", "+1.5", "0.1", "0.2", "2.5e-1", "2.5E-1", "1E5", "1e0",
        "3.14159265358979", "123456789", "-123456789",
    ]);
}

/// Signed zero is observable in the printed bit pattern.
#[test]
fn signed_zeros() {
    assert_all(&[
        "0", "-0", "+0", "0.0", "-0.0", "+0.0", "-0.", "-.0", "-0e0", "-0e", "-0e+", "-0x0",
        "-0x0p0", "-0x0p", "-00x", "000", "-000.000",
    ]);
}

/// Leading white space is skipped by `%f`, including newlines: `scanf` reads
/// across line boundaries where `fgets` would stop.
#[test]
fn leading_whitespace_is_skipped_across_newlines() {
    assert_all(&[
        " 1.5",
        "  \t1.5",
        "\n1.5",
        "\n\n\n1.5",
        "\r\n2.5",
        "\t\n  -3.5",
        "\x0b\x0c1",
        "\n\n\n\n\n\n\n\n\n\n42\n",
    ]);
}

/// Matching failure: the conversion consumes nothing, so `x` stays `0.f`.
#[test]
fn matching_failures_leave_x_at_zero() {
    assert_all(&[
        "abc", "z", ".", "-.", "+.", "-", "+", "--1", "+-1", "e5", "E5", "e", "p", "x", "/",
        "'", "-abc", " . ", "..", ".-5", "$1.5",
    ]);
}

/// Only the first token is converted; the rest of the stream is ignored.
#[test]
fn trailing_junk_is_ignored() {
    assert_all(&[
        "12abc", "1.5.5", "1 2", "1\n2", "1.5 rest of line\nmore\n", "0..5", "1,5", "5.-",
        "7 8 9",
    ]);
}

/// `.5` / `5.` are valid; digits on only one side of the point suffice.
#[test]
fn digits_on_one_side_of_the_point() {
    assert_all(&[".5", "5.", "-.5", "-5.", "+.5", ".0", "0.", ".000", "000."]);
}

/// An exponent marker with no digits after it is not part of the number.
#[test]
fn incomplete_decimal_exponent() {
    assert_all(&[
        "1e", "1e+", "1e-", "1e+x", "1ex", "1E", "0.5e", "0.5e+", "1.5e-", ".5e", "5.e+",
    ]);
}

/// `inf` / `infinity`, every case, plus the partial spellings that fail.
#[test]
fn infinity_spellings() {
    assert_all(&[
        "inf",
        "INF",
        "Inf",
        "iNf",
        "-inf",
        "+inf",
        "infinity",
        "INFINITY",
        "Infinity",
        "InFiNiTy",
        "-infinity",
        "+INFINITY",
        "i",
        "in",
        "infi",
        "infin",
        "infini",
        "infinit",
        "-infinit",
        "inf1",
        "infz",
        "infinityx",
        "inf inity",
    ]);
}

/// `nan`, its sign, and the fact that `%f` does not take a `(payload)`.
#[test]
fn nan_spellings() {
    assert_all(&[
        "nan",
        "NAN",
        "NaN",
        "-nan",
        "+nan",
        "nan(123)",
        "nan(0x7)",
        "-nan(abc)",
        "nan(",
        "nan()",
        "n",
        "na",
        "nax",
        "-na",
    ]);
}

/// Hexadecimal floats, including the prefix-only forms.
#[test]
fn hexadecimal_floats() {
    assert_all(&[
        "0x0", "0x1", "0X1", "0x1p3", "0x1P3", "0X1p-3", "0x.8p1", "0x1.8", "0x1.8p0",
        "0xabcdef", "0xABCDEF", "0x10p-4", "0x123456789abcdef1p0", "-0x1.8p2", "+0x2p1",
    ]);
}

/// A `0x` prefix with no hex digits: `0x` fails, but `0x.` still converts the
/// leading `0` (and keeps the sign).
#[test]
fn hex_prefix_without_digits() {
    assert_all(&[
        "0x", "0X", "-0x", "-0X", "0xz", "-0xz", "0xg", "0x.", "-0x.", "0x.z", "-0x.z",
        "0x.p1", "-0x.p1", "0xp1", "-0xp1", "0x..", "-0x.8",
    ]);
}

/// A `p` marker with no digits after it is not part of the number.
#[test]
fn incomplete_hex_exponent() {
    assert_all(&["0x1p", "0x1p+", "0x1p-", "0x1p+x", "0x1px", "0x1.8p", "-0x1p", "0x0p"]);
}

/// Overflow to infinity and underflow to zero, decimal and hexadecimal.
#[test]
fn overflow_and_underflow() {
    assert_all(&[
        "1e38",
        "1e39",
        "1e999",
        "-1e999",
        "1e-999",
        "-1e-999",
        "340282346638528859811704183484516925440",
        "340282356779733661637539395458142568448",
        "3.4028235e38",
        "3.4028236e38",
        "3.402824e38",
        "0x1p127",
        "0x1p128",
        "0x1p-149",
        "0x1p-150",
        "0x1.8p-149",
        "1e999999999999999999999",
        "-1e999999999999999999999",
        "1e-999999999999999999999",
        "0x1p99999999999999999999",
        "0x1p-99999999999999999999",
        "5e-324",
    ]);
}

/// The exponent accumulator saturates; check either side of the clamp.
#[test]
fn exponent_accumulator_clamp() {
    assert_all(&[
        "1e999999999",
        "1e1000000000",
        "1e1000000001",
        "1e-999999999",
        "1e-1000000000",
        "1e-1000000001",
        "1e00000000000000000000005",
        "1e000000000000000000000000",
        "0x1p1000000000",
        "0x1p-1000000000",
    ]);
}

/// f32 has 24 significand bits: these inputs exercise truncation and
/// round-to-nearest-even at the boundary.
#[test]
fn float_rounding_and_truncation() {
    assert_all(&[
        "16777215", "16777216", "16777217", "16777218", "16777219", "16777220", "2147483647",
        "2147483648", "4294967295", "4294967296", "1.00000001", "1.000000059604644775390625",
        "1.0000000596046447753906250000001", "0.99999999", "8388609.5", "8388608.5",
        "1e-45", "1e-46", "7e-46", "1.4e-45", "1.5e-45", "0.7e-45", "0.75e-45",
    ]);
}

/// Subnormals, including exact halfway points between neighbouring subnormals.
#[test]
fn subnormal_boundaries() {
    let mut cases: Vec<String> = Vec::new();
    for k in 1..40u32 {
        // k * 2^-150  ==  half-integer multiples of the smallest subnormal.
        cases.push(format!("{:.60e}", (k as f64) * 2f64.powi(-150)));
        cases.push(format!("{:.60e}", (k as f64 + 0.25) * 2f64.powi(-150)));
        cases.push(format!("0x{:x}p-150", k));
    }
    for c in &cases {
        assert_same(c.as_bytes());
    }
}

/// Exact midpoints between consecutive f32 values, and a hair either side of
/// them, across normals, subnormals and the top of the range.
#[test]
fn ties_to_even_across_the_range() {
    let seeds: [u32; 16] = [
        0x0000_0000,
        0x0000_0001,
        0x0000_0002,
        0x007f_ffff,
        0x0080_0000,
        0x0080_0001,
        0x0a00_0000,
        0x3380_0000,
        0x3f80_0000,
        0x3f80_0001,
        0x4b00_0000,
        0x4b00_0001,
        0x7000_0000,
        0x7f7f_fffe,
        0x7f7f_ffff,
        0x0000_00ff,
    ];
    for bits in seeds {
        let lo = f32::from_bits(bits) as f64;
        let hi = f32::from_bits(bits + 1) as f64;
        let mid = (lo + hi) / 2.0;
        for v in [lo, hi, mid, mid * (1.0 + 1e-15), mid * (1.0 - 1e-15)] {
            assert_same(format!("{:.60e}", v).as_bytes());
            assert_same(format!("{:.17e}", v).as_bytes());
        }
    }
}

// ---------------------------------------------------------------------------
// Phase C — inputs not covered above
// ---------------------------------------------------------------------------

/// Long digit strings: far more digits than any fixed-size buffer.
#[test]
fn very_long_numeric_input() {
    let cases = vec![
        format!("1{}", "0".repeat(300)),
        format!("0.{}1", "0".repeat(300)),
        "1".repeat(400),
        format!("{}e-400", "1".repeat(400)),
        format!("0.{}", "9".repeat(400)),
        format!("{}e-800", "9".repeat(400)),
        format!("1{}", "0".repeat(100_000)),
        format!("0.{}", "1".repeat(100_000)),
        format!("0x{}", "f".repeat(10_000)),
        format!("0x1.{}p0", "f".repeat(10_000)),
        format!("1e{}", "9".repeat(10_000)),
        format!("1e-{}", "9".repeat(10_000)),
        format!("{}1", "0".repeat(10_000)),
        format!("{}1.5", " ".repeat(10_000)),
        format!("1.5{}", "z".repeat(10_000)),
    ];
    for c in &cases {
        assert_same(c.as_bytes());
    }
}

/// Bytes that are not valid UTF-8 and embedded NULs must not change behaviour.
#[test]
fn non_utf8_and_nul_bytes() {
    let cases: Vec<Vec<u8>> = vec![
        b"\x00".to_vec(),
        b"\x001".to_vec(),
        b"1\x002".to_vec(),
        b"1.5\x00junk".to_vec(),
        b"\xff\xfe".to_vec(),
        b"\xff1.5".to_vec(),
        b"1.5\xff".to_vec(),
        (128u8..=255).collect(),
        b"\xc3\xa9".to_vec(),
        b"1\xc3\xa92".to_vec(),
    ];
    for c in &cases {
        assert_same(c);
    }
}

/// Every single byte on its own: a cheap sweep of the first-character branch.
#[test]
fn every_single_byte() {
    for b in 0u8..=255 {
        assert_same(&[b]);
    }
}

/// Two-byte prefixes over the interesting alphabet, which reaches the
/// "sign then something" and "digit then something" branches exhaustively.
#[test]
fn two_byte_combinations() {
    let alpha = b"0123456789.eEpPxX+- \t\nabcfinty()";
    for &a in alpha {
        for &b in alpha {
            assert_same(&[a, b]);
        }
    }
}

/// Deterministic pseudo-random sweep over shapes the grammar cares about.
#[test]
fn randomised_sweep() {
    // xorshift64*, so the corpus is fixed without pulling in a dependency.
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = move || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state.wrapping_mul(0x2545_F491_4F6C_DD1D)
    };
    let alpha: &[u8] = b"0123456789.eE+-xXpPabcdfinty()_ \t\n";

    for _ in 0..600 {
        let len = (next() % 14) as usize;
        let s: Vec<u8> = (0..len).map(|_| alpha[(next() % alpha.len() as u64) as usize]).collect();
        assert_same(&s);
    }

    for _ in 0..400 {
        let mut s = String::new();
        if next() % 2 == 0 {
            s.push(if next() % 2 == 0 { '-' } else { '+' });
        }
        let ints = (next() % 12) as usize;
        for _ in 0..ints {
            s.push((b'0' + (next() % 10) as u8) as char);
        }
        if next() % 3 != 0 {
            s.push('.');
            for _ in 0..(next() % 12) {
                s.push((b'0' + (next() % 10) as u8) as char);
            }
        }
        if next() % 2 == 0 {
            s.push(if next() % 2 == 0 { 'e' } else { 'E' });
            if next() % 2 == 0 {
                s.push(if next() % 2 == 0 { '-' } else { '+' });
            }
            for _ in 0..(1 + next() % 3) {
                s.push((b'0' + (next() % 10) as u8) as char);
            }
        }
        assert_same(s.as_bytes());
    }

    for _ in 0..400 {
        let mut s = String::new();
        if next() % 2 == 0 {
            s.push('-');
        }
        s.push_str(if next() % 2 == 0 { "0x" } else { "0X" });
        let hx = b"0123456789abcdefABCDEF";
        for _ in 0..(next() % 20) {
            s.push(hx[(next() % hx.len() as u64) as usize] as char);
        }
        if next() % 2 == 0 {
            s.push('.');
            for _ in 0..(next() % 14) {
                s.push(hx[(next() % hx.len() as u64) as usize] as char);
            }
        }
        if next() % 4 != 0 {
            s.push(if next() % 2 == 0 { 'p' } else { 'P' });
            if next() % 2 == 0 {
                s.push(if next() % 2 == 0 { '-' } else { '+' });
            }
            for _ in 0..(next() % 4) {
                s.push((b'0' + (next() % 10) as u8) as char);
            }
        }
        assert_same(s.as_bytes());
    }
}

/// stdout's reader is gone before the program writes: the C program is killed by
/// `SIGPIPE`, so the Rust program must be too (the Rust runtime ignores
/// `SIGPIPE` by default, which would give exit 0 instead).
#[cfg(unix)]
#[test]
fn stdout_reader_closed_gives_the_same_signal() {
    use std::os::fd::FromRawFd;
    use std::os::unix::process::ExitStatusExt;

    extern "C" {
        fn pipe(fds: *mut i32) -> i32;
        fn close(fd: i32) -> i32;
    }

    fn status_of(exe: &Path) -> (Option<i32>, Option<i32>, Vec<u8>) {
        let mut fds = [-1i32; 2];
        assert_eq!(unsafe { pipe(fds.as_mut_ptr()) }, 0, "pipe() failed");
        // Close the read end, so any write from the child raises SIGPIPE.
        assert_eq!(unsafe { close(fds[0]) }, 0, "close(read end) failed");
        // Ownership of the write end moves into Stdio and is closed with it.
        let stdout = unsafe { Stdio::from_raw_fd(fds[1]) };

        let child = Command::new(exe)
            .stdin(Stdio::null())
            .stdout(stdout)
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn");
        let out = child.wait_with_output().expect("wait");
        (out.status.code(), out.status.signal(), out.stderr)
    }

    let c = status_of(c_bin());
    let r = status_of(rust_bin());
    assert_eq!(c, r, "C {:?} vs Rust {:?} with stdout's reader closed", c, r);
}

/// stdin closed / at immediate EOF from a device rather than a pipe.
#[test]
fn stdin_at_immediate_eof() {
    for exe in [c_bin(), rust_bin()] {
        let out = Command::new(exe)
            .stdin(Stdio::null())
            .output()
            .expect("spawn with /dev/null stdin");
        assert_eq!(out.stdout, b"00000000\n".to_vec(), "{}", exe.display());
        assert!(out.stderr.is_empty(), "{}", exe.display());
        assert_eq!(out.status.code(), Some(0), "{}", exe.display());
    }
}
