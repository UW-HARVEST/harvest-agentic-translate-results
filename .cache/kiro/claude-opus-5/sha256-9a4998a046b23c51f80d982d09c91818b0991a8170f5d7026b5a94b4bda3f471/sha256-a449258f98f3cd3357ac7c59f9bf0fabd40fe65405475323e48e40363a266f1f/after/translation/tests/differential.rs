//! Differential tests: run the C binary and the Rust binary as subprocesses on
//! the same input and compare stdout, stderr and exit status byte for byte.
//!
//! The Rust code is never called as a library -- both programs are driven the
//! way a shell would drive them, because that is how they are compared.
//!
//! The C binary is expected at `c_src/build/driver`. Build it with:
//! ```text
//! cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .
//! ```

use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// Path to the C reference binary.
fn c_bin() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.push("c_src/build/driver");
    assert!(
        p.is_file(),
        "C reference binary missing at {}. Build it with:\n  \
         cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .",
        p.display()
    );
    p
}

/// Path to the Rust binary under test (built by cargo for this test).
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    code: Option<i32>,
    signal: Option<i32>,
}

fn run(exe: &PathBuf, args: &[&str], stdin_data: &[u8]) -> Outcome {
    let mut child = Command::new(exe)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));

    {
        let mut si = child.stdin.take().unwrap();
        // The child may exit without draining stdin; a broken pipe is fine.
        let _ = si.write_all(stdin_data);
        let _ = si.flush();
    }

    let out = child.wait_with_output().expect("wait failed");

    // Both programs print their own argv[0] in the usage text, so the differing
    // binary paths are normalized away. This is the only normalization applied.
    let exe_bytes = exe.as_os_str().as_encoded_bytes().to_vec();
    let scrub = |mut v: Vec<u8>| -> Vec<u8> {
        if let Some(pos) = find(&v, &exe_bytes) {
            let mut out = v[..pos].to_vec();
            out.extend_from_slice(b"<PROG>");
            out.extend_from_slice(&v[pos + exe_bytes.len()..]);
            v = out;
        }
        v
    };

    Outcome {
        stdout: scrub(out.stdout),
        stderr: scrub(out.stderr),
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || hay.len() < needle.len() {
        return None;
    }
    (0..=hay.len() - needle.len()).find(|&i| &hay[i..i + needle.len()] == needle)
}

#[track_caller]
fn assert_same(args: &[&str], stdin_data: &[u8]) {
    let c = run(&c_bin(), args, stdin_data);
    let r = run(&rust_bin(), args, stdin_data);

    if c != r {
        panic!(
            "MISMATCH\n  args:   {:?}\n  stdin:  {:?}\n\
             \n  C  code={:?} signal={:?}\n  C  stdout: {}\n  C  stderr: {}\
             \n  R  code={:?} signal={:?}\n  R  stdout: {}\n  R  stderr: {}",
            args,
            String::from_utf8_lossy(&stdin_data[..stdin_data.len().min(200)]),
            c.code,
            c.signal,
            show(&c.stdout),
            show(&c.stderr),
            r.code,
            r.signal,
            show(&r.stdout),
            show(&r.stderr),
        );
    }
}

fn show(b: &[u8]) -> String {
    let head = &b[..b.len().min(400)];
    let mut s = String::from_utf8_lossy(head).into_owned();
    if b.len() > head.len() {
        s.push_str(&format!("... ({} bytes total)", b.len()));
    }
    format!("{s:?}")
}

// ---------------------------------------------------------------------------
// main.c: argument handling
// ---------------------------------------------------------------------------

#[test]
fn no_arguments_is_no_program() {
    // code.len == 0 -> fprintf(stderr, "no program\n"); return 2;
    assert_same(&[], b"");
}

#[test]
fn help_flag() {
    // usage() to stderr, return 0. Also checked after a skipped arg and after
    // bytecodes, since the flag is handled mid-loop and returns immediately.
    assert_same(&["--help"], b"");
    assert_same(&["--help", "0", "5"], b"");
    assert_same(&["0", "5", "--help"], b"");
    assert_same(&["zzz", "--help"], b"");
    assert_same(&["--help", "--help"], b"");
    assert_same(&["--stdin", "--help"], b"1 2 3");
}

