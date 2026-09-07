//! Phase B — valid-path differential tests, one test per CONFIGS.md row.
//!
//! Every test loads BOTH shared objects with `libloading` and compares the full
//! `pcm` buffer byte-for-byte. Seeds are fixed for reproducibility.

mod common;
use common::*;

const SEED: u64 = 0x0DDB_A11C_0FFE_E5EE;

// ---------------------------------------------------------------- row 1
#[test]
fn row01_nch1_unit_random() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 1);
    for i in 0..4096 {
        let z = make_z(0, |_| rng.sym(1.0));
        assert_same(&p, &Case::new(1, &z), &format!("row01 nch=1 iter={i}"));
    }
}

// ---------------------------------------------------------------- row 2
#[test]
fn row02_nch2_unit_random() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 2);
    for i in 0..4096 {
        let z = make_z(0, |_| rng.sym(1.0));
        assert_same(&p, &Case::new(2, &z), &format!("row02 nch=2 iter={i}"));
    }
}

// ---------------------------------------------------------------- row 3
// Straddle the whole int16 range: sum of |coeffs| for acc1 is ~137k, so a
// per-tap magnitude around 0.35 puts the accumulator right across the
// clamp/round boundaries about half the time.
#[test]
fn row03_nch1_straddles_clamp_and_round() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 3);
    for i in 0..4096 {
        let scale = 0.15 + rng.unit() * 0.6;
        let z = make_z(0, |_| rng.sym(scale));
        assert_same(&p, &Case::new(1, &z), &format!("row03 iter={i}"));
    }
}

// ---------------------------------------------------------------- row 4
// Small accumulator in roughly (-3, 3): densely exercises `s -= (s < 0)`,
// the `s == 0` no-decrement edge, and both signs.
#[test]
fn row04_nch2_tiny_accumulator_rounding() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 4);
    for i in 0..4096 {
        let z = make_z(0, |_| rng.sym(2.0e-5));
        assert_same(&p, &Case::new(2, &z), &format!("row04 iter={i}"));
    }
}

// ---------------------------------------------------------------- row 5
#[test]
fn row05_nch2_saturating_large() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 5);
    for i in 0..2048 {
        let z = make_z(0, |_| rng.sym(1.0e3));
        assert_same(&p, &Case::new(2, &z), &format!("row05 iter={i}"));
    }
}

// ---------------------------------------------------------------- row 6
// Fully random bit patterns: normals, subnormals, +-0, +-inf and NaN all occur,
// including inf-inf and inf*0 NaN generation inside the dot product.
#[test]
fn row06_nch2_arbitrary_bit_patterns() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 6);
    for i in 0..8192 {
        let z = make_z(0, |_| rng.any_f32());
        assert_same(&p, &Case::new(2, &z), &format!("row06 iter={i}"));
    }
}

// ---------------------------------------------------------------- row 7
#[test]
fn row07_degenerate_uniform_buffers() {
    let p = Pair::load();
    for (name, v) in [
        ("+0.0", 0.0f32),
        ("-0.0", -0.0f32),
        ("+1.0", 1.0f32),
        ("-1.0", -1.0f32),
        ("f32::MIN_POSITIVE", f32::MIN_POSITIVE),
        ("subnormal", f32::from_bits(1)),
        ("f32::MAX", f32::MAX),
        ("f32::MIN", f32::MIN),
        ("+inf", f32::INFINITY),
        ("-inf", f32::NEG_INFINITY),
        ("NaN", f32::NAN),
    ] {
        let z = make_z(0, |_| v);
        for nch in [0, 1, 2, 3] {
            assert_same(&p, &Case::new(nch, &z), &format!("row07 all={name} nch={nch}"));
        }
    }
}

// ---------------------------------------------------------------- row 8
// One special value injected at each of the 23 tap offsets the C actually
// reads, with the rest random.
#[test]
fn row08_special_value_at_every_tap() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 8);
    let specials: [(&str, f32); 8] = [
        ("+inf", f32::INFINITY),
        ("-inf", f32::NEG_INFINITY),
        ("NaN", f32::NAN),
        ("-NaN", -f32::NAN),
        ("f32::MAX", f32::MAX),
        ("f32::MIN", f32::MIN),
        ("MIN_POSITIVE", f32::MIN_POSITIVE),
        ("-0.0", -0.0f32),
    ];
    for tap in tap_offsets() {
        for (name, v) in specials {
            for rep in 0..4 {
                let mut z = make_z(0, |_| rng.sym(1.0));
                z[tap] = v;
                assert_same(
                    &p,
                    &Case::new(2, &z),
                    &format!("row08 tap={tap} special={name} rep={rep}"),
                );
            }
        }
    }
}

// ---------------------------------------------------------------- row 9
// Drive each accumulator across its exact clamp boundary by using a single tap
// (all other taps zero) and sweeping the value by ULPs around the solution of
// `v * coeff == boundary`.
fn ulp_sweep(v: f32, n: i32) -> Vec<f32> {
    let mut out = Vec::new();
    let mut lo = v;
    for _ in 0..n {
        lo = next_down(lo);
        out.push(lo);
    }
    out.push(v);
    let mut hi = v;
    for _ in 0..n {
        hi = next_up(hi);
        out.push(hi);
    }
    out
}

fn next_up(x: f32) -> f32 {
    if x.is_nan() {
        return x;
    }
    if x == 0.0 {
        return f32::from_bits(1);
    }
    let b = x.to_bits();
    if x > 0.0 {
        f32::from_bits(b + 1)
    } else {
        f32::from_bits(b - 1)
    }
}

