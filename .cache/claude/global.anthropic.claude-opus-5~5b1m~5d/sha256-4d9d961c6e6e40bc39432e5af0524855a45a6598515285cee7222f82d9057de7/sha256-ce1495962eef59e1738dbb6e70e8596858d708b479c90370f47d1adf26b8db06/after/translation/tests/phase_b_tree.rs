//! Phase B — valid-path differential tests, rows 10-22 and 24 of CONFIGS.md.
//!
//! These exercise the code paths that only become live once `node_storage` is
//! populated. `initialize_test_data()` is `static` in the C, so it is reached
//! through the test shim (`tests/csupport/init_shim.c`, which #includes the
//! untouched `c_src/src/lib.c`) on the C side and through the
//! `expose_init_test_data` feature on the Rust side. Both libraries are still
//! driven exclusively via `dlsym`.

#![cfg(feature = "expose_init_test_data")]

mod common;

use common::*;
use std::ffi::c_int;

// ---------------------------------------------------------------------------
// Independent model, transcribed from c_src/src/lib.c
// ---------------------------------------------------------------------------

/// `initialize_test_data()`: (id, parent_id, value) in insertion order.
const NODES: [(c_int, c_int, f64); 7] = [
    (1, -1, 100.5),
    (2, 1, 50.25),
    (3, 1, 75.75),
    (4, 2, 25.125),
    (5, 2, 30.875),
    (6, 3, 40.0625),
    (7, 4, 12.5),
];

/// `add_node()` writes data[] = {0100, 0200, 0300, 0400}.
const DATA: [c_int; 4] = [0o100, 0o200, 0o300, 0o400];

const NODE_COUNT: usize = NODES.len();

fn find(id: c_int) -> Option<usize> {
    NODES.iter().position(|n| n.0 == id)
}

/// `safe_double_to_int()`
fn s2i(v: f64) -> c_int {
    let mut v = v;
    if v > 2147483647.0 {
        v = 2147483647.0;
    }
    if v < -2147483648.0 {
        v = -2147483648.0;
    }
    v as c_int
}

fn model_mode1(node_id: c_int, depth: c_int) -> c_int {
    let Some(mut cur) = find(node_id) else {
        return ERR_MODE1_NOT_FOUND;
    };
    let mut acc = NODES[cur].2;
    let mut i: c_int = 0;
    while i < depth && NODES[cur].1 != -1 {
        let Some(p) = find(NODES[cur].1) else { break };
        acc += NODES[p].2 * 1.5;
        cur = p;
        i += 1;
    }
    s2i(acc)
}

fn model_mode2(node_id: c_int, depth: c_int, flags: c_int) -> c_int {
    if find(node_id).is_none() {
        return ERR_MODE2_NOT_FOUND;
    }
    assert!(depth >= 0, "model is only defined for depth >= 0 (ERRORS.md row 10)");
    let mut temp = [0i32; 20];
    for i in 0..4 {
        temp[i] = DATA[i];
    }
    for i in 4..0o20 {
        temp[i] = (i as c_int).wrapping_mul(0o7);
    }
    // process_backward(temp, 16, depth): sums temp[depth..16] back to front.
    let mut sum: c_int = 0;
    let mut idx: i64 = 0o20;
    while idx > depth as i64 {
        idx -= 1;
        sum = sum.wrapping_add(temp[idx as usize]);
    }
    sum.wrapping_add((0o20 as c_int).wrapping_mul(flags))
}

fn model_mode4(node_id: c_int, depth: c_int) -> c_int {
    if find(node_id).is_none() {
        return ERR_MODE4_NOT_FOUND;
    }
    let mut acc = 0.0f64;
    for i in 0..4 {
        acc += (DATA[i] as f64).sqrt() * 2.718281828;
    }
    acc *= 1.0 + (depth as f64) * 0.1;
    let mut result = s2i(acc);
    if NODE_COUNT as c_int > 2 {
        let mut backward = 0i32;
        for k in 0..3 {
            backward = backward.wrapping_add(s2i(NODES[NODE_COUNT - 1 - k].2));
        }
        result = result.wrapping_add(backward);
    }
    result
}

fn model(m: c_int, n: c_int, d: c_int, f: c_int) -> c_int {
    match m {
        0o1 => model_mode1(n, d),
        0o2 => model_mode2(n, d, f),
        0o3 => expect_mode3(n, d, f),
        0o4 => model_mode4(n, d),
        _ => ERR_UNKNOWN_MODE,
    }
}

/// Loads the pair, initializes BOTH libraries, and serializes against other
/// state-touching tests.
fn ready() -> (&'static Pair, std::sync::MutexGuard<'static, ()>) {
    let g = state_lock();
    let p = Pair::with_init();
    p.init_both();
    (p, g)
}

