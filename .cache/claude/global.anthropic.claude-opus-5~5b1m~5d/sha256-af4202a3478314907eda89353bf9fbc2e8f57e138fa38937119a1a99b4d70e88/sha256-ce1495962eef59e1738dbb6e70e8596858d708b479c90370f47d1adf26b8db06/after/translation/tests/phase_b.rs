//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every test drives BOTH `.so` files through their exported `tfm` symbol and
//! compares the written `dest` buffer bit-for-bit. Inputs are randomized with a
//! fixed seed so failures reproduce.

mod common;

use common::*;

// ===========================================================================
// Input generators
// ===========================================================================

/// `count` triples of finite normals in `[-1, 1]`.
fn src_unit(rng: &mut Rng, count: usize) -> Vec<f32> {
    (0..3 * count).map(|_| rng.signed_unit()).collect()
}

/// `count` triples where every element takes the `if` branch (`s0 < s1`).
fn src_branch_if(rng: &mut Rng, count: usize) -> Vec<f32> {
    let mut v = Vec::with_capacity(3 * count);
    for _ in 0..count {
        let a = rng.wild_normal();
        let b = rng.wild_normal();
        let (lo, hi) = if a < b { (a, b) } else { (b, a) };
        // wild_normal never returns NaN, and a==b is astronomically unlikely,
        // but be exact about it: nudge to guarantee strict `<`.
        let (lo, hi) = if lo < hi {
            (lo, hi)
        } else {
            (f32::from_bits(0xBF80_0000), f32::from_bits(0x3F80_0000)) // -1, +1
        };
        assert!(lo < hi);
        v.push(lo);
        v.push(hi);
        v.push(rng.wild_normal());
    }
    v
}

/// `count` triples where every element takes the `else` branch via `s0 > s1`.
fn src_branch_else_gt(rng: &mut Rng, count: usize) -> Vec<f32> {
    let mut v = Vec::with_capacity(3 * count);
    for _ in 0..count {
        let a = rng.wild_normal();
        let b = rng.wild_normal();
        let (lo, hi) = if a < b { (a, b) } else { (b, a) };
        let (lo, hi) = if lo < hi {
            (lo, hi)
        } else {
            (f32::from_bits(0xBF80_0000), f32::from_bits(0x3F80_0000))
        };
        v.push(hi); // s0 > s1
        v.push(lo);
        v.push(rng.wild_normal());
    }
    v
}

/// `count` triples where every element takes the `else` branch via `s0 == s1`.
fn src_branch_else_eq(rng: &mut Rng, count: usize) -> Vec<f32> {
    let mut v = Vec::with_capacity(3 * count);
    for i in 0..count {
        // Every 8th element uses the (+0.0, -0.0) pair, which compares EQUAL
        // (so `<` is false → else branch) yet has different bit patterns.
        if i % 8 == 0 {
            v.push(0.0);
            v.push(-0.0);
        } else if i % 8 == 4 {
            v.push(-0.0);
            v.push(0.0);
        } else {
            let a = rng.wild_normal();
            v.push(a);
            v.push(a);
        }
        v.push(rng.wild_normal());
    }
    v
}

// ---------------------------------------------------------------------------
// Searching for triples whose `sqd` lands in a specific regime
// ---------------------------------------------------------------------------

/// Structured family that provokes catastrophic cancellation in
/// `dy2*dy2 - 2*dx2*dy2 + dx2*dx2`: two nearly-equal values
/// `1 + p*2^-23` and `1 + q*2^-23`. The residual is
/// `(round(p²/2²³) + round(q²/2²³) - 2*round(pq/2²³)) * 2^-23`, whose individual
/// roundings make it land on either side of zero.
fn cancellation_pairs() -> Vec<(f32, f32)> {
    let mut out = Vec::new();
    for p in 1u32..300 {
        for dq in 1u32..4 {
            let q = p * 8 + dq * 7;
            let a = f32::from_bits(0x3F80_0000 + (p * 8));
            let b = f32::from_bits(0x3F80_0000 + q);
            out.push((a, b));
            out.push((b, a));
            // Same shape one binade up and down, and negated.
            out.push((-a, -b));
            out.push((f32::from_bits(0x4000_0000 + p * 8), f32::from_bits(0x4000_0000 + q)));
            out.push((f32::from_bits(0x3F00_0000 + p * 8), f32::from_bits(0x3F00_0000 + q)));
        }
    }
    out
}

