//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test drives BOTH `.so`s through `dlsym`'d exports with many randomized
//! inputs (fixed seed) and asserts byte-identical `out`, identical return code,
//! and identical `cp_error_reason`.

mod common;

use common::deflate::*;
use common::rng::{Rng, SEED};
use common::*;

const SIZES: [(usize, usize); 5] = [(1, 1), (2, 1), (1, 2), (7, 5), (64, 33)];

// ===========================================================================
// C07..C12 — convert_pix
// ===========================================================================

fn src_for(rng: &mut Rng, bpp: usize, w: usize, h: usize) -> Vec<u8> {
    // One filter byte + w*bpp sample bytes per row, plus slack.
    rng.bytes(h * (1 + w * bpp) + 16)
}

fn convert_pix_row(bpp: i32, tag: &str) {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ (bpp as u64) << 8);
    for &(w, h) in SIZES.iter() {
        for iter in 0..64 {
            let src = src_for(&mut rng, bpp as usize, w, h);
            diff_convert_pix(
                &p,
                &format!("{tag} bpp={bpp} w={w} h={h} iter={iter}"),
                bpp,
                w as i32,
                h as i32,
                &src,
                w * h,
            );
        }
    }
}

#[test]
fn c07_convert_pix_bpp1() {
    convert_pix_row(1, "C07");
}

#[test]
fn c08_convert_pix_bpp2() {
    convert_pix_row(2, "C08");
}

#[test]
fn c09_convert_pix_bpp3() {
    convert_pix_row(3, "C09");
}

#[test]
fn c10_convert_pix_bpp4() {
    convert_pix_row(4, "C10");
}

#[test]
fn c11_convert_pix_cross_product() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 0xC11);
    for bpp in 1..=4i32 {
        for w in 1..=8usize {
            for h in 1..=8usize {
                for _ in 0..8 {
                    let src = src_for(&mut rng, bpp as usize, w, h);
                    diff_convert_pix(
                        &p,
                        &format!("C11 bpp={bpp} w={w} h={h}"),
                        bpp,
                        w as i32,
                        h as i32,
                        &src,
                        w * h,
                    );
                }
            }
        }
    }
}

#[test]
fn c12_convert_pix_empty_dims() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 0xC12);
    for bpp in 1..=4i32 {
        let src = rng.bytes(256);
        // h == 0: nothing happens at all.
        diff_convert_pix(&p, "C12 h=0", bpp, 8, 0, &src, 16);
        // w == 0 with h > 0: only src++ per row, dst untouched.
        diff_convert_pix(&p, "C12 w=0", bpp, 0, 4, &src, 16);
        diff_convert_pix(&p, "C12 w=0 h=1", bpp, 0, 1, &src, 16);
    }
}

// ===========================================================================
// C13..C16 — stored blocks (btype 0)
//
// NOTE: the C code checks `s->bits_left / 8 <= LEN` in `cp_stored`, so a stored
// block that is *not* the last thing in the stream is rejected (E2). Rows C15
// and C16 therefore assert that BOTH implementations reject identically.
// ===========================================================================

#[test]
fn c13_stored_single_len_sweep() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 0xC13);
    for len in [
        0usize, 1, 2, 3, 4, 5, 7, 8, 15, 16, 17, 255, 256, 257, 1024,
    ] {
        for iter in 0..8 {
            let payload = rng.bytes(len);
            let mut w = BitWriter::new();
            emit_stored(&mut w, true, &payload);
            let stream = w.finish();
            diff_inflate(
                &p,
                &format!("C13 LEN={len} iter={iter}"),
                &stream,
                0,
                len + 32,
            );
        }
    }
}

#[test]
fn c14_stored_alignment_cross_product() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 0xC14);
    for align in 0..4usize {
        for len in [0usize, 1, 2, 3, 4, 5, 6, 7, 8, 9, 13, 64, 129] {
            for _ in 0..4 {
                let payload = rng.bytes(len);
                let mut w = BitWriter::new();
                emit_stored(&mut w, true, &payload);
                let stream = w.finish();
                diff_inflate(
                    &p,
                    &format!("C14 align={align} LEN={len}"),
                    &stream,
                    align,
                    len + 32,
                );
            }
        }
    }
}

