//! Differential tests: run the C binary and the Rust binary as subprocesses,
//! feed both the same bytes on stdin, and compare stdout, stderr and exit
//! status.
//!
//! The Rust code is never called as a library; both programs are driven exactly
//! the way a shell would drive them, because that is what is being compared.

use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

// ---------------------------------------------------------------------------
// Locating and building the two programs
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// Path to the built C executable, building it with CMake if necessary.
fn c_binary() -> PathBuf {
    let c_src = workspace_root().join("c_src");
    let build = c_src.join("build");
    let binary = build.join("driver");
    if binary.exists() {
        return binary;
    }

    std::fs::create_dir_all(&build).expect("create c_src/build");
    let configure = Command::new("cmake")
        .arg("..")
        .current_dir(&build)
        .output()
        .expect("cmake must be installed to build the C reference");
    assert!(
        configure.status.success(),
        "cmake configure failed:\n{}",
        String::from_utf8_lossy(&configure.stderr)
    );
    let compile = Command::new("cmake")
        .args(["--build", "."])
        .current_dir(&build)
        .output()
        .expect("run cmake --build");
    assert!(
        compile.status.success(),
        "cmake --build failed:\n{}",
        String::from_utf8_lossy(&compile.stderr)
    );

    assert!(binary.exists(), "C binary missing after build: {binary:?}");
    binary
}

/// Path to the built Rust executable. Cargo builds it before running tests.
fn rust_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

// ---------------------------------------------------------------------------
// Running one program
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// Exit code, or None when the process was killed by a signal.
    code: Option<i32>,
    /// Terminating signal, or None on a normal exit.
    signal: Option<i32>,
}

impl Outcome {
    fn killed_by_signal(&self) -> bool {
        self.signal.is_some()
    }
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Outcome")
            .field("code", &self.code)
            .field("signal", &self.signal)
            .field("stdout", &String::from_utf8_lossy(&self.stdout))
            .field("stderr", &String::from_utf8_lossy(&self.stderr))
            .finish()
    }
}

fn run(binary: &Path, input: &[u8]) -> Outcome {
    let mut child = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {binary:?}: {e}"));

    // The program may die before consuming all of stdin; a broken pipe here is
    // expected and must not fail the test.
    let mut stdin = child.stdin.take().expect("piped stdin");
    let _ = stdin.write_all(input);
    drop(stdin);

    let out = child.wait_with_output().expect("wait for child");
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

/// Renders an input for assertion messages.
fn show(input: &[u8]) -> String {
    let mut s = String::new();
    for &b in input {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            0x20..=0x7e => s.push(b as char),
            other => s.push_str(&format!("\\x{other:02x}")),
        }
    }
    s
}

/// The core assertion: all three observable channels must be identical.
fn assert_identical(input: &[u8]) {
    let c = run(&c_binary(), input);
    let r = run(&rust_binary(), input);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout differs for input \"{}\"\n  C:    {:?}\n  Rust: {:?}",
        show(input),
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr differs for input \"{}\"\n  C:    {:?}\n  Rust: {:?}",
        show(input),
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "exit status differs for input \"{}\"\n  C:    code={:?} signal={:?}\n  Rust: code={:?} signal={:?}",
        show(input),
        c.code,
        c.signal,
        r.code,
        r.signal
    );
}

/// Input that drives `goodB2G` with `first` and `bad()` with `second`.
fn two_lines(first: &str, second: i64) -> Vec<u8> {
    format!("{first}\n{second}\n").into_bytes()
}

// ---------------------------------------------------------------------------
// Bounding the region where the C's out-of-bounds store is deterministic
// ---------------------------------------------------------------------------
//
// `bad()`'s stray store is absorbed while it stays inside the stack mapping and
// faults once it leaves it. The distance from `bad`'s frame to the top of that
// mapping is the size of the argv/env block plus a per-exec random offset that
// the kernel subtracts from the initial stack pointer. Measured spread of that
// offset on this kernel: 8160 bytes, i.e. 16-byte-aligned within 8 KiB.
//
// So there is a band, a couple of thousand indices wide, in which the *same* C
// binary exits 0 on one run and dies on the next. Test points must be chosen
// outside that band, and where the band lies depends on how big the environment
// is when the tests run. These helpers derive the safe bounds at run time
// instead of hard-coding indices that happen to work in one shell.

