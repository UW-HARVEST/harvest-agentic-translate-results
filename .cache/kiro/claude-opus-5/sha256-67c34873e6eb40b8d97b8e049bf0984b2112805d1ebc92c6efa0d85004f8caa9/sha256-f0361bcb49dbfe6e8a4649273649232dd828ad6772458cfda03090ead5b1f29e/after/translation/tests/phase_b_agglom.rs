//! Phase B — rows C48..C50: the composed `agglom` entry point (the only symbol
//! declared in `c_src/include/lib.h`).
//!
//! `agglom` runs all 13 sub-functions in sequence and accumulates into a
//! `double`, filtering NaN contributions with `isnan`.  Bugs that cancel out
//! inside one sub-function can still change the sum, so this is compared
//! bit-for-bit on the `f64`.

mod harness;

use harness::*;

fn chk(p: &Pair, a: &AgglomArgs) {
    let c = p.c.call_agglom(a);
    let r = p.r.call_agglom(a);
    assert_eq!(
        c.to_bits(),
        r.to_bits(),
        "agglom diverged: C={:#018x} ({c:?}) Rust={:#018x} ({r:?})\n  args={a:#?}",
        c.to_bits(),
        r.to_bits()
    );
}

/// Every argument drawn from the full bit space of its type.
fn random_args(rng: &mut Rng) -> AgglomArgs {
    AgglomArgs {
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
    }
}

/// "Tame" arguments: ordinary in-range values, so a single-axis sweep isolates
/// one sub-function's contribution to the sum.
fn tame_args(rng: &mut Rng) -> AgglomArgs {
    AgglomArgs {
        f2_1: rng.tame_f32(5.0),
        f2_2: rng.tame_f32(5.0),
        f2_3: rng.range_f32(0.0, 3.0),
        f2_7: rng.tame_f32(5.0),
        f2_8: rng.tame_f32(5.0),
        f2_9: rng.tame_f32(5.0),
        f2_10: rng.tame_f32(5.0),
        f3_1: (rng.next_u32() >> 8) as i32,
        f3_2: ((rng.next_u32() >> 16) as i32).max(1),
        f4_1: rng.next_u64(),
        f4_2: rng.next_u64(),
        f5_1: rng.next_u32() & 0xFFFF,
        f7_1: rng.next_u32() % 8192,
        f7_2: rng.next_u32() % 8,
        f7_3: rng.next_u32() % 40,
        f9_1: rng.tame_f32(5.0),
        f9_2: rng.tame_f32(5.0),
        f9_4: rng.tame_f32(5.0),
        f9_5: rng.tame_f32(5.0),
        f9_7: rng.tame_f32(5.0),
        f9_8: rng.tame_f32(5.0),
        f9_10: rng.tame_f32(5.0),
        f9_11: rng.tame_f32(5.0),
        f10_1: rng.next_u16(),
        f11_2: rng.range_f32(0.0, 400.0),
        f11_3: rng.range_f32(0.0, 1.0),
        f11_4: rng.range_f32(0.0, 1.0),
        f12_2: rng.range_f32(0.0, 400.0),
        f12_3: rng.range_f32(0.0, 1.0),
        f12_4: rng.range_f32(0.0, 1.0),
        f13_2: rng.range_f32(0.0, 1.0),
        f13_3: rng.range_f32(0.0, 1.0),
        f13_4: rng.range_f32(0.0, 1.0),
    }
}

// ------------------------------------------------------------------------ C48
#[test]
fn c48_agglom_full_random_bit_space() {
    let p = load();
    let mut rng = Rng::new(SEED ^ 48);
    for _ in 0..300_000 {
        let a = random_args(&mut rng);
        chk(&p, &a);
    }
    for _ in 0..100_000 {
        let a = tame_args(&mut rng);
        chk(&p, &a);
    }
}

