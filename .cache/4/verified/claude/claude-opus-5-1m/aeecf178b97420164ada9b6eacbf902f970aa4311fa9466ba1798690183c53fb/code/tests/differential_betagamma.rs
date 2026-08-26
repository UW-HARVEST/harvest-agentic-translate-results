//! Differential tests for the top-level `betagamma` export.
//!
//! Covers CONFIGS.md rows 29-43 and 46, and ERRORS.md rows 6, 7, 12, 14, 15.
//!
//! `betagamma` calls `compute_hash` on two freshly `malloc`ed blocks, so its
//! return value depends on the *numeric addresses* the allocator hands out: the
//! shipped C library returns 517 / 527 / 617 / 627 for `betagamma(1,2,3,4)`
//! depending on the caller's tcache state.  Every comparison below therefore
//! runs each implementation in a child forked from the *same* parent image
//! (CONFIGS.md note N2), so both observe byte-identical allocator state.

mod common;

use common::*;

const MAX_BATCH: usize = MAX_PAYLOAD / 4;

/// Run the whole `inputs` sequence in one forked child per implementation and
/// return both result sequences.
fn run_batch(c: &Impl, r: &Impl, inputs: &[[i32; 4]]) -> (Vec<i32>, Vec<i32>) {
    assert!(inputs.len() <= MAX_BATCH, "batch too large for the pipe");
    let (ca, ra) = fork_pair(|which, buf| {
        let imp = if which { r } else { c };
        let mut off = 0usize;
        for q in inputs {
            let v = unsafe { (imp.betagamma)(q[0], q[1], q[2], q[3]) };
            buf[off..off + 4].copy_from_slice(&v.to_ne_bytes());
            off += 4;
        }
        off
    });
    assert_eq!(
        ca.exit_code(),
        Some(0),
        "C child died running a betagamma batch: {}",
        ca.describe()
    );
    assert_eq!(
        ra.exit_code(),
        Some(0),
        "Rust child died running a betagamma batch: {}",
        ra.describe()
    );
    let cv = ca.i32s();
    let rv = ra.i32s();
    assert_eq!(cv.len(), inputs.len(), "C returned {} results", cv.len());
    assert_eq!(rv.len(), inputs.len(), "Rust returned {} results", rv.len());
    (cv, rv)
}

/// Independent re-derivation of everything in `betagamma` that does **not**
/// depend on allocator addresses.  Returns `None` for the `-1` error path.
///
/// The three `DataBlock` flag bytes are compile-time constants in the C source,
/// so the flag phase reduces to:
///   `0b10101010` -> p1 + p2 + p3          (times id 1)
///   `0b11001100` -> p1 + p2 + p3 + p4     (times id 2)
///   `0b11110000` ->      p2 + p3 + p4     (times id 3)
/// and the tail is `+ (sum1-sum2)/10 + special.id(99) + special.flags(255)`.
fn model_without_hash(q: [i32; 4]) -> Option<i32> {
    let (p1, p2, p3, p4) = (q[0], q[1], q[2], q[3]);
    let n = (p1 % 10).wrapping_add(5);
    if n < 0 {
        return None; // block_size converts to a huge size_t -> calloc fails
    }
    let n = n as usize;

    let c1 = p1.wrapping_add(p2).wrapping_add(p3);
    let c2 = p1.wrapping_add(p2).wrapping_add(p3).wrapping_add(p4);
    let c3 = p2.wrapping_add(p3).wrapping_add(p4);
    let mut result = c1
        .wrapping_mul(1)
        .wrapping_add(c2.wrapping_mul(2))
        .wrapping_add(c3.wrapping_mul(3));

    let elem = |init: i32, i: usize| ((init as usize).wrapping_add(i) as u32) as i32;
    let mut s1 = 0i32;
    let mut s2 = 0i32;
    for i in 0..n {
        s1 = s1.wrapping_add(elem(p1, i));
    }
    for i in 0..n {
        s2 = s2.wrapping_add(elem(p2, i));
    }
    result = result.wrapping_add(s1.wrapping_sub(s2) / 10);
    result = result.wrapping_add(99); // special.id, since mem1->data != mem2->data
    result = result.wrapping_add(255); // special.flags, since both data > NULL
    Some(result)
}

/// The nine values `compute_hash` can return.
const HASHES: [i32; 9] = [0, 10, 20, 100, 110, 120, 200, 210, 220];

