//! Phase B, rows 1-20 of `CONFIGS.md`: the low-level `cp_inflate` entry point.
//!
//! Run with `--test-threads=1` for the most deterministic fork behaviour.

mod harness;

use harness::make::*;
use harness::*;

fn inflate(label: String, deflate: Vec<u8>, align: usize, out_bytes: i32) -> Case {
    Case::inflate(label, deflate, align, out_bytes)
}

/// Row 1-3: stored blocks. `cp_stored` demands `bits_left / 8 <= LEN`, so a
/// stored block is only accepted as the last thing in the stream, with `LEN`
/// equal to the number of bytes that follow it.
#[test]
fn row_01_03_stored() {
    let pair = load_pair();
    let mut rng = Rng::new(1);
    let mut cases = Vec::new();
    for align in 0..4usize {
        for &len in &[1usize, 2, 3, 4, 5, 7, 8, 9, 15, 16, 17, 33, 64, 255, 300] {
            let payload = rng.bytes(len);
            let mut bw = BitW::new();
            block_stored(&mut bw, true, &payload);
            let d = bw.finish();
            for extra in [0i32, 1, 7] {
                cases.push(inflate(
                    format!("stored align={align} len={len} slack={extra}"),
                    d.clone(),
                    align,
                    len as i32 + extra,
                ));
            }
        }
    }
    // and a pile of random sizes
    for i in 0..200 {
        let len = rng.range(1, 200) as usize;
        let payload = rng.bytes(len);
        let mut bw = BitW::new();
        block_stored(&mut bw, true, &payload);
        cases.push(inflate(
            format!("stored random {i}"),
            bw.finish(),
            (rng.below(4)) as usize,
            len as i32 + rng.below(8) as i32,
        ));
    }
    assert_same(&pair, &cases);
}

/// Rows 4, 5, 10: fixed-Huffman literal-only blocks.
#[test]
fn row_04_05_10_fixed_literals() {
    let pair = load_pair();
    let mut rng = Rng::new(2);
    let codes = Codes::fixed();
    let mut cases = Vec::new();

    // every literal value at least once, in blocks of every size class
    for align in 0..4usize {
        for &n in &[1usize, 2, 3, 4, 5, 8, 16, 17, 31, 32, 64, 256, 257, 511] {
            let data: Vec<u8> = (0..n).map(|i| ((i * 7 + n) & 0xFF) as u8).collect();
            let toks: Vec<Tok> = data.iter().map(|b| Tok::Lit(*b)).collect();
            let mut bw = BitW::new();
            block_fixed(&mut bw, true, &toks, &codes);
            let d = bw.finish();
            for slack in [0i32, 1, 5] {
                cases.push(inflate(
                    format!("fixed lit n={n} align={align} slack={slack}"),
                    d.clone(),
                    align,
                    n as i32 + slack,
                ));
            }
        }
    }
    // all 256 literal values (covers the 8-bit and the 9-bit halves of the
    // fixed table)
    let all: Vec<u8> = (0..=255u8).collect();
    let toks: Vec<Tok> = all.iter().map(|b| Tok::Lit(*b)).collect();
    let mut bw = BitW::new();
    block_fixed(&mut bw, true, &toks, &codes);
    cases.push(inflate("fixed lit all 256".into(), bw.finish(), 0, 256));

    // random literal streams
    for i in 0..200 {
        let n = rng.range(1, 400) as usize;
        let data = rng.bytes(n);
        let toks: Vec<Tok> = data.iter().map(|b| Tok::Lit(*b)).collect();
        let mut bw = BitW::new();
        block_fixed(&mut bw, true, &toks, &codes);
        cases.push(inflate(
            format!("fixed lit random {i}"),
            bw.finish(),
            rng.below(4) as usize,
            n as i32 + rng.below(4) as i32,
        ));
    }
    assert_same(&pair, &cases);
}

