//! Differential tests: run the C reference binary and the Rust binary as
//! subprocesses on identical stdin and require byte-identical stdout, stderr
//! and exit status.
//!
//! The Rust code is never called as a library here — only the built executable
//! is driven, exactly as a shell would drive it, because that is how the two
//! programs are compared.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;

/// Path to the Rust executable under test, provided by Cargo.
const RUST_BIN: &str = env!("CARGO_BIN_EXE_driver");

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Builds `c_src` with CMake if the reference binary is not present yet, then
/// returns its path. Nothing inside `c_src` is modified; only the ignored
/// `build/` output directory is produced.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = repo_root().join("c_src");
        let build = c_src.join("build");
        let bin = build.join("driver");
        if !bin.exists() {
            std::fs::create_dir_all(&build).expect("create c_src/build");
            let cfg = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("cmake must be installed to build the C reference");
            assert!(
                cfg.status.success(),
                "cmake configure failed:\n{}",
                String::from_utf8_lossy(&cfg.stderr)
            );
            let bld = Command::new("cmake")
                .args(["--build", "."])
                .current_dir(&build)
                .output()
                .expect("cmake --build");
            assert!(
                bld.status.success(),
                "cmake build failed:\n{}",
                String::from_utf8_lossy(&bld.stderr)
            );
        }
        assert!(bin.exists(), "C reference binary missing at {:?}", bin);
        bin
    })
}

/// Runs `bin` with `stdin_bytes` piped in.
///
/// Neither program drains stdin before exiting, so a large payload can outrun
/// the pipe buffer and the write side sees `EPIPE`. That is expected for both
/// binaries and is not an observable difference, so it is ignored here; only
/// stdout, stderr and the exit status are compared.
fn run_piped(bin: &Path, stdin_bytes: &[u8]) -> Output {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {:?}: {e}", bin));

    let mut stdin = child.stdin.take().expect("stdin piped");
    let payload = stdin_bytes.to_vec();
    // Write from a helper thread so a full pipe cannot deadlock this thread
    // against the child's stdout/stderr pipes.
    let writer = std::thread::spawn(move || {
        match stdin.write_all(&payload) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => {}
            Err(e) => panic!("unexpected stdin write error: {e}"),
        }
        // Dropping `stdin` closes it, signalling EOF.
    });

    let out = child.wait_with_output().expect("collect output");
    writer.join().expect("stdin writer thread");
    out
}

/// Runs `bin` with stdin connected to a regular file rather than a pipe, which
/// is how a shell redirect (`./driver < input`) supplies input.
fn run_with_file_stdin(bin: &Path, stdin_bytes: &[u8]) -> Output {
    let dir = std::env::temp_dir().join(format!(
        "driver-difftest-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("stdin.bin");
    std::fs::write(&path, stdin_bytes).expect("write temp stdin file");
    let file = std::fs::File::open(&path).expect("open temp stdin file");
    let out = Command::new(bin)
        .stdin(Stdio::from(file))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn {:?}: {e}", bin));
    let _ = std::fs::remove_file(&path);
    out
}

/// Asserts the C and Rust programs agree on stdout, stderr and exit status,
/// both with piped stdin and with a file redirected onto stdin.
fn assert_same(label: &str, stdin_bytes: &[u8]) {
    for (mode, runner) in [
        ("pipe", run_piped as fn(&Path, &[u8]) -> Output),
        ("file", run_with_file_stdin as fn(&Path, &[u8]) -> Output),
    ] {
        let c = runner(c_bin(), stdin_bytes);
        let r = runner(Path::new(RUST_BIN), stdin_bytes);
        compare(&format!("{label}/{mode}"), stdin_bytes, &c, &r);
    }
}

fn compare(label: &str, stdin_bytes: &[u8], c: &Output, r: &Output) {
    assert_eq!(
        c.stdout,
        r.stdout,
        "[{label}] stdout differs\ninput = {:?}\n--- C ---\n{}\n--- Rust ---\n{}",
        Escaped(stdin_bytes),
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "[{label}] stderr differs\ninput = {:?}\nC = {:?}\nRust = {:?}",
        Escaped(stdin_bytes),
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "[{label}] exit status differs (C={:?} Rust={:?}) for input {:?}",
        c.status,
        r.status,
        Escaped(stdin_bytes)
    );
}

/// Compact debug rendering for possibly-binary stdin payloads.
struct Escaped<'a>(&'a [u8]);
impl std::fmt::Debug for Escaped<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.len() > 80 {
            write!(f, "<{} bytes>", self.0.len())
        } else {
            write!(f, "{}", String::from_utf8_lossy(self.0).escape_debug())
        }
    }
}

// ---------------------------------------------------------------------------
// Phase A sanity: both binaries exist and run.
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_run() {
    let c = run_piped(c_bin(), b"1");
    let r = run_piped(Path::new(RUST_BIN), b"1");
    assert!(!c.stdout.is_empty(), "C produced no stdout");
    assert!(!r.stdout.is_empty(), "Rust produced no stdout");
    assert_eq!(c.status.code(), Some(0));
    assert_eq!(r.status.code(), Some(0));
}