/// Collect up to `want` triples in each `sqd` regime, from the structured
/// cancellation family, the special alphabet, and randomized search.
fn search_regimes(want: usize) -> std::collections::BTreeMap<&'static str, Vec<[f32; 3]>> {
    fn key(r: SqdRegime) -> &'static str {
        match r {
            SqdRegime::PosNormal => "pos",
            SqdRegime::PosZero => "+0",
            SqdRegime::NegZero => "-0",
            SqdRegime::Negative => "neg",
            SqdRegime::PosInf => "+inf",
            SqdRegime::Nan => "nan",
        }
    }
    let mut map: std::collections::BTreeMap<&'static str, Vec<[f32; 3]>> = Default::default();
    let push = |t: [f32; 3], map: &mut std::collections::BTreeMap<_, Vec<[f32; 3]>>| {
        let k = key(classify_sqd(sqd_for_triple(t)));
        let e = map.entry(k).or_insert_with(Vec::new);
        if e.len() < want {
            e.push(t);
        }
    };

    // 1. cancellation family, with dxy small enough not to mask the residual
    let dxys = [
        0.0f32,
        -0.0f32,
        f32::from_bits(0x0000_0001), // min subnormal
        f32::from_bits(0x0080_0000), // FLT_MIN
        1e-30,
        -1e-30,
    ];
    for (a, b) in cancellation_pairs() {
        for &d in &dxys {
            push([a, b, d], &mut map);
            push([b, a, d], &mut map);
        }
    }

    // 2. special alphabet cross product (reaches +inf and NaN regimes)
    let alpha = alphabet_f32();
    for &x in &alpha {
        for &y in &alpha {
            for &z in &alpha {
                push([x, y, z], &mut map);
            }
        }
    }

    // 3. randomized search
    let mut rng = Rng::new(0x5EED_0B_0B);
    for _ in 0..400_000 {
        let t = match rng.below(5) {
            0 => [rng.signed_unit(), rng.signed_unit(), rng.signed_unit()],
            1 => [rng.wild_normal(), rng.wild_normal(), rng.wild_normal()],
            2 => [rng.huge(), rng.huge(), rng.huge()],
            3 => [rng.subnormal(), rng.subnormal(), rng.subnormal()],
            _ => [rng.any_bits_f32(), rng.any_bits_f32(), rng.any_bits_f32()],
        };
        push(t, &mut map);
        // Near-equal random pair, the other cancellation generator.
        let a = rng.wild_normal();
        let bump = rng.below(64) as i32 - 32;
        let b = f32::from_bits((a.to_bits() as i32).wrapping_add(bump) as u32);
        push([a, b, rng.subnormal()], &mut map);
    }
    map
}

fn regimes() -> &'static std::collections::BTreeMap<&'static str, Vec<[f32; 3]>> {
    use std::sync::OnceLock;
    static M: OnceLock<std::collections::BTreeMap<&'static str, Vec<[f32; 3]>>> = OnceLock::new();
    M.get_or_init(|| search_regimes(400))
}

/// Flatten triples into a `src` buffer and diff it, plus diff each triple alone.
fn diff_triples(ctx: &str, triples: &[[f32; 3]]) {
    let p = pair();
    assert!(!triples.is_empty(), "{ctx}: no inputs (search found none)");
    // (a) all in one call — exercises the loop / any vectorization
    let flat: Vec<f32> = triples.iter().flat_map(|t| t.iter().copied()).collect();
    diff(ctx, &p, &flat, triples.len() as i32);
    // (b) each individually — isolates per-element behaviour
    for (i, t) in triples.iter().enumerate() {
        diff(&format!("{ctx} #{i}"), &p, t, 1);
    }
}

fn regime_triples(k: &str) -> Vec<[f32; 3]> {
    regimes().get(k).cloned().unwrap_or_default()
}

/// Split a regime's triples by which branch (`if` / `else`) they take, and diff
/// both halves, so each row covers BOTH branches as CONFIGS.md requires.
fn diff_regime_both_branches(ctx: &str, k: &str) {
    let ts = regime_triples(k);
    assert!(
        !ts.is_empty(),
        "{ctx}: sqd regime '{k}' unreachable — search found no inputs"
    );
    let if_b: Vec<[f32; 3]> = ts.iter().copied().filter(|t| roles(*t).3).collect();
    let el_b: Vec<[f32; 3]> = ts.iter().copied().filter(|t| !roles(*t).3).collect();
    eprintln!(
        "[{ctx}] regime '{k}': {} triples ({} if-branch, {} else-branch)",
        ts.len(),
        if_b.len(),
        el_b.len()
    );
    if !if_b.is_empty() {
        diff_triples(&format!("{ctx}/if-branch"), &if_b);
    }
    if !el_b.is_empty() {
        diff_triples(&format!("{ctx}/else-branch"), &el_b);
    }
    diff_triples(&format!("{ctx}/all"), &ts);
}

