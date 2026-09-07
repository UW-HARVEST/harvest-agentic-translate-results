//! Phase B — valid-path differential tests for the LOW-LEVEL entry point
//! `cp_inflate`. Covers CONFIGS.md rows 1..21.

mod common;

use common::deflate::*;
use common::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

/// Verifies the test-vector generator itself: the C implementation must accept
/// the stream. Returns what C produced. Without this, a bug in the generator
/// could make C and Rust "agree" on nothing at all.
fn require_accept(label: &str, stream: &[u8], cap: usize) -> Vec<u8> {
    let cap = cap.max(1);
    let r = c_inflate(stream, stream.len() as i32, cap as i32, cap);
    assert_eq!(
        r.ret, 1,
        "[{label}] C rejected our generated stream: {:?}",
        r.error
    );
    r.out
}

/// Strict: C must accept AND decode to exactly `expect`, then Rust must match C.
fn ok(label: &str, stream: &[u8], expect: &[u8]) {
    let got = require_accept(label, stream, expect.len().max(1));
    assert_eq!(
        &got[..expect.len()],
        expect,
        "[{label}] C decoded to unexpected plaintext"
    );
    diff_inflate(label, stream, expect.len().max(1));
}

/// Loose: C must accept, and Rust must match C byte for byte. Used where the
/// C's own bit accounting makes the produced bytes something other than the
/// plaintext we encoded (see `cp_stored`/`cp_ptr`); the C is ground truth, so
/// the requirement is agreement, not "correctness".
fn ok_loose(label: &str, stream: &[u8], cap: usize) {
    let got = require_accept(label, stream, cap);
    assert!(!got.is_empty() || cap == 0);
    diff_inflate(label, stream, cap.max(1));
}

// --- row 1/3: stored blocks -------------------------------------------------

#[test]
fn row01_stored_aligned() {
    for n in [1usize, 2, 3, 4, 5, 7, 8, 15, 16, 17, 255, 256, 1000] {
        let data: Vec<u8> = (0..n).map(|i| (i * 7 + 3) as u8).collect();
        let s = deflate_stored(&data);
        ok_loose(&format!("row01 stored len={n}"), &s, data.len().max(1));
    }
}

#[test]
fn row02_stored_all_input_alignments() {
    for n in [0usize, 1, 3, 4, 5, 6, 7, 8, 9, 100] {
        let data: Vec<u8> = (0..n).map(|i| (i as u8) ^ 0x5A).collect();
        let s = deflate_stored(&data);
        for align in 0..4 {
            let label = format!("row02 stored len={n} align={align}");
            // C first, to confirm the vector is valid at this alignment
            let cap = n.max(1);
            let r = c_inflate_align(&s, s.len() as i32, align, cap as i32, cap);
            assert_eq!(r.ret, 1, "[{label}] C rejected: {:?}", r.error);
            diff_inflate_align(&label, &s, align, cap);
        }
    }
}

#[test]
fn row03_stored_empty() {
    let s = deflate_stored(&[]);
    ok_loose("row03 stored empty", &s, 1);
    // and with a spare output buffer
    diff_inflate_full("row03 stored empty spare out", &s, s.len() as i32, 0, 64, 64);
}

// --- rows 4..10: fixed Huffman ---------------------------------------------

#[test]
fn row04_fixed_literals_only() {
    for n in [1usize, 2, 3, 7, 8, 100, 1000] {
        let data: Vec<u8> = (0..n).map(|i| (i * 31 + 11) as u8).collect();
        let s = deflate_fixed_literals(&data);
        ok(&format!("row04 fixed lit n={n}"), &s, &data);
    }
    // every possible literal byte value
    let all: Vec<u8> = (0..=255u8).collect();
    let s = deflate_fixed_literals(&all);
    ok("row04 fixed all 256 literals", &s, &all);
}