#[test]
fn unparsable_args_are_skipped() {
    // strtol leaves *endptr != '\0'  ->  fprintf(stderr, "skip '%s'\n", argv[i])
    for a in [
        "abc", "0x10", "12abc", "1e5", "-", "+", "--", " ", "1 2", "1\t2", "5.5", "1,2", "０",
        "0b101", "\n", "\t", "1-", "++5", "- 5", "0x", "INF", "nan",
    ] {
        assert_same(&[a], b"");
        // ... and mixed with a valid program so the run actually proceeds.
        assert_same(&["0", "7", a, "5"], b"");
    }
}

#[test]
fn empty_arg_parses_as_zero() {
    // strtol("") performs no conversion, so endptr == nptr and *endptr == '\0'.
    // The check `e && *e=='\0'` therefore passes and pushes 0.
    assert_same(&[""], b"");
    assert_same(&["", ""], b"");
    assert_same(&["", "5"], b"");
}

#[test]
fn strtol_accepted_forms() {
    // Leading whitespace and an explicit sign are consumed by strtol.
    for a in [
        "  7", "\t7", "\n7", "+5", "-5", "007", "+0", "-0", "  -3", "\r\n\t 42",
    ] {
        assert_same(&[a], b"");
    }
}

#[test]
fn integer_truncation_and_overflow() {
    // (int)long truncation, and strtol's ERANGE clamp to LONG_MAX / LONG_MIN
    // before that truncation.
    for a in [
        "2147483647",
        "2147483648",
        "-2147483648",
        "-2147483649",
        "4294967296",
        "4294967297",
        "99999999999999999999",
        "-99999999999999999999",
        "9223372036854775807",
        "9223372036854775808",
        "-9223372036854775808",
        "-9223372036854775809",
        "000000000000000000005",
    ] {
        assert_same(&[a], b"");
        assert_same(&["0", a, "5", "8"], b"");
    }
}

// ---------------------------------------------------------------------------
// main.c: read_stdin
// ---------------------------------------------------------------------------

#[test]
fn stdin_empty_and_whitespace() {
    assert_same(&["--stdin"], b"");
    assert_same(&["--stdin"], b"\n");
    assert_same(&["--stdin"], b"   \t\r\n  ");
    assert_same(&["--stdin"], b"\n\n\n");
}

#[test]
fn stdin_basic_tokens() {
    assert_same(&["--stdin"], b"0 5");
    assert_same(&["--stdin"], b"0 5\n");
    assert_same(&["--stdin"], b"0\n5\n");
    assert_same(&["--stdin"], b"0\t5\r\n1\r");
    assert_same(&["--stdin"], b"  0   5   \n\n  1 2  ");
    assert_same(&["--stdin"], b"0 5 no 3");
    // No trailing newline on the final line.
    assert_same(&["--stdin"], b"3 3 1");
}

#[test]
fn stdin_combines_with_argv() {
    // argv bytecodes come first, then stdin appends to the same IntVec.
    assert_same(&["0", "9", "--stdin"], b"1 2");
    assert_same(&["--stdin", "0", "9"], b"1 2");
    assert_same(&["--stdin", "--stdin"], b"3 5");
}

#[test]
fn stdin_unparsable_tokens_are_dropped_silently() {
    // read_stdin has no "skip" message -- tokens that do not fully convert are
    // simply not pushed.
    assert_same(&["--stdin"], b"0 abc 5");
    assert_same(&["--stdin"], b"0x10 5");
    assert_same(&["--stdin"], b"abc def");
    assert_same(&["--stdin"], b"1.5 2.5");
}

#[test]
fn stdin_embedded_nul_truncates_the_chunk() {
    // read_stdin walks buf as a C string, so a NUL ends that chunk early.
    assert_same(&["--stdin"], b"3 3\0 1 2 3");
    assert_same(&["--stdin"], b"\0 5 5");
    assert_same(&["--stdin"], b"0 5\0");
    // A NUL before the newline hides the rest of that line only.
    assert_same(&["--stdin"], b"3\0hidden\n3 1\n");
}

