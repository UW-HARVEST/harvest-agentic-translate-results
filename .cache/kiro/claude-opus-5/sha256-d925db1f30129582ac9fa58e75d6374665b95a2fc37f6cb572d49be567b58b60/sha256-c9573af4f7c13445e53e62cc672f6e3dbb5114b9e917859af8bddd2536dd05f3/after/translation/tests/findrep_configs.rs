// Phase B — valid-path differential tests, rows C13..C30 of CONFIGS.md.
//
// `findrep` is the only function in the public header, but its behaviour is a
// function of (params × hidden static state). These tests drive both the
// dispatch axes and the hidden-state gates, and interleave the low-level
// exports with `findrep` so the composed pipeline is exercised the way a real
// consumer would.

mod common;
use common::*;

const SEED: u64 = 0x5EED_1234_ABCD_0001;

/// The normalization/gate boundary values used for cross-products.
const BOUNDS: [i32; 11] = [
    i32::MIN,
    -1,
    0,
    1,
    63,
    64,
    65,
    510,
    511,
    512,
    i32::MAX,
];

fn cmp_findrep(c: &Api<'_>, r: &Api<'_>, p: (i32, i32, i32, i32), ctx: &str) {
    let cv = unsafe { (c.findrep)(p.0, p.1, p.2, p.3) };
    let rv = unsafe { (r.findrep)(p.0, p.1, p.2, p.3) };
    assert_eq!(
        cv, rv,
        "{ctx}: findrep({}, {}, {}, {}) C={cv} Rust={rv}",
        p.0, p.1, p.2, p.3
    );
    // Also confirm the observable state agrees afterwards, so a divergence in
    // the hidden statics is caught at the call where it happens rather than
    // several calls later. add(0,0) / multiply(1,1) are state-preserving apart
    // from operation_count, which both must bump identically.
    let ca = unsafe { (c.add_to_accumulator)(0, 0) };
    let ra = unsafe { (r.add_to_accumulator)(0, 0) };
    assert_eq!(ca, ra, "{ctx}: accumulator after findrep C={ca} Rust={ra}");
    let cm = unsafe { (c.multiply_with_multiplier)(1, 1) };
    let rm = unsafe { (r.multiply_with_multiplier)(1, 1) };
    assert_eq!(cm, rm, "{ctx}: multiplier after findrep C={cm} Rust={rm}");
}

/// findrep only, no state probing (used for long sequences).
fn cmp_findrep_bare(c: &Api<'_>, r: &Api<'_>, p: (i32, i32, i32, i32), ctx: &str) {
    let cv = unsafe { (c.findrep)(p.0, p.1, p.2, p.3) };
    let rv = unsafe { (r.findrep)(p.0, p.1, p.2, p.3) };
    assert_eq!(
        cv, rv,
        "{ctx}: findrep({}, {}, {}, {}) C={cv} Rust={rv}",
        p.0, p.1, p.2, p.3
    );
}

// ===========================================================================
// C13..C17 — active_params dispatch shapes × normalization buckets
// ===========================================================================

#[test]
fn c13_active_params_zero() {
    // all four params 0 -> neither the mode_add nor mode_multiply dispatch runs
    let p = LibPair::fresh("c13");
    let (c, r) = p.apis();
    cmp_findrep(&c, &r, (0, 0, 0, 0), "C13 fresh");
    // and again on the now-advanced state
    for i in 0..8 {
        cmp_findrep(&c, &r, (0, 0, 0, 0), &format!("C13 repeat {i}"));
    }
}

/// Representative value from each normalization bucket, plus 0 for "inactive".
const BUCKET_NONZERO: [i32; 8] = [1, 63, 64, 65, 300, 511, 512, -7];

#[test]
fn c14_active_params_one() {
    for pos in 0..4usize {
        for &v in &BUCKET_NONZERO {
            let mut a = [0i32; 4];
            a[pos] = v;
            let p = LibPair::fresh(&format!("c14_{pos}_{v}"));
            let (c, r) = p.apis();
            cmp_findrep(
                &c,
                &r,
                (a[0], a[1], a[2], a[3]),
                &format!("C14 pos={pos} v={v}"),
            );
        }
    }
}

