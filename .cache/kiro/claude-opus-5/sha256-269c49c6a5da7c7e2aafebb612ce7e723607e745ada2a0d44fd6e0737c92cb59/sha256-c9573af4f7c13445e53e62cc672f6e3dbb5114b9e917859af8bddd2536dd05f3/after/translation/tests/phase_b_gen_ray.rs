//! Phase B — CONFIGS.md rows 46–53: `gen_ray` (the only symbol in the public
//! header) end-to-end, and the struct-ABI parity sweep across all 22 exports.

#![allow(non_snake_case)]

mod common;
use common::*;

const SEED: u64 = 0x5EED_C2A1;
const N: usize = 15_000;

/// The 16 float parameters of `gen_ray`, in declaration order.
#[derive(Copy, Clone, Debug, Default)]
struct Params {
    mp_x: f32,
    mp_y: f32,
    r_p_x: f32,
    r_p_y: f32,
    c_p_x: f32,
    c_p_y: f32,
    c_r: f32,
    cap_a_x: f32,
    cap_a_y: f32,
    cap_b_x: f32,
    cap_b_y: f32,
    cap_r: f32,
    bb_min_x: f32,
    bb_min_y: f32,
    bb_max_x: f32,
    bb_max_y: f32,
}

impl Params {
    fn set(&mut self, i: usize, v: f32) {
        match i {
            0 => self.mp_x = v,
            1 => self.mp_y = v,
            2 => self.r_p_x = v,
            3 => self.r_p_y = v,
            4 => self.c_p_x = v,
            5 => self.c_p_y = v,
            6 => self.c_r = v,
            7 => self.cap_a_x = v,
            8 => self.cap_a_y = v,
            9 => self.cap_b_x = v,
            10 => self.cap_b_y = v,
            11 => self.cap_r = v,
            12 => self.bb_min_x = v,
            13 => self.bb_min_y = v,
            14 => self.bb_max_x = v,
            _ => self.bb_max_y = v,
        }
    }
    fn as_array(&self) -> [f32; 16] {
        [
            self.mp_x, self.mp_y, self.r_p_x, self.r_p_y, self.c_p_x, self.c_p_y,
            self.c_r, self.cap_a_x, self.cap_a_y, self.cap_b_x, self.cap_b_y,
            self.cap_r, self.bb_min_x, self.bb_min_y, self.bb_max_x, self.bb_max_y,
        ]
    }
}

fn fmt_params(p: &Params) -> String {
    let a = p.as_array();
    let names = [
        "mp_x", "mp_y", "r_p_x", "r_p_y", "c_p_x", "c_p_y", "c_r", "cap_a_x",
        "cap_a_y", "cap_b_x", "cap_b_y", "cap_r", "bb_min_x", "bb_min_y",
        "bb_max_x", "bb_max_y",
    ];
    names
        .iter()
        .zip(a.iter())
        .map(|(n, v)| format!("{n}={}", fmt_f(*v)))
        .collect::<Vec<_>>()
        .join(" ")
}

/// One `gen_ray` invocation on both libraries, with all three out-params poisoned.
#[allow(clippy::type_complexity)]
fn call(p: &Params) -> (i32, [c2Raycast; 3], i32, [c2Raycast; 3]) {
    let l = libs();
    let mut co = [POISON; 3];
    let mut ro = [POISON; 3];
    let a = p.as_array();
    let cr = unsafe {
        (l.c.gen_ray)(
            &mut co[0], &mut co[1], &mut co[2], a[0], a[1], a[2], a[3], a[4],
            a[5], a[6], a[7], a[8], a[9], a[10], a[11], a[12], a[13], a[14], a[15],
        )
    };
    let rr = unsafe {
        (l.r.gen_ray)(
            &mut ro[0], &mut ro[1], &mut ro[2], a[0], a[1], a[2], a[3], a[4],
            a[5], a[6], a[7], a[8], a[9], a[10], a[11], a[12], a[13], a[14], a[15],
        )
    };
    (cr, co, rr, ro)
}

