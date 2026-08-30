//! Differential tests: run the original C `driver` and the Rust `driver` as
//! subprocesses with identical stdin, and require byte-identical stdout,
//! byte-identical stderr, and an identical exit status.
//!
//! Nothing here links against the Rust code as a library; both programs are
//! driven exactly the way a shell drives them.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Path to the Rust binary under test (Cargo builds it for us).
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn repo_root() -> PathBuf {
    // .../translation -> ...
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// Path to the C binary, building it with CMake on first use.
fn c_bin() -> PathBuf {
    let c_src = repo_root().join("c_src");
    let build = c_src.join("build");
    let bin = build.join("driver");
    if bin.exists() {
        return bin;
    }

    std::fs::create_dir_all(&build).expect("create c_src/build");
    let cfg = Command::new("cmake")
        .arg("..")
        .current_dir(&build)
        .output()
        .expect("run cmake (is cmake installed?)");
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
        .expect("run cmake --build");
    assert!(
        bld.status.success(),
        "cmake build failed:\n{}\n{}",
        String::from_utf8_lossy(&bld.stdout),
        String::from_utf8_lossy(&bld.stderr)
    );
    assert!(bin.exists(), "C binary missing after build: {}", bin.display());
    bin
}

struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    status: Option<i32>,
    signal: Option<i32>,
}

fn run(bin: &Path, stdin_bytes: &[u8]) -> Run {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", bin.display()));

    {
        let mut si = child.stdin.take().expect("stdin pipe");
        // The programs read at most one byte, so the write may fail with
        // EPIPE once they exit; that is not a test failure.
        let _ = si.write_all(stdin_bytes);
        let _ = si.flush();
    }

    let out = child.wait_with_output().expect("wait for child");

    #[cfg(unix)]
    let signal = {
        use std::os::unix::process::ExitStatusExt;
        out.status.signal()
    };
    #[cfg(not(unix))]
    let signal: Option<i32> = None;

    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        status: out.status.code(),
        signal,
    }
}

fn show(b: &[u8]) -> String {
    let mut s = String::new();
    for &c in b {
        match c {
            b'\n' => s.push_str("\\n"),
            0x20..=0x7e => s.push(c as char),
            _ => s.push_str(&format!("\\x{c:02x}")),
        }
    }
    s
}

/// The core assertion: identical stdout, stderr and exit status.
#[track_caller]
fn assert_same(label: &str, stdin_bytes: &[u8]) {
    let c = run(&c_bin(), stdin_bytes);
    let r = run(&rust_bin(), stdin_bytes);

    assert_eq!(
        c.stdout,
        r.stdout,
        "[{label}] stdout differs for stdin {:?}\n  C   : {}\n  Rust: {}",
        show(stdin_bytes),
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "[{label}] stderr differs for stdin {:?}\n  C   : {}\n  Rust: {}",
        show(stdin_bytes),
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        (c.status, c.signal),
        (r.status, r.signal),
        "[{label}] exit status differs for stdin {:?}",
        show(stdin_bytes)
    );
}

// ---------------------------------------------------------------------------
// Phase A sanity: both binaries exist and are runnable.
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_run() {
    let c = run(&c_bin(), b"A");
    let r = run(&rust_bin(), b"A");
    assert!(!c.stdout.is_empty(), "C produced no stdout");
    assert!(!r.stdout.is_empty(), "Rust produced no stdout");
    assert_eq!(c.status, Some(0), "C should exit 0 (main falls off the end)");
    assert_eq!(r.status, Some(0), "Rust should exit 0");
}

/// Pins the exact C output shape (14 lines, glibc's *bitmask* return values,
/// `%c` for the case conversions) so a regression in formatting is visible
/// even if both programs were somehow wrong together.
#[test]
fn output_shape_is_pinned() {
    let expected = b"alphanumeric: 8\n\
                     alphabetic: 1024\n\
                     lowercase: 0\n\
                     uppercase: 256\n\
                     digit: 0\n\
                     hexadecimal: 4096\n\
                     control: 0\n\
                     graphical: 32768\n\
                     space: 0\n\
                     blank: 0\n\
                     printing: 16384\n\
                     punctuation: 0\n\
                     to lower: a\n\
                     to upper: A\n";
    let r = run(&rust_bin(), b"A");
    assert_eq!(r.stdout, expected.to_vec(), "Rust output shape for 'A'");
    let c = run(&c_bin(), b"A");
    assert_eq!(c.stdout, expected.to_vec(), "C output shape for 'A'");
    assert_eq!(r.stdout.iter().filter(|&&b| b == b'\n').count(), 14);
}

// ---------------------------------------------------------------------------
// Phase B: every input class the C branches on.
//
// main() reads ONE byte with getchar() and stores it in a `char`, so the whole
// input space of the program is: {EOF} u {0x00..=0xFF} for the first byte.
// Every is*/tolower/toupper branch is a function of that single value.
// ---------------------------------------------------------------------------

