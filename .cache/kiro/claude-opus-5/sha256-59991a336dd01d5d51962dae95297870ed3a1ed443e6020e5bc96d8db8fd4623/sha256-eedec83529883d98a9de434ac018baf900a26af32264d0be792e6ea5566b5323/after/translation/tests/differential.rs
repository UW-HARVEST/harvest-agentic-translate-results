//! Differential tests: run the C reference binary and the Rust binary as
//! subprocesses with identical `argv`, then compare stdout, stderr and exit
//! status byte for byte.
//!
//! Nothing here links against the Rust crate as a library; both programs are
//! driven exactly the way a shell drives them.

use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Locating / building the two binaries
// ---------------------------------------------------------------------------

/// Path to the Rust binary that cargo just built for us.
fn rust_binary() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

fn repo_root() -> PathBuf {
    // tests/ live in translation/tests, so the workspace root is one level up.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the compiled C reference binary, configuring and building it with
/// CMake on first use so that `cargo test` is self-contained.
fn c_binary() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = repo_root().join("c_src");
        let build_dir = c_src.join("build");
        let exe = build_dir.join("driver");

        if !exe.exists() {
            std::fs::create_dir_all(&build_dir).expect("cannot create c_src/build");

            let configure = Command::new("cmake")
                .arg("..")
                .current_dir(&build_dir)
                .output()
                .expect("failed to run `cmake ..` -- is cmake installed?");
            assert!(
                configure.status.success(),
                "cmake configure failed:\n{}\n{}",
                String::from_utf8_lossy(&configure.stdout),
                String::from_utf8_lossy(&configure.stderr)
            );

            let build = Command::new("cmake")
                .args(["--build", "."])
                .current_dir(&build_dir)
                .output()
                .expect("failed to run `cmake --build .`");
            assert!(
                build.status.success(),
                "cmake build failed:\n{}\n{}",
                String::from_utf8_lossy(&build.stdout),
                String::from_utf8_lossy(&build.stderr)
            );
        }

        assert!(
            exe.exists(),
            "the C reference binary was not produced at {}",
            exe.display()
        );
        exe
    })
}

// ---------------------------------------------------------------------------
// Running and comparing
// ---------------------------------------------------------------------------

