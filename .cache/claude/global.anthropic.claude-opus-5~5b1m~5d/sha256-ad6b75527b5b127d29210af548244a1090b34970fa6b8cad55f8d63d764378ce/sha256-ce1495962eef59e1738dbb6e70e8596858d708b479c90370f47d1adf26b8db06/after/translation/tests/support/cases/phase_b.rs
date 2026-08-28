//! Phase B — valid-path differential cases, one generator per `CONFIGS.md` row
//! group.  Every generator is seeded deterministically.

#![allow(dead_code)]

use super::super::deflate::*;
use super::super::{Case, Expect, Rng, Tbl};
use super::{case_exact, pad_to_last_bytes, random_fixed_stream};

pub const IDS: &[&str] = &[
    "b_align",            // rows 1-16
    "b_stored",           // rows 17-20
    "b_stored_folded",    // row 19 (final-word fold quirk)
    "b_stored_overflow",  // row 21
    "b_fixed_lit",        // rows 22-24
    "b_len_syms",         // rows 25, 27, 29
    "b_dist_syms",        // rows 26, 28
    "b_copy_paths",       // rows 30-33
    "b_big",              // row 34
    "b_dyn_basic",        // rows 35-37
    "b_dyn_rle",          // rows 38-41
    "b_dyn_lens",         // rows 42-46
    "b_dyn_random",       // rows 47-49
    "b_out_sizes",        // rows 50-52
    "b_tables",           // rows 53-59
    "b_reason",           // row 60
    "b_multi",            // rows 61-65
    "b_property",         // row 66
    "b_repeat_calls",     // row 70
];

pub fn build(id: &str) -> Vec<Case> {
    match id {
        "b_align" => b_align(),
        "b_stored" => b_stored(),
        "b_stored_folded" => b_stored_folded(),
        "b_stored_overflow" => b_stored_overflow(),
        "b_fixed_lit" => b_fixed_lit(),
        "b_len_syms" => b_len_syms(),
        "b_dist_syms" => b_dist_syms(),
        "b_copy_paths" => b_copy_paths(),
        "b_big" => b_big(),
        "b_dyn_basic" => b_dyn_basic(),
        "b_dyn_rle" => b_dyn_rle(),
        "b_dyn_lens" => b_dyn_lens(),
        "b_dyn_random" => b_dyn_random(),
        "b_out_sizes" => b_out_sizes(),
        "b_tables" => b_tables(),
        "b_reason" => b_reason(),
        "b_multi" => b_multi(),
        "b_property" => b_property(),
        "b_repeat_calls" => b_repeat_calls(),
        _ => unreachable!(),
    }
}

// ---------------------------------------------------------------------------
// rows 1-16: every (first_bytes, last_bytes) combination
// ---------------------------------------------------------------------------

fn b_align() -> Vec<Case> {
    let t = Tables::default();
    let mut cases = Vec::new();
    for align in 0..4usize {
        let fb = first_bytes_for(align);
        for want_last in 0..4usize {
            for seed in 0..40u64 {
                let mut rng = Rng::new(0x0100_0000 + seed + 16 * (align as u64) + 4 * want_last as u64);
                let n = rng.range(0, 40);
                let (mut toks, _) = random_fixed_stream(&mut rng, n);
                // sprinkle one match in when there is material for it
                if n >= 4 {
                    toks.pop();
                    toks.push(Tok::Match { ls: 257, lx: 0, ds: 0, dx: 0 });
                    toks.push(Tok::End);
                }
                let mut s = Stream::new();
                s.fixed_block(true, &toks, &t);
                let (mut input, exp, olen) = s.finish(fb, false, 0);
                pad_to_last_bytes(&mut input, align, want_last, rng.byte());
                let label = format!("fb={fb} last={want_last} n={n}");
                cases.push(case_exact(label, input, exp, olen, align));
            }
        }
    }
    cases
}

// ---------------------------------------------------------------------------
// rows 17-20: stored blocks (no final-word fold: last_bytes == 0)
// ---------------------------------------------------------------------------

fn b_stored() -> Vec<Case> {
    let mut cases = Vec::new();
    for align in 0..4usize {
        let fb = first_bytes_for(align);
        // header is 5 bytes, so LEN must satisfy (5 + LEN) % 4 == fb % 4
        let l0 = ((fb + 4) - 5 % 4) % 4;
        for (i, len) in [l0, l0 + 4, l0 + 8, l0 + 96, l0 + 4092, 65532 + l0]
            .into_iter()
            .enumerate()
        {
            if len > 65535 {
                continue;
            }
            let mut rng = Rng::new(0x0200_0000 + i as u64 + 8 * align as u64);
            let data: Vec<u8> = (0..len).map(|_| rng.byte()).collect();
            let mut s = Stream::new();
            s.stored_block(true, &data);
            let (input, exp, olen) = s.finish(fb, true, 0);
            assert_eq!(olen, len, "stored LEN should not need padding");
            let mut c = case_exact(format!("fb={fb} LEN={len}"), input, exp, olen, align);
            c = c.out_pad(len + 4096);
            cases.push(c);
        }
    }
    cases
}

/// A stored block whose header runs into the final partial word, so
/// `cp_peak_bits` folds `final_word` and `cp_ptr`'s `words + word_index`
/// arithmetic no longer matches the real byte position.  Pure C-vs-Rust
/// agreement (the copied region is deliberately "wrong").
fn b_stored_folded() -> Vec<Case> {
    let mut cases = Vec::new();
    for align in 0..4usize {
        let fb = first_bytes_for(align);
        for extra in 0..4usize {
            for datalen in [8usize, 16, 64] {
                let mut rng = Rng::new(0x0300_0000 + extra as u64 + 8 * align as u64 + datalen as u64);
                let data: Vec<u8> = (0..datalen).map(|_| rng.byte()).collect();
                let mut s = Stream::new();
                s.stored_block(true, &data);
                let (mut input, _, _) = s.finish(fb, false, 0);
                pad_to_last_bytes(&mut input, align, extra, rng.byte());
                // patch LEN/NLEN so that LEN covers everything after the header
                let l = input.len() - 5;
                let lb = (l as u16).to_le_bytes();
                let nb = (!(l as u16)).to_le_bytes();
                input[0 + 1] = lb[0];
                input[0 + 2] = lb[1];
                input[0 + 3] = nb[0];
                input[0 + 4] = nb[1];
                let c = Case::new(
                    format!("fb={fb} last={extra} LEN={l}"),
                    input,
                    (l + 64) as i32,
                )
                .in_align(align)
                .out_pad(l + 8192)
                .expect(Expect::Any);
                cases.push(c);
            }
        }
    }
    cases
}