/// Empty input: getchar() returns EOF (-1), truncated into `char c` == -1.
#[test]
fn empty_input_eof() {
    assert_same("empty/EOF", b"");
}

/// A single item: exactly one byte and nothing more.
#[test]
fn single_byte_inputs() {
    for b in [
        b'a', b'z', b'A', b'Z', b'0', b'9', b'f', b'F', b'g', b'G', b' ', b'\t', b'\n', b'\r',
        b'!', b'~', b'_', b'@',
    ] {
        assert_same("single byte", &[b]);
    }
}

/// EXHAUSTIVE: all 256 possible first bytes. This covers every classification
/// branch (alpha/digit/xdigit/space/blank/cntrl/graph/print/punct) and both
/// case-conversion branches, including the signed-char negative half
/// (0x80..=0xFF -> negative `char`) where glibc indexes its ctype table below
/// zero and the classifications all come out 0.
#[test]
fn all_256_first_bytes() {
    for i in 0u16..=255 {
        assert_same("byte sweep", &[i as u8]);
    }
}

/// Control characters, individually named: 0x00 (NUL, also the C string
/// terminator), the iscntrl/isspace overlap region 0x07..=0x0d, 0x1f and 0x7f.
#[test]
fn control_characters() {
    for b in [0x00u8, 0x01, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x1f, 0x7f] {
        assert_same("control char", &[b]);
    }
}

/// Boundary bytes around every ASCII class transition.
#[test]
fn class_boundaries() {
    for b in [
        0x1f, 0x20, 0x21, // cntrl | space+print | graph+punct
        0x2f, 0x30, 0x39, 0x3a, // digit boundaries
        0x40, 0x41, 0x46, 0x47, 0x5a, 0x5b, // upper / xdigit boundaries
        0x60, 0x61, 0x66, 0x67, 0x7a, 0x7b, // lower / xdigit boundaries
        0x7e, 0x7f, 0x80, // graph | cntrl | negative char
    ] {
        assert_same("boundary", &[b]);
    }
}

/// The high half, where `char` is negative on x86-64: 0x80 is the most
/// negative char (-128) and 0xFF is -1, which collides with EOF.
#[test]
fn negative_char_range() {
    for b in [0x80u8, 0x81, 0xa0, 0xbf, 0xc0, 0xe9, 0xfe, 0xff] {
        assert_same("negative char", &[b]);
    }
    // 0xFF and EOF both truncate to char -1, so they must agree byte for byte.
    let a = run(&c_bin(), &[0xff]);
    let b = run(&c_bin(), b"");
    assert_eq!(a.stdout, b.stdout, "C: 0xFF and EOF both yield char -1");
    let ra = run(&rust_bin(), &[0xff]);
    let rb = run(&rust_bin(), b"");
    assert_eq!(ra.stdout, rb.stdout, "Rust: 0xFF and EOF both yield char -1");
}

// ---------------------------------------------------------------------------
// Phase C: input shapes no earlier test reaches.
// ---------------------------------------------------------------------------

/// Only the FIRST byte is consumed; every trailing byte must be ignored, and
/// neither program may drain or complain about the rest.
#[test]
fn only_first_byte_is_consumed() {
    for input in [
        &b"Ab"[..],
        &b"A\n"[..],
        &b"AbcdefghijklmnopqrstuvwxyZ0123456789"[..],
        &b"\nA"[..],       // leading newline: fgets-style line logic would differ
        &b"\n\n\n"[..],    // blank lines only
        &b" A"[..],        // leading blank
        &b"\t\tA"[..],
        &b"\x00A"[..],     // embedded NUL first
        &b"A\x00B"[..],    // NUL after the consumed byte
        &b"\xff\x00A"[..], // high byte first
        &b"12 34"[..],     // scanf("%d") would read 12; getchar reads '1'
        &b"-1"[..],
        &b"0x41"[..],
    ] {
        assert_same("multi-byte", input);
    }
}

/// A long input (larger than any stdio buffer) still yields only the first
/// byte's report; also exercises the writer getting EPIPE after the child
/// exits without reading the rest.
#[test]
fn large_input() {
    let mut v = vec![b'Q'];
    v.extend(std::iter::repeat(b'x').take(300_000));
    assert_same("300KB input", &v);

    let mut v2 = vec![b'\n'];
    v2.extend((0..=255u8).cycle().take(100_000));
    assert_same("100KB binary input", &v2);
}

/// Every byte 0x00..=0xFF used as the *first* byte of a longer input, to prove
/// the trailing data never perturbs the classification.
#[test]
fn all_256_first_bytes_with_trailer() {
    for i in 0u16..=255 {
        let input = [i as u8, b'Z', b'\n', 0x00, 0xff];
        assert_same("byte sweep + trailer", &input);
    }
}