#[test]
fn c15_stored_two_blocks() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 0xC15);
    for _ in 0..64 {
        let l1 = rng.below(64);
        let l2 = rng.below(64);
        let a = rng.bytes(l1);
        let b = rng.bytes(l2);
        let mut w = BitWriter::new();
        emit_stored(&mut w, false, &a);
        emit_stored(&mut w, true, &b);
        let stream = w.finish();
        diff_inflate(
            &p,
            &format!("C15 l1={l1} l2={l2}"),
            &stream,
            0,
            l1 + l2 + 32,
        );
    }
}

#[test]
fn c16_stored_many_blocks() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 0xC16);
    for _ in 0..48 {
        let n = 3 + rng.below(4);
        let align = rng.below(4);
        let mut total = 0usize;
        let mut w = BitWriter::new();
        for i in 0..n {
            let l = rng.below(48);
            total += l;
            let payload = rng.bytes(l);
            emit_stored(&mut w, i + 1 == n, &payload);
        }
        let stream = w.finish();
        diff_inflate(
            &p,
            &format!("C16 n={n} align={align}"),
            &stream,
            align,
            total + 64,
        );
    }
}

// ===========================================================================
// C17..C26 — fixed Huffman blocks (btype 1)
// ===========================================================================

fn lits(bytes: &[u8]) -> Vec<Op> {
    bytes.iter().map(|&b| Op::Lit(b)).collect()
}

/// Run a fixed block and also self-check the encoder against `expand`.
fn run_fixed(p: &Pair, ctx: &str, ops: &[Op], align: usize, slack: usize) {
    let expect = expand(ops);
    let mut w = BitWriter::new();
    emit_fixed(&mut w, true, ops);
    let stream = w.finish();
    let out_bytes = expect.len() + slack;
    let c = run_inflate(&p.c, &stream, align, out_bytes, None);
    assert_eq!(c.ret, 1, "[{ctx}] C rejected a valid stream: {c:?}");
    assert_eq!(
        &c.out[..expect.len()],
        &expect[..],
        "[{ctx}] encoder self-check failed"
    );
    diff_inflate(p, ctx, &stream, align, out_bytes);
}

#[test]
fn c17_fixed_literals_low() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 0xC17);
    for iter in 0..96 {
        let n = rng.below(301);
        let data: Vec<u8> = (0..n).map(|_| rng.byte() % 144).collect();
        run_fixed(&p, &format!("C17 n={n} iter={iter}"), &lits(&data), 0, 8);
    }
}

#[test]
fn c18_fixed_literals_high() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 0xC18);
    for iter in 0..96 {
        let n = rng.below(301);
        let data: Vec<u8> = (0..n).map(|_| 144 + (rng.byte() % 112)).collect();
        run_fixed(&p, &format!("C18 n={n} iter={iter}"), &lits(&data), 0, 8);
    }
}

#[test]
fn c19_fixed_literals_full_range() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 0xC19);
    for iter in 0..128 {
        let n = rng.below(513);
        let data = rng.bytes(n);
        run_fixed(&p, &format!("C19 n={n} iter={iter}"), &lits(&data), 0, 8);
    }
}

#[test]
fn c20_fixed_match_dist1_memset() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 0xC20);
    for len in [3u32, 4, 5, 6, 10, 11, 17, 100, 257, 258] {
        for _ in 0..8 {
            let npre = 1 + rng.below(8);
            let mut ops = lits(&rng.bytes(npre));
            ops.push(Op::Match { len, dist: 1 });
            ops.extend(lits(&rng.bytes(3)));
            run_fixed(&p, &format!("C20 len={len}"), &ops, 0, 8);
        }
    }
}

#[test]
fn c21_fixed_match_nonoverlapping() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 0xC21);
    for _ in 0..96 {
        let prefix = 40 + rng.below(120);
        let mut ops = lits(&rng.bytes(prefix));
        // dist >= len keeps the copy non-overlapping.
        let dist = rng.range(3, prefix as i64) as u32;
        let len = rng.range(3, dist as i64) as u32;
        ops.push(Op::Match { len, dist });
        ops.extend(lits(&rng.bytes(4)));
        run_fixed(&p, &format!("C21 len={len} dist={dist}"), &ops, 0, 8);
    }
}

