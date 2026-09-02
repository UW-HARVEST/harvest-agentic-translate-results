//! Differential tests: run the C binary and the Rust binary as subprocesses,
//! feed both the same stdin, and compare stdout, stderr and exit status byte
//! for byte.
//!
//! The Rust code is never called as a library. Both programs are driven exactly
//! the way a shell would drive them.
//!
//! Input classes exercised (derived from reading c_src/src/main.c):
//!
//! ```c
//! int main() {
//!     char text[128];
//!     while (fgets(text, 128, stdin)) {
//!         fputs(text, stdout);
//!     }
//!     return 0;
//! }
//! ```
//!
//! Branches / edges in that code:
//!   * `fgets` returns NULL immediately            -> empty stdin, no output
//!   * `fgets` returns a full line                 -> newline retained
//!   * `fgets` hits EOF with no trailing newline    -> last chunk has no '\n'
//!   * `fgets` fills the buffer (127 bytes read)    -> line split across calls
//!   * `fputs` stops at the first NUL byte          -> embedded NUL truncates
//!   * argv is ignored entirely                     -> args must not change output
//!   * return 0                                     -> exit status is always 0

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

/// Path to the C reference binary produced by `c_src/build`.
fn c_binary() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest
        .parent()
        .expect("translation/ must have a parent directory");
    let candidates = [
        root.join("c_src/build/driver"),
        root.join("c_src/build/Debug/driver"),
        root.join("c_src/build/Release/driver"),
        root.join("c_src/build/driver.exe"),
    ];
    for c in candidates.iter() {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "C reference binary not found. Build it first:\n  \
         cd {} && mkdir -p build && cd build && cmake .. && cmake --build .",
        root.join("c_src").display()
    );
}

/// Path to the Rust binary under test. Cargo hands us the real built artifact,
/// so this is the binary a shell would invoke.
fn rust_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Run one program with the given stdin bytes and argv, capturing everything.
fn run(bin: &Path, args: &[&str], stdin_bytes: &[u8]) -> Output {
    let mut child = Command::new(bin)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    // Feed stdin from a separate thread. Writing inline would deadlock on large
    // inputs: the parent blocks filling the stdin pipe while the child blocks
    // filling the stdout pipe that nobody is draining yet.
    let mut sink = child.stdin.take().expect("stdin was piped");
    let payload = stdin_bytes.to_vec();
    let feeder = std::thread::spawn(move || {
        // The child may exit before consuming all input; a broken pipe here is
        // not a test failure.
        let _ = sink.write_all(&payload);
        let _ = sink.flush();
        drop(sink);
    });

    let out = child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("failed to wait for {}: {e}", bin.display()));
    feeder.join().expect("stdin feeder thread panicked");
    out
}

fn render(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes.iter().take(400) {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            b'\\' => s.push_str("\\\\"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    if bytes.len() > 400 {
        s.push_str(&format!("... ({} bytes total)", bytes.len()));
    }
    s
}

/// Compare stdout, stderr and exit status of both programs for one input.
fn assert_same(case: &str, args: &[&str], stdin_bytes: &[u8]) {
    let c = run(&c_binary(), args, stdin_bytes);
    let r = run(&rust_binary(), args, stdin_bytes);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch [{case}]\n  args:      {args:?}\n  stdin:     {}\n  C stdout:  {}\n  Rust stdout: {}",
        render(stdin_bytes),
        render(&c.stdout),
        render(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch [{case}]\n  args:      {args:?}\n  stdin:     {}\n  C stderr:  {}\n  Rust stderr: {}",
        render(stdin_bytes),
        render(&c.stderr),
        render(&r.stderr)
    );
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "exit code mismatch [{case}]: C {:?} vs Rust {:?} (stdin: {})",
        c.status.code(),
        r.status.code(),
        render(stdin_bytes)
    );
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(
            c.status.signal(),
            r.status.signal(),
            "termination signal mismatch [{case}]: C {:?} vs Rust {:?}",
            c.status.signal(),
            r.status.signal()
        );
    }
}

fn check(case: &str, stdin_bytes: &[u8]) {
    assert_same(case, &[], stdin_bytes);
}

// --------------------------------------------------------------------------
// Phase A sanity: both binaries exist and are runnable.
// --------------------------------------------------------------------------

#[test]
fn both_binaries_exist_and_run() {
    let c = c_binary();
    let r = rust_binary();
    assert!(c.is_file(), "missing C binary at {}", c.display());
    assert!(r.is_file(), "missing Rust binary at {}", r.display());

    let co = run(&c, &[], b"ping\n");
    let ro = run(&r, &[], b"ping\n");
    assert_eq!(co.stdout, b"ping\n");
    assert_eq!(ro.stdout, b"ping\n");
    assert_eq!(co.status.code(), Some(0));
    assert_eq!(ro.status.code(), Some(0));
}

