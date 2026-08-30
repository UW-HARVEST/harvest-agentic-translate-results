//! Differential tests: run the original C program and the Rust translation as
//! subprocesses on identical stdin, and require byte-identical stdout, stderr
//! and exit status.
//!
//! The Rust code is NEVER called as a library here -- both programs are driven
//! exactly the way a shell would drive them, because that is how the
//! translation is graded.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// Workspace root, i.e. the directory holding both `c_src/` and `translation/`.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the Rust binary under test, as built by cargo.
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Path to the C binary, building it with cmake on first use if necessary.
/// `c_src/` is only ever read and configured out-of-tree into `c_src/build/`.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = repo_root().join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");
        if exe.exists() {
            return exe;
        }

        std::fs::create_dir_all(&build).expect("could not create c_src/build");

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

        assert!(exe.exists(), "C binary missing after build: {}", exe.display());
        exe
    })
}

/// What one program produced for one input.
struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Some(code)` for a normal exit, `None` if killed by a signal.
    code: Option<i32>,
}

/// Spawn `prog`, write `input` to its stdin, and collect everything it produced.
fn run(prog: &Path, input: &[u8]) -> Run {
    let mut child = Command::new(prog)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", prog.display()));

    // The child may stop reading (it only ever consumes 1000 bytes), so a
    // failed/short write is expected for oversized inputs and must not fail
    // the test.
    {
        let mut stdin = child.stdin.take().expect("piped stdin");
        let _ = stdin.write_all(input);
        let _ = stdin.flush();
        // dropping stdin closes it, signalling EOF
    }

    let out = child.wait_with_output().expect("failed to wait for child");
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
    }
}

/// Render bytes readably for assertion messages.
fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).escape_debug().to_string()
}

/// Core assertion: for `input`, C and Rust agree on stdout, stderr and status.
fn assert_same(desc: &str, input: &[u8]) {
    let c = run(c_bin(), input);
    let r = run(&rust_bin(), input);

    assert_eq!(
        c.stdout,
        r.stdout,
        "[{desc}] stdout mismatch\n  C   : \"{}\"\n  Rust: \"{}\"",
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "[{desc}] stderr mismatch\n  C   : \"{}\"\n  Rust: \"{}\"",
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        c.code, r.code,
        "[{desc}] exit status mismatch: C={:?} Rust={:?}",
        c.code, r.code
    );
}

// ---------------------------------------------------------------------------
// Phase B: the inputs the C program branches on.
//
// The C program is:
//     foo(in, c)  -- strchr loop counting occurrences of `c`
//     driver(in)  -- printf("A: %d\n", ...), printf("x: %d\n", ...)
//     main()      -- char in[1000] = ""; fread(in, 1, 1000, stdin);
//
// The observable branch points are:
//   * foo's loop body executing zero times vs. many times (strchr == NULL)
//   * the NUL terminator, which bounds every strchr walk
//   * the 1000-byte fread cap
// ---------------------------------------------------------------------------

#[test]
fn empty_input() {
    // fread returns 0; `in` stays "" from its initializer.
    assert_same("empty", b"");
}

#[test]
fn single_byte_inputs() {
    assert_same("single A", b"A");
    assert_same("single x", b"x");
    assert_same("single unrelated byte", b"Q");
    assert_same("single newline", b"\n");
    assert_same("single space", b" ");
}

#[test]
fn neither_char_present() {
    // Both strchr calls return NULL immediately: foo's loop never runs.
    assert_same("no match", b"hello world");
    assert_same("digits", b"1234567890");
}

#[test]
fn only_one_of_the_two_chars() {
    assert_same("only A", b"AAAA");
    assert_same("only x", b"xxxx");
}

#[test]
fn both_chars_interleaved() {
    assert_same("mixed", b"AxAxA");
    assert_same("xA pairs", b"xAxAxA");
    assert_same("separated", b"AAA...xxx");
}

#[test]
fn counting_is_case_sensitive() {
    // 'a' must not count as 'A', and 'X' must not count as 'x'.
    assert_same("lowercase a only", b"aaaa");
    assert_same("uppercase X only", b"XXXX");
    assert_same("mixed case", b"aXbAcx");
}

#[test]
fn embedded_newlines_are_just_bytes() {
    // fread does not stop at a newline (unlike fgets), so bytes on later
    // lines are still counted.
    assert_same("multiline", b"A\nx\nAA");
    assert_same("trailing newline", b"Ax\n");
    assert_same("leading newline", b"\nAx");
    assert_same("blank lines", b"A\n\n\nx\n\n");
    assert_same("crlf", b"A\r\nx\r\n");
}

#[test]
fn whitespace_and_tabs() {
    // fread does not skip leading whitespace the way scanf("%s") would.
    assert_same("spaces around", b"   A   x   ");
    assert_same("tabs", b"\tA\tx\t");
}

// ---------------------------------------------------------------------------
// Phase C: the paths that are easy to leave untested.
// ---------------------------------------------------------------------------

#[test]
fn nul_byte_terminates_the_counted_region() {
    // `in` is treated as a NUL-terminated string, so everything at or after
    // the first NUL byte is invisible to strchr -- even though fread happily
    // copied it into the buffer.
    assert_same("NUL truncates", b"A\0AAAxxx");
    assert_same("leading NUL", b"\0AAAxxx");
    assert_same("NUL after x", b"x\0AAAA");
    assert_same("multiple NULs", b"Ax\0\0Ax\0Ax");
}

#[test]
fn all_nul_input() {
    // A full buffer of NULs is indistinguishable from empty input.
    assert_same("1000 NULs", &[0u8; 1000]);
    assert_same("1 NUL", b"\0");
}

#[test]
fn every_byte_value() {
    // Exercises the whole 1..=255 range at once; exactly one 'A' and one 'x'.
    let all: Vec<u8> = (1u8..=255).collect();
    assert_same("bytes 1..=255", &all);
}

#[test]
fn high_bytes_are_not_confused_with_ascii() {
    // On platforms where `char` is signed, strchr still compares as unsigned
    // char; 0xC1 must not be mistaken for 'A' (0x41), nor 0xF8 for 'x' (0x78).
    assert_same("high bit set", b"\xC1\xF8\xE1\xD8Ax");
    assert_same("utf8 text", "héllo wörld Ax".as_bytes());
}

#[test]
fn buffer_boundary_sizes() {
    // fread reads at most sizeof(in) == 1000 bytes.
    for n in [1usize, 2, 998, 999] {
        assert_same(&format!("{n} x A"), &vec![b'A'; n]);
    }
}

#[test]
fn input_exactly_fills_the_buffer() {
    // Exactly 1000 bytes: the buffer holds no NUL terminator at all.
    assert_same("1000 A", &vec![b'A'; 1000]);
    assert_same("1000 x", &vec![b'x'; 1000]);
    // 999 payload bytes plus an explicit NUL is well-defined and must agree.
    let mut v = vec![b'A'; 999];
    v.push(0);
    assert_same("999 A + NUL", &v);
}

#[test]
fn input_longer_than_the_buffer_is_truncated() {
    // Bytes past offset 1000 are never read, so they cannot be counted.
    let mut v = vec![b'A'; 1000];
    v.extend(std::iter::repeat(b'x').take(500));
    assert_same("1000 A then 500 x", &v);

    let mut v2 = vec![b'A'; 500];
    v2.extend(std::iter::repeat(b'x').take(499));
    v2.push(b'\n');
    v2.extend(std::iter::repeat(b'A').take(1000)); // dropped by the 1000 cap
    assert_same("500 A / 499 x / newline / overflow", &v2);
}

#[test]
fn short_reads_are_retried_until_eof() {
    // fread(in, 1, 1000, stdin) keeps reading until the buffer is full or EOF,
    // so a producer that dribbles bytes out in many small writes must still be
    // fully counted. This is written as many separate small writes with the
    // pipe flushed in between.
    let c = run_dribbled(c_bin());
    let r = run_dribbled(&rust_bin());
    assert_eq!(c.stdout, r.stdout, "dribbled stdout mismatch");
    assert_eq!(c.stderr, r.stderr, "dribbled stderr mismatch");
    assert_eq!(c.code, r.code, "dribbled exit status mismatch");
    // Sanity check that the whole stream really was consumed.
    assert_eq!(c.stdout, b"A: 64\nx: 64\n", "dribbled: unexpected counts");
}

/// Feed `prog` 64 separate flushed 2-byte writes, pausing between each so the
/// reader is very likely to observe short reads.
fn run_dribbled(prog: &Path) -> Run {
    let mut child = Command::new(prog)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", prog.display()));
    {
        let mut stdin = child.stdin.take().expect("piped stdin");
        for _ in 0..64 {
            stdin.write_all(b"Ax").expect("write to child stdin");
            stdin.flush().expect("flush child stdin");
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
    let out = child.wait_with_output().expect("failed to wait for child");
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
    }
}

#[test]
fn stdin_is_immediately_at_eof() {
    // Equivalent to `prog < /dev/null`: fread returns 0 and reports EOF.
    // The C code ignores fread's return value, so this must still print zeros.
    let dev_null = std::fs::File::open("/dev/null").expect("open /dev/null");
    let c = Command::new(c_bin())
        .stdin(Stdio::from(dev_null))
        .output()
        .expect("run C with /dev/null stdin");
    let dev_null = std::fs::File::open("/dev/null").expect("open /dev/null");
    let r = Command::new(rust_bin())
        .stdin(Stdio::from(dev_null))
        .output()
        .expect("run Rust with /dev/null stdin");

    assert_eq!(c.stdout, r.stdout, "/dev/null stdout mismatch");
    assert_eq!(c.stderr, r.stderr, "/dev/null stderr mismatch");
    assert_eq!(c.status.code(), r.status.code(), "/dev/null status mismatch");
    assert_eq!(c.stdout, b"A: 0\nx: 0\n", "/dev/null: unexpected output");
}

#[test]
fn output_format_is_exact() {
    // Pin the literal bytes of printf("A: %d\n") / printf("x: %d\n"):
    // labels, colon, single space, decimal count, trailing newline, and
    // nothing on stderr.
    let r = run(&rust_bin(), b"AAxxx");
    assert_eq!(
        r.stdout, b"A: 2\nx: 3\n",
        "unexpected Rust stdout: \"{}\"",
        show(&r.stdout)
    );
    assert!(r.stderr.is_empty(), "expected empty stderr");
    assert_eq!(r.code, Some(0), "expected exit code 0");

    // And confirm the C program agrees, so the expectation above is grounded.
    let c = run(c_bin(), b"AAxxx");
    assert_eq!(c.stdout, b"A: 2\nx: 3\n");
    assert!(c.stderr.is_empty());
    assert_eq!(c.code, Some(0));
}

#[test]
fn always_exits_zero() {
    // main() unconditionally `return 0;` -- there is no error path at all.
    for input in [
        &b""[..],
        &b"A"[..],
        &b"\0"[..],
        &b"no match here"[..],
        &[0xFFu8; 1000][..],
    ] {
        let c = run(c_bin(), input);
        let r = run(&rust_bin(), input);
        assert_eq!(c.code, Some(0), "C should always exit 0");
        assert_eq!(r.code, Some(0), "Rust should always exit 0");
        assert!(c.stderr.is_empty() && r.stderr.is_empty());
    }
}

#[test]
fn randomized_differential_sweep() {
    // Deterministic xorshift PRNG: broad coverage of lengths (including around
    // the 1000-byte cap) and byte values (including NULs), with 'A'/'x'
    // over-represented so the counts vary.
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    for case in 0..200 {
        let len = (next() % 1100) as usize; // straddles 1000
        let mut input = Vec::with_capacity(len);
        for _ in 0..len {
            let pick = next() % 10;
            input.push(match pick {
                0..=2 => b'A',
                3..=5 => b'x',
                6 => 0u8,
                7 => b'\n',
                _ => (next() % 256) as u8,
            });
        }
        assert_same(&format!("random case {case} (len {len})"), &input);
    }
}