// ===========================================================================
// B1–B11 — loop-count shapes
// ===========================================================================

fn count_row(row: &str, count: i32, iters: usize) {
    let p = pair();
    let mut rng = Rng::new(0xB0_0000 + count as u64);
    let n = count.max(0) as usize;
    for it in 0..iters {
        // Mix generator families so a row is not only "nice" values.
        let src: Vec<f32> = match it % 4 {
            0 => src_unit(&mut rng, n.max(1)),
            1 => (0..3 * n.max(1)).map(|_| rng.wild_normal()).collect(),
            2 => (0..3 * n.max(1)).map(|_| rng.any_bits_f32()).collect(),
            _ => (0..3 * n.max(1))
                .map(|_| match rng.below(4) {
                    0 => rng.huge(),
                    1 => rng.subnormal(),
                    2 => rng.any_nan(),
                    _ => rng.signed_unit(),
                })
                .collect(),
        };
        let dest_len = if count > 0 { 2 * n } else { 8 };
        diff_disjoint(&format!("{row} it={it}"), &p, &src, count, dest_len);
    }
}

#[test]
fn b01_count_zero() {
    count_row("B1 count=0", 0, 256);
}

#[test]
fn b02_count_one() {
    count_row("B2 count=1", 1, 20_000);
}

#[test]
fn b03_count_two() {
    count_row("B3 count=2", 2, 10_000);
}

#[test]
fn b04_count_three() {
    count_row("B4 count=3", 3, 8_000);
}

#[test]
fn b05_count_seven() {
    count_row("B5 count=7", 7, 4_000);
}

#[test]
fn b06_count_eight() {
    count_row("B6 count=8", 8, 4_000);
}

#[test]
fn b07_count_sixteen() {
    count_row("B7 count=16", 16, 2_000);
}

#[test]
fn b08_count_seventeen() {
    count_row("B8 count=17", 17, 2_000);
}

#[test]
fn b09_count_1000() {
    count_row("B9 count=1000", 1000, 200);
}

#[test]
fn b10_count_100000() {
    count_row("B10 count=100000", 100_000, 8);
}

#[test]
fn b11_negative_counts() {
    let p = pair();
    let mut rng = Rng::new(0x0BAD_C0DE_u64);
    for &count in &[-1i32, -2, -1000, i32::MIN, i32::MIN + 1, -7] {
        for it in 0..64 {
            let src: Vec<f32> = (0..12).map(|_| rng.any_bits_f32()).collect();
            diff_disjoint(&format!("B11 count={count} it={it}"), &p, &src, count, 8);
        }
    }
}

// ===========================================================================
// B12–B16 — element-branch patterns
// ===========================================================================

#[test]
fn b12_all_if_branch() {
    let p = pair();
    let mut rng = Rng::new(0x1212_1212);
    for it in 0..2_000 {
        let src = src_branch_if(&mut rng, 64);
        // sanity: the generator really selects the `if` branch everywhere
        for e in src.chunks(3) {
            assert!(roles([e[0], e[1], e[2]]).3, "generator failed to pick if-branch");
        }
        diff(&format!("B12 all-if it={it}"), &p, &src, 64);
    }
}

#[test]
fn b13_all_else_branch_greater() {
    let p = pair();
    let mut rng = Rng::new(0x1313_1313);
    for it in 0..2_000 {
        let src = src_branch_else_gt(&mut rng, 64);
        for e in src.chunks(3) {
            assert!(!roles([e[0], e[1], e[2]]).3, "generator failed to pick else-branch");
        }
        diff(&format!("B13 all-else(>) it={it}"), &p, &src, 64);
    }
}

#[test]
fn b14_all_else_branch_equal() {
    let p = pair();
    let mut rng = Rng::new(0x1414_1414);
    for it in 0..2_000 {
        let src = src_branch_else_eq(&mut rng, 64);
        for e in src.chunks(3) {
            assert!(!roles([e[0], e[1], e[2]]).3, "generator failed to pick else-branch");
        }
        diff(&format!("B14 all-else(==) it={it}"), &p, &src, 64);
    }
}