#[test]
fn stdin_fgets_chunk_boundary_splits_numbers() {
    // fgets reads at most 4095 bytes, so a long line is processed in chunks and
    // a number straddling the boundary is split into two tokens.
    let mut data = Vec::new();
    for _ in 0..1000 {
        data.extend_from_slice(b"3 ");
    }
    // Land a multi-digit number across the 4095-byte boundary.
    while data.len() < 4090 {
        data.push(b'1');
        data.push(b' ');
    }
    data.extend_from_slice(b"1234567890 3 3\n");
    assert_same(&["--stdin"], &data);

    // A single token longer than the buffer.
    let mut big = vec![b'7'; 5000];
    big.push(b'\n');
    assert_same(&["--stdin"], &big);

    // Exactly at the boundary, with no newline at all.
    for pad in [4093usize, 4094, 4095, 4096, 4097] {
        let mut d = vec![b'3'; 1];
        d.push(b' ');
        while d.len() < pad {
            d.push(b'1');
            d.push(b' ');
        }
        d.truncate(pad);
        d.extend_from_slice(b"999 3");
        assert_same(&["--stdin"], &d);
    }
}

// ---------------------------------------------------------------------------
// engine.c: one test per opcode and per early return
// ---------------------------------------------------------------------------

#[test]
fn opcode_push_and_missing_immediate() {
    assert_same(&["0", "42"], b""); // PUSH 42
    assert_same(&["0"], b""); // missing imm -> rc 1
    assert_same(&["0", "0"], b"");
    assert_same(&["0", "-1", "0", "-2147483648"], b"");
}

#[test]
fn opcode_add_and_underflow() {
    assert_same(&["1"], b""); // empty stack -> rc 2
    assert_same(&["0", "5", "1"], b""); // one operand -> rc 2
    assert_same(&["0", "5", "0", "7", "1"], b"");
    // Wrapping addition.
    assert_same(&["0", "2147483647", "0", "1", "1"], b"");
    assert_same(&["0", "-2147483648", "0", "-1", "1"], b"");
}

#[test]
fn opcode_mul_and_underflow() {
    assert_same(&["2"], b""); // rc 3
    assert_same(&["0", "5", "2"], b""); // rc 3
    assert_same(&["0", "6", "0", "7", "2"], b"");
    assert_same(&["0", "65536", "0", "65536", "2"], b""); // wraps
    assert_same(&["0", "-2147483648", "0", "-1", "2"], b"");
}

#[test]
fn opcode_dup_uses_peek_default() {
    assert_same(&["3"], b""); // empty stack: peek returns 0, pushes 0
    assert_same(&["3", "3", "3"], b"");
    assert_same(&["0", "9", "3", "1"], b"");
}

#[test]
fn opcode_drop_and_underflow() {
    assert_same(&["4"], b""); // rc 4
    assert_same(&["0", "1", "4"], b"");
    assert_same(&["0", "1", "4", "4"], b""); // second drop -> rc 4
}

#[test]
fn opcode_classify_all_trace_buckets() {
    // case 5 maps the classify result to trace 5/6/7/8/9, with `case 3:`
    // falling through into `case 4:`. Sweep operands to reach every bucket in
    // all three implementations.
    for v in [
        -1, -2, -7, -10, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 19, 20,
        23, 27, 29, 31, 63, 64, 100, 127, 255, 1000, 12345, 2147483647, -2147483648,
    ] {
        assert_same(&["0", &v.to_string(), "5"], b"");
        assert_same(&["0", &v.to_string(), "8"], b"");
        // Repeated classify exercises the mutable state in a.c / b.c.
        assert_same(&["0", &v.to_string(), "5", "5", "5", "8", "8"], b"");
    }
    // classify on an empty stack (peek default 0).
    assert_same(&["5"], b"");
    assert_same(&["8"], b"");
    assert_same(&["5", "5", "5", "5", "5", "5", "5", "5"], b"");
    assert_same(&["8", "8", "8", "8", "8", "8", "8", "8"], b"");
}

#[test]
fn opcode_jmpif_all_paths() {
    assert_same(&["6"], b""); // missing k -> rc 5
    assert_same(&["6", "1"], b""); // pop cond fails -> rc 6
    // cond == 0 -> trace 11, no jump
    assert_same(&["0", "0", "6", "1", "3"], b"");
    // cond != 0, k within range -> trace 10
    assert_same(&["0", "1", "6", "0", "3", "3"], b"");
    assert_same(&["0", "1", "6", "1", "3", "3"], b"");
    // cond != 0, k just past the end -> rc 7
    assert_same(&["0", "1", "6", "99"], b"");
    assert_same(&["0", "1", "6", "2", "3"], b"");
    // negative k sign-extends to a huge size_t -> rc 7
    assert_same(&["0", "1", "6", "-1", "3", "3"], b"");
    assert_same(&["0", "1", "6", "-2147483648", "3"], b"");
    // k exactly equal to the remaining length is allowed.
    assert_same(&["0", "1", "6", "1", "3"], b"");
    assert_same(&["0", "1", "6", "0"], b"");
}