#[test]
fn c22_fixed_match_overlapping() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 0xC22);
    for _ in 0..96 {
        let prefix = 8 + rng.below(120);
        let mut ops = lits(&rng.bytes(prefix));
        let dist = rng.range(2, prefix as i64) as u32;
        let len = rng.range(dist as i64 + 1, 258) as u32;
        ops.push(Op::Match { len, dist });
        ops.extend(lits(&rng.bytes(4)));
        run_fixed(&p, &format!("C22 len={len} dist={dist}"), &ops, 0, 8);
    }
}

#[test]
fn c23_fixed_length_symbol_sweep() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 0xC23);
    for lsym in 257u16..=285 {
        let eb = LEN_EXTRA[lsym as usize - 257];
        let mut extras: Vec<u32> = vec![0];
        if eb > 0 {
            extras.push((1 << eb) - 1);
            extras.push(rng.below(1usize << eb) as u32);
        }
        for &lextra in &extras {
            for &dsym in &[0u16, 1, 3, 5] {
                let prefix = 300usize;
                let mut ops = lits(&rng.bytes(prefix));
                ops.push(Op::Raw {
                    lsym,
                    lextra,
                    dsym,
                    dextra: 0,
                });
                ops.extend(lits(&rng.bytes(2)));
                run_fixed(
                    &p,
                    &format!("C23 lsym={lsym} lextra={lextra} dsym={dsym}"),
                    &ops,
                    0,
                    8,
                );
            }
        }
    }
}

#[test]
fn c24_fixed_distance_symbol_sweep() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 0xC24);
    for dsym in 0u16..30 {
        let eb = DIST_EXTRA[dsym as usize];
        let mut extras: Vec<u32> = vec![0];
        if eb > 0 {
            extras.push((1 << eb) - 1);
            extras.push(rng.below(1usize << eb) as u32);
        }
        for &dextra in &extras {
            let dist = DIST_BASE[dsym as usize] + dextra;
            let prefix = (dist as usize) + 16;
            let mut ops = lits(&rng.bytes(prefix));
            ops.push(Op::Raw {
                lsym: 260,
                lextra: 0,
                dsym,
                dextra,
            });
            ops.extend(lits(&rng.bytes(2)));
            run_fixed(&p, &format!("C24 dsym={dsym} dextra={dextra}"), &ops, 0, 8);
        }
    }
}

/// Pick a random valid match given the number of bytes already emitted.
fn pick_match(rng: &mut Rng, outlen: usize, max_lsym: u16, max_dsym: u16, len_cap: u32) -> Op {
    let mut cands: Vec<u16> = Vec::new();
    for d in 0..=max_dsym.min(29) {
        if DIST_BASE[d as usize] as usize <= outlen {
            cands.push(d);
        }
    }
    let dsym = cands[rng.below(cands.len())];
    let eb = DIST_EXTRA[dsym as usize];
    let maxe = ((1u32 << eb) - 1).min((outlen as u32) - DIST_BASE[dsym as usize]);
    let dextra = if maxe == 0 {
        0
    } else {
        rng.below(maxe as usize + 1) as u32
    };
    let mut lsyms: Vec<u16> = Vec::new();
    for l in 257..=max_lsym.min(285) {
        if LEN_BASE[l as usize - 257] <= len_cap {
            lsyms.push(l);
        }
    }
    let lsym = lsyms[rng.below(lsyms.len())];
    let leb = LEN_EXTRA[lsym as usize - 257];
    let maxle = ((1u32 << leb) - 1).min(len_cap.saturating_sub(LEN_BASE[lsym as usize - 257]));
    let lextra = if maxle == 0 {
        0
    } else {
        rng.below(maxle as usize + 1) as u32
    };
    Op::Raw {
        lsym,
        lextra,
        dsym,
        dextra,
    }
}