#[test]
fn row05_fixed_match_distance_one() {
    // distance == 1 hits the memset() fast path in cp_block
    for len in [3u32, 4, 5, 16, 17, 100, 257, 258] {
        let toks = vec![Tok::Lit(0xAB), Tok::Match(len, 1)];
        let expect = apply(&toks);
        let mut bw = BitWriter::new();
        write_fixed_block(&mut bw, &toks, true);
        let s = bw.finish();
        ok(&format!("row05 dist=1 len={len}"), &s, &expect);
    }
}

#[test]
fn row06_fixed_overlapping_copy() {
    // distance < length: the byte-at-a-time loop must read bytes it just wrote
    for (len, dist) in [(6u32, 2u32), (9, 3), (10, 4), (258, 5), (100, 7), (258, 257)] {
        let mut toks: Vec<Tok> = (0..dist).map(|i| Tok::Lit((i as u8).wrapping_mul(37) | 1)).collect();
        toks.push(Tok::Match(len, dist));
        let expect = apply(&toks);
        let mut bw = BitWriter::new();
        write_fixed_block(&mut bw, &toks, true);
        let s = bw.finish();
        ok(&format!("row06 overlap len={len} dist={dist}"), &s, &expect);
    }
}

#[test]
fn row07_fixed_non_overlapping_copy() {
    for (len, dist) in [(3u32, 3u32), (3, 10), (8, 8), (20, 40), (258, 258)] {
        let mut toks: Vec<Tok> = (0..dist).map(|i| Tok::Lit(i as u8)).collect();
        toks.push(Tok::Match(len, dist));
        let expect = apply(&toks);
        let mut bw = BitWriter::new();
        write_fixed_block(&mut bw, &toks, true);
        let s = bw.finish();
        ok(&format!("row07 nonoverlap len={len} dist={dist}"), &s, &expect);
    }
}

#[test]
fn row08_fixed_nine_bit_literals() {
    // literals 144..255 use the 9-bit half of cp_fixed_table
    let data: Vec<u8> = (144..=255u8).collect();
    let s = deflate_fixed_literals(&data);
    ok("row08 fixed 9-bit literals", &s, &data);
    // interleave 8-bit and 9-bit literals
    let mut mix = Vec::new();
    for i in 0..200 {
        mix.push(if i % 2 == 0 { (i % 144) as u8 } else { 144 + (i % 112) as u8 });
    }
    let s = deflate_fixed_literals(&mix);
    ok("row08 fixed mixed widths", &s, &mix);
}

#[test]
fn row09_fixed_length_symbol_boundaries() {
    // every length symbol, plus the extremes of every extra-bit range
    let mut lens: Vec<u32> = Vec::new();
    for i in 0..29usize {
        let base = LEN_BASE[i];
        let span = 1u32 << LEN_EXTRA[i];
        lens.push(base);
        if LEN_EXTRA[i] > 0 {
            lens.push(base + span - 1);
            lens.push(base + span / 2);
        }
    }
    lens.sort();
    lens.dedup();
    for len in lens {
        if len > 258 {
            continue;
        }
        let toks = vec![Tok::Lit(0x11), Tok::Lit(0x22), Tok::Lit(0x33), Tok::Match(len, 3)];
        let expect = apply(&toks);
        let mut bw = BitWriter::new();
        write_fixed_block(&mut bw, &toks, true);
        let s = bw.finish();
        ok(&format!("row09 len={len}"), &s, &expect);
    }
}

#[test]
fn row10_fixed_distance_symbol_boundaries() {
    for i in 0..30usize {
        for d in [DIST_BASE[i], DIST_BASE[i] + (1u32 << DIST_EXTRA[i]) - 1] {
            let dist = d as usize;
            let mut toks: Vec<Tok> = (0..dist).map(|k| Tok::Lit((k * 13 + 1) as u8)).collect();
            toks.push(Tok::Match(3, d));
            let expect = apply(&toks);
            let mut bw = BitWriter::new();
            write_fixed_block(&mut bw, &toks, true);
            let s = bw.finish();
            ok(&format!("row10 distsym={i} dist={d}"), &s, &expect);
        }
    }
}

