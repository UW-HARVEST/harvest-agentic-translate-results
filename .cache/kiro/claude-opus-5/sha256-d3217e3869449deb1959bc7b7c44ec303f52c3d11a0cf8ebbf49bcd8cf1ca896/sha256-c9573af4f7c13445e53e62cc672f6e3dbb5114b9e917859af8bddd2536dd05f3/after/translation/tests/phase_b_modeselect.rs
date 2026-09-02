// Phase B — valid-path differential tests for `modeselect`, the composed
// top-level entry point (rows C35..C47 of CONFIGS.md).
//
// `modeselect` both returns an int and writes 8 `printf` lines to stdout. Both
// are compared: the return value always, and the exact stdout bytes for the
// rows that specify it.
mod common;

use common::*;

/// Call `modeselect` on both `.so`s, comparing return value AND stdout bytes.
///
/// Uses the fork-isolated capture so libtest's own parallel progress output on
/// fd 1 cannot contaminate the comparison.
fn ms_full(row: &str, a: i32, b: i32, c: i32, d: i32) {
    let p = pair();
    // SAFETY: plain scalar C ABI calls; the C is only UB for negative
    // `mode_selector` that is not a multiple of 4 (ERRORS.md E29), which callers
    // of this helper never pass.
    let (rc, oc) = capture_forked_i32(|| unsafe { (p.c.modeselect)(a, b, c, d) });
    let (rr, or) = capture_forked_i32(|| unsafe { (p.rs.modeselect)(a, b, c, d) });
    let ctx = format!("modeselect({a}, {b}, {c}, {d})");
    assert!(!oc.is_empty(), "[{row}] captured no C output for {ctx}");
    eq_bytes(row, &ctx, &oc, &or);
    eq_int(row, &ctx, rc, rr);
}

/// Call `modeselect` on both `.so`s comparing only the return value. Still
/// forked, so the test log stays clean and no fd-1 races are possible.
fn ms_ret(row: &str, a: i32, b: i32, c: i32, d: i32) {
    let p = pair();
    // SAFETY: as `ms_full`.
    let (rc, _) = capture_forked_i32(|| unsafe { (p.c.modeselect)(a, b, c, d) });
    let (rr, _) = capture_forked_i32(|| unsafe { (p.rs.modeselect)(a, b, c, d) });
    eq_int(row, format!("modeselect({a}, {b}, {c}, {d})"), rc, rr);
}

#[test]
fn c35_c38_each_mode_index() {
    ms_full("C35", 0, 0, 0, 0);
    ms_full("C36", 1, 0, 0, 0);
    ms_full("C37", 2, 0, 0, 0);
    ms_full("C38", 3, 0, 0, 0);
    // Same indices reached via larger selectors.
    for k in 0..8i32 {
        ms_full("C35..C38", 4 * k + (k % 4), 0, 0, 0);
    }
}

#[test]
fn c39_mode_index_by_complexity_level_cross_product() {
    // 4 mode indices x 5 complexity levels, return value AND stdout.
    for mi in 0..4i32 {
        for cl in 0..5i32 {
            ms_full("C39", mi, 0, cl, 0);
            // ... and reached via non-minimal selectors that reduce identically.
            ms_full("C39", 400 + mi, 0, 500 + cl, 0);
        }
    }
}

#[test]
fn c40_negative_complexity_hits_default_arm() {
    let mut rng = Rng::with_seed(SEED ^ 0xC40);
    for mi in 0..4i32 {
        for cl in [-1i32, -2, -3, -4, -5, -6, -10, i32::MIN, i32::MIN + 1] {
            ms_full("C40", mi, 0, cl, 0);
        }
        for _ in 0..120 {
            ms_ret("C40", mi, 0, rng.range_i32(i32::MIN, -1), 0);
        }
    }
}

#[test]
fn c41_negative_seed_gives_negative_offset_hours() {
    let mut rng = Rng::with_seed(SEED ^ 0xC41);
    for mi in 0..4i32 {
        for s in [-1i32, -23, -24, -25, -47, -48, i32::MIN, i32::MIN + 1] {
            ms_full("C41", mi, 0, 0, s);
        }
        for _ in 0..120 {
            ms_ret("C41", mi, 0, 0, rng.range_i32(i32::MIN, -1));
        }
    }
}

