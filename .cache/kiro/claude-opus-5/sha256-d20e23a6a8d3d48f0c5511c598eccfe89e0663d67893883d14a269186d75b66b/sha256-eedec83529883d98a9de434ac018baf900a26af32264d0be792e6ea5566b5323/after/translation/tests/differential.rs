//! Differential tests: run the C `driver` and the Rust `driver` as *subprocesses*
//! and require byte-identical stdout, byte-identical stderr and an identical
//! exit status for every input.
//!
//! Nothing here loads the Rust code as a library; both programs are driven the
//! way a shell would drive them, because that is how they are compared.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Locating / building the two binaries
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// Path to the Rust binary under test. `cargo test` builds the bin target of
/// this crate before running integration tests, so it is already on disk.
fn rust_bin() -> PathBuf {
    // <root>/translation/target/<profile>/deps/<test>-<hash> -> ../../driver
    let exe = std::env::current_exe().expect("current_exe");
    let mut dir = exe.parent().expect("deps dir").to_path_buf();
    if dir.ends_with("deps") {
        dir.pop();
    }
    let p = dir.join("driver");
    assert!(
        p.is_file(),
        "Rust binary not found at {}; run `cargo build` first",
        p.display()
    );
    p
}

/// Path to the C binary, building it with CMake on first use so that
/// `cargo test` is self-contained.
fn c_bin() -> &'static PathBuf {
    static C: OnceLock<PathBuf> = OnceLock::new();
    C.get_or_init(|| {
        let c_src = workspace_root().join("c_src");
        let build = c_src.join("build");
        let bin = build.join("driver");
        if !bin.is_file() {
            std::fs::create_dir_all(&build).expect("mkdir c_src/build");
            let cfg = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("cmake must be installed to run the differential tests");
            assert!(
                cfg.status.success(),
                "cmake configure failed:\n{}",
                String::from_utf8_lossy(&cfg.stderr)
            );
            let b = Command::new("cmake")
                .args(["--build", "."])
                .current_dir(&build)
                .output()
                .expect("cmake --build");
            assert!(
                b.status.success(),
                "cmake build failed:\n{}",
                String::from_utf8_lossy(&b.stderr)
            );
        }
        assert!(bin.is_file(), "C binary missing at {}", bin.display());
        bin
    })
}

// ---------------------------------------------------------------------------
// Running one case
// ---------------------------------------------------------------------------

