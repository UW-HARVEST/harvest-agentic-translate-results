//! Differential tests: run the original C binary and the Rust binary as
//! subprocesses with identical stdin, and require byte-identical stdout,
//! byte-identical stderr, and an identical exit status.
//!
//! The Rust code is NEVER used as a library here -- both programs are driven
//! exactly the way a shell would drive them, because that is how the
//! translation is graded.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Locating / building the two executables
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

fn rust_bin() -> PathBuf {
    // The integration-test binary lives in target/<profile>/deps/, so the
    // binary under test is two levels up.
    let mut p = std::env::current_exe().expect("current_exe");
    p.pop(); // deps/
    if p.ends_with("deps") {
        p.pop();
    }
    let candidate = p.join(if cfg!(windows) { "driver.exe" } else { "driver" });
    if candidate.exists() {
        return candidate;
    }
    // Fallbacks for unusual invocations.
    for profile in ["release", "debug"] {
        let c = repo_root()
            .join("translation/target")
            .join(profile)
            .join("driver");
        if c.exists() {
            return c;
        }
    }
    panic!(
        "could not find the built Rust `driver` binary near {}",
        candidate.display()
    );
}

/// Build the C program with CMake (out-of-tree, so nothing in c_src/ is
/// touched) and return the path to the executable. Reuses a pre-existing
/// c_src/build/driver if one is already there.
fn c_bin() -> PathBuf {
    static C: OnceLock<PathBuf> = OnceLock::new();
    C.get_or_init(|| {
        let root = repo_root();
        let src = root.join("c_src");
        assert!(
            src.join("src/main.c").exists(),
            "c_src/src/main.c not found at {}",
            src.display()
        );

        let prebuilt = src.join("build/driver");
        if prebuilt.exists() {
            return prebuilt;
        }

        // Out-of-tree build directory under translation/target so that
        // nothing inside c_src/ is created or modified.
        let build_dir = root.join("translation/target/c_build");
        std::fs::create_dir_all(&build_dir).expect("create c build dir");

        let cfg = Command::new("cmake")
            .arg("-S")
            .arg(&src)
            .arg("-B")
            .arg(&build_dir)
            .output()
            .expect("failed to run `cmake` -- is CMake installed?");
        assert!(
            cfg.status.success(),
            "cmake configure failed:\n{}\n{}",
            String::from_utf8_lossy(&cfg.stdout),
            String::from_utf8_lossy(&cfg.stderr)
        );

        let bld = Command::new("cmake")
            .arg("--build")
            .arg(&build_dir)
            .output()
            .expect("failed to run `cmake --build`");
        assert!(
            bld.status.success(),
            "cmake build failed:\n{}\n{}",
            String::from_utf8_lossy(&bld.stdout),
            String::from_utf8_lossy(&bld.stderr)
        );

        let exe = build_dir.join("driver");
        assert!(exe.exists(), "C binary missing at {}", exe.display());
        exe
    })
    .clone()
}

// ---------------------------------------------------------------------------
// Running and comparing
// ---------------------------------------------------------------------------

fn run(exe: &Path, stdin_bytes: &[u8]) -> Output {
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));

    {
        let mut si = child.stdin.take().expect("stdin pipe");
        // The child may exit without draining stdin; a broken pipe is fine.
        let _ = si.write_all(stdin_bytes);
        let _ = si.flush();
    }

    child.wait_with_output().expect("wait_with_output")
}

fn show(b: &[u8]) -> String {
    match std::str::from_utf8(b) {
        Ok(s) => s.to_string(),
        Err(_) => format!("{b:?}"),
    }
}

/// The single assertion used by every test: stdout, stderr and exit status
/// must all match between the C program and the Rust program.
#[track_caller]
fn assert_same(label: &str, stdin_bytes: &[u8]) {
    let c = run(&c_bin(), stdin_bytes);
    let r = run(&rust_bin(), stdin_bytes);

    assert_eq!(
        c.stdout,
        r.stdout,
        "STDOUT mismatch for {label} (stdin = {:?})\n--- C stdout ---\n{}\n--- Rust stdout ---\n{}",
        show(stdin_bytes),
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "STDERR mismatch for {label} (stdin = {:?})\n--- C stderr ---\n{}\n--- Rust stderr ---\n{}",
        show(stdin_bytes),
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "EXIT CODE mismatch for {label} (stdin = {:?}): C = {:?}, Rust = {:?}",
        show(stdin_bytes),
        c.status,
        r.status
    );
    // Also compare signals / raw status representation on unix.
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(
            c.status.signal(),
            r.status.signal(),
            "SIGNAL mismatch for {label} (stdin = {:?})",
            show(stdin_bytes)
        );
    }
}

