//! Phase B -- valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`.  Every row drives the exported
//! `synth_pair` in BOTH shared libraries (loaded with `libloading`) over many
//! randomized inputs with fixed seeds, and requires byte-identical PCM output.
//!
//! `z`'s 876 unread slots are always `NaN`-poisoned, so any tap-index mistake
//! in the translation collapses the Rust accumulator to 0 and diverges loudly.

mod harness;

use harness::*;

/// Drive `iters` randomized vectors through every `nch` in `nchs`.
fn sweep(label: &str, nchs: &[i32], iters: usize, seed: u64, mut make: impl FnMut(&mut Rng) -> f32) {
    let mut rng = Rng::new(seed);
    for it in 0..iters {
        let z = z_from(|_| make(&mut rng));
        for &nch in nchs {
            assert_same(&format!("{label} iter={it}"), nch, &z);
        }
    }
}

// --- row 1 -----------------------------------------------------------------

#[test]
fn cfg01_all_zero_taps_exact_read_set() {
    let z = z_zero();
    // 876 of the 899 slots are NaN; if the C read any of them the output could
    // not be 0, and if Rust read any of them it would diverge from C.
    assert_eq!(read_indices().len(), 23, "read-index set size");
    for nch in [1, 2] {
        let buf = assert_same("cfg01", nch, &z);
        let off = second_store_offset(nch);
        assert_eq!(buf.data[buf.base], 0, "pcm[0] for all-zero taps");
        assert_eq!(buf.data[(buf.base as isize + off) as usize], 0, "pcm[16*nch]");
        let expect: Vec<usize> = vec![buf.base, (buf.base as isize + off) as usize];
        for i in buf.touched() {
            assert!(expect.contains(&i), "unexpected store at buf[{i}]");
        }
    }
}

// --- row 2 -----------------------------------------------------------------

#[test]
fn cfg02_negative_zero_taps() {
    for v in [-0.0f32, 0.0f32] {
        let z = z_const(v);
        for nch in [0, 1, 2, -1] {
            assert_same("cfg02", nch, &z);
        }
    }
    // Mixed signed zeros across taps.
    let mut rng = Rng::new(0x2020);
    for _ in 0..500 {
        let z = z_from(|_| if rng.bool() { -0.0 } else { 0.0 });
        assert_same("cfg02-mixed", 2, &z);
    }
}

// --- rows 3..7 -------------------------------------------------------------

#[test]
fn cfg03_unit_scale_random_nch2() {
    sweep("cfg03", &[2], 2000, 0x03, |r| r.sym(1.0));
}

#[test]
fn cfg04_half_scale_random_nch2() {
    sweep("cfg04", &[2], 2000, 0x04, |r| r.sym(0.5));
}

#[test]
fn cfg05_tiny_log_uniform_nch1() {
    sweep("cfg05", &[1], 2000, 0x05, |r| r.log_uniform(-8.0, -1.0));
}

#[test]
fn cfg06_mid_log_uniform_nch1() {
    sweep("cfg06", &[1], 2000, 0x06, |r| r.log_uniform(-1.0, 2.0));
}

#[test]
fn cfg07_large_log_uniform_nch2() {
    sweep("cfg07", &[2], 2000, 0x07, |r| r.log_uniform(2.0, 6.0));
}

// --- row 8 -----------------------------------------------------------------

#[test]
fn cfg08_arbitrary_bit_patterns_nch2() {
    sweep("cfg08", &[2], 4000, 0x08, |r| r.any_bits_f32());
}

// --- row 9 -----------------------------------------------------------------

#[test]
fn cfg09_subnormal_taps_nch1() {
    sweep("cfg09", &[1], 500, 0x09, |r| r.subnormal());
    // Extremes of the subnormal range.
    for v in [
        f32::from_bits(1),
        -f32::from_bits(1),
        f32::from_bits(0x007F_FFFF),
        -f32::from_bits(0x007F_FFFF),
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
    ] {
        assert_same("cfg09-edge", 1, &z_const(v));
    }
}

