//! Differential tests: run the original C program and the Rust translation as
//! subprocesses with identical stdin, and require byte-identical stdout,
//! byte-identical stderr, and the same exit status.
//!
//! The Rust code is NEVER called as a library here — only the built binary is
//! driven, exactly the way a shell would drive it.
//!
//! What the C program does (c_src/src/main.c):
//!
//! ```c
//! void printHexCharLine(char charHex) { printf("%02x\n", charHex); }
//!
//! int main() {
//!     char data;
//!     data = ' ';
//!     fscanf(stdin, "%c", &data);
//!     { char result = data + 1; printHexCharLine(result); }
//!     return 0;
//! }
//! ```
//!
//! Input classes that the behavior actually branches on:
//!   * the `%c` conversion succeeds  -> `data` is the first byte of stdin
//!   * the `%c` conversion fails     -> `data` keeps its initialized value ' '
//!     (empty stdin, or stdin that cannot be read at all)
//!   * `data + 1` stays in `signed char` range -> plain two-hex-digit output
//!   * `data + 1` overflows at data == 0x7f    -> wraps to -128
//!   * `data` is negative (0x80..=0xff)        -> the char is promoted to a
//!     negative `int`, and `%x` reinterprets that int as `unsigned`, so the
//!     output is eight hex digits ("ffffff80"), not two. The `02` is only a
//!     MINIMUM field width, so it never truncates.

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::Once;

/// Path to the Rust binary under test. Cargo builds it for us and hands us the
/// path through this env var, so this is always the binary from this crate.
const RUST_BIN: &str = env!("CARGO_BIN_EXE_driver");

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Build the C program with CMake (once per test binary) and return its path.
fn c_bin() -> PathBuf {
    static BUILD: Once = Once::new();
    let root = repo_root();
    let c_src = root.join("c_src");
    let build_dir = c_src.join("build");
    let exe = build_dir.join("driver");

    BUILD.call_once(|| {
        if exe.exists() {
            return;
        }
        std::fs::create_dir_all(&build_dir).expect("create c_src/build");
        let conf = Command::new("cmake")
            .arg("..")
            .current_dir(&build_dir)
            .output()
            .expect("run `cmake ..` (is cmake installed?)");
        assert!(
            conf.status.success(),
            "cmake configure failed:\n{}\n{}",
            String::from_utf8_lossy(&conf.stdout),
            String::from_utf8_lossy(&conf.stderr)
        );
        let built = Command::new("cmake")
            .args(["--build", "."])
            .current_dir(&build_dir)
            .output()
            .expect("run `cmake --build .`");
        assert!(
            built.status.success(),
            "cmake build failed:\n{}\n{}",
            String::from_utf8_lossy(&built.stdout),
            String::from_utf8_lossy(&built.stderr)
        );
    });

    assert!(
        exe.exists(),
        "C binary was not produced at {}",
        exe.display()
    );
    exe
}

/// Run `program` with `input` piped to its stdin, capturing stdout and stderr.
fn run_with_stdin(program: impl AsRef<Path>, input: &[u8]) -> Output {
    let program = program.as_ref();
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", program.display()));

    // Write on a helper thread so a program that never drains stdin (this one
    // reads a single byte) cannot deadlock us on a large input.
    let mut stdin = child.stdin.take().expect("piped stdin");
    let owned = input.to_vec();
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&owned);
        let _ = stdin.flush();
        // dropping `stdin` closes the pipe, signalling EOF
    });

    let out = child.wait_with_output().expect("wait_with_output");
    let _ = writer.join();
    out
}

/// Run `program` with its stdin redirected from an already-open file/handle.
fn run_with_stdin_handle(program: impl AsRef<Path>, stdin: Stdio) -> Output {
    let program = program.as_ref();
    Command::new(program)
        .stdin(stdin)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", program.display()))
}

fn describe(input: &[u8]) -> String {
    if input.len() > 32 {
        format!("<{} bytes>", input.len())
    } else {
        format!("{:02x?}", input)
    }
}

fn assert_same_output(label: &str, c: &Output, r: &Output) {
    assert_eq!(
        c.stdout,
        r.stdout,
        "[{label}] stdout differs\n  C   : {:02x?} ({:?})\n  Rust: {:02x?} ({:?})",
        c.stdout,
        String::from_utf8_lossy(&c.stdout),
        r.stdout,
        String::from_utf8_lossy(&r.stdout),
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "[{label}] stderr differs\n  C   : {:02x?} ({:?})\n  Rust: {:02x?} ({:?})",
        c.stderr,
        String::from_utf8_lossy(&c.stderr),
        r.stderr,
        String::from_utf8_lossy(&r.stderr),
    );
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "[{label}] exit status differs: C={:?} Rust={:?}",
        c.status,
        r.status
    );
}