#[test]
fn c15_active_params_two() {
    let pairs = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];
    let mut rng = Rng::new(SEED ^ 0x15);
    for (i, j) in pairs {
        for &v in &BUCKET_NONZERO {
            let mut a = [0i32; 4];
            a[i] = v;
            a[j] = BUCKET_NONZERO[rng.below(BUCKET_NONZERO.len() as u64) as usize];
            let p = LibPair::fresh(&format!("c15_{i}{j}_{v}"));
            let (c, r) = p.apis();
            cmp_findrep(
                &c,
                &r,
                (a[0], a[1], a[2], a[3]),
                &format!("C15 pair=({i},{j}) v={v}"),
            );
        }
    }
}

#[test]
fn c16_active_params_three() {
    let triples = [(0, 1, 2), (0, 1, 3), (0, 2, 3), (1, 2, 3)];
    let mut rng = Rng::new(SEED ^ 0x16);
    for (i, j, k) in triples {
        for &v in &BUCKET_NONZERO {
            let mut a = [0i32; 4];
            a[i] = v;
            a[j] = BUCKET_NONZERO[rng.below(BUCKET_NONZERO.len() as u64) as usize];
            a[k] = BUCKET_NONZERO[rng.below(BUCKET_NONZERO.len() as u64) as usize];
            let p = LibPair::fresh(&format!("c16_{i}{j}{k}_{v}"));
            let (c, r) = p.apis();
            cmp_findrep(
                &c,
                &r,
                (a[0], a[1], a[2], a[3]),
                &format!("C16 triple=({i},{j},{k}) v={v}"),
            );
        }
    }
}

#[test]
fn c17_active_params_four() {
    for &v1 in &BUCKET_NONZERO {
        for &v2 in &BUCKET_NONZERO {
            let p = LibPair::fresh(&format!("c17_{v1}_{v2}"));
            let (c, r) = p.apis();
            cmp_findrep(&c, &r, (v1, v2, v2, v1), &format!("C17 {v1}/{v2}"));
        }
    }
}

// ===========================================================================
// C18 — all 16 zero/nonzero masks, exhaustive, each on fresh state
// ===========================================================================

#[test]
fn c18_all_sixteen_masks_fresh() {
    let mut rng = Rng::new(SEED ^ 0x18);
    for mask in 0u32..16 {
        for trial in 0..8 {
            let mut a = [0i32; 4];
            for bit in 0..4 {
                if mask & (1 << bit) != 0 {
                    // guarantee non-zero
                    let mut v = rng.interesting_i32();
                    if v == 0 {
                        v = 1;
                    }
                    a[bit] = v;
                }
            }
            let p = LibPair::fresh(&format!("c18_{mask}_{trial}"));
            let (c, r) = p.apis();
            cmp_findrep(
                &c,
                &r,
                (a[0], a[1], a[2], a[3]),
                &format!("C18 mask={mask:04b} trial={trial}"),
            );
        }
    }
}

// ===========================================================================
// C19..C22 — the hidden-state gates, approached from below / at / above
// ===========================================================================

/// Seed `accumulator` to exactly `target` using the low-level exports, then run
/// `findrep`. `add_to_accumulator(a,b)` does `accumulator += a + b`, so one
/// call suffices.
fn seed_accumulator(c: &Api<'_>, r: &Api<'_>, target: i32) {
    let cv = unsafe { (c.add_to_accumulator)(target, 0) };
    let rv = unsafe { (r.add_to_accumulator)(target, 0) };
    assert_eq!(cv, rv, "seed accumulator to {target}: C={cv} Rust={rv}");
    assert_eq!(cv, target, "seed accumulator: unexpected value {cv}");
}

/// Seed `multiplier` to exactly `target` (from its initial 1) via
/// `multiplier *= a * b`.
fn seed_multiplier(c: &Api<'_>, r: &Api<'_>, target: i32) {
    let cv = unsafe { (c.multiply_with_multiplier)(target, 1) };
    let rv = unsafe { (r.multiply_with_multiplier)(target, 1) };
    assert_eq!(cv, rv, "seed multiplier to {target}: C={cv} Rust={rv}");
    assert_eq!(cv, target, "seed multiplier: unexpected value {cv}");
}

