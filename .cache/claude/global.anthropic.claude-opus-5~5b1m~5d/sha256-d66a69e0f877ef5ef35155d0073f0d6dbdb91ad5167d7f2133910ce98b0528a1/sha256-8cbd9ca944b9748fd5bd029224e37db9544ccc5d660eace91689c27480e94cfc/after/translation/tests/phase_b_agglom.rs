//! Phase B — valid-path differential tests for `agglom`, the aggregate entry
//! point declared in `c_src/include/lib.h`.
//!
//! Covers `CONFIGS.md` rows 88–94. The return value is compared BITWISE as an
//! `f64`, so every rounding step of the accumulator must agree.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::os::raw::c_int;

/// The 33 `agglom` arguments, in declaration order.
#[derive(Copy, Clone)]
struct Args {
    f2_1: f32,
    f2_2: f32,
    f2_3: f32,
    f2_7: f32,
    f2_8: f32,
    f2_9: f32,
    f2_10: f32,
    f3_1: c_int,
    f3_2: c_int,
    f4_1: u64,
    f4_2: u64,
    f5_1: u32,
    f7_1: u32,
    f7_2: u32,
    f7_3: u32,
    f9_1: f32,
    f9_2: f32,
    f9_4: f32,
    f9_5: f32,
    f9_7: f32,
    f9_8: f32,
    f9_10: f32,
    f9_11: f32,
    f10_1: u16,
    f11_2: f32,
    f11_3: f32,
    f11_4: f32,
    f12_2: f32,
    f12_3: f32,
    f12_4: f32,
    f13_2: f32,
    f13_3: f32,
    f13_4: f32,
}

impl std::fmt::Debug for Args {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Print floats as bit patterns so a failure is exactly reproducible.
        let fl = |v: f32| format!("f32::from_bits(0x{:08x})", v.to_bits());
        write!(
            f,
            "Args {{ f2: [{}, {}, {}, {}, {}, {}, {}], f3: [{}, {}], \
             f4: [{:#x}, {:#x}], f5: {:#x}, f7: [{}, {}, {}], \
             f9: [{}, {}, {}, {}, {}, {}, {}, {}], f10: {:#x}, \
             f11: [{}, {}, {}], f12: [{}, {}, {}], f13: [{}, {}, {}] }}",
            fl(self.f2_1),
            fl(self.f2_2),
            fl(self.f2_3),
            fl(self.f2_7),
            fl(self.f2_8),
            fl(self.f2_9),
            fl(self.f2_10),
            self.f3_1,
            self.f3_2,
            self.f4_1,
            self.f4_2,
            self.f5_1,
            self.f7_1,
            self.f7_2,
            self.f7_3,
            fl(self.f9_1),
            fl(self.f9_2),
            fl(self.f9_4),
            fl(self.f9_5),
            fl(self.f9_7),
            fl(self.f9_8),
            fl(self.f9_10),
            fl(self.f9_11),
            self.f10_1,
            fl(self.f11_2),
            fl(self.f11_3),
            fl(self.f11_4),
            fl(self.f12_2),
            fl(self.f12_3),
            fl(self.f12_4),
            fl(self.f13_2),
            fl(self.f13_3),
            fl(self.f13_4),
        )
    }
}

impl Args {
    fn zeros() -> Args {
        Args {
            f2_1: 0.0,
            f2_2: 0.0,
            f2_3: 0.0,
            f2_7: 0.0,
            f2_8: 0.0,
            f2_9: 0.0,
            f2_10: 0.0,
            f3_1: 0,
            f3_2: 0,
            f4_1: 0,
            f4_2: 0,
            f5_1: 0,
            f7_1: 0,
            f7_2: 0,
            f7_3: 0,
            f9_1: 0.0,
            f9_2: 0.0,
            f9_4: 0.0,
            f9_5: 0.0,
            f9_7: 0.0,
            f9_8: 0.0,
            f9_10: 0.0,
            f9_11: 0.0,
            f10_1: 0,
            f11_2: 0.0,
            f11_3: 0.0,
            f11_4: 0.0,
            f12_2: 0.0,
            f12_3: 0.0,
            f12_4: 0.0,
            f13_2: 0.0,
            f13_3: 0.0,
            f13_4: 0.0,
        }
    }
}

