//! Differential tests: run the original C program and the Rust translation as
//! subprocesses with identical arguments and require byte-identical stdout,
//! byte-identical stderr and an identical exit status.
//!
//! Nothing here loads the Rust code as a library; both programs are driven
//! exactly the way a shell drives them.
//!
//! `argv[0]` is forced to the same string for both processes because the C
//! program echoes it in its usage message (`fprintf(stderr, "Usage: %s ...",
//! argv[0])`), and the two executables naturally live at different paths.

use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// argv[0] handed to both programs.
const ARG0: &str = "driver";

fn manifest_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn repo_root() -> PathBuf {
    manifest_dir()
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

fn rust_binary() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// Directory `target/` of this crate (derived from the test binary location).
fn target_dir() -> PathBuf {
    // CARGO_BIN_EXE_driver == <target>/<profile>/driver
    rust_binary()
        .parent()
        .and_then(|p| p.parent())
        .expect("binary path should be <target>/<profile>/driver")
        .to_path_buf()
}

/// Path to the compiled C program.
///
/// Prefers the CMake build described in `c_src/CMakeLists.txt`
/// (`c_src/build/driver`). If it has not been produced yet, the very same
/// translation unit is compiled with the very same (empty) flag set that CMake
/// uses when no `CMAKE_BUILD_TYPE` is given, so the result is equivalent.
fn c_binary() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let cmake_built = repo_root().join("c_src").join("build").join("driver");
        if cmake_built.is_file() {
            return cmake_built;
        }

        let src = repo_root().join("c_src").join("src").join("main.c");
        assert!(src.is_file(), "cannot find C source at {}", src.display());

        let out_dir = target_dir().join("ctest");
        std::fs::create_dir_all(&out_dir).expect("create target/ctest");
        let out = out_dir.join("driver");

        let cc = std::env::var_os("CC").unwrap_or_else(|| OsString::from("cc"));
        let status = Command::new(&cc)
            .arg(&src)
            .arg("-o")
            .arg(&out)
            .status()
            .unwrap_or_else(|e| panic!("failed to invoke {:?}: {e}", cc));
        assert!(status.success(), "compiling the C reference failed: {status:?}");
        out
    })
}

#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Ok(code)` for a normal exit, `Err(signal)` when killed by a signal.
    status: Result<i32, i32>,
}

fn run(bin: &Path, args: &[&OsStr]) -> Outcome {
    let mut cmd = Command::new(bin);
    cmd.arg0(ARG0)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // Keep the C library's locale-sensitive bits (isspace, printf) pinned.
        .env("LC_ALL", "C")
        .env("LANG", "C");

    let out = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to run {}: {e}", bin.display()));

    let status = match out.status.code() {
        Some(code) => Ok(code),
        None => Err(out.status.signal().unwrap_or(-1)),
    };

    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        status,
    }
}

fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).escape_debug().to_string()
}

fn describe(args: &[&OsStr]) -> String {
    args.iter()
        .map(|a| format!("{:?}", String::from_utf8_lossy(a.as_bytes())))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The whole point of this file: assert C and Rust agree on all three channels.
fn assert_matches(args: &[&OsStr]) {
    let c = run(c_binary(), args);
    let r = run(rust_binary(), args);
    let ctx = describe(args);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout differs for args [{ctx}]\n  C   : \"{}\"\n  Rust: \"{}\"",
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr differs for args [{ctx}]\n  C   : \"{}\"\n  Rust: \"{}\"",
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        c.status, r.status,
        "exit status differs for args [{ctx}]\n  C   : {:?}\n  Rust: {:?}",
        c.status, r.status
    );
}

fn bytes(s: &[u8]) -> &OsStr {
    OsStr::from_bytes(s)
}

/// Convenience for the (many) single-argument cases.
fn check1(arg: &[u8]) {
    assert_matches(&[bytes(arg)]);
}

// ---------------------------------------------------------------------------
// Two exotic-but-observable process-level behaviours.
// ---------------------------------------------------------------------------