/// stdin closed outright (fd 0 unavailable): getchar() fails, returns EOF, and
/// the program must still print the EOF report and exit 0.
#[cfg(unix)]
#[test]
fn stdin_closed() {
    fn go(bin: &Path) -> Run {
        let out = Command::new(bin)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("spawn with /dev/null stdin");
        Run {
            stdout: out.stdout,
            stderr: out.stderr,
            status: out.status.code(),
            signal: None,
        }
    }
    let c = go(&c_bin());
    let r = go(&rust_bin());
    assert_eq!(c.stdout, r.stdout, "stdout with /dev/null stdin");
    assert_eq!(c.stderr, r.stderr, "stderr with /dev/null stdin");
    assert_eq!(c.status, r.status, "status with /dev/null stdin");
}

/// stdin is a *directory*: read(2) fails with EISDIR, getchar() reports EOF.
#[cfg(unix)]
#[test]
fn stdin_is_a_directory() {
    let dir = std::fs::File::open(repo_root().join("c_src")).expect("open c_src as a file");
    let dir2 = std::fs::File::open(repo_root().join("c_src")).expect("open c_src as a file");

    fn go(bin: &Path, f: std::fs::File) -> Run {
        let out = Command::new(bin)
            .stdin(Stdio::from(f))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("spawn with directory stdin");
        Run {
            stdout: out.stdout,
            stderr: out.stderr,
            status: out.status.code(),
            signal: None,
        }
    }
    let c = go(&c_bin(), dir);
    let r = go(&rust_bin(), dir2);
    assert_eq!(c.stdout, r.stdout, "stdout with directory stdin");
    assert_eq!(c.stderr, r.stderr, "stderr with directory stdin");
    assert_eq!(c.status, r.status, "status with directory stdin");
}

/// stdout redirected to /dev/null: printf failures (if any) are ignored by the
/// C and must be ignored by the Rust too -- no panic, no stderr, exit 0.
#[cfg(unix)]
#[test]
fn stdout_to_devnull() {
    fn go(bin: &Path) -> Run {
        let mut child = Command::new(bin)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn with /dev/null stdout");
        let _ = child.stdin.take().unwrap().write_all(b"A");
        let out = child.wait_with_output().expect("wait");
        Run {
            stdout: out.stdout,
            stderr: out.stderr,
            status: out.status.code(),
            signal: None,
        }
    }
    let c = go(&c_bin());
    let r = go(&rust_bin());
    assert_eq!(c.stderr, r.stderr, "stderr with /dev/null stdout");
    assert_eq!(c.status, r.status, "status with /dev/null stdout");
}

/// Arguments are ignored: main() takes no parameters.
#[test]
fn extra_argv_is_ignored() {
    fn go(bin: &Path) -> Run {
        let mut child = Command::new(bin)
            .args(["--help", "-1", "junk"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn with argv");
        let _ = child.stdin.take().unwrap().write_all(b"7");
        let out = child.wait_with_output().expect("wait");
        Run {
            stdout: out.stdout,
            stderr: out.stderr,
            status: out.status.code(),
            signal: None,
        }
    }
    let c = go(&c_bin());
    let r = go(&rust_bin());
    assert_eq!(c.stdout, r.stdout, "stdout with extra argv");
    assert_eq!(c.stderr, r.stderr, "stderr with extra argv");
    assert_eq!(c.status, r.status, "status with extra argv");
}

/// The C calls setlocale(LC_ALL, "C") itself, so the *environment* locale must
/// not change the output. Verify both programs agree under a UTF-8 locale.
#[cfg(unix)]
#[test]
fn locale_env_does_not_matter() {
    fn go(bin: &Path, byte: u8) -> Run {
        let mut child = Command::new(bin)
            .env("LC_ALL", "en_US.UTF-8")
            .env("LANG", "en_US.UTF-8")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn with UTF-8 locale");
        let _ = child.stdin.take().unwrap().write_all(&[byte]);
        let out = child.wait_with_output().expect("wait");
        Run {
            stdout: out.stdout,
            stderr: out.stderr,
            status: out.status.code(),
            signal: None,
        }
    }
    for b in [b'A', b'a', 0xe9, 0xff, 0x20] {
        let c = go(&c_bin(), b);
        let r = go(&rust_bin(), b);
        assert_eq!(c.stdout, r.stdout, "stdout under UTF-8 locale, byte {b:#04x}");
        assert_eq!(c.stderr, r.stderr, "stderr under UTF-8 locale, byte {b:#04x}");
        assert_eq!(c.status, r.status, "status under UTF-8 locale, byte {b:#04x}");
    }
}
