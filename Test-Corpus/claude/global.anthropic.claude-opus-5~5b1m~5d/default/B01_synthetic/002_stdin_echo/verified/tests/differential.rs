//! Differential tests: run the C binary and the Rust binary as subprocesses on
//! the same input and require byte-identical stdout, byte-identical stderr and
//! an identical exit status (including death by signal).
//!
//! Nothing here links the translation as a library — both programs are driven
//! exactly the way a shell drives them, because that is how they are compared.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;

/// Path of the Rust executable built by cargo for this test run.
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn repo_root() -> PathBuf {
    // `translation/` -> repository root holding `c_src/` next to it.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path of the C executable, configured and built with CMake on first use so
/// that `cargo test` alone is enough to run the suite.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = repo_root().join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");
        if exe.is_file() {
            return exe;
        }

        fs::create_dir_all(&build).expect("cannot create c_src/build");
        let configure = Command::new("cmake")
            .arg("..")
            .current_dir(&build)
            .output()
            .expect("failed to run `cmake ..` (is cmake installed?)");
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
            .expect("failed to run `cmake --build .`");
        assert!(
            compile.status.success(),
            "cmake build failed:\n{}\n{}",
            String::from_utf8_lossy(&compile.stdout),
            String::from_utf8_lossy(&compile.stderr)
        );

        assert!(exe.is_file(), "C binary missing after build: {}", exe.display());
        exe
    })
}

/// Exit status rendered so that a normal exit and death by signal are told
/// apart: C is killed by `SIGPIPE`, and a program that merely exits 0 must not
/// be allowed to look equal to it.
fn status_of(output: &Output) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        match (output.status.code(), output.status.signal()) {
            (Some(code), _) => format!("exited {code}"),
            (None, Some(signal)) => format!("signalled {signal}"),
            (None, None) => "unknown".to_string(),
        }
    }
    #[cfg(not(unix))]
    {
        format!("exited {:?}", output.status.code())
    }
}

/// Run one program with `input` on stdin and the given arguments.
fn run(exe: &Path, args: &[&str], input: &[u8]) -> Output {
    let mut child = Command::new(exe)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("cannot spawn {}: {e}", exe.display()));

    let mut stdin = child.stdin.take().expect("stdin was piped");
    let payload = input.to_vec();
    // Write from another thread so a large payload cannot deadlock against the
    // child filling up the stdout pipe.
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&payload);
        let _ = stdin.flush();
        drop(stdin);
    });

    let output = child.wait_with_output().expect("failed to collect output");
    writer.join().expect("stdin writer thread panicked");
    output
}

fn hex(bytes: &[u8]) -> String {
    const MAX: usize = 256;
    let shown: String = bytes.iter().take(MAX).map(|b| format!("{b:02x}")).collect();
    if bytes.len() > MAX {
        format!("{shown}... ({} bytes total)", bytes.len())
    } else {
        shown
    }
}

/// Core assertion: identical stdout, stderr and exit status for one input.
#[track_caller]
fn assert_same_with_args(case: &str, args: &[&str], input: &[u8]) {
    let c = run(c_bin(), args, input);
    let r = run(&rust_bin(), args, input);

    assert_eq!(
        hex(&c.stdout),
        hex(&r.stdout),
        "[{case}] stdout differs (C left, Rust right); input = {}",
        hex(input)
    );
    assert_eq!(
        c.stdout, r.stdout,
        "[{case}] stdout differs; input = {}",
        hex(input)
    );
    assert_eq!(
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr),
        "[{case}] stderr differs; input = {}",
        hex(input)
    );
    assert_eq!(
        c.stderr, r.stderr,
        "[{case}] stderr bytes differ; input = {}",
        hex(input)
    );
    assert_eq!(
        status_of(&c),
        status_of(&r),
        "[{case}] exit status differs; input = {}",
        hex(input)
    );
}