/// Random literal/match program. `alpha` restricts the literal alphabet.
fn random_program(
    rng: &mut Rng,
    n_ops: usize,
    alpha: &[u8],
    max_lsym: u16,
    max_dsym: u16,
) -> Vec<Op> {
    let mut ops: Vec<Op> = Vec::new();
    let mut outlen = 0usize;
    for _ in 0..n_ops {
        let want_match = max_lsym >= 257 && outlen >= 4 && rng.below(3) == 0;
        if want_match {
            let op = pick_match(rng, outlen, max_lsym, max_dsym, 120);
            let (ls, le, _, _) = resolve(op).unwrap();
            outlen += (LEN_BASE[ls as usize - 257] + le) as usize;
            ops.push(op);
        } else {
            let b = alpha[rng.below(alpha.len())];
            ops.push(Op::Lit(b));
            outlen += 1;
        }
    }
    ops
}

/// `random_program` with the op count drawn from `rng` (`1..=max_ops`).
fn random_program_n(
    rng: &mut Rng,
    max_ops: usize,
    alpha: &[u8],
    max_lsym: u16,
    max_dsym: u16,
) -> Vec<Op> {
    let n = 1 + rng.below(max_ops);
    random_program(rng, n, alpha, max_lsym, max_dsym)
}

#[test]
fn c25_fixed_random_program_all_alignments() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 0xC25);
    let alpha: Vec<u8> = (0..=255u8).collect();
    for align in 0..4usize {
        for iter in 0..48 {
            let n = 1 + rng.below(200);
            let ops = random_program(&mut rng, n, &alpha, 285, 29);
            run_fixed(
                &p,
                &format!("C25 align={align} n={n} iter={iter}"),
                &ops,
                align,
                8,
            );
        }
    }
}

#[test]
fn c26_fixed_multiple_blocks() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 0xC26);
    let alpha: Vec<u8> = (0..=255u8).collect();
    for iter in 0..48 {
        let nblocks = 2 + rng.below(4);
        // Build one logical program, split across blocks; matches in later
        // blocks reach back into earlier blocks' output.
        let mut all: Vec<Op> = Vec::new();
        let mut per_block: Vec<Vec<Op>> = Vec::new();
        for _ in 0..nblocks {
            let n = 1 + rng.below(60);
            let mut outlen = expand(&all).len();
            let mut blk: Vec<Op> = Vec::new();
            for _ in 0..n {
                if outlen >= 8 && rng.below(3) == 0 {
                    let op = pick_match(&mut rng, outlen, 285, 29, 100);
                    let (ls, le, _, _) = resolve(op).unwrap();
                    outlen += (LEN_BASE[ls as usize - 257] + le) as usize;
                    blk.push(op);
                } else {
                    blk.push(Op::Lit(alpha[rng.below(alpha.len())]));
                    outlen += 1;
                }
            }
            all.extend(blk.iter().copied());
            per_block.push(blk);
        }
        let expect = expand(&all);
        let mut w = BitWriter::new();
        for (i, blk) in per_block.iter().enumerate() {
            emit_fixed(&mut w, i + 1 == nblocks, blk);
        }
        let stream = w.finish();
        let out_bytes = expect.len() + 8;
        let c = run_inflate(&p.c, &stream, 0, out_bytes, None);
        assert_eq!(c.ret, 1, "[C26 iter={iter}] C rejected valid stream: {c:?}");
        assert_eq!(&c.out[..expect.len()], &expect[..], "[C26] self-check");
        diff_inflate(&p, &format!("C26 iter={iter}"), &stream, 0, out_bytes);
    }
}

// ===========================================================================
// C27..C37 — dynamic Huffman blocks (btype 2)
// ===========================================================================

fn run_dynamic(
    p: &Pair,
    ctx: &str,
    d: &Dynamic,
    ops: &[Op],
    align: usize,
    slack: usize,
) {
    let expect = expand(ops);
    let mut w = BitWriter::new();
    emit_dynamic(&mut w, true, d, ops);
    let stream = w.finish();
    let out_bytes = expect.len() + slack;
    let c = run_inflate(&p.c, &stream, align, out_bytes, None);
    assert_eq!(c.ret, 1, "[{ctx}] C rejected a valid stream: {c:?}");
    assert_eq!(
        &c.out[..expect.len()],
        &expect[..],
        "[{ctx}] encoder self-check failed"
    );
    diff_inflate(p, ctx, &stream, align, out_bytes);
}

