//! Phase C — error / rejection-path differential tests.
//!
//! One test per row of `ERRORS.md`. `flip_horizontal` has no return value and
//! no error code, so "the same error/rejection" means one of:
//!   * both libraries silently do nothing and touch no memory (rows E2–E7,
//!     E10) — asserted byte-for-byte over the whole allocation, and asserted to
//!     be a true no-op rather than "some identical mutation";
//!   * both libraries perform the identical out-of-range accesses (row E9);
//!   * both libraries terminate the process with the *same* fatal signal
//!     (rows E1, E8) — checked by re-spawning this test binary in a child
//!     process and comparing `WTERMSIG`.

mod common;

use common::*;
use std::ffi::c_int;
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Stdio};

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// Assert both libraries leave the entire allocation bit-identical to the input
/// (a true no-op) and agree with each other.
#[track_caller]
fn assert_both_noop(label: &str, w: c_int, h: c_int, arena: &Arena) {
    let out = assert_same(label, w, h, arena);
    assert_eq!(
        out.bytes, arena.bytes,
        "[{label}] w={w} h={h}: expected a complete no-op, but memory changed"
    );
    assert_eq!(out.w, w, "[{label}] img.w changed");
    assert_eq!(out.h, h, "[{label}] img.h changed");
    assert!(out.pix_unchanged, "[{label}] img.pix changed");
}

/// An arena whose *entire* allocation (payload and padding) is random, so any
/// stray write anywhere is detectable.
fn fully_random_arena(rng: &mut Rng, payload_len: usize) -> Arena {
    let mut a = Arena::new(payload_len, 0);
    rng.fill(&mut a.bytes);
    a
}

/// Run a deliberately fatal scenario in a child process and return the signal
/// that killed it (or `None` if it exited normally).
fn run_fatal_child(scenario: &str) -> (Option<i32>, Option<i32>) {
    let exe = std::env::current_exe().expect("current_exe");
    let status = Command::new(exe)
        .args(["--exact", "fatal_child_entry", "--nocapture", "--test-threads=1"])
        .env("HARVEST_FATAL", scenario)
        // Keep the expected crash noise out of the parent's output.
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("spawning the fatal-scenario child failed");
    (status.signal(), status.code())
}

/// Child-process entry point. Does nothing unless `HARVEST_FATAL` is set, so it
/// is a harmless no-op during a normal `cargo test` run.
#[test]
fn fatal_child_entry() {
    let Ok(scenario) = std::env::var("HARVEST_FATAL") else {
        return;
    };
    let p = libs();
    let lib = match scenario.split(':').next().unwrap() {
        "c" => &p.c,
        "rust" => &p.rust,
        other => panic!("unknown library selector {other:?}"),
    };
    let which = scenario.split(':').nth(1).unwrap();
    match which {
        // E1: img == NULL — the C dereferences it unconditionally.
        "null_img" => unsafe { lib.flip(std::ptr::null_mut()) },
        // E8: img->pix == NULL with h >= 2 && w >= 1.
        "null_pix" => {
            let mut img = CpImage { w: 4, h: 4, pix: std::ptr::null_mut() };
            unsafe { lib.flip(&mut img) };
        }
        // E8 variant: the smallest shape that still dereferences pix.
        "null_pix_min" => {
            let mut img = CpImage { w: 1, h: 2, pix: std::ptr::null_mut() };
            unsafe { lib.flip(&mut img) };
        }
        other => panic!("unknown fatal scenario {other:?}"),
    }
    // If we get here the scenario did not fault; report that distinctly.
    std::process::exit(42);
}

#[track_caller]
fn assert_same_fatal(label: &str, scenario: &str) {
    let (sig_c, code_c) = run_fatal_child(&format!("c:{scenario}"));
    let (sig_r, code_r) = run_fatal_child(&format!("rust:{scenario}"));
    assert_eq!(
        (sig_c, code_c),
        (sig_r, code_r),
        "[{label}] termination diverged: C signal={sig_c:?} code={code_c:?} vs \
         Rust signal={sig_r:?} code={code_r:?}"
    );
    assert_eq!(
        sig_c,
        Some(11),
        "[{label}] expected both to die with SIGSEGV (11); got signal={sig_c:?} code={code_c:?}"
    );
}

// ---------------------------------------------------------------------------
// E1 — img == NULL
// ---------------------------------------------------------------------------

#[test]
fn e01_null_img_same_fatal_signal() {
    assert_same_fatal("E1 img=NULL", "null_img");
}

