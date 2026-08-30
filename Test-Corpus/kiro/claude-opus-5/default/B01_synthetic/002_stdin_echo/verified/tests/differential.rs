//! Differential tests: run the original C program and the Rust translation as
//! subprocesses over the same stdin bytes and require that stdout, stderr and
//! the exit status match exactly.
//!
//! Nothing here calls the Rust code as a library; both programs are driven the
//! way a shell would drive them, because that is how they are compared.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::OnceLock;

/// Repository root: the parent of the Rust crate, containing `c_src/`.
fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate directory must have a parent")
}

/// Path to the Rust binary under test (built by cargo for this test run).
fn rust_binary() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// Path to the compiled C binary, configuring and building it on first use.
///
/// Equivalent to:
/// `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`
fn c_binary() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();

    C_BIN.get_or_init(|| {
        let c_src = repo_root().join("c_src");
        let build_dir = c_src.join("build");
        let bin = build_dir.join("driver");

        if !bin.exists() {
            fs::create_dir_all(&build_dir).expect("cannot create c_src/build");

            let configure = Command::new("cmake")
                .arg("..")
                .current_dir(&build_dir)
                .output()
                .expect("failed to run cmake (is it installed?)");
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
                .expect("failed to run cmake --build");
            assert!(
                build.status.success(),
                "cmake --build failed:\n{}\n{}",
                String::from_utf8_lossy(&build.stdout),
                String::from_utf8_lossy(&build.stderr)
            );
        }

        assert!(
            bin.exists(),
            "C binary missing after build: {}",
            bin.display()
        );
        bin
    })
}

/// What one program produced for a given input.
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
            "status={:?} stdout({} bytes)={:?} stderr({} bytes)={:?}",
            self.status,
            self.stdout.len(),
            Preview(&self.stdout),
            self.stderr.len(),
            Preview(&self.stderr)
        )
    }
}

/// Shows at most the first 96 bytes of a stream, escaping non-printables.
struct Preview<'a>(&'a [u8]);

impl std::fmt::Debug for Preview<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let shown = &self.0[..self.0.len().min(96)];
        for &b in shown {
            match b {
                b'\n' => write!(f, "\\n")?,
                b'\r' => write!(f, "\\r")?,
                b'\t' => write!(f, "\\t")?,
                0x20..=0x7e => write!(f, "{}", b as char)?,
                _ => write!(f, "\\x{:02x}", b)?,
            }
        }
        if self.0.len() > shown.len() {
            write!(f, "...")?;
        }
        Ok(())
    }
}

fn exit_status(status: std::process::ExitStatus) -> Result<i32, i32> {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(sig) = status.signal() {
            return Err(sig);
        }
    }
    Ok(status.code().unwrap_or(-1))
}

/// A temp file holding the test input, so both programs read stdin from a file
/// exactly like `prog < input`. Using a file (instead of writing to a pipe)
/// avoids any chance of deadlocking on large inputs.
struct InputFile(PathBuf);

impl InputFile {
    fn new(data: &[u8]) -> Self {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "c2rust-echo-{}-{}.bin",
            std::process::id(),
            n
        ));
        let mut f = File::create(&path).expect("cannot create temp input file");
        f.write_all(data).expect("cannot write temp input file");
        f.sync_all().ok();
        InputFile(path)
    }

    fn open(&self) -> File {
        File::open(&self.0).expect("cannot reopen temp input file")
    }
}

impl Drop for InputFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn run(program: &Path, args: &[&str], input: &InputFile) -> Outcome {
    let out = Command::new(program)
        .args(args)
        .stdin(Stdio::from(input.open()))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap_or_else(|e| panic!("failed to run {}: {e}", program.display()));

    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        status: exit_status(out.status),
    }
}

/// Core assertion: identical stdout, stderr and exit status for `input`.
#[track_caller]
fn assert_same_with_args(label: &str, args: &[&str], input: &[u8]) {
    let file = InputFile::new(input);
    let c = run(c_binary(), args, &file);
    let rust = run(rust_binary(), args, &file);

    assert_eq!(
        c.stdout,
        rust.stdout,
        "[{label}] stdout differs\n  C:    {:?}\n  Rust: {:?}",
        Preview(&c.stdout),
        Preview(&rust.stdout)
    );
    assert_eq!(
        c.stderr,
        rust.stderr,
        "[{label}] stderr differs\n  C:    {:?}\n  Rust: {:?}",
        Preview(&c.stderr),
        Preview(&rust.stderr)
    );
    assert_eq!(
        c.status, rust.status,
        "[{label}] exit status differs\n  C:    {:?}\n  Rust: {:?}",
        c, rust
    );
}