// ---------------------------------------------------------------------------
// row 21: `cp_stored` has no out_end check at all
// ---------------------------------------------------------------------------

fn b_stored_overflow() -> Vec<Case> {
    let mut cases = Vec::new();
    for align in 0..4usize {
        let fb = first_bytes_for(align);
        let l0 = ((fb + 4) - 5 % 4) % 4;
        for len in [l0 + 96, l0 + 4092] {
            let mut rng = Rng::new(0x0400_0000 + len as u64 + align as u64);
            let data: Vec<u8> = (0..len).map(|_| rng.byte()).collect();
            let mut s = Stream::new();
            s.stored_block(true, &data);
            let (input, _, olen) = s.finish(fb, true, 0);
            for out_bytes in [0i32, 1, (olen / 2) as i32] {
                let c = Case::new(
                    format!("fb={fb} LEN={olen} out_bytes={out_bytes}"),
                    input.clone(),
                    out_bytes,
                )
                .in_align(align)
                .out_pad(olen + 8192)
                .expect(Expect::Ret { ret: 1, reason: None });
                cases.push(c);
            }
        }
    }
    cases
}

// ---------------------------------------------------------------------------
// rows 22-24: fixed block, literals only
// ---------------------------------------------------------------------------

fn b_fixed_lit() -> Vec<Case> {
    let t = Tables::default();
    let mut cases = Vec::new();

    // end-of-block immediately, out_bytes == 0
    for align in 0..4usize {
        let fb = first_bytes_for(align);
        let mut s = Stream::new();
        s.fixed_block(true, &[Tok::End], &t);
        let (input, exp, olen) = s.finish(fb, false, 0);
        cases.push(case_exact(format!("empty fb={fb}"), input, exp, olen, align));
    }

    // all 256 byte values, ascending and descending
    for (name, order) in [("asc", false), ("desc", true)] {
        let mut toks: Vec<Tok> = Vec::new();
        for i in 0..256u16 {
            toks.push(Tok::Lit(if order { 255 - i } else { i }));
        }
        toks.push(Tok::End);
        let mut s = Stream::new();
        s.fixed_block(true, &toks, &t);
        let (input, exp, olen) = s.finish(0, false, 0);
        cases.push(case_exact(format!("all256 {name}"), input, exp, olen, 0));
    }

    // 8-bit-code range only, then 9-bit-code range only
    for (name, lo, hi) in [("8bit", 0u16, 143u16), ("9bit", 144, 255)] {
        let mut toks: Vec<Tok> = (lo..=hi).map(Tok::Lit).collect();
        toks.push(Tok::End);
        let mut s = Stream::new();
        s.fixed_block(true, &toks, &t);
        let (input, exp, olen) = s.finish(0, false, 0);
        cases.push(case_exact(format!("range {name}"), input, exp, olen, 0));
    }

    // random literal blocks
    for seed in 0..64u64 {
        let mut rng = Rng::new(0x0500_0000 + seed);
        let n = rng.range(1, 300);
        let (toks, _) = random_fixed_stream(&mut rng, n);
        let align = rng.below(4);
        let fb = first_bytes_for(align);
        let mut s = Stream::new();
        s.fixed_block(true, &toks, &t);
        let (input, exp, olen) = s.finish(fb, false, 0);
        cases.push(case_exact(format!("rand n={n} fb={fb}"), input, exp, olen, align));
    }
    cases
}

// ---------------------------------------------------------------------------
// rows 25, 27, 29: every length symbol, including 286/287 (length 0)
// ---------------------------------------------------------------------------

fn b_len_syms() -> Vec<Case> {
    let t = Tables::default();
    let mut cases = Vec::new();
    for ls in 257u16..=287 {
        let nx = t.len_extra[(ls - 257) as usize] as u32;
        let maxx = if nx == 0 { 0 } else { (1u32 << nx) - 1 };
        let mut xs = vec![0u32, maxx];
        if maxx > 1 {
            xs.push(maxx / 2);
        }
        xs.dedup();
        for lx in xs {
            let length = t.length_of(ls, lx);
            let mut rng = Rng::new(0x0600_0000 + ls as u64 * 64 + lx as u64);
            // one literal so that distance 1 is legal
            let seed_byte = rng.byte();
            let toks = vec![
                Tok::Lit(seed_byte as u16),
                Tok::Match { ls, lx, ds: 0, dx: 0 },
                Tok::End,
            ];
            let mut s = Stream::new();
            s.fixed_block(true, &toks, &t);
            let (input, exp, olen) = s.finish(0, false, 0);
            cases.push(case_exact(
                format!("ls={ls} lx={lx} len={length} dist=1 (memset)"),
                input,
                exp,
                olen,
                0,
            ));
        }
    }
    cases
}

// ---------------------------------------------------------------------------
// rows 26, 28: every distance symbol, including 30/31 (distance 0)
// ---------------------------------------------------------------------------

fn b_dist_syms() -> Vec<Case> {
    let t = Tables::default();
    let mut cases = Vec::new();
    for ds in 0u16..=31 {
        let nx = t.dist_extra[ds as usize] as u32;
        let maxx = if nx == 0 { 0 } else { (1u32 << nx) - 1 };
        let mut xs = vec![0u32, maxx];
        if maxx > 1 {
            xs.push(maxx / 2);
        }
        xs.dedup();
        for dx in xs {
            let dist = t.distance_of(ds, dx) as usize;
            let prefix = dist.max(1);
            let mut rng = Rng::new(0x0700_0000 + ds as u64 * 1024 + dx as u64);
            let mut toks: Vec<Tok> = Vec::with_capacity(prefix + 2);
            for _ in 0..prefix {
                toks.push(Tok::Lit(rng.byte() as u16));
            }
            toks.push(Tok::Match { ls: 257, lx: 0, ds, dx });
            toks.push(Tok::End);
            let mut s = Stream::new();
            s.fixed_block(true, &toks, &t);
            let (input, exp, olen) = s.finish(0, false, 0);
            let mut c = case_exact(
                format!("ds={ds} dx={dx} dist={dist} len=3"),
                input,
                exp,
                olen,
                0,
            );
            c = c.out_pad(olen + 4096);
            cases.push(c);
        }
    }
    cases
}

// ---------------------------------------------------------------------------
// rows 30-33: the two copy paths and their boundaries
// ---------------------------------------------------------------------------