// ---------------------------------------------------------------------------
// E2 — h == 0  (pix is never dereferenced, even when it is garbage)
// ---------------------------------------------------------------------------

#[test]
fn e02_h_zero_is_noop() {
    let p = libs();
    let mut rng = Rng::new(SEED ^ 0xE02);
    for &w in &[0i32, 1, 2, 7, 64, 4096, i32::MAX] {
        let arena = fully_random_arena(&mut rng, 256);
        assert_both_noop(&format!("E2 w={w},h=0"), w, 0, &arena);
    }
    // With h == 0 the C never touches pix, so even a wild pointer is fine.
    for wild in [
        std::ptr::null_mut::<CpPixel>(),
        usize::MAX as *mut CpPixel,
        1usize as *mut CpPixel,
        0xDEAD_BEEFusize as *mut CpPixel,
    ] {
        for lib in [&p.c, &p.rust] {
            let mut img = CpImage { w: 9999, h: 0, pix: wild };
            unsafe { lib.flip(&mut img) };
            assert_eq!(img.w, 9999, "E2: {} mutated w", lib.name);
            assert_eq!(img.h, 0, "E2: {} mutated h", lib.name);
            assert!(std::ptr::eq(img.pix, wild), "E2: {} mutated pix", lib.name);
        }
    }
}

// ---------------------------------------------------------------------------
// E3 — h == 1  (flips = 0)
// ---------------------------------------------------------------------------

#[test]
fn e03_h_one_is_noop() {
    let p = libs();
    let mut rng = Rng::new(SEED ^ 0xE03);
    for &w in &[0i32, 1, 3, 100, i32::MAX] {
        let arena = fully_random_arena(&mut rng, 256);
        assert_both_noop(&format!("E3 w={w},h=1"), w, 1, &arena);
    }
    for lib in [&p.c, &p.rust] {
        let mut img = CpImage { w: i32::MAX, h: 1, pix: std::ptr::null_mut() };
        unsafe { lib.flip(&mut img) };
        assert!(img.pix.is_null(), "E3: {} touched a NULL pix with h=1", lib.name);
    }
}

// ---------------------------------------------------------------------------
// E4 — h < 0
// ---------------------------------------------------------------------------

#[test]
fn e04_negative_h_is_noop() {
    let p = libs();
    let mut rng = Rng::new(SEED ^ 0xE04);
    for &h in &[-1i32, -2, -3, -7, -8, -100, -4097] {
        for &w in &[0i32, 1, 5, 64, -1, -64, i32::MAX, i32::MIN] {
            let arena = fully_random_arena(&mut rng, 256);
            assert_both_noop(&format!("E4 w={w},h={h}"), w, h, &arena);
        }
    }
    // ... and with a NULL pix, to prove pix is never dereferenced.
    for &h in &[-1i32, -2, -1000] {
        for lib in [&p.c, &p.rust] {
            let mut img = CpImage { w: 32, h, pix: std::ptr::null_mut() };
            unsafe { lib.flip(&mut img) };
            assert!(img.pix.is_null(), "E4: {} touched a NULL pix with h={h}", lib.name);
        }
    }
}

// ---------------------------------------------------------------------------
// E5 — h == INT_MIN  (and the neighbouring extreme values)
// ---------------------------------------------------------------------------

#[test]
fn e05_h_int_min_is_noop() {
    let p = libs();
    let mut rng = Rng::new(SEED ^ 0xE05);
    for &h in &[i32::MIN, i32::MIN + 1, i32::MIN + 2, -i32::MAX] {
        for &w in &[0i32, 1, 8, -1, i32::MIN, i32::MAX] {
            let arena = fully_random_arena(&mut rng, 256);
            assert_both_noop(&format!("E5 w={w},h={h}"), w, h, &arena);
        }
        for lib in [&p.c, &p.rust] {
            let mut img = CpImage { w: 4, h, pix: std::ptr::null_mut() };
            unsafe { lib.flip(&mut img) };
            assert!(img.pix.is_null(), "E5: {} touched a NULL pix with h={h}", lib.name);
        }
    }
}

// ---------------------------------------------------------------------------
// E6 — w == 0 with h >= 2 (outer loop runs, inner loop never does)
// ---------------------------------------------------------------------------

