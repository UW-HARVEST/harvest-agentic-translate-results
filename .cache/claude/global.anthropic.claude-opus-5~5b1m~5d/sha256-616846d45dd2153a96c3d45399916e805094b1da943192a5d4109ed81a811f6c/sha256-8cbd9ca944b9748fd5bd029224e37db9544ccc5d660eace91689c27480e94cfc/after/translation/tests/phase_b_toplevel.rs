//! Phase B — differential tests for the level-3/4 entry points.
//!
//! `CONFIGS.md` rows 45-55: `c2Collided` (the pointer/enum dispatcher) and
//! `circle_collide` (the public one-shot in `c_src/include/lib.h`).

mod common;
use common::*;
use std::ffi::c_void;

/// A byte buffer that can hand out a pointer at a chosen odd/even offset, so
/// the unaligned-read path can be exercised.
struct Buf(Vec<u8>);

impl Buf {
    fn new() -> Buf {
        Buf(vec![0u8; 64])
    }
    /// Writes `v` at byte offset `off` and returns a pointer to it.
    fn put<T: Copy>(&mut self, off: usize, v: T) -> *const c_void {
        assert!(off + size_of::<T>() <= self.0.len());
        unsafe {
            let p = self.0.as_mut_ptr().add(off);
            std::ptr::copy_nonoverlapping(
                (&raw const v).cast::<u8>(),
                p,
                size_of::<T>(),
            );
            p.cast()
        }
    }
}

// ===========================================================================
// c2Collided — rows 45-50
// ===========================================================================

#[test]
fn row45_collided_circle() {
    let (c, r) = (&apis().c, &apis().r);
    let mut g = Rng::seeded(45);
    let mut case = Case::new("45 c2Collided typeB=CIRCLE");
    let mut ba = Buf::new();
    let mut bb = Buf::new();
    for _ in 0..20_000 {
        let a = C2Circle {
            p: g.v_finite(50.0),
            r: g.unit() * 40.0,
        };
        let b = C2Circle {
            p: g.v_finite(50.0),
            r: g.unit() * 40.0,
        };
        let pa = ba.put(0, a);
        let pb = bb.put(0, b);
        let cv = unsafe { (c.c2Collided)(pa, pb, C2_TYPE_CIRCLE) };
        let rv = unsafe { (r.c2Collided)(pa, pb, C2_TYPE_CIRCLE) };
        case.eq(cv, rv, || format!("{a:?} {b:?}"));
        // Must also agree with a direct call to the underlying routine.
        let direct = unsafe { (c.c2CircletoCircle)(a, b) };
        case.eq(cv, direct, || format!("dispatch != direct: {a:?} {b:?}"));
    }
    case.finish();
}

#[test]
fn row46_collided_aabb() {
    let (c, r) = (&apis().c, &apis().r);
    let mut g = Rng::seeded(46);
    let mut case = Case::new("46 c2Collided typeB=AABB");
    let mut ba = Buf::new();
    let mut bb = Buf::new();
    for _ in 0..20_000 {
        let a = C2Circle {
            p: g.v_finite(50.0),
            r: g.unit() * 40.0,
        };
        // Half valid boxes, half inverted.
        let (p, q) = (g.v_finite(50.0), g.v_finite(50.0));
        let b = if g.next_u32() & 1 == 0 {
            C2Aabb {
                min: C2v {
                    x: p.x.min(q.x),
                    y: p.y.min(q.y),
                },
                max: C2v {
                    x: p.x.max(q.x),
                    y: p.y.max(q.y),
                },
            }
        } else {
            C2Aabb { min: p, max: q }
        };
        let pa = ba.put(0, a);
        let pb = bb.put(0, b);
        let cv = unsafe { (c.c2Collided)(pa, pb, C2_TYPE_AABB) };
        let rv = unsafe { (r.c2Collided)(pa, pb, C2_TYPE_AABB) };
        case.eq(cv, rv, || format!("{a:?} {b:?}"));
        let direct = unsafe { (c.c2CircletoAABB)(a, b) };
        case.eq(cv, direct, || format!("dispatch != direct: {a:?} {b:?}"));
    }
    case.finish();
}