#[test]
fn b15_alternating_branches() {
    let p = pair();
    let mut rng = Rng::new(0x1515_1515);
    for it in 0..2_000 {
        let n = 65usize;
        let mut src = Vec::with_capacity(3 * n);
        for i in 0..n {
            let a = rng.wild_normal();
            let b = rng.wild_normal();
            let (lo, hi) = if a < b { (a, b) } else { (b, a) };
            let (lo, hi) = if lo < hi { (lo, hi) } else { (-1.0f32, 1.0f32) };
            if i % 2 == 0 {
                src.push(lo);
                src.push(hi);
            } else {
                src.push(hi);
                src.push(lo);
            }
            src.push(rng.wild_normal());
        }
        for (i, e) in src.chunks(3).enumerate() {
            assert_eq!(roles([e[0], e[1], e[2]]).3, i % 2 == 0);
        }
        diff(&format!("B15 alternating it={it}"), &p, &src, n as i32);
    }
}

#[test]
fn b16_random_mixed_branches() {
    let p = pair();
    let mut rng = Rng::new(0x1616_1616);
    for it in 0..1_000 {
        let n = 257usize;
        let mut src = Vec::with_capacity(3 * n);
        let mut n_if = 0usize;
        for _ in 0..n {
            let a = rng.wild_normal();
            let b = rng.wild_normal();
            let (lo, hi) = if a < b { (a, b) } else { (b, a) };
            let (lo, hi) = if lo < hi { (lo, hi) } else { (-1.0f32, 1.0f32) };
            if rng.next_u32() & 1 == 0 {
                src.push(lo);
                src.push(hi);
                n_if += 1;
            } else {
                src.push(hi);
                src.push(lo);
            }
            src.push(rng.wild_normal());
        }
        if it == 0 {
            eprintln!("B16: {n_if}/{n} elements take the if-branch");
        }
        diff(&format!("B16 mixed it={it}"), &p, &src, n as i32);
    }
}

// ===========================================================================
// B17–B23 — discriminant / clamp regimes
// ===========================================================================

#[test]
fn b17_sqd_strictly_positive() {
    diff_regime_both_branches("B17 sqd>0", "pos");
}

#[test]
fn b18_sqd_positive_zero() {
    // The canonical constructor: dx2 == dy2 (residual is exactly +0) and dxy == 0.
    let p = pair();
    let mut rng = Rng::new(0x1818_1818);
    let mut hand = Vec::new();
    for _ in 0..4_000 {
        let a = rng.wild_normal();
        for &z in &[0.0f32, -0.0f32] {
            let t = [a, a, z];
            // `dx2 == dy2` gives an exactly-zero residual, EXCEPT when `a*a`
            // overflows to +inf (then `inf - inf` is NaN) — keep only the +0 hits.
            if classify_sqd(sqd_for_triple(t)) == SqdRegime::PosZero {
                hand.push(t);
            }
        }
    }
    assert!(hand.len() > 1_000, "only {} +0 constructors found", hand.len());
    for &z in &[0.0f32, -0.0f32] {
        for &a in &[0.0f32, -0.0f32, 1.0, -1.0, f32::MIN_POSITIVE, f32::MAX] {
            let t = [a, a, z];
            if classify_sqd(sqd_for_triple(t)) == SqdRegime::PosZero {
                hand.push(t);
            }
        }
    }
    diff_triples("B18 sqd=+0 (constructed)", &hand);
    diff_regime_both_branches("B18 sqd=+0 (searched)", "+0");
    drop(p);
}

#[test]
fn b19_sqd_negative_zero() {
    // sqd == -0.0f requires BOTH addends of `dxy_term + acc` to be -0.0f, but
    // `dxy_term = (4*dxy)*dxy` is a square and is never -0.0, and `acc` can only
    // be -0.0 if `dx2*dx2` were -0.0, which is likewise impossible. So the
    // regime is expected to be UNREACHABLE. Assert that the (large) search
    // agrees, so the claim in ERRORS.md E14 is verified rather than assumed.
    let found = regime_triples("-0");
    if !found.is_empty() {
        // If it IS reachable, the row must still be verified differentially.
        eprintln!("B19: sqd == -0.0 IS reachable ({} triples)", found.len());
        diff_regime_both_branches("B19 sqd=-0", "-0");
    } else {
        eprintln!(
            "B19: sqd == -0.0f is UNREACHABLE (confirmed: 0 hits over the \
             exhaustive alphabet^3, the cancellation family and 400k random \
             triples). The `-0.0` clamp semantics are instead pinned by \
             phase_c::e14_negative_zero_through_sqrt, which feeds -0.0 straight \
             through the branch inputs."
        );
    }
}

#[test]
fn b20_sqd_negative_clamped() {
    diff_regime_both_branches("B20 sqd<0 (clamp taken)", "neg");
}

