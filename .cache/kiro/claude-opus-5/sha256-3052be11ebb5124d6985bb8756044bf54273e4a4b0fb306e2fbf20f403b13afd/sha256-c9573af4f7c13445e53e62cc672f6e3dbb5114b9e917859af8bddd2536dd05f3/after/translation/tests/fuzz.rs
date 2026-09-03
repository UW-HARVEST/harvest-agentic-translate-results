//! CONFIGS.md row C42 — high-volume differential fuzzing.
//!
//! Five strategies, all with a fixed seed:
//!   1. uniform random bytes;
//!   2. bit-flip / byte-splice mutation of *valid* streams (structure aware, so
//!      the mutants get deep into `cp_dynamic` / `cp_build` / `cp_block`);
//!   3. valid streams with randomly poked writable exports;
//!   4. streams built from valid block headers with random payload bits;
//!   5. random `in_bytes` / `out_bytes` / alignment perturbation of valid streams.
//!
//! Everything is compared through the `.so` exports: return value, whole output
//! window, `cp_error_reason`, wait status and assertion site.

mod common;

use common::deflate::*;
use common::{Case, Diff, GlobalPoke};
use std::collections::BTreeMap;

fn a_valid_stream(rng: &mut common::Rng) -> (Vec<u8>, usize) {
    let nb = rng.range(1, 3) as usize;
    let mut w = BitWriter::new();
    let mut all: Vec<Tok> = Vec::new();
    let mut have = 0u32;
    for i in 0..nb {
        let mut toks = Vec::new();
        for _ in 0..rng.range(0, 40) {
            if have >= 4 && rng.below(100) < 30 {
                let dist = rng.range(1, have.min(2048));
                let len = rng.range(3, 258);
                toks.push(Tok::Match { len, dist });
                have += len;
            } else {
                toks.push(Tok::Lit(rng.byte()));
                have += 1;
            }
        }
        let last = i == nb - 1;
        match rng.below(3) {
            0 => emit_fixed(&mut w, last, &toks),
            _ => {
                let o = DynOpts {
                    min_hlit: if rng.below(3) == 0 { 288 } else { 257 },
                    min_hdist: if rng.below(3) == 0 { 32 } else { 1 },
                    full_hclen: rng.below(2) == 0,
                    uniform_weights: rng.below(2) == 0,
                    no_rle: rng.below(4) == 0,
                    ..DynOpts::default()
                };
                emit_dynamic(&mut w, last, &toks, &o);
            }
        }
        all.extend_from_slice(&toks);
    }
    let e = expand(&all);
    (w.bytes(), e.len())
}

fn random_poke(rng: &mut common::Rng) -> GlobalPoke {
    match rng.below(6) {
        0 => GlobalPoke::FixedTable(rng.below(320) as usize, rng.below(18) as u8),
        1 => GlobalPoke::PermutationOrder(rng.below(19) as usize, rng.below(24) as u8),
        2 => GlobalPoke::LenExtraBits(rng.below(31) as usize, rng.below(40) as u8),
        3 => GlobalPoke::LenBase(rng.below(31) as usize, rng.u32() >> rng.below(24)),
        4 => GlobalPoke::DistExtraBits(rng.below(32) as usize, rng.below(40) as u8),
        _ => GlobalPoke::DistBase(rng.below(32) as usize, rng.u32() >> rng.below(24)),
    }
}