// --- rows 11..16: dynamic Huffman -----------------------------------------

#[test]
fn row11_dynamic_literal_code_lengths_only() {
    for n in [1usize, 2, 10, 300] {
        let data: Vec<u8> = (0..n).map(|i| (i * 17) as u8).collect();
        let s = deflate_dynamic_literals(&data, ClMode::Literal);
        ok(&format!("row11 dyn literal-cl n={n}"), &s, &data);
    }
}

#[test]
fn row12_dynamic_cl_symbol_16() {
    // A run of >= 4 equal non-zero code lengths forces symbol 16.
    // 288 literals all present with equal frequency -> long runs of equal lens.
    let data: Vec<u8> = (0..=255u8).chain(0..=255u8).collect();
    let toks: Vec<Tok> = data.iter().map(|&b| Tok::Lit(b)).collect();
    let (ll, dl) = lens_for(&toks, 288, 32);
    // confirm the CL sequence really uses symbol 16
    assert!(has_cl_symbol(&ll, &dl, 16), "vector does not exercise CL symbol 16");
    let mut bw = BitWriter::new();
    write_dynamic_block(&mut bw, &toks, &ll, &dl, ClMode::Rle, None, true);
    let s = bw.finish();
    ok("row12 dyn cl16", &s, &data);
}

#[test]
fn row13_dynamic_cl_symbol_17() {
    // A short zero run (3..10). Use a literal alphabet with small gaps.
    let mut ll = vec![0u8; 257];
    // symbols 0,4,8,...  -> gaps of 3 zeros
    let used: Vec<usize> = (0..32).map(|i| i * 4).chain(std::iter::once(256)).collect();
    let bl = balanced_lens(&used, 257);
    ll.copy_from_slice(&bl);
    let dl = {
        let mut v = vec![0u8; 4];
        v[0] = 1;
        v[3] = 1;
        v
    };
    assert!(has_cl_symbol(&ll, &dl, 17), "vector does not exercise CL symbol 17");
    let data: Vec<u8> = used
        .iter()
        .filter(|&&s| s < 256)
        .map(|&s| s as u8)
        .cycle()
        .take(200)
        .collect();
    let toks: Vec<Tok> = data.iter().map(|&b| Tok::Lit(b)).collect();
    let mut bw = BitWriter::new();
    write_dynamic_block(&mut bw, &toks, &ll, &dl, ClMode::Rle, None, true);
    let s = bw.finish();
    ok("row13 dyn cl17", &s, &data);
}

#[test]
fn row14_dynamic_cl_symbol_18() {
    // A long zero run (11..138): only a handful of literals used out of 288.
    let used = vec![0usize, 1, 200, 256];
    let ll = balanced_lens(&used, 288);
    let mut dl = vec![0u8; 32];
    dl[0] = 1;
    dl[31] = 1;
    assert!(has_cl_symbol(&ll, &dl, 18), "vector does not exercise CL symbol 18");
    let data: Vec<u8> = [0u8, 1, 200, 1, 0, 200].iter().cycle().take(120).copied().collect();
    let toks: Vec<Tok> = data.iter().map(|&b| Tok::Lit(b)).collect();
    let mut bw = BitWriter::new();
    write_dynamic_block(&mut bw, &toks, &ll, &dl, ClMode::Rle, None, true);
    let s = bw.finish();
    ok("row14 dyn cl18", &s, &data);
}

#[test]
fn row15_dynamic_ndst_one() {
    let used = vec![0usize, 1, 2, 3, 256];
    let ll = balanced_lens(&used, 257);
    let dl = vec![1u8; 1]; // ndst == 1, never decoded (no matches)
    let data: Vec<u8> = [0u8, 1, 2, 3].iter().cycle().take(64).copied().collect();
    let toks: Vec<Tok> = data.iter().map(|&b| Tok::Lit(b)).collect();
    for mode in [ClMode::Literal, ClMode::Rle] {
        let mut bw = BitWriter::new();
        write_dynamic_block(&mut bw, &toks, &ll, &dl, mode, None, true);
        let s = bw.finish();
        ok("row15 dyn ndst=1", &s, &data);
    }
}

