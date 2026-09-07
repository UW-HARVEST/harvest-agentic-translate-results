//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every test loads both `.so` files with `libloading` and compares raw `f32`
//! bit patterns. `ldexp_q2` is the lowest-level (and only) public entry point,
//! so there is no wrapper-vs-primitive split to worry about; row 23 chains
//! calls the way a real consumer composes the operation.

mod common;
use common::{check, exp_classes, exp_huge_positive, pair, y_all, Rng, Y_BIT_CLASSES, Y_CLASSES};

/// Randomized `y` count per `exp_q2` value in the sweep rows.
const YS_PER_EXP: usize = 64;

// ---------------------------------------------------------------- row 1
#[test]
fn cfg01_exp_zero_random_normal_y() {
    let mut rng = Rng::new(0x0000_0001);
    check(1.0, 0);
    check(-1.0, 0);
    for _ in 0..20_000 {
        check(rng.next_f32_normal(), 0);
    }
    // `g_expfrac[0] * 2^30` is exactly 1.0f, so this must be the identity even
    // for -0.0 and NaN payloads.
    for &b in Y_BIT_CLASSES {
        check(f32::from_bits(b), 0);
    }
}

// ---------------------------------------------------------------- row 2
#[test]
fn cfg02_exp_1_to_3_all_residues() {
    let mut rng = Rng::new(0x0000_0002);
    for e in 1..=3 {
        for _ in 0..YS_PER_EXP * 20 {
            check(rng.next_f32_normal(), e);
        }
        for y in y_all() {
            check(y, e);
        }
    }
}

// ---------------------------------------------------------------- row 3
#[test]
fn cfg03_exp_4_to_119_exhaustive() {
    let mut rng = Rng::new(0x0000_0003);
    for e in 4..=119 {
        for _ in 0..YS_PER_EXP {
            check(rng.next_f32_normal(), e);
        }
        for _ in 0..YS_PER_EXP {
            check(rng.next_f32_bits(), e);
        }
        for y in y_all() {
            check(y, e);
        }
    }
}

// ---------------------------------------------------------------- row 4
#[test]
fn cfg04_exp_exactly_120_clamp() {
    let mut rng = Rng::new(0x0000_0004);
    for _ in 0..20_000 {
        check(rng.next_f32_bits(), 120);
    }
    for y in y_all() {
        check(y, 120);
    }
}

// ---------------------------------------------------------------- row 5
#[test]
fn cfg05_exp_121_to_240_two_iterations() {
    let mut rng = Rng::new(0x0000_0005);
    for e in 121..=240 {
        for _ in 0..YS_PER_EXP {
            check(rng.next_f32_bits(), e);
        }
        for y in y_all() {
            check(y, e);
        }
    }
}

// ---------------------------------------------------------------- row 6
#[test]
fn cfg06_exp_241_to_1200_multi_iteration() {
    let mut rng = Rng::new(0x0000_0006);
    for e in 241..=1200 {
        for _ in 0..8 {
            check(rng.next_f32_bits(), e);
        }
    }
    for e in [241, 242, 360, 361, 480, 481, 600, 720, 840, 960, 1080, 1200] {
        for y in y_all() {
            check(y, e);
        }
    }
}

// ---------------------------------------------------------------- row 7
#[test]
fn cfg07_exp_large_positive_saturation() {
    let mut rng = Rng::new(0x0000_0007);
    for _ in 0..400 {
        let e = rng.in_range(10_000, 200_000);
        check(rng.next_f32_bits(), e);
        check(rng.next_f32_normal(), e);
    }
    for e in [10_000, 12_000, 99_999, 100_000, 200_000] {
        for y in y_all() {
            check(y, e);
        }
    }
}

// ---------------------------------------------------------------- row 8
#[test]
fn cfg08_exp_minus1_to_minus4_zero_factor() {
    // (e >> 2) & 31 == 31  =>  integer factor is exactly 0.
    for e in [-1, -2, -3, -4] {
        for y in y_all() {
            check(y, e);
        }
        let mut rng = Rng::new(0x0000_0008 ^ (e as u64));
        for _ in 0..20_000 {
            check(rng.next_f32_bits(), e);
        }
    }
}