/// Widest per-exec stack randomisation observed on this kernel, in bytes.
const ASLR_STACK_SPREAD: i64 = 8176;

/// Distance from the initial stack pointer to `bad`'s `%rbp` in the C binary.
const RBP_BELOW_STARTSTACK: i64 = 304;

/// Margin, in indices, kept between a test point and the computed band edge.
const BAND_MARGIN: i64 = 256;

/// `stack_end - startstack` for the *test* process: the argv/env block size
/// plus this process's own random offset.
fn stack_end_minus_startstack() -> Option<i64> {
    let maps = std::fs::read_to_string("/proc/self/maps").ok()?;
    let mut end = None;
    for line in maps.lines() {
        if line.ends_with("[stack]") {
            let range = line.split_whitespace().next()?;
            end = u64::from_str_radix(range.split('-').nth(1)?, 16).ok();
        }
    }
    let stat = std::fs::read_to_string("/proc/self/stat").ok()?;
    let after_comm = &stat[stat.rfind(')')? + 1..];
    let start: u64 = after_comm.split_whitespace().nth(28 - 3)?.parse().ok()?;
    Some((end? - start) as i64)
}

/// Largest index the C absorbs on *every* run, in the current environment.
///
/// The child's headroom is at least `base + RBP_BELOW_STARTSTACK`, and this
/// process's measurement is `base + random`, so `base` is at least
/// `measured - ASLR_STACK_SPREAD`. Index `n` is absorbed while `4n - 60` stays
/// within the headroom.
fn always_absorbed_ceiling() -> i64 {
    let measured = stack_end_minus_startstack().unwrap_or(ASLR_STACK_SPREAD);
    let min_headroom = measured - ASLR_STACK_SPREAD + RBP_BELOW_STARTSTACK;
    ((min_headroom + 60) / 4) - BAND_MARGIN
}

/// Smallest index that faults on *every* run, in the current environment.
///
/// Bounded above by 2e8: past roughly that point the wild address starts
/// landing in file-backed mappings and the C alternates SIGSEGV/SIGBUS.
fn always_faults_floor() -> i64 {
    let measured = stack_end_minus_startstack().unwrap_or(0);
    let max_headroom = measured + ASLR_STACK_SPREAD + RBP_BELOW_STARTSTACK;
    let analytic = ((max_headroom + 60) / 4) + BAND_MARGIN;
    // 100_000 covers any plausible environment on its own; take whichever is
    // larger so neither estimate can be the weak one.
    analytic.max(100_000)
}

// ---------------------------------------------------------------------------
// Phase A: both programs are runnable
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_exist_and_run() {
    let c = run(&c_binary(), b"0\n0\n");
    let r = run(&rust_binary(), b"0\n0\n");
    assert_eq!(c.code, Some(0), "C reference should exit 0 on a benign input");
    assert_eq!(r.code, Some(0), "Rust should exit 0 on a benign input");
    assert!(!c.stdout.is_empty(), "C should produce output");
    assert_eq!(c.stdout, r.stdout);
}

// ---------------------------------------------------------------------------
// Phase B: the source-level branches
// ---------------------------------------------------------------------------

/// `fgets` returns NULL in *both* call sites: "fgets() failed." twice, then
/// data stays -1, so goodB2G takes its out-of-bounds branch and bad() takes its
/// negative branch.
#[test]
fn empty_input_hits_both_fgets_failure_paths() {
    assert_identical(b"");
}

/// One line only: goodB2G consumes it, then bad()'s fgets hits EOF.
#[test]
fn single_line_then_eof_in_bad() {
    assert_identical(b"5");
    assert_identical(b"5\n");
    assert_identical(b"0\n");
    assert_identical(b"9\n");
}

/// Every in-bounds index, in both the good sink and the bad sink.
#[test]
fn every_in_bounds_index() {
    for n in 0..10 {
        assert_identical(&two_lines(&n.to_string(), n));
    }
}

/// The negative branch of `bad()` ("ERROR: Array index is negative.") and the
/// out-of-bounds branch of `goodB2G` ("ERROR: Array index is out-of-bounds").
#[test]
fn negative_and_out_of_bounds_error_branches() {
    for first in ["-1", "-2", "-2147483648", "10", "99", "abc"] {
        for second in ["-1", "-7", "-2147483648"] {
            assert_identical(format!("{first}\n{second}\n").as_bytes());
        }
    }
}

