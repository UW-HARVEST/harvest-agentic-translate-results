//! Phase B — valid-path differential tests for `cp_inflate`.
//! Covers CONFIGS.md rows C13..C31 and C37.

mod harness;

use harness::deflate::*;
use harness::{diff, diff_inflate_expect, pair, run, Pair, Rng, OFF_IN, OFF_OUT};

// ---------------------------------------------------------------------------
// C13..C15 — stored (btype = 0) blocks.
//
// Two C quirks shape these rows:
//
//  1. `cp_stored` rejects when `bits_left / 8 > LEN`, i.e. a stored block must be
//     the *last* thing in the input. So a valid stored block ends exactly at the
//     end of the input, and varying LEN is also how the `last_bytes` axis (C15)
//     gets driven.
//
//  2. `cp_ptr` computes the source address as
//     `(char*)(words + word_index) - count / 8`, which is only correct while
//     `count` counts whole buffered bytes. When the tail path in `cp_peak_bits`
//     runs (`count += s->bits_left`, *not* `+= last_bytes * 8`) `count`
//     over-counts, and the C then `memcpy`s from the wrong offset. That is
//     ground truth, so these rows are compared differentially only; a separate
//     row asserts the payload for the word-aligned shapes where the C is right.
// ---------------------------------------------------------------------------

/// True when `cp_inflate` will never take the `final_word` tail path, i.e. when
/// `(in_bytes - first_bytes) % 4 == 0`.
fn word_aligned(in_len: usize, in_align: usize) -> bool {
    let first_bytes = (4 - ((OFF_IN + in_align) & 3)) & 3;
    in_len >= first_bytes && (in_len - first_bytes) % 4 == 0
}

#[test]
fn c13_c15_stored_blocks() {
    let p = pair();
    let mut rng = Rng::new(0x5707_ED00_0000_0001);
    let mut checked_payload = 0usize;
    for len in [0usize, 1, 2, 3, 4, 5, 6, 7, 8, 16, 17, 255, 256, 257, 1000] {
        for in_align in 0..4usize {
            for rep in 0..4 {
                let payload = rng.bytes(len);
                let mut e = Enc::new();
                e.stored_block(true, &payload);
                let input = e.finish();
                let tag = format!("C13/len={len},align={in_align},rep={rep}");
                if word_aligned(input.len(), in_align) {
                    // the C's cp_ptr arithmetic is correct here: assert the payload
                    diff_inflate_expect(&p, &tag, &input, in_align, (len + 32) as i32, &payload);
                    checked_payload += 1;
                } else {
                    harness::diff_inflate(
                        &p,
                        &tag,
                        &input,
                        in_align,
                        input.len() as i32,
                        (len + 32) as i32,
                        len + 64,
                    );
                }
            }
        }
    }
    assert!(
        checked_payload >= 40,
        "expected many word-aligned stored blocks to be payload-checked, got {checked_payload}"
    );
}

/// C14/C15 explicitly: prove all four `first_bytes` values and all four
/// `last_bytes` values are actually reached by the stored-block corpus.
#[test]
fn c14_c15_alignment_and_tail_coverage() {
    let p = pair();
    let mut rng = Rng::new(0x5707_ED00_0000_0002);
    let mut seen_first = [false; 4];
    let mut seen_last = [false; 4];
    for len in 0usize..16 {
        for in_align in 0..4usize {
            let payload = rng.bytes(len);
            let mut e = Enc::new();
            e.stored_block(true, &payload);
            let input = e.finish();
            let first_bytes = (4 - ((OFF_IN + in_align) & 3)) & 3;
            let last_bytes = (input.len() - first_bytes) & 3;
            seen_first[first_bytes] = true;
            seen_last[last_bytes] = true;
            let tag = format!("C14/len={len},align={in_align},fb={first_bytes},lb={last_bytes}");
            if last_bytes == 0 {
                diff_inflate_expect(&p, &tag, &input, in_align, (len + 32) as i32, &payload);
            } else {
                harness::diff_inflate(
                    &p,
                    &tag,
                    &input,
                    in_align,
                    input.len() as i32,
                    (len + 32) as i32,
                    len + 64,
                );
            }
        }
    }
    assert_eq!(seen_first, [true; 4], "first_bytes axis not fully covered");
    assert_eq!(seen_last, [true; 4], "last_bytes axis not fully covered");
}