fn b_copy_paths() -> Vec<Case> {
    let t = Tables::default();
    let mut cases = Vec::new();

    // (prefix, dist, length symbol, length extra) triples
    let combos: &[(usize, u16, u32, u16, u32)] = &[
        // prefix, ds, dx, ls, lx   (dist = base(ds)+dx, len = base(ls)+lx)
        (10, 1, 0, 258, 0),  // dist 2  < len 4   overlapping
        (10, 2, 0, 260, 0),  // dist 3  < len 6
        (10, 4, 1, 285, 0),  // dist 6  < len 258
        (10, 0, 0, 285, 0),  // dist 1 memset, len 258
        (10, 3, 0, 257, 0),  // dist 4 == len ... (len 3, dist 4 > len)
        (10, 1, 0, 257, 0),  // dist 2, len 3
        (3, 2, 0, 257, 0),   // dist == prefix (out - dist == begin)
        (1, 0, 0, 257, 0),   // dist 1 == prefix
        (10, 2, 0, 257, 0),  // dist 3 == len 3
        (258, 15, 63, 285, 0), // dist 256 < len 258
    ];
    for (i, &(prefix, ds, dx, ls, lx)) in combos.iter().enumerate() {
        let dist = t.distance_of(ds, dx) as usize;
        if dist > prefix {
            continue;
        }
        let mut rng = Rng::new(0x0800_0000 + i as u64);
        let mut toks: Vec<Tok> = (0..prefix).map(|_| Tok::Lit(rng.byte() as u16)).collect();
        toks.push(Tok::Match { ls, lx, ds, dx });
        toks.push(Tok::End);
        let mut s = Stream::new();
        s.fixed_block(true, &toks, &t);
        let (input, exp, olen) = s.finish(0, false, 0);
        cases.push(case_exact(
            format!("prefix={prefix} dist={dist} len={}", t.length_of(ls, lx)),
            input,
            exp,
            olen,
            0,
        ));
    }

    // random chains of matches (all distances legal), out_bytes exact => the
    // final match ends exactly at out_end (boundary of E5)
    for seed in 0..256u64 {
        let mut rng = Rng::new(0x0900_0000 + seed);
        let mut toks: Vec<Tok> = Vec::new();
        let mut produced = 0usize;
        let n0 = rng.range(1, 20);
        for _ in 0..n0 {
            toks.push(Tok::Lit(rng.byte() as u16));
            produced += 1;
        }
        for _ in 0..rng.range(1, 12) {
            let ls = rng.range(257, 285) as u16;
            let nx = t.len_extra[(ls - 257) as usize] as u32;
            let lx = if nx == 0 { 0 } else { rng.next_u32() % (1 << nx) };
            // pick a distance symbol whose distance fits in what we produced
            let mut ds = rng.range(0, 29) as u16;
            let mut dx;
            loop {
                let nd = t.dist_extra[ds as usize] as u32;
                dx = if nd == 0 { 0 } else { rng.next_u32() % (1 << nd) };
                if (t.distance_of(ds, dx) as usize) <= produced {
                    break;
                }
                if ds == 0 {
                    dx = 0;
                    break;
                }
                ds -= 1;
            }
            if (t.distance_of(ds, dx) as usize) > produced {
                continue;
            }
            produced += t.length_of(ls, lx) as usize;
            toks.push(Tok::Match { ls, lx, ds, dx });
            if rng.bool() {
                toks.push(Tok::Lit(rng.byte() as u16));
                produced += 1;
            }
        }
        toks.push(Tok::End);
        let align = rng.below(4);
        let fb = first_bytes_for(align);
        let mut s = Stream::new();
        s.fixed_block(true, &toks, &t);
        let (input, exp, olen) = s.finish(fb, false, 0);
        let mut c = case_exact(format!("chain seed={seed} out={olen}"), input, exp, olen, align);
        c = c.out_pad(olen + 4096);
        cases.push(c);
    }
    cases
}

// ---------------------------------------------------------------------------
// row 34: large output, long distances
// ---------------------------------------------------------------------------

fn b_big() -> Vec<Case> {
    let t = Tables::default();
    let mut cases = Vec::new();
    for seed in 0..3u64 {
        let mut rng = Rng::new(0x0A00_0000 + seed);
        let mut toks: Vec<Tok> = Vec::new();
        let mut produced = 0usize;
        for _ in 0..400 {
            toks.push(Tok::Lit(rng.byte() as u16));
            produced += 1;
        }
        while produced < 70_000 {
            // longest match, largest legal distance
            let mut ds = 29u16;
            let mut dx = 8191u32;
            while t.distance_of(ds, dx) as usize > produced {
                if ds == 0 {
                    dx = 0;
                    break;
                }
                ds -= 1;
                let nd = t.dist_extra[ds as usize] as u32;
                dx = if nd == 0 { 0 } else { (1 << nd) - 1 };
            }
            toks.push(Tok::Match { ls: 285, lx: 0, ds, dx });
            produced += 258;
        }
        toks.push(Tok::End);
        let mut s = Stream::new();
        s.fixed_block(true, &toks, &t);
        let (input, exp, olen) = s.finish(0, false, 0);
        let mut c = case_exact(format!("big seed={seed} out={olen}"), input, exp, olen, 0);
        c = c.out_pad(olen + 4096);
        cases.push(c);
    }
    cases
}

// ---------------------------------------------------------------------------
// rows 35-37: dynamic-block header shapes
// ---------------------------------------------------------------------------

