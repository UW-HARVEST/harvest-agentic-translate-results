//! Differential tests: run the C binary and the Rust binary as *subprocesses*
//! with identical stdin and require byte-identical stdout, byte-identical
//! stderr and an identical exit status.
//!
//! Nothing here links against the Rust crate as a library; both programs are
//! driven exactly the way a shell would drive them.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;

/// `<repo root>` = the parent of the `translation/` crate directory.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the C executable, building it with CMake on first use if needed.
fn c_binary() -> &'static PathBuf {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = repo_root().join("c_src");
        let build = c_src.join("build");
        let bin = build.join("driver");
        if !bin.exists() {
            std::fs::create_dir_all(&build).expect("cannot create c_src/build");
            let cfg = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("failed to run `cmake ..` (is cmake installed?)");
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
                .expect("failed to run `cmake --build .`");
            assert!(
                bld.status.success(),
                "cmake build failed:\n{}\n{}",
                String::from_utf8_lossy(&bld.stdout),
                String::from_utf8_lossy(&bld.stderr)
            );
        }
        assert!(bin.exists(), "C binary missing at {}", bin.display());
        bin
    })
}

/// Path to the Rust executable produced by cargo for this test run.
fn rust_binary() -> &'static PathBuf {
    static R_BIN: OnceLock<PathBuf> = OnceLock::new();
    R_BIN.get_or_init(|| {
        // The integration-test executable lives in target/<profile>/deps/.
        let mut dir = std::env::current_exe().expect("current_exe");
        dir.pop(); // deps/
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
    })
}

fn run(bin: &Path, stdin_bytes: &[u8]) -> Output {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));
    child
        .stdin
        .as_mut()
        .expect("stdin pipe")
        .write_all(stdin_bytes)
        .or_else(|e| {
            // A program that exits without draining stdin can cause EPIPE;
            // that is not a test failure.
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

/// Core assertion: stdout, stderr and exit status must all match.
#[track_caller]
fn assert_same(label: &str, stdin_bytes: &[u8]) {
    let c = run(c_binary(), stdin_bytes);
    let r = run(rust_binary(), stdin_bytes);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch for {label} (input {:?})\n  C   : {:?}\n  Rust: {:?}",
        Escaped(stdin_bytes),
        Escaped(&c.stdout),
        Escaped(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch for {label} (input {:?})\n  C   : {:?}\n  Rust: {:?}",
        Escaped(stdin_bytes),
        Escaped(&c.stderr),
        Escaped(&r.stderr)
    );
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "exit status mismatch for {label} (input {:?}): C={:?} Rust={:?}",
        Escaped(stdin_bytes),
        c.status,
        r.status
    );
}

/// Helper so byte slices print readably in assertion messages.
struct Escaped<'a>(&'a [u8]);
impl std::fmt::Debug for Escaped<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"")?;
        for &b in self.0 {
            match b {
                b'\n' => write!(f, "\\n")?,
                b'\r' => write!(f, "\\r")?,
                b'\t' => write!(f, "\\t")?,
                0 => write!(f, "\\0")?,
                0x20..=0x7e => write!(f, "{}", b as char)?,
                _ => write!(f, "\\x{b:02x}")?,
            }
        }
        write!(f, "\"")
    }
}

// ---------------------------------------------------------------------------
// Phase B: the input classes the C program branches on.
//
// The C program is:
//     fgets(s1, 100, stdin); fgets(s2, 100, stdin);
//     s1[strlen(s1)-1] = 0;  s2[strlen(s2)-1] = 0;
//     printf("%zu\n", strcspn(s1, s2));
//
// Branch/behaviour classes:
//   * fgets returning NULL (EOF before any byte) -> buffer stays ""
//   * fgets stopping at '\n' vs. stopping at EOF vs. stopping at 99 bytes
//   * strlen()-1 chopping the last stored byte (newline, or a real character
//     when the line was not newline-terminated / was truncated)
//   * strlen() == 0  -> strlen()-1 wraps to SIZE_MAX (out-of-bounds store)
//   * strcspn: match at index 0, in the middle, at the last byte, no match
//     (returns strlen(s1)), empty s1, empty s2
// ---------------------------------------------------------------------------

#[test]
fn empty_input() {
    // Both fgets return NULL; both buffers stay "" -> strlen(s)-1 == SIZE_MAX.
    assert_same("empty stdin", b"");
}

#[test]
fn single_newline_only() {
    // s1 = "\n" -> chopped to ""; s2 = "" (EOF/NULL).
    assert_same("single newline", b"\n");
}

#[test]
fn two_empty_lines() {
    assert_same("two newlines", b"\n\n");
}

#[test]
fn single_item_one_line_no_newline() {
    // fgets stops at EOF with no '\n'; strlen-1 chops a real character.
    assert_same("one line, no trailing newline", b"abc");
}

#[test]
fn single_line_with_newline() {
    assert_same("one line with newline", b"abc\n");
}

#[test]
fn single_character_line() {
    // s1 = "x\n" -> "x"; s2 = "" -> strcspn("x", "") == 1.
    assert_same("single char + newline", b"x\n");
}

#[test]
fn happy_path_no_match() {
    assert_same("no common byte", b"abcdef\nxyz\n");
}

#[test]
fn match_at_first_byte() {
    assert_same("match at index 0", b"aaa\naaa\n");
}

#[test]
fn match_in_the_middle() {
    assert_same("match in middle", b"hello world\nlo\n");
}

#[test]
fn match_at_last_byte_after_chop() {
    // s1 = "abc", s2 = "c": strcspn == 2 (last surviving byte).
    assert_same("match at final byte", b"abc\nc\n");
}

#[test]
fn second_line_not_newline_terminated() {
    // s2 = "def" (EOF, no '\n') -> chopped to "de".
    assert_same("second line without newline", b"abc\ndef");
}