// --- row 10 ----------------------------------------------------------------

#[test]
fn cfg10_float_max_taps_nch2() {
    let mut rng = Rng::new(0x10);
    for _ in 0..500 {
        let z = z_from(|_| if rng.bool() { f32::MAX } else { -f32::MAX });
        assert_same("cfg10", 2, &z);
    }
    for v in [f32::MAX, -f32::MAX, f32::INFINITY, f32::NEG_INFINITY] {
        assert_same("cfg10-const", 2, &z_const(v));
    }
}

// --- rows 11 & 12 ----------------------------------------------------------

fn single_tap_row(label: &str, chain: Chain, seed: u64) {
    let mut rng = Rng::new(seed);
    for &(idx, w) in chain.taps() {
        for k in 0..300 {
            let v = match k % 5 {
                0 => rng.sym(1.0),
                1 => rng.log_uniform(-8.0, 1.0),
                2 => rng.log_uniform(1.0, 6.0),
                3 => rng.any_bits_f32(),
                _ => rng.subnormal(),
            };
            let z = z_single(idx, v);
            let label = format!("{label} idx={idx} w={w} v={v:e}");
            assert_same(&label, 1, &z);
            assert_same(&label, 2, &z);
        }
        // Dense ULP sweeps around values that put the accumulator right at the
        // clamp boundary for this specific coefficient.
        for target in [32766.5f32, -32767.5, 0.5, -0.5, 2.5, -2.5] {
            let base = (target as f64 / w as f64) as f32;
            for d in -80i32..=80 {
                let v = nudge(base, d);
                if !v.is_finite() {
                    continue;
                }
                assert_same(&format!("{label} sweep idx={idx} t={target:e} d={d}"), 2, &z_single(idx, v));
            }
        }
    }
}

#[test]
fn cfg11_single_tap_chain0_each_coefficient() {
    single_tap_row("cfg11", Chain::Lo, 0x11);
}

#[test]
fn cfg12_single_tap_chain1_each_coefficient() {
    single_tap_row("cfg12", Chain::Hi, 0x12);
}

// --- row 13 ----------------------------------------------------------------

#[test]
fn cfg13_one_dominant_tap_rest_random() {
    let mut rng = Rng::new(0x13);
    for idx in read_indices() {
        for k in 0..120 {
            let mut z = z_from(|_| rng.sym(1.0));
            z[idx] = match k % 4 {
                0 => rng.log_uniform(3.0, 8.0),
                1 => rng.log_uniform(-8.0, -3.0),
                2 => f32::MAX,
                _ => -f32::MAX,
            };
            assert_same(&format!("cfg13 idx={idx} k={k}"), 2, &z);
        }
    }
}

// --- row 14 ----------------------------------------------------------------

#[test]
fn cfg14_rounding_tie_sweep() {
    // Every half-integer tie and its ULP neighbourhood in [-4, 4], on both
    // chains: exercises truncation-toward-zero and the `s -= (s < 0)` branch.
    for chain in [Chain::Lo, Chain::Hi] {
        for q in -32i32..=32 {
            let target = q as f32 * 0.125;
            let (idx, w) = *chain.taps().first().unwrap();
            let base = (target as f64 / w as f64) as f32;
            for d in -40i32..=40 {
                let v = nudge(base, d);
                if !v.is_finite() {
                    continue;
                }
                let a = single_tap_accumulator(chain, idx, v);
                assert_same(
                    &format!("cfg14 {chain:?} target={target} d={d} a={a:e}"),
                    1,
                    &z_single(idx, v),
                );
            }
            // And the exactly-representable tie, when reachable.
            if let Some((i, v)) = find_single_tap_exact(chain, target) {
                assert_same(&format!("cfg14-exact {chain:?} {target}"), 1, &z_single(i, v));
            }
        }
    }
}

// --- rows 15 & 16 ----------------------------------------------------------