#[test]
fn c19_accumulator_gate_0o150() {
    // gate is `accumulator > 0150` (104)
    for target in [0i32, 1, 100, 103, 104, 105, 200, -105, i32::MAX, i32::MIN] {
        for params in [(0, 0, 0, 0), (1, 1, 1, 1), (512, -3, 64, 0), (7, 0, 0, 0)] {
            let p = LibPair::fresh(&format!("c19_{target}"));
            let (c, r) = p.apis();
            seed_accumulator(&c, &r, target);
            cmp_findrep(&c, &r, params, &format!("C19 acc={target} params={params:?}"));
        }
    }
}

#[test]
fn c20_multiplier_gate_0o100() {
    // gate is `multiplier > 0100` (64)
    for target in [0i32, 1, -1, 63, 64, 65, 100, 512, i32::MAX, i32::MIN] {
        for params in [(0, 0, 0, 0), (1, 1, 1, 1), (511, 512, -9, 65), (0, 0, 3, 0)] {
            let p = LibPair::fresh(&format!("c20_{target}"));
            let (c, r) = p.apis();
            seed_multiplier(&c, &r, target);
            cmp_findrep(&c, &r, params, &format!("C20 mult={target} params={params:?}"));
        }
    }
}

#[test]
fn c21_multiplier_exactly_zero() {
    // multiplier == 0 => has_multiplier == 0 => both_active == 0
    for params in [
        (0, 0, 0, 0),
        (1, 1, 1, 1),
        (64, 64, 64, 64),
        (512, 512, 512, 512),
        (-1, -1, -1, -1),
    ] {
        let p = LibPair::fresh("c21");
        let (c, r) = p.apis();
        seed_multiplier(&c, &r, 0);
        cmp_findrep(&c, &r, params, &format!("C21 mult=0 params={params:?}"));
        // still zero after findrep multiplies by things? verify next call too
        cmp_findrep(&c, &r, params, &format!("C21 second params={params:?}"));
    }
}

#[test]
fn c22_accumulator_exactly_zero() {
    // accumulator == 0 => has_accumulator == 0 => both_active == 0.
    // A fresh library already has accumulator == 0; findrep(0,0,0,0) leaves it
    // at 0 because the add dispatch is skipped.
    let p = LibPair::fresh("c22a");
    let (c, r) = p.apis();
    cmp_findrep(&c, &r, (0, 0, 0, 0), "C22 fresh acc=0");

    // Drive accumulator back to exactly 0 via subtract, then findrep.
    for step in [7i32, 104, -104, 1000] {
        let p = LibPair::fresh(&format!("c22b_{step}"));
        let (c, r) = p.apis();
        seed_accumulator(&c, &r, step);
        let cv = unsafe { (c.subtract_from_accumulator)(step, 0) };
        let rv = unsafe { (r.subtract_from_accumulator)(step, 0) };
        assert_eq!(cv, rv, "C22 zeroing acc: C={cv} Rust={rv}");
        assert_eq!(cv, 0, "C22 accumulator should be 0, got {cv}");
        cmp_findrep(&c, &r, (0, 0, 0, 0), &format!("C22 acc=0 via {step}"));
    }
}

// ===========================================================================
// C23 — the result == 0 -> 0777 sentinel, from the valid side
// ===========================================================================

/// Seed `multiplier` to `m` on a fresh pair (one `multiply_with_multiplier`
/// call, which also bumps `operation_count` to 1), then run `findrep(params)`
/// on both implementations. Returns the agreed result.
fn seeded_findrep(m: i32, params: (i32, i32, i32, i32), ctx: &str) -> i32 {
    let p = LibPair::fresh("sent");
    let (c, r) = p.apis();
    let cm = unsafe { (c.multiply_with_multiplier)(m, 1) };
    let rm = unsafe { (r.multiply_with_multiplier)(m, 1) };
    assert_eq!(cm, rm, "{ctx}: seeding multiplier to {m}");
    assert_eq!(cm, m, "{ctx}: seed produced {cm}");
    let cv = unsafe { (c.findrep)(params.0, params.1, params.2, params.3) };
    let rv = unsafe { (r.findrep)(params.0, params.1, params.2, params.3) };
    assert_eq!(
        cv, rv,
        "{ctx}: findrep({}, {}, {}, {}) with multiplier={m}: C={cv} Rust={rv}",
        params.0, params.1, params.2, params.3
    );
    cv
}