fn check(d: &mut Diff, p: &Params) -> i32 {
    let (cr, co, rr, ro) = call(p);
    let ok = cr == rr
        && rc_eq(co[0], ro[0])
        && rc_eq(co[1], ro[1])
        && rc_eq(co[2], ro[2]);
    d.check(ok, || {
        format!(
            "gen_ray({})\n    C   -> ret={} c1={} c2={} c3={}\n    Rust-> ret={} c1={} c2={} c3={}",
            fmt_params(p),
            cr,
            fmt_rc(co[0]),
            fmt_rc(co[1]),
            fmt_rc(co[2]),
            rr,
            fmt_rc(ro[0]),
            fmt_rc(ro[1]),
            fmt_rc(ro[2])
        )
    });
    cr
}

/// Random parameters covering all three shapes near a common region.
fn rand_params(rng: &mut Rng, spread: f32) -> Params {
    let mut p = Params::default();
    p.r_p_x = rng.range(-spread, spread);
    p.r_p_y = rng.range(-spread, spread);
    p.mp_x = rng.range(-spread, spread);
    p.mp_y = rng.range(-spread, spread);
    p.c_p_x = rng.range(-spread, spread);
    p.c_p_y = rng.range(-spread, spread);
    p.c_r = rng.range(0.01, spread * 0.5);
    p.cap_a_x = rng.range(-spread, spread);
    p.cap_a_y = rng.range(-spread, spread);
    p.cap_b_x = rng.range(-spread, spread);
    p.cap_b_y = rng.range(-spread, spread);
    p.cap_r = rng.range(0.01, spread * 0.4);
    let bx = rng.range(-spread, spread);
    let by = rng.range(-spread, spread);
    p.bb_min_x = bx;
    p.bb_min_y = by;
    p.bb_max_x = bx + rng.range(0.01, spread);
    p.bb_max_y = by + rng.range(0.01, spread);
    p
}

/// Row 46 — general random end-to-end sweep.
#[test]
fn cfg_46_gen_ray_random() {
    let mut rng = Rng::new(SEED ^ 46);
    let mut d = Diff::new("row46 gen_ray random");
    let mut ret_hist = [0usize; 8];
    for _ in 0..N * 2 {
        let p = rand_params(&mut rng, 40.0);
        let r = check(&mut d, &p);
        if (0..8).contains(&r) {
            ret_hist[r as usize] += 1;
        }
    }
    eprintln!("    row46 return-code histogram: {ret_hist:?}");
    d.finish();
}

/// Row 47 — the all-three-hit configuration (`ret == 7`) with the full output
/// triple written. Constructed, then confirmed by observing the C's return.
#[test]
fn cfg_47_gen_ray_all_three_hit() {
    let mut rng = Rng::new(SEED ^ 47);
    let mut d = Diff::new("row47 gen_ray all-three hit");
    let mut found = 0usize;
    let mut tries = 0usize;
    while found < N && tries < N * 400 {
        tries += 1;
        // Ray from left to right through the origin; put all three shapes on it.
        let mut p = Params::default();
        p.r_p_x = -100.0 + rng.range(-5.0, 5.0);
        p.r_p_y = rng.range(-1.0, 1.0);
        p.mp_x = 100.0 + rng.range(-5.0, 5.0);
        p.mp_y = p.r_p_y + rng.range(-0.5, 0.5);
        // Circle straddling the ray.
        p.c_p_x = rng.range(-60.0, -20.0);
        p.c_p_y = p.r_p_y + rng.range(-3.0, 3.0);
        p.c_r = rng.range(6.0, 15.0);
        // Capsule crossing the ray vertically.
        p.cap_a_x = rng.range(-10.0, 10.0);
        p.cap_a_y = p.r_p_y - rng.range(10.0, 30.0);
        p.cap_b_x = p.cap_a_x + rng.range(-2.0, 2.0);
        p.cap_b_y = p.r_p_y + rng.range(10.0, 30.0);
        p.cap_r = rng.range(2.0, 8.0);
        // Box straddling the ray further along.
        let cx = rng.range(30.0, 70.0);
        p.bb_min_x = cx - rng.range(5.0, 15.0);
        p.bb_max_x = cx + rng.range(5.0, 15.0);
        p.bb_min_y = p.r_p_y - rng.range(5.0, 15.0);
        p.bb_max_y = p.r_p_y + rng.range(5.0, 15.0);

        let r = check(&mut d, &p);
        if r == 7 {
            found += 1;
        }
    }
    assert!(
        found > 100,
        "only {found} of {tries} constructed cases returned 7 (all three hit)"
    );
    eprintln!("    row47: {found} all-three-hit cases out of {tries}");
    d.finish();
}