/// goodB2G rejects >= 10 while bad() accepts it: the asymmetry between the two
/// sinks, checked at the boundary.
#[test]
fn good_sink_boundary_at_ten() {
    for n in [8, 9, 10, 11] {
        assert_identical(&two_lines(&n.to_string(), 0));
    }
}

// ---------------------------------------------------------------------------
// Phase B: atoi() surface
// ---------------------------------------------------------------------------

#[test]
fn atoi_accepted_and_rejected_forms() {
    let forms = [
        "0", "9", "00", "007", "+5", "-0", "--5", "+-5", " 7", "  7", "\t3", "\r5", "5\r", "7abc",
        "0x10", "abc", "", " ", "   ", ".5", "5.9", "1e3", "-", "+", "9,9",
    ];
    for f in forms {
        assert_identical(format!("{f}\n{f}\n").as_bytes());
    }
}

/// `inputBuffer` is 14 bytes, so at most 13 digits reach `atoi`. `strtol`
/// therefore cannot overflow, but the narrowing to `int` can and does.
///
/// The value under test is fed to `goodB2G`, whose `data < 10` check makes every
/// result safe, so arbitrarily large inputs can be compared strictly here. The
/// bad sink is driven separately by `oob_index_reached_by_truncation`, using
/// only truncated values that land on deterministic slots.
#[test]
fn atoi_int_truncation() {
    let values = [
        "2147483647",    // INT_MAX
        "-2147483648",   // INT_MIN
        "2147483648",    // wraps negative
        "4294967295",    // -> -1
        "4294967296",    // -> 0
        "4294967306",    // -> 10
        "1234567890123", // 13 digits, truncates to 1912239307
        "9999999999999", // 13 nines, truncates to 1316073471
        "-999999999999",
        "-000000000001",
        "0000000000000",
    ];
    for v in values {
        // Large value in the bounds-checked sink, benign index in the bad sink.
        assert_identical(format!("{v}\n3\n").as_bytes());
    }
}

/// A NUL byte terminates the C string that `atoi` sees, even though `fgets`
/// copied it.
#[test]
fn embedded_nul_and_high_bytes() {
    assert_identical(b"\x005\n\x005\n");
    assert_identical(b"5\x009\n5\x009\n");
    assert_identical(b"\xff\xfe\n\xff\xfe\n");
    assert_identical(b"\x7f\n\x7f\n");
}

// ---------------------------------------------------------------------------
// Phase B: fgets() framing
// ---------------------------------------------------------------------------

/// `fgets(buf, 14, stdin)` takes at most 13 bytes and stops after a newline, so
/// a long line is split across the two call sites rather than being discarded.
#[test]
fn fgets_splits_long_lines_across_call_sites() {
    assert_identical(b"1234567890123\n");          // exactly 13, newline left over
    assert_identical(b"12345678901234\n");          // 14: 13 taken, "4\n" left
    assert_identical(b"0000000000005\n");           // 13 chars, value 5
    assert_identical(b"00000000000009\n");          // splits into "0000000000000" + "9\n"
    assert_identical(b"1\n2\n3\n4\n");              // extra lines simply ignored
    assert_identical(b"\n\n");                      // two empty lines -> atoi("") twice
    assert_identical(b"\n");                        // one empty line, then EOF
    assert_identical(b"\n\n\n\n\n");
}

/// No trailing newline anywhere.
#[test]
fn missing_trailing_newline() {
    assert_identical(b"4");
    assert_identical(b"0\n4");
    assert_identical(b"1234567890123");
}

// ---------------------------------------------------------------------------
// Phase C: the out-of-bounds write in bad(), slot by slot
// ---------------------------------------------------------------------------

/// `bad()` checks only `data >= 0`, so `buffer[data] = 1` on `int buffer[10]`
/// runs off the end. Indices 10..=15 land on dead locals and are absorbed;
/// 16..=19 hit `bad`'s saved frame pointer and return address, and 26..=27 hit
/// `main`'s return address, so the C dies of SIGSEGV having flushed nothing.
#[test]
fn oob_write_slot_by_slot() {
    for n in 10..=48 {
        assert_identical(&two_lines("0", n));
    }
}

