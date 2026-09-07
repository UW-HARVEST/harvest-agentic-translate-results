//! Phase B — valid-path differential tests.
//! One test function per row of CONFIGS.md. Every call goes through the
//! exported `jumpnode` symbol of BOTH shared objects.

mod common;

use common::{Pair, Rng, EDGE_I32, VALID_MODES};

const SEED: u64 = 0xC0FFEE_1234_5678;

// ---------------------------------------------------------------- row 1
#[test]
fn row01_mode1_randomized() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 1);
    for _ in 0..common::iters(20_000) {
        p.assert_same(0o1, rng.next_i32(), rng.next_i32(), rng.next_i32());
    }
    for _ in 0..common::iters(20_000) {
        p.assert_same(
            0o1,
            rng.next_i32_biased(),
            rng.next_i32_biased(),
            rng.next_i32_biased(),
        );
    }
}

// ---------------------------------------------------------------- row 2
#[test]
fn row02_mode1_known_node_ids_and_depths() {
    let p = Pair::load();
    let depths = [0, 1, 2, 3, 10, -1, i32::MAX, i32::MIN];
    for node_id in [-1, 0, 1, 2, 3, 4, 5, 6, 7, 8, 100] {
        for &d in &depths {
            for f in [0, 1, -1, i32::MAX, i32::MIN] {
                p.assert_same(0o1, node_id, d, f);
            }
        }
    }
}

// ---------------------------------------------------------------- row 3
#[test]
fn row03_mode2_randomized() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..common::iters(20_000) {
        p.assert_same(0o2, rng.next_i32(), rng.next_i32(), rng.next_i32());
    }
    for _ in 0..common::iters(20_000) {
        p.assert_same(
            0o2,
            rng.next_i32_biased(),
            rng.next_i32_biased(),
            rng.next_i32_biased(),
        );
    }
}

// ---------------------------------------------------------------- row 4
#[test]
fn row04_mode2_process_backward_boundaries() {
    let p = Pair::load();
    // start_offset boundaries relative to size == 020 == 16
    let depths = [
        i32::MIN,
        i32::MIN + 1,
        -20,
        -4,
        -1,
        0,
        1,
        4,
        14,
        15,
        16,
        17,
        19,
        20,
        21,
        i32::MAX - 1,
        i32::MAX,
    ];
    let flags = [0, 1, -1, 2, 0o177, 0o200, i32::MAX, i32::MIN, 134_217_728];
    for &d in &depths {
        for &f in &flags {
            for node_id in [0, 1, 7, -1, i32::MIN, i32::MAX] {
                p.assert_same(0o2, node_id, d, f);
            }
        }
    }
}

// ---------------------------------------------------------------- row 5
#[test]
fn row05_mode4_randomized_and_scale_boundaries() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 5);
    for _ in 0..common::iters(20_000) {
        p.assert_same(0o4, rng.next_i32(), rng.next_i32(), rng.next_i32());
    }
    for &d in &[0, -1, -10, 10, -100, 100, i32::MIN, i32::MAX, i32::MIN + 1] {
        for node_id in [0, 1, 7, -1, i32::MIN, i32::MAX] {
            for f in [0, -1, i32::MAX] {
                p.assert_same(0o4, node_id, d, f);
            }
        }
    }
}

// ---------------------------------------------------------------- row 6
#[test]
fn row06_mode3_shortest_buffer() {
    let p = Pair::load();
    // "Node_0_Depth_0" -> strlen 14 -> 14*2+8 = 36
    let (c, r) = p.call(0o3, 0, 0, 0);
    assert_eq!(c, r, "mode3 (0,0,0): C={c} Rust={r}");
    assert_eq!(c, 36, "sanity: C ground truth for (3,0,0,0) should be 36");
}

