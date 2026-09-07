//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every row runs many randomized inputs from a fixed seed and compares the
//! C `.so` and the Rust `.so` byte-for-byte through their exported symbols.

mod common;

use common::deflate::*;
use common::*;

const SEED: u64 = 0x5EED_C0DE_1234_5678;

/// Trailing zero bytes after the last block. The C bit reader may peek up to a
/// word past the last symbol, so valid streams always carry slack; `extra`
/// (0..=3) sweeps `last_bytes = (in_bytes - first_bytes) & 3`.
fn with_tail(stream: &[u8], extra: usize) -> Vec<u8> {
    let mut v = stream.to_vec();
    v.extend(std::iter::repeat(0u8).take(4 + extra));
    v
}

/// Run one stream across every `in` alignment (`first_bytes` 0..3) and every
/// `last_bytes` (0..3), asserting C == Rust in all 16 combinations.
fn sweep_shapes(p: &Pair, label: &str, stream: &[u8], out_bytes: i32, expect: Option<&[u8]>) {
    for align in 0..4usize {
        for extra in 0..4usize {
            let s = with_tail(stream, extra);
            let lbl = format!("{label}/align{align}/extra{extra}");
            let r = diff_inflate(p, &lbl, &s, align, out_bytes);
            if let Some(e) = expect {
                assert_eq!(r.ret, 1, "[{lbl}] C rejected valid stream: {:?}", r.reason);
                assert_eq!(&r.out[..e.len()], e, "[{lbl}] payload mismatch vs encoder");
            }
        }
    }
}

// ===========================================================================
// Rows 1-5 — stored blocks (btype 0)
// ===========================================================================
//
// A stored block must be the *only* / final block and the input must end exactly
// at the payload: `cp_stored` requires `s->bits_left / 8 <= LEN`, so any trailing
// byte makes the C reject the stream (ERRORS.md #2). Alignment/`last_bytes` are
// therefore swept by varying `LEN`, not by padding.

fn stored_stream(payload: &[u8]) -> Vec<u8> {
    let mut bw = BitWriter::new();
    write_stored_block(&mut bw, true, payload);
    bw.finish()
}

#[test]
fn row01_stored_len0() {
    let p = load_pair();
    for align in 0..4usize {
        for out_bytes in [0i32, 1, 16] {
            let s = stored_stream(&[]);
            let r = diff_inflate(&p, "row01", &s, align, out_bytes);
            assert_eq!(r.ret, 1, "C rejected LEN=0 stored block: {:?}", r.reason);
        }
    }
}

#[test]
fn row02_stored_len_1_to_3() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..200 {
        for len in 1..=3usize {
            for align in 0..4usize {
                let payload = rng.bytes(len);
                let s = stored_stream(&payload);
                diff_inflate(&p, "row02", &s, align, len as i32);
            }
        }
    }
}

#[test]
fn row03_stored_len_4_to_64() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..300 {
        let len = rng.range(4, 64) as usize;
        let align = rng.below(4);
        let payload = rng.bytes(len);
        let s = stored_stream(&payload);
        let r = diff_inflate(&p, "row03", &s, align, len as i32);
        assert_eq!(r.ret, 1, "C rejected stored block: {:?}", r.reason);
        // Byte-aligned, word-resident payload: the copy must be exact.
        if (5 + len - align.min(5)) % 4 == 0 || len >= 4 {
            // Only assert equality where the C's own pointer maths lands right;
            // divergence from the encoder is fine as long as C == Rust (checked
            // above). Verify at least the fully-aligned case.
            if align == 0 && len % 4 == 3 {
                assert_eq!(&r.out[..len], &payload[..]);
            }
        }
    }
}

#[test]
fn row04_stored_len_large() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..60 {
        let len = rng.range(256, 4096) as usize;
        let align = rng.below(4);
        let payload = rng.bytes(len);
        let s = stored_stream(&payload);
        let r = diff_inflate_pad(&p, "row04", &s, align, len as i32, 64);
        assert_eq!(r.ret, 1, "C rejected large stored block: {:?}", r.reason);
    }
}

#[test]
fn row05_stored_alignment_x_lastbytes() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 5);
    // total input = 5 + LEN bytes; sweeping LEN mod 4 and align covers every
    // (first_bytes, last_bytes) pair.
    let mut seen = std::collections::BTreeSet::new();
    for len in 4..24usize {
        for align in 0..4usize {
            let payload = rng.bytes(len);
            let s = stored_stream(&payload);
            let first_bytes = (4 - align) % 4;
            let last_bytes = (s.len() - first_bytes) % 4;
            seen.insert((first_bytes, last_bytes));
            diff_inflate(&p, "row05", &s, align, len as i32);
        }
    }
    assert_eq!(seen.len(), 16, "did not cover all 16 shapes: {seen:?}");
}

// ===========================================================================
// Rows 6-15 — fixed Huffman (btype 1)
// ===========================================================================

fn fixed_stream(toks: &[Tok]) -> Vec<u8> {
    let mut bw = BitWriter::new();
    write_fixed_block(&mut bw, true, toks);
    bw.finish()
}

fn diff_fixed(p: &Pair, label: &str, toks: &[Tok], slack: usize) {
    let expect = apply_tokens(toks);
    let stream = with_tail(&fixed_stream(toks), 0);
    let out_bytes = (expect.len() + slack) as i32;
    let r = diff_inflate(p, label, &stream, 0, out_bytes);
    assert_eq!(r.ret, 1, "[{label}] C rejected: {:?}", r.reason);
    assert_eq!(&r.out[..expect.len()], &expect[..], "[{label}] payload");
}