/// Run `bin` with a stderr whose reader has already gone away, and report how
/// the process terminated.
fn run_with_broken_stderr(bin: &Path, args: &[&OsStr]) -> Result<i32, i32> {
    use std::os::fd::OwnedFd;
    use std::os::unix::net::UnixStream;

    let (mine, peer) = UnixStream::pair().expect("socketpair");
    // Closed before the child is even created, so there is no race: the first
    // write to fd 2 must fail.
    drop(peer);
    let fd: OwnedFd = mine.into();

    let mut child = Command::new(bin)
        .arg0(ARG0)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::from(fd))
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    let st = child.wait().expect("wait");
    match st.code() {
        Some(code) => Ok(code),
        None => Err(st.signal().unwrap_or(-1)),
    }
}

extern "C" {
    fn fork() -> i32;
    fn dup2(oldfd: i32, newfd: i32) -> i32;
    fn execv(path: *const std::ffi::c_char, argv: *const *const std::ffi::c_char) -> i32;
    fn _exit(status: i32) -> !;
    fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
}

/// `exec` `bin` with a completely empty `argv`, capturing stderr into
/// `stderr_path`.
///
/// (Modern Linux kernels substitute a single empty string, so both programs see
/// `argc == 1` and an empty `argv[0]`; the point is that they must agree.)
///
/// `std::process::Command` cannot express this, hence the raw fork/exec.
fn exec_with_empty_argv(bin: &Path, stderr_path: &Path) -> Result<i32, i32> {
    use std::ffi::CString;
    use std::os::fd::AsRawFd;

    let path = CString::new(bin.as_os_str().as_bytes()).expect("path without NUL");
    let err_file = std::fs::File::create(stderr_path).expect("create stderr capture file");
    let err_fd = err_file.as_raw_fd();
    let null = std::fs::File::create("/dev/null").expect("open /dev/null");
    let null_fd = null.as_raw_fd();

    // SAFETY: the child does nothing but two dup2 calls and an execv, all of
    // which are async-signal-safe.
    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        unsafe {
            dup2(null_fd, 0);
            dup2(null_fd, 1);
            dup2(err_fd, 2);
            let argv: [*const std::ffi::c_char; 1] = [std::ptr::null()];
            execv(path.as_ptr(), argv.as_ptr());
            _exit(127);
        }
    }

    let mut raw: i32 = 0;
    let waited = unsafe { waitpid(pid, &mut raw, 0) };
    assert_eq!(waited, pid, "waitpid failed");
    drop(err_file);
    drop(null);

    if raw & 0x7f == 0 {
        Ok((raw >> 8) & 0xff)
    } else {
        Err(raw & 0x7f)
    }
}

#[test]
fn broken_stderr_pipe_terminates_both_the_same_way() {
    // The C program has no SIGPIPE handler, so `fprintf(stderr, ...)` on a
    // closed pipe kills it with SIGPIPE. The Rust runtime ignores SIGPIPE by
    // default, which would otherwise make the translation exit 1 instead.
    let c = run_with_broken_stderr(c_binary(), &[]);
    let r = run_with_broken_stderr(rust_binary(), &[]);
    assert_eq!(c, r, "termination differs with a broken stderr pipe");

    let c = run_with_broken_stderr(c_binary(), &[bytes(b"nope")]);
    let r = run_with_broken_stderr(rust_binary(), &[bytes(b"nope")]);
    assert_eq!(c, r, "termination differs with a broken stderr pipe");
}

#[test]
fn empty_argv_usage_message_matches() {
    // Exec'ing with an empty argv. Linux turns this into argc == 1 with an
    // empty argv[0], so the usage line must read "Usage:  <seed>".
    let dir = target_dir().join("argv0");
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    let c_err = dir.join("c.err");
    let r_err = dir.join("rust.err");

    let c_status = exec_with_empty_argv(c_binary(), &c_err);
    let r_status = exec_with_empty_argv(rust_binary(), &r_err);

    let c_bytes = std::fs::read(&c_err).expect("read c stderr");
    let r_bytes = std::fs::read(&r_err).expect("read rust stderr");

    assert_eq!(
        c_bytes,
        r_bytes,
        "stderr differs for an empty argv\n  C   : \"{}\"\n  Rust: \"{}\"",
        show(&c_bytes),
        show(&r_bytes)
    );
    assert_eq!(c_status, r_status, "exit status differs for an empty argv");
}