#[test]
fn opcode_repeat_all_paths() {
    assert_same(&["7"], b""); // missing times -> rc 8
    assert_same(&["7", "3"], b""); // ip >= n -> rc 9
    // times <= 0 runs the body zero times.
    assert_same(&["7", "0", "3"], b"");
    assert_same(&["7", "-1", "3"], b"");
    assert_same(&["7", "-2147483648", "3"], b"");
    // Normal repeats of a one-instruction body.
    assert_same(&["7", "1", "3"], b"");
    assert_same(&["7", "5", "3"], b"");
    assert_same(&["7", "5", "3", "1", "1"], b"");
    // Body that fails on the first iteration -> trace 12 then break.
    assert_same(&["7", "5", "0"], b""); // inner PUSH lacks its immediate
    assert_same(&["7", "5", "1"], b""); // inner ADD underflows
    assert_same(&["7", "5", "4"], b""); // inner DROP underflows
    assert_same(&["7", "5", "6"], b""); // inner JMPIF lacks k
    assert_same(&["7", "5", "7"], b""); // inner REPEAT lacks times
    assert_same(&["7", "5", "9"], b""); // inner REDUCE lacks m
    assert_same(&["7", "5", "99"], b""); // inner unknown opcode
    assert_same(&["7", "5", "10"], b""); // inner HALT returns 0, so it repeats
    // Body that succeeds a few times then fails.
    assert_same(&["0", "1", "0", "2", "7", "3", "1"], b"");
    // After the loop, ip is saved_ip + 1 regardless of what happened.
    assert_same(&["7", "2", "3", "99"], b"");
    assert_same(&["7", "2", "3", "10", "3"], b"");
}

#[test]
fn opcode_reduce_all_paths() {
    assert_same(&["9"], b""); // missing m -> rc 10
    assert_same(&["9", "-1"], b""); // m < 0 -> rc 11
    assert_same(&["9", "1"], b""); // m > stack.len -> rc 11
    assert_same(&["9", "0"], b""); // m == 0: empty stream
    assert_same(&["0", "5", "9", "2"], b""); // rc 11
    // The duplicated pop loop drains up to m extra values; where iv_pop fails
    // it leaves tmp[i] holding the value from the first loop.
    assert_same(&["0", "5", "9", "1"], b"");
    assert_same(&["0", "1", "0", "2", "9", "2"], b"");
    assert_same(&["0", "1", "0", "2", "0", "3", "0", "4", "9", "2"], b"");
    assert_same(&["0", "1", "0", "2", "0", "3", "0", "4", "9", "4"], b"");
    // Longer streams to exercise process_a_stream / process_b_stream loops.
    for m in 1..=6 {
        let mut args: Vec<String> = Vec::new();
        for v in 0..8 {
            args.push("0".into());
            args.push(v.to_string());
        }
        args.push("9".into());
        args.push(m.to_string());
        let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        assert_same(&refs, b"");
    }
    // Negative values in the reduced stream reach the `code < 0` early returns.
    assert_same(&["0", "-1", "0", "-2", "0", "-3", "9", "3"], b"");
    assert_same(&["0", "-2147483648", "9", "1"], b"");
}

#[test]
fn opcode_halt_and_unknown() {
    assert_same(&["10"], b""); // rc 0, stops immediately
    assert_same(&["10", "3", "3"], b""); // trailing code never runs
    assert_same(&["0", "5", "10", "1"], b"");
    for bad in ["11", "12", "99", "-1", "-2", "2147483647", "-2147483648", "255"] {
        assert_same(&[bad], b""); // rc 99
        assert_same(&["3", bad], b"");
    }
}

// ---------------------------------------------------------------------------
// Cross-cutting: the a.c / b.c static state persists across the whole process
// ---------------------------------------------------------------------------