// --------------------------------------------------------------------------
// Phase B: the input classes the C code branches on.
// --------------------------------------------------------------------------

#[test]
fn empty_input() {
    // fgets returns NULL on the very first call: loop body never runs.
    check("empty stdin", b"");
}

#[test]
fn single_newline_only() {
    check("just a newline", b"\n");
}

#[test]
fn single_line_with_newline() {
    check("one line, terminated", b"hello\n");
}

#[test]
fn single_line_without_newline() {
    // EOF reached with a partial buffer: fgets still returns the buffer.
    check("one line, unterminated", b"hello");
}

#[test]
fn single_byte_no_newline() {
    check("single byte", b"x");
}

#[test]
fn multiple_lines() {
    check("three lines", b"alpha\nbeta\ngamma\n");
}

#[test]
fn multiple_lines_last_unterminated() {
    check("three lines, last unterminated", b"alpha\nbeta\ngamma");
}

#[test]
fn many_blank_lines() {
    let input = "\n".repeat(50);
    check("50 blank lines", input.as_bytes());
}

// --------------------------------------------------------------------------
// Buffer-boundary behavior: fgets(text, 128, ...) reads at most 127 bytes.
// --------------------------------------------------------------------------

#[test]
fn lengths_around_buffer_boundary_with_newline() {
    for len in [0usize, 1, 125, 126, 127, 128, 129, 253, 254, 255, 256, 257] {
        let mut input = vec![b'a'; len];
        input.push(b'\n');
        check(&format!("{len} 'a's + newline"), &input);
    }
}

#[test]
fn lengths_around_buffer_boundary_without_newline() {
    for len in [1usize, 126, 127, 128, 129, 254, 255, 256] {
        let input = vec![b'b'; len];
        check(&format!("{len} 'b's, no newline"), &input);
    }
}

#[test]
fn exactly_127_bytes_then_newline_then_more() {
    // First fgets fills the buffer without seeing '\n'; the '\n' arrives on the
    // next call, then a further line follows.
    let mut input = vec![b'c'; 127];
    input.extend_from_slice(b"\nnext\n");
    check("127 bytes, newline on next fgets, then a line", &input);
}

#[test]
fn very_long_single_line() {
    let mut input = vec![b'z'; 5000];
    input.push(b'\n');
    check("5000-byte line", &input);
}

#[test]
fn very_long_single_line_unterminated() {
    let input = vec![b'y'; 5000];
    check("5000-byte unterminated line", &input);
}

#[test]
fn long_lines_of_increasing_length() {
    let mut input = Vec::new();
    for len in 0..300 {
        input.extend(std::iter::repeat(b'#').take(len));
        input.push(b'\n');
    }
    check("lines of length 0..300", &input);
}

// --------------------------------------------------------------------------
// Phase C: paths reached only by unusual bytes.
// fputs writes a C string, so it stops at the first NUL in the buffer even
// though fgets happily read past it.
// --------------------------------------------------------------------------

#[test]
fn embedded_nul_truncates_output() {
    check("NUL in the middle of a line", b"abc\0def\ntail\n");
}

#[test]
fn leading_nul_suppresses_whole_line() {
    check("NUL first", b"\0abc\ndef\n");
}

#[test]
fn nul_only() {
    check("a lone NUL byte", b"\0");
}

#[test]
fn nul_then_newline() {
    check("NUL then newline", b"\0\n");
}

#[test]
fn nul_at_end_of_line() {
    check("NUL right before the newline", b"abc\0\nxyz\n");
}

#[test]
fn many_nuls() {
    check("run of NULs", b"\0\0\0\0\nafter\n");
}

#[test]
fn nul_past_buffer_boundary() {
    // The NUL lands in the second fgets chunk, so the first chunk echoes fully
    // and the second is truncated.
    let mut input = vec![b'a'; 127];
    input.extend_from_slice(b"bb\0cc\n");
    check("NUL in the second chunk", &input);
}

#[test]
fn nul_exactly_at_last_buffer_slot() {
    let mut input = vec![b'a'; 126];
    input.push(0);
    input.extend_from_slice(b"rest\n");
    check("NUL as the 127th byte read", &input);
}

#[test]
fn all_byte_values_one_per_line() {
    let mut input = Vec::new();
    for b in 0u16..=255 {
        input.push(b as u8);
        input.push(b'\n');
    }
    check("every byte value, one per line", &input);
}

#[test]
fn all_byte_values_one_chunk() {
    let mut input: Vec<u8> = (0u16..=255).map(|b| b as u8).collect();
    input.push(b'\n');
    check("every byte value in one blob", &input);
}

