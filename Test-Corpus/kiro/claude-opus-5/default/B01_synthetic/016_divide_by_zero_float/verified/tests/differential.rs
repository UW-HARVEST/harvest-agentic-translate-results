//! Differential tests: run the C `driver` and the Rust `driver` as
//! subprocesses on identical stdin and require byte-identical stdout, stderr
//! and exit status.
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

fn workspace_root() -> PathBuf {
    // .../<root>/translation/Cargo.toml -> .../<root>
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

fn rust_binary() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// Path to the compiled C program, building it with CMake on first use.
fn c_binary() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = workspace_root().join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");
        if !exe.exists() {
            std::fs::create_dir_all(&build).expect("cannot create c_src/build");
            let configure = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("failed to run `cmake ..` (is cmake installed?)");
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
        }
        assert!(
            exe.exists(),
            "the C program was not produced at {}",
            exe.display()
        );
        exe
    })
}

// ---------------------------------------------------------------------------
// Running one program
// ---------------------------------------------------------------------------

struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    // `None` when the process was killed by a signal.
    code: Option<i32>,
}

fn run(exe: &Path, stdin_bytes: &[u8]) -> Run {
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));

    {
        let mut sink = child.stdin.take().expect("stdin was piped");
        let bytes = stdin_bytes.to_vec();
        // Write on a helper thread so a program that never drains stdin cannot
        // deadlock the test.
        std::thread::spawn(move || {
            let _ = sink.write_all(&bytes);
            let _ = sink.flush();
        });
    }

    let out = child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("failed to wait for {}: {e}", exe.display()));
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
    }
}

fn show(bytes: &[u8]) -> String {
    // Escaped rendering so NULs, CRs and non-UTF-8 bytes stay visible.
    let mut s = String::new();
    for &b in bytes {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    s
}

/// Asserts that both programs agree on stdout, stderr and exit status.
#[track_caller]
fn assert_same(label: &str, stdin_bytes: &[u8]) {
    let c = run(c_binary(), stdin_bytes);
    let r = run(rust_binary(), stdin_bytes);
    compare_with_input(label, stdin_bytes, &c, &r);
}

/// How stdin is supplied to a child process.
enum Stdin<'a> {
    Bytes(&'a [u8]),
    /// Hand the child an fd opened on this path; used to reach `fgets`
    /// returning NULL because of a read error rather than end-of-file.
    FromPath(&'a Path),
}

fn run_with(exe: &Path, args: &[&str], input: Stdin<'_>) -> Run {
    match input {
        Stdin::Bytes(bytes) => {
            let mut child = Command::new(exe)
                .args(args)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));
            let mut sink = child.stdin.take().expect("stdin was piped");
            let owned = bytes.to_vec();
            std::thread::spawn(move || {
                let _ = sink.write_all(&owned);
                let _ = sink.flush();
            });
            let out = child.wait_with_output().expect("wait failed");
            Run {
                stdout: out.stdout,
                stderr: out.stderr,
                code: out.status.code(),
            }
        }
        Stdin::FromPath(path) => {
            let file = std::fs::File::open(path)
                .unwrap_or_else(|e| panic!("cannot open {} for stdin: {e}", path.display()));
            let out = Command::new(exe)
                .args(args)
                .stdin(Stdio::from(file))
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .unwrap_or_else(|e| panic!("failed to run {}: {e}", exe.display()));
            Run {
                stdout: out.stdout,
                stderr: out.stderr,
                code: out.status.code(),
            }
        }
    }
}

#[track_caller]
fn compare(label: &str, c: &Run, r: &Run) {
    compare_with_input(label, b"", c, r);
}

#[track_caller]
fn compare_with_input(label: &str, stdin_bytes: &[u8], c: &Run, r: &Run) {
    let mut problems = Vec::new();
    if c.stdout != r.stdout {
        problems.push(format!(
            "stdout differs:\n  C   : \"{}\"\n  Rust: \"{}\"",
            show(&c.stdout),
            show(&r.stdout)
        ));
    }
    if c.stderr != r.stderr {
        problems.push(format!(
            "stderr differs:\n  C   : \"{}\"\n  Rust: \"{}\"",
            show(&c.stderr),
            show(&r.stderr)
        ));
    }
    if c.code != r.code {
        problems.push(format!(
            "exit status differs: C = {:?}, Rust = {:?}",
            c.code, r.code
        ));
    }
    assert!(
        problems.is_empty(),
        "case `{label}` (stdin = \"{}\") mismatched:\n{}",
        show(stdin_bytes),
        problems.join("\n")
    );
}

