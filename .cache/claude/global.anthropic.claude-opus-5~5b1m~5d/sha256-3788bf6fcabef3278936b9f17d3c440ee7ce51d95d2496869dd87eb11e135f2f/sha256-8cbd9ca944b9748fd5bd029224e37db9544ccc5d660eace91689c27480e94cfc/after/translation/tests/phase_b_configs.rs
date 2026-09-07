//! Phase B - valid-path differential tests, one per row of `CONFIGS.md`.
//!
//! Every test loads BOTH the C `.so` and the Rust `.so` via `libloading` and
//! compares the exported `tritanopia` outputs byte-for-byte.

mod common;

use common::{CbRgb255, Pair, Rng, GAMMA_LINEAR_MAX, GAMMA_POW_MIN, SEED};

const N: usize = 20_000;

/// Random triples where each channel is drawn from its own `(lo, hi)` range.
fn random_in(
    seed: u64,
    n: usize,
    rr: (u8, u8),
    gr: (u8, u8),
    br: (u8, u8),
) -> impl Iterator<Item = CbRgb255> {
    let mut rng = Rng::new(seed);
    (0..n)
        .map(move |_| {
            CbRgb255::new(
                rng.range_u8(rr.0, rr.1),
                rng.range_u8(gr.0, gr.1),
                rng.range_u8(br.0, br.1),
            )
        })
        .collect::<Vec<_>>()
        .into_iter()
}

const LIN: (u8, u8) = (0, GAMMA_LINEAR_MAX);
const POW: (u8, u8) = (GAMMA_POW_MIN, 255);

// ---------------------------------------------------------------- C1
/// C1: remove-gamma linear arm on all three channels (bytes 0..=10),
/// exhaustive over all 11^3 = 1331 such triples.
#[test]
fn c1_all_linear_arm_exhaustive() {
    let p = Pair::load();
    let inputs = (0..=GAMMA_LINEAR_MAX).flat_map(|r| {
        (0..=GAMMA_LINEAR_MAX)
            .flat_map(move |g| (0..=GAMMA_LINEAR_MAX).map(move |b| CbRgb255::new(r, g, b)))
    });
    let n = p.check_all("C1", inputs);
    assert_eq!(n, 11 * 11 * 11);
}

// ---------------------------------------------------------------- C2..C8
/// C2: remove-gamma `pow` arm on all three channels.
#[test]
fn c2_all_pow_arm_random() {
    let p = Pair::load();
    p.check_all("C2", random_in(SEED ^ 2, N, POW, POW, POW));
}

/// C3: linear arm on R only.
#[test]
fn c3_linear_r_only() {
    let p = Pair::load();
    p.check_all("C3", random_in(SEED ^ 3, N, LIN, POW, POW));
}

/// C4: linear arm on G only.
#[test]
fn c4_linear_g_only() {
    let p = Pair::load();
    p.check_all("C4", random_in(SEED ^ 4, N, POW, LIN, POW));
}

/// C5: linear arm on B only.
#[test]
fn c5_linear_b_only() {
    let p = Pair::load();
    p.check_all("C5", random_in(SEED ^ 5, N, POW, POW, LIN));
}

/// C6: linear arm on R and G.
#[test]
fn c6_linear_r_and_g() {
    let p = Pair::load();
    p.check_all("C6", random_in(SEED ^ 6, N, LIN, LIN, POW));
}

/// C7: linear arm on R and B.
#[test]
fn c7_linear_r_and_b() {
    let p = Pair::load();
    p.check_all("C7", random_in(SEED ^ 7, N, LIN, POW, LIN));
}

/// C8: linear arm on G and B.
#[test]
fn c8_linear_g_and_b() {
    let p = Pair::load();
    p.check_all("C8", random_in(SEED ^ 8, N, POW, LIN, LIN));
}

