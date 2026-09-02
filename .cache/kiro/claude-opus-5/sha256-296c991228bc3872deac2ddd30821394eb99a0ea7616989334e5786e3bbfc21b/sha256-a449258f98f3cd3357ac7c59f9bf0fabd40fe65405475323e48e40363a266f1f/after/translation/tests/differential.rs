//! Differential tests: run the C binary and the Rust binary as subprocesses with
//! identical stdin and require byte-identical stdout, byte-identical stderr and
//! an identical exit status.
//!
//! Nothing here links the Rust crate as a library — both programs are driven
//! exactly the way a shell drives them, because that is how they are compared.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Once;

// ---------------------------------------------------------------------------
// locating / building the two executables
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // .../<root>/translation/Cargo.toml -> .../<root>
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the release Rust binary, built on first use so the artifact under
/// test is the same one `cargo build --release` produces.
fn rust_bin() -> PathBuf {
    static BUILD: Once = Once::new();
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let bin = manifest.join("target/release/driver");
    BUILD.call_once(|| {
        let status = Command::new(env!("CARGO"))
            .args(["build", "--release"])
            .current_dir(manifest)
            .status()
            .expect("failed to invoke cargo");
        assert!(status.success(), "cargo build --release failed");
    });
    assert!(bin.is_file(), "rust binary missing at {}", bin.display());
    bin
}

/// Path to the C binary. Uses `c_src/build/driver` when it is already there;
/// otherwise configures and builds into a scratch directory *outside* `c_src`
/// so the C subtree is never written to.
fn c_bin() -> PathBuf {
    static BUILD: Once = Once::new();
    let root = workspace_root();
    let in_tree = root.join("c_src/build/driver");
    if in_tree.is_file() {
        return in_tree;
    }
    let out = root.join("translation/target/c_build");
    let bin = out.join("driver");
    BUILD.call_once(|| {
        if bin.is_file() {
            return;
        }
        std::fs::create_dir_all(&out).expect("create c build dir");
        let cfg = Command::new("cmake")
            .arg("-S")
            .arg(root.join("c_src"))
            .arg("-B")
            .arg(&out)
            .output()
            .expect("failed to invoke cmake");
        assert!(
            cfg.status.success(),
            "cmake configure failed:\n{}",
            String::from_utf8_lossy(&cfg.stderr)
        );
        let build = Command::new("cmake")
            .arg("--build")
            .arg(&out)
            .output()
            .expect("failed to invoke cmake --build");
        assert!(
            build.status.success(),
            "cmake build failed:\n{}",
            String::from_utf8_lossy(&build.stderr)
        );
    });
    assert!(bin.is_file(), "c binary missing at {}", bin.display());
    bin
}

// ---------------------------------------------------------------------------
// running one program
// ---------------------------------------------------------------------------

struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Ok(code)` for a normal exit, `Err(signal)` when killed by a signal.
    status: Result<i32, i32>,
}

fn run(bin: &Path, input: &[u8]) -> Run {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    // The programs read at most two lines and then exit, so a large input can
    // leave the pipe full. Feed stdin from a helper thread and ignore
    // BrokenPipe, which is what a shell's write side sees too.
    let mut stdin = child.stdin.take().expect("piped stdin");
    let data = input.to_vec();
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&data);
        let _ = stdin.flush();
        drop(stdin);
    });

    let out = child.wait_with_output().expect("failed to wait for child");
    let _ = writer.join();

    let status = match out.status.code() {
        Some(code) => Ok(code),
        None => {
            #[cfg(unix)]
            {
                use std::os::unix::process::ExitStatusExt;
                Err(out.status.signal().unwrap_or(-1))
            }
            #[cfg(not(unix))]
            {
                Err(-1)
            }
        }
    };

    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        status,
    }
}

fn show(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            b'\\' => s.push_str("\\\\"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    s
}

/// Assert the C and Rust programs agree on stdout, stderr and exit status.
#[track_caller]
fn assert_same(name: &str, input: &[u8]) {
    let c = run(&c_bin(), input);
    let r = run(&rust_bin(), input);

    assert_eq!(
        show(&c.stdout),
        show(&r.stdout),
        "[{name}] stdout differs (input = \"{}\")",
        show(input)
    );
    assert_eq!(
        c.stdout, r.stdout,
        "[{name}] stdout bytes differ (input = \"{}\")",
        show(input)
    );
    assert_eq!(
        show(&c.stderr),
        show(&r.stderr),
        "[{name}] stderr differs (input = \"{}\")",
        show(input)
    );
    assert_eq!(
        c.stderr, r.stderr,
        "[{name}] stderr bytes differ (input = \"{}\")",
        show(input)
    );
    assert_eq!(
        c.status, r.status,
        "[{name}] exit status differs (input = \"{}\")",
        show(input)
    );
}

/// Same, plus a check that stdout is exactly what we expect, so a test cannot
/// silently pass because both programs became equally broken.
#[track_caller]
fn assert_same_and_stdout(name: &str, input: &[u8], expected_stdout: &str) {
    assert_same(name, input);
    let c = run(&c_bin(), input);
    assert_eq!(
        show(&c.stdout),
        show(expected_stdout.as_bytes()),
        "[{name}] C stdout is not the expected value"
    );
}

fn rep(b: u8, n: usize) -> Vec<u8> {
    vec![b; n]
}

fn cat(parts: &[&[u8]]) -> Vec<u8> {
    parts.concat()
}

// ---------------------------------------------------------------------------
// Phase A — both programs build and run
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_build_and_run() {
    let c = c_bin();
    let r = rust_bin();
    assert!(c.is_file(), "C binary not built: {}", c.display());
    assert!(r.is_file(), "Rust binary not built: {}", r.display());
    // Both must actually execute.
    assert_eq!(run(&c, b"a\nb\n").status, Ok(0));
    assert_eq!(run(&r, b"a\nb\n").status, Ok(0));
}

// ---------------------------------------------------------------------------
// Phase B — the input classes main() and driver() branch on
//
// main(): two fgets into 100-byte zeroed buffers, then s[strlen(s)-1] = 0 on
// each, then driver(). The branch points are: fgets returning NULL (EOF with no
// bytes read), fgets stopping on '\n' vs. filling 99 bytes, and strlen(s) == 0
// (which makes the store go out of bounds).
//
// driver(): printf("%zu\n", strcspn(s1, s2)) — branches are "match at index i"
// vs. "no match at all", and an empty reject set.
// ---------------------------------------------------------------------------

#[test]
fn empty_input_both_fgets_return_null() {
    // Both buffers stay "" ; strlen is 0 so both stores are out of bounds;
    // strcspn("", "") == 0.
    assert_same_and_stdout("empty", b"", "0\n");
}

#[test]
fn only_a_newline_second_fgets_returns_null() {
    // s1 = "\n" -> stripped to ""; s2 never read -> "".
    assert_same_and_stdout("just_newline", b"\n", "0\n");
}

#[test]
fn two_empty_lines() {
    assert_same_and_stdout("two_newlines", b"\n\n", "0\n");
}

#[test]
fn single_line_with_newline_second_fgets_null() {
    // s1 = "abc\n" -> "abc"; s2 = "" -> empty reject set -> strcspn == 3.
    assert_same_and_stdout("one_line_nl", b"abc\n", "3\n");
}

#[test]
fn single_line_without_newline() {
    // fgets stops at EOF, so the last real character is the one stripped:
    // "abc" -> "ab" -> 2.
    assert_same_and_stdout("one_line_no_nl", b"abc", "2\n");
}

#[test]
fn single_char_no_newline() {
    // "a" -> stripped to "" -> 0.
    assert_same_and_stdout("one_char_no_nl", b"a", "0\n");
}

#[test]
fn happy_path_match_in_middle() {
    // s1 = "hello world", s2 = "o" -> first 'o' is at index 4.
    assert_same_and_stdout("basic", b"hello world\no\n", "4\n");
}

#[test]
fn no_character_of_s2_occurs_in_s1() {
    // Full length is returned.
    assert_same_and_stdout("no_match", b"abcdef\nxyz\n", "6\n");
}

#[test]
fn match_at_first_character() {
    assert_same_and_stdout("first_char_match", b"abcdef\na\n", "0\n");
}

#[test]
fn match_at_last_character() {
    assert_same_and_stdout("match_at_end", b"abcZ\nZ\n", "3\n");
}

#[test]
fn second_line_empty_gives_empty_reject_set() {
    // s2 = "\n" -> "" -> strcspn returns strlen(s1).
    assert_same_and_stdout("s2_empty_line", b"abcdef\n\n", "6\n");
}

#[test]
fn multi_character_reject_set_takes_earliest() {
    // s2 = "leo": 'e' at index 1 beats 'l' at 2 and 'o' at 4.
    assert_same_and_stdout("s2_multi", b"hello\nleo\n", "1\n");
}

#[test]
fn single_char_strings() {
    assert_same_and_stdout("s1_single_no_match", b"a\nb\n", "1\n");
    assert_same_and_stdout("s1_single_match", b"a\na\n", "0\n");
}

#[test]
fn third_line_is_never_read() {
    assert_same_and_stdout("third_line_ignored", b"abc\ndef\nghi\n", "3\n");
}

#[test]
fn neither_line_has_a_trailing_newline() {
    // s1 = "abc\n" -> "abc"; s2 = "def" (EOF) -> "de"; 'd' is not in "abc".
    assert_same_and_stdout("no_trailing_nl", b"abc\ndef", "3\n");
}

#[test]
fn crlf_line_endings_leave_the_cr_in_place() {
    // fgets keeps "\r\n"; the strip removes only '\n', so s1 = "abc\r" and
    // s2 = "def\r". '\r' is in the reject set, so the answer is 3, not 4.
    assert_same_and_stdout("crlf", b"abc\r\ndef\r\n", "3\n");
}

// ---------------------------------------------------------------------------
// Phase B — the 99/100-byte fgets boundary (the maximum the code handles)
// ---------------------------------------------------------------------------

#[test]
fn line_of_98_chars_plus_newline_fits() {
    // 98 chars + '\n' = 99 bytes, exactly what fgets(.., 100, ..) accepts.
    let input = cat(&[&rep(b'a', 98), b"\n", b"b\n"]);
    assert_same_and_stdout("s1_98_plus_nl", &input, "98\n");
}

#[test]
fn line_of_99_chars_fills_the_buffer_and_leaves_the_newline() {
    // fgets takes 99 'a's and stops on size, so the '\n' is left for the second
    // fgets: s1 -> 98 'a's, s2 = "\n" -> "" -> 98. The "b" line is never read.
    let input = cat(&[&rep(b'a', 99), b"\nb\n"]);
    assert_same_and_stdout("s1_99_chars", &input, "98\n");
}

#[test]
fn line_of_100_chars_spills_the_100th_into_s2() {
    // s1 = 99 'a's -> 98 'a's; s2 = "a\n" -> "a"; so the match is at index 0.
    let input = cat(&[&rep(b'a', 100), b"\nb\n"]);
    assert_same_and_stdout("s1_100_chars", &input, "0\n");
}

#[test]
fn line_much_longer_than_the_buffer() {
    let input = cat(&[&rep(b'a', 150), b"\nb\n"]);
    assert_same_and_stdout("s1_150_chars", &input, "0\n");
}

#[test]
fn both_lines_overflow_the_buffer() {
    let input = cat(&[&rep(b'a', 120), b"\n", &rep(b'b', 120), b"\n"]);
    assert_same_and_stdout("long_both", &input, "0\n");
}

#[test]
fn reject_char_at_the_far_end_of_a_full_s2() {
    // s2 is 99 bytes (98 'q's + 'Z'), stripped to 98 -> 'Z' is dropped, so the
    // reject set is only 'q' and nothing matches.
    let s1 = cat(&[&rep(b'a', 50), b"Z"]);
    let s2 = cat(&[&rep(b'q', 98), b"Z"]);
    let input = cat(&[&s1, b"\n", &s2, b"\n"]);
    assert_same_and_stdout("s2_long_match", &input, "51\n");
}

#[test]
fn very_large_input_only_first_two_lines_matter() {
    let input = cat(&[&rep(b'x', 500_000), b"\n", &rep(b'y', 500_000), b"\n"]);
    assert_same("huge_input", &input);
}

// ---------------------------------------------------------------------------
// Phase C — paths the happy-path tests miss
// ---------------------------------------------------------------------------

#[test]
fn embedded_nul_truncates_the_string_early() {
    // fgets stores the NUL, but strlen sees length 1, so s1[0] = 0 and s1 is
    // empty -> 0.
    assert_same_and_stdout("nul_in_s1", b"a\0bcd\nz\n", "0\n");
}

#[test]
fn leading_nul_makes_strlen_zero_and_the_store_out_of_bounds() {
    // strlen(s1) == 0 -> the C writes s1[-1]. It must stay unobservable.
    assert_same_and_stdout("nul_first_s1", b"\0abc\nz\n", "0\n");
}

#[test]
fn leading_nul_on_both_lines() {
    assert_same_and_stdout("nul_first_both", b"\0abc\n\0def\n", "0\n");
}

#[test]
fn leading_nul_in_s2_empties_the_reject_set() {
    // strlen(s2) == 0 -> s2[-1] store; reject set empty -> strcspn == strlen(s1).
    assert_same_and_stdout("s2_nul_first", b"abcdef\n\0abc\n", "6\n");
}

#[test]
fn out_of_bounds_store_with_a_completely_full_neighbour_buffer() {
    // Pair a zero-length string (which triggers the s[-1] store) with a
    // neighbour buffer filled to all 99 usable bytes, so any spill into the
    // adjacent array would change the printed number.
    let full = rep(b'b', 99);
    assert_same("oob_s1nul_s2full", &cat(&[b"\0\n", &full, b"\n"]));
    assert_same("oob_s2nul_s1full", &cat(&[&full, b"\n\0\n"]));
    assert_same("oob_s1empty_s2full", &cat(&[b"\n", &full, b"\n"]));
    assert_same("oob_s1nul_s2_over", &cat(&[b"\0\n", &rep(b'b', 120)]));
    assert_same("oob_nul_only", b"\0");
    assert_same("oob_nuls_only", b"\0\0\0");
}

#[test]
fn nul_bytes_only_on_both_lines() {
    assert_same_and_stdout("nul_lines", b"\0\n\0\n", "0\n");
}

#[test]
fn whitespace_is_an_ordinary_character_fgets_does_not_split_on_it() {
    // Unlike scanf, fgets keeps spaces and tabs; " \n" -> " " rejects at index 1.
    assert_same_and_stdout("tab_space", b"a b\tc\n \n", "1\n");
}

#[test]
fn high_bytes_and_invalid_utf8_are_compared_byte_wise() {
    assert_same("high_bytes", b"\xc3\xa9abc\n\xa9\n");
    assert_same("invalid_utf8", b"\xff\xfe\xfd\n\xfe\n");
    assert_same("latin1_match", b"caf\xe9\n\xe9\n");
}

#[test]
fn every_byte_value_1_through_255() {
    // s1 holds bytes 1..=98 (99 bytes with the newline is over, so use 1..=90),
    // s2 rejects a byte in the middle.
    let s1: Vec<u8> = (1u8..=90).filter(|&b| b != b'\n').collect();
    let mut input = s1.clone();
    input.push(b'\n');
    input.push(0x50);
    input.push(b'\n');
    assert_same("bytes_1_90", &input);

    let s1b: Vec<u8> = (150u8..=250).take(90).collect();
    let mut inb = s1b;
    inb.extend_from_slice(b"\n\xc8\n");
    assert_same("bytes_150_240", &inb);
}

#[test]
fn stdin_closed_immediately() {
    // Equivalent to reading from /dev/null: both fgets return NULL.
    assert_same_and_stdout("stdin_closed", b"", "0\n");
}

// ---------------------------------------------------------------------------
// Phase C — randomized differential sweep
//
// A deterministic PRNG (no dev-dependency) biased toward the interesting bytes:
// newline, NUL and a couple of ordinary letters, with lengths that straddle the
// 99/100-byte fgets boundary.
// ---------------------------------------------------------------------------

struct Lcg(u64);

impl Lcg {
    fn next_u32(&mut self) -> u32 {
        // Numerical Recipes constants.
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.0 >> 33) as u32
    }
    fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
}

#[test]
fn randomized_differential_sweep() {
    let mut rng = Lcg(0x5eed_1234_abcd_ef01);
    for case in 0..1500 {
        let len = rng.below(261) as usize;
        let mut input = Vec::with_capacity(len);
        for _ in 0..len {
            let b = match rng.below(10) {
                0 | 1 | 2 => b'\n',
                3 | 4 => 0u8,
                5 => b'a',
                6 => b'b',
                _ => rng.below(256) as u8,
            };
            input.push(b);
        }
        assert_same(&format!("fuzz#{case}"), &input);
    }
}

#[test]
fn randomized_sweep_near_the_buffer_boundary() {
    let mut rng = Lcg(0xdead_beef_0000_0001);
    for case in 0..400 {
        // Lengths 90..=110 on the first line, so fgets stops on '\n' for some
        // cases and on the size limit for others.
        let n1 = 90 + rng.below(21) as usize;
        let n2 = 90 + rng.below(21) as usize;
        let mut input = Vec::new();
        for _ in 0..n1 {
            input.push(if rng.below(20) == 0 { 0u8 } else { b'a' + rng.below(4) as u8 });
        }
        input.push(b'\n');
        for _ in 0..n2 {
            input.push(if rng.below(20) == 0 { 0u8 } else { b'a' + rng.below(4) as u8 });
        }
        input.push(b'\n');
        assert_same(&format!("boundary#{case}"), &input);
    }
}

// ---------------------------------------------------------------------------
// output shape
// ---------------------------------------------------------------------------

#[test]
fn output_is_a_decimal_number_and_one_trailing_newline_on_stdout_only() {
    let c = run(&c_bin(), b"hello world\no\n");
    let r = run(&rust_bin(), b"hello world\no\n");
    assert_eq!(c.stdout, b"4\n");
    assert_eq!(r.stdout, b"4\n");
    assert!(c.stderr.is_empty(), "C wrote to stderr: {}", show(&c.stderr));
    assert!(r.stderr.is_empty(), "Rust wrote to stderr: {}", show(&r.stderr));
    assert_eq!(c.status, Ok(0));
    assert_eq!(r.status, Ok(0));
}