#[test]
fn e06_zero_width_positive_height_is_noop() {
    let p = libs();
    let mut rng = Rng::new(SEED ^ 0xE06);
    for &h in &[2i32, 3, 4, 7, 8, 63, 64, 1000] {
        let arena = fully_random_arena(&mut rng, 512);
        assert_both_noop(&format!("E6 w=0,h={h}"), 0, h, &arena);
    }
    // w == 0 means every row pointer is `pix + 0`, which is never dereferenced,
    // so even a NULL pix must not fault.
    for &h in &[2i32, 3, 1_000_000] {
        for lib in [&p.c, &p.rust] {
            let mut img = CpImage { w: 0, h, pix: std::ptr::null_mut() };
            unsafe { lib.flip(&mut img) };
            assert!(img.pix.is_null(), "E6: {} touched a NULL pix with w=0,h={h}", lib.name);
        }
    }
}

// ---------------------------------------------------------------------------
// E7 — w < 0 with h >= 2 (row pointers computed, never dereferenced)
// ---------------------------------------------------------------------------

#[test]
fn e07_negative_width_is_noop() {
    let mut rng = Rng::new(SEED ^ 0xE07);
    // The payload sits in the middle of a large allocation, so the negative row
    // offsets the C computes stay inside memory this test owns.
    for &w in &[-1i32, -2, -4, -8, -97, -4096] {
        for &h in &[2i32, 3, 4, 9, 64] {
            let mut a = Arena::with_pad(4096, 0, 1 << 16, 0);
            rng.fill(&mut a.bytes);
            assert_both_noop(&format!("E7 w={w},h={h}"), w, h, &a);
        }
    }
}

// ---------------------------------------------------------------------------
// E8 — pix == NULL with h >= 2 && w >= 1
// ---------------------------------------------------------------------------

#[test]
fn e08_null_pix_same_fatal_signal() {
    assert_same_fatal("E8 pix=NULL w=4,h=4", "null_pix");
    assert_same_fatal("E8 pix=NULL w=1,h=2", "null_pix_min");
}

// ---------------------------------------------------------------------------
// E9 — undersized buffer: identical out-of-range accesses
// ---------------------------------------------------------------------------