// ---------------------------------------------------------------- row 9
#[test]
fn cfg09_exp_minus5_to_minus128_exhaustive() {
    let mut rng = Rng::new(0x0000_0009);
    for e in -128..=-5 {
        for _ in 0..YS_PER_EXP {
            check(rng.next_f32_bits(), e);
        }
        for y in y_all() {
            check(y, e);
        }
    }
}

// ---------------------------------------------------------------- row 10
#[test]
fn cfg10_exp_minus129_to_minus512_aliasing() {
    let mut rng = Rng::new(0x0000_000a);
    for e in -512..=-129 {
        for _ in 0..YS_PER_EXP {
            check(rng.next_f32_bits(), e);
        }
        for y in y_all() {
            check(y, e);
        }
    }
}

// ---------------------------------------------------------------- row 11
#[test]
fn cfg11_exp_large_negative() {
    let mut rng = Rng::new(0x0000_000b);
    for _ in 0..40_000 {
        let e = rng.in_range(-200_000, -10_000);
        check(rng.next_f32_bits(), e);
    }
    for e in [-10_000, -12_345, -65_536, -100_000, -199_999, -200_000] {
        for y in y_all() {
            check(y, e);
        }
    }
}

// ---------------------------------------------------------------- row 12
#[test]
fn cfg12_random_i32_excluding_slow_positive_tail() {
    let mut rng = Rng::new(0x0000_000c);
    let mut n = 0usize;
    while n < 10_000 {
        let e = rng.next_i32();
        // Skip only the region whose loop count would dominate the runtime.
        if e > 500_000 {
            continue;
        }
        check(rng.next_f32_bits(), e);
        n += 1;
    }
}

// ---------------------------------------------------------------- row 13
#[test]
fn cfg13_random_nonpositive_i32() {
    let mut rng = Rng::new(0x0000_000d);
    for _ in 0..10_000 {
        let e = rng.in_range(i32::MIN, 0);
        check(rng.next_f32_bits(), e);
        check(rng.next_f32_normal(), e);
    }
}

// ---------------------------------------------------------------- row 14
#[test]
fn cfg14_signed_zero_across_all_exp_classes() {
    for e in exp_classes() {
        check(0.0, e);
        check(-0.0, e);
    }
    for e in exp_huge_positive() {
        check(0.0, e);
        check(-0.0, e);
    }
}

// ---------------------------------------------------------------- row 15
#[test]
fn cfg15_infinities_across_all_exp_classes() {
    for e in exp_classes() {
        check(f32::INFINITY, e);
        check(f32::NEG_INFINITY, e);
    }
    for e in exp_huge_positive() {
        check(f32::INFINITY, e);
        check(f32::NEG_INFINITY, e);
    }
}

// ---------------------------------------------------------------- row 16
#[test]
fn cfg16_nan_payloads_across_all_exp_classes() {
    let nans = [
        0x7fc0_0000u32,
        0xffc0_0000,
        0x7fc0_dead,
        0xffc0_dead,
        0x7f80_0001,
        0xff80_0001,
        0x7fff_ffff,
    ];
    for e in exp_classes() {
        for &b in &nans {
            check(f32::from_bits(b), e);
        }
    }
    for e in exp_huge_positive() {
        for &b in &nans {
            check(f32::from_bits(b), e);
        }
    }
}

// ---------------------------------------------------------------- row 17
#[test]
fn cfg17_subnormal_y() {
    let subs: Vec<f32> = (0..24)
        .map(|k| f32::from_bits(1u32 << k))
        .chain((0..24).map(|k| f32::from_bits(0x8000_0000 | (1u32 << k))))
        .chain([0x007f_ffff, 0x807f_ffff, 0x0000_0001, 0x8000_0001].map(f32::from_bits))
        .collect();
    for y in &subs {
        for e in exp_classes() {
            check(*y, e);
        }
    }
    let mut rng = Rng::new(0x0000_0011);
    for _ in 0..20_000 {
        // random subnormal
        let y = f32::from_bits((rng.next_u32() & 0x807f_ffff) | 0);
        check(y, rng.in_range(-4096, 4096));
    }
}