fn b_dyn_basic() -> Vec<Case> {
    let t = Tables::default();
    let mut cases = Vec::new();

    // HLIT=257, HDIST=1, direct code lengths, literals only
    {
        let mut rng = Rng::new(0x0B00_0001);
        let lits: Vec<u16> = vec![0, 1, 2, 3, 250, 251, 256];
        let spec = dyn_spec(&mut rng, &lits, &[], 257, 1, 15, (false, false, false), true);
        assert_eq!(spec.hlit, 257);
        assert_eq!(spec.hdist, 1);
        let mut toks: Vec<Tok> = Vec::new();
        for _ in 0..50 {
            toks.push(Tok::Lit(lits[rng.below(lits.len() - 1)]));
        }
        toks.push(Tok::End);
        let mut s = Stream::new();
        s.dynamic_block(true, &spec, &toks, &t);
        let (input, exp, olen) = s.finish(0, false, 0);
        cases.push(case_exact("hlit=257 hdist=1 direct", input, exp, olen, 0));
    }

    // HLIT=288, HDIST=32 with matches
    for seed in 0..24u64 {
        let mut rng = Rng::new(0x0B00_0010 + seed);
        let mut lits: Vec<u16> = vec![256, 287];
        for _ in 0..40 {
            lits.push(rng.below(256) as u16);
        }
        lits.push(285);
        let dists: Vec<u16> = vec![0, 1, 2, 31];
        let spec = dyn_spec(&mut rng, &lits, &dists, 288, 32, 15, (false, false, false), false);
        assert_eq!(spec.hlit, 288);
        assert_eq!(spec.hdist, 32);
        let mut toks: Vec<Tok> = Vec::new();
        let mut produced = 0usize;
        for _ in 0..60 {
            let s = lits[rng.below(lits.len())];
            if s < 256 {
                toks.push(Tok::Lit(s));
                produced += 1;
            }
        }
        if produced >= 4 {
            toks.push(Tok::Match { ls: 285, lx: 0, ds: 2, dx: 0 });
        }
        toks.push(Tok::End);
        let mut sm = Stream::new();
        sm.dynamic_block(true, &spec, &toks, &t);
        let (input, exp, olen) = sm.finish(0, false, 0);
        cases.push(case_exact(format!("hlit=288 hdist=32 seed={seed}"), input, exp, olen, 0));
    }

    // HCLEN swept from the minimum up to 19 (extra slots carry length 0)
    {
        let mut rng = Rng::new(0x0B00_0100);
        let lits: Vec<u16> = vec![0, 1, 2, 3, 4, 5, 6, 256];
        let base = dyn_spec(&mut rng, &lits, &[], 257, 1, 15, (false, false, false), true);
        let lo = base.hclen;
        for hclen in lo..=19 {
            let mut spec = base.clone();
            spec.hclen = hclen;
            let mut toks: Vec<Tok> = Vec::new();
            for i in 0..20u16 {
                toks.push(Tok::Lit(lits[(i as usize) % (lits.len() - 1)]));
            }
            toks.push(Tok::End);
            let mut s = Stream::new();
            s.dynamic_block(true, &spec, &toks, &t);
            let (input, exp, olen) = s.finish(0, false, 0);
            cases.push(case_exact(format!("hclen={hclen}"), input, exp, olen, 0));
        }
    }

    // HCLEN = 5: only code-length symbols 0 and 8 are describable
    {
        let mut lit_lens = vec![0u8; 257];
        for i in 0..255usize {
            lit_lens[i] = 8;
        }
        lit_lens[256] = 8;
        let dist_lens = vec![0u8; 1];
        let mut all = lit_lens.clone();
        all.extend_from_slice(&dist_lens);
        let ops = ops_direct(&all);
        let mut cl_lens = [0u8; 19];
        cl_lens[0] = 1;
        cl_lens[8] = 1;
        let spec = DynSpec {
            hlit: 257,
            hdist: 1,
            hclen: hclen_for(&cl_lens, &PERM),
            cl_lens,
            ops,
            lit_lens,
            dist_lens,
        };
        assert_eq!(spec.hclen, 5);
        let mut rng = Rng::new(0x0B00_1000);
        let mut toks: Vec<Tok> = Vec::new();
        for _ in 0..64 {
            toks.push(Tok::Lit(rng.below(255) as u16));
        }
        toks.push(Tok::End);
        let mut s = Stream::new();
        s.dynamic_block(true, &spec, &toks, &t);
        let (input, exp, olen) = s.finish(0, false, 0);
        cases.push(case_exact("hclen=5 (symbols 0 and 8 only)", input, exp, olen, 0));
    }
    cases
}

// ---------------------------------------------------------------------------
// rows 38-41: code-length repeat codes 16 / 17 / 18
// ---------------------------------------------------------------------------

fn b_dyn_rle() -> Vec<Case> {
    let t = Tables::default();
    let mut cases = Vec::new();
    let variants: &[(&str, (bool, bool, bool))] = &[
        ("rep16", (true, false, false)),
        ("rep17", (false, true, false)),
        ("rep18", (false, false, true)),
        ("rep16+17", (true, true, false)),
        ("rep16+18", (true, false, true)),
        ("rep17+18", (false, true, true)),
        ("all", (true, true, true)),
    ];
    for (name, rle) in variants {
        for seed in 0..16u64 {
            let mut rng = Rng::new(0x0C00_0000 + seed + name.len() as u64 * 977);
            // a wide, sparse alphabet produces long zero runs (17/18) and the
            // balanced tree produces long equal-length runs (16)
            let mut lits: Vec<u16> = vec![256];
            let k = rng.range(2, 40);
            for _ in 0..k {
                lits.push(rng.below(288) as u16);
            }
            lits.retain(|&s| s < 286 || s == 286 || s == 287);
            let dists: Vec<u16> = if rng.bool() { vec![0, 1] } else { vec![] };
            let bal = rng.bool();
            let spec = dyn_spec(&mut rng, &lits, &dists, 257, 1, 15, *rle, bal);
            let mut toks: Vec<Tok> = Vec::new();
            let usable: Vec<u16> = lits.iter().cloned().filter(|&s| s < 256).collect();
            let mut produced = 0usize;
            if !usable.is_empty() {
                for _ in 0..40 {
                    toks.push(Tok::Lit(usable[rng.below(usable.len())]));
                    produced += 1;
                }
            }
            if !dists.is_empty() && produced >= 4 && spec.lit_lens.len() > 257 {
                // only if a length symbol happens to be in the tree
                for ls in 257..spec.lit_lens.len().min(286) {
                    if spec.lit_lens[ls] != 0 {
                        toks.push(Tok::Match { ls: ls as u16, lx: 0, ds: 0, dx: 0 });
                        break;
                    }
                }
            }
            toks.push(Tok::End);
            let mut s = Stream::new();
            s.dynamic_block(true, &spec, &toks, &t);
            let (input, exp, olen) = s.finish(0, false, 0);
            cases.push(case_exact(
                format!("{name} seed={seed} hlit={} hdist={}", spec.hlit, spec.hdist),
                input,
                exp,
                olen,
                0,
            ));
        }
    }
    cases
}

// ---------------------------------------------------------------------------
// rows 42-46: code-length shapes (lookup table vs. no lookup, tiny trees)
// ---------------------------------------------------------------------------

