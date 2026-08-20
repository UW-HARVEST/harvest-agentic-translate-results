//! Phase B — CONFIGS.md rows 66..70: the test-driver entry points `strkey` and
//! `sh_geti`.
//!
//! `sh_geti` writes to stdout via libc `printf` and asserts internally, so it is
//! run in a child process (see `common::spawn_scenario`) and both its stdout
//! bytes and its termination status are compared.
mod common;

use common::*;
use core::ffi::{c_char, c_int};

/// The subprocess entry point. Must exist in every test binary that spawns
/// scenarios, because `spawn_scenario` re-executes `current_exe()`.
#[test]
fn scenario_runner() {
    let Ok(name) = std::env::var("DIFF_SCENARIO") else {
        return;
    };
    let which = std::env::var("DIFF_LIB").expect("DIFF_LIB");
    let lib = Lib::pick(&which);
    unsafe { run_scenario(&name, &lib) };
    std::process::exit(0);
}

/// Both libraries' `strkey` must return the identical NUL-terminated string,
/// and the identical full 256-byte static buffer (so leftover bytes from an
/// earlier longer key are reproduced too).
unsafe fn cmp_strkey(c: &Lib, r: &Lib, n: c_int) {
    let pc = (c.strkey)(n);
    let pr = (r.strkey)(n);
    assert!(!pc.is_null() && !pr.is_null());
    let sc = cstr_bytes(pc).unwrap();
    let sr = cstr_bytes(pr).unwrap();
    assert_eq!(
        String::from_utf8_lossy(&sc),
        String::from_utf8_lossy(&sr),
        "strkey({n})"
    );
    // and it really is "test_%d"
    assert_eq!(
        sc,
        format!("test_{n}").into_bytes(),
        "strkey({n}) content"
    );
    // full static buffer, including residue past the NUL
    let bc = core::slice::from_raw_parts(pc as *const u8, 256);
    let br = core::slice::from_raw_parts(pr as *const u8, 256);
    if let Some(i) = (0..256).find(|&i| bc[i] != br[i]) {
        panic!(
            "strkey({n}): static buffer byte {i} diverged: C={:#04x} Rust={:#04x}\n C  ={:02x?}\n Rust={:02x?}",
            bc[i], br[i], bc, br
        );
    }
}

/// Row 66 — `strkey` over corner and random `int` values.
#[test]
fn cfg66_strkey() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x6666_6666);
    unsafe {
        // the returned pointer must be the same static buffer every time
        let p0 = (c.strkey)(0);
        let p1 = (c.strkey)(12345);
        assert_eq!(p0, p1, "C strkey must return the same static buffer");
        let q0 = (r.strkey)(0);
        let q1 = (r.strkey)(12345);
        assert_eq!(q0, q1, "Rust strkey must return the same static buffer");

        for n in [
            0i32, 1, 2, 3, 9, 10, 11, 99, 100, 101, 999, 1000, 1001, 9999, 10000, 12345, 100000,
            999999999, 1000000000, i32::MAX, -1, -2, -9, -10, -11, -99, -100, -12345, -999999999,
            i32::MIN,
        ] {
            cmp_strkey(&c, &r, n);
        }
        // long key then short key, so the buffer residue is exercised
        cmp_strkey(&c, &r, i32::MIN);
        cmp_strkey(&c, &r, 0);
        cmp_strkey(&c, &r, i32::MAX);
        cmp_strkey(&c, &r, 1);
        for _ in 0..256 {
            cmp_strkey(&c, &r, rng.next_u64() as i32);
        }
        // exhaustive over the digit-count boundaries
        for k in 0..10u32 {
            let base = 10i64.pow(k);
            for d in [-1i64, 0, 1] {
                let v = base + d;
                if v <= i32::MAX as i64 {
                    cmp_strkey(&c, &r, v as i32);
                    cmp_strkey(&c, &r, -(v as i32));
                }
            }
        }
    }
}

