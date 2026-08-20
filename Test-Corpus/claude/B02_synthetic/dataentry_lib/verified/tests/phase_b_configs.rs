//! Phase B — valid-path differential tests, one test per row of CONFIGS.md.
//!
//! Every test drives BOTH shared objects through the exported `dataentry`
//! symbol and compares the returned `int` bit-for-bit. Each row uses many
//! randomized inputs from a fixed-seed PRNG plus its boundary values.

mod common;

use common::{Pair, Rng, SEED};

const ITERS: usize = 400;

// ---------------------------------------------------------------------------
// mode 1: dataentry -> create_entries -> (sprintf/strcpy) -> find_entry -> free
// ---------------------------------------------------------------------------

#[test]
fn cfg_01_mode1_default_count_hit() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 1);
    // count defaults to 5 for param1 <= 0; ids are 100..104.
    for p1 in [0, -1, -5, i32::MIN, -12345] {
        for p2 in 0..5 {
            let p3 = rng.next_i32();
            p.assert_same_and_eq("cfg01", 1, p1, p2, p3, (100 + p2) * 10);
        }
    }
    for _ in 0..ITERS {
        let p1 = rng.range(i32::MIN, 0);
        let p2 = rng.range(0, 4);
        let p3 = rng.mixed_i32();
        p.assert_same_and_eq("cfg01/rand", 1, p1, p2, p3, (100 + p2) * 10);
    }
}

#[test]
fn cfg_02_mode1_default_count_miss() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 2);
    for p2 in [-1, 5, 6, 100, -100] {
        p.assert_same_and_eq("cfg02", 1, 0, p2, rng.next_i32(), -2);
    }
    for _ in 0..ITERS {
        let p1 = rng.range(i32::MIN, 0);
        // any param2 outside 0..=4
        let p2 = if rng.next_u64() % 2 == 0 {
            rng.range(5, 1_000_000)
        } else {
            rng.range(-1_000_000, -1)
        };
        let p3 = rng.mixed_i32();
        p.assert_same_and_eq("cfg02/rand", 1, p1, p2, p3, -2);
    }
}

#[test]
fn cfg_03_mode1_single_element() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 3);
    p.assert_same_and_eq("cfg03", 1, 1, 0, 0, 1000);
    for _ in 0..ITERS {
        let p3 = rng.mixed_i32();
        p.assert_same_and_eq("cfg03/rand", 1, 1, 0, p3, 1000);
        // one-past on a single-element table
        p.assert_same_and_eq("cfg03/miss", 1, 1, 1, p3, -2);
    }
}

#[test]
fn cfg_04_mode1_first_element() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..ITERS {
        let count = rng.range(2, 64);
        let p3 = rng.mixed_i32();
        p.assert_same_and_eq("cfg04", 1, count, 0, p3, 1000);
    }
}

#[test]
fn cfg_05_mode1_last_element() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 5);
    for _ in 0..ITERS {
        let count = rng.range(2, 64);
        let p2 = count - 1;
        let p3 = rng.mixed_i32();
        p.assert_same_and_eq("cfg05", 1, count, p2, p3, (100 + p2) * 10);
    }
}

#[test]
fn cfg_06_mode1_middle_element() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 6);
    for _ in 0..ITERS {
        let count = rng.range(3, 64);
        let p2 = rng.range(1, count - 2);
        let p3 = rng.mixed_i32();
        p.assert_same_and_eq("cfg06", 1, count, p2, p3, (100 + p2) * 10);
    }
}

#[test]
fn cfg_07_mode1_one_past_end() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..ITERS {
        let count = rng.range(2, 64);
        let p3 = rng.mixed_i32();
        p.assert_same_and_eq("cfg07", 1, count, count, p3, -2);
        p.assert_same_and_eq("cfg07/+1", 1, count, count + 1, p3, -2);
    }
}

#[test]
fn cfg_08_mode1_negative_target() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..ITERS {
        let count = rng.range(2, 64);
        let p2 = rng.range(-4096, -1);
        let p3 = rng.mixed_i32();
        p.assert_same_and_eq("cfg08", 1, count, p2, p3, -2);
    }
}