#[track_caller]
fn diff_agglom(tag: &str, x: Args) {
    let a = apis();
    let call = |f: FnAgglom| unsafe {
        f(
            x.f2_1, x.f2_2, x.f2_3, x.f2_7, x.f2_8, x.f2_9, x.f2_10, x.f3_1, x.f3_2, x.f4_1,
            x.f4_2, x.f5_1, x.f7_1, x.f7_2, x.f7_3, x.f9_1, x.f9_2, x.f9_4, x.f9_5, x.f9_7,
            x.f9_8, x.f9_10, x.f9_11, x.f10_1, x.f11_2, x.f11_3, x.f11_4, x.f12_2, x.f12_3,
            x.f12_4, x.f13_2, x.f13_3, x.f13_4,
        )
    };
    eq_f64(tag, &x, call(a.c.agglom), call(a.r.agglom));
}

/// Row 88 — all 33 arguments as fully random bit patterns.
#[test]
fn cfg88_agglom_random_bits() {
    let mut rng = Rng::new();
    for _ in 0..200_000 {
        let x = Args {
            f2_1: rng.any_f32(),
            f2_2: rng.any_f32(),
            f2_3: rng.any_f32(),
            f2_7: rng.any_f32(),
            f2_8: rng.any_f32(),
            f2_9: rng.any_f32(),
            f2_10: rng.any_f32(),
            f3_1: rng.next_i32(),
            f3_2: rng.next_i32(),
            f4_1: rng.next_u64(),
            f4_2: rng.next_u64(),
            f5_1: rng.next_u32(),
            f7_1: rng.next_u32(),
            f7_2: rng.next_u32(),
            f7_3: rng.next_u32(),
            f9_1: rng.any_f32(),
            f9_2: rng.any_f32(),
            f9_4: rng.any_f32(),
            f9_5: rng.any_f32(),
            f9_7: rng.any_f32(),
            f9_8: rng.any_f32(),
            f9_10: rng.any_f32(),
            f9_11: rng.any_f32(),
            f10_1: rng.next_u16(),
            f11_2: rng.any_f32(),
            f11_3: rng.any_f32(),
            f11_4: rng.any_f32(),
            f12_2: rng.any_f32(),
            f12_3: rng.any_f32(),
            f12_4: rng.any_f32(),
            f13_2: rng.any_f32(),
            f13_3: rng.any_f32(),
            f13_4: rng.any_f32(),
        };
        diff_agglom("agglom/bits", x);
    }
}

/// Row 89 — all-zero arguments.
#[test]
fn cfg89_agglom_zeros() {
    diff_agglom("agglom/zeros", Args::zeros());
    // and all -0.0 for the floats
    let mut x = Args::zeros();
    x.f2_1 = -0.0;
    x.f2_2 = -0.0;
    x.f2_3 = -0.0;
    x.f2_7 = -0.0;
    x.f2_8 = -0.0;
    x.f2_9 = -0.0;
    x.f2_10 = -0.0;
    x.f9_1 = -0.0;
    x.f9_2 = -0.0;
    x.f9_4 = -0.0;
    x.f9_5 = -0.0;
    x.f9_7 = -0.0;
    x.f9_8 = -0.0;
    x.f9_10 = -0.0;
    x.f9_11 = -0.0;
    x.f11_2 = -0.0;
    x.f11_3 = -0.0;
    x.f11_4 = -0.0;
    x.f12_2 = -0.0;
    x.f12_3 = -0.0;
    x.f12_4 = -0.0;
    x.f13_2 = -0.0;
    x.f13_3 = -0.0;
    x.f13_4 = -0.0;
    diff_agglom("agglom/negzeros", x);
}