fn invoke(bin: &Path, args: &[&str], stdin: &[u8]) -> Output {
    use std::io::Write;
    let mut child = Command::new(bin)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", bin.display()));
    child
        .stdin
        .as_mut()
        .expect("stdin pipe")
        .write_all(stdin)
        .or_else(|e| {
            // A program that exits before reading stdin gives EPIPE; that is
            // not a test failure, and it happens identically for both binaries.
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

/// `usage()` prints `argv[0]`, which necessarily differs between the two
/// builds. Replace each program's own path with a fixed marker so that the
/// rest of the bytes are still compared exactly.
fn normalize(stream: &[u8], own_path: &Path) -> Vec<u8> {
    let needle = own_path.to_string_lossy().into_owned().into_bytes();
    let mut out = Vec::with_capacity(stream.len());
    let mut i = 0;
    while i < stream.len() {
        if stream[i..].starts_with(&needle) {
            out.extend_from_slice(b"<PROG>");
            i += needle.len();
        } else {
            out.push(stream[i]);
            i += 1;
        }
    }
    out
}

fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

/// Compare stdout, stderr and exit status for one (args, stdin) pair.
#[track_caller]
fn check(args: &[&str], stdin: &[u8]) {
    let c_path = c_bin().clone();
    let r_path = rust_bin();

    let c = invoke(&c_path, args, stdin);
    let r = invoke(&r_path, args, stdin);

    let ctx = || {
        let preview: Vec<u8> = stdin.iter().copied().take(120).collect();
        format!(
            "args={args:?} stdin(len={})={:?}{}",
            stdin.len(),
            String::from_utf8_lossy(&preview),
            if stdin.len() > 120 { " ..." } else { "" }
        )
    };

    assert_eq!(
        show(&c.stdout),
        show(&r.stdout),
        "stdout mismatch for {}",
        ctx()
    );
    assert_eq!(
        c.stdout, r.stdout,
        "stdout byte mismatch for {}\n C={:?}\n R={:?}",
        ctx(),
        c.stdout,
        r.stdout
    );

    let ce = normalize(&c.stderr, &c_path);
    let re = normalize(&r.stderr, &r_path);
    assert_eq!(show(&ce), show(&re), "stderr mismatch for {}", ctx());
    assert_eq!(
        ce,
        re,
        "stderr byte mismatch for {}\n C={:?}\n R={:?}",
        ctx(),
        ce,
        re
    );

    assert_eq!(
        c.status.code(),
        r.status.code(),
        "exit status mismatch for {} (C={:?} Rust={:?})",
        ctx(),
        c.status,
        r.status
    );
}

/// Convenience: bytecode arguments only, empty stdin.
#[track_caller]
fn prog(words: &str) -> Vec<String> {
    words.split_whitespace().map(str::to_owned).collect()
}

#[track_caller]
fn check_prog(words: &str) {
    let owned = prog(words);
    let args: Vec<&str> = owned.iter().map(String::as_str).collect();
    check(&args, b"");
}

// ---------------------------------------------------------------------------
// Phase B: argument handling
// ---------------------------------------------------------------------------

#[test]
fn no_arguments_is_the_empty_program_error() {
    // `code.len == 0` -> "no program" on stderr, exit 2.
    check(&[], b"");
}

#[test]
fn help_flag() {
    check(&["--help"], b"");
}

#[test]
fn help_wins_immediately_wherever_it_appears() {
    // usage() + return 0 happens as soon as --help is seen, so bytecodes and
    // "skip" messages before it still take effect / still print.
    check(&["--help", "5"], b"");
    check(&["5", "--help"], b"");
    check(&["abc", "--help"], b"");
    check(&["--stdin", "--help"], b"0 5");
    check(&["--help", "--stdin"], b"0 5");
}

#[test]
fn non_numeric_arguments_are_skipped_with_a_message() {
    check(&["abc"], b"");
    check(&["abc", "10"], b"");
    check(&["abc", "def", "0", "5"], b"");
    check(&["-"], b"");
    check(&["+"], b"");
    check(&["--"], b"");
    check(&["--verbose", "10"], b"");
}

#[test]
fn strtol_acceptance_matches_exactly() {
    // Whole-string conversion required: `e && *e=='\0'`.
    check(&["+5", "10"], b"");
    check(&[" 5", "10"], b""); // leading whitespace is skipped by strtol
    check(&["\t5", "10"], b"");
    check(&["5 ", "10"], b""); // trailing byte -> skipped
    check(&["0x10", "10"], b""); // base 10: stops at 'x' -> skipped
    check(&["5abc", "10"], b"");
    check(&["  -7  ", "10"], b"");
    check(&["007", "10"], b"");
    check(&["-0", "10"], b"");
}

#[test]
fn empty_argument_parses_as_zero() {
    // strtol performs no conversion, so endptr == nptr, which points at the
    // terminating NUL -- the check passes and 0 is pushed.
    check(&[""], b"");
    check(&["", "10"], b"");
    check(&["0", "", "1"], b"");
}

#[test]
fn strtol_saturation_then_truncation_to_int() {
    // strtol clamps to LONG_MAX/LONG_MIN, then `(int)t` truncates.
    check(&["99999999999999999999", "10"], b"");
    check(&["-99999999999999999999", "10"], b"");
    check(&["2147483648", "10"], b"");
    check(&["-2147483649", "10"], b"");
    check(&["4294967296", "10"], b"");
    check(&["4294967306", "10"], b"");
    check(&["9223372036854775807", "10"], b"");
    check(&["-9223372036854775808", "10"], b"");
    check(&["2147483647"], b"");
    check(&["-2147483648"], b"");
}

#[test]
fn stdin_flag_may_repeat() {
    check(&["--stdin", "--stdin", "5"], b"0 1");
}

// ---------------------------------------------------------------------------
// Phase B: stdin reading (fgets + manual tokenizer)
// ---------------------------------------------------------------------------

#[test]
fn stdin_is_ignored_without_the_flag() {
    check(&[], b"0 5\n");
    check(&["10"], b"0 5\n");
}

#[test]
fn stdin_empty_is_still_the_empty_program() {
    check(&["--stdin"], b"");
    check(&["--stdin"], b"\n");
    check(&["--stdin"], b"\n\n\n");
    check(&["--stdin"], b"   \t \r\n");
}

#[test]
fn stdin_single_and_multiple_tokens() {
    check(&["--stdin"], b"10");
    check(&["--stdin"], b"10\n");
    check(&["--stdin"], b"0 5\n");
    check(&["--stdin"], b"0 5\n1 2\n");
    check(&["--stdin"], b"0\t5\r\n");
    check(&["--stdin"], b"  0 \t\t 5 \r\r\n \n");
    check(&["--stdin"], b"0 5"); // no trailing newline
}

#[test]
fn stdin_and_argv_bytecodes_concatenate_argv_first() {
    check(&["--stdin", "3"], b"0 1\n");
    check(&["3", "--stdin", "8"], b"9 2\n");
}

#[test]
fn stdin_non_numeric_tokens_are_dropped_silently() {
    // read_stdin has no "skip" message, unlike the argv path.
    check(&["--stdin"], b"abc 0 5\n");
    check(&["--stdin"], b"0 5 xyz\n");
    check(&["--stdin"], b"0x10 3\n");
    check(&["--stdin"], b"5abc 3\n");
    check(&["--stdin"], b"- + 3\n");
}

#[test]
fn stdin_embedded_nul_truncates_the_rest_of_the_chunk() {
    // The tokenizer walks a C string, so a NUL discards everything after it in
    // that fgets buffer.
    check(&["--stdin"], b"0 5\x00 1 2\n");
    check(&["--stdin"], b"\x000 5\n");
    check(&["--stdin"], b"0 5\n\x001 2\n");
    check(&["--stdin"], b"\x00");
}

#[test]
fn stdin_tokens_straddling_the_fgets_buffer_boundary_are_split() {
    // buf is char[4096], so fgets stores at most 4095 bytes. A number crossing
    // that edge is tokenized as two separate numbers.
    for pad in [4090usize, 4092, 4093, 4094, 4095, 4096, 4097, 4100] {
        let mut input = vec![b'3'; pad];
        input.extend_from_slice(b" 10\n");
        check(&["--stdin"], &input);

        let mut digits = vec![b'5'; pad];
        digits.push(b'\n');
        check(&["--stdin"], &digits);
    }
}

#[test]
fn stdin_large_inputs() {
    check(&["--stdin"], &b"3\n".repeat(3000));
    let mut one_line = b"3 ".repeat(3000);
    one_line.push(b'\n');
    check(&["--stdin"], &one_line);
    check(&["--stdin"], &b"0 1 2 3 4 5 8 ".repeat(400));
}

// ---------------------------------------------------------------------------
// Phase B/C: one test per opcode, and per run_engine return path
// ---------------------------------------------------------------------------

#[test]
fn every_opcode_on_its_own() {
    for op in -3i64..=13 {
        check_prog(&op.to_string());
    }
}

#[test]
fn unknown_opcodes_return_99() {
    for op in ["11", "12", "13", "42", "1000", "-1", "-2", "2147483647", "-2147483648"] {
        check_prog(op);
    }
}

#[test]
fn run_engine_return_code_1_push_without_immediate() {
    check_prog("0");
    check_prog("0 5 0");
}

#[test]
fn run_engine_return_codes_2_and_3_arithmetic_underflow() {
    check_prog("1"); // ADD, empty stack        -> 2
    check_prog("0 7 1"); // ADD, one operand    -> 2
    check_prog("2"); // MUL, empty stack        -> 3
    check_prog("0 7 2"); // MUL, one operand    -> 3
    check_prog("0 3 0 4 1");
    check_prog("0 3 0 4 2");
}

#[test]
fn run_engine_return_code_4_drop_on_empty_stack() {
    check_prog("4");
    check_prog("0 9 4 4");
}

#[test]
fn run_engine_return_codes_5_and_6_jump_operand_and_condition() {
    check_prog("6"); // no k             -> 5
    check_prog("6 1"); // k, empty stack -> 6
    check_prog("0 1 6"); // cond pushed but no k -> 5
}

#[test]
fn run_engine_return_code_7_jump_target_out_of_range() {
    check_prog("0 1 6 5"); // k beyond the end
    check_prog("0 1 6 1"); // k == remaining + 1
    check_prog("0 1 6 -1"); // (size_t)(-1) is huge -> out of range
    check_prog("0 1 6 -1 10 10 10");
}

#[test]
fn run_engine_jump_in_range_boundaries() {
    check_prog("0 1 6 0"); // k == 0, taken
    check_prog("0 1 6 1 10"); // k == remaining
    check_prog("0 1 6 2 10 10");
    check_prog("0 1 6 1 42"); // skips the bad opcode
    check_prog("0 0 6 5 10"); // cond == 0 -> not taken, k ignored
    check_prog("0 0 6 -1 10"); // cond == 0 -> bound check skipped
}

#[test]
fn run_engine_return_codes_8_and_9_repeat_operands() {
    check_prog("7"); // no times           -> 8
    check_prog("7 2"); // nothing to repeat -> 9
    check_prog("0 5 7 3"); // ditto
}

#[test]
fn run_engine_repeat_bodies() {
    for times in ["-3", "0", "1", "2", "5", "40"] {
        for body in ["3", "0", "1", "2", "4", "5", "8", "9", "10", "42", "7"] {
            check_prog(&format!("7 {times} {body}"));
        }
    }
    check_prog("7 3 7 2 3");
    check_prog("7 2 9 1");
    check_prog("0 4 0 6 7 3 1");
    check_prog("7 1000 3");
}

#[test]
fn run_engine_return_codes_10_and_11_reduce_operands() {
    check_prog("9"); // no m                 -> 10
    check_prog("9 -1"); // m < 0             -> 11
    check_prog("9 3"); // m > stack.len      -> 11
    check_prog("0 1 9 2"); // m > stack.len  -> 11
    check_prog("9 -2147483648");
}

#[test]
fn reduce_pops_its_operands_twice() {
    // The C pops m values, then pops them *again*. When the second pass runs
    // the stack dry, iv_pop leaves the slot holding the first pass's value.
    check_prog("9 0"); // m == 0, zero-length VLA
    check_prog("0 10 9 1");
    check_prog("0 10 0 20 9 1"); // second pop succeeds
    check_prog("0 10 0 20 9 2"); // second pop fully fails
    check_prog("0 10 0 20 0 30 9 2"); // second pop partially succeeds
    check_prog("0 10 0 20 0 30 9 3");
    check_prog("0 10 0 20 0 30 0 40 9 3"); // partial: only tmp[2] overwritten
    check_prog("0 10 0 20 0 30 0 40 0 50 9 3");
    check_prog("0 10 0 20 0 30 0 40 0 50 0 60 9 3"); // both passes succeed
    check_prog("0 1 0 2 0 3 0 4 0 5 0 6 0 7 0 8 9 4");
}

#[test]
fn dup_and_classify_use_peek_defaults_on_an_empty_stack() {
    check_prog("3"); // DUP pushes iv_peek(stack, 0) == 0
    check_prog("3 3 3");
    check_prog("5"); // CLASSIFY of the default 0
    check_prog("8");
    check_prog("5 5 5 5");
    check_prog("8 8 8 8");
    check_prog("5 8 5 8");
}

#[test]
fn halt_stops_before_the_rest_of_the_program() {
    check_prog("10");
    check_prog("0 5 10 42"); // 42 would return 99, but HALT wins
    check_prog("10 0");
    check_prog("3 10 1");
}

#[test]
fn stack_top_default_is_printed_when_the_stack_is_empty() {
    // vm_print uses iv_peek(stack, -777).
    check_prog("0 1 4"); // push then drop -> empty
    check_prog("6 1"); // errors out with an empty stack
}

#[test]
fn reduce_of_a_stream_hits_the_clamping_bug_in_process_a_stream() {
    // process_a_stream's clamps are unsigned comparisons, so it always ends up
    // returning INT_MIN; the Rust must do the same.
    check_prog("9 0");
    check_prog("0 3 9 1");
    check_prog("0 100 0 200 0 300 9 3");
    check_prog("0 -1 0 -2 9 2");
    check_prog("0 2147483647 0 -2147483648 9 2");
}

#[test]
fn negative_and_extreme_immediates_reach_the_negative_code_paths() {
    // target(code < 0) is a distinct branch in a.c, b.c and lib.c.
    for imm in [
        "-1",
        "-2",
        "-7",
        "-100",
        "-2147483648",
        "2147483647",
        "1073741824",
    ] {
        check_prog(&format!("0 {imm} 5"));
        check_prog(&format!("0 {imm} 8"));
        check_prog(&format!("0 {imm} 9 1"));
    }
}

#[test]
fn lib_target_covers_every_modulo_bucket() {
    // lib.c's `target` branches on `code % 10`: 0, 1..3, 4..6, 7, else.
    // The EXT engine reaches it via classify (target(target(x+1))) and via
    // process_stream (target(buf[i])). Buckets m == 7, 8, 9 need immediates
    // that random opcode-shaped programs almost never produce.
    for imm in [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 16, 17, 18, 19, 26, 27, 28, 29, 37, 38, 39, 77, 88, 99,
    ] {
        check_prog(&format!("0 {imm} 5"));
        check_prog(&format!("0 {imm} 8"));
        check_prog(&format!("0 {imm} 9 1"));
    }
    check_prog("0 7 0 8 0 9 9 3");
    check_prog("0 17 0 28 0 39 9 3");
    check_prog("0 6 0 7 0 8 0 9 9 4");
}

#[test]
fn classify_bucket_switch_covers_every_arm() {
    // engine.c's `case 5` maps the classify result to a trace letter through a
    // 0/1/2/(3,4)/default switch; bucket == 2 only happens for particular
    // immediates, so it is pinned down explicitly here.
    for imm in [0i64, 1, 2, 3, 4, 5, 6, 7, 8, 9, 13, 15, 23, 33, -1, -9] {
        check_prog(&format!("0 {imm} 5"));
        check_prog(&format!("0 {imm} 5 5"));
        check_prog(&format!("0 {imm} 3 5"));
    }
}

#[test]
fn negative_codes_exercise_the_early_return_in_every_target() {
    // a.c, b.c and lib.c each have a distinct `code < 0` path, and a.c's
    // depends on the parity of the accumulated `state_a`.
    for n in 1..=25i64 {
        check_prog(&format!("0 {} 5", -n));
        check_prog(&format!("0 {} 8", -n));
        check_prog(&format!("0 {} 5 0 {} 5", -n, -n - 1));
        check_prog(&format!("0 {} 8 3 8 3 8", -n));
    }
}

#[test]
fn static_state_carries_across_the_three_engine_runs() {
    // state_a / flipflop are file-scope statics shared by the A, B and EXT
    // runs, so results depend on how many classify calls happened before.
    check_prog("5 5 5 5 5 5 5 5 5 5");
    check_prog("8 8 8 8 8 8 8 8 8 8");
    check_prog("0 1 5 0 2 5 0 3 5 0 4 5");
    check_prog("3 5 3 5 3 5 3 5 3 5 3 5");
    check_prog("0 7 9 1 0 7 9 1 0 7 9 1");
}

#[test]
fn engines_can_disagree_on_the_return_code() {
    // Cases found by search where rcA, rcB and rcE are not all equal, i.e.
    // control flow really does depend on the classify implementation.
    for p in [
        "9 0 6 8 7 42",
        "5 9 0 2 6 3",
        "8 0 -1 1 6 10 0",
        "8 0 9 2 5 6 -1",
        "5 5 9 0 6 42 7",
        "9 0 6 5 1",
        "0 4 0 -1 8 6 6",
        "0 3 9 0 6 5 8",
        "5 0 9 5 6 5 4",
        "8 9 0 6 5 7 8",
        "9 0 6 4 2 0",
        "9 0 6 8",
    ] {
        check_prog(p);
    }
}

// ---------------------------------------------------------------------------
// Phase C: exhaustive short programs and a deterministic fuzz
// ---------------------------------------------------------------------------

#[test]
fn exhaustive_two_opcode_programs() {
    for a in -2i64..=12 {
        for b in -2i64..=12 {
            check_prog(&format!("{a} {b}"));
        }
    }
}

/// Small xorshift so the corpus is fixed and reproducible without adding a
/// dependency.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[(self.next() % xs.len() as u64) as usize]
    }
    fn range(&mut self, lo: usize, hi: usize) -> usize {
        lo + (self.next() as usize) % (hi - lo + 1)
    }
}

#[test]
fn fuzz_opcode_heavy_programs() {
    const OPS: [i64; 18] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, -1, 42, 25, 26, 2147483647, -2147483648,
    ];
    let mut rng = Rng(0x1234_5678_9abc_def1);
    for _ in 0..400 {
        let n = rng.range(1, 10);
        let words: Vec<String> = (0..n).map(|_| rng.pick(&OPS).to_string()).collect();
        check_prog(&words.join(" "));
    }
}