#[test]
fn c42_negative_time_offset() {
    let mut rng = Rng::with_seed(SEED ^ 0xC42);
    for mi in 0..4i32 {
        for t in [-1i32, -365, -1000, i32::MIN, i32::MIN + 1] {
            ms_full("C42", mi, t, 0, 0);
        }
        for _ in 0..120 {
            ms_ret("C42", mi, rng.range_i32(i32::MIN, -1), 0, 0);
        }
    }
}

#[test]
fn c43_time_offset_overflows_seconds_computation() {
    // days * 86400 crosses INT_MAX around +-24855.
    for mi in 0..4i32 {
        for t in [
            24854i32, 24855, 24856, 25000, 100000, 1_000_000, i32::MAX, i32::MAX - 1, -24854,
            -24855, -24856, -100000, i32::MIN, i32::MIN + 1,
        ] {
            ms_full("C43", mi, t, 0, 0);
        }
    }
}

#[test]
fn c44_seed_overflows_double_to_int_cast() {
    for mi in 0..4i32 {
        for s in [
            0i32, 1, 21, 22, 23, 24, -1, -23, 1_000_000, i32::MAX, i32::MAX - 1, i32::MIN,
            i32::MIN + 1,
        ] {
            ms_full("C44", mi, 0, 0, s);
        }
    }
}

#[test]
fn c45_randomized_return_value() {
    let mut rng = Rng::with_seed(SEED ^ 0xC45);
    for _ in 0..2500 {
        // mode_selector >= 0 only: negatives are the verified-SIGSEGV UB of
        // ERRORS.md E29, exercised separately in the error-path suite.
        let sel = (rng.next_u32() >> 1) as i32;
        ms_ret("C45", sel, rng.next_i32(), rng.next_i32(), rng.next_i32());
    }
    // Negative multiples of 4 are NOT UB (index reduces to 0), so include them.
    for _ in 0..500 {
        let sel = rng.range_i32(-536_870_912, 0) * 4;
        ms_ret("C45", sel, rng.next_i32(), rng.next_i32(), rng.next_i32());
    }
    ms_ret("C45", i32::MIN, 0, 0, 0);
}

#[test]
fn c46_randomized_stdout_bytes() {
    let mut rng = Rng::with_seed(SEED ^ 0xC46);
    for _ in 0..1200 {
        let sel = (rng.next_u32() >> 1) as i32;
        ms_full("C46", sel, rng.next_i32(), rng.next_i32(), rng.next_i32());
    }
    for _ in 0..300 {
        let sel = rng.range_i32(-536_870_912, 0) * 4;
        ms_full("C46", sel, rng.next_i32(), rng.next_i32(), rng.next_i32());
    }
}

#[test]
fn c47_boundary_cross_product() {
    let sels = [0i32, 1, 2, 3, 4, i32::MIN, i32::MAX - 3, i32::MAX];
    let comps = [i32::MIN, -1, 0, 4, 5, i32::MAX];
    let seeds = [i32::MIN, -1, 0, 23, 24, i32::MAX];
    let toffs = [i32::MIN, -1, 0, 1, i32::MAX];
    for &s in &sels {
        for &c in &comps {
            for &d in &seeds {
                for &t in &toffs {
                    ms_ret("C47", s, t, c, d);
                }
            }
        }
    }
    // A stdout-byte pass over a representative slice of the same grid.
    for &s in &sels {
        for &c in &[i32::MIN, -1, 0, 4, i32::MAX] {
            ms_full("C47", s, i32::MAX, c, i32::MIN);
        }
    }
}