#[test]
fn cfg_09_mode1_multi_page_alloc() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 9);
    // 1..4096 entries => 40 B .. 160 KB, names with 3 and 4 decimal digits.
    for _ in 0..ITERS {
        let count = rng.range(1, 4096);
        let p2 = rng.range(0, count - 1);
        let p3 = rng.mixed_i32();
        p.assert_same_and_eq("cfg09", 1, count, p2, p3, (100 + p2) * 10);
    }
    // digit-width boundaries of sprintf("Entry_%d") for base_id 100
    for count in [900, 901, 9900, 9901, 4096] {
        for p2 in [0, count - 1, count / 2, 899, 900, 8999, 9000] {
            if p2 >= 0 && p2 < count {
                p.assert_same_and_eq("cfg09/widths", 1, count, p2, 7, (100 + p2) * 10);
            }
        }
    }
}

#[test]
fn cfg_10_mode1_large_alloc() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 10);
    // 100_000..2_000_000 entries => 4 MB .. 80 MB per call; 5-7 digit names.
    for _ in 0..8 {
        let count = rng.range(100_000, 2_000_000);
        let p2 = rng.range(0, count - 1);
        p.assert_same_and_eq("cfg10/hit", 1, count, p2, rng.mixed_i32(), (100 + p2) * 10);
        p.assert_same_and_eq("cfg10/miss", 1, count, count, rng.mixed_i32(), -2);
    }
}

#[test]
fn cfg_11_mode1_target_wraparound() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 11);
    for p2 in [
        i32::MAX,
        i32::MIN,
        i32::MAX - 99,
        i32::MAX - 100,
        i32::MIN + 100,
        -100,
        -101,
        -99,
    ] {
        for p1 in [0, 1, 5, 17] {
            p.assert_same("cfg11", 1, p1, p2, rng.mixed_i32());
        }
    }
    for _ in 0..ITERS {
        let p1 = rng.range(1, 32);
        let p2 = if rng.next_u64() % 2 == 0 {
            i32::MAX - rng.range(0, 200)
        } else {
            i32::MIN + rng.range(0, 200)
        };
        p.assert_same("cfg11/rand", 1, p1, p2, rng.mixed_i32());
    }
}

#[test]
fn cfg_12_mode1_random_all() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 12);
    for _ in 0..(ITERS * 8) {
        let p1 = rng.range(-8, 64);
        let p2 = rng.range(-8, 72);
        let p3 = rng.mixed_i32();
        p.assert_same("cfg12", 1, p1, p2, p3);
    }
}

// ---------------------------------------------------------------------------
// mode 2: dataentry -> create_entries -> modify_entries -> free
// ---------------------------------------------------------------------------

/// Reference model for mode 2 (used only to sanity-check the C oracle values;
/// the authoritative comparison is always C vs Rust).
fn model_mode2(p1: i32, p2: i32, p3: i32) -> i32 {
    let count = if p1 > 0 { p1 } else { 3 };
    let mut total: i32 = 0;
    for i in 0..count {
        let v = (200i32.wrapping_add(i)).wrapping_mul(10);
        if v != 0 {
            total = total.wrapping_add(v.wrapping_mul(p2));
        }
    }
    if total != 0 {
        total = total.wrapping_add(p3);
    }
    total
}

#[test]
fn cfg_13_mode2_default_count() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 13);
    for p1 in [0, -1, -7, i32::MIN] {
        for p2 in [1, 2, 3, -1, -2, 10] {
            let p3 = rng.range(-1000, 1000);
            p.assert_same_and_eq("cfg13", 2, p1, p2, p3, model_mode2(p1, p2, p3));
        }
    }
    for _ in 0..ITERS {
        let p1 = rng.range(i32::MIN, 0);
        let mut p2 = rng.range(-64, 64);
        if p2 == 0 {
            p2 = 1;
        }
        let p3 = rng.mixed_i32();
        p.assert_same_and_eq("cfg13/rand", 2, p1, p2, p3, model_mode2(p1, p2, p3));
    }
}

#[test]
fn cfg_14_mode2_single_element() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 14);
    for _ in 0..ITERS {
        let p2 = rng.mixed_i32();
        let p3 = rng.mixed_i32();
        p.assert_same_and_eq("cfg14", 2, 1, p2, p3, model_mode2(1, p2, p3));
    }
}