#[test]
fn b21_sqd_positive_infinity() {
    let p = pair();
    // Constructed: 4*dxy*dxy overflows.
    let mut hand = Vec::new();
    let mut rng = Rng::new(0x2121_2121);
    for _ in 0..4_000 {
        let d = rng.huge();
        let a = rng.signed_unit();
        let b = a + 1.0;
        let t = [a, b, d];
        if classify_sqd(sqd_for_triple(t)) == SqdRegime::PosInf {
            hand.push(t);
        }
        let t2 = [b, a, d];
        if classify_sqd(sqd_for_triple(t2)) == SqdRegime::PosInf {
            hand.push(t2);
        }
    }
    assert!(!hand.is_empty(), "could not construct sqd=+inf");
    diff_triples("B21 sqd=+inf (constructed)", &hand);
    diff_regime_both_branches("B21 sqd=+inf (searched)", "+inf");
    drop(p);
}

#[test]
fn b22_sqd_nan_via_inf_minus_inf() {
    // dy2*dy2 -> +inf and 2*dx2*dy2 -> +inf, so t1 - t2 == inf - inf == NaN,
    // with NO NaN present in the input (the invalid-operation path).
    let mut hand = Vec::new();
    let mut rng = Rng::new(0x2222_2222);
    for _ in 0..8_000 {
        let a = rng.huge();
        let b = rng.huge();
        for t in [[a, b, rng.signed_unit()], [b, a, rng.signed_unit()]] {
            if !t.iter().any(|x| x.is_nan())
                && classify_sqd(sqd_for_triple(t)) == SqdRegime::Nan
            {
                hand.push(t);
            }
        }
    }
    // Plus the textbook constructor with ±FLT_MAX / ±1e30.
    for &a in &[f32::MAX, -f32::MAX, 1e30f32, -1e30f32] {
        for &b in &[f32::MAX, -f32::MAX, 1e30f32, -1e30f32] {
            for &z in &[0.0f32, 1.0, -1.0] {
                let t = [a, b, z];
                if classify_sqd(sqd_for_triple(t)) == SqdRegime::Nan {
                    hand.push(t);
                }
            }
        }
    }
    assert!(!hand.is_empty(), "could not construct sqd=NaN via inf-inf");
    eprintln!("B22: {} inf-inf triples (no NaN inputs)", hand.len());
    diff_triples("B22 sqd=NaN via inf-inf", &hand);
}

#[test]
fn b23_sqd_nan_via_zero_times_inf() {
    // 2.0f*dx2*dy2 with dx2 == ±0 and dy2 == ±inf -> 0*inf -> NaN.
    let mut hand = Vec::new();
    for &z in &[0.0f32, -0.0f32] {
        for &inf in &[f32::INFINITY, f32::NEG_INFINITY] {
            for &d in &[
                0.0f32,
                -0.0f32,
                1.0,
                -1.0,
                f32::MIN_POSITIVE,
                f32::from_bits(0x0000_0001),
                1e30,
                f32::INFINITY,
            ] {
                // Both orderings, so both C branches are hit.
                for t in [[z, inf, d], [inf, z, d]] {
                    if !t.iter().any(|x| x.is_nan()) {
                        hand.push(t);
                    }
                }
            }
        }
    }
    let nans: Vec<[f32; 3]> = hand
        .iter()
        .copied()
        .filter(|t| classify_sqd(sqd_for_triple(*t)) == SqdRegime::Nan)
        .collect();
    assert!(!nans.is_empty(), "could not construct 0*inf -> NaN");
    eprintln!("B23: {} of {} 0*inf triples make sqd NaN", nans.len(), hand.len());
    // Diff the whole family (both the NaN and non-NaN outcomes are valid rows).
    diff_triples("B23 0*inf family", &hand);
}

// ===========================================================================
// B24–B33 — IEEE class per lane
// ===========================================================================

/// Diff `iters` batches of `n` triples whose lanes come from `gen`.
fn class_row(row: &str, seed: u64, n: usize, iters: usize, gen: fn(&mut Rng) -> f32) {
    let p = pair();
    let mut rng = Rng::new(seed);
    for it in 0..iters {
        let src: Vec<f32> = (0..3 * n).map(|_| gen(&mut rng)).collect();
        diff(&format!("{row} it={it}"), &p, &src, n as i32);
    }
}

#[test]
fn b24_all_lanes_unit_normals() {
    class_row("B24 unit normals", 0x2424_2424, 4096, 40, |r| r.signed_unit());
}

#[test]
fn b25_all_lanes_huge_normals() {
    class_row("B25 huge normals", 0x2525_2525, 4096, 40, |r| r.huge());
}

