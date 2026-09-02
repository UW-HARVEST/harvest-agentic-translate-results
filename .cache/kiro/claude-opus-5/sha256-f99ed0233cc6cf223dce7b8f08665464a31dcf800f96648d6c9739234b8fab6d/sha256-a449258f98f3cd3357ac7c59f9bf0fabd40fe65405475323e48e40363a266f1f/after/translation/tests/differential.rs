// Differential tests: run the ORIGINAL C program and the Rust translation as
// subprocesses and require byte-identical stdout, byte-identical stderr and an
// identical exit status for every input.
//
// The Rust code is never linked as a library here -- only the built binary is
// driven, the way a shell would drive it.
//
// NOTE ON RUNTIME: the workload is 2000 * 262144 * 100 arithmetic steps, which
// takes ~8 minutes for the (unoptimised) C build and ~5 for Rust.  Every input
// that parses to a valid seed therefore costs a full run of both programs.
// Each such case is its own `#[test]` so libtest runs them in parallel; the
// two subprocesses inside one case are also spawned concurrently.  Expect the
// whole suite to take roughly 10-15 wall-clock minutes on a many-core box.
// Nothing is `#[ignore]`d.

use std::ffi::{OsStr, OsString};

use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Locating / building the two executables
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// The Rust binary under test (built by cargo before the test runs).
fn rust_bin() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// The C binary. Built once per test process with cmake if it is not present.
/// `c_src/` itself is never modified; only `c_src/build/` is created.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN
        .get_or_init(|| {
            let c_src = repo_root().join("c_src");
            let build = c_src.join("build");
            let exe = build.join("driver");
            if exe.is_file() {
                return exe;
            }
            std::fs::create_dir_all(&build).expect("create c_src/build");
            let cfg = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("cmake must be installed to build the C reference");
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
            assert!(exe.is_file(), "C driver not produced at {}", exe.display());
            exe
        })
        .as_path()
}

// ---------------------------------------------------------------------------
// Running and comparing
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    code: Option<i32>,
    signal: Option<i32>,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "code={:?} signal={:?}\n  stdout={:?}\n  stderr={:?}",
            self.code,
            self.signal,
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr)
        )
    }
}

/// `argv[0]` is forced to the same value for both programs, because the C
/// program prints `argv[0]` verbatim in its usage message; without this the
/// two binaries would legitimately differ only in their own path.
const DEFAULT_ARG0: &str = "driver";

fn spawn(bin: &Path, arg0: &OsStr, args: &[OsString]) -> std::process::Child {
    let mut cmd = Command::new(bin);
    cmd.arg0(arg0);
    for a in args {
        cmd.arg(a);
    }
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    cmd.spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", bin.display()))
}

