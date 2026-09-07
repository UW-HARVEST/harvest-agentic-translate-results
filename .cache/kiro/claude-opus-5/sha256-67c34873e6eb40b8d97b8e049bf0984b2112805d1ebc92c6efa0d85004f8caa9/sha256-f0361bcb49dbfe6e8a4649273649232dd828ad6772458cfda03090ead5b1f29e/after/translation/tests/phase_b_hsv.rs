//! Phase B — rows C29..C47: `f11` (HSL->RGB), `f12` (HSV->RGB), `f13` (RGB->HSV).

mod harness;

use harness::*;

fn chk(p: &Pair, which: Triple, src: [f32; 3]) {
    let c = p.c.call_triple(which, src);
    let r = p.r.call_triple(which, src);
    let cb = [c[0].to_bits(), c[1].to_bits(), c[2].to_bits()];
    let rb = [r[0].to_bits(), r[1].to_bits(), r[2].to_bits()];
    assert_eq!(
        cb,
        rb,
        "{} diverged for src=[{:#010x},{:#010x},{:#010x}] ({:?})\n  C   =[{:#010x},{:#010x},{:#010x}]\n  Rust=[{:#010x},{:#010x},{:#010x}]",
        which.name(),
        src[0].to_bits(),
        src[1].to_bits(),
        src[2].to_bits(),
        src,
        cb[0],
        cb[1],
        cb[2],
        rb[0],
        rb[1],
        rb[2]
    );
}

/// Hue values that land in, and exactly on the boundary of, every sector the
/// C distinguishes (`f11`: 0/60/120/180/240/300/360; `f12`: the same via
/// `(int)floorf(h/60)`), plus one ULP either side of each boundary.
fn boundary_hues() -> Vec<f32> {
    let mut v = Vec::new();
    for b in [0.0f32, 60.0, 120.0, 180.0, 240.0, 300.0, 360.0, 420.0] {
        v.push(b);
        v.push(f32::from_bits(b.to_bits().wrapping_sub(1)));
        v.push(f32::from_bits(b.to_bits().wrapping_add(1)));
        v.push(-b);
        v.push(f32::from_bits((-b).to_bits().wrapping_add(1)));
    }
    v.extend_from_slice(&[
        1e-45,
        -1e-45,
        1e30,
        -1e30,
        f32::MAX,
        f32::MIN,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ]);
    for &nb in NAN_BITS {
        v.push(f32::from_bits(nb));
    }
    v
}

const SL_VALUES: &[f32] = &[
    0.0,
    -0.0,
    1e-45,
    -1e-45,
    0.25,
    0.5,
    0.75,
    1.0,
    -1.0,
    2.0,
    -2.0,
    1.175_494_4e-38,
    3.402_823_5e38,
    f32::INFINITY,
    f32::NEG_INFINITY,
];