const ALL_IDS: [c_int; 7] = [1, 2, 3, 4, 5, 6, 7];

// --- row 10 ---------------------------------------------------------------

#[test]
fn cfg_row10_mode1_root() {
    let (p, _g) = ready();
    // id 1 is the root: parent_id == -1, so the loop never runs regardless of depth.
    for d in 0..=8 {
        p.assert_same_eq(0o1, 1, d, 0, model_mode1(1, d));
    }
    // value 100.5 truncates to 100 for every depth.
    for d in 0..=8 {
        p.assert_same_eq(0o1, 1, d, 0, 100);
    }
}

// --- row 11 ---------------------------------------------------------------

#[test]
fn cfg_row11_mode1_depth1_nodes() {
    let (p, _g) = ready();
    let mut rng = Rng::new(0x2222_0011);
    for id in [2, 3] {
        for d in 0..=8 {
            let f = rng.i32_any();
            p.assert_same_eq(0o1, id, d, f, model_mode1(id, d));
        }
    }
    // Spot-check the arithmetic: node 2 (50.25) + root 100.5*1.5 = 201.0 -> 201
    p.assert_same_eq(0o1, 2, 1, 0, 201);
    p.assert_same_eq(0o1, 2, 0, 0, 50);
}

// --- row 12 ---------------------------------------------------------------

#[test]
fn cfg_row12_mode1_deep_nodes() {
    let (p, _g) = ready();
    let mut rng = Rng::new(0x2222_0012);
    for id in [4, 5, 6, 7] {
        for d in 0..=8 {
            let f = rng.i32_any();
            p.assert_same_eq(0o1, id, d, f, model_mode1(id, d));
        }
    }
    // Node 7 -> 4 -> 2 -> 1 : chain length 3. Verify the loop stops on the
    // counter for d < 3 and on parent_id == -1 for d >= 3.
    let at3 = p.assert_same(0o1, 7, 3, 0);
    for d in 3..=20 {
        p.assert_same_eq(0o1, 7, d, 0, at3);
    }
    assert_ne!(at3, p.assert_same(0o1, 7, 2, 0), "depth 2 must differ from 3");
}

// --- row 13 ---------------------------------------------------------------

#[test]
fn cfg_row13_mode1_huge_depth() {
    let (p, _g) = ready();
    let mut rng = Rng::new(0x2222_0013);
    for &id in &ALL_IDS {
        for d in [i32::MAX, i32::MAX - 1, 1_000_000, 100_000, 1000] {
            p.assert_same_eq(0o1, id, d, 0, model_mode1(id, d));
        }
        for _ in 0..2000 {
            let d = rng.i32_range(0, i32::MAX);
            let f = rng.i32_any();
            p.assert_same_eq(0o1, id, d, f, model_mode1(id, d));
        }
    }
}

// --- row 14 ---------------------------------------------------------------

#[test]
fn cfg_row14_mode1_missing_id() {
    let (p, _g) = ready();
    let mut rng = Rng::new(0x2222_0014);
    for n in [0, 8, 9, 100, -1, -7, i32::MIN, i32::MAX] {
        for d in [0, 1, 5, i32::MAX, i32::MIN] {
            p.assert_same_eq(0o1, n, d, 0, ERR_MODE1_NOT_FOUND);
        }
    }
    for _ in 0..20_000 {
        let n = rng.i32_interesting();
        if (1..=7).contains(&n) {
            continue;
        }
        let d = rng.i32_interesting();
        let f = rng.i32_interesting();
        p.assert_same_eq(0o1, n, d, f, ERR_MODE1_NOT_FOUND);
    }
}

// --- row 15 ---------------------------------------------------------------

#[test]
fn cfg_row15_mode2_depth_in_range() {
    let (p, _g) = ready();
    for &id in &ALL_IDS {
        for d in 0..16 {
            p.assert_same_eq(0o2, id, d, 0, model_mode2(id, d, 0));
        }
    }
    // depth 0 sums the whole 16-element window: data{64,128,192,256}=640 plus
    // sum(i*7, i=4..15)=798  =>  1438.
    p.assert_same_eq(0o2, 1, 0, 0, 1438);
    // depth 15 leaves only temp[15] = 15*7 = 105.
    p.assert_same_eq(0o2, 1, 15, 0, 105);
}

// --- row 16 ---------------------------------------------------------------

#[test]
fn cfg_row16_mode2_depth_past_end() {
    let (p, _g) = ready();
    for d in [16, 17, 18, 100, 1000, i32::MAX] {
        p.assert_same_eq(0o2, 1, d, 0, 0);
        p.assert_same_eq(0o2, 3, d, 0, model_mode2(3, d, 0));
        for f in [-5, -1, 0, 1, 7, 1000] {
            p.assert_same_eq(0o2, 5, d, f, 0o20 * f);
        }
    }
}

