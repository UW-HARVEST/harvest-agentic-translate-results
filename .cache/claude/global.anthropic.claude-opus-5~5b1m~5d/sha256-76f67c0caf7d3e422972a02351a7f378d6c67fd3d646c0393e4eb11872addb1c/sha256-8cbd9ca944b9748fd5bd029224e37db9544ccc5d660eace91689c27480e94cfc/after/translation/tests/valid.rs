//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every row uses many randomized inputs from
//! a fixed-seed RNG. Both libraries are driven only through `dlsym`.

mod common;

use common::deflate::*;
use common::*;
use std::io::Write;

const SEED: u64 = 0x5EED_1234;

fn flate2_raw(data: &[u8], level: u32) -> Vec<u8> {
    let mut e = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::new(level));
    e.write_all(data).unwrap();
    e.finish().unwrap()
}

// --------------------------------------------------------------------------
// rows 1-4: fixed Huffman, literals only, every input-pointer alignment
// --------------------------------------------------------------------------

fn fixed_literals_at_align(align: usize) {
    let rng = Rng::new(SEED ^ align as u64);
    for i in 0..200 {
        let n = rng.range(1, 64);
        let data = rng.bytes(n);
        let stream = fixed_literal_stream(&data);
        diff_inflate_expect(&format!("row{}/fixed-lit align={align} #{i}", align + 1), &stream, align, &data);
    }
}

#[test]
fn row01_fixed_literals_align0() {
    fixed_literals_at_align(0);
}
#[test]
fn row02_fixed_literals_align1() {
    fixed_literals_at_align(1);
}
#[test]
fn row03_fixed_literals_align2() {
    fixed_literals_at_align(2);
}
#[test]
fn row04_fixed_literals_align3() {
    fixed_literals_at_align(3);
}

// --------------------------------------------------------------------------
// rows 5-8: every `last_bytes` (final partial word) value, all alignments
// --------------------------------------------------------------------------

fn tail_case(want_tail: usize) {
    let rng = Rng::new(SEED ^ (0xABCD + want_tail as u64));
    for align in 0..4usize {
        for i in 0..40 {
            let n = rng.range(1, 40);
            let data = rng.bytes(n);
            let stream = fixed_literal_stream(&data);
            // grow the stream with never-read trailing bytes until
            // (in_bytes - first_bytes) & 3 == want_tail
            let first_bytes = (4 - align % 4) % 4;
            let mut len = stream.len();
            while (len.wrapping_sub(first_bytes)) & 3 != want_tail || len < stream.len() {
                len += 1;
            }
            let padded = pad_to_len(stream.clone(), len);
            assert_eq!((padded.len().wrapping_sub(first_bytes)) & 3, want_tail);
            diff_inflate_expect(
                &format!("row{}/tail={want_tail} align={align} #{i}", 5 + want_tail),
                &padded,
                align,
                &data,
            );
        }
    }
}

#[test]
fn row05_tail0() {
    tail_case(0);
}
#[test]
fn row06_tail1() {
    tail_case(1);
}
#[test]
fn row07_tail2() {
    tail_case(2);
}
#[test]
fn row08_tail3() {
    tail_case(3);
}

// --------------------------------------------------------------------------
// row 9: backwards_distance == 1 -> the `memset` arm of cp_block
// --------------------------------------------------------------------------

#[test]
fn row09_distance_one_memset_path() {
    let rng = Rng::new(SEED ^ 9);
    for i in 0..120 {
        let b = rng.byte();
        let len = rng.range(3, 258) as u32;
        let toks = vec![Tok::Lit(b), Tok::Match { len, dist: 1 }];
        let expected = expand(&toks);
        let mut w = BitWriter::new();
        write_fixed_block(&mut w, true, &toks);
        diff_inflate_expect(&format!("row9/dist1 len={len} #{i}"), &w.finish(), i % 4, &expected);
    }
}

// --------------------------------------------------------------------------
// row 10: overlapping copies with distance > 1 (byte-wise arm)
// --------------------------------------------------------------------------

#[test]
fn row10_overlapping_copies() {
    let rng = Rng::new(SEED ^ 10);
    for dist in [2u32, 3, 5, 17] {
        for i in 0..80 {
            let seed_len = rng.range(dist as usize, dist as usize + 8);
            let mut toks: Vec<Tok> = rng.bytes(seed_len).into_iter().map(Tok::Lit).collect();
            let len = rng.range(3, 258) as u32;
            toks.push(Tok::Match { len, dist });
            // a second, chained overlapping match
            let len2 = rng.range(3, 40) as u32;
            toks.push(Tok::Match { len: len2, dist });
            let expected = expand(&toks);
            let mut w = BitWriter::new();
            write_fixed_block(&mut w, true, &toks);
            diff_inflate_expect(
                &format!("row10/dist={dist} len={len}+{len2} #{i}"),
                &w.finish(),
                i % 4,
                &expected,
            );
        }
    }
}