// ---------------------------------------------------------------- row 18
#[test]
fn cfg18_flt_max_overflow() {
    for y in [f32::MAX, f32::MIN] {
        for e in exp_classes() {
            check(y, e);
        }
        for e in exp_huge_positive() {
            check(y, e);
        }
    }
}

// ---------------------------------------------------------------- row 19
#[test]
fn cfg19_fixed_y_sweep_exp_minus600_to_600() {
    let ys = [
        f32::MIN_POSITIVE,
        f32::EPSILON,
        1.0f32,
        -1.0f32,
        2.0f32,
        0.1f32,
        -7.5e21f32,
        1.234_567_9e-15f32,
    ];
    for &y in &ys {
        for e in -600..=600 {
            check(y, e);
        }
    }
}

// ---------------------------------------------------------------- row 20
#[test]
fn cfg20_exp_at_i32_min_boundary() {
    for e in [
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 2,
        i32::MIN + 3,
        i32::MIN + 4,
        i32::MIN + 5,
        i32::MIN + 127,
        i32::MIN + 128,
    ] {
        for y in y_all() {
            check(y, e);
        }
        let mut rng = Rng::new(0x0000_0014 ^ (e as u32 as u64));
        for _ in 0..5_000 {
            check(rng.next_f32_bits(), e);
        }
    }
}

// ---------------------------------------------------------------- row 21
#[test]
fn cfg21_exp_at_i32_max_boundary() {
    // ~18 M loop iterations per call — keep the input set small.
    let ys = [1.0f32, -1.0f32, 0.0f32, -0.0f32, f32::INFINITY, f32::from_bits(0x7fc0_dead)];
    for e in [i32::MAX, i32::MAX - 1, i32::MAX - 119, i32::MAX - 120] {
        for &y in &ys {
            check(y, e);
        }
    }
}

// ---------------------------------------------------------------- row 22
#[test]
fn cfg22_power_of_two_neighbourhood_walk() {
    let mut rng = Rng::new(0x0000_0016);
    for e in exp_classes() {
        for _ in 0..32 {
            check(rng.next_f32_bits(), e);
        }
        for y in Y_CLASSES {
            check(*y, e);
        }
    }
}

// ---------------------------------------------------------------- row 23
#[test]
fn cfg23_chained_composition() {
    let p = pair();
    let mut rng = Rng::new(0x0000_0017);
    for _ in 0..20_000 {
        let y = rng.next_f32_bits();
        let a = rng.in_range(-5_000, 5_000);
        let b = rng.in_range(-5_000, 5_000);
        let c_out = unsafe { (p.c)((p.c)(y, a), b) };
        let rs_out = unsafe { (p.rs)((p.rs)(y, a), b) };
        assert_eq!(
            c_out.to_bits(),
            rs_out.to_bits(),
            "chained divergence: y=0x{:08x} a={a} b={b} -> C 0x{:08x} vs RS 0x{:08x}",
            y.to_bits(),
            c_out.to_bits(),
            rs_out.to_bits()
        );
        // and the mixed chain (C then Rust) must agree too
        let mixed = unsafe { (p.rs)((p.c)(y, a), b) };
        assert_eq!(c_out.to_bits(), mixed.to_bits(), "mixed chain divergence");
    }
}

// ---------------------------------------------------------------- row 24
#[test]
fn cfg24_exhaustive_exp_minus1024_to_1024() {
    let ys = [
        1.0f32,
        -1.0f32,
        0.5f32,
        3.141_592_7f32,
        f32::MIN_POSITIVE,
        f32::MAX,
        f32::from_bits(0x0000_0001),
        f32::from_bits(0x4c23_d70a),
    ];
    for e in -1024..=1024 {
        for &y in &ys {
            check(y, e);
        }
    }
}