// ------------------------------------------------------------------ C29..C37
#[test]
fn c29_c37_f11_all_sectors_and_early_return() {
    let p = load();
    let mut rng = Rng::new(SEED ^ 29);

    // C29: s == 0 (and -0.0), h and l arbitrary -> all three outputs = l.
    for &s in &[0.0f32, -0.0f32] {
        for h in boundary_hues() {
            for &l in SL_VALUES {
                chk(&p, Triple::F11, [h, s, l]);
            }
            for &nb in NAN_BITS {
                chk(&p, Triple::F11, [h, s, f32::from_bits(nb)]);
            }
        }
        for _ in 0..20_000 {
            chk(&p, Triple::F11, [rng.any_f32(), s, rng.any_f32()]);
        }
    }

    // C30..C35: one row per hue sector, randomized inside the sector.
    let sectors: &[(f32, f32)] = &[
        (0.0, 60.0),
        (60.0, 120.0),
        (120.0, 180.0), // C32 -- actually falls to the final `else` in the C
        (180.0, 240.0),
        (240.0, 300.0),
        (300.0, 360.0),
    ];
    for &(lo, hi) in sectors {
        for _ in 0..30_000 {
            let h = rng.range_f32(lo, hi);
            let s = rng.range_f32(-2.0, 2.0);
            let l = rng.range_f32(-2.0, 2.0);
            chk(&p, Triple::F11, [h, s, l]);
        }
        // C37: sector crossed with the l/s extremes.
        for &l in SL_VALUES {
            for &s in SL_VALUES {
                chk(&p, Triple::F11, [lo, s, l]);
                chk(&p, Triple::F11, [(lo + hi) * 0.5, s, l]);
                chk(&p, Triple::F11, [hi, s, l]);
            }
        }
    }

    // C36: h out of [0, 360) / NaN / infinite -> the final `else`.
    for h in boundary_hues() {
        for &s in SL_VALUES {
            for &l in SL_VALUES {
                chk(&p, Triple::F11, [h, s, l]);
            }
        }
    }
    for _ in 0..20_000 {
        chk(
            &p,
            Triple::F11,
            [rng.range_f32(-1000.0, -1.0), rng.range_f32(-2.0, 2.0), rng.tame_f32(2.0)],
        );
        chk(
            &p,
            Triple::F11,
            [rng.range_f32(360.0, 5000.0), rng.range_f32(-2.0, 2.0), rng.tame_f32(2.0)],
        );
    }

    // Unconstrained full-bit-space fuzz: every float class in every slot.
    for _ in 0..300_000 {
        chk(
            &p,
            Triple::F11,
            [rng.any_f32(), rng.any_f32(), rng.any_f32()],
        );
    }
    // Fuzz drawn from the special pool only (maximises NaN/inf interactions).
    let sp = all_special_f32();
    for _ in 0..200_000 {
        chk(
            &p,
            Triple::F11,
            [rng.pick(&sp), rng.pick(&sp), rng.pick(&sp)],
        );
    }
}

// ------------------------------------------------------------------ C38..C41
#[test]
fn c38_c41_f12_all_switch_arms_and_early_return() {
    let p = load();
    let mut rng = Rng::new(SEED ^ 38);

    // C38: s == 0 -> all three outputs = v.
    for &s in &[0.0f32, -0.0f32] {
        for h in boundary_hues() {
            for &v in SL_VALUES {
                chk(&p, Triple::F12, [h, s, v]);
            }
        }
        for _ in 0..20_000 {
            chk(&p, Triple::F12, [rng.any_f32(), s, rng.any_f32()]);
        }
    }

    // C39: i == 0..=4, one row per `switch` arm.
    for i in 0..5 {
        let lo = i as f32 * 60.0;
        for _ in 0..30_000 {
            let h = rng.range_f32(lo, lo + 60.0);
            chk(
                &p,
                Triple::F12,
                [h, rng.range_f32(-2.0, 2.0), rng.range_f32(-2.0, 2.0)],
            );
        }
        for &s in SL_VALUES {
            for &v in SL_VALUES {
                chk(&p, Triple::F12, [lo + 1.0, s, v]);
                chk(&p, Triple::F12, [lo + 30.0, s, v]);
                chk(&p, Triple::F12, [lo + 59.0, s, v]);
            }
        }
    }

    // C40: the `default:` arm -- i outside 0..=4.
    for h in [
        300.0f32,
        360.0,
        1e30,
        -1.0,
        -60.0,
        -1e30,
        f32::INFINITY,
        f32::NEG_INFINITY,
        2.147_483_6e9,
        -2.147_483_6e9,
        1.288e11, // h/60 far beyond INT_MAX
    ] {
        for &s in SL_VALUES {
            for &v in SL_VALUES {
                chk(&p, Triple::F12, [h, s, v]);
            }
        }
    }
    for &nb in NAN_BITS {
        let h = f32::from_bits(nb);
        for &s in SL_VALUES {
            for &v in SL_VALUES {
                chk(&p, Triple::F12, [h, s, v]);
            }
        }
    }

    // C41: exact sector boundaries and one ULP either side.
    for h in boundary_hues() {
        for &s in SL_VALUES {
            for &v in SL_VALUES {
                chk(&p, Triple::F12, [h, s, v]);
            }
        }
    }

    // Unconstrained fuzz.
    for _ in 0..300_000 {
        chk(
            &p,
            Triple::F12,
            [rng.any_f32(), rng.any_f32(), rng.any_f32()],
        );
    }
    let sp = all_special_f32();
    for _ in 0..200_000 {
        chk(
            &p,
            Triple::F12,
            [rng.pick(&sp), rng.pick(&sp), rng.pick(&sp)],
        );
    }
}