// --------------------------------------------------------------------------
// row 11: one match per cp_len_base / cp_len_extra_bits bucket
// --------------------------------------------------------------------------

#[test]
fn row11_every_length_bucket() {
    let rng = Rng::new(SEED ^ 11);
    let mut lengths: Vec<u32> = Vec::new();
    for &b in LEN_BASE.iter() {
        lengths.push(b);
    }
    // plus one value inside each bucket (base + 1) where the bucket is wider
    for i in 0..28 {
        if LEN_EXTRA[i] > 0 {
            lengths.push(LEN_BASE[i] + 1);
            lengths.push(LEN_BASE[i + 1] - 1);
        }
    }
    lengths.push(258);
    for (k, &len) in lengths.iter().enumerate() {
        let pre = rng.bytes(300);
        let mut toks: Vec<Tok> = pre.iter().copied().map(Tok::Lit).collect();
        toks.push(Tok::Match { len, dist: 300 });
        let expected = expand(&toks);
        let mut w = BitWriter::new();
        write_fixed_block(&mut w, true, &toks);
        diff_inflate_expect(&format!("row11/len={len}"), &w.finish(), k % 4, &expected);
    }
}

// --------------------------------------------------------------------------
// row 12: one match per cp_dist_base / cp_dist_extra_bits bucket
// --------------------------------------------------------------------------

#[test]
fn row12_every_distance_bucket() {
    let rng = Rng::new(SEED ^ 12);
    let mut dists: Vec<u32> = DIST_BASE.to_vec();
    for i in 0..29 {
        if DIST_EXTRA[i] > 0 {
            dists.push(DIST_BASE[i] + 1);
            dists.push(DIST_BASE[i + 1] - 1);
        }
    }
    dists.push(32768);
    for (k, &dist) in dists.iter().enumerate() {
        let pre = rng.bytes_alphabet(dist as usize, 251);
        let mut toks: Vec<Tok> = pre.iter().copied().map(Tok::Lit).collect();
        toks.push(Tok::Match { len: 3, dist });
        toks.push(Tok::Match { len: 17, dist });
        let expected = expand(&toks);
        let mut w = BitWriter::new();
        write_fixed_block(&mut w, true, &toks);
        diff_inflate_expect(&format!("row12/dist={dist}"), &w.finish(), k % 4, &expected);
    }
}

// --------------------------------------------------------------------------
// row 13: 9-bit literal codes (symbols 144..255) so cp_build's `len <= 9`
// lookup-fill boundary and the wider tree entries are exercised
// --------------------------------------------------------------------------

#[test]
fn row13_nine_bit_literals() {
    let rng = Rng::new(SEED ^ 13);
    for i in 0..150 {
        let n = rng.range(1, 80);
        // only symbols with 9-bit fixed codes
        let data: Vec<u8> = (0..n).map(|_| 144 + rng.below(112) as u8).collect();
        let stream = fixed_literal_stream(&data);
        diff_inflate_expect(&format!("row13/9bit #{i}"), &stream, i % 4, &data);
    }
    // mixed 7/8/9-bit code lengths in one block
    for i in 0..150 {
        let n = rng.range(1, 200);
        let data: Vec<u8> = (0..n)
            .map(|_| if rng.below(2) == 0 { rng.below(144) as u8 } else { (144 + rng.below(112)) as u8 })
            .collect();
        let stream = fixed_literal_stream(&data);
        diff_inflate_expect(&format!("row13/mixed #{i}"), &stream, i % 4, &data);
    }
}

// --------------------------------------------------------------------------
// rows 14-19: dynamic Huffman blocks, produced by a real encoder
// --------------------------------------------------------------------------

/// Drive one dynamic-Huffman configuration end to end and return the exact
/// code-length-code item list that was transmitted, so the caller can assert
/// which of the RLE symbols 16/17/18 really appeared in the stream.
#[track_caller]
fn dyn_case(
    label: &str,
    toks: &[Tok],
    assign: fn(usize) -> Vec<u8>,
    rle: bool,
    hclen: Option<usize>,
    pad_nlit: Option<usize>,
    pad_ndst: Option<usize>,
    align: usize,
) -> (DynSpec, Vec<ClItem>) {
    let mut spec = dyn_spec_for(toks, assign, rle);
    spec.hclen = hclen;
    if let Some(n) = pad_nlit {
        assert!(n >= spec.litlens.len());
        spec.litlens.resize(n, 0);
    }
    if let Some(n) = pad_ndst {
        assert!(n >= spec.distlens.len());
        spec.distlens.resize(n, 0);
    }
    let mut w = BitWriter::new();
    write_dynamic_block(&mut w, true, &spec, toks);
    let expected = expand(toks);
    diff_inflate_expect(label, &w.finish(), align, &expected);
    let mut seq = spec.litlens.clone();
    seq.extend_from_slice(&spec.distlens);
    let items = rle_code_lengths(&seq, spec.rle);
    (spec, items)
}