#[test]
fn cfg_15_mode2_zero_multiplier() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 15);
    for _ in 0..ITERS {
        let p1 = rng.range(2, 64);
        let p3 = rng.mixed_i32();
        // total becomes 0 => the `if` is false => param3 is NOT added.
        p.assert_same_and_eq("cfg15", 2, p1, 0, p3, 0);
    }
    for p1 in [-1, 0, 1, 2, 3, 10] {
        p.assert_same_and_eq("cfg15/fixed", 2, p1, 0, 12345, 0);
    }
}

#[test]
fn cfg_16_mode2_identity_multiplier() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 16);
    for _ in 0..ITERS {
        let p1 = rng.range(2, 64);
        let p3 = rng.mixed_i32();
        p.assert_same_and_eq("cfg16", 2, p1, 1, p3, model_mode2(p1, 1, p3));
    }
}

#[test]
fn cfg_17_mode2_negative_multiplier() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 17);
    for _ in 0..ITERS {
        let p1 = rng.range(2, 64);
        let p2 = rng.range(-100_000, -1);
        let p3 = rng.mixed_i32();
        p.assert_same("cfg17", 2, p1, p2, p3);
        p.assert_same_and_eq("cfg17/-1", 2, p1, -1, p3, model_mode2(p1, -1, p3));
    }
}

#[test]
fn cfg_18_mode2_multiplier_wraparound() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 18);
    for _ in 0..(ITERS * 4) {
        let p1 = rng.range(2, 64);
        let p2 = rng.next_i32();
        let p3 = rng.mixed_i32();
        p.assert_same("cfg18", 2, p1, p2, p3);
    }
    for p2 in [i32::MAX, i32::MIN, 1 << 30, -(1 << 30), 0x1234_5678] {
        for p1 in [1, 2, 3, 7, 64] {
            p.assert_same("cfg18/fixed", 2, p1, p2, 999);
        }
    }
}

#[test]
fn cfg_19_mode2_param3_extremes() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 19);
    for p3 in [i32::MAX, i32::MIN, 0, 1, -1] {
        for _ in 0..64 {
            let p1 = rng.range(1, 64);
            let mut p2 = rng.range(-8, 8);
            if p2 == 0 {
                p2 = 3;
            }
            p.assert_same_and_eq("cfg19", 2, p1, p2, p3, model_mode2(p1, p2, p3));
        }
    }
}

#[test]
fn cfg_20_mode2_large_count() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 20);
    for _ in 0..8 {
        let p1 = rng.range(10_000, 1_000_000);
        let p2 = rng.range(-8, 8);
        let p3 = rng.mixed_i32();
        p.assert_same_and_eq("cfg20", 2, p1, p2, p3, model_mode2(p1, p2, p3));
    }
}

#[test]
fn cfg_21_mode2_random_all() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 21);
    for _ in 0..(ITERS * 8) {
        let p1 = rng.range(-8, 256);
        let p2 = rng.mixed_i32();
        let p3 = rng.mixed_i32();
        p.assert_same("cfg21", 2, p1, p2, p3);
    }
}

// ---------------------------------------------------------------------------
// mode 3: dataentry -> calculate_lookup
// ---------------------------------------------------------------------------

const LOOKUP: [[i32; 3]; 4] = [[10, 20, 30], [40, 50, 60], [70, 80, 90], [100, 110, 120]];

#[test]
fn cfg_22_mode3_all_cells() {
    let p = Pair::load();
    for row in 0..4i32 {
        for col in 0..3i32 {
            let expect = LOOKUP[row as usize][col as usize] * 2;
            p.assert_same_and_eq("cfg22", 3, row, col, 0, expect);
        }
    }
}

#[test]
fn cfg_23_mode3_random_param3() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 23);
    for _ in 0..ITERS {
        for row in 0..4i32 {
            for col in 0..3i32 {
                let p3 = rng.next_i32();
                let expect = (LOOKUP[row as usize][col as usize] * 2).wrapping_add(p3);
                p.assert_same_and_eq("cfg23", 3, row, col, p3, expect);
            }
        }
    }
}

