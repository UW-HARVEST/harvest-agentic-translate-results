//! Differential tests: run the C binary and the Rust binary as subprocesses on
//! the same stdin bytes and require byte-identical stdout, byte-identical
//! stderr and an identical exit status.
//!
//! The Rust code is never linked as a library here; both programs are driven
//! exactly the way a shell would drive them, because that is how they are
//! compared.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// Path to the Rust binary that cargo built for this integration test.
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn workspace_root() -> PathBuf {
    // translation/ -> repository root
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the C binary, building it with CMake on first use if necessary.
/// `c_src/` is only ever read and built out-of-tree into `c_src/build/`.
fn c_bin() -> &'static PathBuf {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = workspace_root().join("c_src");
        let build_dir = c_src.join("build");
        let bin = build_dir.join("driver");
        if !bin.exists() {
            std::fs::create_dir_all(&build_dir).expect("create c_src/build");
            let cfg = Command::new("cmake")
                .arg("..")
                .current_dir(&build_dir)
                .output()
                .expect("failed to run `cmake ..` (is cmake installed?)");
            assert!(
                cfg.status.success(),
                "cmake configure failed:\n{}\n{}",
                String::from_utf8_lossy(&cfg.stdout),
                String::from_utf8_lossy(&cfg.stderr)
            );
            let build = Command::new("cmake")
                .args(["--build", "."])
                .current_dir(&build_dir)
                .output()
                .expect("failed to run `cmake --build .`");
            assert!(
                build.status.success(),
                "cmake build failed:\n{}\n{}",
                String::from_utf8_lossy(&build.stdout),
                String::from_utf8_lossy(&build.stderr)
            );
        }
        assert!(bin.exists(), "C binary missing at {}", bin.display());
        bin
    })
}

struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    status: Option<i32>,
}

fn run(bin: &Path, input: &[u8]) -> Outcome {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    // Write on a worker thread: a program that stops reading at 1000 bytes can
    // exit while a larger payload is still queued, and a blocking write on this
    // thread would then deadlock (or fail with EPIPE, which we ignore, exactly
    // as a shell pipeline would).
    let mut stdin = child.stdin.take().expect("piped stdin");
    let payload = input.to_vec();
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&payload);
        let _ = stdin.flush();
        drop(stdin);
    });

    let out = child.wait_with_output().expect("wait_with_output");
    let _ = writer.join();

    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        status: out.status.code(),
    }
}