fn cl_syms(items: &[ClItem]) -> std::collections::BTreeSet<u32> {
    items.iter().map(|&(s, _, _)| s).collect()
}

#[test]
fn row14_dynamic_small_alphabet() {
    let rng = Rng::new(SEED ^ 14);
    let mut saw_zero_run = false;
    for i in 0..80 {
        let alpha = rng.range(2, 8);
        let n = rng.range(8, 200);
        let mut toks: Vec<Tok> =
            rng.bytes_alphabet(n, alpha).into_iter().map(Tok::Lit).collect();
        // a couple of matches so the distance tree is non-trivial too
        toks.push(Tok::Match { len: rng.range(3, 20) as u32, dist: rng.range(1, n) as u32 });
        let (_, items) = dyn_case(
            &format!("row14/own alpha={alpha} n={n} #{i}"),
            &toks,
            balanced_lengths,
            true,
            None,
            None,
            None,
            i % 4,
        );
        let s = cl_syms(&items);
        saw_zero_run |= s.contains(&17) || s.contains(&18);
    }
    assert!(saw_zero_run, "row14 never transmitted a zero-run code");

    // plus real encoder output for the same payload shape
    for i in 0..40 {
        let n = rng.range(64, 2048);
        let data = rng.bytes_alphabet(n, rng.range(2, 8));
        for level in [6u32, 9] {
            let stream = flate2_raw(&data, level);
            diff_inflate_expect(&format!("row14/flate2 lvl={level} #{i}"), &stream, i % 4, &data);
        }
    }
}

#[test]
fn row15_dynamic_long_zero_runs_code18() {
    let rng = Rng::new(SEED ^ 15);
    let mut saw18 = 0;
    for i in 0..60 {
        // a single distinct literal => 250+ consecutive zero code lengths => 18
        let b = rng.byte();
        let n = rng.range(1, 300);
        let toks: Vec<Tok> = std::iter::repeat(Tok::Lit(b)).take(n).collect();
        let (_, items) = dyn_case(
            &format!("row15/own single-literal n={n} #{i}"),
            &toks,
            balanced_lengths,
            true,
            None,
            None,
            None,
            i % 4,
        );
        if cl_syms(&items).contains(&18) {
            saw18 += 1;
        }
    }
    assert!(saw18 >= 50, "row15: code 18 appeared only {saw18} times");

    for i in 0..40 {
        let n = rng.range(32, 4096);
        let b = rng.byte();
        let data = vec![b; n];
        let stream = flate2_raw(&data, 9);
        diff_inflate_expect(&format!("row15/flate2 n={n} #{i}"), &stream, i % 4, &data);
    }
}

#[test]
fn row16_dynamic_repeat_code16() {
    let rng = Rng::new(SEED ^ 16);
    let mut saw16 = 0;
    for i in 0..60 {
        // a wide literal alphabet => long runs of *equal* code lengths => 16
        let alpha = rng.range(64, 256);
        let n = rng.range(alpha, alpha * 3);
        let mut data: Vec<u8> = (0..alpha).map(|k| k as u8).collect();
        data.extend(rng.bytes_alphabet(n, alpha));
        let toks: Vec<Tok> = data.into_iter().map(Tok::Lit).collect();
        let (_, items) = dyn_case(
            &format!("row16/own alpha={alpha} #{i}"),
            &toks,
            balanced_lengths,
            true,
            None,
            None,
            None,
            i % 4,
        );
        if cl_syms(&items).contains(&16) {
            saw16 += 1;
        }
    }
    assert!(saw16 >= 50, "row16: code 16 appeared only {saw16} times");
}