#[track_caller]
fn same(label: &str, stdin_text: &str) {
    assert_same(label, stdin_text.as_bytes());
}

// ---------------------------------------------------------------------------
// Phase A -- both programs exist and are runnable
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_build_and_run() {
    let c = c_bin();
    let r = rust_bin();
    assert!(c.exists(), "C binary missing: {}", c.display());
    assert!(r.exists(), "Rust binary missing: {}", r.display());

    // A trivial run must succeed for both.
    let oc = run(&c, b"1");
    let or = run(&r, b"1");
    assert!(oc.status.success(), "C program failed on trivial input");
    assert!(or.status.success(), "Rust program failed on trivial input");
    assert!(!oc.stdout.is_empty(), "C program printed nothing");
}

// ---------------------------------------------------------------------------
// Phase B -- the input classes the C program branches on
//
// main() is:   int x = 0; scanf("%d", &x); run(x); run(x);
// so the branch structure is entirely inside glibc's %d conversion:
//   * successful conversion            -> x = value
//   * matching failure / EOF           -> x stays 0 (scanf's return ignored)
// plus the arithmetic in run(): floors++ twice, bathrooms += 1.0 twice,
// bedrooms += x twice (which can overflow int).
// ---------------------------------------------------------------------------

#[test]
fn empty_input_scanf_hits_eof_and_x_stays_zero() {
    // EOF before any conversion: scanf returns EOF, x is left at 0.
    same("empty input", "");
}

#[test]
fn single_item_happy_path() {
    same("single value 3", "3");
    same("single value with newline", "3\n");
    same("single value 1", "1");
}

#[test]
fn zero_value() {
    same("zero", "0");
    same("negative zero", "-0");
    same("plus zero", "+0");
}

#[test]
fn positive_and_negative_and_signs() {
    same("negative", "-3");
    same("explicit plus", "+7");
    same("large positive", "1000000");
    same("large negative", "-1000000");
}

#[test]
fn leading_whitespace_is_skipped_across_newlines() {
    // %d skips ALL leading whitespace, including newlines -- unlike fgets,
    // scanf happily reads across line boundaries.
    same("spaces then value", "   42");
    same("newlines then value", "\n\n\n42");
    same("mixed whitespace then value", " \t\r\n\x0b\x0c 42 \n");
    same("many blank lines", "\n\n\n\n\n\n\n\n7\n");
}

#[test]
fn whitespace_only_input_is_eof_for_percent_d() {
    same("single space", " ");
    same("single newline", "\n");
    same("only whitespace", "   \t\n\r\n  ");
    same("only newlines", "\n\n\n\n");
}

#[test]
fn matching_failure_leaves_x_untouched() {
    // Non-numeric first non-space character: matching failure, x stays 0.
    same("letters", "abc");
    same("leading letter then digits", "a123");
    same("punctuation", "!!!");
    same("dot", ".");
    same("comma", ",");
    same("underscore", "_5");
    same("whitespace then letters", "   \n xyz");
}

#[test]
fn sign_with_no_digits_is_a_matching_failure() {
    same("lone minus", "-");
    same("lone plus", "+");
    same("minus then letter", "-a");
    same("plus then letter", "+a");
    same("minus then newline", "-\n");
    same("double minus", "--5");
    same("plus minus", "+-5");
    same("minus then space then digits", "- 5");
}

#[test]
fn conversion_stops_at_the_first_non_digit() {
    same("digits then letters", "12abc");
    same("hex-looking input stops after 0", "0x10");
    same("decimal point stops at dot", "3.7");
    same("negative decimal", "-3.7");
    same("digits then comma", "5,6");
    same("scientific notation", "2e3");
    same("digits then minus", "5-6");
}

#[test]
fn only_the_first_item_is_read() {
    // There is exactly one scanf call; trailing data is never consumed.
    same("two numbers", "3 99");
    same("three numbers on lines", "3\n99\n-7\n");
    same("number then garbage", "4 this is ignored");
}

#[test]
fn leading_zeros() {
    same("leading zeros", "000000000000005");
    same("negative leading zeros", "-00000000000000042");
    same("all zeros", "0000000000000000000000000");
}

// ---------------------------------------------------------------------------
// Phase B/C -- integer boundaries: `bedrooms += x` is done twice, so the
// int arithmetic must truncate/wrap exactly the way the C does.
// ---------------------------------------------------------------------------

#[test]
fn int_boundaries_and_overflow_in_add_bedrooms() {
    same("INT_MAX", "2147483647");
    same("INT_MAX-1", "2147483646");
    same("INT_MAX-5", "2147483642");
    same("INT_MIN", "-2147483648");
    same("INT_MIN+1", "-2147483647");
    same("just past INT_MAX", "2147483648");
    same("just past INT_MIN", "-2147483649");
    same("2^31", "2147483648");
    same("2^31+1", "2147483649");
    // bedrooms starts at 5; these make the two `+=` overflow int.
    same("half of INT_MAX", "1073741824");
    same("negative half", "-1073741824");
}

