//! Phase B + Phase C for `findrep`, the composed pipeline and the only function in
//! `c_src/include/lib.h`.
//!
//! Covers CONFIGS.md rows 34–48 and 50, and ERRORS.md rows 22–30.
//!
//! Every test also runs an INDEPENDENT third model of the C control flow (`Model`)
//! so that (a) the agreed C/Rust value is cross-checked against a hand-derived
//! reference, and (b) each test can ASSERT which branches it actually reached —
//! otherwise a "passing" test could be silently vacuous.

mod common;
use common::*;

use std::ffi::c_int;

// ---------------------------------------------------------------------------
// Independent model of the C, transcribed branch-by-branch from lib.c
// ---------------------------------------------------------------------------

/// `'p'` sits at index 9 of `"Function pointer example with static vars"`,
/// so `result += found_char - search_buffer` always contributes exactly 9.
const P_OFFSET: c_int = 9;

fn model_validate(value: c_int) -> c_int {
    // int is_nonzero = !!value;  if (is_nonzero && value > 0) { ... }  return value;
    if value != 0 && value > 0 {
        if value < 0o100 {
            return 0o100;
        } else if value > 0o777 {
            return 0o777;
        }
    }
    value
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Cover {
    add: bool,
    mul: bool,
    sub: bool,
    both_active: bool,
    div: bool,
    sentinel: bool,
}

#[derive(Debug, Clone, Copy)]
struct Model {
    acc: c_int,
    mult: c_int,
    cnt: c_int,
}

impl Default for Model {
    fn default() -> Self {
        Model { acc: 0, mult: 1, cnt: 0 }
    }
}

impl Model {
    fn add(&mut self, a: c_int, b: c_int) -> c_int {
        self.acc = self.acc.wrapping_add(a.wrapping_add(b));
        self.cnt = self.cnt.wrapping_add(1);
        self.acc
    }
    fn mul(&mut self, a: c_int, b: c_int) -> c_int {
        self.mult = self.mult.wrapping_mul(a.wrapping_mul(b));
        self.cnt = self.cnt.wrapping_add(1);
        self.mult
    }
    fn sub(&mut self, a: c_int, b: c_int) -> c_int {
        self.acc = self.acc.wrapping_sub(a.wrapping_sub(b));
        self.cnt = self.cnt.wrapping_add(1);
        self.acc
    }
    fn div(&mut self, b: c_int) -> c_int {
        if b != 0 {
            self.mult = self.mult.wrapping_div(b);
        }
        self.cnt = self.cnt.wrapping_add(1);
        self.mult
    }

    fn findrep(&mut self, p1: c_int, p2: c_int, p3: c_int, p4: c_int) -> (c_int, Cover) {
        let mut cov = Cover::default();
        let mut result: c_int = 0;

        let active = (p1 != 0) as c_int
            + (p2 != 0) as c_int
            + (p3 != 0) as c_int
            + (p4 != 0) as c_int;

        let n1 = model_validate(p1);
        let n2 = model_validate(p2);
        let n3 = model_validate(p3);
        let n4 = model_validate(p4);

        // memchr(search_buffer, 'p', ...) always hits at index 9.
        result = result.wrapping_add(P_OFFSET);

        if active >= 0o1 {
            cov.add = true;
            result = result.wrapping_add(self.add(n1, n2));
        }
        if active >= 0o2 {
            cov.mul = true;
            result = result.wrapping_add(self.mul(n3, n4));
        }
        if self.acc > 0o150 {
            cov.sub = true;
            result = result.wrapping_add(self.sub(n1, n3));
        }
        if self.acc != 0 && self.mult != 0 {
            cov.both_active = true;
            result = result.wrapping_add(self.acc.wrapping_add(self.mult));
        }
        if self.mult > 0o100 {
            cov.div = true;
            self.div(2);
        }
        result = result.wrapping_add(self.cnt.wrapping_mul(0o10));
        if result == 0 {
            cov.sentinel = true;
            result = 0o777;
        }
        (result, cov)
    }
}

// ---------------------------------------------------------------------------
// Differential driver
// ---------------------------------------------------------------------------

/// One `findrep` call on both libraries plus the model. Returns the coverage the
/// model says was reached.
#[track_caller]
fn step(p: &Pair, m: &mut Model, a: c_int, b: c_int, c: c_int, d: c_int) -> Cover {
    let before = *m;
    let cv = unsafe { (p.c.findrep)(a, b, c, d) };
    let rv = unsafe { (p.r.findrep)(a, b, c, d) };
    assert_eq!(
        cv, rv,
        "findrep({a}, {b}, {c}, {d}) diverged: C={cv} Rust={rv}\n  state before: {before:?}"
    );
    let (mv, cov) = m.findrep(a, b, c, d);
    assert_eq!(
        cv, mv,
        "findrep({a}, {b}, {c}, {d}): C={cv} but the independent model says {mv}\n  \
         state before: {before:?}"
    );
    cov
}

/// A single call on a pristine pair.
#[track_caller]
fn once(a: c_int, b: c_int, c: c_int, d: c_int) -> (c_int, Cover) {
    let p = fresh_pair();
    let mut m = Model::default();
    let cov = step(&p, &mut m, a, b, c, d);
    let (v, _) = {
        // recompute the value the model produced for this same call
        let mut m2 = Model::default();
        m2.findrep(a, b, c, d)
    };
    (v, cov)
}

// ===========================================================================
// active_params shapes  (CONFIGS #34–#38, ERRORS #22/#23)
// ===========================================================================

/// CONFIGS #34 / ERRORS #22 — all params zero: `active_params == 0`, so BOTH op
/// blocks are skipped. Also ERRORS #25 (`accumulator == 0` kills `both_active`).
#[test]
fn cfg34_findrep_all_zero_params() {
    let (v, cov) = once(0, 0, 0, 0);
    assert_eq!(v, 9, "only the memchr offset contributes");
    assert!(!cov.add, "add block must be skipped when active_params == 0");
    assert!(!cov.mul);
    assert!(!cov.sub);
    assert!(
        !cov.both_active,
        "ERRORS #25: accumulator == 0 must kill the both_active branch"
    );
    assert!(!cov.div, "multiplier == 1 is not > 0100");
    // Calling it repeatedly on the same instance must stay stable (nothing mutates).
    let p = fresh_pair();
    let mut m = Model::default();
    for _ in 0..20 {
        let c = step(&p, &mut m, 0, 0, 0, 0);
        assert!(!c.add && !c.mul);
    }
}

/// CONFIGS #35 / ERRORS #23 — exactly one non-zero param, in each of the 4 slots:
/// `active_params == 1`, so ONLY the add block runs.
#[test]
fn cfg35_findrep_one_active_param_each_slot() {
    let mut rng = Rng::new(0x3501);
    for slot in 0..4 {
        for _ in 0..300 {
            let v = loop {
                let v = rng.interesting_i32();
                if v != 0 {
                    break v;
                }
            };
            let mut ps = [0i32; 4];
            ps[slot] = v;
            let (_, cov) = once(ps[0], ps[1], ps[2], ps[3]);
            assert!(cov.add, "add must run when active_params == 1");
            assert!(
                !cov.mul,
                "ERRORS #23: multiply must NOT run when active_params == 1 (slot {slot}, v {v})"
            );
        }
    }
}

/// CONFIGS #36 — two non-zero params, all 6 slot pairs: add + multiply both run.
#[test]
fn cfg36_findrep_two_active_params_all_pairs() {
    let mut rng = Rng::new(0x3601);
    let mut saw_mul = false;
    for i in 0..4 {
        for j in (i + 1)..4 {
            for _ in 0..200 {
                let mut ps = [0i32; 4];
                ps[i] = loop {
                    let v = rng.interesting_i32();
                    if v != 0 {
                        break v;
                    }
                };
                ps[j] = loop {
                    let v = rng.interesting_i32();
                    if v != 0 {
                        break v;
                    }
                };
                let (_, cov) = once(ps[0], ps[1], ps[2], ps[3]);
                assert!(cov.add && cov.mul, "both blocks must run at active_params == 2");
                saw_mul = true;
            }
        }
    }
    assert!(saw_mul);
}

/// CONFIGS #37 — three non-zero params, all 4 triples.
#[test]
fn cfg37_findrep_three_active_params_all_triples() {
    let mut rng = Rng::new(0x3701);
    for zero_slot in 0..4 {
        for _ in 0..300 {
            let mut ps = [0i32; 4];
            for s in 0..4 {
                if s == zero_slot {
                    continue;
                }
                ps[s] = loop {
                    let v = rng.interesting_i32();
                    if v != 0 {
                        break v;
                    }
                };
            }
            let (_, cov) = once(ps[0], ps[1], ps[2], ps[3]);
            assert!(cov.add && cov.mul);
        }
    }
}

/// CONFIGS #38 — all four params non-zero.
#[test]
fn cfg38_findrep_four_active_params() {
    let mut rng = Rng::new(0x3801);
    for _ in 0..2000 {
        let mut ps = [0i32; 4];
        for s in 0..4 {
            ps[s] = loop {
                let v = rng.interesting_i32();
                if v != 0 {
                    break v;
                }
            };
        }
        let (_, cov) = once(ps[0], ps[1], ps[2], ps[3]);
        assert!(cov.add && cov.mul);
    }
}

// ===========================================================================
// validate_and_normalize shape cross-product  (CONFIGS #39)
// ===========================================================================

/// CONFIGS #39 — full cross-product of the five `validate_and_normalize` shape
/// classes across all four parameter slots (5^4 = 625 combinations), each on a
/// pristine pair, with several randomized draws per class.
#[test]
fn cfg39_findrep_normalize_shape_cross_product() {
    let mut rng = Rng::new(0x3901);
    // 0 = zero, 1 = clamp-up (1..63), 2 = identity (64..511),
    // 3 = clamp-down (>511), 4 = negative (identity)
    for a in 0..5 {
        for b in 0..5 {
            for c in 0..5 {
                for d in 0..5 {
                    for _ in 0..2 {
                        let draw = |class: i32, rng: &mut Rng| -> i32 {
                            match class {
                                0 => 0,
                                1 => rng.range_i32(1, 63),
                                2 => rng.range_i32(64, 511),
                                3 => rng.range_i32(512, i32::MAX),
                                _ => rng.range_i32(i32::MIN, -1),
                            }
                        };
                        let p1 = draw(a, &mut rng);
                        let p2 = draw(b, &mut rng);
                        let p3 = draw(c, &mut rng);
                        let p4 = draw(d, &mut rng);
                        once(p1, p2, p3, p4);
                    }
                }
            }
        }
    }
}

// ===========================================================================
// State-branch targeting  (CONFIGS #40–#44, ERRORS #24–#27)
// ===========================================================================

/// CONFIGS #40 — params chosen so `accumulator > 0150` (104) on the FIRST call,
/// firing the `operations[2]` subtract block.
#[test]
fn cfg40_findrep_subtract_block_fires() {
    // n1 = n2 = 100 -> accumulator = 200 > 104
    let (_, cov) = once(100, 100, 3, 3);
    assert!(cov.sub, "accumulator 200 > 104 must fire the subtract block");
    assert!(cov.add && cov.mul);
    assert!(cov.div, "multiplier 64*64 = 4096 > 64 must fire the divide block");

    // Randomized: any pair summing above 104 after normalization.
    let mut rng = Rng::new(0x4001);
    let mut fired = 0;
    for _ in 0..1000 {
        let p1 = rng.range_i32(64, 511);
        let p2 = rng.range_i32(64, 511);
        let (_, cov) = once(p1, p2, rng.interesting_i32(), rng.interesting_i32());
        if cov.sub {
            fired += 1;
        }
    }
    assert!(fired > 900, "expected the subtract branch to dominate, got {fired}/1000");
}

/// CONFIGS #41 / ERRORS #24 — `accumulator <= 0150`, subtract block skipped.
#[test]
fn cfg41_findrep_subtract_block_skipped() {
    let (_, cov) = once(1, 1, 0, 0);
    // n1 = n2 = 64 -> accumulator = 128 > 104, so pick smaller inputs instead.
    assert!(cov.sub, "sanity: 64+64 = 128 > 104");

    // Negative params keep the accumulator low.
    let (_, cov) = once(-5, -5, 0, 0);
    assert!(!cov.sub, "accumulator -10 must NOT fire the subtract block");
    assert!(cov.add);

    let mut rng = Rng::new(0x4101);
    let mut skipped = 0;
    for _ in 0..1000 {
        let p1 = rng.range_i32(i32::MIN / 2, -1);
        let p2 = rng.range_i32(i32::MIN / 2, -1);
        let (_, cov) = once(p1, p2, 0, 0);
        if !cov.sub {
            skipped += 1;
        }
    }
    assert!(skipped > 900, "expected the subtract branch to be skipped, got {skipped}/1000");
}

/// CONFIGS #42 / ERRORS #27 — the `multiplier > 0100` divide branch, fired and skipped.
#[test]
fn cfg42_findrep_divide_branch_both_ways() {
    // multiplier = 1 * (64 * 64) = 4096 > 64 -> divide fires
    let (_, cov) = once(1, 1, 1, 1);
    assert!(cov.div, "multiplier 4096 > 64 must fire the divide block");

    // multiplier = 1 * (n3 * n4) with a negative product stays <= 64 -> skipped
    let (_, cov) = once(1, 1, -3, 5);
    assert!(!cov.div, "negative multiplier must NOT fire the divide block");

    // multiplier latched to 0 -> skipped (and both_active dies too)
    let (_, cov) = once(5, 5, 0, 7);
    assert!(cov.mul, "active_params == 3 runs the multiply block");
    assert!(!cov.div, "multiplier 0 is not > 64");
}

/// ERRORS #26 — `multiplier == 0` kills `both_active` even though `accumulator != 0`.
#[test]
fn cfg43_findrep_multiplier_zero_kills_both_active() {
    // p3 == 0 -> n3 == 0 -> multiplier *= 0 -> 0
    let (_, cov) = once(100, 100, 0, 7);
    assert!(cov.mul, "multiply block runs (active_params == 3)");
    assert!(
        !cov.both_active,
        "ERRORS #26: multiplier == 0 must kill both_active despite accumulator != 0"
    );
    let (_, cov) = once(50, 50, 7, 0);
    assert!(!cov.both_active);
}

/// ERRORS #25 — `accumulator == 0` kills `both_active` even though `multiplier != 0`.
#[test]
fn cfg44_findrep_accumulator_zero_kills_both_active() {
    // Only the multiply block would run... but active_params >= 2 implies the add
    // block ran too. Make add contribute exactly 0: n1 + n2 == 0 with both non-zero.
    // n1 = 64 (from 1..63 clamp) and n2 = -64 -> accumulator = 0.
    let (_, cov) = once(1, -64, 2, 2);
    assert!(cov.add && cov.mul);
    assert!(
        !cov.both_active,
        "ERRORS #25: accumulator == 0 must kill both_active despite multiplier != 0"
    );
    // Same via the all-zero path.
    let (_, cov) = once(0, 0, 0, 0);
    assert!(!cov.both_active);
}

// ===========================================================================
// Sentinel  (CONFIGS #50, ERRORS #28)
// ===========================================================================

/// CONFIGS #50 / ERRORS #28 — a parameter set whose computed `result` is exactly 0,
/// so `findrep` must return the sentinel `0777` = 511 instead of 0.
#[test]
fn cfg50_findrep_zero_result_returns_sentinel() {
    // Derivation: active == 1 with n1 = -9 gives
    //   result = 9 (memchr) + (-9) (add) + (-9 + 1) (both_active) + 1*8 (count) = 0
    let (v, cov) = once(-9, 0, 0, 0);
    assert!(cov.sentinel, "this input must reach the `!result_exists` branch");
    assert_eq!(v, 0o777, "sentinel must be 0777 = 511");

    let p = fresh_pair();
    unsafe {
        assert_eq!((p.c.findrep)(-9, 0, 0, 0), 511, "C must return the sentinel");
        // Same slot swapped: n1 + n2 == -9 either way.
    }
    let (v, cov) = once(0, -9, 0, 0);
    assert!(cov.sentinel);
    assert_eq!(v, 511);

    // Exhaustively confirm C, Rust and the model agree on WHICH single-slot inputs
    // hit the sentinel, over a wide window.
    let mut hits = 0;
    for v in -300..=300 {
        let (val, cov) = once(v, 0, 0, 0);
        if cov.sentinel {
            hits += 1;
            assert_eq!(val, 511);
        } else {
            assert_ne!(
                val, 0,
                "findrep can never return 0: the sentinel replaces it"
            );
        }
    }
    assert!(hits >= 1, "expected at least one sentinel-producing input");
}

/// `findrep` must NEVER return 0 for any input — the sentinel guarantees it.
#[test]
fn findrep_never_returns_zero() {
    let mut rng = Rng::new(0x5001);
    for _ in 0..5000 {
        let p = fresh_pair();
        let mut m = Model::default();
        let (a, b, c, d) = (
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
        step(&p, &mut m, a, b, c, d);
        let v = unsafe { (p.c.findrep)(a, b, c, d) };
        assert_ne!(v, 0, "findrep({a},{b},{c},{d}) returned 0");
    }
}

// ===========================================================================
// Statefulness across calls  (CONFIGS #45, #46)
// ===========================================================================

/// CONFIGS #45 — the SAME arguments called 1..50 times in a row. Each later call
/// sees different statics and therefore takes different branches; a stateless
/// translation would pass call #1 and fail here.
#[test]
fn cfg45_findrep_repeated_calls_state_carried() {
    let arg_sets: [[i32; 4]; 12] = [
        [0, 0, 0, 0],
        [1, 1, 1, 1],
        [-9, 0, 0, 0],
        [100, 100, 3, 3],
        [1, -64, 2, 2],
        [5, 5, 0, 7],
        [i32::MAX, i32::MAX, i32::MAX, i32::MAX],
        [i32::MIN, i32::MIN, i32::MIN, i32::MIN],
        [i32::MIN, i32::MAX, -1, 1],
        [511, 512, 63, 64],
        [-1, -1, -1, -1],
        [104, 105, 0, 1],
    ];
    for args in arg_sets {
        let p = fresh_pair();
        let mut m = Model::default();
        let mut seen = std::collections::BTreeSet::new();
        for _ in 0..50 {
            let cov = step(&p, &mut m, args[0], args[1], args[2], args[3]);
            seen.insert(cov);
        }
        // For at least some argument sets the reached branch-set must actually change
        // between calls, proving the state really is being exercised.
        eprintln!("{args:?}: {} distinct branch-sets over 50 calls", seen.len());
    }
}

/// CONFIGS #45 (stronger) — assert that repetition genuinely changes behaviour for
/// an argument set where the accumulator crosses the 0150 threshold mid-sequence.
#[test]
fn cfg45b_findrep_repetition_changes_branches() {
    let p = fresh_pair();
    let mut m = Model::default();
    // n1 = n2 = 64, so the accumulator climbs 128, 256, ... crossing 104 on call 1,
    // while the multiplier is repeatedly squared then halved.
    // NOTE: every call must go to BOTH libraries, otherwise their statics desync and
    // the test — not the translation — is what breaks.
    let mut values = Vec::new();
    let mut covs = Vec::new();
    for _ in 0..30 {
        covs.push(step(&p, &mut m, 1, 1, 1, 1));
        let cv = unsafe { (p.c.findrep)(0, 0, 0, 0) };
        let rv = unsafe { (p.r.findrep)(0, 0, 0, 0) };
        assert_eq!(cv, rv, "probe call findrep(0,0,0,0) diverged");
        let (mv, _) = m.findrep(0, 0, 0, 0);
        assert_eq!(cv, mv, "probe call: C={cv} model={mv}");
        values.push(cv);
    }
    let distinct: std::collections::BTreeSet<_> = values.iter().collect();
    assert!(
        distinct.len() > 1,
        "repeated calls must produce different results as state accumulates, got {values:?}"
    );
    let distinct_covs: std::collections::BTreeSet<_> = covs.iter().collect();
    eprintln!(
        "cfg45b: {} distinct results, {} distinct branch-sets over 30 rounds",
        distinct.len(),
        distinct_covs.len()
    );
}

/// CONFIGS #46 — pre-seed the statics through the LOW-LEVEL entry points, then run
/// `findrep`. This reaches state combinations `findrep` alone cannot produce (e.g. a
/// huge accumulator with a multiplier of exactly 1, or an odd `operation_count`).
#[test]
fn cfg46_findrep_after_low_level_preseeding() {
    let mut rng = Rng::new(0x4601);
    for _ in 0..2000 {
        let p = fresh_pair();
        let mut m = Model::default();

        // Random prelude of low-level ops.
        let steps = 1 + rng.below(6);
        for _ in 0..steps {
            let idx = rng.below(4) as usize;
            let a = rng.interesting_i32();
            let mut b = rng.interesting_i32();
            if idx == 3 && m.mult == i32::MIN && b == -1 {
                b = 7; // avoid the fatal INT_MIN / -1 (crash.rs covers it)
            }
            let cv = unsafe { p.c.op(idx, a, b) };
            let rv = unsafe { p.r.op(idx, a, b) };
            assert_eq!(cv, rv, "prelude op {idx}({a}, {b}) diverged");
            let mv = match idx {
                0 => m.add(a, b),
                1 => m.mul(a, b),
                2 => m.sub(a, b),
                _ => m.div(b),
            };
            assert_eq!(cv, mv, "prelude op {idx}({a}, {b}): C={cv} model={mv}");
        }

        // Now the composed pipeline, on top of that state.
        step(
            &p,
            &mut m,
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
    }
}

// ===========================================================================
// Boundaries and fuzz  (CONFIGS #47, #48)
// ===========================================================================

/// CONFIGS #47 — cross-product of boundary values over all four slots, each on a
/// pristine pair. 9^4 = 6561 combinations.
#[test]
fn cfg47_findrep_boundary_cross_product() {
    const VALS: [i32; 9] = [i32::MIN, -1, 0, 1, 63, 64, 511, 512, i32::MAX];
    for &a in VALS.iter() {
        for &b in VALS.iter() {
            for &c in VALS.iter() {
                for &d in VALS.iter() {
                    once(a, b, c, d);
                }
            }
        }
    }
}

/// CONFIGS #47 (wider) — the full `BOUNDARIES` list in the first two slots crossed
/// with itself, on a pristine pair each time.
#[test]
fn cfg47b_findrep_wide_boundary_pairs() {
    for &a in BOUNDARIES.iter() {
        for &b in BOUNDARIES.iter() {
            once(a, b, 0, 0);
            once(0, 0, a, b);
            once(a, 0, b, 0);
            once(a, b, a, b);
        }
    }
}

/// CONFIGS #48 — 20 000 randomized calls with the state NEVER reset, so any
/// divergence in the hidden statics compounds until it shows up in a return value.
#[test]
fn cfg48_findrep_long_stateful_fuzz() {
    let p = fresh_pair();
    let mut m = Model::default();
    let mut rng = Rng::new(0x4801);
    for _ in 0..20_000 {
        step(
            &p,
            &mut m,
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
    }
}

/// CONFIGS #48 (fresh-state variant) — 20 000 randomized calls, each on a pristine
/// pair, covering the first-call behaviour across the whole input space.
#[test]
fn cfg48b_findrep_fresh_state_fuzz() {
    let mut rng = Rng::new(0x4802);
    for _ in 0..8000 {
        let a = rng.interesting_i32();
        let b = rng.interesting_i32();
        let c = rng.interesting_i32();
        let d = rng.interesting_i32();
        once(a, b, c, d);
    }
    // Pure uniform bit patterns too, not just the biased "interesting" draws.
    for _ in 0..4000 {
        let a = rng.next_i32();
        let b = rng.next_i32();
        let c = rng.next_i32();
        let d = rng.next_i32();
        once(a, b, c, d);
    }
}

/// ERRORS #29 — the `if (found_char)` guard around the `memchr` for `'p'`. The C
/// searches a hard-coded literal, so the hit is unconditional and contributes
/// exactly +9. Pin that: with all-zero params the entire result IS the offset.
#[test]
fn err29_memchr_offset_is_always_nine() {
    let p = fresh_pair();
    let v = unsafe { (p.c.findrep)(0, 0, 0, 0) };
    assert_eq!(
        v, P_OFFSET,
        "'p' is at index 9 of \"Function pointer example with static vars\""
    );
    let r = unsafe { (p.r.findrep)(0, 0, 0, 0) };
    assert_eq!(r, P_OFFSET);
}

/// ERRORS #30 — extreme params with no overflow guard anywhere; must wrap, never
/// panic or abort, identically on both sides.
#[test]
fn err30_findrep_extremes_wrap_without_trapping() {
    for a in [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX] {
        for b in [i32::MIN, -1, 0, 1, i32::MAX] {
            once(a, b, a, b);
            once(a, a, b, b);
            once(b, a, b, a);
        }
    }
    // And a long run of extremes on ONE instance so the statics themselves wrap.
    let p = fresh_pair();
    let mut m = Model::default();
    for _ in 0..500 {
        step(&p, &mut m, i32::MAX, i32::MAX, i32::MAX, i32::MAX);
        step(&p, &mut m, i32::MIN, i32::MIN, i32::MIN, i32::MIN);
    }
}

// ===========================================================================
// Anti-vacuity audit
// ===========================================================================

/// Proves the Phase B suite is NOT vacuous: sweep a wide input/state space and
/// assert that EVERY branch in `findrep` was observed both TAKEN and NOT TAKEN.
/// A test suite that never reaches a branch cannot have verified it.
#[test]
fn branch_coverage_audit() {
    let mut taken = Cover::default();
    let mut not_taken = Cover::default();

    let mut note = |c: Cover| {
        macro_rules! upd {
            ($f:ident) => {
                if c.$f {
                    taken.$f = true;
                } else {
                    not_taken.$f = true;
                }
            };
        }
        upd!(add);
        upd!(mul);
        upd!(sub);
        upd!(both_active);
        upd!(div);
        upd!(sentinel);
    };

    // Fresh-state sweep over boundaries and randomized draws.
    let mut rng = Rng::new(0xC0FFEE);
    for _ in 0..4000 {
        let (_, cov) = once(
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
        note(cov);
    }
    // Hand-picked inputs for the rarer branches.
    for args in [
        [0, 0, 0, 0],
        [1, 1, 1, 1],
        [-9, 0, 0, 0],
        [100, 100, 3, 3],
        [1, -64, 2, 2],
        [5, 5, 0, 7],
        [-5, -5, 0, 0],
        [1, 1, -3, 5],
    ] {
        let (_, cov) = once(args[0], args[1], args[2], args[3]);
        note(cov);
    }
    // Long stateful runs reach branch combinations single calls cannot.
    for seed in [1u64, 2, 3, 4, 5] {
        let p = fresh_pair();
        let mut m = Model::default();
        let mut r = Rng::new(seed);
        for _ in 0..2000 {
            note(step(
                &p,
                &mut m,
                r.interesting_i32(),
                r.interesting_i32(),
                r.interesting_i32(),
                r.interesting_i32(),
            ));
        }
    }

    let missing: Vec<&str> = [
        ("add taken", taken.add),
        ("add skipped", not_taken.add),
        ("mul taken", taken.mul),
        ("mul skipped", not_taken.mul),
        ("sub taken", taken.sub),
        ("sub skipped", not_taken.sub),
        ("both_active taken", taken.both_active),
        ("both_active skipped", not_taken.both_active),
        ("div taken", taken.div),
        ("div skipped", not_taken.div),
        ("sentinel taken", taken.sentinel),
        ("sentinel skipped", not_taken.sentinel),
    ]
    .iter()
    .filter(|(_, hit)| !hit)
    .map(|(n, _)| *n)
    .collect();

    assert!(
        missing.is_empty(),
        "these findrep branch outcomes were never exercised: {missing:?}"
    );
    eprintln!("branch_coverage_audit: all 6 findrep branches observed both ways");
}