#[test]
fn fuzz_small_integer_programs() {
    let mut rng = Rng(0x0f0f_0f0f_dead_beef);
    let vals: Vec<i64> = (-20..=20).collect();
    for _ in 0..400 {
        let n = rng.range(1, 14);
        let words: Vec<String> = (0..n).map(|_| rng.pick(&vals).to_string()).collect();
        check_prog(&words.join(" "));
    }
}

#[test]
fn fuzz_long_programs() {
    const OPS: [i64; 15] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, -1, 42, 3, 5];
    let mut rng = Rng(0xfeed_face_0000_0001);
    for _ in 0..200 {
        let n = rng.range(20, 60);
        let words: Vec<String> = (0..n).map(|_| rng.pick(&OPS).to_string()).collect();
        check_prog(&words.join(" "));
    }
}

#[test]
fn fuzz_programs_delivered_over_stdin() {
    const OPS: [i64; 14] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, -1, 42, 11];
    let mut rng = Rng(0xabcd_ef01_2345_6789);
    for _ in 0..200 {
        let n = rng.range(1, 20);
        let words: Vec<String> = (0..n).map(|_| rng.pick(&OPS).to_string()).collect();
        // Randomly split across lines and separator kinds.
        let mut input = Vec::new();
        for (i, w) in words.iter().enumerate() {
            if i > 0 {
                input.extend_from_slice(rng.pick(&[" ", "\t", "\n", " \r\n", "  "]).as_bytes());
            }
            input.extend_from_slice(w.as_bytes());
        }
        input.push(b'\n');
        check(&["--stdin"], &input);
    }
}