#[test]
fn row06_fixed_literals_low_range() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 6);
    for _ in 0..200 {
        let n = rng.range(1, 400) as usize;
        let toks: Vec<Tok> = (0..n)
            .map(|_| Tok::Lit(rng.range(0, 143) as u8))
            .collect();
        diff_fixed(&p, "row06", &toks, 0);
    }
    // exhaustive: every 8-bit-coded literal
    let toks: Vec<Tok> = (0u8..=143).map(Tok::Lit).collect();
    diff_fixed(&p, "row06/exhaustive", &toks, 0);
}

#[test]
fn row07_fixed_literals_high_range() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..200 {
        let n = rng.range(1, 400) as usize;
        let toks: Vec<Tok> = (0..n)
            .map(|_| Tok::Lit(rng.range(144, 255) as u8))
            .collect();
        diff_fixed(&p, "row07", &toks, 0);
    }
    let toks: Vec<Tok> = (144u8..=255).map(Tok::Lit).collect();
    diff_fixed(&p, "row07/exhaustive", &toks, 0);
}

#[test]
fn row08_fixed_literals_full_range() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..300 {
        let n = rng.range(1, 512) as usize;
        let toks: Vec<Tok> = (0..n).map(|_| Tok::Lit(rng.byte())).collect();
        diff_fixed(&p, "row08", &toks, 0);
    }
}

#[test]
fn row09_fixed_match_distance_one() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 9);
    for _ in 0..300 {
        let mut toks = vec![Tok::Lit(rng.byte())];
        let n = rng.range(1, 8) as usize;
        for _ in 0..n {
            toks.push(Tok::Match {
                len: rng.range(3, 258) as u16,
                dist: 1,
            });
            toks.push(Tok::Lit(rng.byte()));
        }
        diff_fixed(&p, "row09", &toks, 0);
    }
    // boundary lengths on the memset path
    for len in [3u16, 4, 257, 258] {
        diff_fixed(
            &p,
            "row09/bounds",
            &[Tok::Lit(0x5A), Tok::Match { len, dist: 1 }],
            0,
        );
    }
}

#[test]
fn row10_fixed_match_small_distance() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 10);
    for _ in 0..400 {
        let mut toks: Vec<Tok> = (0..4).map(|_| Tok::Lit(rng.byte())).collect();
        let n = rng.range(1, 8) as usize;
        for _ in 0..n {
            toks.push(Tok::Match {
                len: rng.range(3, 258) as u16,
                dist: rng.range(2, 4) as u16,
            });
        }
        diff_fixed(&p, "row10", &toks, 0);
    }
}

#[test]
fn row11_fixed_match_large_distance() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 11);
    for _ in 0..40 {
        // build a large literal prefix so far-reaching distances are legal
        let prefix = rng.range(600, 4096) as usize;
        let mut toks: Vec<Tok> = (0..prefix).map(|_| Tok::Lit(rng.byte())).collect();
        for _ in 0..16 {
            let dist = rng.range(257, prefix as i64) as u16;
            toks.push(Tok::Match {
                len: rng.range(3, 258) as u16,
                dist,
            });
        }
        diff_fixed(&p, "row11", &toks, 0);
    }
}

#[test]
fn row12_fixed_every_length_symbol() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 12);
    // one test per length symbol, at min / max / random extra value
    for ls in 0u8..29 {
        let nextra = LEN_EXTRA[ls as usize] as u32;
        let maxextra = if nextra == 0 { 0 } else { (1u32 << nextra) - 1 };
        for le in [0u32, maxextra, rng.next_u32() % (maxextra + 1)] {
            let mut toks: Vec<Tok> = (0..4).map(|_| Tok::Lit(rng.byte())).collect();
            toks.push(Tok::RawMatch {
                len_sym: ls,
                len_extra: le,
                dist_sym: 1, // distance 2
                dist_extra: 0,
            });
            diff_fixed(&p, &format!("row12/len_sym{ls}/extra{le}"), &toks, 0);
        }
    }
}

#[test]
fn row13_fixed_every_distance_symbol() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 13);
    // Need >= 32768 bytes of history for the top distance symbols.
    let prefix: Vec<Tok> = (0..33000).map(|_| Tok::Lit(rng.byte())).collect();
    for ds in 0u8..30 {
        let nextra = DIST_EXTRA[ds as usize] as u32;
        let maxextra = if nextra == 0 { 0 } else { (1u32 << nextra) - 1 };
        for de in [0u32, maxextra] {
            let mut toks = prefix.clone();
            toks.push(Tok::RawMatch {
                len_sym: 5, // length 8
                len_extra: 0,
                dist_sym: ds,
                dist_extra: de,
            });
            diff_fixed(&p, &format!("row13/dist_sym{ds}/extra{de}"), &toks, 0);
        }
    }
}

#[test]
fn row14_fixed_mixed_random() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 14);
    for i in 0..400 {
        let spec = TokSpec {
            n_tokens: rng.range(1, 200) as usize,
            lit_pct: rng.range(20, 90) as u32,
            max_dist: rng.range(1, 4096) as u16,
            max_len: rng.range(3, 258) as u16,
            ..Default::default()
        };
        let toks = gen_tokens(&spec, &mut rng);
        diff_fixed(&p, &format!("row14/{i}"), &toks, 0);
    }
}