#[test]
fn crlf_line_endings() {
    check("CRLF", b"one\r\ntwo\r\n");
}

#[test]
fn lone_carriage_returns() {
    check("bare CR", b"one\rtwo\rthree");
}

#[test]
fn invalid_utf8_bytes() {
    // A UTF-8-validating implementation would choke here; the C code does not.
    check("invalid UTF-8", b"\xff\xfe\x80\x81 tail\n");
}

#[test]
fn truncated_utf8_sequence_across_buffer_boundary() {
    // Split a multi-byte UTF-8 character across two fgets chunks.
    let mut input = vec![b'q'; 126];
    input.extend_from_slice("é".as_bytes()); // 0xC3 0xA9
    input.extend_from_slice(b"\n");
    check("UTF-8 char straddling the 127-byte boundary", &input);
}

#[test]
fn utf8_text() {
    check(
        "multibyte text",
        "héllo wörld — ünïcodé ✓ 日本語\n".as_bytes(),
    );
}

#[test]
fn binary_blob_no_newlines() {
    let input: Vec<u8> = (0..4096).map(|i| (i * 7 % 256) as u8).collect();
    check("4096 bytes of binary, no newlines", &input);
}

#[test]
fn control_characters() {
    check("control chars", b"\x01\x02\x03\x07\x08\x0b\x0c\x1b[31m\x7f\n");
}

#[test]
fn vertical_tab_and_form_feed_are_not_line_breaks() {
    check("VT/FF", b"a\x0bb\x0cc\n");
}

#[test]
fn trailing_newlines_preserved_exactly() {
    check("trailing blank lines", b"a\n\n\n\n");
}

#[test]
fn no_trailing_newline_after_blank_lines() {
    check("blank lines then unterminated text", b"\n\n\nz");
}

// --------------------------------------------------------------------------
// argv is ignored by the C program.
// --------------------------------------------------------------------------

#[test]
fn arguments_are_ignored() {
    assert_same("one arg", &["ignored"], b"data\n");
    assert_same("several args", &["-h", "--help", "file.txt"], b"data\n");
    assert_same("args with empty input", &["whatever"], b"");
    assert_same("arg that looks like a flag", &["--version"], b"x");
}

// --------------------------------------------------------------------------
// Volume / buffering.
// --------------------------------------------------------------------------

#[test]
fn large_multiline_input() {
    let mut input = String::new();
    for i in 0..20_000 {
        input.push_str(&format!("line {i}\n"));
    }
    check("20000 lines", input.as_bytes());
}

#[test]
fn large_input_mixed_lengths_and_nuls() {
    let mut input = Vec::new();
    for i in 0..2000usize {
        let len = i % 300;
        input.extend(std::iter::repeat(b'm').take(len));
        if i % 17 == 0 {
            input.push(0);
            input.extend_from_slice(b"hidden");
        }
        input.push(b'\n');
    }
    check("2000 varied lines with NULs", &input);
}

// --------------------------------------------------------------------------
// stdin closed immediately (no bytes at all, pipe shut) and stdin from
// /dev/null - both must behave like empty input.
// --------------------------------------------------------------------------

#[test]
#[cfg(unix)]
fn stdin_from_dev_null() {
    let dev_null = || std::fs::File::open("/dev/null").expect("open /dev/null");

    let c = Command::new(c_binary())
        .stdin(Stdio::from(dev_null()))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run C with /dev/null stdin");
    let r = Command::new(rust_binary())
        .stdin(Stdio::from(dev_null()))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run Rust with /dev/null stdin");

    assert_eq!(c.stdout, r.stdout, "stdout mismatch with /dev/null stdin");
    assert_eq!(c.stderr, r.stderr, "stderr mismatch with /dev/null stdin");
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "exit code mismatch with /dev/null stdin"
    );
}

#[test]
#[cfg(unix)]
fn stdin_closed() {
    // File descriptor 0 is closed outright: fgets fails, the loop never runs,
    // and the program still returns 0.
    let c = Command::new(c_binary())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run C with null stdin");
    let r = Command::new(rust_binary())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run Rust with null stdin");

    assert_eq!(c.stdout, r.stdout);
    assert_eq!(c.stderr, r.stderr);
    assert_eq!(c.status.code(), r.status.code());
}

// --------------------------------------------------------------------------
// Phase C: stdout buffering. C stdio block-buffers stdout when it is not a
// terminal, so bytes still sitting in the buffer are lost if the process is
// killed by a signal, and visible output only appears at buffer boundaries.
// A line-buffered Rust `io::stdout()` would differ on both counts.
// --------------------------------------------------------------------------