// ---------------------------------------------------------------------------
// Reachable-value invariants
// ---------------------------------------------------------------------------
//
// Mutation testing flagged four mutants the suite accepts:
//   * `result1 & 0xFF`    -> `& 0xFFF`
//   * `result2 & 0xFF00`  -> `& 0xF000`
//   * `Result 1: (0x%X)`  -> `(0x%x)`
//   * `mode_selector % 4` -> `.rem_euclid(4)`
//
// The first three are EQUIVALENT mutants, not coverage gaps, and this test is
// the evidence: inside `modeselect` the two `(int)double` results can only ever
// be `0` or `INT_MIN`, because `factor1 = seed * 1e8` is scaled by a further
// `1e12` (so `|seed| >= 1` always overflows) and `factor2 = time_offset * -1e7`
// by a further `-1e15`. `INT_MIN` is `0x80000000`, whose low 16 bits are zero,
// so both `& 0xFF` and `& 0xFF00` are always 0 and both mask widths agree; and
// neither `0` nor `80000000` contains a hex letter, so `%X` and `%x` render
// identically on that line.
//
// The fourth is unobservable for a different, documented reason: `% 4` and
// `rem_euclid(4)` differ only for negative `mode_selector`, and every negative
// non-multiple of 4 makes the C SIGSEGV (ERRORS.md E29), so no comparison is
// possible there. Negative multiples of 4 give 0 under both.
#[test]
fn invariant_modeselect_cast_results_are_only_zero_or_int_min() {
    let p = pair();
    let mut rng = Rng::with_seed(SEED ^ 0x1234);
    let mut seen1 = std::collections::BTreeSet::new();
    let mut seen2 = std::collections::BTreeSet::new();

    let mut cases: Vec<(i32, i32)> = vec![
        (0, 0),
        (1, 0),
        (0, 1),
        (-1, 0),
        (0, -1),
        (i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN),
    ];
    for _ in 0..3000 {
        cases.push((rng.next_i32(), rng.next_i32()));
    }

    for (seed, time_offset) in cases {
        // Exactly how modeselect builds them (lib.c:120-121).
        let f1 = (seed as f64) * 1e8;
        let f2 = (time_offset as f64) * -1e7;
        // SAFETY: plain scalar C ABI calls.
        let (r1, r2) = unsafe {
            (
                (p.c.convert_time_factor)(f1),
                (p.c.convert_negative_overflow)(f2),
            )
        };
        seen1.insert(r1);
        seen2.insert(r2);
    }

    let expect: std::collections::BTreeSet<i32> = [0, i32::MIN].into_iter().collect();
    assert_eq!(
        seen1, expect,
        "convert_time_factor reachable from modeselect yielded more than {{0, INT_MIN}}: \
         the `result1 & 0xFF` mask width and the `%X` case are then observable and need \
         dedicated coverage"
    );
    assert_eq!(
        seen2, expect,
        "convert_negative_overflow reachable from modeselect yielded more than {{0, INT_MIN}}"
    );

    // Therefore both XOR terms are always zero...
    for &v in &seen1 {
        assert_eq!(v & 0xFF, 0);
        assert_eq!(v & 0xFFF, 0);
    }
    for &v in &seen2 {
        assert_eq!(v & 0xFF00, 0);
        assert_eq!(v & 0xF000, 0);
    }
    // ...and neither value renders any hex letter.
    for &v in seen1.iter().chain(seen2.iter()) {
        let hex = format!("{:X}", v as u32);
        assert!(
            !hex.chars().any(|c| c.is_ascii_alphabetic()),
            "0x{hex} contains a hex letter, so %X vs %x IS observable"
        );
    }
}

#[test]
fn invariant_hex_letters_do_appear_on_the_other_printf_lines() {
    // The `%X` conversions that CAN print letters -- the multiplier and the final
    // result -- are exercised, so the `%X`/`%x` distinction is covered where it
    // is observable at all. Proven by finding real inputs whose C output contains
    // hex letters on those lines.
    let p = pair();
    // SAFETY: mode_selector 0 is in range.
    let (_, out) = capture_forked_i32(|| unsafe { (p.c.modeselect)(0, 0, -1, 0) });
    let s = String::from_utf8_lossy(&out).to_string();
    let mult = s
        .lines()
        .find(|l| l.starts_with("Complexity level: "))
        .expect("multiplier line");
    assert!(
        mult.contains("0xDEAD"),
        "expected hex letters on the multiplier line; got {mult:?}"
    );
    let final_line = s
        .lines()
        .find(|l| l.starts_with("Final result: "))
        .expect("final line");
    let hex = final_line.rsplit("0x").next().unwrap().trim_end_matches(')');
    assert!(
        hex.chars().any(|c| c.is_ascii_alphabetic()),
        "expected hex letters on the final line; got {final_line:?}"
    );
}
