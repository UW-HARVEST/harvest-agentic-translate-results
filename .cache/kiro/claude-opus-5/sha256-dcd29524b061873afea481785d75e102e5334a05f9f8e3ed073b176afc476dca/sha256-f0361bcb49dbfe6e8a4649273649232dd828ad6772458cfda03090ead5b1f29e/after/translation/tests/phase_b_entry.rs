//! Phase B — the public header entry point `gjk`, driven through both `.so`s.
//! CONFIGS.md rows 80–90.

mod common;
use common::*;

/// One differential `gjk` call, comparing both out-vectors bit-for-bit.
fn diff(ctx: &str, reverse: i8, p: [f32; 9]) {
    let (c, r) = (&libs().c, &libs().r);
    let poison = c2v { x: -1.5e30, y: 2.5e-30 };
    let (mut ca, mut cb) = (poison, poison);
    let (mut ra, mut rb) = (poison, poison);
    unsafe {
        (c.gjk)(reverse, &mut ca, &mut cb, p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8]);
        (r.gjk)(reverse, &mut ra, &mut rb, p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8]);
    }
    assert_veq(&format!("{ctx} outA rev={reverse} p={p:?}"), ca, ra);
    assert_veq(&format!("{ctx} outB rev={reverse} p={p:?}"), cb, rb);
}

/// row 80/81: 20 000 randomized cases per `reverse` value.
#[test]
fn row80_row81_large_random_sweep() {
    let mut rng = Rng::new(SEED ^ 80);
    for i in 0..20_000 {
        let p = [
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.radius(),
        ];
        diff(&format!("row80 rev=0 #{i}"), 0, p);
        diff(&format!("row81 rev=1 #{i}"), 1, p);
        // also with a properly ordered AABB (min <= max), which is the intended
        // usage and takes different `c2Support` paths
        let q = [
            p[0].min(p[2]),
            p[1].min(p[3]),
            p[0].max(p[2]),
            p[1].max(p[3]),
            p[4],
            p[5],
            p[6],
            p[7],
            p[8].abs(),
        ];
        diff(&format!("row80 ordered rev=0 #{i}"), 0, q);
        diff(&format!("row81 ordered rev=1 #{i}"), 1, q);
    }
}

/// row 82: `reverse` is a `char`, so every non-zero bit pattern is truthy.
#[test]
fn row82_reverse_bit_patterns() {
    let mut rng = Rng::new(SEED ^ 82);
    // all 256 possible `char` values
    for i in 0..40 {
        let p = [
            rng.coord(), rng.coord(), rng.coord(), rng.coord(),
            rng.coord(), rng.coord(), rng.coord(), rng.coord(),
            rng.radius(),
        ];
        for v in 0..256u32 {
            let rev = v as u8 as i8;
            diff(&format!("row82 rev=0x{v:02x} #{i}"), rev, p);
        }
    }
}

/// row 83: overlapping AABB / capsule (the `hit` path through the public API).
#[test]
fn row83_overlapping() {
    let mut rng = Rng::new(SEED ^ 83);
    for i in 0..8000 {
        let (cx, cy) = (rng.sym(50.0), rng.sym(50.0));
        let (hw, hh) = (0.5 + rng.unit() * 5.0, 0.5 + rng.unit() * 5.0);
        // capsule endpoints placed inside the box
        let p = [
            cx - hw,
            cy - hh,
            cx + hw,
            cy + hh,
            cx + rng.sym(hw * 0.8),
            cy + rng.sym(hh * 0.8),
            cx + rng.sym(hw * 0.8),
            cy + rng.sym(hh * 0.8),
            rng.unit() * 2.0,
        ];
        diff(&format!("row83 overlap rev=0 #{i}"), 0, p);
        diff(&format!("row83 overlap rev=1 #{i}"), 1, p);
        let _ = i;
    }
}

/// row 84: far apart.
#[test]
fn row84_far_apart() {
    let mut rng = Rng::new(SEED ^ 84);
    for i in 0..8000 {
        let scale = [1.0f32, 1e3, 1e6, 1e18, 1e30][i % 5];
        let p = [
            -1.0,
            -1.0,
            1.0,
            1.0,
            scale + rng.sym(1.0),
            scale * 0.5 + rng.sym(1.0),
            scale + 1.0 + rng.sym(1.0),
            scale * 0.5 + 1.0 + rng.sym(1.0),
            rng.unit() * 3.0,
        ];
        diff(&format!("row84 far rev=0 #{i}"), 0, p);
        diff(&format!("row84 far rev=1 #{i}"), 1, p);
    }
}