// ------------------------------------------------------------------------ C49
/// Hold everything at the tame baseline and sweep the arguments of exactly one
/// sub-function at a time, so a divergence points straight at one stage.
#[test]
fn c49_agglom_per_subfunction_sweeps() {
    let p = load();
    let mut rng = Rng::new(SEED ^ 49);
    let sp = all_special_f32();
    let base = AgglomArgs::default();

    macro_rules! sweep_f32 {
        ($($field:ident),+ $(,)?) => {
            // Each float field, one at a time, over the special pool.
            $(
                for &s in &sp {
                    let mut a = base;
                    a.$field = s;
                    chk(&p, &a);
                }
            )+
            // All of this group's float fields together, sampled.
            for _ in 0..30_000 {
                let mut a = base;
                $( a.$field = rng.pick(&sp); )+
                chk(&p, &a);
            }
            for _ in 0..30_000 {
                let mut a = base;
                $( a.$field = rng.any_f32(); )+
                chk(&p, &a);
            }
        };
    }

    // f2 group (circle + AABB geometry)
    sweep_f32!(f2_1, f2_2, f2_3, f2_7, f2_8, f2_9, f2_10);
    // f9 group (triangle + probe)
    sweep_f32!(f9_1, f9_2, f9_4, f9_5, f9_7, f9_8, f9_10, f9_11);
    // f11 group (HSL)
    sweep_f32!(f11_2, f11_3, f11_4);
    // f12 group (HSV)
    sweep_f32!(f12_2, f12_3, f12_4);
    // f13 group (RGB)
    sweep_f32!(f13_2, f13_3, f13_4);

    // f3 group: every sign quadrant and INT_MIN/INT_MAX.
    let ints = [
        i32::MIN,
        i32::MIN + 1,
        -1000,
        -3,
        -1,
        0,
        1,
        3,
        1000,
        i32::MAX - 1,
        i32::MAX,
    ];
    for &v1 in &ints {
        for &v2 in &ints {
            let mut a = base;
            a.f3_1 = v1;
            a.f3_2 = v2;
            chk(&p, &a);
        }
    }
    for _ in 0..50_000 {
        let mut a = base;
        a.f3_1 = rng.next_i32();
        a.f3_2 = rng.next_i32();
        chk(&p, &a);
    }

    // f4 group: the generator state, incl. the {0,0} fixed point.
    for st in [
        [0u64, 0u64],
        [1, 0],
        [0, 1],
        [u64::MAX, u64::MAX],
        [1 << 63, 1 << 63],
    ] {
        let mut a = base;
        a.f4_1 = st[0];
        a.f4_2 = st[1];
        chk(&p, &a);
    }
    for _ in 0..50_000 {
        let mut a = base;
        a.f4_1 = rng.next_u64();
        a.f4_2 = rng.next_u64();
        chk(&p, &a);
    }

    // f5 group: exhaustive over the meaningful low 16 bits + high-bit noise.
    for lo in (0u32..=0xFFFF).step_by(7) {
        let mut a = base;
        a.f5_1 = lo | ((lo as u32) << 16);
        chk(&p, &a);
    }
    for _ in 0..50_000 {
        let mut a = base;
        a.f5_1 = rng.next_u32();
        chk(&p, &a);
    }

    // f7 group: all four (channels==2, bitdepth==32) combinations.
    for &ch in &[0u32, 1, 2, 3, 8, 0xFFFF_FFFF] {
        for &bd in &[0u32, 1, 16, 31, 32, 33, 0xFFFF_FFFF] {
            for &bs in &[0u32, 1, 4096, 0xFFFF_FFFF] {
                let mut a = base;
                a.f7_1 = bs;
                a.f7_2 = ch;
                a.f7_3 = bd;
                chk(&p, &a);
            }
        }
    }
    for _ in 0..50_000 {
        let mut a = base;
        a.f7_1 = rng.next_u32();
        a.f7_2 = rng.next_u32();
        a.f7_3 = rng.next_u32();
        chk(&p, &a);
    }

    // f10 group: exhaustive over the whole uint16_t domain.
    for h in 0u16..=u16::MAX {
        let mut a = base;
        a.f10_1 = h;
        chk(&p, &a);
    }
}

// ------------------------------------------------------------------------ C50
#[test]
fn c50_agglom_all_zero_all_ones_and_extremes() {
    let p = load();

    // Every argument zero.
    let zero = AgglomArgs {
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
    };
    chk(&p, &zero);

    // Every float argument set to each special value simultaneously, crossed
    // with the integer extremes.
    for &f in &[
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        1e-45,
        1.175_494_4e-38,
        3.402_823_5e38,
        -3.402_823_5e38,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::from_bits(0xFF80_0001),
        f32::from_bits(0x7FFF_FFFF),
    ] {
        for &(i1, i2) in &[
            (0i32, 0i32),
            (i32::MIN, i32::MIN),
            (i32::MIN, -1),
            (i32::MAX, i32::MAX),
            (i32::MIN, 1),
            (-1, i32::MIN),
        ] {
            for &(u1, u2) in &[(0u64, 0u64), (u64::MAX, u64::MAX)] {
                for &uu in &[0u32, u32::MAX] {
                    for &h in &[0u16, u16::MAX, 0x7C00, 0xFC00] {
                        let mut a = zero;
                        a.f2_1 = f;
                        a.f2_2 = f;
                        a.f2_3 = f;
                        a.f2_7 = f;
                        a.f2_8 = f;
                        a.f2_9 = f;
                        a.f2_10 = f;
                        a.f9_1 = f;
                        a.f9_2 = f;
                        a.f9_4 = f;
                        a.f9_5 = f;
                        a.f9_7 = f;
                        a.f9_8 = f;
                        a.f9_10 = f;
                        a.f9_11 = f;
                        a.f11_2 = f;
                        a.f11_3 = f;
                        a.f11_4 = f;
                        a.f12_2 = f;
                        a.f12_3 = f;
                        a.f12_4 = f;
                        a.f13_2 = f;
                        a.f13_3 = f;
                        a.f13_4 = f;
                        a.f3_1 = i1;
                        a.f3_2 = i2;
                        a.f4_1 = u1;
                        a.f4_2 = u2;
                        a.f5_1 = uu;
                        a.f7_1 = uu;
                        a.f7_2 = uu;
                        a.f7_3 = uu;
                        a.f10_1 = h;
                        chk(&p, &a);
                    }
                }
            }
        }
    }
}
