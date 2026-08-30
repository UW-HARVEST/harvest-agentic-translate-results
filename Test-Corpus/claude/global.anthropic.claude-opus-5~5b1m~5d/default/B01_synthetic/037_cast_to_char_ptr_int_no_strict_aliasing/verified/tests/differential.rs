//! Differential tests: run the original C program and the Rust translation as
//! subprocesses on identical stdin and require byte-identical stdout, stderr
//! and exit status.
//!
//! Nothing here links the Rust code as a library; both programs are driven the
//! way a shell would drive them, which is how this translation is graded.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

fn repo_root() -> PathBuf {
    // tests/ live in <root>/translation/tests
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// Path to the Rust binary under test (built by cargo for this test run).
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Path to the compiled C binary, building it on first use.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(build_c_binary).as_path()
}

fn build_c_binary() -> PathBuf {
    let root = repo_root();
    let c_src = root.join("c_src");
    let build_dir = c_src.join("build");
    let cmake_out = build_dir.join("driver");
    if cmake_out.is_file() {
        return cmake_out;
    }

    // Preferred: the project's own CMake build.
    std::fs::create_dir_all(&build_dir).expect("create c_src/build");
    let configured = Command::new("cmake")
        .arg("..")
        .current_dir(&build_dir)
        .output();
    if let Ok(out) = configured {
        if out.status.success() {
            let built = Command::new("cmake")
                .args(["--build", "."])
                .current_dir(&build_dir)
                .output()
                .expect("run cmake --build");
            assert!(
                built.status.success(),
                "cmake --build failed:\n{}\n{}",
                String::from_utf8_lossy(&built.stdout),
                String::from_utf8_lossy(&built.stderr)
            );
            assert!(cmake_out.is_file(), "cmake did not produce {cmake_out:?}");
            return cmake_out;
        }
    }

    // Fallback: compile directly with the same flag CMakeLists.txt uses.
    let direct = build_dir.join("driver_cc");
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
    let out = Command::new(&cc)
        .arg("-fno-strict-aliasing")
        .arg("-o")
        .arg(&direct)
        .arg(c_src.join("src/main.c"))
        .output()
        .unwrap_or_else(|e| panic!("failed to invoke {cc}: {e}"));
    assert!(
        out.status.success(),
        "{cc} failed to build the C program:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    direct
}

struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    status: Option<i32>,
    signal: Option<i32>,
}

fn run(program: &Path, stdin_bytes: &[u8]) -> Run {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {program:?}: {e}"));

    {
        let mut sin = child.stdin.take().expect("stdin pipe");
        let bytes = stdin_bytes.to_vec();
        // Write on a helper thread so a program that never reads stdin (or a
        // large payload exceeding the pipe buffer) cannot deadlock the test.
        std::thread::spawn(move || {
            let _ = sin.write_all(&bytes);
            let _ = sin.flush();
        });
    }

    let out = child.wait_with_output().expect("wait for child");

    #[cfg(unix)]
    let signal = {
        use std::os::unix::process::ExitStatusExt;
        out.status.signal()
    };
    #[cfg(not(unix))]
    let signal = None;

    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        status: out.status.code(),
        signal,
    }
}

fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).escape_debug().to_string()
}

/// The core assertion: identical stdout, stderr and exit status.
fn assert_same(name: &str, stdin_bytes: &[u8]) {
    let c = run(c_bin(), stdin_bytes);
    let r = run(&rust_bin(), stdin_bytes);

    let input = if stdin_bytes.len() > 80 {
        format!(
            "<{} bytes, starts {:?}>",
            stdin_bytes.len(),
            show(&stdin_bytes[..40])
        )
    } else {
        show(stdin_bytes)
    };

    assert_eq!(
        c.stdout,
        r.stdout,
        "[{name}] stdout differs for input {input}\n  C: {}\n  R: {}",
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "[{name}] stderr differs for input {input}\n  C: {}\n  R: {}",
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        c.status, r.status,
        "[{name}] exit code differs for input {input}: C={:?} R={:?}",
        c.status, r.status
    );
    assert_eq!(
        c.signal, r.signal,
        "[{name}] termination signal differs for input {input}: C={:?} R={:?}",
        c.signal, r.signal
    );
}

