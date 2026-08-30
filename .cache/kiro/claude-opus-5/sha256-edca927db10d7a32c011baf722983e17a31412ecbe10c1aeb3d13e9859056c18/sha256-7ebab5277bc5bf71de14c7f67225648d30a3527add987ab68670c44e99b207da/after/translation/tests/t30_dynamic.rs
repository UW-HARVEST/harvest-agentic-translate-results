//! Level 3: `cp_dynamic`.
//!
//! Dynamic blocks are the only path that reaches `cp_build` with caller-chosen
//! code lengths, the code-length alphabet in `cp_permutation_order` order, and
//! the run-length symbols 16/17/18. They also drive `cp_decode` with trees of
//! every depth from 1 to 15 bits.

mod harness;

use harness::deflate::*;
use harness::Differ;

const PAD: usize = 16;

fn dynamic_stream(
    tokens: &[Token],
    lit_lens: &[u8],
    dist_lens: &[u8],
    enc: ClEncoding,
    hclen_full: bool,
) -> Vec<u8> {
    let mut w = BitWriter::new();
    write_dynamic_block(&mut w, true, tokens, lit_lens, dist_lens, enc, hclen_full);
    with_padding(w.finish(), PAD)
}

#[test]
fn dynamic_literals_all_cl_encodings() {
    let mut d = Differ::new();
    let text = b"the quick brown fox jumps over the lazy dog 0123456789";
    let tokens: Vec<Token> = text.iter().copied().map(Token::Lit).collect();
    let (lit_lens, dist_lens) = tables_for(&tokens, 257, 1);
    let expected = expand(&tokens);
    for enc in [ClEncoding::Literal, ClEncoding::ZeroRuns, ClEncoding::Full] {
        for hclen_full in [false, true] {
            let stream = dynamic_stream(&tokens, &lit_lens, &dist_lens, enc, hclen_full);
            for offset in 0..8usize {
                d.check_ok(
                    &format!("dyn literals {enc:?} hclen_full={hclen_full} off={offset}"),
                    &stream,
                    offset,
                    expected.len() + 8,
                    &expected,
                );
            }
        }
    }
    d.finish("dynamic literals");
}

#[test]
fn dynamic_all_hlit_hdist_sizes() {
    // HLIT spans 257..=288 and HDIST 1..=32; every combination changes how many
    // code lengths `cp_dynamic` reads and how `cp_build` is called.
    let mut d = Differ::new();
    for nlit in [257usize, 258, 260, 270, 280, 287, 288] {
        for ndst in [1usize, 2, 3, 5, 16, 31, 32] {
            let mut tokens: Vec<Token> = (0..6u8).map(|i| Token::Lit(b'a' + i)).collect();
            // A match needs length symbol 257 (len 3) and a distance symbol
            // inside HDIST; distances 1, 2 and 3 map to symbols 0, 1 and 2.
            if nlit > 257 {
                tokens.push(Token::Match {
                    len: 3,
                    dist: ndst.min(3) as u32,
                });
            }
            let (lit_lens, dist_lens) = tables_for(&tokens, nlit, ndst);
            let expected = expand(&tokens);
            for enc in [ClEncoding::Literal, ClEncoding::Full] {
                let stream = dynamic_stream(&tokens, &lit_lens, &dist_lens, enc, true);
                d.check_ok(
                    &format!("nlit={nlit} ndst={ndst} {enc:?}"),
                    &stream,
                    0,
                    expected.len() + 8,
                    &expected,
                );
            }
        }
    }
    d.finish("dynamic HLIT/HDIST");
}

#[test]
fn dynamic_code_lengths_up_to_15_bits() {
    // A deliberately lopsided code: symbol lengths 1,2,3,...,15,15 form a
    // complete Huffman code, so `cp_decode`'s binary search has to walk trees
    // of maximum depth and `cp_build`'s `len <= 9` lookup branch is exercised
    // on both sides of the boundary.
    let mut d = Differ::new();
    let mut lit_lens = vec![0u8; 257];
    let symbols: Vec<usize> = (0..16).map(|i| 32 + i * 7).collect();
    for (i, &s) in symbols.iter().enumerate() {
        lit_lens[s] = if i == 15 { 15 } else { (i + 1) as u8 };
    }
    lit_lens[256] = 15; // end-of-block shares the deepest level
    // Rebuild: lengths 1..14 on 14 symbols plus two 15s -> Kraft sum
    // 1/2 + 1/4 + ... + 1/2^14 + 2/2^15 = 1.
    let mut lens = vec![0u8; 257];
    for i in 0..14usize {
        lens[symbols[i]] = (i + 1) as u8;
    }
    lens[symbols[14]] = 15;
    lens[256] = 15;
    lit_lens = lens;

    let dist_lens = vec![1u8, 1];
    let mut tokens: Vec<Token> = Vec::new();
    for &s in symbols.iter().take(14) {
        tokens.push(Token::Lit(s as u8));
    }
    tokens.push(Token::Lit(symbols[14] as u8));
    let expected = expand(&tokens);

    for enc in [ClEncoding::Literal, ClEncoding::ZeroRuns, ClEncoding::Full] {
        for offset in 0..4usize {
            let stream = dynamic_stream(&tokens, &lit_lens, &dist_lens, enc, true);
            d.check_ok(
                &format!("deep tree {enc:?} off={offset}"),
                &stream,
                offset,
                expected.len() + 8,
                &expected,
            );
        }
    }
    d.finish("dynamic deep trees");
}

