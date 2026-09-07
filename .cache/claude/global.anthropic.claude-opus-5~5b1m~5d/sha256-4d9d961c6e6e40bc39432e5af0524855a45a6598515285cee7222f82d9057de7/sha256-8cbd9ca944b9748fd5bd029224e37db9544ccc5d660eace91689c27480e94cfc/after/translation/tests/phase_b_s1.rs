//! Phase B — valid-path differential tests, surface S1 (the shipped `.so`s).
//! CONFIGS.md rows 1-10. Both sides are loaded via `libloading`.

mod common;

use common::{JumpnodeFn, Pair, Rng, SEED, assert_eq_call};

const DIGIT_BOUNDARIES: &[i32] = &[
    0,
    1,
    -1,
    9,
    -9,
    10,
    -10,
    99,
    -99,
    100,
    -100,
    999,
    -999,
    9_999,
    -9_999,
    99_999,
    -99_999,
    999_999,
    -999_999,
    9_999_999,
    -9_999_999,
    99_999_999,
    -99_999_999,
    999_999_999,
    -999_999_999,
    1_000_000_000,
    -1_000_000_000,
    i32::MAX,
    i32::MIN,
    i32::MAX - 1,
    i32::MIN + 1,
];

/// Row 1: mode 3, small non-negative node_id/depth, random flags.
#[test]
fn cfg_row1_mode3_small() {
    let p = Pair::shipped();
    let (c, rs) = p.sym::<JumpnodeFn>(b"jumpnode\0");
    let mut r = Rng::new(SEED);
    for _ in 0..4000 {
        let id = r.range(0, 500);
        let depth = r.range(0, 500);
        let flags = r.interesting_i32();
        assert_eq_call("row1", &c, &rs, 3, id, depth, flags);
    }
}

/// Row 2: mode 3, negative node_id/depth (the '-' lengthens the sprintf output).
#[test]
fn cfg_row2_mode3_negative() {
    let p = Pair::shipped();
    let (c, rs) = p.sym::<JumpnodeFn>(b"jumpnode\0");
    let mut r = Rng::new(SEED ^ 2);
    for _ in 0..4000 {
        let id = r.range(-500, -1);
        let depth = r.range(-500, -1);
        let flags = r.interesting_i32();
        assert_eq_call("row2", &c, &rs, 3, id, depth, flags);
    }
    // and mixed signs
    for _ in 0..4000 {
        let id = r.range(-1_000_000, 1_000_000);
        let depth = r.range(-1_000_000, 1_000_000);
        assert_eq_call("row2-mixed", &c, &rs, 3, id, depth, r.interesting_i32());
    }
}

/// Row 3: mode 3, full cross product of digit-count boundaries.
#[test]
fn cfg_row3_mode3_digit_boundaries() {
    let p = Pair::shipped();
    let (c, rs) = p.sym::<JumpnodeFn>(b"jumpnode\0");
    let mut r = Rng::new(SEED ^ 3);
    for &id in DIGIT_BOUNDARIES {
        for &depth in DIGIT_BOUNDARIES {
            // flags = 0 isolates the string-length contribution, then randomize.
            assert_eq_call("row3", &c, &rs, 3, id, depth, 0);
            assert_eq_call("row3", &c, &rs, 3, id, depth, r.interesting_i32());
        }
    }
}

/// Row 4: mode 3, every residue of the `& 0177` mask plus signed extremes.
#[test]
fn cfg_row4_mode3_flag_mask() {
    let p = Pair::shipped();
    let (c, rs) = p.sym::<JumpnodeFn>(b"jumpnode\0");
    let mut r = Rng::new(SEED ^ 4);

    // All 128 residues, reached from several different base values so that both
    // positive and negative `flags` with the same low 7 bits are covered.
    for bits in 0..128i32 {
        for base in [0i32, 128, 256, 1 << 20, -128, -256, i32::MIN, i32::MAX & !127] {
            let flags = base | bits;
            assert_eq_call("row4", &c, &rs, 3, 7, 42, flags);
        }
        // negative values whose two's-complement low bits equal `bits`
        assert_eq_call("row4-neg", &c, &rs, 3, 7, 42, -(bits + 1));
    }
    for _ in 0..2000 {
        assert_eq_call("row4-rand", &c, &rs, 3, 1, 1, r.next_i32());
    }
    for &f in &[0, -1, i32::MAX, i32::MIN, 127, 128, -127, -128] {
        assert_eq_call("row4-extreme", &c, &rs, 3, 0, 0, f);
    }
}