fn check_all(cases: &[(&str, &[u8])]) {
    for (name, input) in cases {
        assert_same(name, input);
    }
}

// ---------------------------------------------------------------------------
// Sanity: both binaries exist and the C one behaves as documented.
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_exist() {
    assert!(c_bin().is_file(), "C binary missing at {:?}", c_bin());
    assert!(
        rust_bin().is_file(),
        "Rust binary missing at {:?}",
        rust_bin()
    );
}

#[test]
fn c_reference_output_is_little_endian_hex() {
    // Guards the harness itself: if this ever changes, the platform changed,
    // not the translation.
    let c = run(c_bin(), b"1\n");
    assert_eq!(c.stdout, b"01000000\n");
    assert_eq!(c.stderr, b"");
    assert_eq!(c.status, Some(0));
}

// ---------------------------------------------------------------------------
// Phase B: the branches the C code actually has.
//
// main() does exactly: x = 0; scanf("%d", &x); driver(x).
// The branch points are therefore entirely inside scanf's %d conversion:
//   * EOF before any non-whitespace  -> x untouched (stays 0)
//   * matching failure               -> x untouched (stays 0)
//   * successful conversion          -> x set, then truncated to int
// plus print_hex's loop over sizeof(int) bytes.
// ---------------------------------------------------------------------------

#[test]
fn empty_and_whitespace_only_input() {
    check_all(&[
        ("empty", b""),
        ("single_newline", b"\n"),
        ("many_newlines", b"\n\n\n\n"),
        ("spaces_only", b"    "),
        ("tabs_only", b"\t\t"),
        ("mixed_ws_only", b" \t\n\r\x0b\x0c"),
        ("no_trailing_newline_eof", b"   "),
    ]);
}

#[test]
fn single_value_happy_path() {
    check_all(&[
        ("zero", b"0"),
        ("zero_nl", b"0\n"),
        ("one", b"1"),
        ("seven_nl", b"7\n"),
        ("neg_one", b"-1"),
        ("plus_five", b"+5"),
        ("leading_zeros", b"007"),
        ("neg_leading_zeros", b"-0007"),
        ("neg_zero", b"-0"),
        ("plus_zero", b"+0"),
        ("byte_boundary_255", b"255"),
        ("byte_boundary_256", b"256"),
        ("65535", b"65535"),
        ("65536", b"65536"),
        ("16777215", b"16777215"),
        ("16777216", b"16777216"),
    ]);
}

#[test]
fn leading_whitespace_is_skipped() {
    check_all(&[
        ("spaces_then_num", b"   42  "),
        ("newlines_then_num", b"\n\n\n7\n"),
        ("tab_then_neg", b"\t-3"),
        ("crlf_then_num", b"\r\n8"),
        ("vt_ff_then_num", b"\x0b\x0c9"),
        ("all_ws_kinds_then_num", b" \t\n\x0b\x0c\r123"),
        // scanf reads across newlines: the number on line 3 is still found.
        ("number_on_third_line", b"\n\n12345\n"),
    ]);
}

#[test]
fn int_limits_and_truncation() {
    check_all(&[
        ("int_max", b"2147483647"),
        ("int_min", b"-2147483648"),
        // Beyond int range: the long value is truncated to int by the
        // assignment scanf performs.
        ("int_max_plus_1", b"2147483648"),
        ("int_min_minus_1", b"-2147483649"),
        ("u32_max", b"4294967295"),
        ("u32_max_plus_1", b"4294967296"),
        ("u32_max_plus_2", b"4294967297"),
        ("neg_u32_max_plus_1", b"-4294967296"),
        ("long_max", b"9223372036854775807"),
        ("long_min", b"-9223372036854775808"),
    ]);
}