#[test]
fn c27_dynamic_literal_lengths_minimal() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 0xC27);
    let alpha: Vec<u8> = (0..=255u8).collect();
    for iter in 0..48 {
        let n = 1 + rng.below(120);
        let ops = random_program(&mut rng, n, &alpha, 256, 0); // literals only
        let mut d = dynamic_for(&mut rng, &ops, Shape::Balanced, RepeatOpts::none(), 257, 1);
        d.force_nlen = Some(19);
        run_dynamic(&p, &format!("C27 iter={iter} n={n}"), &d, &ops, 0, 8);
    }
}

#[test]
fn c28_dynamic_hclen_sweep() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 0xC28);
    let alpha: Vec<u8> = (0..=255u8).collect();
    for nlen in 4..=19usize {
        for _ in 0..4 {
            let ops = random_program_n(&mut rng, 80, &alpha, 256, 0);
            let mut d =
                dynamic_for(&mut rng, &ops, Shape::Balanced, RepeatOpts::none(), 257, 1);
            // Only force nlen upward; the emitter asserts if it is too small.
            let mut min_nlen = 4usize;
            for (i, &pi) in PERMUTATION.iter().enumerate() {
                let _ = pi;
                let _ = i;
            }
            // Recompute the minimum from the code-length code the emitter will
            // build by simply trying: if nlen is too small, skip this pairing.
            min_nlen = min_nlen.max(4);
            if nlen < min_nlen {
                continue;
            }
            d.force_nlen = Some(nlen);
            let expect = expand(&ops);
            let mut w = BitWriter::new();
            // The emitter asserts if `nlen` cannot carry the code; catching that
            // would hide bugs, so instead only emit when it fits.
            let fits = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let mut probe = BitWriter::new();
                emit_dynamic(&mut probe, true, &d, &ops);
            }))
            .is_ok();
            if !fits {
                continue;
            }
            emit_dynamic(&mut w, true, &d, &ops);
            let stream = w.finish();
            let out_bytes = expect.len() + 8;
            let c = run_inflate(&p.c, &stream, 0, out_bytes, None);
            assert_eq!(c.ret, 1, "[C28 nlen={nlen}] C rejected: {c:?}");
            assert_eq!(&c.out[..expect.len()], &expect[..], "[C28] self-check");
            diff_inflate(&p, &format!("C28 nlen={nlen}"), &stream, 0, out_bytes);
        }
    }
}

#[test]
fn c29_dynamic_nlit_ndst_sweep() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 0xC29);
    let alpha: Vec<u8> = (0..=255u8).collect();
    for nlit in [257usize, 258, 260, 270, 280, 285, 286, 287, 288] {
        for ndst in [1usize, 2, 3, 8, 16, 30, 31, 32] {
            let max_lsym = if nlit >= 258 { (nlit - 1).min(285) as u16 } else { 256 };
            let max_dsym = (ndst - 1).min(29) as u16;
            let ops = random_program_n(&mut rng, 80, &alpha, max_lsym, max_dsym);
            let d = dynamic_for(&mut rng, &ops, Shape::Balanced, RepeatOpts::none(), nlit, ndst);
            run_dynamic(
                &p,
                &format!("C29 nlit={nlit} ndst={ndst}"),
                &d,
                &ops,
                0,
                8,
            );
        }
    }
}

fn dynamic_with_repeats(tag: &str, r: RepeatOpts, salt: u64) {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ salt);
    let alpha: Vec<u8> = (0..=255u8).collect();
    for iter in 0..48 {
        let n = 1 + rng.below(150);
        let ops = random_program(&mut rng, n, &alpha, 285, 29);
        let d = dynamic_for(&mut rng, &ops, Shape::Balanced, r, 288, 30);
        run_dynamic(&p, &format!("{tag} iter={iter}"), &d, &ops, 0, 8);
    }
}

#[test]
fn c30_dynamic_repeat16() {
    dynamic_with_repeats(
        "C30",
        RepeatOpts {
            use16: true,
            use17: false,
            use18: false,
        },
        0xC30,
    );
}

