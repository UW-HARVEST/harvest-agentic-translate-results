//! Phase C rows E5, E6, E7 — the allocation-failure rejections.
//!
//! `gotomach` clamps `iterations` to `[0, 65535]`, so the largest allocation it
//! ever requests is 262 140 bytes: `malloc` cannot be made to fail by choosing
//! argument values. These rows are therefore driven with a `malloc` interposer
//! (`tests/support/failmalloc.c`) that makes the Nth allocation inside a single
//! `gotomach` call return `NULL`.
//!
//! Each library is exercised by the *same* runner program
//! (`tests/support/runner.c`), which `dlopen`s the `.so`, resolves `gotomach`
//! and calls it — so the Rust `#[no_mangle]` export is still what is under
//! test. The runner's whole stdout (log lines plus `RESULT=`/`MALLOCS=`) is
//! compared between the two libraries, which additionally proves they issue the
//! same number of allocations in the same order.
//!
//! This test binary runs in its own process, and compiles its helpers into
//! `target/phase_c/`. Nothing in `c_src/` is touched.

mod common;

use common::*;
use std::path::{Path, PathBuf};
use std::process::Command;

struct Harness {
    runner: PathBuf,
}

/// The gcc helpers are built exactly once per test process: several tests in
/// this binary run concurrently, and rebuilding `runner` while a sibling test is
/// executing it fails with ETXTBSY.
fn harness() -> &'static Harness {
    static ONCE: std::sync::OnceLock<Harness> = std::sync::OnceLock::new();
    ONCE.get_or_init(Harness::build)
}

impl Harness {
    fn build() -> Harness {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let support = manifest.join("tests/support");
        // Per-profile directory so a debug and a release test binary never
        // clobber each other's helpers.
        let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
        let out = manifest.join("target/phase_c").join(profile);
        std::fs::create_dir_all(&out).expect("create target/phase_c");

        let shim = out.join("libfailmalloc.so");
        run(
            Command::new("gcc")
                .args(["-shared", "-fPIC", "-O0", "-o"])
                .arg(&shim)
                .arg(support.join("failmalloc.c")),
            "build libfailmalloc.so",
        );

        let runner = out.join("runner");
        run(
            Command::new("gcc")
                .args(["-O0", "-o"])
                .arg(&runner)
                .arg(support.join("runner.c"))
                .arg(&shim)
                .arg("-ldl")
                .arg(format!("-Wl,-rpath,{}", out.display())),
            "build runner",
        );

        Harness { runner }
    }

    /// Returns the runner's full stdout.
    fn run_lib(&self, so: &Path, it: i32, seed: i32, mode: i32, thr: i32, fail_at: i64) -> String {
        let out = Command::new(&self.runner)
            .arg(so)
            .arg(it.to_string())
            .arg(seed.to_string())
            .arg(mode.to_string())
            .arg(thr.to_string())
            .arg(fail_at.to_string())
            .output()
            .expect("spawn runner");
        assert!(
            out.status.success(),
            "runner failed for {} (fail_at={fail_at}): status={:?}\nstderr: {}",
            so.display(),
            out.status,
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).into_owned()
    }
}

