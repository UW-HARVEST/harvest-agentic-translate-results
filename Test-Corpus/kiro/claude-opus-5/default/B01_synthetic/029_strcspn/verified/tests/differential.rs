//! Differential tests: run the C program and the Rust program as subprocesses
//! with identical stdin and require byte-identical stdout, byte-identical
//! stderr and an identical exit status (including death by signal).
//!
//! The Rust code is never linked as a library; both sides are driven exactly
//! the way a shell would drive them.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Path to the Rust binary under test, as produced by cargo.
const RUST_BIN: &str = env!("CARGO_BIN_EXE_driver");

/// Observable result of one run.
#[derive(PartialEq, Eq)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Ok(code)` for a normal exit, `Err(signal)` when killed by a signal.
    status: Result<i32, i32>,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "stdout={:?} stderr={:?} status={}",
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr),
            match self.status {
                Ok(c) => format!("exit {c}"),
                Err(s) => format!("signal {s}"),
            }
        )
    }
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// Build the C program with CMake if it is not already built, and return its path.
fn c_bin() -> PathBuf {
    let root = workspace_root();
    let c_src = root.join("c_src");
    let build = c_src.join("build");
    let bin = build.join("driver");
    if !bin.exists() {
        std::fs::create_dir_all(&build).expect("create c_src/build");
        let cfg = Command::new("cmake")
            .arg("..")
            .current_dir(&build)
            .output()
            .expect("cmake must be installed to run the differential tests");
        assert!(
            cfg.status.success(),
            "cmake configure failed:\n{}",
            String::from_utf8_lossy(&cfg.stderr)
        );
        let out = Command::new("cmake")
            .args(["--build", "."])
            .current_dir(&build)
            .output()
            .expect("run cmake --build");
        assert!(
            out.status.success(),
            "cmake --build failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    assert!(bin.exists(), "C binary missing at {}", bin.display());
    bin
}

fn status_of(status: std::process::ExitStatus) -> Result<i32, i32> {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(sig) = status.signal() {
            return Err(sig);
        }
    }
    Ok(status.code().expect("process exited without a code"))
}

fn run(program: &Path, stdin_bytes: &[u8]) -> Outcome {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", program.display()));

    {
        let mut sink = child.stdin.take().expect("stdin pipe");
        // The child may exit before consuming all input; a broken pipe here is
        // expected and must not fail the test.
        let _ = sink.write_all(stdin_bytes);
        let _ = sink.flush();
    }

    let out = child.wait_with_output().expect("wait for child");
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        status: status_of(out.status),
    }
}

/// Assert full observable equivalence for one input.
#[track_caller]
fn assert_same(label: &str, input: &[u8]) {
    let c = c_bin();
    let expected = run(&c, input);
    let actual = run(Path::new(RUST_BIN), input);
    assert_eq!(
        expected.stdout,
        actual.stdout,
        "stdout mismatch for {label} (input {:?})\n  C:    {expected:?}\n  Rust: {actual:?}",
        Escaped(input)
    );
    assert_eq!(
        expected.stderr,
        actual.stderr,
        "stderr mismatch for {label} (input {:?})\n  C:    {expected:?}\n  Rust: {actual:?}",
        Escaped(input)
    );
    assert_eq!(
        expected.status,
        actual.status,
        "exit status mismatch for {label} (input {:?})\n  C:    {expected:?}\n  Rust: {actual:?}",
        Escaped(input)
    );
}

/// Compact, non-lossy rendering of an input buffer for failure messages.
struct Escaped<'a>(&'a [u8]);
impl std::fmt::Debug for Escaped<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<{} bytes> \"", self.0.len())?;
        for &b in self.0.iter().take(120) {
            match b {
                b'\n' => write!(f, "\\n")?,
                b'\r' => write!(f, "\\r")?,
                b'\t' => write!(f, "\\t")?,
                0x20..=0x7e => write!(f, "{}", b as char)?,
                _ => write!(f, "\\x{b:02x}")?,
            }
        }
        if self.0.len() > 120 {
            write!(f, "...")?;
        }
        write!(f, "\"")
    }
}