/// Row 90 — "sane" arguments, the way a real consumer would call it.
#[test]
fn cfg90_agglom_sane() {
    let mut rng = Rng::new();
    let unit = |rng: &mut Rng| (rng.next_u32() as f64 / u32::MAX as f64) as f32;
    let hue = |rng: &mut Rng| (rng.next_u32() as f64 / u32::MAX as f64 * 360.0) as f32;
    for _ in 0..200_000 {
        let x = Args {
            f2_1: rng.finite_f32(20.0),
            f2_2: rng.finite_f32(20.0),
            f2_3: rng.finite_f32(10.0).abs(),
            f2_7: rng.finite_f32(20.0),
            f2_8: rng.finite_f32(20.0),
            f2_9: rng.finite_f32(20.0),
            f2_10: rng.finite_f32(20.0),
            f3_1: rng.next_i32() % 1_000_000,
            f3_2: {
                let d = rng.next_i32() % 1000;
                if d == 0 {
                    7
                } else {
                    d
                }
            },
            f4_1: rng.next_u64(),
            f4_2: rng.next_u64(),
            f5_1: rng.below(0x1_0000),
            f7_1: rng.below(65536),
            f7_2: rng.below(9),
            f7_3: rng.pick(&[8u32, 12, 16, 20, 24, 32]),
            f9_1: rng.finite_f32(10.0),
            f9_2: rng.finite_f32(10.0),
            f9_4: rng.finite_f32(10.0),
            f9_5: rng.finite_f32(10.0),
            f9_7: rng.finite_f32(10.0),
            f9_8: rng.finite_f32(10.0),
            f9_10: rng.finite_f32(10.0),
            f9_11: rng.finite_f32(10.0),
            f10_1: rng.next_u16(),
            f11_2: hue(&mut rng),
            f11_3: unit(&mut rng),
            f11_4: unit(&mut rng),
            f12_2: hue(&mut rng),
            f12_3: unit(&mut rng),
            f12_4: unit(&mut rng),
            f13_2: unit(&mut rng),
            f13_3: unit(&mut rng),
            f13_4: unit(&mut rng),
        };
        diff_agglom("agglom/sane", x);
    }
}

