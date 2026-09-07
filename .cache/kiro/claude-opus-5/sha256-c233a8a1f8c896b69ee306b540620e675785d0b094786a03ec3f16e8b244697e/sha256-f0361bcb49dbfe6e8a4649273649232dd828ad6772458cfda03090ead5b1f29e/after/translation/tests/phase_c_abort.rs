//! Phase C — abort parity for the reachable `assert` paths (ERRORS.md #29/#30).
//!
//! `stbds_hmdel_key` with an out-of-range `mode >= 2` takes the
//! `mode == STBDS_HM_STRING` branches as *false*, so the relocation lookup is
//! handed the raw element address instead of the stored `char *`.  That lookup
//! finds nothing, `slot` comes back `-1`, and `STBDS_ASSERT(slot >= 0)` kills
//! the process.  Because the C build defines no `NDEBUG`, this is a real,
//! deterministic, observable outcome — so the Rust must die the same way.
//!
//! The scenario cannot run in-process (it terminates the process), so the test
//! re-executes this same test binary as a child, once per library, and compares
//! the exit status exactly.

mod common;
use common::*;
use std::ffi::c_void;
use std::os::unix::process::ExitStatusExt;
use std::process::Command;

const SH_DEFAULT: i32 = 1;
const SH_STRDUP: i32 = 2;
const SH_ARENA: i32 = 3;

/// Builds a string map with `n` entries, then deletes the FIRST inserted key
/// using `mode`, which forces the relocation branch.
fn scenario(api: &Api, shmode: i32, mode: i32, n: usize) {
    unsafe {
        (api.rand_seed)(0x3141_5926);
        let elemsize = 16usize;
        let mut t = (api.shmode_func)(elemsize, shmode);
        let keys: Vec<CBuf> = (0..n)
            .map(|i| CBuf::new(format!("abortkey-{i}\0").as_bytes()))
            .collect();
        for k in &keys {
            t = (api.hmput_key)(t, elemsize, k.cptr() as *mut c_void, 8, mode);
            // zero the payload so `hash_string` on the raw element address can
            // never run off the end of the element
            let idx = (*hdr(hash_to_arr(t, elemsize))).temp;
            std::ptr::write_bytes(
                (t as *mut u8).add(elemsize * idx as usize + 8),
                0,
                8,
            );
        }
        // element 1 is the first inserted key; deleting it moves the last
        // element into its slot, which is the relocation path.
        t = (api.hmdel_key)(t, elemsize, keys[0].cptr() as *mut c_void, 8, 0, mode);
        // If we get here the assert did NOT fire; report that distinctly.
        eprintln!("scenario returned normally, t={t:?}");
    }
}

const SCENARIOS: [(i32, i32, usize); 9] = [
    (SH_DEFAULT, 2, 6),
    (SH_DEFAULT, 9, 6),
    (SH_DEFAULT, i32::MAX, 6),
    (SH_STRDUP, 2, 6),
    (SH_STRDUP, 3, 12),
    (SH_ARENA, 2, 6),
    (SH_ARENA, 7, 12),
    (SH_DEFAULT, 2, 40),
    (SH_STRDUP, 2, 40),
];

/// The child half: picked by env vars, runs one scenario against one library.
#[test]
#[ignore = "child process helper, driven by abort_parity"]
fn abort_child() {
    let which = std::env::var("ABORT_LIB").expect("ABORT_LIB not set");
    let idx: usize = std::env::var("ABORT_SCENARIO")
        .expect("ABORT_SCENARIO not set")
        .parse()
        .unwrap();
    let (_g, b) = both();
    let api = match which.as_str() {
        "c" => &b.c,
        "rust" => &b.r,
        other => panic!("bad ABORT_LIB {other}"),
    };
    let (shmode, mode, n) = SCENARIOS[idx];
    scenario(api, shmode, mode, n);
}

fn run_child(idx: usize, lib: &str) -> (Option<i32>, Option<i32>) {
    let exe = std::env::current_exe().unwrap();
    let out = Command::new(exe)
        .args(["abort_child", "--exact", "--ignored", "--test-threads=1"])
        .env("ABORT_LIB", lib)
        .env("ABORT_SCENARIO", idx.to_string())
        .env("RUST_BACKTRACE", "0")
        .output()
        .expect("failed to spawn child");
    (out.status.code(), out.status.signal())
}

/// ERRORS.md #29/#30 — both libraries must die identically.
#[test]
fn abort_parity() {
    // sanity: the harness itself must be able to load both libraries
    {
        let (_g, _b) = both();
    }
    for idx in 0..SCENARIOS.len() {
        let (shmode, mode, n) = SCENARIOS[idx];
        let c = run_child(idx, "c");
        let r = run_child(idx, "rust");
        assert_eq!(
            c, r,
            "scenario {idx} (shmode={shmode} mode={mode} n={n}): C exited {c:?} but Rust exited {r:?}"
        );
        // and it really is the assert firing, not a normal return
        assert_eq!(
            c.1,
            Some(6),
            "scenario {idx} was expected to die by SIGABRT (assert), got {c:?}"
        );
    }
}