#[test]
fn static_state_persists_across_the_three_engines() {
    // state_a and flipflop are file-static and never reset, so engine A's run
    // leaves state that engine B and later classify calls observe. Long
    // classify chains make any divergence visible.
    let mut args: Vec<String> = Vec::new();
    for i in 0..40 {
        args.push("0".into());
        args.push(i.to_string());
        args.push("5".into());
        args.push("8".into());
    }
    let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    assert_same(&refs, b"");

    // Interleave classify with reduce so both the per-call and per-stream
    // state updates are exercised in order.
    let mut args2: Vec<String> = Vec::new();
    for i in 0..12 {
        args2.push("0".into());
        args2.push((i * 7 - 3).to_string());
        args2.push("8".into());
        args2.push("9".into());
        args2.push("2".into());
        args2.push("5".into());
    }
    let refs2: Vec<&str> = args2.iter().map(|s| s.as_str()).collect();
    assert_same(&refs2, b"");
}

#[test]
fn trace_letter_masking() {
    // vm_print indexes the alphabet with `trace & 25` (not `% 26`), so several
    // distinct trace values collapse onto the same letter. Drive a program that
    // emits many different trace values.
    assert_same(
        &[
            "0", "1", "3", "1", "0", "2", "2", "5", "8", "3", "4", "0", "1", "6", "0", "9", "1",
        ],
        b"",
    );
}

// ---------------------------------------------------------------------------
// Resource-exhaustion boundary
// ---------------------------------------------------------------------------

#[test]
fn reduce_vla_within_stack_limit() {
    // `int tmp[m]` fits, so the program completes normally.
    assert_same(&["7", "1000000", "3", "9", "1000000"], b"");
}

#[test]
fn reduce_vla_overflows_stack() {
    // `int tmp[m]` overshoots RLIMIT_STACK: C dies with SIGSEGV and an empty
    // stderr, and the translation must do the same rather than aborting with
    // Rust's "has overflowed its stack" message.
    assert_same(&["7", "3000000", "3", "9", "3000000"], b"");
    assert_same(&["7", "8000000", "3", "9", "8000000"], b"");
}

// ---------------------------------------------------------------------------
// Broad sweeps
// ---------------------------------------------------------------------------

#[test]
fn exhaustive_single_opcode() {
    for a in -5i64..=20 {
        assert_same(&[&a.to_string()], b"");
    }
}

#[test]
fn exhaustive_two_opcode_programs() {
    let vals: Vec<i64> = (-3..=14).chain([99, -100, 255]).collect();
    for &a in &vals {
        for &b in &vals {
            assert_same(&[&a.to_string(), &b.to_string()], b"");
        }
    }
}

#[test]
fn exhaustive_three_opcode_programs() {
    let vals: Vec<i64> = (-1..=11).collect();
    for &a in &vals {
        for &b in &vals {
            for &c in &vals {
                assert_same(&[&a.to_string(), &b.to_string(), &c.to_string()], b"");
            }
        }
    }
}

/// Deterministic LCG so the randomized sweep is reproducible without adding a
/// dependency.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[(self.next() as usize) % xs.len()]
    }
}

#[test]
fn randomized_programs() {
    // Opcode-weighted pool: mostly valid opcodes, plus operands and edge values.
    let pool: Vec<i64> = vec![
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 0, 1, 2, 3, 5, 6, 7, 8, 9, 11, 99, -1, -2, -5, 12, 13,
        14, 20, 255, 1000, 2147483647, -2147483648, 4, 4, 6, 6, 7, 7, 9, 9,
    ];
    let mut rng = Rng(0x9E3779B97F4A7C15);
    for _ in 0..1500 {
        let n = 1 + (rng.next() as usize) % 14;
        let args: Vec<String> = (0..n).map(|_| rng.pick(&pool).to_string()).collect();
        let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        assert_same(&refs, b"");
    }
}

#[test]
fn randomized_stdin_programs() {
    let toks: &[&str] = &[
        "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "99", "-1", "-3", "12", "abc",
        "0x1", "", "2147483648", "7", "9", "3",
    ];
    let seps: &[&str] = &[" ", "\n", "\t", "\r\n", "  ", " \t "];
    let mut rng = Rng(0xDEADBEEFCAFEBABE);
    for _ in 0..600 {
        let n = 1 + (rng.next() as usize) % 16;
        let mut data = String::new();
        for _ in 0..n {
            data.push_str(rng.pick(toks));
            data.push_str(rng.pick(seps));
        }
        assert_same(&["--stdin"], data.as_bytes());
    }
}