/// Row 5: mode 3, fully random ints in all four positions.
#[test]
fn cfg_row5_mode3_fuzz() {
    let p = Pair::shipped();
    let (c, rs) = p.sym::<JumpnodeFn>(b"jumpnode\0");
    let mut r = Rng::new(SEED ^ 5);
    for _ in 0..20000 {
        assert_eq_call(
            "row5",
            &c,
            &rs,
            3,
            r.interesting_i32(),
            r.interesting_i32(),
            r.interesting_i32(),
        );
    }
}

/// Row 6: mode 1 against empty storage.
#[test]
fn cfg_row6_mode1_empty() {
    let p = Pair::shipped();
    let (c, rs) = p.sym::<JumpnodeFn>(b"jumpnode\0");
    let mut r = Rng::new(SEED ^ 6);
    for _ in 0..5000 {
        assert_eq_call(
            "row6",
            &c,
            &rs,
            1,
            r.interesting_i32(),
            r.interesting_i32(),
            r.interesting_i32(),
        );
    }
}

/// Row 7: mode 2 against empty storage (returns before `process_backward`, so
/// even hostile `depth` values are well-defined here).
#[test]
fn cfg_row7_mode2_empty() {
    let p = Pair::shipped();
    let (c, rs) = p.sym::<JumpnodeFn>(b"jumpnode\0");
    let mut r = Rng::new(SEED ^ 7);
    for _ in 0..5000 {
        assert_eq_call(
            "row7",
            &c,
            &rs,
            2,
            r.interesting_i32(),
            r.interesting_i32(),
            r.interesting_i32(),
        );
    }
}

/// Row 8: mode 4 against empty storage.
#[test]
fn cfg_row8_mode4_empty() {
    let p = Pair::shipped();
    let (c, rs) = p.sym::<JumpnodeFn>(b"jumpnode\0");
    let mut r = Rng::new(SEED ^ 8);
    for _ in 0..5000 {
        assert_eq_call(
            "row8",
            &c,
            &rs,
            4,
            r.interesting_i32(),
            r.interesting_i32(),
            r.interesting_i32(),
        );
    }
}

/// Row 9: unknown operation modes take the `default:` arm.
#[test]
fn cfg_row9_default_mode() {
    let p = Pair::shipped();
    let (c, rs) = p.sym::<JumpnodeFn>(b"jumpnode\0");
    let mut r = Rng::new(SEED ^ 9);
    for mode in [-3i32, -2, -1, 0, 5, 6, 7, 8, 100, i32::MAX, i32::MIN] {
        for _ in 0..500 {
            assert_eq_call(
                "row9",
                &c,
                &rs,
                mode,
                r.interesting_i32(),
                r.interesting_i32(),
                r.interesting_i32(),
            );
        }
    }
}

/// Row 10: whole-S1-surface fuzz, mode included in the randomization.
#[test]
fn cfg_row10_s1_fuzz_all_modes() {
    let p = Pair::shipped();
    let (c, rs) = p.sym::<JumpnodeFn>(b"jumpnode\0");
    let mut r = Rng::new(SEED ^ 10);
    for _ in 0..40000 {
        let mode = match r.next_u64() % 4 {
            0 => r.range(-3, 8),
            1 => r.range(1, 4),
            2 => r.interesting_i32(),
            _ => r.next_i32(),
        };
        assert_eq_call(
            "row10",
            &c,
            &rs,
            mode,
            r.interesting_i32(),
            r.interesting_i32(),
            r.interesting_i32(),
        );
    }
}

/// Sanity: the shipped C `.so` really does have empty storage, so modes 1/2/4
/// are pure error paths. Documents the premise the S1 rows rest on.
#[test]
fn s1_storage_is_empty_premise() {
    let p = Pair::shipped();
    let (c, rs) = p.sym::<JumpnodeFn>(b"jumpnode\0");
    for id in -5..200 {
        assert_eq!(unsafe { c(1, id, 3, 0) }, 18);
        assert_eq!(unsafe { rs(1, id, 3, 0) }, 18);
        assert_eq!(unsafe { c(2, id, 3, 0) }, 34);
        assert_eq!(unsafe { rs(2, id, 3, 0) }, 34);
        assert_eq!(unsafe { c(4, id, 3, 0) }, 66);
        assert_eq!(unsafe { rs(4, id, 3, 0) }, 66);
    }
}
