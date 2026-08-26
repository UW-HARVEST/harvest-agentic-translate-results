//! Phase B — differential tests for `cp_inflate`, the lowest-level public
//! entry point.  CONFIGS.md rows 1-26.
//!
//! Both implementations are called through their `.so` exports; the *same*
//! input buffer (hence the same alignment) is handed to both, and the output
//! allocations are pre-filled with `0xCD` and compared in full (including a
//! 64-byte guard tail) so that any stray write shows up.

mod common;

use common::*;

/// `cp_stored()` recovers the source pointer with
/// `cp_ptr() = words + word_index - count/8`, which is only exact while the bit
/// buffer has been refilled from `words[]` alone.  As soon as the 40 header bits
/// (BFINAL+BTYPE+align+LEN+NLEN) have to be completed from `s->final_word`, the
/// pointer is off by `bits_left/8` bytes and the C library copies the wrong
/// bytes.  `first_bytes = (4-align)%4` bits are pre-loaded, then whole 32-bit
/// words, so the header is satisfied without `final_word` exactly when:
///
/// | align | `first_bytes` | exact for |
/// |-------|---------------|-----------|
/// | 0     | 0             | `LEN >= 3` |
/// | 1     | 3             | `LEN >= 2` |
/// | 2     | 2             | `LEN >= 1` |
/// | 3     | 1             | always     |
///
/// (verified against the C `.so` for `LEN = 0..6` x `align = 0..3`).
/// Outside that set C and Rust must still agree byte for byte — they just do
/// not agree with the DEFLATE specification.
fn stored_content_is_exact(align: usize, len: usize) -> bool {
    match align % 4 {
        0 => len >= 3,
        1 => len >= 2,
        2 => len >= 1,
        _ => true,
    }
}

/// Only assert that the two implementations agree (no claim about *what* the
/// right answer is).  Used where the C code has undefined-ish behaviour.
fn agree(deflate: &[u8], align: usize, out_bytes: i32, label: &str) -> InflateResult {
    diff_inflate(deflate, align, out_bytes, label)
}

/// Wrap a raw DEFLATE stream the way `load_png_mem` would (`data + 2`,
/// `datalen - 6`) — but here we drive `cp_inflate` directly, so the stream is
/// passed verbatim.
fn check(deflate: &[u8], expected: &[u8], align: usize, label: &str) {
    let r = diff_inflate(deflate, align, expected.len() as i32, label);
    assert_eq!(r.rc, 1, "[{label}] cp_inflate failed: {:?}", r.err);
    assert_eq!(&r.out[..expected.len()], expected, "[{label}] wrong output");
    // guard tail must be untouched
    assert!(
        r.out[expected.len()..].iter().all(|&b| b == 0xCD),
        "[{label}] wrote past out_bytes"
    );
}

// ---------------------------------------------------------------------------
// rows 1-9: input alignment (`first_bytes`) and input tail (`last_bytes`)
// ---------------------------------------------------------------------------

/// Rows 1-4 — `first_bytes` = 0,1,2,3 via the input pointer alignment.
#[test]
fn row_1_4_input_alignment() {
    let mut rng = Rng::new(0x1001);
    for align in 0..4usize {
        for iter in 0..40 {
            let n = rng.range(1, 200) as usize;
            let data = rng.bytes(n);
            let d = deflate_literals_fixed(&data);
            check(&d, &data, align, &format!("align={align} iter={iter}"));
        }
    }
}

/// Rows 5-8 — `last_bytes` = 0,1,2,3 (`final_word_available` 0/1).
/// The DEFLATE stream length is grown with harmless trailing bits so that
/// `in_bytes - first_bytes` hits every residue class mod 4.
#[test]
fn row_5_8_input_tail() {
    let mut rng = Rng::new(0x2002);
    for want_tail in 0..4usize {
        let mut found = 0;
        for iter in 0..4000 {
            let n = rng.range(1, 300) as usize;
            let data = rng.bytes(n);
            let d = deflate_literals_fixed(&data);
            // first_bytes == 0 when align == 0
            if d.len() % 4 != want_tail {
                continue;
            }
            check(&d, &data, 0, &format!("tail={want_tail} iter={iter}"));
            found += 1;
            if found == 15 {
                break;
            }
        }
        assert!(found > 0, "no stream with last_bytes={want_tail} was produced");
    }
}