// ---------------------------------------------------------------------------
// Phase A sanity: both programs exist and are runnable.
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_are_executable() {
    assert!(
        c_binary().is_file(),
        "C binary missing at {}",
        c_binary().display()
    );
    assert!(
        rust_binary().is_file(),
        "Rust binary missing at {}",
        rust_binary().display()
    );
    // Cheapest possible round-trip through both: the usage error path.
    assert_matches(&[]);
}

// ---------------------------------------------------------------------------
// `argc != 2` -> usage message on stderr, exit 1.
// ---------------------------------------------------------------------------

#[test]
fn argc_zero_extra_args_prints_usage() {
    assert_matches(&[]);
}

#[test]
fn argc_two_extra_args_prints_usage() {
    assert_matches(&[bytes(b"1"), bytes(b"2")]);
}

#[test]
fn argc_three_extra_args_prints_usage() {
    assert_matches(&[bytes(b"1"), bytes(b"2"), bytes(b"3")]);
}

/// Even an otherwise-invalid seed must not be reported: the `argc` check comes
/// first in the C source.
#[test]
fn argc_check_precedes_seed_validation() {
    assert_matches(&[bytes(b"not-a-number"), bytes(b"extra")]);
}

#[test]
fn argc_check_with_empty_extra_args() {
    assert_matches(&[bytes(b""), bytes(b"")]);
}

// ---------------------------------------------------------------------------
// Invalid seeds: `*endptr != '\0' || errno != 0 || temp_seed > UINT_MAX`.
// Every one of these exits 1 after printing `Invalid seed: '<arg>'`.
// ---------------------------------------------------------------------------

#[test]
fn invalid_no_digits_at_all() {
    for arg in [
        &b"abc"[..],
        b"x",
        b".",
        b".5",
        b"-",
        b"+",
        b"--1",
        b"++1",
        b"+-1",
        b"- 1",
        b"+ 1",
        b"/",
        b":",
        b"e5",
        b"_1",
    ] {
        check1(arg);
    }
}

#[test]
fn invalid_whitespace_only() {
    // strtoul skips the leading white space, finds no digits, and leaves
    // endptr at the start of the string, so `*endptr` is the space itself.
    for arg in [
        &b" "[..],
        b"\t",
        b"\n",
        b"\r",
        b"\x0b",
        b"\x0c",
        b"   ",
        b" \t\n\x0b\x0c\r",
    ] {
        check1(arg);
    }
}

#[test]
fn invalid_trailing_garbage() {
    for arg in [
        &b"12a"[..],
        b"0x10",
        b"0X10",
        b"0b101",
        b"1e5",
        b"1_000",
        b"1,5",
        b"1.5",
        b"1 ",
        b"1\n",
        b"1\t",
        b"  42  ",
        b"42 ",
        b"4294967295x",
        b"2147483648x",
        b"0-",
        b"1+1",
    ] {
        check1(arg);
    }
}

#[test]
fn invalid_negative_values() {
    // strtoul negates modulo 2^64, so a small negative input lands far above
    // UINT_MAX. (-0 and the two wrap-all-the-way-around values are tested
    // separately, as *valid* seeds.)
    for arg in [
        &b"-1"[..],
        b"-2",
        b"-42",
        b"-4294967295",
        b"-4294967296",
        b"-00000000001",
    ] {
        check1(arg);
    }
}

#[test]
fn invalid_above_uint_max() {
    for arg in [
        &b"4294967296"[..],
        b"4294967297",
        b"4294967300",
        b"5000000000",
        b"9999999999",
        b"18446744073709551614",
        b"18446744073709551615", // ULONG_MAX, but errno is *not* set
        b"0004294967296",
    ] {
        check1(arg);
    }
}

