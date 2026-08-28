// Phase C — ERRORS.md row E16: `checkshift`'s allocation-failure guard.
//
//     ComputeState* state = (ComputeState*)malloc(sizeof(ComputeState));
//     if (state == NULL) {
//         printf("Error: Failed to allocate memory for state\n");
//         return -1;
//     }
//
// A 12-byte allocation never fails in practice, so this branch cannot be reached
// by choosing arguments. It IS reachable with an LD_PRELOAD `malloc` interposer
// that fails allocations of exactly `sizeof(ComputeState)`. Both libraries are
// driven out-of-process through the same driver and interposer, and their full
// stdout + exit status are compared byte for byte.

mod common;
use common::*;

use std::path::{Path, PathBuf};
use std::process::Command;

fn helpers_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/helpers")
}

fn out_dir() -> PathBuf {
    let d = PathBuf::from(std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".into()))
        .join("checkshift-e16");
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Compile a helper with `cc`. Returns None if the toolchain is unavailable.
fn compile(src: &str, out: &Path, extra: &[&str]) -> Option<PathBuf> {
    let src_path = helpers_dir().join(src);
    assert!(src_path.exists(), "missing helper source {}", src_path.display());
    let mut cmd = Command::new("cc");
    cmd.arg(&src_path).arg("-O1").arg("-o").arg(out);
    for e in extra {
        cmd.arg(e);
    }
    match cmd.output() {
        Err(e) => {
            eprintln!("E16: cannot run `cc` ({e}); skipping malloc-failure test");
            None
        }
        Ok(o) if !o.status.success() => {
            panic!(
                "E16: failed to compile {src}:\nstdout: {}\nstderr: {}",
                String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr)
            );
        }
        Ok(_) => Some(out.to_path_buf()),
    }
}

struct Harness {
    driver: PathBuf,
    preload: PathBuf,
}

fn build_harness() -> Option<Harness> {
    let od = out_dir();
    let preload = compile(
        "malloc_fail.c",
        &od.join("libmallocfail.so"),
        &["-shared", "-fPIC"],
    )?;
    let driver = compile("driver.c", &od.join("driver"), &["-ldl"])?;
    Some(Harness { driver, preload })
}

/// Run the driver against `lib`, failing allocations of `fail_size` bytes.
fn run(h: &Harness, lib: &Path, fail_size: usize, params: [i32; 4]) -> (i32, String, String) {
    let out = Command::new(&h.driver)
        .arg(lib)
        .arg(fail_size.to_string())
        .arg(params[0].to_string())
        .arg(params[1].to_string())
        .arg(params[2].to_string())
        .arg(params[3].to_string())
        .env("LD_PRELOAD", &h.preload)
        .output()
        .unwrap_or_else(|e| panic!("failed to run driver: {e}"));
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

// ---------------------------------------------------------------------------
// Sanity: with the interposer loaded but NOT armed, both libraries behave
// exactly as they do in-process. This validates the harness itself.
// ---------------------------------------------------------------------------

#[test]
fn e16_harness_sanity_unarmed() {
    let Some(h) = build_harness() else { return };
    let (c, r) = libs();

    for params in [[1i32, 2, 3, 4], [-1, -2, -3, -4], [i32::MAX, i32::MIN, 0, 7]] {
        let (cs, cout, cerr) = run(&h, &c.path, 0, params);
        let (rs, rout, rerr) = run(&h, &r.path, 0, params);
        assert_eq!(cs, 0, "E16 sanity: C driver exit status (stderr: {cerr})");
        assert_eq!(rs, 0, "E16 sanity: Rust driver exit status (stderr: {rerr})");
        assert_eq!(
            cout, rout,
            "E16 sanity: unarmed stdout must match for {params:?}"
        );
        assert!(
            cout.contains("=== Ending foo function ===") && cout.contains("RESULT="),
            "E16 sanity: unexpected transcript: {cout:?}"
        );
        assert!(
            !cout.contains("Failed to allocate memory"),
            "E16 sanity: unarmed run must not hit the failure path"
        );
    }
}

// ---------------------------------------------------------------------------
// E16 — armed: malloc(sizeof(ComputeState)) returns NULL in BOTH libraries.
// ---------------------------------------------------------------------------

#[test]
fn err_e16_checkshift_malloc_failure_path() {
    let Some(h) = build_harness() else { return };
    let (c, r) = libs();

    assert_eq!(STATE_SIZE, 12, "sizeof(ComputeState)");

    for params in [
        [1i32, 2, 3, 4],
        [0, 0, 0, 0],
        [-1, -2, -3, -4],
        [i32::MAX, i32::MIN, 1, -1],
    ] {
        let (cs, cout, cerr) = run(&h, &c.path, STATE_SIZE, params);
        let (rs, rout, rerr) = run(&h, &r.path, STATE_SIZE, params);

        assert_eq!(cs, 0, "E16: C driver exit status (stderr: {cerr})");
        assert_eq!(rs, 0, "E16: Rust driver exit status (stderr: {rerr})");

        // The C must have taken the failure branch; if it did not, the interposer
        // is not reaching the library and the test would be vacuous.
        assert!(
            cout.contains("Error: Failed to allocate memory for state\n"),
            "E16: the interposer did not reach the C library. stdout: {cout:?}"
        );
        assert!(
            cout.contains("RESULT=-1"),
            "E16: C must return the -1 sentinel. stdout: {cout:?}"
        );
        // The pipeline must have been abandoned immediately.
        assert!(
            !cout.contains("State initialized with accumulator"),
            "E16: C must return before init_state. stdout: {cout:?}"
        );
        assert!(
            !cout.contains("=== Ending foo function ==="),
            "E16: C must not reach the end. stdout: {cout:?}"
        );

        // And the Rust must reject it identically, byte for byte.
        assert_eq!(
            cout, rout,
            "E16: allocation-failure transcript diverges for {params:?}\n  C    = {cout:?}\n  Rust = {rout:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// A near-miss control: failing a size the library never requests must leave the
// success path completely intact in both libraries.
// ---------------------------------------------------------------------------

#[test]
fn e16_near_miss_sizes_do_not_trigger_the_guard() {
    let Some(h) = build_harness() else { return };
    let (c, r) = libs();

    for fail_size in [STATE_SIZE - 1, STATE_SIZE + 1, 11, 13, 12345] {
        let (cs, cout, cerr) = run(&h, &c.path, fail_size, [1, 2, 3, 4]);
        let (rs, rout, rerr) = run(&h, &r.path, fail_size, [1, 2, 3, 4]);
        assert_eq!(cs, 0, "E16 near-miss {fail_size}: C exit (stderr: {cerr})");
        assert_eq!(rs, 0, "E16 near-miss {fail_size}: Rust exit (stderr: {rerr})");
        assert_eq!(
            cout, rout,
            "E16 near-miss {fail_size}: stdout must match"
        );
        assert!(
            !cout.contains("Failed to allocate memory"),
            "E16 near-miss {fail_size}: guard must not fire for C. stdout: {cout:?}"
        );
        assert!(
            cout.contains("=== Ending foo function ==="),
            "E16 near-miss {fail_size}: must run to completion"
        );
    }
}