#[test]
fn values_beyond_int_are_truncated_the_way_glibc_does() {
    same("2^32", "4294967296");
    same("2^32+5", "4294967301");
    same("2^32-1", "4294967295");
    same("LONG_MAX", "9223372036854775807");
    same("LONG_MAX+1", "9223372036854775808");
    same("LONG_MIN", "-9223372036854775808");
    same("LONG_MIN-1", "-9223372036854775809");
    same("2^64", "18446744073709551616");
    same("2^64-1", "18446744073709551615");
    same("20 nines", "99999999999999999999");
    same("negative 20 nines", "-99999999999999999999");
    same("1e25", "10000000000000000000000000");
    same("2^63 + 2^31", "9223372039002259456");
}

#[test]
fn absurdly_long_digit_run() {
    let s = "9".repeat(5000);
    assert_same("5000 nines", s.as_bytes());
    let s = format!("-{}", "8".repeat(5000));
    assert_same("negative 5000 eights", s.as_bytes());
    let s = format!("{}{}", "0".repeat(5000), "7");
    assert_same("5000 leading zeros then 7", s.as_bytes());
}

// ---------------------------------------------------------------------------
// Phase C -- non-text / hostile stdin
// ---------------------------------------------------------------------------

#[test]
fn embedded_nul_and_non_utf8_bytes() {
    assert_same("nul byte first", b"\x00");
    assert_same("nul after letters", b"ab\x00cd");
    assert_same("digits then nul", b"12\x00 34");
    assert_same("invalid utf8", b"\xff\xfe\x80 12");
    assert_same("high bytes then digits", b"\xc3\xa9 9");
    assert_same("nul then digits", b"\x0012");
}

#[test]
fn no_trailing_newline_variants() {
    same("value, no newline", "17");
    same("value, crlf", "17\r\n");
    same("value, cr only", "17\r");
}

#[test]
fn stdin_closed_immediately() {
    // Same as empty input, but exercised via an immediately-closed pipe.
    assert_same("closed stdin", b"");
}

// ---------------------------------------------------------------------------
// Phase C -- broad sweep so no reachable branch is left untested
// ---------------------------------------------------------------------------

#[test]
fn sweep_small_values() {
    for v in -40i64..=40 {
        assert_same(&format!("value {v}"), v.to_string().as_bytes());
    }
}

#[test]
fn sweep_generated_token_soup() {
    // Deterministic pseudo-random inputs drawn from the alphabet that the
    // %d conversion actually distinguishes between.
    let alpha: &[u8] = b"0123456789+- \t\n\r\x0b\x0cxabZ.,";
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = |n: u64| -> u64 {
        // xorshift64*
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state.wrapping_mul(0x2545_F491_4F6C_DD1D) % n
    };
    for case in 0..300 {
        let len = next(27) as usize;
        let mut buf = Vec::with_capacity(len);
        for _ in 0..len {
            buf.push(alpha[next(alpha.len() as u64) as usize]);
        }
        assert_same(&format!("soup case {case}"), &buf);
    }
}

#[test]
fn sweep_random_bytes() {
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = || -> u64 {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state.wrapping_mul(0x2545_F491_4F6C_DD1D)
    };
    for case in 0..150 {
        let len = (next() % 18) as usize;
        let mut buf = Vec::with_capacity(len);
        for _ in 0..len {
            buf.push((next() >> 33) as u8);
        }
        assert_same(&format!("random bytes case {case}"), &buf);
    }
}

// ---------------------------------------------------------------------------
// Phase C -- stdout write failures. `printf` returns an error code that the
// C program ignores, so a failed write must stay silent; and a C program
// inherits the default SIGPIPE disposition, so a closed reader kills it.
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn stdout_write_error_is_silent_like_printf() {
    use std::fs::OpenOptions;

    // /dev/full accepts opens but fails every write with ENOSPC.
    let full = match OpenOptions::new().write(true).open("/dev/full") {
        Ok(f) => f,
        Err(_) => return, // platform without /dev/full: nothing to compare
    };
    drop(full);

    let run_to_dev_full = |exe: &Path| -> (Option<i32>, Vec<u8>) {
        let out = OpenOptions::new().write(true).open("/dev/full").unwrap();
        let mut child = Command::new(exe)
            .stdin(Stdio::piped())
            .stdout(Stdio::from(out))
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        {
            let mut si = child.stdin.take().unwrap();
            let _ = si.write_all(b"3");
        }
        let o = child.wait_with_output().unwrap();
        (o.status.code(), o.stderr)
    };

    let c = run_to_dev_full(&c_bin());
    let r = run_to_dev_full(&rust_bin());
    assert_eq!(
        c.0, r.0,
        "exit code differs when stdout writes fail: C = {:?}, Rust = {:?}",
        c.0, r.0
    );
    assert_eq!(
        show(&c.1),
        show(&r.1),
        "stderr differs when stdout writes fail (C printf fails silently)"
    );
    assert_eq!(c.0, Some(0), "C exits 0 even though printf failed");
}