// ---------------------------------------------------------------------------
// C16 — fixed Huffman, literals only.
// ---------------------------------------------------------------------------

#[test]
fn c16_fixed_literals_only() {
    let p = pair();
    let mut rng = Rng::new(0xF1_2ED0_0000_0001);
    for n in [1usize, 2, 3, 7, 8, 9, 15, 16, 31, 64, 143, 144, 255, 256, 300] {
        for rep in 0..6 {
            let toks: Vec<Tok> = (0..n).map(|_| Tok::Lit(rng.u8())).collect();
            let expect = expand(&toks);
            let mut e = Enc::new();
            e.fixed_block(true, &toks);
            let input = e.finish();
            diff_inflate_expect(
                &p,
                &format!("C16/n={n},rep={rep}"),
                &input,
                0,
                expect.len() as i32,
                &expect,
            );
        }
    }
    // literal boundary symbols: 0..143 use 8-bit codes, 144..255 use 9-bit codes
    for boundary in [[0u8, 143, 144, 255], [143, 144, 143, 144], [255, 0, 255, 0]] {
        let toks: Vec<Tok> = boundary.iter().map(|&b| Tok::Lit(b)).collect();
        let expect = expand(&toks);
        let mut e = Enc::new();
        e.fixed_block(true, &toks);
        let input = e.finish();
        diff_inflate_expect(
            &p,
            &format!("C16/boundary={boundary:?}"),
            &input,
            0,
            expect.len() as i32,
            &expect,
        );
    }
}

// ---------------------------------------------------------------------------
// C17 — distance == 1 (the `memset` fast path in cp_block).
// ---------------------------------------------------------------------------