/// Pins the exact reference output so a regression in either program is
/// visible even without the differential comparison.
#[test]
fn golden_output_shape_for_zero() {
    let c = run_piped(c_bin(), b"0");
    let expected = "\
The house has 2 floors, 5 bedrooms, and 2.5 bathrooms
The house has 3 floors, 5 bedrooms, and 2.5 bathrooms
The house has 3 floors, 5 bedrooms, and 3.5 bathrooms
The house has 3 floors, 5 bedrooms, and 3.5 bathrooms
The house has 3 floors, 5 bedrooms, and 3.5 bathrooms
The house has 4 floors, 5 bedrooms, and 3.5 bathrooms
The house has 4 floors, 5 bedrooms, and 4.5 bathrooms
The house has 4 floors, 5 bedrooms, and 4.5 bathrooms
";
    assert_eq!(String::from_utf8_lossy(&c.stdout), expected);
    assert_same("golden_zero", b"0");
}

// ---------------------------------------------------------------------------
// Phase B: the input classes `scanf("%d", &x)` branches on.
//
// The C program has exactly one input-dependent decision: whether the single
// `%d` directive converts a value (assigning `x`) or fails to match / hits EOF
// (leaving `x` at its initializer 0). Everything downstream is arithmetic on
// that `int`, so the input classes are the classes of the conversion.
// ---------------------------------------------------------------------------

#[test]
fn empty_input_eof_before_any_conversion() {
    // scanf returns EOF, x keeps its initializer 0.
    assert_same("empty", b"");
}

#[test]
fn single_item_smallest_useful_values() {
    for s in ["0", "1", "2", "5", "9"] {
        assert_same(s, s.as_bytes());
    }
}

#[test]
fn negative_and_explicit_plus_sign() {
    for s in ["-1", "-3", "-9", "+1", "+7", "-0", "+0"] {
        assert_same(s, s.as_bytes());
    }
}

#[test]
fn leading_whitespace_is_skipped_across_newlines() {
    // %d skips leading whitespace, including newlines: scanf reads across
    // lines where fgets would not.
    for s in [
        "   42",
        "\n42",
        "\n\n\n42",
        "\t42",
        " \t\n\r\x0b\x0c 8",
        "\r\n-9",
    ] {
        assert_same(&format!("ws:{}", s.escape_debug()), s.as_bytes());
    }
}

#[test]
fn whitespace_only_input_is_eof() {
    for s in ["   ", "\n", "\n\n\n", "\t\t", " \t\n\r\x0b\x0c"] {
        assert_same(&format!("ws_only:{}", s.escape_debug()), s.as_bytes());
    }
}

#[test]
fn matching_failure_leaves_x_at_zero() {
    // No digits available after optional sign -> matching failure, no
    // assignment. x stays 0, and the program still exits 0.
    for s in [
        "abc", "-", "+", "--5", "-+5", "+-5", ".5", "x", "-abc", "+ 5", "- 5", "e5", "/", ":",
    ] {
        assert_same(&format!("nomatch:{}", s.escape_debug()), s.as_bytes());
    }
}

#[test]
fn conversion_stops_at_first_non_digit() {
    for s in ["12abc", "1e5", "0x10", "3 4", "9\n", "7\n\n", "5.5", "2-3", "4+5"] {
        assert_same(&format!("partial:{}", s.escape_debug()), s.as_bytes());
    }
}

#[test]
fn leading_zeros_are_decimal_not_octal() {
    for s in ["007", "0000", "-007", "0000000000000000000000000000005"] {
        assert_same(&format!("zeros:{}", s.escape_debug()), s.as_bytes());
    }
}

#[test]
fn int_extremes_and_signed_wraparound_in_add_bedrooms() {
    // bedrooms is `int`; `bedrooms += extra` runs twice, so these exercise
    // signed wraparound exactly as the C performs it.
    for s in [
        "2147483647",  // INT_MAX
        "2147483646",
        "-2147483648", // INT_MIN
        "-2147483647",
        "1073741824", // 2^30
        "2147483643",
    ] {
        assert_same(s, s.as_bytes());
    }
}

#[test]
fn values_beyond_int_are_truncated_from_long() {
    // These fit in a 64-bit `long`, so glibc converts them and the store
    // through `int *` truncates.
    for s in [
        "2147483648",           // INT_MAX + 1
        "-2147483649",          // INT_MIN - 1
        "4294967296",           // 2^32  -> 0
        "4294967297",           // 2^32+1 -> 1
        "-4294967296",
        "9223372036854775807",  // LONG_MAX -> -1
        "-9223372036854775808", // LONG_MIN -> 0
        "2147483647999999",
    ] {
        assert_same(s, s.as_bytes());
    }
}