fn run(cmd: &mut Command, what: &str) {
    let out = cmd.output().unwrap_or_else(|e| panic!("{what}: spawn: {e}"));
    assert!(
        out.status.success(),
        "{what} failed:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn parse(line: &str) -> (i32, i64) {
    let last = line
        .lines()
        .rev()
        .find(|l| l.starts_with("RESULT="))
        .unwrap_or_else(|| panic!("no RESULT= line in runner output:\n{line}"));
    let mut result = None;
    let mut mallocs = None;
    for tok in last.split_whitespace() {
        if let Some(v) = tok.strip_prefix("RESULT=") {
            result = Some(v.parse().expect("RESULT int"));
        } else if let Some(v) = tok.strip_prefix("MALLOCS=") {
            mallocs = Some(v.parse().expect("MALLOCS int"));
        }
    }
    (result.expect("RESULT"), mallocs.expect("MALLOCS"))
}

/// Compare the two libraries under the same interposer settings.
#[track_caller]
fn diff(
    h: &Harness,
    row: &str,
    it: i32,
    seed: i32,
    mode: i32,
    thr: i32,
    fail_at: i64,
) -> (i32, i64) {
    let c_out = h.run_lib(&c_so_path(), it, seed, mode, thr, fail_at);
    let r_out = h.run_lib(&rust_so_path(), it, seed, mode, thr, fail_at);
    if c_out != r_out {
        panic!(
            "[{row}] runner output differs for (iterations={it}, seed={seed}, mode={mode}, \
             threshold={thr}, fail_at={fail_at})\n--- C ---\n{c_out}--- Rust ---\n{r_out}"
        );
    }
    parse(&c_out)
}

/// Baseline: a successful call must issue exactly the C source's three
/// allocations — `ProcessorState`, `results`, `temp_buffer` — in both libraries.
#[test]
fn e5_e6_e7_baseline_three_allocations_per_successful_call() {
    let h = harness();
    for &mode in &[0, 1, 2, -1, 3] {
        for &(it, seed, thr) in &[
            (0, 0, 0),
            (1, 7, i32::MAX),
            (4, 7, i32::MAX),
            (64, 999, 2000),
            (512, 1, i32::MIN),
        ] {
            let (r, n) = diff(h, "E5/E6/E7 baseline", it, seed, mode, thr, 0);
            assert_eq!(
                n, 3,
                "expected 3 mallocs per gotomach call (state, results, temp_buffer), got {n} \
                 for (iterations={it}, seed={seed}, mode={mode}, threshold={thr})"
            );
            assert!(
                !(-6..=-1).contains(&r),
                "unexpected error code {r} on the success path"
            );
        }
    }
}

/// E5 — `malloc(sizeof(ProcessorState))` fails => `init_processor` returns NULL
/// => `gotomach` returns -3.
#[test]
fn e5_processorstate_malloc_failure_returns_minus_3() {
    let h = harness();
    for &(it, seed, mode, thr) in &[
        (0, 0, 0, 0),
        (1, 7, 0, i32::MAX),
        (4, 7, 1, i32::MAX),
        (64, 999, 2, 2000),
        (512, 1, -1, i32::MIN),
        (65535, 1, 0, i32::MAX),
    ] {
        let (r, n) = diff(h, "E5", it, seed, mode, thr, 1);
        assert_eq!(
            r, -3,
            "[E5] expected -3 when the ProcessorState allocation fails, got {r}"
        );
        assert_eq!(n, 1, "[E5] expected exactly 1 malloc attempt, got {n}");
    }
}

/// E6 — `malloc(capacity * sizeof(int))` fails => `init_processor` frees the
/// state and returns NULL => `gotomach` returns -3.
#[test]
fn e6_results_malloc_failure_returns_minus_3() {
    let h = harness();
    for &(it, seed, mode, thr) in &[
        (0, 0, 0, 0),
        (1, 7, 0, i32::MAX),
        (4, 7, 1, i32::MAX),
        (64, 999, 2, 2000),
        (512, 1, 3, i32::MIN),
        (65535, 1, 0, i32::MAX),
    ] {
        let (r, n) = diff(h, "E6", it, seed, mode, thr, 2);
        assert_eq!(
            r, -3,
            "[E6] expected -3 when the results allocation fails, got {r}"
        );
        assert_eq!(n, 2, "[E6] expected exactly 2 malloc attempts, got {n}");
    }
}

/// E7 — `temp_buffer = malloc(iterations * sizeof(int))` fails => -4.
#[test]
fn e7_temp_buffer_malloc_failure_returns_minus_4() {
    let h = harness();
    for &(it, seed, mode, thr) in &[
        (0, 0, 0, 0),
        (1, 7, 0, i32::MAX),
        (4, 7, 1, i32::MAX),
        (64, 999, 2, 2000),
        (512, 1, i32::MIN, i32::MIN),
        (65535, 1, 0, i32::MAX),
    ] {
        let (r, n) = diff(h, "E7", it, seed, mode, thr, 3);
        assert_eq!(
            r, -4,
            "[E7] expected -4 when the temp_buffer allocation fails, got {r}"
        );
        assert_eq!(n, 3, "[E7] expected exactly 3 malloc attempts, got {n}");
    }
}

/// A 4th failure point does not exist: `gotomach` never allocates again, so
/// arming `fail_at = 4` must behave exactly like the unarmed baseline.
#[test]
fn e5_e6_e7_no_fourth_allocation() {
    let h = harness();
    for &(it, seed, mode, thr) in &[(4, 7, 0, i32::MAX), (512, 1, 2, 2000), (65535, 1, 1, i32::MAX)]
    {
        let (r4, n4) = diff(h, "E5/E6/E7 fail_at=4", it, seed, mode, thr, 4);
        let (r0, n0) = diff(h, "E5/E6/E7 fail_at=0", it, seed, mode, thr, 0);
        assert_eq!((r4, n4), (r0, n0), "arming a 4th failure changed behaviour");
    }
}

/// Rejections that happen before any allocation must be unaffected by the
/// interposer, and must allocate nothing at all.
#[test]
fn e1_e4_reject_before_allocating() {
    let h = harness();
    for &(it, seed, want) in &[
        (-1, 0, -1),
        (65536, 0, -1),
        (i32::MIN, 0, -1),
        (i32::MAX, 0, -1),
        (4, -1, -2),
        (4, 65536, -2),
        (4, i32::MIN, -2),
    ] {
        for fail_at in [0i64, 1, 2, 3] {
            let (r, n) = diff(h, "E1/E4 pre-allocation", it, seed, 0, 0, fail_at);
            assert_eq!(r, want, "expected {want} for iterations={it}, seed={seed}");
            assert_eq!(n, 0, "no allocation may happen before validation, got {n}");
        }
    }
}