/// Rows 6-9: fixed-Huffman blocks with back-references -- the `memset`
/// (distance 1) path, the byte-copy path, overlapping copies, and every
/// length/distance symbol that carries extra bits.
#[test]
fn row_06_09_fixed_matches() {
    let pair = load_pair();
    let mut rng = Rng::new(3);
    let codes = Codes::fixed();
    let mut cases = Vec::new();

    // row 6: distance == 1 -> memset
    for &len in &[3u32, 4, 5, 10, 11, 12, 258] {
        for align in 0..4usize {
            let toks = vec![Tok::Lit(0xA5), Tok::Match(len, 1)];
            let out = expand(&toks);
            let mut bw = BitW::new();
            block_fixed(&mut bw, true, &toks, &codes);
            cases.push(inflate(
                format!("memset len={len} align={align}"),
                bw.finish(),
                align,
                out.len() as i32,
            ));
        }
    }

    // rows 7-9: every length symbol x every distance symbol that fits in a
    // reasonable output buffer
    for ls in 0..29usize {
        let base = LEN_BASE[ls];
        if base == 0 {
            continue;
        }
        for extra in [0u32, 1, (1u32 << LEN_EXTRA[ls]) - 1] {
            let length = base + extra.min((1u32 << LEN_EXTRA[ls]) - 1);
            for ds in 0..30usize {
                let dist = DIST_BASE[ds];
                if dist > 2048 {
                    continue;
                }
                let dx = ((1u32 << DIST_EXTRA[ds]) - 1).min(3);
                for d in [dist, dist + dx] {
                    if d > 2048 {
                        continue;
                    }
                    let mut toks: Vec<Tok> =
                        (0..d).map(|i| Tok::Lit((i & 0xFF) as u8 ^ 0x5A)).collect();
                    toks.push(Tok::Match(length, d));
                    let out = expand(&toks);
                    let mut bw = BitW::new();
                    block_fixed(&mut bw, true, &toks, &codes);
                    cases.push(inflate(
                        format!("match ls={ls} len={length} ds={ds} dist={d}"),
                        bw.finish(),
                        (ls + ds) % 4,
                        out.len() as i32,
                    ));
                }
            }
        }
    }

    // a few big distances
    for ds in 18..30usize {
        let dist = DIST_BASE[ds];
        if dist > 30000 {
            continue;
        }
        let mut toks: Vec<Tok> = (0..dist).map(|i| Tok::Lit((i * 31 & 0xFF) as u8)).collect();
        toks.push(Tok::Match(258, dist));
        let out = expand(&toks);
        let mut bw = BitW::new();
        block_fixed(&mut bw, true, &toks, &codes);
        cases.push(inflate(
            format!("far match ds={ds} dist={dist}"),
            bw.finish(),
            ds % 4,
            out.len() as i32,
        ));
    }

    // randomized mixtures
    for i in 0..250 {
        let mut toks: Vec<Tok> = Vec::new();
        let mut produced = 0usize;
        for _ in 0..rng.range(1, 40) {
            if produced >= 4 && rng.bool() {
                let dist = rng.range(1, produced.min(600) as u32);
                let len = rng.range(3, 60);
                toks.push(Tok::Match(len, dist));
                produced += len as usize;
            } else {
                toks.push(Tok::Lit(rng.byte()));
                produced += 1;
            }
        }
        let out = expand(&toks);
        let mut bw = BitW::new();
        block_fixed(&mut bw, true, &toks, &codes);
        cases.push(inflate(
            format!("fixed mix {i}"),
            bw.finish(),
            rng.below(4) as usize,
            out.len() as i32 + rng.below(3) as i32,
        ));
    }
    assert_same(&pair, &cases);
}

/// Row 10 (continued): the fixed literal alphabet has codes for symbols 286 and
/// 287, which are *not* valid length symbols -- `cp_len_base[29] == 0`, so the
/// C computes `length = 0` and then still reads a distance symbol.
#[test]
fn row_10_symbols_286_287() {
    let pair = load_pair();
    let codes = Codes::fixed();
    let mut cases = Vec::new();
    for sym in [286usize, 287] {
        for dsym in [0usize, 1, 5] {
            let mut bw = BitW::new();
            bw.bits(1, 1);
            bw.bits(1, 2);
            // 4 literals so a back-reference has somewhere to point
            for b in [1u8, 2, 3, 4] {
                bw.huff(codes.lit_codes[b as usize], codes.lit_lens[b as usize]);
            }
            bw.huff(codes.lit_codes[sym], codes.lit_lens[sym]);
            bw.huff(codes.dst_codes[dsym], codes.dst_lens[dsym]);
            bw.bits(0, DIST_EXTRA[dsym] as usize);
            bw.huff(codes.lit_codes[256], codes.lit_lens[256]);
            cases.push(inflate(format!("sym {sym} dist sym {dsym}"), bw.finish(), 0, 8));
        }
    }
    assert_same(&pair, &cases);
}