/// Row 91 — arguments that drive each `isnan` guard in `agglom`.
#[test]
fn cfg91_agglom_nan_guards() {
    let mut rng = Rng::new();
    // f10_1 values whose half-float decode is NaN or Inf.
    let half_nan: &[u16] = &[
        0x7C00, 0x7C01, 0x7E00, 0x7FFF, 0xFC00, 0xFE00, 0xFFFF, 0x0000, 0x8000, 0x0001, 0x8001,
        0x3C00, 0xBC00,
    ];
    for &h in half_nan {
        let mut x = Args::zeros();
        x.f10_1 = h;
        diff_agglom("agglom/f10-nan", x);
        // combined with a degenerate f9 (all points equal -> u,v = NaN)
        x.f9_1 = 1.0;
        x.f9_2 = 2.0;
        x.f9_4 = 1.0;
        x.f9_5 = 2.0;
        x.f9_7 = 1.0;
        x.f9_8 = 2.0;
        x.f9_10 = 1.0;
        x.f9_11 = 2.0;
        diff_agglom("agglom/f10-nan+f9-degen", x);
    }
    // f11/f12/f13 NaN-producing inputs, one subsystem at a time.
    for &nb in NAN_ZOO {
        let n = f32::from_bits(nb);
        for slot in 0..9 {
            let mut x = Args::zeros();
            x.f11_3 = 0.5; // keep f11 out of the s==0 early-out
            x.f12_3 = 0.5;
            match slot {
                0 => x.f11_2 = n,
                1 => x.f11_3 = n,
                2 => x.f11_4 = n,
                3 => x.f12_2 = n,
                4 => x.f12_3 = n,
                5 => x.f12_4 = n,
                6 => x.f13_2 = n,
                7 => x.f13_3 = n,
                _ => x.f13_4 = n,
            }
            diff_agglom("agglom/nan-slot", x);
        }
        // every float argument set to the same NaN at once
        let mut x = Args::zeros();
        for f in [
            &mut x.f2_1,
            &mut x.f2_2,
            &mut x.f2_3,
            &mut x.f2_7,
            &mut x.f2_8,
            &mut x.f2_9,
            &mut x.f2_10,
            &mut x.f9_1,
            &mut x.f9_2,
            &mut x.f9_4,
            &mut x.f9_5,
            &mut x.f9_7,
            &mut x.f9_8,
            &mut x.f9_10,
            &mut x.f9_11,
            &mut x.f11_2,
            &mut x.f11_3,
            &mut x.f11_4,
            &mut x.f12_2,
            &mut x.f12_3,
            &mut x.f12_4,
            &mut x.f13_2,
            &mut x.f13_3,
            &mut x.f13_4,
        ] {
            *f = n;
        }
        diff_agglom("agglom/all-nan", x);
    }
    // Randomized: half the float args NaN/inf, the rest sane.
    for _ in 0..50_000 {
        let zoo = zoo_f32();
        let pickf = |rng: &mut Rng| {
            if rng.next_u32() % 2 == 0 {
                rng.pick(&zoo)
            } else {
                rng.finite_f32(100.0)
            }
        };
        let x = Args {
            f2_1: pickf(&mut rng),
            f2_2: pickf(&mut rng),
            f2_3: pickf(&mut rng),
            f2_7: pickf(&mut rng),
            f2_8: pickf(&mut rng),
            f2_9: pickf(&mut rng),
            f2_10: pickf(&mut rng),
            f3_1: rng.next_i32(),
            f3_2: rng.next_i32(),
            f4_1: rng.next_u64(),
            f4_2: rng.next_u64(),
            f5_1: rng.next_u32(),
            f7_1: rng.next_u32(),
            f7_2: rng.below(5),
            f7_3: rng.pick(&[8u32, 16, 32, 33]),
            f9_1: pickf(&mut rng),
            f9_2: pickf(&mut rng),
            f9_4: pickf(&mut rng),
            f9_5: pickf(&mut rng),
            f9_7: pickf(&mut rng),
            f9_8: pickf(&mut rng),
            f9_10: pickf(&mut rng),
            f9_11: pickf(&mut rng),
            f10_1: rng.next_u16(),
            f11_2: pickf(&mut rng),
            f11_3: pickf(&mut rng),
            f11_4: pickf(&mut rng),
            f12_2: pickf(&mut rng),
            f12_3: pickf(&mut rng),
            f12_4: pickf(&mut rng),
            f13_2: pickf(&mut rng),
            f13_3: pickf(&mut rng),
            f13_4: pickf(&mut rng),
        };
        diff_agglom("agglom/mixed-zoo", x);
    }
}

/// Row 92 — `±inf` contributions, which the `isnan` guards do NOT skip.
#[test]
fn cfg92_agglom_infinities() {
    for &(p, q) in &[
        (f32::INFINITY, f32::INFINITY),
        (f32::INFINITY, f32::NEG_INFINITY),
        (f32::NEG_INFINITY, f32::INFINITY),
        (f32::NEG_INFINITY, f32::NEG_INFINITY),
        (f32::MAX, f32::MAX),
        (f32::MIN, f32::MAX),
    ] {
        let mut x = Args::zeros();
        x.f11_3 = 0.5;
        x.f12_3 = 0.5;
        x.f11_4 = p;
        x.f12_4 = q;
        x.f13_2 = p;
        x.f13_3 = q;
        x.f13_4 = 1.0;
        diff_agglom("agglom/inf", x);

        // f9 with an infinite coordinate -> inf or NaN barycentrics
        let mut y = Args::zeros();
        y.f9_1 = p;
        y.f9_5 = q;
        y.f9_7 = 1.0;
        y.f9_11 = 2.0;
        diff_agglom("agglom/inf-f9", y);
    }
}