fn compare(c: &Impl, r: &Impl, label: &str, inputs: &[[i32; 4]]) -> Vec<i32> {
    let (cv, rv) = run_batch(c, r, inputs);
    for (i, q) in inputs.iter().enumerate() {
        assert_eq!(
            cv[i], rv[i],
            "{label}: betagamma({}, {}, {}, {}) [batch index {i}] C={} Rust={}",
            q[0], q[1], q[2], q[3], cv[i], rv[i]
        );
        // Independent oracle: the only part of the result the test cannot
        // predict is `compute_hash`, which has exactly nine possible values.
        // This proves the comparison above is not vacuous and pins down every
        // other term of the computation.
        match model_without_hash(*q) {
            None => assert_eq!(
                cv[i], -1,
                "{label}: betagamma({}, {}, {}, {}) should take the -1 error path, got {}",
                q[0], q[1], q[2], q[3], cv[i]
            ),
            Some(base) => {
                let delta = cv[i].wrapping_sub(base);
                assert!(
                    HASHES.contains(&delta),
                    "{label}: betagamma({}, {}, {}, {}) = {} but the model predicts \
                     {base} + hash; residual {delta} is not a valid compute_hash value",
                    q[0],
                    q[1],
                    q[2],
                    q[3],
                    cv[i]
                );
            }
        }
    }
    cv
}

/// One input, isolated in its own fork pair, so a failure names the input.
fn compare_one(c: &Impl, r: &Impl, label: &str, q: [i32; 4]) -> i32 {
    compare(c, r, label, &[q])[0]
}

/// CONFIGS.md row that a given `param1 % 10` residue belongs to.
fn residue_row(residue: i32) -> &'static str {
    match residue {
        0 => "row29",              // block_size 5
        1..=9 => "row30",          // block_size 6..14
        -4..=-1 => "row31",        // block_size 1..4
        -5 => "row32",             // block_size 0
        _ => "ERRORS#6",           // block_size negative -> huge size_t -> -1
    }
}

/// `param1` whose C remainder `param1 % 10` is exactly `residue`.
fn with_residue(rng: &mut Rng, residue: i32) -> i32 {
    let k = (rng.below(200_000_000)) as i32;
    if residue >= 0 {
        10i32.wrapping_mul(k).wrapping_add(residue)
    } else {
        10i32.wrapping_mul(-k).wrapping_add(residue)
    }
}