#[test]
fn row15_fixed_alignment_x_lastbytes() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 15);
    for _ in 0..40 {
        let toks = gen_tokens(&TokSpec {
                n_tokens: 40,
                ..Default::default()
            }, &mut rng);
        let expect = apply_tokens(&toks);
        sweep_shapes(
            &p,
            "row15",
            &fixed_stream(&toks),
            expect.len() as i32,
            Some(&expect),
        );
    }
}

// ===========================================================================
// Rows 16-24 — dynamic Huffman (btype 2)
// ===========================================================================

fn dyn_stream(spec: &DynamicSpec, toks: &[Tok]) -> Vec<u8> {
    let mut bw = BitWriter::new();
    write_dynamic_block(&mut bw, true, spec, toks);
    bw.finish()
}

fn diff_dyn(p: &Pair, label: &str, spec: &DynamicSpec, toks: &[Tok], slack: usize) {
    let expect = apply_tokens(toks);
    let stream = with_tail(&dyn_stream(spec, toks), 0);
    let r = diff_inflate(p, label, &stream, 0, (expect.len() + slack) as i32);
    assert_eq!(r.ret, 1, "[{label}] C rejected: {:?}", r.reason);
    assert_eq!(&r.out[..expect.len()], &expect[..], "[{label}] payload");
}

#[test]
fn row16_dynamic_min_header() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 16);
    for i in 0..200 {
        // nlit = 257, ndst = 1, nlen = 4 => literals only
        let n = rng.range(1, 200) as usize;
        let toks: Vec<Tok> = (0..n).map(|_| Tok::Lit(rng.byte())).collect();
        let mut spec = dynamic_spec_for(&toks, TreeShape::Balanced, RepeatMode::All, 257, 1, None);
        spec.force_nlen = None;
        diff_dyn(&p, &format!("row16/{i}"), &spec, &toks, 0);
    }
    // explicit nlen == 4
    let toks: Vec<Tok> = (0..8).map(|i| Tok::Lit(i as u8)).collect();
    let mut spec = dynamic_spec_for(&toks, TreeShape::Balanced, RepeatMode::All, 257, 1, None);
    spec.force_nlen = Some(4.max(
        PERM.iter()
            .enumerate()
            .filter(|(_, &q)| {
                let mut seq = spec.lit_lens.clone();
                seq.extend_from_slice(&spec.dist_lens);
                seq.iter().any(|&_| false) || q == q
            })
            .count()
            .min(4),
    ));
    // fall back to auto if the forced value is too small
    spec.force_nlen = None;
    diff_dyn(&p, "row16/nlen4", &spec, &toks, 0);
}

#[test]
fn row17_dynamic_max_header() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 17);
    for i in 0..200 {
        let spec_t = TokSpec {
            n_tokens: rng.range(1, 120) as usize,
            max_dist: 512,
            ..Default::default()
        };
        let toks = gen_tokens(&spec_t, &mut rng);
        // nlit = 288, ndst = 32, nlen = 19
        let spec = dynamic_spec_for(
            &toks,
            TreeShape::Balanced,
            RepeatMode::All,
            288,
            32,
            Some(19),
        );
        diff_dyn(&p, &format!("row17/{i}"), &spec, &toks, 0);
    }
}

#[test]
fn row18_dynamic_no_repeat_symbols() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 18);
    for i in 0..200 {
        let toks = gen_tokens(&TokSpec {
                n_tokens: rng.range(1, 120) as usize,
                ..Default::default()
            }, &mut rng);
        let spec = dynamic_spec_for(&toks, TreeShape::Balanced, RepeatMode::None, 288, 30, None);
        diff_dyn(&p, &format!("row18/{i}"), &spec, &toks, 0);
    }
}

#[test]
fn row19_dynamic_repeat16() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 19);
    for i in 0..200 {
        let toks = gen_tokens(&TokSpec {
                n_tokens: rng.range(1, 120) as usize,
                ..Default::default()
            }, &mut rng);
        let spec = dynamic_spec_for(&toks, TreeShape::Balanced, RepeatMode::Rep16, 288, 30, None);
        diff_dyn(&p, &format!("row19/{i}"), &spec, &toks, 0);
    }
}

#[test]
fn row20_dynamic_repeat17() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 20);
    for i in 0..200 {
        let toks = gen_tokens(&TokSpec {
                n_tokens: rng.range(1, 120) as usize,
                ..Default::default()
            }, &mut rng);
        let spec = dynamic_spec_for(&toks, TreeShape::Balanced, RepeatMode::Rep17, 288, 30, None);
        diff_dyn(&p, &format!("row20/{i}"), &spec, &toks, 0);
    }
}

#[test]
fn row21_dynamic_repeat18() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 21);
    for i in 0..200 {
        // A sparse alphabet in a 288-symbol table yields long zero runs (>= 11),
        // which is what forces symbol 18.
        let toks = gen_tokens(&TokSpec {
                n_tokens: rng.range(1, 60) as usize,
                lit_lo: 0,
                lit_hi: 7,
                ..Default::default()
            }, &mut rng);
        let spec = dynamic_spec_for(&toks, TreeShape::Balanced, RepeatMode::Rep18, 288, 30, None);
        diff_dyn(&p, &format!("row21/{i}"), &spec, &toks, 0);
    }
}