// ---------------------------------------------------------------- row 7
#[test]
fn row07_mode3_positive_digit_counts_cross_product() {
    let p = Pair::load();
    let mut vals: Vec<i32> = Vec::new();
    let mut pow: i64 = 1;
    for _ in 0..10 {
        for delta in [-1i64, 0, 1] {
            let v = pow + delta;
            if v >= 0 && v <= i32::MAX as i64 {
                vals.push(v as i32);
            }
        }
        pow *= 10;
    }
    vals.push(i32::MAX);
    vals.push(i32::MAX - 1);
    for &n in &vals {
        for &d in &vals {
            for f in [0, 1, 0o177, -1] {
                p.assert_same(0o3, n, d, f);
            }
        }
    }
}

// ---------------------------------------------------------------- rows 8/9/10
fn mode3_sign_matrix(p: &Pair, n_neg: bool, d_neg: bool) {
    let mut mags: Vec<i64> = Vec::new();
    let mut pow: i64 = 1;
    for _ in 0..10 {
        for delta in [-1i64, 0, 1] {
            let v = pow + delta;
            if v >= 0 && v <= 2_147_483_648 {
                mags.push(v);
            }
        }
        pow *= 10;
    }
    mags.push(2_147_483_647);
    let cvt = |m: i64, neg: bool| -> Option<i32> {
        let v = if neg { -m } else { m };
        if v >= i32::MIN as i64 && v <= i32::MAX as i64 {
            Some(v as i32)
        } else {
            None
        }
    };
    for &mn in &mags {
        for &md in &mags {
            if let (Some(n), Some(d)) = (cvt(mn, n_neg), cvt(md, d_neg)) {
                for f in [0, 1, 0o177, -1, i32::MIN] {
                    p.assert_same(0o3, n, d, f);
                }
            }
        }
    }
}

#[test]
fn row08_mode3_negative_node_positive_depth() {
    let p = Pair::load();
    mode3_sign_matrix(&p, true, false);
}

#[test]
fn row09_mode3_positive_node_negative_depth() {
    let p = Pair::load();
    mode3_sign_matrix(&p, false, true);
}

#[test]
fn row10_mode3_both_negative() {
    let p = Pair::load();
    mode3_sign_matrix(&p, true, true);
}

// ---------------------------------------------------------------- row 11
#[test]
fn row11_mode3_longest_buffer_int_min() {
    let p = Pair::load();
    // "Node_-2147483648_Depth_-2147483648" -> strlen 34 -> 34*2+8 = 76
    let (c, r) = p.call(0o3, i32::MIN, i32::MIN, 0);
    assert_eq!(c, r, "mode3 INT_MIN/INT_MIN: C={c} Rust={r}");
    assert_eq!(c, 76, "sanity: C ground truth for (3,INT_MIN,INT_MIN,0) = 76");
    for f in [0, 1, 0o177, 0o200, -1, i32::MAX, i32::MIN] {
        p.assert_same(0o3, i32::MIN, i32::MIN, f);
    }
}

// ---------------------------------------------------------------- row 12
#[test]
fn row12_mode3_edge_values() {
    let p = Pair::load();
    for &n in &EDGE_I32 {
        for &d in &EDGE_I32 {
            for f in [0, -1, 0o177, 0o200, i32::MAX, i32::MIN] {
                p.assert_same(0o3, n, d, f);
            }
        }
    }
}

// ---------------------------------------------------------------- row 13
#[test]
fn row13_mode3_randomized_full_range() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 13);
    for _ in 0..common::iters(50_000) {
        p.assert_same(0o3, rng.next_i32(), rng.next_i32(), rng.next_i32());
    }
    for _ in 0..common::iters(50_000) {
        p.assert_same(
            0o3,
            rng.next_i32_biased(),
            rng.next_i32_biased(),
            rng.next_i32_biased(),
        );
    }
}

