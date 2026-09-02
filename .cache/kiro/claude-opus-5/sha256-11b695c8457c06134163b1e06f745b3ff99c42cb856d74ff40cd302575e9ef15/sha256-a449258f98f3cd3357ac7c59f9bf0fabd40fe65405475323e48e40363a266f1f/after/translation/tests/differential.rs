//! Differential harness: locates (and if necessary builds) the reference C
//! executable and the translated Rust executable, then runs both as
//! subprocesses and compares stdout, stderr and exit status byte for byte.
//!
//! The Rust code is never called as a library from here -- the binary is
//! driven exactly the way a shell would drive it, because that is how the
//! translation is graded.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// The working directory that holds both `c_src/` and `translation/`.
fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is `<root>/translation`.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the reference C executable.
///
/// Uses `c_src/build/driver` when it is already there. Otherwise configures
/// and builds out-of-tree into `translation/target/c_build`, so that nothing
/// is ever written inside `c_src/`.
fn c_binary() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let root = workspace_root();

        if let Ok(p) = std::env::var("DRIVER_C_BIN") {
            let p = PathBuf::from(p);
            assert!(p.is_file(), "DRIVER_C_BIN={} is not a file", p.display());
            return p;
        }

        let prebuilt = root.join("c_src/build/driver");
        if prebuilt.is_file() {
            return prebuilt;
        }

        let src = root.join("c_src");
        let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/c_build");
        std::fs::create_dir_all(&out).expect("create C build directory");

        let cfg = Command::new("cmake")
            .arg("-S")
            .arg(&src)
            .arg("-B")
            .arg(&out)
            .output()
            .expect("failed to run cmake (is cmake installed?)");
        assert!(
            cfg.status.success(),
            "cmake configure failed:\n{}\n{}",
            String::from_utf8_lossy(&cfg.stdout),
            String::from_utf8_lossy(&cfg.stderr)
        );

        let bld = Command::new("cmake")
            .arg("--build")
            .arg(&out)
            .output()
            .expect("failed to run cmake --build");
        assert!(
            bld.status.success(),
            "cmake build failed:\n{}\n{}",
            String::from_utf8_lossy(&bld.stdout),
            String::from_utf8_lossy(&bld.stderr)
        );

        let built = out.join("driver");
        assert!(
            built.is_file(),
            "C driver was not produced at {}",
            built.display()
        );
        built
    })
}

/// Path to the translated Rust executable.
///
/// Prefers the `--release` binary, which is what the task builds and what the
/// grader runs; falls back to the binary cargo builds for this test target.
fn rust_binary() -> &'static Path {
    static RUST_BIN: OnceLock<PathBuf> = OnceLock::new();
    RUST_BIN.get_or_init(|| {
        if let Ok(p) = std::env::var("DRIVER_RUST_BIN") {
            let p = PathBuf::from(p);
            assert!(p.is_file(), "DRIVER_RUST_BIN={} is not a file", p.display());
            return p;
        }
        let release = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/driver");
        if release.is_file() {
            return release;
        }
        PathBuf::from(env!("CARGO_BIN_EXE_driver"))
    })
}

/// Everything a caller can observe about one run of the program.
#[derive(PartialEq, Eq)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Some(code)` for a normal exit, `None` when killed by a signal.
    code: Option<i32>,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "exit={:?} stdout={:?} stderr={:?}",
            self.code,
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr)
        )
    }
}

fn run(exe: &Path, stdin_bytes: &[u8]) -> Outcome {
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));

    {
        let mut sink = child.stdin.take().expect("stdin was piped");
        // The program may exit before consuming all of stdin (a short read on
        // the first line is enough to make it bail out), so a broken pipe here
        // is expected and must not fail the test.
        let _ = sink.write_all(stdin_bytes);
        let _ = sink.flush();
    }

    let out = child.wait_with_output().expect("wait for child");
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
    }
}

