// Integration test that runs both the C binary and the Rust binary with
// identical stdin input, then compares their stdout byte-for-byte.
//
// The C source compiles to an executable (see c_src/CMakeLists.txt), and the
// Rust translation is also a binary (see Cargo.toml). Neither produces a
// shared library, so there is no FFI boundary in this project — the only
// observable behavior is the program's stdout. Comparing stdout from both
// executables therefore exercises every line of code in `foo`, `driver`, and
// `main`.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR points at translated_rust/.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_binary() -> PathBuf {
    workspace_root().join("c_src").join("build").join("driver")
}

fn rust_binary() -> PathBuf {
    // Built by `cargo build --release` (or the default test profile).
    // Cargo sets CARGO_BIN_EXE_<name> for binaries declared in [[bin]] when
    // running tests, which is the canonical way to locate them.
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn run_with_stdin(bin: &PathBuf, input: &[u8]) -> Vec<u8> {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {:?}: {}", bin, e));
    {
        let stdin = child.stdin.as_mut().expect("no stdin");
        stdin.write_all(input).expect("write stdin");
    }
    let output = child.wait_with_output().expect("wait");
    assert!(
        output.status.success(),
        "binary {:?} exited with {:?}; stderr: {}",
        bin,
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn assert_match(input: &[u8], label: &str) {
    let c_out = run_with_stdin(&c_binary(), input);
    let r_out = run_with_stdin(&rust_binary(), input);
    assert_eq!(
        c_out, r_out,
        "stdout mismatch for case {}: C={:?} Rust={:?}",
        label,
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out)
    );
}

#[test]
fn empty_input() {
    assert_match(b"", "empty");
}

#[test]
fn no_match() {
    assert_match(b"hello world", "no_match");
}

#[test]
fn only_a() {
    assert_match(b"AAAA", "only_a");
}

#[test]
fn only_x() {
    assert_match(b"xxxxx", "only_x");
}

#[test]
fn mixed() {
    assert_match(b"AxAxAx", "mixed");
}

#[test]
fn case_sensitivity() {
    // Lowercase 'a' should not be counted as 'A'; uppercase 'X' should not
    // be counted as 'x'.
    assert_match(b"aaaXXX", "case_sensitivity");
}

#[test]
fn newlines_and_whitespace() {
    assert_match(b"A\nA\tA xx\nx", "newlines");
}

#[test]
fn embedded_null_truncates() {
    // The C `foo` uses `strchr`, which stops at the first NUL byte.
    // After the NUL, characters should not be counted.
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"AAA");
    buf.push(0);
    buf.extend_from_slice(b"AAA"); // these should NOT be counted
    assert_match(&buf, "embedded_null");
}

#[test]
fn long_input_under_buffer() {
    // 999 bytes (one less than the C buffer of 1000) — stays inside the
    // fixed-size buffer with room for a terminating NUL.
    let mut buf = Vec::with_capacity(999);
    for i in 0..999 {
        // Mix in a few 'A's and 'x's at deterministic positions.
        let c = match i % 7 {
            0 => b'A',
            1 => b'x',
            2 => b'B',
            3 => b'y',
            4 => b'A',
            5 => b'.',
            _ => b'-',
        };
        buf.push(c);
    }
    assert_match(&buf, "long_under");
}

#[test]
fn exactly_buffer_size() {
    // Exactly 1000 bytes — fread will fill the buffer; in C the buffer is
    // initialized to zero so the last byte stays 0 only if input < 1000.
    // Here we provide 1000 bytes, so there is no trailing zero from init,
    // but the buffer being 1000 wide and `foo` walks via strchr will only
    // stop at an actual NUL byte. We include a NUL near the end to ensure
    // strchr terminates within the buffer.
    let mut buf = vec![b'A'; 1000];
    buf[500] = 0;
    // Bytes 0..500 are 'A'; bytes 501..1000 are 'A' but should not be
    // counted because of the NUL at index 500.
    assert_match(&buf, "exact_buffer");
}

#[test]
fn over_buffer_size() {
    // More than 1000 bytes — the C reads at most 1000 via fread, so extra
    // input is ignored. Verify both implementations agree.
    let buf = vec![b'A'; 2000];
    assert_match(&buf, "over_buffer");
}

#[test]
fn binary_garbage() {
    // Non-printable bytes including high bits set.
    let buf: Vec<u8> = (1u8..=255u8).collect(); // skip 0 to avoid early stop
    assert_match(&buf, "binary_garbage");
}
