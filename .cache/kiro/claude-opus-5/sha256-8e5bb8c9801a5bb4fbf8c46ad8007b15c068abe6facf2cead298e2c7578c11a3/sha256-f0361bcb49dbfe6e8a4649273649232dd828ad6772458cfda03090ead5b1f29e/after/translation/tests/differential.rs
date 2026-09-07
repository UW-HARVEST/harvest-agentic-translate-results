//! Differential tests: C `libdriver.so` vs Rust `libdriver.so`.
//!
//! Both libraries are loaded through `libloading` and every function is invoked
//! through a `dlsym`-resolved symbol, so the `#[no_mangle] extern "C"` export
//! wrappers are under test too. No Rust function is ever called directly.
//!
//! Each check runs the `harness` example twice as a subprocess — once per `.so` —
//! with an identical op script, then compares the raw stdout bytes, the exit
//! code and the terminating signal. A subprocess is required because the library
//! under test dereferences unchecked pointers and can die of `SIGSEGV`, and
//! because the only observable output is `printf` to fd 1.
//!
//! `LD_BIND_NOW=1` is set for every comparison. Rationale, with measurements, is
//! in [`lazy_binding_first_call_is_not_reproducible_even_c_vs_c`].

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Locating the artifacts
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The C shared library. Built by
/// `cd c_src && mkdir -p build && cd build && cmake .. \
///  -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`
fn c_lib() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let p = manifest_dir()
            .parent()
            .unwrap()
            .join("c_src/build/libdriver.so");
        assert!(
            p.is_file(),
            "C shared library not found at {p:?}.\nBuild it with:\n  cd c_src && mkdir -p build && \
             cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
        );
        p
    })
}

/// The Rust shared library, built as a `cdylib` in release mode.
fn rust_lib() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let p = manifest_dir().join("target/release/libdriver.so");
        if !p.is_file() {
            let st = Command::new(env!("CARGO"))
                .args(["build", "--release"])
                .current_dir(manifest_dir())
                .status()
                .expect("spawn cargo build --release");
            assert!(st.success(), "cargo build --release failed");
        }
        assert!(p.is_file(), "Rust cdylib not found at {p:?}");
        p
    })
}

/// The subprocess harness (`examples/harness.rs`).
fn harness() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        // The test binary lives in <target>/<profile>/deps/, so the example
        // sits in <target>/<profile>/examples/.
        let exe = std::env::current_exe().expect("current_exe");
        let profile_dir = exe.parent().unwrap().parent().unwrap().to_path_buf();
        let p = profile_dir.join("examples/harness");
        if !p.is_file() {
            let st = Command::new(env!("CARGO"))
                .args(["build", "--example", "harness"])
                .current_dir(manifest_dir())
                .status()
                .expect("spawn cargo build --example harness");
            assert!(st.success(), "cargo build --example harness failed");
        }
        assert!(p.is_file(), "harness example not found at {p:?}");
        p
    })
}

// ---------------------------------------------------------------------------
// Running one side
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq)]
struct Run {
    stdout: Vec<u8>,
    code: Option<i32>,
    signal: Option<i32>,
}

impl std::fmt::Debug for Run {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Run {{ stdout: {:?}, code: {:?}, signal: {:?} }}",
            String::from_utf8_lossy(&self.stdout),
            self.code,
            self.signal
        )
    }
}