#[test]
fn c31_dynamic_repeat17() {
    dynamic_with_repeats(
        "C31",
        RepeatOpts {
            use16: false,
            use17: true,
            use18: false,
        },
        0xC31,
    );
}

#[test]
fn c32_dynamic_repeat18() {
    dynamic_with_repeats(
        "C32",
        RepeatOpts {
            use16: false,
            use17: false,
            use18: true,
        },
        0xC32,
    );
}

#[test]
fn c32b_dynamic_all_repeats() {
    dynamic_with_repeats("C32b", RepeatOpts::all(), 0xC32B);
}

#[test]
fn c33_dynamic_shallow_codes() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 0xC33);
    let alpha: Vec<u8> = (0..=255u8).collect();
    for iter in 0..48 {
        let ops = random_program_n(&mut rng, 150, &alpha, 285, 29);
        let d = dynamic_for(&mut rng, &ops, Shape::Shallow, RepeatOpts::all(), 288, 30);
        assert!(
            d.lit_lens.iter().all(|&l| l <= 9),
            "C33 must stay within cp_build's lookup cutoff"
        );
        run_dynamic(&p, &format!("C33 iter={iter}"), &d, &ops, 0, 8);
    }
}

#[test]
fn c34_dynamic_deep_codes() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 0xC34);
    // 16 lit/len symbols => skewed complete code with lengths 1..15,15, which
    // forces cp_build's `len > 9` (tree-only) path and 15-bit codes.
    for iter in 0..48 {
        let n_lits = 14usize;
        let mut alpha: Vec<u8> = Vec::new();
        while alpha.len() < n_lits {
            let b = rng.byte();
            if !alpha.contains(&b) {
                alpha.push(b);
            }
        }
        // exactly one length symbol + 256 + 14 literals = 16 symbols
        let lsym = 257 + rng.below(29) as u16;
        let mut ops: Vec<Op> = Vec::new();
        let mut outlen = 0usize;
        for _ in 0..40 {
            ops.push(Op::Lit(alpha[rng.below(alpha.len())]));
            outlen += 1;
        }
        for _ in 0..10 {
            let eb = LEN_EXTRA[lsym as usize - 257];
            let lextra = if eb == 0 { 0 } else { rng.below(1 << eb) as u32 };
            let dist = 1 + rng.below(outlen.min(64));
            let mut dsym = 29u16;
            while DIST_BASE[dsym as usize] as usize > dist {
                dsym -= 1;
            }
            let dextra = dist as u32 - DIST_BASE[dsym as usize];
            ops.push(Op::Raw {
                lsym,
                lextra,
                dsym,
                dextra,
            });
            outlen += (LEN_BASE[lsym as usize - 257] + lextra) as usize;
            for _ in 0..3 {
                ops.push(Op::Lit(alpha[rng.below(alpha.len())]));
                outlen += 1;
            }
        }
        let d = dynamic_for(&mut rng, &ops, Shape::Skewed, RepeatOpts::all(), 288, 30);
        let maxlen = *d.lit_lens.iter().max().unwrap();
        assert!(
            maxlen > 9,
            "C34 iter={iter}: expected a code longer than 9 bits, got {maxlen}"
        );
        run_dynamic(&p, &format!("C34 iter={iter} maxlen={maxlen}"), &d, &ops, 0, 8);
    }
}

#[test]
fn c35_dynamic_single_distance_code() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 0xC35);
    let alpha: Vec<u8> = (0..=255u8).collect();
    for iter in 0..48 {
        // ndst == 1 => only distance symbol 0 (dist == 1, the memset path).
        let mut ops: Vec<Op> = Vec::new();
        let mut outlen = 0usize;
        for _ in 0..(1 + rng.below(60)) {
            if outlen >= 1 && rng.below(3) == 0 {
                let lsym = 257 + rng.below(20) as u16;
                let eb = LEN_EXTRA[lsym as usize - 257];
                let lextra = if eb == 0 { 0 } else { rng.below(1 << eb) as u32 };
                ops.push(Op::Raw {
                    lsym,
                    lextra,
                    dsym: 0,
                    dextra: 0,
                });
                outlen += (LEN_BASE[lsym as usize - 257] + lextra) as usize;
            } else {
                ops.push(Op::Lit(alpha[rng.below(alpha.len())]));
                outlen += 1;
            }
        }
        let d = dynamic_for(&mut rng, &ops, Shape::Balanced, RepeatOpts::all(), 288, 1);
        assert_eq!(d.dst_lens.len(), 1);
        run_dynamic(&p, &format!("C35 iter={iter}"), &d, &ops, 0, 8);
    }
}