fn b_dyn_lens() -> Vec<Case> {
    let t = Tables::default();
    let mut cases = Vec::new();

    // all code lengths <= 9  =>  cp_build fills `lookup`
    for seed in 0..24u64 {
        let mut rng = Rng::new(0x0D00_0000 + seed);
        let mut lits: Vec<u16> = vec![256];
        for _ in 0..rng.range(3, 100) {
            lits.push(rng.below(256) as u16);
        }
        let spec = dyn_spec(&mut rng, &lits, &[], 257, 1, 9, (false, true, true), false);
        assert!(spec.lit_lens.iter().all(|&l| l <= 9));
        let usable: Vec<u16> = lits.iter().cloned().filter(|&s| s < 256).collect();
        let mut toks: Vec<Tok> =
            (0..60).map(|_| Tok::Lit(usable[rng.below(usable.len())])).collect();
        toks.push(Tok::End);
        let mut s = Stream::new();
        s.dynamic_block(true, &spec, &toks, &t);
        let (input, exp, olen) = s.finish(0, false, 0);
        cases.push(case_exact(format!("lens<=9 seed={seed}"), input, exp, olen, 0));
    }

    // code lengths 1..15 (so 10..15 skip the `lookup` write)
    {
        // 16 symbols with lengths 1,2,...,14,15,15  (Kraft sum == 1)
        let mut lens: Vec<u8> = (1..=15u8).collect();
        lens.push(15);
        let syms: Vec<u16> = vec![
            256, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14,
        ];
        let mut lit_lens = vec![0u8; 257];
        for (i, &s) in syms.iter().enumerate() {
            lit_lens[s as usize] = lens[i];
        }
        let dist_lens = vec![0u8; 1];
        let mut all = lit_lens.clone();
        all.extend_from_slice(&dist_lens);
        let ops = ops_direct(&all);
        let mut used: Vec<usize> = Vec::new();
        for op in &ops {
            if !used.contains(&op.sym()) {
                used.push(op.sym());
            }
        }
        used.sort_unstable();
        let depths = balanced_lengths(used.len());
        let mut cl_lens = [0u8; 19];
        for (i, &s) in used.iter().enumerate() {
            cl_lens[s] = depths[i];
        }
        let spec = DynSpec {
            hlit: 257,
            hdist: 1,
            hclen: hclen_for(&cl_lens, &PERM),
            cl_lens,
            ops,
            lit_lens,
            dist_lens,
        };
        let mut rng = Rng::new(0x0D00_1000);
        let mut toks: Vec<Tok> = Vec::new();
        for _ in 0..80 {
            let s = syms[1 + rng.below(syms.len() - 1)];
            toks.push(Tok::Lit(s));
        }
        toks.push(Tok::End);
        let mut s = Stream::new();
        s.dynamic_block(true, &spec, &toks, &t);
        let (input, exp, olen) = s.finish(0, false, 0);
        cases.push(case_exact("lens 1..15 (deep codes)", input, exp, olen, 0));
    }

    // literal tree with exactly one symbol (256), length 1 => empty output
    {
        let mut lit_lens = vec![0u8; 257];
        lit_lens[256] = 1;
        let dist_lens = vec![0u8; 1];
        let mut all = lit_lens.clone();
        all.extend_from_slice(&dist_lens);
        let ops = ops_direct(&all);
        let mut cl_lens = [0u8; 19];
        cl_lens[0] = 1;
        cl_lens[1] = 1;
        let spec = DynSpec {
            hlit: 257,
            hdist: 1,
            hclen: hclen_for(&cl_lens, &PERM),
            cl_lens,
            ops,
            lit_lens,
            dist_lens,
        };
        let mut s = Stream::new();
        s.dynamic_block(true, &spec, &[Tok::End], &t);
        let (input, exp, olen) = s.finish(0, false, 0);
        cases.push(case_exact("single-symbol literal tree", input, exp, olen, 0));
    }

    // distance tree with 1 and with 2 symbols
    for k in 1..=2usize {
        let dists: Vec<u16> = (0..k as u16).collect();
        let mut rng = Rng::new(0x0D00_2000 + k as u64);
        let mut lits: Vec<u16> = vec![256, 257, 258];
        for _ in 0..8 {
            lits.push(rng.below(256) as u16);
        }
        let spec = dyn_spec(&mut rng, &lits, &dists, 257, k, 15, (false, false, false), true);
        assert_eq!(spec.hdist, k);
        let usable: Vec<u16> = lits.iter().cloned().filter(|&s| s < 256).collect();
        let mut toks: Vec<Tok> =
            (0..12).map(|_| Tok::Lit(usable[rng.below(usable.len())])).collect();
        for ds in 0..k as u16 {
            toks.push(Tok::Match { ls: 257, lx: 0, ds, dx: 0 });
        }
        toks.push(Tok::End);
        let mut s = Stream::new();
        s.dynamic_block(true, &spec, &toks, &t);
        let (input, exp, olen) = s.finish(0, false, 0);
        cases.push(case_exact(format!("hdist={k}"), input, exp, olen, 0));
    }
    cases
}

// ---------------------------------------------------------------------------
// rows 47-49: randomized dynamic blocks over all alignments
// ---------------------------------------------------------------------------

fn b_dyn_random() -> Vec<Case> {
    let t = Tables::default();
    let mut cases = Vec::new();
    for seed in 0..512u64 {
        let mut rng = Rng::new(0x0E00_0000 + seed);
        let align = rng.below(4);
        let fb = first_bytes_for(align);
        let want_last = rng.below(4);

        let mut lits: Vec<u16> = vec![256];
        for _ in 0..rng.range(1, 60) {
            lits.push(rng.below(256) as u16);
        }
        let n_len_syms = rng.range(0, 6);
        for _ in 0..n_len_syms {
            lits.push(rng.range(257, 287) as u16);
        }
        let n_dist = rng.range(0, 8);
        let dists: Vec<u16> = (0..n_dist).map(|_| rng.below(32) as u16).collect();

        let max_len = rng.range(9, 15) as u8;
        let rle = (rng.bool(), rng.bool(), rng.bool());
        let balanced = rng.bool();
        let spec = dyn_spec(&mut rng, &lits, &dists, 257, 1, max_len, rle, balanced);

        let usable: Vec<u16> = lits.iter().cloned().filter(|&s| s < 256).collect();
        let len_syms: Vec<u16> = (257..spec.lit_lens.len())
            .filter(|&s| spec.lit_lens[s] != 0)
            .map(|s| s as u16)
            .collect();
        let dist_syms: Vec<u16> = (0..spec.dist_lens.len())
            .filter(|&s| spec.dist_lens[s] != 0)
            .map(|s| s as u16)
            .collect();

        let mut toks: Vec<Tok> = Vec::new();
        let mut produced = 0usize;
        for _ in 0..rng.range(1, 120) {
            if !len_syms.is_empty() && !dist_syms.is_empty() && produced > 0 && rng.below(4) == 0 {
                let ls = len_syms[rng.below(len_syms.len())];
                let nx = t.len_extra[(ls - 257) as usize] as u32;
                let lx = if nx == 0 { 0 } else { rng.next_u32() % (1 << nx) };
                // find a distance symbol whose distance fits
                let mut ok = None;
                for _ in 0..8 {
                    let ds = dist_syms[rng.below(dist_syms.len())];
                    let nd = t.dist_extra[ds as usize] as u32;
                    let dx = if nd == 0 { 0 } else { rng.next_u32() % (1 << nd) };
                    let d = t.distance_of(ds, dx) as usize;
                    if d <= produced && d > 0 {
                        ok = Some((ds, dx, d));
                        break;
                    }
                }
                if let Some((ds, dx, _)) = ok {
                    toks.push(Tok::Match { ls, lx, ds, dx });
                    produced += t.length_of(ls, lx) as usize;
                    continue;
                }
            }
            if usable.is_empty() {
                break;
            }
            toks.push(Tok::Lit(usable[rng.below(usable.len())]));
            produced += 1;
        }
        toks.push(Tok::End);

        let mut s = Stream::new();
        s.dynamic_block(true, &spec, &toks, &t);
        let (mut input, exp, olen) = s.finish(fb, false, 0);
        pad_to_last_bytes(&mut input, align, want_last, rng.byte());
        let mut c = case_exact(
            format!("seed={seed} fb={fb} last={want_last} out={olen}"),
            input,
            exp,
            olen,
            align,
        );
        c = c.out_pad(olen + 4096);
        cases.push(c);
    }
    cases
}

