//! Differential tests: run the original C binary and the Rust binary as
//! subprocesses on identical stdin, then compare stdout, stderr and exit
//! status byte for byte.
//!
//! Nothing here links the translation as a library. The Rust program is driven
//! exactly the way a shell drives the C program, because that is what is being
//! compared.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Path to the Rust binary produced by this crate.
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Path to the C binary built by `c_src/CMakeLists.txt`.
///
/// If it is missing, the CMake build is attempted once so the suite is
/// self-contained; a still-missing binary is a hard failure, never a skip.
fn c_bin() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf();
    let c_src = root.join("c_src");
    let build = c_src.join("build");
    let bin = build.join("driver");
    if bin.is_file() {
        return bin;
    }

    std::fs::create_dir_all(&build).expect("could not create c_src/build");
    let configure = Command::new("cmake")
        .arg("..")
        .current_dir(&build)
        .output()
        .expect("failed to run `cmake ..` — is cmake installed?");
    assert!(
        configure.status.success(),
        "cmake configure failed:\n{}",
        String::from_utf8_lossy(&configure.stderr)
    );
    let compile = Command::new("cmake")
        .args(["--build", "."])
        .current_dir(&build)
        .output()
        .expect("failed to run `cmake --build .`");
    assert!(
        compile.status.success(),
        "cmake build failed:\n{}",
        String::from_utf8_lossy(&compile.stderr)
    );

    assert!(
        bin.is_file(),
        "C reference binary still absent at {}",
        bin.display()
    );
    bin
}

/// What one program did with one input.
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
        .unwrap_or_else(|e| panic!("could not spawn {}: {e}", bin.display()));

    {
        let mut sink = child.stdin.take().expect("stdin was piped");
        // The program reads at most one byte and may exit before the write
        // completes, so a short or broken write is not an error here.
        let _ = sink.write_all(stdin_bytes);
        let _ = sink.flush();
    }

    let out = child.wait_with_output().expect("child did not terminate");
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
    }
}