#[test]
fn row17_dynamic_hdist_one() {
    let rng = Rng::new(SEED ^ 17);
    for i in 0..80 {
        // no matches at all => HDIST == 1 with a single zero-length distance code
        let n = rng.range(1, 60);
        let toks: Vec<Tok> = rng.bytes(n).into_iter().map(Tok::Lit).collect();
        let (spec, _) = dyn_case(
            &format!("row17/own hdist1 n={n} #{i}"),
            &toks,
            balanced_lengths,
            true,
            None,
            None,
            None,
            i % 4,
        );
        assert_eq!(spec.distlens, vec![0u8], "row17 expected HDIST==1");
    }

    for i in 0..40 {
        let n = rng.range(2, 30);
        let data: Vec<u8> = (0..n).map(|k| rng.byte() ^ (k as u8).wrapping_mul(37)).collect();
        let stream = flate2_raw(&data, 9);
        diff_inflate_expect(&format!("row17/flate2 n={n} #{i}"), &stream, i % 4, &data);
    }
}

#[test]
fn row18_dynamic_full_header_sizes() {
    let rng = Rng::new(SEED ^ 18);
    // HLIT == 288 and HDIST == 32 (the maxima), with trailing zero lengths
    for i in 0..40 {
        let mut toks: Vec<Tok> = (0..=255u8).map(Tok::Lit).collect();
        toks.extend(rng.bytes(200).into_iter().map(Tok::Lit));
        toks.push(Tok::Match { len: 258, dist: 1 });
        toks.push(Tok::Match { len: 3, dist: 400 });
        let (spec, _) = dyn_case(
            &format!("row18/own hlit288 hdist32 #{i}"),
            &toks,
            balanced_lengths,
            true,
            None,
            Some(288),
            Some(32),
            i % 4,
        );
        assert_eq!(spec.litlens.len(), 288);
        assert_eq!(spec.distlens.len(), 32);
    }
    // minimal HLIT (257)
    for i in 0..40 {
        let n = rng.range(1, 40);
        let toks: Vec<Tok> = rng.bytes_alphabet(n, 8).into_iter().map(Tok::Lit).collect();
        let (spec, _) = dyn_case(
            &format!("row18/own hlit257 #{i}"),
            &toks,
            balanced_lengths,
            true,
            None,
            None,
            None,
            i % 4,
        );
        assert_eq!(spec.litlens.len(), 257, "row18 expected minimal HLIT");
    }
    for i in 0..20 {
        let mut data: Vec<u8> = (0..=255u8).collect();
        data.extend(rng.bytes(1024));
        let tail = data.clone();
        data.extend_from_slice(&tail);
        data.extend(rng.bytes_alphabet(512, 16));
        let stream = flate2_raw(&data, 9);
        diff_inflate_expect(&format!("row18/flate2 #{i}"), &stream, i % 4, &data);
    }
}

#[test]
fn row19_dynamic_matches_all_classes() {
    let rng = Rng::new(SEED ^ 19);
    // every length bucket and every distance bucket, through *dynamic* trees
    for (k, &len) in LEN_BASE.iter().enumerate() {
        let pre = rng.bytes_alphabet(300, 40);
        let mut toks: Vec<Tok> = pre.into_iter().map(Tok::Lit).collect();
        toks.push(Tok::Match { len, dist: 300 });
        toks.push(Tok::Match { len: 3, dist: 1 });
        dyn_case(
            &format!("row19/dyn len={len}"),
            &toks,
            balanced_lengths,
            true,
            None,
            None,
            None,
            k % 4,
        );
    }
    for (k, &dist) in DIST_BASE.iter().enumerate() {
        let pre = rng.bytes_alphabet(dist as usize, 40);
        let mut toks: Vec<Tok> = pre.into_iter().map(Tok::Lit).collect();
        toks.push(Tok::Match { len: 258, dist });
        dyn_case(
            &format!("row19/dyn dist={dist}"),
            &toks,
            balanced_lengths,
            true,
            None,
            None,
            None,
            k % 4,
        );
    }
    // real encoder, every level, on structured data
    for i in 0..20 {
        let mut data: Vec<u8> = Vec::new();
        data.extend(std::iter::repeat(rng.byte()).take(rng.range(3, 258)));
        let block = rng.bytes(rng.range(3, 300));
        for _ in 0..rng.range(2, 6) {
            data.extend_from_slice(&block);
        }
        data.extend(rng.bytes_alphabet(rng.range(100, 1000), 6));
        let far = data.clone();
        data.extend_from_slice(&far);
        for level in 1..=9u32 {
            let stream = flate2_raw(&data, level);
            diff_inflate_expect(&format!("row19/flate2 lvl={level} #{i}"), &stream, i % 4, &data);
        }
    }
}

// --------------------------------------------------------------------------
// rows 20-22: stored (BTYPE=00) blocks
// --------------------------------------------------------------------------