// ---------------------------------------------------------------- C9
/// C9: EXHAUSTIVE sweep of the entire 2^24 input domain.
///
/// The public API takes a 3-byte struct by value, so this is the complete set
/// of inputs the library can ever be given. Passing this row means the two
/// implementations are provably identical functions, not merely
/// indistinguishable on a sample.
///
/// Also records, for free, which `cbDenorm` outcomes are reachable (feeding the
/// `ERRORS.md` E1/E2/E3 claims).
#[test]
fn c9_exhaustive_all_16m_inputs() {
    let p = Pair::load();
    let mut diverged = Vec::new();
    let mut n: u64 = 0;
    // Coverage counters for the cbDenorm outcome axis.
    let (mut in_range, mut neg_wrap, mut over_wrap, mut indefinite) = (0u64, 0u64, 0u64, 0u64);

    for r in 0u16..=255 {
        for g in 0u16..=255 {
            for b in 0u16..=255 {
                let v = CbRgb255::new(r as u8, g as u8, b as u8);
                let cv = p.c(v);
                let rv = p.rust(v);
                if cv != rv && diverged.len() < 20 {
                    diverged.push((v, cv, rv));
                }
                n += 1;
                for pre in common::pre_denorm(v) {
                    if pre.is_nan() || pre.trunc() < -2147483648.0 || pre.trunc() >= 2147483648.0 {
                        indefinite += 1;
                    } else if pre < 0.0 {
                        neg_wrap += 1;
                    } else if pre >= 256.0 {
                        over_wrap += 1;
                    } else {
                        in_range += 1;
                    }
                }
            }
        }
    }

    assert!(
        diverged.is_empty(),
        "EXHAUSTIVE DIVERGENCE: first {:?}",
        diverged
    );
    assert_eq!(n, 1 << 24, "swept {n} inputs, expected 16777216");

    println!(
        "C9 exhaustive: {n} inputs OK. cbDenorm channel outcomes: \
         in_range={in_range} neg_wrap={neg_wrap} over_wrap={over_wrap} indefinite={indefinite}"
    );
    // ERRORS.md E1 and E2 must be genuinely reachable, E3 must not be.
    assert!(neg_wrap > 0, "E1 (negative wrap) proved unreachable?");
    assert!(over_wrap > 0, "E2 (over-255 wrap) proved unreachable?");
    assert_eq!(indefinite, 0, "E3 claimed unreachable but was reached");
}

// ---------------------------------------------------------------- C10
/// C10: apply-gamma LINEAR arm on red, i.e. post-matrix red < 0.0031308.
/// Needs `B > G` with small `R`. Asserts the configuration is really hit.
#[test]
fn c10_apply_gamma_linear_arm_on_red() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 10);
    let mut inputs = Vec::with_capacity(N);
    let mut hits = 0;
    while inputs.len() < N {
        let r = rng.range_u8(0, 20);
        let g = rng.range_u8(0, 200);
        let b = rng.range_u8(0, 255);
        if b <= g {
            continue;
        }
        let v = CbRgb255::new(r, g, b);
        if common::pre_denorm(v)[0] < 0.5 {
            // pre = apply_gamma(red)*255+0.5 < 0.5  <=>  apply_gamma(red) < 0
            hits += 1;
        }
        inputs.push(v);
    }
    p.check_all("C10", inputs);
    assert!(hits > 100, "C10 only hit the negative-red arm {hits} times");
    println!("C10: {hits}/{N} inputs took the negative apply-gamma arm on red");
}

// ---------------------------------------------------------------- C11
/// C11: post-matrix red > 1.0, so `cbDenorm` overflows past 255 and wraps.
/// Needs `G > B` with large `R`.
#[test]
fn c11_denorm_overflow_past_255() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 11);
    let mut inputs = Vec::with_capacity(N);
    let mut hits = 0;
    while inputs.len() < N {
        let r = rng.range_u8(235, 255);
        let g = rng.range_u8(0, 255);
        let b = rng.range_u8(0, 255);
        if g <= b {
            continue;
        }
        let v = CbRgb255::new(r, g, b);
        if common::pre_denorm(v)[0] >= 256.0 {
            hits += 1;
        }
        inputs.push(v);
    }
    p.check_all("C11", inputs);
    assert!(hits > 100, "C11 only hit the >255 wrap {hits} times");
    println!("C11: {hits}/{N} inputs overflowed the red byte past 255");
}

// ---------------------------------------------------------------- C12
/// C12: apply-gamma linear arm on green AND blue at once (`G,B in {0,1}`),
/// R swept over its full range. Exhaustive: 256 * 2 * 2 = 1024.
#[test]
fn c12_apply_gamma_linear_green_and_blue() {
    let p = Pair::load();
    let inputs = (0u16..=255)
        .flat_map(|r| (0u8..=1).flat_map(move |g| (0u8..=1).map(move |b| CbRgb255::new(r as u8, g, b))));
    let n = p.check_all("C12", inputs);
    assert_eq!(n, 256 * 2 * 2);
}