/// Assert C and Rust agree on stdout, stderr and exit status for `input`.
#[track_caller]
fn assert_same(desc: &str, input: &[u8]) {
    let c = run(c_bin(), input);
    let r = run(&rust_bin(), input);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch for {desc}\n  C:    {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch for {desc}\n  C:    {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
    assert_eq!(
        c.status, r.status,
        "exit status mismatch for {desc}: C {:?} vs Rust {:?}",
        c.status, r.status
    );
}

/// Also pin down the absolute expected bytes, so a shared regression in both
/// programs cannot pass unnoticed.
#[track_caller]
fn assert_same_and_eq(desc: &str, input: &[u8], expected_stdout: &str) {
    assert_same(desc, input);
    let c = run(c_bin(), input);
    assert_eq!(
        String::from_utf8_lossy(&c.stdout),
        expected_stdout,
        "C stdout for {desc} is not what the test expects"
    );
    let r = run(&rust_bin(), input);
    assert_eq!(
        String::from_utf8_lossy(&r.stdout),
        expected_stdout,
        "Rust stdout for {desc} is not what the test expects"
    );
}

// ---------------------------------------------------------------------------
// main(): fread into a zero-initialised char in[1000]
// ---------------------------------------------------------------------------

#[test]
fn empty_input() {
    // fread returns 0; the buffer stays all-zero, so the string is "".
    assert_same_and_eq("empty input", b"", "A: 0\nx: 0\n");
}

#[test]
fn single_a() {
    assert_same_and_eq("single 'A'", b"A", "A: 1\nx: 0\n");
}

#[test]
fn single_x() {
    assert_same_and_eq("single 'x'", b"x", "A: 0\nx: 1\n");
}

#[test]
fn single_unrelated_byte() {
    // foo() takes the `strchr(...) == NULL` exit on the very first iteration.
    assert_same_and_eq("single unrelated byte", b"q", "A: 0\nx: 0\n");
}

#[test]
fn no_trailing_newline_vs_trailing_newline() {
    // fread has no line semantics: the newline is just another byte.
    assert_same_and_eq("no trailing newline", b"Ax", "A: 1\nx: 1\n");
    assert_same_and_eq("trailing newline", b"Ax\n", "A: 1\nx: 1\n");
}

#[test]
fn reads_across_newlines() {
    // Unlike fgets, fread keeps going past '\n' until EOF or 1000 bytes, so
    // every line contributes to the counts.
    assert_same_and_eq(
        "multiple lines",
        b"A\nx\nAA\nxx\n",
        "A: 3\nx: 3\n",
    );
    assert_same_and_eq("blank lines only", b"\n\n\n\n", "A: 0\nx: 0\n");
}

#[test]
fn counts_are_case_sensitive() {
    // 'a' is not 'A' and 'X' is not 'x'.
    assert_same_and_eq("wrong case only", b"aaaXXX", "A: 0\nx: 0\n");
    assert_same_and_eq("mixed case", b"aAxX", "A: 1\nx: 1\n");
}

// ---------------------------------------------------------------------------
// foo(): the strchr scan loop
// ---------------------------------------------------------------------------

#[test]
fn adjacent_matches() {
    // s++ after a hit must not skip the immediately following match.
    assert_same_and_eq("adjacent matches", b"AAAxxx", "A: 3\nx: 3\n");
}

#[test]
fn match_at_first_and_last_position() {
    assert_same_and_eq("match at both ends", b"A....A", "A: 2\nx: 0\n");
    assert_same_and_eq("x at last position", b"....x", "A: 0\nx: 1\n");
    assert_same_and_eq("A at last position", b"....A", "A: 1\nx: 0\n");
}

#[test]
fn interleaved_matches() {
    assert_same_and_eq("interleaved", b"AxAxAx", "A: 3\nx: 3\n");
}

#[test]
fn only_one_of_the_two_characters_present() {
    assert_same_and_eq("only A present", b"AAAA", "A: 4\nx: 0\n");
    assert_same_and_eq("only x present", b"xxxxx", "A: 0\nx: 5\n");
}

// ---------------------------------------------------------------------------
// NUL handling: the buffer is read as raw bytes but scanned as a C string
// ---------------------------------------------------------------------------

#[test]
fn embedded_nul_truncates_the_scan() {
    // Everything after the first NUL byte is invisible to strchr.
    assert_same_and_eq(
        "NUL in the middle",
        b"AA\0xxAA",
        "A: 2\nx: 0\n",
    );
}

#[test]
fn leading_nul_yields_empty_string() {
    assert_same_and_eq("leading NUL", b"\0AAxx", "A: 0\nx: 0\n");
}

#[test]
fn nul_after_all_matches() {
    assert_same_and_eq("NUL at the very end", b"Ax\0", "A: 1\nx: 1\n");
}

#[test]
fn multiple_nuls() {
    assert_same_and_eq("several NULs", b"A\0\0\0x", "A: 1\nx: 0\n");
}

// ---------------------------------------------------------------------------
// Buffer capacity: sizeof(in) == 1000
// ---------------------------------------------------------------------------

#[test]
fn just_under_capacity() {
    assert_same_and_eq("999 'A' bytes", &vec![b'A'; 999], "A: 999\nx: 0\n");
}

#[test]
fn exactly_at_capacity() {
    // The maximum fread will store. Nothing is left for a NUL terminator.
    assert_same_and_eq("1000 'A' bytes", &vec![b'A'; 1000], "A: 1000\nx: 0\n");
}

#[test]
fn one_byte_over_capacity() {
    // The 1001st byte is never stored, so it cannot be counted.
    let mut input = vec![b'A'; 1000];
    input.push(b'x');
    assert_same_and_eq("1000 'A' then an 'x'", &input, "A: 1000\nx: 0\n");
}

#[test]
fn far_over_capacity_is_truncated() {
    let mut input = vec![b'x'; 1000];
    input.extend(vec![b'A'; 5000]);
    assert_same_and_eq("1000 'x' then 5000 'A'", &input, "A: 0\nx: 1000\n");
}

#[test]
fn matches_straddling_the_capacity_boundary() {
    // 998 filler bytes, then "Ax" ends the buffer, then more that is dropped.
    let mut input = vec![b'.'; 998];
    input.extend_from_slice(b"Ax");
    input.extend_from_slice(b"AAAxxx");
    assert_same_and_eq("boundary straddle", &input, "A: 1\nx: 1\n");
}

#[test]
fn capacity_reached_then_nul() {
    // The NUL is byte 1001 and is therefore never read.
    let mut input = vec![b'A'; 1000];
    input.extend_from_slice(b"\0AAA");
    assert_same_and_eq("NUL just past capacity", &input, "A: 1000\nx: 0\n");
}

// ---------------------------------------------------------------------------
// Byte-level details: no UTF-8 or locale awareness anywhere
// ---------------------------------------------------------------------------

#[test]
fn high_bytes_are_not_special() {
    assert_same_and_eq(
        "bytes >= 0x80 around the matches",
        b"\xff\xfeA\x80\xc3\x84x\xff",
        "A: 1\nx: 1\n",
    );
}

#[test]
fn invalid_utf8_only() {
    assert_same_and_eq("invalid UTF-8, no matches", b"\xc3\x28\xa0\xa1\xff", "A: 0\nx: 0\n");
}

#[test]
fn multibyte_utf8_containing_no_ascii_matches() {
    // "Ä" is 0xC3 0x84 - it must not be confused with 'A'.
    assert_same_and_eq("Ä repeated", "ÄÄÄ".as_bytes(), "A: 0\nx: 0\n");
}

#[test]
fn control_bytes() {
    assert_same_and_eq(
        "control characters mixed in",
        b"\t\rA\x0b\x0cx\x1b[0m",
        "A: 1\nx: 1\n",
    );
}

#[test]
fn binary_payload_all_byte_values() {
    // Every byte 0..=255 in order. The 0x00 at index 0 terminates the string,
    // so both counts are 0.
    let input: Vec<u8> = (0u16..=255).map(|b| b as u8).collect();
    assert_same_and_eq("all byte values, NUL first", &input, "A: 0\nx: 0\n");

    // Same payload with the NUL moved to the end: 'A' (0x41) and 'x' (0x78)
    // now appear exactly once each.
    let input: Vec<u8> = (1u16..=255).map(|b| b as u8).chain([0]).collect();
    assert_same_and_eq("all byte values, NUL last", &input, "A: 1\nx: 1\n");
}

// ---------------------------------------------------------------------------
// stdin / stdout conditions
// ---------------------------------------------------------------------------

#[test]
fn input_delivered_in_several_chunks() {
    // fread keeps reading until the buffer is full or EOF, so a short read
    // must not end the input early.
    let mut child_c = Command::new(c_bin())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn C");
    let mut child_r = Command::new(rust_bin())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn Rust");

    for child in [&mut child_c, &mut child_r] {
        let mut stdin = child.stdin.take().expect("piped stdin");
        for chunk in [&b"AA"[..], &b"xx"[..], &b"A\nx"[..]] {
            stdin.write_all(chunk).expect("write chunk");
            stdin.flush().expect("flush chunk");
            std::thread::sleep(std::time::Duration::from_millis(30));
        }
        drop(stdin);
    }

    let c = child_c.wait_with_output().expect("wait C");
    let r = child_r.wait_with_output().expect("wait Rust");
    assert_eq!(c.stdout, r.stdout, "stdout mismatch on chunked stdin");
    assert_eq!(c.stderr, r.stderr, "stderr mismatch on chunked stdin");
    assert_eq!(c.status.code(), r.status.code(), "status mismatch on chunked stdin");
    assert_eq!(String::from_utf8_lossy(&c.stdout), "A: 3\nx: 3\n");
}

#[test]
fn stdin_at_eof_immediately() {
    // Equivalent to `< /dev/null`: fread fails to read anything, and its
    // return value is ignored, so the zeroed buffer is scanned.
    let dev_null = std::fs::File::open("/dev/null").expect("open /dev/null");
    let c = Command::new(c_bin())
        .stdin(Stdio::from(dev_null))
        .output()
        .expect("run C");
    let dev_null = std::fs::File::open("/dev/null").expect("open /dev/null");
    let r = Command::new(rust_bin())
        .stdin(Stdio::from(dev_null))
        .output()
        .expect("run Rust");
    assert_eq!(c.stdout, r.stdout);
    assert_eq!(c.stderr, r.stderr);
    assert_eq!(c.status.code(), r.status.code());
    assert_eq!(String::from_utf8_lossy(&c.stdout), "A: 0\nx: 0\n");
}

#[test]
fn unreadable_stdin_is_ignored() {
    // A directory fd makes read(2) fail with EISDIR. The C code ignores the
    // fread return value and prints counts for the zeroed buffer; the Rust
    // code must do the same rather than reporting the error.
    let dir = std::fs::File::open(env!("CARGO_MANIFEST_DIR")).expect("open manifest dir");
    let c = Command::new(c_bin())
        .stdin(Stdio::from(dir))
        .output()
        .expect("run C");
    let dir = std::fs::File::open(env!("CARGO_MANIFEST_DIR")).expect("open manifest dir");
    let r = Command::new(rust_bin())
        .stdin(Stdio::from(dir))
        .output()
        .expect("run Rust");
    assert_eq!(c.stdout, r.stdout, "stdout mismatch on unreadable stdin");
    assert_eq!(c.stderr, r.stderr, "stderr mismatch on unreadable stdin");
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "status mismatch on unreadable stdin"
    );
    assert_eq!(String::from_utf8_lossy(&c.stdout), "A: 0\nx: 0\n");
}

#[test]
fn stderr_is_always_empty_and_status_is_always_zero() {
    for input in [&b""[..], &b"A"[..], &b"\0"[..], &vec![b'x'; 2000][..]] {
        let c = run(c_bin(), input);
        let r = run(&rust_bin(), input);
        assert!(c.stderr.is_empty(), "C wrote to stderr unexpectedly");
        assert!(r.stderr.is_empty(), "Rust wrote to stderr unexpectedly");
        assert_eq!(c.status, Some(0));
        assert_eq!(r.status, Some(0));
    }
}

#[test]
fn ignores_command_line_arguments() {
    // main() takes no parameters, so argv must make no difference.
    let args = ["A", "xxx", "--help", "-1"];
    let c = Command::new(c_bin())
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|mut ch| {
            ch.stdin.take().unwrap().write_all(b"Ax")?;
            ch.wait_with_output()
        })
        .expect("run C with args");
    let r = Command::new(rust_bin())
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|mut ch| {
            ch.stdin.take().unwrap().write_all(b"Ax")?;
            ch.wait_with_output()
        })
        .expect("run Rust with args");
    assert_eq!(c.stdout, r.stdout);
    assert_eq!(c.stderr, r.stderr);
    assert_eq!(c.status.code(), r.status.code());
    assert_eq!(String::from_utf8_lossy(&c.stdout), "A: 1\nx: 1\n");
}