/// Rows 11-14: dynamic-Huffman blocks.
#[test]
fn row_11_14_dynamic() {
    let pair = load_pair();
    let mut rng = Rng::new(4);
    let mut cases = Vec::new();

    // row 11 + 12: literal-only dynamic blocks, both code-length encodings
    for enc in [ClEncoding::Literal, ClEncoding::RunLength] {
        for i in 0..120 {
            let n = rng.range(1, 300) as usize;
            let data = rng.bytes(n);
            let toks: Vec<Tok> = data.iter().map(|b| Tok::Lit(*b)).collect();
            let (lit, dst) = random_codes_for(&mut rng, &toks);
            let mut bw = BitW::new();
            block_dynamic(
                &mut bw,
                true,
                &toks,
                &lit,
                &dst,
                &PERMUTATION_ORDER,
                enc,
                &mut rng,
            );
            cases.push(inflate(
                format!("dynamic lit {enc:?} {i}"),
                bw.finish(),
                rng.below(4) as usize,
                n as i32 + rng.below(3) as i32,
            ));
        }
    }

    // row 14: dynamic blocks with back-references
    for i in 0..150 {
        let mut toks: Vec<Tok> = Vec::new();
        let mut produced = 0usize;
        for _ in 0..rng.range(2, 30) {
            if produced >= 5 && rng.bool() {
                let dist = rng.range(1, produced.min(500) as u32);
                let len = rng.range(3, 100);
                toks.push(Tok::Match(len, dist));
                produced += len as usize;
            } else {
                toks.push(Tok::Lit(rng.byte()));
                produced += 1;
            }
        }
        let out = expand(&toks);
        let (lit, dst) = random_codes_for(&mut rng, &toks);
        let enc = if rng.bool() {
            ClEncoding::Literal
        } else {
            ClEncoding::RunLength
        };
        let mut bw = BitW::new();
        block_dynamic(
            &mut bw,
            true,
            &toks,
            &lit,
            &dst,
            &PERMUTATION_ORDER,
            enc,
            &mut rng,
        );
        cases.push(inflate(
            format!("dynamic match {i}"),
            bw.finish(),
            rng.below(4) as usize,
            out.len() as i32,
        ));
    }
    assert_same(&pair, &cases);
}

/// Row 13: `nlit`/`ndst`/`nlen` at their extremes.
#[test]
fn row_13_dynamic_extremes() {
    let pair = load_pair();
    let mut rng = Rng::new(5);
    let mut cases = Vec::new();

    // nlit == 257, ndst == 1: 256 of the 257 literal symbols get length 8
    // (a complete code) and the single distance symbol is unused (length 0).
    for drop in [0usize, 1, 255] {
        let mut lit = vec![8u8; 257];
        lit[drop] = 0;
        if drop == 256 {
            continue;
        }
        let dst = vec![0u8; 1];
        let data: Vec<u8> = (0..40u8).map(|i| i.wrapping_add(1)).collect();
        // `drop` is the one symbol without a code, so substitute any other
        let subst = ((drop + 1) % 256) as u8;
        let toks: Vec<Tok> = data
            .iter()
            .map(|b| Tok::Lit(if *b as usize == drop { subst } else { *b }))
            .collect();
        let mut bw = BitW::new();
        block_dynamic(
            &mut bw,
            true,
            &toks,
            &lit,
            &dst,
            &PERMUTATION_ORDER,
            ClEncoding::RunLength,
            &mut rng,
        );
        cases.push(inflate(
            format!("nlit=257 ndst=1 drop={drop}"),
            bw.finish(),
            0,
            expand(&toks).len() as i32,
        ));
    }

    // nlit == 288, ndst == 32
    for i in 0..40 {
        let mut toks: Vec<Tok> = Vec::new();
        let mut produced = 0usize;
        for _ in 0..20 {
            if produced >= 40 && rng.bool() {
                let dist = rng.range(1, produced.min(300) as u32);
                toks.push(Tok::Match(rng.range(3, 30), dist));
                produced = expand(&toks).len();
            } else {
                toks.push(Tok::Lit(rng.byte()));
                produced += 1;
            }
        }
        // build complete codes over the full 288/32 alphabets
        let lit_syms: Vec<usize> = (0..288).collect();
        let dst_syms: Vec<usize> = (0..32).collect();
        let ld = random_complete_depths(&mut rng, 288, 15);
        let dd = random_complete_depths(&mut rng, 32, 15);
        let lit = lengths_for(&lit_syms, &ld, 288);
        let dst = lengths_for(&dst_syms, &dd, 32);
        let mut bw = BitW::new();
        block_dynamic(
            &mut bw,
            true,
            &toks,
            &lit,
            &dst,
            &PERMUTATION_ORDER,
            if i % 2 == 0 {
                ClEncoding::Literal
            } else {
                ClEncoding::RunLength
            },
            &mut rng,
        );
        cases.push(inflate(
            format!("nlit=288 ndst=32 {i}"),
            bw.finish(),
            i as usize % 4,
            expand(&toks).len() as i32,
        ));
    }
    assert_same(&pair, &cases);
}