fn boundary_sweep(label: &str, chain: Chain) {
    for target in [
        32766.5f32, -32767.5, 32767.0, -32768.0, 32766.0, -32767.0, 32765.5, -32766.5,
    ] {
        for &(idx, w) in chain.taps() {
            let base = (target as f64 / w as f64) as f32;
            for d in -160i32..=160 {
                let v = nudge(base, d);
                if !v.is_finite() {
                    continue;
                }
                assert_same(
                    &format!("{label} {chain:?} idx={idx} target={target:e} d={d}"),
                    1,
                    &z_single(idx, v),
                );
            }
        }
    }
}

#[test]
fn cfg15_clamp_boundary_ulp_sweep_chain0() {
    boundary_sweep("cfg15", Chain::Lo);
}

#[test]
fn cfg16_clamp_boundary_ulp_sweep_chain1() {
    boundary_sweep("cfg16", Chain::Hi);
}

// --- row 17 ----------------------------------------------------------------

#[test]
fn cfg17_both_chains_independent_regimes() {
    // Chain 0 and chain 1 read disjoint slots, so their regimes can be set
    // independently: this walks the full cross-product of
    // {in-range +, in-range -, clamp high, clamp low, NaN, inf, -inf, zero}.
    let regimes: Vec<f32> = vec![
        0.0, 1.0, -1.0, 100.0, -100.0, 32766.5, -32767.5, 1e9, -1e9,
        f32::INFINITY, f32::NEG_INFINITY, f32::NAN, 32766.0, -32767.0,
    ];
    for &t0 in &regimes {
        for &t1 in &regimes {
            let mut z = z_zero();
            let (i0, w0) = CHAIN0[7]; // z[7*64], weight 75038
            let (i1, w1) = CHAIN1[4]; // z[2+8*64], weight 64019
            z[i0] = t0 / w0;
            z[i1] = t1 / w1;
            for nch in [0, 1, 2, -2] {
                assert_same(&format!("cfg17 t0={t0:e} t1={t1:e}"), nch, &z);
            }
        }
    }
    // Randomized version of the same cross-product.
    let mut rng = Rng::new(0x17);
    for _ in 0..2000 {
        let mut z = z_zero();
        for &(idx, _) in CHAIN0.iter() {
            z[idx] = rng.log_uniform(-3.0, 4.0);
        }
        for &(idx, _) in CHAIN1.iter() {
            z[idx] = rng.log_uniform(-3.0, 4.0);
        }
        assert_same("cfg17-rand", 2, &z);
    }
}

// --- row 18 ----------------------------------------------------------------

#[test]
fn cfg18_nch_zero_aliased_store() {
    let mut rng = Rng::new(0x18);
    for _ in 0..500 {
        let z = z_from(|_| rng.log_uniform(-4.0, 4.0));
        let buf = assert_same("cfg18", 0, &z);
        // Both stores land on pcm[0]; only that one slot may change.
        for i in buf.touched() {
            assert_eq!(i, buf.base, "nch=0 must only write pcm[0]");
        }
    }
}

// --- row 19 ----------------------------------------------------------------

#[test]
fn cfg19_negative_nch() {
    let mut rng = Rng::new(0x19);
    for nch in [-1, -2, -8, -64, -3, -100] {
        for _ in 0..200 {
            let z = z_from(|_| rng.sym(2.0));
            let buf = assert_same(&format!("cfg19 nch={nch}"), nch, &z);
            let off = second_store_offset(nch);
            assert!(off < 0, "expected a negative store offset for nch={nch}");
            let hi = (buf.base as isize + off) as usize;
            assert!(hi < buf.base, "second store must precede pcm");
        }
    }
}

// --- row 20 ----------------------------------------------------------------

#[test]
fn cfg20_positive_nch_spread() {
    let mut rng = Rng::new(0x20);
    for nch in [1, 2, 3, 4, 8, 16, 64, 1024] {
        for _ in 0..200 {
            let z = z_from(|_| rng.sym(2.0));
            assert_same(&format!("cfg20 nch={nch}"), nch, &z);
        }
    }
}