/// Row 9 — full cross product alignment(4) x tail(4).
#[test]
fn row_9_alignment_tail_cross_product() {
    let mut rng = Rng::new(0x3003);
    let mut covered = [[0u32; 4]; 4];
    for _ in 0..1200 {
        let n = rng.range(1, 120) as usize;
        let data = rng.bytes(n);
        let d = deflate_literals_fixed(&data);
        for align in 0..4usize {
            let first_bytes = (4 - align) % 4;
            if d.len() < first_bytes {
                continue;
            }
            let tail = (d.len() - first_bytes) % 4;
            covered[align][tail] += 1;
            check(&d, &data, align, &format!("align={align} tail={tail}"));
        }
    }
    for align in 0..4 {
        for tail in 0..4 {
            assert!(
                covered[align][tail] > 0,
                "combination align={align} tail={tail} never exercised"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// rows 10-11: stored blocks
// ---------------------------------------------------------------------------

/// Row 10 — a single stored block, all four input alignments.
#[test]
fn row_10_stored_block() {
    let mut rng = Rng::new(0x4004);
    for align in 0..4usize {
        for &n in &[0usize, 1, 2, 3, 4, 5, 7, 8, 15, 16, 17, 63, 64, 255, 256, 1023, 4096] {
            let data = rng.bytes(n);
            let d = deflate_stored(&data);
            let label = format!("stored n={n} align={align}");
            if stored_content_is_exact(align, n) {
                check(&d, &data, align, &label);
            } else {
                let r = agree(&d, align, n as i32, &label);
                assert_eq!(r.rc, 1, "[{label}] {:?}", r.err);
            }
        }
        for _ in 0..25 {
            let n = rng.range(1, 2000) as usize;
            let data = rng.bytes(n);
            let d = deflate_stored(&data);
            let label = format!("stored rnd n={n} align={align}");
            if stored_content_is_exact(align, n) {
                check(&d, &data, align, &label);
            } else {
                agree(&d, align, n as i32, &label);
            }
        }
    }
}

/// Row 11 — a stored block with `LEN == 0` (`memcpy(dst, src, 0)`).
#[test]
fn row_11_stored_empty() {
    for align in 0..4usize {
        let d = deflate_stored(&[]);
        // out_bytes 0 and out_bytes > 0 both must work: nothing is emitted
        for out_bytes in [0i32, 1, 16] {
            let r = diff_inflate_full(&d, d.len() as i32, align, out_bytes, 64, "stored empty");
            assert_eq!(r.rc, 1, "empty stored block must succeed: {:?}", r.err);
            assert!(r.out.iter().all(|&b| b == 0xCD));
        }
    }
}

// ---------------------------------------------------------------------------
// rows 12-15: dynamic blocks
// ---------------------------------------------------------------------------

/// Rows 12-14 — dynamic blocks produced by flate2/miniz_oxide at every level.
#[test]
fn row_12_14_dynamic_flate2() {
    let mut rng = Rng::new(0x5005);
    for level in 0..=9u32 {
        for iter in 0..12 {
            let n = rng.range(1, 3000) as usize;
            let data = if iter % 2 == 0 {
                rng.bytes(n)
            } else {
                rng.repetitive(n, 6)
            };
            let d = deflate_flate2(&data, level);
            for align in 0..4usize {
                check(&d, &data, align, &format!("flate2 L{level} n={n} a={align}"));
            }
        }
    }
}

/// Row 15 — our own dynamic-block encoder, with and without the code-length
/// RLE symbols 16/17/18, so `cp_dynamic` is driven over its full header
/// grammar (HLIT/HDIST/HCLEN + permutation order + run-length symbols).
#[test]
fn row_15_dynamic_handrolled() {
    let mut rng = Rng::new(0x6006);
    for use_rle in [false, true] {
        for iter in 0..60 {
            let n = rng.range(1, 400) as usize;
            let mut toks: Vec<Tok> = Vec::new();
            let mut produced = 0usize;
            while produced < n {
                if produced >= 4 && rng.below(3) == 0 {
                    let dist = rng.range(1, produced.min(300) as u32);
                    let len = rng.range(3, 60);
                    toks.push(Tok::Match { len, dist });
                    produced += len as usize;
                } else {
                    let b = if iter % 3 == 0 {
                        rng.u8()
                    } else {
                        (rng.u8() % 12) + b'a'
                    };
                    toks.push(Tok::Lit(b));
                    produced += 1;
                }
            }
            let expected = expand(&toks);
            let d = deflate_dynamic(&toks, use_rle);
            for align in 0..4usize {
                check(
                    &d,
                    &expected,
                    align,
                    &format!("dyn rle={use_rle} iter={iter} a={align}"),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// rows 16-17: several blocks per stream
// ---------------------------------------------------------------------------

/// Row 16 — fixed + fixed, BFINAL only on the last block.
#[test]
fn row_16_multi_fixed_blocks() {
    let mut rng = Rng::new(0x7007);
    for nblocks in 2..=5usize {
        for iter in 0..15 {
            let mut bw = BitWriter::new();
            let mut expected = Vec::new();
            for b in 0..nblocks {
                let n = rng.range(1, 80) as usize;
                let data = rng.bytes(n);
                let toks: Vec<Tok> = data.iter().map(|&x| Tok::Lit(x)).collect();
                write_fixed_block(&mut bw, &toks, b + 1 == nblocks);
                expected.extend_from_slice(&data);
            }
            let d = bw.finish();
            for align in 0..4usize {
                check(
                    &d,
                    &expected,
                    align,
                    &format!("multi-fixed n={nblocks} iter={iter} a={align}"),
                );
            }
        }
    }
}

/// Row 17 — fixed + dynamic + fixed (+ random mixes).
#[test]
fn row_17_mixed_block_types() {
    let mut rng = Rng::new(0x8008);
    for iter in 0..60 {
        let nblocks = rng.range(2, 5) as usize;
        let mut bw = BitWriter::new();
        let mut expected = Vec::new();
        for b in 0..nblocks {
            let n = rng.range(1, 60) as usize;
            let data = rng.bytes(n);
            let toks: Vec<Tok> = data.iter().map(|&x| Tok::Lit(x)).collect();
            let last = b + 1 == nblocks;
            match if iter < 3 { b % 2 } else { rng.below(2) as usize } {
                0 => write_fixed_block(&mut bw, &toks, last),
                _ => write_dynamic_block(&mut bw, &toks, last, rng.below(2) == 0),
            }
            expected.extend_from_slice(&data);
        }
        let d = bw.finish();
        for align in 0..4usize {
            check(&d, &expected, align, &format!("mixed iter={iter} a={align}"));
        }
    }
}

// ---------------------------------------------------------------------------
// rows 18-22: back-references and the full length/distance code space
// ---------------------------------------------------------------------------

/// Row 18 — `backwards_distance == 1` takes the `memset` branch.
#[test]
fn row_18_distance_one_memset() {
    let mut rng = Rng::new(0x9009);
    for &len in &[3u32, 4, 5, 10, 57, 58, 130, 257, 258] {
        for align in 0..4usize {
            let b = rng.u8();
            let toks = vec![Tok::Lit(b), Tok::Match { len, dist: 1 }];
            let expected = expand(&toks);
            check(
                &deflate_fixed(&toks),
                &expected,
                align,
                &format!("memset len={len} a={align}"),
            );
            check(
                &deflate_dynamic(&toks, true),
                &expected,
                align,
                &format!("memset dyn len={len} a={align}"),
            );
        }
    }
    // every length 3..=258 with distance 1
    for len in 3..=258u32 {
        let toks = vec![Tok::Lit(0x5A), Tok::Match { len, dist: 1 }];
        let expected = expand(&toks);
        check(&deflate_fixed(&toks), &expected, 0, &format!("memset all len={len}"));
    }
}

/// Row 19 — non-overlapping back-references (`distance >= length`).
#[test]
fn row_19_nonoverlapping_matches() {
    let mut rng = Rng::new(0xA00A);
    for iter in 0..200 {
        let pre = rng.range(3, 300) as usize;
        let data = rng.bytes(pre);
        let mut toks: Vec<Tok> = data.iter().map(|&x| Tok::Lit(x)).collect();
        let len = rng.range(3, pre.min(258) as u32);
        let dist = rng.range(len, pre as u32);
        toks.push(Tok::Match { len, dist });
        let expected = expand(&toks);
        check(
            &deflate_fixed(&toks),
            &expected,
            iter % 4,
            &format!("nonoverlap iter={iter} len={len} dist={dist}"),
        );
    }
}

/// Row 20 — overlapping back-references (`length > distance`, distance 2..8).
#[test]
fn row_20_overlapping_matches() {
    for dist in 2..=8u32 {
        for len in 3..=258u32 {
            let mut toks: Vec<Tok> = (0..dist as u8).map(|i| Tok::Lit(0x40 + i)).collect();
            toks.push(Tok::Match { len, dist });
            let expected = expand(&toks);
            check(
                &deflate_fixed(&toks),
                &expected,
                (dist as usize) % 4,
                &format!("overlap dist={dist} len={len}"),
            );
        }
    }
}

/// Row 21 — every length code 257..285 (all `cp_len_base` /
/// `cp_len_extra_bits` entries, including all extra-bit values).
#[test]
fn row_21_all_length_codes() {
    let mut rng = Rng::new(0xB00B);
    for sym in 0..29usize {
        let base = LEN_BASE[sym];
        let nex = LEN_EXTRA[sym];
        let span = 1u32 << nex;
        for e in 0..span {
            let len = base + e;
            if len > 258 {
                continue;
            }
            let dist = rng.range(1, 300);
            let pre = dist as usize;
            let data: Vec<u8> = (0..pre).map(|i| (i * 7 + 3) as u8).collect();
            let mut toks: Vec<Tok> = data.iter().map(|&x| Tok::Lit(x)).collect();
            toks.push(Tok::Match { len, dist });
            let expected = expand(&toks);
            let label = format!("lensym={} len={} dist={}", 257 + sym, len, dist);
            check(&deflate_fixed(&toks), &expected, (e as usize) % 4, &label);
            check(&deflate_dynamic(&toks, true), &expected, (e as usize) % 4, &label);
        }
    }
}

/// Row 22 — every distance code 0..29 (all `cp_dist_base` /
/// `cp_dist_extra_bits` entries, distances up to 32768).
#[test]
fn row_22_all_distance_codes() {
    for sym in 0..30usize {
        let base = DIST_BASE[sym];
        let nex = DIST_EXTRA[sym];
        // sample the extra-bit space (exhaustive for small codes, sampled for big)
        let span = 1u32 << nex;
        let step = (span / 8).max(1);
        let mut e = 0u32;
        while e < span {
            let dist = base + e;
            let pre = dist as usize;
            let data: Vec<u8> = (0..pre).map(|i| (i * 31 + 7) as u8).collect();
            let mut toks: Vec<Tok> = data.iter().map(|&x| Tok::Lit(x)).collect();
            toks.push(Tok::Match { len: 3, dist });
            toks.push(Tok::Match { len: 258, dist });
            let expected = expand(&toks);
            let label = format!("distsym={sym} dist={dist}");
            check(&deflate_fixed(&toks), &expected, (e as usize) % 4, &label);
            if sym < 16 {
                check(&deflate_dynamic(&toks, true), &expected, 0, &label);
            }
            e += step;
        }
        // and the very last value of the code's range
        let dist = base + span - 1;
        if dist <= 32768 {
            let pre = dist as usize;
            let data: Vec<u8> = (0..pre).map(|i| (i * 13 + 1) as u8).collect();
            let mut toks: Vec<Tok> = data.iter().map(|&x| Tok::Lit(x)).collect();
            toks.push(Tok::Match { len: 4, dist });
            let expected = expand(&toks);
            check(
                &deflate_fixed(&toks),
                &expected,
                3,
                &format!("distsym={sym} maxdist={dist}"),
            );
        }
    }
}

/// Row 23 — literal alphabet boundaries: 8-bit codes (0..143), 9-bit codes
/// (144..255) and the 7-bit end-of-block code.
#[test]
fn row_23_literal_alphabet() {
    // every single literal on its own
    for b in 0..=255u8 {
        let toks = vec![Tok::Lit(b)];
        check(&deflate_fixed(&toks), &[b], (b as usize) % 4, &format!("lit {b}"));
    }
    // the whole alphabet, forwards and backwards
    let fwd: Vec<u8> = (0..=255u8).collect();
    let rev: Vec<u8> = (0..=255u8).rev().collect();
    for (name, data) in [("fwd", &fwd), ("rev", &rev)] {
        for align in 0..4usize {
            check(
                &deflate_literals_fixed(data),
                data,
                align,
                &format!("alphabet {name} a={align}"),
            );
            let toks: Vec<Tok> = data.iter().map(|&x| Tok::Lit(x)).collect();
            check(
                &deflate_dynamic(&toks, true),
                data,
                align,
                &format!("alphabet dyn {name} a={align}"),
            );
        }
    }
    // the boundary literals only
    for &b in &[142u8, 143, 144, 145, 254, 255, 0, 1] {
        let data = vec![b; 40];
        check(&deflate_literals_fixed(&data), &data, 1, &format!("boundary {b}"));
    }
}

// ---------------------------------------------------------------------------
// rows 24-26: output buffer shapes
// ---------------------------------------------------------------------------

/// Row 24 — `out_bytes` larger than what the stream produces.
#[test]
fn row_24_out_bytes_slack() {
    let mut rng = Rng::new(0xC00C);
    for iter in 0..80 {
        let n = rng.range(1, 200) as usize;
        let data = rng.bytes(n);
        let d = deflate_literals_fixed(&data);
        let slack_out = rng.range(0, 500) as i32;
        let r = diff_inflate_full(
            &d,
            d.len() as i32,
            iter % 4,
            n as i32 + slack_out,
            64,
            &format!("slack iter={iter}"),
        );
        assert_eq!(r.rc, 1, "{:?}", r.err);
        assert_eq!(&r.out[..n], &data[..]);
        assert!(
            r.out[n..].iter().all(|&b| b == 0xCD),
            "bytes past the stream must stay untouched"
        );
    }
}

/// Row 25 — output larger than 64 KiB, forcing several flate2 blocks and
/// distances up to 32768.
#[test]
fn row_25_large_output() {
    let mut rng = Rng::new(0xD00D);
    for &n in &[65536usize, 70000, 131072] {
        for level in [1u32, 6, 9] {
            let data = rng.repetitive(n, 30);
            let d = deflate_flate2(&data, level);
            check(&d, &data, 0, &format!("large n={n} L{level}"));
        }
    }
    // Incompressible data of the same size: every flate2 level falls back to
    // *stored* blocks, and >64 KiB needs several of them, which trips
    // `s->bits_left / 8 <= LEN` in `cp_stored` (ERRORS.md row 26).  Both
    // implementations must reject it with the identical message.
    let data = rng.bytes(70000);
    for level in 0..=9u32 {
        let d = deflate_flate2(&data, level);
        let label = format!("large random L{level}");
        let r = agree(&d, 2, data.len() as i32, &label);
        if r.rc == 1 {
            assert_eq!(&r.out[..data.len()], &data[..], "[{label}] wrong output");
        } else {
            assert_eq!(
                r.err.as_deref(),
                Some("Stored block extends beyond end of input stream."),
                "[{label}] unexpected rejection"
            );
        }
    }
}

/// Row 26 — 1-byte outputs combined with every input-tail residue.
#[test]
fn row_26_single_byte() {
    let mut rng = Rng::new(0xE00E);
    for align in 0..4usize {
        for b in [0u8, 1, 127, 128, 143, 144, 255] {
            check(&deflate_literals_fixed(&[b]), &[b], align, &format!("1byte {b}"));
            if stored_content_is_exact(align, 1) {
                check(&deflate_stored(&[b]), &[b], align, &format!("1byte stored {b}"));
            } else {
                agree(&deflate_stored(&[b]), align, 1, &format!("1byte stored {b}"));
            }
            let toks = vec![Tok::Lit(b)];
            check(
                &deflate_dynamic(&toks, false),
                &[b],
                align,
                &format!("1byte dyn {b}"),
            );
        }
        for _ in 0..20 {
            let b = rng.u8();
            check(&deflate_literals_fixed(&[b]), &[b], align, "1byte rnd");
        }
    }
}