fn collect(child: std::process::Child) -> Outcome {
    let out = child.wait_with_output().expect("wait_with_output");
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

fn describe(arg0: &OsStr, args: &[OsString]) -> String {
    let mut s = format!("argv0={:?}", arg0.as_bytes());
    for (i, a) in args.iter().enumerate() {
        let b = a.as_bytes();
        if b.len() > 64 {
            s += &format!(
                " argv{}=<{} bytes: {:?}...>",
                i + 1,
                b.len(),
                String::from_utf8_lossy(&b[..32])
            );
        } else {
            s += &format!(" argv{}={:?}", i + 1, String::from_utf8_lossy(b));
        }
    }
    s
}

/// Run both programs with the same argv and assert stdout, stderr and exit
/// status all match byte for byte.
fn assert_identical(arg0: &OsStr, args: &[OsString]) {
    // Spawn both at once so a slow case costs one run, not two.
    let c = spawn(c_bin(), arg0, args);
    let r = spawn(rust_bin(), arg0, args);
    let co = collect(c);
    let ro = collect(r);

    let ctx = describe(arg0, args);
    assert_eq!(
        co.stdout, ro.stdout,
        "STDOUT differs for {ctx}\n  C  ={:?}\n  RUST={:?}",
        String::from_utf8_lossy(&co.stdout),
        String::from_utf8_lossy(&ro.stdout)
    );
    assert_eq!(
        co.stderr, ro.stderr,
        "STDERR differs for {ctx}\n  C  ={:?}\n  RUST={:?}",
        String::from_utf8_lossy(&co.stderr),
        String::from_utf8_lossy(&ro.stderr)
    );
    assert_eq!(co.code, ro.code, "exit code differs for {ctx}");
    assert_eq!(co.signal, ro.signal, "termination signal differs for {ctx}");
    assert_eq!(co, ro, "outcome differs for {ctx}");
}

fn osb(bytes: &[u8]) -> OsString {
    OsString::from_vec(bytes.to_vec())
}

fn check(args: &[&[u8]]) {
    let v: Vec<OsString> = args.iter().map(|a| osb(a)).collect();
    assert_identical(OsStr::new(DEFAULT_ARG0), &v);
}

fn check_one(arg: &[u8]) {
    check(&[arg]);
}

// ===========================================================================
// Phase A -- both programs exist and are runnable
// ===========================================================================

#[test]
fn both_binaries_are_built_and_runnable() {
    let c = c_bin();
    let r = rust_bin();
    assert!(c.is_file(), "C binary missing: {}", c.display());
    assert!(r.is_file(), "Rust binary missing: {}", r.display());
    // A no-argument run is the cheap "does it start at all" probe.
    let out = collect(spawn(c, OsStr::new(DEFAULT_ARG0), &[]));
    assert_eq!(out.code, Some(1));
    let out = collect(spawn(r, OsStr::new(DEFAULT_ARG0), &[]));
    assert_eq!(out.code, Some(1));
}

// ===========================================================================
// argc branch:  `if (argc != 2)` -> usage on stderr, exit 1
// ===========================================================================

#[test]
fn argc_zero_extra_args_usage() {
    check(&[]);
}

#[test]
fn argc_two_extra_args_usage() {
    check(&[b"1", b"2"]);
}

#[test]
fn argc_three_extra_args_usage() {
    check(&[b"1", b"2", b"3"]);
}

#[test]
fn argc_many_extra_args_usage() {
    check(&[b"5", b"", b"x", b"-1", b"999999999999999999999"]);
}

#[test]
fn argc_extra_args_are_not_parsed_even_if_first_is_valid() {
    // Proves the argc check runs *before* any seed parsing.
    check(&[b"42", b"42"]);
}

/// The usage message embeds `argv[0]` verbatim; make sure both programs render
/// it the same way for awkward values.
#[test]
fn usage_message_echoes_argv0_verbatim() {
    for a0 in [
        &b""[..],
        b"driver",
        b"./driver",
        b"/usr/local/bin/driver",
        b"name with spaces",
        b"name\twith\ttabs",
        b"name\nwith\nnewlines",
        b"%s%d%n",            // printf-format bytes in argv[0]
        b"\xff\xfe non-utf8", // invalid UTF-8 in argv[0]
        b"a-very-long-argv0-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    ] {
        assert_identical(OsStr::from_bytes(a0), &[]);
        assert_identical(OsStr::from_bytes(a0), &[osb(b"x"), osb(b"y")]);
    }
}

// ---------------------------------------------------------------------------
// `argc == 0`.  A shell can never produce this, and `std::process::Command`
// always supplies an argv[0], so this one branch needs a raw execve with an
// empty argv.  It matters because the C program then does
// `fprintf(stderr, "Usage: %s\n", argv[0])` with argv[0] == NULL.
// ---------------------------------------------------------------------------

mod raw_exec {
    use super::*;
    use std::os::fd::AsRawFd;

    extern "C" {
        fn fork() -> i32;
        fn dup2(oldfd: i32, newfd: i32) -> i32;
        fn execve(
            path: *const std::ffi::c_char,
            argv: *const *const std::ffi::c_char,
            envp: *const *const std::ffi::c_char,
        ) -> i32;
        fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
        fn _exit(code: i32) -> !;
    }

    /// Exec `bin` with a completely empty argv (argc == 0), capturing its
    /// stdout/stderr into files.  Returns (stdout, stderr, raw wait status).
    fn run_with_empty_argv(bin: &Path, dir: &Path, tag: &str) -> Outcome {
        let out_path = dir.join(format!("{tag}.out"));
        let err_path = dir.join(format!("{tag}.err"));
        let out_file = std::fs::File::create(&out_path).expect("create stdout file");
        let err_file = std::fs::File::create(&err_path).expect("create stderr file");
        let path_c = std::ffi::CString::new(bin.as_os_str().as_bytes()).expect("no NUL in path");

        let (ofd, efd) = (out_file.as_raw_fd(), err_file.as_raw_fd());
        let pid = unsafe { fork() };
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // Child: only async-signal-safe calls from here on.
            unsafe {
                dup2(ofd, 1);
                dup2(efd, 2);
                let argv: [*const std::ffi::c_char; 1] = [std::ptr::null()];
                let envp: [*const std::ffi::c_char; 1] = [std::ptr::null()];
                execve(path_c.as_ptr(), argv.as_ptr(), envp.as_ptr());
                _exit(127);
            }
        }
        let mut status: i32 = -1;
        let w = unsafe { waitpid(pid, &mut status as *mut i32, 0) };
        assert_eq!(w, pid, "waitpid failed");
        drop(out_file);
        drop(err_file);

        let exited = (status & 0x7f) == 0;
        Outcome {
            stdout: std::fs::read(&out_path).expect("read stdout file"),
            stderr: std::fs::read(&err_path).expect("read stderr file"),
            code: if exited {
                Some((status >> 8) & 0xff)
            } else {
                None
            },
            signal: if exited { None } else { Some(status & 0x7f) },
        }
    }

    #[test]
    fn argc_zero_via_raw_execve() {
        let dir = std::env::temp_dir().join(format!(
            "driver-difftest-argc0-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("create temp dir");

        let co = run_with_empty_argv(c_bin(), &dir, "c");
        let ro = run_with_empty_argv(rust_bin(), &dir, "rs");

        assert_eq!(co.code, Some(1), "C should still exit 1 with argc == 0");
        assert_eq!(
            co.stdout, ro.stdout,
            "STDOUT differs (argc==0): C={:?} RUST={:?}",
            String::from_utf8_lossy(&co.stdout),
            String::from_utf8_lossy(&ro.stdout)
        );
        assert_eq!(
            co.stderr, ro.stderr,
            "STDERR differs (argc==0): C={:?} RUST={:?}",
            String::from_utf8_lossy(&co.stderr),
            String::from_utf8_lossy(&ro.stderr)
        );
        assert_eq!(co, ro, "outcome differs for argc == 0");

        let _ = std::fs::remove_dir_all(&dir);
    }
}

// ===========================================================================
// Phase B/C -- the seed-validation branch
//   `if (*endptr != '\0' || errno != 0 || temp_seed > UINT_MAX)`
// All of these exit 1 without running the workload, so they are fast.
// ===========================================================================

#[test]
fn invalid_seed_whitespace_only() {
    // isspace() is consumed by strtoul, then no digits => endptr == nptr,
    // so *endptr is the first space and the check fails.
    for s in [
        &b" "[..],
        b"  ",
        b"\t",
        b"\n",
        b"\x0b",
        b"\x0c",
        b"\r",
        b" \t\n\x0b\x0c\r ",
    ] {
        check_one(s);
    }
}

#[test]
fn invalid_seed_sign_without_digits() {
    for s in [
        &b"+"[..], b"-", b"++", b"--", b"+-", b"-+", b"+ 1", b"- 1", b"--5", b"++5",
    ] {
        check_one(s);
    }
}

#[test]
fn invalid_seed_negative_values_wrap_above_uint_max() {
    // strtoul negates modulo 2^64, so "-1" becomes ULONG_MAX > UINT_MAX.
    for s in [
        &b"-1"[..],
        b"-2",
        b"-42",
        b"-4294967295",
        b"-4294967296",
        b"-18446744073709551616",
    ] {
        check_one(s);
    }
}

#[test]
fn invalid_seed_just_above_uint_max() {
    for s in [
        &b"4294967296"[..],
        b"4294967297",
        b"10000000000",
        b"1000000000000000000000",
    ] {
        check_one(s);
    }
}

#[test]
fn invalid_seed_ulong_range_but_above_uint_max() {
    // Fits in unsigned long (no ERANGE) yet still exceeds UINT_MAX.
    for s in [
        &b"18446744073709551614"[..],
        b"18446744073709551615", // exactly ULONG_MAX
        b"99999999999999999999",
        b"1111111111111111111",
        b"11111111111111111111",
    ] {
        check_one(s);
    }
}

#[test]
fn invalid_seed_erange_overflow() {
    // Overflows unsigned long => strtoul sets ERANGE and returns ULONG_MAX.
    for s in [
        &b"18446744073709551616"[..],
        b"18446744073709551617",
        b"184467440737095516150",
        b"999999999999999999999",
        b"99999999999999999999999999",
        b"-99999999999999999999999999",
    ] {
        check_one(s);
    }
}

#[test]
fn invalid_seed_erange_overflow_very_long_digit_strings() {
    let long9_100 = vec![b'9'; 100];
    let long9_5000 = vec![b'9'; 5000];
    let mut neg9_100 = vec![b'-'];
    neg9_100.extend(std::iter::repeat(b'9').take(100));
    check_one(&long9_100);
    check_one(&long9_5000);
    check_one(&neg9_100);
}

#[test]
fn invalid_seed_trailing_garbage() {
    for s in [
        &b"abc"[..],
        b"x",
        b"0x10",
        b"0X10",
        b"12x",
        b"x12",
        b"1 2",
        b"1,2",
        b"1.5",
        b"1e5",
        b"0a",
        b"9a",
        b"5-",
        b"5+",
    ] {
        check_one(s);
    }
}

#[test]
fn invalid_seed_trailing_whitespace() {
    // Leading whitespace is fine, trailing whitespace is not: *endptr != '\0'.
    for s in [&b"5 "[..], b" 5 ", b"5\t", b"5\n", b"5\r", b"-0 ", b"4294967295 "] {
        check_one(s);
    }
}

#[test]
fn invalid_seed_non_ascii_bytes() {
    for s in [
        &b"\xff"[..],
        b"\x80",
        b"\xc2\xb2", // U+00B2 SUPERSCRIPT TWO -- not an ASCII digit
        b"1\xff",
        b"\xff1",
        b"1\x7f",
        b"\xef\xbc\x91", // U+FF11 FULLWIDTH DIGIT ONE
    ] {
        check_one(s);
    }
}

#[test]
fn invalid_seed_embedded_newline_before_digits_is_ok_but_after_is_not() {
    check_one(b"+\n1"); // sign then whitespace => no digits => error
    check_one(b"1\n+"); // trailing garbage
}

#[test]
fn invalid_seed_zero_padded_above_uint_max() {
    check_one(b"+000000000000000000000000000000000000000000000000004294967296");
}

#[test]
fn invalid_seed_very_long_argument() {
    // 100 000 digits: exercises ERANGE plus a huge error message.  (Linux caps
    // a single argv entry at MAX_ARG_STRLEN = 128 KiB, so stay under that --
    // above it neither program can even be exec'd.)
    let s = vec![b'7'; 100_000];
    check_one(&s);
}

#[test]
fn invalid_seed_error_message_quotes_argument_verbatim() {
    // The message is  Invalid seed: '%s'  -- check awkward payloads round-trip.
    for s in [
        &b"%s%d%n"[..],
        b"'quoted'",
        b"tab\there",
        b"nl\nhere",
        b"\x01\x02\x03",
    ] {
        check_one(s);
    }
}

// ===========================================================================
// Phase B/C -- the accepting path.  Each of these runs the full workload in
// both programs, so each is its own test to get libtest parallelism.
// ===========================================================================

/// Grouped in a module so the expensive cases share the `full_run::` name
/// prefix (handy for `cargo test full_run::` / `--skip full_run::`).
mod full_run {
    use super::*;

macro_rules! seed_case {
    ($name:ident, $arg:expr) => {
        #[test]
        fn $name() {
            check_one($arg);
        }
    };
}

// --- seed 0 (note: glibc's srand maps 0 -> 1, so these equal seed 1) -------
// Empty argument: strtoul performs no conversion, so endptr == nptr, *endptr
// is the terminating NUL, errno stays 0 and temp_seed is 0.  Accepted.
seed_case!(seed_empty_string_is_accepted_as_zero, b"");
seed_case!(seed_zero, b"0");
seed_case!(seed_zero_padded, b"000");
seed_case!(seed_plus_zero, b"+0");
seed_case!(seed_minus_zero, b"-0");
seed_case!(seed_minus_zero_with_leading_space, b" -0");

// --- seed 1, including the wrap-around route -------------------------------
seed_case!(seed_one, b"1");
seed_case!(seed_plus_one, b"+1");
// -(2^64 - 1) mod 2^64 == 1: a negative literal that lands on a valid seed.
seed_case!(seed_negative_wraps_to_one, b"-18446744073709551615");
seed_case!(seed_leading_newline_plus_one, b"\n+1");

// --- small seeds -----------------------------------------------------------
seed_case!(seed_two, b"2");
seed_case!(seed_five, b"5");
seed_case!(seed_five_leading_space, b" 5");
seed_case!(seed_five_leading_tab, b"\t5");
seed_case!(seed_plus_five, b"+5");
seed_case!(seed_fortytwo, b"42");
seed_case!(seed_fortytwo_plus_zero_padded, b"+000000042");
seed_case!(seed_fortytwo_many_leading_zeros, b"0000000000000000000000000042");

// base 10, not octal: "010" is ten, "07"/"08"/"09" are seven/eight/nine
seed_case!(seed_010_is_decimal_ten, b"010");
seed_case!(seed_0010_is_decimal_ten, b"0010");
seed_case!(seed_07_is_seven, b"07");
seed_case!(seed_08_is_eight, b"08");
seed_case!(seed_09_is_nine, b"09");
seed_case!(seed_seven_leading_spaces, b"   7");

// every character isspace() accepts, as a prefix
seed_case!(seed_nine_mixed_whitespace_prefix, b"\t\n 9");
seed_case!(seed_nine_vtab_formfeed_prefix, b"\x0b\x0c9");

// --- the signed/unsigned boundary of the seed ------------------------------
seed_case!(seed_int32_max, b"2147483647");
// From here on `(int32_t) seed` is negative inside glibc's srand.
seed_case!(seed_int32_max_plus_one, b"2147483648");
seed_case!(seed_int32_max_plus_two, b"2147483649");
seed_case!(seed_three_billion, b"3000000000");
seed_case!(seed_four_billion, b"4000000000");
seed_case!(seed_uint_max_minus_one, b"4294967294");
// The largest accepted value: temp_seed == UINT_MAX exactly.
seed_case!(seed_uint_max, b"4294967295");
seed_case!(seed_uint_max_leading_space, b" 4294967295");
seed_case!(
    seed_uint_max_zero_padded,
    b"+000000000000000000000000000000000000000000000000004294967295"
);

// --- long-but-valid arguments ---------------------------------------------
#[test]
fn seed_hundred_leading_zeros_then_fortytwo() {
    let mut s = vec![b'0'; 100];
    s.extend_from_slice(b"42");
    check_one(&s);
}

#[test]
fn seed_five_thousand_leading_zeros_then_one() {
    let mut s = vec![b'0'; 5000];
    s.push(b'1');
    check_one(&s);
}

/// The C program starts with the default `SIGPIPE` disposition, so if nothing
/// is left reading stdout when it finally calls `printf`, it is killed by
/// signal 13 (wait status 141) rather than exiting 0.  The Rust runtime
/// installs `SIG_IGN` instead, so the translation has to restore `SIG_DFL`.
/// Only the accepting path writes to stdout, so this needs a full run.
#[test]
fn stdout_reader_gone_kills_both_the_same_way() {
    use std::os::fd::{FromRawFd, OwnedFd};

    extern "C" {
        fn pipe2(fds: *mut i32, flags: i32) -> i32;
    }

    fn spawn_with_dead_reader(bin: &Path) -> std::process::Child {
        const O_CLOEXEC: i32 = 0o2000000;
        let mut fds = [-1i32; 2];
        // O_CLOEXEC matters: with a plain pipe(2) the child would *inherit* the
        // read end, keep the pipe alive and never see EPIPE at all.
        assert_eq!(
            unsafe { pipe2(fds.as_mut_ptr(), O_CLOEXEC) },
            0,
            "pipe2() failed"
        );
        // SAFETY: both fds come straight from a successful pipe2(2).
        let read_end = unsafe { OwnedFd::from_raw_fd(fds[0]) };
        let write_end = unsafe { OwnedFd::from_raw_fd(fds[1]) };

        let child = Command::new(bin)
            .arg0(DEFAULT_ARG0)
            .arg("5")
            .stdin(Stdio::null())
            // dup2'd onto fd 1 in the child, which clears O_CLOEXEC.
            .stdout(Stdio::from(write_end))
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("spawn {}: {e}", bin.display()));

        drop(read_end); // no reader remains; the eventual write must fail
        child
    }

    let c = spawn_with_dead_reader(c_bin());
    let r = spawn_with_dead_reader(rust_bin());
    let co = collect(c);
    let ro = collect(r);

    assert_eq!(
        co.signal,
        Some(13),
        "expected the C program to be killed by SIGPIPE, got {co:?}"
    );
    assert_eq!(co.signal, ro.signal, "termination signal differs:\n  C  ={co:?}\n  RUST={ro:?}");
    assert_eq!(co.code, ro.code, "exit code differs:\n  C  ={co:?}\n  RUST={ro:?}");
    assert_eq!(co.stderr, ro.stderr, "stderr differs:\n  C  ={co:?}\n  RUST={ro:?}");
    assert_eq!(co, ro, "outcome differs when the stdout reader is gone");
}

} // mod full_run

// ===========================================================================
// Sanity: the harness itself must be able to see a difference.
// (Runs no program; guards against assert_identical being vacuous.)
// ===========================================================================

#[test]
fn harness_outcome_compares_all_three_channels() {
    let base = Outcome {
        stdout: b"1\n".to_vec(),
        stderr: Vec::new(),
        code: Some(0),
        signal: None,
    };
    let diff_code = Outcome {
        code: Some(1),
        ..Outcome {
            stdout: base.stdout.clone(),
            stderr: base.stderr.clone(),
            code: base.code,
            signal: base.signal,
        }
    };
    let diff_stdout = Outcome {
        stdout: b"2\n".to_vec(),
        stderr: Vec::new(),
        code: Some(0),
        signal: None,
    };
    let diff_stderr = Outcome {
        stdout: b"1\n".to_vec(),
        stderr: b"x".to_vec(),
        code: Some(0),
        signal: None,
    };
    assert_ne!(base, diff_code, "equality must compare the exit code");
    assert_ne!(base, diff_stdout, "equality must compare stdout");
    assert_ne!(base, diff_stderr, "equality must compare stderr");
}
