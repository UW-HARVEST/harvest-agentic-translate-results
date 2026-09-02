//! Differential tests: run the C binary and the Rust binary as *subprocesses*
//! and require byte-identical stdout, byte-identical stderr, and an identical
//! exit status for every input.
//!
//! Nothing here links the translation as a library. The C program is only
//! comparable by execution, so both sides are driven exactly the way a shell
//! drives them.
//!
//! The C program (`c_src/src/main.c`) is:
//!
//! ```c
//! void printLine(const char *line) { if (line != NULL) printf("%s\n", line); }
//! void bad()  { char *data;                printLine(data); }
//! void good() { char *data; data = "string"; printLine(data); }
//! int main() { int x = 0; scanf("%d", &x); if (x) good(); else bad(); return 0; }
//! ```
//!
//! so the input classes are driven entirely by what `scanf("%d", &x)` leaves in
//! `x`, plus the stream-level side effects of that one call.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

// ---------------------------------------------------------------------------
// Locating and building the two binaries
// ---------------------------------------------------------------------------

/// Repository root: the directory holding both `c_src/` and `translation/`.
fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is `<root>/translation`.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the C executable, building it with CMake if it is not there yet.
///
/// Build only -- `c_src/` itself is never modified; CMake writes exclusively
/// into `c_src/build/`, which is its own output directory.
fn c_binary() -> PathBuf {
    let c_src = repo_root().join("c_src");
    let build = c_src.join("build");
    let bin = build.join("driver");
    if bin.exists() {
        return bin;
    }

    std::fs::create_dir_all(&build).expect("create c_src/build");
    let conf = Command::new("cmake")
        .arg("..")
        .current_dir(&build)
        .output()
        .expect("run cmake (is cmake installed?)");
    assert!(
        conf.status.success(),
        "cmake configure failed:\n{}\n{}",
        String::from_utf8_lossy(&conf.stdout),
        String::from_utf8_lossy(&conf.stderr)
    );
    let comp = Command::new("cmake")
        .args(["--build", "."])
        .current_dir(&build)
        .output()
        .expect("run cmake --build");
    assert!(
        comp.status.success(),
        "cmake build failed:\n{}\n{}",
        String::from_utf8_lossy(&comp.stdout),
        String::from_utf8_lossy(&comp.stderr)
    );

    assert!(bin.exists(), "C binary missing after build: {}", bin.display());
    bin
}

/// Path to the Rust executable under test.
///
/// `CARGO_BIN_EXE_<name>` is set by cargo for integration tests and points at
/// the binary cargo just built, so the test can never race the build.
fn rust_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

// ---------------------------------------------------------------------------
// Running one program
// ---------------------------------------------------------------------------

/// What a run produced. `code` is `Some(n)` for a normal exit and `None` when
/// the process was killed by a signal; `signal` carries that signal number.
#[derive(PartialEq, Eq)]
struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    code: Option<i32>,
    signal: Option<i32>,
}

impl std::fmt::Debug for Run {
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

impl From<Output> for Run {
    fn from(o: Output) -> Run {
        use std::os::unix::process::ExitStatusExt;
        Run {
            stdout: o.stdout,
            stderr: o.stderr,
            code: o.status.code(),
            signal: o.status.signal(),
        }
    }
}

/// Run `bin` with `input` on stdin (a pipe) and collect everything.
fn run_with_stdin(bin: &Path, input: &[u8]) -> Run {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", bin.display()));

    let mut stdin = child.stdin.take().expect("stdin pipe");
    let owned = input.to_vec();
    // Write from a helper thread: a large input can exceed the pipe capacity,
    // which would deadlock if the parent wrote and waited on the same thread.
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&owned);
        // Dropping `stdin` closes the pipe, so the child sees EOF.
    });

    let out = child.wait_with_output().expect("wait_with_output");
    writer.join().expect("stdin writer thread");
    Run::from(out)
}

// ---------------------------------------------------------------------------
// The assertion
// ---------------------------------------------------------------------------

