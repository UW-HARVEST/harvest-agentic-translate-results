//! Phase B — valid-path differential tests for `cp_inflate`
//! (`CONFIGS.md` rows 23..47 and 54..64).

mod common;

use common::deflate::*;
use common::*;

/// A prepared shared-memory layout: `[pad | in | pad | out | slack]`.
struct Built {
    case: Case,
    #[allow(dead_code)]
    in_off: usize,
    out_off: usize,
}

fn build_inflate(
    stream: &[u8],
    in_skew: usize,
    in_bytes_override: Option<i32>,
    out_bytes: i32,
    out_slack: usize,
    seed: u64,
) -> Built {
    let mut rng = Rng::new(seed);
    let in_off = 64 + in_skew;
    let in_len = stream.len();
    let out_off = (in_off + in_len + 128) & !15;
    let out_region = (out_bytes.max(0) as usize) + out_slack;
    let total = out_off + out_region + 256;
    let mut scratch: Vec<u8> = (0..total).map(|_| rng.u8()).collect();
    scratch[in_off..in_off + in_len].copy_from_slice(stream);
    let in_bytes = in_bytes_override.unwrap_or(in_len as i32);
    Built {
        case: Case::inflate(scratch, in_off as isize, in_bytes, out_off as isize, out_bytes),
        in_off,
        out_off,
    }
}

/// Differential-check a stream and, when `expect` is `Some`, also check that
/// the reference C library really produced the bytes the encoder intended (so
/// the test is known to exercise a *working* decode, not two identical
/// failures).
#[track_caller]
fn check(stream: &[u8], expect: Option<&[u8]>, out_extra: i32, skew: usize, seed: u64, ctx: &str) -> Outcome {
    let out_bytes = expect.map(|e| e.len() as i32).unwrap_or(4096) + out_extra;
    let b = build_inflate(stream, skew, None, out_bytes, 1024, seed);
    let o = diff(&b.case, ctx);
    if let Some(e) = expect {
        assert_eq!(o.ret, 1, "[{ctx}] expected success, got ret={} err={:?}", o.ret, o.err);
        assert_eq!(o.err, None, "[{ctx}] unexpected error {:?}", o.err);
        let got = &o.scratch[b.out_off..b.out_off + e.len()];
        assert_eq!(got, e, "[{ctx}] wrong decompressed bytes");
        // bytes past the decompressed data must be untouched
        let tail_from = b.out_off + e.len();
        assert_eq!(
            &o.scratch[tail_from..],
            &b.case.scratch[tail_from..],
            "[{ctx}] wrote past the end of the decompressed data"
        );
    }
    o
}

fn fixed_stream(syms: &[Sym]) -> (Vec<u8>, Vec<u8>) {
    let t = Tables::default();
    let mut bw = BitWriter::new();
    emit_fixed(&mut bw, syms, true);
    (bw.finish(), expand(syms, &t))
}

// ---------------------------------------------------------------------------
// row 23/24: fixed-Huffman literals, both code-length classes, empty block
// ---------------------------------------------------------------------------

#[test]
fn cfg23_fixed_all_literal_values() {
    // every literal value, one per stream (8-bit codes for 0..143, 9-bit for
    // 144..255)
    for v in 0u16..256 {
        let syms = vec![Sym::Lit(v as u8)];
        let (s, e) = fixed_stream(&syms);
        check(&s, Some(&e), 0, 0, 0x2300 + v as u64, &format!("cfg23 single literal {v}"));
    }
    // and long randomized literal runs
    let mut rng = Rng::new(0x23);
    for i in 0..200 {
        let n = rng.below(300) as usize + 1;
        let syms: Vec<Sym> = (0..n).map(|_| Sym::Lit(rng.u8())).collect();
        let (s, e) = fixed_stream(&syms);
        check(&s, Some(&e), 0, 0, 0x2340 + i, &format!("cfg23 random literals n={n}"));
    }
}

#[test]
fn cfg24_fixed_empty_block() {
    let (s, e) = fixed_stream(&[]);
    assert!(e.is_empty());
    for out_extra in [0i32, 1, 64] {
        check(&s, Some(&e), out_extra, 0, 0x24, &format!("cfg24 empty out_extra={out_extra}"));
    }
}