// ---------------------------------------------------------------------------
// Phase A sanity: both programs exist and run.
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_run() {
    let c = c_bin();
    let a = run(&c, b"abc\nc\n");
    let b = run(Path::new(RUST_BIN), b"abc\nc\n");
    assert_eq!(a.status, Ok(0), "C program should exit 0 on a normal input");
    assert_eq!(a, b);
}

// ---------------------------------------------------------------------------
// Phase B: the input classes the C code branches on.
//
// The C code is:
//     char s1[100] = "", s2[100] = "";
//     fgets(s1, 100, stdin); fgets(s2, 100, stdin);
//     s1[strlen(s1)-1] = 0; s2[strlen(s2)-1] = 0;
//     printf("%zu\n", strcspn(s1, s2));
//
// Branch points, all inside libc rather than in visible `if`s:
//   * fgets: EOF before any byte (buffer untouched, stays ""), '\n' seen,
//     99-byte limit reached, read error.
//   * strlen == 0 -> `s[-1] = 0`, an out-of-bounds write.
//   * strcspn: match at index 0, match in the middle, no match, empty reject
//     set, empty subject.
// ---------------------------------------------------------------------------

#[test]
fn empty_input() {
    // Both fgets calls fail; both buffers stay "" and both `s[-1] = 0` writes
    // are executed out of bounds.
    assert_same("empty stdin", b"");
}

#[test]
fn only_a_newline() {
    assert_same("single newline", b"\n");
}

#[test]
fn two_blank_lines() {
    assert_same("two blank lines", b"\n\n");
}

#[test]
fn single_line_with_newline() {
    // Second fgets hits EOF immediately.
    assert_same("one line, newline terminated", b"abc\n");
}

#[test]
fn single_line_without_newline() {
    // fgets stores "abc" with no '\n', so the chop removes a data byte.
    assert_same("one line, no trailing newline", b"abc");
}

#[test]
fn single_char_line_no_newline() {
    // strlen == 1, chop leaves "".
    assert_same("single char, no newline", b"a");
}

#[test]
fn match_in_the_middle() {
    assert_same("match in the middle", b"hello world\nlo\n");
}

#[test]
fn match_at_index_zero() {
    assert_same("match at index 0", b"abcdef\na\n");
}

#[test]
fn no_match_at_all() {
    assert_same("no match", b"abcdef\nxyz\n");
}

#[test]
fn empty_reject_set() {
    // s2 is a blank line -> "" after the chop -> strcspn returns strlen(s1).
    assert_same("empty reject set", b"abcdef\n\n");
}

#[test]
fn empty_subject() {
    assert_same("empty subject", b"\nabcdef\n");
}

#[test]
fn reject_set_with_duplicates() {
    assert_same("duplicate reject chars", b"abcdefg\nzzzzzzzzzzzzzzzf\n");
}

#[test]
fn third_line_is_ignored() {
    assert_same("trailing third line", b"abc\ndef\nghi\n");
}

#[test]
fn second_line_without_newline() {
    assert_same("second line, no newline", b"abcdef\nfe");
}

#[test]
fn chop_removes_the_matching_char() {
    // The last byte of s2 is chopped, so 'f' is NOT in the reject set.
    assert_same("chop removes reject char", b"abcdef\nzf");
}

#[test]
fn tabs_and_carriage_returns() {
    assert_same("tab and CR", b"a\tb\rc\n\r\n");
    assert_same("crlf line endings", b"abc\r\nc\r\n");
}

#[test]
fn high_bytes_and_invalid_utf8() {
    assert_same("latin-1 bytes", b"\xe9\xfe\xffabc\n\xff\n");
    assert_same("lone continuation byte", b"a\x80b\n\x80\n");
    assert_same("truncated utf8 sequence", b"\xf0\x9f\x98\n\x9f\n");
}