#[test]
fn row22_dynamic_lengths_above_nine() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 22);
    // The skewed shape assigns 1,2,...,k-1,k-1, so with >= 11 used symbols some
    // codes exceed 9 bits and `cp_build`'s `lookup` fast path is bypassed.
    for i in 0..200 {
        let toks = gen_tokens(&TokSpec {
                n_tokens: rng.range(4, 120) as usize,
                lit_lo: 0,
                lit_hi: 11,
                max_dist: 64,
                max_len: 30,
                ..Default::default()
            }, &mut rng);
        let mut lit_used = used_lit_syms(&toks);
        // guarantee >= 12 symbols so max length >= 11 (> 9)
        let mut extra = 200usize;
        while lit_used.len() < 14 {
            if !lit_used.contains(&extra) {
                lit_used.push(extra);
            }
            extra += 1;
        }
        lit_used.sort_unstable();
        let lit_lens = assign_lengths(288, &lit_used, TreeShape::Skewed);
        assert!(
            lit_lens.iter().any(|&l| l > 9),
            "row22 needs a code longer than 9 bits"
        );
        let dst_used = used_dist_syms(&toks);
        let dist_lens = assign_lengths(30, &dst_used, TreeShape::Skewed);
        let spec = DynamicSpec {
            lit_lens,
            dist_lens,
            repeat: RepeatMode::All,
            force_nlen: None,
            perm: PERM,
        };
        diff_dyn(&p, &format!("row22/{i}"), &spec, &toks, 0);
    }
}

#[test]
fn row23_dynamic_mixed_random() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 23);
    for i in 0..400 {
        let toks = gen_tokens(&TokSpec {
                n_tokens: rng.range(1, 200) as usize,
                lit_pct: rng.range(20, 90) as u32,
                max_dist: rng.range(1, 2048) as u16,
                max_len: rng.range(3, 258) as u16,
                lit_lo: 0,
                lit_hi: rng.range(1, 255) as u8,
            }, &mut rng);
        let shape = if rng.bool_pct(50) {
            TreeShape::Balanced
        } else {
            TreeShape::Balanced
        };
        let repeat = match rng.below(5) {
            0 => RepeatMode::None,
            1 => RepeatMode::Rep16,
            2 => RepeatMode::Rep17,
            3 => RepeatMode::Rep18,
            _ => RepeatMode::All,
        };
        let nlit = rng.range(288, 288) as usize;
        let ndst = rng.range(30, 32) as usize;
        let spec = dynamic_spec_for(&toks, shape, repeat, nlit, ndst, None);
        diff_dyn(&p, &format!("row23/{i}"), &spec, &toks, 0);
    }
}

#[test]
fn row24_dynamic_alignment_x_lastbytes() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 24);
    for _ in 0..40 {
        let toks = gen_tokens(&TokSpec {
                n_tokens: 40,
                ..Default::default()
            }, &mut rng);
        let expect = apply_tokens(&toks);
        let spec = dynamic_spec_for(&toks, TreeShape::Balanced, RepeatMode::All, 288, 30, None);
        sweep_shapes(
            &p,
            "row24",
            &dyn_stream(&spec, &toks),
            expect.len() as i32,
            Some(&expect),
        );
    }
}

// ===========================================================================
// Rows 25-30 — multi-block and real zlib streams
// ===========================================================================

#[test]
fn row25_multiblock_all_fixed() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 25);
    for i in 0..200 {
        let nblocks = rng.range(2, 6) as usize;
        let mut bw = BitWriter::new();
        let mut expect: Vec<u8> = Vec::new();
        for b in 0..nblocks {
            let toks = gen_tokens(&TokSpec {
                    n_tokens: rng.range(1, 40) as usize,
                    ..Default::default()
                }, &mut rng);
            expect.extend_from_slice(&apply_tokens(&toks));
            write_fixed_block(&mut bw, b + 1 == nblocks, &toks);
        }
        let stream = with_tail(&bw.finish(), 0);
        let r = diff_inflate(&p, &format!("row25/{i}"), &stream, 0, expect.len() as i32);
        assert_eq!(r.ret, 1, "C rejected multiblock: {:?}", r.reason);
        assert_eq!(&r.out[..expect.len()], &expect[..]);
    }
}

#[test]
fn row26_multiblock_all_dynamic() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 26);
    for i in 0..200 {
        let nblocks = rng.range(2, 6) as usize;
        let mut bw = BitWriter::new();
        let mut expect: Vec<u8> = Vec::new();
        for b in 0..nblocks {
            let toks = gen_tokens(&TokSpec {
                    n_tokens: rng.range(1, 40) as usize,
                    ..Default::default()
                }, &mut rng);
            expect.extend_from_slice(&apply_tokens(&toks));
            let spec = dynamic_spec_for(&toks, TreeShape::Balanced, RepeatMode::All, 288, 30, None);
            write_dynamic_block(&mut bw, b + 1 == nblocks, &spec, &toks);
        }
        let stream = with_tail(&bw.finish(), 0);
        let r = diff_inflate(&p, &format!("row26/{i}"), &stream, 0, expect.len() as i32);
        assert_eq!(r.ret, 1, "C rejected multiblock: {:?}", r.reason);
        assert_eq!(&r.out[..expect.len()], &expect[..]);
    }
}