fn assert_all(cases: &[(&str, &[u8])]) {
    for (label, input) in cases {
        assert_same(label, input);
    }
}

// ---------------------------------------------------------------------------
// Phase A — both programs exist, run, and produce the fixed scaffolding
// ---------------------------------------------------------------------------

#[test]
fn both_programs_run_and_agree_on_the_scaffolding() {
    // Sanity check on the harness itself: the C output for the simplest input
    // is known, and Rust must reproduce it.
    let input = b"2\n2\n";
    let c = run(c_binary(), input);
    assert_eq!(
        String::from_utf8_lossy(&c.stdout),
        "Calling good()...\n50\n50\nFinished good()\nCalling bad()...\n50\nFinished bad()\n"
    );
    assert!(c.stderr.is_empty(), "the C program writes nothing to stderr");
    assert_eq!(c.code, Some(0), "the C program always returns 0");
    assert_same("baseline 2/2", input);
}

// ---------------------------------------------------------------------------
// Phase B — the input classes the C program branches on
// ---------------------------------------------------------------------------

/// `fgets` returning NULL: the "fgets() failed." path in both `goodB2G` and
/// `bad`, plus the partially-consumed variants.
#[test]
fn fgets_null_paths() {
    assert_all(&[
        // Empty stdin: both fgets calls fail.
        ("empty stdin", b""),
        // One byte of input: goodB2G consumes it, bad() hits EOF.
        ("single newline", b"\n"),
        ("single digit, no newline", b"3"),
        ("single digit with newline", b"3\n"),
        ("two newlines", b"\n\n"),
        // Whitespace-only lines convert to 0.0 -> divide-by-zero message.
        ("blank-ish lines", b" \n \n"),
        ("tabs and CR", b"\t\r\n\t\r\n"),
    ]);
}

/// The `fabs(data) > 0.000001` guard in `goodB2G` versus the unguarded
/// division in `bad`.
#[test]
fn zero_and_threshold_paths() {
    assert_all(&[
        ("zero / zero", b"0\n0\n"),
        ("negative zero", b"-0\n-0\n"),
        ("negative zero decimal", b"-0.0\n-0.0\n"),
        ("exactly the threshold", b"0.000001\n0.000001\n"),
        // (float)1e-6 is slightly below 1e-6 as a double, so goodB2G takes the
        // divide-by-zero branch while bad() still prints 100000000.
        ("1e-06 spelled with an exponent", b"1e-06\n1e-06\n"),
        ("just above the threshold", b"0.0000011\n0.0000011\n"),
        ("just below the threshold", b"0.0000009\n0.0000009\n"),
        ("1.0000001e-06", b"1.0000001e-06\n1.0000001e-06\n"),
        ("9.9999999e-07", b"9.9999999e-07\n9.9999999e-07\n"),
        ("guard passes, bad divides", b"2e-6\n2e-6\n"),
        ("negative just above", b"-2e-6\n-2e-6\n"),
        // Only one of the two reads is a zero.
        ("good ok, bad divides by zero", b"3\n0\n"),
        ("good divides by zero, bad ok", b"0\n3\n"),
    ]);
}

/// Values that make `(int)(100.0 / data)` overflow, so the C cast yields the
/// x86-64 "integer indefinite" value.
#[test]
fn int_cast_overflow_paths() {
    assert_all(&[
        ("2^31 boundary", b"4.6566127e-08\n4.6566127e-08\n"),
        ("2^31 boundary, other side", b"4.6566129e-08\n4.6566129e-08\n"),
        ("just inside 2^31", b"4.65662e-08\n4.65662e-08\n"),
        ("negative 2^31 boundary", b"-4.6566127e-08\n-4.6566127e-08\n"),
        ("negative just inside", b"-4.65662e-08\n-4.65662e-08\n"),
        ("1e-7", b"1e-7\n1e-7\n"),
        ("-1e-7", b"-1e-7\n-1e-7\n"),
        // Denormal and underflowing floats.
        ("1e-40 (float denormal)", b"1e-40\n1e-40\n"),
        ("-1e-40", b"-1e-40\n-1e-40\n"),
        ("1e-45", b"1e-45\n1e-45\n"),
        ("1.4e-45", b"1.4e-45\n1.4e-45\n"),
        ("7e-46 (rounds to denormal)", b"7e-46\n7e-46\n"),
        ("2.5e-46", b"2.5e-46\n2.5e-46\n"),
        ("1e-46 (underflows to 0.0f)", b"1e-46\n1e-46\n"),
        ("4.9e-324 (double denormal)", b"4.9e-324\n4.9e-324\n"),
        ("1e-1000 (underflows in strtod)", b"1e-1000\n1e-1000\n"),
    ]);
}