/// Assert the C and Rust executables agree on stdout, stderr and exit status.
#[track_caller]
fn assert_same(stdin_bytes: &[u8]) {
    let c = run(c_binary(), stdin_bytes);
    let r = run(rust_binary(), stdin_bytes);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout differs for input {:?}\n  C:    {:?}\n  Rust: {:?}",
        Escaped(stdin_bytes),
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr differs for input {:?}\n  C:    {:?}\n  Rust: {:?}",
        Escaped(stdin_bytes),
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
    assert_eq!(
        c.code,
        r.code,
        "exit status differs for input {:?}\n  C:    {:?}\n  Rust: {:?}",
        Escaped(stdin_bytes),
        c.code,
        r.code
    );
}

struct Escaped<'a>(&'a [u8]);

impl std::fmt::Debug for Escaped<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let shown = &self.0[..self.0.len().min(200)];
        for &b in shown {
            match b {
                b'\n' => write!(f, "\\n")?,
                b'\r' => write!(f, "\\r")?,
                b'\t' => write!(f, "\\t")?,
                0x20..=0x7e => write!(f, "{}", b as char)?,
                _ => write!(f, "\\x{b:02x}")?,
            }
        }
        if shown.len() < self.0.len() {
            write!(f, "...<{} bytes total>", self.0.len())?;
        }
        Ok(())
    }
}

/// Build a three-line stdin the way `main` expects it.
fn input(op: &str, param: &str, decisions: &str) -> Vec<u8> {
    format!("{op}\n{param}\n{decisions}\n").into_bytes()
}

// ---------------------------------------------------------------------------
// Phase A sanity: both executables exist, run, and produce a known result.
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_are_runnable() {
    let stdin = input("0", "0", "yyy");
    let c = run(c_binary(), &stdin);
    let r = run(rust_binary(), &stdin);

    // rwx -> apply_permissions returns 100 + 7.
    assert_eq!(c.stdout, b"107\n", "unexpected C baseline: {c:?}");
    assert_eq!(c.code, Some(0));
    assert_eq!(r.stdout, c.stdout, "Rust baseline differs: {r:?}");
    assert_eq!(r.stderr, c.stderr);
    assert_eq!(r.code, c.code);
}

// ---------------------------------------------------------------------------
// main.c: the three fgets calls and their error paths.
// ---------------------------------------------------------------------------

#[test]
fn stdin_truncated_at_every_fgets() {
    // EOF before the operation line -> "Error reading operation", exit 1.
    assert_same(b"");
    // EOF before the parameter line -> "Error reading parameter", exit 1.
    assert_same(b"0\n");
    assert_same(b"0"); // no newline, still one successful fgets
    // EOF before the decision line -> "Error reading decision string", exit 1.
    assert_same(b"0\n0\n");
    assert_same(b"0\n0");
    // All three lines present.
    assert_same(b"0\n0\nyyy\n");
    assert_same(b"0\n0\nyyy"); // final line without a newline
}

#[test]
fn blank_lines_and_missing_newlines() {
    assert_same(b"\n");
    assert_same(b"\n\n");
    assert_same(b"\n\n\n"); // empty decision string -> length 0 -> -1
    assert_same(b"0\n0\n\n");
    assert_same(b"2\n0\n\n");
    assert_same(b"3\n0\n\n");
    assert_same(b"\n\n\n\n");
    // fgets keeps the newline; only a *trailing* one is stripped in main.
    assert_same(b"0\r\n0\r\nyyy\r\n");
    assert_same(b"0\n0\nyyy\r\n");
}

#[test]
fn extra_input_after_the_third_line_is_ignored() {
    assert_same(b"0\n0\nyyy\nEXTRA\n");
    assert_same(b"3\n0\nyn\nmore\nand more\n");
}