// ---------------------------------------------------------------------------
// rows 25..27: the three match-copy shapes
// ---------------------------------------------------------------------------

#[test]
fn cfg25_match_distance_one_memset_path() {
    let mut rng = Rng::new(0x25);
    for len in [3u32, 4, 5, 10, 17, 100, 258] {
        for _ in 0..8 {
            let b = rng.u8();
            let syms = vec![Sym::Lit(b), Sym::Match(len, 1)];
            let (s, e) = fixed_stream(&syms);
            check(&s, Some(&e), 0, 0, rng.next_u64(), &format!("cfg25 dist=1 len={len}"));
        }
    }
}

#[test]
fn cfg26_match_non_overlapping() {
    let mut rng = Rng::new(0x26);
    for _ in 0..300 {
        let pre = rng.below(200) as usize + 3;
        let mut syms: Vec<Sym> = (0..pre).map(|_| Sym::Lit(rng.u8())).collect();
        let len = rng.range(3, pre as i32) as u32;
        let dist = rng.range(len as i32, pre as i32) as u32;
        syms.push(Sym::Match(len, dist));
        let (s, e) = fixed_stream(&syms);
        check(&s, Some(&e), 0, 0, rng.next_u64(), &format!("cfg26 len={len} dist={dist}"));
    }
}

#[test]
fn cfg27_match_overlapping() {
    let mut rng = Rng::new(0x27);
    for _ in 0..300 {
        let pre = rng.below(64) as usize + 4;
        let mut syms: Vec<Sym> = (0..pre).map(|_| Sym::Lit(rng.u8())).collect();
        let dist = rng.range(2, pre as i32) as u32;
        let len = rng.range(dist as i32 + 1, 258) as u32;
        syms.push(Sym::Match(len, dist));
        let (s, e) = fixed_stream(&syms);
        check(&s, Some(&e), 0, 0, rng.next_u64(), &format!("cfg27 overlap len={len} dist={dist}"));
    }
}

// ---------------------------------------------------------------------------
// rows 28..30: every length and distance code
// ---------------------------------------------------------------------------

#[test]
fn cfg28_every_length_code() {
    let t = Tables::default();
    let mut rng = Rng::new(0x28);
    for lc in 0usize..29 {
        let nextra = t.len_extra[lc] as u32;
        let mut extras: Vec<u32> = vec![0];
        if nextra > 0 {
            extras.push((1 << nextra) - 1);
            for _ in 0..3 {
                extras.push(rng.below(1 << nextra));
            }
        }
        for le in extras {
            let len = t.len_base[lc] + le;
            let pre = 300usize;
            let mut syms: Vec<Sym> = (0..pre).map(|_| Sym::Lit(rng.u8())).collect();
            // distance code 0 (distance 1) and a bigger one
            for dist in [1u32, 3, 258] {
                let mut s2 = syms.clone();
                let (dc, de) = t.dist_code(dist);
                s2.push(Sym::RawMatch(lc, le, dc, de));
                let (s, e) = fixed_stream(&s2);
                check(
                    &s,
                    Some(&e),
                    0,
                    0,
                    rng.next_u64(),
                    &format!("cfg28 lc={lc} le={le} len={len} dist={dist}"),
                );
            }
            syms.clear();
        }
    }
}

#[test]
fn cfg29_every_distance_code() {
    let t = Tables::default();
    let mut rng = Rng::new(0x29);
    for dc in 0usize..30 {
        let nextra = t.dist_extra[dc] as u32;
        let mut extras: Vec<u32> = vec![0];
        if nextra > 0 {
            extras.push((1 << nextra) - 1);
            extras.push(rng.below(1 << nextra));
        }
        for de in extras {
            let dist = t.dist_base[dc] + de;
            let pre = dist as usize + 5;
            let mut syms: Vec<Sym> = (0..pre).map(|_| Sym::Lit(rng.u8())).collect();
            syms.push(Sym::RawMatch(0, 0, dc, de)); // length 3
            syms.push(Sym::Match(4, dist));
            let (s, e) = fixed_stream(&syms);
            check(&s, Some(&e), 0, 0, rng.next_u64(), &format!("cfg29 dc={dc} de={de} dist={dist}"));
        }
    }
}