#[test]
fn overflow_saturates_then_truncates() {
    check_all(&[
        ("long_max_plus_1", b"9223372036854775808"),
        ("long_min_minus_1", b"-9223372036854775809"),
        ("u64_max", b"18446744073709551615"),
        ("twenty_nines", b"99999999999999999999"),
        ("twenty_nines_neg", b"-99999999999999999999"),
        ("forty_zeros_then_5", b"00000000000000000000000000000000000000005"),
        (
            "forty_zeros_then_overflow",
            b"000000000000000000000000000000000000000099999999999999999999",
        ),
    ]);
}

#[test]
fn huge_digit_strings() {
    let five_thousand_nines = vec![b'9'; 5000];
    let ten_thousand_zeros_then_one = {
        let mut v = vec![b'0'; 10_000];
        v.push(b'1');
        v
    };
    let neg_huge = {
        let mut v = vec![b'-'];
        v.extend(std::iter::repeat(b'7').take(4096));
        v
    };
    check_all(&[
        ("five_thousand_nines", &five_thousand_nines),
        ("ten_thousand_zeros_then_one", &ten_thousand_zeros_then_one),
        ("neg_four_thousand_sevens", &neg_huge),
    ]);
}

#[test]
fn matching_failure_leaves_x_at_zero() {
    check_all(&[
        ("letters", b"abc"),
        ("letters_nl", b"abc\n"),
        ("sign_only_minus", b"-"),
        ("sign_only_plus", b"+"),
        ("sign_only_minus_nl", b"-\n"),
        ("sign_then_space", b"- 5"),
        ("sign_then_letter", b"-a"),
        ("double_minus", b"--5"),
        ("plus_then_minus", b"+-5"),
        ("dot_five", b".5"),
        ("comma_first", b",1"),
        ("underscore", b"_1"),
        ("hash", b"#42"),
        ("nul_byte_first", b"\x005"),
        ("high_byte_first", b"\xff7"),
        ("utf8_first", "é7".as_bytes()),
        ("ws_then_letters", b"   xyz"),
    ]);
}

#[test]
fn conversion_stops_at_first_non_digit() {
    check_all(&[
        ("digits_then_letters", b"12abc"),
        ("digits_then_dot", b"3.99"),
        ("digits_then_comma", b"1,2"),
        ("digits_then_sign", b"5-6"),
        ("e_notation", b"1e3"),
        ("hex_prefix", b"0x1f"),
        ("digits_then_nul", b"42\x00xyz"),
        ("digits_then_high_byte", b"42\xff"),
        // Only the first conversion happens; the rest of stdin is ignored.
        ("two_numbers", b"10 20"),
        ("three_numbers", b"1 2 3\n"),
        ("number_then_garbage_lines", b"99\nignored\nalso ignored\n"),
    ]);
}

#[test]
fn every_output_byte_position_is_exercised() {
    // print_hex walks all sizeof(int) bytes; make each one non-zero and make
    // each one require the leading zero of "%02x".
    check_all(&[
        ("byte0_low_nibble", b"1"),
        ("byte0_needs_pad", b"15"),   // 0x0f
        ("byte1", b"256"),            // 0x00000100
        ("byte1_pad", b"3840"),       // 0x00000f00
        ("byte2", b"65536"),          // 0x00010000
        ("byte2_pad", b"983040"),     // 0x000f0000
        ("byte3", b"16777216"),       // 0x01000000
        ("byte3_pad", b"251658240"),  // 0x0f000000
        ("all_ff", b"-1"),            // ffffffff
        ("alternating_aa", b"-1431655766"), // aaaaaaaa
        ("alternating_55", b"1431655765"),  // 55555555
        ("high_bit_only", b"-2147483648"),  // 00000080
    ]);
}

#[test]
fn no_stdin_at_all_is_same_as_empty() {
    // A closed stdin gives immediate EOF, so x stays 0.
    assert_same("closed_stdin", b"");
}