#[track_caller]
fn assert_same(case: &str, input: &[u8]) {
    assert_same_with_args(case, &[], input);
}

// ---------------------------------------------------------------------------
// The loop condition: `while (fgets(text, 128, stdin))`
// ---------------------------------------------------------------------------

#[test]
fn empty_input_never_enters_the_loop() {
    assert_same("empty", b"");
}

#[test]
fn single_line_is_one_iteration() {
    assert_same("one line", b"hello\n");
}

#[test]
fn single_line_without_trailing_newline() {
    assert_same("no trailing newline", b"hello");
}

#[test]
fn single_character_inputs() {
    assert_same("one char + newline", b"a\n");
    assert_same("one char, no newline", b"a");
    assert_same("just a newline", b"\n");
}

#[test]
fn many_lines() {
    assert_same("three lines", b"a\nb\nc\n");
    assert_same("three lines, last unterminated", b"a\nb\nc");
    assert_same("blank lines only", b"\n\n\n\n\n");
    assert_same("interleaved blank lines", b"a\n\nb\n\n\nc\n");
}

// ---------------------------------------------------------------------------
// The 128-byte buffer boundary: fgets stores at most 127 bytes per call
// ---------------------------------------------------------------------------

#[test]
fn lengths_around_the_buffer_boundary() {
    for len in [1usize, 2, 125, 126, 127, 128, 129, 130, 253, 254, 255, 256, 257] {
        let body = vec![b'x'; len];

        let mut with_nl = body.clone();
        with_nl.push(b'\n');
        assert_same(&format!("{len} bytes + newline"), &with_nl);

        assert_same(&format!("{len} bytes, no newline"), &body);
    }
}

#[test]
fn newline_exactly_after_a_full_buffer() {
    // First fgets fills 127 bytes with no newline, the second returns just "\n".
    let mut input = vec![b'a'; 127];
    input.push(b'\n');
    input.extend(b"tail\n");
    assert_same("127 then newline then tail", &input);
}

#[test]
fn very_long_line_spans_many_calls() {
    let mut input = vec![b'z'; 5000];
    input.push(b'\n');
    assert_same("5000-byte line", &input);

    let unterminated = vec![b'q'; 5000];
    assert_same("5000-byte line, no newline", &unterminated);
}

#[test]
fn mixed_long_and_short_lines() {
    let mut input = Vec::new();
    for len in [200usize, 1, 127, 128, 0, 300, 5] {
        input.extend(std::iter::repeat(b'm').take(len));
        input.push(b'\n');
    }
    assert_same("mixed lengths", &input);
}

// ---------------------------------------------------------------------------
// fputs stops at the first NUL: bytes fgets read after an embedded NUL are
// silently dropped. This is C's behaviour and must be reproduced.
// ---------------------------------------------------------------------------

#[test]
fn embedded_nul_truncates_the_line() {
    assert_same("nul in the middle", b"a\0b\n");
    assert_same("leading nul", b"\0\n");
    assert_same("only a nul", b"\0");
    assert_same("nul before newline", b"abc\0\n");
    assert_same("nul then more lines", b"abc\0def\nsecond\n");
    assert_same("several nuls", b"a\0\0\0b\nc\0d\n");
}

#[test]
fn nul_at_the_buffer_boundary() {
    for pos in [0usize, 1, 63, 125, 126] {
        let mut input = vec![b'p'; 127];
        input[pos] = 0;
        input.push(b'\n');
        input.extend(b"after\n");
        assert_same(&format!("nul at index {pos} of a full buffer"), &input);
    }
}

/// A NUL beyond the first 127 bytes of a long line pins down *where* fgets
/// splits: the bytes dropped are the ones after the NUL within that chunk only,
/// so an off-by-one in the chunk size changes the output.
#[test]
fn nul_past_the_first_chunk() {
    for pos in [126usize, 127, 128, 129, 130, 253, 254, 255, 256, 380] {
        let mut input = vec![b'c'; 500];
        input[pos] = 0;
        input.push(b'\n');
        input.extend(b"next line\n");
        assert_same(&format!("nul at index {pos} of a 500-byte line"), &input);
    }
}