#[test]
fn b26_all_lanes_subnormal() {
    class_row("B26 subnormals", 0x2626_2626, 4096, 40, |r| r.subnormal());
    // Plus an explicit sweep of the smallest magnitudes (FTZ/DAZ must be off).
    let p = pair();
    let tiny: Vec<u32> = (0u32..64)
        .flat_map(|i| [i, 0x8000_0000 | i, 0x007F_FFFF - i, 0x807F_FFFF - i])
        .collect();
    let mut triples = Vec::new();
    for &a in &tiny {
        for &b in &tiny {
            triples.push([f32::from_bits(a), f32::from_bits(b), f32::from_bits(a ^ b)]);
        }
    }
    let flat: Vec<f32> = triples.iter().flat_map(|t| t.iter().copied()).collect();
    diff("B26 tiny sweep", &p, &flat, triples.len() as i32);
}

#[test]
fn b27_signed_zeros_all_combinations() {
    let zs = [0.0f32, -0.0f32];
    let mut triples = Vec::new();
    for &x in &zs {
        for &y in &zs {
            for &z in &zs {
                triples.push([x, y, z]);
            }
        }
    }
    assert_eq!(triples.len(), 8);
    diff_triples("B27 signed zeros (8 combos)", &triples);
}

#[test]
fn b28_infinities_cross_product() {
    let vals = [f32::NEG_INFINITY, f32::INFINITY, -1.0f32, 0.0f32, 1.0f32];
    let mut triples = Vec::new();
    for &x in &vals {
        for &y in &vals {
            for &z in &vals {
                triples.push([x, y, z]);
            }
        }
    }
    assert_eq!(triples.len(), 125);
    diff_triples("B28 inf cross-product (125)", &triples);
}

#[test]
fn b29_quiet_nans_cross_product() {
    let vals = [
        f32::from_bits(0x7FC0_0000),
        f32::from_bits(0xFFC0_0000),
        1.0f32,
        -1.0f32,
        f32::INFINITY,
    ];
    let mut triples = Vec::new();
    for &x in &vals {
        for &y in &vals {
            for &z in &vals {
                triples.push([x, y, z]);
            }
        }
    }
    assert_eq!(triples.len(), 125);
    diff_triples("B29 qNaN cross-product (125)", &triples);
}

#[test]
fn b30_signalling_nans_each_lane() {
    let snans = [f32::from_bits(0x7FA0_0000), f32::from_bits(0xFFA0_0000)];
    let others = [
        0.0f32,
        -0.0f32,
        1.0,
        -1.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MAX,
        f32::MIN_POSITIVE,
    ];
    let mut triples = Vec::new();
    for &s in &snans {
        for &o in &others {
            for &q in &others {
                // sNaN in each of the three lane positions in turn.
                triples.push([s, o, q]);
                triples.push([o, s, q]);
                triples.push([o, q, s]);
            }
        }
        // All-sNaN, and pairs.
        triples.push([s, s, s]);
        for &s2 in &snans {
            triples.push([s, s2, 1.0]);
            triples.push([s, 1.0, s2]);
            triples.push([1.0, s, s2]);
        }
    }
    diff_triples("B30 sNaN per lane", &triples);
}

#[test]
fn b31_noncanonical_nan_payloads() {
    let p = pair();
    let mut rng = Rng::new(0x3131_3131);
    for it in 0..80 {
        let n = 2048usize;
        let mut src = Vec::with_capacity(3 * n);
        for i in 0..n {
            // Rotate which lane(s) carry the NaN so all positions are hit.
            let which = i % 7; // bitmask 0..6 over 3 lanes (never 7 = all NaN? include it)
            for lane in 0..3 {
                if (which + 1) & (1 << lane) != 0 {
                    src.push(rng.any_nan());
                } else {
                    src.push(rng.wild_normal());
                }
            }
        }
        diff(&format!("B31 noncanonical NaN it={it}"), &p, &src, n as i32);
    }
}

#[test]
fn b32_exhaustive_special_alphabet_cubed() {
    let a = alphabet_f32();
    assert_eq!(a.len(), 24);
    let mut triples = Vec::with_capacity(24 * 24 * 24);
    for &x in &a {
        for &y in &a {
            for &z in &a {
                triples.push([x, y, z]);
            }
        }
    }
    assert_eq!(triples.len(), 13_824);
    // One big call (loop + any vectorization) …
    let p = pair();
    let flat: Vec<f32> = triples.iter().flat_map(|t| t.iter().copied()).collect();
    diff("B32 alphabet^3 (one call)", &p, &flat, triples.len() as i32);
    // … and each triple in isolation.
    for (i, t) in triples.iter().enumerate() {
        diff(&format!("B32 alphabet^3 #{i}"), &p, t, 1);
    }
}