/// row 85: exactly touching — the boundary of `dist > rA + rB`.
#[test]
fn row85_exactly_touching() {
    let mut rng = Rng::new(SEED ^ 85);
    for i in 0..8000 {
        let rad = [0.0f32, 0.5, 1.0, 2.0, 1.192_092_9e-7][i % 5];
        // box spans [-1,1]^2; capsule is a vertical segment at x = 1 + rad,
        // so the surface distance is exactly zero
        let deltas = [
            0.0f32,
            1.192_092_9e-7,
            -1.192_092_9e-7,
            f32::EPSILON,
            -f32::EPSILON,
            1e-4,
            -1e-4,
        ];
        let d = deltas[i % deltas.len()];
        let x = 1.0 + rad + d;
        let p = [-1.0, -1.0, 1.0, 1.0, x, -0.5, x, 0.5, rad];
        diff(&format!("row85 touch rad={rad} d={d:?} rev=0 #{i}"), 0, p);
        diff(&format!("row85 touch rad={rad} d={d:?} rev=1 #{i}"), 1, p);
        let _ = rng.next_u32();
    }
}

/// row 86: integer-grid coordinates, which produce many exact ties in
/// `c2Support` (`dot > dmax` is strict, so ties resolve by lowest index).
#[test]
fn row86_integer_grid() {
    let mut rng = Rng::new(SEED ^ 86);
    for i in 0..20_000 {
        let g = |rng: &mut Rng| (rng.below(21) as i32 - 10) as f32;
        let p = [
            g(&mut rng), g(&mut rng), g(&mut rng), g(&mut rng),
            g(&mut rng), g(&mut rng), g(&mut rng), g(&mut rng),
            (rng.below(5)) as f32,
        ];
        diff(&format!("row86 grid rev=0 #{i}"), 0, p);
        diff(&format!("row86 grid rev=1 #{i}"), 1, p);
        // half-integer grid too (still exactly representable)
        let h = [
            p[0] * 0.5, p[1] * 0.5, p[2] * 0.5, p[3] * 0.5,
            p[4] * 0.5, p[5] * 0.5, p[6] * 0.5, p[7] * 0.5,
            p[8] * 0.5,
        ];
        diff(&format!("row86 half-grid rev=0 #{i}"), 0, h);
        diff(&format!("row86 half-grid rev=1 #{i}"), 1, h);
    }
}

/// row 87: degenerate extents and radii.
#[test]
fn row87_degenerate_extents() {
    let mut rng = Rng::new(SEED ^ 87);
    let radii = [
        0.0f32, -0.0, 1e-30, 1e-7, 1.0, 1e6, 1e30,
        f32::MAX, f32::MIN_POSITIVE, f32::from_bits(1),
        -1.0, -1e6, -f32::MAX,
    ];
    for i in 0..800 {
        let (x, y) = (rng.coord(), rng.coord());
        let (bx, by) = (rng.coord(), rng.coord());
        for &rad in &radii {
            for rev in [0i8, 1] {
                // zero-extent AABB
                diff(&format!("row87 zero-aabb rad={rad:?} #{i}"), rev, [x, y, x, y, bx, by, bx + 1.0, by + 2.0, rad]);
                // zero-height / zero-width AABB
                diff(&format!("row87 flat-aabb-h rad={rad:?} #{i}"), rev, [x, y, x + 3.0, y, bx, by, bx + 1.0, by, rad]);
                diff(&format!("row87 flat-aabb-w rad={rad:?} #{i}"), rev, [x, y, x, y + 3.0, bx, by, bx, by + 1.0, rad]);
                // zero-length capsule
                diff(&format!("row87 zero-cap rad={rad:?} #{i}"), rev, [x, y, x + 2.0, y + 2.0, bx, by, bx, by, rad]);
                // both degenerate
                diff(&format!("row87 both rad={rad:?} #{i}"), rev, [x, y, x, y, bx, by, bx, by, rad]);
                // inverted AABB
                diff(&format!("row87 inverted rad={rad:?} #{i}"), rev, [x + 2.0, y + 2.0, x, y, bx, by, bx + 1.0, by, rad]);
            }
        }
    }
}

/// row 88: NaN / ±inf in each parameter, one at a time and all at once.
#[test]
fn row88_special_values_per_parameter() {
    let mut rng = Rng::new(SEED ^ 88);
    let bad = [
        f32::NAN, -f32::NAN, f32::from_bits(0x7FC0_DEAD),
        f32::INFINITY, f32::NEG_INFINITY,
        f32::MAX, f32::MIN, f32::MIN_POSITIVE, -f32::MIN_POSITIVE,
        f32::from_bits(1), f32::from_bits(0x8000_0001), -0.0,
    ];
    for i in 0..400 {
        let base = [
            rng.coord(), rng.coord(), rng.coord(), rng.coord(),
            rng.coord(), rng.coord(), rng.coord(), rng.coord(),
            rng.radius(),
        ];
        for slot in 0..9usize {
            for &v in &bad {
                let mut p = base;
                p[slot] = v;
                diff(&format!("row88 slot={slot} v={v:?} rev=0 #{i}"), 0, p);
                diff(&format!("row88 slot={slot} v={v:?} rev=1 #{i}"), 1, p);
            }
        }
        // two slots poisoned simultaneously
        for s1 in 0..9usize {
            let s2 = (s1 + 4) % 9;
            for &v in &bad[..5] {
                let mut p = base;
                p[s1] = v;
                p[s2] = v;
                diff(&format!("row88 slots={s1},{s2} v={v:?} #{i}"), 0, p);
            }
        }
    }
}