/// Two NULs in different chunks of one line: each chunk is truncated on its own.
#[test]
fn nuls_in_several_chunks_of_one_line() {
    let mut input = vec![b'd'; 600];
    for pos in [10usize, 140, 300, 460] {
        input[pos] = 0;
    }
    input.push(b'\n');
    assert_same("nuls in four chunks", &input);
}

#[test]
fn stale_buffer_bytes_are_not_reused() {
    // A long line followed by a short line that starts with a NUL: the C buffer
    // still holds the previous contents, but fputs must print nothing for it.
    let mut input = vec![b'L'; 127];
    input.push(b'\n');
    input.extend(b"\0\n");
    input.extend(b"end\n");
    assert_same("short nul line after a full buffer", &input);
}

#[test]
fn all_byte_values() {
    let all: Vec<u8> = (0u8..=255).collect();
    assert_same("every byte value", &all);

    let mut reversed: Vec<u8> = (0u8..=255).rev().collect();
    reversed.push(b'\n');
    assert_same("every byte value, reversed", &reversed);
}

// ---------------------------------------------------------------------------
// Bytes that are not text: fgets/fputs are byte oriented, never UTF-8 aware
// ---------------------------------------------------------------------------

#[test]
fn invalid_utf8_passes_through_unchanged() {
    assert_same("lone continuation bytes", b"\xff\xfe\x80 bad utf8\n");
    assert_same("truncated multibyte", b"\xe2\x82\n");
    assert_same("high bytes, no newline", b"\x80\x81\x82\xf4\x90");
}

#[test]
fn carriage_returns_are_not_line_terminators() {
    assert_same("crlf", b"crlf\r\nsecond\r\n");
    assert_same("lone cr", b"no newline here\rstill the same line\n");
    assert_same("cr only", b"\r");
}

#[test]
fn other_control_bytes() {
    assert_same("tabs and escapes", b"a\tb\x1b[0m\x07\n");
    assert_same("vertical tab and form feed", b"a\x0bb\x0cc\n");
    assert_same("del byte", b"\x7f\n");
}

// ---------------------------------------------------------------------------
// Larger payloads: crosses every internal buffer size on both sides
// ---------------------------------------------------------------------------

#[test]
fn large_input_streams_identically() {
    let mut input = Vec::new();
    for i in 0..5000u32 {
        input.extend(format!("line {i} ").into_bytes());
        input.extend(std::iter::repeat(b'.').take((i % 200) as usize));
        input.push(b'\n');
    }
    assert_same("5000 varied lines", &input);
}

#[test]
fn one_huge_line_without_any_newline() {
    let input = vec![b'#'; 300_000];
    assert_same("300k bytes, no newline at all", &input);
}

// ---------------------------------------------------------------------------
// Arguments are ignored: `int main()` takes none
// ---------------------------------------------------------------------------

#[test]
fn arguments_are_ignored() {
    assert_same_with_args("one argument", &["foo"], b"hi\n");
    assert_same_with_args("several arguments", &["a", "b", "c"], b"hi\nthere\n");
    assert_same_with_args("flag-like argument", &["--help"], b"hi\n");
    assert_same_with_args("empty argument", &[""], b"");
}

// ---------------------------------------------------------------------------
// Error paths on the descriptors themselves
// ---------------------------------------------------------------------------