fn run_with_env(lib: &Path, ops: &[String], env: &[(&str, &str)]) -> Run {
    use std::os::unix::process::ExitStatusExt;
    let mut cmd = Command::new(harness());
    cmd.arg(lib);
    cmd.args(ops);
    // A clean, fixed environment: the harness's own stack/heap state must depend
    // only on the op script, not on the caller's environment block.
    cmd.env_clear();
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd.output().expect("spawn harness");
    Run {
        stdout: out.stdout,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

/// Deterministic run: PLT entries are bound eagerly so the lazy resolver never
/// executes between a caller's frame and `bad()`'s uninitialized read.
fn run(lib: &Path, ops: &[String]) -> Run {
    run_with_env(lib, ops, &[("LD_BIND_NOW", "1")])
}

/// Run with the loader's default lazy PLT binding. `LD_BIND_NOW` must be *absent*,
/// not set to `"0"` — glibc treats any non-empty value as a request for eager
/// binding.
fn run_lazy(lib: &Path, ops: &[String]) -> Run {
    run_with_env(lib, ops, &[])
}

/// The core assertion: identical op script, both `.so`s, byte-identical result.
#[track_caller]
fn assert_match(label: &str, ops: &[String]) {
    let c = run(c_lib(), ops);
    let r = run(rust_lib(), ops);
    assert_eq!(
        c,
        r,
        "\n{label}\n  ops  = {ops:?}\n  C    = {c:?}\n  Rust = {r:?}\n"
    );
    // A test that compares two crashes with no output would pass vacuously for
    // any pair of libraries; make sure each row actually observed something.
    assert!(
        !c.stdout.is_empty() || c.signal.is_some(),
        "{label}: neither output nor a signal was observed — test is vacuous"
    );
}

fn ops(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

/// Parse a run's stdout as one decimal integer per line.
#[track_caller]
fn lines_as_i64(r: &Run) -> Vec<i64> {
    String::from_utf8_lossy(&r.stdout)
        .lines()
        .map(|l| {
            l.parse::<i64>()
                .unwrap_or_else(|e| panic!("not an integer line {l:?}: {e}"))
        })
        .collect()
}

/// The poison slot index that a *directly called* `bad()` reads, measured from
/// the C library.
///
/// The absolute index depends on the harness's own frame size at the call site
/// (it is 2 when the harness is built as a debug binary and 1 when built as a
/// release binary), so it is measured rather than hard-coded. What is *not*
/// harness-dependent, and is what the C's frame geometry actually pins down, is
/// the offset between this and the index reached via `driver` — see
/// [`DRIVER_FRAME_SLOTS`].
fn direct_bad_slot() -> i64 {
    static V: OnceLock<i64> = OnceLock::new();
    *V.get_or_init(|| {
        let r = run(c_lib(), &ops(&["pbad"]));
        assert_eq!(r.signal, None, "measuring direct bad() slot: C faulted: {r:?}");
        let v = lines_as_i64(&r);
        assert_eq!(v.len(), 1);
        assert!(
            (0..8).contains(&v[0]),
            "direct bad() should read a slot just below the caller's rsp, got {}",
            v[0]
        );
        v[0]
    })
}

/// Reaching `bad()` through `driver()` inserts 32 bytes = 4 poison slots between
/// the caller's `rsp` and `bad()`'s uninitialized read: `call driver` (8) +
/// driver's `push rbp` (8) + driver's `sub rsp,0x10` (16). This is a direct
/// consequence of GCC's `-O0` frame for `driver` plus a real `call bad`, and it is
/// the exact quantity an optimized tail-jumping `driver` gets wrong.
const DRIVER_FRAME_SLOTS: i64 = 4;

// ---------------------------------------------------------------------------
// Seeded RNG (xorshift64*) — reproducible across runs and machines.
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn next_i32(&mut self) -> i32 {
        (self.next_u64() >> 32) as u32 as i32
    }
    fn next_nonzero_i32(&mut self) -> i32 {
        loop {
            let v = self.next_i32();
            if v != 0 {
                return v;
            }
        }
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}

// ===========================================================================
// PHASE B — valid-path differential tests, one per CONFIGS.md row
// ===========================================================================

/// CONFIGS row 1 — `printIntPtrLine` on a stack `int`, 512 seeded random values.
#[test]
fn cfg_01_print_stack_random() {
    let mut rng = Rng::new(0x1111_2222_3333_4444);
    let script: Vec<String> = (0..512).map(|_| format!("p:{}", rng.next_i32())).collect();
    assert_match("cfg_01 printIntPtrLine / stack / 512 random", &script);
}

/// CONFIGS row 2 — value boundaries at the `printf("%d")` sink.
#[test]
fn cfg_02_print_stack_boundaries() {
    let script: Vec<String> = [
        0i32,
        1,
        -1,
        i32::MAX,
        i32::MIN,
        i32::MIN + 1,
        i32::MAX - 1,
        10,
        -10,
        100000,
    ]
    .iter()
    .map(|v| format!("p:{v}"))
    .collect();
    assert_match("cfg_02 printIntPtrLine / stack / boundaries", &script);
}

/// CONFIGS row 3 — heap-allocated `int`, 256 seeded random values.
#[test]
fn cfg_03_print_heap_random() {
    let mut rng = Rng::new(0xAAAA_BBBB_CCCC_DDDD);
    let script: Vec<String> = (0..256).map(|_| format!("ph:{}", rng.next_i32())).collect();
    assert_match("cfg_03 printIntPtrLine / heap / 256 random", &script);
}

/// CONFIGS row 4 — static / `.data` `int`, 256 seeded random values.
#[test]
fn cfg_04_print_static_random() {
    let mut rng = Rng::new(0x0F0F_1E1E_2D2D_3C3C);
    let script: Vec<String> = (0..256).map(|_| format!("ps:{}", rng.next_i32())).collect();
    assert_match("cfg_04 printIntPtrLine / static / 256 random", &script);
}

/// CONFIGS row 5 — misaligned pointer into a mapped byte buffer. x86-64 tolerates
/// this and the C emits a plain `mov (%rax),%eax`, so it must not be rejected.
#[test]
fn cfg_05_print_misaligned_random() {
    let mut rng = Rng::new(0x5EED_0000_0BAD_F00D);
    for off in 1..=3usize {
        let script: Vec<String> = (0..256)
            .map(|_| format!("pm:{off}:{:08x}", rng.next_u64() as u32))
            .collect();
        assert_match(
            &format!("cfg_05 printIntPtrLine / misaligned off={off} / 256 random"),
            &script,
        );
    }
    // Aligned controls.
    for off in [0usize, 4] {
        let script: Vec<String> = (0..64)
            .map(|_| format!("pm:{off}:{:08x}", rng.next_u64() as u32))
            .collect();
        assert_match(
            &format!("cfg_05 printIntPtrLine / aligned off={off}"),
            &script,
        );
    }
}

/// CONFIGS row 6 — N calls in one process; verifies the buffered stdout byte
/// order over many calls, which is this project's analogue of comparing two
/// driver binaries' stdout (it builds none).
#[test]
fn cfg_06_print_repeated_counts() {
    let mut rng = Rng::new(0xDEAD_BEEF_CAFE_0001);
    for n in [1usize, 2, 17, 1000] {
        let script: Vec<String> = (0..n).map(|_| format!("p:{}", rng.next_i32())).collect();
        assert_match(&format!("cfg_06 printIntPtrLine x{n}"), &script);
    }
}

/// CONFIGS row 7 — the lower-level `good` entry point, called directly rather
/// than through the `driver` convenience wrapper.
#[test]
fn cfg_07_good_direct() {
    let r = run(c_lib(), &ops(&["good"]));
    assert_eq!(r.stdout, b"5\n", "C good() must print 5");
    assert_match("cfg_07 good() direct", &ops(&["good"]));
}

/// CONFIGS row 8 — `good` repeated; there is no hidden state.
#[test]
fn cfg_08_good_repeated() {
    for n in [1usize, 2, 64] {
        let script: Vec<String> = (0..n).map(|_| "good".to_string()).collect();
        assert_match(&format!("cfg_08 good() x{n}"), &script);
    }
}

/// CONFIGS row 9 — `driver(1)`, the documented value.
#[test]
fn cfg_09_driver_one() {
    let r = run(c_lib(), &ops(&["d:1"]));
    assert_eq!(r.stdout, b"5\n", "C driver(1) must print 5");
    assert_match("cfg_09 driver(1)", &ops(&["d:1"]));
}

/// CONFIGS row 10 — 256 seeded random non-zero `useGood` values, including
/// negatives. All must take the `good` arm.
#[test]
fn cfg_10_driver_random_nonzero() {
    let mut rng = Rng::new(0x1357_9BDF_0246_8ACE);
    let script: Vec<String> = (0..256)
        .map(|_| format!("d:{}", rng.next_nonzero_i32()))
        .collect();
    assert_match("cfg_10 driver / 256 random non-zero", &script);
    let r = run(c_lib(), &script);
    assert_eq!(
        r.stdout,
        b"5\n".repeat(256),
        "every non-zero useGood must take the good() arm"
    );
}

/// CONFIGS row 11 — non-zero `useGood` boundaries.
#[test]
fn cfg_11_driver_nonzero_boundaries() {
    let script: Vec<String> = [1i32, -1, 2, i32::MAX, i32::MIN, i32::MIN + 1, i32::MAX - 1]
        .iter()
        .map(|v| format!("d:{v}"))
        .collect();
    assert_match("cfg_11 driver / non-zero boundaries", &script);
}

/// CONFIGS row 12 — `driver(0)` as the first library call in the process, routing
/// into `bad()`. The stack is poisoned with an index table first (see the harness
/// docs) so the uninitialized read becomes a deterministic observation of *which*
/// stack slot each library reads.
#[test]
fn cfg_12_driver_zero_first_call() {
    let script = ops(&["pd:0"]);
    assert_match("cfg_12 driver(0) first call, poisoned stack", &script);
    // The C `-O0` frame for driver puts bad's `-0x8(%rbp)` exactly
    // DRIVER_FRAME_SLOTS slots below where a directly-called bad() reads.
    let r = run(c_lib(), &script);
    assert_eq!(
        lines_as_i64(&r),
        vec![direct_bad_slot() + DRIVER_FRAME_SLOTS],
        "C driver(0)->bad() must read {} slots past the direct-call slot {}",
        DRIVER_FRAME_SLOTS,
        direct_bad_slot()
    );
}

/// CONFIGS row 13 — `driver(0)` after `driver(1)`: different stack residue.
#[test]
fn cfg_13_driver_zero_after_one() {
    assert_match("cfg_13 driver(1) then driver(0)", &ops(&["pd:1", "pd:0"]));
    assert_match(
        "cfg_13 driver(1) then driver(0), unpoisoned",
        &ops(&["d:1", "d:0"]),
    );
}

/// CONFIGS row 14 — alternating `driver(1)` / `driver(0)` eight times in one
/// process, comparing the whole stdout byte stream.
#[test]
fn cfg_14_driver_alternating() {
    let mut script = Vec::new();
    for _ in 0..8 {
        script.push("pd:1".to_string());
        script.push("pd:0".to_string());
    }
    assert_match("cfg_14 alternating driver(1)/driver(0) x8", &script);

    let mut script = Vec::new();
    for _ in 0..8 {
        script.push("d:1".to_string());
        script.push("d:0".to_string());
    }
    assert_match("cfg_14 alternating, unpoisoned", &script);
}

/// CONFIGS row 15 — `bad` as the very first library call, with no `driver` frame
/// above it. Poisoned, so the read is deterministic; the slot index it lands on
/// is measured by [`direct_bad_slot`] and reused by rows 12 and 20.
#[test]
fn cfg_15_bad_direct_first_call() {
    let script = ops(&["pbad"]);
    assert_match("cfg_15 bad() direct, first call", &script);
    let c = run(c_lib(), &script);
    let r = run(rust_lib(), &script);
    assert_eq!(
        lines_as_i64(&c),
        vec![direct_bad_slot()],
        "C bad() must read the poisoned slot deterministically"
    );
    assert_eq!(
        lines_as_i64(&r),
        vec![direct_bad_slot()],
        "Rust bad() must read the same slot as C"
    );
}

/// CONFIGS row 16 — `bad` twice in a row.
#[test]
fn cfg_16_bad_twice() {
    assert_match("cfg_16 bad() x2 poisoned", &ops(&["pbad", "pbad"]));
    assert_match("cfg_16 bad() x2 unpoisoned", &ops(&["bad", "bad"]));
}

/// CONFIGS row 17 — `bad` after `good`: residue from `good` + `printIntPtrLine`
/// + `printf`.
#[test]
fn cfg_17_bad_after_good() {
    assert_match("cfg_17 good() then bad()", &ops(&["good", "bad"]));
    assert_match(
        "cfg_17 good() x3 then bad()",
        &ops(&["good", "good", "good", "bad"]),
    );
}

/// CONFIGS row 18 — `bad` after `printIntPtrLine`.
#[test]
fn cfg_18_bad_after_print() {
    assert_match(
        "cfg_18 printIntPtrLine then bad()",
        &ops(&["p:12345", "bad"]),
    );
    assert_match(
        "cfg_18 printIntPtrLine x4 then bad()",
        &ops(&["p:1", "p:-2", "p:3", "p:-4", "bad"]),
    );
}

/// CONFIGS row 19 — `bad` after `driver(1)`.
#[test]
fn cfg_19_bad_after_driver_one() {
    assert_match("cfg_19 driver(1) then bad()", &ops(&["d:1", "bad"]));
}

/// CONFIGS row 20 — `bad` reached directly vs through `driver`, in the same
/// process. Confirms the extra `driver` frame shifts the uninitialized read by
/// the same amount in C and Rust. This is the exact check the original
/// translation failed: an optimized Rust `driver` tail-jumped (`jmp bad`) instead
/// of building a frame and issuing `call bad`, so its `bad()` read the
/// *direct-call* slot in both cases and the shift was 0 instead of
/// [`DRIVER_FRAME_SLOTS`].
#[test]
fn cfg_20_bad_direct_vs_via_driver() {
    let script = ops(&["pbad", "pd:0", "pbad", "pd:0"]);
    assert_match("cfg_20 bad() direct vs via driver(0)", &script);
    let d = direct_bad_slot();
    let expected = vec![d, d + DRIVER_FRAME_SLOTS, d, d + DRIVER_FRAME_SLOTS];
    assert_eq!(
        lines_as_i64(&run(c_lib(), &script)),
        expected,
        "the driver frame must shift bad()'s read by {DRIVER_FRAME_SLOTS} slots"
    );
    assert_eq!(
        lines_as_i64(&run(rust_lib(), &script)),
        expected,
        "Rust must shift identically"
    );
}

/// CONFIGS row 21 — the composed pipeline: 32 seeded random operations drawn
/// from the full public API, replayed identically against both `.so`s, for 40
/// independent seeds.
#[test]
fn cfg_21_mixed_pipeline_random() {
    for seed in 0..40u64 {
        let mut rng = Rng::new(0xF00D_0000_0000_0000 ^ seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let script: Vec<String> = (0..32)
            .map(|_| match rng.below(7) {
                0 => format!("p:{}", rng.next_i32()),
                1 => format!("ph:{}", rng.next_i32()),
                2 => format!("ps:{}", rng.next_i32()),
                3 => "good".to_string(),
                4 => format!("pd:{}", rng.next_nonzero_i32()),
                5 => "pd:0".to_string(),
                _ => "pbad".to_string(),
            })
            .collect();
        assert_match(&format!("cfg_21 mixed pipeline, seed {seed}"), &script);
    }
}

/// CONFIGS row 22 — the same pipelines with stdout line-buffered instead of
/// fully buffered, comparing the byte stream.
#[test]
fn cfg_22_stdout_buffering_modes() {
    for seed in 0..12u64 {
        let mut rng = Rng::new(0xB0FF_0000_0000_0001 ^ seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let mut script = vec!["linebuf".to_string()];
        for _ in 0..24 {
            script.push(match rng.below(5) {
                0 => format!("p:{}", rng.next_i32()),
                1 => "good".to_string(),
                2 => format!("pd:{}", rng.next_nonzero_i32()),
                3 => "pd:0".to_string(),
                _ => "flush".to_string(),
            });
        }
        assert_match(
            &format!("cfg_22 line-buffered pipeline, seed {seed}"),
            &script,
        );
    }
}

// ===========================================================================
// PHASE C — error-path differential tests, one per ERRORS.md row
// ===========================================================================

/// Assert that both libraries die from the *same* signal with the *same* stdout.
/// Comparing "both failed somehow" would be vacuous, so the signal number and
/// the exit code are compared individually as well as through `assert_match`.
#[track_caller]
fn assert_same_fault(label: &str, script: &[String], expect_signal: i32) {
    let c = run(c_lib(), script);
    let r = run(rust_lib(), script);
    assert_eq!(
        c.signal,
        Some(expect_signal),
        "{label}: C should die of signal {expect_signal}, got {c:?}"
    );
    assert_eq!(
        r.signal,
        Some(expect_signal),
        "{label}: Rust should die of signal {expect_signal}, got {r:?}"
    );
    assert_eq!(c.code, r.code, "{label}: exit code differs");
    assert_eq!(
        c.stdout, r.stdout,
        "{label}: stdout before the fault differs"
    );
}

const SIGSEGV: i32 = 11;

/// ERRORS row 1 — `printIntPtrLine(NULL)`. There is no null check in the C, so
/// `*intNumber` reads address 0.
#[test]
fn err_01_print_null_ptr() {
    assert_same_fault("err_01 printIntPtrLine(NULL)", &ops(&["pnull"]), SIGSEGV);
    // Also with prior output, to confirm the pre-fault stdout matches too.
    assert_same_fault(
        "err_01 printIntPtrLine(NULL) after output",
        &ops(&["p:1", "good", "flush", "pnull"]),
        SIGSEGV,
    );
    // And with `praw:0`, i.e. a null arriving as a raw integer across the FFI
    // boundary rather than as a Rust `null()`.
    assert_same_fault(
        "err_01 printIntPtrLine((int*)0)",
        &ops(&["praw:0"]),
        SIGSEGV,
    );
}

/// ERRORS row 2 — non-null but unmapped low address, also misaligned.
#[test]
fn err_02_print_unmapped_low() {
    for addr in ["1", "2", "3", "4", "8", "ff", "fff"] {
        assert_same_fault(
            &format!("err_02 printIntPtrLine((int*)0x{addr})"),
            &ops(&[&format!("praw:{addr}")]),
            SIGSEGV,
        );
    }
}

/// ERRORS row 3 — large unmapped canonical address.
#[test]
fn err_03_print_unmapped_high() {
    for addr in ["7fffffff0000", "7ffffffff000", "600000000000"] {
        assert_same_fault(
            &format!("err_03 printIntPtrLine((int*)0x{addr})"),
            &ops(&[&format!("praw:{addr}")]),
            SIGSEGV,
        );
    }
}

/// ERRORS row 4 — non-canonical / kernel-space address.
#[test]
fn err_04_print_noncanonical() {
    for addr in ["fffffffffffffff0", "ffffffffffffffff", "8000000000000000"] {
        assert_same_fault(
            &format!("err_04 printIntPtrLine((int*)0x{addr})"),
            &ops(&[&format!("praw:{addr}")]),
            SIGSEGV,
        );
    }
}

/// ERRORS row 5 — misaligned but mapped. The C does *not* reject this; a Rust
/// translation using an alignment-requiring read would. Must be accepted and
/// must print the unaligned little-endian `int`.
#[test]
fn err_05_print_misaligned() {
    let script = ops(&[
        "pm:1:deadbeef",
        "pm:2:deadbeef",
        "pm:3:deadbeef",
        "pm:1:80000000",
        "pm:3:ffffffff",
    ]);
    assert_match("err_05 misaligned reads are accepted", &script);
    let c = run(c_lib(), &script);
    assert_eq!(c.signal, None, "C must not fault on a misaligned read");
    assert_eq!(c.code, Some(0));
    assert_eq!(
        c.stdout, b"-559038737\n-559038737\n-559038737\n-2147483648\n-1\n",
        "C must print the unaligned little-endian int"
    );
}

/// ERRORS row 6 — one-past-the-end of a heap allocation: dangling, but in
/// practice still mapped, so the C does not fault.
#[test]
fn err_06_print_one_past_end() {
    assert_match("err_06 one-past-end heap read", &ops(&["ppast"]));
    let c = run(c_lib(), &ops(&["ppast"]));
    assert_eq!(c.signal, None, "C must not fault reading one past the end");
}

/// ERRORS row 7 — `bad()`'s uninitialized read, the library's defining defect.
/// Compared both with a controlled (poisoned) stack, where the read is
/// deterministic and identifies the exact slot, and with the natural stack.
#[test]
fn err_07_bad_uninit_read() {
    assert_match("err_07 bad() poisoned", &ops(&["pbad"]));
    assert_match("err_07 bad() natural", &ops(&["bad"]));
    let c = run(c_lib(), &ops(&["pbad"]));
    assert_eq!(c.code, Some(0), "C bad() exits 0 on a readable stale slot");
    assert_eq!(c.signal, None);
}

/// ERRORS row 8 — `driver(0)`, the false arm that routes into row 7's UB.
#[test]
fn err_08_driver_zero() {
    assert_match("err_08 driver(0) poisoned", &ops(&["pd:0"]));
    assert_match("err_08 driver(0) natural", &ops(&["d:0"]));
}

/// ERRORS row 9 — `useGood == INT_MIN`, one step past the low end of `int`.
#[test]
fn err_09_driver_int_min() {
    let script = ops(&["d:-2147483648"]);
    assert_match("err_09 driver(INT_MIN)", &script);
    assert_eq!(run(c_lib(), &script).stdout, b"5\n");
}

/// ERRORS row 10 — `useGood == INT_MAX`.
#[test]
fn err_10_driver_int_max() {
    let script = ops(&["d:2147483647"]);
    assert_match("err_10 driver(INT_MAX)", &script);
    assert_eq!(run(c_lib(), &script).stdout, b"5\n");
}

/// ERRORS row 11 — "out-of-range enum" values. `useGood` is an `int`, and C
/// enums accept any `int`, so a value with no valid variant is a real input.
/// Every non-zero value must take the `good` arm and never be rejected.
#[test]
fn err_11_driver_out_of_range_enumlike() {
    let mut rng = Rng::new(0xE1E1_E1E1_2020_2020);
    let mut script: Vec<String> = [-1i32, 2, 3, 4, 255, 256, 65535, 65536, i32::MIN, i32::MAX]
        .iter()
        .map(|v| format!("d:{v}"))
        .collect();
    for _ in 0..128 {
        script.push(format!("d:{}", rng.next_nonzero_i32()));
    }
    let n = script.len();
    assert_match("err_11 out-of-range enum-like useGood", &script);
    let c = run(c_lib(), &script);
    assert_eq!(
        c.stdout,
        b"5\n".repeat(n),
        "no non-zero useGood is ever rejected"
    );
    assert_eq!(c.code, Some(0));
}

/// ERRORS row 12 — a 64-bit argument whose low 32 bits are zero. The C spills and
/// tests `edi`, so the high half is discarded and the value counts as zero,
/// taking the `bad` arm. A translation that tested the full 64-bit register would
/// take the `good` arm and print `5`.
#[test]
fn err_12_driver_high_half_only() {
    for raw in ["100000000", "ffffffff00000000", "dead000000000000"] {
        let script = ops(&[&format!("draw:{raw}")]);
        assert_match(&format!("err_12 driver(0x{raw} in rdi)"), &script);
        let c = run(c_lib(), &script);
        assert_ne!(
            c.stdout, b"5\n",
            "0x{raw} truncates to 0 in edi, so C must take the bad() arm"
        );
    }
    // Control: a raw value whose low half is non-zero takes the good arm.
    for raw in ["1", "ffffffff00000001", "dead0000deadbeef"] {
        let script = ops(&[&format!("draw:{raw}")]);
        assert_match(&format!("err_12 control driver(0x{raw} in rdi)"), &script);
        assert_eq!(run(c_lib(), &script).stdout, b"5\n");
    }
}

/// ERRORS row 13 — `INT_MIN` through the `%d` conversion, the value whose
/// negation overflows.
#[test]
fn err_13_print_int_min() {
    let script = ops(&["p:-2147483648", "ph:-2147483648", "ps:-2147483648"]);
    assert_match("err_13 printIntPtrLine(INT_MIN)", &script);
    assert_eq!(
        run(c_lib(), &script).stdout,
        b"-2147483648\n-2147483648\n-2147483648\n"
    );
}

/// ERRORS row 14 — no initialization is required and no entry point rejects a
/// repeat call. Each of the four entry points is invoked first-in-process.
#[test]
fn err_14_no_init_required() {
    for first in [
        vec!["p:42"],
        vec!["good"],
        vec!["pbad"],
        vec!["pd:1"],
        vec!["pd:0"],
    ] {
        let mut script: Vec<String> = first.iter().map(|s| s.to_string()).collect();
        script.extend(script.clone());
        script.extend(script.clone());
        assert_match(&format!("err_14 {first:?} repeated"), &script);
    }
}

// ===========================================================================
// PHASE D — symbol parity and configuration coverage
// ===========================================================================

/// Every dynamic symbol the C `.so` defines must be defined by the Rust `.so`
/// under the exact same name. Reads `nm -D --defined-only` on both.
#[test]
fn sym_01_defined_symbol_parity() {
    fn defined(lib: &Path) -> Vec<String> {
        let out = Command::new("nm")
            .args(["-D", "--defined-only"])
            .arg(lib)
            .output()
            .expect("run nm -D --defined-only");
        assert!(
            out.status.success(),
            "nm failed on {lib:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.split_whitespace().nth(2).map(str::to_string))
            .collect();
        v.sort();
        v.dedup();
        v
    }

    let c = defined(c_lib());
    let r = defined(rust_lib());
    assert_eq!(
        c,
        vec![
            "bad".to_string(),
            "driver".to_string(),
            "good".to_string(),
            "printIntPtrLine".to_string(),
        ],
        "the C .so's exported set changed; SYMBOLS.md needs regenerating"
    );
    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}"
    );
}

/// Every symbol must also be resolvable through `dlsym`, not merely present in
/// the symbol table. This is what the harness relies on, and it is what an
/// external consumer does.
#[test]
fn sym_02_all_symbols_resolvable_via_dlsym() {
    for lib in [c_lib(), rust_lib()] {
        let l = unsafe { libloading::Library::new(lib) }.expect("dlopen");
        for name in [
            b"printIntPtrLine\0".as_slice(),
            b"bad\0",
            b"good\0",
            b"driver\0",
        ] {
            let sym = unsafe { l.get::<unsafe extern "C" fn()>(name) };
            assert!(
                sym.is_ok(),
                "dlsym({:?}) failed in {lib:?}",
                String::from_utf8_lossy(name)
            );
        }
    }
}

/// Documents why every comparison sets `LD_BIND_NOW=1`, and verifies the claim
/// rather than asserting it.
///
/// The C library is linked with partial RELRO and *lazy* PLT binding (`readelf -d`
/// shows a `GNU_RELRO` segment but no `BIND_NOW`), so the first `call bad@plt`
/// inside `driver` detours through `_dl_runtime_resolve`. The resolver consumes a
/// large amount of stack below the caller's frame and leaves ASLR-dependent
/// addresses there — exactly the bytes `bad()` then reads. The result is that the
/// *first* lazily-bound call's printed value is not reproducible even when the
/// C library is compared against itself, so no translation can be required to
/// match it.
///
/// This test asserts (a) that C-vs-C is itself unstable there, and (b) that from
/// the second call on, where no resolver runs, C and Rust agree exactly.
#[test]
fn lazy_binding_first_call_is_not_reproducible_even_c_vs_c() {
    let script = ops(&["pd:0", "pd:0", "pd:0", "pd:0"]);
    // The slot a resolver-free driver(0)->bad() settles on.
    let stable = (direct_bad_slot() + DRIVER_FRAME_SLOTS).to_string();

    let mut first_values = std::collections::HashSet::new();
    for _ in 0..12 {
        let out = run_lazy(c_lib(), &script);
        let text = String::from_utf8_lossy(&out.stdout).into_owned();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 4, "expected four lines, got {text:?}");
        first_values.insert(lines[0].to_string());
        // Calls 2..4 no longer run the resolver and settle on the true slot.
        assert_eq!(
            &lines[1..],
            [stable.as_str(), stable.as_str(), stable.as_str()],
            "post-resolver calls should read poison slot {stable}"
        );
    }
    assert!(
        first_values.len() > 1,
        "the first lazily-bound call was stable across 12 runs of the *same* C \
         library ({first_values:?}); if the loader has become deterministic here, \
         drop LD_BIND_NOW=1 and compare this case directly"
    );

    // The Rust library must exhibit the *same* nondeterminism, because it must be
    // linked with the same lazy binding. If it were linked `-z now` (rustc's
    // default) its first call would be stable while C's is not — a real
    // observable difference that no amount of value comparison can paper over.
    let mut rust_first_values = std::collections::HashSet::new();
    for _ in 0..12 {
        let out = run_lazy(rust_lib(), &script);
        let text = String::from_utf8_lossy(&out.stdout).into_owned();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 4, "expected four lines, got {text:?}");
        rust_first_values.insert(lines[0].to_string());
        assert_eq!(
            &lines[1..],
            [stable.as_str(), stable.as_str(), stable.as_str()],
            "post-resolver calls should read poison slot {stable}"
        );
    }
    assert!(
        rust_first_values.len() > 1,
        "the Rust library's first driver(0)->bad() call is stable ({rust_first_values:?}) \
         while the C library's is not ({first_values:?}). The Rust cdylib is eagerly \
         bound and the C .so is not; see build.rs"
    );

    // And the stable tail agrees between C and Rust under lazy binding too.
    let c = run_lazy(c_lib(), &script);
    let r = run_lazy(rust_lib(), &script);
    let ct = String::from_utf8_lossy(&c.stdout).into_owned();
    let rt = String::from_utf8_lossy(&r.stdout).into_owned();
    assert_eq!(
        ct.lines().skip(1).collect::<Vec<_>>(),
        rt.lines().skip(1).collect::<Vec<_>>(),
        "under lazy binding, C and Rust must agree from the second call on"
    );
    assert_eq!(c.code, r.code);
    assert_eq!(c.signal, r.signal);
}

/// The Rust `cdylib` must keep the C library's link-time binding mode, since it
/// is observable through `bad()`. Guards against a future change that reinstates
/// rustc's default `-z now`.
#[test]
fn link_mode_matches_c() {
    fn has_bind_now(lib: &Path) -> bool {
        let out = Command::new("readelf")
            .arg("-d")
            .arg(lib)
            .output()
            .expect("run readelf -d");
        let text = String::from_utf8_lossy(&out.stdout);
        text.contains("BIND_NOW") || text.contains("Flags: NOW")
    }
    assert!(
        !has_bind_now(c_lib()),
        "the C .so is now BIND_NOW; build.rs's -z lazy override must be removed"
    );
    assert!(
        !has_bind_now(rust_lib()),
        "the Rust .so is BIND_NOW but the C .so is lazily bound; see build.rs"
    );
}

/// The Rust `.so` must declare the same dynamic dependencies as the C `.so`.
///
/// This is not cosmetic. The loader's `_dl_runtime_resolve` walks the dependency
/// list and symbol tables on the first lazily-bound call, and its stack
/// consumption is exactly what `bad()` reads afterwards. Linking the Rust
/// standard library added `libgcc_s.so.1` and `ld-linux-x86-64.so.2` alongside
/// `libc.so.6` and grew `.dynsym` from 43 to 1003 entries, which changed that
/// residue enough to turn a clean exit into a `SIGSEGV` — see
/// [`lazy_binding_fault_behaviour_matches`].
#[test]
fn elf_02_dependency_parity() {
    fn needed(lib: &Path) -> Vec<String> {
        let out = Command::new("readelf")
            .arg("-d")
            .arg(lib)
            .output()
            .expect("run readelf -d");
        let text = String::from_utf8_lossy(&out.stdout);
        let mut v: Vec<String> = text
            .lines()
            .filter(|l| l.contains("(NEEDED)"))
            .filter_map(|l| {
                let s = l.find('[')?;
                let e = l.find(']')?;
                Some(l[s + 1..e].to_string())
            })
            .collect();
        v.sort();
        v
    }
    let c = needed(c_lib());
    let r = needed(rust_lib());
    assert_eq!(c, vec!["libc.so.6".to_string()], "unexpected C dependencies");
    assert_eq!(
        c, r,
        "the Rust .so's NEEDED list must match the C .so's; extra dependencies \
         change the lazy-binding stack residue that bad() reads"
    );
}

/// `.dynsym` size must stay in the same ballpark as the C library's, for the same
/// reason as [`elf_02_dependency_parity`]. The C `.so` has 43 entries; a
/// std-linked Rust `cdylib` had 1003. The bound is deliberately loose — this
/// guards against pulling in a runtime, not against a one-symbol drift.
#[test]
fn elf_03_dynsym_size_comparable() {
    fn dynsym_count(lib: &Path) -> usize {
        let out = Command::new("readelf")
            .args(["-sW", "--dyn-syms"])
            .arg(lib)
            .output()
            .expect("run readelf --dyn-syms");
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|l| l.contains(':'))
            .count()
    }
    let c = dynsym_count(c_lib());
    let r = dynsym_count(rust_lib());
    assert!(
        r <= c * 4,
        "the Rust .so exports/imports {r} dynamic symbols against the C .so's {c}. \
         That much extra means a runtime got linked in, which changes the lazy \
         resolver's stack footprint and therefore what bad() reads."
    );
}

/// Under the loader's default lazy binding, `driver(0)` must succeed or fault at
/// the *same rate* in both libraries. This is the property that one `std`-linked
/// Rust `cdylib` configuration violated outright: over 30 runs the C library
/// never faulted and that Rust build faulted every single time, because the
/// larger symbol table and extra `NEEDED` entries made `_dl_runtime_resolve`
/// leave an unmapped pointer in the slot `bad()` reads. Whether a given
/// std-linked build faults is itself configuration-sensitive, which is why
/// [`elf_02_dependency_parity`] and [`elf_03_dynsym_size_comparable`] pin the
/// cause down directly while this test checks the observable consequence.
///
/// The first call's printed *value* is ASLR-dependent and cannot be compared
/// (see [`lazy_binding_first_call_is_not_reproducible_even_c_vs_c`]), but whether
/// it faults is a stable, comparable property.
#[test]
fn lazy_binding_fault_behaviour_matches() {
    const RUNS: usize = 30;
    let script = ops(&["pd:0", "pd:0", "pd:0"]);
    let mut faults = [0usize; 2];
    let mut tails: [std::collections::HashSet<String>; 2] = Default::default();

    for (i, lib) in [c_lib(), rust_lib()].into_iter().enumerate() {
        for _ in 0..RUNS {
            let out = run_lazy(lib, &script);
            if out.signal.is_some() {
                faults[i] += 1;
            } else {
                let text = String::from_utf8_lossy(&out.stdout).into_owned();
                tails[i].insert(text.lines().skip(1).collect::<Vec<_>>().join(","));
            }
        }
    }

    assert_eq!(
        faults[0], faults[1],
        "over {RUNS} lazily-bound runs of driver(0), C faulted {} time(s) and Rust \
         faulted {} time(s). The two libraries must present the same fault \
         behaviour to the loader; check the NEEDED list and .dynsym size.",
        faults[0], faults[1]
    );
    assert_eq!(faults[0], 0, "expected the C library not to fault here");
    assert_eq!(
        tails[0], tails[1],
        "the post-resolver calls must produce identical output in both libraries"
    );
    assert_eq!(tails[0].len(), 1, "the post-resolver tail should be stable");
}