/// One stored-block case.
///
/// IMPORTANT (C ground truth): `cp_ptr()` derives the byte position from
/// `s->count`, but `cp_peak_bits()` adds `s->bits_left` — not the *unbuffered*
/// bit count — when it folds in `s->final_word`. So whenever the final partial
/// word has already been merged with a non-empty accumulator, `s->count` is too
/// large by exactly the old `count`, and `cp_stored()` copies from the wrong
/// offset. That happens exactly when `(in_bytes - first_bytes) % 4 != 0`.
///
/// The payload is therefore only asserted to round-trip when the input is a
/// whole number of 32-bit words past `first_bytes`; in every other case we still
/// require the C and Rust libraries to produce the *same* (wrong) bytes, which
/// is the property under test. Returns `true` when the payload was checked.
#[track_caller]
fn stored_case(label: &str, data: &[u8], align: usize) -> bool {
    let mut w = BitWriter::new();
    write_stored_block(&mut w, true, data);
    let stream = w.finish();
    // give the output room for the misaligned copy, and slack in the input so
    // the (deliberately reproduced) over-read stays inside the allocation
    let o = diff_inflate_full(
        label,
        &stream,
        align,
        stream.len() as i32,
        data.len() as i32,
        data.len() + 64,
    );
    assert_eq!(o.ret, 1, "[{label}] stored block unexpectedly rejected: {:?}", o.err);
    let first_bytes = (4 - align % 4) % 4;
    let word_aligned = (stream.len().wrapping_sub(first_bytes)) % 4 == 0;
    if word_aligned {
        assert_eq!(
            &o.out[..data.len()],
            data,
            "[{label}] word-aligned stored block must round-trip exactly"
        );
    }
    word_aligned
}

#[test]
fn row20_stored_various_lengths() {
    let rng = Rng::new(SEED ^ 20);
    let mut checked = 0;
    for len in [0usize, 1, 2, 3, 4, 5, 17, 64, 255, 256, 1000] {
        for i in 0..12 {
            let data = rng.bytes(len);
            if stored_case(&format!("row20/stored len={len} #{i}"), &data, i % 4) {
                checked += 1;
            }
        }
    }
    assert!(checked > 20, "row20 never hit the exactly-round-tripping shape");
}

#[test]
fn row21_stored_after_alignment_discard() {
    // The byte-alignment discard `cp_read_bits(s, s->count & 7)` is always
    // non-zero for a stored block (3 header bits consumed), and its size varies
    // with the input alignment.
    let rng = Rng::new(SEED ^ 21);
    let mut checked = 0;
    for align in 0..4usize {
        for i in 0..40 {
            let data = rng.bytes(rng.range(1, 300));
            if stored_case(&format!("row21/align={align} #{i}"), &data, align) {
                checked += 1;
            }
        }
    }
    assert!(checked > 20, "row21 never hit the exactly-round-tripping shape");
}

#[test]
fn row22_stored_alignment_times_tail_cross_product() {
    // cp_ptr()'s byte address is (words + word_index*4) - count/8; sweep every
    // (align, in_bytes mod 4) pair. in_bytes = 5 + LEN, so LEN drives the tail.
    let rng = Rng::new(SEED ^ 22);
    let mut seen = [[0usize; 4]; 4];
    for align in 0..4usize {
        for len in 0..24usize {
            let data = rng.bytes(len);
            let first_bytes = (4 - align % 4) % 4;
            let tail = ((5 + len).wrapping_sub(first_bytes)) & 3;
            seen[align][tail] += 1;
            stored_case(&format!("row22/align={align} len={len} tail={tail}"), &data, align);
        }
    }
    for align in 0..4 {
        for tail in 0..4 {
            assert!(seen[align][tail] > 0, "row22: (align {align}, tail {tail}) never exercised");
        }
    }
}

// --------------------------------------------------------------------------
// rows 23-24: BFINAL chaining / mixed block types
// --------------------------------------------------------------------------

#[test]
fn row23_two_fixed_blocks() {
    let rng = Rng::new(SEED ^ 23);
    for i in 0..120 {
        let a = rng.bytes(rng.range(1, 40));
        let b = rng.bytes(rng.range(1, 40));
        let mut w = BitWriter::new();
        write_fixed_block(&mut w, false, &a.iter().copied().map(Tok::Lit).collect::<Vec<_>>());
        write_fixed_block(&mut w, true, &b.iter().copied().map(Tok::Lit).collect::<Vec<_>>());
        let mut expected = a.clone();
        expected.extend_from_slice(&b);
        diff_inflate_expect(&format!("row23/two-blocks #{i}"), &w.finish(), i % 4, &expected);
    }
}