/// stdin is a directory: every `read` fails with EISDIR, so `fgets` returns
/// NULL on the first call and the program exits 0 with no output.
#[test]
fn stdin_that_cannot_be_read() {
    let dir = fs::File::open(repo_root()).expect("cannot open the repo root as a file");
    let dir2 = fs::File::open(repo_root()).expect("cannot open the repo root as a file");

    let c = Command::new(c_bin())
        .stdin(Stdio::from(dir))
        .output()
        .expect("cannot run the C binary");
    let r = Command::new(rust_bin())
        .stdin(Stdio::from(dir2))
        .output()
        .expect("cannot run the Rust binary");

    assert_eq!(hex(&c.stdout), hex(&r.stdout), "stdout differs on unreadable stdin");
    assert_eq!(c.stderr, r.stderr, "stderr differs on unreadable stdin");
    assert_eq!(status_of(&c), status_of(&r), "status differs on unreadable stdin");
}

/// stdin closed outright: `read(0, ...)` fails with EBADF and `fgets` returns
/// NULL immediately.
#[cfg(unix)]
#[test]
fn stdin_closed_before_exec() {
    use std::os::unix::process::CommandExt;

    extern "C" {
        fn close(fd: i32) -> i32;
    }

    fn run_without_stdin(exe: &Path) -> Output {
        let mut cmd = Command::new(exe);
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        unsafe {
            cmd.pre_exec(|| {
                close(0);
                Ok(())
            });
        }
        cmd.output().expect("cannot run the binary without stdin")
    }

    let c = run_without_stdin(c_bin());
    let r = run_without_stdin(&rust_bin());

    assert_eq!(hex(&c.stdout), hex(&r.stdout), "stdout differs with stdin closed");
    assert_eq!(c.stderr, r.stderr, "stderr differs with stdin closed");
    assert_eq!(status_of(&c), status_of(&r), "status differs with stdin closed");
}

/// stdout closed while the program still has output to write: the C program is
/// killed by SIGPIPE, so the translation must be too (Rust ignores SIGPIPE by
/// default, which would make it exit 0 instead).
#[cfg(unix)]
#[test]
fn stdout_closed_early_raises_sigpipe() {
    use std::io::Read;

    fn run_with_closed_reader(exe: &Path) -> String {
        let mut child = Command::new(exe)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("cannot spawn {}: {e}", exe.display()));

        let mut stdin = child.stdin.take().expect("stdin was piped");
        let feeder = std::thread::spawn(move || {
            let line = vec![b'w'; 200];
            for _ in 0..5000 {
                if stdin.write_all(&line).is_err() || stdin.write_all(b"\n").is_err() {
                    return;
                }
            }
        });

        // Read a single byte, then drop the read end of the pipe.
        let mut stdout = child.stdout.take().expect("stdout was piped");
        let mut one = [0u8; 1];
        let _ = stdout.read(&mut one);
        drop(stdout);

        let status = child.wait().expect("failed to wait for the child");
        let _ = feeder.join();

        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            match (status.code(), status.signal()) {
                (Some(code), _) => format!("exited {code}"),
                (None, Some(signal)) => format!("signalled {signal}"),
                (None, None) => "unknown".to_string(),
            }
        }
    }

    let c = run_with_closed_reader(c_bin());
    let r = run_with_closed_reader(&rust_bin());
    assert_eq!(c, r, "status differs when stdout is closed early (C left, Rust right)");
}