/// Assert that the C and Rust programs agree on stdout, stderr and exit status
/// for `input`.
fn assert_same(label: &str, input: &[u8]) {
    let c = run_with_stdin(&c_binary(), input);
    let r = run_with_stdin(&rust_binary(), input);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout differs for {label} (input {:?})\n  C:    {:?}\n  Rust: {:?}",
        Trunc(input),
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr differs for {label} (input {:?})\n  C:    {:?}\n  Rust: {:?}",
        Trunc(input),
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "exit status differs for {label} (input {:?})\n  C:    {c:?}\n  Rust: {r:?}",
        Trunc(input),
    );
}

/// Keeps panic messages readable when an input is thousands of bytes long.
struct Trunc<'a>(&'a [u8]);
impl std::fmt::Debug for Trunc<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.len() <= 64 {
            write!(f, "{}", String::from_utf8_lossy(self.0))
        } else {
            write!(
                f,
                "{}... ({} bytes)",
                String::from_utf8_lossy(&self.0[..64]),
                self.0.len()
            )
        }
    }
}

// ===========================================================================
// Phase A -- both binaries exist and are runnable
// ===========================================================================

#[test]
fn both_binaries_build_and_run() {
    let c = c_binary();
    let r = rust_binary();
    assert!(c.exists(), "C binary not built: {}", c.display());
    assert!(r.exists(), "Rust binary not built: {}", r.display());

    // A trivial run must succeed for both, otherwise every later comparison is
    // comparing against a program that never ran.
    for bin in [&c, &r] {
        let run = run_with_stdin(bin, b"1\n");
        assert_eq!(
            run.code,
            Some(0),
            "{} did not exit 0 on a trivial input: {run:?}",
            bin.display()
        );
    }
}

// ===========================================================================
// Phase B -- the branches `main` actually takes
// ===========================================================================

/// `x != 0` -> `good()` -> `printLine("string")` -> `"string\n"`.
#[test]
fn nonzero_takes_good_branch() {
    for (label, input) in [
        ("one", &b"1"[..]),
        ("one_nl", b"1\n"),
        ("negative_one", b"-1"),
        ("negative_one_nl", b"-1\n"),
        ("plus_one", b"+1"),
        ("large", b"123456"),
        ("int_max", b"2147483647"),
        ("int_min", b"-2147483648"),
    ] {
        assert_same(label, input);
    }
}

/// `x == 0` -> `bad()` -> `printLine(<uninitialized>)`.
///
/// Reading the uninitialized `char *data` is undefined behavior; the reference
/// build's observable result is the thing being matched, whatever it is. Both
/// programs must agree, which is what this asserts.
#[test]
fn zero_takes_bad_branch() {
    for (label, input) in [
        ("zero", &b"0"[..]),
        ("zero_nl", b"0\n"),
        ("negative_zero", b"-0"),
        ("plus_zero", b"+0"),
        ("many_zeros", b"0000000000"),
        ("zeros_then_nl", b"000\n"),
    ] {
        assert_same(label, input);
    }
}

/// `scanf` never converts anything, so `x` keeps its initializer `0` and the
/// `bad()` path runs. Empty input is the `EOF`-return case.
#[test]
fn empty_input() {
    assert_same("empty", b"");
}

/// A single item -- exactly one integer and nothing else -- for both branches.
#[test]
fn single_item_only() {
    assert_same("single_nonzero_no_newline", b"7");
    assert_same("single_zero_no_newline", b"0");
}

/// `%d` skips leading whitespace *including newlines*, so a value on a later
/// line is still converted. This is the `scanf`-reads-across-newlines behavior
/// that an `fgets`-based translation would get wrong.
#[test]
fn whitespace_is_skipped_across_newlines() {
    for (label, input) in [
        ("leading_spaces", &b"   5"[..]),
        ("leading_tabs", b"\t\t5"),
        ("leading_newlines", b"\n\n\n5"),
        ("mixed_ws_then_value", b"  \t\n\n  42\n"),
        ("all_c_space_chars", b" \t\n\x0b\x0c\r5"),
        ("newlines_then_zero", b"\n\n\n0"),
        ("only_whitespace", b"   \t\n  "),
        ("only_newline", b"\n"),
        ("only_tabs", b"\t\t\t"),
        ("vertical_tab_formfeed_cr", b"\x0b\x0c\r"),
    ] {
        assert_same(label, input);
    }
}

