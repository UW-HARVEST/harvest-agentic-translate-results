//! Shared corpus/mutation helpers for the fuzz targets.
//!
//! This is the safety net for the rows of `ERRORS.md` that cannot be
//! constructed by hand (A7 `count <= 64` and A8 `cp_would_overflow`, which need
//! a retuned extra-bits table *and* a nearly-exhausted stream), and for every
//! interaction of options and data shapes the hand-written rows miss.
//!
//! `fuzz_same` only compares inputs on which the C library is self-consistent
//! across two runs, which filters out the C's layout-dependent undefined
//! behaviour (chiefly `cp_stored`'s `memcpy`, which ignores `out_end` and can
//! smash the heap).

#![allow(dead_code)]

use super::harness::make::*;
use super::harness::*;

/// Big enough that a stored block's unchecked `memcpy` (up to 65535 bytes)
/// lands in our own allocation instead of the heap's metadata.
pub const OUT_SLACK: usize = 70_000;

// Keep the digest cheap: only the first `HASH_CAP` bytes of a large output blob
// need hashing to distinguish streams (the rest is untouched filler).

pub fn fuzz_inflate(label: String, input: Vec<u8>, align: usize, out_bytes: i32) -> Case {
    let in_bytes = input.len() as i32;
    Case {
        label,
        mutations: Vec::new(),
        digest: true,
        call: Call::Inflate {
            input,
            in_bytes,
            align,
            out_bytes,
            out_slack: OUT_SLACK,
        },
    }
}

/// A corpus of well-formed DEFLATE streams to mutate.
pub fn corpus(rng: &mut Rng) -> Vec<Vec<u8>> {
    let codes = Codes::fixed();
    let mut out = Vec::new();
    // stored
    for len in [1usize, 4, 17, 100] {
        let mut bw = BitW::new();
        block_stored(&mut bw, true, &rng.bytes(len));
        out.push(bw.finish());
    }
    // fixed, literals and matches
    for n in [1usize, 8, 40, 200] {
        let toks: Vec<Tok> = rng.bytes(n).iter().map(|b| Tok::Lit(*b)).collect();
        let mut bw = BitW::new();
        block_fixed(&mut bw, true, &toks, &codes);
        out.push(bw.finish());
    }
    for _ in 0..4 {
        let mut toks: Vec<Tok> = (0..60u8).map(Tok::Lit).collect();
        toks.push(Tok::Match(rng.range(3, 258), rng.range(1, 60)));
        toks.push(Tok::Match(rng.range(3, 20), 1));
        let mut bw = BitW::new();
        block_fixed(&mut bw, true, &toks, &codes);
        out.push(bw.finish());
    }
    // dynamic
    for enc in [ClEncoding::Literal, ClEncoding::RunLength] {
        for _ in 0..4 {
            let mut toks: Vec<Tok> = rng.bytes(80).iter().map(|b| Tok::Lit(*b)).collect();
            toks.push(Tok::Match(rng.range(3, 100), rng.range(1, 70)));
            let (lit, dst) = random_codes_for(rng, &toks);
            let mut bw = BitW::new();
            block_dynamic(
                &mut bw, true, &toks, &lit, &dst, &PERMUTATION_ORDER, enc, rng,
            );
            out.push(bw.finish());
        }
    }
    // multi-block
    for _ in 0..4 {
        let mut bw = BitW::new();
        for b in 0..3 {
            let toks: Vec<Tok> = rng.bytes(20).iter().map(|x| Tok::Lit(*x)).collect();
            block_fixed(&mut bw, b == 2, &toks, &codes);
        }
        out.push(bw.finish());
    }
    out
}

pub fn mutate(rng: &mut Rng, base: &[u8]) -> Vec<u8> {
    let mut v = base.to_vec();
    match rng.below(6) {
        0 => {
            // single byte flip
            if !v.is_empty() {
                let i = rng.below(v.len() as u32) as usize;
                v[i] ^= 1 << rng.below(8);
            }
        }
        1 => {
            // several random byte overwrites
            for _ in 0..rng.range(1, 6) {
                if !v.is_empty() {
                    let i = rng.below(v.len() as u32) as usize;
                    v[i] = rng.byte();
                }
            }
        }
        2 => {
            // truncate
            let keep = rng.below(v.len().max(1) as u32) as usize;
            v.truncate(keep);
        }
        3 => {
            // append garbage
            let n = rng.range(1, 16) as usize;
            v.extend(rng.bytes(n));
        }
        4 => {
            // splice: replace a slice with random bytes
            if v.len() > 2 {
                let a = rng.below(v.len() as u32) as usize;
                let b = (a + rng.range(1, 8) as usize).min(v.len());
                let repl = rng.bytes(b - a);
                v.splice(a..b, repl);
            }
        }
        _ => {
            // completely random stream
            let n = rng.range(1, 64) as usize;
            v = rng.bytes(n);
        }
    }
    v
}

/// Mutating the IHDR width/height can ask for a ~2 GiB `img.pix` and then a
/// multi-billion-iteration `cp_convert`, which is far too slow to fuzz (and is
/// already covered exactly by `ERRORS.md` rows 11-13). Restore the original
/// dimensions when a mutation blew them up.
pub fn clamp_image_size(mut m: Vec<u8>, base: &[u8]) -> Vec<u8> {
    if m.len() >= 24 && base.len() >= 24 && &m[12..16] == b"IHDR" {
        let w = u32::from_be_bytes(m[16..20].try_into().unwrap()) as u64;
        let h = u32::from_be_bytes(m[20..24].try_into().unwrap()) as u64;
        if (w + 1) * h * 4 > 4_000_000 {
            let orig: Vec<u8> = base[16..24].to_vec();
            m[16..24].copy_from_slice(&orig);
        }
    }
    m
}