/// stdout redirected to /dev/full: every write fails with ENOSPC. The C program
/// neither checks nor reports it, and still exits 0.
#[cfg(target_os = "linux")]
#[test]
fn stdout_write_errors_are_not_reported() {
    fn run_to_dev_full(exe: &Path) -> (Vec<u8>, String) {
        let full = fs::OpenOptions::new()
            .write(true)
            .open("/dev/full")
            .expect("cannot open /dev/full");
        let mut child = Command::new(exe)
            .stdin(Stdio::piped())
            .stdout(Stdio::from(full))
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("cannot spawn {}: {e}", exe.display()));

        let mut stdin = child.stdin.take().expect("stdin was piped");
        let feeder = std::thread::spawn(move || {
            let mut payload = Vec::new();
            for i in 0..2000 {
                payload.extend(format!("line {i}\n").into_bytes());
            }
            let _ = stdin.write_all(&payload);
        });

        let output = child.wait_with_output().expect("failed to collect output");
        let _ = feeder.join();

        #[cfg(unix)]
        let status = {
            use std::os::unix::process::ExitStatusExt;
            match (output.status.code(), output.status.signal()) {
                (Some(code), _) => format!("exited {code}"),
                (None, Some(signal)) => format!("signalled {signal}"),
                (None, None) => "unknown".to_string(),
            }
        };
        (output.stderr, status)
    }

    let (c_err, c_status) = run_to_dev_full(c_bin());
    let (r_err, r_status) = run_to_dev_full(&rust_bin());
    assert_eq!(c_err, r_err, "stderr differs when stdout is /dev/full");
    assert_eq!(c_status, r_status, "status differs when stdout is /dev/full");
}

// ---------------------------------------------------------------------------
// Streaming behaviour: how input arrives, and when output leaves
// ---------------------------------------------------------------------------

/// Input dribbled in so that a single `read` returns only part of a line.
/// `fgets` must keep reading until a newline, the 127-byte limit or EOF — never
/// stop at a short read. The NUL placement is what makes the difference visible:
/// truncation happens once per `fgets` call, so splitting a line differently
/// changes the bytes printed.
#[test]
fn lines_are_assembled_across_short_reads() {
    fn run_slowly(exe: &Path) -> (Vec<u8>, Vec<u8>, String) {
        let mut child = Command::new(exe)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("cannot spawn {}: {e}", exe.display()));

        let mut stdin = child.stdin.take().expect("stdin was piped");
        let feeder = std::thread::spawn(move || {
            let parts: Vec<Vec<u8>> = vec![
                // "A"*10, NUL, "B"*10 — no newline yet, so fgets must wait.
                [vec![b'A'; 10], vec![0], vec![b'B'; 10]].concat(),
                [vec![b'C'; 10], vec![b'\n']].concat(),
                [vec![b'D'; 5], vec![0], vec![b'E'; 5]].concat(),
                [vec![b'F'; 5], vec![b'\n']].concat(),
            ];
            for part in parts {
                if stdin.write_all(&part).is_err() || stdin.flush().is_err() {
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(150));
            }
        });

        let output = child.wait_with_output().expect("failed to collect output");
        feeder.join().expect("feeder thread panicked");
        let status = status_of(&output);
        (output.stdout, output.stderr, status)
    }

    let c = run_slowly(c_bin());
    let r = run_slowly(&rust_bin());
    assert_eq!(
        hex(&c.0),
        hex(&r.0),
        "stdout differs when a line arrives in pieces (C left, Rust right)"
    );
    assert_eq!(c.1, r.1, "stderr differs when a line arrives in pieces");
    assert_eq!(c.2, r.2, "status differs when a line arrives in pieces");
    // Each of the two lines is truncated at its NUL exactly once.
    assert_eq!(c.0, [vec![b'A'; 10], vec![b'D'; 5]].concat());
}

/// Reads `want` bytes from the child's stdout, giving up after a timeout so a
/// program that never flushes cannot hang the suite.
fn read_exactly_within(
    stdout: std::process::ChildStdout,
    want: usize,
    timeout: std::time::Duration,
) -> (Option<Vec<u8>>, std::thread::JoinHandle<Vec<u8>>) {
    use std::io::Read;
    use std::sync::mpsc;

    let (tx, rx) = mpsc::channel();
    let handle = std::thread::spawn(move || {
        let mut stdout = stdout;
        let mut block = vec![0u8; want];
        let mut filled = 0;
        while filled < want {
            match stdout.read(&mut block[filled..]) {
                Ok(0) => break,
                Ok(n) => filled += n,
                Err(_) => break,
            }
        }
        block.truncate(filled);
        let _ = tx.send(block.clone());
        // Drain the rest so the child is never blocked on a full pipe.
        let mut rest = Vec::new();
        let _ = stdout.read_to_end(&mut rest);
        rest
    });

    (rx.recv_timeout(timeout).ok(), handle)
}

