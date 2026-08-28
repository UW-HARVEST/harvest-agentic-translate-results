//! Level 3: integer / table-driven scalar functions (`f3`, `f4`, `f5`, `f7`, `f10`).

mod harness;

use harness::*;

/// `f3` is floor-division with a hand-rolled INT_MIN dance — exhaustive over
/// the edge set plus a large random sweep.
#[test]
fn f3_matches() {
    let i = impls();
    let (c, r) = i.sym::<FnF3>("f3");

    for &a in EDGE_I32 {
        for &b in EDGE_I32 {
            assert_eq!(unsafe { c(a, b) }, unsafe { r(a, b) }, "f3({a},{b})");
        }
    }

    // Small dense grid: catches every sign/rounding combination.
    for a in -300i32..=300 {
        for b in -300i32..=300 {
            assert_eq!(unsafe { c(a, b) }, unsafe { r(a, b) }, "f3({a},{b})");
        }
    }

    // INT_MIN / INT_MAX neighbourhoods against small divisors.
    for base in [i32::MIN, i32::MAX, 0] {
        for d in -40i32..=40 {
            let a = base.wrapping_add(d);
            for &b in EDGE_I32 {
                assert_eq!(unsafe { c(a, b) }, unsafe { r(a, b) }, "f3({a},{b})");
            }
            for b in -40i32..=40 {
                assert_eq!(unsafe { c(a, b) }, unsafe { r(a, b) }, "f3({a},{b})");
            }
        }
    }

    let mut rng = Rng::new(0xF3);
    for _ in 0..3_000_000 {
        let a = rng.next_u32() as i32;
        let b = rng.next_u32() as i32;
        assert_eq!(unsafe { c(a, b) }, unsafe { r(a, b) }, "f3({a},{b})");
    }
}

/// `f4` mutates the RNG state, so compare both the return value and the
/// resulting state, and chain many draws from the same seed.
#[test]
fn f4_matches() {
    let i = impls();
    let (c, r) = i.sym::<FnF4>("f4");

    let mut seeds: Vec<[u64; 2]> = Vec::new();
    for &a in EDGE_U64 {
        for &b in EDGE_U64 {
            seeds.push([a, b]);
        }
    }
    let mut rng = Rng::new(0xF4);
    for _ in 0..2000 {
        seeds.push([rng.next_u64(), rng.next_u64()]);
    }

    for seed in seeds {
        let mut cs = CnRnd { state: seed };
        let mut rs = CnRnd { state: seed };
        for step in 0..32 {
            let cv = unsafe { c(&mut cs) };
            let rv = unsafe { r(&mut rs) };
            eq_f64(&format!("f4({seed:?}) step {step}"), cv, rv);
            assert_eq!(
                cs.state, rs.state,
                "f4 state after step {step} from {seed:?}"
            );
        }
    }
}

#[test]
fn f5_matches() {
    let i = impls();
    let (c, r) = i.sym::<FnF5>("f5");

    // Exhaustive over the low 16 bits (the only bits f5 reads) ...
    for a in 0u32..=0xFFFF {
        assert_eq!(unsafe { c(a) }, unsafe { r(a) }, "f5({a:#x})");
    }
    // ... plus high-bit patterns to confirm they are discarded identically.
    for &a in EDGE_U32 {
        assert_eq!(unsafe { c(a) }, unsafe { r(a) }, "f5({a:#x})");
    }
    let mut rng = Rng::new(0xF5);
    for _ in 0..1_000_000 {
        let a = rng.next_u32();
        assert_eq!(unsafe { c(a) }, unsafe { r(a) }, "f5({a:#x})");
    }
}

/// `f7` is full of unsigned wraparound; hammer the overflow-prone ranges.
#[test]
fn f7_matches() {
    let i = impls();
    let (c, r) = i.sym::<FnF7>("f7");

    let mut pool: Vec<u32> = EDGE_U32.to_vec();
    // Realistic FLAC values and the special-cased channels==2 / bitdepth==32.
    pool.extend_from_slice(&[
        1, 2, 3, 8, 12, 16, 20, 24, 32, 64, 192, 576, 1152, 2304, 4608, 65535, 65536,
    ]);

    for &a in pool.iter() {
        for &b in pool.iter() {
            for &d in pool.iter() {
                assert_eq!(
                    unsafe { c(a, b, d) },
                    unsafe { r(a, b, d) },
                    "f7({a},{b},{d})"
                );
            }
        }
    }

    // Dense small grid around the channel/bitdepth branches.
    for a in 0u32..40 {
        for b in 0u32..40 {
            for d in 28u32..36 {
                assert_eq!(
                    unsafe { c(a, b, d) },
                    unsafe { r(a, b, d) },
                    "f7({a},{b},{d})"
                );
            }
        }
    }

    let mut rng = Rng::new(0xF7);
    for _ in 0..1_000_000 {
        let a = rng.next_u32();
        let b = rng.next_u32();
        let d = rng.next_u32();
        assert_eq!(
            unsafe { c(a, b, d) },
            unsafe { r(a, b, d) },
            "f7({a},{b},{d})"
        );
    }
}

/// `f10` (half -> float) has a 16-bit domain: test it exhaustively.
#[test]
fn f10_exhaustive() {
    let i = impls();
    let (c, r) = i.sym::<FnF10>("f10");
    for h in 0u16..=0xFFFF {
        let cv = unsafe { c(h) };
        let rv = unsafe { r(h) };
        eq_f32(&format!("f10({h:#06x})"), cv, rv);
        if h == 0xFFFF {
            break;
        }
    }
}