/// Rows 15-16: multi-block streams.
#[test]
fn row_15_16_multi_block() {
    let pair = load_pair();
    let mut rng = Rng::new(6);
    let codes = Codes::fixed();
    let mut cases = Vec::new();

    for i in 0..150 {
        let nblocks = rng.range(2, 4) as usize;
        let mut bw = BitW::new();
        let mut total = 0usize;
        for b in 0..nblocks {
            let last = b == nblocks - 1;
            let n = rng.range(1, 60) as usize;
            let data = rng.bytes(n);
            let toks: Vec<Tok> = data.iter().map(|x| Tok::Lit(*x)).collect();
            if rng.bool() {
                block_fixed(&mut bw, last, &toks, &codes);
            } else {
                let (lit, dst) = random_codes_for(&mut rng, &toks);
                block_dynamic(
                    &mut bw,
                    last,
                    &toks,
                    &lit,
                    &dst,
                    &PERMUTATION_ORDER,
                    if rng.bool() {
                        ClEncoding::Literal
                    } else {
                        ClEncoding::RunLength
                    },
                    &mut rng,
                );
            }
            total += n;
        }
        cases.push(inflate(
            format!("multi {i}"),
            bw.finish(),
            rng.below(4) as usize,
            total as i32,
        ));
    }

    // row 16: N-1 Huffman blocks then a final stored block
    for i in 0..60 {
        let mut bw = BitW::new();
        let n = rng.range(1, 40) as usize;
        let data = rng.bytes(n);
        let toks: Vec<Tok> = data.iter().map(|x| Tok::Lit(*x)).collect();
        block_fixed(&mut bw, false, &toks, &codes);
        let plen = rng.range(1, 80) as usize;
        let payload = rng.bytes(plen);
        block_stored(&mut bw, true, &payload);
        cases.push(inflate(
            format!("fixed+stored {i}"),
            bw.finish(),
            rng.below(4) as usize,
            (n + plen) as i32 + 8,
        ));
    }
    assert_same(&pair, &cases);
}