// ---------------------------------------------------------------------------
// Phase B/C: fgets' 99-byte limit, exercised one byte at a time around the
// boundary. Above the limit the remainder of line 1 is what the second fgets
// reads, so s2 comes from the same physical line.
// ---------------------------------------------------------------------------

#[test]
fn fgets_length_boundary() {
    for n in [0usize, 1, 2, 97, 98, 99, 100, 101, 102, 197, 198, 199, 200, 250] {
        let mut input = vec![b'a'; n];
        input.push(b'\n');
        input.extend_from_slice(b"a\n");
        assert_same(&format!("line1 of {n} 'a' + newline"), &input);

        let mut input = vec![b'a'; n];
        input.extend_from_slice(b"z\n");
        assert_same(&format!("line1 of {n} 'a', no newline, then z"), &input);
    }
}

#[test]
fn full_length_sweep() {
    // Every line-1 length from 0..=205, crossed with newline presence and
    // several shapes of line 2.
    let tails: [&[u8]; 5] = [b"z\n", b"\n", b"", b"abc\n", b"a\n"];
    for n in 0..=205usize {
        for nl in [true, false] {
            for tail in tails {
                let mut input = vec![b'a'; n];
                if nl {
                    input.push(b'\n');
                }
                input.extend_from_slice(tail);
                assert_same(&format!("sweep n={n} nl={nl} tail={tail:?}"), &input);
            }
        }
    }
}

#[test]
fn match_position_across_the_boundary() {
    // The single rejected byte sits right around index 98/99, where the chop
    // and the fgets truncation interact.
    for n in 90..=110usize {
        let mut input = vec![b'a'; n];
        input.extend_from_slice(b"Z\nZ\n");
        assert_same(&format!("reject byte at index {n}"), &input);
    }
}

// ---------------------------------------------------------------------------
// Phase C: embedded NUL bytes. fgets stores them, but strlen/strcspn stop
// there, so a NUL at offset 0 makes strlen 0 and triggers the `s[-1] = 0`
// out-of-bounds write while the buffer still holds data.
// ---------------------------------------------------------------------------

#[test]
fn nul_at_start_of_each_line() {
    assert_same("nul first byte of line 1", b"\0abc\ndef\n");
    assert_same("nul first byte of line 2", b"abc\n\0def\n");
    assert_same("nul first byte of both lines", b"\0abc\n\0def\n");
}

#[test]
fn nul_sweep_over_offsets() {
    for n in 0..=101usize {
        let mut input = vec![b'a'; n];
        input.extend_from_slice(b"\0bc\nc\n");
        assert_same(&format!("nul at offset {n}"), &input);
    }
}

#[test]
fn nul_only_input() {
    assert_same("single nul byte", b"\0");
    assert_same("nul then newline", b"\0\n");
    assert_same("many nuls", &[0u8; 150]);
}

#[test]
fn input_with_no_newline_at_all() {
    assert_same("250 bytes, no newline", &vec![b'x'; 250]);
    assert_same("99 bytes, no newline", &vec![b'x'; 99]);
    assert_same("100 bytes, no newline", &vec![b'x'; 100]);
}

#[test]
fn all_byte_values_as_the_reject_set() {
    // One case per possible reject byte; 0 and '\n' are handled by the cases
    // above but are included so the loop has no silent gap.
    for b in 1u16..=255 {
        let b = b as u8;
        if b == b'\n' {
            continue;
        }
        let input = [b"abc".as_slice(), &[b], b"def\n".as_slice(), &[b], b"\n\n"].concat();
        assert_same(&format!("reject byte 0x{b:02x}"), &input);
    }
}

// ---------------------------------------------------------------------------
// Phase C: deterministic pseudo-random fuzz over arbitrary byte strings.
// ---------------------------------------------------------------------------

/// xorshift64*, so the corpus is identical on every run and every machine.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