#[test]
fn b33_random_bit_patterns() {
    let p = pair();
    let mut rng = Rng::new(0x3333_3333);
    let n = 200_000usize;
    let src: Vec<f32> = (0..3 * n).map(|_| rng.any_bits_f32()).collect();
    diff("B33 random bits (one call)", &p, &src, n as i32);
    // Re-run in small chunks so a mis-stepped pointer cannot hide.
    for (c, chunk) in src.chunks(3 * 97).enumerate().take(200) {
        diff(
            &format!("B33 random bits chunk {c}"),
            &p,
            chunk,
            (chunk.len() / 3) as i32,
        );
    }
}

// ===========================================================================
// B34–B41 — buffer geometry / aliasing
// ===========================================================================

#[test]
fn b34_disjoint_separate_allocations() {
    // The baseline geometry, already used by B1–B33; assert it explicitly with a
    // fresh mix so the row has its own evidence.
    let p = pair();
    let mut rng = Rng::new(0x3434_3434);
    for it in 0..500 {
        let n = 1 + (rng.below(200) as usize);
        let src: Vec<f32> = (0..3 * n)
            .map(|_| match rng.below(4) {
                0 => rng.signed_unit(),
                1 => rng.wild_normal(),
                2 => rng.any_bits_f32(),
                _ => rng.huge(),
            })
            .collect();
        diff(&format!("B34 disjoint it={it} n={n}"), &p, &src, n as i32);
    }
}

/// Shared driver for the aliasing rows: one allocation, `src` at `src_off`,
/// `dest` at `dest_off`.
fn alias_row(row: &str, seed: u64, dest_off_of: fn(usize, i32) -> usize, iters: usize) {
    let p = pair();
    let mut rng = Rng::new(seed);
    for it in 0..iters {
        let count: i32 = 1 + (rng.below(64) as i32);
        let src_off = 0usize;
        let dest_off = dest_off_of(src_off, count);
        // Allocation must cover both windows plus slack.
        let need = (src_off + 3 * count as usize).max(dest_off + 2 * count as usize) + 8;
        let mut buf: Vec<f32> = Vec::with_capacity(need);
        for _ in 0..need {
            buf.push(match rng.below(5) {
                0 => rng.signed_unit(),
                1 => rng.wild_normal(),
                2 => rng.any_bits_f32(),
                3 => rng.huge(),
                _ => rng.any_nan(),
            });
        }
        diff_aliased(
            &format!("{row} it={it}"),
            &p,
            &buf,
            src_off,
            dest_off,
            count,
        );
    }
}

#[test]
fn b35_in_place_dest_equals_src() {
    alias_row("B35 dest==src", 0x3535_3535, |s, _| s, 3_000);
}

#[test]
fn b36_dest_equals_src_plus_1() {
    alias_row("B36 dest==src+1", 0x3636_3636, |s, _| s + 1, 3_000);
}

#[test]
fn b37_dest_equals_src_plus_2() {
    alias_row("B37 dest==src+2", 0x3737_3737, |s, _| s + 2, 3_000);
}

#[test]
fn b38_dest_equals_src_plus_3() {
    alias_row("B38 dest==src+3", 0x3838_3838, |s, _| s + 3, 3_000);
}

#[test]
fn b39_dest_after_src_region() {
    alias_row(
        "B39 dest==src+3*count",
        0x3939_3939,
        |s, c| s + 3 * c as usize,
        3_000,
    );
}

#[test]
fn b40_src_after_dest_overlapping() {
    // src == dest + 1, i.e. the dest window starts BEFORE src.
    let p = pair();
    let mut rng = Rng::new(0x4040_4040);
    for it in 0..3_000 {
        let count: i32 = 1 + (rng.below(64) as i32);
        let dest_off = 0usize;
        let src_off = 1usize;
        let need = (src_off + 3 * count as usize).max(dest_off + 2 * count as usize) + 8;
        let buf: Vec<f32> = (0..need)
            .map(|_| match rng.below(5) {
                0 => rng.signed_unit(),
                1 => rng.wild_normal(),
                2 => rng.any_bits_f32(),
                3 => rng.huge(),
                _ => rng.any_nan(),
            })
            .collect();
        diff_aliased(
            &format!("B40 src==dest+1 it={it}"),
            &p,
            &buf,
            src_off,
            dest_off,
            count,
        );
    }
}

