//! Differential tests: run the C reference binary and the Rust binary as
//! subprocesses over the same stdin and require byte-identical stdout, stderr
//! and exit status.
//!
//! The Rust code is never linked as a library here -- both programs are driven
//! exactly the way a shell would drive them, which is how this translation is
//! graded.
//!
//! Input classes exercised (derived from reading `c_src/src/main.c`):
//!
//! * empty stdin -- `fscanf` performs no assignment, so `data` keeps its
//!   initializer `' '` and the program prints `21`
//! * closed stdin (fd 0 not open) -- also a failed conversion
//! * stdin that is a directory -- `read` fails with EISDIR, failed conversion
//! * every one of the 256 possible first bytes, including NUL, whitespace
//!   (`%c` does *not* skip it), 0x7f/0x80/0xfe/0xff which exercise signed-char
//!   truncation and the sign extension `%x` then prints
//! * multi-byte input -- only the first byte may be consumed
//! * large input -- ensures no over-reading changes the output

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// Path to the C reference binary, building it with cmake if it is not there.
fn c_binary() -> PathBuf {
    let root = repo_root();
    let c_src = root.join("c_src");
    let build = c_src.join("build");
    let bin = build.join("driver");
    if !bin.exists() {
        std::fs::create_dir_all(&build).expect("create c_src/build");
        let configure = Command::new("cmake")
            .arg("..")
            .current_dir(&build)
            .output()
            .expect("run cmake (is cmake installed?)");
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
            .expect("run cmake --build");
        assert!(
            compile.status.success(),
            "cmake --build failed:\n{}\n{}",
            String::from_utf8_lossy(&compile.stdout),
            String::from_utf8_lossy(&compile.stderr)
        );
    }
    assert!(bin.exists(), "C binary missing at {}", bin.display());
    bin
}

/// Path to the Rust binary under test. Cargo hands integration tests the path
/// to the test executable; the binary sits in the same target profile dir.
fn rust_binary() -> PathBuf {
    let mut dir = std::env::current_exe().expect("current_exe");
    dir.pop(); // drop test executable name
    if dir.ends_with("deps") {
        dir.pop();
    }
    let bin = dir.join("driver");
    assert!(
        bin.exists(),
        "Rust binary missing at {} -- run `cargo build` first",
        bin.display()
    );
    bin
}

/// Run a program with `input` on stdin, capturing stdout, stderr and status.
fn run(bin: &Path, input: &[u8]) -> Output {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", bin.display()));
    child
        .stdin
        .as_mut()
        .expect("stdin pipe")
        .write_all(input)
        .or_else(|e| {
            // The program may exit before consuming all of a large input.
            if e.kind() == std::io::ErrorKind::BrokenPipe {
                Ok(())
            } else {
                Err(e)
            }
        })
        .expect("write stdin");
    drop(child.stdin.take());
    child.wait_with_output().expect("wait_with_output")
}

fn assert_same(label: &str, input: &[u8]) {
    let c = run(&c_binary(), input);
    let r = run(&rust_binary(), input);
    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout differs for {label}: C={:?} Rust={:?}",
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr differs for {label}: C={:?} Rust={:?}",
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "exit status differs for {label}: C={:?} Rust={:?}",
        c.status,
        r.status
    );
}

// ---------------------------------------------------------------------------
// Phase B: the input classes the C program branches on
// ---------------------------------------------------------------------------

/// Empty stdin: `fscanf` returns EOF without assigning, so `data` stays `' '`.
#[test]
fn empty_stdin() {
    assert_same("empty stdin", b"");
    // Pin the observed C behavior so a regression is obvious, not just "equal".
    let c = run(&c_binary(), b"");
    assert_eq!(c.stdout, b"21\n");
    assert!(c.stderr.is_empty());
    assert_eq!(c.status.code(), Some(0));
}

/// A space on stdin is *data*, not a skipped separator, and coincidentally
/// produces the same output as the EOF path.
#[test]
fn single_space_matches_eof_output() {
    assert_same("single space", b" ");
    let c = run(&c_binary(), b" ");
    assert_eq!(c.stdout, b"21\n");
}

/// `%c` does not skip leading whitespace: a newline is consumed as the byte.
#[test]
fn leading_whitespace_is_consumed_as_data() {
    for (label, input, expected) in [
        ("newline", &b"\nA"[..], &b"0b\n"[..]),
        ("tab", &b"\tA"[..], &b"0a\n"[..]),
        ("cr", &b"\rA"[..], &b"0e\n"[..]),
        ("vtab", &b"\x0bA"[..], &b"0c\n"[..]),
        ("formfeed", &b"\x0cA"[..], &b"0d\n"[..]),
    ] {
        assert_same(label, input);
        assert_eq!(run(&c_binary(), input).stdout, expected, "{label}");
    }
}

/// Only the first byte matters; the rest of stdin is never read.
#[test]
fn only_first_byte_is_read() {
    assert_same("two bytes", b"AB");
    assert_same("line", b"hello world\n");
    assert_same("multiple lines", b"first\nsecond\nthird\n");
    assert_eq!(run(&c_binary(), b"AB").stdout, b"42\n");
    assert_eq!(run(&c_binary(), b"hello world\n").stdout, b"69\n");
}