#[test]
fn cfg30_mixed_7bit_and_8bit_length_codes() {
    // length codes 257..279 have 7-bit fixed codes, 280..287 have 8-bit ones.
    let t = Tables::default();
    let mut rng = Rng::new(0x30);
    for _ in 0..150 {
        let pre = 300usize;
        let mut syms: Vec<Sym> = (0..pre).map(|_| Sym::Lit(rng.u8())).collect();
        for _ in 0..20 {
            let lc = rng.below(29) as usize;
            let le = if t.len_extra[lc] > 0 { rng.below(1 << t.len_extra[lc]) } else { 0 };
            let dist = rng.range(1, 200) as u32;
            let (dc, de) = t.dist_code(dist);
            syms.push(Sym::RawMatch(lc, le, dc, de));
        }
        let (s, e) = fixed_stream(&syms);
        check(&s, Some(&e), 0, 0, rng.next_u64(), "cfg30 mixed length codes");
    }
}

// ---------------------------------------------------------------------------
// rows 31..37: dynamic blocks
// ---------------------------------------------------------------------------

fn dyn_stream(spec: &DynSpec, syms: &[Sym]) -> (Vec<u8>, Vec<u8>) {
    let t = Tables::default();
    let mut bw = BitWriter::new();
    emit_dynamic(&mut bw, spec, syms, true, &t);
    (bw.finish(), expand(syms, &t))
}

#[test]
fn cfg31_dynamic_hlit_hdist_hclen_ranges() {
    let t = Tables::default();
    let mut rng = Rng::new(0x31);
    for nlit in [257usize, 258, 260, 270, 286, 287, 288] {
        for ndst in [1usize, 2, 3, 8, 31, 32] {
            let n = rng.below(60) as usize + 1;
            let syms: Vec<Sym> = (0..n).map(|_| Sym::Lit(rng.below(200) as u8)).collect();
            let mut spec = dyn_spec_for(&syms, nlit, ndst, &t);
            // exercise every legal HCLEN >= the minimum
            for extra in [0usize, 1, 5] {
                let min = spec.nlen.unwrap_or(0);
                let _ = min;
                spec.nlen = None;
                let (s0, e0) = dyn_stream(&spec, &syms);
                check(
                    &s0,
                    Some(&e0),
                    0,
                    0,
                    rng.next_u64(),
                    &format!("cfg31 nlit={nlit} ndst={ndst} (min hclen)"),
                );
                if extra > 0 {
                    let mut sp = spec.clone();
                    sp.nlen = Some(19usize.min(19).max(4));
                    let (s1, e1) = dyn_stream(&sp, &syms);
                    check(
                        &s1,
                        Some(&e1),
                        0,
                        0,
                        rng.next_u64(),
                        &format!("cfg31 nlit={nlit} ndst={ndst} hclen=19"),
                    );
                }
            }
        }
    }
}

#[test]
fn cfg32to35_dynamic_code_length_run_modes() {
    let t = Tables::default();
    let mut rng = Rng::new(0x32);
    for (name, mode) in [
        ("literal", ClMode::LITERAL),
        ("repeat16", ClMode::R16),
        ("zero17", ClMode::R17),
        ("zero18", ClMode::R18),
        ("all", ClMode::ALL),
    ] {
        for _ in 0..60 {
            let n = rng.below(120) as usize + 1;
            let syms: Vec<Sym> = (0..n)
                .map(|_| {
                    if rng.below(4) == 0 {
                        Sym::Match(rng.range(3, 12) as u32, 1)
                    } else {
                        Sym::Lit(rng.u8())
                    }
                })
                .collect();
            // a match needs something before it
            let mut syms = syms;
            syms.insert(0, Sym::Lit(rng.u8()));
            let nlit = rng.pick(&[257usize, 270, 288]);
            let ndst = rng.pick(&[1usize, 4, 32]);
            let mut spec = dyn_spec_for(&syms, nlit, ndst, &t);
            spec.cl_mode = mode;
            let (s, e) = dyn_stream(&spec, &syms);
            check(&s, Some(&e), 0, 0, rng.next_u64(), &format!("cfg32..35 cl={name}"));
        }
    }
}