// ---------------------------------------------------------------------------
// rows 50-52: output sizing and alignment
// ---------------------------------------------------------------------------

fn b_out_sizes() -> Vec<Case> {
    let t = Tables::default();
    let mut cases = Vec::new();
    for seed in 0..16u64 {
        let mut rng = Rng::new(0x0F00_0000 + seed);
        let n = rng.range(1, 60);
        let (toks, _) = random_fixed_stream(&mut rng, n);
        let mut s = Stream::new();
        s.fixed_block(true, &toks, &t);
        let (input, exp, olen) = s.finish(0, false, 0);
        for extra in [0i32, 1, 1000] {
            let mut c = Case::new(
                format!("out_bytes={}+{extra}", olen),
                input.clone(),
                olen as i32 + extra,
            );
            c = c.out_pad(olen + 4096);
            c = match &exp {
                Some(e) if extra == 0 => c.expect(Expect::Out { ret: 1, out: e.clone() }),
                _ => c.expect(Expect::Ret { ret: 1, reason: None }),
            };
            cases.push(c);
        }
        // out_bytes = INT_MAX
        cases.push(
            Case::new(format!("out_bytes=INT_MAX seed={seed}"), input.clone(), i32::MAX)
                .out_pad(olen + 4096)
                .expect(Expect::Ret { ret: 1, reason: None }),
        );
        // out pointer at every alignment
        for oa in 0..4usize {
            let mut c = Case::new(format!("out_align={oa} seed={seed}"), input.clone(), olen as i32)
                .out_align(oa)
                .out_pad(olen + 4096);
            c = match &exp {
                Some(e) => c.expect(Expect::Out { ret: 1, out: e.clone() }),
                None => c.expect(Expect::Ret { ret: 1, reason: None }),
            };
            cases.push(c);
        }
    }
    cases
}

// ---------------------------------------------------------------------------
// rows 53-59: the writable exported tables
// ---------------------------------------------------------------------------

fn b_tables() -> Vec<Case> {
    let mut cases = Vec::new();

    // rows 53/54: rotate the fixed code lengths (same multiset => still a
    // complete canonical code, but a completely different assignment)
    for rot in [1usize, 7, 144, 255] {
        let base: Vec<u8> = fixed_lit_lens().into_iter().chain(fixed_dist_lens()).collect();
        let mut table = base.clone();
        for i in 0..288 {
            table[i] = base[(i + rot) % 288];
        }
        let t = Tables::default();
        let mut rng = Rng::new(0x1000_0000 + rot as u64);
        let mut toks: Vec<Tok> = (0..40).map(|_| Tok::Lit(rng.byte() as u16)).collect();
        toks.push(Tok::Match { ls: 257, lx: 0, ds: 0, dx: 0 });
        toks.push(Tok::End);
        let mut s = Stream::new();
        s.fixed_block_with(true, &toks, &t, &table);
        let (input, exp, olen) = s.finish(0, false, 0);
        let mut c = case_exact(format!("cp_fixed_table rot={rot}"), input, exp, olen, 0);
        for i in 0..320usize {
            if table[i] != base[i] {
                c = c.patch(Tbl::Fixed, i, table[i] as u32);
            }
        }
        cases.push(c);
    }

    // rows 55-58: length/distance base and extra-bit tables
    let patches: &[(&str, Tbl, usize, u32)] = &[
        ("len_base[0]=7", Tbl::LenBase, 0, 7),
        ("len_base[28]=17", Tbl::LenBase, 28, 17),
        ("len_extra[0]=3", Tbl::LenExtra, 0, 3),
        ("len_extra[28]=4", Tbl::LenExtra, 28, 4),
        ("dist_base[0]=4", Tbl::DistBase, 0, 4),
        ("dist_base[29]=9", Tbl::DistBase, 29, 9),
        ("dist_extra[0]=5", Tbl::DistExtra, 0, 5),
        ("dist_extra[29]=2", Tbl::DistExtra, 29, 2),
    ];
    for (name, tbl, idx, val) in patches {
        let mut t = Tables::default();
        match tbl {
            Tbl::LenBase => t.len_base[*idx] = *val,
            Tbl::LenExtra => t.len_extra[*idx] = *val as u8,
            Tbl::DistBase => t.dist_base[*idx] = *val,
            Tbl::DistExtra => t.dist_extra[*idx] = *val as u8,
            _ => unreachable!(),
        }
        let (ls, ds): (u16, u16) = match tbl {
            Tbl::LenBase | Tbl::LenExtra => (257 + *idx as u16, 0),
            _ => (257, *idx as u16),
        };
        let nx = t.len_extra[(ls - 257) as usize] as u32;
        let nd = t.dist_extra[ds as usize] as u32;
        let lx = if nx == 0 { 0 } else { (1u32 << nx) - 1 };
        let dx = if nd == 0 { 0 } else { (1u32 << nd) - 1 };
        let dist = t.distance_of(ds, dx) as usize;
        let prefix = dist.max(1);
        let mut rng = Rng::new(0x1100_0000 + *idx as u64);
        let mut toks: Vec<Tok> = (0..prefix).map(|_| Tok::Lit(rng.byte() as u16)).collect();
        toks.push(Tok::Match { ls, lx, ds, dx });
        toks.push(Tok::End);
        let mut s = Stream::new();
        s.fixed_block(true, &toks, &t);
        let (input, exp, olen) = s.finish(0, false, 0);
        let mut c = case_exact(format!("{name}"), input, exp, olen, 0);
        c = c.patch(*tbl, *idx, *val).out_pad(olen + 4096);
        cases.push(c);
    }

    // row 59: reversed cp_permutation_order
    {
        let mut perm = [0u8; 19];
        for i in 0..19 {
            perm[i] = PERM[18 - i];
        }
        let t = Tables::default();
        let mut rng = Rng::new(0x1200_0000);
        let lits: Vec<u16> = vec![0, 1, 2, 3, 4, 5, 6, 7, 256];
        let mut spec = dyn_spec(&mut rng, &lits, &[], 257, 1, 15, (false, false, false), true);
        spec.hclen = hclen_for(&spec.cl_lens, &perm);
        let mut toks: Vec<Tok> = (0..40).map(|_| Tok::Lit(lits[rng.below(8)])).collect();
        toks.push(Tok::End);
        let mut s = Stream::new();
        s.dynamic_block_perm(true, &spec, &toks, &t, &perm);
        let (input, exp, olen) = s.finish(0, false, 0);
        let mut c = case_exact("cp_permutation_order reversed", input, exp, olen, 0);
        for i in 0..19usize {
            if perm[i] != PERM[i] {
                c = c.patch(Tbl::Perm, i, perm[i] as u32);
            }
        }
        cases.push(c);
    }
    cases
}