#[test]
fn c23_sentinel_reached_by_construction() {
    // Solve for the state that drives `result` to exactly 0.
    //
    // Fresh state, then one `multiply_with_multiplier(M, 1)` seeds
    // multiplier = M and operation_count = 1. Now call findrep(1, 0, 0, 0):
    //   result  = 9                     (memchr offset of 'p')
    //   active_params == 1 >= mode_add  -> add_to_accumulator(64, 0)
    //                                     accumulator = 64, result += 64 -> 73
    //   active_params == 1 <  mode_mul  -> multiply dispatch skipped
    //   accumulator (64) > 0150 (104)?  -> no, subtract dispatch skipped
    //   both_active (64 != 0 && M != 0) -> result += 64 + M
    //   operation_count == 2            -> result += 16
    //   => result = 73 + 64 + M + 16 = 153 + M
    // So M == -153 makes result exactly 0, which the C replaces with 0777.
    let got = seeded_findrep(-153, (1, 0, 0, 0), "C23 constructed");
    assert_eq!(
        got, 0o777,
        "C23: expected the sentinel 0o777 (511) for multiplier=-153, got {got}"
    );

    // The immediate neighbours must NOT be the sentinel — this pins the exact
    // branch boundary rather than accidentally matching 511 some other way.
    let below = seeded_findrep(-154, (1, 0, 0, 0), "C23 M=-154");
    let above = seeded_findrep(-152, (1, 0, 0, 0), "C23 M=-152");
    assert_eq!(below, -1, "C23: multiplier=-154 should give result -1, got {below}");
    assert_eq!(above, 1, "C23: multiplier=-152 should give result 1, got {above}");
}

#[test]
fn c23_sentinel_scan_over_seeded_multiplier() {
    // Sweep the seeded multiplier across a wide band for several param shapes
    // and confirm C and Rust agree everywhere, that the sentinel is hit, and
    // that 0 is never returned.
    let param_sets: [(i32, i32, i32, i32); 4] = [
        (1, 0, 0, 0),
        (7, 0, 0, 0),
        (600, 0, 0, 0),
        (-5, 0, 0, 0),
    ];
    let mut total_hits = 0usize;
    for params in param_sets {
        let mut hits = Vec::new();
        for m in -400i32..=200 {
            if m == 0 {
                continue; // multiply(0,1) would make the seed assertion trivial
            }
            let got = seeded_findrep(m, params, &format!("C23 scan m={m} p={params:?}"));
            assert_ne!(got, 0, "C23: findrep returned 0 for m={m} params={params:?}");
            if got == 0o777 {
                hits.push(m);
            }
        }
        eprintln!("C23 scan params={params:?}: sentinel at multiplier {hits:?}");
        total_hits += hits.len();
    }
    assert!(
        total_hits > 0,
        "C23: the result==0 -> 0777 sentinel branch was never reached; \
         the scan does not actually cover ERRORS.md row E13"
    );
}