// --- row 21 ----------------------------------------------------------------

#[test]
fn cfg21_unaligned_z_pointer() {
    let mut rng = Rng::new(0x21);
    for shift in 0..4usize {
        for _ in 0..200 {
            let mut buf = vec![f32::NAN; Z_LEN + 4];
            for idx in read_indices() {
                buf[shift + idx] = rng.sym(1.5);
            }
            let p = unsafe { buf.as_ptr().add(shift) };
            let addr = p as usize;
            unsafe {
                assert_same_ptr(
                    &format!("cfg21 shift={shift} addr%16={}", addr % 16),
                    2,
                    p,
                );
            }
        }
    }
}

// --- row 22 ----------------------------------------------------------------

/// Taps in evaluation order for a chain, paired with the *positive*-side index
/// so that setting one value contributes exactly `v * w` as that term.
fn ordered_terms(chain: Chain) -> Vec<(usize, f32)> {
    match chain {
        // (z[14*64]-z[0])*29, (z[1*64]+z[13*64])*213, (z[12*64]-z[2*64])*459,
        // (z[3*64]+z[11*64])*2037, (z[10*64]-z[4*64])*5153,
        // (z[5*64]+z[9*64])*6574, (z[8*64]-z[6*64])*37489, z[7*64]*75038
        Chain::Lo => vec![
            (14 * 64, 29.0),
            (1 * 64, 213.0),
            (12 * 64, 459.0),
            (3 * 64, 2037.0),
            (10 * 64, 5153.0),
            (5 * 64, 6574.0),
            (8 * 64, 37489.0),
            (7 * 64, 75038.0),
        ],
        Chain::Hi => vec![
            (2 + 14 * 64, 104.0),
            (2 + 12 * 64, 1567.0),
            (2 + 10 * 64, 9727.0),
            (2 + 8 * 64, 64019.0),
            (2 + 6 * 64, -9975.0),
            (2 + 4 * 64, -45.0),
            (2 + 2 * 64, 146.0),
            (2 + 0 * 64, -5.0),
        ],
    }
}

#[test]
fn cfg22_catastrophic_cancellation_order_sensitivity() {
    // Alternating huge terms: the running sum cancels to a tiny residual, so the
    // result is extremely sensitive to any re-association of the eight `+=`.
    let mut rng = Rng::new(0x22);
    for mag in [1e6f32, 1e12, 1e20, 1e30, 1e37] {
        for _ in 0..400 {
            let mut z = z_zero();
            for (k, &(idx, w)) in ordered_terms(Chain::Lo).iter().enumerate() {
                let sign = if k % 2 == 0 { 1.0 } else { -1.0 };
                let jitter = 1.0 + rng.sym(1e-6);
                z[idx] = sign * mag * jitter / w;
            }
            for (k, &(idx, w)) in ordered_terms(Chain::Hi).iter().enumerate() {
                let sign = if k % 2 == 0 { 1.0 } else { -1.0 };
                let jitter = 1.0 + rng.sym(1e-6);
                z[idx] = sign * mag * jitter / w;
            }
            assert_same(&format!("cfg22 mag={mag:e}"), 2, &z);
        }
    }
}

// --- row 23 ----------------------------------------------------------------

#[test]
fn cfg23_intermediate_overflow_to_nan() {
    // Term 1 overflows the accumulator to +inf and a later term is -inf, so the
    // running sum becomes inf + (-inf) = NaN.  Order-dependent by construction.
    for chain in [Chain::Lo, Chain::Hi] {
        let terms = ordered_terms(chain);
        for i in 0..terms.len() {
            for j in 0..terms.len() {
                if i == j {
                    continue;
                }
                let mut z = z_zero();
                z[terms[i].0] = f32::MAX * terms[i].1.signum();
                z[terms[j].0] = -f32::MAX * terms[j].1.signum();
                assert_same(&format!("cfg23 {chain:?} +{i} -{j}"), 2, &z);
            }
        }
    }
    // inf and -inf taps directly.
    for chain in [Chain::Lo, Chain::Hi] {
        let terms = ordered_terms(chain);
        for i in 0..terms.len() {
            for j in 0..terms.len() {
                if i == j {
                    continue;
                }
                let mut z = z_zero();
                z[terms[i].0] = f32::INFINITY * terms[i].1.signum();
                z[terms[j].0] = f32::NEG_INFINITY * terms[j].1.signum();
                assert_same(&format!("cfg23-inf {chain:?} +{i} -{j}"), 2, &z);
            }
        }
    }
}

