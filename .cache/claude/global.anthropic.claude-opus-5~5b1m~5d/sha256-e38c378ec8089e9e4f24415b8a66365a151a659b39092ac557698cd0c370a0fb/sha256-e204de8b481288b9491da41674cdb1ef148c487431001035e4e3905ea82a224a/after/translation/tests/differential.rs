//! Differential tests: run the C `driver` and the Rust `driver` as *subprocesses*
//! and compare stdout, stderr and exit status byte-for-byte / value-for-value.
//!
//! Nothing here links the Rust code as a library; both programs are driven
//! exactly the way a shell would drive them.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Locating / building the two binaries
// ---------------------------------------------------------------------------

/// `<workspace>/translation`
fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `<workspace>` — the directory holding both `c_src/` and `translation/`.
fn workspace_root() -> PathBuf {
    crate_dir()
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the Rust binary under test (same profile as the test binary).
fn rust_bin() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        // The integration test executable lives in
        // target/<profile>/deps/<name>-<hash>; the program is two levels up.
        let exe = std::env::current_exe().expect("current_exe");
        let dir = exe
            .parent()
            .and_then(Path::parent)
            .expect("target/<profile>")
            .to_path_buf();
        let cand = dir.join(if cfg!(windows) { "driver.exe" } else { "driver" });
        assert!(
            cand.is_file(),
            "Rust binary not found at {}. Run `cargo build` (or `cargo build --release`) first.",
            cand.display()
        );
        cand
    })
    .as_path()
}

/// Path to the C binary, building it with CMake on first use if necessary.
fn c_bin() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let c_src = workspace_root().join("c_src");
        let build = c_src.join("build");
        let exe = build.join(if cfg!(windows) { "driver.exe" } else { "driver" });
        if !exe.is_file() {
            std::fs::create_dir_all(&build).expect("create c_src/build");
            let cfg = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("failed to run `cmake ..` — is CMake installed?");
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
        assert!(
            exe.is_file(),
            "C binary still missing at {}",
            exe.display()
        );
        exe
    })
    .as_path()
}

// ---------------------------------------------------------------------------
// Running
// ---------------------------------------------------------------------------

struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Some(code)` for a normal exit, `None` if killed by a signal.
    code: Option<i32>,
}

fn run(bin: &Path, stdin_bytes: &[u8]) -> Run {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    {
        let mut sin = child.stdin.take().expect("piped stdin");
        // The child may exit without draining stdin; a broken pipe is fine.
        let _ = sin.write_all(stdin_bytes);
        let _ = sin.flush();
    }

    let out = child.wait_with_output().expect("wait_with_output");
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
    }
}

fn show(b: &[u8]) -> String {
    match std::str::from_utf8(b) {
        Ok(s) => format!("{s:?}"),
        Err(_) => format!("{b:02x?}"),
    }
}

/// Assert the two programs agree on stdout, stderr and exit status.
#[track_caller]
fn assert_same(label: &str, stdin_bytes: &[u8]) {
    let c = run(c_bin(), stdin_bytes);
    let r = run(rust_bin(), stdin_bytes);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch for {label} (input {}):\n  C   : {}\n  Rust: {}",
        show(stdin_bytes),
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch for {label} (input {}):\n  C   : {}\n  Rust: {}",
        show(stdin_bytes),
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        c.code, r.code,
        "exit status mismatch for {label} (input {}): C={:?} Rust={:?}",
        show(stdin_bytes),
        c.code,
        r.code
    );
}

#[track_caller]
fn same(input: &str) {
    assert_same(input, input.as_bytes());
}

// ---------------------------------------------------------------------------
// Phase A — both binaries exist and are runnable
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_run() {
    let c = run(c_bin(), b"1");
    let r = run(rust_bin(), b"1");
    assert_eq!(c.code, Some(0));
    assert_eq!(r.code, Some(0));
    assert!(!c.stdout.is_empty());
    assert!(!r.stdout.is_empty());
}

// ---------------------------------------------------------------------------
// Phase B — the branches main() actually takes
// ---------------------------------------------------------------------------

/// `scanf` fails on empty input (input failure): `x` keeps its initializer 0,
/// so `main` takes the `else` branch and calls `bad()`.
#[test]
fn empty_input_takes_bad_branch() {
    assert_same("empty", b"");
}

/// A single item, zero => `bad()`.
#[test]
fn zero_takes_bad_branch() {
    for s in ["0", "+0", "-0", "00", "000000000000", "0\n", " 0", "\t0\n"] {
        same(s);
    }
}

/// A single item, non-zero => `good()` prints 5.
#[test]
fn nonzero_takes_good_branch() {
    for s in ["1", "5", "-1", "+3", "42", "-42", "1\n", "   \n\t 7\n", "0001"] {
        same(s);
    }
}

/// `scanf` skips leading whitespace and crosses newlines (unlike `fgets`).
#[test]
fn whitespace_is_skipped_across_newlines() {
    for s in [
        " ",
        "\n",
        "\n\n\n",
        "\t",
        "\r",
        "\x0b",
        "\x0c",
        "   ",
        "\n\n\n\n5",
        " \t\r\n\x0b\x0c 0",
        "\n\n\n\n\n\n\n\n-9",
        "        ", // whitespace only => EOF => input failure
    ] {
        same(s);
    }
}

// ---------------------------------------------------------------------------
// Phase C — matching failures, overflow, truncation, trailing junk
// ---------------------------------------------------------------------------

/// Non-numeric first character: matching failure, `x` untouched (still 0).
#[test]
fn matching_failure_leaves_x_zero() {
    for s in [
        "abc", "x", ".", ".5", "e", "E", "+", "-", "++1", "--1", "+-1", "-+1", "/", ":", "\"", "%",
        "#", "\x7f", "\x01",
    ] {
        same(s);
    }
}

/// A sign followed immediately by EOF or by a non-digit is a matching failure.
#[test]
fn sign_then_non_digit() {
    for s in ["-", "+", "-\n", "+\n", "- 1", "+ 1", "-a", "+a", "-.", "+."] {
        same(s);
    }
}

/// Conversion stops at the first non-digit; the rest of stdin is never read.
#[test]
fn trailing_junk_is_ignored() {
    for s in [
        "0abc", "1abc", "0.5", "1.5", "1e5", "0e5", "3 4", "0 1", "1 0", "1\n2", "0\n1", "12,34",
        "0-", "1-", "7)", "0)",
    ] {
        same(s);
    }
}

/// Values outside `int`: glibc clamps to `long` range and the store into `int`
/// truncates, so e.g. 4294967296 becomes 0 and takes the `bad()` branch.
#[test]
fn overflow_truncation_and_signedness() {
    for s in [
        "2147483647",            // INT_MAX
        "2147483648",            // INT_MAX+1 -> truncates to INT_MIN (non-zero)
        "2147483649",
        "-2147483648",           // INT_MIN
        "-2147483649",
        "4294967295",            // 2^32-1 -> -1
        "4294967296",            // 2^32   -> 0  => bad()
        "-4294967296",           // -2^32  -> 0  => bad()
        "8589934592",            // 2^33   -> 0  => bad()
        "-8589934592",
        "10000000000",
        "9223372036854775807",   // LONG_MAX
        "9223372036854775808",   // LONG_MAX+1 -> ERANGE, clamped
        "-9223372036854775808",  // LONG_MIN
        "-9223372036854775809",  // LONG_MIN-1 -> ERANGE, clamped
        "18446744073709551616",  // 2^64
        "99999999999999999999",
        "999999999999999999999999999999999999",
        "-999999999999999999999999999999999999",
    ] {
        same(s);
    }
}

/// Long runs of leading zeros and very long digit strings.
#[test]
fn long_digit_strings() {
    same(&"0".repeat(4096));
    same(&format!("{}7", "0".repeat(4096)));
    same(&"9".repeat(4096));
    same(&format!("-{}", "9".repeat(4096)));
    // Larger than any plausible stdio buffer.
    same(&format!("{}7", "0".repeat(200_000)));
    same(&"0".repeat(200_000));
}

/// Bytes that are not valid UTF-8 must behave identically.
#[test]
fn non_utf8_input() {
    for bytes in [
        &b"\xff"[..],
        &b"\xff\xfe"[..],
        &b"\x00"[..],
        &b"\x005"[..],
        &b"5\x00"[..],
        &b"\x00\x00\x00"[..],
        &b"\xc3\x28"[..],
        &b" \xff 5"[..],
        &b"5\xff"[..],
    ] {
        assert_same("non-utf8", bytes);
    }
}

/// stdin closed rather than merely empty.
#[test]
fn stdin_immediately_closed() {
    let c = {
        let child = Command::new(c_bin())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn C");
        child.wait_with_output().expect("wait C")
    };
    let r = {
        let child = Command::new(rust_bin())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn Rust");
        child.wait_with_output().expect("wait Rust")
    };
    assert_eq!(c.stdout, r.stdout, "stdout mismatch with /dev/null stdin");
    assert_eq!(c.stderr, r.stderr, "stderr mismatch with /dev/null stdin");
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "exit status mismatch with /dev/null stdin"
    );
}

/// The program ignores argv entirely.
#[test]
fn extra_argv_is_ignored() {
    for args in [vec!["ignored"], vec!["-h"], vec!["a", "b", "c"]] {
        let mut cc = Command::new(c_bin());
        let mut rr = Command::new(rust_bin());
        cc.args(&args);
        rr.args(&args);
        let c = cc
            .stdin(Stdio::null())
            .output()
            .expect("C with args");
        let r = rr
            .stdin(Stdio::null())
            .output()
            .expect("Rust with args");
        assert_eq!(c.stdout, r.stdout, "stdout mismatch with args {args:?}");
        assert_eq!(c.stderr, r.stderr, "stderr mismatch with args {args:?}");
        assert_eq!(
            c.status.code(),
            r.status.code(),
            "exit status mismatch with args {args:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Exhaustive-ish sweeps and deterministic fuzzing
// ---------------------------------------------------------------------------

/// Every single byte as the whole input.
#[test]
fn every_single_byte_input() {
    for b in 0u16..=255 {
        assert_same("single byte", &[b as u8]);
    }
}

/// A dense sweep of small integers, both branches.
#[test]
fn small_integer_sweep() {
    for v in -300i64..=300 {
        same(&v.to_string());
        same(&format!("{v}\n"));
        same(&format!("  {v}  "));
    }
}

/// Deterministic pseudo-random inputs drawn from a numeric-ish alphabet.
#[test]
fn deterministic_fuzz() {
    const ALPHA: &[u8] = b"0123456789 +-\n\t.eEx\r\x0b\x0c";
    // xorshift64* — no external crates.
    let mut state: u64 = 0x9E3779B97F4A7C15;
    let mut next = move || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state = state.wrapping_mul(0x2545F4914F6CDD1D);
        state
    };
    for _ in 0..1500 {
        let len = (next() % 18) as usize;
        let mut buf = Vec::with_capacity(len);
        for _ in 0..len {
            buf.push(ALPHA[(next() % ALPHA.len() as u64) as usize]);
        }
        assert_same("fuzz", &buf);
    }
}

/// Deterministic pseudo-random raw byte inputs (including NULs and high bytes).
#[test]
fn deterministic_fuzz_raw_bytes() {
    let mut state: u64 = 0xDEADBEEFCAFEF00D;
    let mut next = move || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state = state.wrapping_mul(0x2545F4914F6CDD1D);
        state
    };
    for _ in 0..1000 {
        let len = (next() % 12) as usize;
        let mut buf = Vec::with_capacity(len);
        for _ in 0..len {
            buf.push((next() & 0xFF) as u8);
        }
        assert_same("raw fuzz", &buf);
    }
}

/// The output is exactly one line with no extra bytes: `printf("%d\n", ...)`.
#[test]
fn output_shape_is_one_line_no_extra_bytes() {
    for (input, expected) in [("", "0\n"), ("0", "0\n"), ("1", "5\n"), ("-7", "5\n")] {
        let c = run(c_bin(), input.as_bytes());
        let r = run(rust_bin(), input.as_bytes());
        assert_eq!(c.stdout, expected.as_bytes(), "C output shape for {input:?}");
        assert_eq!(r.stdout, expected.as_bytes(), "Rust output shape for {input:?}");
        assert!(c.stderr.is_empty());
        assert!(r.stderr.is_empty());
    }
}