#[test]
fn e09_undersized_buffer_identical_accesses() {
    let mut rng = Rng::new(SEED ^ 0xE09);
    // `claimed_h` rows are declared, but only `real_h` rows' worth of payload is
    // nominally owned; the surplus rows land in the (randomised) padding, which
    // this arena also owns, so C and Rust can be compared over the whole region.
    for &(w, real_h, claimed_h) in &[
        (6i32, 4i32, 8i32),
        (1, 1, 16),
        (3, 2, 7),
        (16, 3, 6),
        (5, 0, 5),
        (2, 1, 2),
        (9, 5, 11),
    ] {
        for rep in 0..8 {
            let payload = payload_bytes(w, real_h);
            let mut a = Arena::with_pad(payload, 0, 1 << 16, 0);
            rng.fill(&mut a.bytes);

            let label = format!("E9 w={w} real_h={real_h} claimed_h={claimed_h} rep{rep}");
            let out = assert_same(&label, w, claimed_h, &a);

            // The accesses must be exactly the ones the C loop computes, i.e.
            // identical to the model run over the whole owned region viewed as
            // a w*claimed_h image starting at pix_off.
            let mut expected = a.clone();
            let start = expected.pix_off;
            let end = start + payload_bytes(w, claimed_h);
            model_flip(w, claimed_h, &mut expected.bytes[start..end]);
            assert_eq!(
                out.bytes, expected.bytes,
                "[{label}] out-of-range accesses do not match the C loop's addressing"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// E10 — int overflow in the row offsets
// ---------------------------------------------------------------------------

#[test]
fn e10_row_offset_overflow_matches() {
    let mut rng = Rng::new(SEED ^ 0xE10);
    // `w` is negative or huge, so `w * i` / `w * (h-i-1)` overflow `int`. The
    // inner loop bound `j < w` is false for the negative cases, so the wrapped
    // pointers are computed but never dereferenced; both libraries must agree
    // (and must not panic on the overflow, which a non-wrapping Rust `*` would
    // do in a debug build).
    for &(w, h) in &[
        (i32::MIN, 2i32),
        (i32::MIN, 3),
        (i32::MIN, 4),
        (i32::MIN, 1024),
        (i32::MIN + 1, 8),
        (-1_000_000_000, 8),
        (-2_000_000_000, 6),
        (-65536, 65536),
        (-3, i32::MAX),
        (-1, i32::MAX),
        (i32::MIN, i32::MAX),
        (i32::MIN, i32::MIN),
    ] {
        let mut a = Arena::with_pad(4096, 0, 1 << 16, 0);
        rng.fill(&mut a.bytes);
        assert_both_noop(&format!("E10 w={w},h={h}"), w, h, &a);
    }

    // Huge positive `w` with `h <= 1`: `flips == 0`, so nothing is dereferenced
    // and the overflowing product is never even formed.
    for &(w, h) in &[(i32::MAX, 0i32), (i32::MAX, 1), (i32::MAX, -1), (1 << 30, 1)] {
        let mut a = Arena::with_pad(4096, 0, 1 << 16, 0);
        rng.fill(&mut a.bytes);
        assert_both_noop(&format!("E10 w={w},h={h}"), w, h, &a);
    }
}

// ---------------------------------------------------------------------------
// E11 — generic FFI boundary: arbitrary int values in w/h, i.e. the analogue of
// an out-of-range enum for this API (there is no enum parameter). Every 32-bit
// pattern is a legal `int` the C accepts; sweep random ones including the
// extremes, and confirm C and Rust agree bit-for-bit.
// ---------------------------------------------------------------------------

#[test]
fn e11_arbitrary_int_fields_agree() {
    let mut rng = Rng::new(SEED ^ 0xE11);

    // Boundary values one step either side of every threshold the C has
    // (`h/2 > 0`, `j < w`), crossed with each other.
    let edges: [i32; 13] = [
        i32::MIN,
        i32::MIN + 1,
        -2,
        -1,
        0,
        1,
        2,
        3,
        4,
        1023,
        1024,
        i32::MAX - 1,
        i32::MAX,
    ];
    for &w in &edges {
        for &h in &edges {
            // Only exercise combinations that cannot dereference memory:
            // that requires h >= 2 && w >= 1 (covered by Phase B with a real
            // buffer). Everything else must be an exact no-op in both libs.
            if h >= 2 && w >= 1 {
                continue;
            }
            let mut a = Arena::with_pad(1024, 0, 1 << 16, 0);
            rng.fill(&mut a.bytes);
            assert_both_noop(&format!("E11 edge w={w},h={h}"), w, h, &a);
        }
    }

    // Random 32-bit patterns. `w` spans the full `int` range; `h` is drawn from
    // a bounded range because the C's outer loop runs `h/2` times even when it
    // touches nothing, so an unbounded `h` would only cost wall-clock time
    // without reaching a new code path (the extreme `h` values are covered
    // exactly by the `edges` cross-product above).
    let mut checked = 0usize;
    for case in 0..2000 {
        let w = rng.next_u64() as i32;
        let h = rng.range_i32(-4096, 4096);
        if h >= 2 && w >= 1 {
            continue; // would dereference; not an error-path case
        }
        let mut a = Arena::with_pad(256, 0, 1 << 16, 0);
        rng.fill(&mut a.bytes);
        assert_both_noop(&format!("E11 rand case{case} w={w},h={h}"), w, h, &a);
        checked += 1;
    }
    assert!(checked > 100, "E11: expected a meaningful number of random cases, got {checked}");
}

// ---------------------------------------------------------------------------
// Generic boundary sweep: zero and oversized lengths, one step past every
// documented range, and a NULL struct pointer combined with each of them.
// ---------------------------------------------------------------------------

#[test]
fn generic_boundaries_agree() {
    let mut rng = Rng::new(SEED ^ 0xB0DE);

    // Zero-length payload with a *valid* (but 0-sized) allocation, at every
    // degenerate dimension pair around zero.
    for &w in &[-1i32, 0, 1] {
        for &h in &[-1i32, 0, 1] {
            let mut a = Arena::with_pad(0, 0, 1 << 16, 0);
            rng.fill(&mut a.bytes);
            assert_both_noop(&format!("bound w={w},h={h}"), w, h, &a);
        }
    }

    // One step past the largest shape actually backed by the buffer: the buffer
    // holds exactly h rows, and h+1 is requested. Both libs must reach the same
    // (out-of-range) addresses.
    for &(w, h) in &[(4i32, 4i32), (1, 8), (7, 3)] {
        let mut a = Arena::with_pad(payload_bytes(w, h), 0, 1 << 16, 0);
        rng.fill(&mut a.bytes);
        assert_same(&format!("bound one-past w={w},h={h}+1"), w, h + 1, &a);

        let mut b = Arena::with_pad(payload_bytes(w, h), 0, 1 << 16, 0);
        rng.fill(&mut b.bytes);
        assert_same(&format!("bound one-past w={w}+1,h={h}"), w + 1, h, &b);
    }
}