/// Row 48 — every return code 0..=7 must be observed at least once, so each hit
/// bit and each combination of hit bits is exercised.
#[test]
fn cfg_48_gen_ray_all_return_codes() {
    let mut rng = Rng::new(SEED ^ 48);
    let mut d = Diff::new("row48 gen_ray all return codes");
    let mut hist = [0usize; 8];

    // Broad sweep at several spatial scales to make every combination appear.
    for spread in [3.0f32, 10.0, 40.0, 150.0, 600.0] {
        for _ in 0..N {
            let p = rand_params(&mut rng, spread);
            let r = check(&mut d, &p);
            if (0..8).contains(&r) {
                hist[r as usize] += 1;
            }
        }
    }
    // Targeted single-shape configurations: park the other two shapes far away.
    for _ in 0..N {
        let mut p = rand_params(&mut rng, 40.0);
        let far = 1.0e5;
        // circle only
        let mut q = p;
        q.cap_a_x = far;
        q.cap_a_y = far;
        q.cap_b_x = far + 10.0;
        q.cap_b_y = far;
        q.bb_min_x = -far;
        q.bb_min_y = -far;
        q.bb_max_x = -far + 1.0;
        q.bb_max_y = -far + 1.0;
        let r = check(&mut d, &q);
        if (0..8).contains(&r) {
            hist[r as usize] += 1;
        }
        // capsule only
        let mut q = p;
        q.c_p_x = far;
        q.c_p_y = far;
        q.c_r = 0.001;
        q.bb_min_x = -far;
        q.bb_min_y = -far;
        q.bb_max_x = -far + 1.0;
        q.bb_max_y = -far + 1.0;
        let r = check(&mut d, &q);
        if (0..8).contains(&r) {
            hist[r as usize] += 1;
        }
        // box only
        p.c_p_x = far;
        p.c_p_y = far;
        p.c_r = 0.001;
        p.cap_a_x = -far;
        p.cap_a_y = -far;
        p.cap_b_x = -far + 10.0;
        p.cap_b_y = -far;
        p.cap_r = 0.001;
        let r = check(&mut d, &p);
        if (0..8).contains(&r) {
            hist[r as usize] += 1;
        }
    }
    eprintln!("    row48 return-code histogram: {hist:?}");
    for code in 0..8 {
        assert!(
            hist[code] > 0,
            "gen_ray return code {code} was never produced (histogram {hist:?})"
        );
    }
    d.finish();
}