/// `atof` corner cases: partial conversions, no conversion at all, signs,
/// exponents, hex floats and the special names.
#[test]
fn atof_parsing_paths() {
    assert_all(&[
        ("plain text", b"abc\nabc\n"),
        ("empty subject after sign", b"-\n+\n"),
        ("double sign", b"--5\n--5\n"),
        ("lone dot", b".\n.\n"),
        ("leading dot", b".5\n.5\n"),
        ("trailing dot", b"5.\n5.\n"),
        ("bare exponent marker", b"1e\n1e\n"),
        ("exponent with no digits", b"1e+\n1e-\n"),
        ("exponent letter only", b"e5\ne5\n"),
        ("comma decimal separator", b"1,5\n1,5\n"),
        ("leading whitespace", b"  12  \n  12  \n"),
        ("all C whitespace kinds", b"  \t\x0b\x0c\r 8\n \t8\n"),
        ("explicit plus", b"+5\n+5\n"),
        ("positive exponents", b"5e+2\n5E2\n"),
        ("negative exponent", b"5e-2\n5e-2\n"),
        ("hex integer", b"0x10\n0x10\n"),
        ("hex with binary exponent", b"0X1.8p3\n0X1.8p3\n"),
        ("hex prefix with no digits", b"0x\n0x\n"),
        ("hex prefix then junk", b"0xg\n0xg\n"),
        ("hex denormal", b"0x1p-149\n0x1p-149\n"),
        ("hex far below denormal", b"0x1p-200\n0x1p-200\n"),
        ("largest hex double", b"0x1.fffffffffffffp1023\n"),
        ("hex overflow", b"0x1p1024\n0x1p1024\n"),
        ("infinity", b"inf\ninf\n"),
        ("negative infinity", b"-inf\n-inf\n"),
        ("INFINITY spelled out", b"INFINITY\n-Infinity\n"),
        ("nan", b"nan\nnan\n"),
        ("NaN with parens and sign", b"NaN(xyz)\n-NAN\n"),
        ("hex-looking letters", b"FF\nFF\n"),
        ("decimal overflow to inf", b"1e40\n1e40\n"),
        ("double overflow to inf", b"1e1000\n-1e1000\n"),
        ("float max boundary", b"3.4028235e38\n3.4028236e38\n"),
        ("float min normal", b"1.17549435e-38\n1e-38\n"),
        ("simple values", b"7\n-1\n"),
        ("one hundred", b"100\n0.5\n"),
        ("small fraction", b"0.001\n0.001\n"),
        ("int boundaries", b"2147483647\n-2147483648\n"),
    ]);
}

// ---------------------------------------------------------------------------
// Phase C — inputs no earlier case reaches
// ---------------------------------------------------------------------------

/// `fgets` stops after 19 bytes; the remainder of a long line is what the
/// *next* `fgets` sees. This is the "maximum the code handles" boundary.
#[test]
fn buffer_boundary_paths() {
    assert_all(&[
        // 18 payload bytes + newline: exactly fills the buffer including '\n'.
        ("18 chars then newline", b"123456789012345678\n"),
        // 19 payload bytes: fgets stops before the newline, which the second
        // fgets then reads on its own.
        ("19 chars then newline", b"1234567890123456789\n"),
        ("19 chars, no newline", b"1234567890123456789"),
        // 20+ bytes: the tail of line 1 becomes the input of bad().
        ("25 digits, one line", b"1234567890123456789012345\n"),
        ("split across the boundary", b"0123456789012345678\x30123456789012345678\n"),
        ("tail is junk", b"01234567890123456789ZZZ\n"),
        // Leading zeros so the truncated prefix still parses.
        ("20 leading zeros", b"00000000000000000001\n"),
        ("21 leading zeros", b"000000000000000000001\n"),
        // Truncation lands inside an exponent.
        ("exponent cut in half", b"1.000000000000000e-40\n"),
        // A very long line: only the first 38 bytes are ever read.
        (
            "long line",
            b"9999999999999999999999999999999999999999999999999999\n",
        ),
        // More lines than the program reads.
        ("extra lines are ignored", b"4\n5\n6\n7\n8\n"),
    ]);
}