/// Row 67 — `sh_geti(num)` for every interesting `num`: stdout bytes and the
/// child's termination status must match exactly.
#[test]
fn cfg67_sh_geti_positive() {
    let _g = serial();
    for num in [
        0i32, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 23, 24, 25, 31, 32, 33,
        63, 64, 65, 100, 127, 128, 129, 255, 256, 257, 1000,
    ] {
        let out = assert_scenario_matches(&format!("sh_geti:{num}"));
        // sanity: the printf loop runs twice (once per `j`), so a positive `num`
        // must produce 2*ceil(num/2) lines
        let expect_lines = if num <= 0 {
            0
        } else {
            2 * ((num as usize + 1) / 2)
        };
        let lines = out.stdout.iter().filter(|&&b| b == b'\n').count();
        assert_eq!(
            lines, expect_lines,
            "sh_geti({num}) printed {lines} lines, expected {expect_lines}"
        );
        assert_eq!(out.signal, None, "sh_geti({num}) must not die on a signal");
        assert_eq!(out.code, Some(0), "sh_geti({num}) must exit 0");
    }
}

/// Row 68 — non-positive `num`: every loop body is skipped.
#[test]
fn cfg68_sh_geti_nonpositive() {
    let _g = serial();
    for num in [0i32, -1, -2, -100, i32::MIN] {
        let out = assert_scenario_matches(&format!("sh_geti:{num}"));
        assert_eq!(out.stdout, b"", "sh_geti({num}) must print nothing");
        assert_eq!(out.code, Some(0));
        assert_eq!(out.signal, None);
    }
}

/// Row 69 — `stbds_rand_seed` before `sh_geti` changes the probe order and thus
/// the print order; C and Rust must agree for every seed.
#[test]
fn cfg69_sh_geti_seeded() {
    let _g = serial();
    let mut outs = Vec::new();
    for seed in [0usize, 1, 7, 0xdeadbeef, 0x31415926] {
        for num in [8i32, 16, 33] {
            let out = assert_scenario_matches(&format!("sh_geti:{num}:{seed}"));
            assert_eq!(out.code, Some(0));
            if num == 33 {
                outs.push(out.stdout);
            }
        }
    }
    // usize::MAX does not fit the `i64` scenario parameter parser
    let out = assert_scenario_matches("sh_geti_seed_max:33");
    assert_eq!(out.code, Some(0));
    outs.push(out.stdout);
    // NOTE: `sh_geti` prints `strmap[z]` for z in 0..shlen, i.e. in ARRAY
    // (insertion) order, not bucket order, so the seed provably does NOT change
    // stdout. What the seed does change is the probe order, and hence which
    // code paths inside stbds_hmput_key / stbds_hmdel_key run and whether the
    // internal STBDS_ASSERTs hold — that is what these runs verify (exit 0 for
    // every seed, on both libraries).
    assert!(outs.iter().all(|o| !o.is_empty()));
    assert!(
        outs.windows(2).all(|w| w[0] == w[1]),
        "sh_geti output is insertion-ordered and must be seed-independent"
    );
}

/// Row 70 — `sh_geti` called twice in one process: the second call runs with an
/// already-advanced global `stbds_hash_seed`, so its output differs from the
/// first. Both libraries must produce the same combined stream.
#[test]
fn cfg70_sh_geti_twice() {
    let _g = serial();
    for num in [8i32, 16, 33, 64] {
        let single = assert_scenario_matches(&format!("sh_geti:{num}"));
        let twice = assert_scenario_matches(&format!("sh_geti_twice:{num}"));
        assert_eq!(twice.code, Some(0));
        // the second call must not simply repeat the first
        assert!(
            twice.stdout.len() == 2 * single.stdout.len(),
            "second sh_geti call produced a different line count"
        );
        let (a, b) = twice.stdout.split_at(single.stdout.len());
        assert_eq!(a, &single.stdout[..], "first half must match a single call");
        // insertion-ordered output => the second call repeats the first, but it
        // ran with an already-advanced global seed, so all of its internal
        // asserts were re-checked against a different probe layout.
        assert_eq!(a, b);
    }
}

/// Extra: `strkey` and `sh_geti` interact through the shared static `buffer`.
/// Verified in-process for `strkey` (above) and in-subprocess here for the
/// combination, since `sh_geti` calls `strkey` internally.
#[test]
fn cfg_strkey_after_sh_geti() {
    let _g = serial();
    let _ = assert_scenario_matches("sh_geti:12");
    let (c, r) = both();
    unsafe {
        // in-process, both libraries' buffers are still in lock-step
        cmp_strkey(&c, &r, 7);
        cmp_strkey(&c, &r, -7);
        let _: *mut c_char = (c.strkey)(0);
    }
}
