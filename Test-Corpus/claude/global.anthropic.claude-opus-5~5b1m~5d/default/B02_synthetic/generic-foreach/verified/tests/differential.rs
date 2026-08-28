// Differential tests: run the original C program and the Rust translation as
// subprocesses on identical stdin, then compare stdout (byte for byte), stderr
// (byte for byte) and the exit status (both the exit code and the terminating
// signal).
//
// The Rust code is never called as a library here; only the built binary is
// driven, the way a shell drives it.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Locating / building the two executables
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rust_exe() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Build `c_src` with CMake (out of tree, so nothing in `c_src/` is touched)
/// and return the path of the resulting `driver` executable.  If the tree has
/// already been built in place by hand, that binary is reused.
fn c_exe() -> &'static Path {
    static C_EXE: OnceLock<PathBuf> = OnceLock::new();
    C_EXE.get_or_init(|| {
        let root = manifest_dir()
            .parent()
            .expect("translation/ has a parent")
            .to_path_buf();
        let c_src = root.join("c_src");
        assert!(
            c_src.join("CMakeLists.txt").is_file(),
            "cannot find c_src/CMakeLists.txt next to the crate (looked in {})",
            c_src.display()
        );

        // Reuse an existing in-tree build if one is present.
        let in_tree = c_src.join("build").join("driver");
        if in_tree.is_file() {
            return in_tree;
        }

        // Otherwise configure and build into the crate's target directory so
        // that c_src/ stays pristine.
        let build_dir = manifest_dir().join("target").join("c_build");
        std::fs::create_dir_all(&build_dir).expect("create c build dir");

        let cfg = Command::new("cmake")
            .arg("-S")
            .arg(&c_src)
            .arg("-B")
            .arg(&build_dir)
            .output()
            .expect("run cmake (is cmake installed?)");
        assert!(
            cfg.status.success(),
            "cmake configure failed:\n{}\n{}",
            String::from_utf8_lossy(&cfg.stdout),
            String::from_utf8_lossy(&cfg.stderr)
        );

        let bld = Command::new("cmake")
            .arg("--build")
            .arg(&build_dir)
            .output()
            .expect("run cmake --build");
        assert!(
            bld.status.success(),
            "cmake build failed:\n{}\n{}",
            String::from_utf8_lossy(&bld.stdout),
            String::from_utf8_lossy(&bld.stderr)
        );

        let exe = build_dir.join("driver");
        assert!(exe.is_file(), "C driver not produced at {}", exe.display());
        exe
    })
}

// ---------------------------------------------------------------------------
// Running one program
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq)]
struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    code: Option<i32>,
    signal: Option<i32>,
}

fn signal_of(status: &std::process::ExitStatus) -> Option<i32> {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        status.signal()
    }
    #[cfg(not(unix))]
    {
        let _ = status;
        None
    }
}

/// Run `exe` with `input` on stdin and everything captured.
fn run_with(exe: &Path, args: &[&str], input: &[u8]) -> Run {
    let mut child = Command::new(exe)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));

    // Feed stdin from a helper thread: the program may exit (choice 7) while
    // input remains, and it may emit more output than a pipe holds.  Write
    // errors are ignored, exactly as a shell's redirection would.
    let mut sink = child.stdin.take().expect("stdin piped");
    let data = input.to_vec();
    let writer = std::thread::spawn(move || {
        let _ = sink.write_all(&data);
        let _ = sink.flush();
        drop(sink);
    });

    let out = child.wait_with_output().expect("wait_with_output");
    writer.join().expect("stdin writer thread");

    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: signal_of(&out.status),
    }
}

/// Run with stdin closed rather than piped (fgets sees a read error / EOF).
fn run_closed_stdin(exe: &Path) -> Run {
    let out = Command::new(exe)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: signal_of(&out.status),
    }
}

// ---------------------------------------------------------------------------
// Comparison
// ---------------------------------------------------------------------------

fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Byte position and context of the first difference, for a useful failure.
fn first_diff(a: &[u8], b: &[u8]) -> String {
    let at = a
        .iter()
        .zip(b.iter())
        .position(|(x, y)| x != y)
        .unwrap_or_else(|| a.len().min(b.len()));
    let lo = at.saturating_sub(60);
    format!(
        "first difference at byte {at} (C len {}, Rust len {})\n\
         C   ...{:?}\n\
         Rust...{:?}",
        a.len(),
        b.len(),
        show(&a[lo..a.len().min(at + 60)]),
        show(&b[lo..b.len().min(at + 60)]),
    )
}