/// Compare both programs on one stdin payload: stdout, stderr and exit status.
#[track_caller]
fn assert_same(input: &[u8]) -> Output {
    let label = describe(input);
    let c = run_with_stdin(&c_bin(), input);
    let r = run_with_stdin(RUST_BIN, input);
    assert_same_output(&label, &c, &r);
    c
}

/// Compare, and additionally pin the exact expected stdout so the test still
/// has teeth if both implementations were to drift together.
#[track_caller]
fn assert_same_and_stdout(input: &[u8], expected_stdout: &str) {
    let out = assert_same(input);
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        expected_stdout,
        "unexpected stdout for input {}",
        describe(input)
    );
    assert!(
        out.stderr.is_empty(),
        "expected empty stderr for input {}",
        describe(input)
    );
    assert_eq!(out.status.code(), Some(0), "expected exit 0");
}

// ---------------------------------------------------------------------------
// The `%c` conversion fails: `data` keeps its initialized ' ' (0x20),
// so result == 0x21.
// ---------------------------------------------------------------------------

#[test]
fn empty_stdin_keeps_initialized_space() {
    assert_same_and_stdout(b"", "21\n");
}

#[test]
fn stdin_from_dev_null_is_eof() {
    let c = run_with_stdin_handle(&c_bin(), Stdio::null());
    let r = run_with_stdin_handle(RUST_BIN, Stdio::null());
    assert_same_output("stdin=/dev/null", &c, &r);
    assert_eq!(String::from_utf8_lossy(&c.stdout), "21\n");
}

#[test]
fn stdin_that_cannot_be_read_is_a_failed_conversion() {
    // On Linux, opening a directory succeeds but read(2) fails with EISDIR.
    // fscanf's conversion therefore fails and `data` stays ' ' -> "21".
    let dir = repo_root();
    let c = run_with_stdin_handle(
        &c_bin(),
        Stdio::from(File::open(&dir).expect("open directory as stdin")),
    );
    let r = run_with_stdin_handle(
        RUST_BIN,
        Stdio::from(File::open(&dir).expect("open directory as stdin")),
    );
    assert_same_output("stdin=directory", &c, &r);
    assert_eq!(String::from_utf8_lossy(&c.stdout), "21\n");
    assert!(c.stderr.is_empty());
    assert_eq!(c.status.code(), Some(0));
}

// ---------------------------------------------------------------------------
// Single item: the ordinary, non-negative range 0x00..=0x7e -> two hex digits.
// ---------------------------------------------------------------------------

#[test]
fn single_byte_happy_path() {
    assert_same_and_stdout(b"A", "42\n"); // 0x41 + 1
    assert_same_and_stdout(b"a", "62\n"); // 0x61 + 1
    assert_same_and_stdout(b"0", "31\n"); // 0x30 + 1
    assert_same_and_stdout(b" ", "21\n"); // 0x20 + 1
}

#[test]
fn nul_byte_is_read_like_any_other_byte() {
    // The lowest input: 0x00 + 1 == 0x01 -> the "02" minimum width pads it.
    assert_same_and_stdout(b"\x00", "01\n");
}

#[test]
fn low_and_high_ends_of_the_positive_range() {
    assert_same_and_stdout(b"\x01", "02\n");
    assert_same_and_stdout(b"\x1f", "20\n");
    assert_same_and_stdout(b"\x7e", "7f\n"); // largest input with 2-digit output
}

// ---------------------------------------------------------------------------
// The signed-char overflow boundary: 0x7f + 1 wraps to -128, which is promoted
// to the int -128 and printed by %x as the unsigned 0xffffff80.
// ---------------------------------------------------------------------------

#[test]
fn signed_char_overflow_at_0x7f() {
    assert_same_and_stdout(b"\x7f", "ffffff80\n");
}

// ---------------------------------------------------------------------------
// Negative chars (0x80..=0xff): sign-extended to int, reinterpreted as
// unsigned by %x, so eight hex digits -- except 0xff, which wraps to 0.
// ---------------------------------------------------------------------------

#[test]
fn negative_chars_sign_extend_to_eight_hex_digits() {
    assert_same_and_stdout(b"\x80", "ffffff81\n");
    assert_same_and_stdout(b"\x81", "ffffff82\n");
    assert_same_and_stdout(b"\xc8", "ffffffc9\n");
    assert_same_and_stdout(b"\xfe", "ffffffff\n");
}

#[test]
fn byte_0xff_wraps_back_to_zero() {
    // -1 + 1 == 0, so the "02" minimum width pads it to "00".
    assert_same_and_stdout(b"\xff", "00\n");
}