/// Row 49 — `mp == ray.p` ⇒ `c2Norm` of a zero vector ⇒ NaN direction and NaN
/// `ray.t` flowing through all three shapes.
#[test]
fn cfg_49_gen_ray_degenerate_ray() {
    let mut rng = Rng::new(SEED ^ 49);
    let mut d = Diff::new("row49 gen_ray degenerate ray");
    for _ in 0..N {
        let mut p = rand_params(&mut rng, 40.0);
        // Exactly coincident mouse point and ray origin.
        p.mp_x = p.r_p_x;
        p.mp_y = p.r_p_y;
        check(&mut d, &p);
        // Signed-zero variants of the same coincidence.
        let mut q = p;
        q.mp_x = -p.r_p_x * 0.0;
        q.r_p_x = 0.0;
        q.mp_y = 0.0;
        q.r_p_y = -0.0;
        check(&mut d, &q);
        // One component coincident only (direction has a zero component).
        let mut s = rand_params(&mut rng, 40.0);
        s.mp_y = s.r_p_y;
        check(&mut d, &s);
        let mut u = rand_params(&mut rng, 40.0);
        u.mp_x = u.r_p_x;
        check(&mut d, &u);
    }
    d.finish();
}

/// Row 50 — every special float class injected into each of the 16 parameters
/// in turn, on top of an otherwise-hitting configuration.
#[test]
fn cfg_50_gen_ray_special_field_sweep() {
    let mut rng = Rng::new(SEED ^ 50);
    let mut d = Diff::new("row50 gen_ray special field sweep");

    let mut inject: Vec<f32> = SPECIALS.to_vec();
    inject.extend(NAN_BITS.iter().map(|&b| f32::from_bits(b)));

    // Several distinct base configurations so the injection lands on different
    // code paths.
    let bases: Vec<Params> = (0..12).map(|_| rand_params(&mut rng, 40.0)).collect();
    for base in &bases {
        for &s in &inject {
            for field in 0..16 {
                let mut p = *base;
                p.set(field, s);
                check(&mut d, &p);
            }
            // Two fields at once, to catch interaction (e.g. NaN in both the
            // capsule endpoints, or both box corners).
            for (f1, f2) in [(0, 2), (1, 3), (7, 9), (8, 10), (12, 14), (13, 15), (6, 11)] {
                let mut p = *base;
                p.set(f1, s);
                p.set(f2, s);
                check(&mut d, &p);
            }
        }
    }
    // Fully arbitrary bit patterns in all 16 parameters.
    for _ in 0..N * 2 {
        let mut p = Params::default();
        for i in 0..16 {
            p.set(i, rng.spicy());
        }
        check(&mut d, &p);
    }
    d.finish();
}

/// Row 51 — degenerate shape combinations: inverted box, zero-radius circle,
/// zero-radius capsule, zero-length capsule, all together.
#[test]
fn cfg_51_gen_ray_degenerate_shapes() {
    let mut rng = Rng::new(SEED ^ 51);
    let mut d = Diff::new("row51 gen_ray degenerate shapes");

    for _ in 0..N {
        let base = rand_params(&mut rng, 40.0);

        // Inverted bb.
        let mut p = base;
        std::mem::swap(&mut p.bb_min_x, &mut p.bb_max_x);
        std::mem::swap(&mut p.bb_min_y, &mut p.bb_max_y);
        check(&mut d, &p);

        // Degenerate bb (point / segment).
        let mut p = base;
        p.bb_max_x = p.bb_min_x;
        p.bb_max_y = p.bb_min_y;
        check(&mut d, &p);
        let mut p = base;
        p.bb_max_y = p.bb_min_y;
        check(&mut d, &p);

        // Zero and negative radii.
        for r in [0.0f32, -0.0, -1.0, 1e-45] {
            let mut p = base;
            p.c_r = r;
            check(&mut d, &p);
            let mut p = base;
            p.cap_r = r;
            check(&mut d, &p);
        }

        // Zero-length capsule axis.
        let mut p = base;
        p.cap_b_x = p.cap_a_x;
        p.cap_b_y = p.cap_a_y;
        check(&mut d, &p);

        // Everything degenerate at once.
        let mut p = base;
        p.c_r = 0.0;
        p.cap_r = 0.0;
        p.cap_b_x = p.cap_a_x;
        p.cap_b_y = p.cap_a_y;
        p.bb_max_x = p.bb_min_x;
        p.bb_max_y = p.bb_min_y;
        check(&mut d, &p);
    }
    d.finish();
}