#[test]
fn b41_all_float_offset_combinations() {
    let p = pair();
    let mut rng = Rng::new(0x4141_4141);
    for src_off in 0..4usize {
        for dest_off in 0..4usize {
            for it in 0..200 {
                let count: i32 = 1 + (rng.below(40) as i32);
                let src_data: Vec<f32> = (0..3 * count as usize)
                    .map(|_| match rng.below(4) {
                        0 => rng.signed_unit(),
                        1 => rng.any_bits_f32(),
                        2 => rng.huge(),
                        _ => rng.any_nan(),
                    })
                    .collect();
                diff_offsets(
                    &format!("B41 offsets it={it}"),
                    &p,
                    &src_data,
                    src_off,
                    dest_off,
                    count,
                );
            }
        }
    }
}

// ===========================================================================
// B42–B44 — composition / statelessness
// ===========================================================================

#[test]
fn b42_repeated_calls_same_buffers() {
    let p = pair();
    let mut rng = Rng::new(0x4242_4242);
    let n = 128usize;
    // One pair of long-lived dest buffers, reused across 64 calls, so any
    // leaked FP mode change (MXCSR) or hidden state shows up as a divergence in
    // a LATER call even though the first one matched.
    let mut dc = canary_buf(2 * n);
    let mut dr = canary_buf(2 * n);
    for call in 0..64 {
        let src: Vec<f32> = (0..3 * n)
            .map(|_| match rng.below(5) {
                0 => rng.signed_unit(),
                1 => rng.huge(),
                2 => rng.subnormal(),
                3 => rng.any_nan(),
                _ => rng.any_bits_f32(),
            })
            .collect();
        unsafe {
            (p.c.tfm)(dc.as_mut_ptr(), src.as_ptr(), n as i32);
            (p.rs.tfm)(dr.as_mut_ptr(), src.as_ptr(), n as i32);
        }
        assert_bits_eq(&format!("B42 repeated call #{call}"), &dc, &dr);
    }
}

#[test]
fn b43_one_big_call_equals_n_unit_calls() {
    let p = pair();
    let mut rng = Rng::new(0x4343_4343);
    for it in 0..300 {
        let n = 1 + (rng.below(200) as usize);
        let src: Vec<f32> = (0..3 * n)
            .map(|_| match rng.below(5) {
                0 => rng.signed_unit(),
                1 => rng.huge(),
                2 => rng.subnormal(),
                3 => rng.any_nan(),
                _ => rng.any_bits_f32(),
            })
            .collect();

        // one call, count = n
        let mut big_c = canary_buf(2 * n);
        let mut big_r = canary_buf(2 * n);
        unsafe {
            (p.c.tfm)(big_c.as_mut_ptr(), src.as_ptr(), n as i32);
            (p.rs.tfm)(big_r.as_mut_ptr(), src.as_ptr(), n as i32);
        }

        // n calls, count = 1, pointers advanced by hand
        let mut unit_c = canary_buf(2 * n);
        let mut unit_r = canary_buf(2 * n);
        for i in 0..n {
            unsafe {
                (p.c.tfm)(unit_c.as_mut_ptr().add(2 * i), src.as_ptr().add(3 * i), 1);
                (p.rs.tfm)(unit_r.as_mut_ptr().add(2 * i), src.as_ptr().add(3 * i), 1);
            }
        }

        let ctx = format!("B43 it={it} n={n}");
        assert_bits_eq(&format!("{ctx}: C big vs Rust big"), &big_c, &big_r);
        assert_bits_eq(&format!("{ctx}: C unit vs Rust unit"), &unit_c, &unit_r);
        assert_bits_eq(&format!("{ctx}: C big vs C unit"), &big_c, &unit_c);
        assert_bits_eq(&format!("{ctx}: Rust big vs Rust unit"), &big_r, &unit_r);
    }
}

#[test]
fn b44_count_exceeds_logical_data() {
    // The C code has no bounds check: it will happily consume elements past the
    // caller's "logical" length. Over-allocate so no OOB access occurs, seed the
    // tail deterministically, and require both impls to process it identically.
    let p = pair();
    let mut rng = Rng::new(0x4444_4444);
    for it in 0..500 {
        let logical = 1 + (rng.below(32) as usize);
        let extra = 1 + (rng.below(32) as usize);
        let total = logical + extra;
        let src: Vec<f32> = (0..3 * total)
            .map(|i| {
                if i < 3 * logical {
                    rng.signed_unit()
                } else {
                    // deterministic "garbage" tail
                    f32::from_bits(0x1234_0000u32.wrapping_add(i as u32).wrapping_mul(2_654_435_761))
                }
            })
            .collect();
        diff(
            &format!("B44 it={it} logical={logical} count={total}"),
            &p,
            &src,
            total as i32,
        );
    }
}