#[track_caller]
fn assert_same(label: &str, input: &[u8]) {
    assert_same_with_args(label, &[], input);
}

// ---------------------------------------------------------------------------
// Phase A sanity: both programs exist and run.
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_are_runnable() {
    let file = InputFile::new(b"ping\n");
    let c = run(c_binary(), &[], &file);
    let rust = run(rust_binary(), &[], &file);
    assert_eq!(c.status, Ok(0), "C program did not exit 0: {c:?}");
    assert_eq!(rust.status, Ok(0), "Rust program did not exit 0: {rust:?}");
    assert_eq!(c.stdout, b"ping\n");
    assert_eq!(rust.stdout, b"ping\n");
}

// ---------------------------------------------------------------------------
// Empty / minimal inputs. `fgets` returns NULL immediately at EOF.
// ---------------------------------------------------------------------------

#[test]
fn empty_input() {
    assert_same("empty", b"");
}

#[test]
fn newline_only() {
    assert_same("newline only", b"\n");
    assert_same("two newlines", b"\n\n");
    assert_same("many newlines", &vec![b'\n'; 200]);
}

#[test]
fn single_line() {
    assert_same("one line, terminated", b"hello\n");
    assert_same("one line, unterminated", b"hello");
    assert_same("one char, unterminated", b"x");
}

#[test]
fn multiple_lines() {
    assert_same("three lines", b"a\nb\nc\n");
    assert_same("last line unterminated", b"a\nb\nccc");
    assert_same("blank lines interleaved", b"a\n\n\nb\n\n");
    assert_same("whitespace only", b"  \t a \t \n\t\n");
}

// ---------------------------------------------------------------------------
// The 128-byte buffer: `fgets` stores at most 127 bytes per call, so lines at
// and beyond that length are split across iterations.
// ---------------------------------------------------------------------------

#[test]
fn line_length_at_buffer_boundary() {
    for len in [1usize, 125, 126, 127, 128, 129, 253, 254, 255, 256, 300] {
        let mut terminated = vec![b'x'; len];
        terminated.push(b'\n');
        assert_same(&format!("{len} bytes + newline"), &terminated);
        assert_same(&format!("{len} bytes, no newline"), &vec![b'x'; len]);
    }
}

#[test]
fn line_far_longer_than_buffer() {
    let mut data = vec![b'x'; 1000];
    data.push(b'\n');
    assert_same("1000 bytes + newline", &data);
    assert_same("5000 bytes, no newline", &vec![b'y'; 5000]);
}

#[test]
fn newline_exactly_after_a_full_buffer() {
    // First fgets fills 127 bytes; the newline is read by the next call.
    let mut data = vec![b'a'; 127];
    data.extend_from_slice(b"\nrest\n");
    assert_same("newline at offset 127", &data);
}

// ---------------------------------------------------------------------------
// Embedded NUL bytes: `fgets` stores them, `fputs` stops at the first one, so
// the rest of that chunk never reaches stdout. This looks like a bug and must
// be reproduced.
// ---------------------------------------------------------------------------

#[test]
fn nul_truncates_the_chunk() {
    assert_same("NUL mid line", b"ab\x00cd\nef\n");
    assert_same("NUL only", b"\x00");
    assert_same("NUL then newline", b"\x00\n");
    assert_same("NUL starts second line", b"x\n\x00y\nz\n");
    assert_same("NUL before newline", b"abc\x00\ndef\n");
    assert_same("several NULs", b"a\x00b\x00c\nd\x00\n");
}

#[test]
fn nul_interacts_with_the_buffer_boundary() {
    let mut data = vec![b'y'; 127];
    data.extend_from_slice(b"\x00rest\n");
    assert_same("NUL at offset 127", &data);

    let mut data = vec![b'z'; 126];
    data.extend_from_slice(b"\x00tail\nnext\n");
    assert_same("NUL at offset 126", &data);
}

// ---------------------------------------------------------------------------
// Arbitrary bytes: the program is byte-oriented, not text-oriented.
// ---------------------------------------------------------------------------