#[test]
fn row47_collided_capsule() {
    let (c, r) = (&apis().c, &apis().r);
    let mut g = Rng::seeded(47);
    let mut case = Case::new("47 c2Collided typeB=CAPSULE");
    let mut ba = Buf::new();
    let mut bb = Buf::new();
    for _ in 0..20_000 {
        let a = C2Circle {
            p: g.v_finite(80.0),
            r: g.unit() * 25.0,
        };
        let a0 = g.v_finite(50.0);
        let b = C2Capsule {
            a: a0,
            b: C2v {
                x: a0.x + g.sym(60.0),
                y: a0.y + g.sym(60.0),
            },
            r: g.unit() * 20.0,
        };
        let pa = ba.put(0, a);
        let pb = bb.put(0, b);
        let cv = unsafe { (c.c2Collided)(pa, pb, C2_TYPE_CAPSULE) };
        let rv = unsafe { (r.c2Collided)(pa, pb, C2_TYPE_CAPSULE) };
        case.eq(cv, rv, || format!("{a:?} {b:?}"));
        let direct = unsafe { (c.c2CircletoCapsule)(a, b) };
        case.eq(cv, direct, || format!("dispatch != direct: {a:?} {b:?}"));
    }
    case.finish();
}

#[test]
fn row48_collided_random_byte_payloads() {
    // The buffers are filled with raw random bytes, so every shape slot can be
    // any float class (NaN payloads, sNaN, subnormals, ±inf, ±0).
    let (c, r) = (&apis().c, &apis().r);
    let mut g = Rng::seeded(48);
    let mut case = Case::new("48 c2Collided random byte payloads");
    for _ in 0..30_000 {
        let mut ba = Buf::new();
        let mut bb = Buf::new();
        for i in 0..64 {
            ba.0[i] = g.next_u32() as u8;
            bb.0[i] = g.next_u32() as u8;
        }
        let pa: *const c_void = ba.0.as_ptr().cast();
        let pb: *const c_void = bb.0.as_ptr().cast();
        let t = [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE][g.below(3) as usize];
        let cv = unsafe { (c.c2Collided)(pa, pb, t) };
        let rv = unsafe { (r.c2Collided)(pa, pb, t) };
        case.eq(cv, rv, || {
            format!("typeB={t} A={:x?} B={:x?}", &ba.0[..12], &bb.0[..20])
        });
    }
    case.finish();
}

#[test]
fn row49_collided_aliased_pointers() {
    // C makes no aliasing assumption (`const void *`), so A and B may be the
    // same object; for typeB=CIRCLE this is "circle against itself".
    let (c, r) = (&apis().c, &apis().r);
    let mut g = Rng::seeded(49);
    let mut case = Case::new("49 c2Collided aliased A==B");
    let mut buf = Buf::new();
    for _ in 0..20_000 {
        let cap = C2Capsule {
            a: g.v_finite(40.0),
            b: g.v_finite(40.0),
            r: if g.next_u32() % 4 == 0 { 0.0 } else { g.unit() * 20.0 },
        };
        let p = buf.put(0, cap);
        for t in [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE] {
            let cv = unsafe { (c.c2Collided)(p, p, t) };
            let rv = unsafe { (r.c2Collided)(p, p, t) };
            case.eq(cv, rv, || format!("aliased typeB={t} {cap:?}"));
        }
    }
    case.finish();
}

#[test]
fn row50_collided_unaligned_b() {
    // `B` placed at an odd byte offset. The C code reads it with plain `mov`s;
    // the Rust port must use unaligned reads to match.
    let (c, r) = (&apis().c, &apis().r);
    let mut g = Rng::seeded(50);
    let mut case = Case::new("50 c2Collided unaligned B");
    for _ in 0..20_000 {
        let mut ba = Buf::new();
        let mut bb = Buf::new();
        let a = C2Circle {
            p: g.v_finite(50.0),
            r: g.unit() * 30.0,
        };
        let off_a = [0usize, 1, 2, 3, 5, 7][g.below(6) as usize];
        let off_b = [1usize, 3, 5, 7, 9, 11][g.below(6) as usize];
        let pa = ba.put(off_a, a);
        let (pb, t) = match g.below(3) {
            0 => (
                bb.put(
                    off_b,
                    C2Circle {
                        p: g.v_finite(50.0),
                        r: g.unit() * 30.0,
                    },
                ),
                C2_TYPE_CIRCLE,
            ),
            1 => (
                bb.put(
                    off_b,
                    C2Aabb {
                        min: g.v_finite(50.0),
                        max: g.v_finite(50.0),
                    },
                ),
                C2_TYPE_AABB,
            ),
            _ => (
                bb.put(
                    off_b,
                    C2Capsule {
                        a: g.v_finite(50.0),
                        b: g.v_finite(50.0),
                        r: g.unit() * 20.0,
                    },
                ),
                C2_TYPE_CAPSULE,
            ),
        };
        let cv = unsafe { (c.c2Collided)(pa, pb, t) };
        let rv = unsafe { (r.c2Collided)(pa, pb, t) };
        case.eq(cv, rv, || {
            format!("typeB={t} off_a={off_a} off_b={off_b}")
        });
    }
    case.finish();
}

