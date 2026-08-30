//! Differential tests: run the original C program and the Rust translation as
//! subprocesses with identical stdin, then compare stdout, stderr and exit
//! status byte for byte.
//!
//! Nothing here links against the Rust crate as a library; both programs are
//! driven exactly the way a shell would drive them.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// Workspace root: the directory containing both `c_src/` and `translation/`.
fn repo_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")); // .../translation
    manifest
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Build `c_src` with CMake (once per test binary) and return the C executable.
fn c_binary() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = repo_root().join("c_src");
        let build_dir = c_src.join("build");
        std::fs::create_dir_all(&build_dir).expect("create c_src/build");

        let exe = build_dir.join("driver");
        if !exe.exists() {
            let configure = Command::new("cmake")
                .arg("..")
                .current_dir(&build_dir)
                .output()
                .expect("failed to run `cmake ..` (is cmake installed?)");
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
        assert!(exe.exists(), "C executable missing at {}", exe.display());
        exe
    })
    .as_path()
}

/// The Rust executable under test, built by cargo for this integration test.
fn rust_binary() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    status: Option<i32>,
}

fn run(program: &Path, stdin_bytes: &[u8]) -> Run {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", program.display()));

    {
        let mut sink = child.stdin.take().expect("stdin piped");
        // The program may exit without draining stdin (e.g. huge inputs); a
        // broken pipe there is not a test failure.
        let _ = sink.write_all(stdin_bytes);
        let _ = sink.flush();
    }

    let out = child.wait_with_output().expect("wait for child");
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        status: out.status.code(),
    }
}

fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).escape_debug().to_string()
}

/// Assert that both programs agree on stdout, stderr and exit status.
fn assert_same(label: &str, stdin_bytes: &[u8]) {
    let c = run(c_binary(), stdin_bytes);
    let r = run(rust_binary(), stdin_bytes);

    assert_eq!(
        c.stdout,
        r.stdout,
        "[{label}] stdout mismatch for stdin {:?}\n  C   : \"{}\"\n  Rust: \"{}\"",
        show(stdin_bytes),
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "[{label}] stderr mismatch for stdin {:?}\n  C   : \"{}\"\n  Rust: \"{}\"",
        show(stdin_bytes),
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        c.status,
        r.status,
        "[{label}] exit status mismatch for stdin {:?}: C {:?} vs Rust {:?}",
        show(stdin_bytes),
        c.status,
        r.status
    );
}

// ---------------------------------------------------------------------------
// The two top-level branches of main(): `if (x) good(); else bad();`
// ---------------------------------------------------------------------------

#[test]
fn empty_input_leaves_x_zero_and_takes_bad_branch() {
    // scanf() returns EOF, x stays 0 -> bad() -> helperBad() returns a dangling
    // pointer that the reference build turns into NULL -> printLine prints
    // nothing.
    assert_same("empty", b"");
}

#[test]
fn single_zero_takes_bad_branch() {
    assert_same("zero", b"0");
    assert_same("zero-newline", b"0\n");
    assert_same("minus-zero", b"-0");
    assert_same("plus-zero", b"+0");
    assert_same("many-zeros", b"0000000000");
}

#[test]
fn single_nonzero_takes_good_branch() {
    assert_same("one", b"1");
    assert_same("one-newline", b"1\n");
    assert_same("negative", b"-5");
    assert_same("explicit-plus", b"+7");
    assert_same("leading-zeros-then-digit", b"0000001");
}

// ---------------------------------------------------------------------------
// scanf("%d") matching failures: x is left at 0, so bad() runs.
// ---------------------------------------------------------------------------

#[test]
fn non_numeric_input_is_a_matching_failure() {
    for case in [
        &b"abc"[..],
        b"e5",
        b"xyz 1",
        b".5",
        b"/1",
        b":1",
        b"-",
        b"+",
        b"--7",
        b"++7",
        b"+-1",
        b"-+1",
        b"\x1b[A5",
    ] {
        assert_same("matching-failure", case);
    }
}

#[test]
fn whitespace_only_input_is_an_input_failure() {
    for case in [
        &b" "[..],
        b"\n",
        b"\t",
        b"\r",
        b"\x0b",
        b"\x0c",
        b"   \n\t\r\x0b\x0c   ",
    ] {
        assert_same("whitespace-only", case);
    }
}

#[test]
fn non_ascii_and_nul_bytes_are_matching_failures() {
    for case in [&b"\x00"[..], b"\x00 1", b"\x80", b"\xff\xfe", b"\xc3\xa9"] {
        assert_same("binary-garbage", case);
    }
}

// ---------------------------------------------------------------------------
// scanf reads across newlines and skips arbitrary leading whitespace, and it
// stops at the first non-digit (leaving the rest of stdin unread).
// ---------------------------------------------------------------------------

#[test]
fn scanf_skips_leading_whitespace_including_newlines() {
    assert_same("newlines-then-digit", b"\n\n\n  \t 5");
    assert_same("all-space-kinds", b"  \t\n\x0b\x0c\r 42\nmore text\n");
    assert_same("vertical-tab-then-digit", b" \x0b1");
}

#[test]
fn scanf_stops_at_first_non_digit_and_ignores_the_rest() {
    assert_same("digit-then-letters", b"1abc");
    assert_same("float-is-truncated", b"3.7");
    assert_same("zero-then-letters", b"0abc");
    assert_same("hex-literal-reads-as-zero", b"0x10");
    assert_same("digit-then-nul", b"1\x002");
    assert_same("negative-with-suffix", b"  -0000000000000005xyz");
    assert_same("trailing-lines", b"7\n8\n9\n");
    assert_same("zero-then-nonzero-line", b"0\n1\n");
}