/// Matching failure: the first non-whitespace byte is not part of an integer, so
/// `scanf` returns 0 and leaves `x` at 0.
#[test]
fn matching_failure_leaves_x_zero() {
    for (label, input) in [
        ("letters", &b"abc"[..]),
        ("space_then_letters", b" abc"),
        ("single_letter", b"e"),
        ("dot", b"."),
        ("dot_five", b".5"),
        ("hash", b"#"),
        ("underscore", b"_1"),
        ("quote", b"\"1\""),
        ("newline_then_letters", b"\nzz"),
    ] {
        assert_same(label, input);
    }
}

/// Conversion stops at the first non-digit; the trailing garbage is pushed back
/// and never looked at, so `3abc` converts to 3.
#[test]
fn conversion_stops_at_first_non_digit() {
    for (label, input) in [
        ("digits_then_letters", &b"3abc"[..]),
        ("zero_then_letters", b"0abc"),
        ("hex_literal_reads_as_zero", b"0x10"),
        ("float_reads_integer_part", b"1.9"),
        ("zero_point_nine", b"0.9"),
        ("digits_then_space_digits", b"1 2 3"),
        ("digits_then_comma", b"12,34"),
        ("digits_then_minus", b"12-34"),
    ] {
        assert_same(label, input);
    }
}

/// Sign handling, including a sign that is never followed by a digit.
#[test]
fn sign_handling() {
    for (label, input) in [
        ("bare_minus", &b"-"[..]),
        ("bare_plus", b"+"),
        ("bare_minus_nl", b"-\n"),
        ("bare_plus_nl", b"+\n"),
        ("minus_space_one", b"- 1"),
        ("double_minus", b"--1"),
        ("minus_plus", b"-+1"),
        ("plus_minus", b"+-1"),
        ("minus_letter", b"-a"),
        ("plus_dot", b"+."),
    ] {
        assert_same(label, input);
    }
}

/// Overflow, truncation and signedness exactly as the C performs them: the
/// accumulation saturates at `long` width and the store through `int *`
/// truncates, so e.g. `4294967296` lands in `x` as 0 (-> `bad()`) while
/// `2147483648` lands as `INT_MIN` (-> `good()`).
#[test]
fn overflow_and_truncation() {
    for (label, input) in [
        ("int_max_plus_one", &b"2147483648"[..]),
        ("int_min_minus_one", b"-2147483649"),
        ("u32_max", b"4294967295"),
        ("two_to_the_32_truncates_to_zero", b"4294967296"),
        ("two_to_the_32_plus_one", b"4294967297"),
        ("negative_two_to_the_32", b"-4294967296"),
        ("long_max", b"9223372036854775807"),
        ("long_max_plus_one_saturates", b"9223372036854775808"),
        ("long_min", b"-9223372036854775808"),
        ("long_min_minus_one_saturates", b"-9223372036854775809"),
        ("u64_max", b"18446744073709551615"),
        ("u64_max_plus_one", b"18446744073709551616"),
        ("u64_max_plus_two", b"18446744073709551617"),
        ("way_past_u64", b"99999999999999999999999999"),
        ("fifty_digits", b"12345678901234567890123456789012345678901234567890"),
        ("negative_way_past_u64", b"-99999999999999999999999999"),
        ("leading_zeros_then_one", b"0000000000000000000000001"),
        ("leading_zeros_then_big", b"000000000004294967296"),
    ] {
        assert_same(label, input);
    }
}

