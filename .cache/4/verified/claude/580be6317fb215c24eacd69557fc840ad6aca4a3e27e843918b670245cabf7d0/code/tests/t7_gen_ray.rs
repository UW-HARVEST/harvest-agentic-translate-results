//! Phase B rows B62..B68 and Phase C row E34 for `gen_ray`, the only entry
//! point declared in the public header `c_src/include/lib.h`.
//!
//! Parameter order:
//! `mp_x mp_y r_p_x r_p_y c_p_x c_p_y c_r cap_a_x cap_a_y cap_b_x cap_b_y cap_r
//!  bb_min_x bb_min_y bb_max_x bb_max_y`

mod common;
use common::*;
use std::collections::HashSet;

fn both(d: &mut Diff, label: &str, a: &GenRayArgs) -> i32 {
    let (c, r) = apis();
    let rc = call_gen_ray(c, a);
    let rr = call_gen_ray(r, a);
    let ret = rc.ret;
    d.gen_cmp(label, || format!("{:?}", a), rc, rr);
    ret
}

/// Build a parameter set that aims for the requested 3-bit hit mask
/// (bit0 = circle, bit1 = capsule, bit2 = AABB).
fn args_for_mask(rng: &mut Rng, mask: u32) -> GenRayArgs {
    // Ray goes from `r_p` to `mp`.
    let px = rng.uniform(30.0);
    let py = rng.uniform(30.0);
    let len = (rng.uniform(40.0)).abs() + 5.0;
    let dir = rng.unit();
    let mx = px + dir.x * len;
    let my = py + dir.y * len;
    let perp = c2v { x: -dir.y, y: dir.x };
    let at = |f: f32| c2v {
        x: px + dir.x * len * f,
        y: py + dir.y * len * f,
    };

    // circle
    let (cx, cy, cr) = if mask & 1 != 0 {
        let p = at(0.5);
        (p.x, p.y, len * 0.1)
    } else {
        (px + perp.x * 1e4, py + perp.y * 1e4, 1.0)
    };
    // capsule: a short segment straddling the ray at f = 0.7
    let (ax, ay, bx2, by2, capr) = if mask & 2 != 0 {
        let p = at(0.7);
        (
            p.x - perp.x * len * 0.2,
            p.y - perp.y * len * 0.2,
            p.x + perp.x * len * 0.2,
            p.y + perp.y * len * 0.2,
            len * 0.05,
        )
    } else {
        let q = c2v {
            x: px - perp.x * 1e4,
            y: py - perp.y * 1e4,
        };
        (q.x, q.y, q.x + 1.0, q.y + 1.0, 0.5)
    };
    // aabb around f = 0.3
    let (bminx, bminy, bmaxx, bmaxy) = if mask & 4 != 0 {
        let p = at(0.3);
        let h = len * 0.05;
        (p.x - h, p.y - h, p.x + h, p.y + h)
    } else {
        (1e5, 1e5, 1e5 + 1.0, 1e5 + 1.0)
    };

    [
        mx, my, px, py, cx, cy, cr, ax, ay, bx2, by2, capr, bminx, bminy, bmaxx, bmaxy,
    ]
}

/// B62 + E34: every one of the eight hit masks.
#[test]
fn b62_e34_all_hit_masks() {
    let mut d = Diff::new();
    let mut rng = Rng::new(0xB62);
    let mut seen: HashSet<i32> = HashSet::new();
    for i in 0..40_000u32 {
        let mask = i % 8;
        let a = args_for_mask(&mut rng, mask);
        let ret = both(&mut d, "B62", &a);
        seen.insert(ret);
        d.check((0..=7).contains(&ret), || {
            format!("E34: gen_ray returned {ret}, outside 0..=7, args={:?}", a)
        });
    }
    let mut missing = Vec::new();
    for m in 0..8 {
        if !seen.contains(&m) {
            missing.push(m);
        }
    }
    assert!(
        missing.is_empty(),
        "hit masks never produced: {:?} (saw {:?})",
        missing,
        seen
    );
    d.finish("B62/E34 gen_ray all hit masks");
}