fn assert_matches_named(case: &str, args: &[&str], input: &[u8]) {
    let c = run_with(c_exe(), args, input);
    let r = run_with(&rust_exe(), args, input);

    assert!(
        c.stdout == r.stdout,
        "[{case}] stdout differs\ninput = {:?}\n{}",
        show(input),
        first_diff(&c.stdout, &r.stdout)
    );
    assert!(
        c.stderr == r.stderr,
        "[{case}] stderr differs\ninput = {:?}\nC   = {:?}\nRust= {:?}",
        show(input),
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "[{case}] exit status differs (code, signal)\ninput = {:?}",
        show(input)
    );
}

fn assert_matches(case: &str, input: &[u8]) {
    assert_matches_named(case, &[], input);
}

/// Assert every case in a table, reporting all failures at once.
fn assert_all(cases: &[(&str, &[u8])]) {
    let mut failures = Vec::new();
    for (name, input) in cases {
        let res = std::panic::catch_unwind(|| assert_matches(name, input));
        if let Err(e) = res {
            let msg = e
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_else(|| "<non-string panic>".to_string());
            failures.push(msg);
        }
    }
    assert!(
        failures.is_empty(),
        "{} case(s) differed:\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}

// ===========================================================================
// Menu dispatch: every `case` of the switch, and the `default`
// ===========================================================================

#[test]
fn empty_input_eof_immediately() {
    // fgets() returns NULL on the first call -> break -> return 0.
    assert_matches("empty", b"");
}

#[test]
fn single_item_exit_choice_7() {
    // The only `return` inside the loop.
    assert_matches("exit", b"7\n");
}

#[test]
fn each_demo_individually() {
    assert_all(&[
        ("demo1_integer_containers", b"1\n7\n"),
        ("demo2_double_containers", b"2\n7\n"),
        ("demo3_inventory_array", b"3\n7\n"),
        ("demo4_order_list", b"4\n7\n"),
        ("demo5_mixed_operations", b"5\n7\n"),
        ("demo6_run_all", b"6\n7\n"),
    ]);
}

#[test]
fn every_choice_in_one_session() {
    assert_matches("sequence_1_to_7", b"1\n2\n3\n4\n5\n6\n7\n");
}

#[test]
fn demos_without_exit_then_eof() {
    // Loop terminates through the fgets()-returns-NULL path, not `case 7`.
    assert_all(&[
        ("demo1_then_eof", b"1\n"),
        ("demo6_then_eof", b"6\n"),
        ("demos_then_eof", b"3\n4\n"),
    ]);
}

#[test]
fn default_branch_invalid_choice() {
    // Parsed as a number, but not 1..=7 -> "Invalid choice".
    assert_all(&[
        ("choice_0", b"0\n7\n"),
        ("choice_8", b"8\n7\n"),
        ("choice_9", b"9\n7\n"),
        ("choice_negative", b"-1\n7\n"),
        ("choice_large", b"1000000\n7\n"),
        ("choice_int_max", b"2147483647\n7\n"),
        ("choice_int_min", b"-2147483648\n7\n"),
    ]);
}

// ===========================================================================
// The sscanf(input, "%d", &choice) != 1 branch -> "Invalid input"
// ===========================================================================

#[test]
fn invalid_input_no_conversion() {
    assert_all(&[
        ("blank_line", b"\n7\n"),
        ("spaces_only", b"   \n7\n"),
        ("tabs_only", b"\t\t\n7\n"),
        ("crlf_only", b"\r\n7\n"),
        ("alpha", b"abc\n7\n"),
        ("plus_only", b"+\n7\n"),
        ("minus_only", b"-\n7\n"),
        ("plus_plus", b"++7\n7\n"),
        ("minus_alpha", b"-abc\n7\n"),
        ("leading_dot", b".5\n7\n"),
        ("leading_e", b"e5\n7\n"),
        ("hash", b"#\n7\n"),
        ("nul_byte_first", b"\x007\n7\n"),
        ("high_bytes", b"\xff\xfe\n7\n"),
        ("utf8_fullwidth_seven", "\u{ff17}\n7\n".as_bytes()),
        ("many_blank_lines", b"\n\n\n\n\n\n\n\n\n\n"),
    ]);
}

#[test]
fn repeated_invalid_input_keeps_looping() {
    assert_matches("invalid_then_valid_then_invalid", b"zz\n6\nqq\n7\n");
}

// ===========================================================================
// sscanf partial matches: it converts a prefix and ignores the rest
// ===========================================================================

#[test]
fn partial_conversions_are_accepted() {
    assert_all(&[
        ("digits_then_letters", b"3abc\n7\n"),
        ("digits_then_dot", b"5.9\n7\n"),
        ("two_numbers_on_one_line", b"1 2\n7\n"),
        ("leading_whitespace", b"  \t 5\n7\n"),
        ("leading_newlines_skipped_in_line", b"  \t7\n"),
        ("explicit_plus", b"+4\n7\n"),
        ("leading_zeros", b"007\n"),
        ("many_leading_zeros", b"00000000000000000000001\n7\n"),
        ("hex_prefix_reads_zero", b"0x10\n7\n"),
        ("trailing_space", b"7 \n"),
        ("vertical_tab_and_form_feed", b"\x0b\x0c7\n"),
        ("nul_after_digit", b"1\x002\n7\n"),
        ("nul_terminates_line", b"7\x00\n"),
        ("negative_with_spaces", b"   -5   \n7\n"),
    ]);
}

// ===========================================================================
// Integer overflow / truncation exactly as the C performs it
// ===========================================================================

#[test]
fn overflow_and_truncation_of_choice() {
    assert_all(&[
        // 2^31: no longer a positive int.
        ("int_max_plus_1", b"2147483648\n7\n"),
        ("int_min_minus_1", b"-2147483649\n7\n"),
        // 2^32 + n truncates to n, so these really do run the demos.
        ("two_pow_32_plus_1_runs_demo1", b"4294967297\n7\n"),
        ("two_pow_32_plus_6_runs_demo6", b"4294967302\n7\n"),
        ("two_pow_32_plus_7_exits", b"4294967303\n"),
        ("negative_wrap_runs_demo1", b"-4294967295\n7\n"),
        // Saturation at LONG_MAX / LONG_MIN before the store into int.
        ("long_max", b"9223372036854775807\n7\n"),
        ("long_max_plus_1", b"9223372036854775808\n7\n"),
        ("long_min", b"-9223372036854775808\n7\n"),
        ("long_min_minus_1", b"-9223372036854775809\n7\n"),
        ("two_pow_64_plus_1", b"18446744073709551617\n7\n"),
        ("twenty_nines", b"99999999999999999999\n7\n"),
        ("twenty_nines_negative", b"-99999999999999999999\n7\n"),
        ("forty_digits", b"1234567890123456789012345678901234567890\n7\n"),
    ]);
}

// ===========================================================================
// fgets() buffer boundaries: char input[256] reads at most 255 bytes and does
// not read across newlines
// ===========================================================================

#[test]
fn fgets_line_length_boundaries() {
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();

    // A line of exactly the maximum fgets() takes (255 bytes), and its
    // neighbours, each still starting with a valid choice.
    for len in [254usize, 255, 256, 257] {
        let mut v = b"1".to_vec();
        v.extend(std::iter::repeat(b' ').take(len - 1));
        v.extend_from_slice(b"\n7\n");
        cases.push((format!("line_of_{len}_then_newline"), v));
    }

    // 255 bytes of whitespace fills the buffer with no digit at all, so the
    // first read is an "Invalid input" and the tail of the line is the next.
    let mut ws = vec![b' '; 255];
    ws.extend_from_slice(b"7\n");
    cases.push(("whitespace_fills_buffer".to_string(), ws));

    // A single line far longer than the buffer is chopped into several reads.
    let mut long = vec![b'9'; 300];
    long.extend_from_slice(b"\n7\n");
    cases.push(("line_of_300".to_string(), long));

    let mut huge = b"5".to_vec();
    huge.extend(std::iter::repeat(b' ').take(10_000));
    huge.extend_from_slice(b"\n7\n");
    cases.push(("line_of_10001".to_string(), huge));

    let refs: Vec<(&str, &[u8])> = cases
        .iter()
        .map(|(n, v)| (n.as_str(), v.as_slice()))
        .collect();
    assert_all(&refs);
}

#[test]
fn last_line_without_trailing_newline() {
    assert_all(&[
        ("exit_no_newline", b"7"),
        ("demo6_no_newline", b"6"),
        ("invalid_no_newline", b"x"),
        ("empty_choice_no_newline", b" "),
        ("after_demo_no_newline", b"1\n7"),
    ]);
}

#[test]
fn line_of_exactly_255_bytes_without_newline() {
    let mut v = b"1".to_vec();
    v.extend(std::iter::repeat(b' ').take(254));
    assert_matches("255_bytes_no_newline", &v);

    let mut v = b"2".to_vec();
    v.extend(std::iter::repeat(b' ').take(255));
    assert_matches("256_bytes_no_newline", &v);
}

// ===========================================================================
// Environment-level behaviour
// ===========================================================================

#[test]
fn command_line_arguments_are_ignored() {
    // main() takes (void); argv must make no difference.
    let c = run_with(c_exe(), &["foo", "bar"], b"7\n");
    let r = run_with(&rust_exe(), &["foo", "bar"], b"7\n");
    assert!(c.stdout == r.stdout, "stdout differs with argv");
    assert!(c.stderr == r.stderr, "stderr differs with argv");
    assert_eq!((c.code, c.signal), (r.code, r.signal), "status with argv");
}

#[test]
fn stdin_from_dev_null() {
    let c = run_closed_stdin(c_exe());
    let r = run_closed_stdin(&rust_exe());
    assert!(
        c.stdout == r.stdout,
        "stdout differs with stdin closed\n{}",
        first_diff(&c.stdout, &r.stdout)
    );
    assert!(c.stderr == r.stderr, "stderr differs with stdin closed");
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "status with stdin closed"
    );
}

#[test]
fn neither_program_writes_to_stderr() {
    // Guards the stderr comparison above from being vacuous.
    let c = run_with(c_exe(), &[], b"1\n2\n3\n4\n5\n6\nzz\n9\n7\n");
    assert!(
        c.stderr.is_empty() && !c.stdout.is_empty(),
        "expected the C program to write only to stdout"
    );
}

#[test]
fn output_is_not_trivially_empty() {
    // Guards every stdout comparison from passing on two empty streams.
    let r = run_with(&rust_exe(), &[], b"6\n7\n");
    assert!(
        r.stdout.len() > 5_000,
        "expected substantial output from 'run all demos', got {} bytes",
        r.stdout.len()
    );
}

// ===========================================================================
// Breadth: a fixed-seed sweep so the suite covers input shapes that were not
// enumerated by hand.  The seed is constant, so this is reproducible, never
// flaky, and any failure it reports can be replayed from the printed bytes.
// ===========================================================================

#[test]
fn deterministic_random_inputs() {
    // xorshift64*, so no external crate is needed.
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            self.0 = x;
            x.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }
        fn below(&mut self, n: usize) -> usize {
            (self.next() % n as u64) as usize
        }
    }

    const ALPHABETS: [&[u8]; 5] = [
        b"1234567\n",
        b"0123456789\n",
        b"0123456789 \t\n+-",
        b"\n \t\r\x00abcxyz+-.0123456789",
        b"\n7\xff\xfe\x01 0123456789",
    ];
    const LENGTHS: [usize; 8] = [0, 1, 2, 4, 12, 40, 260, 600];

    let mut rng = Rng(0x2026_0828_C0FF_EE01);
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    for i in 0..250 {
        let alpha = ALPHABETS[rng.below(ALPHABETS.len())];
        let len = LENGTHS[rng.below(LENGTHS.len())];
        let mut data: Vec<u8> = (0..len).map(|_| alpha[rng.below(alpha.len())]).collect();
        if rng.below(2) == 0 {
            data.extend_from_slice(b"\n7\n"); // sometimes let it exit cleanly
        }
        cases.push((format!("fuzz_{i}"), data));
    }

    let refs: Vec<(&str, &[u8])> = cases
        .iter()
        .map(|(n, v)| (n.as_str(), v.as_slice()))
        .collect();
    assert_all(&refs);
}