/// The four indices that corrupt `bad`'s own saved %rbp / return address.
#[test]
fn oob_indices_corrupting_bad_return() {
    for n in 16..=19 {
        assert_identical(&two_lines("0", n));
        let c = run(&c_binary(), &two_lines("0", n));
        assert!(
            c.killed_by_signal(),
            "C should be killed by a signal at index {n}, got {c:?}"
        );
        assert!(
            c.stdout.is_empty(),
            "C's buffered stdout should be lost at index {n}, got {c:?}"
        );
    }
}

/// The two indices that corrupt `main`'s return address.
#[test]
fn oob_indices_corrupting_main_return() {
    for n in 26..=27 {
        assert_identical(&two_lines("0", n));
        let c = run(&c_binary(), &two_lines("0", n));
        assert!(
            c.killed_by_signal(),
            "C should be killed by a signal at index {n}, got {c:?}"
        );
        assert!(c.stdout.is_empty(), "C's stdout should be lost at index {n}");
    }
}

/// The indices next to the crashing ones must still exit 0 with the normal
/// all-zero dump: an off-by-one in the model would show up here.
#[test]
fn oob_indices_adjacent_to_crash_slots_are_absorbed() {
    for n in [10, 11, 12, 13, 14, 15, 20, 21, 22, 23, 24, 25, 28, 29, 30] {
        assert_identical(&two_lines("0", n));
        let c = run(&c_binary(), &two_lines("0", n));
        assert_eq!(c.code, Some(0), "index {n} should be absorbed, got {c:?}");
    }
}

/// A wide sweep of the absorbed region above the frame, where the store lands
/// in the argv/env block.
///
/// Strict comparison is used only as far up as the current environment provably
/// allows; past that the store is close enough to the top of the stack mapping
/// that the per-exec randomisation decides the outcome, and
/// `oob_mid_region_invariant` takes over.
#[test]
fn oob_absorbed_region_sweep() {
    // Always exercised: these offsets are far below the top of the stack
    // mapping in any environment.
    for n in [28, 30, 33, 37, 41, 44, 48] {
        assert_identical(&two_lines("0", n));
    }
    // Extended as far up as the current environment provably allows.
    let ceiling = always_absorbed_ceiling();
    if ceiling > 48 {
        for n in (49..=ceiling).step_by(37) {
            assert_identical(&two_lines("0", n));
        }
        assert_identical(&two_lines("0", ceiling));
    }
}

/// The region between the provably-absorbed indices and the provably-faulting
/// ones, where the outcome depends on the per-exec stack randomisation.
///
/// The *same* C binary exits 0 on one run and dies on the next here, so per-run
/// equality is not a property either program has. What is asserted is the
/// invariant that does hold, in both programs: a run either survives and prints
/// exactly the normal all-zero dump, or it dies from a signal having flushed
/// nothing. Neither ever writes to stderr. Documented in ERRORS.md.
#[test]
fn oob_mid_region_invariant() {
    // n=30 is absorbed in every environment, so its output is exactly what a
    // surviving run in this region prints.
    let reference = run(&c_binary(), &two_lines("0", 30)).stdout;
    assert!(!reference.is_empty(), "reference run should produce output");

    let mut n = 64i64;
    while n <= always_faults_floor() {
        let input = two_lines("0", n);
        for binary in [c_binary(), rust_binary()] {
            let o = run(&binary, &input);
            if o.killed_by_signal() {
                assert!(
                    o.stdout.is_empty(),
                    "a faulting run must flush nothing (n={n}, {binary:?}): {o:?}"
                );
            } else {
                assert_eq!(o.code, Some(0), "n={n} {binary:?}: {o:?}");
                assert_eq!(
                    o.stdout, reference,
                    "a surviving run must print the normal dump (n={n}, {binary:?})"
                );
            }
            assert!(
                o.stderr.is_empty(),
                "neither program writes to stderr (n={n}, {binary:?})"
            );
        }
        n = (n * 3) / 2;
    }
}