/// B63 + E30: `mp == ray.p` => `c2Norm((0,0))` => NaN direction and NaN `ray.t`.
#[test]
fn b63_zero_length_ray() {
    let mut d = Diff::new();
    let mut rng = Rng::new(0xB63);
    for _ in 0..20_000 {
        let m = rng.below(8);
        let mut a = args_for_mask(&mut rng, m);
        // force mp == r_p
        a[0] = a[2];
        a[1] = a[3];
        both(&mut d, "B63", &a);
    }
    // exact zeros and signed zeros
    for (mp, rp) in [
        (0.0f32, 0.0f32),
        (-0.0, 0.0),
        (0.0, -0.0),
        (1.0, 1.0),
        (-3.5, -3.5),
    ] {
        let mut rng2 = Rng::new(0x63000);
        let mut a = args_for_mask(&mut rng2, 7);
        a[0] = mp;
        a[1] = mp;
        a[2] = rp;
        a[3] = rp;
        both(&mut d, "B63/exact", &a);
    }
    d.finish("B63 gen_ray zero-length ray");
}

/// B64: far-away mouse point, tiny/subnormal shapes.
#[test]
fn b64_extreme_scales() {
    let mut d = Diff::new();
    let mut rng = Rng::new(0xB64);
    let scales = [
        1e-30f32,
        1e-10,
        1e-3,
        1.0,
        1e3,
        1e10,
        1e30,
        f32::MIN_POSITIVE,
        f32::MAX,
    ];
    for _ in 0..2_000 {
        for &s in &scales {
            let m = rng.below(8);
        let mut a = args_for_mask(&mut rng, m);
            // scale the geometry
            for v in a.iter_mut() {
                *v *= s;
            }
            both(&mut d, "B64/scaled", &a);
            // huge ray, tiny shapes
            let mut b = a;
            b[0] = 1e30;
            b[1] = 1e30;
            b[6] = f32::MIN_POSITIVE;
            b[11] = f32::MIN_POSITIVE;
            both(&mut d, "B64/mixed", &b);
        }
    }
    d.finish("B64 gen_ray extreme scales");
}

/// B65: hostile floats in every one of the 16 scalar parameters.
#[test]
fn b65_specials_per_parameter() {
    let mut d = Diff::new();
    let mut rng = Rng::new(0xB65);
    for _ in 0..200 {
        let m = rng.below(8);
        let base = args_for_mask(&mut rng, m);
        for idx in 0..16 {
            for &s in &SPECIALS {
                let mut a = base;
                a[idx] = s;
                both(&mut d, "B65", &a);
            }
        }
    }
    d.finish("B65 gen_ray specials per parameter");
}

/// B66: all three out-pointers aliased to the same `c2Raycast`.
#[test]
fn b66_aliased_out_pointers() {
    let (c, r) = apis();
    let mut d = Diff::new();
    let mut rng = Rng::new(0xB66);
    for i in 0..40_000u32 {
        let a = args_for_mask(&mut rng, i % 8);
        let (rc, oc) = call_gen_ray_aliased(c, &a);
        let (rr, orr) = call_gen_ray_aliased(r, &a);
        d.check(rc == rr && cast_eq(oc, orr), || {
            format!(
                "B66 [{:?}]: C ret={} out={} | RUST ret={} out={}",
                a,
                rc,
                fmt_cast(oc),
                rr,
                fmt_cast(orr)
            )
        });
    }
    d.finish("B66 gen_ray aliased out pointers");
}

/// B67: fuzz over "nice" geometry-plausible parameters.
#[test]
fn b67_fuzz_nice() {
    let mut d = Diff::new();
    let mut rng = Rng::new(0xB67);
    for _ in 0..20_000 {
        let mut a: GenRayArgs = [0.0; 16];
        for v in a.iter_mut() {
            *v = rng.nice();
        }
        both(&mut d, "B67", &a);
    }
    d.finish("B67 gen_ray fuzz (nice)");
}

/// B68: fuzz over hostile parameters (`±0`, `±inf`, qNaN, sNaN, subnormals,
/// raw bit patterns).
#[test]
fn b68_fuzz_hostile() {
    let mut d = Diff::new();
    let mut rng = Rng::new(0xB68);
    for _ in 0..20_000 {
        let mut a: GenRayArgs = [0.0; 16];
        for v in a.iter_mut() {
            *v = rng.hostile();
        }
        both(&mut d, "B68/all-hostile", &a);
    }
    for _ in 0..20_000 {
        // mostly nice with a few hostile values sprinkled in
        let mut a: GenRayArgs = [0.0; 16];
        for v in a.iter_mut() {
            *v = rng.nice();
        }
        for _ in 0..3 {
            a[rng.below(16) as usize] = rng.hostile();
        }
        both(&mut d, "B68/sprinkled", &a);
    }
    d.finish("B68 gen_ray fuzz (hostile)");
}