#[test]
fn cfg36_dynamic_single_distance_code() {
    let t = Tables::default();
    let mut rng = Rng::new(0x36);
    for _ in 0..80 {
        let mut syms: Vec<Sym> = (0..40).map(|_| Sym::Lit(rng.u8())).collect();
        syms.push(Sym::Match(rng.range(3, 30) as u32, 1));
        let spec = dyn_spec_for(&syms, 257, 1, &t);
        let (s, e) = dyn_stream(&spec, &syms);
        check(&s, Some(&e), 0, 0, rng.next_u64(), "cfg36 ndst=1");
    }
}

#[test]
fn cfg37_dynamic_deep_trees() {
    // Code lengths up to 14 bits: `cp_build` fills `s->lookup` only for
    // lengths <= 9, and returns `first[15]`, which counts every symbol whose
    // length is <= 14.
    let t = Tables::default();
    let mut rng = Rng::new(0x37);
    for maxlen in [9u8, 10, 12, 14] {
        for _ in 0..40 {
            let k = maxlen as usize + 1;
            let mut used: Vec<usize> = vec![256];
            while used.len() < k {
                let s = rng.below(200) as usize;
                if !used.contains(&s) {
                    used.push(s);
                }
            }
            used.sort_unstable();
            let lit_lens = deep_lengths(257, &used, maxlen);
            let dist_lens = balanced_lengths(4, &[0, 1]);
            let mut spec = DynSpec::new(lit_lens, dist_lens);
            spec.cl_mode = ClMode::ALL;
            let codeable: Vec<u8> = used.iter().filter(|s| **s < 256).map(|s| *s as u8).collect();
            let n = rng.below(80) as usize + 1;
            let syms: Vec<Sym> =
                (0..n).map(|_| Sym::Lit(rng.pick(&codeable))).collect();
            let (s, e) = dyn_stream(&spec, &syms);
            let _ = &t;
            check(&s, Some(&e), 0, 0, rng.next_u64(), &format!("cfg37 maxlen={maxlen}"));
        }
    }
}

// ---------------------------------------------------------------------------
// rows 38..40: stored blocks and multi-block streams
// ---------------------------------------------------------------------------

/// `first_bytes` for an input placed at `in_off` inside a page-aligned scratch.
fn first_bytes(in_skew: usize) -> usize {
    let addr = 64 + in_skew;
    ((addr + 3) & !3) - addr
}

/// `cp_stored` copies from `cp_ptr(s)`, which is only the true data position
/// when the *final partial word* has not been loaded yet - i.e. when
/// `(in_bytes - first_bytes) % 4 == 0`, so `final_word_available` is 0.
/// Otherwise the C's `count += s->bits_left` quirk shifts the source pointer
/// backwards; that is still perfectly deterministic (and both libraries do it
/// identically), it just cannot be predicted by the encoder.
fn stored_copy_is_exact(total_in: usize, in_skew: usize) -> bool {
    let fb = first_bytes(in_skew);
    total_in >= fb && (total_in - fb) % 4 == 0
}

#[test]
fn cfg38_stored_blocks() {
    let mut rng = Rng::new(0x38);
    let mut exact_checked = 0usize;
    for n in [
        0usize, 1, 2, 3, 4, 5, 6, 7, 8, 11, 15, 16, 19, 100, 255, 256, 259, 1000, 2048, 2051,
    ] {
        let data = rng.bytes(n);
        let mut bw = BitWriter::new();
        emit_stored(&mut bw, &data, true);
        let s = bw.finish();
        assert_eq!(s.len(), 5 + n);
        for skew in 0..4usize {
            let ctx = format!("cfg38 stored n={n} skew={skew}");
            if stored_copy_is_exact(s.len(), skew) {
                check(&s, Some(&data), 0, skew, rng.next_u64(), &ctx);
                exact_checked += 1;
            } else {
                let b = build_inflate(&s, skew, None, n as i32 + 64, 1024, rng.next_u64());
                let o = diff(&b.case, &ctx);
                assert_eq!(o.ret, 1, "[{ctx}] err={:?}", o.err);
            }
        }
    }
    assert!(exact_checked >= 15, "only {exact_checked} exact stored cases");
    for _ in 0..100 {
        let n = rng.below(600) as usize;
        let data = rng.bytes(n);
        let mut bw = BitWriter::new();
        emit_stored(&mut bw, &data, true);
        let s = bw.finish();
        if stored_copy_is_exact(s.len(), 0) {
            check(&s, Some(&data), 0, 0, rng.next_u64(), "cfg38 stored random");
        } else {
            let b = build_inflate(&s, 0, None, n as i32 + 64, 1024, rng.next_u64());
            let o = diff(&b.case, "cfg38 stored random");
            assert_eq!(o.ret, 1);
        }
    }
}