#[test]
fn values_beyond_long_saturate_then_truncate() {
    // Out-of-range for `long`: glibc's %d saturates to LONG_MAX / LONG_MIN
    // (ERANGE) rather than wrapping, and the saturated value is truncated to
    // `int` (-1 and 0 respectively).
    for s in [
        "9223372036854775808",   // LONG_MAX + 1
        "-9223372036854775809",  // LONG_MIN - 1
        "18446744073709551616",  // 2^64
        "18446744073709551617",  // 2^64 + 1
        "-18446744073709551617",
        "99999999999999999999",
        "-99999999999999999999",
        "10000000000000000000000000000000000000000000",
    ] {
        assert_same(s, s.as_bytes());
    }
}

#[test]
fn very_long_digit_runs() {
    for n in [40usize, 1000, 4095, 4096, 4097, 5000] {
        let s = "1".repeat(n);
        assert_same(&format!("digits*{n}"), s.as_bytes());
        let s = format!("-{}", "9".repeat(n));
        assert_same(&format!("negdigits*{n}"), s.as_bytes());
        // Leading zeros must not trigger overflow.
        let s = format!("{}7", "0".repeat(n));
        assert_same(&format!("zeros*{n}_then7"), s.as_bytes());
    }
}

// ---------------------------------------------------------------------------
// Phase C: paths and boundaries not exercised above.
// ---------------------------------------------------------------------------

#[test]
fn stdin_buffer_boundaries() {
    // Exercises refilling the input buffer in the middle of the directive:
    // whitespace skip, sign, and digit run each straddling a 4096-byte edge.
    for n in [4094usize, 4095, 4096, 4097, 8192] {
        let s = format!("{}42", " ".repeat(n));
        assert_same(&format!("ws{n}_then_42"), s.as_bytes());
        let s = format!("{}-9", "\n".repeat(n));
        assert_same(&format!("nl{n}_then_neg9"), s.as_bytes());
        // Sign lands exactly on the boundary, digits after it.
        let s = format!("{}-1234", " ".repeat(n - 1));
        assert_same(&format!("sign_at_{n}"), s.as_bytes());
    }
}

#[test]
fn binary_and_non_ascii_input() {
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("nul_first", vec![0x00, b'5']),
        ("nul_only", vec![0x00]),
        ("high_bytes", vec![0xff, 0xfe, b'3']),
        ("digit_then_nul", vec![b'7', 0x00, b'9']),
        ("all_256_bytes", (0u8..=255).collect()),
        ("invalid_utf8", vec![0xc3, 0x28, b'4']),
        ("ws_then_high", vec![b' ', 0x80, b'2']),
    ];
    for (label, bytes) in cases {
        assert_same(label, &bytes);
    }
}

#[test]
fn large_inputs_that_are_mostly_ignored() {
    // Only the first directive consumes input; the rest is never read, and the
    // program must still exit 0 without touching stderr.
    assert_same("200k_nuls", &vec![0u8; 200_000]);
    let mut v = b"7 ".to_vec();
    v.extend(std::iter::repeat(b'x').take(200_000));
    assert_same("7_then_200k_x", &v);
    let mut v = Vec::new();
    v.extend(std::iter::repeat(b'\n').take(100_000));
    v.extend_from_slice(b"11");
    assert_same("100k_nl_then_11", &v);
}

#[test]
fn multiple_tokens_only_first_is_consumed() {
    for s in [
        "1 2 3 4 5",
        "1\n2\n3\n",
        "-1 -2",
        "2147483647 2147483647",
        "0 abc",
        "abc 5", // matching failure on the first token; 5 is never reached
    ] {
        assert_same(&format!("multi:{}", s.escape_debug()), s.as_bytes());
    }
}

/// Deterministic randomized sweep over the alphabet the conversion branches on.
#[test]
fn randomized_differential_sweep() {
    const ALPHA: &[u8] = b"0123456789 +-\n\t\rabcx.\x00\x0b\x0c/:";
    // Small xorshift so the sweep is reproducible without a dependency.
    let mut state: u64 = 0x243f_6a88_85a3_08d3;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for i in 0..400 {
        let len = (next() % 14) as usize;
        let input: Vec<u8> = (0..len)
            .map(|_| ALPHA[(next() % ALPHA.len() as u64) as usize])
            .collect();
        assert_same(&format!("fuzz#{i}"), &input);
    }
}

/// Randomized sweep biased toward long digit strings around the int/long edges.
#[test]
fn randomized_numeric_sweep() {
    let mut state: u64 = 0x1357_9bdf_2468_ace0;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for i in 0..150 {
        let digits = 1 + (next() % 25) as usize;
        let mut s = String::new();
        if next() % 3 == 0 {
            s.push('-');
        } else if next() % 5 == 0 {
            s.push('+');
        }
        for _ in 0..digits {
            s.push((b'0' + (next() % 10) as u8) as char);
        }
        assert_same(&format!("numfuzz#{i}:{s}"), s.as_bytes());
    }
}