// ---------------------------------------------------------------- row 14
#[test]
fn row14_mode3_flag_mask_boundaries() {
    let p = Pair::load();
    let mut flags: Vec<i32> = vec![
        0,
        1,
        0o176,
        0o177,
        0o200,
        0o201,
        0o377,
        0o400,
        -1,
        -2,
        -128,
        -129,
        i32::MAX,
        i32::MIN,
        i32::MIN + 1,
    ];
    for k in 0..32 {
        flags.push(1i32.wrapping_shl(k));
        flags.push(1i32.wrapping_shl(k).wrapping_neg());
    }
    for &f in &flags {
        for (n, d) in [(0, 0), (7, 3), (-1, -1), (i32::MIN, i32::MAX), (123456, -987654)] {
            p.assert_same(0o3, n, d, f);
        }
    }
}

// ---------------------------------------------------------------- row 15
#[test]
fn row15_default_arm_exhaustive_small_modes() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 15);
    for m in -1024i32..=1024 {
        if VALID_MODES.contains(&m) {
            continue;
        }
        p.assert_same(m, rng.next_i32(), rng.next_i32(), rng.next_i32());
        p.assert_same(m, 0, 0, 0);
    }
}

// ---------------------------------------------------------------- row 16
#[test]
fn row16_default_arm_extreme_modes() {
    let p = Pair::load();
    let mut modes: Vec<i32> = vec![
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        i32::MAX - 1,
        0x10001,
        0x1_0000,
        -0x10001,
        // 0x100000001 truncated to i32 == 1 -> lands on a *valid* case; keep it
        // to prove truncation across the FFI boundary behaves the same.
        (0x1_0000_0001u64 as u32) as i32,
    ];
    for k in 0..32 {
        modes.push(1i32.wrapping_shl(k));
        modes.push(1i32.wrapping_shl(k).wrapping_neg());
    }
    for &m in &modes {
        for (n, d, f) in [(0, 0, 0), (7, 3, 0o177), (i32::MIN, i32::MIN, i32::MIN)] {
            p.assert_same(m, n, d, f);
        }
    }
}

// ---------------------------------------------------------------- row 17
#[test]
fn row17_full_four_axis_fuzz() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 17);
    for i in 0..common::iters(200_000) {
        // Bias operation_mode: half the time pick from the interesting set.
        let m = if i % 2 == 0 {
            let set = [0o1, 0o2, 0o3, 0o4, 0, 5, 6, -1, 0o10, 0o377];
            set[(rng.next_u64() % set.len() as u64) as usize]
        } else {
            rng.next_i32_biased()
        };
        p.assert_same(m, rng.next_i32_biased(), rng.next_i32_biased(), rng.next_i32_biased());
    }
}

// ---------------------------------------------------------------- row 18
#[test]
fn row18_no_hidden_state_across_calls() {
    let p = Pair::load();
    // Baseline values for each mode.
    let probes: [(i32, i32, i32, i32); 6] = [
        (0o1, 1, 3, 0),
        (0o2, 2, 4, 5),
        (0o3, 7, 42, 0o177),
        (0o4, 3, 2, 9),
        (0, 0, 0, 0),
        (5, -1, -1, -1),
    ];
    let baseline: Vec<(i32, i32)> = probes.iter().map(|&(m, n, d, f)| p.call(m, n, d, f)).collect();
    for (i, &(m, n, d, f)) in probes.iter().enumerate() {
        assert_eq!(
            baseline[i].0, baseline[i].1,
            "baseline diverges at probe {i} = jumpnode({m},{n},{d},{f})"
        );
    }
    // Interleave 5000 calls in a rotating order; results must never drift.
    for k in 0..5_000usize {
        let idx = k % probes.len();
        let (m, n, d, f) = probes[idx];
        let got = p.call(m, n, d, f);
        assert_eq!(
            got, baseline[idx],
            "state leaked: probe {idx} ({m},{n},{d},{f}) changed from {:?} to {:?} at iteration {k}",
            baseline[idx], got
        );
    }
    // And the same call repeated back to back.
    for _ in 0..1_000 {
        p.assert_same(0o3, 12345, -6789, 0o321);
    }
}