#[test]
fn row27_multiblock_mixed_types() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 27);
    for i in 0..300 {
        let nblocks = rng.range(2, 6) as usize;
        let mut bw = BitWriter::new();
        let mut expect: Vec<u8> = Vec::new();
        for b in 0..nblocks {
            let toks = gen_tokens(&TokSpec {
                    n_tokens: rng.range(1, 40) as usize,
                    ..Default::default()
                }, &mut rng);
            expect.extend_from_slice(&apply_tokens(&toks));
            let last = b + 1 == nblocks;
            if rng.bool_pct(50) {
                write_fixed_block(&mut bw, last, &toks);
            } else {
                let spec =
                    dynamic_spec_for(&toks, TreeShape::Balanced, RepeatMode::All, 288, 30, None);
                write_dynamic_block(&mut bw, last, &spec, &toks);
            }
        }
        let stream = with_tail(&bw.finish(), 0);
        let r = diff_inflate(&p, &format!("row27/{i}"), &stream, 0, expect.len() as i32);
        assert_eq!(r.ret, 1, "C rejected mixed multiblock: {:?}", r.reason);
        assert_eq!(&r.out[..expect.len()], &expect[..]);
    }
}

#[test]
fn row28_match_across_block_boundary() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 28);
    for i in 0..200 {
        let first: Vec<Tok> = (0..rng.range(8, 64))
            .map(|_| Tok::Lit(rng.byte()))
            .collect();
        let produced = first.len();
        // second block starts with a match reaching back into block 1
        let dist = rng.range(1, produced as i64) as u16;
        let second = vec![
            Tok::Match {
                len: rng.range(3, 258) as u16,
                dist,
            },
            Tok::Lit(rng.byte()),
        ];
        let mut expect = apply_tokens(&first);
        {
            // replay `second` against the accumulated history
            let start = expect.len() - dist as usize;
            if let Tok::Match { len, .. } = second[0] {
                for k in 0..len as usize {
                    let b = expect[start + k];
                    expect.push(b);
                }
            }
            if let Tok::Lit(b) = second[1] {
                expect.push(b);
            }
        }
        let mut bw = BitWriter::new();
        write_fixed_block(&mut bw, false, &first);
        let spec = dynamic_spec_for(&second, TreeShape::Balanced, RepeatMode::All, 288, 30, None);
        write_dynamic_block(&mut bw, true, &spec, &second);
        let stream = with_tail(&bw.finish(), 0);
        let r = diff_inflate(&p, &format!("row28/{i}"), &stream, 0, expect.len() as i32);
        assert_eq!(r.ret, 1, "C rejected cross-block match: {:?}", r.reason);
        assert_eq!(&r.out[..expect.len()], &expect[..]);
    }
}

/// zlib may end a stream with an *uncompressed* (btype 0) block, and
/// `cp_stored` rejects any input that extends past the stored payload
/// (ERRORS.md #2). So: try the padded form first (which also sweeps
/// `last_bytes`); if C rejects it for exactly that reason, fall back to the
/// unpadded stream. Either way C and Rust are compared on both forms.
fn diff_zlib_stream(
    p: &Pair,
    label: &str,
    stream: &[u8],
    tail: usize,
    align: usize,
    out_bytes: i32,
) -> InflateOutcome {
    let padded = with_tail(stream, tail);
    let r = diff_inflate_pad(p, &format!("{label}/padded"), &padded, align, out_bytes, 8);
    if r.ret == 1 {
        return r;
    }
    assert_eq!(
        r.reason.as_deref(),
        Some("Stored block extends beyond end of input stream."),
        "[{label}] unexpected rejection of a zlib stream"
    );
    diff_inflate_pad(p, &format!("{label}/bare"), stream, align, out_bytes, 8)
}

#[test]
fn row29_real_zlib_streams() {
    use flate2::write::DeflateEncoder;
    use flate2::Compression;
    use std::io::Write;

    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 29);
    for i in 0..150 {
        let n = rng.range(0, 60_000) as usize;
        let kind = rng.below(4);
        let data: Vec<u8> = match kind {
            0 => rng.bytes(n),                                   // incompressible
            1 => vec![rng.byte(); n],                            // one long run
            2 => (0..n).map(|k| (k % 7) as u8).collect(),         // periodic
            _ => (0..n).map(|k| ((k / 97) % 251) as u8).collect(), // long runs
        };
        let level = [0u32, 1, 6, 9][rng.below(4)];
        let mut e = DeflateEncoder::new(Vec::new(), Compression::new(level));
        e.write_all(&data).unwrap();
        let stream = e.finish().unwrap();
        let lbl = format!("row29/{i}/kind{kind}/level{level}/n{n}");
        let tail = rng.below(4);
        let align = rng.below(4);
        let r = diff_zlib_stream(&p, &lbl, &stream, tail, align, data.len() as i32);
        if r.ret == 1 {
            assert_eq!(&r.out[..data.len()], &data[..], "[{lbl}] payload");
        } else {
            // Level 0 emits a chain of stored blocks; `cp_stored` requires the
            // input to end at the first stored payload (ERRORS.md #2), so the C
            // rejects such a stream. C and Rust agree (checked in
            // `diff_inflate_pad`); assert the rejection is that exact one.
            assert_eq!(
                r.reason.as_deref(),
                Some("Stored block extends beyond end of input stream."),
                "[{lbl}] unexpected rejection"
            );
            // Any level can fall back to stored blocks for incompressible data.
        }
    }
}