// ------------------------------------------------------------------ C42..C47
#[test]
fn c42_c47_f13_max_channel_selection_and_degenerates() {
    let p = load();
    let mut rng = Rng::new(SEED ^ 42);

    // C42/C43/C44: force each channel to be the strict maximum, and cover both
    // signs of the intermediate hue (so the `h += 360` correction fires).
    for _ in 0..60_000 {
        let big = rng.range_f32(0.2, 4.0);
        let a = rng.range_f32(-1.0, 0.19);
        let b = rng.range_f32(-1.0, 0.19);
        chk(&p, Triple::F13, [big, a, b]); // max == r
        chk(&p, Triple::F13, [a, big, b]); // max == g
        chk(&p, Triple::F13, [a, b, big]); // max == b
        // r max with g < b  ->  (g-b)/delta < 0  ->  h < 0  ->  h += 360
        chk(&p, Triple::F13, [big, a.min(b), a.max(b)]);
        chk(&p, Triple::F13, [big, a.max(b), a.min(b)]);
    }

    // C45: ties -- the C's `==` chain resolves them by branch order.
    for _ in 0..40_000 {
        let hi = rng.range_f32(0.2, 4.0);
        let lo = rng.range_f32(-2.0, 0.19);
        chk(&p, Triple::F13, [hi, hi, lo]); // r == g > b
        chk(&p, Triple::F13, [lo, hi, hi]); // g == b > r
        chk(&p, Triple::F13, [hi, lo, hi]); // r == b > g
    }

    // C46: delta == 0 / max == 0.
    for &x in &[0.0f32, -0.0, 1.0, -1.0, f32::INFINITY, f32::NEG_INFINITY, 1e-45] {
        chk(&p, Triple::F13, [x, x, x]);
    }
    for &z in &[0.0f32, -0.0] {
        for &n in &[-1.0f32, -1e-45, f32::NEG_INFINITY] {
            chk(&p, Triple::F13, [z, n, n]);
            chk(&p, Triple::F13, [n, z, n]);
            chk(&p, Triple::F13, [n, n, z]);
            chk(&p, Triple::F13, [z, z, n]);
            chk(&p, Triple::F13, [n, z, z]);
        }
    }
    // Signed-zero permutations (delta == 0 but the channels differ bitwise).
    for &a in &[0.0f32, -0.0] {
        for &b in &[0.0f32, -0.0] {
            for &c in &[0.0f32, -0.0] {
                chk(&p, Triple::F13, [a, b, c]);
            }
        }
    }

    // C47: negative / special channels, incl. the all-NaN `delta` path.
    let sp = all_special_f32();
    for &a in &sp {
        for &b in &sp {
            for &c in &sp {
                chk(&p, Triple::F13, [a, b, c]);
            }
        }
    }
    for _ in 0..300_000 {
        chk(
            &p,
            Triple::F13,
            [rng.any_f32(), rng.any_f32(), rng.any_f32()],
        );
    }
    for _ in 0..100_000 {
        // Mixed magnitudes so delta can overflow to +inf and s can be inf/NaN.
        chk(
            &p,
            Triple::F13,
            [
                rng.pick(&[f32::MAX, f32::MIN, 1.0, -1.0, 0.0]),
                rng.pick(&[f32::MAX, f32::MIN, 1e-45, -1e-45, 0.0]),
                rng.any_f32(),
            ],
        );
    }
}