// --- row 17 ---------------------------------------------------------------

#[test]
fn cfg_row17_mode2_flags() {
    let (p, _g) = ready();
    let mut rng = Rng::new(0x2222_0017);
    // Keep 16*flags (+ at most 1438) inside int range: see ERRORS.md row 11.
    const LIM: c_int = 134_000_000;
    for &id in &ALL_IDS {
        for d in 0..=16 {
            for _ in 0..300 {
                let f = rng.i32_range(-LIM, LIM);
                p.assert_same_eq(0o2, id, d, f, model_mode2(id, d, f));
            }
        }
    }
}

// --- row 18 ---------------------------------------------------------------

#[test]
fn cfg_row18_mode3_state_independent() {
    let (p, _g) = ready();
    let mut rng = Rng::new(0x2222_0018);
    for _ in 0..50_000 {
        let n = rng.i32_interesting();
        let d = rng.i32_interesting();
        let f = rng.i32_interesting();
        // Case 0003 never reads node_storage, so the populated-state answer
        // must equal the pristine-state formula.
        p.assert_same_eq(0o3, n, d, f, expect_mode3(n, d, f));
    }
}

// --- row 19 ---------------------------------------------------------------

#[test]
fn cfg_row19_mode4_tree() {
    let (p, _g) = ready();
    for &id in &ALL_IDS {
        for d in 0..=8 {
            p.assert_same_eq(0o4, id, d, 0, model_mode4(id, d));
        }
    }
    // The result must not depend on WHICH node was found (data[] is identical
    // for every node), only on depth.
    for d in 0..=8 {
        let base = p.assert_same(0o4, 1, d, 0);
        for &id in &ALL_IDS {
            p.assert_same_eq(0o4, id, d, 0, base);
        }
    }
    // node_count == 7 > 2, so the backward scan adds trunc(12.5)+trunc(40.0625)
    // +trunc(30.875) = 12+40+30 = 82.
    let with_scan = p.assert_same(0o4, 1, 0, 0);
    let sqrt_part = s2i((64f64.sqrt() + 128f64.sqrt() + 192f64.sqrt() + 256f64.sqrt())
        * 2.718281828
        * 0.0
        + {
            let mut a = 0.0;
            for d in DATA {
                a += (d as f64).sqrt() * 2.718281828;
            }
            a
        });
    assert_eq!(with_scan, sqrt_part + 82, "backward node_storage scan == 82");
}

// --- row 20 ---------------------------------------------------------------

#[test]
fn cfg_row20_mode4_scaling_saturation() {
    let (p, _g) = ready();
    let mut rng = Rng::new(0x2222_0020);
    // 1.0 + depth*0.1 goes hugely positive / hugely negative, driving
    // safe_double_to_int into both clamps, and passes through zero near -10.
    for d in [
        i32::MIN,
        i32::MIN + 1,
        -2_000_000_000,
        -1_000_000,
        -100,
        -20,
        -12,
        -11,
        -10,
        -9,
        -5,
        -1,
        0,
        1,
        10,
        100,
        1_000_000,
        2_000_000_000,
        i32::MAX - 1,
        i32::MAX,
    ] {
        p.assert_same_eq(0o4, 1, d, 0, model_mode4(1, d));
    }
    for _ in 0..50_000 {
        let d = rng.i32_interesting();
        let id = rng.pick(&ALL_IDS);
        let f = rng.i32_interesting();
        p.assert_same_eq(0o4, id, d, f, model_mode4(id, d));
    }
    // Both clamps must actually have been exercised. After saturation the
    // `result += backward_sum` (82) itself overflows, and both libraries wrap
    // identically.
    assert_eq!(
        p.assert_same(0o4, 1, i32::MAX, 0),
        i32::MAX.wrapping_add(82),
        "upper clamp 2147483647 then +82 wraps"
    );
    assert_eq!(
        p.assert_same(0o4, 1, i32::MIN, 0),
        i32::MIN.wrapping_add(82),
        "lower clamp -2147483648 then +82"
    );
}

// --- row 21 ---------------------------------------------------------------

#[test]
fn cfg_row21_default_with_state() {
    let (p, _g) = ready();
    let mut rng = Rng::new(0x2222_0021);
    let mut n_checked = 0;
    while n_checked < 20_000 {
        let m = rng.i32_interesting();
        if (1..=4).contains(&m) {
            continue;
        }
        let n = rng.i32_interesting();
        let d = rng.i32_interesting();
        let f = rng.i32_interesting();
        p.assert_same_eq(m, n, d, f, ERR_UNKNOWN_MODE);
        n_checked += 1;
    }
}

// --- row 22 ---------------------------------------------------------------