#[test]
fn row24_many_chained_blocks() {
    let rng = Rng::new(SEED ^ 24);
    for i in 0..60 {
        let nblocks = rng.range(2, 6);
        let mut w = BitWriter::new();
        let mut expected: Vec<u8> = Vec::new();
        for b in 0..nblocks {
            let chunk = rng.bytes(rng.range(1, 30));
            let final_ = b + 1 == nblocks;
            let mut toks: Vec<Tok> = chunk.iter().copied().map(Tok::Lit).collect();
            // add a back-reference that reaches into an earlier block's output
            if expected.len() + chunk.len() > 8 {
                toks.push(Tok::Match { len: 4, dist: (expected.len() + chunk.len()).min(300) as u32 });
            }
            let mut sim = expected.clone();
            for t in &toks {
                match *t {
                    Tok::Lit(x) => sim.push(x),
                    Tok::Match { len, dist } => {
                        let s = sim.len() - dist as usize;
                        for k in 0..len as usize {
                            let v = sim[s + k];
                            sim.push(v);
                        }
                    }
                }
            }
            expected = sim;
            write_fixed_block(&mut w, final_, &toks);
        }
        diff_inflate_expect(&format!("row24/chained #{i}"), &w.finish(), i % 4, &expected);
    }
}

// --------------------------------------------------------------------------
// row 25: out_bytes larger than the decompressed size
// --------------------------------------------------------------------------

#[test]
fn row25_out_buffer_with_slack() {
    let rng = Rng::new(SEED ^ 25);
    for i in 0..150 {
        let data = rng.bytes(rng.range(1, 200));
        let stream = flate2_raw(&data, 6);
        let slack = rng.range(1, 512);
        diff_inflate_expect_slack(
            &format!("row25/slack={slack} #{i}"),
            &stream,
            i % 4,
            &data,
            data.len() + slack,
        );
    }
}

// --------------------------------------------------------------------------
// row 26: every compression level
// --------------------------------------------------------------------------

#[test]
fn row26_all_compression_levels() {
    let rng = Rng::new(SEED ^ 26);
    for level in 0..=9u32 {
        for i in 0..40 {
            // keep level 0 payloads under 65535 so it is a single stored block
            let n = rng.range(1, 4000);
            let kind = rng.below(3);
            let data = match kind {
                0 => rng.bytes(n),
                1 => rng.bytes_alphabet(n, rng.range(2, 20)),
                _ => {
                    let unit = rng.bytes(rng.range(1, 40));
                    unit.iter().cycle().take(n).copied().collect()
                }
            };
            let stream = flate2_raw(&data, level);
            diff_inflate_expect(&format!("row26/lvl={level} #{i}"), &stream, i % 4, &data);
        }
    }
}

// --------------------------------------------------------------------------
// row 27: large payload with 32 KiB-scale distances
// --------------------------------------------------------------------------

#[test]
fn row27_large_payload() {
    let rng = Rng::new(SEED ^ 27);
    for i in 0..4 {
        let half = rng.bytes_alphabet(32 * 1024, 64);
        let mut data = half.clone();
        data.extend_from_slice(&half); // matches at distance 32768
        let stream = flate2_raw(&data, 9);
        diff_inflate_expect(&format!("row27/64KiB #{i}"), &stream, i % 4, &data);
    }
}

// --------------------------------------------------------------------------
// row 28: repeated calls must not leak state between invocations
// --------------------------------------------------------------------------

#[test]
fn row28_no_state_leak_between_calls() {
    let rng = Rng::new(SEED ^ 28);
    let p = pair();
    let mut streams: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
    for _ in 0..60 {
        let data = rng.bytes_alphabet(rng.range(1, 500), rng.range(1, 40));
        streams.push((flate2_raw(&data, rng.below(10) as u32), data));
    }
    // interleave the same call sequence in both libraries and compare each step
    let _g = global_lock();
    for (k, (stream, data)) in streams.iter().enumerate() {
        let a = run_inflate(&p.c, stream, k % 4, stream.len() as i32, data.len() as i32, data.len() + 32, 0xCD);
        let b = run_inflate(&p.rs, stream, k % 4, stream.len() as i32, data.len() as i32, data.len() + 32, 0xCD);
        assert_eq!(a.ret, b.ret, "row28 step {k}: ret");
        assert_eq!(a.out, b.out, "row28 step {k}: out");
        assert_eq!(a.err, b.err, "row28 step {k}: err");
        assert_eq!(a.ret, 1, "row28 step {k}: expected success");
        assert_eq!(&a.out[..data.len()], &data[..], "row28 step {k}: payload");
    }
}

// --------------------------------------------------------------------------
// rows 29-33: convert_pix
// --------------------------------------------------------------------------