/// Row 52 — extreme coordinate magnitudes so intermediate dot products overflow
/// to ±inf and underflow to 0 inside the pipeline.
#[test]
fn cfg_52_gen_ray_extreme_magnitudes() {
    let mut rng = Rng::new(SEED ^ 52);
    let mut d = Diff::new("row52 gen_ray extreme magnitudes");

    for _ in 0..N {
        // Log-uniform magnitudes across the whole f32 range.
        let mut p = Params::default();
        for i in 0..16 {
            p.set(i, rng.wide());
        }
        check(&mut d, &p);

        // Mixed: huge ray, tiny shapes and vice versa.
        let mut q = rand_params(&mut rng, 40.0);
        q.r_p_x *= 1e30;
        q.r_p_y *= 1e30;
        q.mp_x *= 1e30;
        q.mp_y *= 1e30;
        check(&mut d, &q);

        let mut s = rand_params(&mut rng, 40.0);
        s.c_p_x *= 1e-30;
        s.c_p_y *= 1e-30;
        s.c_r *= 1e-30;
        s.cap_a_x *= 1e-30;
        s.cap_a_y *= 1e-30;
        s.cap_b_x *= 1e-30;
        s.cap_b_y *= 1e-30;
        s.cap_r *= 1e-30;
        check(&mut d, &s);

        // Values that make `c2Len` overflow (dot product > f32::MAX).
        let mut u = rand_params(&mut rng, 40.0);
        u.r_p_x = 3.0e38;
        u.mp_x = -3.0e38;
        check(&mut d, &u);
    }
    d.finish();
}

