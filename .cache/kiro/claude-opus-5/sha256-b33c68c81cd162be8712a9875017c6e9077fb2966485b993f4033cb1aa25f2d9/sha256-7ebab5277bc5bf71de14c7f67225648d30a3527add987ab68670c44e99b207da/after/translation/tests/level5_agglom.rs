//! Level 5: the public entry point `agglom`, which is the only function
//! declared in `c_src/include/lib.h`.

mod harness;

use harness::*;

struct Args {
    f2: [f32; 7],
    f3: [i32; 2],
    f4: [u64; 2],
    f5: u32,
    f7: [u32; 3],
    f9: [f32; 8],
    f10: u16,
    f11: [f32; 3],
    f12: [f32; 3],
    f13: [f32; 3],
}

fn call(f: &FnAgglom, a: &Args) -> f64 {
    unsafe {
        f(
            a.f2[0], a.f2[1], a.f2[2], a.f2[3], a.f2[4], a.f2[5], a.f2[6], a.f3[0], a.f3[1],
            a.f4[0], a.f4[1], a.f5, a.f7[0], a.f7[1], a.f7[2], a.f9[0], a.f9[1], a.f9[2], a.f9[3],
            a.f9[4], a.f9[5], a.f9[6], a.f9[7], a.f10, a.f11[0], a.f11[1], a.f11[2], a.f12[0],
            a.f12[1], a.f12[2], a.f13[0], a.f13[1], a.f13[2],
        )
    }
}

fn baseline() -> Args {
    Args {
        f2: [0.0, 0.0, 1.0, -1.0, -1.0, 1.0, 1.0],
        f3: [7, 2],
        f4: [1, 2],
        f5: 0x1234,
        f7: [4096, 2, 16],
        f9: [0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.25, 0.25],
        f10: 0x3C00,
        f11: [30.0, 0.5, 0.5],
        f12: [200.0, 0.5, 0.5],
        f13: [0.25, 0.5, 0.75],
    }
}

fn compare(ctx: &str, c: &FnAgglom, r: &FnAgglom, a: &Args) {
    eq_f64(ctx, call(c, a), call(r, a));
}

#[test]
fn agglom_baseline_and_axis_sweeps() {
    let i = impls();
    let (cs, rs) = i.sym::<FnAgglom>("agglom");
    let (c, r) = (*cs, *rs);

    compare("agglom baseline", &c, &r, &baseline());

    // Vary one argument group at a time over its interesting domain, holding
    // the rest at the baseline. Keeps failures attributable to one function.
    for &v in EDGE_F32 {
        for k in 0..7 {
            let mut a = baseline();
            a.f2[k] = v;
            compare(&format!("agglom f2[{k}]={v:?}"), &c, &r, &a);
        }
        for k in 0..8 {
            let mut a = baseline();
            a.f9[k] = v;
            compare(&format!("agglom f9[{k}]={v:?}"), &c, &r, &a);
        }
        for k in 0..3 {
            let mut a = baseline();
            a.f11[k] = v;
            compare(&format!("agglom f11[{k}]={v:?}"), &c, &r, &a);
            let mut a = baseline();
            a.f12[k] = v;
            compare(&format!("agglom f12[{k}]={v:?}"), &c, &r, &a);
            let mut a = baseline();
            a.f13[k] = v;
            compare(&format!("agglom f13[{k}]={v:?}"), &c, &r, &a);
        }
    }

    for &v in EDGE_I32 {
        for k in 0..2 {
            let mut a = baseline();
            a.f3[k] = v;
            compare(&format!("agglom f3[{k}]={v}"), &c, &r, &a);
        }
    }

    for &v in EDGE_U64 {
        for k in 0..2 {
            let mut a = baseline();
            a.f4[k] = v;
            compare(&format!("agglom f4[{k}]={v:#x}"), &c, &r, &a);
        }
    }

    for &v in EDGE_U32 {
        let mut a = baseline();
        a.f5 = v;
        compare(&format!("agglom f5={v:#x}"), &c, &r, &a);
        for k in 0..3 {
            let mut a = baseline();
            a.f7[k] = v;
            compare(&format!("agglom f7[{k}]={v}"), &c, &r, &a);
        }
    }

    // f10's whole domain, through agglom.
    for h in 0u16..=0xFFFF {
        let mut a = baseline();
        a.f10 = h;
        compare(&format!("agglom f10={h:#06x}"), &c, &r, &a);
        if h == 0xFFFF {
            break;
        }
    }
}

#[test]
fn agglom_random_fuzz() {
    let i = impls();
    let (cs, rs) = i.sym::<FnAgglom>("agglom");
    let (c, r) = (*cs, *rs);
    let mut rng = Rng::new(0xA6610);

    let hue = |rng: &mut Rng| {
        let pick = rng.next_u32() % 8;
        match pick {
            0 => 0.0,
            1 => 60.0,
            2 => 120.0,
            3 => 180.0,
            4 => 240.0,
            5 => 300.0,
            6 => 360.0,
            _ => rng.next_f32_in(400.0),
        }
    };

    for n in 0..400_000u32 {
        // Mix "tame" and fully random bit patterns.
        let wild = n % 3 == 0;
        let f = |rng: &mut Rng, range: f32| {
            if wild {
                rng.next_f32_bits()
            } else {
                rng.next_f32_in(range)
            }
        };
        let a = Args {
            f2: [
                f(&mut rng, 10.0),
                f(&mut rng, 10.0),
                f(&mut rng, 10.0),
                f(&mut rng, 10.0),
                f(&mut rng, 10.0),
                f(&mut rng, 10.0),
                f(&mut rng, 10.0),
            ],
            f3: [rng.next_u32() as i32, rng.next_u32() as i32],
            f4: [rng.next_u64(), rng.next_u64()],
            f5: rng.next_u32(),
            f7: [rng.next_u32(), rng.next_u32(), rng.next_u32()],
            f9: [
                f(&mut rng, 4.0),
                f(&mut rng, 4.0),
                f(&mut rng, 4.0),
                f(&mut rng, 4.0),
                f(&mut rng, 4.0),
                f(&mut rng, 4.0),
                f(&mut rng, 4.0),
                f(&mut rng, 4.0),
            ],
            f10: rng.next_u32() as u16,
            f11: [hue(&mut rng), f(&mut rng, 1.0), f(&mut rng, 1.0)],
            f12: [hue(&mut rng), f(&mut rng, 1.0), f(&mut rng, 1.0)],
            f13: [f(&mut rng, 1.0), f(&mut rng, 1.0), f(&mut rng, 1.0)],
        };
        compare(&format!("agglom fuzz #{n}"), &c, &r, &a);
    }
}

/// Small-magnitude integer divisors are where `f3`'s INT_MIN paths live; make
/// sure they are reached through `agglom` too.
#[test]
fn agglom_f3_grid() {
    let i = impls();
    let (cs, rs) = i.sym::<FnAgglom>("agglom");
    let (c, r) = (*cs, *rs);
    for v1 in -60i32..=60 {
        for v2 in -60i32..=60 {
            let mut a = baseline();
            a.f3 = [v1, v2];
            compare(&format!("agglom f3=({v1},{v2})"), &c, &r, &a);
        }
    }
    for base in [i32::MIN, i32::MAX] {
        for d in -8i32..=8 {
            for &v2 in EDGE_I32 {
                let mut a = baseline();
                a.f3 = [base.wrapping_add(d), v2];
                compare("agglom f3 extreme", &c, &r, &a);
                a.f3 = [v2, base.wrapping_add(d)];
                compare("agglom f3 extreme swapped", &c, &r, &a);
            }
        }
    }
}