// ===========================================================================
// circle_collide — rows 51-55
// ===========================================================================

fn cc3_row(row: &'static str, n: usize, seed: u64, mk: impl Fn(&mut Rng) -> (f32, f32, f32)) {
    let (c, r) = (&apis().c, &apis().r);
    let mut g = Rng::seeded(seed);
    let mut case = Case::new(row);
    let mut seen = [0usize; 8];
    for _ in 0..n {
        let (x, y, rr) = mk(&mut g);
        let cv = unsafe { (c.circle_collide)(x, y, rr) };
        let rv = unsafe { (r.circle_collide)(x, y, rr) };
        case.eq(cv, rv, || {
            format!(
                "circle_collide({x}, {y}, {rr})  bits=({:#010x},{:#010x},{:#010x})",
                fb(x),
                fb(y),
                fb(rr)
            )
        });
        if (0..8).contains(&cv) {
            seen[cv as usize] += 1;
        }
    }
    eprintln!("  {row}: result histogram {seen:?}");
    case.finish();
}

#[test]
fn row51_circle_collide_interesting_region() {
    // The three hard-coded shapes all live in x∈[-80,-10], y∈[-45,105].
    cc3_row("51 circle_collide region", 40_000, 51, |g| {
        (g.sym(150.0), g.sym(150.0), g.unit() * 60.0)
    });
}

#[test]
fn row52_circle_collide_targeted() {
    // Aimed at each hard-coded shape in turn, and at their overlaps, so every
    // one of the 8 packed results is produced.
    let (c, r) = (&apis().c, &apis().r);
    let mut g = Rng::seeded(52);
    let mut case = Case::new("52 circle_collide targeted");
    let mut seen = [0usize; 8];
    // circle (-70,0) r20 ; aabb [-40,-15]^2 ; capsule (-40,40)-(-20,100) r10
    let targets = [
        (-70.0f32, 0.0f32),
        (-27.5, -27.5),
        (-40.0, 40.0),
        (-20.0, 100.0),
        (-30.0, 70.0),
        (-55.0, -14.0), // between circle and box
        (-45.0, 20.0),  // between box and capsule
        (-50.0, 10.0),  // near all three
    ];
    for _ in 0..40_000 {
        let (tx, ty) = targets[g.below(targets.len() as u32) as usize];
        let spread = [0.5f32, 2.0, 8.0, 30.0, 60.0][g.below(5) as usize];
        let x = tx + g.sym(spread);
        let y = ty + g.sym(spread);
        let rr = g.unit() * 70.0;
        let cv = unsafe { (c.circle_collide)(x, y, rr) };
        let rv = unsafe { (r.circle_collide)(x, y, rr) };
        case.eq(cv, rv, || format!("circle_collide({x}, {y}, {rr})"));
        if (0..8).contains(&cv) {
            seen[cv as usize] += 1;
        }
    }
    eprintln!("  row 52: result histogram {seen:?}");
    assert!(
        seen.iter().all(|&s| s > 0),
        "not all 8 packed results produced: {seen:?}"
    );
    case.finish();
}

#[test]
fn row53_circle_collide_radius_specials() {
    cc3_row("53 circle_collide radius specials", 20_000, 53, |g| {
        let rr = [
            0.0f32,
            -0.0,
            -20.0,
            -1.0,
            f32::MIN_POSITIVE,
            f32::from_bits(1),
            f32::MAX,
            1e38,
            3.0e38,
        ][g.below(9) as usize];
        (g.sym(120.0), g.sym(120.0), rr)
    });
}

#[test]
fn row54_circle_collide_nan_inf_random_bits() {
    cc3_row("54 circle_collide NaN/inf/random bits", 40_000, 54, |g| {
        (g.spicy(), g.spicy(), g.spicy())
    });
}

#[test]
fn row55_circle_collide_edges_cubed() {
    let (c, r) = (&apis().c, &apis().r);
    let mut case = Case::new("55 circle_collide edges^3");
    for &x in EDGES {
        for &y in EDGES {
            for &rr in EDGES {
                let cv = unsafe { (c.circle_collide)(x, y, rr) };
                let rv = unsafe { (r.circle_collide)(x, y, rr) };
                case.eq(cv, rv, || {
                    format!(
                        "circle_collide bits=({:#010x},{:#010x},{:#010x})",
                        fb(x),
                        fb(y),
                        fb(rr)
                    )
                });
            }
        }
    }
    case.finish();
}
