//! Reconnaissance: report ALL divergences found (not just the first), so a
//! whole class of bugs is visible in one run instead of one-at-a-time.
//! Kept in the suite as a broad backstop for Phases B and C.

mod common;

use common::*;

/// Call both impls element-by-element (count=1) and collect every divergence.
fn scan(label: &str, triples: &[[f32; 3]]) -> usize {
    let p = pair();
    let mut diverged = 0usize;
    let mut shown = 0usize;
    for t in triples {
        let src = *t;
        let mut dc = [f32::from_bits(CANARY); 2];
        let mut dr = [f32::from_bits(CANARY); 2];
        unsafe {
            (p.c.tfm)(dc.as_mut_ptr(), src.as_ptr(), 1);
            (p.rs.tfm)(dr.as_mut_ptr(), src.as_ptr(), 1);
        }
        if dc[0].to_bits() != dr[0].to_bits() || dc[1].to_bits() != dr[1].to_bits() {
            diverged += 1;
            if shown < 40 {
                shown += 1;
                eprintln!(
                    "DIVERGE [{label}] src=[{}, {}, {}]\n   C   = [{}, {}]\n   Rust= [{}, {}]",
                    fmt_f32(src[0]),
                    fmt_f32(src[1]),
                    fmt_f32(src[2]),
                    fmt_f32(dc[0]),
                    fmt_f32(dc[1]),
                    fmt_f32(dr[0]),
                    fmt_f32(dr[1]),
                );
            }
        }
    }
    eprintln!(
        "[{label}] {} / {} triples diverged",
        diverged,
        triples.len()
    );
    diverged
}

#[test]
fn recon_exhaustive_special_alphabet() {
    let a = alphabet_f32();
    let mut triples = Vec::with_capacity(a.len().pow(3));
    for &x in &a {
        for &y in &a {
            for &z in &a {
                triples.push([x, y, z]);
            }
        }
    }
    assert_eq!(triples.len(), 24 * 24 * 24);
    let d = scan("alphabet^3", &triples);
    assert_eq!(d, 0, "{d} divergences over the exhaustive special alphabet");
}

#[test]
fn recon_random_bit_patterns() {
    let mut rng = Rng::new(0xC0FFEE_1234_5678);
    let triples: Vec<[f32; 3]> = (0..200_000)
        .map(|_| {
            [
                rng.any_bits_f32(),
                rng.any_bits_f32(),
                rng.any_bits_f32(),
            ]
        })
        .collect();
    let d = scan("random-bits", &triples);
    assert_eq!(d, 0, "{d} divergences over random bit patterns");
}

#[test]
fn recon_mixed_generators() {
    let mut rng = Rng::new(0xABCD_0F0F_1111);
    let mut triples = Vec::new();
    for _ in 0..200_000 {
        let lane = |r: &mut Rng| match r.below(8) {
            0 => r.signed_unit(),
            1 => r.wild_normal(),
            2 => r.subnormal(),
            3 => r.huge(),
            4 => r.any_nan(),
            5 => {
                if r.next_u32() & 1 == 0 {
                    f32::INFINITY
                } else {
                    f32::NEG_INFINITY
                }
            }
            6 => {
                if r.next_u32() & 1 == 0 {
                    0.0
                } else {
                    -0.0
                }
            }
            _ => r.any_bits_f32(),
        };
        triples.push([lane(&mut rng), lane(&mut rng), lane(&mut rng)]);
    }
    let d = scan("mixed", &triples);
    assert_eq!(d, 0, "{d} divergences over mixed generators");
}