#[cfg(unix)]
#[test]
fn closed_stdout_pipe_produces_the_same_termination() {
    use std::os::unix::io::FromRawFd;
    use std::os::unix::process::ExitStatusExt;

    let run_with_closed_reader = |exe: &Path| -> (Option<i32>, Option<i32>, Vec<u8>) {
        // Create a pipe, hand the write end to the child, then close the read
        // end so the very first write hits a reader-less pipe.
        let mut fds = [0i32; 2];
        let rc = unsafe { libc_pipe(fds.as_mut_ptr()) };
        assert_eq!(rc, 0, "pipe() failed");
        let (read_fd, write_fd) = (fds[0], fds[1]);
        let stdout = unsafe { std::fs::File::from_raw_fd(write_fd) };

        let mut child = Command::new(exe)
            .stdin(Stdio::piped())
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        unsafe { libc_close(read_fd) };
        {
            let mut si = child.stdin.take().unwrap();
            let _ = si.write_all(b"3");
        }
        let o = child.wait_with_output().unwrap();
        (o.status.code(), o.status.signal(), o.stderr)
    };

    let c = run_with_closed_reader(&c_bin());
    let r = run_with_closed_reader(&rust_bin());
    assert_eq!(
        (c.0, c.1),
        (r.0, r.1),
        "termination differs for a closed stdout pipe: C = {:?}, Rust = {:?}",
        (c.0, c.1),
        (r.0, r.1)
    );
    assert_eq!(
        show(&c.2),
        show(&r.2),
        "stderr differs for a closed stdout pipe"
    );
}

#[cfg(unix)]
extern "C" {
    #[link_name = "pipe"]
    fn libc_pipe(fds: *mut i32) -> i32;
    #[link_name = "close"]
    fn libc_close(fd: i32) -> i32;
}

// ---------------------------------------------------------------------------
// Output-shape checks: printf formatting must be reproduced exactly.
// ---------------------------------------------------------------------------

#[test]
fn output_shape_matches_printf_exactly() {
    // Eight lines total (four per run(), run() called twice), each ending in
    // a newline, with "%.1f" formatting for bathrooms.
    let out = run(&c_bin(), b"3").stdout;
    let rout = run(&rust_bin(), b"3").stdout;
    assert_eq!(out, rout);

    let text = String::from_utf8(out).expect("ascii output");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 8, "expected 8 lines, got: {text:?}");
    assert!(text.ends_with('\n'), "output must end with a newline");
    assert_eq!(
        lines[0],
        "The house has 2 floors, 5 bedrooms, and 2.5 bathrooms"
    );
    assert_eq!(
        lines[7],
        "The house has 4 floors, 11 bedrooms, and 4.5 bathrooms"
    );
    // Every line must carry exactly one decimal place.
    for l in &lines {
        let frac = l
            .rsplit_once("and ")
            .and_then(|(_, t)| t.split_once(" bathrooms"))
            .map(|(n, _)| n)
            .expect("bathroom count present");
        let dec = frac.split_once('.').expect("decimal point").1;
        assert_eq!(dec.len(), 1, "expected %.1f formatting, saw {frac:?}");
    }
}

#[test]
fn stderr_is_empty_for_both_on_every_class() {
    for input in ["", "5", "abc", "-", "2147483647", "\n\n\n"] {
        let c = run(&c_bin(), input.as_bytes());
        let r = run(&rust_bin(), input.as_bytes());
        assert!(
            c.stderr.is_empty(),
            "C wrote to stderr for {input:?}: {}",
            show(&c.stderr)
        );
        assert_eq!(c.stderr, r.stderr, "stderr differs for {input:?}");
    }
}

#[test]
fn exit_status_is_zero_for_both_on_every_class() {
    for input in ["", "5", "abc", "-", "+", "0x10", "2147483647", "-99999999999999999999"] {
        let c = run(&c_bin(), input.as_bytes());
        let r = run(&rust_bin(), input.as_bytes());
        assert_eq!(c.status.code(), Some(0), "C exit code for {input:?}");
        assert_eq!(
            c.status.code(),
            r.status.code(),
            "exit code differs for {input:?}"
        );
    }
}