#[test]
fn c36_dynamic_match_shapes() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 0xC36);
    let alpha: Vec<u8> = (0..=255u8).collect();
    for iter in 0..48 {
        let prefix = 64 + rng.below(200);
        let mut ops: Vec<Op> = (0..prefix)
            .map(|_| Op::Lit(alpha[rng.below(alpha.len())]))
            .collect();
        // dist == 1 (memset)
        let l0 = rng.range(3, 53) as u32;
        ops.push(Op::Match { len: l0, dist: 1 });
        // dist > 1, non-overlapping (len <= dist)
        let d1 = rng.range(40, prefix as i64) as u32;
        ops.push(Op::Match {
            len: rng.range(3, d1 as i64) as u32,
            dist: d1,
        });
        // dist > 1, overlapping (len > dist)
        let d2 = rng.range(2, 30) as u32;
        ops.push(Op::Match {
            len: rng.range(d2 as i64 + 1, 200) as u32,
            dist: d2,
        });
        ops.push(Op::Lit(alpha[rng.below(alpha.len())]));
        let d = dynamic_for(&mut rng, &ops, Shape::Random, RepeatOpts::all(), 288, 30);
        run_dynamic(&p, &format!("C36 iter={iter}"), &d, &ops, 0, 8);
    }
}

#[test]
fn c37_dynamic_random_alignments() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 0xC37);
    let alpha: Vec<u8> = (0..=255u8).collect();
    for align in 0..4usize {
        for pad in 0..4usize {
            for iter in 0..12 {
                let n = 1 + rng.below(150);
                let ops = random_program(&mut rng, n, &alpha, 285, 29);
                let shape = match rng.below(3) {
                    0 => Shape::Balanced,
                    1 => Shape::Random,
                    _ => Shape::Shallow,
                };
                let d = dynamic_for(&mut rng, &ops, shape, RepeatOpts::all(), 288, 30);
                let expect = expand(&ops);
                let mut w = BitWriter::new();
                emit_dynamic(&mut w, true, &d, &ops);
                let mut stream = w.finish();
                // `pad` extra trailing bytes changes in_bytes & 3 -> last_bytes,
                // final_word_available and final_word.
                for _ in 0..pad {
                    stream.push(rng.byte());
                }
                let out_bytes = expect.len() + 8;
                let c = run_inflate(&p.c, &stream, align, out_bytes, None);
                assert_eq!(
                    c.ret, 1,
                    "[C37 align={align} pad={pad} iter={iter}] C rejected: {c:?}"
                );
                assert_eq!(&c.out[..expect.len()], &expect[..], "[C37] self-check");
                diff_inflate(
                    &p,
                    &format!("C37 align={align} pad={pad} iter={iter}"),
                    &stream,
                    align,
                    out_bytes,
                );
            }
        }
    }
}

// ===========================================================================
// C38 — mixed multi-block stream
// ===========================================================================