/// Bytes that are not text: `fgets` stores NULs, `atof` stops at them, and
/// non-UTF-8 bytes must not disturb anything.
#[test]
fn non_text_input_paths() {
    assert_all(&[
        ("NUL after digits", b"5\x00abc\n5\n"),
        ("leading NUL", b"\x005\n\x005\n"),
        ("NUL inside a number", b"2\x005\n2\x005\n"),
        ("only NULs", b"\x00\x00\n\x00\n"),
        ("invalid UTF-8", b"\xff\xfe\n\xff\n"),
        ("high bytes after a number", b"3\xc3\xa9\n3\xff\n"),
        ("CR LF line endings", b"6\r\n6\r\n"),
        ("CR only", b"\r\r"),
        ("form feed and vertical tab", b"\x0c\x0b\n\x0c\x0b\n"),
    ]);
}

/// A deterministic sweep over many numeric spellings, to catch rounding or
/// truncation differences the hand-written cases would miss.
#[test]
fn numeric_sweep() {
    // Simple xorshift so the sweep is reproducible without extra dependencies.
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    let mut cases: Vec<Vec<u8>> = Vec::new();
    for _ in 0..120 {
        let a = next();
        let b = next();
        // Exponents chosen to straddle the float and double limits as well as
        // the 1e-6 guard and the 2^31 int-cast boundary.
        let exp = (a % 96) as i64 - 48;
        let mantissa = b % 1_000_000_007;
        cases.push(format!("{mantissa}e{exp}\n{mantissa}e{exp}\n").into_bytes());
        cases.push(format!("-{mantissa}e{exp}\n{mantissa}.{a}e{exp}\n").into_bytes());
        cases.push(format!("0x{a:x}p{exp}\n0x{a:x}.{b:x}p{exp}\n").into_bytes());
    }
    for (i, input) in cases.iter().enumerate() {
        assert_same(&format!("sweep #{i}"), input);
    }
}

/// `main` ignores `argc`/`argv`, and `fgets` returning NULL because of a read
/// *error* (not EOF) must take the same branch as EOF.
#[test]
fn argv_and_stdin_error_paths() {
    // Arguments are accepted and ignored.
    let c = run_with(c_binary(), &["one", "two", "three"], Stdin::Bytes(b"8\n8\n"));
    let r = run_with(
        rust_binary(),
        &["one", "two", "three"],
        Stdin::Bytes(b"8\n8\n"),
    );
    compare("arguments are ignored", &c, &r);

    // stdin open on a directory: read(2) fails with EISDIR, so fgets returns
    // NULL without reaching end-of-file.
    let dir = workspace_root();
    let c = run_with(c_binary(), &[], Stdin::FromPath(&dir));
    let r = run_with(rust_binary(), &[], Stdin::FromPath(&dir));
    compare("stdin is a directory (read error)", &c, &r);

    // stdin at /dev/null: immediate EOF from a real file rather than a pipe.
    let dev_null = PathBuf::from("/dev/null");
    let c = run_with(c_binary(), &[], Stdin::FromPath(&dev_null));
    let r = run_with(rust_binary(), &[], Stdin::FromPath(&dev_null));
    compare("stdin is /dev/null", &c, &r);
}

/// stdin redirected from an empty source and closed early, which is how the
/// `fgets() failed.` branch is reached in practice.
#[test]
fn closed_stdin_paths() {
    // Nothing at all, then immediate EOF: `bad()` prints the failure message
    // and still performs the division by the untouched 0.0f.
    assert_same("stdin closed immediately", b"");
    // Exactly one line, so only the second read fails.
    assert_same("one line then EOF", b"25\n");
    assert_same("one unterminated line then EOF", b"25");
}