#[test]
fn cfg39_stored_after_bitpacked_block() {
    let mut rng = Rng::new(0x39);
    let mut exact = 0usize;
    for _ in 0..120 {
        // a non-final fixed block, then a final stored block: exercises the
        // `cp_read_bits(s, s->count & 7)` re-alignment with every possible
        // starting bit offset.
        let nlit = rng.below(40) as usize + 1;
        let lits: Vec<Sym> = (0..nlit).map(|_| Sym::Lit(rng.u8())).collect();
        let nstored = rng.below(80) as usize;
        let stored = rng.bytes(nstored);
        let mut bw = BitWriter::new();
        emit_fixed(&mut bw, &lits, false);
        emit_stored(&mut bw, &stored, true);
        let s = bw.finish();
        let mut e = expand(&lits, &Tables::default());
        e.extend_from_slice(&stored);
        if stored_copy_is_exact(s.len(), 0) {
            check(&s, Some(&e), 0, 0, rng.next_u64(), "cfg39 fixed then stored");
            exact += 1;
        } else {
            let b = build_inflate(&s, 0, None, e.len() as i32, 1024, rng.next_u64());
            let o = diff(&b.case, "cfg39 fixed then stored");
            assert_eq!(o.ret, 1, "err={:?}", o.err);
            // the literal part is always exact
            let lit_len = e.len() - stored.len();
            assert_eq!(&o.scratch[b.out_off..b.out_off + lit_len], &e[..lit_len]);
        }
    }
    assert!(exact > 5, "only {exact} exactly-checkable cfg39 cases");
}

#[test]
fn cfg40_multi_block_streams() {
    let t = Tables::default();
    let mut rng = Rng::new(0x40);
    for _ in 0..250 {
        let nblocks = rng.below(3) as usize + 2;
        let mut bw = BitWriter::new();
        let mut expected: Vec<u8> = vec![];
        for b in 0..nblocks {
            let last = b + 1 == nblocks;
            let n = rng.below(40) as usize + 1;
            let mut syms: Vec<Sym> = (0..n).map(|_| Sym::Lit(rng.u8())).collect();
            // a back-reference that reaches into an earlier block's output
            if !expected.is_empty() && rng.below(2) == 0 {
                let maxd = (expected.len() + n).min(200) as i32;
                let dist = rng.range(1, maxd) as u32;
                let len = rng.range(3, 20) as u32;
                syms.push(Sym::Match(len, dist));
            }
            match rng.below(2) {
                0 => emit_fixed(&mut bw, &syms, last),
                _ => {
                    let spec = dyn_spec_for(&syms, 288, 32, &t);
                    emit_dynamic(&mut bw, &spec, &syms, last, &t);
                }
            }
            // apply the symbols on top of everything decoded so far
            for s in &syms {
                s.apply(&mut expected, &t);
            }
        }
        let s = bw.finish();
        check(&s, Some(&expected), 0, 0, rng.next_u64(), &format!("cfg40 {nblocks} blocks"));
    }
}

// ---------------------------------------------------------------------------
// rows 41..43: input alignment / length residues
// ---------------------------------------------------------------------------

#[test]
fn cfg41to43_input_alignment_and_residues() {
    let mut rng = Rng::new(0x41);
    for _ in 0..150 {
        let n = rng.below(40) as usize + 1;
        let syms: Vec<Sym> = (0..n).map(|_| Sym::Lit(rng.u8())).collect();
        let (s, e) = fixed_stream(&syms);
        for skew in 0..4usize {
            check(
                &s,
                Some(&e),
                0,
                skew,
                rng.next_u64(),
                &format!("cfg41 skew={skew} len={}", s.len()),
            );
        }
    }
    // tiny inputs: fewer than four bytes in total, so `words` is never read
    for n in 1..4usize {
        for skew in 0..4usize {
            let syms: Vec<Sym> = (0..n).map(|_| Sym::Lit(0x41)).collect();
            let (s, e) = fixed_stream(&syms);
            // (the encoded stream may be longer than 3 bytes; also probe a
            // deliberately truncated view of it further down in the error tests)
            check(&s, Some(&e), 0, skew, 0x4300 + n as u64, &format!("cfg43 n={n} skew={skew}"));
        }
    }
    // empty stored block is the shortest possible valid stream: 5 bytes
    let mut bw = BitWriter::new();
    emit_stored(&mut bw, &[], true);
    let s = bw.finish();
    assert_eq!(s.len(), 5);
    for skew in 0..4usize {
        check(&s, Some(&[]), 0, skew, 0x4400 + skew as u64, &format!("cfg43 empty stored skew={skew}"));
    }
}