#[test]
fn nothing_is_written_to_stderr_on_any_path() {
    // Cross-check: the C program never writes stderr, so neither may Rust.
    for input in [
        &b""[..],
        &b"abc"[..],
        &b"-"[..],
        &b"7"[..],
        &b"99999999999999999999"[..],
    ] {
        let c = run(c_bin(), input);
        let r = run(&rust_bin(), input);
        assert_eq!(c.stderr, b"", "C wrote stderr for {:?}", show(input));
        assert_eq!(r.stderr, c.stderr, "Rust stderr mismatch for {:?}", show(input));
        assert_eq!(c.status, Some(0));
        assert_eq!(r.status, Some(0));
    }
}

/// Regression test for the one real mismatch this suite found (see ERRORS.md):
/// the Rust runtime installs `SIG_IGN` for `SIGPIPE`, so writing to a pipe with
/// no reader used to exit 0 where the C program is killed by signal 13.
#[cfg(unix)]
#[test]
fn stdout_pipe_with_dead_reader_matches() {
    use std::os::fd::{FromRawFd, OwnedFd};
    use std::os::unix::process::ExitStatusExt;

    fn run_with_dead_stdout_reader(program: &Path) -> (Option<i32>, Option<i32>, Vec<u8>) {
        // Make a pipe and drop the read end, so the very first write gets EPIPE.
        let mut fds = [0i32; 2];
        let rc = unsafe { pipe(fds.as_mut_ptr()) };
        assert_eq!(rc, 0, "pipe() failed");
        let read_end = unsafe { OwnedFd::from_raw_fd(fds[0]) };
        let write_end = unsafe { OwnedFd::from_raw_fd(fds[1]) };
        drop(read_end);

        let mut child = Command::new(program)
            .stdin(Stdio::piped())
            .stdout(Stdio::from(write_end))
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("spawn {program:?}: {e}"));
        {
            let mut sin = child.stdin.take().expect("stdin");
            let _ = sin.write_all(b"7");
        }
        let out = child.wait_with_output().expect("wait");
        (out.status.code(), out.status.signal(), out.stderr)
    }

    extern "C" {
        fn pipe(fds: *mut i32) -> i32;
    }

    let c = run_with_dead_stdout_reader(c_bin());
    let r = run_with_dead_stdout_reader(&rust_bin());
    assert_eq!(
        c, r,
        "dead-reader stdout: C=(code,signal,stderr){c:?} Rust={r:?}"
    );
}

/// A failing write to a still-open stdout (ENOSPC on /dev/full) must also agree.
#[cfg(unix)]
#[test]
fn stdout_write_error_matches() {
    fn run_to_dev_full(program: &Path) -> (Option<i32>, Vec<u8>) {
        let sink = std::fs::OpenOptions::new()
            .write(true)
            .open("/dev/full")
            .expect("/dev/full");
        let mut child = Command::new(program)
            .stdin(Stdio::piped())
            .stdout(Stdio::from(sink))
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("spawn {program:?}: {e}"));
        {
            let mut sin = child.stdin.take().expect("stdin");
            let _ = sin.write_all(b"7");
        }
        let out = child.wait_with_output().expect("wait");
        (out.status.code(), out.stderr)
    }

    if !Path::new("/dev/full").exists() {
        // Not available on this platform; the dead-reader test still covers
        // the write-failure class.
        return;
    }
    assert_eq!(
        run_to_dev_full(c_bin()),
        run_to_dev_full(&rust_bin()),
        "/dev/full stdout behaviour differs"
    );
}

#[test]
fn exhaustive_small_range_and_powers_of_two() {
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    for v in -40i64..=40 {
        cases.push((format!("small_{v}"), format!("{v}\n").into_bytes()));
    }
    for shift in 0..32 {
        let v = 1i64 << shift;
        cases.push((format!("pow2_{shift}"), format!("{v}\n").into_bytes()));
        cases.push((format!("neg_pow2_{shift}"), format!("-{v}\n").into_bytes()));
        cases.push((format!("pow2m1_{shift}"), format!("{}\n", v - 1).into_bytes()));
    }
    for (name, input) in &cases {
        assert_same(name, input);
    }
}
