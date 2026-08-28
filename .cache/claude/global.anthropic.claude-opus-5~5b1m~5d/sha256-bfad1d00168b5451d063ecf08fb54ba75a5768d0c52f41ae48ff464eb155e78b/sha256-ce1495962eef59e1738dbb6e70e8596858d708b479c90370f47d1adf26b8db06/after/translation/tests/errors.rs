//! Phase C — error/boundary-path differential tests, one per `ERRORS.md` row.
//!
//! The C library has ZERO explicit rejection paths (see `ERRORS.md` for the
//! mechanical grep evidence): `max_size_frame` is total over all 2^96 argument
//! bit patterns, takes no pointer, no length and no enum, and can never return
//! an error sentinel. So each row asserts that BOTH implementations accept the
//! boundary/invalid-looking input identically and return the SAME exact value
//! (rather than merely "both failed somehow"), including the value derived from
//! the C source semantics.

mod common;

use common::{impls, model, Rng, SEED};

const MAX: u32 = u32::MAX;

// E1: channels = 0 annihilates t1 -> constant 18, never rejected.
#[test]
fn e1_channels_zero() {
    let f = impls();
    f.assert_eq_expect(4096, 0, 32, 18);
    let mut r = Rng::new(SEED ^ 1);
    for _ in 0..50_000 {
        let got = f.assert_eq(r.next_u32(), 0, r.next_u32());
        assert_eq!(got, 18);
    }
}

// E2: blocksize = 0 (empty block) is accepted -> 18 + channels.
#[test]
fn e2_blocksize_zero() {
    let f = impls();
    f.assert_eq_expect(0, 2, 16, 20);
    let mut r = Rng::new(SEED ^ 2);
    for _ in 0..50_000 {
        let ch = r.next_u32();
        let got = f.assert_eq(0, ch, r.next_u32());
        assert_eq!(got, 18u32.wrapping_add(ch));
    }
}

// E3: bitdepth = 0 (invalid FLAC depth) with channels != 2 -> accepted.
#[test]
fn e3_bitdepth_zero_non_stereo() {
    let f = impls();
    f.assert_eq_expect(4096, 1, 0, 19);
    let mut r = Rng::new(SEED ^ 3);
    for _ in 0..50_000 {
        let mut ch = r.next_u32();
        if ch == 2 {
            ch = 3;
        }
        let got = f.assert_eq(r.next_u32(), ch, 0);
        assert_eq!(got, 18u32.wrapping_add(ch));
    }
}

// E4: bitdepth = 0 with channels == 2 -> t3 = bs*(0+1) survives.
#[test]
fn e4_bitdepth_zero_stereo() {
    let f = impls();
    f.assert_eq_expect(4096, 2, 0, 532);
    let mut r = Rng::new(SEED ^ 4);
    for _ in 0..50_000 {
        let bs = r.next_u32();
        f.assert_eq_expect(bs, 2, 0, model(bs, 2, 0));
    }
}

// E5/E6/E7: the `bitdepth != 32` boundary and one step past the documented max.
#[test]
fn e5_bitdepth_exactly_32() {
    let f = impls();
    f.assert_eq_expect(4096, 2, 32, 32788);
}

#[test]
fn e6_bitdepth_31() {
    let f = impls();
    f.assert_eq_expect(4096, 2, 31, 32276);
}

#[test]
fn e7_bitdepth_33_one_past_max() {
    let f = impls();
    f.assert_eq_expect(4096, 2, 33, 34324);
    // sweep the whole neighbourhood of the boundary on both branches
    let mut r = Rng::new(SEED ^ 7);
    for bd in 28..=36u32 {
        for ch in [0u32, 1, 2, 3] {
            f.assert_eq_expect(4096, ch, bd, model(4096, ch, bd));
            let bs = r.next_u32();
            f.assert_eq_expect(bs, ch, bd, model(bs, ch, bd));
        }
    }
}

// E8/E9/E10: the `channels == 2` boundary and one step either side.
#[test]
fn e8_channels_exactly_2() {
    let f = impls();
    f.assert_eq_expect(4096, 2, 16, 16916);
}

#[test]
fn e9_channels_1() {
    let f = impls();
    f.assert_eq_expect(4096, 1, 16, 8211);
}