#[test]
fn invalid_erange_overflow() {
    // > ULONG_MAX: glibc sets errno = ERANGE and returns ULONG_MAX.
    let long_nines = vec![b'9'; 400];
    let long_zeros_then_overflow = {
        let mut v = vec![b'0'; 100];
        v.extend_from_slice(b"18446744073709551616");
        v
    };
    for arg in [
        &b"18446744073709551616"[..],
        b"18446744073709551617",
        b"99999999999999999999999",
        b"-99999999999999999999999",
        b"+18446744073709551616",
        &long_nines,
        &long_zeros_then_overflow,
    ] {
        check1(arg);
    }
}

#[test]
fn invalid_ulong_max_boundary_without_errno() {
    // Exactly ULONG_MAX parses cleanly (no ERANGE) but still exceeds UINT_MAX.
    check1(b"18446744073709551615");
    check1(b"+18446744073709551615");
}

#[test]
fn invalid_non_utf8_arguments() {
    // The C program byte-copies argv[1] into its error message; the Rust one
    // must too, which rules out any lossy UTF-8 handling.
    for arg in [
        &b"\xff"[..],
        b"\xff\xfe",
        b"5\xff",
        b"\xffi5",
        b"\xc3\x28",
        b"\xe2\x82",           // truncated UTF-8
        b"\xd9\xa3",           // U+0663 ARABIC-INDIC DIGIT THREE
        b"\xef\xbc\x91",       // U+FF11 FULLWIDTH DIGIT ONE
    ] {
        check1(arg);
    }
}

#[test]
fn invalid_seed_message_echoes_argument_verbatim() {
    // Includes bytes that a naive formatter might mangle.
    for arg in [
        &b"%d"[..],
        b"%s",
        b"%n",
        b"{}",
        b"'quoted'",
        b"\x01\x02\x7f",
    ] {
        check1(arg);
    }
}

// ---------------------------------------------------------------------------
// Valid seeds. These exercise the whole 2000 x 256Ki x 100 workload, so they
// are slow; `cargo test` runs them in parallel.
// ---------------------------------------------------------------------------

#[test]
fn fullrun_seed_zero() {
    // srand(0): glibc substitutes 1 for a zero seed.
    check1(b"0");
}

#[test]
fn fullrun_empty_argument_is_seed_zero() {
    // "" -> strtoul converts nothing, leaves endptr at the NUL terminator, so
    // `*endptr == '\0'`, errno stays 0 and temp_seed is 0. The C program
    // therefore *accepts* the empty string. This looks like a bug; it is the
    // ground truth.
    check1(b"");
}

#[test]
fn fullrun_negative_zero_is_seed_zero() {
    // "-0" -> 0 negated modulo 2^64 is still 0, so it is accepted.
    check1(b"-0");
}

#[test]
fn fullrun_plus_zero_and_leading_zeros() {
    check1(b"+0");
}

#[test]
fn fullrun_seed_one() {
    check1(b"1");
}

#[test]
fn fullrun_leading_whitespace_and_plus() {
    // strtoul skips white space and an explicit '+', so this is seed 42.
    check1(b" \t\n+42");
}

#[test]
fn fullrun_leading_zeros() {
    // "007" is seed 7 in base 10 (no octal interpretation).
    check1(b"007");
}

#[test]
fn fullrun_seed_above_int_max() {
    // 2^31: the seed is stored into glibc's `int32_t state[0]`, i.e. negative.
    check1(b"2147483648");
}

#[test]
fn fullrun_seed_uint_max() {
    // The largest accepted value.
    check1(b"4294967295");
}

#[test]
fn fullrun_negative_wraps_back_into_range() {
    // -ULONG_MAX negated modulo 2^64 is 1, and glibc does *not* set ERANGE for
    // it, so this monstrosity is accepted and behaves exactly like seed 1.
    check1(b"-18446744073709551615");
}

#[test]
fn fullrun_negative_wraps_to_uint_max() {
    // -(2^64 - 2^32 + 1) mod 2^64 == 4294967295 == UINT_MAX: accepted.
    check1(b"-18446744069414584321");
}
