//! Differential tests: run the C reference binary and the Rust binary as
//! subprocesses over the same stdin and require byte-identical stdout,
//! byte-identical stderr, and an identical exit status.
//!
//! Nothing here links the Rust code as a library; both programs are driven the
//! way a shell drives them, which is how this translation is graded.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// Path to the Rust binary under test. Cargo hands us the freshly built one.
fn rust_binary() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// Path to the compiled C reference binary, building it via CMake on first use
/// so that `cargo test` alone is enough to run the suite.
///
/// Exact commands reproduced here:
///   cmake -S <repo>/c_src -B <repo>/c_src/build
///   cmake --build <repo>/c_src/build
/// producing <repo>/c_src/build/driver
fn c_binary() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("translation/ must have a parent directory")
            .join("c_src");
        let build_dir = c_src.join("build");
        let bin = build_dir.join("driver");

        if !bin.exists() {
            let configure = Command::new("cmake")
                .arg("-S")
                .arg(&c_src)
                .arg("-B")
                .arg(&build_dir)
                .output()
                .expect("failed to invoke `cmake` to configure the C reference build");
            assert!(
                configure.status.success(),
                "cmake configure failed:\n{}\n{}",
                String::from_utf8_lossy(&configure.stdout),
                String::from_utf8_lossy(&configure.stderr)
            );

            let build = Command::new("cmake")
                .arg("--build")
                .arg(&build_dir)
                .output()
                .expect("failed to invoke `cmake --build` for the C reference build");
            assert!(
                build.status.success(),
                "cmake build failed:\n{}\n{}",
                String::from_utf8_lossy(&build.stdout),
                String::from_utf8_lossy(&build.stderr)
            );
        }

        assert!(
            bin.exists(),
            "C reference binary missing after build: {}",
            bin.display()
        );
        bin
    })
}

struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Some(code)` for a normal exit, `None` if killed by a signal.
    code: Option<i32>,
}

/// Run `bin` with `input` piped to its stdin and collect everything it produced.
fn run(bin: &Path, input: &[u8]) -> Outcome {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    {
        let mut stdin = child.stdin.take().expect("stdin was piped");
        let owned = input.to_vec();
        // Write on a helper thread so a program that never drains stdin cannot
        // deadlock us on a full pipe buffer.
        std::thread::spawn(move || {
            let _ = stdin.write_all(&owned);
            let _ = stdin.flush();
        });
    }

    let out = child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("failed to wait on {}: {e}", bin.display()));

    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
    }
}

/// Assert the two programs are indistinguishable for `input`.
fn assert_identical(label: &str, input: &[u8]) {
    let c = run(c_binary(), input);
    let r = run(rust_binary(), input);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch for {label}\n  input : {}\n  C     : {}\n  Rust  : {}",
        hex(input),
        hex(&c.stdout),
        hex(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch for {label}\n  input : {}\n  C     : {}\n  Rust  : {}",
        hex(input),
        hex(&c.stderr),
        hex(&r.stderr)
    );
    assert_eq!(
        c.code, r.code,
        "exit status mismatch for {label}\n  input : {}\n  C     : {:?}\n  Rust  : {:?}",
        hex(input),
        c.code,
        r.code
    );

    // Guard against a harness that "passes" because neither program ran:
    // this program always prints a line and always exits 0.
    assert!(
        !c.stdout.is_empty(),
        "C reference produced no stdout for {label}; the harness is broken"
    );
    assert_eq!(
        c.code,
        Some(0),
        "C reference is expected to exit 0 for {label}"
    );
}

fn hex(bytes: &[u8]) -> String {
    if bytes.len() > 64 {
        format!(
            "{}... ({} bytes total)",
            bytes[..64].iter().map(|b| format!("{b:02x}")).collect::<String>(),
            bytes.len()
        )
    } else {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }
}

// ---------------------------------------------------------------------------
// The C program's entire observable input is the first byte of stdin, or the
// absence of one. Each test below pins an input class the C source branches on.
// ---------------------------------------------------------------------------

/// Empty stdin: `fscanf` matches nothing and returns EOF. The C code ignores
/// the return value, so `data` keeps its pre-initialised ' ' (0x20) and the
/// program prints 0x21. This is the "error path" of the only input operation.
#[test]
fn empty_stdin() {
    assert_identical("empty stdin", b"");
}

/// The same EOF path reached with a genuinely empty pipe rather than a
/// zero-length write.
#[test]
fn empty_stdin_matches_preinitialised_space_plus_one() {
    let c = run(c_binary(), b"");
    assert_eq!(
        c.stdout, b"21\n",
        "EOF must fall back to the pre-initialised ' ' + 1 == 0x21"
    );
    assert_identical("empty stdin (golden)", b"");
}

/// A single item: exactly one byte on stdin and nothing else.
#[test]
fn single_byte_printable() {
    for b in b'!'..=b'~' {
        assert_identical(&format!("single printable byte {b:#04x}"), &[b]);
    }
}

/// Exhaustive over the whole input alphabet: every possible first byte.
/// This is the maximum the code can distinguish, so it is fully enumerable.
#[test]
fn every_possible_first_byte() {
    for b in 0u8..=255 {
        assert_identical(&format!("first byte {b:#04x}"), &[b]);
    }
}

/// `%c` performs no whitespace skipping, unlike most scanf conversions.
/// A leading newline / space / tab must be consumed as data, not skipped.
#[test]
fn whitespace_is_data_not_skipped() {
    for b in [b'\n', b' ', b'\t', b'\r', 0x0b, 0x0c] {
        assert_identical(&format!("leading whitespace {b:#04x}"), &[b]);
        // ...and the byte after the whitespace must never be reached.
        assert_identical(&format!("whitespace {b:#04x} then 'Z'"), &[b, b'Z']);
    }
}

/// Signed-char boundaries. `char` is signed here, so `data + 1` truncated back
/// into a `char` wraps, and `%02x` then sign-extends through the default
/// promotion to `int`: 0x7f -> -128 -> "ffffff80", 0xff -> 0 -> "00".
#[test]
fn signed_char_overflow_and_sign_extension() {
    for b in [0x00u8, 0x7e, 0x7f, 0x80, 0x81, 0xfe, 0xff] {
        assert_identical(&format!("signedness boundary {b:#04x}"), &[b]);
    }
}

/// Pin the sign-extended widths as golden values so a translation that emitted
/// two-digit output (e.g. treating the char as unsigned) cannot slip through.
#[test]
fn sign_extension_golden_values() {
    let cases: [(u8, &[u8]); 6] = [
        (0x00, b"01\n"),
        (0x1f, b"20\n"),
        (0x7e, b"7f\n"),
        (0x7f, b"ffffff80\n"), // -127-1 wraps to -128, printed sign-extended
        (0xfe, b"ffffffff\n"),
        (0xff, b"00\n"), // -1 + 1 == 0
    ];
    for (input, expected) in cases {
        let c = run(c_binary(), &[input]);
        assert_eq!(
            c.stdout, expected,
            "C reference changed for input {input:#04x}"
        );
        let r = run(rust_binary(), &[input]);
        assert_eq!(r.stdout, expected, "Rust differs for input {input:#04x}");
    }
}

/// Only the first byte is ever consumed; trailing input is ignored and must not
/// change the output or leave the program blocked.
#[test]
fn only_first_byte_is_consumed() {
    let inputs: &[&[u8]] = &[
        b"AB",
        b"hello world",
        b"\nX",
        b"  a",
        b"0123456789",
        b"\x00\x01\x02",
        b"a\nb\nc\n",
    ];
    for input in inputs {
        assert_identical(&format!("multi-byte {}", hex(input)), input);
    }
}

/// Embedded NUL bytes are ordinary data for `%c`.
#[test]
fn nul_bytes() {
    assert_identical("leading NUL", b"\x00");
    assert_identical("NUL then text", b"\x00abc");
    assert_identical("text then NUL", b"a\x00bc");
}

/// Input far larger than any stdio buffer: still only the first byte matters,
/// and neither program may block, crash or change its exit status.
#[test]
fn very_large_input() {
    let big = vec![b'x'; 1 << 20];
    assert_identical("1 MiB of 'x'", &big);

    let mut mixed = vec![b'\n'];
    mixed.extend(std::iter::repeat(b'A').take(100_000));
    assert_identical("newline then 100k 'A'", &mixed);
}

/// Non-UTF-8 / high-bit byte sequences must be handled as raw bytes.
#[test]
fn invalid_utf8_input() {
    let inputs: &[&[u8]] = &[
        b"\xff\xfe\xfd",
        b"\xc3", // truncated 2-byte UTF-8 sequence
        b"\xed\xa0\x80", // encoded surrogate
        b"\x80\x80",
    ];
    for input in inputs {
        assert_identical(&format!("invalid utf-8 {}", hex(input)), input);
    }
}

/// stdin attached to /dev/null: EOF immediately, same as the empty case.
#[test]
fn stdin_from_null_device() {
    let mut results = Vec::new();
    for bin in [c_binary(), rust_binary()] {
        let out = Command::new(bin)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .unwrap_or_else(|e| panic!("failed to run {}: {e}", bin.display()));
        results.push(out);
    }
    assert_eq!(results[0].stdout, results[1].stdout, "stdout mismatch on /dev/null stdin");
    assert_eq!(results[0].stderr, results[1].stderr, "stderr mismatch on /dev/null stdin");
    assert_eq!(
        results[0].status.code(),
        results[1].status.code(),
        "exit status mismatch on /dev/null stdin"
    );
    assert_eq!(results[0].stdout, b"21\n");
}

/// Command-line arguments are declared away (`int main()`) and never inspected,
/// so passing some must not change anything.
#[test]
fn arguments_are_ignored() {
    for bin in [c_binary(), rust_binary()] {
        let out = Command::new(bin)
            .args(["--help", "-x", "garbage"])
            .stdin(Stdio::null())
            .output()
            .unwrap_or_else(|e| panic!("failed to run {}: {e}", bin.display()));
        assert_eq!(out.stdout, b"21\n", "{} reacted to argv", bin.display());
        assert!(out.stderr.is_empty(), "{} wrote to stderr", bin.display());
        assert_eq!(out.status.code(), Some(0));
    }
}

/// Neither program should ever write anything to stderr, on any input class.
#[test]
fn stderr_always_empty() {
    for b in 0u8..=255 {
        let c = run(c_binary(), &[b]);
        let r = run(rust_binary(), &[b]);
        assert!(c.stderr.is_empty(), "C wrote stderr for {b:#04x}");
        assert!(r.stderr.is_empty(), "Rust wrote stderr for {b:#04x}");
    }
    assert!(run(c_binary(), b"").stderr.is_empty());
    assert!(run(rust_binary(), b"").stderr.is_empty());
}

/// Exit status is unconditionally 0 (`return 0;`) for both programs.
#[test]
fn exit_status_always_zero() {
    let inputs: &[&[u8]] = &[b"", b"\x00", b"\x7f", b"\xff", b"\n", b"abc"];
    for input in inputs {
        assert_eq!(run(c_binary(), input).code, Some(0));
        assert_eq!(run(rust_binary(), input).code, Some(0));
    }
}