#[test]
fn deterministic_fuzz() {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    for case in 0..600u32 {
        let len = rng.below(260) as usize;
        let mut input = Vec::with_capacity(len);
        for _ in 0..len {
            // Bias towards the bytes that matter: newline, NUL, a small
            // alphabet, plus the occasional arbitrary byte.
            input.push(match rng.below(10) {
                0..=2 => b'\n',
                3..=4 => 0,
                5..=7 => b'a' + rng.below(4) as u8,
                _ => rng.below(256) as u8,
            });
        }
        assert_same(&format!("fuzz case {case}"), &input);
    }
}

// ---------------------------------------------------------------------------
// Phase C: exit status paths that stdout comparison alone would never catch.
// ---------------------------------------------------------------------------

#[test]
fn stdin_at_eof_from_dev_null() {
    // Same as empty stdin, but through a real file rather than a pipe, so the
    // read path differs at the syscall level.
    let c = c_bin();
    let expected = run_with_stdin_file(&c, "/dev/null");
    let actual = run_with_stdin_file(Path::new(RUST_BIN), "/dev/null");
    assert_eq!(expected, actual, "stdin=/dev/null");
    assert_eq!(expected.status, Ok(0));
}

fn run_with_stdin_file(program: &Path, path: &str) -> Outcome {
    let f = std::fs::File::open(path).expect("open stdin file");
    let out = Command::new(program)
        .stdin(Stdio::from(f))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run with file stdin");
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        status: status_of(out.status),
    }
}

#[test]
fn unreadable_stdin_is_a_read_error() {
    // stdin is a directory: read(2) fails with EISDIR, fgets returns NULL and
    // the buffer keeps its initial "". Exercises fgets' error branch.
    let c = c_bin();
    let expected = run_with_stdin_file(&c, ".");
    let actual = run_with_stdin_file(Path::new(RUST_BIN), ".");
    assert_eq!(expected, actual, "stdin is a directory");
}

#[cfg(unix)]
#[test]
fn closed_stdout_is_fatal_via_sigpipe() {
    // The C program keeps SIGPIPE at SIG_DFL, so writing to a pipe with no
    // reader kills it (status: signal 13). The Rust runtime ignores SIGPIPE by
    // default, which would silently produce exit 0 instead.
    let c = c_bin();
    let expected = run_with_closed_stdout(&c, b"abc\nz\n");
    let actual = run_with_closed_stdout(Path::new(RUST_BIN), b"abc\nz\n");
    assert_eq!(
        expected.status,
        Err(13),
        "expected the C program to die from SIGPIPE"
    );
    assert_eq!(expected, actual, "closed stdout");
}

#[cfg(unix)]
fn run_with_closed_stdout(program: &Path, stdin_bytes: &[u8]) -> Outcome {
    use std::os::unix::io::FromRawFd;

    extern "C" {
        fn pipe(fds: *mut i32) -> i32;
        fn close(fd: i32) -> i32;
    }

    let mut fds = [-1i32; 2];
    // SAFETY: `fds` is a valid array of two i32s, which is what pipe(2) writes.
    let rc = unsafe { pipe(fds.as_mut_ptr()) };
    assert_eq!(rc, 0, "pipe(2) failed");
    let (read_end, write_end) = (fds[0], fds[1]);
    // Drop the reader so any write from the child gets EPIPE / SIGPIPE.
    // SAFETY: `read_end` is a fresh fd owned by this process.
    unsafe { close(read_end) };

    // SAFETY: `write_end` is a fresh fd; ownership moves into the Stdio.
    let stdout = unsafe { Stdio::from_raw_fd(write_end) };

    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(stdout)
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", program.display()));

    {
        let mut sink = child.stdin.take().expect("stdin pipe");
        let _ = sink.write_all(stdin_bytes);
        let _ = sink.flush();
    }

    let out = child.wait_with_output().expect("wait for child");
    Outcome {
        stdout: Vec::new(), // the child's stdout went to the closed pipe
        stderr: out.stderr,
        status: status_of(out.status),
    }
}