#[test]
fn cfg_row22_init_idempotent() {
    let g = state_lock();
    let p = Pair::with_init();

    // A fixed script whose answers depend on node_count and node_storage.
    let script: Vec<(c_int, c_int, c_int, c_int)> = vec![
        (0o1, 7, 3, 0),
        (0o1, 1, 5, 0),
        (0o2, 4, 0, 3),
        (0o2, 4, 9, -3),
        (0o4, 6, 2, 0),
        (0o4, 2, 0, 0),
        (0o3, 42, -42, 99),
    ];

    let mut baseline: Option<Vec<c_int>> = None;
    for round in 1..=5 {
        p.init_both();
        let got: Vec<c_int> = script
            .iter()
            .map(|&(m, n, d, f)| p.assert_same(m, n, d, f))
            .collect();
        match &baseline {
            None => baseline = Some(got),
            Some(b) => assert_eq!(
                &got, b,
                "results changed after {round} init calls: node_count was not reset to 7"
            ),
        }
    }
    // And they match the model (node_count pinned at 7, not 7*rounds).
    p.init_both();
    for &(m, n, d, f) in &script {
        p.assert_same_eq(m, n, d, f, model(m, n, d, f));
    }
    drop(g);
}

// --- contiguous soak sweeps over the populated tree -----------------------

#[test]
fn soak_tree_contiguous_depth_sweeps() {
    let (p, _g) = ready();
    // Mode 1 and 4 over a dense contiguous depth band, both signs, every id.
    for &id in &ALL_IDS {
        for d in -2000..=2000 {
            p.assert_same_eq(0o1, id, d, 0, model_mode1(id, d));
            p.assert_same_eq(0o4, id, d, 0, model_mode4(id, d));
        }
    }
    // Mode 2 over every defined offset (negative depth is UB, ERRORS.md row 10).
    for &id in &ALL_IDS {
        for d in 0..=64 {
            p.assert_same_eq(0o2, id, d, 0, model_mode2(id, d, 0));
        }
    }
}

#[test]
fn soak_tree_mode4_scale_band() {
    // The interesting region of `1.0 + depth*0.1` for case 0004: the sign flip
    // at depth == -10 and the whole small-magnitude band, exhaustively.
    let (p, _g) = ready();
    for d in -100_000..=100_000 {
        p.assert_same_eq(0o4, 1, d, 0, model_mode4(1, d));
    }
}

#[test]
fn soak_tree_mode1_random_ids_and_depths() {
    let (p, _g) = ready();
    let mut rng = Rng::new(0x2222_5555);
    for _ in 0..300_000 {
        let n = if rng.next_u64() % 3 == 0 {
            rng.pick(&ALL_IDS)
        } else {
            rng.i32_interesting()
        };
        let d = rng.i32_interesting();
        let f = rng.i32_interesting();
        p.assert_same_eq(0o1, n, d, f, model_mode1(n, d));
        p.assert_same_eq(0o4, n, d, f, model_mode4(n, d));
    }
}

#[test]
fn soak_tree_mode2_full_offset_flag_grid() {
    let (p, _g) = ready();
    const LIM: c_int = 134_000_000;
    let mut rng = Rng::new(0x2222_6666);
    for &id in &ALL_IDS {
        for d in 0..=17 {
            for f in [-LIM, -1000, -16, -1, 0, 1, 16, 1000, LIM] {
                p.assert_same_eq(0o2, id, d, f, model_mode2(id, d, f));
            }
            for _ in 0..200 {
                let f = rng.i32_range(-LIM, LIM);
                p.assert_same_eq(0o2, id, d, f, model_mode2(id, d, f));
            }
        }
    }
}

// --- row 24 ---------------------------------------------------------------

#[test]
fn cfg_row24_tree_random_property() {
    let (p, _g) = ready();
    let mut rng = Rng::new(0x2222_0024);
    for _ in 0..200_000 {
        let m = match rng.next_u64() % 6 {
            0 => 0o1,
            1 => 0o2,
            2 => 0o3,
            3 => 0o4,
            4 => rng.i32_range(-6, 10),
            _ => rng.i32_interesting(),
        };
        // Prefer real ids half the time so the "found" paths dominate.
        let n = if rng.next_u64() % 2 == 0 {
            rng.pick(&ALL_IDS)
        } else {
            rng.i32_interesting()
        };
        let mut d = rng.i32_interesting();
        let f = rng.i32_interesting();
        if m == 0o2 && d < 0 {
            // ERRORS.md row 10: negative depth is out-of-bounds UB in the C.
            d = -d.max(i32::MIN + 1);
            if d < 0 {
                d = 0;
            }
        }
        if m == 0o2 {
            p.assert_same_eq(m, n, d, f, model(m, n, d, f));
        } else {
            p.assert_same_eq(m, n, d, f, model(m, n, d, f));
        }
    }
}