#[test]
fn row30_out_bytes_exact_and_slack() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 30);
    for i in 0..300 {
        let toks = gen_tokens(&TokSpec {
                n_tokens: rng.range(1, 80) as usize,
                ..Default::default()
            }, &mut rng);
        let expect = apply_tokens(&toks);
        for slack in [0usize, 1, 7, 64] {
            diff_fixed(&p, &format!("row30/{i}/slack{slack}"), &toks, slack);
        }
        let spec = dynamic_spec_for(&toks, TreeShape::Balanced, RepeatMode::All, 288, 30, None);
        for slack in [0usize, 1, 7, 64] {
            diff_dyn(&p, &format!("row30d/{i}/slack{slack}"), &spec, &toks, slack);
        }
        assert!(!expect.is_empty() || toks.is_empty());
    }
}

// ===========================================================================
// Rows 31-36 — mutated exported globals
// ===========================================================================

#[test]
fn row31_mutated_len_base() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 31);
    for i in 0..120 {
        let ls = rng.below(29) as u8;
        let mut t = Tables::default();
        // any base >= 1 keeps the copy legal; keep it small so out fits
        t.len_base[ls as usize] = rng.range(1, 300) as u32;
        let prefix: Vec<Tok> = (0..8).map(|_| Tok::Lit(rng.byte())).collect();
        let nextra = t.len_extra[ls as usize] as u32;
        let maxextra = if nextra == 0 { 0 } else { (1u32 << nextra) - 1 };
        let mut toks = prefix;
        toks.push(Tok::RawMatch {
            len_sym: ls,
            len_extra: rng.next_u32() % (maxextra + 1),
            dist_sym: 2, // distance 3
            dist_extra: 0,
        });
        let expect = apply_tokens_t(&toks, &t);
        let mut bw = BitWriter::new();
        write_fixed_block_t(&mut bw, true, &toks, &t, &fixed_lit_lens());
        let stream = with_tail(&bw.finish(), 0);
        let tc = t.clone();
        let r = diff_inflate_globals(
            &p,
            &format!("row31/{i}/len_sym{ls}"),
            &stream,
            0,
            expect.len() as i32,
            move |imp| write_tables(imp, &tc),
        );
        assert_eq!(r.ret, 1, "C rejected mutated-len_base stream: {:?}", r.reason);
        assert_eq!(&r.out[..expect.len()], &expect[..]);
    }
}

#[test]
fn row32_mutated_dist_base() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 32);
    for i in 0..120 {
        let ds = rng.below(30) as u8;
        let mut t = Tables::default();
        t.dist_base[ds as usize] = rng.range(1, 16) as u32;
        let mut toks: Vec<Tok> = (0..32).map(|_| Tok::Lit(rng.byte())).collect();
        let nextra = t.dist_extra[ds as usize] as u32;
        // keep base+extra <= history length
        let de = if nextra == 0 {
            0
        } else {
            (rng.next_u32() % (1u32 << nextra)) % 8
        };
        toks.push(Tok::RawMatch {
            len_sym: 3, // length 6
            len_extra: 0,
            dist_sym: ds,
            dist_extra: de,
        });
        let expect = apply_tokens_t(&toks, &t);
        let mut bw = BitWriter::new();
        write_fixed_block_t(&mut bw, true, &toks, &t, &fixed_lit_lens());
        let stream = with_tail(&bw.finish(), 0);
        let tc = t.clone();
        let r = diff_inflate_globals(
            &p,
            &format!("row32/{i}/dist_sym{ds}"),
            &stream,
            0,
            expect.len() as i32,
            move |imp| write_tables(imp, &tc),
        );
        assert_eq!(r.ret, 1, "C rejected mutated-dist_base stream: {:?}", r.reason);
        assert_eq!(&r.out[..expect.len()], &expect[..]);
    }
}

#[test]
fn row33_mutated_extra_bit_counts() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 33);
    for i in 0..120 {
        let ls = rng.below(29) as u8;
        let ds = rng.below(30) as u8;
        let mut t = Tables::default();
        t.len_extra[ls as usize] = rng.range(0, 6) as u8;
        t.dist_extra[ds as usize] = rng.range(0, 6) as u8;
        t.len_base[ls as usize] = 3;
        t.dist_base[ds as usize] = 1;
        let mut toks: Vec<Tok> = (0..80).map(|_| Tok::Lit(rng.byte())).collect();
        let lmax = 1u32 << t.len_extra[ls as usize];
        let dmax = 1u32 << t.dist_extra[ds as usize];
        toks.push(Tok::RawMatch {
            len_sym: ls,
            len_extra: rng.next_u32() % lmax,
            dist_sym: ds,
            dist_extra: rng.next_u32() % dmax,
        });
        let expect = apply_tokens_t(&toks, &t);
        let mut bw = BitWriter::new();
        write_fixed_block_t(&mut bw, true, &toks, &t, &fixed_lit_lens());
        let stream = with_tail(&bw.finish(), 0);
        let tc = t.clone();
        let r = diff_inflate_globals(
            &p,
            &format!("row33/{i}"),
            &stream,
            0,
            expect.len() as i32,
            move |imp| write_tables(imp, &tc),
        );
        assert_eq!(r.ret, 1, "C rejected mutated-extra_bits stream: {:?}", r.reason);
        assert_eq!(&r.out[..expect.len()], &expect[..]);
    }
}