#[test]
fn dynamic_uniform_code_lengths() {
    // Complete codes where every used symbol has the same length, for every
    // length 1..=9 (the `cp_build` lookup path) and 10..=15 (beyond it).
    let mut d = Differ::new();
    for bits in 1..=9usize {
        let count = 1usize << bits;
        if count > 257 {
            continue;
        }
        let mut lit_lens = vec![0u8; 257];
        let mut used: Vec<usize> = vec![256];
        let mut sym = 0usize;
        while used.len() < count {
            if sym != 256 {
                used.push(sym);
            }
            sym += 1;
        }
        for &u in &used {
            lit_lens[u] = bits as u8;
        }
        let tokens: Vec<Token> = used
            .iter()
            .filter(|&&u| u < 256)
            .take(40)
            .map(|&u| Token::Lit(u as u8))
            .collect();
        let dist_lens = vec![1u8, 1];
        let expected = expand(&tokens);
        for enc in [ClEncoding::Literal, ClEncoding::Full] {
            let stream = dynamic_stream(&tokens, &lit_lens, &dist_lens, enc, true);
            d.check_ok(
                &format!("uniform bits={bits} {enc:?}"),
                &stream,
                0,
                expected.len() + 8,
                &expected,
            );
        }
    }
    d.finish("dynamic uniform lengths");
}

#[test]
fn dynamic_with_matches_and_all_length_symbols() {
    let mut d = Differ::new();
    for len in 3..=258u32 {
        let mut tokens: Vec<Token> = (0..8u8).map(|i| Token::Lit(b'p' + (i % 8))).collect();
        tokens.push(Token::Match { len, dist: 8 });
        tokens.push(Token::Match { len: 3, dist: 1 });
        let (lit_lens, dist_lens) = tables_for(&tokens, 288, 30);
        let expected = expand(&tokens);
        d.check_ok(
            &format!("dyn len={len}"),
            &dynamic_stream(&tokens, &lit_lens, &dist_lens, ClEncoding::Full, true),
            0,
            expected.len() + 8,
            &expected,
        );
    }
    d.finish("dynamic length symbols");
}

#[test]
fn dynamic_all_distance_symbols() {
    let mut d = Differ::new();
    for sym in 0..30usize {
        let base = DIST_BASE[sym];
        let max = base + ((1u32 << DIST_EXTRA[sym]) - 1);
        for dist in [base, max] {
            if dist > 32768 {
                continue;
            }
            let mut tokens: Vec<Token> = vec![Token::Lit(b'W')];
            let mut have = 1u32;
            while have < dist {
                let n = (dist - have).min(258);
                if n < 3 {
                    for _ in 0..n {
                        tokens.push(Token::Lit(b'V'));
                        have += 1;
                    }
                } else {
                    tokens.push(Token::Match { len: n, dist: 1 });
                    have += n;
                }
            }
            tokens.push(Token::Match { len: 3, dist });
            let (lit_lens, dist_lens) = tables_for(&tokens, 288, 30);
            let expected = expand(&tokens);
            d.check_ok(
                &format!("dyn dist sym={sym} dist={dist}"),
                &dynamic_stream(&tokens, &lit_lens, &dist_lens, ClEncoding::Full, true),
                0,
                expected.len() + 8,
                &expected,
            );
        }
    }
    d.finish("dynamic distance symbols");
}

#[test]
fn dynamic_run_length_symbol_boundaries() {
    // Symbol 16 repeats 3..=6, symbol 17 zero-runs 3..=10, symbol 18 zero-runs
    // 11..=138. Shape the code-length sequence so every extra-bit value of each
    // of the three shows up.
    //
    // The block itself carries no tokens -- only the header is interesting --
    // so the literal alphabet is three symbols with lengths 1, 2, 2 (Kraft sum
    // 1/2 + 1/4 + 1/4 = 1) placed to leave a zero run of exactly `zeros`.
    let mut d = Differ::new();
    let nlit = 288usize;
    for zeros in 0..=145usize {
        let x = 0usize;
        let y = 1 + zeros;
        if y >= nlit || y == 256 {
            continue;
        }
        let mut lit_lens = vec![0u8; nlit];
        lit_lens[256] = 1;
        lit_lens[x] = 2;
        lit_lens[y] = 2;
        let dist_lens = vec![1u8, 1];
        for enc in [ClEncoding::Literal, ClEncoding::ZeroRuns, ClEncoding::Full] {
            d.check_ok(
                &format!("zero-run={zeros} {enc:?}"),
                &dynamic_stream(&[], &lit_lens, &dist_lens, enc, true),
                0,
                16,
                &[],
            );
        }
    }

    // Long runs of an identical non-zero length, which is what drives symbol 16.
    for bits in 1..=9usize {
        let count = 1usize << bits;
        if count >= nlit {
            continue;
        }
        let mut lit_lens = vec![0u8; nlit];
        let mut used: Vec<usize> = vec![256];
        let mut sym = 0usize;
        while used.len() < count {
            if sym != 256 {
                used.push(sym);
            }
            sym += 1;
        }
        for &u in &used {
            lit_lens[u] = bits as u8;
        }
        let dist_lens = vec![1u8, 1];
        for enc in [ClEncoding::Literal, ClEncoding::Full] {
            d.check_ok(
                &format!("repeat-run bits={bits} {enc:?}"),
                &dynamic_stream(&[], &lit_lens, &dist_lens, enc, true),
                0,
                16,
                &[],
            );
        }
    }
    d.finish("dynamic RLE symbols");
}

