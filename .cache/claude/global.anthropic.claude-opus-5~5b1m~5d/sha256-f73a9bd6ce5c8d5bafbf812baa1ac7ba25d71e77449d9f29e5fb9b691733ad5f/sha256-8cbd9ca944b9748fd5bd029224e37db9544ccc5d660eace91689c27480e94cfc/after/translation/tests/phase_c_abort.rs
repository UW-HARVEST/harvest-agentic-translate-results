// Phase C (part 2) — ABORT-PARITY tests.
//
// Some rejection paths in the C source end in `assert()` (SIGABRT) or in
// libc-level undefined behaviour.  Those cannot be observed in-process, so each
// case is run in a CHILD process against the C `.so` and against the Rust `.so`
// and the two termination statuses are compared.
//
// Covered:
//   * ERRORS.md row 17 — `assert(slot >= 0)` in `stbds_hmdel_key`, reachable
//     with an out-of-range `mode >= 2` when the deleted element is NOT the last
//     one (the re-find then hashes the raw element bytes as a C string).
//   * ERRORS.md row 27 — `stbds_arrfreef(NULL)` -> `free((char*)NULL - 32)`.

mod common;
use common::*;
use std::os::raw::c_char;
use std::process::{Command, Output};

const CASE_ENV: &str = "DIFF_ABORT_CASE";
const IMPL_ENV: &str = "DIFF_ABORT_IMPL";

fn run_child(case: &str, which: &str) -> Output {
    let exe = std::env::current_exe().expect("current_exe");
    Command::new(exe)
        .args(["--exact", "abort_worker", "--nocapture", "--test-threads=1"])
        .env(CASE_ENV, case)
        .env(IMPL_ENV, which)
        .output()
        .expect("spawn child")
}

fn status_of(o: &Output) -> (Option<i32>, Option<i32>, bool) {
    use std::os::unix::process::ExitStatusExt;
    (
        o.status.code(),
        o.status.signal(),
        String::from_utf8_lossy(&o.stdout).contains("SURVIVED"),
    )
}

fn compare_case(case: &str) {
    let c = run_child(case, "c");
    let r = run_child(case, "rust");
    let sc = status_of(&c);
    let sr = status_of(&r);
    assert_eq!(
        sc, sr,
        "abort parity mismatch for case `{case}`\n  C    = {sc:?}\n  RUST = {sr:?}\n\
         C stderr: {}\nRUST stderr: {}",
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
    eprintln!("case `{case}`: both terminated with {sc:?}");
}

// ---------------------------------------------------------------------------
// The worker: only does anything when the driver env vars are set.
// ---------------------------------------------------------------------------

#[test]
fn abort_worker() {
    let case = match std::env::var(CASE_ENV) {
        Ok(c) => c,
        Err(_) => return, // ordinary test run: nothing to do
    };
    let which = std::env::var(IMPL_ENV).expect("impl env");
    let p = pair();
    let im = if which == "c" { &p.c } else { &p.r };
    unsafe {
        (im.rand_seed)(0x31415926);
    }
    match case.as_str() {
        // row 17: delete a NON-final element with mode = 2
        "del_mode2_nonfinal" => {
            const ES: usize = 16;
            const KS: usize = 8;
            let mut keys: Vec<Vec<u8>> = (0..10)
                .map(|i| {
                    let mut v = cstring(format!("abortkey_{i}").as_bytes());
                    v.resize(v.len().max(KS + 1), 0);
                    v
                })
                .collect();
            let mut m = Map::new(im, ES, KS);
            for (i, k) in keys.iter_mut().enumerate() {
                let kp = k.as_mut_ptr() as *mut c_char;
                m.sput(kp, &(i as u64).to_ne_bytes(), 2);
            }
            // element index 1 is the first inserted key; final index is 10
            let kp = keys[0].as_mut_ptr() as *mut c_char;
            let t = m.sdel(kp, 2, 0);
            println!("SURVIVED temp={t}");
        }
        // row 27: stbds_arrfreef(NULL) -> free((char*)NULL - sizeof(header))
        "arrfreef_null" => unsafe {
            (im.arrfreef)(std::ptr::null_mut());
            println!("SURVIVED");
        },
        other => panic!("unknown abort case {other}"),
    }
}

// ---------------------------------------------------------------------------
// The parity assertions
// ---------------------------------------------------------------------------

#[test]
fn err17_abort_parity_hmdel_mode2_nonfinal() {
    let _g = guard();
    compare_case("del_mode2_nonfinal");
}

#[test]
fn err27_abort_parity_arrfreef_null() {
    let _g = guard();
    compare_case("arrfreef_null");
}