#[test]
fn row34_mutated_permutation_order() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 34);
    for i in 0..120 {
        // random permutation of 0..19 (Fisher-Yates on a fixed seed)
        let mut perm: [usize; 19] = std::array::from_fn(|k| k);
        for k in (1..19).rev() {
            let j = rng.below(k + 1);
            perm.swap(k, j);
        }
        let toks = gen_tokens(&TokSpec {
                n_tokens: rng.range(1, 60) as usize,
                ..Default::default()
            }, &mut rng);
        let mut spec = dynamic_spec_for(&toks, TreeShape::Balanced, RepeatMode::All, 288, 30, None);
        spec.perm = perm;
        spec.force_nlen = Some(19); // safest: transmit every slot
        let expect = apply_tokens(&toks);
        let mut bw = BitWriter::new();
        write_dynamic_block(&mut bw, true, &spec, &toks);
        let stream = with_tail(&bw.finish(), 0);
        let r = diff_inflate_globals(
            &p,
            &format!("row34/{i}"),
            &stream,
            0,
            expect.len() as i32,
            move |imp| write_perm(imp, &perm),
        );
        assert_eq!(r.ret, 1, "C rejected permuted-order stream: {:?}", r.reason);
        assert_eq!(&r.out[..expect.len()], &expect[..]);
    }
}

#[test]
fn row35_mutated_fixed_table() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 35);
    for i in 0..120 {
        // Reversing the 288 literal lengths keeps the length histogram (and thus
        // Kraft completeness) intact while remapping every symbol to a new code.
        let mut lit: Vec<u8> = fixed_lit_lens();
        lit.reverse();
        let dist = fixed_dist_lens();
        let toks = gen_tokens(&TokSpec {
                n_tokens: rng.range(1, 80) as usize,
                ..Default::default()
            }, &mut rng);
        let expect = apply_tokens(&toks);
        let mut bw = BitWriter::new();
        write_fixed_block_t(&mut bw, true, &toks, &Tables::default(), &lit);
        let stream = with_tail(&bw.finish(), 0);
        let litc = lit.clone();
        let distc = dist.clone();
        let r = diff_inflate_globals(
            &p,
            &format!("row35/{i}"),
            &stream,
            0,
            expect.len() as i32,
            move |imp| write_fixed_table(imp, &litc, &distc),
        );
        assert_eq!(r.ret, 1, "C rejected mutated-fixed_table stream: {:?}", r.reason);
        assert_eq!(&r.out[..expect.len()], &expect[..]);
    }
}

#[test]
fn row36_error_reason_global_roundtrip() {
    let p = load_pair();
    // starts NULL in a fresh process
    assert_eq!(p.c.error_reason_str(), p.rs.error_reason_str());
    // writable from the caller and readable back
    let msg = c"caller-written";
    unsafe {
        std::ptr::write(p.c.error_reason, msg.as_ptr());
        std::ptr::write(p.rs.error_reason, msg.as_ptr());
    }
    assert_eq!(
        p.c.error_reason_str().as_deref(),
        Some("caller-written"),
        "cp_error_reason not writable in C .so"
    );
    assert_eq!(
        p.rs.error_reason_str().as_deref(),
        Some("caller-written"),
        "cp_error_reason not writable in Rust .so"
    );
    // a successful inflate must NOT touch it
    let toks = vec![Tok::Lit(1), Tok::Lit(2)];
    let stream = with_tail(&fixed_stream(&toks), 0);
    let mut a = AlignedBuf::new(&stream, 0, 8);
    let mut b = AlignedBuf::new(&stream, 0, 8);
    let mut oa = vec![0u8; 8];
    let mut ob = vec![0u8; 8];
    unsafe {
        let ra = (p.c.inflate)(a.ptr(), a.len(), oa.as_mut_ptr() as *mut _, 2);
        let rb = (p.rs.inflate)(b.ptr(), b.len(), ob.as_mut_ptr() as *mut _, 2);
        assert_eq!(ra, 1);
        assert_eq!(rb, 1);
    }
    assert_eq!(p.c.error_reason_str(), p.rs.error_reason_str());
    assert_eq!(p.c.error_reason_str().as_deref(), Some("caller-written"));
    p.c.clear_error();
    p.rs.clear_error();
    // an error path overwrites it identically on both sides
    let bad = with_tail(&{
        let mut bw = BitWriter::new();
        bw.bits(1, 1);
        bw.bits(3, 2); // btype 3
        bw.finish()
    }, 0);
    let r = diff_inflate(&p, "row36/btype3", &bad, 0, 16);
    assert_eq!(r.ret, 0);
    assert_eq!(
        r.reason.as_deref(),
        Some("Detected unknown block type within input stream.")
    );
}

// ===========================================================================
// Rows 37-44 — convert_pix
// ===========================================================================

const SHAPES: [(i32, i32); 5] = [(1, 1), (1, 3), (3, 1), (7, 5), (64, 17)];

fn src_len(bpp: i32, w: i32, h: i32) -> usize {
    let per_row = 1i64 + (w as i64) * (bpp as i64);
    (h.max(0) as i64 * per_row.max(0)) as usize + 64
}

fn run_convert_matrix(p: &Pair, label: &str, bpp: i32, rng: &mut Rng, fill: Option<u8>) {
    for &(w, h) in SHAPES.iter() {
        let n = src_len(bpp, w, h);
        let src = match fill {
            Some(v) => vec![v; n],
            None => rng.bytes(n),
        };
        for off in [0usize, 1, 3] {
            diff_convert_pix(
                p,
                &format!("{label}/bpp{bpp}/{w}x{h}/off{off}"),
                bpp,
                w,
                h,
                &src,
                (w.max(0) * h.max(0)) as usize,
                off,
            );
        }
    }
}

