//! Differential tests: run the C binary and the Rust binary as *subprocesses*
//! and compare stdout, stderr and exit status byte-for-byte / value-for-value.
//!
//! Nothing here links against the translation as a library -- both programs are
//! driven exactly the way a shell drives them, because that is how they are
//! compared.
//!
//! ## Branches in `c_src/src/main.c` that these cases enumerate
//!
//! ```c
//! static void print_hex(unsigned char *p, int len) {
//!     for (int i = 0; i < len; i++) printf("%02x", p[i]);   // (3)
//!     printf("\n");                                          // (4)
//! }
//! void driver(int x) { print_hex((unsigned char *)&x, sizeof(x)); }
//! int main() { int x = 0; scanf("%d", &x); driver(x); return 0; }  // (1)(2)(5)
//! ```
//!
//! 1. `scanf("%d", &x)` -- the return value is *ignored*, so every failure mode
//!    leaves `x` at its initializer `0` and the program still prints `00000000`:
//!      * input failure at EOF (empty input / whitespace-only input)
//!      * matching failure on a non-digit (`abc`, `.5`, a NUL byte, `- 5`)
//!      * matching failure after a lone sign (`-`, `+`, `-\n`, `--5`)
//!    On success glibc's `%d` skips *any* leading whitespace including
//!    newlines, takes an optional sign, consumes digits, runs `strtol`
//!    (saturating at `LONG_MAX`/`LONG_MIN`) and then assigns that `long` to an
//!    `int`, truncating.
//! 2. `int x = 0;` -- the initializer is observable on every failure path.
//! 3. the `i < len` loop always runs exactly `sizeof(int)` == 4 times, and
//!    `%02x` has a digit-vs-letter branch per nibble, so the cases below cover
//!    all 16 nibble values.
//! 4. the unconditional trailing `printf("\n")`.
//! 5. `return 0` -- exit status 0, *except* that the C program has the default
//!    `SIGPIPE` disposition and so dies from signal 13 when its exit-time
//!    flush hits a closed pipe.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Locating / building the two binaries
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    // translation/ -> repo root
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Build `c_src` with CMake (once per test binary) and return the executable.
/// The C program is the ground truth; if it cannot be built there is nothing
/// meaningful to compare against, so this panics loudly rather than skipping.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = repo_root().join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");

        if !exe.is_file() {
            std::fs::create_dir_all(&build).expect("create c_src/build");
            let cfg = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("failed to run `cmake ..` -- is cmake installed?");
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
        assert!(exe.is_file(), "C binary missing after build: {}", exe.display());
        exe
    })
}

/// Every Rust binary worth exercising: the one `cargo test` just built, plus
/// the `--release` artifact when it is present (that is what ships).
fn rust_bins() -> &'static [PathBuf] {
    static BINS: OnceLock<Vec<PathBuf>> = OnceLock::new();
    BINS.get_or_init(|| {
        let mut v = vec![PathBuf::from(env!("CARGO_BIN_EXE_driver"))];
        let rel = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("release")
            .join("driver");
        if rel.is_file() && !v.contains(&rel) {
            v.push(rel);
        }
        v
    })
}

// ---------------------------------------------------------------------------
// Running one program
// ---------------------------------------------------------------------------

/// Everything observable about a run: stdout, stderr, exit code, death signal.
#[derive(PartialEq, Eq)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    code: Option<i32>,
    signal: Option<i32>,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "stdout={:?} stderr={:?} code={:?} signal={:?}",
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr),
            self.code,
            self.signal
        )
    }
}