#[test]
fn row16_dynamic_maximum_header() {
    // nlit = 288, ndst = 32, nlen = 19
    let lused: Vec<usize> = (0..288).collect();
    let ll = balanced_lens(&lused, 288);
    let dused: Vec<usize> = (0..32).collect();
    let dl = balanced_lens(&dused, 32);
    let data: Vec<u8> = (0..300).map(|i| (i * 5) as u8).collect();
    let mut toks: Vec<Tok> = data.iter().map(|&b| Tok::Lit(b)).collect();
    toks.push(Tok::Match(4, 300));
    let expect = apply(&toks);
    for mode in [ClMode::Literal, ClMode::Rle] {
        let mut bw = BitWriter::new();
        write_dynamic_block(&mut bw, &toks, &ll, &dl, mode, Some(19), true);
        let s = bw.finish();
        ok("row16 dyn max header", &s, &expect);
    }
    // HCLEN sweep over every legal value >= the natural minimum
    for nlen in 4..=19usize {
        let mut bw = BitWriter::new();
        let cl_min = natural_hclen(&ll, &dl, ClMode::Rle);
        if nlen < cl_min {
            continue;
        }
        write_dynamic_block(&mut bw, &toks, &ll, &dl, ClMode::Rle, Some(nlen), true);
        let s = bw.finish();
        ok(&format!("row16 hclen={nlen}"), &s, &expect);
    }
}

// --- rows 17..20: multi-block and buffer shapes ----------------------------

#[test]
fn row17_multiblock_fixed_dynamic_stored() {
    // A stored block must be last (the C requires bits_left/8 <= LEN), so the
    // order is fixed -> dynamic -> stored(final).
    let a: Vec<u8> = (0..40u8).collect();
    let b: Vec<u8> = (40..90u8).collect();
    let c: Vec<u8> = (90..160u8).collect();
    let mut bw = BitWriter::new();
    write_fixed_block(&mut bw, &a.iter().map(|&x| Tok::Lit(x)).collect::<Vec<_>>(), false);
    let btoks: Vec<Tok> = b.iter().map(|&x| Tok::Lit(x)).collect();
    let (ll, dl) = lens_for(&btoks, 288, 32);
    write_dynamic_block(&mut bw, &btoks, &ll, &dl, ClMode::Rle, None, false);
    write_stored_block(&mut bw, &c, true);
    let s = bw.finish();
    let mut expect = a.clone();
    expect.extend_from_slice(&b);
    expect.extend_from_slice(&c);
    ok_loose("row17 multiblock f/d/s", &s, expect.len());
}

#[test]
fn row18_multiblock_backref_across_boundary() {
    let a: Vec<u8> = (0..64u8).collect();
    let mut bw = BitWriter::new();
    write_fixed_block(&mut bw, &a.iter().map(|&x| Tok::Lit(x)).collect::<Vec<_>>(), false);
    // second block references data emitted by the first
    let toks2 = vec![Tok::Match(20, 64), Tok::Lit(0xFF), Tok::Match(30, 50)];
    write_fixed_block(&mut bw, &toks2, false);
    // third block, dynamic, also referencing earlier output
    let toks3 = vec![Tok::Match(8, 100), Tok::Lit(0x01)];
    let (ll, dl) = lens_for(&toks3, 288, 32);
    write_dynamic_block(&mut bw, &toks3, &ll, &dl, ClMode::Rle, None, true);
    let s = bw.finish();

    let mut expect = a.clone();
    let push_match = |e: &mut Vec<u8>, l: usize, d: usize| {
        let st = e.len() - d;
        for i in 0..l {
            let v = e[st + i];
            e.push(v);
        }
    };
    push_match(&mut expect, 20, 64);
    expect.push(0xFF);
    push_match(&mut expect, 30, 50);
    push_match(&mut expect, 8, 100);
    expect.push(0x01);
    ok("row18 backref across blocks", &s, &expect);
}