// ---------------------------------------------------------------- C13
/// C13: `cbDenorm` negative wrap genuinely observed - the pure-blue family
/// (`G = 0`, large `B`) drives red strongly negative.
#[test]
fn c13_denorm_negative_wrap() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 13);
    let mut inputs = Vec::with_capacity(N);
    let mut hits = 0;
    for _ in 0..N {
        let v = CbRgb255::new(rng.range_u8(0, 255), 0, rng.range_u8(128, 255));
        if common::pre_denorm(v)[0] < 0.0 {
            hits += 1;
        }
        inputs.push(v);
    }
    p.check_all("C13", inputs);
    assert!(hits > 100, "C13 only hit the negative wrap {hits} times");

    // The documented concrete case from ERRORS.md E1: pure blue.
    let blue = CbRgb255::new(0, 0, 255);
    let out = p.check(blue);
    assert_ne!(
        out.r, 0,
        "pure blue's red channel should WRAP (not clamp to 0); got {out:?}"
    );
    println!("C13: {hits}/{N} negative wraps; tritanopia(0,0,255) = {out:?}");
}

// ---------------------------------------------------------------- C14
/// C14: remove-gamma threshold crossing. `> 0.04045` is strict and
/// `0.04045 * 255 = 10.31`, so 10 -> linear and 11 -> pow. Exhaustive over
/// each channel in {9, 10, 11, 12}.
#[test]
fn c14_remove_gamma_threshold_crossing() {
    let p = Pair::load();
    const EDGE: [u8; 4] = [9, 10, 11, 12];
    let inputs = EDGE
        .iter()
        .flat_map(|&r| EDGE.iter().flat_map(move |&g| EDGE.iter().map(move |&b| CbRgb255::new(r, g, b))));
    let n = p.check_all("C14", inputs);
    assert_eq!(n, 64);
}

// ---------------------------------------------------------------- C15
/// C15: ABI - the 3-byte struct rides in a 64-bit register, so a caller may
/// leave arbitrary garbage in the upper 5 bytes. Called through a raw
/// `u64 -> u64` signature. Only the low 3 result bytes are architecturally
/// defined, so only those are compared; the point is that both sides IGNORE
/// the padding identically.
#[test]
fn c15_abi_garbage_in_padding_bytes() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 15);
    for _ in 0..N {
        let payload = rng.next_u64() & 0x00FF_FFFF;
        let garbage = rng.next_u64() & 0xFFFF_FFFF_FF00_0000;
        let dirty = payload | garbage;

        let c_clean = p.c_raw(payload) & 0xFF_FFFF;
        let c_dirty = p.c_raw(dirty) & 0xFF_FFFF;
        let r_clean = p.rust_raw(payload) & 0xFF_FFFF;
        let r_dirty = p.rust_raw(dirty) & 0xFF_FFFF;

        assert_eq!(c_clean, c_dirty, "C itself was affected by padding bits");
        assert_eq!(
            c_dirty, r_dirty,
            "padding-bits divergence: input {dirty:#018x} C={c_dirty:#08x} Rust={r_dirty:#08x}"
        );
        assert_eq!(c_clean, r_clean);
    }
}

// ---------------------------------------------------------------- C16
/// C16: grayscale `R == G == B`, exhaustive over all 256.
#[test]
fn c16_grayscale_exhaustive() {
    let p = Pair::load();
    let n = p.check_all("C16", (0u16..=255).map(|v| CbRgb255::new(v as u8, v as u8, v as u8)));
    assert_eq!(n, 256);
}

// ---------------------------------------------------------------- C17
/// C17: sparse shapes - exactly one non-zero channel, exactly two non-zero
/// channels, and the 8 corners of `{0,255}^3`.
#[test]
fn c17_sparse_shapes_and_corners() {
    let p = Pair::load();
    let mut inputs = Vec::new();

    // exactly one non-zero channel
    for v in 1u16..=255 {
        let v = v as u8;
        inputs.push(CbRgb255::new(v, 0, 0));
        inputs.push(CbRgb255::new(0, v, 0));
        inputs.push(CbRgb255::new(0, 0, v));
    }
    // exactly two non-zero channels (paired sweep)
    for v in 1u16..=255 {
        let v = v as u8;
        let w = (256 - v as u16) as u8;
        inputs.push(CbRgb255::new(v, w, 0));
        inputs.push(CbRgb255::new(v, 0, w));
        inputs.push(CbRgb255::new(0, v, w));
    }
    // the 8 corners
    for &r in &[0u8, 255] {
        for &g in &[0u8, 255] {
            for &b in &[0u8, 255] {
                inputs.push(CbRgb255::new(r, g, b));
            }
        }
    }

    let n = p.check_all("C17", inputs);
    assert_eq!(n, 255 * 3 + 255 * 3 + 8);
}