#[test]
fn cfg_24_mode3_param3_extremes() {
    let p = Pair::load();
    for p3 in [i32::MAX, i32::MIN, 1, -1, 0, i32::MAX - 239, i32::MIN + 239] {
        for row in 0..4i32 {
            for col in 0..3i32 {
                let expect = (LOOKUP[row as usize][col as usize] * 2).wrapping_add(p3);
                p.assert_same_and_eq("cfg24", 3, row, col, p3, expect);
            }
        }
    }
}

#[test]
fn cfg_25_mode3_corners() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 25);
    for (row, col) in [(0, 0), (0, 2), (3, 0), (3, 2)] {
        for _ in 0..ITERS {
            let p3 = rng.mixed_i32();
            let expect = (LOOKUP[row as usize][col as usize] * 2).wrapping_add(p3);
            p.assert_same_and_eq("cfg25", 3, row, col, p3, expect);
        }
    }
}

// ---------------------------------------------------------------------------
// default branch: dataentry -> process_name (strcpy/strlen)
// ---------------------------------------------------------------------------

#[test]
fn cfg_26_default_mode_zero() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 26);
    for _ in 0..ITERS {
        let p1 = rng.range(-10_000, 10_000);
        p.assert_same_and_eq("cfg26", 0, p1, rng.next_i32(), rng.next_i32(), 8 * p1);
    }
}

#[test]
fn cfg_27_default_other_modes() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 27);
    for mode in [-1, 4, 5, 1000, i32::MIN, i32::MAX, -1000, 6, 7] {
        for _ in 0..64 {
            let p1 = rng.range(-100_000, 100_000);
            p.assert_same_and_eq(
                "cfg27",
                mode,
                p1,
                rng.next_i32(),
                rng.next_i32(),
                8i32.wrapping_mul(p1),
            );
        }
    }
}

#[test]
fn cfg_28_default_param1_extremes() {
    let p = Pair::load();
    for p1 in [
        0,
        1,
        -1,
        i32::MAX,
        i32::MIN,
        268_435_456,
        -268_435_456,
        268_435_455,
    ] {
        for mode in [0, 4, -1, i32::MIN] {
            p.assert_same_and_eq("cfg28", mode, p1, 0, 0, 8i32.wrapping_mul(p1));
        }
    }
}

#[test]
fn cfg_29_default_random_all() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 29);
    for _ in 0..(ITERS * 8) {
        let mut mode = rng.next_i32();
        if mode == 1 || mode == 2 || mode == 3 {
            mode = 0;
        }
        let p1 = rng.mixed_i32();
        let p2 = rng.mixed_i32();
        let p3 = rng.mixed_i32();
        p.assert_same_and_eq("cfg29", mode, p1, p2, p3, 8i32.wrapping_mul(p1));
    }
}

// ---------------------------------------------------------------------------
// Cross-branch fuzz
// ---------------------------------------------------------------------------

#[test]
fn cfg_30_global_fuzz_small_modes() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 30);
    for _ in 0..20_000 {
        let mode = rng.range(-4, 8);
        let p1 = rng.range(-16, 96);
        let p2 = rng.mixed_i32();
        let p3 = rng.mixed_i32();
        p.assert_same("cfg30", mode, p1, p2, p3);
    }
}

#[test]
fn cfg_31_global_fuzz_full_range_mode() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 31);
    for _ in 0..20_000 {
        let mode = rng.mixed_i32();
        // keep allocation sizes bounded for modes 1/2 while still covering
        // the `param1 <= 0` default-count path
        let p1 = rng.range(-64, 512);
        let p2 = rng.mixed_i32();
        let p3 = rng.mixed_i32();
        p.assert_same("cfg31", mode, p1, p2, p3);
    }
}

// ---------------------------------------------------------------------------
// Repeated-call lifecycle (rows 32-33)
// ---------------------------------------------------------------------------

/// Resident set size of the test process, in KiB (Linux).
fn rss_kib() -> u64 {
    let statm = std::fs::read_to_string("/proc/self/statm").unwrap_or_default();
    let pages: u64 = statm
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    pages * 4
}

