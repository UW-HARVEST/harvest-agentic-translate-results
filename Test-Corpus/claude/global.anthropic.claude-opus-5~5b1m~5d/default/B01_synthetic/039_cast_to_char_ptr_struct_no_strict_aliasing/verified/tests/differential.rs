//! Differential tests: run the original C binary and the Rust binary as
//! subprocesses with identical stdin, and require byte-identical stdout,
//! byte-identical stderr and an identical exit status.
//!
//! Nothing here links against the Rust crate as a library; both programs are
//! driven exactly the way a shell would drive them, because that is how the
//! translation is graded.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Locating / building the two executables
// ---------------------------------------------------------------------------

/// Root of the checkout (the directory that holds `c_src/` and `translation/`).
fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// The Rust binary under test, built by cargo for us.
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// The reference C binary. Built with cmake on first use if it is missing.
fn c_bin() -> &'static PathBuf {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = repo_root().join("c_src");
        let build_dir = c_src.join("build");
        let exe = build_dir.join("driver");
        if exe.is_file() {
            return exe;
        }

        std::fs::create_dir_all(&build_dir).expect("create c_src/build");

        let configure = Command::new("cmake")
            .arg("..")
            .current_dir(&build_dir)
            .output()
            .expect("failed to spawn `cmake ..` -- is cmake installed?");
        assert!(
            configure.status.success(),
            "cmake configure failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&configure.stdout),
            String::from_utf8_lossy(&configure.stderr)
        );

        let build = Command::new("cmake")
            .args(["--build", "."])
            .current_dir(&build_dir)
            .output()
            .expect("failed to spawn `cmake --build .`");
        assert!(
            build.status.success(),
            "cmake build failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&build.stdout),
            String::from_utf8_lossy(&build.stderr)
        );

        assert!(
            exe.is_file(),
            "expected the C reference binary at {}",
            exe.display()
        );
        exe
    })
}

// ---------------------------------------------------------------------------
// Running one program
// ---------------------------------------------------------------------------

struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Some(code)` for a normal exit, `None` if killed by a signal.
    code: Option<i32>,
}

fn run(exe: &Path, stdin_bytes: &[u8]) -> Outcome {
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));

    {
        let mut sink = child.stdin.take().expect("piped stdin");
        // The program may exit without draining stdin (large inputs); a broken
        // pipe here is not a test failure.
        let _ = sink.write_all(stdin_bytes);
        let _ = sink.flush();
    }

    let out = child.wait_with_output().expect("wait_with_output");
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
    }
}

fn show(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes.iter().take(200) {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    if bytes.len() > 200 {
        s.push_str(&format!("... (+{} bytes)", bytes.len() - 200));
    }
    s
}

/// The core assertion: same stdin => same stdout, same stderr, same status.
#[track_caller]
fn assert_same(label: &str, stdin_bytes: &[u8]) {
    let c = run(c_bin(), stdin_bytes);
    let r = run(&rust_bin(), stdin_bytes);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch for {label}\n  stdin: {}\n  C   : {}\n  Rust: {}",
        show(stdin_bytes),
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch for {label}\n  stdin: {}\n  C   : {}\n  Rust: {}",
        show(stdin_bytes),
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        c.code,
        r.code,
        "exit status mismatch for {label}\n  stdin: {}\n  C   : {:?}\n  Rust: {:?}",
        show(stdin_bytes),
        c.code,
        r.code
    );
}

#[track_caller]
fn check_all(cases: &[(&str, &[u8])]) {
    for (label, input) in cases {
        assert_same(label, input);
    }
}

// ===========================================================================
// Phase A sanity: both binaries exist, run, and produce the documented shape
// ===========================================================================

#[test]
fn both_binaries_run_and_emit_32_hex_digits_plus_newline() {
    let c = run(c_bin(), b"1");
    let r = run(&rust_bin(), b"1");

    // sizeof(house_t) == 16 -> 16 * "%02x" + "\n"
    assert_eq!(c.stdout.len(), 33, "C stdout: {}", show(&c.stdout));
    assert_eq!(r.stdout.len(), 33, "Rust stdout: {}", show(&r.stdout));
    assert_eq!(c.stdout, r.stdout);
    assert!(c.stdout.ends_with(b"\n"));
    assert_eq!(c.code, Some(0));
    assert_eq!(r.code, Some(0));
    assert!(c.stderr.is_empty());
    assert!(r.stderr.is_empty());
}

// ===========================================================================
// Phase B: the input classes main()/driver() actually branch on
// ===========================================================================

/// `scanf` returns EOF, so `x` keeps its initialiser of 0. This is the
/// "empty input" class.
#[test]
fn empty_input() {
    check_all(&[
        ("empty stdin", b""),
        ("single newline", b"\n"),
        ("single space", b" "),
        ("whitespace only", b"  \t\n\r\n  "),
        ("all isspace kinds", b" \t\n\x0b\x0c\r"),
    ]);
}

/// A single well-formed item -- the happy path.
#[test]
fn single_item_happy_path() {
    check_all(&[
        ("zero", b"0"),
        ("one", b"1"),
        ("three", b"3"),
        ("small", b"5"),
        ("with newline", b"7\n"),
        ("trailing spaces", b"  42  "),
        ("explicit plus", b"+7"),
        ("negative one", b"-1"),
        ("negative", b"-12345"),
        ("minus zero", b"-0"),
        ("plus zero", b"+0"),
        ("leading zeros", b"0012"),
        ("many leading zeros", b"000000000000000000000000000012"),
        ("round number", b"1000000"),
        ("byte-ish values", b"255"),
        ("short-ish values", b"65535"),
    ]);
}

/// `scanf("%d")` skips *all* leading whitespace, newlines included, and only
/// consumes a single conversion. Everything after it is ignored because the
/// program never reads again.
#[test]
fn scanf_reads_across_newlines_and_stops_at_first_conversion() {
    check_all(&[
        ("leading newlines", b"\n\n\n12"),
        ("newlines and spaces", b"\n   \n\t 12"),
        ("two numbers, second ignored", b"12 34"),
        ("two numbers newline separated", b"12\n34"),
        ("many numbers", b"1 2 3 4 5 6 7 8 9\n"),
        ("number then junk", b"12abc"),
        ("number then punctuation", b"12,34"),
        ("decimal point stops scan", b"3.9"),
        ("hex prefix stops after 0", b"0x10"),
        ("exponent stops after digits", b"1e9"),
        ("underscore stops scan", b"12_34"),
        ("second sign stops scan", b"12-34"),
    ]);
}

/// Matching failure: nothing is stored, so `x` stays 0.
#[test]
fn matching_failure_leaves_x_at_zero() {
    check_all(&[
        ("alpha", b"abc"),
        ("alpha then digits", b"abc 12"),
        ("bare plus", b"+"),
        ("bare minus", b"-"),
        ("sign then alpha", b"-abc"),
        ("sign then space", b"- 12"),
        ("double sign", b"--12"),
        ("plus minus", b"+-12"),
        ("leading dot", b".5"),
        ("leading comma", b",12"),
        ("hex letters only", b"xyz"),
        ("nul byte first", b"\0\0 12"),
        ("whitespace then alpha", b"   \n zz"),
        ("arabic-indic digits (not ASCII)", "\u{661}\u{662}".as_bytes()),
        ("newline separated junk", b"junk\nmore junk\n"),
    ]);
}

/// int truncation / signedness exactly as the C performs it. glibc's `%d`
/// accumulates into a `long` (saturating like strtol) and then stores the low
/// 32 bits into the `int` destination.
#[test]
fn integer_overflow_truncation_and_signedness() {
    check_all(&[
        ("INT_MAX", b"2147483647"),
        ("INT_MAX - 1", b"2147483646"),
        ("INT_MAX + 1", b"2147483648"),
        ("INT_MAX + 2", b"2147483649"),
        ("INT_MIN", b"-2147483648"),
        ("INT_MIN + 1", b"-2147483647"),
        ("INT_MIN - 1", b"-2147483649"),
        ("UINT_MAX", b"4294967295"),
        ("2^32", b"4294967296"),
        ("2^32 + 1", b"4294967297"),
        ("2^32 + 5", b"4294967301"),
        ("-2^32", b"-4294967296"),
        ("LONG_MAX - 1", b"9223372036854775806"),
        ("LONG_MAX", b"9223372036854775807"),
        ("LONG_MAX + 1 (saturates)", b"9223372036854775808"),
        ("LONG_MIN", b"-9223372036854775808"),
        ("LONG_MIN - 1 (saturates)", b"-9223372036854775809"),
        ("ULONG_MAX", b"18446744073709551615"),
        ("2^64", b"18446744073709551616"),
        ("1e19", b"10000000000000000000"),
        ("20 nines", b"99999999999999999999"),
        ("26 nines", b"99999999999999999999999999"),
        ("negative 26 nines", b"-99999999999999999999999999"),
        (
            "leading zeros then negative value",
            b"  -0000000000000000000000000000012 ",
        ),
    ]);
}

/// The largest thing `%d` will chew through: absurdly long digit runs. glibc
/// saturates the accumulator, so every one of these lands on the same
/// truncated value.
#[test]
fn maximum_length_digit_runs() {
    let zeros_then_seven = {
        let mut v = vec![b'0'; 10_000];
        v.push(b'7');
        v
    };
    let nines = vec![b'9'; 10_000];
    let neg_nines = {
        let mut v = vec![b'-'];
        v.extend(std::iter::repeat(b'9').take(5_000));
        v
    };
    let huge_zeros = vec![b'0'; 100_000];
    let padded = {
        let mut v = vec![b' '; 8_192];
        v.extend_from_slice(b"-4242");
        v.extend(std::iter::repeat(b'\n').take(1_000));
        v
    };

    check_all(&[
        ("10000 zeros then 7", &zeros_then_seven),
        ("10000 nines", &nines),
        ("negative 5000 nines", &neg_nines),
        ("100000 zeros", &huge_zeros),
        ("8k spaces then -4242", &padded),
    ]);
}

/// Binary / non-UTF-8 stdin must not change behaviour or crash either side.
#[test]
fn binary_and_embedded_nul_input() {
    check_all(&[
        ("nul then digits", b"\0\0 12"),
        ("digits then nul", b"12\0 34"),
        ("high bytes", b"\xff\xfe\xfd"),
        ("high bytes then digits", b"\xff 12"),
        ("digits then high bytes", b"12\xff\xfe"),
        ("invalid utf8 mid number", b"1\xc3\x282"),
        ("ff repeated", &[0xffu8; 1024]),
    ]);
}

/// stdin that is not a readable file at all.
#[test]
fn stdin_is_empty_device() {
    // /dev/null: immediate EOF.
    let c = Command::new(c_bin())
        .stdin(Stdio::null())
        .output()
        .expect("run C with /dev/null stdin");
    let r = Command::new(rust_bin())
        .stdin(Stdio::null())
        .output()
        .expect("run Rust with /dev/null stdin");
    assert_eq!(c.stdout, r.stdout, "stdout mismatch with /dev/null stdin");
    assert_eq!(c.stderr, r.stderr, "stderr mismatch with /dev/null stdin");
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "exit status mismatch with /dev/null stdin"
    );
}

/// stdout is a pipe whose read end is already closed. The C program keeps the
/// default `SIGPIPE` disposition and is killed by the signal; the Rust program
/// must not silently exit 0 instead (Rust's std ignores SIGPIPE by default).
///
/// The child blocks reading stdin until we feed it, so the read end of its
/// stdout pipe is guaranteed to be closed before it ever calls printf.
#[test]
fn closed_stdout_pipe_matches_sigpipe_death() {
    fn run_with_closed_stdout(exe: &Path) -> (Option<i32>, Option<i32>) {
        let mut child = Command::new(exe)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap_or_else(|e| panic!("spawn {}: {e}", exe.display()));

        // Close the read end of the child's stdout before it can write.
        drop(child.stdout.take());

        {
            let mut sink = child.stdin.take().expect("piped stdin");
            let _ = sink.write_all(b"5\n");
            let _ = sink.flush();
        }

        let status = child.wait().expect("wait");
        #[cfg(unix)]
        let signal = {
            use std::os::unix::process::ExitStatusExt;
            status.signal()
        };
        #[cfg(not(unix))]
        let signal = None;

        (status.code(), signal)
    }

    let c = run_with_closed_stdout(c_bin());
    let r = run_with_closed_stdout(&rust_bin());
    assert_eq!(
        c, r,
        "closed-stdout outcome mismatch: C (code, signal) = {c:?}, Rust = {r:?}"
    );
}

// ===========================================================================
// Phase C: broad sweeps, to catch input classes hand enumeration missed
// ===========================================================================

/// Deterministic sweep over every value the struct's first field can plausibly
/// take, plus neighbourhoods of each power of two.
#[test]
fn exhaustive_small_values_and_powers_of_two() {
    let mut cases: Vec<Vec<u8>> = Vec::new();

    for v in -300i64..=300 {
        cases.push(v.to_string().into_bytes());
    }
    for bit in 0..64u32 {
        let p = 1u128 << bit;
        for delta in [-1i128, 0, 1] {
            let v = p as i128 + delta;
            cases.push(v.to_string().into_bytes());
            cases.push(format!("-{v}").into_bytes());
        }
    }

    for c in &cases {
        assert_same("power-of-two sweep", c);
    }
}

/// Deterministic pseudo-random fuzz over the byte alphabet that `%d` cares
/// about: digits, signs, whitespace, letters and NUL.
#[test]
fn deterministic_fuzz_over_scanf_alphabet() {
    const ALPHABET: &[u8] = b"0123456789 \n\t\r\x0b\x0c+-abcxX.,_\0";

    // xorshift64* -- no external crates, fully reproducible.
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = move || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state.wrapping_mul(0x2545_F491_4F6C_DD1D)
    };

    for _ in 0..600 {
        let len = (next() % 17) as usize;
        let input: Vec<u8> = (0..len)
            .map(|_| ALPHABET[(next() % ALPHABET.len() as u64) as usize])
            .collect();
        assert_same("fuzz(alphabet)", &input);
    }
}

/// Deterministic fuzz over numeric literals of every interesting width,
/// covering the whole long/int truncation surface.
#[test]
fn deterministic_fuzz_over_numeric_literals() {
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = move || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state.wrapping_mul(0x2545_F491_4F6C_DD1D)
    };

    const WIDTHS: [usize; 12] = [1, 2, 3, 5, 9, 10, 11, 15, 19, 20, 21, 40];

    for _ in 0..600 {
        let width = WIDTHS[(next() % WIDTHS.len() as u64) as usize];
        let mut s = Vec::new();
        match next() % 3 {
            0 => s.push(b'-'),
            1 => s.push(b'+'),
            _ => {}
        }
        for _ in 0..width {
            s.push(b'0' + (next() % 10) as u8);
        }
        // Sometimes append trailing junk that scanf must stop before.
        match next() % 4 {
            0 => s.extend_from_slice(b"\n"),
            1 => s.extend_from_slice(b" 99"),
            2 => s.extend_from_slice(b"abc"),
            _ => {}
        }
        assert_same("fuzz(numeric)", &s);
    }
}

/// Fuzz over completely arbitrary bytes: the C must never be out-diffed by
/// something the Rust reader chokes on (e.g. invalid UTF-8).
#[test]
fn deterministic_fuzz_over_arbitrary_bytes() {
    let mut state: u64 = 0xDEAD_BEEF_CAFE_F00D;
    let mut next = move || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state.wrapping_mul(0x2545_F491_4F6C_DD1D)
    };

    for _ in 0..300 {
        let len = (next() % 33) as usize;
        let input: Vec<u8> = (0..len).map(|_| (next() % 256) as u8).collect();
        assert_same("fuzz(bytes)", &input);
    }
}

// ===========================================================================
// Pinned expected bytes: guards against BOTH programs drifting together.
// ===========================================================================

/// house_t is {floors, bedrooms=3, bathrooms=2.0} zero-initialised first, so
/// the 16-byte image is: <floors LE> 03000000 <2.0 as LE double = 0000000000000040>.
#[test]
fn pinned_output_bytes_match_the_struct_layout() {
    let expected = |floors: i32| -> String {
        let mut s = String::new();
        for b in floors.to_le_bytes() {
            s.push_str(&format!("{b:02x}"));
        }
        s.push_str("03000000");
        for b in 2.0f64.to_le_bytes() {
            s.push_str(&format!("{b:02x}"));
        }
        s.push('\n');
        s
    };

    for (input, floors) in [
        (&b""[..], 0i32),
        (&b"0"[..], 0),
        (&b"1"[..], 1),
        (&b"5"[..], 5),
        (&b"-1"[..], -1),
        (&b"1000000"[..], 1_000_000),
        (&b"2147483647"[..], i32::MAX),
        (&b"2147483648"[..], i32::MIN),
        (&b"9223372036854775807"[..], -1),
        (&b"abc"[..], 0),
    ] {
        let want = expected(floors);
        let c = run(c_bin(), input);
        let r = run(&rust_bin(), input);
        assert_eq!(
            String::from_utf8_lossy(&c.stdout),
            want,
            "C output for {}",
            show(input)
        );
        assert_eq!(
            String::from_utf8_lossy(&r.stdout),
            want,
            "Rust output for {}",
            show(input)
        );
    }
}