#[test]
fn embedded_nul_bytes_truncate_the_decision_string() {
    // strlen stops at the NUL, so `len` shrinks and the tail is ignored.
    assert_same(b"2\n0\ny\x00nn\n");
    assert_same(b"2\n0\n\x00yyy\n");
    assert_same(b"0\n0\nyy\x00y\n");
    assert_same(b"3\n0\nyn\x00nnnnn\n");
    assert_same(b"\x000\n0\nyyy\n");
    assert_same(b"0\n\x000\nyyy\n");
}

#[test]
fn lines_longer_than_the_1024_byte_buffer() {
    // fgets stops after MAX_INPUT_SIZE-1 bytes and leaves the rest in the
    // stream, so a long first line becomes the operation *and* the parameter.
    let long_op = format!("{}\n0\nyny\n", "0".repeat(1030));
    assert_same(long_op.as_bytes());
    let long_param = format!("2\n{}\nyny\n", "1".repeat(1030));
    assert_same(long_param.as_bytes());

    for n in [1020usize, 1021, 1022, 1023, 1024, 1025, 1026, 2048] {
        for op in ["2", "3"] {
            assert_same(&input(op, "0", &"y".repeat(n)));
            let alternating = "yn".repeat(n / 2 + 1);
            assert_same(&input(op, "0", &alternating[..n]));
            // No trailing newline at all.
            assert_same(format!("{op}\n0\n{}", "y".repeat(n)).as_bytes());
        }
    }
}

// ---------------------------------------------------------------------------
// main.c: atoi / strtol behaviour on the operation and parameter lines.
// ---------------------------------------------------------------------------

#[test]
fn atoi_parsing_of_operation_and_parameter() {
    let odd = [
        "", " ", "\t", "  ", "-", "+", "--1", "+-1", "- 1", "abc", "12abc",
        "abc12", "  \t 42xyz", "0x1f", "0X1F", "010", "007", "3.99", ".5",
        "1e3", "1 2", "\u{b}3", "\u{c}4", "  -0  ", "-0", "+0", "+3",
        "000000000000000000003", "y", "n",
    ];
    for a in odd {
        for b in ["0", "1", "2", "3"] {
            assert_same(&input(a, b, "yny"));
            assert_same(&input(b, a, "yny"));
        }
    }
}

#[test]
fn atoi_saturates_like_strtol_then_truncates_to_int() {
    // glibc's atoi is `(int) strtol(...)`: it saturates at long bounds and
    // then truncates, so these must not be parsed as arbitrary precision.
    let boundary = [
        "2147483647",
        "2147483648",
        "2147483649",
        "-2147483648",
        "-2147483649",
        "4294967295",
        "4294967296",
        "4294967297",
        "4294967298",
        "4294967299",
        "9223372036854775806",
        "9223372036854775807",
        "9223372036854775808",
        "9223372036854775809",
        "-9223372036854775807",
        "-9223372036854775808",
        "-9223372036854775809",
        "18446744073709551615",
        "18446744073709551616",
        "18446744073709551619",
        "99999999999999999999999999999",
        "-99999999999999999999999999999",
        "1111111111111111111111111111111111111111",
    ];
    for a in boundary {
        for b in boundary {
            assert_same(&input(a, b, "ynn"));
        }
        for b in ["0", "1", "2", "3"] {
            assert_same(&input(a, b, "yny"));
            assert_same(&input(b, a, "yny"));
        }
    }
}

// ---------------------------------------------------------------------------
// lib.c: process_decisions dispatch, including the guards and the default.
// ---------------------------------------------------------------------------

#[test]
fn empty_decision_string_returns_minus_one() {
    for op in ["0", "1", "2", "3", "4", "-1"] {
        assert_same(&input(op, "0", ""));
    }
}

#[test]
fn operations_zero_and_one_require_three_decisions() {
    // length < 3 -> -2
    for op in ["0", "1"] {
        for s in ["y", "n", "Y", "N", "x", "yy", "yn", "ny", "nn", "ab"] {
            for p in ["0", "1", "2", "3"] {
                assert_same(&input(op, p, s));
            }
        }
    }
}