#[test]
fn row19_input_alignment_and_tail_matrix() {
    // (in & 3) x (in_bytes & 3) -> first_bytes 0..3 and final_word_available 0/1
    for pad in 0..4usize {
        for extra in 0..4usize {
            // pad the plaintext so the compressed length lands on each residue
            let data: Vec<u8> = (0..(37 + extra)).map(|i| (i * 3) as u8).collect();
            let s = deflate_fixed_lz(&data);
            let label = format!("row19 align={pad} inlen%4={}", s.len() % 4);
            let r = c_inflate_align(&s, s.len() as i32, pad, data.len() as i32, data.len());
            assert_eq!(r.ret, 1, "[{label}] C rejected: {:?}", r.error);
            assert_eq!(&r.out[..data.len()], &data[..], "[{label}] C plaintext wrong");
            diff_inflate_align(&label, &s, pad, data.len());
        }
    }
}

#[test]
fn row20_output_buffer_larger_than_needed() {
    let data: Vec<u8> = (0..100u8).collect();
    for mode in 0..3 {
        let s = match mode {
            0 => deflate_stored(&data),
            1 => deflate_fixed_lz(&data),
            _ => deflate_dynamic_lz(&data),
        };
        for slack in [1usize, 3, 4, 64, 1000] {
            let cap = data.len() + slack;
            let r = c_inflate(&s, s.len() as i32, cap as i32, cap);
            assert_eq!(r.ret, 1, "C rejected mode={mode} slack={slack}: {:?}", r.error);
            diff_inflate_full(
                &format!("row20 mode={mode} slack={slack}"),
                &s,
                s.len() as i32,
                0,
                cap as i32,
                cap,
            );
        }
    }
}

// --- row 21: randomized property testing ----------------------------------

#[test]
fn row21_randomized_payloads_all_block_types() {
    let mut rng = StdRng::seed_from_u64(0xC0FFEE_1234_5678);
    for case in 0..300 {
        let n = match case % 5 {
            0 => rng.gen_range(0..8),
            1 => rng.gen_range(8..64),
            2 => rng.gen_range(64..600),
            3 => rng.gen_range(600..4096),
            _ => rng.gen_range(1..40),
        };
        // three payload flavours: uniform noise, low entropy, long runs
        let data: Vec<u8> = match case % 3 {
            0 => (0..n).map(|_| rng.gen()).collect(),
            1 => (0..n).map(|_| rng.gen_range(0..4u8)).collect(),
            _ => {
                let mut v = Vec::with_capacity(n);
                while v.len() < n {
                    let b: u8 = rng.gen();
                    let r = rng.gen_range(1..40).min(n - v.len());
                    for _ in 0..r {
                        v.push(b);
                    }
                }
                v
            }
        };
        let modes: &[u8] = if data.len() <= 0xFFFF {
            &[0, 1, 2, 3, 4]
        } else {
            &[1, 2, 3, 4]
        };
        for &m in modes {
            let s = match m {
                0 => deflate_stored(&data),
                1 => deflate_fixed_literals(&data),
                2 => deflate_fixed_lz(&data),
                3 => deflate_dynamic_literals(&data, ClMode::Rle),
                _ => deflate_dynamic_lz(&data),
            };
            if data.is_empty() && m != 0 {
                // literal-only stream with no literals is still valid
            }
            let label = format!("row21 case={case} mode={m} n={}", data.len());
            if m == 0 {
                ok_loose(&label, &s, data.len().max(1));
            } else {
                ok(&label, &s, &data);
            }
        }
    }
}