/// When the reader of stdout disappears, a C program is killed by SIGPIPE.
/// The Rust runtime ignores SIGPIPE unless the translation restores the
/// default disposition, so this pins the exit status down.
#[cfg(unix)]
#[test]
fn dead_stdout_reader_terminates_both_the_same_way() {
    fn run_and_drop_reader(exe: &Path) -> (Option<i32>, Option<i32>) {
        let mut child = Command::new(exe)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap_or_else(|e| panic!("spawn {}: {e}", exe.display()));

        // Close the read end of the stdout pipe immediately.  From here on any
        // write the child performs must fail with EPIPE.
        drop(child.stdout.take());

        let mut sink = child.stdin.take().expect("stdin piped");
        let writer = std::thread::spawn(move || {
            // "Run all demos" 500 times is ~3 MB of output.  That is far more
            // than a pipe can hold (64 KiB by default), so the child cannot
            // race ahead and finish before the reader is gone: it must either
            // hit EPIPE straight away or block on a full pipe and then be
            // woken by the closed read end.  The input itself is only 1 kB, so
            // it fits in the stdin pipe and this write never blocks.
            let _ = sink.write_all(&b"6\n".repeat(500));
            let _ = sink.flush();
        });

        let status = child.wait().expect("wait");
        let _ = writer.join();
        (status.code(), signal_of(&status))
    }

    let c = run_and_drop_reader(c_exe());
    let r = run_and_drop_reader(&rust_exe());
    assert_eq!(
        c, r,
        "(code, signal) differ when the stdout reader is gone: C {c:?} vs Rust {r:?}"
    );
}