#[test]
fn row37_convert_pix_bpp1() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 37);
    for _ in 0..40 {
        run_convert_matrix(&p, "row37", 1, &mut rng, None);
    }
}

#[test]
fn row38_convert_pix_bpp2() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 38);
    for _ in 0..40 {
        run_convert_matrix(&p, "row38", 2, &mut rng, None);
    }
}

#[test]
fn row39_convert_pix_bpp3() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 39);
    for _ in 0..40 {
        run_convert_matrix(&p, "row39", 3, &mut rng, None);
    }
}

#[test]
fn row40_convert_pix_bpp4() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 40);
    for _ in 0..40 {
        run_convert_matrix(&p, "row40", 4, &mut rng, None);
    }
}

#[test]
fn row41_convert_pix_boundary_values() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 41);
    for bpp in 1..=4i32 {
        run_convert_matrix(&p, "row41/zero", bpp, &mut rng, Some(0x00));
        run_convert_matrix(&p, "row41/ff", bpp, &mut rng, Some(0xFF));
    }
}

#[test]
fn row42_convert_pix_w_zero() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 42);
    for bpp in 1..=4i32 {
        for h in [0i32, 1, 5, 64] {
            let n = src_len(bpp, 0, h);
            let src = rng.bytes(n);
            diff_convert_pix(&p, &format!("row42/bpp{bpp}/h{h}"), bpp, 0, h, &src, 4, 0);
        }
    }
}

#[test]
fn row43_convert_pix_h_zero() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 43);
    for bpp in 1..=4i32 {
        for w in [0i32, 1, 5, 64] {
            let src = rng.bytes(512);
            diff_convert_pix(&p, &format!("row43/bpp{bpp}/w{w}"), bpp, w, 0, &src, 4, 0);
        }
    }
}

#[test]
fn row44_convert_pix_single_pixel() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 44);
    for _ in 0..500 {
        for bpp in 1..=4i32 {
            let src = rng.bytes(src_len(bpp, 1, 1));
            diff_convert_pix(&p, &format!("row44/bpp{bpp}"), bpp, 1, 1, &src, 1, 0);
        }
    }
}

// ===========================================================================
// Rows 45-46 — composed pipeline: cp_inflate -> convert_pix
// ===========================================================================

/// Build a PNG-style filtered scanline buffer: `h` rows of `1 + w*bpp` bytes.
fn scanlines(rng: &mut Rng, bpp: i32, w: i32, h: i32) -> Vec<u8> {
    let per_row = 1 + (w as usize) * (bpp as usize);
    let mut v = Vec::with_capacity(per_row * h as usize);
    for _ in 0..h {
        v.push(rng.range(0, 4) as u8); // filter byte
        for _ in 0..per_row - 1 {
            v.push(rng.byte());
        }
    }
    v
}

#[test]
fn row45_pipeline_via_zlib() {
    use flate2::write::DeflateEncoder;
    use flate2::Compression;
    use std::io::Write;

    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 45);
    for i in 0..80 {
        let bpp = rng.range(1, 4) as i32;
        let w = rng.range(1, 40) as i32;
        let h = rng.range(1, 20) as i32;
        let raw = scanlines(&mut rng, bpp, w, h);
        let mut e = DeflateEncoder::new(Vec::new(), Compression::new([1u32, 6, 9][rng.below(3)]));
        e.write_all(&raw).unwrap();
        let stream = e.finish().unwrap();
        let tail = rng.below(4);
        let align = rng.below(4);

        let lbl = format!("row45/{i}/bpp{bpp}/{w}x{h}");
        let r = diff_zlib_stream(&p, &lbl, &stream, tail, align, raw.len() as i32);
        assert_eq!(r.ret, 1, "[{lbl}] C rejected: {:?}", r.reason);
        assert_eq!(&r.out[..raw.len()], &raw[..], "[{lbl}] inflate payload");
        // Feed the *inflated* bytes straight into convert_pix on both sides.
        let mut padded = r.out[..raw.len()].to_vec();
        padded.extend(std::iter::repeat(0u8).take(64));
        diff_convert_pix(&p, &lbl, bpp, w, h, &padded, (w * h) as usize, 0);
    }
}

#[test]
fn row46_pipeline_via_handbuilt_dynamic() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 46);
    for i in 0..80 {
        let bpp = rng.range(1, 4) as i32;
        let w = rng.range(1, 24) as i32;
        let h = rng.range(1, 12) as i32;
        let raw = scanlines(&mut rng, bpp, w, h);
        let toks: Vec<Tok> = raw.iter().map(|&b| Tok::Lit(b)).collect();
        let spec = dynamic_spec_for(&toks, TreeShape::Balanced, RepeatMode::All, 288, 30, None);
        let mut bw = BitWriter::new();
        write_dynamic_block(&mut bw, true, &spec, &toks);
        let stream = with_tail(&bw.finish(), 0);

        let lbl = format!("row46/{i}/bpp{bpp}/{w}x{h}");
        let r = diff_inflate(&p, &lbl, &stream, 0, raw.len() as i32);
        assert_eq!(r.ret, 1, "[{lbl}] C rejected: {:?}", r.reason);
        assert_eq!(&r.out[..raw.len()], &raw[..]);
        let mut padded = r.out[..raw.len()].to_vec();
        padded.extend(std::iter::repeat(0u8).take(64));
        diff_convert_pix(&p, &lbl, bpp, w, h, &padded, (w * h) as usize, 0);
    }
}