// ---------------------------------------------------------------------------
// Output formatting
// ---------------------------------------------------------------------------

#[test]
fn output_format_is_exact() {
    // "A: %d\n" then "x: %d\n": that order, single spaces, no padding, no
    // extra trailing newline, LF (not CRLF).
    let c = run(c_bin(), b"AAxAx");
    assert_eq!(c.stdout, b"A: 3\nx: 2\n");
    let r = run(&rust_bin(), b"AAxAx");
    assert_eq!(r.stdout, b"A: 3\nx: 2\n");
    assert_eq!(c.stdout.iter().filter(|&&b| b == b'\n').count(), 2);
    assert!(!c.stdout.windows(2).any(|w| w == b"\r\n"));
    assert!(!r.stdout.windows(2).any(|w| w == b"\r\n"));
}

// ---------------------------------------------------------------------------
// Deterministic pseudo-random sweep
// ---------------------------------------------------------------------------

#[test]
fn randomised_sweep() {
    // xorshift64*, so the corpus is reproducible without a dev-dependency.
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = move || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state.wrapping_mul(0x2545_F491_4F6C_DD1D)
    };

    let alphabet = [b'A', b'x', b'a', b'X', b'\n', b'\0', b' ', b'.', 0xff, 0x80];
    let lengths = [0usize, 1, 2, 3, 7, 64, 512, 998, 999, 1000, 1001, 1500, 3000];

    for len in lengths {
        for _ in 0..12 {
            let input: Vec<u8> = (0..len)
                .map(|_| alphabet[(next() % alphabet.len() as u64) as usize])
                .collect();
            assert_same(&format!("random input of {len} bytes"), &input);
        }
    }
}