#[test]
fn c17_fixed_distance_one_memset_path() {
    let p = pair();
    let mut rng = Rng::new(0xD157_0001_0000_0001);
    for len in [3u32, 4, 5, 16, 17, 100, 257, 258] {
        for rep in 0..6 {
            let mut toks = vec![Tok::Lit(rng.u8())];
            toks.push(Tok::Match(len, 1));
            // a second run so the path is taken more than once
            toks.push(Tok::Lit(rng.u8()));
            toks.push(Tok::Match(len.min(50), 1));
            let expect = expand(&toks);
            let mut e = Enc::new();
            e.fixed_block(true, &toks);
            let input = e.finish();
            diff_inflate_expect(
                &p,
                &format!("C17/len={len},rep={rep}"),
                &input,
                rep % 4,
                expect.len() as i32,
                &expect,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C18 — overlapping matches (distance > 1 but < length): the C copies
// byte-at-a-time so the match self-extends.
// ---------------------------------------------------------------------------

#[test]
fn c18_fixed_overlapping_matches() {
    let p = pair();
    let mut rng = Rng::new(0x0EEE_1AB0_0000_0018);
    for dist in [2u32, 3, 4, 5, 7, 11, 32] {
        for len in [3u32, 4, 5, 8, 13, 100, 258] {
            for rep in 0..3 {
                let mut toks: Vec<Tok> = (0..dist).map(|_| Tok::Lit(rng.u8())).collect();
                toks.push(Tok::Match(len, dist));
                toks.push(Tok::Match(len, dist));
                let expect = expand(&toks);
                let mut e = Enc::new();
                e.fixed_block(true, &toks);
                let input = e.finish();
                diff_inflate_expect(
                    &p,
                    &format!("C18/dist={dist},len={len},rep={rep}"),
                    &input,
                    rep,
                    expect.len() as i32,
                    &expect,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C19 — every length symbol 257..=285 crossed with every distance symbol
// 0..=29, including both the minimum and maximum extra-bit payload.
// ---------------------------------------------------------------------------

#[test]
fn c19_all_length_and_distance_symbols() {
    let p = pair();
    let mut rng = Rng::new(0x5EEE_5EEE_0000_0001);
    for len_sym in 0usize..=28 {
        let nx = LEN_EXTRA[len_sym] as u32;
        let extras: Vec<u32> = if nx == 0 {
            vec![0]
        } else {
            vec![0, (1u32 << nx) - 1, 1]
        };
        for &len_extra in &extras {
            let length = LEN_BASE[len_sym] + len_extra;
            if length > 258 {
                continue; // not encodable, the C would emit past `cp_len_base`
            }
            for dist_sym in 0usize..=29 {
                let dnx = DIST_EXTRA[dist_sym] as u32;
                let dextras: Vec<u32> = if dnx == 0 {
                    vec![0]
                } else {
                    vec![0, (1u32 << dnx) - 1]
                };
                for &dist_extra in &dextras {
                    let dist = DIST_BASE[dist_sym] + dist_extra;
                    if dist > 20000 {
                        continue; // keep the bitstreams a sane size
                    }
                    let mut toks: Vec<Tok> = (0..dist).map(|_| Tok::Lit(rng.u8())).collect();
                    toks.push(Tok::RawMatch {
                        len_sym,
                        len_extra,
                        dist_sym,
                        dist_extra,
                    });
                    let expect = expand(&toks);
                    let mut e = Enc::new();
                    e.fixed_block(true, &toks);
                    let input = e.finish();
                    diff_inflate_expect(
                        &p,
                        &format!(
                            "C19/ls={len_sym}+{len_extra},ds={dist_sym}+{dist_extra}"
                        ),
                        &input,
                        0,
                        expect.len() as i32,
                        &expect,
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C20 — fixed block x input alignment x input tail (all 16 combinations).
// ---------------------------------------------------------------------------

#[test]
fn c20_fixed_alignment_x_tail() {
    let p = pair();
    let mut rng = Rng::new(0xA116_0000_0000_0001);
    let mut seen = [[false; 4]; 4];
    for in_align in 0..4usize {
        for pad in 0..4usize {
            for rep in 0..6 {
                let toks = random_toks(&mut rng, 40, true, 64);
                let expect = expand(&toks);
                let mut e = Enc::new();
                e.fixed_block(true, &toks);
                let mut input = e.finish();
                for _ in 0..pad {
                    input.push(rng.u8());
                }
                let first_bytes = (4 - ((OFF_IN + in_align) & 3)) & 3;
                let last_bytes = (input.len() - first_bytes) & 3;
                seen[first_bytes][last_bytes] = true;
                diff_inflate_expect(
                    &p,
                    &format!("C20/align={in_align},pad={pad},rep={rep}"),
                    &input,
                    in_align,
                    expect.len() as i32,
                    &expect,
                );
            }
        }
    }
    for fb in 0..4 {
        assert!(
            seen[fb].iter().any(|&x| x),
            "first_bytes={fb} never reached"
        );
    }
    let mut lb_seen = [false; 4];
    for fb in 0..4 {
        for lb in 0..4 {
            if seen[fb][lb] {
                lb_seen[lb] = true;
            }
        }
    }
    assert_eq!(lb_seen, [true; 4], "last_bytes axis not fully covered");
}

// ---------------------------------------------------------------------------
// C21..C24 — dynamic blocks, one row per code-length RLE mode.
// ---------------------------------------------------------------------------

fn dyn_case(p: &Pair, tag: &str, rng: &mut Rng, hlit: usize, hdist: usize, mode: RleMode, n: usize, matches: bool) {
    let (max_len, max_dist) = limits(hlit, hdist);
    // hlit == 257 covers only symbols 0..=256, so no length symbol exists at all
    let matches = matches && hlit >= 258;
    let toks = random_toks_bounded(rng, n, matches, max_dist.min(200), max_len);
    let (lit_lens, dist_lens) = alphabets_for(&toks, hlit, hdist);
    let expect = expand(&toks);
    let mut e = Enc::new();
    e.dynamic_block(true, &lit_lens, &dist_lens, &toks, mode);
    let input = e.finish();
    diff_inflate_expect(p, tag, &input, 0, expect.len() as i32, &expect);
}

#[test]
fn c21_dynamic_minimal_hclen_literals_only() {
    let p = pair();
    let mut rng = Rng::new(0xD1_0000_0000_0001);
    for rep in 0..40 {
        dyn_case(
            &p,
            &format!("C21/rep={rep}"),
            &mut rng,
            257,
            1,
            RleMode::None,
            1 + rep,
            false,
        );
    }
}

#[test]
fn c22_dynamic_cl_symbol_16_repeat() {
    let p = pair();
    let mut rng = Rng::new(0xD1_0000_0000_0016);
    for rep in 0..40 {
        dyn_case(
            &p,
            &format!("C22/rep={rep}"),
            &mut rng,
            288,
            32,
            RleMode::Rep16,
            5 + rep * 3,
            true,
        );
    }
}

#[test]
fn c23_dynamic_cl_symbol_17_short_zero_run() {
    let p = pair();
    let mut rng = Rng::new(0xD1_0000_0000_0017);
    for rep in 0..40 {
        dyn_case(
            &p,
            &format!("C23/rep={rep}"),
            &mut rng,
            270,
            8,
            RleMode::Zero17,
            5 + rep * 3,
            true,
        );
    }
}

#[test]
fn c24_dynamic_cl_symbol_18_long_zero_run() {
    let p = pair();
    let mut rng = Rng::new(0xD1_0000_0000_0018);
    for rep in 0..40 {
        dyn_case(
            &p,
            &format!("C24/rep={rep}"),
            &mut rng,
            288,
            32,
            RleMode::Zero18,
            5 + rep * 3,
            true,
        );
    }
    // and the "shortest encoding" mode, which mixes 16/17/18
    for rep in 0..40 {
        dyn_case(
            &p,
            &format!("C24-all/rep={rep}"),
            &mut rng,
            288,
            32,
            RleMode::All,
            5 + rep * 3,
            true,
        );
    }
}

// ---------------------------------------------------------------------------
// C25 — HLIT x HDIST sweep.
// ---------------------------------------------------------------------------

#[test]
fn c25_dynamic_hlit_hdist_sweep() {
    let p = pair();
    let mut rng = Rng::new(0xD1_0000_0000_0025);
    for hlit in [257usize, 258, 270, 287, 288] {
        for hdist in [1usize, 2, 3, 17, 31, 32] {
            for mode in [RleMode::None, RleMode::All] {
                for rep in 0..3 {
                    dyn_case(
                        &p,
                        &format!("C25/hlit={hlit},hdist={hdist},mode={mode:?},rep={rep}"),
                        &mut rng,
                        hlit,
                        hdist,
                        mode,
                        20,
                        hdist >= 2,
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C26 — dynamic blocks with matches (memset path and overlapping path).
// ---------------------------------------------------------------------------

#[test]
fn c26_dynamic_with_matches() {
    let p = pair();
    let mut rng = Rng::new(0xD1_0000_0000_0026);
    for dist in [1u32, 2, 3, 5, 17, 200] {
        for len in [3u32, 8, 100, 258] {
            for rep in 0..3 {
                let mut toks: Vec<Tok> = (0..dist).map(|_| Tok::Lit(rng.u8())).collect();
                toks.push(Tok::Match(len, dist));
                toks.push(Tok::Lit(rng.u8()));
                toks.push(Tok::Match(len, dist));
                let (lit_lens, dist_lens) = alphabets_for(&toks, 288, 32);
                let expect = expand(&toks);
                let mut e = Enc::new();
                e.dynamic_block(true, &lit_lens, &dist_lens, &toks, RleMode::All);
                let input = e.finish();
                diff_inflate_expect(
                    &p,
                    &format!("C26/dist={dist},len={len},rep={rep}"),
                    &input,
                    rep,
                    expect.len() as i32,
                    &expect,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C27 — dynamic block x input alignment x tail.
// ---------------------------------------------------------------------------

#[test]
fn c27_dynamic_alignment_x_tail() {
    let p = pair();
    let mut rng = Rng::new(0xD1_0000_0000_0027);
    for in_align in 0..4usize {
        for pad in 0..4usize {
            for rep in 0..4 {
                let toks = random_toks(&mut rng, 30, true, 64);
                let (lit_lens, dist_lens) = alphabets_for(&toks, 288, 32);
                let expect = expand(&toks);
                let mut e = Enc::new();
                e.dynamic_block(true, &lit_lens, &dist_lens, &toks, RleMode::All);
                let mut input = e.finish();
                for _ in 0..pad {
                    input.push(rng.u8());
                }
                diff_inflate_expect(
                    &p,
                    &format!("C27/align={in_align},pad={pad},rep={rep}"),
                    &input,
                    in_align,
                    expect.len() as i32,
                    &expect,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C28 — multi-block streams (`bfinal == 0` chains).
//
// A *stored* block can only be last, because `cp_stored` rejects when
// `bits_left / 8 > LEN`.
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
enum Kind {
    Fixed,
    Dynamic,
    Stored,
}

fn build_chain(rng: &mut Rng, kinds: &[Kind], per_block: usize) -> (Vec<u8>, Vec<u8>) {
    let mut e = Enc::new();
    let mut expect: Vec<u8> = Vec::new();
    for (i, k) in kinds.iter().enumerate() {
        let last = i + 1 == kinds.len();
        match k {
            Kind::Stored => {
                let payload = rng.bytes(per_block);
                e.stored_block(last, &payload);
                expect.extend_from_slice(&payload);
            }
            Kind::Fixed => {
                let toks = random_toks(rng, per_block, !expect.is_empty(), 64);
                expect.extend_from_slice(&expand(&toks));
                e.fixed_block(last, &toks);
            }
            Kind::Dynamic => {
                let toks = random_toks(rng, per_block, !expect.is_empty(), 64);
                expect.extend_from_slice(&expand(&toks));
                let (l, d) = alphabets_for(&toks, 288, 32);
                e.dynamic_block(last, &l, &d, &toks, RleMode::All);
            }
        }
    }
    (e.finish(), expect)
}

#[test]
fn c28_multi_block_chains() {
    let p = pair();
    let mut rng = Rng::new(0xC0DE_C0DE_0000_0028);
    let chains: &[&[Kind]] = &[
        &[Kind::Fixed, Kind::Fixed],
        &[Kind::Fixed, Kind::Dynamic],
        &[Kind::Dynamic, Kind::Fixed],
        &[Kind::Dynamic, Kind::Dynamic],
        &[Kind::Fixed, Kind::Stored],
        &[Kind::Dynamic, Kind::Stored],
        &[Kind::Dynamic, Kind::Fixed, Kind::Fixed],
        &[Kind::Fixed, Kind::Dynamic, Kind::Fixed, Kind::Stored],
        &[Kind::Fixed, Kind::Fixed, Kind::Fixed, Kind::Fixed, Kind::Fixed],
    ];
    for (ci, chain) in chains.iter().enumerate() {
        for per in [1usize, 3, 20] {
            for rep in 0..4 {
                let (input, expect) = build_chain(&mut rng, chain, per);
                diff_inflate_expect(
                    &p,
                    &format!("C28/chain#{ci}={chain:?},per={per},rep={rep}"),
                    &input,
                    0,
                    expect.len() as i32,
                    &expect,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C29 — matches that reach back *across* a block boundary. `s->begin` is the
// start of the whole output buffer, not of the current block, so this is legal.
// ---------------------------------------------------------------------------

#[test]
fn c29_cross_block_matches() {
    let p = pair();
    let mut rng = Rng::new(0xC0DE_C0DE_0000_0029);
    for first_n in [1usize, 5, 40] {
        for dist_back in [1u32, 2, 5] {
            for rep in 0..4 {
                let mut e = Enc::new();
                let first: Vec<Tok> = (0..first_n).map(|_| Tok::Lit(rng.u8())).collect();
                let mut expect = expand(&first);
                e.fixed_block(false, &first);

                let dist = dist_back.min(expect.len() as u32).max(1);
                // second block starts with a match reaching into block 1's output
                let second = vec![
                    Tok::Match(3 + rng.below(20) as u32, dist),
                    Tok::Lit(rng.u8()),
                ];
                expect.extend_from_slice(&expand_with_prefix(&expect, &second));
                let (l, d) = alphabets_for(&second, 288, 32);
                e.dynamic_block(true, &l, &d, &second, RleMode::All);
                let input = e.finish();
                diff_inflate_expect(
                    &p,
                    &format!("C29/first_n={first_n},dist={dist},rep={rep}"),
                    &input,
                    0,
                    expect.len() as i32,
                    &expect,
                );
            }
        }
    }
}

/// Expand `toks` given an already-produced `prefix`, returning only the newly
/// produced bytes.
fn expand_with_prefix(prefix: &[u8], toks: &[Tok]) -> Vec<u8> {
    let mut out = prefix.to_vec();
    let base = out.len();
    for t in toks {
        match *t {
            Tok::Lit(b) => out.push(b),
            Tok::Match(len, dist) => {
                let start = out.len() - dist as usize;
                for i in 0..len as usize {
                    let b = out[start + i];
                    out.push(b);
                }
            }
            Tok::RawMatch {
                len_sym,
                len_extra,
                dist_sym,
                dist_extra,
            } => {
                let len = LEN_BASE[len_sym] + len_extra;
                let dist = DIST_BASE[dist_sym] + dist_extra;
                let start = out.len() - dist as usize;
                for i in 0..len as usize {
                    let b = out[start + i];
                    out.push(b);
                }
            }
        }
    }
    out[base..].to_vec()
}

// ---------------------------------------------------------------------------
// C30 — output buffer with slack; the untouched tail must match too (the
// snapshot in `diff_inflate_expect` covers out_bytes + 64).
// ---------------------------------------------------------------------------

#[test]
fn c30_output_slack() {
    let p = pair();
    let mut rng = Rng::new(0x51AC_0000_0000_0030);
    for slack in [0i32, 1, 2, 7, 64, 1000] {
        for rep in 0..8 {
            let toks = random_toks(&mut rng, 30, true, 32);
            let expect = expand(&toks);
            let mut e = Enc::new();
            e.fixed_block(true, &toks);
            let input = e.finish();
            diff_inflate_expect(
                &p,
                &format!("C30/slack={slack},rep={rep}"),
                &input,
                0,
                expect.len() as i32 + slack,
                &expect,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C31 — the `if (s && len <= 9)` boundary in cp_build: code lengths 9 fill the
// `s->lookup` table, lengths >= 10 skip it.
// ---------------------------------------------------------------------------

#[test]
fn c31_code_length_9_and_10_boundary() {
    let p = pair();
    let mut rng = Rng::new(0x9A9A_0000_0000_0031);
    // Kraft-complete: 1/2 + 1/4 + ... + 1/512 + 1/1024 + 1/1024 == 1
    let ladder: [(usize, u8); 11] = [
        (65, 1),
        (66, 2),
        (67, 3),
        (68, 4),
        (69, 5),
        (70, 6),
        (71, 7),
        (72, 8),
        (73, 9),
        (74, 10),
        (256, 10),
    ];
    let mut lit_lens = vec![0u8; 257];
    for (s, l) in ladder {
        lit_lens[s] = l;
    }
    assert!(is_complete(&lit_lens), "hand-built ladder must be complete");
    let dist_lens = vec![1u8, 1];

    for rep in 0..30 {
        let n = 1 + rng.below(60);
        let toks: Vec<Tok> = (0..n)
            .map(|_| Tok::Lit(rng.pick(&[65u8, 66, 67, 68, 69, 70, 71, 72, 73, 74])))
            .collect();
        let expect = expand(&toks);
        let mut e = Enc::new();
        e.dynamic_block(true, &lit_lens, &dist_lens, &toks, RleMode::All);
        let input = e.finish();
        diff_inflate_expect(
            &p,
            &format!("C31/rep={rep}"),
            &input,
            rep % 4,
            expect.len() as i32,
            &expect,
        );
    }

    // a deeper ladder whose maximum length is 15 (the largest cp_build accepts)
    let mut deep = vec![0u8; 257];
    for i in 0..14 {
        deep[100 + i] = (i + 1) as u8;
    }
    deep[200] = 15;
    deep[256] = 15;
    assert!(is_complete(&deep), "deep ladder must be complete");
    for rep in 0..10 {
        let n = 1 + rng.below(40);
        let toks: Vec<Tok> = (0..n)
            .map(|_| {
                let k = rng.below(15);
                Tok::Lit(if k < 14 { (100 + k) as u8 } else { 200 })
            })
            .collect();
        let expect = expand(&toks);
        let mut e = Enc::new();
        e.dynamic_block(true, &deep, &dist_lens, &toks, RleMode::All);
        let input = e.finish();
        diff_inflate_expect(
            &p,
            &format!("C31-deep/rep={rep}"),
            &input,
            0,
            expect.len() as i32,
            &expect,
        );
    }
}

// ---------------------------------------------------------------------------
// C37 — the real pipeline: cp_inflate then unfilter over the decoded bytes,
// both calls in the same child so state carries over exactly as it would for a
// real consumer.
// ---------------------------------------------------------------------------

#[test]
fn c37_inflate_then_unfilter_pipeline() {
    let p = pair();
    let sh = &p.shared;
    let mut rng = Rng::new(0x9177_0000_0000_0037);
    for w in [1i32, 4, 13, 40] {
        for h in [1i32, 2, 7] {
            for bpp in [1i32, 3, 4] {
                for rep in 0..4 {
                    let len = (w * bpp) as usize;
                    let total = h as usize * (len + 1);
                    // raw PNG-ish scanlines: filter byte then data
                    let mut raw = Vec::with_capacity(total);
                    for _ in 0..h {
                        raw.push(rng.below(5) as u8);
                        for _ in 0..len {
                            raw.push(rng.u8());
                        }
                    }
                    let toks: Vec<Tok> = raw.iter().map(|&b| Tok::Lit(b)).collect();
                    let mut e = Enc::new();
                    e.fixed_block(true, &toks);
                    let input = e.finish();
                    let snap = (OFF_OUT, total + 64);

                    let mut outs = Vec::new();
                    for lib in [&p.c, &p.rust] {
                        sh.fill_pattern();
                        sh.write(OFF_IN, &input);
                        outs.push(run(sh, lib, snap, |l| unsafe {
                            let r = (l.cp_inflate)(
                                sh.in_ptr(0) as *mut _,
                                input.len() as i32,
                                sh.out_ptr(0) as *mut _,
                                total as i32,
                            );
                            if r != 1 {
                                return -1;
                            }
                            (l.unfilter)(w, h, bpp, sh.out_ptr(0))
                        }));
                    }
                    diff(
                        &format!("C37/w={w},h={h},bpp={bpp},rep={rep}"),
                        &outs[0],
                        &outs[1],
                    );
                    assert!(outs[0].completed && outs[0].ret == 1, "{:?}", outs[0]);
                }
            }
        }
    }
}
