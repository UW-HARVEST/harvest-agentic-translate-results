//! Equivalence test for the C and Rust `driver` binaries.
//!
//! Both implementations are executables that read a single byte from stdin
//! and print 14 lines describing that character's classification and case.
//! We spawn both binaries with identical stdin payloads and assert their
//! stdout (and stderr, exit status) match byte-for-byte.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_binary() -> PathBuf {
    workspace_root().join("c_src/build/driver")
}

fn rust_binary() -> PathBuf {
    // Use release build for parity. cargo test builds the integration test
    // binary, but the `driver` binary is built separately; we kick a build
    // off here lazily via cargo to ensure it's present.
    let release = workspace_root().join("target/release/driver");
    if !release.exists() {
        let status = Command::new(env!("CARGO"))
            .args(["build", "--release", "--bin", "driver"])
            .current_dir(workspace_root())
            .status()
            .expect("failed to spawn cargo build");
        assert!(status.success(), "cargo build --release failed");
    }
    release
}

fn ensure_c_built() {
    let bin = c_binary();
    if bin.exists() {
        return;
    }
    let c_src = workspace_root().join("c_src");
    let build_dir = c_src.join("build");
    std::fs::create_dir_all(&build_dir).unwrap();

    let status = Command::new("cmake")
        .args([".."])
        .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
        .current_dir(&build_dir)
        .status()
        .expect("failed to spawn cmake");
    assert!(status.success(), "cmake configure failed");

    let status = Command::new("cmake")
        .args(["--build", "."])
        .current_dir(&build_dir)
        .status()
        .expect("failed to spawn cmake --build");
    assert!(status.success(), "cmake build failed");
    assert!(bin.exists(), "C driver binary missing after build");
}

fn run_with_input(bin: &Path, input: &[u8]) -> (Vec<u8>, Vec<u8>, Option<i32>) {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {}", bin.display(), e));

    if !input.is_empty() {
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(input)
            .expect("failed to write to stdin");
    }
    // Closing stdin happens on drop of child.stdin when we call wait_with_output.
    let output = child.wait_with_output().expect("failed to read child output");
    (output.stdout, output.stderr, output.status.code())
}

fn assert_match(input: &[u8]) {
    ensure_c_built();
    let c_bin = c_binary();
    let rust_bin = rust_binary();

    let (c_out, c_err, c_code) = run_with_input(&c_bin, input);
    let (r_out, r_err, r_code) = run_with_input(&rust_bin, input);

    if c_out != r_out {
        panic!(
            "stdout mismatch for input {:?}\n--- C ---\n{}\n--- Rust ---\n{}\n",
            input,
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
    }
    assert_eq!(c_err, r_err, "stderr mismatch for input {:?}", input);
    assert_eq!(c_code, r_code, "exit code mismatch for input {:?}", input);
}

#[test]
fn empty_stdin_eof() {
    // getchar() returns EOF (-1); assignment to char yields 0xFF.
    assert_match(b"");
}

#[test]
fn ascii_letters_and_digits() {
    let inputs: &[&[u8]] = &[
        b"A", b"Z", b"a", b"z", b"0", b"9", b"5", b"M", b"m",
    ];
    for input in inputs {
        assert_match(input);
    }
}

#[test]
fn ascii_punctuation_and_specials() {
    let inputs: &[&[u8]] = &[
        b" ", b"\t", b"\n", b"\r", b"!", b"@", b"#", b"$", b"%",
        b"^", b"&", b"*", b"(", b")", b"_", b"-", b"+", b"=",
        b"[", b"]", b"{", b"}", b";", b":", b",", b".", b"?",
        b"/", b"\\", b"|", b"~", b"`", b"'", b"\"",
    ];
    for input in inputs {
        assert_match(input);
    }
}

#[test]
fn ascii_control_chars() {
    // Each byte in 0x00..=0x1F plus 0x7F.
    for b in 0u8..=0x1F {
        assert_match(&[b]);
    }
    assert_match(&[0x7F]);
}

#[test]
fn high_bit_bytes() {
    // The C code stores getchar()'s return in `char`; on signed-char x86_64,
    // bytes 0x80..=0xFF become negative when promoted to int. The Rust
    // implementation must replicate this — including the fact that the
    // glibc ctype tables are valid for indices -1..=255.
    for b in 0x80u8..=0xFFu8 {
        assert_match(&[b]);
    }
}

#[test]
fn extra_input_is_ignored() {
    // Both implementations only consume one byte. Anything afterwards is
    // ignored — we just verify they still match.
    assert_match(b"abcdef");
    assert_match(b"\x00\x01\x02");
}
