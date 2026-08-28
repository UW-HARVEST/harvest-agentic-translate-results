//! Phase C — error-path differential tests for the ERRORS.md rows that live
//! inside `find_node_by_id` / `add_node` / `process_backward` /
//! `safe_double_to_int`, i.e. rows 5-11. These are only reachable once
//! `node_storage` is populated, so they run through the C init shim and the
//! Rust `expose_init_test_data` feature.

#![cfg(feature = "expose_init_test_data")]

mod common;

use common::*;
use std::ffi::c_int;

fn ready() -> (&'static Pair, std::sync::MutexGuard<'static, ()>) {
    let g = state_lock();
    let p = Pair::with_init();
    p.init_both();
    (p, g)
}

/// Ids that `initialize_test_data` creates.
const PRESENT: [c_int; 7] = [1, 2, 3, 4, 5, 6, 7];

// --- ERRORS.md row 5 ------------------------------------------------------

#[test]
fn err_row5_unknown_id_with_data() {
    // find_node_by_id returns NULL when no stored node matches, even though
    // node_count == 7. Each of the three callers must map that to its own code.
    let (p, _g) = ready();
    let absent: Vec<c_int> = {
        let mut v = vec![0, 8, 9, 10, -1, -7, 100, 1000, i32::MIN, i32::MAX, i32::MIN + 1];
        v.retain(|x| !PRESENT.contains(x));
        v
    };
    for &n in &absent {
        for &d in &[0, 1, 3, 16, 17, i32::MAX] {
            p.assert_same_eq(0o1, n, d, 0, ERR_MODE1_NOT_FOUND);
            p.assert_same_eq(0o2, n, d, 0, ERR_MODE2_NOT_FOUND);
            p.assert_same_eq(0o4, n, d, 0, ERR_MODE4_NOT_FOUND);
        }
    }
    // Boundary: id 7 exists, id 8 does not (node_count == 7 stops the scan).
    assert_ne!(p.assert_same(0o1, 7, 0, 0), ERR_MODE1_NOT_FOUND);
    p.assert_same_eq(0o1, 8, 0, 0, ERR_MODE1_NOT_FOUND);
    // id 0 must NOT match the zeroed slots beyond node_count.
    p.assert_same_eq(0o1, 0, 0, 0, ERR_MODE1_NOT_FOUND);

    let mut rng = Rng::new(0xC000_0005);
    for _ in 0..30_000 {
        let n = rng.i32_interesting();
        if PRESENT.contains(&n) {
            continue;
        }
        p.assert_same_eq(0o1, n, rng.i32_interesting(), 0, ERR_MODE1_NOT_FOUND);
        p.assert_same_eq(0o4, n, rng.i32_interesting(), 0, ERR_MODE4_NOT_FOUND);
    }
}

// --- ERRORS.md row 6 ------------------------------------------------------

#[test]
fn err_row6_add_node_capacity() {
    // `add_node` returns STATUS_ERROR when node_count >= MAX_NODES (100).
    // The exported surface can only ever add 7 nodes (initialize_test_data
    // resets node_count to 0 first), so the guard itself is unreachable from
    // outside. What IS observable — and what would break if the guard or the
    // reset were mistranslated — is that node_count stays pinned at 7 no matter
    // how many times init runs. Assert that in both libraries.
    let g = state_lock();
    let p = Pair::with_init();
    for round in 1..=40 {
        p.init_both();
        // mode 0004's backward scan sums the last min(3, node_count) values and
        // is gated on `node_count > 2`; mode 0001 walks the parent chain. Both
        // would change if storage kept growing.
        p.assert_same_eq(0o4, 1, 0, 0, {
            let mut a = 0.0f64;
            for d in [0o100, 0o200, 0o300, 0o400] {
                a += (d as f64).sqrt() * 2.718281828;
            }
            (a as c_int) + 82
        });
        // Node id 8 must never appear, i.e. nothing beyond the 7 was stored.
        p.assert_same_eq(0o1, 8, 1, 0, ERR_MODE1_NOT_FOUND);
        assert_ne!(p.assert_same(0o1, 7, 3, 0), ERR_MODE1_NOT_FOUND, "round {round}");
    }
    drop(g);
}