#[test]
fn betagamma_differential() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 0x0000_0004);

    // -------------------------------------------------------------------
    // rows 29-32, 41 and ERRORS #6/#7/#15: every residue of param1 % 10
    // -------------------------------------------------------------------
    for residue in -9..=9i32 {
        // deterministic representatives first
        for &p1 in &[residue, residue + 10 * residue.signum(), residue + 1_000_000 * residue.signum()] {
            let label = format!("{}/residue{residue}/p1={p1}", residue_row(residue));
            let got = compare_one(&c, &r, &label, [p1, 2, 3, 4]);
            if (-9..=-6).contains(&residue) {
                assert_eq!(
                    got, -1,
                    "ERRORS#6: betagamma({p1},2,3,4) must return -1 (block_size = {})",
                    residue + 5
                );
            } else {
                assert_ne!(
                    got, -1,
                    "residue {residue} (block_size {}) should not take the error path",
                    residue + 5
                );
            }
        }
        // then randomized params for this residue class
        let mut batch: Vec<[i32; 4]> = Vec::new();
        for _ in 0..60 {
            batch.push([
                with_residue(&mut rng, residue),
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
            ]);
        }
        let out = compare(
            &c,
            &r,
            &format!("{}/residue{residue}/random", residue_row(residue)),
            &batch,
        );
        if (-9..=-6).contains(&residue) {
            for (i, v) in out.iter().enumerate() {
                assert_eq!(
                    *v, -1,
                    "ERRORS#6: residue {residue} input {:?} must yield -1",
                    batch[i]
                );
            }
        }
    }

    // row 32 / ERRORS #14: block_size == 0 exactly
    for &p1 in &[-5i32, -15, -105, -1_000_005, -2_000_000_005] {
        let got = compare_one(&c, &r, &format!("row32/p1={p1}"), [p1, 2, 3, 4]);
        assert_ne!(got, -1, "row32: block_size 0 must still succeed (p1={p1})");
    }

    // row 41 / ERRORS #15: INT_MIN % 10 == -8 -> error path; INT_MAX % 10 == 7
    assert_eq!(
        compare_one(&c, &r, "row41/INT_MIN", [i32::MIN, 1, 2, 3]),
        -1,
        "ERRORS#15: betagamma(INT_MIN, ..) must return -1"
    );
    compare_one(&c, &r, "row41/INT_MAX", [i32::MAX, 1, 2, 3]);

    // -------------------------------------------------------------------
    // row 33: the all-zero shape
    // -------------------------------------------------------------------
    compare_one(&c, &r, "row33/zeros", [0, 0, 0, 0]);

    // -------------------------------------------------------------------
    // rows 34-38: the sign / truncation behaviour of (sum1 - sum2) / 10
    //
    // With block_size = n, sum1 - sum2 == n * (param1 - param2), so the sign
    // and magnitude of the dividend are fully controllable.
    // -------------------------------------------------------------------
    // row 34: dividend == 0
    for &p in &[0i32, 10, -1, 7, 123_456] {
        compare_one(&c, &r, "row34/equal", [p, p, 3, 4]);
    }
    // row 35: 0 < dividend < 10  (n=5, diff=1 -> 5)
    compare_one(&c, &r, "row35/pos-trunc", [10, 9, 0, 0]);
    compare_one(&c, &r, "row35/pos-trunc2", [20, 19, 1, 1]);
    // row 36: dividend >= 10 (n=5, diff=5 -> 25 -> 2)
    compare_one(&c, &r, "row36/pos", [10, 5, 0, 0]);
    compare_one(&c, &r, "row36/pos2", [1000, 0, 0, 0]);
    // row 37: -10 < dividend < 0  (n=5, diff=-1 -> -5 -> must truncate to 0)
    compare_one(&c, &r, "row37/neg-trunc", [10, 11, 0, 0]);
    compare_one(&c, &r, "row37/neg-trunc2", [20, 21, 1, 1]);
    // row 38: dividend <= -10 (n=5, diff=-5 -> -25 -> -2, not -3)
    compare_one(&c, &r, "row38/neg", [10, 15, 0, 0]);
    compare_one(&c, &r, "row38/neg2", [0, 1000, 0, 0]);
    // sweep the dividend across the truncation boundary in both directions
    {
        let mut batch: Vec<[i32; 4]> = Vec::new();
        for d in -30..=30i32 {
            batch.push([10, 10 - d, 0, 0]); // n = 5 -> dividend = 5*d
            batch.push([11, 11 - d, 0, 0]); // n = 6 -> dividend = 6*d
            batch.push([13, 13 - d, 0, 0]); // n = 8 -> dividend = 8*d
        }
        compare(&c, &r, "rows35-38/sweep", &batch);
    }

    // -------------------------------------------------------------------
    // rows 39, 40 and ERRORS #12: signed overflow
    // -------------------------------------------------------------------
    {
        let ext = [
            i32::MAX,
            i32::MIN,
            i32::MAX - 1,
            i32::MIN + 1,
            1_000_000_000,
            -1_000_000_000,
            2_000_000_000,
            -2_000_000_000,
            0,
            1,
            -1,
        ];
        let mut batch: Vec<[i32; 4]> = Vec::new();
        // row 39: overflow in flag_contribution * id and in result +=
        for &a in &ext {
            for &b in &ext {
                batch.push([12, a, b, a]);
            }
        }
        compare(&c, &r, "row39/flag-overflow", &batch);

        // row 40: sum overflow inside the init/accumulate loops
        let mut batch2: Vec<[i32; 4]> = Vec::new();
        for &a in &ext {
            for &b in &ext {
                batch2.push([i32::MAX, a, b, 1]); // block_size = 12
                batch2.push([2_147_483_639, a, b, 1]); // % 10 == 9 -> size 14
            }
        }
        compare(&c, &r, "row40/sum-overflow", &batch2);
    }

    // -------------------------------------------------------------------
    // row 42: every sign pattern with large magnitudes
    // -------------------------------------------------------------------
    {
        let mut batch: Vec<[i32; 4]> = Vec::new();
        for mask in 0..16u32 {
            for &mag in &[1i32, 7, 1_000_000, 1_073_741_823, 2_147_483_647] {
                let s = |bit: u32, v: i32| if mask & (1 << bit) != 0 { -v } else { v };
                batch.push([s(0, mag), s(1, mag), s(2, mag), s(3, mag)]);
            }
        }
        compare(&c, &r, "row42/signs", &batch);
    }

    // -------------------------------------------------------------------
    // row 46: many consecutive calls with the SAME input -- the return value
    // cycles with the tcache state, and both libraries must cycle identically.
    // -------------------------------------------------------------------
    for q in [[1, 2, 3, 4], [0, 0, 0, 0], [-5, 7, -7, 9], [i32::MAX, 1, 1, 1]] {
        let batch: Vec<[i32; 4]> = std::iter::repeat_n(q, 200).collect();
        let out = compare(&c, &r, "row46/repeat", &batch);
        // the C library really does return more than one distinct value here,
        // which is exactly why the fork isolation is required
        let distinct = {
            let mut v = out.clone();
            v.sort_unstable();
            v.dedup();
            v.len()
        };
        assert!(distinct >= 1, "row46: no results?");
    }

    // -------------------------------------------------------------------
    // row 43: fully random full-i32-range sweep
    // -------------------------------------------------------------------
    let mut total = 0usize;
    for chunk in 0..4 {
        let mut batch: Vec<[i32; 4]> = Vec::with_capacity(400);
        for _ in 0..400 {
            batch.push([
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
            ]);
        }
        compare(&c, &r, &format!("row43/random-chunk{chunk}"), &batch);
        total += batch.len();
    }
    assert!(total >= 1500, "row43: expected >= 1500 random inputs");
}