/// Far past the end of the stack mapping the store itself faults, before the
/// print loop runs. The lower limit is derived from the current environment;
/// the upper limit is bounded because beyond roughly 2e8 the wild address
/// starts landing in file-backed mappings and the C alternates between SIGSEGV
/// and SIGBUS from run to run (see ERRORS.md).
#[test]
fn oob_far_out_of_stack_always_faults() {
    let floor = always_faults_floor();
    for n in [
        floor,
        floor + 1,
        floor * 2,
        1_000_000,
        10_000_000,
        50_000_000,
        100_000_000,
    ] {
        assert_identical(&two_lines("0", n));
        let c = run(&c_binary(), &two_lines("0", n));
        assert!(
            c.killed_by_signal(),
            "C should fault at index {n}, got {c:?}"
        );
        assert!(
            c.stdout.is_empty(),
            "a faulting run flushes nothing (n={n}), got {c:?}"
        );
    }
}

/// An out-of-bounds index reached only through `int` truncation, not by typing
/// a large number directly.
#[test]
fn oob_index_reached_by_truncation() {
    // 4294967306 truncates to 10; 1234567890123 truncates to 1912239307.
    assert_identical(b"0\n4294967306\n");
    assert_identical(b"0\n4294967322\n"); // -> 26, a crashing slot
    assert_identical(b"0\n4294967314\n"); // -> 18, a crashing slot
    assert_identical(b"12345678901234567890\n");
}

// ---------------------------------------------------------------------------
// Phase C: dense sweep
// ---------------------------------------------------------------------------

/// Every index from 0 through 48 in the bad sink: the in-bounds range, the six
/// crashing slots and every absorbed slot in between. This is the test that
/// would catch a wrong slot in the frame model.
///
/// 48 is the highest index that is deterministic in *any* environment: it stores
/// 132 bytes above `bad`'s `%rbp`, while the smallest argv/env block seen (464
/// bytes, under `env -i`) still leaves 768 bytes of headroom.
#[test]
fn dense_index_sweep() {
    for n in 0..=48 {
        assert_identical(&two_lines("0", n));
    }
}

/// The same range through the good sink, where the `data < 10` check makes every
/// value safe, so this stays deterministic for arbitrarily large indices.
#[test]
fn dense_index_sweep_through_good_sink() {
    for n in 0..=600 {
        assert_identical(&two_lines(&n.to_string(), 0));
    }
}

// ---------------------------------------------------------------------------
// Phase C: the region where the C is not deterministic even against itself
// ---------------------------------------------------------------------------

/// Above roughly 2e8 the wild address sometimes lands in a file-backed mapping,
/// and the C alternates between SIGSEGV and SIGBUS across runs. Both programs
/// always die without output; the exact signal is not reproducible even C
/// against C. Documented in ERRORS.md.
#[test]
fn very_large_indices_always_die_without_output() {
    for n in [
        536_870_912i64,
        1_073_741_824,
        2_000_000_000,
        2_147_483_647,
    ] {
        let input = two_lines("0", n);
        for binary in [c_binary(), rust_binary()] {
            let o = run(&binary, &input);
            assert!(
                o.killed_by_signal(),
                "n={n} {binary:?} should die from a signal: {o:?}"
            );
            assert!(o.stdout.is_empty(), "n={n} {binary:?} should print nothing");
            assert!(o.stderr.is_empty(), "n={n} {binary:?} stderr should be empty");
        }
    }
}

// ---------------------------------------------------------------------------
// Output shape
// ---------------------------------------------------------------------------

/// Pins the exact bytes for a benign run, so a formatting drift (spacing,
/// trailing newline, message wording) is caught directly and not only by
/// comparison.
#[test]
fn exact_output_bytes_for_a_benign_run() {
    let expected: &[u8] = b"Calling good()...\n\
        0\n0\n0\n0\n0\n0\n0\n1\n0\n0\n\
        0\n0\n0\n1\n0\n0\n0\n0\n0\n0\n\
        Finished good()\n\
        Calling bad()...\n\
        0\n0\n0\n0\n0\n1\n0\n0\n0\n0\n\
        Finished bad()\n";
    let input = b"3\n5\n";
    assert_eq!(run(&c_binary(), input).stdout, expected, "C output drifted");
    assert_eq!(run(&rust_binary(), input).stdout, expected);
}

/// Neither program ever writes to stderr on a successful run.
#[test]
fn stderr_is_empty_on_success() {
    for input in [&b""[..], b"0\n", b"3\n5\n", b"9\n9\n", b"-1\n-1\n"] {
        assert!(run(&c_binary(), input).stderr.is_empty());
        assert!(run(&rust_binary(), input).stderr.is_empty());
    }
}