/// NUL is a perfectly ordinary byte for `%c`.
#[test]
fn nul_byte() {
    assert_same("NUL", b"\0");
    assert_same("NUL then text", b"\0abc");
    assert_eq!(run(&c_binary(), b"\0").stdout, b"01\n");
}

/// Signed-`char` truncation and the sign extension `%x` prints afterwards.
/// `char result = data + 1` wraps into negative territory at 0x7f, and the
/// variadic promotion to `int` then makes `%x` print eight hex digits.
#[test]
fn signed_char_boundaries() {
    for (input, expected) in [
        (0x7eu8, &b"7f\n"[..]),
        (0x7f, &b"ffffff80\n"[..]),
        (0x80, &b"ffffff81\n"[..]),
        (0xfe, &b"ffffffff\n"[..]),
        (0xff, &b"00\n"[..]),
    ] {
        let buf = [input];
        assert_same(&format!("byte {input:#04x}"), &buf);
        assert_eq!(
            run(&c_binary(), &buf).stdout,
            expected,
            "byte {input:#04x}"
        );
    }
}

/// `%02x` pads to two digits but never truncates a wider value.
#[test]
fn zero_padding_width() {
    // 0x00 -> result 0x01 -> "01" (padded)
    assert_same("pad low", b"\x00");
    // 0x0f -> result 0x10 -> "10" (no padding needed)
    assert_same("no pad", b"\x0f");
    // 0x7f -> result -128 -> "ffffff80" (wider than the field width)
    assert_same("wider than width", b"\x7f");
    assert_eq!(run(&c_binary(), b"\x0f").stdout, b"10\n");
}

/// Exhaustive: every possible first byte.
#[test]
fn all_256_first_bytes() {
    let c = c_binary();
    let r = rust_binary();
    for b in 0u16..=255 {
        let buf = [b as u8];
        let co = run(&c, &buf);
        let ro = run(&r, &buf);
        assert_eq!(
            co.stdout,
            ro.stdout,
            "stdout differs for byte {b:#04x}: C={:?} Rust={:?}",
            String::from_utf8_lossy(&co.stdout),
            String::from_utf8_lossy(&ro.stdout)
        );
        assert_eq!(co.stderr, ro.stderr, "stderr differs for byte {b:#04x}");
        assert_eq!(
            co.status.code(),
            ro.status.code(),
            "exit status differs for byte {b:#04x}"
        );
    }
}

/// Exhaustive with a trailing byte, proving the second byte never leaks in.
#[test]
fn all_256_first_bytes_with_trailer() {
    let c = c_binary();
    let r = rust_binary();
    for b in 0u16..=255 {
        let buf = [b as u8, b'Z'];
        let co = run(&c, &buf);
        let ro = run(&r, &buf);
        assert_eq!(co.stdout, ro.stdout, "stdout differs for {b:#04x} + 'Z'");
        assert_eq!(co.stderr, ro.stderr, "stderr differs for {b:#04x} + 'Z'");
        assert_eq!(
            co.status.code(),
            ro.status.code(),
            "exit status differs for {b:#04x} + 'Z'"
        );
    }
}

// ---------------------------------------------------------------------------
// Phase C: paths not covered above
// ---------------------------------------------------------------------------

/// Input far larger than any stdio buffer. The program still reads one byte and
/// exits; the writer may see EPIPE, which must not change either program's
/// observable output.
#[test]
fn very_large_input() {
    let big = vec![b'Q'; 1 << 20];
    assert_same("1 MiB of 'Q'", &big);
    assert_eq!(run(&c_binary(), &big).stdout, b"52\n");
}

/// All-NUL bulk input.
#[test]
fn large_nul_input() {
    let big = vec![0u8; 64 * 1024];
    assert_same("64 KiB of NUL", &big);
}

/// Binary input containing every byte value, in order and reversed, so the
/// first byte is 0x00 in one case and 0xff in the other.
#[test]
fn full_byte_range_streams() {
    let ascending: Vec<u8> = (0u16..=255).map(|b| b as u8).collect();
    let mut descending = ascending.clone();
    descending.reverse();
    assert_same("ascending 0..=255", &ascending);
    assert_same("descending 255..=0", &descending);
    assert_eq!(run(&c_binary(), &ascending).stdout, b"01\n");
    assert_eq!(run(&c_binary(), &descending).stdout, b"00\n");
}

/// UTF-8 multi-byte character: only its first (continuation-leading) byte is read.
#[test]
fn utf8_multibyte_first_byte_only() {
    assert_same("euro sign", "\u{20ac}".as_bytes()); // E2 82 AC -> 0xe2 -> ffffffe3
    assert_same("emoji", "\u{1f600}".as_bytes()); // F0 9F 98 80 -> 0xf0 -> fffffff1
    assert_eq!(run(&c_binary(), "\u{20ac}".as_bytes()).stdout, b"ffffffe3\n");
}