/// Rows 17-18: retuning `cp_len_base` / `cp_len_extra_bits` /
/// `cp_dist_base` / `cp_dist_extra_bits` at runtime.
#[test]
fn row_17_18_retuned_len_dist_tables() {
    let pair = load_pair();
    let mut rng = Rng::new(7);
    let codes = Codes::fixed();
    let mut cases = Vec::new();

    for i in 0..160 {
        // a small stream with one back-reference
        let mut toks: Vec<Tok> = (0..40u8).map(|b| Tok::Lit(b ^ 0x33)).collect();
        toks.push(Tok::Match(rng.range(3, 30), rng.range(1, 30)));
        let out = expand(&toks);
        let mut bw = BitW::new();
        block_fixed(&mut bw, true, &toks, &codes);
        let d = bw.finish();

        let table = match i % 4 {
            0 => Table::LenBase,
            1 => Table::LenExtraBits,
            2 => Table::DistBase,
            _ => Table::DistExtraBits,
        };
        let mut off = rng.below(table.byte_len() as u32) as usize;
        // `cp_len_base`/`cp_dist_base` are uint32; a high byte makes the C's
        // `int length` negative, and `while (length--)` then writes gigabytes
        // before faulting. Only retune the low byte of those entries.
        if matches!(table, Table::LenBase | Table::DistBase) {
            off &= !3;
        }
        // keep extra-bit counts <= 32 so the `assert(num_bits <= 32)` in
        // cp_read_bits is not the thing being tested here
        let val = match table {
            Table::LenExtraBits | Table::DistExtraBits => rng.below(6) as u8,
            _ => rng.byte(),
        };
        cases.push(
            inflate(
                format!("mutate {table:?}[{off}]={val} {i}"),
                d,
                rng.below(4) as usize,
                out.len() as i32 + 64,
            )
            .with_mutations(vec![Mutation { table, off, val }]),
        );
    }
    assert_same(&pair, &cases);
}

/// Row 19: retuning `cp_fixed_table`. The generator is pointed at the mutated
/// table so the stream is still decodable by the mutated library.
#[test]
fn row_19_retuned_fixed_table() {
    let pair = load_pair();
    let mut rng = Rng::new(8);
    let mut cases = Vec::new();

    for i in 0..120 {
        // swap the code lengths of two literal symbols -- keeps the code complete
        let mut table = [0u8; 320];
        table[..288].copy_from_slice(&fixed_lit_lens());
        table[288..].copy_from_slice(&fixed_dist_lens());
        let a = rng.below(288) as usize;
        let mut b = rng.below(288) as usize;
        while table[b] == table[a] {
            b = rng.below(288) as usize;
        }
        table.swap(a, b);

        let codes = Codes::from_fixed_table(&table);
        let n = rng.range(1, 80) as usize;
        let data: Vec<u8> = (0..n).map(|_| rng.byte()).collect();
        let toks: Vec<Tok> = data.iter().map(|x| Tok::Lit(*x)).collect();
        let mut bw = BitW::new();
        block_fixed(&mut bw, true, &toks, &codes);
        cases.push(
            inflate(
                format!("fixed table swap {a}<->{b} {i}"),
                bw.finish(),
                rng.below(4) as usize,
                n as i32,
            )
            .with_mutations(vec![
                Mutation {
                    table: Table::FixedTable,
                    off: a,
                    val: table[a],
                },
                Mutation {
                    table: Table::FixedTable,
                    off: b,
                    val: table[b],
                },
            ]),
        );
    }
    assert_same(&pair, &cases);
}

/// Row 20: retuning `cp_permutation_order`. The generator writes the code-length
/// lengths in the permuted order, so the stream stays decodable.
#[test]
fn row_20_retuned_permutation_order() {
    let pair = load_pair();
    let mut rng = Rng::new(9);
    let mut cases = Vec::new();

    for i in 0..120 {
        let mut perm = PERMUTATION_ORDER;
        // swap two entries -- still a permutation of 0..18
        let a = rng.below(19) as usize;
        let b = rng.below(19) as usize;
        perm.swap(a, b);

        let n = rng.range(1, 120) as usize;
        let data = rng.bytes(n);
        let toks: Vec<Tok> = data.iter().map(|x| Tok::Lit(*x)).collect();
        let (lit, dst) = random_codes_for(&mut rng, &toks);
        let mut bw = BitW::new();
        block_dynamic(
            &mut bw,
            true,
            &toks,
            &lit,
            &dst,
            &perm,
            if rng.bool() {
                ClEncoding::Literal
            } else {
                ClEncoding::RunLength
            },
            &mut rng,
        );
        let muts = vec![
            Mutation {
                table: Table::PermutationOrder,
                off: a,
                val: perm[a],
            },
            Mutation {
                table: Table::PermutationOrder,
                off: b,
                val: perm[b],
            },
        ];
        cases.push(
            inflate(
                format!("perm swap {a}<->{b} {i}"),
                bw.finish(),
                rng.below(4) as usize,
                n as i32,
            )
            .with_mutations(muts),
        );
    }
    assert_same(&pair, &cases);
}
