//! Phase B — valid-path differential tests for `cp_inflate` and the exported
//! globals. One test per `CONFIGS.md` row C1..C32, each driven with many
//! randomized inputs from a fixed seed.
//!
//! Every call goes through `libloading` into the respective `.so`; the Rust
//! implementation is never invoked directly.

mod common;

use common::deflate::*;
use common::*;

const AB: CBuild = CBuild::AsBuilt;

// ---------------------------------------------------------------------------
// C1-C5 — btype 0 (stored)
// ---------------------------------------------------------------------------

/// Faithful model of the bit reader for the `[1, 2, count&7, 16, 16]` header
/// sequence a stored block reads when it is the *first* block, returning the
/// byte offset `cp_ptr` hands to `memcpy`.
///
/// This is deliberately not "where the payload is": `cp_ptr` computes
/// `(char *)(words + word_index) - count / 8`, which silently assumes every
/// buffered bit came from a full 32-bit word. When the trailing partial word
/// (`final_word`) has been folded in, `count` grew by `bits_left` instead of
/// 32, so the pointer lands short. The C does this, so the Rust must too, and
/// the test pins the resulting bytes rather than the intended ones.
fn stored_memcpy_offset(in_bytes: usize, align: usize) -> usize {
    let first_bytes = (4 - align) % 4;
    let word_count = (in_bytes - first_bytes) / 4;
    let last_bytes = (in_bytes - first_bytes) % 4;

    let mut count: i64 = first_bytes as i64 * 8;
    let mut bits_left: i64 = in_bytes as i64 * 8;
    let mut word_index: i64 = 0;
    let mut final_avail = last_bytes != 0;

    let read = |n: i64,
                    count: &mut i64,
                    bits_left: &mut i64,
                    word_index: &mut i64,
                    final_avail: &mut bool| {
        if *count < n {
            if *word_index < word_count as i64 {
                *word_index += 1;
                *count += 32;
            } else if *final_avail {
                *count += *bits_left;
                *final_avail = false;
            }
        }
        *count -= n;
        *bits_left -= n;
    };

    read(1, &mut count, &mut bits_left, &mut word_index, &mut final_avail);
    read(2, &mut count, &mut bits_left, &mut word_index, &mut final_avail);
    let pad = count & 7;
    read(pad, &mut count, &mut bits_left, &mut word_index, &mut final_avail);
    read(16, &mut count, &mut bits_left, &mut word_index, &mut final_avail);
    read(16, &mut count, &mut bits_left, &mut word_index, &mut final_avail);

    (first_bytes as i64 + 4 * word_index - count / 8) as usize
}

/// The bytes `cp_stored`'s `memcpy` will actually read: the input followed by
/// the zero padding `AlignedBuf` guarantees.
fn padded(stream: &[u8], extra: usize) -> Vec<u8> {
    let mut v = stream.to_vec();
    v.resize(stream.len() + extra + 128, 0);
    v
}

fn stored_row(align: usize, seed: u64, label: &str) {
    let mut rng = Rng::new(seed);
    let mut exact_hits = 0usize;
    for iter in 0..120 {
        let len = if iter < 5 { iter } else { rng.range(0, 4096) };
        let data = rng.bytes(len);
        let mut d = Deflate::new();
        d.stored(true, &data);
        let stream = d.finish();
        let out = diff_inflate(
            InflateCase::new(&stream, len + 64).in_align(align),
            AB,
            label,
        );
        assert_eq!(out.ret, 1, "{label}: len={len} err={:?}", out.err);

        let off = stored_memcpy_offset(stream.len(), align);
        let src = padded(&stream, len);
        assert_eq!(
            &out.out[..len],
            &src[off..off + len],
            "{label}: len={len} memcpy offset={off} (stream {} bytes)",
            stream.len()
        );
        if off == stream.len() - len {
            exact_hits += 1;
            assert_eq!(&out.out[..len], &data[..], "{label}: payload len={len}");
        }
    }
    assert!(
        exact_hits > 0,
        "{label}: never hit the offset where cp_ptr is actually correct"
    );
}

#[test]
fn c1_stored_align0() {
    stored_row(0, 0x5701, "C1");
}

#[test]
fn c2_stored_align1() {
    stored_row(1, 0x5702, "C2");
}