#[test]
fn e10_channels_3() {
    let f = impls();
    f.assert_eq_expect(4096, 3, 16, 24597);
}

// E11: blocksize = MAX -> bs*bd wraps mod 2^32, no trap.
#[test]
fn e11_blocksize_max_wraps() {
    let f = impls();
    f.assert_eq_expect(MAX, 1, 1, 19);
    let mut r = Rng::new(SEED ^ 11);
    for _ in 0..50_000 {
        let ch = r.next_u32();
        let bd = r.next_u32();
        f.assert_eq_expect(MAX, ch, bd, model(MAX, ch, bd));
    }
}

// E12: bitdepth = MAX with channels == 2 -> `bitdepth + 1` wraps to 0, t3 dies.
#[test]
fn e12_bitdepth_max_wraps_to_zero_stereo() {
    let f = impls();
    f.assert_eq_expect(4096, 2, MAX, 536870420);
    let mut r = Rng::new(SEED ^ 12);
    for _ in 0..50_000 {
        let bs = r.next_u32();
        f.assert_eq_expect(bs, 2, MAX, model(bs, 2, MAX));
    }
}

// E13: channels = MAX -> `18 + channels` itself overflows.
#[test]
fn e13_channels_max_overflow() {
    let f = impls();
    f.assert_eq_expect(1, MAX, 1, 17);
}

// E14: all three MAX -> maximal simultaneous overflow.
#[test]
fn e14_all_max() {
    let f = impls();
    f.assert_eq_expect(MAX, MAX, MAX, 17);
}

// E15: channels = MAX-17 makes `18 + channels` land exactly on 0.
#[test]
fn e15_channels_max_minus_17_lands_on_zero() {
    let f = impls();
    f.assert_eq_expect(0, MAX - 17, 0, 0);
    // full sweep across the wrap point of `18 + channels`
    for ch in (MAX - 25)..=MAX {
        f.assert_eq_expect(0, ch, 0, model(0, ch, 0));
    }
}

// E16: the `+ 7` addition wraps past 0.
#[test]
fn e16_sum_plus_seven_wraps() {
    let f = impls();
    f.assert_eq_expect(MAX, 1, MAX, 20);
    // search deliberately for arguments whose pre-+7 sum sits in MAX-6..=MAX
    let mut r = Rng::new(SEED ^ 16);
    let mut hits = 0usize;
    for _ in 0..500_000 {
        let bs = r.next_u32();
        let ch = r.next_u32();
        let bd = r.next_u32();
        let b = |c: bool| if c { 1u32 } else { 0u32 };
        let t1 = bs.wrapping_mul(bd).wrapping_mul(ch.wrapping_mul(b(ch != 2)));
        let t2 = bs.wrapping_mul(bd).wrapping_mul(b(ch == 2));
        let t3 = bs
            .wrapping_mul(bd.wrapping_add(b(bd != 32)))
            .wrapping_mul(b(ch == 2));
        let pre = t1.wrapping_add(t2).wrapping_add(t3);
        if pre >= MAX - 6 {
            hits += 1;
            f.assert_eq_expect(bs, ch, bd, model(bs, ch, bd));
        }
    }
    // Also construct the wrap directly: mono, bd=1, bs = MAX-k gives pre = MAX-k.
    for k in 0..=6u32 {
        f.assert_eq_expect(MAX - k, 1, 1, model(MAX - k, 1, 1));
        hits += 1;
    }
    assert!(hits > 0, "expected to exercise the +7 wraparound");
}

// E17: blocksize = 65536, one step past FLAC's 16-bit max blocksize.
#[test]
fn e17_blocksize_65536_one_past_max() {
    let f = impls();
    f.assert_eq_expect(65536, 8, 32, 2097178);
    for bs in [65534u32, 65535, 65536, 65537] {
        for ch in 0..=9u32 {
            for bd in [0u32, 1, 16, 31, 32, 33] {
                f.assert_eq_expect(bs, ch, bd, model(bs, ch, bd));
            }
        }
    }
}

// E18: division truncation floor -- sum = 7 -> 7/8 = 0.
#[test]
fn e18_division_floor() {
    let f = impls();
    f.assert_eq_expect(0, 1, 1, 19);
}

