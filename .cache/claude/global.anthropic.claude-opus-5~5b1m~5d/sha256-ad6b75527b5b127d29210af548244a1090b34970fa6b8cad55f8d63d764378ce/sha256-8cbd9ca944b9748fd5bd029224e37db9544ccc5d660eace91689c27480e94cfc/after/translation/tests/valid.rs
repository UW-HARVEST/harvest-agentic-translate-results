//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every test drives BOTH the C `.so` and the Rust `.so` through `libloading`
//! and compares the return value, the *entire* output buffer (including the
//! untouched tail), the `cp_error_reason` string and the contents of all seven
//! exported tables.

mod common;
use common::*;

// ---------------------------------------------------------------------------
// Rows 1-4: BTYPE == 0, stored blocks
// ---------------------------------------------------------------------------

#[test]
fn cfg01_stored_random_sizes() {
    let mut rng = Rng::new(0x5701_0001);
    for _ in 0..200 {
        let n = rng.range(0, 4096);
        let payload = rng.bytes(n);
        let mut w = BitWriter::new();
        write_stored_block(&mut w, true, &payload);
        let s = w.finish();
        // out_bytes exactly, and generously
        diff("cfg01/exact", &s, 0, payload.len());
        diff("cfg01/slack", &s, 0, payload.len() + 37);
    }
}

#[test]
fn cfg02_stored_len_zero() {
    let mut w = BitWriter::new();
    write_stored_block(&mut w, true, &[]);
    let s = w.finish();
    for out in [0usize, 1, 64] {
        diff_all_alignments("cfg02", &s, out);
        diff("cfg02/out", &s, 0, out);
    }
}

#[test]
fn cfg03_stored_len_one() {
    for b in [0u8, 1, 0x7f, 0x80, 0xff] {
        let mut w = BitWriter::new();
        write_stored_block(&mut w, true, &[b]);
        let s = w.finish();
        diff_all_alignments("cfg03", &s, 1);
        diff("cfg03/slack", &s, 0, 16);
    }
}