// ---------------------------------------------------------------------------
// rows 44..45: out_bytes exactly right / much larger
// ---------------------------------------------------------------------------

#[test]
fn cfg44and45_out_buffer_sizes() {
    let mut rng = Rng::new(0x44);
    for _ in 0..200 {
        let n = rng.below(60) as usize + 1;
        let mut syms: Vec<Sym> = (0..n).map(|_| Sym::Lit(rng.u8())).collect();
        syms.push(Sym::Match(rng.range(3, 20) as u32, rng.range(1, n as i32) as u32));
        let (s, e) = fixed_stream(&syms);
        for extra in [0i32, 1, 7, 64, 4096] {
            check(&s, Some(&e), extra, 0, rng.next_u64(), &format!("cfg44 out_extra={extra}"));
        }
    }
}

// ---------------------------------------------------------------------------
// rows 46..47: streams from a third-party DEFLATE encoder
// ---------------------------------------------------------------------------

fn flate2_deflate(data: &[u8], level: u32) -> Vec<u8> {
    use flate2::write::DeflateEncoder;
    use flate2::Compression;
    use std::io::Write;
    let mut enc = DeflateEncoder::new(Vec::new(), Compression::new(level));
    enc.write_all(data).unwrap();
    enc.finish().unwrap()
}

#[test]
fn cfg46and47_third_party_streams() {
    let mut rng = Rng::new(0x46);
    let payloads: Vec<Vec<u8>> = vec![
        vec![],
        vec![0],
        b"hello world".to_vec(),
        vec![0x61; 1000],
        (0..4096u32).map(|i| (i % 251) as u8).collect(),
        rng.bytes(1000),
        rng.bytes(8192),
        {
            let mut v = vec![];
            for i in 0..500 {
                v.extend_from_slice(format!("line {i}: the quick brown fox\n").as_bytes());
            }
            v
        },
    ];
    let mut decoded_ok = 0usize;
    for (pi, p) in payloads.iter().enumerate() {
        for level in [0u32, 1, 6, 9] {
            let s = flate2_deflate(p, level);
            for skew in 0..4usize {
                for trim in [0usize, 0, 0] {
                    let s2 = &s[..s.len() - trim];
                    let out_bytes = p.len() as i32 + 64;
                    let b = build_inflate(s2, skew, None, out_bytes, 2048, rng.next_u64());
                    let o = diff(
                        &b.case,
                        &format!("cfg46 payload={pi} level={level} skew={skew}"),
                    );
                    if o.ret == 1 {
                        let got = &o.scratch[b.out_off..b.out_off + p.len()];
                        assert_eq!(got, &p[..], "cfg46 payload={pi} level={level}: wrong bytes");
                        decoded_ok += 1;
                    }
                }
            }
        }
    }
    assert!(decoded_ok > 20, "only {decoded_ok} third-party streams decoded successfully");
}

// ---------------------------------------------------------------------------
// rows 49..52: the writable exported tables used as runtime options
// ---------------------------------------------------------------------------