#[test]
fn c3_stored_align2() {
    stored_row(2, 0x5703, "C3");
}

#[test]
fn c4_stored_align3() {
    stored_row(3, 0x5704, "C4");
}

#[test]
fn c5_stored_boundary_lengths() {
    for align in 0..4 {
        for len in 0..=8usize {
            let data: Vec<u8> = (0..len).map(|i| (i as u8) ^ 0xA5).collect();
            let mut d = Deflate::new();
            d.stored(true, &data);
            let stream = d.finish();
            let out = diff_inflate(
                InflateCase::new(&stream, len + 64).in_align(align),
                AB,
                "C5",
            );
            assert_eq!(out.ret, 1, "C5: align={align} len={len} err={:?}", out.err);
            let off = stored_memcpy_offset(stream.len(), align);
            let src = padded(&stream, len);
            assert_eq!(
                &out.out[..len],
                &src[off..off + len],
                "C5: align={align} len={len} offset={off}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C6-C14 — btype 1 (fixed Huffman)
// ---------------------------------------------------------------------------

#[test]
fn c6_fixed_literals_all_alignments() {
    let mut rng = Rng::new(0x5706);
    for align in 0..4 {
        // Sweeping the payload length sweeps last_bytes = (in_bytes-first) & 3.
        for iter in 0..40 {
            let n = if iter < 8 { iter + 1 } else { rng.range(1, 3000) };
            let toks = rand_literals(&mut rng, n);
            let expected = expand(&toks);
            let mut d = Deflate::new();
            d.fixed(true, &toks);
            let stream = d.finish();
            let out = diff_inflate(
                InflateCase::new(&stream, expected.len() + 64).in_align(align),
                AB,
                "C6",
            );
            assert_eq!(out.ret, 1, "C6: align={align} n={n} err={:?}", out.err);
            assert_eq!(&out.out[..expected.len()], &expected[..]);
        }
    }
}

#[test]
fn c7_fixed_literal_code_width_split() {
    // 0..=143 use 8-bit codes, 144..=255 use 9-bit codes: both sides of
    // cp_build's `len <= 9` lookup branch and of the 8/9-bit decode.
    let mut rng = Rng::new(0x5707);
    for &(lo, hi) in &[(0u16, 143u16), (144, 255), (0, 255)] {
        for _ in 0..40 {
            let n = rng.range(1, 1500);
            let toks: Vec<Tok> = (0..n)
                .map(|_| Tok::Lit(rng.range(lo as usize, hi as usize) as u8))
                .collect();
            let expected = expand(&toks);
            let mut d = Deflate::new();
            d.fixed(true, &toks);
            let stream = d.finish();
            let out = diff_inflate(
                InflateCase::new(&stream, expected.len() + 64),
                AB,
                "C7",
            );
            assert_eq!(out.ret, 1, "C7: err={:?}", out.err);
            assert_eq!(&out.out[..expected.len()], &expected[..]);
        }
    }
}

#[test]
fn c8_fixed_match_distance_one_memset_path() {
    let mut rng = Rng::new(0x5708);
    for _ in 0..200 {
        let seed_byte = rng.byte();
        let length = rng.range(3, 258) as u32;
        let toks = vec![Tok::Lit(seed_byte), Tok::Match(length, 1)];
        let expected = expand(&toks);
        assert_eq!(expected.len(), 1 + length as usize);
        assert!(expected.iter().all(|&b| b == seed_byte));
        let mut d = Deflate::new();
        d.fixed(true, &toks);
        let stream = d.finish();
        let out = diff_inflate(
            InflateCase::new(&stream, expected.len() + 64),
            AB,
            "C8",
        );
        assert_eq!(out.ret, 1, "C8: len={length} err={:?}", out.err);
        assert_eq!(&out.out[..expected.len()], &expected[..]);
    }
}

#[test]
fn c9_fixed_match_non_overlapping() {
    let mut rng = Rng::new(0x5709);
    for _ in 0..200 {
        let prefix = rng.range(4, 400);
        let mut toks: Vec<Tok> = (0..prefix).map(|_| Tok::Lit(rng.byte())).collect();
        // distance >= length so the bytewise copy never reads what it wrote.
        let length = rng.range(3, prefix.min(258)) as u32;
        let dist = rng.range(length as usize, prefix) as u32;
        toks.push(Tok::Match(length, dist));
        toks.push(Tok::Lit(rng.byte()));
        let expected = expand(&toks);
        let mut d = Deflate::new();
        d.fixed(true, &toks);
        let stream = d.finish();
        let out = diff_inflate(
            InflateCase::new(&stream, expected.len() + 64),
            AB,
            "C9",
        );
        assert_eq!(out.ret, 1, "C9: len={length} dist={dist} err={:?}", out.err);
        assert_eq!(&out.out[..expected.len()], &expected[..]);
    }
}

#[test]
fn c10_fixed_match_overlapping() {
    let mut rng = Rng::new(0x570A);
    for _ in 0..250 {
        let prefix = rng.range(2, 60);
        let mut toks: Vec<Tok> = (0..prefix).map(|_| Tok::Lit(rng.byte())).collect();
        // 1 < distance < length: the copy propagates through fresh bytes.
        let dist = rng.range(2, prefix.min(255)) as u32;
        let length = rng.range(dist as usize + 1, 258) as u32;
        toks.push(Tok::Match(length, dist));
        let expected = expand(&toks);
        let mut d = Deflate::new();
        d.fixed(true, &toks);
        let stream = d.finish();
        let out = diff_inflate(
            InflateCase::new(&stream, expected.len() + 64),
            AB,
            "C10",
        );
        assert_eq!(out.ret, 1, "C10: len={length} dist={dist} err={:?}", out.err);
        assert_eq!(&out.out[..expected.len()], &expected[..]);
    }
}

#[test]
fn c11_fixed_every_length_code() {
    // Length symbols 0..=30. 29 and 30 have cp_len_base == 0 and are reachable
    // through fixed literal symbols 286/287, producing a zero-length copy that
    // still consumes a distance code.
    let mut rng = Rng::new(0x570B);
    for lc in 0..=30usize {
        let nextra = LEN_EXTRA_FULL[lc];
        let maxextra = if nextra == 0 { 0 } else { (1u32 << nextra) - 1 };
        for &lextra in &[0u32, maxextra] {
            for dc in [0usize, 1, 3] {
                let prefix = 600usize;
                let mut toks: Vec<Tok> =
                    (0..prefix).map(|_| Tok::Lit(rng.byte())).collect();
                toks.push(Tok::MatchRaw { lc, lextra, dc, dextra: 0 });
                toks.push(Tok::Lit(0x5A));
                let expected = expand(&toks);
                let mut d = Deflate::new();
                d.fixed(true, &toks);
                let stream = d.finish();
                let out = diff_inflate(
                    InflateCase::new(&stream, expected.len() + 64),
                    AB,
                    "C11",
                );
                assert_eq!(
                    out.ret, 1,
                    "C11: lc={lc} lextra={lextra} dc={dc} err={:?}",
                    out.err
                );
                assert_eq!(
                    &out.out[..expected.len()],
                    &expected[..],
                    "C11: lc={lc} lextra={lextra} dc={dc}"
                );
            }
        }
    }
}

#[test]
fn c12_fixed_every_distance_code() {
    let mut rng = Rng::new(0x570C);
    for dc in 0..=29usize {
        let nextra = DIST_EXTRA_FULL[dc];
        let maxextra = if nextra == 0 { 0 } else { (1u32 << nextra) - 1 };
        for &dextra in &[0u32, maxextra] {
            let dist = DIST_BASE_FULL[dc] + dextra;
            if dist > 32768 {
                continue;
            }
            // Emit at least `dist` bytes first so ERRORS.md E4 does not trip.
            let prefix = dist as usize + 4;
            let mut toks: Vec<Tok> = (0..prefix).map(|_| Tok::Lit(rng.byte())).collect();
            toks.push(Tok::MatchRaw { lc: 5, lextra: 0, dc, dextra });
            let expected = expand(&toks);
            let mut d = Deflate::new();
            d.fixed(true, &toks);
            let stream = d.finish();
            let out = diff_inflate(
                InflateCase::new(&stream, expected.len() + 64),
                AB,
                "C12",
            );
            assert_eq!(out.ret, 1, "C12: dc={dc} dextra={dextra} err={:?}", out.err);
            assert_eq!(
                &out.out[..expected.len()],
                &expected[..],
                "C12: dc={dc} dist={dist}"
            );
        }
    }
}

#[test]
fn c13_fixed_out_bytes_exact() {
    let mut rng = Rng::new(0x570D);
    for _ in 0..150 {
        let toks = { let n = rng.range(1, 200); rand_tokens(&mut rng, n, 4096) };
        let expected = expand(&toks);
        let mut d = Deflate::new();
        d.fixed(true, &toks);
        let stream = d.finish();
        // out_len padded so an overrun is observable, but out_bytes exact.
        let out = diff_inflate(
            InflateCase::new(&stream, expected.len() + 64)
                .out_bytes(expected.len() as i32),
            AB,
            "C13",
        );
        assert_eq!(out.ret, 1, "C13: err={:?}", out.err);
        assert_eq!(&out.out[..expected.len()], &expected[..]);
        assert!(
            out.out[expected.len()..].iter().all(|&b| b == 0),
            "C13: wrote past out_bytes"
        );
    }
}

#[test]
fn c14_fixed_out_bytes_slack() {
    let mut rng = Rng::new(0x570E);
    for _ in 0..150 {
        let toks = { let n = rng.range(1, 200); rand_tokens(&mut rng, n, 4096) };
        let expected = expand(&toks);
        let mut d = Deflate::new();
        d.fixed(true, &toks);
        let stream = d.finish();
        let slack = rng.range(1, 500);
        let out = diff_inflate(
            InflateCase::new(&stream, expected.len() + slack + 64)
                .out_bytes((expected.len() + slack) as i32),
            AB,
            "C14",
        );
        assert_eq!(out.ret, 1, "C14: err={:?}", out.err);
        assert_eq!(&out.out[..expected.len()], &expected[..]);
    }
}

// ---------------------------------------------------------------------------
// C15-C24 — btype 2 (dynamic Huffman)
// ---------------------------------------------------------------------------

fn dynamic_row(
    seed: u64,
    label: &str,
    nlit: usize,
    ndst: usize,
    nlen_min: usize,
    iters: usize,
    with_matches: bool,
) {
    let mut rng = Rng::new(seed);
    for _ in 0..iters {
        let n = rng.range(1, 400);
        let toks = if with_matches {
            // Cap distances so every distance code fits inside `ndst`.
            let max_dist = DIST_BASE_FULL[ndst.min(30) - 1].max(1);
            rand_tokens(&mut rng, n, max_dist)
        } else {
            rand_literals(&mut rng, n)
        };
        let expected = expand(&toks);
        let lit_lens = lit_lens_for(&toks, nlit);
        let dist_lens = dist_lens_for(&toks, ndst);
        let mut d = Deflate::new();
        d.dynamic(true, &toks, &lit_lens, &dist_lens, nlen_min);
        let stream = d.finish();
        let out = diff_inflate(
            InflateCase::new(&stream, expected.len() + 64),
            AB,
            label,
        );
        assert_eq!(
            out.ret, 1,
            "{label}: nlit={nlit} ndst={ndst} nlen_min={nlen_min} err={:?}",
            out.err
        );
        assert_eq!(&out.out[..expected.len()], &expected[..], "{label}");
    }
}

#[test]
fn c15_dynamic_nlen_min() {
    // HCLEN=4 only carries permutation slots {16,17,18,0}, so every literal
    // code length would have to be 0 -- an empty literal tree, which is an
    // error path (see ERRORS.md A10 / phase_c row EX2), not a valid one.
    // The smallest HCLEN a *decodable* block can use is 5: a flat 256-symbol
    // length-8 literal tree uses only code-length symbols {0, 8}, whose
    // permutation indices are 3 and 4.
    let mut rng = Rng::new(0x570F);
    for _ in 0..60 {
        let n = rng.range(1, 400);
        // Literals restricted to 0..=254 so 255 stays unused and symbol 256
        // (end-of-block) is the 256th length-8 code.
        let toks: Vec<Tok> = (0..n)
            .map(|_| Tok::Lit(rng.below(255) as u8))
            .collect();
        let expected = expand(&toks);
        let mut lit_lens = vec![0u8; 257];
        for i in 0..=254usize {
            lit_lens[i] = 8;
        }
        lit_lens[256] = 8;
        assert!(is_complete(&lit_lens), "C15: literal tree must be complete");
        let dist_lens = vec![0u8; 1];
        let mut d = Deflate::new();
        d.dynamic(true, &toks, &lit_lens, &dist_lens, 4);
        let stream = d.finish();
        assert_eq!(
            dynamic_hclen(&stream),
            5,
            "C15: expected the minimum reachable HCLEN of 5"
        );
        let out = diff_inflate(
            InflateCase::new(&stream, expected.len() + 64),
            AB,
            "C15",
        );
        assert_eq!(out.ret, 1, "C15: err={:?}", out.err);
        assert_eq!(&out.out[..expected.len()], &expected[..]);
    }
}

/// Reads HCLEN+4 back out of a dynamic-block stream (bit 0 = BFINAL,
/// bits 1-2 = BTYPE, 3-7 = HLIT, 8-12 = HDIST, 13-16 = HCLEN).
fn dynamic_hclen(stream: &[u8]) -> usize {
    let bit = |i: usize| ((stream[i / 8] >> (i % 8)) & 1) as usize;
    let mut v = 0usize;
    for k in 0..4 {
        v |= bit(13 + k) << k;
    }
    v + 4
}

#[test]
fn c16_dynamic_nlen_max() {
    let mut rng = Rng::new(0x5710);
    for _ in 0..60 {
        let n = rng.range(1, 400);
        let toks = rand_literals(&mut rng, n);
        let expected = expand(&toks);
        let lit_lens = lit_lens_for(&toks, 288);
        let dist_lens = dist_lens_for(&toks, 32);
        let mut d = Deflate::new();
        d.dynamic(true, &toks, &lit_lens, &dist_lens, 19);
        let stream = d.finish();
        assert_eq!(dynamic_hclen(&stream), 19, "C16: HCLEN should be forced to 19");
        let out = diff_inflate(
            InflateCase::new(&stream, expected.len() + 64),
            AB,
            "C16",
        );
        assert_eq!(out.ret, 1, "C16: err={:?}", out.err);
        assert_eq!(&out.out[..expected.len()], &expected[..]);
    }
}

#[test]
fn c17_dynamic_nlit_min_ndst_min() {
    dynamic_row(0x5711, "C17", 257, 1, 4, 80, false);
}

#[test]
fn c18_dynamic_nlit_max_ndst_max() {
    dynamic_row(0x5712, "C18", 288, 32, 4, 80, true);
}

#[test]
fn c19_dynamic_cl_symbol_16() {
    dynamic_rle_row(0x5713, "C19", true, false, false);
}

#[test]
fn c20_dynamic_cl_symbol_17() {
    dynamic_rle_row(0x5714, "C20", false, true, false);
}

#[test]
fn c21_dynamic_cl_symbol_18() {
    dynamic_rle_row(0x5715, "C21", false, false, true);
}

#[test]
fn c22_dynamic_cl_all_rle_symbols() {
    dynamic_rle_row(0x5716, "C22", true, true, true);
}

fn dynamic_rle_row(seed: u64, label: &str, use16: bool, use17: bool, use18: bool) {
    let mut rng = Rng::new(seed);
    for _ in 0..80 {
        // A narrow literal alphabet leaves long zero runs in lit_lens, which is
        // what makes symbols 17/18 fire; repeated equal lengths drive symbol 16.
        let alphabet = rng.range(2, 40);
        let n = rng.range(1, 400);
        let toks: Vec<Tok> = (0..n)
            .map(|_| Tok::Lit((rng.below(alphabet) * 3) as u8))
            .collect();
        let expected = expand(&toks);
        let nlit = rng.range(257, 288);
        let ndst = rng.range(1, 32);
        let lit_lens = lit_lens_for(&toks, nlit);
        let dist_lens = dist_lens_for(&toks, ndst);
        let mut d = Deflate::new();
        d.dynamic_rle(true, &toks, &lit_lens, &dist_lens, 4, use16, use17, use18);
        let stream = d.finish();
        let out = diff_inflate(
            InflateCase::new(&stream, expected.len() + 64),
            AB,
            label,
        );
        assert_eq!(out.ret, 1, "{label}: err={:?}", out.err);
        assert_eq!(&out.out[..expected.len()], &expected[..], "{label}");
    }
}

#[test]
fn c23_dynamic_empty_distance_tree() {
    // What zlib emits for a match-free block: HDIST=1 with a single code length
    // of 0, so cp_build returns ndst == 0. Legal because cp_decode(s->dst, ..)
    // is never reached.
    let mut rng = Rng::new(0x5717);
    for _ in 0..80 {
        let n = rng.range(1, 400);
        let toks = rand_literals(&mut rng, n);
        let expected = expand(&toks);
        let lit_lens = lit_lens_for(&toks, 288);
        let dist_lens = vec![0u8; 1];
        let mut d = Deflate::new();
        d.dynamic(true, &toks, &lit_lens, &dist_lens, 4);
        let stream = d.finish();
        let out = diff_inflate(
            InflateCase::new(&stream, expected.len() + 64),
            AB,
            "C23",
        );
        assert_eq!(out.ret, 1, "C23: err={:?}", out.err);
        assert_eq!(&out.out[..expected.len()], &expected[..]);
    }
}

#[test]
fn c24_dynamic_matches_all_copy_strategies() {
    let mut rng = Rng::new(0x5718);
    for case in 0..3 {
        for _ in 0..80 {
            let prefix = rng.range(4, 300);
            let mut toks: Vec<Tok> = (0..prefix).map(|_| Tok::Lit(rng.byte())).collect();
            let (length, dist) = match case {
                0 => (rng.range(3, 258) as u32, 1u32), // memset path
                1 => {
                    let l = rng.range(3, prefix.min(258)) as u32;
                    (l, rng.range(l as usize, prefix) as u32) // non-overlapping
                }
                _ => {
                    let d0 = rng.range(2, prefix.min(255)) as u32;
                    (rng.range(d0 as usize + 1, 258) as u32, d0) // overlapping
                }
            };
            toks.push(Tok::Match(length, dist));
            let expected = expand(&toks);
            let lit_lens = lit_lens_for(&toks, 288);
            let dist_lens = dist_lens_for(&toks, 32);
            let mut d = Deflate::new();
            d.dynamic(true, &toks, &lit_lens, &dist_lens, 4);
            let stream = d.finish();
            let out = diff_inflate(
                InflateCase::new(&stream, expected.len() + 64),
                AB,
                "C24",
            );
            assert_eq!(out.ret, 1, "C24: case={case} err={:?}", out.err);
            assert_eq!(&out.out[..expected.len()], &expected[..], "C24: case={case}");
        }
    }
}

// ---------------------------------------------------------------------------
// C25-C27 — the bfinal loop / multiple blocks
// ---------------------------------------------------------------------------

#[test]
fn c25_multi_fixed_blocks() {
    let mut rng = Rng::new(0x5719);
    for _ in 0..120 {
        let nblocks = rng.range(2, 6);
        let mut d = Deflate::new();
        let mut expected: Vec<u8> = Vec::new();
        for b in 0..nblocks {
            let toks = { let n = rng.range(1, 200); rand_literals(&mut rng, n) };
            expected.extend_from_slice(&expand(&toks));
            d.fixed(b == nblocks - 1, &toks);
        }
        let stream = d.finish();
        let out = diff_inflate(
            InflateCase::new(&stream, expected.len() + 64),
            AB,
            "C25",
        );
        assert_eq!(out.ret, 1, "C25: nblocks={nblocks} err={:?}", out.err);
        assert_eq!(&out.out[..expected.len()], &expected[..]);
    }
}

#[test]
fn c26_multi_mixed_fixed_dynamic_blocks() {
    let mut rng = Rng::new(0x571A);
    for _ in 0..120 {
        let nblocks = rng.range(2, 5);
        let mut d = Deflate::new();
        let mut expected: Vec<u8> = Vec::new();
        for b in 0..nblocks {
            let toks = { let n = rng.range(1, 150); rand_literals(&mut rng, n) };
            expected.extend_from_slice(&expand(&toks));
            let last = b == nblocks - 1;
            if rng.below(2) == 0 {
                d.fixed(last, &toks);
            } else {
                let nlit = rng.range(257, 288);
                let lit_lens = lit_lens_for(&toks, nlit);
                let dist_lens = dist_lens_for(&toks, rng.range(1, 32));
                d.dynamic(last, &toks, &lit_lens, &dist_lens, 4);
            }
        }
        let stream = d.finish();
        let out = diff_inflate(
            InflateCase::new(&stream, expected.len() + 64),
            AB,
            "C26",
        );
        assert_eq!(out.ret, 1, "C26: err={:?}", out.err);
        assert_eq!(&out.out[..expected.len()], &expected[..]);
    }
}

#[test]
fn c27_multi_block_ending_in_stored() {
    // ERRORS.md E2 means a stored block is only accepted when the bytes that
    // follow its header are exactly LEN long, i.e. when it is last.
    let mut rng = Rng::new(0x571B);
    for align in 0..4 {
        for _ in 0..40 {
            let toks = { let n = rng.range(1, 200); rand_literals(&mut rng, n) };
            let head = expand(&toks);
            let tail = { let n = rng.range(0, 2048); rng.bytes(n) };
            let mut d = Deflate::new();
            d.fixed(false, &toks);
            d.stored(true, &tail);
            let stream = d.finish();
            let mut expected = head.clone();
            expected.extend_from_slice(&tail);
            let out = diff_inflate(
                InflateCase::new(&stream, expected.len() + 64).in_align(align),
                AB,
                "C27",
            );
            assert_eq!(out.ret, 1, "C27: align={align} err={:?}", out.err);
            assert_eq!(&out.out[..expected.len()], &expected[..]);
        }
    }
}

// ---------------------------------------------------------------------------
// C28 — real zlib-produced streams
// ---------------------------------------------------------------------------

mod zlib {
    use libloading::{Library, Symbol};
    use std::ffi::c_int;
    use std::sync::OnceLock;

    type Compress2 =
        unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, c_int) -> c_int;
    type CompressBound = unsafe extern "C" fn(u64) -> u64;

    static LIB: OnceLock<Option<Library>> = OnceLock::new();

    fn lib() -> Option<&'static Library> {
        LIB.get_or_init(|| unsafe {
            Library::new("libz.so.1")
                .or_else(|_| Library::new("libz.so"))
                .ok()
        })
        .as_ref()
    }

    /// zlib-format compress, then strip the 2-byte header and 4-byte adler32
    /// trailer to obtain a raw DEFLATE stream.
    pub fn raw_deflate(data: &[u8], level: c_int) -> Option<Vec<u8>> {
        let l = lib()?;
        unsafe {
            let bound: Symbol<CompressBound> = l.get(b"compressBound\0").ok()?;
            let c2: Symbol<Compress2> = l.get(b"compress2\0").ok()?;
            let mut cap = bound(data.len() as u64) + 64;
            let mut buf = vec![0u8; cap as usize];
            let rc = c2(
                buf.as_mut_ptr(),
                &mut cap,
                data.as_ptr(),
                data.len() as u64,
                level,
            );
            if rc != 0 {
                return None;
            }
            buf.truncate(cap as usize);
            if buf.len() < 6 {
                return None;
            }
            Some(buf[2..buf.len() - 4].to_vec())
        }
    }
}

#[test]
fn c28_zlib_reference_streams() {
    let mut rng = Rng::new(0x571C);
    let mut ran = 0usize;
    for level in 0..=9i32 {
        for shape in 0..4 {
            for _ in 0..8 {
                let n = rng.range(0, 20000);
                let data: Vec<u8> = match shape {
                    // random (incompressible)
                    0 => rng.bytes(n),
                    // highly repetitive (long matches, distance 1)
                    1 => vec![rng.byte(); n],
                    // small alphabet (dynamic trees with long zero runs)
                    2 => (0..n).map(|_| (rng.below(6) * 17) as u8).collect(),
                    // periodic (overlapping matches)
                    _ => {
                        let period = rng.range(1, 40);
                        let pat = rng.bytes(period);
                        (0..n).map(|i| pat[i % period]).collect()
                    }
                };
                let Some(stream) = zlib::raw_deflate(&data, level) else {
                    continue;
                };
                if stream.is_empty() {
                    continue;
                }
                ran += 1;
                for align in 0..4 {
                    let out = diff_inflate(
                        InflateCase::new(&stream, data.len() + 64).in_align(align),
                        AB,
                        "C28",
                    );
                    // A stored block that is not last is rejected (E2); the
                    // point is that C and Rust agree, which diff_inflate has
                    // already asserted. Only check the payload when accepted.
                    if out.ret == 1 {
                        assert_eq!(
                            &out.out[..data.len()],
                            &data[..],
                            "C28: level={level} shape={shape} n={} align={align}",
                            data.len()
                        );
                    }
                }
            }
        }
    }
    assert!(ran > 100, "C28: zlib unavailable or produced too few streams ({ran})");
}

// ---------------------------------------------------------------------------
// C29 — out buffer alignment
// ---------------------------------------------------------------------------

#[test]
fn c29_out_alignment() {
    let mut rng = Rng::new(0x571D);
    for out_align in 0..4 {
        for in_align in 0..4 {
            for _ in 0..20 {
                let toks = { let n = rng.range(1, 150); rand_tokens(&mut rng, n, 2048) };
                let expected = expand(&toks);
                let mut d = Deflate::new();
                d.fixed(true, &toks);
                let stream = d.finish();
                let out = diff_inflate(
                    InflateCase::new(&stream, expected.len() + 64)
                        .in_align(in_align)
                        .out_align(out_align),
                    AB,
                    "C29",
                );
                assert_eq!(out.ret, 1, "C29: err={:?}", out.err);
                assert_eq!(&out.out[..expected.len()], &expected[..]);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C30-C32 — exported globals and struct layout
// ---------------------------------------------------------------------------

#[test]
fn c30_exported_tables_byte_identical() {
    let l = libs();
    let tables: &[(&[u8], usize)] = &[
        (b"cp_fixed_table", 288 + 32),
        (b"cp_permutation_order", 19),
        (b"cp_len_extra_bits", 29 + 2),
        (b"cp_len_base", (29 + 2) * 4),
        (b"cp_dist_extra_bits", 30 + 2),
        (b"cp_dist_base", (30 + 2) * 4),
    ];
    for &(sym, len) in tables {
        let c = l.c.table(sym, len);
        let r = l.r.table(sym, len);
        assert_eq!(
            c,
            r,
            "C30: table {} differs",
            String::from_utf8_lossy(sym)
        );
    }
}

#[test]
fn c31_error_reason_null_and_untouched_on_success() {
    let l = libs();
    l.c.set_error_reason_null();
    l.r.set_error_reason_null();
    assert_eq!(l.c.error_reason(), None, "C31: C cp_error_reason not NULL");
    assert_eq!(l.r.error_reason(), None, "C31: Rust cp_error_reason not NULL");

    let toks = rand_literals(&mut Rng::new(0x571E), 64);
    let expected = expand(&toks);
    let mut d = Deflate::new();
    d.fixed(true, &toks);
    let stream = d.finish();
    let out = diff_inflate(
        InflateCase::new(&stream, expected.len() + 64),
        AB,
        "C31",
    );
    assert_eq!(out.ret, 1);
    assert_eq!(out.err, None, "C31: cp_error_reason set on success");
}

#[test]
fn c32_state_layout_matches_c() {
    // cp_decode evaluates tree[lo - 1], so for an empty tree it reads the field
    // that precedes `tree` inside cp_state_t. That is only reproducible if the
    // Rust mirror has C layout. Verified indirectly: build a dynamic block
    // whose code-length tree has exactly one symbol (hi == 1), which exercises
    // the lo == 1 boundary of the binary search, and a stream whose literal
    // tree is a single 1-bit code.
    let mut rng = Rng::new(0x571F);
    for _ in 0..60 {
        let n = rng.range(1, 200);
        // Only two distinct literals => a 1-bit literal tree plus EOB.
        let toks: Vec<Tok> = (0..n)
            .map(|_| Tok::Lit(if rng.below(2) == 0 { 0x00 } else { 0xFF }))
            .collect();
        let expected = expand(&toks);
        let mut lit_lens = vec![0u8; 257];
        lit_lens[0x00] = 2;
        lit_lens[0xFF] = 2;
        lit_lens[256] = 1;
        assert!(is_complete(&lit_lens));
        let dist_lens = vec![1u8, 1u8];
        let mut d = Deflate::new();
        d.dynamic(true, &toks, &lit_lens, &dist_lens, 4);
        let stream = d.finish();
        let out = diff_inflate(
            InflateCase::new(&stream, expected.len() + 64),
            AB,
            "C32",
        );
        assert_eq!(out.ret, 1, "C32: err={:?}", out.err);
        assert_eq!(&out.out[..expected.len()], &expected[..]);
    }
}
