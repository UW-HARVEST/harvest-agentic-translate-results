//! Phase B — valid-path differential tests for the composed `modeselect`
//! pipeline. Covers CONFIGS.md rows 33..=46.
//!
//! Every call compares BOTH the returned `int` AND the exact bytes `printf`
//! wrote to stdout (8 lines per call, including `%.2e` and `%X`-of-negative
//! formatting), because most of this function's behaviour is observable only
//! through its output.
//!
//! `mode_selector` is restricted to values whose `% 4` is in `0..4`, i.e.
//! non-negative selectors plus negative multiples of 4. For
//! `mode_selector % 4 ∈ {-1,-2,-3}` the C indexes its 4-element local array out
//! of bounds and segfaults; see ERRORS.md row 32 and
//! `phase_c_errors::row_32_modeselect_negative_index_is_ub`.

mod common;
use common::*;

// `diff_modeselect` lives in `common`: it runs each `.so`'s `modeselect` in its
// own forked child so the captured stdout cannot be polluted by libtest's
// concurrent progress output, then compares return value and printed bytes.

// ===========================================================================
// rows 33..=37 — the mode x complexity cross product
// ===========================================================================

#[test]
fn row_33_36_modeselect_matched_mode_and_complexity() {
    let mut rng = Rng::fixed();
    for k in 0..4i32 {
        for _ in 0..60 {
            diff_modeselect(
                &format!("mode%4={k} complexity%5={k}"),
                k,
                rng.range_i32(-5000, 5000),
                k,
                rng.range_i32(-5000, 5000),
            );
        }
    }
}

#[test]
fn row_37_modeselect_full_mode_complexity_cross_product() {
    let mut rng = Rng::fixed();
    for m in 0..4i32 {
        for c in 0..5i32 {
            for rep in 0..25 {
                // vary the multiple of 4/5 too, so it is not always the
                // smallest representative of the residue class
                let ms = m + 4 * rng.range_i32(0, 1_000_000);
                let cx = c + 5 * rng.range_i32(0, 1_000_000);
                diff_modeselect(
                    &format!("mode%4={m} complexity%5={c} rep={rep}"),
                    ms,
                    rng.next_i32(),
                    cx,
                    rng.next_i32(),
                );
            }
        }
    }
}

// ===========================================================================
// row 38 — negative complexity reaches apply_multiplier's `default:` (0xDEAD)
// ===========================================================================

#[test]
fn row_38_modeselect_negative_complexity_hits_default_branch() {
    let mut rng = Rng::fixed();
    for r in 1..=4i32 {
        // complexity % 5 == -r  =>  apply_multiplier(0xA0, -r) -> 0xDEAD
        for _ in 0..40 {
            let cx = -r - 5 * rng.range_i32(0, 1_000_000);
            assert_eq!(cx % 5, -r);
            diff_modeselect(
                &format!("complexity%5=-{r}"),
                rng.range_i32(0, 1_000_000),
                rng.next_i32(),
                cx,
                rng.next_i32(),
            );
        }
    }
    for cx in [-1, -2, -3, -4, -5, -6, -10, -100, i32::MIN, i32::MIN + 1] {
        diff_modeselect("negative complexity corner", 0, 0, cx, 0);
    }
}

// ===========================================================================
// rows 39..=42 — zero / sign / overflow shapes of the double and time paths
// ===========================================================================

#[test]
fn row_39_40_modeselect_zero_seed_and_zero_time_offset() {
    // seed = 0  -> factor1 = 0.0  -> "0.00e+00", Result 1: 0
    // time_offset = 0 -> factor2 = -0.0 -> "-0.00e+00" (negative zero!)
    for m in 0..4i32 {
        for c in 0..5i32 {
            diff_modeselect("seed=0 time_offset=0", m, 0, c, 0);
            diff_modeselect("seed=0", m, 7, c, 0);
            diff_modeselect("time_offset=0", m, 0, c, 7);
        }
    }
}

#[test]
fn row_41_modeselect_seed_sign_and_mod24() {
    let mut rng = Rng::fixed();
    // seed % 24 across all residues, both signs, plus overflow of factor1
    for r in -23..=23i32 {
        let seed = if r >= 0 {
            r + 24 * rng.range_i32(0, 100_000)
        } else {
            r - 24 * rng.range_i32(0, 100_000)
        };
        assert_eq!(seed % 24, r);
        diff_modeselect(
            &format!("seed%24={r}"),
            rng.range_i32(0, 1_000_000),
            rng.range_i32(-100_000, 100_000),
            rng.range_i32(0, 1_000_000),
            seed,
        );
    }
    for seed in [i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1, 1, -1] {
        diff_modeselect("seed corner", 0, 0, 0, seed);
    }
}

#[test]
fn row_42_modeselect_time_offset_overflow_inside_pipeline() {
    let mut rng = Rng::fixed();
    for _ in 0..300 {
        // |time_offset| > 24855 overflows time_offset*86400 in get_modified_time
        let to = rng.range_i32(24856, i32::MAX);
        let to = if rng.next_u64() & 1 == 0 { to } else { to.wrapping_neg() };
        diff_modeselect(
            "time_offset overflow",
            rng.range_i32(0, 1_000_000),
            to,
            rng.next_i32(),
            rng.next_i32(),
        );
    }
    for to in [24855, 24856, -24855, -24856, i32::MAX, i32::MIN] {
        diff_modeselect("time_offset corner", 0, to, 0, 0);
    }
}

