//! Differential integration tests: run the C `driver` and the Rust `driver` as
//! subprocesses over the same inputs and require byte-identical stdout, stderr
//! and identical exit status.
//!
//! The Rust code is never linked as a library here -- both programs are driven
//! exactly the way a shell would drive them.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

// ---------------------------------------------------------------------------
// Locating and building the two binaries
// ---------------------------------------------------------------------------

/// The workspace root: the directory holding `c_src/` and `translation/`.
fn root() -> PathBuf {
    // CARGO_MANIFEST_DIR is <root>/translation
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// Path to the C executable, building it via CMake on first use.
fn c_binary() -> PathBuf {
    let c_src = root().join("c_src");
    let build = c_src.join("build");
    let exe = build.join("driver");
    if exe.exists() {
        return exe;
    }
    std::fs::create_dir_all(&build).expect("create c_src/build");
    let cfg = Command::new("cmake")
        .arg("..")
        .current_dir(&build)
        .output()
        .expect("cmake must be installed to run the differential tests");
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
        .expect("cmake --build");
    assert!(
        bld.status.success(),
        "cmake build failed:\n{}\n{}",
        String::from_utf8_lossy(&bld.stdout),
        String::from_utf8_lossy(&bld.stderr)
    );
    assert!(exe.exists(), "C driver was not produced at {:?}", exe);
    exe
}

/// Path to the Rust executable under test. Cargo builds the binary before
/// running integration tests, so it is already on disk next to the test exe.
fn rust_binary() -> PathBuf {
    // Test executable lives in target/<profile>/deps/<name>-<hash>
    let mut dir = std::env::current_exe().expect("current_exe");
    dir.pop(); // deps
    if dir.ends_with("deps") {
        dir.pop();
    }
    let exe = dir.join("driver");
    assert!(
        exe.exists(),
        "Rust driver binary not found at {:?}; run `cargo build` first",
        exe
    );
    exe
}

// ---------------------------------------------------------------------------
// Running a program
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Some(code)` for a normal exit, `None` if killed by a signal.
    code: Option<i32>,
}

fn show(bytes: &[u8]) -> String {
    // Printable rendering that still distinguishes trailing newlines and NULs.
    let mut s = String::new();
    for &b in bytes {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{:02x}", b)),
        }
    }
    s
}

fn run(exe: &Path, args: &[&str], stdin_bytes: &[u8]) -> Outcome {
    let mut child = Command::new(exe)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {:?}: {e}", exe));
    {
        let mut si = child.stdin.take().expect("stdin pipe");
        // The child may exit without reading stdin (e.g. no `--stdin`); a
        // broken pipe here is expected and must not fail the test.
        let _ = si.write_all(stdin_bytes);
        let _ = si.flush();
    }
    let out = child.wait_with_output().expect("wait_with_output");
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
    }
}

/// The one unavoidable difference between the two programs: `usage()` prints
/// `argv[0]`, which is the path of whichever binary ran. Normalize it away so
/// the rest of the `--help` output is still compared byte for byte.
fn normalize_argv0(bytes: &[u8], exe: &Path) -> Vec<u8> {
    let needle = exe.as_os_str().to_string_lossy().into_owned().into_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i..].starts_with(&needle) {
            out.extend_from_slice(b"$PROG");
            i += needle.len();
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    out
}

/// Core assertion: identical stdout, identical stderr, identical exit status.
fn assert_same(args: &[&str], stdin_bytes: &[u8]) {
    let c = c_binary();
    let r = rust_binary();
    let co = run(&c, args, stdin_bytes);
    let ro = run(&r, args, stdin_bytes);

    let c_err = normalize_argv0(&co.stderr, &c);
    let r_err = normalize_argv0(&ro.stderr, &r);

    let ctx = format!(
        "args={:?} stdin={:?}",
        args,
        show(&stdin_bytes[..stdin_bytes.len().min(120)])
    );

    assert_eq!(
        show(&co.stdout),
        show(&ro.stdout),
        "stdout mismatch for {ctx}"
    );
    assert_eq!(co.stdout, ro.stdout, "stdout byte mismatch for {ctx}");
    assert_eq!(show(&c_err), show(&r_err), "stderr mismatch for {ctx}");
    assert_eq!(c_err, r_err, "stderr byte mismatch for {ctx}");
    assert_eq!(
        co.code, ro.code,
        "exit status mismatch for {ctx} (C={:?} Rust={:?})",
        co.code, ro.code
    );
}

fn assert_same_args(args: &[&str]) {
    assert_same(args, b"");
}

/// Same comparison, but with raw (possibly non-UTF-8) argument bytes.
fn assert_same_raw_args(args: &[&[u8]]) {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let c = c_binary();
    let r = rust_binary();
    let osargs: Vec<&OsStr> = args.iter().map(|a| OsStr::from_bytes(a)).collect();

    let go = |exe: &Path| -> Outcome {
        let out = Command::new(exe)
            .args(&osargs)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("spawn");
        Outcome {
            stdout: out.stdout,
            stderr: out.stderr,
            code: out.status.code(),
        }
    };
    let co = go(&c);
    let ro = go(&r);
    let ctx = format!("raw args={:?}", args);
    assert_eq!(show(&co.stdout), show(&ro.stdout), "stdout mismatch {ctx}");
    assert_eq!(
        show(&normalize_argv0(&co.stderr, &c)),
        show(&normalize_argv0(&ro.stderr, &r)),
        "stderr mismatch {ctx}"
    );
    assert_eq!(co.code, ro.code, "exit status mismatch {ctx}");
}

// ---------------------------------------------------------------------------
// Phase A -- both programs exist and run
// ---------------------------------------------------------------------------

#[test]
fn phase_a_both_binaries_build_and_run() {
    let c = c_binary();
    let r = rust_binary();
    assert!(c.exists(), "C binary missing: {c:?}");
    assert!(r.exists(), "Rust binary missing: {r:?}");
    // Smallest program that does something: PUSH 5.
    let co = run(&c, &["0", "5"], b"");
    let ro = run(&r, &["0", "5"], b"");
    assert_eq!(co.code, Some(0));
    assert_eq!(ro.code, Some(0));
    assert!(!co.stdout.is_empty());
    assert_eq!(co.stdout, ro.stdout);
}

// ---------------------------------------------------------------------------
// Phase B -- argument handling and the main.c error paths
// ---------------------------------------------------------------------------

#[test]
fn argv_no_arguments_is_no_program_rc2() {
    // main.c: `if (code.len==0) { fprintf(stderr,"no program\n"); return 2; }`
    assert_same_args(&[]);
}

#[test]
fn argv_help_prints_usage_and_returns_0() {
    // main.c: `--help` -> usage() on stderr, return 0, remaining args ignored.
    assert_same_args(&["--help"]);
    assert_same_args(&["--help", "0", "5"]);
    assert_same_args(&["--help", "garbage"]);
    // --help wins even after a skip message has already been emitted.
    assert_same_args(&["garbage", "--help"]);
    assert_same_args(&["0", "5", "--help"]);
}

#[test]
fn argv_unparsable_tokens_are_skipped_to_stderr() {
    // main.c else-branch: `fprintf(stderr,"skip '%s'\n", argv[i]);`
    assert_same_args(&["abc"]); // skip, then "no program", rc 2
    assert_same_args(&["abc", "0", "5"]); // skip, then a valid program
    assert_same_args(&["5abc"]); // strtol stops early -> skip
    assert_same_args(&["0x10"]); // base 10: stops at 'x' -> skip
    assert_same_args(&["1e3"]);
    assert_same_args(&["-"]);
    assert_same_args(&["--"]);
    assert_same_args(&["5 "]); // trailing space -> *endptr != 0 -> skip
    assert_same_args(&["abc", "def", "ghi"]); // three skips
}

#[test]
fn argv_empty_string_parses_as_zero() {
    // strtol("") performs no conversion and sets endptr == nptr, so
    // `*e=='\0'` is true and the value 0 is pushed. An empty argv entry is
    // therefore a PUSH opcode, not a skip.
    assert_same_args(&[""]); // op 0 with no immediate -> rc 1
    assert_same_args(&["", "5"]); // PUSH 5
    assert_same_args(&["", "", ""]);
}

#[test]
fn argv_strtol_accepts_leading_space_and_sign() {
    assert_same_args(&["+5"]);
    assert_same_args(&[" 5"]);
    assert_same_args(&["\t7"]);
    assert_same_args(&["  -3  "]); // trailing spaces -> skip
    assert_same_args(&["007", "5"]);
    assert_same_args(&["-0", "5"]);
}

#[test]
fn argv_strtol_overflow_truncates_to_int() {
    // `(int)strtol(...)`: out-of-range longs clamp to LONG_MAX/LONG_MIN and are
    // then truncated to int.
    assert_same_args(&["0", "2147483648"]);
    assert_same_args(&["0", "4294967296"]);
    assert_same_args(&["0", "9223372036854775807"]);
    assert_same_args(&["0", "9223372036854775808"]);
    assert_same_args(&["0", "-9223372036854775808"]);
    assert_same_args(&["0", "-9223372036854775809"]);
    assert_same_args(&["0", "99999999999999999999999999"]);
    assert_same_args(&["2147483648"]); // truncates to 0 -> PUSH
    assert_same_args(&["4294967306"]); // truncates to 10 -> HALT
}

// ---------------------------------------------------------------------------
// Phase B -- read_stdin()
// ---------------------------------------------------------------------------

#[test]
fn stdin_empty_is_no_program() {
    assert_same(&["--stdin"], b"");
    assert_same(&["--stdin"], b"\n");
    assert_same(&["--stdin"], b"   \n");
    assert_same(&["--stdin"], b"\n\n\n");
}

#[test]
fn stdin_is_ignored_without_the_flag() {
    // read_stdin is only called when --stdin was seen.
    assert_same(&[], b"0 5\n");
    assert_same(&["0", "5"], b"9 9 9\n");
}

#[test]
fn stdin_single_and_multiple_tokens() {
    assert_same(&["--stdin"], b"5");
    assert_same(&["--stdin"], b"5\n");
    assert_same(&["--stdin"], b"0 5\n");
    assert_same(&["--stdin"], b"0 5 1 2 3\n");
    assert_same(&["--stdin"], b"0\n5\n1\n"); // one per line
}

#[test]
fn stdin_delimiters_space_tab_cr_newline() {
    // read_stdin splits on ' ', '\t', '\n', '\r' only.
    assert_same(&["--stdin"], b"0\t5\t8\n");
    assert_same(&["--stdin"], b"0\r\n5\r\n");
    assert_same(&["--stdin"], b"  0   5   \n");
    assert_same(&["--stdin"], b"\t\t0\t\t5\t\t\n");
    assert_same(&["--stdin"], b"0\r5\r8\r");
    // A vertical tab / form feed is NOT a delimiter here, but strtol does skip
    // leading whitespace, so "\x0b5" parses while "5\x0b" does not.
    assert_same(&["--stdin"], b"\x0b5\n");
    assert_same(&["--stdin"], b"5\x0b\n");
}

#[test]
fn stdin_unparsable_tokens_are_silently_dropped() {
    // Unlike argv, read_stdin prints nothing for a bad token.
    assert_same(&["--stdin"], b"abc\n");
    assert_same(&["--stdin"], b"abc 5\n");
    assert_same(&["--stdin"], b"5abc 8\n");
    assert_same(&["--stdin"], b"0 x 5\n");
    assert_same(&["--stdin"], b"-\n");
}

#[test]
fn stdin_embedded_nul_truncates_the_chunk() {
    // read_stdin walks the fgets buffer as a C string, so a NUL byte ends
    // processing of that chunk while later lines are still read.
    assert_same(&["--stdin"], b"\x00 5\n");
    assert_same(&["--stdin"], b"5 \x00 8\n");
    assert_same(&["--stdin"], b"5\n\x00\n8\n");
    assert_same(&["--stdin"], b"0\x005\n");
}

#[test]
fn stdin_fgets_4096_byte_chunking_splits_tokens() {
    // fgets reads at most 4095 bytes, so a long line is delivered in pieces and
    // a number straddling the boundary is split into two tokens.
    let mut a = b"1 ".repeat(2047); // 4094 bytes
    a.extend_from_slice(b"123456\n");
    assert_same(&["--stdin"], &a);

    let mut b = b"1 ".repeat(2046);
    b.extend_from_slice(b"12345678\n");
    assert_same(&["--stdin"], &b);

    // Long line of real opcodes, several chunks.
    let mut c = b"3 ".repeat(3000);
    c.push(b'\n');
    assert_same(&["--stdin"], &c);

    // One enormous token, no newline at all.
    assert_same(&["--stdin"], &b"9".repeat(5000));
}

#[test]
fn stdin_combines_with_argv_in_order() {
    // argv values are pushed first, during the option loop; stdin is appended
    // afterwards regardless of where --stdin appeared.
    assert_same(&["--stdin", "0", "9"], b"3 1\n");
    assert_same(&["0", "9", "--stdin"], b"3 1\n");
    assert_same(&["0", "--stdin", "9"], b"3 1\n");
    assert_same(&["--stdin", "--stdin"], b"5\n");
    assert_same(&["--stdin", "abc"], b"5\n"); // skip message + stdin program
}

// ---------------------------------------------------------------------------
// Phase B -- every opcode, and every early `return` in run_engine
// ---------------------------------------------------------------------------

#[test]
fn opcode_push_and_its_missing_immediate() {
    assert_same_args(&["0", "5"]); // ok
    assert_same_args(&["0"]); // rc 1: immediate fetch fails
    assert_same_args(&["0", "5", "0"]); // rc 1 after one good push
    assert_same_args(&["0", "0", "0", "1"]);
    assert_same_args(&["0", "-7"]);
}

#[test]
fn opcode_add_underflow_paths() {
    assert_same_args(&["1"]); // rc 2: empty stack
    assert_same_args(&["0", "5", "1"]); // rc 2: only one operand
    assert_same_args(&["0", "5", "0", "6", "1"]); // ok
}

#[test]
fn opcode_mul_underflow_paths() {
    assert_same_args(&["2"]); // rc 3: empty stack
    assert_same_args(&["0", "5", "2"]); // rc 3: only one operand
    assert_same_args(&["0", "5", "0", "6", "2"]); // ok
}

#[test]
fn opcode_dup_uses_peek_default_on_empty_stack() {
    // case 3 peeks with default 0, so DUP on an empty stack pushes 0.
    assert_same_args(&["3"]);
    assert_same_args(&["3", "3", "3"]);
    assert_same_args(&["0", "7", "3", "1"]);
}

#[test]
fn opcode_drop_underflow_path() {
    assert_same_args(&["4"]); // rc 4: empty stack
    assert_same_args(&["0", "5", "4"]); // ok
    assert_same_args(&["0", "5", "4", "4"]); // rc 4 on the second DROP
}

#[test]
fn opcode_classify_covers_every_trace_bucket() {
    // case 5 maps the classify() result to trace 5/6/7/8/9; `case 3:` falls
    // through into `case 4:`. Different immediates land in different buckets,
    // and the three engines disagree because a.c/b.c/lib.c have different
    // `target` implementations and per-process mutable state.
    for imm in [
        "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "17", "23", "-1",
        "-9",
    ] {
        assert_same_args(&["0", imm, "5"]);
        assert_same_args(&["0", imm, "8"]);
    }
    assert_same_args(&["5"]); // peek default 0
    assert_same_args(&["8"]);
    // Repeated classify: exercises the persistent state_a / flipflop statics.
    assert_same_args(&["5", "5", "5", "5", "5", "5", "5", "5"]);
    assert_same_args(&["8", "8", "8", "8", "8", "8", "8", "8"]);
    assert_same_args(&["0", "3", "5", "8", "5", "8", "5", "8"]);
}

#[test]
fn opcode_jump_all_three_outcomes() {
    assert_same_args(&["6"]); // rc 5: no k operand
    assert_same_args(&["0", "1", "6"]); // rc 5: k missing after cond push
    assert_same_args(&["6", "0"]); // rc 6: nothing to pop for cond
    assert_same_args(&["0", "0", "6", "1", "10"]); // cond == 0 -> trace 11
    assert_same_args(&["0", "1", "6", "0", "10"]); // taken, k == 0
    assert_same_args(&["0", "1", "6", "1", "10"]); // taken, skips one word
    assert_same_args(&["0", "1", "6", "2", "10"]); // k == remaining, boundary
    assert_same_args(&["0", "1", "6", "3", "10"]); // k > remaining -> rc 7
    assert_same_args(&["0", "1", "6", "-1", "10"]); // (size_t)(-1) -> rc 7
    assert_same_args(&["0", "1", "6", "-2147483648", "10"]); // rc 7
    assert_same_args(&["0", "1", "6", "1"]); // taken to the very end
    assert_same_args(&["0", "5", "6", "1", "3", "3"]); // nonzero cond
}

#[test]
fn opcode_repeat_all_outcomes() {
    assert_same_args(&["7"]); // rc 8: no `times`
    assert_same_args(&["7", "3"]); // rc 9: nothing left to repeat
    assert_same_args(&["7", "0", "3"]); // times == 0, body never runs
    assert_same_args(&["7", "-5", "3"]); // negative times, body never runs
    assert_same_args(&["7", "1", "3"]);
    assert_same_args(&["7", "4", "3"]);
    assert_same_args(&["7", "3", "0"]); // body fails (rc 1) -> trace 12
    assert_same_args(&["7", "3", "1"]); // body fails (rc 2) -> trace 12
    assert_same_args(&["7", "3", "7"]); // nested REPEAT, rc 8 -> trace 12
    assert_same_args(&["7", "3", "10"]); // body HALTs with rc 0
    assert_same_args(&["7", "3", "11"]); // body is an unknown op -> rc 99
    assert_same_args(&["7", "2", "5"]);
    assert_same_args(&["7", "2", "3", "1"]); // execution resumes after the body
    assert_same_args(&["200000", "7", "200000", "3"]);
    assert_same_args(&["7", "2147483647", "0"]); // huge count, body fails at once
}

#[test]
fn opcode_reduce_all_outcomes() {
    assert_same_args(&["9"]); // rc 10: no `m`
    assert_same_args(&["9", "-1"]); // rc 11: m < 0
    assert_same_args(&["9", "-2147483648"]); // rc 11
    assert_same_args(&["9", "1"]); // rc 11: m > stack len (0)
    assert_same_args(&["9", "0"]); // m == 0: empty stream
    assert_same_args(&["0", "5", "9", "1"]); // m == stack len, boundary
    assert_same_args(&["0", "5", "9", "2"]); // rc 11
    // The duplicated pop loop: with >= 2m values on the stack the second loop
    // overwrites tmp with the next m values down.
    assert_same_args(&["0", "1", "0", "2", "9", "1"]);
    assert_same_args(&["0", "1", "0", "2", "0", "3", "0", "4", "9", "2"]);
    assert_same_args(&["3", "3", "3", "3", "9", "2"]);
    assert_same_args(&["0", "5", "0", "6", "0", "7", "0", "8", "9", "3"]);
    assert_same_args(&["0", "5", "0", "6", "0", "7", "0", "8", "9", "4"]);
    assert_same_args(&["0", "5", "0", "6", "0", "7", "0", "8", "9", "5"]); // rc 11
    // Reduce then keep going.
    assert_same_args(&["3", "3", "3", "9", "2", "3", "1"]);
}

#[test]
fn opcode_halt_and_unknown_opcodes() {
    assert_same_args(&["10"]); // rc 0, stops immediately
    assert_same_args(&["10", "1", "2", "3"]); // trailing words never execute
    assert_same_args(&["0", "5", "10", "0", "6"]);
    assert_same_args(&["11"]); // rc 99
    assert_same_args(&["-1"]); // rc 99
    assert_same_args(&["100"]);
    assert_same_args(&["2147483647"]);
    assert_same_args(&["-2147483648"]);
    assert_same_args(&["0", "5", "11"]); // rc 99 after some work
}

#[test]
fn integer_wraparound_matches_c() {
    // ADD / MUL wrap, and classify()'s `x + 1` wraps at INT_MAX.
    assert_same_args(&["0", "2147483647", "0", "2147483647", "1"]);
    assert_same_args(&["0", "2147483647", "0", "2147483647", "2"]);
    assert_same_args(&["0", "-2147483648", "0", "-1", "2"]);
    assert_same_args(&["0", "2147483647", "5"]);
    assert_same_args(&["0", "2147483647", "8"]);
    assert_same_args(&["0", "-2147483648", "5"]);
    assert_same_args(&["0", "-2147483648", "8"]);
    assert_same_args(&["0", "1073741824", "5", "5", "5", "5"]);
    assert_same_args(&["0", "-2147483648", "3", "9", "2"]);
    assert_same_args(&["0", "2147483647", "3", "3", "3", "9", "3"]);
    // Plain signed ADD/MUL overflow (UB in C, wrapping in Rust).
    assert_same_args(&["0", "100000", "0", "100000", "2"]);
    assert_same_args(&["0", "2147483647", "0", "1", "1"]);
    assert_same_args(&["0", "2147483647", "3", "3", "3", "3", "9", "4"]);
    // a.c's `state_a ^ (code << 1)` shifts into the sign bit for code >= 2^30,
    // and the corrupted state cascades into every later A-engine classify.
    for v in [
        "1073741824",
        "1073741825",
        "1500000000",
        "2000000000",
        "2147483646",
    ] {
        assert_same_args(&["0", v, "5", "5", "5", "5", "8", "8", "8"]);
    }
    assert_same_args(&["0", "1073741823", "5", "0", "2147483647", "5"]);
}

// ---------------------------------------------------------------------------
// Phase C -- broader sweeps over the same comparison
// ---------------------------------------------------------------------------

#[test]
fn sweep_every_single_word_program() {
    // Covers each opcode dispatch arm plus the `default:` arm at length 1.
    for v in -3..=13 {
        let s = v.to_string();
        assert_same_args(&[&s]);
    }
}

#[test]
fn sweep_all_two_word_programs() {
    // 17 x 17 = 289 programs: every opcode paired with every operand value in
    // range, which reaches most operand-fetch failure paths.
    let vals: Vec<String> = (-3..=13).map(|v| v.to_string()).collect();
    for a in &vals {
        for b in &vals {
            assert_same_args(&[a, b]);
        }
    }
}

#[test]
fn sweep_all_three_word_programs_over_core_opcodes() {
    // 9^3 = 729 programs built from the opcodes with the most interesting
    // control flow, plus an out-of-range value for the `default:` arm.
    let vals = ["-1", "0", "1", "3", "5", "6", "7", "9", "10"];
    for a in vals {
        for b in vals {
            for c in vals {
                assert_same_args(&[a, b, c]);
            }
        }
    }
}

#[test]
fn sweep_deterministic_pseudorandom_programs() {
    // A small xorshift keeps this reproducible without a dev-dependency.
    let mut state: u64 = 0x2545F4914F6CDD1D;
    let mut next = |n: u64| -> u64 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state % n
    };
    let big = [
        -2147483648i64,
        2147483647,
        65536,
        1000,
        -1000,
        123456789,
        -123456789,
        7,
        100,
    ];
    for _ in 0..400 {
        let len = 1 + next(14) as usize;
        let mut words: Vec<String> = Vec::with_capacity(len);
        for _ in 0..len {
            let r = next(100);
            let v: i64 = if r < 60 {
                next(11) as i64 // an opcode
            } else if r < 85 {
                next(25) as i64 - 4
            } else {
                big[next(big.len() as u64) as usize]
            };
            words.push(v.to_string());
        }
        let refs: Vec<&str> = words.iter().map(|s| s.as_str()).collect();
        assert_same_args(&refs);
        // The same program fed through stdin must also agree.
        let mut input = words.join(" ").into_bytes();
        input.push(b'\n');
        assert_same(&["--stdin"], &input);
    }
}

// ---------------------------------------------------------------------------
// Phase C -- non-UTF-8 bytes
// ---------------------------------------------------------------------------

#[test]
fn non_utf8_arguments_and_stdin_are_handled_as_bytes() {
    // The C program treats argv and stdin as raw bytes; the Rust port must not
    // go through a lossy UTF-8 conversion.
    assert_same_raw_args(&[b"\xff\xfe", b"0", b"5"]);
    assert_same_raw_args(&[b"\x805", b"5"]);
    assert_same_raw_args(&[b"5\xff"]);
    assert_same_raw_args(&[b"--help", b"\xff"]);
    assert_same(&["--stdin"], b"\xff\xfe 5\n");
    assert_same(&["--stdin"], b"5 \x80 8\n");
    assert_same(&["--stdin"], b"\xc3\n5\n");
}

// ---------------------------------------------------------------------------
// Phase C -- signal disposition
// ---------------------------------------------------------------------------

#[test]
fn sigpipe_kills_both_programs_the_same_way() {
    // Rust's runtime ignores SIGPIPE by default, which would make the Rust
    // program exit 0 where the C program is killed by the signal. main.rs
    // restores SIG_DFL; this checks that it took effect.
    fn killed_by_signal(exe: &Path) -> bool {
        let mut child = Command::new(exe)
            // A long trace guarantees more output than a pipe buffer holds.
            .args(["7", "1000000", "3"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn");
        // Read one byte, then drop the pipe so the writer gets EPIPE/SIGPIPE.
        {
            use std::io::Read;
            let mut so = child.stdout.take().expect("stdout");
            let mut one = [0u8; 1];
            let _ = so.read(&mut one);
        }
        let st = child.wait().expect("wait");
        st.code().is_none()
    }

    let c_killed = killed_by_signal(&c_binary());
    let r_killed = killed_by_signal(&rust_binary());
    assert_eq!(
        c_killed, r_killed,
        "SIGPIPE disposition differs (C killed={c_killed}, Rust killed={r_killed})"
    );
}