fn convert_pix_shapes(bpp: i32, row: &str) {
    let rng = Rng::new(SEED ^ (0x100 + bpp as u64));
    for &w in &[0i32, 1, 7, 64] {
        for &h in &[0i32, 1, 5, 33] {
            for i in 0..20 {
                // src layout: h rows of (1 filter byte + w*bpp pixel bytes)
                let need = (h.max(0) as usize) * (1 + (w.max(0) as usize) * bpp as usize) + 16;
                let src = rng.bytes(need.max(16));
                let dst_len = (w.max(0) as usize) * (h.max(0) as usize) + 8;
                diff_convert_pix(
                    &format!("{row}/bpp={bpp} w={w} h={h} #{i}"),
                    bpp,
                    w,
                    h,
                    &src,
                    dst_len,
                );
            }
        }
    }
}

#[test]
fn row29_convert_pix_bpp1() {
    convert_pix_shapes(1, "row29");
}
#[test]
fn row30_convert_pix_bpp2() {
    convert_pix_shapes(2, "row30");
}
#[test]
fn row31_convert_pix_bpp3() {
    convert_pix_shapes(3, "row31");
}
#[test]
fn row32_convert_pix_bpp4() {
    convert_pix_shapes(4, "row32");
}

#[test]
fn row33_convert_pix_large() {
    let rng = Rng::new(SEED ^ 33);
    for bpp in 1..=4i32 {
        for i in 0..10 {
            let w = 256i32;
            let h = 64i32;
            let need = h as usize * (1 + w as usize * bpp as usize) + 16;
            let src = rng.bytes(need);
            let dst_len = (w * h) as usize + 37;
            diff_convert_pix(&format!("row33/bpp={bpp} #{i}"), bpp, w, h, &src, dst_len);
        }
    }
}

// --------------------------------------------------------------------------
// row 34: exported data tables must be byte-identical
// --------------------------------------------------------------------------

#[test]
fn row34_exported_tables_identical() {
    let p = pair();
    let u8_tables: &[(&[u8], usize)] = &[
        (b"cp_fixed_table", 288 + 32),
        (b"cp_permutation_order", 19),
        (b"cp_len_extra_bits", 29 + 2),
        (b"cp_dist_extra_bits", 30 + 2),
    ];
    for (name, len) in u8_tables {
        let a = p.c.data_u8(name, *len);
        let b = p.rs.data_u8(name, *len);
        assert_eq!(
            a,
            b,
            "row34: {} differs\n  C   ={:?}\n  RUST={:?}",
            String::from_utf8_lossy(name),
            a,
            b
        );
    }
    let u32_tables: &[(&[u8], usize)] = &[(b"cp_len_base", 29 + 2), (b"cp_dist_base", 30 + 2)];
    for (name, len) in u32_tables {
        let a = p.c.data_u32(name, *len);
        let b = p.rs.data_u32(name, *len);
        assert_eq!(a, b, "row34: {} differs", String::from_utf8_lossy(name));
    }
}

// --------------------------------------------------------------------------
// rows 36-38: dynamic-block axes that no real encoder exposes
// --------------------------------------------------------------------------

#[test]
fn row36_dynamic_codes_longer_than_nine_bits() {
    // A maximally skewed complete code gives literal codes of 1..15 bits, so
    // cp_build takes BOTH its `len <= 9` (lookup fill) and `len > 9` (skip)
    // paths within one tree, and cp_decode binary-searches deep tree entries.
    let rng = Rng::new(SEED ^ 36);
    for nsym in 2..=16usize {
        for i in 0..12 {
            // nsym-1 distinct literals + end-of-block == nsym symbols
            let alphabet: Vec<u8> = (0..nsym as u8 - 1).map(|k| k.wrapping_mul(17)).collect();
            let n = rng.range(1, 120);
            // every alphabet letter must occur, otherwise the skewed assignment
            // is applied to fewer symbols and the max code length shrinks
            let mut toks: Vec<Tok> = alphabet.iter().copied().map(Tok::Lit).collect();
            toks.extend((0..n).map(|_| Tok::Lit(alphabet[rng.below(alphabet.len())])));
            let spec = {
                let mut s = dyn_spec_for(&toks, skewed_lengths, true);
                s.hclen = None;
                s
            };
            let maxlen = spec.litlens.iter().copied().max().unwrap();
            assert_eq!(maxlen as usize, nsym - 1, "row36 skewed length assignment");
            let mut w = BitWriter::new();
            write_dynamic_block(&mut w, true, &spec, &toks);
            diff_inflate_expect(
                &format!("row36/skewed nsym={nsym} maxlen={maxlen} #{i}"),
                &w.finish(),
                i % 4,
                &expand(&toks),
            );
        }
    }
}