// ===========================================================================
// rows 43..=44 — corner arguments, incl. in-bounds negative selectors
// ===========================================================================

#[test]
fn row_43_modeselect_corner_argument_grid() {
    // mode_selector kept in bounds (see module docs); the other three are free.
    let selectors = [0i32, 1, 2, 3, 4, 5, 23, 24, i32::MAX, i32::MIN, -4, -8, -1024];
    let others = [0i32, 1, -1, 4, 5, 23, 24, i32::MAX, i32::MIN];
    for &ms in &selectors {
        for &v in &others {
            diff_modeselect("corner: vary time_offset", ms, v, 0, 0);
            diff_modeselect("corner: vary complexity", ms, 0, v, 0);
            diff_modeselect("corner: vary seed", ms, 0, 0, v);
            diff_modeselect("corner: all equal", ms, v, v, v);
        }
    }
}

#[test]
fn row_44_modeselect_in_bounds_negative_selectors() {
    // INT_MIN % 4 == 0 and every negative multiple of 4 is index 0 ("standard").
    let mut rng = Rng::fixed();
    for ms in [i32::MIN, -4, -8, -12, -400, -4_000_000, -2_147_483_644] {
        assert_eq!(ms % 4, 0);
        for _ in 0..8 {
            diff_modeselect(
                "in-bounds negative selector",
                ms,
                rng.next_i32(),
                rng.next_i32(),
                rng.next_i32(),
            );
        }
    }
}

// ===========================================================================
// rows 45..=46 — fully randomized, return value AND stdout
// ===========================================================================

#[test]
fn row_45_46_modeselect_randomized_return_and_stdout() {
    let mut rng = Rng::fixed();
    for _ in 0..2000 {
        // non-negative selector => always an in-bounds index
        let ms = (rng.next_u64() >> 33) as i32;
        diff_modeselect(
            "fully randomized",
            ms,
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
        );
    }
}

/// Explicitly pins the expected stdout *shape*, so a regression that made both
/// implementations print nothing (and therefore "match") could not pass
/// unnoticed.
#[test]
fn stdout_capture_is_actually_capturing() {
    let (a, b) = modeselect_both(1, 2, 3, 4);
    assert_eq!(
        show(&a.stdout),
        show(&b.stdout),
        "stdout differs for modeselect(1,2,3,4)"
    );
    assert_eq!(a.ret, b.ret, "return differs for modeselect(1,2,3,4)");
    let s = String::from_utf8(a.stdout).expect("stdout is utf-8");
    for needle in [
        "Selected mode: enhanced (0x20)\n",
        "Complexity level: 3, Multiplier: 0x",
        "Modified time: ",
        ", Hash: 0x",
        "Converting double 4.00e+08 to int (may overflow)...\n",
        "Result 1: ",
        "Converting double -2.00e+07 to int (may underflow)...\n",
        "Result 2: ",
        "\nFinal result: ",
    ] {
        assert!(s.contains(needle), "captured stdout missing {needle:?}:\n{s}");
    }
    assert_eq!(s.lines().count(), 9, "expected 9 lines (one blank):\n{s}");
}

/// Documents why `result ^= (result1 & 0xFF)` and `result ^= (result2 & 0xFF00)`
/// in the C are **dead code for every possible input**, and pins the fact.
///
/// `result1 = convert_time_factor((double)seed * 1e8)`:
///   * `seed == 0` -> the product is `0.0` -> `0`
///   * `seed != 0` -> `|(double)seed * 1e8| >= 1e8`, so `|x * 1e12| >= 1e20`,
///     far outside `int` range -> the `cvttsd2si` indefinite value `INT_MIN`
///
/// so `result1` is always `0x00000000` or `0x80000000`, and therefore
/// `result1 & 0xFF == 0`. The same argument gives `result2 & 0xFF00 == 0`.
///
/// Consequence: mutating either mask (or deleting either XOR) cannot change the
/// observable output. Both were confirmed to survive mutation testing precisely
/// because of this, not because the suite is blind to them.
#[test]
fn modeselect_result1_result2_xor_steps_are_provably_dead() {
    let p = pair();
    let ctf = p.c.convert_time_factor();
    let cno = p.c.convert_negative_overflow();
    let rtf = p.rs.convert_time_factor();
    let rno = p.rs.convert_negative_overflow();

    let mut rng = Rng::fixed();
    let mut check = |v: i32| {
        let f1 = v as f64 * 1e8;
        let f2 = v as f64 * -1e7;
        // SAFETY: plain scalar FFI calls.
        let (c1, r1, c2, r2) = unsafe { (ctf(f1), rtf(f1), cno(f2), rno(f2)) };
        assert_eq!(c1, r1, "convert_time_factor diverged for seed={v}");
        assert_eq!(c2, r2, "convert_negative_overflow diverged for time_offset={v}");
        let want = if v == 0 { 0 } else { i32::MIN };
        assert_eq!(c1, want, "result1 must be 0 or INT_MIN for seed={v}, got {c1}");
        assert_eq!(c2, want, "result2 must be 0 or INT_MIN for time_offset={v}, got {c2}");
        assert_eq!(c1 & 0xFF, 0, "result1 & 0xFF must always be 0 (seed={v})");
        assert_eq!(c2 & 0xFF00, 0, "result2 & 0xFF00 must always be 0 (time_offset={v})");
    };

    for v in [0i32, 1, -1, 2, -2, 41, -41, i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1] {
        check(v);
    }
    for _ in 0..20000 {
        check(rng.next_i32());
    }
}