fn next_down(x: f32) -> f32 {
    if x.is_nan() {
        return x;
    }
    if x == 0.0 {
        return -f32::from_bits(1);
    }
    let b = x.to_bits();
    if x > 0.0 {
        f32::from_bits(b - 1)
    } else {
        f32::from_bits(b + 1)
    }
}

#[test]
fn row09_exact_clamp_boundaries() {
    let p = Pair::load();
    // (tap index, coefficient) for a single-tap drive of each accumulator.
    // acc1: z[7*64] * 75038      acc2: z[2 + 8*64] * 64019
    for (tap, coeff, which) in [(7 * 64usize, 75038.0f32, "acc1"), (2 + 8 * 64, 64019.0f32, "acc2")]
    {
        for boundary in [32766.5f32, -32767.5f32, 32767.0, -32768.0, 0.5, -0.5, -1.5, 1.0, -1.0] {
            let base = boundary / coeff;
            for v in ulp_sweep(base, 64) {
                let mut z = make_z(0, |_| 0.0);
                z[tap] = v;
                assert_same(
                    &p,
                    &Case::new(2, &z),
                    &format!("row09 {which} tap={tap} boundary={boundary} v={v:e}"),
                );
            }
        }
    }
}

// ---------------------------------------------------------------- row 10
// nch == 0: pcm[16*0] aliases pcm[0]; the second store must overwrite the first
// in both implementations.
#[test]
fn row10_nch0_aliased_store() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 10);
    for i in 0..2048 {
        let z = make_z(0, |_| rng.sym(0.4));
        let mut case = Case::new(0, &z);
        case.pcm_len = 4; // extra cells act as canaries
        assert_same(&p, &case, &format!("row10 nch=0 iter={i}"));
    }
    // And with values large enough that the two accumulators clamp differently.
    let mut rng = Rng::new(SEED ^ 0x10A);
    for i in 0..512 {
        let z = make_z(0, |_| rng.sym(500.0));
        let mut case = Case::new(0, &z);
        case.pcm_len = 4;
        assert_same(&p, &case, &format!("row10 nch=0 saturating iter={i}"));
    }
}

// ---------------------------------------------------------------- row 11
#[test]
fn row11_negative_nch() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 11);
    for nch in [-1i32, -2, -8, -3, -64] {
        for i in 0..1024 {
            let z = make_z(0, |_| rng.sym(0.4));
            let mut case = Case::new(nch, &z);
            // Case::new already reserves `16*|nch|` cells of head-room.
            case.pcm_len = case.head + 4;
            assert_same(&p, &case, &format!("row11 nch={nch} iter={i}"));
        }
    }
}

// ---------------------------------------------------------------- row 12
#[test]
fn row12_large_nch_strides() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 12);
    for nch in [3i32, 4, 7, 16, 64, 512, 4096] {
        for i in 0..256 {
            let z = make_z(0, |_| rng.sym(0.4));
            assert_same(&p, &Case::new(nch, &z), &format!("row12 nch={nch} iter={i}"));
        }
    }
}

// ---------------------------------------------------------------- row 13
// Canary check: exactly two cells (0 and 16*nch) may change.
#[test]
fn row13_only_two_cells_written() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 13);
    for nch in [1i32, 2, 5] {
        for i in 0..256 {
            let z = make_z(0, |_| rng.sym(0.4));
            let mut case = Case::new(nch, &z);
            case.pcm_len = 16 * nch as usize + 8;
            case.canary = -0x2B2B;
            let ctx = format!("row13 nch={nch} iter={i}");
            assert_same(&p, &case, &ctx);
            let (c, _) = run(&p, &case);
            for (idx, cell) in c.iter().enumerate() {
                if idx != 0 && idx != 16 * nch as usize {
                    assert_eq!(*cell, case.canary, "{ctx}: C wrote unexpected index {idx}");
                }
            }
        }
    }
}

// ---------------------------------------------------------------- row 14
// z buffer of exactly the minimum length (899 floats).
#[test]
fn row14_exact_minimum_z_length() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 14);
    for i in 0..512 {
        let z: Vec<f32> = (0..Z_MIN_LEN).map(|_| rng.sym(0.4)).collect();
        assert_eq!(z.len(), Z_MIN_LEN);
        assert_same(&p, &Case::new(1, &z), &format!("row14 iter={i}"));
    }
}

// ---------------------------------------------------------------- row 15
// Unaligned-by-element view: pointer advanced by 1..=7 floats inside a bigger
// buffer, so the tap offsets land on different addresses.
#[test]
fn row15_offset_z_view() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 15);
    for off in 1..=7usize {
        for i in 0..128 {
            let z = make_z(8, |_| rng.sym(0.4));
            let mut case = Case::new(2, &z);
            case.z_offset = off;
            assert_same(&p, &case, &format!("row15 z_offset={off} iter={i}"));
        }
    }
}

// ---------------------------------------------------------------- row 16
// Decoder-shaped loop: 32 sub-band calls sharing one pcm buffer and one z
// window, exactly how mp3 synthesis drives this function.
#[test]
fn row16_decoder_loop_shape() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 16);
    let nch = 2i32;
    for iter in 0..64 {
        let z = make_z(64, |_| rng.sym(0.3));
        // Replay the full 32-step loop on each library and compare at the end.
        let mut bufs: Vec<Vec<i16>> = Vec::new();
        for f in [p.c, p.rust] {
            let mut pcm = vec![0x5A5Au16 as i16; 32 * 16 * nch as usize + 64];
            for sb in 0..32usize {
                unsafe {
                    f(
                        pcm.as_mut_ptr().add(sb * nch as usize),
                        nch,
                        z.as_ptr().add(sb % 33),
                    );
                }
            }
            bufs.push(pcm);
        }
        assert_eq!(bufs[0], bufs[1], "row16 decoder loop diverged at iter={iter}");
    }
}