// --- row 24 ----------------------------------------------------------------

#[test]
fn cfg24_statelessness_replay_in_different_order() {
    let mut rng = Rng::new(0x24);
    let cases: Vec<(i32, Vec<f32>)> = (0..300)
        .map(|k| {
            let nch = *rng.pick(&[0i32, 1, 2, 3, -1, -4, 8]);
            let z = match k % 3 {
                0 => z_from(|_| rng.sym(1.0)),
                1 => z_from(|_| rng.log_uniform(-2.0, 5.0)),
                _ => z_from(|_| rng.any_bits_f32()),
            };
            (nch, z)
        })
        .collect();

    let first: Vec<Vec<i16>> = cases
        .iter()
        .map(|(nch, z)| assert_same("cfg24-pass1", *nch, z).data)
        .collect();

    // Replay in a shuffled order; results must be identical to pass 1.
    let mut order: Vec<usize> = (0..cases.len()).collect();
    for i in (1..order.len()).rev() {
        order.swap(i, rng.below(i + 1));
    }
    for &i in &order {
        let (nch, z) = &cases[i];
        let again = assert_same("cfg24-pass2", *nch, z).data;
        assert_eq!(again, first[i], "case {i} is not stateless");
    }
}

// --- row 25 ----------------------------------------------------------------

#[test]
fn cfg25_no_extra_stores() {
    let mut rng = Rng::new(0x25);
    for nch in [1, 2, 3, 8, -1, -8, 0] {
        for _ in 0..200 {
            let z = z_from(|_| rng.sym(3.0));
            let buf = assert_same(&format!("cfg25 nch={nch}"), nch, &z);
            let off = second_store_offset(nch);
            let allowed = [buf.base, (buf.base as isize + off) as usize];
            for i in buf.touched() {
                assert!(
                    allowed.contains(&i),
                    "store outside pcm[0]/pcm[16*nch]: buf[{i}] nch={nch}"
                );
            }
        }
    }
}

// --- row 26 ----------------------------------------------------------------

#[test]
fn cfg26_full_axis_fuzz() {
    let nchs = [0i32, 1, 2, 3, 4, 5, 8, 16, -1, -2, -3, -16, 100, -100, 1024];
    let mut rng = Rng::new(0xDEADBEEF);
    // Soak-testable: `SP_FUZZ_ITERS=2000000 cargo test cfg26` for a long run.
    let iters: usize = std::env::var("SP_FUZZ_ITERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(20000);
    for it in 0..iters {
        let class = rng.below(9);
        let z = z_from(|r_unused| {
            let _ = r_unused;
            match class {
                0 => rng.sym(1.0),
                1 => rng.sym(1e-6),
                2 => rng.log_uniform(-8.0, 8.0),
                3 => rng.any_bits_f32(),
                4 => rng.subnormal(),
                5 => {
                    if rng.bool() {
                        f32::MAX
                    } else {
                        -f32::MAX
                    }
                }
                6 => *rng.pick(&[f32::INFINITY, f32::NEG_INFINITY, f32::NAN, 0.0, -0.0]),
                7 => rng.sym(0.4368), // straddles the clamp for the 75038 tap
                _ => rng.log_uniform(-1.0, 2.0),
            }
        });
        let nch = *rng.pick(&nchs);
        assert_same(&format!("cfg26 it={it} class={class}"), nch, &z);
    }
}