// ---------------------------------------------------------------------------
// row 60: cp_error_reason is never cleared on success
// ---------------------------------------------------------------------------

fn b_reason() -> Vec<Case> {
    let t = Tables::default();
    let mut cases = Vec::new();
    for seed in 0..4u64 {
        let mut rng = Rng::new(0x1300_0000 + seed);
        let n = rng.range(1, 40);
        let (toks, _) = random_fixed_stream(&mut rng, n);
        let mut s = Stream::new();
        s.fixed_block(true, &toks, &t);
        let (input, _, olen) = s.finish(0, false, 0);
        cases.push(
            Case::new(format!("preset reason, success seed={seed}"), input, olen as i32)
                .preset_reason()
                .expect(Expect::Ret { ret: 1, reason: Some("<<untouched sentinel>>") }),
        );
    }
    cases
}

// ---------------------------------------------------------------------------
// rows 61-65: multi-block streams
// ---------------------------------------------------------------------------

fn b_multi() -> Vec<Case> {
    let t = Tables::default();
    let mut cases = Vec::new();

    // stored -> fixed  (the stored block's LEN covers the following block, and
    // the reader is *not* advanced past the stored data)
    for align in 0..4usize {
        let fb = first_bytes_for(align);
        let mut rng = Rng::new(0x1400_0000 + align as u64);
        let nlit = rng.range(1, 20);
        let (toks, _) = random_fixed_stream(&mut rng, nlit);
        let mut s = Stream::new();
        s.stored_block(false, &[]);
        s.fixed_block(true, &toks, &t);
        let (input, exp, olen) = s.finish(fb, true, 0);
        let mut c = case_exact(format!("stored->fixed fb={fb}"), input, exp, olen, align);
        c = c.out_pad(olen + 4096);
        cases.push(c);
    }

    // fixed -> stored
    for align in 0..4usize {
        let fb = first_bytes_for(align);
        let mut rng = Rng::new(0x1500_0000 + align as u64);
        let nlit = rng.range(1, 20);
        let (toks, _) = random_fixed_stream(&mut rng, nlit);
        let data: Vec<u8> = (0..rng.range(1, 40)).map(|_| rng.byte()).collect();
        let mut s = Stream::new();
        s.fixed_block(false, &toks, &t);
        s.stored_block(true, &data);
        let (input, exp, olen) = s.finish(fb, true, 0);
        let mut c = case_exact(format!("fixed->stored fb={fb}"), input, exp, olen, align);
        c = c.out_pad(olen + 4096);
        cases.push(c);
    }

    // dynamic -> fixed with a match reaching back into the previous block
    for seed in 0..16u64 {
        let mut rng = Rng::new(0x1600_0000 + seed);
        let lits: Vec<u16> = vec![0, 1, 2, 3, 4, 5, 6, 7, 256];
        let spec = dyn_spec(&mut rng, &lits, &[], 257, 1, 15, (false, false, false), true);
        let mut toks1: Vec<Tok> = (0..30).map(|_| Tok::Lit(lits[rng.below(8)])).collect();
        toks1.push(Tok::End);
        let mut toks2: Vec<Tok> = (0..5).map(|_| Tok::Lit(rng.byte() as u16)).collect();
        toks2.push(Tok::Match { ls: 285, lx: 0, ds: 5, dx: 0 }); // dist 7 -> crosses back
        toks2.push(Tok::End);
        let mut s = Stream::new();
        s.dynamic_block(false, &spec, &toks1, &t);
        s.fixed_block(true, &toks2, &t);
        let (input, exp, olen) = s.finish(0, false, 0);
        let mut c = case_exact(format!("dynamic->fixed cross-match seed={seed}"), input, exp, olen, 0);
        c = c.out_pad(olen + 4096);
        cases.push(c);
    }

    // fixed -> dynamic (rebuilds lit/dst/lookup)
    for seed in 0..16u64 {
        let mut rng = Rng::new(0x1700_0000 + seed);
        let nlit1 = rng.range(1, 30);
        let (toks1, _) = random_fixed_stream(&mut rng, nlit1);
        let lits: Vec<u16> = vec![10, 11, 12, 13, 256];
        let spec = dyn_spec(&mut rng, &lits, &[], 257, 1, 15, (false, false, false), true);
        let mut toks2: Vec<Tok> = (0..20).map(|_| Tok::Lit(lits[rng.below(4)])).collect();
        toks2.push(Tok::End);
        let mut s = Stream::new();
        s.fixed_block(false, &toks1, &t);
        s.dynamic_block(true, &spec, &toks2, &t);
        let (input, exp, olen) = s.finish(0, false, 0);
        let mut c = case_exact(format!("fixed->dynamic seed={seed}"), input, exp, olen, 0);
        c = c.out_pad(olen + 4096);
        cases.push(c);
    }

    // stored -> dynamic -> stored
    for align in 0..4usize {
        let fb = first_bytes_for(align);
        let mut rng = Rng::new(0x1800_0000 + align as u64);
        let lits: Vec<u16> = vec![20, 21, 22, 23, 256];
        let spec = dyn_spec(&mut rng, &lits, &[], 257, 1, 15, (false, false, false), true);
        let mut toks: Vec<Tok> = (0..15).map(|_| Tok::Lit(lits[rng.below(4)])).collect();
        toks.push(Tok::End);
        let data: Vec<u8> = (0..rng.range(1, 20)).map(|_| rng.byte()).collect();
        let mut s = Stream::new();
        s.stored_block(false, &[]);
        s.dynamic_block(false, &spec, &toks, &t);
        s.stored_block(true, &data);
        let (input, exp, olen) = s.finish(fb, true, 0);
        let mut c = case_exact(format!("stored->dynamic->stored fb={fb}"), input, exp, olen, align);
        c = c.out_pad(olen + 8192);
        cases.push(c);
    }
    cases
}