#[test]
fn c23_sentinel_second_family_active_params_two() {
    // A different dispatch shape that also reaches the sentinel, so the branch
    // is covered from more than one code path.
    //   findrep(1, 0, 1, 0) -> active_params == 2
    //   result = 9
    //   add(64, 0)            : accumulator = 64, result += 64      -> 73
    //   multiply(64, 0)       : multiplier = M * (64*0) = 0, result += 0
    //   accumulator 64 > 104? : no
    //   both_active           : multiplier == 0 -> FALSE, no state term
    //   operation_count == 3  : result += 24                        -> 97
    // The multiply zeroes the multiplier, so this shape is state-independent at
    // 97 and cannot hit the sentinel. Verify that invariant holds in both.
    for m in [-153i32, -1, 1, 5, 1000, i32::MAX, i32::MIN] {
        let got = seeded_findrep(m, (1, 0, 1, 0), &format!("C23 fam2 m={m}"));
        assert_eq!(got, 97, "C23 fam2: expected 97 for m={m}, got {got}");
    }

    // findrep(1, 0, 0, 1) keeps the multiplier alive:
    //   multiply(0, 64) -> multiplier = M * 0 = 0 as well. Use a shape where
    //   both normalized p3 and p4 are nonzero instead: findrep(1, 0, 2, 3).
    //   normalized: p1=64, p2=0, p3=64, p4=64
    //   result = 9; add(64,0) -> acc=64, result=73
    //   multiply(64,64) -> multiplier = M*4096, result += M*4096
    //   acc 64 > 104? no
    //   both_active: 64 != 0 && M*4096 != 0 -> result += 64 + M*4096
    //   if M*4096 > 0100 (64) the divide op runs; it does not touch `result`
    //   but it DOES bump operation_count, and the C reads operation_count
    //   *after* that call (lib.c:161 then lib.c:166) -- so the divide adds a
    //   further 010 (8) to the result.
    //   => result = 73 + M*4096 + 64 + M*4096 + 010*(3 or 4)
    for m in [-2i32, -1, 1, 2, 3, 100, -100] {
        let got = seeded_findrep(m, (1, 0, 2, 3), &format!("C23 fam3 m={m}"));
        let mult_after = m.wrapping_mul(4096);
        let ops: i32 = if mult_after > 0o100 { 4 } else { 3 };
        let want = 73i32
            .wrapping_add(mult_after)
            .wrapping_add(64)
            .wrapping_add(mult_after)
            .wrapping_add(0o10 * ops);
        assert_eq!(
            got, want,
            "C23 fam3: m={m} mult_after={mult_after} ops={ops} expected {want}, got {got}"
        );
    }
}

#[test]
fn c23_sentinel_search_for_zero_result() {
    // Search a wide space of (seeded state, params) for cases where the C
    // returns exactly 0777 (511) and confirm the Rust agrees. Also assert we
    // actually hit the sentinel at least once, so the row is genuinely covered.
    let mut rng = Rng::new(SEED ^ 0x23);
    let mut sentinel_hits = 0usize;
    for trial in 0..400 {
        let p = LibPair::fresh(&format!("c23_{trial}"));
        let (c, r) = p.apis();
        let acc_seed = match rng.below(4) {
            0 => 0,
            1 => -(rng.below(400) as i32),
            2 => rng.below(400) as i32,
            _ => rng.next_i32(),
        };
        if acc_seed != 0 {
            let cv = unsafe { (c.add_to_accumulator)(acc_seed, 0) };
            let rv = unsafe { (r.add_to_accumulator)(acc_seed, 0) };
            assert_eq!(cv, rv, "C23 seed acc {acc_seed}");
        }
        let mut params = [0i32; 4];
        for slot in params.iter_mut() {
            *slot = match rng.below(3) {
                0 => 0,
                1 => (rng.below(64) as i32) - 32,
                _ => rng.interesting_i32(),
            };
        }
        let cv = unsafe { (c.findrep)(params[0], params[1], params[2], params[3]) };
        let rv = unsafe { (r.findrep)(params[0], params[1], params[2], params[3]) };
        assert_eq!(cv, rv, "C23 trial {trial} params={params:?} acc_seed={acc_seed}");
        if cv == 0o777 {
            sentinel_hits += 1;
        }
        assert_ne!(cv, 0, "C23: findrep must never return 0 (sentinel)");
    }
    // The sentinel path (result == 0 -> 0777) is a narrow target; the strong
    // invariant every call must satisfy is "never returns 0", asserted above.
    eprintln!("C23: observed {sentinel_hits} returns equal to 0o777");
}