// --- ERRORS.md rows 7 & 8 -------------------------------------------------

/// `sum(sqrt(data[i]) * 2.718281828)` exactly as case 0004 accumulates it.
fn acc0() -> f64 {
    let mut a = 0.0f64;
    for d in [0o100, 0o200, 0o300, 0o400] {
        a += (d as f64).sqrt() * 2.718281828;
    }
    a
}

/// The full case-0004 model: scale, `safe_double_to_int`, then the
/// `node_count > 2` backward scan (12 + 40 + 30 = 82).
fn m4(depth: c_int) -> c_int {
    let scaled = acc0() * (1.0 + (depth as f64) * 0.1);
    let clamped = if scaled > 2147483647.0 {
        2147483647.0
    } else if scaled < -2147483648.0 {
        -2147483648.0
    } else {
        scaled
    };
    (clamped as c_int).wrapping_add(82)
}

/// Whether `safe_double_to_int` clamps for this depth (and in which direction).
fn clamp_dir(depth: c_int) -> i32 {
    let scaled = acc0() * (1.0 + (depth as f64) * 0.1);
    if scaled > 2147483647.0 {
        1
    } else if scaled < -2147483648.0 {
        -1
    } else {
        0
    }
}

#[test]
fn err_row7_saturate_high() {
    // safe_double_to_int clamps value > 2147483647.0 -> 2147483647.
    // acc0 ~= 130.86, so the clamp engages once 1.0 + depth*0.1 > ~1.641e7,
    // i.e. depth > ~1.64e8.
    let (p, _g) = ready();
    let boundary = (((2147483647.0 / acc0()) - 1.0) / 0.1) as c_int;
    println!("upper-clamp boundary depth = {boundary}");

    // Deep inside the clamped region.
    for d in [
        boundary * 2,
        500_000_000,
        1_000_000_000,
        2_000_000_000,
        i32::MAX - 1,
        i32::MAX,
    ] {
        assert_eq!(clamp_dir(d), 1, "depth {d} must saturate high");
        p.assert_same_eq(0o4, 1, d, 0, i32::MAX.wrapping_add(82));
    }

    // Straddle the exact boundary: unclamped on one side, clamped on the other.
    let mut saw_clamped = false;
    let mut saw_unclamped = false;
    for d in (boundary - 4).max(0)..=(boundary + 4) {
        match clamp_dir(d) {
            1 => saw_clamped = true,
            0 => saw_unclamped = true,
            _ => unreachable!(),
        }
        p.assert_same_eq(0o4, 1, d, 0, m4(d));
    }
    assert!(saw_clamped && saw_unclamped, "boundary not straddled");
}

#[test]
fn err_row8_saturate_low() {
    // safe_double_to_int clamps value < -2147483648.0 -> -2147483648.
    let (p, _g) = ready();
    let boundary = (((-2147483648.0 / acc0()) - 1.0) / 0.1) as c_int;
    println!("lower-clamp boundary depth = {boundary}");

    for d in [
        boundary * 2,
        -500_000_000,
        -1_000_000_000,
        -2_000_000_000,
        i32::MIN + 1,
        i32::MIN,
    ] {
        assert_eq!(clamp_dir(d), -1, "depth {d} must saturate low");
        p.assert_same_eq(0o4, 1, d, 0, i32::MIN.wrapping_add(82));
    }

    let mut saw_clamped = false;
    let mut saw_unclamped = false;
    for d in (boundary - 4)..=(boundary + 4).min(0) {
        match clamp_dir(d) {
            -1 => saw_clamped = true,
            0 => saw_unclamped = true,
            _ => unreachable!(),
        }
        p.assert_same_eq(0o4, 1, d, 0, m4(d));
    }
    assert!(saw_clamped && saw_unclamped, "boundary not straddled");

    // The sign flip of 1.0 + depth*0.1 around depth == -10 (no clamping there).
    for d in -14..=-6 {
        assert_eq!(clamp_dir(d), 0);
        p.assert_same_eq(0o4, 1, d, 0, m4(d));
    }

    // Dense sweep of the unclamped region, both signs.
    let mut rng = Rng::new(0xC000_0008);
    for _ in 0..20_000 {
        let d = rng.i32_range(boundary, -boundary.max(-2_000_000_000));
        p.assert_same_eq(0o4, 1, d, 0, m4(d));
    }
}