#[test]
fn fuzz_c42() {
    let mut d = Diff::new();
    let mut rng = common::Rng::new(0x5EED_1234_C42);
    let mut sites: BTreeMap<String, usize> = BTreeMap::new();
    let mut softs: BTreeMap<String, usize> = BTreeMap::new();
    let mut oks = 0usize;
    let mut segv = 0usize;

    let tally = |o: &common::Outcome,
                     sites: &mut BTreeMap<String, usize>,
                     softs: &mut BTreeMap<String, usize>,
                     oks: &mut usize,
                     segv: &mut usize| {
        if let Some(a) = &o.assert_site {
            *sites.entry(a.clone()).or_insert(0) += 1;
        } else if o.signal == Some(libc::SIGSEGV) {
            *segv += 1;
        } else if o.signal.is_none() && o.ret == 0 {
            let m = String::from_utf8_lossy(o.err.as_deref().unwrap_or(b"<null>")).into_owned();
            *softs.entry(m).or_insert(0) += 1;
        } else if o.signal.is_none() && o.ret == 1 {
            *oks += 1;
        }
    };

    // ---- strategy 1: uniform random bytes -------------------------------
    let b = d.row_start("C42.1 uniform random bytes");
    for _ in 0..3000 {
        let n = rng.range(1, 64) as usize;
        let input = rng.bytes(n);
        let case = Case::new(input, [0i32, 1, 7, 64, 700][rng.below(5) as usize])
            .in_align(rng.below(4) as usize)
            .out_align(rng.below(4) as usize);
        let o = d.check("C42.1", "random bytes", &case);
        tally(&o, &mut sites, &mut softs, &mut oks, &mut segv);
    }
    d.row_end(b);

    // ---- strategy 2: mutated valid streams ------------------------------
    let b = d.row_start("C42.2 bit-flip / splice mutation of valid streams");
    for _ in 0..6000 {
        let (mut s, need) = a_valid_stream(&mut rng);
        if s.is_empty() {
            continue;
        }
        let nmut = rng.range(1, 4);
        for _ in 0..nmut {
            let i = rng.below(s.len() as u32) as usize;
            match rng.below(4) {
                0 => s[i] ^= 1 << rng.below(8),
                1 => s[i] = rng.byte(),
                2 => s[i] = 0,
                _ => s[i] = 0xFF,
            }
        }
        if rng.below(4) == 0 {
            s.truncate((rng.below(s.len() as u32) as usize).max(1));
        }
        let ob = match rng.below(4) {
            0 => need as i32,
            1 => need as i32 + rng.below(64) as i32,
            2 => (need as i32 - rng.below(8) as i32).max(0),
            _ => rng.below(2048) as i32,
        };
        let case = Case::new(s, ob)
            .in_align(rng.below(4) as usize)
            .out_align(rng.below(4) as usize)
            .out_fill(rng.byte());
        let o = d.check("C42.2", "mutated valid stream", &case);
        tally(&o, &mut sites, &mut softs, &mut oks, &mut segv);
    }
    d.row_end(b);

    // ---- strategy 3: valid streams + poked globals -----------------------
    let b = d.row_start("C42.3 valid streams with randomly poked writable exports");
    for _ in 0..4000 {
        let (s, need) = a_valid_stream(&mut rng);
        let mut case = Case::new(s, need as i32 + rng.below(32) as i32)
            .in_align(rng.below(4) as usize)
            .out_align(rng.below(4) as usize);
        for _ in 0..rng.range(1, 3) {
            let p = random_poke(&mut rng);
            case = case.poke(p);
        }
        let o = d.check("C42.3", "poked globals", &case);
        tally(&o, &mut sites, &mut softs, &mut oks, &mut segv);
    }
    d.row_end(b);

    // ---- strategy 4: valid headers, random body bits ---------------------
    let b = d.row_start("C42.4 well-formed block headers with random body bits");
    for _ in 0..3000 {
        let mut w = BitWriter::new();
        let nb = rng.range(1, 3);
        for i in 0..nb {
            w.push((i == nb - 1) as u32, 1);
            w.push(rng.range(0, 2), 2); // btype 0..2, never the reserved 3
            for _ in 0..rng.range(1, 24) {
                w.push(rng.u32(), rng.range(1, 16));
            }
        }
        let s = w.bytes();
        let case = Case::new(s, [0i32, 4, 64, 4096][rng.below(4) as usize])
            .in_align(rng.below(4) as usize);
        let o = d.check("C42.4", "random body bits", &case);
        tally(&o, &mut sites, &mut softs, &mut oks, &mut segv);
    }
    d.row_end(b);

    // ---- strategy 5: length/alignment perturbation of valid streams ------
    let b = d.row_start("C42.5 in_bytes / out_bytes / alignment perturbation");
    for _ in 0..3000 {
        let (s, need) = a_valid_stream(&mut rng);
        let real = s.len() as i32;
        let nb = match rng.below(5) {
            0 => real,
            1 => (real - rng.below(4) as i32).max(0),
            2 => real + rng.below(8) as i32,
            3 => 0,
            _ => -(rng.below(8) as i32),
        };
        let ob = match rng.below(4) {
            0 => need as i32,
            1 => -(rng.below(16) as i32),
            2 => 0,
            _ => need as i32 + rng.below(128) as i32,
        };
        let case = Case::new(s, ob)
            .in_bytes(nb)
            .in_align(rng.below(4) as usize)
            .out_align(rng.below(4) as usize);
        let o = d.check("C42.5", "perturbed lengths", &case);
        tally(&o, &mut sites, &mut softs, &mut oks, &mut segv);
    }
    d.row_end(b);

    println!("\n--- fuzz outcome histogram ---");
    println!("ret==1: {oks}   SIGSEGV: {segv}");
    for (k, v) in &sites {
        println!("  {v:6}x  {k}");
    }
    for (k, v) in &softs {
        println!("  {v:6}x  soft: {k}");
    }

    d.finish("CONFIGS.md C42 (differential fuzzing)");
}