#[test]
fn arbitrary_binary_input() {
    let all: Vec<u8> = (0u8..=255).collect();
    assert_same("all 256 byte values", &all);

    let mut repeated = Vec::new();
    for _ in 0..4 {
        repeated.extend_from_slice(&all);
    }
    assert_same("all byte values x4", &repeated);

    assert_same("invalid UTF-8", b"\xff\xfe\x80\n\xc3\x28\n");
    assert_same("lone continuation bytes", b"\x80\x81\x82");

    let pattern: Vec<u8> = (0..5000).map(|i| ((i * 7) % 256) as u8).collect();
    assert_same("5000 byte pattern", &pattern);
}

#[test]
fn carriage_returns_are_not_line_terminators() {
    assert_same("CRLF", b"line\r\nsecond\r\n");
    assert_same("CR only", b"line\rsecond\r");
    assert_same("lone CR at EOF", b"abc\r");
}

// ---------------------------------------------------------------------------
// Output volume: crosses the stdout buffer several times.
// ---------------------------------------------------------------------------

#[test]
fn many_lines_cross_the_stdout_buffer() {
    let mut data = Vec::new();
    for i in 0..2000 {
        data.extend_from_slice(format!("line {i}\n").as_bytes());
    }
    assert_same("2000 lines", &data);
}

// ---------------------------------------------------------------------------
// Arguments: `main()` takes none, so every argument is ignored.
// ---------------------------------------------------------------------------

#[test]
fn arguments_are_ignored() {
    for args in [
        vec!["foo", "bar"],
        vec!["-h"],
        vec!["--help"],
        vec!["--version"],
        vec!["-"],
        vec!["/nonexistent/file"],
    ] {
        assert_same_with_args(&format!("args {args:?}"), &args, b"hi\nthere\n");
        assert_same_with_args(&format!("args {args:?} / empty stdin"), &args, b"");
    }
}

// ---------------------------------------------------------------------------
// Error paths on the streams themselves.
// ---------------------------------------------------------------------------

/// `/dev/null` as stdin: immediate EOF, same as an empty file.
#[test]
fn stdin_from_dev_null() {
    if !Path::new("/dev/null").exists() {
        return;
    }
    let mut outcomes = Vec::new();
    for program in [c_binary(), rust_binary()] {
        let out = Command::new(program)
            .stdin(Stdio::from(File::open("/dev/null").expect("open /dev/null")))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("failed to run program");
        outcomes.push(Outcome {
            stdout: out.stdout,
            stderr: out.stderr,
            status: exit_status(out.status),
        });
    }
    assert_eq!(outcomes[0], outcomes[1], "stdin=/dev/null: {outcomes:?}");
    assert_eq!(outcomes[0].status, Ok(0));
    assert!(outcomes[0].stdout.is_empty());
}

/// A directory as stdin makes `read(2)` fail with `EISDIR`, so `fgets` returns
/// NULL with nothing read: no output, exit 0.
#[cfg(unix)]
#[test]
fn stdin_is_a_directory_read_error() {
    let mut outcomes = Vec::new();
    for program in [c_binary(), rust_binary()] {
        let dir = File::open(repo_root()).expect("cannot open repo root as a file");
        let out = Command::new(program)
            .stdin(Stdio::from(dir))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("failed to run program");
        outcomes.push(Outcome {
            stdout: out.stdout,
            stderr: out.stderr,
            status: exit_status(out.status),
        });
    }
    assert_eq!(outcomes[0], outcomes[1], "stdin=directory: {outcomes:?}");
}

/// stdout closed by the reader: the C program inherits the default `SIGPIPE`
/// disposition and dies from the signal. The Rust translation must too.
#[cfg(unix)]
#[test]
fn stdout_closed_early_kills_both() {
    // Far more than any pipe buffer, so the write is guaranteed to block and
    // then fail once the read end is gone.
    let mut data = Vec::new();
    for _ in 0..20_000 {
        data.extend_from_slice(&[b'x'; 100]);
        data.push(b'\n');
    }
    let file = InputFile::new(&data);

    let mut statuses = Vec::new();
    for program in [c_binary(), rust_binary()] {
        let mut child = Command::new(program)
            .stdin(Stdio::from(file.open()))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to spawn program");

        {
            let mut stdout = child.stdout.take().expect("piped stdout");
            let mut head = [0u8; 10];
            stdout.read_exact(&mut head).expect("read first bytes");
            assert_eq!(&head, b"xxxxxxxxxx");
            // Dropping the read end closes the pipe while the child is still
            // writing.
        }

        let status = child.wait().expect("wait on child");
        statuses.push(exit_status(status));
    }

    assert_eq!(
        statuses[0], statuses[1],
        "closed stdout: C {:?} vs Rust {:?}",
        statuses[0], statuses[1]
    );
}