#[test]
fn empty_first_line_nonempty_second() {
    assert_same("empty first line", b"\nabc\n");
}

#[test]
fn empty_second_line() {
    // s2 = "\n" -> "" -> strcspn(s1, "") == strlen(s1).
    assert_same("empty second line", b"hello\n\n");
}

#[test]
fn third_line_is_ignored() {
    assert_same("extra input ignored", b"abc\ndef\nghi\njkl\n");
}

// ---------------------------------------------------------------------------
// Phase C: the boundary and error-ish paths.
// ---------------------------------------------------------------------------

#[test]
fn maximum_line_length_99_bytes_no_newline_stored() {
    // fgets stores at most 99 bytes, so the '\n' is NOT consumed by the first
    // call; the leftover "\n" becomes s2, which chops to "".
    let mut input = vec![b'a'; 99];
    input.push(b'\n');
    assert_same("99 bytes then newline", &input);
}

#[test]
fn line_longer_than_buffer_overflows_into_second_read() {
    // 150 bytes: first fgets takes 99, second takes the remaining 51 + '\n'.
    let mut input = vec![b'a'; 150];
    input.push(b'\n');
    input.extend_from_slice(b"tail\n");
    assert_same("150-byte line spills into s2", &input);
}

#[test]
fn line_exactly_98_bytes_plus_newline() {
    let mut input = vec![b'b'; 98];
    input.push(b'\n');
    input.extend_from_slice(b"b\n");
    assert_same("98 bytes + newline", &input);
}

#[test]
fn both_lines_at_maximum_length() {
    let mut input = vec![b'a'; 99];
    input.push(b'\n');
    input.extend(vec![b'a'; 99]);
    input.push(b'\n');
    assert_same("two maximal lines", &input);
}

#[test]
fn exactly_99_bytes_then_eof() {
    // s1 filled to the brim, terminating NUL at index 99; s2 stays "".
    assert_same("99 bytes then EOF", &vec![b'z'; 99]);
}

#[test]
fn very_long_single_run_no_newline_at_all() {
    assert_same("250 bytes, no newline", &vec![b'q'; 250]);
}

#[test]
fn leading_nul_byte_makes_strlen_zero() {
    // fgets stores the NUL, but strlen(s1) == 0 -> the s1[-1] store happens
    // even though a byte *was* read, and strcspn("", s2) == 0.
    assert_same("NUL-led first line", b"\0abc\ndef\n");
}

#[test]
fn nul_bytes_in_both_lines() {
    assert_same("NUL-led both lines", b"\0\n\0\n");
}

#[test]
fn embedded_nul_truncates_the_string() {
    // s1 = "ab\0cd\n": strlen == 2, so s1[1] = 0 -> "a".
    assert_same("embedded NUL in s1", b"ab\0cd\ncb\n");
}

#[test]
fn embedded_nul_in_second_line() {
    assert_same("embedded NUL in s2", b"abcdef\nxy\0c\n");
}

#[test]
fn nul_only_input() {
    assert_same("single NUL byte", b"\0");
}

#[test]
fn nul_led_first_line_with_maximal_second_line() {
    // Probes the s1[strlen(s1)-1] out-of-bounds store next to a full s2.
    let mut input = b"\0\n".to_vec();
    input.extend(vec![b'a'; 99]);
    assert_same("NUL-led s1, 99-byte s2", &input);
}

#[test]
fn crlf_line_endings() {
    // '\r' survives the chop of '\n' and participates in strcspn.
    assert_same("CRLF endings", b"abc\r\ndef\r\n");
}

#[test]
fn carriage_return_is_a_matchable_byte() {
    assert_same("CR in s2's set", b"ab\r\ncd\r\n");
}

#[test]
fn high_bit_bytes() {
    // strcspn compares unsigned chars; make sure >= 0x80 bytes match.
    assert_same("high-bit bytes", b"\xff\xfe\xc3\n\xfe\n");
}

#[test]
fn utf8_multibyte_input() {
    assert_same("UTF-8 input", "héllo wörld\nö\n".as_bytes());
}

#[test]
fn invalid_utf8_input() {
    // The Rust program must not choke on non-UTF-8 bytes.
    assert_same("invalid UTF-8", b"\x80\x81ab\n\x81\n");
}

#[test]
fn whitespace_only_lines() {
    assert_same("spaces", b"   \n \n");
    assert_same("tabs", b"tab\there\n\t\n");
}

#[test]
fn scanf_style_whitespace_is_not_skipped() {
    // fgets (unlike scanf) keeps leading whitespace: s1 = "  ab".
    assert_same("leading whitespace kept", b"  ab\n \n");
}

#[test]
fn second_line_shares_only_the_chopped_character() {
    // s1 = "abcd" (from "abcde"), s2 = "e" -> no match, prints 4.
    assert_same("chopped char not matched", b"abcde\ne\n");
}

#[test]
fn newline_char_cannot_match_because_it_was_chopped() {
    assert_same("newline chopped from both", b"abc\n\ndef\n");
}

// ---------------------------------------------------------------------------
// Broad randomized sweep over the alphabet that exercises every branch above.
// ---------------------------------------------------------------------------

#[test]
fn randomized_sweep() {
    // Deterministic xorshift so failures are reproducible.
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    // Bytes chosen to hit NULs, newlines, high-bit bytes and repeats.
    const ALPHA: [u8; 8] = [b'a', b'b', 0, b'\n', 0xff, b' ', b'z', b'\r'];

    for _ in 0..400 {
        let len = (next() % 211) as usize;
        let input: Vec<u8> = (0..len).map(|_| ALPHA[(next() % 8) as usize]).collect();
        assert_same("randomized", &input);
    }
}