/// Row 93 — integer arguments at their extremes.
#[test]
fn cfg93_agglom_integer_extremes() {
    let mut rng = Rng::new();
    for &i1 in SPECIAL_I32 {
        for &i2 in SPECIAL_I32 {
            let mut x = Args::zeros();
            x.f3_1 = i1;
            x.f3_2 = i2;
            x.f5_1 = u32::MAX;
            x.f7_1 = u32::MAX;
            x.f7_2 = 2;
            x.f7_3 = 32;
            x.f4_1 = u64::MAX;
            x.f4_2 = u64::MAX;
            x.f10_1 = u16::MAX;
            diff_agglom("agglom/int-extremes", x);
        }
    }
    for &u1 in SPECIAL_U32 {
        for &u2 in SPECIAL_U32 {
            let mut x = Args::zeros();
            x.f5_1 = u1;
            x.f7_1 = u2;
            x.f7_2 = u1;
            x.f7_3 = u2;
            diff_agglom("agglom/u32-extremes", x);
        }
    }
    for &s0 in SPECIAL_U64 {
        for &s1 in SPECIAL_U64 {
            let mut x = Args::zeros();
            x.f4_1 = s0;
            x.f4_2 = s1;
            diff_agglom("agglom/u64-extremes", x);
        }
    }
    // f10_1 exhaustive through agglom (cheap: 65536 calls).
    for h in 0u16..=u16::MAX {
        let mut x = Args::zeros();
        x.f10_1 = h;
        diff_agglom("agglom/f10-sweep", x);
        if h == u16::MAX {
            break;
        }
    }
    // Random integer args with sane floats.
    for _ in 0..100_000 {
        let mut x = Args::zeros();
        x.f3_1 = rng.next_i32();
        x.f3_2 = rng.next_i32();
        x.f4_1 = rng.next_u64();
        x.f4_2 = rng.next_u64();
        x.f5_1 = rng.next_u32();
        x.f7_1 = rng.next_u32();
        x.f7_2 = rng.next_u32();
        x.f7_3 = rng.next_u32();
        x.f10_1 = rng.next_u16();
        diff_agglom("agglom/int-rand", x);
    }
}

/// Row 94 — sweep the `f11` sectors x `f12` `i` values x `f13` max-channels.
#[test]
fn cfg94_agglom_sector_sweep() {
    let mut rng = Rng::new();
    let hues: &[f32] = &[
        -30.0, -0.0, 0.0, 30.0, 60.0, 90.0, 120.0, 150.0, 180.0, 210.0, 240.0, 270.0, 300.0,
        330.0, 359.9, 360.0, 400.0,
    ];
    for &h11 in hues {
        for &h12 in hues {
            for maxch in 0..3 {
                let mut x = Args::zeros();
                x.f11_2 = h11;
                x.f11_3 = 0.75;
                x.f11_4 = 0.4;
                x.f12_2 = h12;
                x.f12_3 = 0.6;
                x.f12_4 = 0.9;
                let (r, g, b) = match maxch {
                    0 => (0.9f32, 0.2, 0.5),
                    1 => (0.2f32, 0.9, 0.5),
                    _ => (0.2f32, 0.5, 0.9),
                };
                x.f13_2 = r;
                x.f13_3 = g;
                x.f13_4 = b;
                // vary the collision + f9 inputs too so the whole aggregate moves
                x.f2_1 = rng.finite_f32(20.0);
                x.f2_2 = rng.finite_f32(20.0);
                x.f2_3 = rng.finite_f32(10.0).abs();
                x.f2_7 = -5.0;
                x.f2_8 = -5.0;
                x.f2_9 = 5.0;
                x.f2_10 = 5.0;
                x.f9_1 = 0.0;
                x.f9_2 = 0.0;
                x.f9_4 = 1.0;
                x.f9_5 = 0.0;
                x.f9_7 = 0.0;
                x.f9_8 = 1.0;
                x.f9_10 = rng.finite_f32(2.0);
                x.f9_11 = rng.finite_f32(2.0);
                x.f3_1 = rng.next_i32() % 10_000;
                x.f3_2 = 7;
                x.f5_1 = rng.next_u32();
                x.f7_1 = rng.below(70000);
                x.f7_2 = rng.below(5);
                x.f7_3 = rng.pick(&[8u32, 16, 24, 32]);
                x.f4_1 = rng.next_u64();
                x.f4_2 = rng.next_u64();
                x.f10_1 = rng.next_u16();
                diff_agglom("agglom/sweep", x);
            }
        }
    }
}