#[test]
fn unknown_operation_returns_minus_three() {
    for op in [
        "4", "5", "6", "-1", "-2", "-3", "99", "-99", "2147483647",
        "-2147483648", "1000000",
    ] {
        for s in ["", "y", "yyy", "ynynyn"] {
            assert_same(&input(op, "0", s));
        }
    }
}

// ---------------------------------------------------------------------------
// lib.c: apply_permissions -- all 8 rwx combinations plus the fall-through.
// ---------------------------------------------------------------------------

#[test]
fn apply_permissions_covers_every_combination() {
    for c0 in ["y", "n", "Y", "N", "x"] {
        for c1 in ["y", "n", "Y", "N", "x"] {
            for c2 in ["y", "n", "Y", "N", "x"] {
                let s = format!("{c0}{c1}{c2}");
                assert_same(&input("0", "0", &s));
                // A longer string must not change the answer: only the first
                // three characters are read.
                assert_same(&input("0", "7", &format!("{s}nnnyyy")));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// lib.c: evaluate_conditions -- every logic_op including the default.
// ---------------------------------------------------------------------------

#[test]
fn evaluate_conditions_covers_every_logic_op_and_combination() {
    let chars = ["y", "n", "Y", "N", "q"];
    let params = [
        "0", "1", "2", "3", "4", "5", "-1", "-2", "100", "2147483647",
        "-2147483648",
    ];
    for c0 in chars {
        for c1 in chars {
            for c2 in chars {
                let s = format!("{c0}{c1}{c2}");
                for p in params {
                    assert_same(&input("1", p, &s));
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// lib.c: configure_flags -- each of its return paths.
// ---------------------------------------------------------------------------

#[test]
fn configure_flags_named_return_paths() {
    // all false -> 0
    let all_false: Vec<String> = vec![
        "n".into(),
        "nn".into(),
        "nnn".into(),
        "n".repeat(32),
        "n".repeat(40),
    ];
    for s in &all_false {
        assert_same(&input("2", "0", s));
    }
    // all true -> 1000 + count (count saturates at 32)
    for n in 1..=40 {
        assert_same(&input("2", "0", &"y".repeat(n)));
    }
    // exactly one true -> 100 + index
    for n in [2usize, 3, 5, 8, 32, 33, 40] {
        for i in 0..n {
            let mut s = vec![b'n'; n];
            s[i] = b'y';
            assert_same(&input("2", "0", std::str::from_utf8(&s).unwrap()));
        }
    }
    // exactly one false -> 200 + index
    for n in [2usize, 3, 5, 8, 32, 33, 40] {
        for i in 0..n {
            let mut s = vec![b'y'; n];
            s[i] = b'n';
            assert_same(&input("2", "0", std::str::from_utf8(&s).unwrap()));
        }
    }
    // alternating -> 500 + special_count
    let alternating: Vec<String> = vec![
        "yn".into(),
        "ny".into(),
        "ynyn".into(),
        "nyny".into(),
        "ynynyn".into(),
        "nynyny".into(),
        "ynynynyn".into(),
        "yn".repeat(16),
        "ny".repeat(16),
        "yn".repeat(20),
    ];
    for s in &alternating {
        assert_same(&input("2", "0", s));
    }
    // >= 3 consecutive true -> 300 + max_consecutive
    let consecutive: Vec<String> = vec![
        "yyynn".into(),
        "nyyyn".into(),
        "nnyyy".into(),
        "yyyynn".into(),
        "yyyyynn".into(),
        "yyynyyyn".into(),
        "nyyyyyyn".into(),
        "yyynnnyyynnn".into(),
        "yyyn".repeat(8),
        "y".repeat(31) + "n" + "y",
    ];
    for s in &consecutive {
        assert_same(&input("2", "0", s));
    }
    // falls through to `return special_count`
    for s in [
        "yynny", "yynn", "nnyy", "yyn nyy", "yynnyy", "nyynn", "yynnyynn",
        "nnyynnyy", "yyxnnyy",
    ] {
        assert_same(&input("2", "0", s));
    }
    // the 32-element cap
    for n in [30usize, 31, 32, 33, 34, 64, 100] {
        assert_same(&input("2", "0", &"y".repeat(n)));
        assert_same(&input("2", "0", &("y".repeat(n - 1) + "n")));
        assert_same(&input("2", "0", &("n".repeat(n - 1) + "y")));
        assert_same(&input("2", "0", &"yn".repeat(n / 2)));
        assert_same(&input("2", "0", &"yyn".repeat(n / 3 + 1)));
    }
}

// ---------------------------------------------------------------------------
// lib.c: validate_sequence -- each rule and each length bucket.
// ---------------------------------------------------------------------------

#[test]
fn validate_sequence_rule_violations() {
    // Rule 1: must start with a true value -> -10
    let rule1: Vec<String> = vec![
        "n".into(),
        "nn".into(),
        "nyn".into(),
        "nnn".into(),
        "xyn".into(),
        "nyynn".into(),
        "n".repeat(20),
    ];
    for s in &rule1 {
        assert_same(&input("3", "0", s));
    }
    // Rule 2: must end with a false value when len > 1 -> -11
    let rule2: Vec<String> = vec![
        "yy".into(),
        "yny".into(),
        "ynyny".into(),
        "yyny".into(),
        "yn".repeat(10) + "y",
    ];
    for s in &rule2 {
        assert_same(&input("3", "0", s));
    }
    // Rule 3: more than 3 consecutive equal values -> -12
    let rule3: Vec<String> = vec![
        "yyyyn".into(),
        "yyyyyn".into(),
        "ynnnnn".into(),
        "ynnnn".into(),
        "yynnnny".into(),
        "yyyynnnn".into(),
        "y".repeat(10) + "n",
        "yn".to_owned() + &"n".repeat(10),
    ];
    for s in &rule3 {
        assert_same(&input("3", "0", s));
    }
}

#[test]
fn validate_sequence_length_buckets() {
    // len == 1
    for s in ["y", "Y", "n", "N", "x"] {
        assert_same(&input("3", "0", s));
    }
    // len <= 3
    for a in ["y", "n"] {
        for b in ["y", "n"] {
            for c in ["y", "n"] {
                assert_same(&input("3", "0", &format!("{a}{b}")));
                assert_same(&input("3", "0", &format!("{a}{b}{c}")));
            }
        }
    }
    // 4 <= len <= 10, all three sub-branches
    for s in [
        "ynnn", "yynn", "ynyn", "yyynnn", "ynynyn", "yynnyn", "ynnyyn",
        "yynnyynn", "ynynynyn", "yyynnnyn", "ynnynnyn", "yynyynnn",
        "ynyynnyn", "yynnynyn", "ynynnyyn",
        // Non-ASCII bytes: multi-byte UTF-8 makes `len` larger than the
        // character count and every byte parses as false.
        "yy\u{043d}\u{043d}n",
    ] {
        assert_same(&input("3", "0", s));
    }
    // len > 10
    for s in [
        "ynynynynynn",
        "yynnyynnyynn",
        "ynnyynnyynnyn",
        "yyynnnyyynnnyyynnn",
        "yynnyynnyynnyynn",
        "ynyn ynynynyn",
        "ynynynynynynynyn",
        "ynynynynynynynynn",
        "yyn yyn yyn yyn n",
    ] {
        assert_same(&input("3", "0", s));
    }
    // long alternating sequences of many lengths, both parities
    for n in 4..=64 {
        let alt: String = (0..n)
            .map(|i| if i % 2 == 0 { 'y' } else { 'n' })
            .collect();
        assert_same(&input("3", "0", &alt));
        let mut trailing = alt.clone();
        trailing.push('n');
        assert_same(&input("3", "0", &trailing));
    }
    // repeating blocks that stay within the 3-in-a-row limit
    for pat in ["yyn", "ynn", "yynn", "yyynnn", "yn"] {
        for reps in 1..=12 {
            assert_same(&input("3", "0", &pat.repeat(reps)));
        }
    }
}

#[test]
fn validate_sequence_param_is_ignored() {
    for p in ["0", "1", "2", "3", "-5", "99999", "abc"] {
        assert_same(&input("3", p, "ynynyn"));
        assert_same(&input("3", p, "yyn"));
    }
}

// ---------------------------------------------------------------------------
// Broad sweeps: exhaustive short strings, then deterministic pseudo-random
// byte soup, to catch anything the enumerated cases missed.
// ---------------------------------------------------------------------------

/// Every y/n string up to `max_len`, against one operation and all params.
fn exhaustive_for_operation(op: &str, max_len: u32) {
    for len in 0..=max_len {
        for mask in 0u32..(1u32 << len) {
            let s: String = (0..len)
                .map(|i| if mask >> i & 1 == 1 { 'y' } else { 'n' })
                .collect();
            for p in ["0", "1", "2", "3"] {
                assert_same(&input(op, p, &s));
            }
        }
    }
}

#[test]
fn exhaustive_short_strings_operation_0() {
    exhaustive_for_operation("0", 8);
}

#[test]
fn exhaustive_short_strings_operation_1() {
    exhaustive_for_operation("1", 8);
}

#[test]
fn exhaustive_short_strings_operation_2() {
    exhaustive_for_operation("2", 9);
}

#[test]
fn exhaustive_short_strings_operation_3() {
    exhaustive_for_operation("3", 9);
}

/// Small deterministic PRNG so the sweep is reproducible run to run.
struct Rng(u64);

impl Rng {
    fn next_u32(&mut self) -> u32 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 32) as u32
    }

    fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
}

#[test]
fn random_decision_strings_over_all_operations() {
    let mut rng = Rng(0x1234_5678_9abc_def1);
    let alphabet = b"ynYNx \t01?";
    let ops = ["0", "1", "2", "3", "4", "-1", "7"];
    for _ in 0..1500 {
        let n = rng.below(48) as usize;
        let s: Vec<u8> = (0..n)
            .map(|_| alphabet[rng.below(alphabet.len() as u32) as usize])
            .collect();
        let op = ops[rng.below(ops.len() as u32) as usize];
        let p = (rng.below(11) as i32) - 4;
        let mut stdin = format!("{op}\n{p}\n").into_bytes();
        stdin.extend_from_slice(&s);
        stdin.push(b'\n');
        assert_same(&stdin);
    }
}

#[test]
fn random_raw_byte_streams() {
    // Whole-stream garbage: exercises the fgets/atoi/EOF paths together,
    // including inputs with no newlines and with non-ASCII bytes.
    let mut rng = Rng(0x0bad_c0de_dead_beef);
    for _ in 0..1200 {
        let n = rng.below(64) as usize;
        let stdin: Vec<u8> = (0..n).map(|_| rng.below(256) as u8).collect();
        assert_same(&stdin);
    }
}

#[test]
fn random_high_byte_decision_strings() {
    // parse_bool defaults to false for anything that is not y/Y/n/N; make
    // sure signed-char handling of bytes >= 0x80 matches.
    let mut rng = Rng(0xfeed_face_1234_5678);
    for _ in 0..600 {
        let n = rng.below(40) as usize;
        let s: Vec<u8> = (0..n)
            .map(|_| {
                let b = rng.below(256) as u8;
                if b == b'\n' || b == 0 {
                    0x80
                } else {
                    b
                }
            })
            .collect();
        let op = rng.below(5) as i32;
        let p = rng.below(5) as i32;
        let mut stdin = format!("{op}\n{p}\n").into_bytes();
        stdin.extend_from_slice(&s);
        stdin.push(b'\n');
        assert_same(&stdin);
    }
}