// ---------------------------------------------------------------------------
// row 66: property test over random block sequences
// ---------------------------------------------------------------------------

fn b_property() -> Vec<Case> {
    let t = Tables::default();
    let mut cases = Vec::new();
    for seed in 0..1536u64 {
        let mut rng = Rng::new(0x1900_0000 + seed);
        let align = rng.below(4);
        let fb = first_bytes_for(align);
        let nblocks = rng.range(1, 5);
        let mut s = Stream::new();
        let mut has_stored = false;
        let mut produced = 0usize;
        for b in 0..nblocks {
            let last = b + 1 == nblocks;
            let kind = rng.below(3);
            match kind {
                0 if !has_stored => {
                    // stored: LEN swallows the rest of the input
                    has_stored = true;
                    if last {
                        let data: Vec<u8> = (0..rng.range(0, 40)).map(|_| rng.byte()).collect();
                        s.stored_block(true, &data);
                    } else {
                        s.stored_block(false, &[]);
                    }
                }
                1 => {
                    let n = rng.range(0, 40);
                    let mut toks: Vec<Tok> = Vec::new();
                    for _ in 0..n {
                        toks.push(Tok::Lit(rng.byte() as u16));
                        produced += 1;
                    }
                    if !has_stored && produced >= 4 && rng.bool() {
                        let ds = rng.below(3) as u16;
                        let d = t.distance_of(ds, 0) as usize;
                        if d <= produced {
                            toks.push(Tok::Match { ls: 257, lx: 0, ds, dx: 0 });
                            produced += 3;
                        }
                    }
                    toks.push(Tok::End);
                    s.fixed_block(last, &toks, &t);
                }
                _ => {
                    let mut lits: Vec<u16> = vec![256];
                    for _ in 0..rng.range(1, 20) {
                        lits.push(rng.below(256) as u16);
                    }
                    let rle = (rng.bool(), rng.bool(), rng.bool());
                    let bal = rng.bool();
                    let spec = dyn_spec(&mut rng, &lits, &[], 257, 1, 15, rle, bal);
                    let usable: Vec<u16> = lits.iter().cloned().filter(|&x| x < 256).collect();
                    let mut toks: Vec<Tok> = Vec::new();
                    for _ in 0..rng.range(0, 40) {
                        toks.push(Tok::Lit(usable[rng.below(usable.len())]));
                        produced += 1;
                    }
                    toks.push(Tok::End);
                    s.dynamic_block(last, &spec, &toks, &t);
                }
            }
        }
        let (mut input, exp, olen) = s.finish(fb, has_stored, 0);
        if !has_stored {
            let wl = rng.below(4);
            pad_to_last_bytes(&mut input, align, wl, rng.byte());
        }
        let extra = if rng.bool() { 0 } else { rng.range(1, 64) as i32 };
        let mut c = Case::new(
            format!("seed={seed} blocks={nblocks} fb={fb} out={olen}+{extra}"),
            input,
            olen as i32 + extra,
        )
        .in_align(align)
        .out_align(rng.below(4))
        .out_pad(olen + 4096);
        c = match exp {
            Some(e) if extra == 0 => c.expect(Expect::Out { ret: 1, out: e }),
            _ => c.expect(Expect::Ret { ret: 1, reason: None }),
        };
        cases.push(c);
    }
    cases
}

// ---------------------------------------------------------------------------
// row 70: several pinflate() calls in a row on the same buffers
// ---------------------------------------------------------------------------

fn b_repeat_calls() -> Vec<Case> {
    let t = Tables::default();
    let mut cases = Vec::new();
    for seed in 0..24u64 {
        let mut rng = Rng::new(0x1A00_0000 + seed);
        let align = rng.below(4);
        let fb = first_bytes_for(align);
        let kind = rng.below(3);
        let mut s = Stream::new();
        let mut pad = false;
        match kind {
            0 => {
                let n = rng.range(1, 40);
                let (toks, _) = random_fixed_stream(&mut rng, n);
                s.fixed_block(true, &toks, &t);
            }
            1 => {
                let data: Vec<u8> = (0..rng.range(1, 40)).map(|_| rng.byte()).collect();
                s.stored_block(true, &data);
                pad = true;
            }
            _ => {
                let mut lits: Vec<u16> = vec![256];
                for _ in 0..rng.range(1, 30) {
                    lits.push(rng.below(256) as u16);
                }
                let bal = rng.bool();
                let spec = dyn_spec(&mut rng, &lits, &[], 257, 1, 15, (false, true, true), bal);
                let usable: Vec<u16> = lits.iter().cloned().filter(|&x| x < 256).collect();
                let mut toks: Vec<Tok> =
                    (0..30).map(|_| Tok::Lit(usable[rng.below(usable.len())])).collect();
                toks.push(Tok::End);
                s.dynamic_block(true, &spec, &toks, &t);
            }
        }
        let (input, exp, olen) = s.finish(fb, pad, 0);
        for n in [2usize, 3, 5] {
            let mut c = Case::new(
                format!("kind={kind} calls={n} seed={seed}"),
                input.clone(),
                olen as i32,
            )
            .in_align(align)
            .out_pad(olen + 4096)
            .calls(n);
            // the output must be identical after every call, and a *successful*
            // call must not touch cp_error_reason
            c = match &exp {
                Some(e) => c.expect(Expect::Out { ret: 1, out: e.clone() }),
                None => c.expect(Expect::Ret { ret: 1, reason: None }),
            };
            cases.push(c);
        }
        // an error stream repeated: cp_error_reason must stay set
        {
            let mut s2 = Stream::new();
            s2.raw_bits(1, 1);
            s2.raw_bits(3, 2);
            let (input2, _, _) = s2.finish(fb, false, 0);
            cases.push(
                Case::new(format!("btype=3 x4 seed={seed}"), input2, 64)
                    .in_align(align)
                    .calls(4)
                    .expect(Expect::Ret {
                        ret: 0,
                        reason: Some(
                            "Detected unknown block type within input stream.",
                        ),
                    }),
            );
        }
    }
    cases
}