/// Start the program with stdin on a pipe we keep open and stdout on `out_path`,
/// write `payload`, then return the child so the caller can poke at it.
#[cfg(unix)]
fn spawn_with_stdout_file(bin: &Path, out_path: &Path) -> std::process::Child {
    let out = std::fs::File::create(out_path).expect("create stdout capture file");
    Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::from(out))
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()))
}

#[cfg(unix)]
fn tmp_path(name: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("driver_difftest_{}_{}", std::process::id(), name));
    p
}

/// Bytes visible on stdout while the program is still running (stdin held open).
#[cfg(unix)]
fn bytes_flushed_so_far(bin: &Path, payload: &[u8]) -> u64 {
    let out_path = tmp_path(&format!("flush_{}", bin.file_name().unwrap().to_string_lossy()));
    let mut child = spawn_with_stdout_file(bin, &out_path);
    {
        let sink = child.stdin.as_mut().expect("stdin piped");
        sink.write_all(payload).expect("write payload");
        sink.flush().expect("flush payload");
    }
    // Give the child time to consume and buffer everything sent so far.
    std::thread::sleep(std::time::Duration::from_millis(400));
    let visible = std::fs::metadata(&out_path).map(|m| m.len()).unwrap_or(0);
    let _ = child.kill();
    let _ = child.wait();
    let _ = std::fs::remove_file(&out_path);
    visible
}

#[test]
#[cfg(unix)]
fn flush_boundaries_match() {
    // 5000 bytes of complete lines: more than one 4096-byte block, less than two.
    let payload: Vec<u8> = std::iter::repeat(b"aaaaaaaaa\n")
        .take(500)
        .flat_map(|s| s.iter().copied())
        .collect();
    assert_eq!(payload.len(), 5000);

    let c = bytes_flushed_so_far(&c_binary(), &payload);
    let r = bytes_flushed_so_far(&rust_binary(), &payload);
    assert_eq!(
        c, r,
        "stdout flush boundary mismatch: C had written {c} bytes while still \
         running, Rust had written {r}. The Rust program must buffer stdout the \
         way C stdio does instead of line-buffering."
    );
}

#[test]
#[cfg(unix)]
fn buffered_output_lost_on_signal_identically() {
    fn survive_sigterm(bin: &Path) -> Vec<u8> {
        let out_path = tmp_path(&format!(
            "sigterm_{}",
            bin.file_name().unwrap().to_string_lossy()
        ));
        let mut child = spawn_with_stdout_file(bin, &out_path);
        {
            let sink = child.stdin.as_mut().expect("stdin piped");
            sink.write_all(b"hello\nworld\n").expect("write payload");
            sink.flush().expect("flush payload");
        }
        std::thread::sleep(std::time::Duration::from_millis(400));

        // SIGTERM: whatever is still in the stdio buffer never reaches the file.
        let pid = child.id() as i32;
        extern "C" {
            fn kill(pid: i32, sig: i32) -> i32;
        }
        // SAFETY: signalling a child process we spawned; no memory involved.
        unsafe {
            kill(pid, 15);
        }
        let _ = child.wait();
        let data = std::fs::read(&out_path).unwrap_or_default();
        let _ = std::fs::remove_file(&out_path);
        data
    }

    let c = survive_sigterm(&c_binary());
    let r = survive_sigterm(&rust_binary());
    assert_eq!(
        render(&c),
        render(&r),
        "output surviving SIGTERM differs: C left {} bytes on stdout, Rust left {}",
        c.len(),
        r.len()
    );
}

// --------------------------------------------------------------------------
// Phase C: stdout that cannot be written. `fputs`'s return value is discarded
// by the C program, so the loop keeps draining stdin and still returns 0.
// --------------------------------------------------------------------------

#[test]
#[cfg(unix)]
fn unwritable_stdout_still_exits_zero() {
    fn run_with_stdout(bin: &Path, dest: &str) -> Option<i32> {
        let out = match std::fs::OpenOptions::new().write(true).open(dest) {
            Ok(f) => f,
            Err(_) => return None,
        };
        let mut child = Command::new(bin)
            .stdin(Stdio::piped())
            .stdout(Stdio::from(out))
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn");
        {
            let sink = child.stdin.as_mut().expect("stdin piped");
            let _ = sink.write_all(b"abc\ndef\n");
        }
        drop(child.stdin.take());
        child.wait().expect("wait").code()
    }

    // /dev/full accepts opens but fails every write with ENOSPC.
    let c = run_with_stdout(&c_binary(), "/dev/full");
    let r = run_with_stdout(&rust_binary(), "/dev/full");
    assert_eq!(c, r, "exit code mismatch when stdout writes fail");
}