/// Non-ASCII and NUL bytes reach the same matching-failure path; a NUL is not
/// whitespace and not a digit.
#[test]
fn non_ascii_and_nul_bytes() {
    for (label, input) in [
        ("nul", &b"\x00"[..]),
        ("nul_then_digit", b"\x001"),
        ("digit_then_nul", b"1\x00"),
        ("high_bit_0x80", b"\x80"),
        ("high_bit_0xff", b"\xff"),
        ("utf8_multibyte", "é1".as_bytes()),
        ("all_high_bytes", b"\xc0\xc1\xc2"),
    ] {
        assert_same(label, input);
    }
}

// ===========================================================================
// Phase C -- paths not reached above
// ===========================================================================

/// The digit run spans glibc's 4096-byte stdin buffer, forcing a refill in the
/// middle of one conversion.
#[test]
fn input_spanning_the_stdin_buffer() {
    for n in [4095usize, 4096, 4097, 8191, 8192, 8193] {
        // Saturates to LONG_MAX -> truncates to -1 -> `good()`.
        let mut v = vec![b'9'; n];
        assert_same(&format!("{n}_nines"), &v);

        // All zeros -> x stays 0 -> `bad()`.
        v.iter_mut().for_each(|b| *b = b'0');
        assert_same(&format!("{n}_zeros"), &v);

        // Whitespace longer than the buffer, then a value.
        let mut ws = vec![b' '; n];
        ws.push(b'7');
        assert_same(&format!("{n}_spaces_then_7"), &ws);

        // Leading zeros longer than the buffer, then a significant digit.
        let mut lead = vec![b'0'; n];
        lead.push(b'1');
        assert_same(&format!("{n}_zeros_then_1"), &lead);
    }
}

/// Much more input than either program will ever convert.
#[test]
fn very_large_input() {
    let mut big = Vec::with_capacity(200_001);
    big.push(b'1');
    big.extend(std::iter::repeat(b'x').take(200_000));
    assert_same("1_then_200k_junk", &big);

    let mut big0 = Vec::with_capacity(200_001);
    big0.push(b'0');
    big0.extend(std::iter::repeat(b'\n').take(200_000));
    assert_same("0_then_200k_newlines", &big0);
}