/// row 89: NULL out-pointers, all four combinations.
#[test]
fn row89_null_out_pointers() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 89);
    let poison = c2v { x: 11.0, y: -22.0 };
    for i in 0..5000 {
        let p = [
            rng.coord(), rng.coord(), rng.coord(), rng.coord(),
            rng.coord(), rng.coord(), rng.coord(), rng.coord(),
            rng.radius(),
        ];
        for rev in [0i8, 1] {
            for (wa, wb) in [(true, true), (true, false), (false, true), (false, false)] {
                let (mut ca, mut cb, mut ra, mut rb) = (poison, poison, poison, poison);
                unsafe {
                    (c.gjk)(
                        rev,
                        if wa { &mut ca } else { std::ptr::null_mut() },
                        if wb { &mut cb } else { std::ptr::null_mut() },
                        p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8],
                    );
                    (r.gjk)(
                        rev,
                        if wa { &mut ra } else { std::ptr::null_mut() },
                        if wb { &mut rb } else { std::ptr::null_mut() },
                        p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8],
                    );
                }
                assert_veq(&format!("row89 wa={wa} wb={wb} rev={rev} A #{i}"), ca, ra);
                assert_veq(&format!("row89 wa={wa} wb={wb} rev={rev} B #{i}"), cb, rb);
                assert_eq!(veq(ca, poison), !wa, "row89 C outA write pattern");
                assert_eq!(veq(ra, poison), !wa, "row89 Rust outA write pattern");
                assert_eq!(veq(cb, poison), !wb, "row89 C outB write pattern");
                assert_eq!(veq(rb, poison), !wb, "row89 Rust outB write pattern");
            }
        }
    }
}

/// row 90: `a` and `b` aliasing the same `c2v`.
#[test]
fn row90_aliased_out_pointers() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 90);
    for i in 0..5000 {
        let p = [
            rng.coord(), rng.coord(), rng.coord(), rng.coord(),
            rng.coord(), rng.coord(), rng.coord(), rng.coord(),
            rng.radius(),
        ];
        for rev in [0i8, 1] {
            let mut cv = c2v { x: 3.0, y: -4.0 };
            let mut rv = cv;
            unsafe {
                let cp: *mut c2v = &mut cv;
                (c.gjk)(rev, cp, cp, p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8]);
                let rp: *mut c2v = &mut rv;
                (r.gjk)(rev, rp, rp, p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8]);
            }
            assert_veq(&format!("row90 aliased rev={rev} #{i}"), cv, rv);
        }
    }
}

/// The header's declared signature is the contract an external consumer sees;
/// this drives it exactly as `c_src/include/lib.h` spells it out and checks
/// that the two libraries agree over a very large sample.
#[test]
fn row80_header_contract_bulk() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 0xABCD);
    let mut nonzero_a = 0usize;
    let mut nonzero_b = 0usize;
    for i in 0..60_000 {
        let rev = (rng.next_u32() & 1) as i8;
        let p = [
            rng.wild(), rng.wild(), rng.wild(), rng.wild(),
            rng.wild(), rng.wild(), rng.wild(), rng.wild(),
            rng.wild(),
        ];
        let (mut ca, mut cb) = (c2v::default(), c2v::default());
        let (mut ra, mut rb) = (c2v::default(), c2v::default());
        unsafe {
            (c.gjk)(rev, &mut ca, &mut cb, p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8]);
            (r.gjk)(rev, &mut ra, &mut rb, p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8]);
        }
        assert_veq(&format!("bulk outA #{i}"), ca, ra);
        assert_veq(&format!("bulk outB #{i}"), cb, rb);
        if ca.x != 0.0 || ca.y != 0.0 {
            nonzero_a += 1;
        }
        if cb.x != 0.0 || cb.y != 0.0 {
            nonzero_b += 1;
        }
    }
    eprintln!("bulk sweep: non-trivial outA={nonzero_a}, outB={nonzero_b} of 60000");
    assert!(nonzero_a > 1000 && nonzero_b > 1000, "bulk sweep produced mostly zeros");
}