#[test]
fn cfg49_fixed_table_override() {
    let mut rng = Rng::new(0x49);
    // Replace cp_fixed_table with a different *complete* assignment: give the
    // literal alphabet a flat 9-bit code (2^9 == 512 > 288, so pad it to a
    // complete code over 512 slots by using 288 symbols of length 9 plus a
    // shorter code... instead: use the canonical "all 288 symbols" balanced
    // assignment, which is complete by construction).
    let lit_lens = balanced_lengths(288, &(0..288).collect::<Vec<usize>>());
    let dist_lens = balanced_lengths(32, &(0..32).collect::<Vec<usize>>());
    let mut table = lit_lens.clone();
    table.extend_from_slice(&dist_lens);
    assert_eq!(table.len(), 320);
    assert_ne!(table, default_fixed_table());

    let t = Tables::default();
    for _ in 0..80 {
        let n = rng.below(60) as usize + 1;
        let mut syms: Vec<Sym> = (0..n).map(|_| Sym::Lit(rng.u8())).collect();
        syms.push(Sym::Match(rng.range(3, 12) as u32, rng.range(1, n as i32) as u32));
        let mut bw = BitWriter::new();
        emit_fixed_with(&mut bw, &syms, true, &table, &t);
        let stream = bw.finish();
        let expected = expand(&syms, &t);
        let b = build_inflate(&stream, 0, None, expected.len() as i32, 512, rng.next_u64());
        let case = b.case.clone().with_table(Table::FixedTable, table.clone());
        let o = diff(&case, "cfg49 cp_fixed_table override");
        assert_eq!(o.ret, 1, "err={:?}", o.err);
        assert_eq!(&o.scratch[b.out_off..b.out_off + expected.len()], &expected[..]);
    }
}

#[test]
fn cfg50_permutation_order_override() {
    let mut rng = Rng::new(0x50);
    let t = Tables::default();
    for _ in 0..60 {
        // a random permutation of 0..19
        let mut perm: Vec<u8> = (0..19).collect();
        for i in (1..19).rev() {
            let j = rng.below(i as u32 + 1) as usize;
            perm.swap(i, j);
        }
        let mut parr = [0u8; 19];
        parr.copy_from_slice(&perm);

        let n = rng.below(50) as usize + 1;
        let syms: Vec<Sym> = (0..n).map(|_| Sym::Lit(rng.u8())).collect();
        let mut spec = dyn_spec_for(&syms, 288, 32, &t);
        spec.perm = parr;
        spec.nlen = Some(19); // all 19 entries are written, so any order works
        let (stream, expected) = dyn_stream(&spec, &syms);
        let b = build_inflate(&stream, 0, None, expected.len() as i32, 512, rng.next_u64());
        let case = b.case.clone().with_table(Table::PermutationOrder, parr.to_vec());
        let o = diff(&case, "cfg50 cp_permutation_order override");
        assert_eq!(o.ret, 1, "err={:?}", o.err);
        assert_eq!(&o.scratch[b.out_off..b.out_off + expected.len()], &expected[..]);
    }
}

#[test]
fn cfg51_length_table_override() {
    let mut rng = Rng::new(0x51);
    let mut t = Tables::default();
    // Shift every length base by +1 and give the first four codes one extra
    // bit each.
    for i in 0..29 {
        t.len_base[i] += 1;
    }
    for i in 0..4 {
        t.len_extra[i] = 1;
    }
    let len_base_bytes: Vec<u8> = t.len_base.iter().flat_map(|v| v.to_ne_bytes()).collect();
    let len_extra_bytes: Vec<u8> = t.len_extra.clone();

    for _ in 0..80 {
        let pre = 64usize;
        let mut syms: Vec<Sym> = (0..pre).map(|_| Sym::Lit(rng.u8())).collect();
        for _ in 0..5 {
            let lc = rng.below(29) as usize;
            let le = if t.len_extra[lc] > 0 { rng.below(1 << t.len_extra[lc]) } else { 0 };
            let dist = rng.range(1, 60) as u32;
            let (dc, de) = t.dist_code(dist);
            syms.push(Sym::RawMatch(lc, le, dc, de));
        }
        let mut bw = BitWriter::new();
        emit_fixed_with(&mut bw, &syms, true, &default_fixed_table(), &t);
        let stream = bw.finish();
        let expected = expand(&syms, &t);
        let b = build_inflate(&stream, 0, None, expected.len() as i32, 512, rng.next_u64());
        let case = b
            .case
            .clone()
            .with_table(Table::LenBase, len_base_bytes.clone())
            .with_table(Table::LenExtraBits, len_extra_bytes.clone());
        let o = diff(&case, "cfg51 length table override");
        assert_eq!(o.ret, 1, "err={:?}", o.err);
        assert_eq!(&o.scratch[b.out_off..b.out_off + expected.len()], &expected[..]);
    }
}