/// Everything observable about one run of a program.
#[derive(PartialEq, Eq)]
struct Observed {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Some(code)` for a normal exit.
    code: Option<i32>,
    /// `Some(signum)` when the process was killed by a signal.
    signal: Option<i32>,
}

impl std::fmt::Debug for Observed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Observed")
            .field("stdout", &String::from_utf8_lossy(&self.stdout))
            .field("stdout_bytes", &self.stdout)
            .field("stderr", &String::from_utf8_lossy(&self.stderr))
            .field("stderr_bytes", &self.stderr)
            .field("code", &self.code)
            .field("signal", &self.signal)
            .finish()
    }
}

fn run(exe: &Path, args: &[OsString]) -> Observed {
    let out = Command::new(exe)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));
    Observed {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

fn to_args<S: AsRef<[u8]>>(args: &[S]) -> Vec<OsString> {
    args.iter()
        .map(|a| OsStr::from_bytes(a.as_ref()).to_os_string())
        .collect()
}

fn render(args: &[OsString]) -> String {
    args.iter()
        .map(|a| format!("{:?}", String::from_utf8_lossy(a.as_bytes())))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Assert that both programs behave identically for one `argv` vector.
/// Compares stdout, stderr and exit status (including death-by-signal).
fn assert_same<S: AsRef<[u8]>>(args: &[S]) {
    let args = to_args(args);
    let c = run(c_binary(), &args);
    let r = run(rust_binary(), &args);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch for argv [{}]\n  C: {:?}\n  Rust: {:?}",
        render(&args),
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch for argv [{}]\n  C: {:?}\n  Rust: {:?}",
        render(&args),
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "exit status mismatch for argv [{}]\n  C: {:?}\n  Rust: {:?}",
        render(&args),
        c,
        r
    );
}

fn assert_all(cases: &[&[&str]]) {
    for case in cases {
        assert_same(case);
    }
}

/// Run a batch of `argv` vectors in parallel. Each case spawns two processes,
/// so the sweeps below are entirely I/O bound; fanning them out across threads
/// keeps `cargo test` fast even on a loaded machine.
fn assert_all_parallel(cases: Vec<Vec<String>>) {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    // Warm the OnceLock (and any CMake build) before the threads start.
    let _ = c_binary();

    let cases = Arc::new(cases);
    let next = Arc::new(AtomicUsize::new(0));
    let failures: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

    let workers = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .clamp(2, 16);

    let mut handles = Vec::new();
    for _ in 0..workers {
        let cases = Arc::clone(&cases);
        let next = Arc::clone(&next);
        let failures = Arc::clone(&failures);
        handles.push(std::thread::spawn(move || loop {
            let i = next.fetch_add(1, Ordering::Relaxed);
            if i >= cases.len() {
                break;
            }
            let argv = to_args(&cases[i]);
            let c = run(c_binary(), &argv);
            let r = run(rust_binary(), &argv);
            if c != r {
                failures.lock().unwrap().push(format!(
                    "argv [{}]\n    C:    {:?}\n    Rust: {:?}",
                    render(&argv),
                    c,
                    r
                ));
            }
        }));
    }
    for h in handles {
        h.join().expect("worker thread panicked");
    }

    let failures = failures.lock().unwrap();
    assert!(
        failures.is_empty(),
        "{} of {} cases mismatched:\n  {}",
        failures.len(),
        cases.len(),
        failures.join("\n  ")
    );
}

// ===========================================================================
// Phase A sanity: both binaries exist and are runnable
// ===========================================================================

#[test]
fn both_binaries_are_runnable() {
    // `driver` with no arguments takes the argc==1 error path in both programs.
    let args: Vec<OsString> = Vec::new();
    let c = run(c_binary(), &args);
    let r = run(rust_binary(), &args);
    assert_eq!(c.code, Some(1), "C binary did not run as expected: {c:?}");
    assert_eq!(r.code, Some(1), "Rust binary did not run as expected: {r:?}");
    assert!(!c.stdout.is_empty());
    assert_eq!(c, r);
}

// ===========================================================================
// argc gate: `(argc > 4) || (argc == 1)`
// ===========================================================================

#[test]
fn argc_1_no_arguments_is_a_usage_error() {
    assert_same::<&str>(&[]);
}

#[test]
fn argc_greater_than_4_is_a_usage_error() {
    assert_all(&[
        &["a", "b", "c", "d"],
        &["a", "b", "c", "d", "e"],
        &["hello", "0", "5", "extra"],
        &["hello", "0", "5", "extra", "more", "still-more"],
        // Valid-looking leading arguments still lose to the argc gate.
        &["hello", "1", "3", ""],
    ]);
}

#[test]
fn argc_gate_is_checked_before_anything_else() {
    // A bogus start plus too many args must report the usage error, not the
    // "must be an integer" error: the argc check comes first in the C.
    assert_all(&[
        &["hello", "not-a-number", "also-not", "surplus"],
        &["", "", "", ""],
    ]);
}

// ===========================================================================
// argc == 2: string only, `stop = len`
// ===========================================================================

#[test]
fn argc_2_prints_the_whole_string() {
    assert_all(&[
        &["hello"],
        &["a"],
        &["abcdefghij"],
        &["a b c"],
        &["0"],
        &["-1"],
        &["--help"],
        &["-x"],
        &[" leading and trailing "],
        &["tab\there"],
        &["newline\nhere"],
    ]);
}

#[test]
fn argc_2_empty_string_prints_just_a_newline() {
    assert_same(&[""]);
}

// ===========================================================================
// argc == 3: start parsing and the `start > len` check
// ===========================================================================

#[test]
fn start_zero_prints_the_whole_string() {
    assert_all(&[&["hello", "0"], &["", "0"], &["a", "0"], &["hello", "+0"], &["hello", "-0"]]);
}

#[test]
fn start_in_range_skips_that_many_bytes() {
    assert_all(&[
        &["hello", "1"],
        &["hello", "2"],
        &["hello", "3"],
        &["hello", "4"],
        &["abcdefghij", "7"],
    ]);
}

#[test]
fn start_equal_to_len_prints_the_empty_substring() {
    // Boundary: `start > len` is false when start == len, so this succeeds and
    // prints an empty substring plus the newline.
    assert_all(&[&["hello", "5"], &["a", "1"], &["abcdefghij", "10"]]);
}

#[test]
fn start_one_past_len_is_off_the_end() {
    assert_all(&[&["hello", "6"], &["a", "2"], &["", "1"], &["abcdefghij", "11"]]);
}

#[test]
fn negative_start_is_reported_as_off_the_end() {
    // C QUIRK: `start > len` promotes the int to size_t, so every negative
    // start becomes a huge unsigned value and is rejected here rather than
    // being treated as an index from the end.
    assert_all(&[
        &["hello", "-1"],
        &["hello", "-2"],
        &["hello", "-5"],
        &["hello", "-100"],
        &["", "-1"],
    ]);
}

#[test]
fn non_numeric_start_is_rejected_without_a_trailing_newline() {
    // C QUIRK: this printf has no "\n", unlike every other error message.
    assert_all(&[
        &["hello", "abc"],
        &["hello", ""],
        &["hello", " "],
        &["hello", "   "],
        &["hello", "+"],
        &["hello", "-"],
        &["hello", "--3"],
        &["hello", "- 3"],
        &["hello", "+ 3"],
        &["hello", "."],
        &["hello", "."],
        &["hello", "--start"],
        &["hello", "\t"],
        &["hello", "\n"],
        &["hello", "abc123"],
        &["hello", "x1"],
    ]);
}

#[test]
fn start_accepts_a_partial_numeric_prefix() {
    // strtol stops at the first non-digit; `end != argv[2]` so this is valid.
    assert_all(&[
        &["hello", "2abc"],
        &["hello", "3.7"],
        &["hello", "1e3"],
        &["hello", "0x2"],
        &["hello", "0b1"],
        &["hello", "2 "],
        &["hello", "4-"],
        &["hello", "010"],
    ]);
}

#[test]
fn start_honours_c_locale_whitespace_and_sign() {
    assert_all(&[
        &["hello", " 3"],
        &["hello", "\t3"],
        &["hello", "\n3"],
        &["hello", "\x0b3"],
        &["hello", "\x0c3"],
        &["hello", "\r3"],
        &["hello", "   +3"],
        &["hello", " \t\n 2"],
        &["hello", "+2"],
        &["hello", "00000000000000000000003"],
    ]);
}

#[test]
fn non_ascii_digits_are_not_digits_in_the_c_locale() {
    assert_all(&[
        &["hello", "\u{0663}"],       // Arabic-Indic three
        &["hello", "\u{ff13}"],       // fullwidth three
        &["hello", "\u{0661}\u{0662}"],
    ]);
}

#[test]
fn start_is_truncated_from_long_to_int() {
    // strtol returns a long; assigning it to `int start` truncates.
    assert_all(&[
        // 4294967296 == 2^32 -> truncates to 0
        &["hello", "4294967296"],
        // 4294967298 == 2^32 + 2 -> truncates to 2
        &["hello", "4294967298"],
        // -4294967296 -> truncates to 0
        &["hello", "-4294967296"],
        // 2^31 -> INT_MIN -> rejected as off the end
        &["hello", "2147483648"],
        &["hello", "2147483647"],
        &["hello", "-2147483648"],
        &["hello", "-2147483649"],
        &["hello", "4294967295"],
    ]);
}

#[test]
fn start_saturates_on_strtol_overflow() {
    assert_all(&[
        &["hello", "9223372036854775807"],   // LONG_MAX
        &["hello", "9223372036854775808"],   // ERANGE -> LONG_MAX
        &["hello", "-9223372036854775808"],  // LONG_MIN
        &["hello", "-9223372036854775809"],  // ERANGE -> LONG_MIN
        &["hello", "99999999999999999999"],
        &["hello", "-99999999999999999999"],
        &["hello", "1000000000000000000000000000000"],
    ]);
}

// ===========================================================================
// argc == 4: stop parsing, `stop > len`, `stop <= start`
// ===========================================================================

#[test]
fn stop_in_range_prints_the_half_open_slice() {
    assert_all(&[
        &["hello", "0", "5"],
        &["hello", "0", "1"],
        &["hello", "1", "3"],
        &["hello", "2", "5"],
        &["hello", "4", "5"],
        &["abcdefghij", "3", "8"],
    ]);
}

#[test]
fn stop_off_the_end_is_rejected() {
    assert_all(&[
        &["hello", "0", "6"],
        &["hello", "0", "100"],
        &["a", "0", "2"],
        &["abcdefghij", "0", "11"],
    ]);
}

#[test]
fn negative_stop_is_reported_as_off_the_end() {
    // Same signed/unsigned promotion quirk as `start`.
    assert_all(&[
        &["hello", "0", "-1"],
        &["hello", "0", "-5"],
        &["hello", "2", "-1"],
        &["hello", "0", "-2147483648"],
    ]);
}

#[test]
fn stop_not_after_start_is_rejected() {
    assert_all(&[
        &["hello", "2", "2"],
        &["hello", "3", "1"],
        &["hello", "5", "5"],
        &["hello", "0", "0"],
        &["", "0", "0"],
        &["hello", "4", "2"],
    ]);
}

#[test]
fn stop_check_order_is_off_the_end_before_after_start() {
    // stop == 6 is both off the end and greater than start, and stop == -1 is
    // "off the end" (unsigned) even though it is also <= start. The off-the-end
    // check runs first in the C, so it wins.
    assert_all(&[&["hello", "3", "6"], &["hello", "3", "-1"], &["hello", "0", "-1"]]);
}

#[test]
fn non_numeric_stop_silently_becomes_zero() {
    // C QUIRK: the third-argument check compares `end` (which strtol left
    // pointing into argv[2], because NULL was passed as endptr here) against
    // argv[3]. Two distinct argv strings never share an address, so the
    // "Third argument must be an integer!" branch is unreachable and a
    // non-numeric stop just yields 0 -- which then trips `stop <= start`.
    assert_all(&[
        &["hello", "0", "abc"],
        &["hello", "0", ""],
        &["hello", "0", " "],
        &["hello", "0", "+"],
        &["hello", "0", "-"],
        &["hello", "0", "."],
        &["hello", "0", "\u{0663}"],
        &["hello", "2", "xyz"],
        &["", "0", ""],
        &["", "", ""],
    ]);
}

#[test]
fn stop_accepts_a_partial_numeric_prefix() {
    assert_all(&[
        &["hello", "0", "3abc"],
        &["hello", "0", "3.7"],
        &["hello", "0", "0x5"],
        &["hello", "1", " 4"],
        &["hello", "1", "\t4"],
        &["hello", "1", "+4"],
        &["hello", "0", "005"],
    ]);
}

#[test]
fn stop_is_truncated_from_long_to_int() {
    assert_all(&[
        // 2^32 + 5 -> 5
        &["hello", "0", "4294967301"],
        // -4294967291 -> 5
        &["hello", "0", "-4294967291"],
        // 2^32 -> 0 -> `stop <= start`
        &["hello", "0", "4294967296"],
        &["hello", "2", "4294967296"],
        &["hello", "0", "2147483648"],
        &["hello", "0", "-2147483648"],
        // LONG_MIN truncates to 0
        &["hello", "0", "-9223372036854775808"],
        &["hello", "0", "9223372036854775807"],
        &["hello", "0", "99999999999999999999"],
    ]);
}

#[test]
fn both_arguments_truncate_together() {
    assert_all(&[
        &["hello", "4294967298", "4294967301"],
        &["hello", "-4294967296", "4294967301"],
        &["abcdefghij", "4294967296", "4294967306"],
    ]);
}

#[test]
fn bad_start_is_reported_before_stop_is_examined() {
    // The second-argument checks all run before any third-argument work.
    assert_all(&[
        &["hello", "abc", "3"],
        &["hello", "abc", "abc"],
        &["hello", "9", "3"],
        &["hello", "-1", "3"],
        &["hello", "", "0"],
    ]);
}

// ===========================================================================
// Byte-level output behaviour
// ===========================================================================

#[test]
fn non_utf8_arguments_round_trip_as_raw_bytes() {
    assert_same(&[b"\xff\xfe\xfd".as_slice()]);
    assert_same(&[b"\xff\xfe\xfd".as_slice(), b"0".as_slice()]);
    assert_same(&[b"\xff\xfe\xfd".as_slice(), b"1".as_slice(), b"3".as_slice()]);
    assert_same(&[b"\xc3".as_slice(), b"0".as_slice(), b"1".as_slice()]);
    assert_same(&[b"\x80\x80\x80\x80".as_slice(), b"2".as_slice()]);
    assert_same(&[b"a\xffb".as_slice(), b"1".as_slice(), b"2".as_slice()]);
    // Non-UTF8 numeric arguments too.
    assert_same(&[b"hello".as_slice(), b"\xff".as_slice()]);
    assert_same(&[b"hello".as_slice(), b"1\xff".as_slice()]);
    assert_same(&[b"hello".as_slice(), b"0".as_slice(), b"\xff".as_slice()]);
}

#[test]
fn multibyte_characters_are_cut_by_byte_not_by_character() {
    // "héllo" is 6 bytes; slicing at 1..2 yields half of the 2-byte 'é'.
    assert_all(&[
        &["héllo", "0"],
        &["héllo", "1"],
        &["héllo", "1", "2"],
        &["héllo", "0", "3"],
        &["héllo", "0", "6"],
        &["héllo", "0", "7"],
        &["héllo", "6"],
        &["日本語", "0", "4"],
        &["日本語", "3"],
    ]);
}

#[test]
fn long_string_boundaries() {
    let s = "a".repeat(300);
    let n = s.len().to_string();
    let n_plus = (s.len() + 1).to_string();
    let n_minus = (s.len() - 1).to_string();
    assert_same(&[s.as_str(), n_minus.as_str()]);
    assert_same(&[s.as_str(), n.as_str()]);
    assert_same(&[s.as_str(), n_plus.as_str()]);
    assert_same(&[s.as_str(), "0", n.as_str()]);
    assert_same(&[s.as_str(), "0", n_plus.as_str()]);
    assert_same(&[s.as_str(), "150", n.as_str()]);

    // A string long enough to exceed stdio's internal buffer, so the C program
    // performs several write() calls.
    let big = "z".repeat(70000);
    let bn = big.len().to_string();
    assert_same(&[big.as_str()]);
    assert_same(&[big.as_str(), "1", bn.as_str()]);
}

// ===========================================================================
// Exhaustive matrix over the interesting argument values
// ===========================================================================

const NUMS: &[&str] = &[
    "",
    "0",
    "1",
    "-1",
    "+1",
    "5",
    "6",
    "-0",
    "abc",
    "2abc",
    " 3",
    "\t3",
    "3 ",
    "3.7",
    ".",
    "+",
    "-",
    "--3",
    "0x2",
    "010",
    "  ",
    "\n2",
    "2147483647",
    "2147483648",
    "-2147483648",
    "-2147483649",
    "4294967295",
    "4294967296",
    "4294967297",
    "4294967301",
    "9223372036854775807",
    "9223372036854775808",
    "-9223372036854775808",
    "-9223372036854775809",
    "99999999999999999999",
    "-99999999999999999999",
    "\u{0663}",
    "1e3",
];

const STRS: &[&str] = &["", "a", "hi", "hello", "héllo", "abcdefghij", "0", "-1", "\t"];

#[test]
fn matrix_argc_2_and_3() {
    let mut cases: Vec<Vec<String>> = Vec::new();
    for s in STRS {
        cases.push(vec![s.to_string()]);
        for n in NUMS {
            cases.push(vec![s.to_string(), n.to_string()]);
        }
    }
    assert_all_parallel(cases);
}

#[test]
fn matrix_argc_4() {
    let mut cases: Vec<Vec<String>> = Vec::new();
    for s in ["", "a", "hello", "abcdefghij"] {
        for n1 in NUMS {
            for n2 in NUMS {
                cases.push(vec![s.to_string(), n1.to_string(), n2.to_string()]);
            }
        }
    }
    assert_all_parallel(cases);
}

// ===========================================================================
// Process-level behaviour that is not driven by argv
// ===========================================================================

#[test]
fn dying_on_a_closed_stdout_matches() {
    use std::io::Write;
    use std::os::fd::OwnedFd;
    use std::os::unix::net::UnixStream;
    use std::process::Stdio;

    // A socketpair with the peer dropped makes every write fail with EPIPE and
    // raise SIGPIPE. The C program keeps the default disposition and dies; the
    // Rust program must do the same rather than exiting 0.
    let big = "q".repeat(70000);

    let observe = |exe: &Path| -> Observed {
        let (mine, theirs) = UnixStream::pair().expect("socketpair");
        let theirs: OwnedFd = theirs.into();
        let mut child = Command::new(exe)
            .arg(&big)
            .stdout(Stdio::from(theirs))
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn");
        // Drop the reading end so the child's writes cannot succeed.
        drop(mine);
        let mut stderr = Vec::new();
        if let Some(mut e) = child.stderr.take() {
            let _ = std::io::Read::read_to_end(&mut e, &mut stderr);
        }
        let status = child.wait().expect("wait");
        let _ = std::io::stdout().flush();
        Observed {
            stdout: Vec::new(),
            stderr,
            code: status.code(),
            signal: status.signal(),
        }
    };

    let c = observe(c_binary());
    let r = observe(rust_binary());
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "closed-stdout exit status mismatch\n  C: {c:?}\n  Rust: {r:?}"
    );
    assert_eq!(c.stderr, r.stderr, "closed-stdout stderr mismatch");
}

#[test]
fn many_surplus_arguments_still_hit_the_usage_error() {
    let args: Vec<&str> = std::iter::repeat("x").take(64).collect();
    assert_same(&args);
}