// ---------------------------------------------------------------------------
// Integer width: truncation to `int` and glibc's overflow clamping.
// ---------------------------------------------------------------------------

#[test]
fn int_boundaries_round_trip() {
    for case in [
        &b"2147483647"[..],  // INT_MAX
        b"2147483648",       // INT_MAX + 1
        b"-2147483648",      // INT_MIN
        b"-2147483649",      // INT_MIN - 1
        b"-2147483647",
    ] {
        assert_same("int-boundary", case);
    }
}

#[test]
fn values_whose_low_32_bits_are_zero_take_the_bad_branch() {
    for case in [
        &b"4294967296"[..], // 2^32 -> truncates to 0
        b"-4294967296",
        b"8589934592",  // 2^33
        b"6442450944",  // 3 * 2^31
        b"18446744073709551616", // 2^64 (overflow clamp path)
    ] {
        assert_same("truncation", case);
    }
}

#[test]
fn overflowing_conversions_clamp_like_glibc() {
    for case in [
        &b"9223372036854775807"[..], // LONG_MAX
        b"9223372036854775808",      // LONG_MAX + 1 -> clamps
        b"-9223372036854775808",     // LONG_MIN -> truncates to 0
        b"-9223372036854775809",     // clamps to LONG_MIN -> 0
        b"10000000000000000000",
        b"99999999999999999999",
        b"-99999999999999999999",
    ] {
        assert_same("overflow", case);
    }
}

#[test]
fn very_long_digit_runs() {
    let pos = vec![b'9'; 400];
    assert_same("400-nines", &pos);

    let mut neg = vec![b'-'];
    neg.extend(std::iter::repeat(b'9').take(400));
    assert_same("400-nines-negative", &neg);

    let mut padded = vec![b'0'; 500];
    padded.push(b'1');
    assert_same("500-zeros-then-one", &padded);
}

#[test]
fn input_much_larger_than_a_stdio_buffer() {
    let big = vec![b'9'; 1 << 20];
    assert_same("1MiB-of-nines", &big);

    let mut big_zeros = vec![b'0'; 1 << 20];
    big_zeros.push(b'\n');
    assert_same("1MiB-of-zeros", &big_zeros);
}

// ---------------------------------------------------------------------------
// Exhaustive-ish sweep so no branch escapes: every combination of leading
// whitespace, sign and first character class.
// ---------------------------------------------------------------------------

#[test]
fn cross_product_of_whitespace_sign_and_body() {
    let leads: [&[u8]; 4] = [b"", b" ", b"\n\n", b" \t\r\n\x0b\x0c "];
    let signs: [&[u8]; 4] = [b"", b"+", b"-", b"--"];
    let bodies: [&[u8]; 8] = [b"", b"0", b"1", b"10", b"007", b"a", b"9", b"2147483648"];
    let tails: [&[u8]; 3] = [b"", b"\n", b"z\n1\n"];

    for lead in leads {
        for sign in signs {
            for body in bodies {
                for tail in tails {
                    let mut input = Vec::new();
                    input.extend_from_slice(lead);
                    input.extend_from_slice(sign);
                    input.extend_from_slice(body);
                    input.extend_from_slice(tail);
                    assert_same("cross-product", &input);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Invocation details that must not change behavior.
// ---------------------------------------------------------------------------

#[test]
fn extra_command_line_arguments_are_ignored() {
    let expect = |args: &[&str], stdin_bytes: &[u8]| {
        let mut c = Command::new(c_binary());
        let mut r = Command::new(rust_binary());
        c.args(args);
        r.args(args);
        for cmd in [&mut c, &mut r] {
            cmd.stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
        }
        let feed = |cmd: &mut Command| {
            let mut child = cmd.spawn().expect("spawn");
            let mut sink = child.stdin.take().unwrap();
            let _ = sink.write_all(stdin_bytes);
            drop(sink);
            child.wait_with_output().expect("wait")
        };
        let co = feed(&mut c);
        let ro = feed(&mut r);
        assert_eq!(co.stdout, ro.stdout, "stdout differs with args {args:?}");
        assert_eq!(co.stderr, ro.stderr, "stderr differs with args {args:?}");
        assert_eq!(
            co.status.code(),
            ro.status.code(),
            "exit status differs with args {args:?}"
        );
    };

    expect(&["foo", "bar"], b"1");
    expect(&["--help"], b"");
    expect(&["0"], b"0");
}

#[test]
fn output_is_identical_when_stdout_is_a_file_rather_than_a_pipe() {
    // C stdio is fully buffered to a file and line buffered to a terminal; the
    // Rust translation must produce the same bytes either way.
    let dir = std::env::temp_dir().join(format!("driver-difftest-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");

    for (name, input) in [("good", &b"1"[..]), ("bad", &b"0"[..]), ("eof", &b""[..])] {
        let mut outs = Vec::new();
        for (tag, bin) in [("c", c_binary()), ("rust", rust_binary())] {
            let in_path = dir.join(format!("{name}.in"));
            std::fs::write(&in_path, input).expect("write stdin file");
            let out_path = dir.join(format!("{name}.{tag}.out"));
            let status = Command::new(bin)
                .stdin(Stdio::from(std::fs::File::open(&in_path).unwrap()))
                .stdout(Stdio::from(std::fs::File::create(&out_path).unwrap()))
                .stderr(Stdio::piped())
                .spawn()
                .expect("spawn")
                .wait_with_output()
                .expect("wait");
            outs.push((
                std::fs::read(&out_path).expect("read stdout file"),
                status.stderr,
                status.status.code(),
            ));
        }
        assert_eq!(outs[0], outs[1], "file-redirected output differs for {name}");
    }

    let _ = std::fs::remove_dir_all(&dir);
}