#[test]
fn c23b_findrep_never_returns_zero() {
    // The sentinel makes `findrep` total: it can never return 0. Verified
    // against the C across a long randomized sequence.
    let p = LibPair::fresh("c23b");
    let (c, r) = p.apis();
    let mut rng = Rng::new(SEED ^ 0x2B);
    for i in 0..2000 {
        let q = (
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
        let cv = unsafe { (c.findrep)(q.0, q.1, q.2, q.3) };
        let rv = unsafe { (r.findrep)(q.0, q.1, q.2, q.3) };
        assert_eq!(cv, rv, "C23b step {i} params={q:?}");
        assert_ne!(cv, 0, "C23b step {i}: C returned 0");
    }
}

// ===========================================================================
// C24 — full 11^4 boundary cross-product
// ===========================================================================

#[test]
fn c24_boundary_cross_product_11_pow_4() {
    // 14641 combinations. Batched: a fresh library pair every BATCH calls, so
    // the hidden state is exercised both from a clean slate and mid-sequence.
    const BATCH: usize = 64;
    let mut n = 0usize;
    let mut pair = LibPair::fresh("c24_0");
    let mut apis = pair.apis();
    for &a in &BOUNDS {
        for &b in &BOUNDS {
            for &cc in &BOUNDS {
                for &d in &BOUNDS {
                    if n % BATCH == 0 && n != 0 {
                        drop(apis);
                        pair = LibPair::fresh(&format!("c24_{n}"));
                        apis = pair.apis();
                    }
                    cmp_findrep_bare(
                        &apis.0,
                        &apis.1,
                        (a, b, cc, d),
                        &format!("C24 #{n}"),
                    );
                    n += 1;
                }
            }
        }
    }
    assert_eq!(n, 11 * 11 * 11 * 11, "C24 must cover the full cross-product");
}

// ===========================================================================
// C25 / C26 — long sequences on shared state
// ===========================================================================

#[test]
fn c25_repeated_invocation_same_state() {
    let p = LibPair::fresh("c25");
    let (c, r) = p.apis();
    let mut rng = Rng::new(SEED ^ 0x25);
    for i in 0..512 {
        let q = (
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
        cmp_findrep(&c, &r, q, &format!("C25 step {i}"));
    }
}

#[test]
fn c26_randomized_full_range_sequence() {
    let p = LibPair::fresh("c26");
    let (c, r) = p.apis();
    let mut rng = Rng::new(SEED ^ 0x26);
    for i in 0..4096 {
        let q = (
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
        );
        cmp_findrep_bare(&c, &r, q, &format!("C26 step {i}"));
    }
}

// ===========================================================================
// C27 / C28 — all 8 exports interleaved
// ===========================================================================

fn interleaved(tag: &str, seed: u64, iters: usize, biased: bool) {
    let p = LibPair::fresh(tag);
    let (c, r) = p.apis();
    let mut rng = Rng::new(seed);
    for i in 0..iters {
        let nx = |rng: &mut Rng| {            if biased {
                rng.interesting_i32()
            } else {
                rng.next_i32()
            }
        };
        let a = nx(&mut rng);
        let b = nx(&mut rng);
        match rng.below(8) {
            0 => {
                let cv = unsafe { (c.add_to_accumulator)(a, b) };
                let rv = unsafe { (r.add_to_accumulator)(a, b) };
                assert_eq!(cv, rv, "{tag} step {i}: add({a},{b})");
            }
            1 => {
                let cv = unsafe { (c.multiply_with_multiplier)(a, b) };
                let rv = unsafe { (r.multiply_with_multiplier)(a, b) };
                assert_eq!(cv, rv, "{tag} step {i}: multiply({a},{b})");
            }
            2 => {
                let cv = unsafe { (c.subtract_from_accumulator)(a, b) };
                let rv = unsafe { (r.subtract_from_accumulator)(a, b) };
                assert_eq!(cv, rv, "{tag} step {i}: subtract({a},{b})");
            }
            3 => {
                // never b == -1: avoids the INT_MIN / -1 hardware trap (E3)
                let b = if b == -1 { 1 } else { b };
                let cv = unsafe { (c.divide_multiplier)(a, b) };
                let rv = unsafe { (r.divide_multiplier)(a, b) };
                assert_eq!(cv, rv, "{tag} step {i}: divide({a},{b})");
            }
            4 => {
                let cv = unsafe { (c.validate_and_normalize)(a) };
                let rv = unsafe { (r.validate_and_normalize)(a) };
                assert_eq!(cv, rv, "{tag} step {i}: validate({a})");
            }
            5 => {
                let mut cb = scratch(0xAA);
                let mut rb = scratch(0xAA);
                unsafe { (c.process_octal_string)(cb.as_mut_ptr(), a) };
                unsafe { (r.process_octal_string)(rb.as_mut_ptr(), a) };
                assert_eq!(
                    as_u8(&cb),
                    as_u8(&rb),
                    "{tag} step {i}: process_octal_string({a})\n  C   ={}\n  Rust={}",
                    show(&cb),
                    show(&rb)
                );
            }
            6 => {
                let len = rng.below(70) as usize;
                let s: Vec<u8> = (0..len).map(|_| (rng.below(255) as u8) + 1).collect();
                let mut cb = scratch(0xAA);
                let mut rb = scratch(0xAA);
                set_cstr(&mut cb, &s);
                set_cstr(&mut rb, &s);
                unsafe { (c.find_and_replace_char)(cb.as_mut_ptr(), b) };
                unsafe { (r.find_and_replace_char)(rb.as_mut_ptr(), b) };
                assert_eq!(
                    as_u8(&cb),
                    as_u8(&rb),
                    "{tag} step {i}: find_and_replace_char(len={len}, {b})\n  C   ={}\n  Rust={}",
                    show(&cb),
                    show(&rb)
                );
            }
            _ => {
                let c3 = nx(&mut rng);
                let c4 = nx(&mut rng);
                let cv = unsafe { (c.findrep)(a, b, c3, c4) };
                let rv = unsafe { (r.findrep)(a, b, c3, c4) };
                assert_eq!(cv, rv, "{tag} step {i}: findrep({a},{b},{c3},{c4})");
            }
        }
    }
}

#[test]
fn c27_all_exports_interleaved_random() {
    interleaved("c27", SEED ^ 0x27, 8192, false);
}

#[test]
fn c28_all_exports_interleaved_biased() {
    interleaved("c28", SEED ^ 0x28, 8192, true);
}

// ===========================================================================
// C29 / C30 — pre-seed state through the low-level exports, then findrep
// ===========================================================================

#[test]
fn c29_preseed_accumulator_then_findrep() {
    let mut rng = Rng::new(SEED ^ 0x29);
    for trial in 0..120 {
        let p = LibPair::fresh(&format!("c29_{trial}"));
        let (c, r) = p.apis();
        // push accumulator well past the 0150 gate using both add and subtract
        let n = 1 + rng.below(5);
        for _ in 0..n {
            let (a, b) = (rng.interesting_i32(), rng.interesting_i32());
            if rng.below(2) == 0 {
                let cv = unsafe { (c.add_to_accumulator)(a, b) };
                let rv = unsafe { (r.add_to_accumulator)(a, b) };
                assert_eq!(cv, rv, "C29 trial {trial}: add({a},{b})");
            } else {
                let cv = unsafe { (c.subtract_from_accumulator)(a, b) };
                let rv = unsafe { (r.subtract_from_accumulator)(a, b) };
                assert_eq!(cv, rv, "C29 trial {trial}: subtract({a},{b})");
            }
        }
        for k in 0..3 {
            let q = (
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
            );
            cmp_findrep(&c, &r, q, &format!("C29 trial {trial} call {k}"));
        }
    }
}

#[test]
fn c30_preseed_multiplier_then_findrep() {
    let mut rng = Rng::new(SEED ^ 0x30);
    for trial in 0..120 {
        let p = LibPair::fresh(&format!("c30_{trial}"));
        let (c, r) = p.apis();
        let n = 1 + rng.below(5);
        for _ in 0..n {
            let (a, mut b) = (rng.interesting_i32(), rng.interesting_i32());
            if rng.below(2) == 0 {
                let cv = unsafe { (c.multiply_with_multiplier)(a, b) };
                let rv = unsafe { (r.multiply_with_multiplier)(a, b) };
                assert_eq!(cv, rv, "C30 trial {trial}: multiply({a},{b})");
            } else {
                if b == -1 {
                    b = 1;
                }
                let cv = unsafe { (c.divide_multiplier)(a, b) };
                let rv = unsafe { (r.divide_multiplier)(a, b) };
                assert_eq!(cv, rv, "C30 trial {trial}: divide({a},{b})");
            }
        }
        for k in 0..3 {
            let q = (
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
            );
            cmp_findrep(&c, &r, q, &format!("C30 trial {trial} call {k}"));
        }
    }
}