/// Render bytes so a failure message is readable for non-UTF-8 output.
fn show(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\t' => s.push_str("\\t"),
            b'\\' => s.push_str("\\\\"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    s
}

/// Assert the two programs are indistinguishable on `input`.
fn assert_same(label: &str, input: &[u8]) {
    let c = run(&c_bin(), input);
    let r = run(&rust_bin(), input);

    assert_eq!(
        show(&c.stdout),
        show(&r.stdout),
        "stdout differs for {label} (input {})",
        show(input)
    );
    assert_eq!(
        c.stdout, r.stdout,
        "stdout differs for {label} (input {})",
        show(input)
    );
    assert_eq!(
        show(&c.stderr),
        show(&r.stderr),
        "stderr differs for {label} (input {})",
        show(input)
    );
    assert_eq!(
        c.stderr, r.stderr,
        "stderr differs for {label} (input {})",
        show(input)
    );
    assert_eq!(
        c.code, r.code,
        "exit status differs for {label} (input {}): C={:?} Rust={:?}",
        show(input),
        c.code,
        r.code
    );
}

// ---------------------------------------------------------------------------
// EOF path: `getchar()` returns -1, which `char c = getchar()` stores as -1.
// ---------------------------------------------------------------------------

#[test]
fn empty_input_hits_eof() {
    assert_same("empty stdin (EOF)", b"");
}

// ---------------------------------------------------------------------------
// Single byte: the only value the program ever classifies.
// ---------------------------------------------------------------------------

#[test]
fn every_single_byte_value() {
    for b in 0u8..=255 {
        assert_same(&format!("single byte 0x{b:02x}"), &[b]);
    }
}

// ---------------------------------------------------------------------------
// Named boundary cases. Each is a distinct region of glibc's "C"-locale ctype
// table, so a table error shows up as a specific failing test rather than as
// one opaque sweep failure.
// ---------------------------------------------------------------------------

#[test]
fn nul_byte() {
    // A NUL first byte: control, and `printf("%c")` emits a literal NUL.
    assert_same("NUL", b"\0");
    assert_same("NUL followed by text", b"\0abc");
}

#[test]
fn control_characters() {
    for &b in &[0x01u8, 0x07, 0x08, 0x0e, 0x1a, 0x1f, 0x7f] {
        assert_same(&format!("control 0x{b:02x}"), &[b]);
    }
}

#[test]
fn whitespace_family() {
    // 0x09 is blank+space; 0x0a..=0x0d are space but not blank; 0x20 is both
    // blank and printing but not graphical.
    for &b in &[0x09u8, 0x0a, 0x0b, 0x0c, 0x0d, 0x20] {
        assert_same(&format!("whitespace 0x{b:02x}"), &[b]);
    }
}

#[test]
fn print_graph_boundaries() {
    // 0x1f/0x20 straddle the printing boundary; 0x20/0x21 the graphical one;
    // 0x7e/0x7f the top of both.
    for &b in &[0x1fu8, 0x20, 0x21, 0x7e, 0x7f] {
        assert_same(&format!("boundary 0x{b:02x}"), &[b]);
    }
}

#[test]
fn digits_and_their_neighbours() {
    for &b in b"/0123456789:" {
        assert_same(&format!("digit region {:?}", b as char), &[b]);
    }
}

#[test]
fn hex_digit_letters_and_their_neighbours() {
    // 'F'/'G' and 'f'/'g' are the hexadecimal cut-offs.
    for &b in b"ABCDEFGabcdefg" {
        assert_same(&format!("hex region {:?}", b as char), &[b]);
    }
}

#[test]
fn alphabetic_case_boundaries() {
    // '@'/'A', 'Z'/'[', '`'/'a', 'z'/'{' bracket the two letter ranges and
    // drive tolower/toupper.
    for &b in b"@AZ[`az{" {
        assert_same(&format!("alpha boundary {:?}", b as char), &[b]);
    }
}

#[test]
fn punctuation() {
    for &b in b"!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~" {
        assert_same(&format!("punctuation {:?}", b as char), &[b]);
    }
}

#[test]
fn high_bytes_arrive_as_negative_chars() {
    // `char` is signed here, so 0x80..=0xff sign-extend to -128..=-1 when
    // promoted to the `int` parameter of the ctype functions. They classify as
    // nothing and both case conversions are the identity, which means
    // `printf("%c")` echoes the original byte back.
    for &b in &[0x80u8, 0x81, 0xa0, 0xc0, 0xc3, 0xe9, 0xfe, 0xff] {
        assert_same(&format!("high byte 0x{b:02x}"), &[b]);
    }
}

#[test]
fn byte_0xff_is_not_confused_with_eof() {
    // 0xff stored in a signed `char` is -1, the same value EOF produces, so
    // these two inputs must yield identical output — and both must be the
    // "nothing classifies" output.
    let eof = run(&rust_bin(), b"");
    let ff = run(&rust_bin(), b"\xff");
    assert_eq!(eof.stdout, ff.stdout, "0xff and EOF must coincide");
    assert_same("0xff", b"\xff");
}

// ---------------------------------------------------------------------------
// Only the first byte is ever read; the rest of stdin is irrelevant.
// ---------------------------------------------------------------------------

#[test]
fn only_the_first_byte_matters() {
    assert_same("word", b"Zebra");
    assert_same("word with newline", b"Zebra\n");
    assert_same("leading space then text", b" leading");
    assert_same("leading newline then text", b"\nsecond line\n");
    assert_same("leading tab", b"\tindented\n");
    assert_same("multi-line", b"first\nsecond\nthird\n");
    assert_same("utf-8 multibyte first char", "\u{e9}clair".as_bytes());
    assert_same("digits then letters", b"42abc");
}

#[test]
fn large_input_is_not_drained_differently() {
    // 256 KiB is well past any stdio buffer; the program still reads one byte
    // and exits, leaving the writer with a broken pipe on both sides.
    let mut big = vec![b'q'; 256 * 1024];
    big[0] = b'Q';
    assert_same("256 KiB input", &big);
}

#[test]
fn stdin_at_eof_from_dev_null_equivalent() {
    // A zero-length pipe is the same observable situation as /dev/null.
    assert_same("zero-length pipe", b"");
}