/// stdout on a pipe is fully buffered in C, so nothing is written until `BUFSIZ`
/// bytes have piled up — and then exactly one full block appears, even though
/// stdin is still open. Both programs must reach that point with the same bytes.
#[test]
fn full_blocks_are_flushed_before_exit() {
    fn first_block(exe: &Path) -> Option<Vec<u8>> {
        let mut child = Command::new(exe)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("cannot spawn {}: {e}", exe.display()));

        let mut stdin = child.stdin.take().expect("stdin was piped");
        // Feed for as long as the child will take it: stdin never reaches EOF,
        // so only a program that really flushes a full block can be read from.
        let feeder = std::thread::spawn(move || {
            for i in 0..100_000u32 {
                let line = format!("{:05}-abcdefghijklmnopqrstuvwxyz\n", i % 10_000);
                if stdin.write_all(line.as_bytes()).is_err() || stdin.flush().is_err() {
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        });

        let stdout = child.stdout.take().expect("stdout was piped");
        let (block, drain) = read_exactly_within(stdout, 4096, std::time::Duration::from_secs(5));
        // Stop the child so the feeder's next write fails and the thread ends.
        let _ = child.kill();
        let _ = child.wait();
        let _ = drain.join();
        let _ = feeder.join();
        block
    }

    let c = first_block(c_bin());
    let r = first_block(&rust_bin());
    let c = c.expect("the C program never produced a 4 KiB block");
    let r = r.expect("the Rust program never produced a 4 KiB block");
    assert_eq!(c.len(), 4096, "expected a full block from C");
    assert_eq!(hex(&c), hex(&r), "the first flushed block differs (C left, Rust right)");
}

/// Nothing is written before the buffer fills: one short line, stdin still open,
/// and both programs stay silent (C stdio is fully buffered on a pipe).
#[test]
fn short_output_is_withheld_until_eof() {
    fn peek(exe: &Path) -> Option<Vec<u8>> {
        let mut child = Command::new(exe)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("cannot spawn {}: {e}", exe.display()));

        let mut stdin = child.stdin.take().expect("stdin was piped");
        stdin.write_all(b"hello\n").expect("cannot write to stdin");
        stdin.flush().expect("cannot flush stdin");

        let stdout = child.stdout.take().expect("stdout was piped");
        // A single byte within the window would mean the program flushed early.
        let (early, drain) = read_exactly_within(stdout, 1, std::time::Duration::from_millis(700));
        drop(stdin);
        let _ = child.wait();
        let _ = drain.join();
        early
    }

    let c = peek(c_bin());
    let r = peek(&rust_bin());
    assert_eq!(
        c.is_none(),
        r.is_none(),
        "early-output behaviour differs: C withheld = {}, Rust withheld = {}",
        c.is_none(),
        r.is_none()
    );
}

// ---------------------------------------------------------------------------
// Randomised sweep: the same byte-level quirks over inputs nobody enumerated
// ---------------------------------------------------------------------------

#[test]
fn pseudo_random_binary_inputs() {
    // Deterministic xorshift so a failure can be reproduced exactly.
    let mut state: u64 = 0x9E3779B97F4A7C15;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    for case in 0..40 {
        let len = (next() % 900) as usize;
        let mut input = Vec::with_capacity(len);
        for _ in 0..len {
            let r = next();
            // Bias towards newlines and NULs, the two bytes the C code reacts to.
            input.push(match r % 8 {
                0 | 1 => b'\n',
                2 => 0,
                _ => (r >> 8) as u8,
            });
        }
        assert_same(&format!("random case {case} ({len} bytes)"), &input);
    }
}
