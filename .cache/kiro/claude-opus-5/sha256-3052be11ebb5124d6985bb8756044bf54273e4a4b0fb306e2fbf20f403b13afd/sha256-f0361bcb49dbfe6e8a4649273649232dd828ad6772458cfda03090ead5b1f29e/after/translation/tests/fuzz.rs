//! High-volume independent fuzz campaign.
//!
//! Uses seeds disjoint from `configs.rs` / `errors.rs` so that it is a genuine
//! additional sample rather than a re-run.  Every case is compared through both
//! `.so` exports; any divergence in return value, output bytes,
//! `cp_error_reason` or fatal signal fails the test.

mod common;
use common::*;

/// Overridable so the campaign can be scaled up outside the normal test run:
/// `PINFLATE_FUZZ_N=50000 PINFLATE_FUZZ_SEED=7 cargo test --test fuzz`.
fn campaign_size() -> usize {
    std::env::var("PINFLATE_FUZZ_N")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(6000)
}

fn campaign_seed() -> u64 {
    std::env::var("PINFLATE_FUZZ_SEED")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0xA11CE_5EED)
}

fn shapes(rng: &mut Rng) -> (Vec<u8>, usize) {
    // mix of: pure random bytes, valid streams, valid streams with mutated
    // bytes, and valid streams with mutated bits
    match rng.below(6) {
        0 => {
            let n = rng.range(0, 64) as usize;
            (rng.bytes(n), rng.range(0, 4096) as usize)
        }
        1 => {
            let n = rng.range(1, 40) as usize;
            let toks: Vec<Tok> = (0..n).map(|_| Tok::Lit(rng.byte())).collect();
            let expect = simulate(&[], &toks);
            let mut w = BitWriter::new();
            fixed_block(&mut w, true, &toks);
            (w.finish(4), expect.len())
        }
        2 => {
            // valid fixed stream with k random bytes flipped
            let n = rng.range(1, 40) as usize;
            let mut toks: Vec<Tok> = (0..n).map(|_| Tok::Lit(rng.byte())).collect();
            if n >= 4 {
                let d = rng.range(1, n as u64) as u32;
                toks.push(match_tok(rng.range(3, 60) as u32, d));
            }
            let mut w = BitWriter::new();
            fixed_block(&mut w, true, &toks);
            let mut s = w.finish(4);
            let k = rng.range(1, 3) as usize;
            for _ in 0..k {
                if !s.is_empty() {
                    let i = rng.below(s.len() as u64) as usize;
                    s[i] = rng.byte();
                }
            }
            (s, rng.range(0, 4096) as usize)
        }
        3 => {
            // valid fixed stream with single bits flipped
            let n = rng.range(1, 30) as usize;
            let toks: Vec<Tok> = (0..n).map(|_| Tok::Lit(rng.byte())).collect();
            let mut w = BitWriter::new();
            fixed_block(&mut w, true, &toks);
            let mut s = w.finish(4);
            let k = rng.range(1, 4) as usize;
            for _ in 0..k {
                if !s.is_empty() {
                    let i = rng.below(s.len() as u64) as usize;
                    s[i] ^= 1u8 << rng.below(8);
                }
            }
            (s, rng.range(0, 4096) as usize)
        }
        4 => {
            // real deflate output, then truncated at a random point
            use flate2::write::DeflateEncoder;
            use flate2::Compression;
            use std::io::Write;
            let n = rng.range(0, 1500) as usize;
            let data: Vec<u8> = (0..n).map(|i| ((i * 7) % 61) as u8).collect();
            let mut e = DeflateEncoder::new(Vec::new(), Compression::new(rng.below(10) as u32));
            e.write_all(&data).unwrap();
            let mut s = e.finish().unwrap();
            if !s.is_empty() && rng.below(2) == 0 {
                let cut = rng.range(1, s.len() as u64) as usize;
                s.truncate(cut);
            }
            for _ in 0..4 {
                s.push(0);
            }
            (s, data.len().max(1))
        }
        _ => {
            // stored blocks with random LEN/NLEN relationships
            let len = rng.range(0, 200) as usize;
            let payload = rng.bytes(len);
            let mut w = BitWriter::new();
            w.bits(1, 1);
            w.bits(0, 2);
            w.align();
            let l = len as u16;
            w.bits(l as u32, 16);
            let nl = if rng.below(2) == 0 {
                !l
            } else {
                rng.next_u64() as u16
            };
            w.bits(nl as u32 & 0xFFFF, 16);
            w.raw(&payload);
            let extra = rng.below(8) as usize;
            let mut s = w.buf;
            for _ in 0..extra {
                s.push(0);
            }
            (s, len + 32)
        }
    }
}

#[test]
fn fuzz_campaign() {
    let p = Pair::new();
    let n_cases = campaign_size();
    let mut rng = Rng::new(campaign_seed());
    let mut sigs = 0usize;
    let mut errs = 0usize;
    let mut oks = 0usize;
    let mut by_sig: std::collections::BTreeMap<i32, usize> = Default::default();
    for i in 0..n_cases {
        let (stream, out_bytes) = shapes(&mut rng);
        let case = Case::new(stream, 0)
            .out_bytes(out_bytes as i32)
            .align(rng.below(4) as usize);
        match p.check2(&format!("campaign #{i}"), &case).0 {
            Outcome::Signal(x) => {
                sigs += 1;
                *by_sig.entry(x).or_default() += 1;
            }
            Outcome::Ret { ret: 0, .. } => errs += 1,
            Outcome::Ret { .. } => oks += 1,
            Outcome::Broken(m) => panic!("broken child: {m}"),
        }
    }
    eprintln!(
        "fuzz_campaign: {n_cases} cases -> {oks} successes, {errs} rejections, \
         {sigs} fatal ({by_sig:?})"
    );
    assert!(oks > 100 && errs > 100 && sigs > 100, "poor outcome spread");
}