/// stdin is a regular file rather than a pipe. glibc reads a whole block and
/// rewinds the shared file offset while tearing the stream down at exit, so a
/// process that inherits the same descriptor afterwards sees the unread
/// remainder. `sh` runs the pair so the descriptor really is shared.
#[test]
fn regular_file_stdin_leaves_the_offset_where_c_leaves_it() {
    let dir = std::env::temp_dir().join(format!("driver_diff_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");

    let cases: [(&str, Vec<u8>); 14] = [
        ("short_tail", b"1 REST-OF-FILE\nsecond line\n".to_vec()),
        ("zero_short_tail", b"0 REST-OF-FILE\nsecond line\n".to_vec()),
        // The digit run stops at a non-digit, which `scanf` pushes back; the
        // pushed-back byte must be readable again by the next reader.
        ("pushback_after_digits", b"12 REST-OF-FILE\n".to_vec()),
        ("pushback_after_zero", b"0x10 REST-OF-FILE\n".to_vec()),
        // Matching failure: the offending byte is pushed back too.
        ("pushback_on_matching_failure", b"aREST-OF-FILE\n".to_vec()),
        ("pushback_letter_then_space", b"a REST-OF-FILE\n".to_vec()),
        ("pushback_after_leading_space", b" aREST-OF-FILE\n".to_vec()),
        ("pushback_after_newline", b"\naREST-OF-FILE\n".to_vec()),
        ("pushback_after_nul", b"\x00REST-OF-FILE\n".to_vec()),
        ("pushback_on_dot", b".5REST-OF-FILE\n".to_vec()),
        // A sign consumed before the failure is *not* pushed back -- only the
        // single byte that stopped the conversion is.
        ("sign_then_letter", b"+aREST-OF-FILE\n".to_vec()),
        ("minus_then_letter", b"-aREST-OF-FILE\n".to_vec()),
        ("tail_past_one_block", {
            let mut v = b"1".to_vec();
            v.extend(std::iter::repeat(b'x').take(20_000));
            v
        }),
        ("tail_past_several_blocks", {
            let mut v = b"1".to_vec();
            v.extend(std::iter::repeat(b'y').take(60_000));
            v
        }),
    ];

    for (label, content) in cases {
        let path = dir.join(format!("{label}.txt"));
        std::fs::write(&path, &content).expect("write input file");

        let mut outs = Vec::new();
        for bin in [c_binary(), rust_binary()] {
            // `cat` inherits fd 0 from the shell and reads whatever the driver
            // left behind.
            let script = format!(
                "{} ; printf '|LEFTOVER|' ; cat",
                shell_quote(bin.to_str().expect("utf-8 path"))
            );
            let out = Command::new("sh")
                .arg("-c")
                .arg(&script)
                .stdin(std::fs::File::open(&path).expect("open input file"))
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .expect("run sh");
            outs.push(Run::from(out));
        }

        assert_eq!(
            outs[0].stdout,
            outs[1].stdout,
            "shared-stdin leftover differs for {label}:\n  C:    {} bytes\n  Rust: {} bytes",
            outs[0].stdout.len(),
            outs[1].stdout.len()
        );
        assert_eq!(
            (outs[0].code, outs[0].signal),
            (outs[1].code, outs[1].signal),
            "shared-stdin exit status differs for {label}"
        );
    }

    let _ = std::fs::remove_dir_all(&dir);
}

/// stdin is a pipe with more data than glibc's buffer. Here glibc *cannot*
/// rewind, so the block it swallowed is gone for the next reader -- the mirror
/// image of the regular-file case, and the reason the buffer size matters.
#[test]
fn pipe_stdin_swallows_exactly_one_block() {
    for total in [100usize, 5_000, 60_000] {
        let mut outs = Vec::new();
        for bin in [c_binary(), rust_binary()] {
            let full = format!(
                "{{ printf '1' ; head -c {total} /dev/zero | tr '\\0' 'z' ; }} \
                 | {{ {} >/dev/null ; printf '|LEFTOVER|' ; wc -c ; }}",
                shell_quote(bin.to_str().expect("utf-8 path"))
            );
            let out = Command::new("sh")
                .arg("-c")
                .arg(&full)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .expect("run sh");
            outs.push(Run::from(out));
        }
        assert_eq!(
            outs[0].stdout,
            outs[1].stdout,
            "pipe leftover differs for total={total}:\n  C:    {:?}\n  Rust: {:?}",
            String::from_utf8_lossy(&outs[0].stdout),
            String::from_utf8_lossy(&outs[1].stdout)
        );
    }
}

/// stdout is a pipe whose read end is already closed. A C program keeps the
/// default `SIGPIPE` disposition and dies from signal 13; the Rust runtime
/// ignores `SIGPIPE` unless that is undone. The child is held at its `scanf`
/// until after the read end is dropped, so the race is decided, not hoped for.
#[test]
fn closed_stdout_pipe_kills_both_the_same_way() {
    for input in [&b"1\n"[..], b"0\n"] {
        let mut runs = Vec::new();
        for bin in [c_binary(), rust_binary()] {
            let mut child = Command::new(&bin)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("spawn");

            // Close the read end *before* the child can write anything: it is
            // still blocked reading stdin, which has not been fed yet.
            drop(child.stdout.take());

            let mut stdin = child.stdin.take().expect("stdin pipe");
            let _ = stdin.write_all(input);
            drop(stdin);

            let status = child.wait().expect("wait");
            use std::os::unix::process::ExitStatusExt;
            runs.push((status.code(), status.signal()));
        }
        assert_eq!(
            runs[0], runs[1],
            "closed-stdout exit status differs for input {:?}: C {:?} vs Rust {:?}",
            String::from_utf8_lossy(input),
            runs[0],
            runs[1]
        );
    }
}

/// Unusual descriptors: no stdin at all, and a stdin that cannot be read.
#[test]
fn unusual_stdin_descriptors() {
    // /dev/null: immediate EOF, same as empty input.
    for label in ["dev_null"] {
        let mut outs = Vec::new();
        for bin in [c_binary(), rust_binary()] {
            let out = Command::new(&bin)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .expect("run");
            outs.push(Run::from(out));
        }
        assert_eq!(outs[0].stdout, outs[1].stdout, "{label}: stdout differs");
        assert_eq!(outs[0].stderr, outs[1].stderr, "{label}: stderr differs");
        assert_eq!(
            (outs[0].code, outs[0].signal),
            (outs[1].code, outs[1].signal),
            "{label}: exit status differs"
        );
    }

    // fd 0 closed entirely -> read() fails with EBADF -> scanf reports EOF.
    // A directory as stdin -> read() fails with EISDIR, likewise.
    for (label, redirect) in [("fd0_closed", "0<&-"), ("stdin_is_a_directory", "< /")] {
        let mut outs = Vec::new();
        for bin in [c_binary(), rust_binary()] {
            let script = format!("{} {redirect}", shell_quote(bin.to_str().expect("utf-8")));
            let out = Command::new("sh")
                .arg("-c")
                .arg(&script)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .expect("run sh");
            outs.push(Run::from(out));
        }
        assert_eq!(outs[0].stdout, outs[1].stdout, "{label}: stdout differs");
        assert_eq!(outs[0].stderr, outs[1].stderr, "{label}: stderr differs");
        assert_eq!(
            (outs[0].code, outs[0].signal),
            (outs[1].code, outs[1].signal),
            "{label}: exit status differs"
        );
    }
}

/// `main()` is declared without parameters and ignores argv, so extra arguments
/// change nothing for either program.
#[test]
fn extra_argv_is_ignored() {
    for input in [&b"1\n"[..], b"0\n", b""] {
        let mut outs = Vec::new();
        for bin in [c_binary(), rust_binary()] {
            let mut child = Command::new(&bin)
                .args(["--help", "-x", "extra", ""])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("spawn");
            let mut stdin = child.stdin.take().expect("stdin");
            let _ = stdin.write_all(input);
            drop(stdin);
            outs.push(Run::from(child.wait_with_output().expect("wait")));
        }
        assert_eq!(outs[0], outs[1], "argv-ignoring behavior differs");
    }
}

/// A broad sweep over every combination of the bytes `%d` treats specially,
/// which is the cheapest way to reach branch orderings the named cases above
/// might have missed.
#[test]
fn exhaustive_short_inputs_over_the_significant_alphabet() {
    const ALPHABET: &[u8] = b"0123456789+- \t\nax\x00";

    // Length 1 and 2: exhaustive.
    for &a in ALPHABET {
        assert_same("len1", &[a]);
        for &b in ALPHABET {
            assert_same("len2", &[a, b]);
        }
    }

    // Length 3: a reduced but still wide alphabet, keeping the run time sane.
    const SHORT: &[u8] = b"0-+ 1\n9a";
    for &a in SHORT {
        for &b in SHORT {
            for &c in SHORT {
                assert_same("len3", &[a, b, c]);
            }
        }
    }
}

/// Deterministic pseudo-random inputs over the same alphabet plus arbitrary
/// bytes. Seeded, so a failure is reproducible.
#[test]
fn randomized_inputs_are_identical() {
    // xorshift64*, so the test has no dependency on an RNG crate.
    let mut state: u64 = 0x2545F4914F6CDD1D;
    let mut next = move || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state.wrapping_mul(0x2545F4914F6CDD1D)
    };

    const ALPHABET: &[u8] = b"0123456789+-. \t\n\rabcxeE\x00\x80\xff";

    for _ in 0..400 {
        let len = (next() % 24) as usize;
        let input: Vec<u8> = (0..len)
            .map(|_| ALPHABET[(next() % ALPHABET.len() as u64) as usize])
            .collect();
        assert_same("random_alphabet", &input);
    }

    for _ in 0..300 {
        let len = (next() % 20) as usize;
        let input: Vec<u8> = (0..len).map(|_| (next() % 256) as u8).collect();
        assert_same("random_bytes", &input);
    }
}

// ---------------------------------------------------------------------------

/// Single-quote a string for `sh`.
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}