fn outcome_of(child: Child, stdin_data: Option<&[u8]>) -> Outcome {
    use std::os::unix::process::ExitStatusExt;
    let mut child = child;
    if let Some(data) = stdin_data {
        // A short write can block if the child never drains it, so hand the
        // write to a thread and let `wait_with_output` drive the pipes.
        let mut sink = child.stdin.take().expect("stdin piped");
        let owned = data.to_vec();
        std::thread::spawn(move || {
            let _ = sink.write_all(&owned);
            let _ = sink.flush();
            // dropping `sink` closes the pipe -> the child sees EOF
        });
    }
    let out = child.wait_with_output().expect("wait_with_output");
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

/// Run `bin` with `input` on stdin and no arguments.
fn run(bin: &Path, input: &[u8]) -> Outcome {
    let child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", bin.display()));
    outcome_of(child, Some(input))
}

/// Assert the C program and every Rust program agree on all three channels.
fn assert_same(label: &str, input: &[u8]) {
    let expected = run(c_bin(), input);
    for rb in rust_bins() {
        let actual = run(rb, input);
        assert_eq!(
            expected.stdout,
            actual.stdout,
            "stdout mismatch for {label} (input {:?}) using {}\n  C   : {:?}\n  rust: {:?}",
            String::from_utf8_lossy(input),
            rb.display(),
            expected,
            actual
        );
        assert_eq!(
            expected.stderr,
            actual.stderr,
            "stderr mismatch for {label} (input {:?}) using {}\n  C   : {:?}\n  rust: {:?}",
            String::from_utf8_lossy(input),
            rb.display(),
            expected,
            actual
        );
        assert_eq!(
            (expected.code, expected.signal),
            (actual.code, actual.signal),
            "exit status mismatch for {label} (input {:?}) using {}\n  C   : {:?}\n  rust: {:?}",
            String::from_utf8_lossy(input),
            rb.display(),
            expected,
            actual
        );
    }
}

// ---------------------------------------------------------------------------
// Phase A -- the programs exist and run
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_are_runnable() {
    assert!(c_bin().is_file(), "C binary not built");
    for rb in rust_bins() {
        assert!(rb.is_file(), "rust binary missing: {}", rb.display());
    }
    // Sanity: the documented happy path.
    let out = run(c_bin(), b"1\n");
    assert_eq!(out.stdout, b"01000000\n");
    assert_eq!(out.code, Some(0));
}

// ---------------------------------------------------------------------------
// Phase B -- the input classes the C actually branches on
// ---------------------------------------------------------------------------

/// scanf input failure at EOF: nothing at all to read, `x` keeps its `0`.
#[test]
fn empty_input() {
    assert_same("empty", b"");
}

/// Whitespace-only input: the leading-whitespace skip consumes everything and
/// then hits EOF -- still an input failure, still `x == 0`.
#[test]
fn whitespace_only_input() {
    for (label, input) in [
        ("newline", &b"\n"[..]),
        ("spaces", b"   "),
        ("tabs", b"\t\t"),
        ("crlf", b"\r\n"),
        ("vtab_ff", b"\x0b\x0c"),
        ("all_c_spaces", b" \t\n\x0b\x0c\r"),
        ("many_newlines", b"\n\n\n\n\n"),
    ] {
        assert_same(label, input);
    }
}

/// A single successful item, the happy path, with and without a trailing
/// newline (the C reads a number, not a line).
#[test]
fn single_value() {
    for (label, input) in [
        ("zero", &b"0"[..]),
        ("zero_nl", b"0\n"),
        ("one", b"1"),
        ("one_nl", b"1\n"),
        ("seven", b"7"),
        ("ten", b"10"),
        ("255", b"255"),
        ("256", b"256"),
        ("65535", b"65535"),
        ("65536", b"65536"),
        ("16777215", b"16777215"),
        ("16777216", b"16777216"),
    ] {
        assert_same(label, input);
    }
}

/// Signs. `+` is accepted by `%d`; `-0` is still zero.
#[test]
fn signs() {
    for (label, input) in [
        ("plus_42", &b"+42"[..]),
        ("minus_42", b"-42"),
        ("minus_1", b"-1"),
        ("minus_zero", b"-0"),
        ("plus_zero", b"+0"),
        ("plus_nl", b"+7\n"),
        ("ws_then_minus", b"   -9"),
    ] {
        assert_same(label, input);
    }
}

/// `scanf` skips whitespace *across newlines* -- unlike `fgets`, the number may
/// live several lines down.
#[test]
fn scanf_reads_across_newlines() {
    for (label, input) in [
        ("leading_newlines", &b"\n\n\n42"[..]),
        ("mixed_ws", b"  \t\n\r\x0b\x0c 7"),
        ("newline_then_negative", b"\n-5\n"),
        ("blank_lines_then_num", b"\n \n \n 123 \n"),
    ] {
        assert_same(label, input);
    }
}

/// Conversion stops at the first non-digit; the rest of stdin is simply never
/// looked at, and only the *first* number is used.
#[test]
fn stops_at_first_non_digit() {
    for (label, input) in [
        ("trailing_alpha", &b"12abc"[..]),
        ("two_numbers", b"5 6"),
        ("two_numbers_nl", b"5\n6\n"),
        ("hex_prefix_reads_zero", b"0x10"),
        ("float_reads_int_part", b"3.99"),
        ("comma", b"7,8"),
        ("exponent_ignored", b"2e5"),
        ("num_then_sign", b"4-5"),
        ("num_then_nul", b"9\x00abc"),
    ] {
        assert_same(label, input);
    }
}

// ---------------------------------------------------------------------------
// Phase B/C -- error paths: every way `scanf` can fail to store anything
// ---------------------------------------------------------------------------

/// Matching failure on the very first non-whitespace character.
#[test]
fn matching_failure_on_non_digit() {
    for (label, input) in [
        ("alpha", &b"abc"[..]),
        ("upper", b"ABC"),
        ("dot", b".5"),
        ("dot_only", b"."),
        ("comma_first", b",5"),
        ("nul_first", b"\x005"),
        ("high_byte", b"\xff\xfe"),
        ("punct", b"!@#"),
        ("letter_x", b"x1"),
        ("star", b"*"),
        ("hash_only", b"#"),
    ] {
        assert_same(label, input);
    }
}

/// A sign that is not followed by a digit: matching failure, `x` stays 0.
#[test]
fn matching_failure_after_lone_sign() {
    for (label, input) in [
        ("minus_eof", &b"-"[..]),
        ("plus_eof", b"+"),
        ("minus_newline", b"-\n"),
        ("plus_newline", b"+\n"),
        ("minus_space_digit", b"- 5"),
        ("plus_space_digit", b"+ 5"),
        ("double_minus", b"--5"),
        ("double_plus", b"++5"),
        ("plus_minus", b"+-5"),
        ("minus_alpha", b"-a"),
        ("minus_dot", b"-."),
        ("plus_nl_then_digit", b"+\n5"),
        ("ws_then_minus_eof", b"   -"),
    ] {
        assert_same(label, input);
    }
}

// ---------------------------------------------------------------------------
// Phase C -- integer boundaries: truncation, signedness, saturation
// ---------------------------------------------------------------------------

/// The exact `int` limits.
#[test]
fn int_boundaries() {
    for (label, input) in [
        ("int_max", &b"2147483647"[..]),
        ("int_min", b"-2147483648"),
        ("int_max_minus_1", b"2147483646"),
        ("int_min_plus_1", b"-2147483647"),
    ] {
        assert_same(label, input);
    }
}

/// Past the `int` limits: glibc converts with `strtol` (64-bit `long`) and then
/// truncates into the `int`, so these wrap rather than saturate.
#[test]
fn long_to_int_truncation() {
    for (label, input) in [
        ("int_max_plus_1", &b"2147483648"[..]),      // -> INT_MIN
        ("int_max_plus_2", b"2147483649"),
        ("int_min_minus_1", b"-2147483649"),         // -> INT_MAX
        ("u32_max", b"4294967295"),                  // -> -1
        ("two_pow_32", b"4294967296"),               // -> 0
        ("two_pow_32_plus_1", b"4294967297"),        // -> 1
        ("neg_two_pow_32", b"-4294967296"),          // -> 0
        ("two_pow_33", b"8589934592"),               // -> 0
        ("mixed_high_low", b"4023233417"),           // -> 0xefcdab89
        ("trillion", b"1000000000000"),
        ("neg_trillion", b"-1000000000000"),
    ] {
        assert_same(label, input);
    }
}

/// At and past the `long` limits, where glibc's `strtol` saturates at
/// `LONG_MAX` / `LONG_MIN` before the truncation to `int`.
#[test]
fn long_saturation() {
    for (label, input) in [
        ("long_max_minus_1", &b"9223372036854775806"[..]),
        ("long_max", b"9223372036854775807"),
        ("long_max_plus_1", b"9223372036854775808"), // ERANGE -> LONG_MAX -> -1
        ("long_min", b"-9223372036854775808"),       //          LONG_MIN ->  0
        ("long_min_minus_1", b"-9223372036854775809"),
        ("u64_max", b"18446744073709551615"),
        ("u64_max_plus_1", b"18446744073709551616"),
        ("thirty_six_nines", b"999999999999999999999999999999999999"),
        ("thirty_six_nines_neg", b"-999999999999999999999999999999999999"),
    ] {
        assert_same(label, input);
    }
}

/// Leading zeros are still decimal digits (not octal) and do not overflow.
#[test]
fn leading_zeros() {
    for (label, input) in [
        ("zeros_then_5", &b"000000000000000000000005"[..]),
        ("neg_zeros_then_12", b"-00000000000000000000000012"),
        ("plus_zeros_then_1", b"+0000001"),
        ("only_zeros", b"00000000000000000000"),
        ("zeros_then_intmax", b"0000002147483647"),
    ] {
        assert_same(label, input);
    }
}

/// Very long digit runs, straddling the sizes glibc's internal scan buffer
/// grows through. The saturating conversion must not be length-sensitive.
#[test]
fn very_long_digit_runs() {
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    for k in [
        1usize, 2, 5, 9, 10, 11, 18, 19, 20, 21, 31, 32, 33, 63, 64, 65, 127, 128, 129, 255, 256,
        257, 511, 512, 513, 1000, 4095, 4096, 4097,
    ] {
        cases.push((format!("nines_{k}"), vec![b'9'; k]));
        cases.push((format!("neg_nines_{k}"), {
            let mut v = vec![b'-'];
            v.extend(std::iter::repeat(b'9').take(k));
            v
        }));
        cases.push((format!("one_then_{k}_zeros"), {
            let mut v = vec![b'1'];
            v.extend(std::iter::repeat(b'0').take(k));
            v
        }));
        cases.push((format!("{k}_zeros_then_7"), {
            let mut v = vec![b'0'; k];
            v.push(b'7');
            v
        }));
    }
    for (label, input) in &cases {
        assert_same(label, input);
    }
}

// ---------------------------------------------------------------------------
// Phase C -- output formatting: `%02x` over all 16 nibble values
// ---------------------------------------------------------------------------

/// `%02x` is lowercase and zero-padded, and the loop emits exactly 4 bytes
/// (8 hex digits) plus one `\n`. These inputs make every nibble 0..=f appear.
#[test]
fn hex_formatting_covers_all_nibbles() {
    for (label, input) in [
        // bytes 01 23 45 67 -> "01234567"
        ("nibbles_0_to_7", &b"1732584193"[..]),
        // bytes 89 ab cd ef -> "89abcdef"
        ("nibbles_8_to_f", b"-271733879"),
        // 0 -> "00000000": zero padding on every byte
        ("all_zero_bytes", b"0"),
        // -1 -> "ffffffff": every nibble is a letter
        ("all_ff_bytes", b"-1"),
        // 0x0a0a0a0a: a single-letter nibble next to a zero nibble
        ("repeating_0a", b"168430090"),
        // 0x10101010
        ("repeating_10", b"269488144"),
        // 0x000000ff -> "ff000000"
        ("low_byte_ff", b"255"),
        // 0x0000ff00 -> "00ff0000"
        ("second_byte_ff", b"65280"),
        // 0x00ff0000 -> "0000ff00"
        ("third_byte_ff", b"16711680"),
        // 0xff000000 -> "000000ff"
        ("high_byte_ff", b"-16777216"),
    ] {
        assert_same(label, input);
    }

    // And pin the exact bytes, so a change in spacing/case/newline is caught
    // even if both programs were to drift together.
    assert_eq!(run(c_bin(), b"1732584193").stdout, b"01234567\n");
    assert_eq!(run(c_bin(), b"-271733879").stdout, b"89abcdef\n");
    for rb in rust_bins() {
        assert_eq!(run(rb, b"1732584193").stdout, b"01234567\n");
        assert_eq!(run(rb, b"-271733879").stdout, b"89abcdef\n");
    }
}

/// Output is always exactly 9 bytes with a single trailing newline and stderr
/// is always empty -- no prompt, no extra spacing.
#[test]
fn output_shape_is_always_nine_bytes() {
    for input in [&b""[..], b"0", b"-1", b"abc", b"-", b"2147483647", b"\n\n5"] {
        let c = run(c_bin(), input);
        assert_eq!(c.stdout.len(), 9, "input {input:?} -> {c:?}");
        assert_eq!(*c.stdout.last().unwrap(), b'\n');
        assert!(c.stderr.is_empty());
        for rb in rust_bins() {
            let r = run(rb, input);
            assert_eq!(r.stdout.len(), 9, "input {input:?} -> {r:?}");
            assert_eq!(*r.stdout.last().unwrap(), b'\n');
            assert!(r.stderr.is_empty());
        }
    }
}

// ---------------------------------------------------------------------------
// Phase C -- environmental branches around the process, not the parser
// ---------------------------------------------------------------------------

/// stdin redirected from /dev/null: read returns EOF immediately.
#[test]
fn stdin_from_devnull() {
    fn go(bin: &Path) -> Outcome {
        let child = Command::new(bin)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn");
        outcome_of(child, None)
    }
    let expected = go(c_bin());
    assert_eq!(expected.stdout, b"00000000\n");
    for rb in rust_bins() {
        assert_eq!(go(rb), expected, "devnull stdin, {}", rb.display());
    }
}

/// `main()` is declared without parameters, so extra argv is ignored.
#[test]
fn extra_argv_is_ignored() {
    fn go(bin: &Path) -> Outcome {
        let child = Command::new(bin)
            .args(["--help", "5", "garbage"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn");
        outcome_of(child, Some(b"5"))
    }
    let expected = go(c_bin());
    for rb in rust_bins() {
        assert_eq!(go(rb), expected, "extra argv, {}", rb.display());
    }
}

/// stdout redirected to /dev/null: the write succeeds, nothing is visible.
#[test]
fn stdout_to_devnull() {
    fn go(bin: &Path) -> (Vec<u8>, Option<i32>, Option<i32>) {
        let child = Command::new(bin)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn");
        let o = outcome_of(child, Some(b"5"));
        (o.stderr, o.code, o.signal)
    }
    let expected = go(c_bin());
    assert_eq!(expected.1, Some(0));
    for rb in rust_bins() {
        assert_eq!(go(rb), expected, "stdout to /dev/null, {}", rb.display());
    }
}

/// A gigantic stdin: `scanf` only consumes the leading number, so the trailing
/// megabytes must not change the result (nor deadlock).
#[test]
fn huge_stdin() {
    let mut num_then_junk = b"42\n".to_vec();
    num_then_junk.extend(std::iter::repeat(b'x').take(1 << 20));

    let mut ws_then_num = vec![b' '; 1 << 20];
    ws_then_num.extend_from_slice(b"42");

    let mut junk_first = vec![b'z'; 1 << 20];
    junk_first.extend_from_slice(b"42");

    assert_same("huge_junk_after_number", &num_then_junk);
    assert_same("huge_whitespace_prefix", &ws_then_num);
    assert_same("huge_junk_before_number", &junk_first);
}

/// The C program keeps the default `SIGPIPE` disposition, so writing to a pipe
/// with no reader kills it with signal 13 -- it does *not* exit 0. The Rust
/// runtime sets `SIGPIPE` to `SIG_IGN`, so the translation has to restore the
/// default to match. This is a real mismatch this suite caught (see ERRORS.md).
#[test]
fn dies_from_sigpipe_when_stdout_has_no_reader() {
    use std::os::unix::process::ExitStatusExt;
    fn go(bin: &Path) -> (Option<i32>, Option<i32>) {
        let mut child = Command::new(bin)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn");
        // Close the read end *first*. The child is still blocked reading stdin,
        // so there is no race: by the time it prints, the pipe is broken.
        drop(child.stdout.take().expect("stdout piped"));
        {
            let mut sink = child.stdin.take().expect("stdin piped");
            let _ = sink.write_all(b"5\n");
        }
        let status = child.wait().expect("wait");
        (status.code(), status.signal())
    }
    let expected = go(c_bin());
    assert_eq!(
        expected,
        (None, Some(13)),
        "the C program is expected to die from SIGPIPE here"
    );
    for rb in rust_bins() {
        assert_eq!(
            go(rb),
            expected,
            "broken stdout pipe: exit status must match the C program, {}",
            rb.display()
        );
    }
}

// ---------------------------------------------------------------------------
// Phase C -- brute force over the classes above
// ---------------------------------------------------------------------------

/// Exhaustive sweep of short inputs built from the alphabet the parser reacts
/// to (digits, signs, every C whitespace character, and separators). This is
/// what turns "I think I enumerated the branches" into "I checked them".
#[test]
fn exhaustive_short_inputs() {
    let alphabet: &[u8] = b"0159 \t\n+-a.\x00";

    // length 0, 1 and 2 exhaustively
    assert_same("len0", b"");
    for &a in alphabet {
        assert_same("len1", &[a]);
    }
    for &a in alphabet {
        for &b in alphabet {
            assert_same("len2", &[a, b]);
        }
    }
    // length 3 over a reduced alphabet keeps the runtime sane while still
    // covering sign/digit/whitespace orderings.
    let small: &[u8] = b"09 \n+-a";
    for &a in small {
        for &b in small {
            for &c in small {
                assert_same("len3", &[a, b, c]);
            }
        }
    }
}

/// Pseudo-random inputs over the same alphabet, deterministic seed so a failure
/// is reproducible.
#[test]
fn randomized_inputs() {
    let alphabet: &[u8] = b"0123456789 \t\n\r+-abcxX.\x00\xff,eE";
    let mut state: u64 = 0x243F_6A88_85A3_08D3;
    let mut next = move || {
        // xorshift64*
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state.wrapping_mul(0x2545_F491_4F6C_DD1D)
    };
    for _ in 0..400 {
        let len = (next() % 18) as usize;
        let input: Vec<u8> = (0..len)
            .map(|_| alphabet[(next() % alphabet.len() as u64) as usize])
            .collect();
        assert_same("random", &input);
    }
}

/// Numeric sweep: every decimal string near a power of two, and a spread of
/// ordinary values, checked through the full pipeline.
#[test]
fn numeric_sweep() {
    let mut inputs: Vec<String> = Vec::new();
    for bits in 0..=64u32 {
        let base = 1u128 << bits;
        for delta in -2i128..=2 {
            let v = base as i128 + delta;
            inputs.push(v.to_string());
            inputs.push(format!("-{}", v.abs()));
            inputs.push(format!("+{}", v.abs()));
        }
    }
    for v in [0i64, 1, 9, 10, 99, 100, 12345, -12345, 1_000_000, -1_000_000] {
        inputs.push(v.to_string());
    }
    for s in &inputs {
        assert_same("numeric_sweep", s.as_bytes());
    }
}