#[test]
fn row21b_randomized_token_streams() {
    let mut rng = StdRng::seed_from_u64(0xBADC0DE_9999);
    for case in 0..300 {
        let mut toks: Vec<Tok> = Vec::new();
        let mut produced = 0usize;
        let nt = rng.gen_range(1..60);
        for _ in 0..nt {
            if produced >= 3 && rng.gen_bool(0.45) {
                let dist = rng.gen_range(1..=produced.min(32768)) as u32;
                let len = rng.gen_range(3..=258u32);
                toks.push(Tok::Match(len, dist));
                produced += len as usize;
            } else {
                toks.push(Tok::Lit(rng.gen()));
                produced += 1;
            }
        }
        let expect = apply(&toks);
        // fixed
        let mut bw = BitWriter::new();
        write_fixed_block(&mut bw, &toks, true);
        let s = bw.finish();
        let label = format!("row21b fixed case={case}");
        ok(&label, &s, &expect);
        // dynamic, both CL encodings
        let (ll, dl) = lens_for(&toks, 288, 32);
        for mode in [ClMode::Literal, ClMode::Rle] {
            let mut bw = BitWriter::new();
            write_dynamic_block(&mut bw, &toks, &ll, &dl, mode, None, true);
            let s = bw.finish();
            let label = format!("row21b dyn case={case} mode={}", mode == ClMode::Rle);
            ok(&label, &s, &expect);
        }
    }
}

#[test]
fn row21c_randomized_multiblock() {
    let mut rng = StdRng::seed_from_u64(0x5EED_5EED);
    for case in 0..120 {
        let nblocks = rng.gen_range(2..6);
        let mut bw = BitWriter::new();
        let mut expect: Vec<u8> = Vec::new();
        for b in 0..nblocks {
            let last = b == nblocks - 1;
            let kind = if last { rng.gen_range(0..3) } else { rng.gen_range(1..3) };
            let n = rng.gen_range(1..80);
            if kind == 0 {
                let data: Vec<u8> = (0..n).map(|_| rng.gen()).collect();
                write_stored_block(&mut bw, &data, true);
                expect.extend_from_slice(&data);
            } else {
                let mut toks: Vec<Tok> = Vec::new();
                for _ in 0..n {
                    if expect.len() >= 3 && rng.gen_bool(0.35) {
                        let dist = rng.gen_range(1..=expect.len().min(32768)) as u32;
                        let len = rng.gen_range(3..=64u32);
                        toks.push(Tok::Match(len, dist));
                        let st = expect.len() - dist as usize;
                        for i in 0..len as usize {
                            let v = expect[st + i];
                            expect.push(v);
                        }
                    } else {
                        let v: u8 = rng.gen();
                        toks.push(Tok::Lit(v));
                        expect.push(v);
                    }
                }
                if kind == 1 {
                    write_fixed_block(&mut bw, &toks, last);
                } else {
                    let (ll, dl) = lens_for(&toks, 288, 32);
                    write_dynamic_block(&mut bw, &toks, &ll, &dl, ClMode::Rle, None, last);
                }
            }
        }
        let s = bw.finish();
        let label = format!("row21c multiblock case={case}");
        ok_loose(&label, &s, expect.len().max(1));
    }
}

// --- helpers ---------------------------------------------------------------

fn cl_syms_used(ll: &[u8], dl: &[u8], mode: ClMode) -> Vec<usize> {
    let mut all: Vec<u8> = ll.to_vec();
    all.extend_from_slice(dl);
    let seq = cl_sequence(&all, mode);
    let mut v: Vec<usize> = seq.iter().map(|x| x.0).collect();
    v.sort();
    v.dedup();
    v
}

fn has_cl_symbol(ll: &[u8], dl: &[u8], sym: usize) -> bool {
    cl_syms_used(ll, dl, ClMode::Rle).contains(&sym)
}

fn natural_hclen(ll: &[u8], dl: &[u8], mode: ClMode) -> usize {
    let used = cl_syms_used(ll, dl, mode);
    let mut natural = 4usize;
    for i in 0..19 {
        if used.contains(&PERM[i]) {
            natural = natural.max(i + 1);
        }
    }
    natural
}