#[test]
fn invalid_utf8_input_is_handled_as_raw_bytes() {
    // A lone continuation byte and a truncated multi-byte sequence: the C reads
    // raw bytes, so the Rust must not require valid UTF-8 on stdin.
    assert_same_and_stdout(b"\x9f", "ffffffa0\n");
    assert_same_and_stdout(b"\xe2\x82", "ffffffe3\n");
    assert_same(b"\xff\xfe\xfd");
}

// ---------------------------------------------------------------------------
// `fscanf("%c")` does NOT skip leading whitespace and does NOT stop at
// newlines -- unlike "%d"/"%s" and unlike fgets. The very first byte wins.
// ---------------------------------------------------------------------------

#[test]
fn leading_whitespace_is_not_skipped() {
    assert_same_and_stdout(b"   x", "21\n"); // reads the space, not 'x'
    assert_same_and_stdout(b"\tx", "0a\n"); // 0x09 + 1
    assert_same_and_stdout(b"\n", "0b\n"); // 0x0a + 1
    assert_same_and_stdout(b"\nX", "0b\n"); // newline is a real conversion
    assert_same_and_stdout(b"\r\n", "0e\n"); // 0x0d + 1
    assert_same_and_stdout(b"\x0bx", "0c\n"); // vertical tab
    assert_same_and_stdout(b"\x0cx", "0d\n"); // form feed
}

#[test]
fn only_the_first_byte_of_a_longer_input_matters() {
    assert_same_and_stdout(b"ABC", "42\n");
    assert_same_and_stdout(b"A\nB\nC\n", "42\n");
    assert_same_and_stdout(b"hello world\n", "69\n"); // 'h' == 0x68
    assert_same_and_stdout(b"\x7f\x7f\x7f", "ffffff80\n");
}

#[test]
fn trailing_newline_presence_does_not_change_anything() {
    assert_same_and_stdout(b"A", "42\n");
    assert_same_and_stdout(b"A\n", "42\n");
}

// ---------------------------------------------------------------------------
// Volume: far more input than the program consumes. There is no buffer to
// overflow, but this pins that the extra input is simply ignored and that
// neither program blocks or errors on an unread pipe.
// ---------------------------------------------------------------------------

#[test]
fn very_large_input_is_ignored_after_the_first_byte() {
    let mut big = vec![b'Z'; 1 << 20]; // 1 MiB, larger than any stdio buffer
    big[0] = b'Q'; // 0x51 + 1 == 0x52
    assert_same_and_stdout(&big, "52\n");
}

#[test]
fn large_all_nul_input() {
    assert_same_and_stdout(&vec![0u8; 5000], "01\n");
}

#[test]
fn large_input_starting_with_a_negative_byte() {
    let mut big = vec![0xffu8; 100_000];
    big[0] = 0x80;
    assert_same_and_stdout(&big, "ffffff81\n");
}

// ---------------------------------------------------------------------------
// Exhaustive: every possible first byte. This is the complete input space of
// the `%c` conversion, so together with the empty-stdin case above it covers
// every reachable path through the program.
// ---------------------------------------------------------------------------

#[test]
fn exhaustive_every_single_byte_value() {
    let c = c_bin();
    for b in 0u8..=255 {
        let input = [b];
        let co = run_with_stdin(&c, &input);
        let ro = run_with_stdin(RUST_BIN, &input);
        assert_same_output(&format!("byte 0x{b:02x}"), &co, &ro);

        // Independently recompute what C must print, from the C semantics:
        // (signed char)b + 1 truncated to signed char, promoted to int,
        // printed with %02x as an unsigned int.
        let result = (b as i8).wrapping_add(1);
        let expected = format!("{:02x}\n", result as i32 as u32);
        assert_eq!(
            String::from_utf8_lossy(&co.stdout),
            expected,
            "C output for byte 0x{b:02x} was not the expected %02x of the promoted char"
        );
        assert!(co.stderr.is_empty(), "byte 0x{b:02x}: C wrote to stderr");
        assert_eq!(co.status.code(), Some(0), "byte 0x{b:02x}: C exit status");
    }
}

// ---------------------------------------------------------------------------
// Every byte value, this time as the first byte of a two-byte input, to prove
// the second byte can never influence the result.
// ---------------------------------------------------------------------------

#[test]
fn exhaustive_first_byte_with_a_trailing_byte() {
    let c = c_bin();
    for b in 0u8..=255 {
        let input = [b, b'\n'];
        let co = run_with_stdin(&c, &input);
        let ro = run_with_stdin(RUST_BIN, &input);
        assert_same_output(&format!("byte 0x{b:02x} + newline"), &co, &ro);

        let single = run_with_stdin(&c, &[b]);
        assert_eq!(
            co.stdout, single.stdout,
            "byte 0x{b:02x}: trailing newline changed the C output"
        );
    }
}