#[test]
fn c38_mixed_block_types() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 0xC38);
    let alpha: Vec<u8> = (0..=255u8).collect();
    for iter in 0..48 {
        // fixed -> dynamic -> stored(final). A stored block is only accepted as
        // the final block (see the C's bits_left/8 <= LEN check).
        let a = random_program_n(&mut rng, 60, &alpha, 285, 29);
        let out_a = expand(&a);
        let b_ops = random_program_n(&mut rng, 60, &alpha, 285, 29);
        let d = dynamic_for(&mut rng, &b_ops, Shape::Random, RepeatOpts::all(), 288, 30);
        let nstored = rng.below(40);
        let stored = rng.bytes(nstored);

        let mut w = BitWriter::new();
        emit_fixed(&mut w, false, &a);
        emit_dynamic(&mut w, false, &d, &b_ops);
        emit_stored(&mut w, true, &stored);
        let stream = w.finish();

        let mut expect = out_a.clone();
        expect.extend(expand(&b_ops));
        expect.extend_from_slice(&stored);
        let out_bytes = expect.len() + 16;

        let c = run_inflate(&p.c, &stream, 0, out_bytes, None);
        let r = run_inflate(&p.rust, &stream, 0, out_bytes, None);
        assert_eq!(c.ret, r.ret, "[C38 iter={iter}] ret\n C:{c:?}\n R:{r:?}");
        assert_eq!(c.err, r.err, "[C38 iter={iter}] err\n C:{c:?}\n R:{r:?}");
        assert_eq!(c.out, r.out, "[C38 iter={iter}] out\n C:{c:?}\n R:{r:?}");
        // No self-check against a reference inflate here: `cp_stored` recovers
        // the payload address with `cp_ptr()`, which points into the *input
        // buffer* and ignores bytes that are only present in `s->bits` /
        // `s->final_word`. When the stored payload falls inside the final
        // partial word the C reads the wrong bytes. That is the C's behaviour,
        // so only the C-vs-Rust comparison above is meaningful.
        let _ = &expect;
    }

    // Also: fixed -> dynamic (final), no stored block, so a success is expected.
    for iter in 0..48 {
        let a = random_program_n(&mut rng, 60, &alpha, 285, 29);
        let b_ops = random_program_n(&mut rng, 60, &alpha, 285, 29);
        let d = dynamic_for(&mut rng, &b_ops, Shape::Balanced, RepeatOpts::all(), 288, 30);
        let mut w = BitWriter::new();
        emit_fixed(&mut w, false, &a);
        emit_dynamic(&mut w, true, &d, &b_ops);
        let stream = w.finish();
        let mut expect = expand(&a);
        expect.extend(expand(&b_ops));
        let out_bytes = expect.len() + 16;
        let c = run_inflate(&p.c, &stream, 0, out_bytes, None);
        assert_eq!(c.ret, 1, "[C38b iter={iter}] C rejected: {c:?}");
        assert_eq!(&c.out[..expect.len()], &expect[..], "[C38b] self-check");
        diff_inflate(&p, &format!("C38b iter={iter}"), &stream, 0, out_bytes);
    }
}

// ===========================================================================
// C40 — exact-fit out buffer
// ===========================================================================

#[test]
fn c40_exact_out_buffer() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 0xC40);
    let alpha: Vec<u8> = (0..=255u8).collect();
    for iter in 0..64 {
        let ops = random_program_n(&mut rng, 120, &alpha, 285, 29);
        let expect = expand(&ops);
        if expect.is_empty() {
            continue;
        }
        // fixed, exact fit
        let mut w = BitWriter::new();
        emit_fixed(&mut w, true, &ops);
        let stream = w.finish();
        let c = run_inflate(&p.c, &stream, 0, expect.len(), None);
        assert_eq!(c.ret, 1, "[C40 fixed exact iter={iter}] C rejected: {c:?}");
        assert_eq!(c.out, expect, "[C40] self-check");
        diff_inflate(&p, &format!("C40 fixed exact iter={iter}"), &stream, 0, expect.len());
        // and one byte short => both must reject identically (E3/E5)
        diff_inflate(
            &p,
            &format!("C40 fixed short iter={iter}"),
            &stream,
            0,
            expect.len() - 1,
        );

        // dynamic, exact fit
        let d = dynamic_for(&mut rng, &ops, Shape::Random, RepeatOpts::all(), 288, 30);
        let mut w = BitWriter::new();
        emit_dynamic(&mut w, true, &d, &ops);
        let stream = w.finish();
        let c = run_inflate(&p.c, &stream, 0, expect.len(), None);
        assert_eq!(c.ret, 1, "[C40 dyn exact iter={iter}] C rejected: {c:?}");
        diff_inflate(&p, &format!("C40 dyn exact iter={iter}"), &stream, 0, expect.len());
        diff_inflate(
            &p,
            &format!("C40 dyn short iter={iter}"),
            &stream,
            0,
            expect.len() - 1,
        );
    }
}