// E19: division carry -- sum = 8 -> quotient steps to 1.
#[test]
fn e19_division_carry() {
    let f = impls();
    f.assert_eq_expect(1, 1, 1, 20);
}

// E20: bitdepth = MAX with channels != 2 -- the bd+1 wrap must be UNUSED.
#[test]
fn e20_bitdepth_max_non_stereo() {
    let f = impls();
    f.assert_eq_expect(1, 1, MAX, 19);
    let mut r = Rng::new(SEED ^ 20);
    for _ in 0..50_000 {
        let mut ch = r.next_u32();
        if ch == 2 {
            ch = 3;
        }
        let bs = r.next_u32();
        f.assert_eq_expect(bs, ch, MAX, model(bs, ch, MAX));
    }
}

// E21: all 8 residues of `sum mod 8` -- truncating, not rounding, division.
#[test]
fn e21_all_division_residues() {
    let f = impls();
    let expected = [19u32, 20, 20, 20, 20, 20, 20, 20, 20, 21];
    for (bs, &want) in expected.iter().enumerate() {
        f.assert_eq_expect(bs as u32, 1, 1, want);
    }
    // every residue class, on both branches, at several magnitudes
    for base in [0u32, 8, 64, 4096, 1 << 20, MAX - 64] {
        for d in 0..16u32 {
            let bs = base.wrapping_add(d);
            f.assert_eq_expect(bs, 1, 1, model(bs, 1, 1));
            f.assert_eq_expect(bs, 2, 1, model(bs, 2, 1));
            f.assert_eq_expect(bs, 3, 1, model(bs, 3, 1));
        }
    }
}

// E22: division by zero is impossible (divisor is the literal 8); no trap path.
#[test]
fn e22_no_division_by_zero_trap() {
    let f = impls();
    // If either implementation could trap/abort, the process would die here.
    // The divisor is a constant 8, and `bitdepth`/`channels` never become one.
    let mut r = Rng::new(SEED ^ 22);
    for _ in 0..200_000 {
        f.assert_eq(r.next_u32(), r.next_u32(), r.next_u32());
    }
    // The specific inputs that would be dangerous if the divisor were derived
    // from an argument:
    for &(bs, ch, bd) in &[
        (0u32, 0u32, 0u32),
        (0, 0, 8),
        (8, 0, 0),
        (0, 8, 0),
        (MAX, 0, 0),
        (0, MAX, 0),
        (0, 0, MAX),
    ] {
        f.assert_eq_expect(bs, ch, bd, model(bs, ch, bd));
    }
}

// E23: the N/A-by-construction rejection classes.
//
// The API takes no pointer, no length and no enum, so there is no null-pointer,
// oversized-length or invalid-enum-variant surface: EVERY bit pattern of all
// three u32 arguments is valid input that the C accepts. This test drives the
// values that WOULD be the "one step past a documented valid range" and
// "out-of-range enum variant" cases if the parameters were bounded, and asserts
// both implementations agree exactly (never an error sentinel).
#[test]
fn e23_out_of_range_values_are_all_accepted() {
    let f = impls();

    // "one step past" every documented FLAC range, plus the classic
    // out-of-range-enum stand-ins (negative-as-u32, huge, and 0).
    let past_range: &[u32] = &[
        0,
        9,          // channels: FLAC max is 8
        33,         // bitdepth: FLAC max is 32
        65536,      // blocksize: FLAC max is 65535
        (-1i32) as u32,
        (-2i32) as u32,
        (i32::MIN) as u32,
        0x8000_0000,
        MAX,
    ];
    for &bs in past_range {
        for &ch in past_range {
            for &bd in past_range {
                // must return normally, and identically
                f.assert_eq_expect(bs, ch, bd, model(bs, ch, bd));
            }
        }
    }

    // Exhaustive over the full small-value neighbourhood of every documented
    // range boundary in all three positions simultaneously.
    for bs in [0u32, 1, 65535, 65536] {
        for ch in 0..=10u32 {
            for bd in 0..=34u32 {
                f.assert_eq_expect(bs, ch, bd, model(bs, ch, bd));
            }
        }
    }
}