#[test]
fn dynamic_hclen_every_size() {
    // HCLEN is a 4-bit field, i.e. 4..=19 code-length entries read in
    // `cp_permutation_order` order. Only symbols whose permutation index is
    // below HCLEN can carry a length, so the smallest sizes only allow the
    // symbols 16, 17, 18 and 0 -- which cannot describe a usable tree. Those
    // streams are therefore driven as malformed input, with C as the reference.
    let mut d = Differ::new();
    let tokens = vec![Token::Lit(b'a'), Token::Lit(b'b')];
    let (lit_lens, dist_lens) = tables_for(&tokens, 257, 1);
    let mut w = BitWriter::new();
    write_dynamic_block(
        &mut w,
        true,
        &tokens,
        &lit_lens,
        &dist_lens,
        ClEncoding::Literal,
        false,
    );
    let base = with_padding(w.finish(), PAD);

    for hclen in 4..=19usize {
        let mut bytes = base.clone();
        // Patch HCLEN in place: it sits at bits 13..17 (BFINAL+BTYPE+HLIT+HDIST).
        let bitpos = 1 + 2 + 5 + 5;
        let value = (hclen - 4) as u32;
        for i in 0..4usize {
            let b = bitpos + i;
            let bit = (value >> i) & 1;
            let byte = &mut bytes[b / 8];
            if bit == 1 {
                *byte |= 1 << (b % 8);
            } else {
                *byte &= !(1u8 << (b % 8));
            }
        }
        d.check(&format!("hclen patched {hclen}"), &bytes, 0, 64);
    }

    // And the well-formed extremes: the natural minimum HCLEN for a stream and
    // the maximum of 19.
    for hclen_full in [false, true] {
        let stream = dynamic_stream(&tokens, &lit_lens, &dist_lens, ClEncoding::Literal, hclen_full);
        let expected = expand(&tokens);
        d.check_ok(
            &format!("hclen_full={hclen_full}"),
            &stream,
            0,
            expected.len() + 8,
            &expected,
        );
    }
    d.finish("dynamic HCLEN");
}

#[test]
fn dynamic_incomplete_and_degenerate_trees() {
    // Malformed-but-parseable header shapes. C is the reference for whatever
    // these do (including asserting); the requirement is only that Rust agrees.
    let mut d = Differ::new();

    // Single-symbol literal code (incomplete: Kraft sum 1/2).
    let mut lit_lens = vec![0u8; 257];
    lit_lens[256] = 1;
    let dist_lens = vec![1u8];
    let mut w = BitWriter::new();
    write_dynamic_block(&mut w, true, &[], &lit_lens, &dist_lens, ClEncoding::Literal, true);
    d.check("single-symbol lit code", &with_padding(w.finish(), PAD), 0, 64);

    // Over-subscribed code (Kraft sum 2).
    let mut lit_lens = vec![0u8; 257];
    for s in [0usize, 1, 2, 256] {
        lit_lens[s] = 1;
    }
    let mut w = BitWriter::new();
    write_dynamic_block(&mut w, true, &[], &lit_lens, &dist_lens, ClEncoding::Literal, true);
    d.check("oversubscribed lit code", &with_padding(w.finish(), PAD), 0, 64);

    // All-zero distance lengths.
    let (lit_lens, _) = tables_for(&[Token::Lit(b'a')], 257, 1);
    let mut w = BitWriter::new();
    write_dynamic_block(
        &mut w,
        true,
        &[],
        &lit_lens,
        &[0u8],
        ClEncoding::Literal,
        true,
    );
    d.check("empty dist code", &with_padding(w.finish(), PAD), 0, 64);

    // All-zero literal lengths: `cp_build` returns 0 and `cp_decode` then reads
    // `tree[-1]`.
    let mut w = BitWriter::new();
    write_dynamic_block(
        &mut w,
        true,
        &[],
        &vec![0u8; 257],
        &[0u8],
        ClEncoding::Literal,
        true,
    );
    d.check("all-zero lit code", &with_padding(w.finish(), PAD), 0, 64);

    d.finish("dynamic degenerate trees");
}