/// Row 32: both implementations must allocate AND free per call — a missing
/// `free`/`dealloc` shows up as unbounded RSS growth over many calls, and the
/// returned value must not drift (no hidden per-call state).
#[test]
fn cfg_32_repeated_call_lifecycle() {
    let p = Pair::load();
    let count = 512; // 20 KiB per call; 50k calls would leak ~1 GiB
    let first_c = p.call_c(1, count, 7, 0);
    let first_r = p.call_rust(1, count, 7, 0);
    assert_eq!(first_c, first_r);

    // warm the allocators up so the baseline is stable
    for _ in 0..2000 {
        p.call_c(1, count, 7, 0);
        p.call_rust(1, count, 7, 0);
        p.call_c(2, count, 3, 1);
        p.call_rust(2, count, 3, 1);
    }

    let base = rss_kib();
    for _ in 0..50_000 {
        assert_eq!(p.call_c(1, count, 7, 0), first_c, "C result drifted");
        assert_eq!(p.call_rust(1, count, 7, 0), first_r, "Rust result drifted");
        let c2 = p.call_c(2, count, 3, 1);
        let r2 = p.call_rust(2, count, 3, 1);
        assert_eq!(c2, r2, "mode 2 diverged on repeat");
    }
    let after = rss_kib();
    let growth = after.saturating_sub(base);
    assert!(
        growth < 32 * 1024,
        "RSS grew by {growth} KiB over 50k allocating calls — a leak in one of the \
         implementations (C={base} KiB -> {after} KiB)"
    );
}

/// Row 33: interleave every mode so any residual state in one branch would
/// perturb another; results must stay identical to the isolated calls.
#[test]
fn cfg_33_interleaved_modes_are_stateless() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 33);
    let probes: Vec<(i32, i32, i32, i32)> = (0..64)
        .map(|_| {
            (
                rng.range(-2, 5),
                rng.range(-4, 48),
                rng.mixed_i32(),
                rng.mixed_i32(),
            )
        })
        .collect();

    // baseline: each probe measured on a freshly loaded pair of libraries
    let baseline: Vec<i32> = probes
        .iter()
        .map(|&(m, a, b, c)| {
            let fresh = Pair::load();
            let cv = fresh.call_c(m, a, b, c);
            let rv = fresh.call_rust(m, a, b, c);
            assert_eq!(cv, rv, "fresh-load divergence for ({m},{a},{b},{c})");
            cv
        })
        .collect();

    // now hammer them in interleaved order many times over
    for round in 0..50 {
        for (i, &(m, a, b, c)) in probes.iter().enumerate() {
            let v = p.assert_same("cfg33", m, a, b, c);
            assert_eq!(
                v, baseline[i],
                "round {round}: dataentry({m},{a},{b},{c}) changed after interleaving"
            );
        }
    }
}

/// Row 34: reentrancy. Neither implementation may keep mutable global state
/// (`lookup_table` is read-only `static` in C), so concurrent callers must see
/// exactly the same results as the single-threaded runs.
#[test]
fn cfg_34_concurrent_reentrancy() {
    let mut rng = Rng::new(SEED ^ 34);
    let cases: Vec<(i32, i32, i32, i32)> = (0..256)
        .map(|_| {
            (
                rng.range(-2, 5),
                rng.range(-4, 64),
                rng.mixed_i32(),
                rng.mixed_i32(),
            )
        })
        .collect();

    // single-threaded reference, taken through both .so exports
    let p = Pair::load();
    let reference: Vec<i32> = cases
        .iter()
        .map(|&(m, a, b, c)| p.assert_same("cfg34/ref", m, a, b, c))
        .collect();
    drop(p);

    let cases = std::sync::Arc::new(cases);
    let reference = std::sync::Arc::new(reference);
    let mut handles = Vec::new();
    for t in 0..8 {
        let cases = cases.clone();
        let reference = reference.clone();
        handles.push(std::thread::spawn(move || {
            let p = Pair::load();
            for _round in 0..4 {
                for (i, &(m, a, b, c)) in cases.iter().enumerate() {
                    let v = p.assert_same("cfg34", m, a, b, c);
                    assert_eq!(
                        v, reference[i],
                        "thread {t}: dataentry({m},{a},{b},{c}) = {v}, single-threaded gave {}",
                        reference[i]
                    );
                }
            }
        }));
    }
    for h in handles {
        h.join().expect("worker thread panicked");
    }
}