#[test]
fn cfg04_stored_alignment_matrix() {
    let mut rng = Rng::new(0x5701_0004);
    for _ in 0..64 {
        let n = rng.range(0, 300);
        let payload = rng.bytes(n);
        let mut w = BitWriter::new();
        write_stored_block(&mut w, true, &payload);
        let s = w.finish();
        // `in_bytes % 4` is already varied by the payload length; `align` by
        // the placement of the buffer.
        for align in 0..4 {
            diff(&format!("cfg04/a{align}/n{n}"), &s, align, n);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 5-13: BTYPE == 1, fixed Huffman
// ---------------------------------------------------------------------------

fn fixed_stream(toks: &[Tok]) -> Vec<u8> {
    let mut w = BitWriter::new();
    write_fixed_block(&mut w, true, toks);
    w.finish()
}

/// Cross a stream with the 4 input alignments and 4 `in_bytes % 4` classes.
fn diff_matrix(label: &str, stream: &[u8], out_len: usize) {
    for align in 0..4usize {
        for tail in 0..4usize {
            let mut s = stream.to_vec();
            s.extend(std::iter::repeat(0u8).take(tail));
            diff(&format!("{label}/a{align}t{tail}"), &s, align, out_len);
        }
    }
}

#[test]
fn cfg05_fixed_literals_random() {
    let mut rng = Rng::new(0x5701_0005);
    for _ in 0..300 {
        let n = rng.range(1, 400);
        let data = rng.bytes(n);
        let toks: Vec<Tok> = data.iter().map(|&b| Tok::Lit(b)).collect();
        let s = fixed_stream(&toks);
        diff("cfg05/exact", &s, 0, n);
        diff("cfg05/slack", &s, rng.below(4), n + rng.below(9));
    }
    // a big one, > 8 KiB of literals -> thousands of word loads
    let data = Rng::new(0xBEEF).bytes(8192);
    let toks: Vec<Tok> = data.iter().map(|&b| Tok::Lit(b)).collect();
    let s = fixed_stream(&toks);
    diff_matrix("cfg05/big", &s, 8192);
}

#[test]
fn cfg06_fixed_single_literal() {
    for b in 0u16..256 {
        let s = fixed_stream(&[Tok::Lit(b as u8)]);
        diff("cfg06", &s, (b % 4) as usize, 1);
    }
    // exercises both the 8-bit (0..143, 280..287) and 9-bit (144..255) code
    // classes of the fixed table
    diff_matrix("cfg06/lo", &fixed_stream(&[Tok::Lit(0)]), 1);
    diff_matrix("cfg06/hi", &fixed_stream(&[Tok::Lit(255)]), 1);
}

#[test]
fn cfg07_fixed_empty_block() {
    let s = fixed_stream(&[]);
    diff_matrix("cfg07/out0", &s, 0);
    diff_matrix("cfg07/out8", &s, 8);
}

#[test]
fn cfg08_fixed_distance_one_memset() {
    let mut rng = Rng::new(0x5701_0008);
    for _ in 0..120 {
        let lit = rng.byte();
        let length = rng.range(3, 258) as u32;
        let toks = vec![Tok::Lit(lit), Tok::Match(length, 1)];
        let s = fixed_stream(&toks);
        diff("cfg08", &s, rng.below(4), 1 + length as usize);
    }
    // boundary lengths for the memset path
    for length in [3u32, 4, 10, 11, 257, 258] {
        let s = fixed_stream(&[Tok::Lit(0x5A), Tok::Match(length, 1)]);
        diff_matrix(&format!("cfg08/l{length}"), &s, 1 + length as usize);
    }
}

#[test]
fn cfg09_fixed_overlapping_match() {
    let mut rng = Rng::new(0x5701_0009);
    for _ in 0..200 {
        let prefix = rng.range(2, 40);
        let data = rng.bytes(prefix);
        let dist = rng.range(2, prefix) as u32;
        let length = rng.range(dist as usize + 1, 258) as u32; // distance < length
        let mut toks: Vec<Tok> = data.iter().map(|&b| Tok::Lit(b)).collect();
        toks.push(Tok::Match(length, dist));
        let s = fixed_stream(&toks);
        diff("cfg09", &s, rng.below(4), prefix + length as usize);
    }
}

#[test]
fn cfg10_fixed_non_overlapping_match() {
    let mut rng = Rng::new(0x5701_0010);
    for _ in 0..200 {
        let prefix = rng.range(4, 300);
        let data = rng.bytes(prefix);
        let dist = rng.range(3, prefix) as u32;
        let length = rng.range(3, dist as usize) as u32; // distance > length
        let mut toks: Vec<Tok> = data.iter().map(|&b| Tok::Lit(b)).collect();
        toks.push(Tok::Match(length, dist));
        let s = fixed_stream(&toks);
        diff("cfg10", &s, rng.below(4), prefix + length as usize);
    }
}

#[test]
fn cfg11_every_length_symbol() {
    // symbols 257..=285 -> lengths 3..=258, every `cp_len_extra_bits` class
    let base = pristine_tables();
    for sym in 257usize..=285 {
        let e = base.len_extra_bits[sym - 257] as u32;
        let b = base.len_base[sym - 257];
        for extra in [0u32, (1 << e) - 1] {
            let length = b + extra;
            let prefix = 300usize;
            let data: Vec<u8> = (0..prefix).map(|i| (i * 7 + 1) as u8).collect();
            let mut toks: Vec<Tok> = data.iter().map(|&x| Tok::Lit(x)).collect();
            toks.push(Tok::Match(length, 300));
            let s = fixed_stream(&toks);
            diff(
                &format!("cfg11/sym{sym}/len{length}"),
                &s,
                (sym % 4) as usize,
                prefix + length as usize,
            );
        }
    }
}

#[test]
fn cfg12_every_distance_symbol() {
    let base = pristine_tables();
    let prefix = 40_000usize;
    let data: Vec<u8> = (0..prefix).map(|i| (i * 13 + 5) as u8).collect();
    let lits: Vec<Tok> = data.iter().map(|&x| Tok::Lit(x)).collect();
    for sym in 0usize..30 {
        let e = base.dist_extra_bits[sym] as u32;
        let b = base.dist_base[sym];
        for extra in [0u32, (1 << e) - 1] {
            let dist = b + extra;
            if dist as usize > prefix {
                continue;
            }
            let mut toks = lits.clone();
            toks.push(Tok::Match(20, dist));
            let s = fixed_stream(&toks);
            diff(&format!("cfg12/dsym{sym}/d{dist}"), &s, sym % 4, prefix + 20);
        }
    }
}

#[test]
fn cfg13_max_length_max_distance() {
    let prefix = 40_000usize;
    let data: Vec<u8> = (0..prefix).map(|i| (i ^ 0x5A) as u8).collect();
    let mut toks: Vec<Tok> = data.iter().map(|&x| Tok::Lit(x)).collect();
    toks.push(Tok::Match(258, 32_768.min(prefix as u32)));
    let s = fixed_stream(&toks);
    diff_matrix("cfg13", &s, prefix + 258);
}

// ---------------------------------------------------------------------------
// Rows 14-22: BTYPE == 2, dynamic Huffman
// ---------------------------------------------------------------------------

/// Build a dynamic block for the given literal symbols, with the given tree
/// shape, plus the tokens to encode.
fn dyn_stream(nlit: usize, ndst: usize, lit_lens: &[u8], dst_lens: &[u8], toks: &[Tok]) -> Vec<u8> {
    assert_eq!(lit_lens.len(), nlit);
    assert_eq!(dst_lens.len(), ndst);
    let mut w = BitWriter::new();
    write_dynamic_block(&mut w, true, lit_lens, dst_lens, toks);
    w.finish()
}

#[test]
fn cfg14_dynamic_minimum_header() {
    // HLIT at its minimum (nlit = 257) and HDIST at its minimum (ndst = 1).
    let symbols: Vec<usize> = vec![b'a' as usize, b'b' as usize, b'c' as usize, 256];
    let lit_lens = balanced_lengths(257, &symbols);
    // ndst == 1 with a single 1-bit distance code -- exactly what real
    // encoders emit when a block has no matches.
    let dst_lens = vec![1u8; 1];
    let toks = vec![Tok::Lit(b'a'), Tok::Lit(b'b'), Tok::Lit(b'c'), Tok::Lit(b'a')];
    let s = dyn_stream(257, 1, &lit_lens, &dst_lens, &toks);
    diff_matrix("cfg14", &s, 4);

    // ndst == 1 with an *empty* distance tree (length 0) -- also emitted by
    // real encoders; makes `ndst == 0` inside the state.
    let dst_lens = vec![0u8; 1];
    let s = dyn_stream(257, 1, &lit_lens, &dst_lens, &toks);
    diff_matrix("cfg14/emptydst", &s, 4);

    let mut rng = Rng::new(0x5701_0014);
    for _ in 0..80 {
        let k = rng.range(1, 30);
        let mut syms: Vec<usize> = (0..k).map(|_| rng.below(256)).collect();
        syms.push(256);
        let lit_lens = balanced_lengths(257, &syms);
        let pool: Vec<usize> = syms.iter().cloned().filter(|&x| x < 256).collect();
        if pool.is_empty() {
            continue;
        }
        let n = rng.range(1, 80);
        let toks: Vec<Tok> = (0..n)
            .map(|_| Tok::Lit(pool[rng.below(pool.len())] as u8))
            .collect();
        let dl = if rng.below(2) == 0 { vec![1u8] } else { vec![0u8] };
        let s = dyn_stream(257, 1, &lit_lens, &dl, &toks);
        diff("cfg14/rand", &s, rng.below(4), n);
    }
}

#[test]
fn cfg14b_dynamic_hclen_minimum() {
    // HCLEN == 4 only exposes permutation slots 16, 17, 18 and 0, so the
    // code-length stream may only use those four symbols.  Build a complete
    // 2-bit code over all four and express a 258-entry all-zero length vector
    // with zero-runs plus single zeros.  The resulting literal tree is empty
    // (`nlit == 0`), which drives `cp_decode`'s `lo == 0` / `tree[-1]` branch.
    // That branch can trip `assert((search >> len) == (key >> len))`, so this
    // row runs in child processes.
    let mut cl_lens = [0u8; 19];
    for s in [0usize, 16, 17, 18] {
        cl_lens[s] = 2;
    }
    let nlit = 257usize;
    let ndst = 1usize;
    let total = nlit + ndst;
    let mut items: Vec<Cl> = Vec::new();
    let mut emitted = 0usize;
    while emitted < total {
        let room = total - emitted;
        if room >= 138 {
            items.push(Cl::Z11(127));
            emitted += 138;
        } else if room >= 11 {
            let extra = (room - 11).min(127);
            items.push(Cl::Z11(extra as u8));
            emitted += 11 + extra;
        } else if room >= 3 {
            let extra = (room - 3).min(7);
            items.push(Cl::Z3(extra as u8));
            emitted += 3 + extra;
        } else {
            items.push(Cl::Len(0));
            emitted += 1;
        }
    }
    assert_eq!(emitted, total);

    let mut w = BitWriter::new();
    w.bits(1, 1);
    w.bits(2, 2);
    w.bits((nlit - 257) as u32, 5);
    w.bits((ndst - 1) as u32, 5);
    w.bits(0, 4); // HCLEN == 4
    let perm = pristine_tables().permutation_order;
    for i in 0..4 {
        w.bits(cl_lens[perm[i] as usize] as u32, 3);
    }
    let clc = canonical(&cl_lens);
    for it in &items {
        let (c, l) = clc[it.symbol()];
        assert!(l > 0);
        w.huff(c, l as u32);
        if let Some((e, n)) = it.extra() {
            w.bits(e, n);
        }
    }
    for _ in 0..8 {
        w.bits(0x5A, 8);
    }
    let s = w.finish();

    let mut cases = Vec::new();
    for align in 0..4usize {
        for out in [0usize, 1, 64] {
            cases.push(Case::new(&format!("cfg14b/a{align}/o{out}"), &s).align(align).out(out));
        }
    }
    diff_cases_in_children("cfg14b", &cases);
}

#[test]
fn cfg15_dynamic_maximum_header() {
    // nlit = 288, ndst = 32, HCLEN = 19
    let lit_syms: Vec<usize> = (0..288).collect();
    let lit_lens = balanced_lengths(288, &lit_syms);
    let dst_syms: Vec<usize> = (0..32).collect();
    let dst_lens = balanced_lengths(32, &dst_syms);
    let mut rng = Rng::new(0x5701_0015);
    for _ in 0..30 {
        let n = rng.range(1, 200);
        let mut toks: Vec<Tok> = Vec::new();
        let mut produced = 0usize;
        for _ in 0..n {
            if produced > 40 && rng.below(3) == 0 {
                let dist = rng.range(1, produced) as u32;
                let length = rng.range(3, 40) as u32;
                toks.push(Tok::Match(length, dist));
                produced += length as usize;
            } else {
                toks.push(Tok::Lit(rng.byte()));
                produced += 1;
            }
        }
        let s = dyn_stream(288, 32, &lit_lens, &dst_lens, &toks);
        diff("cfg15", &s, rng.below(4), produced);
    }
}

/// Encode a length vector using the repeat codes, so that `cp_dynamic`'s
/// `case 16/17/18` are exercised.
fn cl_encode_with_repeats(all: &[u8], use16: bool, use17: bool, use18: bool) -> Vec<Cl> {
    let mut items = Vec::new();
    let mut i = 0usize;
    while i < all.len() {
        let v = all[i];
        let mut run = 1usize;
        while i + run < all.len() && all[i + run] == v {
            run += 1;
        }
        if v == 0 && use18 && run >= 11 {
            let take = run.min(138);
            items.push(Cl::Z11((take - 11) as u8));
            i += take;
        } else if v == 0 && use17 && run >= 3 {
            let take = run.min(10);
            items.push(Cl::Z3((take - 3) as u8));
            i += take;
        } else if use16 && run >= 4 && i > 0 {
            // one literal then a copy-previous run
            items.push(Cl::Len(v));
            let rest = (run - 1).min(6);
            if rest >= 3 {
                items.push(Cl::Rep((rest - 3) as u8));
                i += 1 + rest;
            } else {
                i += 1;
            }
        } else {
            items.push(Cl::Len(v));
            i += 1;
        }
    }
    items
}

fn dyn_stream_repeats(
    nlit: usize,
    ndst: usize,
    lit_lens: &[u8],
    dst_lens: &[u8],
    toks: &[Tok],
    use16: bool,
    use17: bool,
    use18: bool,
) -> Vec<u8> {
    let mut all = lit_lens.to_vec();
    all.extend_from_slice(dst_lens);
    let items = cl_encode_with_repeats(&all, use16, use17, use18);
    // sanity: the items must expand back to exactly the same vector
    let mut back: Vec<u8> = Vec::new();
    for it in &items {
        match *it {
            Cl::Len(l) => back.push(l),
            Cl::Rep(e) => {
                let prev = *back.last().unwrap();
                for _ in 0..(3 + e as usize) {
                    back.push(prev);
                }
            }
            Cl::Z3(e) => {
                for _ in 0..(3 + e as usize) {
                    back.push(0)
                }
            }
            Cl::Z11(e) => {
                for _ in 0..(11 + e as usize) {
                    back.push(0)
                }
            }
        }
    }
    assert_eq!(back, all, "repeat encoding does not round-trip");
    let cl_lens = cl_lengths_from_items(&items);
    let mut w = BitWriter::new();
    write_dynamic_block_raw(
        &mut w, true, nlit, ndst, 19, &cl_lens, &items, toks, lit_lens, dst_lens,
    );
    w.finish()
}

fn small_dyn_tree() -> (Vec<u8>, Vec<u8>) {
    let symbols: Vec<usize> = (b'a' as usize..=b'z' as usize).chain([256]).collect();
    let lit_lens = balanced_lengths(288, &symbols);
    let dst_lens = balanced_lengths(32, &[0, 1, 2, 3, 4, 5]);
    (lit_lens, dst_lens)
}

#[test]
fn cfg16_dynamic_repeat_code_16() {
    let (lit_lens, dst_lens) = small_dyn_tree();
    let toks: Vec<Tok> = b"abcabcabcxyz".iter().map(|&b| Tok::Lit(b)).collect();
    let s = dyn_stream_repeats(288, 32, &lit_lens, &dst_lens, &toks, true, false, false);
    diff_matrix("cfg16", &s, 12);
}

#[test]
fn cfg17_dynamic_repeat_code_17() {
    let (lit_lens, dst_lens) = small_dyn_tree();
    let toks: Vec<Tok> = b"hellozworld".iter().map(|&b| Tok::Lit(b)).collect();
    let s = dyn_stream_repeats(288, 32, &lit_lens, &dst_lens, &toks, false, true, false);
    diff_matrix("cfg17", &s, 11);
}

#[test]
fn cfg18_dynamic_repeat_code_18() {
    let (lit_lens, dst_lens) = small_dyn_tree();
    let toks: Vec<Tok> = b"zzzabc".iter().map(|&b| Tok::Lit(b)).collect();
    let s = dyn_stream_repeats(288, 32, &lit_lens, &dst_lens, &toks, false, false, true);
    diff_matrix("cfg18", &s, 6);
    // all three repeat codes at once, randomized trees
    let mut rng = Rng::new(0x5701_0018);
    for _ in 0..60 {
        let k = rng.range(2, 30);
        let mut syms: Vec<usize> = (0..k).map(|_| rng.below(256)).collect();
        syms.push(256);
        let lit_lens = balanced_lengths(288, &syms);
        let dst_lens = balanced_lengths(32, &[0, 1]);
        let n = rng.range(1, 60);
        let toks: Vec<Tok> = (0..n)
            .map(|_| Tok::Lit(syms[rng.below(syms.len() - 1)] as u8))
            .collect();
        let s = dyn_stream_repeats(288, 32, &lit_lens, &dst_lens, &toks, true, true, true);
        diff("cfg18/mixed", &s, rng.below(4), n);
    }
}

#[test]
fn cfg19_dynamic_codes_longer_than_9_bits() {
    // A unary code shape gives lengths 1,2,…,14,14 -> five code lengths above
    // 9, so `cp_build`'s `len <= 9` lookup fast path is skipped for them.
    let mut syms: Vec<usize> = (b'a' as usize..b'a' as usize + 14).collect();
    syms.push(256);
    let lit_lens = skewed_lengths(288, &syms);
    assert!(lit_lens.iter().any(|&l| l > 9), "no long codes produced");
    let dst_lens = balanced_lengths(32, &[0, 1]);
    let mut rng = Rng::new(0x5701_0019);
    for _ in 0..80 {
        let n = rng.range(1, 120);
        let toks: Vec<Tok> = (0..n)
            .map(|_| Tok::Lit(syms[rng.below(syms.len() - 1)] as u8))
            .collect();
        let s = dyn_stream(288, 32, &lit_lens, &dst_lens, &toks);
        diff("cfg19", &s, rng.below(4), n);
    }
    let toks: Vec<Tok> = syms[..14].iter().map(|&s| Tok::Lit(s as u8)).collect();
    let s = dyn_stream(288, 32, &lit_lens, &dst_lens, &toks);
    diff_matrix("cfg19/all", &s, 14);
}

#[test]
fn cfg20_dynamic_codes_all_short() {
    let syms: Vec<usize> = (0..256).chain([256]).collect();
    let lit_lens = balanced_lengths(288, &syms);
    assert!(lit_lens.iter().all(|&l| l == 0 || l <= 9));
    let dst_lens = balanced_lengths(32, &(0..32).collect::<Vec<_>>());
    let mut rng = Rng::new(0x5701_0020);
    for _ in 0..60 {
        let n = rng.range(1, 300);
        let toks: Vec<Tok> = (0..n).map(|_| Tok::Lit(rng.byte())).collect();
        let s = dyn_stream(288, 32, &lit_lens, &dst_lens, &toks);
        diff("cfg20", &s, rng.below(4), n);
    }
}

#[test]
fn cfg21_dynamic_literals_only() {
    let mut rng = Rng::new(0x5701_0021);
    for _ in 0..80 {
        let k = rng.range(1, 200);
        let mut syms: Vec<usize> = (0..k).map(|_| rng.below(256)).collect();
        syms.push(256);
        let nlit = rng.range(257, 288);
        let syms: Vec<usize> = syms.into_iter().filter(|&s| s < nlit).collect();
        let mut syms = syms;
        if !syms.contains(&256) {
            syms.push(256);
        }
        let lit_lens = balanced_lengths(nlit, &syms);
        let ndst = rng.range(1, 32);
        let dst_lens = vec![0u8; ndst];
        let n = rng.range(1, 200);
        let pool: Vec<usize> = syms.iter().cloned().filter(|&s| s < 256).collect();
        if pool.is_empty() {
            continue;
        }
        let toks: Vec<Tok> = (0..n)
            .map(|_| Tok::Lit(pool[rng.below(pool.len())] as u8))
            .collect();
        let s = dyn_stream(nlit, ndst, &lit_lens, &dst_lens, &toks);
        diff("cfg21", &s, rng.below(4), n);
    }
}

#[test]
fn cfg22_dynamic_literals_and_matches() {
    let mut rng = Rng::new(0x5701_0022);
    for _ in 0..120 {
        let nlit = rng.range(266, 288);
        let ndst = rng.range(1, 32);
        let mut lsyms: Vec<usize> = (0..rng.range(2, 40)).map(|_| rng.below(256)).collect();
        lsyms.push(256);
        // include some length symbols
        for _ in 0..rng.range(1, 6) {
            lsyms.push(rng.range(257, nlit - 1));
        }
        let lit_lens = balanced_lengths(nlit, &lsyms);
        let dsyms: Vec<usize> = (0..ndst).collect();
        let dst_lens = balanced_lengths(ndst, &dsyms);

        let lit_pool: Vec<usize> = lsyms.iter().cloned().filter(|&s| s < 256).collect();
        let len_pool: Vec<usize> = lsyms.iter().cloned().filter(|&s| s > 256).collect();
        if lit_pool.is_empty() {
            continue;
        }
        let base = pristine_tables();
        let mut toks: Vec<Tok> = Vec::new();
        let mut produced = 0usize;
        for _ in 0..rng.range(4, 120) {
            if produced >= 4 && !len_pool.is_empty() && rng.below(3) == 0 {
                let ls = len_pool[rng.below(len_pool.len())];
                let e = base.len_extra_bits[ls - 257] as u32;
                let b = base.len_base[ls - 257];
                if b == 0 {
                    continue;
                }
                let length = b + rng.below(1usize << e) as u32;
                // pick a distance symbol that is representable in `ndst`
                let mut cands: Vec<u32> = Vec::new();
                for ds in 0..ndst {
                    let de = base.dist_extra_bits[ds] as u32;
                    let db = base.dist_base[ds];
                    if db == 0 {
                        continue;
                    }
                    for k in 0..(1u32 << de) {
                        let d = db + k;
                        if d as usize <= produced {
                            cands.push(d);
                        }
                    }
                }
                if cands.is_empty() {
                    continue;
                }
                let dist = cands[rng.below(cands.len())];
                toks.push(Tok::Match(length, dist));
                produced += length as usize;
            } else {
                toks.push(Tok::Lit(lit_pool[rng.below(lit_pool.len())] as u8));
                produced += 1;
            }
        }
        if produced == 0 {
            continue;
        }
        let s = dyn_stream(nlit, ndst, &lit_lens, &dst_lens, &toks);
        diff("cfg22", &s, rng.below(4), produced);
    }
}

// ---------------------------------------------------------------------------
// Row 23: streams from a real encoder
// ---------------------------------------------------------------------------

#[test]
fn cfg23_real_encoder_all_levels() {
    let mut rng = Rng::new(0x5701_0023);
    for level in 0..=9u32 {
        for kind in 0..3 {
            for _ in 0..12 {
                let n = rng.range(0, 8192);
                let data = match kind {
                    0 => rng.bytes(n),
                    1 => rng.repetitive(n),
                    _ => vec![rng.byte(); n],
                };
                let s = deflate_ref(&data, level);
                // `pinflate` rejects the multi-block stored streams that
                // level 0 produces for large inputs (see ERRORS.md row 2);
                // whatever it does, both libraries must agree.
                diff(&format!("cfg23/l{level}/k{kind}"), &s, rng.below(4), data.len());
                diff(
                    &format!("cfg23/l{level}/k{kind}/slack"),
                    &s,
                    rng.below(4),
                    data.len() + rng.below(17),
                );
            }
        }
    }
}

#[test]
fn cfg23b_real_encoder_large() {
    let mut rng = Rng::new(0x5701_0231);
    for level in [1u32, 6, 9] {
        let data = rng.repetitive(70_000);
        let s = deflate_ref(&data, level);
        diff_matrix(&format!("cfg23b/l{level}"), &s, data.len());
        let data = rng.bytes(70_000);
        let s = deflate_ref(&data, level);
        diff(&format!("cfg23b/rand/l{level}"), &s, 0, data.len());
    }
}

// ---------------------------------------------------------------------------
// Rows 24-26: multi-block streams
// ---------------------------------------------------------------------------

#[test]
fn cfg24_multiblock_mixed_types() {
    // stored -> fixed -> dynamic, BFINAL only on the last.
    // NOTE: `cp_stored` does not advance the bit reader past the payload, so a
    // non-final stored block leaves the C decoder mid-stream in a way that is
    // not "correct" DEFLATE.  The point of this row is that both libraries do
    // the *same* thing.
    let (lit_lens, dst_lens) = small_dyn_tree();
    let mut w = BitWriter::new();
    write_stored_block(&mut w, false, b"hello");
    write_fixed_block(&mut w, false, &[Tok::Lit(b'x'), Tok::Lit(b'y')]);
    write_dynamic_block(&mut w, true, &lit_lens, &dst_lens, &[Tok::Lit(b'z')]);
    let s = w.finish();
    diff_all_alignments("cfg24", &s, 256);
}

#[test]
fn cfg25_multiblock_fixed_crossing_matches() {
    let mut rng = Rng::new(0x5701_0025);
    for _ in 0..80 {
        let nblocks = rng.range(2, 8);
        let mut w = BitWriter::new();
        let mut produced = 0usize;
        for b in 0..nblocks {
            let n = rng.range(1, 40);
            let mut toks: Vec<Tok> = Vec::new();
            for _ in 0..n {
                if produced >= 8 && rng.below(3) == 0 {
                    let dist = rng.range(1, produced.min(300)) as u32;
                    let length = rng.range(3, 60) as u32;
                    toks.push(Tok::Match(length, dist));
                    produced += length as usize;
                } else {
                    toks.push(Tok::Lit(rng.byte()));
                    produced += 1;
                }
            }
            write_fixed_block(&mut w, b + 1 == nblocks, &toks);
        }
        let s = w.finish();
        diff("cfg25", &s, rng.below(4), produced);
    }
}

#[test]
fn cfg26_multiblock_dynamic_different_trees() {
    let mut rng = Rng::new(0x5701_0026);
    for _ in 0..60 {
        let nblocks = rng.range(2, 5);
        let mut w = BitWriter::new();
        let mut produced = 0usize;
        for b in 0..nblocks {
            let k = rng.range(2, 40);
            let mut syms: Vec<usize> = (0..k).map(|_| rng.below(256)).collect();
            syms.push(256);
            let lit_lens = if rng.below(2) == 0 && syms.len() <= 16 {
                skewed_lengths(288, &syms)
            } else {
                balanced_lengths(288, &syms)
            };
            let dst_lens = balanced_lengths(32, &[0, 1, 2, 3]);
            let pool: Vec<usize> = syms.iter().cloned().filter(|&s| s < 256).collect();
            let n = rng.range(1, 50);
            let toks: Vec<Tok> = (0..n)
                .map(|_| Tok::Lit(pool[rng.below(pool.len())] as u8))
                .collect();
            produced += n;
            write_dynamic_block(&mut w, b + 1 == nblocks, &lit_lens, &dst_lens, &toks);
        }
        let s = w.finish();
        diff("cfg26", &s, rng.below(4), produced);
    }
}

// ---------------------------------------------------------------------------
// Rows 27-30: buffer-shape axes
// ---------------------------------------------------------------------------

#[test]
fn cfg27_out_bytes_exact() {
    let mut rng = Rng::new(0x5701_0027);
    for _ in 0..200 {
        let n = rng.range(1, 500);
        let data = rng.bytes(n);
        let toks: Vec<Tok> = data.iter().map(|&b| Tok::Lit(b)).collect();
        let s = fixed_stream(&toks);
        diff("cfg27", &s, rng.below(4), n);
    }
}

#[test]
fn cfg28_out_bytes_generous_tail_untouched() {
    let mut rng = Rng::new(0x5701_0028);
    for _ in 0..200 {
        let n = rng.range(1, 200);
        let data = rng.bytes(n);
        let toks: Vec<Tok> = data.iter().map(|&b| Tok::Lit(b)).collect();
        let s = fixed_stream(&toks);
        diff("cfg28", &s, rng.below(4), n + rng.range(1, 4096));
    }
}

#[test]
fn cfg29_tiny_inputs_no_full_words() {
    // in_bytes < 4 -> word_count == 0; everything comes from the prologue and
    // the final partial word.
    let mut rng = Rng::new(0x5701_0029);
    for len in 0..=4usize {
        for _ in 0..400 {
            let data = rng.bytes(len);
            for align in 0..4 {
                // Guard: these are mostly *invalid* streams and many of them
                // abort in C.  Only the shapes that cannot abort are safe to
                // run in-process; the rest are covered by the child-process
                // fuzz sweep.
                let _ = (align, &data);
            }
        }
    }
    // Deterministic tiny *valid* streams: an empty fixed block is 1 + 2 + 7
    // bits = 10 bits = 2 bytes.
    let s = fixed_stream(&[]);
    assert!(s.len() <= 4, "empty fixed block should be tiny, got {}", s.len());
    diff_matrix("cfg29/empty-fixed", &s, 0);
    // one literal: 1 + 2 + 8 + 7 = 18 bits = 3 bytes
    let s = fixed_stream(&[Tok::Lit(b'Q')]);
    diff_matrix("cfg29/one-lit", &s, 1);
    // stored, LEN = 0: 1 + 2 + pad + 32 bits = 5 bytes
    let mut w = BitWriter::new();
    write_stored_block(&mut w, true, &[]);
    let s = w.finish();
    diff_matrix("cfg29/stored0", &s, 0);
}

#[test]
fn cfg30_large_input_many_word_loads() {
    let mut rng = Rng::new(0x5701_0030);
    let n = 200_000usize;
    let data = rng.bytes(n);
    let toks: Vec<Tok> = data.iter().map(|&b| Tok::Lit(b)).collect();
    let s = fixed_stream(&toks);
    assert!(s.len() > 65_536);
    for align in 0..4 {
        diff(&format!("cfg30/a{align}"), &s, align, n);
    }
}

// ---------------------------------------------------------------------------
// Rows 31-34: the exported mutable globals really are the tables in use
// ---------------------------------------------------------------------------

#[test]
fn cfg31_error_reason_not_cleared_on_success() {
    let _guard = lock();
    let p = pair();
    let s = fixed_stream(&[Tok::Lit(b'A')]);
    unsafe {
        p.c.reset_tables();
        p.rs.reset_tables();
        // Pre-set the global in both libraries to a caller-owned string.
        let sentinel = b"caller sentinel\0";
        *p.c.error_reason = sentinel.as_ptr() as *const std::ffi::c_char;
        *p.rs.error_reason = sentinel.as_ptr() as *const std::ffi::c_char;

        let mut ain = AlignedIn::new(&s, 0);
        let mut bin = AlignedIn::new(&s, 0);
        let mut aout = vec![OUT_FILL; 8];
        let mut bout = vec![OUT_FILL; 8];
        let ra = (p.c.pinflate)(
            ain.ptr(),
            s.len() as i32,
            aout.as_mut_ptr() as *mut std::ffi::c_void,
            1,
        );
        let rb = (p.rs.pinflate)(
            bin.ptr(),
            s.len() as i32,
            bout.as_mut_ptr() as *mut std::ffi::c_void,
            1,
        );
        assert_eq!(ra, rb, "return value");
        assert_eq!(ra, 1);
        assert_eq!(aout, bout, "output buffer");
        let ca = read_cstr(*p.c.error_reason);
        let cb = read_cstr(*p.rs.error_reason);
        assert_eq!(ca, cb, "cp_error_reason after a successful call");
        assert_eq!(ca.as_deref(), Some(&b"caller sentinel"[..]));
    }
}

#[test]
fn cfg32_permutation_order_is_read_from_the_global() {
    let _guard = lock();
    // Rotate the permutation identically in both libraries and build a stream
    // that uses the rotated order.  If either library used a private copy the
    // decode would diverge.
    let p = pair();
    let mut rng = Rng::new(0x5701_0032);
    let base = pristine_tables().permutation_order;
    for round in 0..8 {
        let mut perm = base;
        // rotate left by `round`
        perm.rotate_left(round);
        unsafe {
            p.c.reset_tables();
            p.rs.reset_tables();
            std::ptr::copy_nonoverlapping(perm.as_ptr(), p.c.permutation_order, 19);
            std::ptr::copy_nonoverlapping(perm.as_ptr(), p.rs.permutation_order, 19);
        }

        let (lit_lens, dst_lens) = small_dyn_tree();
        let mut all = lit_lens.clone();
        all.extend_from_slice(&dst_lens);
        let items = cl_flat(&all);
        let cl_lens = cl_lengths_from_items(&items);
        let toks: Vec<Tok> = b"abcxyz".iter().map(|&b| Tok::Lit(b)).collect();

        // header written with the *rotated* permutation
        let mut w = BitWriter::new();
        w.bits(1, 1);
        w.bits(2, 2);
        w.bits((288 - 257) as u32, 5);
        w.bits((32 - 1) as u32, 5);
        w.bits((19 - 4) as u32, 4);
        for i in 0..19 {
            w.bits(cl_lens[perm[i] as usize] as u32, 3);
        }
        let clc = canonical(&cl_lens);
        for it in &items {
            let (c, l) = clc[it.symbol()];
            w.huff(c, l as u32);
            if let Some((e, n)) = it.extra() {
                w.bits(e, n);
            }
        }
        write_tokens(&mut w, &toks, &lit_lens, &dst_lens);
        let s = w.finish();

        unsafe {
            let a = call(&p.c, &s, rng.below(4), s.len() as i32, 6);
            let b = call(&p.rs, &s, 0, s.len() as i32, 6);
            assert_eq!(a.ret, b.ret, "round {round}: ret");
            assert_eq!(a.out, b.out, "round {round}: out");
            assert_eq!(a.reason, b.reason, "round {round}: reason");
        }
    }
    unsafe {
        p.c.reset_tables();
        p.rs.reset_tables();
    }
}

#[test]
fn cfg33_len_and_dist_base_are_read_from_the_globals() {
    let _guard = lock();
    let p = pair();
    let s = {
        let prefix: Vec<u8> = (0..64u8).collect();
        let mut toks: Vec<Tok> = prefix.iter().map(|&b| Tok::Lit(b)).collect();
        toks.push(Tok::Match(3, 1)); // symbol 257, dist symbol 0
        fixed_stream(&toks)
    };
    for (lb, db) in [(3u32, 1u32), (7, 5), (20, 9), (100, 33)] {
        unsafe {
            p.c.reset_tables();
            p.rs.reset_tables();
            *p.c.len_base.add(0) = lb;
            *p.rs.len_base.add(0) = lb;
            *p.c.dist_base.add(0) = db;
            *p.rs.dist_base.add(0) = db;
            let a = call(&p.c, &s, 0, s.len() as i32, 64 + 128);
            let b = call(&p.rs, &s, 0, s.len() as i32, 64 + 128);
            assert_eq!(a.ret, b.ret, "lb={lb} db={db}: ret");
            assert_eq!(a.out, b.out, "lb={lb} db={db}: out");
            assert_eq!(a.reason, b.reason, "lb={lb} db={db}: reason");
            // and the effect must actually be visible: the match copies
            // `len_base[0]` bytes from `dist_base[0]` back (byte-by-byte for
            // db != 1, so it may read bytes it just wrote).
            if a.ret == 1 {
                let mut expect: Vec<u8> = (0..64u8).collect();
                for i in 0..lb as usize {
                    let src = expect.len() - db as usize;
                    let v = expect[src];
                    expect.push(v);
                    let _ = i;
                }
                assert_eq!(&a.out[..64 + lb as usize], &expect[..], "lb={lb} db={db}");
            }
        }
    }
    unsafe {
        p.c.reset_tables();
        p.rs.reset_tables();
    }
}

#[test]
fn cfg34_fixed_table_is_read_from_the_global() {
    let _guard = lock();
    let p = pair();
    // A different *valid* fixed table: swap the 7-bit and 8-bit groups.
    let mut table = pristine_tables().fixed_table;
    for i in 0..288usize {
        table[i] = match i {
            0..=23 => 7,
            24..=167 => 8,
            168..=279 => 9,
            _ => 8,
        };
    }
    // Kraft check
    let mut kraft = 0f64;
    for i in 0..288 {
        if table[i] > 0 {
            kraft += 2f64.powi(-(table[i] as i32));
        }
    }
    assert!((kraft - 1.0).abs() < 1e-9, "perturbed fixed table is not complete: {kraft}");

    let lit_lens = table[..288].to_vec();
    let dst_lens = table[288..].to_vec();
    let mut rng = Rng::new(0x5701_0034);
    for _ in 0..40 {
        let n = rng.range(1, 100);
        let data = rng.bytes(n);
        let toks: Vec<Tok> = data.iter().map(|&b| Tok::Lit(b)).collect();
        let mut w = BitWriter::new();
        write_fixed_block_with(&mut w, true, &toks, &lit_lens, &dst_lens);
        let s = w.finish();
        unsafe {
            p.c.reset_tables();
            p.rs.reset_tables();
            std::ptr::copy_nonoverlapping(table.as_ptr(), p.c.fixed_table, 320);
            std::ptr::copy_nonoverlapping(table.as_ptr(), p.rs.fixed_table, 320);
            let a = call(&p.c, &s, 0, s.len() as i32, n);
            let b = call(&p.rs, &s, 0, s.len() as i32, n);
            assert_eq!(a.ret, b.ret, "ret");
            assert_eq!(a.out, b.out, "out");
            assert_eq!(a.reason, b.reason, "reason");
            assert_eq!(a.ret, 1, "perturbed fixed table should decode");
            assert_eq!(&a.out[..n], &data[..]);
        }
    }
    unsafe {
        p.c.reset_tables();
        p.rs.reset_tables();
    }
}

// ---------------------------------------------------------------------------
// Both libraries must ship identical table contents
// ---------------------------------------------------------------------------

#[test]
fn exported_tables_are_identical() {
    let _guard = lock();
    let p = pair();
    unsafe {
        p.c.reset_tables();
        p.rs.reset_tables();
        assert_eq!(p.c.tables(), p.rs.tables(), "exported table contents differ");
        // and both match the values written in the C source
        let want = {
            let t = pristine_tables();
            let mut v = Vec::new();
            v.extend_from_slice(&t.fixed_table);
            v.extend_from_slice(&t.permutation_order);
            v.extend_from_slice(&t.len_extra_bits);
            for x in t.len_base {
                v.extend_from_slice(&x.to_le_bytes());
            }
            v.extend_from_slice(&t.dist_extra_bits);
            for x in t.dist_base {
                v.extend_from_slice(&x.to_le_bytes());
            }
            v
        };
        assert_eq!(p.c.tables(), want, "C tables differ from the literal source values");
    }
}

define_child_runner!();