#[test]
fn row37_dynamic_hclen_sweep() {
    // HCLEN (nlen) selects how many of the 19 code-length code lengths are
    // transmitted; sweep from the minimum that fits up to the maximum 19.
    let rng = Rng::new(SEED ^ 37);
    let mut seen: std::collections::BTreeSet<usize> = Default::default();
    for i in 0..60 {
        let n = rng.range(1, 100);
        let toks: Vec<Tok> = rng.bytes_alphabet(n, rng.range(2, 30)).into_iter().map(Tok::Lit).collect();
        for hclen in [None, Some(19usize), Some(18), Some(17)] {
            let spec = dyn_spec_for(&toks, balanced_lengths, true);
            let mut spec2 = spec.clone();
            // the minimum viable nlen for this stream
            let mut seq = spec.litlens.clone();
            seq.extend_from_slice(&spec.distlens);
            let items = rle_code_lengths(&seq, true);
            let mut used = [false; 19];
            for &(s, _, _) in &items {
                used[s as usize] = true;
            }
            let mut min_nlen = 4usize;
            for (k, &p) in CL_PERM.iter().enumerate() {
                if used[p] {
                    min_nlen = min_nlen.max(k + 1);
                }
            }
            // NOTE: the code-length *code lengths* are recomputed inside
            // write_dynamic_block, so only forcing nlen >= min_nlen is legal.
            let want = match hclen {
                Some(v) if v >= min_nlen => Some(v),
                Some(_) => continue,
                None => None,
            };
            spec2.hclen = want;
            let mut w = BitWriter::new();
            write_dynamic_block(&mut w, true, &spec2, &toks);
            seen.insert(want.unwrap_or(min_nlen));
            diff_inflate_expect(
                &format!("row37/hclen={:?} #{i}", want),
                &w.finish(),
                i % 4,
                &expand(&toks),
            );
        }
    }
    assert!(seen.len() >= 2, "row37 only exercised one HCLEN value: {seen:?}");
    assert!(seen.contains(&19), "row37 never exercised HCLEN == 19");
}

#[test]
fn row38_dynamic_code_lengths_without_rle() {
    // Same trees, but every code length transmitted literally (no 16/17/18):
    // cp_dynamic's `default:` arm only, 320 iterations of it.
    let rng = Rng::new(SEED ^ 38);
    for i in 0..60 {
        let n = rng.range(1, 200);
        let toks: Vec<Tok> = rng.bytes_alphabet(n, rng.range(2, 60)).into_iter().map(Tok::Lit).collect();
        let (_, items) = dyn_case(
            &format!("row38/no-rle n={n} #{i}"),
            &toks,
            balanced_lengths,
            false,
            None,
            None,
            None,
            i % 4,
        );
        let s = cl_syms(&items);
        assert!(!s.contains(&16) && !s.contains(&17) && !s.contains(&18), "row38 leaked an RLE code");
    }
}

// --------------------------------------------------------------------------
// row 40 — stored block whose LEN exceeds the bytes that remain in the input
//
// cp_stored's only length check is `s->bits_left / 8 <= LEN`, which bounds LEN
// from *below*: a LEN larger than the remaining input is accepted and the
// `memcpy` then reads past `in + in_bytes` and writes past `out + out_bytes`
// (there is no `out_end` check in cp_stored at all). Both are real,
// caller-reachable configurations, so they are tested with the over-read region
// filled deterministically (the input slice handed to the harness is longer than
// the `in_bytes` the library is told about) and with a genuinely large `out`
// allocation, so the comparison is well defined.
// --------------------------------------------------------------------------

#[test]
fn row40_stored_len_beyond_remaining_input() {
    let rng = Rng::new(SEED ^ 40);
    for align in 0..4usize {
        for &m in &[0usize, 1, 3, 4, 7, 16, 33] {
            for &k in &[1usize, 2, 5, 16, 100] {
                let len = (m + k) as u16;
                let payload = rng.bytes(m);
                let mut w = BitWriter::new();
                write_stored_block_raw(&mut w, true, len, !len, &payload);
                let mut stream = w.finish();
                let in_bytes = stream.len() as i32; // == 5 + m
                // deterministic bytes the over-read will pick up
                stream.extend((0..k + 64).map(|i| (i as u8).wrapping_mul(31).wrapping_add(7)));
                let cap = len as usize + 512;
                let o = diff_inflate_full(
                    &format!("row40/align={align} m={m} k={k} len={len}"),
                    &stream,
                    align,
                    in_bytes,
                    (len as usize) as i32,
                    cap,
                );
                assert_eq!(o.ret, 1, "row40: stored block should be accepted");
            }
        }
    }
}

// row 41 lives in `tests/risky.rs`: some of its (align, in_bytes) combinations
// abort inside the C library, so it has to be driven in a child process.