/// Row 53 — struct-ABI parity sweep. Every export is called with recognisable
/// sentinel bit patterns in every field of every by-value struct, so a
/// misclassified parameter (SSE vs MEMORY, or a wrong eightbyte split) shows up
/// as a garbled result rather than passing by luck on plausible floats.
#[test]
fn cfg_53_struct_abi_parity() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 53);
    let mut d = Diff::new("row53 struct ABI parity");

    // Distinct, easily recognisable values per field position.
    let sentinels: [f32; 8] = [
        f32::from_bits(0x4111_1111),
        f32::from_bits(0x4222_2222),
        f32::from_bits(0x4333_3333),
        f32::from_bits(0x4444_4444),
        f32::from_bits(0x4555_5555),
        f32::from_bits(0x4666_6666),
        f32::from_bits(0x4777_7777),
        f32::from_bits(0x4888_8888),
    ];

    let mut rounds: Vec<[f32; 8]> = vec![sentinels];
    // Rotations of the sentinels so each value visits each field position.
    for shift in 1..8 {
        let mut r = [0f32; 8];
        for i in 0..8 {
            r[i] = sentinels[(i + shift) % 8];
        }
        rounds.push(r);
    }
    for _ in 0..3000 {
        let mut r = [0f32; 8];
        for i in 0..8 {
            r[i] = rng.any_bits();
        }
        rounds.push(r);
    }

    for s in rounds {
        let a = c2v { x: s[0], y: s[1] };
        let b = c2v { x: s[2], y: s[3] };
        let m = c2m { x: a, y: b };
        let ray = c2Ray { p: a, d: b, t: s[4] };
        let circ = c2Circle { p: c2v { x: s[5], y: s[6] }, r: s[7] };
        let box_ = c2AABB { min: c2v { x: s[4], y: s[5] }, max: c2v { x: s[6], y: s[7] } };
        let cap = c2Capsule {
            a: c2v { x: s[4], y: s[5] },
            b: c2v { x: s[6], y: s[7] },
            r: s[0],
        };

        // 8-byte SSE struct in, 8-byte SSE struct out.
        d.check_v(unsafe { (l.c.c2V)(s[0], s[1]) }, unsafe { (l.r.c2V)(s[0], s[1]) }, || "abi c2V".into());
        for (name, f_c, f_r) in [
            ("c2Skew", l.c.c2Skew, l.r.c2Skew),
            ("c2Absv", l.c.c2Absv, l.r.c2Absv),
            ("c2CCW90", l.c.c2CCW90, l.r.c2CCW90),
            ("c2Norm", l.c.c2Norm, l.r.c2Norm),
        ] {
            d.check_v(unsafe { f_c(a) }, unsafe { f_r(a) }, || format!("abi {name}({})", fmt_v(a)));
        }
        for (name, f_c, f_r) in [
            ("c2Add", l.c.c2Add, l.r.c2Add),
            ("c2Sub", l.c.c2Sub, l.r.c2Sub),
            ("c2Minv", l.c.c2Minv, l.r.c2Minv),
            ("c2Maxv", l.c.c2Maxv, l.r.c2Maxv),
        ] {
            d.check_v(unsafe { f_c(a, b) }, unsafe { f_r(a, b) }, || {
                format!("abi {name}({}, {})", fmt_v(a), fmt_v(b))
            });
        }
        d.check_f(unsafe { (l.c.c2Dot)(a, b) }, unsafe { (l.r.c2Dot)(a, b) }, || "abi c2Dot".into());
        d.check_f(unsafe { (l.c.c2Len)(a) }, unsafe { (l.r.c2Len)(a) }, || "abi c2Len".into());
        d.check_v(unsafe { (l.c.c2Mulvs)(a, s[4]) }, unsafe { (l.r.c2Mulvs)(a, s[4]) }, || "abi c2Mulvs".into());
        d.check_v(unsafe { (l.c.c2Div)(a, s[4]) }, unsafe { (l.r.c2Div)(a, s[4]) }, || "abi c2Div".into());
        // 16-byte all-float struct (xmm0+xmm1).
        d.check_v(unsafe { (l.c.c2MulmvT)(m, a) }, unsafe { (l.r.c2MulmvT)(m, a) }, || "abi c2MulmvT".into());
        d.check_i(
            unsafe { (l.c.c2AABBtoAABB)(box_, box_) },
            unsafe { (l.r.c2AABBtoAABB)(box_, box_) },
            || "abi c2AABBtoAABB".into(),
        );
        d.check_i(
            unsafe { (l.c.c2AABBtoPoint)(box_, a) },
            unsafe { (l.r.c2AABBtoPoint)(box_, a) },
            || "abi c2AABBtoPoint".into(),
        );
        // 12-byte all-float struct (xmm0+xmm1, second eightbyte half-used).
        d.check_i(
            unsafe { (l.c.c2CircleToPoint)(circ, a) },
            unsafe { (l.r.c2CircleToPoint)(circ, a) },
            || "abi c2CircleToPoint".into(),
        );
        // 20-byte MEMORY-class structs passed on the stack.
        let (cr, co, rr, ro) = both_ray(|lib, r, sh, o| unsafe { (lib.c2RaytoCircle)(r, sh, o) }, ray, circ);
        d.check_ray(cr, co, rr, ro, || "abi c2RaytoCircle".into());
        let (cr, co, rr, ro) = both_ray(|lib, r, sh, o| unsafe { (lib.c2RaytoAABB)(r, sh, o) }, ray, box_);
        d.check_ray(cr, co, rr, ro, || "abi c2RaytoAABB".into());
        let (cr, co, rr, ro) = both_ray(|lib, r, sh, o| unsafe { (lib.c2RaytoCapsule)(r, sh, o) }, ray, cap);
        d.check_ray(cr, co, rr, ro, || "abi c2RaytoCapsule".into());
        // 19-argument mixed stack/register call.
        let mut p = Params::default();
        for i in 0..16 {
            p.set(i, s[i % 8]);
        }
        check(&mut d, &p);
    }
    d.finish();
}