// --- ERRORS.md row 9 ------------------------------------------------------

#[test]
fn err_row9_depth_at_or_past_end() {
    // process_backward: start_offset >= size means `ptr > start` is false at
    // once, so the sum is 0 and the result is purely 16*flags.
    let (p, _g) = ready();
    for d in [16, 17, 18, 32, 1000, 1 << 20, i32::MAX - 1, i32::MAX] {
        p.assert_same_eq(0o2, 1, d, 0, 0);
        for f in [-1000, -16, -1, 0, 1, 16, 1000, 134_000_000, -134_000_000] {
            p.assert_same_eq(0o2, 1, d, f, 0o20i32.wrapping_mul(f));
        }
    }
    // depth == 15 is the last offset that still contributes; 16 is the first
    // that does not.
    assert_eq!(p.assert_same(0o2, 1, 15, 0), 105);
    assert_eq!(p.assert_same(0o2, 1, 16, 0), 0);
}

// --- ERRORS.md row 10 -----------------------------------------------------

#[test]
fn err_row10_negative_depth_is_ub() {
    // start_offset < 0 makes `start` point before `temp_array`, so
    // process_backward reads out of bounds of the C local. That is undefined
    // behaviour reading unrelated stack bytes, so byte-equality is NOT
    // asserted; we only exercise the path and record what each library did.
    let (p, _g) = ready();
    let mut differed = 0;
    for d in [-1, -2, -3, -4, -8, -16] {
        let cv = p.c(0o2, 1, d, 0);
        let rv = p.r(0o2, 1, d, 0);
        if cv != rv {
            differed += 1;
        }
        println!("depth={d}: C={cv} Rust={rv} (out-of-bounds read; UB in C)");
    }
    println!(
        "{differed} of 6 negative-depth probes differed — expected, this input is UB in the C"
    );
    // Both libraries must at least remain alive and keep answering correctly
    // for defined inputs afterwards.
    p.assert_same_eq(0o2, 1, 0, 0, 1438);
    p.assert_same_eq(0o3, 1, 1, 0, expect_mode3(1, 1, 0));
}

// --- ERRORS.md row 11 -----------------------------------------------------

#[test]
fn err_row11_flags_overflow() {
    let (p, _g) = ready();
    // Inside the non-overflowing range the results must match exactly.
    const LIM: c_int = 134_000_000;
    let mut rng = Rng::new(0xC000_0011);
    for _ in 0..20_000 {
        let f = rng.i32_range(-LIM, LIM);
        let d = rng.i32_range(0, 20);
        let sum = if d >= 16 {
            0
        } else {
            (d..16).map(|i| if i < 4 { [64, 128, 192, 256][i as usize] } else { i * 7 }).sum::<c_int>()
        };
        p.assert_same_eq(0o2, 1, d, f, sum.wrapping_add(16 * f));
    }
    // Beyond that, `16 * flags` is signed-overflow UB in C. gcc wraps in
    // practice; check that the two libraries agree on the wrapped value so a
    // divergence here would still be caught.
    let mut mismatches = Vec::new();
    for f in [
        LIM + 1,
        134_217_728,
        200_000_000,
        1_000_000_000,
        i32::MAX,
        i32::MAX - 1,
        -134_217_729,
        -200_000_000,
        -1_000_000_000,
        i32::MIN,
        i32::MIN + 1,
    ] {
        let cv = p.c(0o2, 1, 16, f);
        let rv = p.r(0o2, 1, 16, f);
        println!("flags={f}: C={cv} Rust={rv} (expected wrap {})", 16i32.wrapping_mul(f));
        if cv != rv {
            mismatches.push((f, cv, rv));
        }
    }
    assert!(
        mismatches.is_empty(),
        "C and Rust disagreed on the (UB) overflow wrap: {mismatches:?}"
    );
}