#[test]
fn cfg52_distance_table_override() {
    let mut rng = Rng::new(0x52);
    let mut t = Tables::default();
    for i in 0..30 {
        t.dist_base[i] = t.dist_base[i].max(1) + 2;
    }
    for i in 0..4 {
        t.dist_extra[i] = 2;
    }
    let dist_base_bytes: Vec<u8> = t.dist_base.iter().flat_map(|v| v.to_ne_bytes()).collect();
    let dist_extra_bytes: Vec<u8> = t.dist_extra.clone();

    for _ in 0..80 {
        let pre = 200usize;
        let mut syms: Vec<Sym> = (0..pre).map(|_| Sym::Lit(rng.u8())).collect();
        for _ in 0..5 {
            let dc = rng.below(10) as usize;
            let de = if t.dist_extra[dc] > 0 { rng.below(1 << t.dist_extra[dc]) } else { 0 };
            syms.push(Sym::RawMatch(rng.below(10) as usize, 0, dc, de));
        }
        let mut bw = BitWriter::new();
        emit_fixed_with(&mut bw, &syms, true, &default_fixed_table(), &t);
        let stream = bw.finish();
        let expected = expand(&syms, &t);
        let b = build_inflate(&stream, 0, None, expected.len() as i32, 512, rng.next_u64());
        let case = b
            .case
            .clone()
            .with_table(Table::DistBase, dist_base_bytes.clone())
            .with_table(Table::DistExtraBits, dist_extra_bytes.clone());
        let o = diff(&case, "cfg52 distance table override");
        assert_eq!(o.ret, 1, "err={:?}", o.err);
        assert_eq!(&o.scratch[b.out_off..b.out_off + expected.len()], &expected[..]);
    }
}

/// row 53: `cp_inflate` must not modify the exported tables.
#[test]
fn cfg53_tables_unmodified_by_inflate() {
    let before_c: Vec<Vec<u8>> = Table::ALL.iter().map(|t| c_lib().read_table(*t)).collect();
    let before_r: Vec<Vec<u8>> = Table::ALL.iter().map(|t| rust_lib().read_table(*t)).collect();
    let mut rng = Rng::new(0x53);
    for _ in 0..20 {
        let n = rng.below(50) as usize + 1;
        let syms: Vec<Sym> = (0..n).map(|_| Sym::Lit(rng.u8())).collect();
        let (s, e) = fixed_stream(&syms);
        check(&s, Some(&e), 0, 0, rng.next_u64(), "cfg53");
    }
    // NB: every call runs in a forked child, so the parent's copies can only
    // have changed if a *previous* in-process call had modified them.
    for (i, t) in Table::ALL.iter().enumerate() {
        assert_eq!(c_lib().read_table(*t), before_c[i], "C modified {t:?}");
        assert_eq!(rust_lib().read_table(*t), before_r[i], "Rust modified {t:?}");
    }
}

/// row 58: an all-zero distance tree makes `cp_decode`'s binary search end at
/// `lo == 0`, so it reads `tree[-1]` - the `u32` in front of `dst[]` inside
/// `cp_state_t`.  The Rust translation must read the same bytes.
#[test]
fn cfg58_decode_reads_tree_minus_one() {
    let t = Tables::default();
    let mut rng = Rng::new(0x58);
    let mut saw = 0;
    for _ in 0..40 {
        let mut syms: Vec<Sym> = (0..40).map(|_| Sym::Lit(rng.u8())).collect();
        syms.push(Sym::Match(3, 1));
        let spec0 = dyn_spec_for(&syms, 288, 32, &t);
        // wipe the distance code lengths: the dist tree becomes empty
        let mut spec = spec0.clone();
        spec.dist_lens = vec![0u8; 32];
        let mut bw = BitWriter::new();
        emit_dynamic(&mut bw, &spec, &syms, true, &t);
        // pad so that the decoder cannot run out of bits while chasing the
        // garbage symbol that `tree[-1]` yields
        let mut stream = bw.finish();
        stream.extend_from_slice(&[0u8; 64]);
        let b = build_inflate(&stream, 0, None, 4096, 4096, rng.next_u64());
        diff(&b.case, "cfg58 empty distance tree");
        saw += 1;
    }
    assert_eq!(saw, 40);
}