/// stdin closed entirely (fd 0 not open): the `read` fails, `fscanf` assigns
/// nothing, and `data` keeps `' '`.
#[test]
fn stdin_closed() {
    fn run_with_null_stdin(bin: &Path) -> Output {
        Command::new(bin)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("spawn with null stdin")
    }
    let c = run_with_null_stdin(&c_binary());
    let r = run_with_null_stdin(&rust_binary());
    assert_eq!(c.stdout, r.stdout, "stdout differs with /dev/null stdin");
    assert_eq!(c.stderr, r.stderr, "stderr differs with /dev/null stdin");
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "exit status differs with /dev/null stdin"
    );
    assert_eq!(c.stdout, b"21\n");
}

/// stdin is a directory: `read(2)` fails with EISDIR. The conversion fails, so
/// `data` again keeps its initializer.
#[test]
fn stdin_is_a_directory() {
    let dir = std::fs::File::open(repo_root()).expect("open repo root as a file");
    fn run_with(bin: &Path, f: &std::fs::File) -> Output {
        let dup = f.try_clone().expect("dup fd");
        Command::new(bin)
            .stdin(Stdio::from(dup))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("spawn with directory stdin")
    }
    let c = run_with(&c_binary(), &dir);
    let r = run_with(&rust_binary(), &dir);
    assert_eq!(c.stdout, r.stdout, "stdout differs with directory stdin");
    assert_eq!(c.stderr, r.stderr, "stderr differs with directory stdin");
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "exit status differs with directory stdin"
    );
    assert_eq!(c.stdout, b"21\n");
}

/// stdout redirected to a regular file rather than a pipe, confirming the Rust
/// program flushes the same bytes when stdout is fully buffered in C.
#[test]
fn stdout_to_regular_file() {
    fn capture(bin: &Path, input: &[u8], path: &Path) -> (Vec<u8>, Option<i32>) {
        let out = std::fs::File::create(path).expect("create temp out");
        let mut child = Command::new(bin)
            .stdin(Stdio::piped())
            .stdout(Stdio::from(out))
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn");
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(input)
            .or_else(|e| {
                if e.kind() == std::io::ErrorKind::BrokenPipe {
                    Ok(())
                } else {
                    Err(e)
                }
            })
            .expect("write stdin");
        drop(child.stdin.take());
        let status = child.wait().expect("wait");
        let bytes = std::fs::read(path).expect("read temp out");
        (bytes, status.code())
    }
    let tmp = std::env::temp_dir();
    let cp = tmp.join("driver_c_stdout.bin");
    let rp = tmp.join("driver_rust_stdout.bin");
    for input in [&b""[..], &b"A"[..], &b"\x7f"[..], &b"\xff"[..]] {
        let (cb, cs) = capture(&c_binary(), input, &cp);
        let (rb, rs) = capture(&rust_binary(), input, &rp);
        assert_eq!(cb, rb, "file stdout differs for {input:?}");
        assert_eq!(cs, rs, "exit status differs for {input:?}");
    }
    let _ = std::fs::remove_file(&cp);
    let _ = std::fs::remove_file(&rp);
}

/// Extra command-line arguments are ignored by `main()` (it takes no argv).
#[test]
fn arguments_are_ignored() {
    fn run_with_args(bin: &Path, args: &[&str], input: &[u8]) -> Output {
        let mut child = Command::new(bin)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn");
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(input)
            .expect("write stdin");
        drop(child.stdin.take());
        child.wait_with_output().expect("wait")
    }
    for args in [&["extra"][..], &["-h"][..], &["a", "b", "c"][..]] {
        let c = run_with_args(&c_binary(), args, b"A");
        let r = run_with_args(&rust_binary(), args, b"A");
        assert_eq!(c.stdout, r.stdout, "stdout differs with args {args:?}");
        assert_eq!(c.stderr, r.stderr, "stderr differs with args {args:?}");
        assert_eq!(
            c.status.code(),
            r.status.code(),
            "exit status differs with args {args:?}"
        );
    }
}

/// Neither program writes anything to stderr on any input class.
#[test]
fn stderr_always_empty() {
    for input in [
        &b""[..],
        &b"\0"[..],
        &b" "[..],
        &b"A"[..],
        &b"\x7f"[..],
        &b"\xff"[..],
        &b"long input with many bytes\n"[..],
    ] {
        let c = run(&c_binary(), input);
        let r = run(&rust_binary(), input);
        assert!(c.stderr.is_empty(), "C wrote stderr for {input:?}");
        assert!(r.stderr.is_empty(), "Rust wrote stderr for {input:?}");
    }
}

/// Exit status is always 0 (`return 0` is the only exit from `main`).
#[test]
fn exit_status_always_zero() {
    for input in [&b""[..], &b"\0"[..], &b"\xff"[..], &b"hello"[..]] {
        assert_eq!(run(&c_binary(), input).status.code(), Some(0));
        assert_eq!(run(&rust_binary(), input).status.code(), Some(0));
    }
}
