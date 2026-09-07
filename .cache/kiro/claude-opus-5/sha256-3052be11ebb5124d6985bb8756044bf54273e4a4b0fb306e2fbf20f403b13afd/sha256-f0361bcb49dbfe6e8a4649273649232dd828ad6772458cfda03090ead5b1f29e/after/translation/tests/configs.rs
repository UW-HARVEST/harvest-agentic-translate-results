//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every case is executed against BOTH shared objects through their exported
//! `pinflate` symbol and the outcome (return value, whole output buffer
//! including 64 bytes of slack, and `cp_error_reason`) must match byte for
//! byte.

mod common;
use common::*;

const SEED: u64 = 0x5EED_1234;

// ---------------------------------------------------------------------------
// helpers for building well-formed dynamic blocks
// ---------------------------------------------------------------------------

fn perm_pos(sym: usize) -> usize {
    PERMUTATION_ORDER
        .iter()
        .position(|&p| p as usize == sym)
        .unwrap()
}

/// Smallest legal HCLEN count that can transmit all of `used`.
fn min_nlen(used: &[usize]) -> usize {
    let mut need = 4usize;
    for &u in used {
        need = need.max(perm_pos(u) + 1);
    }
    need
}

/// Complete code-length-code lengths for `used`, transmittable with `nlen`.
/// Uses `flat_lens`, which yields a complete Huffman code for any symbol count
/// (not just powers of two), so no padding symbols are needed.
fn cl_lenlens(used: &[usize], nlen: usize) -> [u8; 19] {
    let allowed: Vec<usize> = (0..nlen).map(|i| PERMUTATION_ORDER[i] as usize).collect();
    let mut set = used.to_vec();
    set.sort_unstable();
    set.dedup();
    for u in &set {
        assert!(allowed.contains(u), "CL symbol {u} needs a bigger HCLEN");
    }
    let lens = flat_lens(set.len());
    let mut out = [0u8; 19];
    for (i, &s) in set.iter().enumerate() {
        assert!(lens[i] <= 7, "CL code length {} exceeds the 3-bit field", lens[i]);
        out[s] = lens[i];
    }
    out
}

/// Lengths `1,2,..,k-1,k,k` — a complete code whose longest code is `k`.
fn chain_lens(k: u8) -> Vec<u8> {
    assert!(k >= 1);
    if k == 1 {
        return vec![1, 1];
    }
    let mut v: Vec<u8> = (1..k).collect();
    v.push(k);
    v.push(k);
    v
}

/// Spread `lens` over a `size`-wide alphabet, assigning them to `syms`.
fn place(size: usize, syms: &[usize], lens: &[u8]) -> Vec<u8> {
    assert!(syms.len() <= lens.len(), "{} > {}", syms.len(), lens.len());
    let mut v = vec![0u8; size];
    for (i, &s) in syms.iter().enumerate() {
        v[s] = lens[i];
    }
    v
}

/// A complete tree over exactly `syms` (uses `flat_lens`).
fn tree_for(size: usize, syms: &[usize]) -> Vec<u8> {
    let mut s = syms.to_vec();
    s.sort_unstable();
    s.dedup();
    let lens = flat_lens(s.len());
    place(size, &s, &lens)
}

struct DynSpec {
    nlit: usize,
    ndst: usize,
    nlen: usize,
    lit_lens: Vec<u8>,
    dst_lens: Vec<u8>,
}

impl DynSpec {
    fn build(&self) -> Dynamic {
        assert_eq!(self.lit_lens.len(), self.nlit);
        assert_eq!(self.dst_lens.len(), self.ndst);
        let mut flat = self.lit_lens.clone();
        flat.extend_from_slice(&self.dst_lens);
        let mut used: Vec<usize> = flat.iter().map(|&l| l as usize).collect();
        used.sort_unstable();
        used.dedup();
        let nlen = self.nlen.max(min_nlen(&used));
        Dynamic {
            nlit: self.nlit,
            ndst: self.ndst,
            nlen,
            lenlens: cl_lenlens(&used, nlen),
            prog: prog_literal(&flat),
        }
    }
    fn lit(&self) -> Huff {
        Huff::new(self.lit_lens.clone())
    }
    fn dst(&self) -> Huff {
        Huff::new(self.dst_lens.clone())
    }
}

/// Convenience: a dynamic spec whose literal tree covers `lit_syms` (256 is
/// added automatically) and whose distance tree covers `dst_syms`.
fn spec(nlit: usize, ndst: usize, lit_syms: &[usize], dst_syms: &[usize]) -> DynSpec {
    let mut ls = lit_syms.to_vec();
    ls.push(256);
    DynSpec {
        nlit,
        ndst,
        nlen: 4,
        lit_lens: tree_for(nlit, &ls),
        dst_lens: tree_for(ndst, dst_syms),
    }
}

fn check_ok(p: &Pair, label: &str, stream: Vec<u8>, expect: &[u8], out_bytes: usize) {
    let case = Case::new(stream, out_bytes);
    let o = p.check(label, &case);
    match o {
        Outcome::Ret { ret, ref out, .. } => {
            assert_eq!(ret, 1, "[{label}] expected success, out={}", hex(out));
            assert_eq!(
                &out[..expect.len()],
                expect,
                "[{label}] output mismatch vs reference simulation"
            );
            assert!(
                out[expect.len()..].iter().all(|&b| b == 0),
                "[{label}] wrote past the produced size"
            );
        }
        other => panic!("[{label}] expected a normal return, got {other:?}"),
    }
}

/// Differential-only check: the C is the ground truth, so we merely require
/// that both libraries agree.  Used where the C's own behaviour deviates from
/// the DEFLATE spec (e.g. `cp_ptr` mis-points for short stored blocks, see
/// CONFIGS.md rows 1-6) and no independent reference exists.
fn check_diff(p: &Pair, label: &str, stream: Vec<u8>, out_bytes: usize) -> Outcome {
    p.check(label, &Case::new(stream, out_bytes))
}

/// Run one stream at every input alignment 0..4 and every trailing-byte
/// residue (achieved by varying the amount of trailing padding).
fn check_all_shapes(p: &Pair, label: &str, base: &BitWriterOut, expect: &[u8]) {
    for align in 0..4usize {
        for extra in 0..4usize {
            let mut s = base.0.clone();
            for _ in 0..extra {
                s.push(0);
            }
            let case = Case::new(s, expect.len()).align(align);
            let o = p.check(&format!("{label} align={align} extra={extra}"), &case);
            match o {
                Outcome::Ret { ret, ref out, .. } => {
                    assert_eq!(ret, 1, "[{label}] align={align} extra={extra}");
                    assert_eq!(&out[..expect.len()], expect);
                }
                other => panic!("[{label}] align={align} extra={extra}: {other:?}"),
            }
        }
    }
}

pub struct BitWriterOut(pub Vec<u8>);

fn fixed_stream(toks: &[Tok], pad: usize) -> Vec<u8> {
    let mut w = BitWriter::new();
    fixed_block(&mut w, true, toks);
    w.finish(pad)
}

// ===========================================================================
// Rows 1-6: btype 0, stored blocks
// ===========================================================================

#[test]
fn row01_stored_len0() {
    let p = Pair::new();
    let mut w = BitWriter::new();
    stored_block(&mut w, true, &[]);
    let o = check_diff(&p, "row01", w.buf, 0);
    assert!(matches!(o, Outcome::Ret { ret: 1, .. }), "{o:?}");
}

#[test]
fn row02_stored_len1() {
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..64 {
        let payload = rng.bytes(1);
        let mut w = BitWriter::new();
        stored_block(&mut w, true, &payload);
        let o = check_diff(&p, "row02", w.buf, 1);
        assert!(matches!(o, Outcome::Ret { ret: 1, .. }), "{o:?}");
    }
}

#[test]
fn row03_stored_subword() {
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 3);
    for len in [2usize, 3, 4] {
        for _ in 0..32 {
            let payload = rng.bytes(len);
            let mut w = BitWriter::new();
            stored_block(&mut w, true, &payload);
            let o = check_diff(&p, &format!("row03 len={len}"), w.buf, len);
            assert!(matches!(o, Outcome::Ret { ret: 1, .. }), "{o:?}");
        }
    }
}

#[test]
fn row04_stored_multiword() {
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..96 {
        let len = rng.range(5, 4096) as usize;
        let payload = rng.bytes(len);
        let mut w = BitWriter::new();
        stored_block(&mut w, true, &payload);
        let o = check_diff(&p, &format!("row04 len={len}"), w.buf, len);
        // for len % 4 == 1 the C's cp_ptr lands on the payload exactly
        match o {
            Outcome::Ret { ret: 1, ref out, .. } if len % 4 == 1 => {
                assert_eq!(&out[..len], &payload[..], "row04 len={len}");
            }
            Outcome::Ret { ret: 1, .. } => {}
            other => panic!("row04 len={len}: {other:?}"),
        }
    }
}

#[test]
fn row05_stored_misaligned() {
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 5);
    for align in 0..4usize {
        for _ in 0..24 {
            let len = rng.range(0, 300) as usize;
            let payload = rng.bytes(len);
            let mut w = BitWriter::new();
            stored_block(&mut w, true, &payload);
            let case = Case::new(w.buf, len).align(align);
            p.check(&format!("row05 align={align} len={len}"), &case);
        }
    }
}

#[test]
fn row06_stored_out_larger() {
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 6);
    for _ in 0..64 {
        let len = rng.range(0, 512) as usize;
        let payload = rng.bytes(len);
        let mut w = BitWriter::new();
        stored_block(&mut w, true, &payload);
        let _ = payload;
        check_diff(&p, "row06", w.buf, len + rng.range(1, 200) as usize);
    }
}

// ===========================================================================
// Rows 7-20: btype 1, fixed Huffman
// ===========================================================================

#[test]
fn row07_fixed_literal_8bit() {
    let p = Pair::new();
    for b in 0u16..144 {
        let toks = vec![Tok::Lit(b as u8)];
        let expect = simulate(&[], &toks);
        check_ok(&p, &format!("row07 lit={b}"), fixed_stream(&toks, 4), &expect, 1);
    }
}

#[test]
fn row08_fixed_literal_9bit() {
    let p = Pair::new();
    for b in 144u16..256 {
        let toks = vec![Tok::Lit(b as u8)];
        let expect = simulate(&[], &toks);
        check_ok(&p, &format!("row08 lit={b}"), fixed_stream(&toks, 4), &expect, 1);
    }
}

#[test]
fn row09_fixed_literal_runs() {
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 9);
    for _ in 0..96 {
        let n = rng.range(1, 400) as usize;
        let toks: Vec<Tok> = (0..n).map(|_| Tok::Lit(rng.byte())).collect();
        let expect = simulate(&[], &toks);
        check_ok(&p, "row09", fixed_stream(&toks, 4), &expect, expect.len());
    }
}

#[test]
fn row10_fixed_empty_block() {
    let p = Pair::new();
    let toks: Vec<Tok> = vec![];
    check_ok(&p, "row10", fixed_stream(&toks, 4), &[], 0);
    // and with a non-zero out buffer
    check_ok(&p, "row10b", fixed_stream(&toks, 4), &[], 32);
}

#[test]
fn row11_fixed_every_length_symbol() {
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 11);
    // length symbols 257..=285 (indices 0..=28 into cp_len_extra_bits)
    for ls in 0usize..=28 {
        let extra_bits = LEN_EXTRA_BITS[ls] as u32;
        let n_extra = 1u32 << extra_bits;
        for e in 0..n_extra {
            let length = LEN_BASE[ls] + e;
            // prime the window with `dist` literals, then a match with dist > 1
            let dist = rng.range(2, 40) as u32;
            let mut toks: Vec<Tok> =
                (0..dist).map(|_| Tok::Lit(rng.byte().max(1))).collect();
            toks.push(Tok::Raw {
                lsym: 257 + ls,
                lextra: e,
                dsym: dist_to_sym(dist).0,
                dextra: dist_to_sym(dist).1,
            });
            let expect = simulate(&[], &toks);
            check_ok(
                &p,
                &format!("row11 lsym={} len={length} dist={dist}", 257 + ls),
                fixed_stream(&toks, 4),
                &expect,
                expect.len(),
            );
        }
    }
}

#[test]
fn row12_fixed_memset_path() {
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 12);
    for length in [3u32, 4, 5, 17, 63, 128, 257, 258] {
        let b = rng.byte();
        let mut toks = vec![Tok::Lit(b)];
        toks.push(match_tok(length, 1));
        let expect = simulate(&[], &toks);
        check_ok(
            &p,
            &format!("row12 len={length}"),
            fixed_stream(&toks, 4),
            &expect,
            expect.len(),
        );
    }
    for _ in 0..64 {
        let length = rng.range(3, 258) as u32;
        let b = rng.byte();
        let mut toks = vec![Tok::Lit(b)];
        toks.push(match_tok(length, 1));
        let expect = simulate(&[], &toks);
        check_ok(&p, "row12r", fixed_stream(&toks, 4), &expect, expect.len());
    }
}

#[test]
fn row13_fixed_overlapping_match() {
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 13);
    for dist in 2u32..=16 {
        for length in [dist + 1, dist * 2, dist * 3 + 1, 258] {
            if length < 3 {
                continue;
            }
            let prime: Vec<Tok> = (0..dist).map(|_| Tok::Lit(rng.byte())).collect();
            let mut toks = prime.clone();
            toks.push(match_tok(length, dist));
            let expect = simulate(&[], &toks);
            check_ok(
                &p,
                &format!("row13 d={dist} l={length}"),
                fixed_stream(&toks, 4),
                &expect,
                expect.len(),
            );
        }
    }
}

#[test]
fn row14_fixed_every_distance_symbol() {
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 14);
    for ds in 0usize..=29 {
        let eb = DIST_EXTRA_BITS[ds] as u32;
        // sample the extra-bit space (exhaustive for small, random for large)
        let mut samples: Vec<u32> = Vec::new();
        if eb <= 4 {
            samples.extend(0..(1u32 << eb));
        } else {
            samples.push(0);
            samples.push((1u32 << eb) - 1);
            for _ in 0..6 {
                samples.push(rng.below(1u64 << eb) as u32);
            }
        }
        for e in samples {
            let dist = DIST_BASE[ds] + e;
            let prime: Vec<Tok> = (0..dist).map(|_| Tok::Lit(rng.byte())).collect();
            let mut toks = prime;
            toks.push(match_tok(3, dist));
            let expect = simulate(&[], &toks);
            check_ok(
                &p,
                &format!("row14 dsym={ds} dist={dist}"),
                fixed_stream(&toks, 4),
                &expect,
                expect.len(),
            );
        }
    }
}

#[test]
fn row15_fixed_length_symbols_286_287() {
    let p = Pair::new();
    // cp_len_extra_bits[29] = cp_len_extra_bits[30] = 0 and
    // cp_len_base[29] = cp_len_base[30] = 0  =>  a zero-length match.
    for lsym in [286usize, 287] {
        for dist in [1u32, 2, 5] {
            let prime: Vec<Tok> = (0..dist).map(|i| Tok::Lit(b'a' + i as u8)).collect();
            let mut toks = prime;
            let (dsym, dextra) = dist_to_sym(dist);
            toks.push(Tok::Raw {
                lsym,
                lextra: 0,
                dsym,
                dextra,
            });
            toks.push(Tok::Lit(b'Z'));
            // length is 0, so the reference output is just the literals
            let mut expect: Vec<u8> = (0..dist).map(|i| b'a' + i as u8).collect();
            expect.push(b'Z');
            check_ok(
                &p,
                &format!("row15 lsym={lsym} dist={dist}"),
                fixed_stream(&toks, 4),
                &expect,
                expect.len(),
            );
        }
    }
}

#[test]
fn row16_fixed_misaligned() {
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 16);
    for _ in 0..24 {
        let n = rng.range(1, 60) as usize;
        let mut toks: Vec<Tok> = (0..n).map(|_| Tok::Lit(rng.byte())).collect();
        toks.push(match_tok(rng.range(3, 20) as u32, rng.range(1, n as u64) as u32));
        let expect = simulate(&[], &toks);
        let base = BitWriterOut(fixed_stream(&toks, 4));
        check_all_shapes(&p, "row16", &base, &expect);
    }
}

#[test]
fn row17_fixed_word_count_zero() {
    let p = Pair::new();
    // Force word_count == 0: place `in` so that first_bytes covers most of the
    // stream and keep the stream short (<= first_bytes + 3 bytes).
    for align in 1..4usize {
        for pad in 0..3usize {
            let toks = vec![Tok::Lit(b'x')];
            let expect = simulate(&[], &toks);
            let mut s = fixed_stream(&toks, 1);
            s.truncate(3);
            for _ in 0..pad {
                s.push(0);
            }
            let case = Case::new(s, expect.len()).align(align);
            let o = p.check(&format!("row17 align={align} pad={pad}"), &case);
            // whatever it is (success or abort) both must agree; record it
            let _ = o;
        }
    }
    // a properly sized short stream whose whole body fits in first_bytes+final
    let toks = vec![Tok::Lit(b'x')];
    let expect = simulate(&[], &toks);
    let s = fixed_stream(&toks, 4);
    for align in 0..4usize {
        let case = Case::new(s.clone(), expect.len()).align(align);
        let o = p.check(&format!("row17b align={align}"), &case);
        match o {
            Outcome::Ret { ret, ref out, .. } => {
                assert_eq!(ret, 1);
                assert_eq!(&out[..expect.len()], &expect[..]);
            }
            other => panic!("row17b align={align}: {other:?}"),
        }
    }
}

#[test]
fn row18_fixed_final_word_refill() {
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 18);
    // vary the total stream length modulo 4 so that last_bytes takes every
    // value and the final-word refill branch of cp_peak_bits is exercised
    for _ in 0..64 {
        let n = rng.range(1, 80) as usize;
        let toks: Vec<Tok> = (0..n).map(|_| Tok::Lit(rng.byte())).collect();
        let expect = simulate(&[], &toks);
        for pad in 4..8usize {
            let s = fixed_stream(&toks, pad);
            for align in 0..4usize {
                let case = Case::new(s.clone(), expect.len()).align(align);
                let o = p.check(&format!("row18 pad={pad} align={align}"), &case);
                match o {
                    Outcome::Ret { ret, ref out, .. } => {
                        assert_eq!(ret, 1);
                        assert_eq!(&out[..expect.len()], &expect[..]);
                    }
                    other => panic!("row18: {other:?}"),
                }
            }
        }
    }
}

#[test]
fn row19_fixed_out_exact() {
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 19);
    for _ in 0..64 {
        let n = rng.range(1, 200) as usize;
        let toks: Vec<Tok> = (0..n).map(|_| Tok::Lit(rng.byte())).collect();
        let expect = simulate(&[], &toks);
        check_ok(&p, "row19", fixed_stream(&toks, 4), &expect, expect.len());
    }
}

#[test]
fn row20_fixed_random_mixture() {
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 20);
    for _ in 0..256 {
        let mut toks: Vec<Tok> = Vec::new();
        let mut produced = 0usize;
        let steps = rng.range(1, 60);
        for _ in 0..steps {
            if produced >= 3 && rng.below(3) == 0 {
                let dist = rng.range(1, produced as u64) as u32;
                let length = rng.range(3, 258) as u32;
                toks.push(match_tok(length, dist));
                produced += length as usize;
            } else {
                toks.push(Tok::Lit(rng.byte()));
                produced += 1;
            }
        }
        let expect = simulate(&[], &toks);
        check_ok(&p, "row20", fixed_stream(&toks, 4), &expect, expect.len());
    }
}

// ===========================================================================
// Rows 21-36: btype 2, dynamic Huffman
// ===========================================================================

/// Run-length encode a code-length array into a `CL` program, optionally using
/// opcodes 16 / 17 / 18.  Never emits opcode 16 first (that would make the C
/// read `lens[-1]`, see CONFIGS.md "Excluded").
fn rle_prog(lens: &[u8], use16: bool, use17: bool, use18: bool) -> Vec<CL> {
    let mut out: Vec<CL> = Vec::new();
    let n = lens.len();
    let mut i = 0usize;
    while i < n {
        let v = lens[i];
        let mut r = 1usize;
        while i + r < n && lens[i + r] == v {
            r += 1;
        }
        if v == 0 {
            let mut rem = r;
            while rem > 0 {
                if use18 && rem >= 11 {
                    let t = rem.min(138);
                    out.push(CL::Z11(t as u32));
                    rem -= t;
                } else if use17 && rem >= 3 {
                    let t = rem.min(10);
                    out.push(CL::Z3(t as u32));
                    rem -= t;
                } else {
                    out.push(CL::Lit(0));
                    rem -= 1;
                }
            }
        } else {
            out.push(CL::Lit(v));
            let mut rem = r - 1;
            while rem > 0 {
                if use16 && rem >= 3 {
                    let t = rem.min(6);
                    out.push(CL::Rep(t as u32));
                    rem -= t;
                } else {
                    out.push(CL::Lit(v));
                    rem -= 1;
                }
            }
        }
        i += r;
    }
    out
}

fn prog_symbols(prog: &[CL]) -> Vec<usize> {
    let mut v: Vec<usize> = Vec::new();
    for c in prog {
        v.push(match *c {
            CL::Lit(l) => l as usize,
            CL::Rep(_) => 16,
            CL::Z3(_) => 17,
            CL::Z11(_) => 18,
        });
    }
    v.sort_unstable();
    v.dedup();
    v
}

struct DynBuild {
    dynamic: Dynamic,
    #[allow(dead_code)]
    lit: Huff,
    #[allow(dead_code)]
    dst: Huff,
}

fn build_dyn(
    nlit: usize,
    ndst: usize,
    nlen_min: usize,
    lit_lens: &[u8],
    dst_lens: &[u8],
    use16: bool,
    use17: bool,
    use18: bool,
) -> DynBuild {
    assert_eq!(lit_lens.len(), nlit);
    assert_eq!(dst_lens.len(), ndst);
    let mut flat = lit_lens.to_vec();
    flat.extend_from_slice(dst_lens);
    let prog = rle_prog(&flat, use16, use17, use18);
    assert_eq!(expand(&prog), flat, "RLE program does not reproduce lens");
    let used = prog_symbols(&prog);
    let nlen = nlen_min.max(min_nlen(&used));
    DynBuild {
        dynamic: Dynamic {
            nlit,
            ndst,
            nlen,
            lenlens: cl_lenlens(&used, nlen),
            prog,
        },
        lit: Huff::new(lit_lens.to_vec()),
        dst: Huff::new(dst_lens.to_vec()),
    }
}

fn dyn_for(nlit: usize, ndst: usize, lit_syms: &[usize], dst_syms: &[usize]) -> DynBuild {
    let mut ls = lit_syms.to_vec();
    ls.push(256);
    let lit_lens = tree_for(nlit, &ls);
    let dst_lens = if dst_syms.is_empty() {
        vec![0u8; ndst]
    } else {
        tree_for(ndst, dst_syms)
    };
    build_dyn(nlit, ndst, 4, &lit_lens, &dst_lens, false, false, false)
}

fn dyn_stream(b: &DynBuild, toks: &[Tok], pad: usize) -> Vec<u8> {
    let mut w = BitWriter::new();
    b.dynamic.write(&mut w, true, toks);
    w.finish(pad)
}

/// A `count`-symbol tree over the 288-wide literal alphabet, all codes of
/// length `l`, always including symbol 256.
fn flat_lit_tree(count: usize, l: u8) -> (Vec<u8>, Vec<usize>) {
    let mut lit_lens = vec![0u8; 288];
    let mut targets: Vec<usize> = vec![256];
    let mut t = 0usize;
    while targets.len() < count {
        if t != 256 {
            targets.push(t);
        }
        t += 1;
    }
    targets.sort_unstable();
    for &s in &targets {
        lit_lens[s] = l;
    }
    (lit_lens, targets)
}

/// A literal tree whose longest code is exactly `k` (lengths 1,2,..,k-1,k,k).
fn chain_tree(k: u8) -> (Vec<u8>, Vec<usize>) {
    let chain = chain_lens(k);
    let mut lit_lens = vec![0u8; 288];
    let mut targets: Vec<usize> = vec![256];
    let mut t = 0usize;
    while targets.len() < chain.len() {
        if t != 256 {
            targets.push(t);
        }
        t += 1;
    }
    targets.sort_unstable();
    for (i, &s) in targets.iter().enumerate() {
        lit_lens[s] = chain[i];
    }
    (lit_lens, targets)
}

#[test]
fn row21_dynamic_hlit_min() {
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 21);
    let syms: Vec<usize> = (b'a' as usize..b'a' as usize + 7).collect();
    let b = dyn_for(257, 1, &syms, &[0]);
    for _ in 0..32 {
        let n = rng.range(1, 120) as usize;
        let toks: Vec<Tok> = (0..n)
            .map(|_| Tok::Lit(syms[rng.below(syms.len() as u64) as usize] as u8))
            .collect();
        let expect = simulate(&[], &toks);
        check_ok(&p, "row21", dyn_stream(&b, &toks, 4), &expect, expect.len());
    }
}

#[test]
fn row22_dynamic_hlit_max() {
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 22);
    let mut ls: Vec<usize> = (0..8).map(|i| b'A' as usize + i).collect();
    ls.push(285);
    ls.push(287);
    let b = dyn_for(288, 30, &ls, &[0, 1, 3, 7]);
    for _ in 0..32 {
        let mut toks: Vec<Tok> = Vec::new();
        let mut produced = 0usize;
        for _ in 0..rng.range(1, 30) {
            toks.push(Tok::Lit(b'A' + rng.below(8) as u8));
            produced += 1;
        }
        if produced >= 4 {
            toks.push(Tok::Raw {
                lsym: 285,
                lextra: 0,
                dsym: 3,
                dextra: 0,
            });
        }
        let expect = simulate(&[], &toks);
        check_ok(&p, "row22", dyn_stream(&b, &toks, 4), &expect, expect.len());
    }
}

#[test]
fn row23_dynamic_hdist_min() {
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 23);
    let ls: Vec<usize> = vec![b'q' as usize, b'r' as usize, 257, 260, 264];
    let b = dyn_for(288, 1, &ls, &[0]);
    for _ in 0..48 {
        let mut toks: Vec<Tok> = vec![Tok::Lit(b'q')];
        for _ in 0..rng.range(1, 6) {
            let lsym = [257usize, 260, 264][rng.below(3) as usize];
            let eb = LEN_EXTRA_BITS[lsym - 257] as u32;
            let lextra = rng.below(1u64 << eb) as u32;
            toks.push(Tok::Raw {
                lsym,
                lextra,
                dsym: 0,
                dextra: 0,
            });
        }
        let expect = simulate(&[], &toks);
        check_ok(&p, "row23", dyn_stream(&b, &toks, 4), &expect, expect.len());
    }
}

#[test]
fn row24_dynamic_hdist_max() {
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 24);
    let ls: Vec<usize> = vec![b'x' as usize, b'y' as usize, 257, 265, 270, 277];
    let dsyms: Vec<usize> = (0..16).collect();
    let b = dyn_for(288, 32, &ls, &dsyms);
    for _ in 0..48 {
        let mut toks: Vec<Tok> = Vec::new();
        let mut produced = 0usize;
        for _ in 0..600 {
            toks.push(Tok::Lit(if rng.below(2) == 0 { b'x' } else { b'y' }));
            produced += 1;
        }
        for _ in 0..rng.range(1, 8) {
            let dsym = rng.below(16) as usize;
            let deb = DIST_EXTRA_BITS[dsym] as u32;
            let dextra = rng.below(1u64 << deb) as u32;
            let dist = DIST_BASE[dsym] + dextra;
            if dist as usize > produced {
                continue;
            }
            let lsym = [257usize, 265, 270, 277][rng.below(4) as usize];
            let eb = LEN_EXTRA_BITS[lsym - 257] as u32;
            let lextra = rng.below(1u64 << eb) as u32;
            produced += (LEN_BASE[lsym - 257] + lextra) as usize;
            toks.push(Tok::Raw {
                lsym,
                lextra,
                dsym,
                dextra,
            });
        }
        let expect = simulate(&[], &toks);
        check_ok(&p, "row24", dyn_stream(&b, &toks, 4), &expect, expect.len());
    }
}

#[test]
fn row25_dynamic_hclen_min() {
    // HCLEN = 4 transmits only cp_permutation_order[0..4] = {16,17,18,0}, so
    // every literal/distance code length must be 0.  The literal tree is then
    // empty (nlit == 0) and cp_decode reads tree[-1] -- purely differential.
    let p = Pair::new();
    let nlit = 257usize;
    let ndst = 1usize;
    let total = nlit + ndst;
    let mut prog: Vec<CL> = Vec::new();
    let mut left = total;
    while left >= 11 {
        let t = left.min(138);
        prog.push(CL::Z11(t as u32));
        left -= t;
    }
    while left > 0 {
        if left >= 3 {
            let t = left.min(10);
            prog.push(CL::Z3(t as u32));
            left -= t;
        } else {
            prog.push(CL::Z3(3));
            left = 0;
        }
    }
    let used = prog_symbols(&prog);
    let d = Dynamic {
        nlit,
        ndst,
        nlen: 4,
        lenlens: cl_lenlens(&used, 4),
        prog,
    };
    let mut w = BitWriter::new();
    w.bits(1, 1);
    w.bits(2, 2);
    w.bits((d.nlit - 257) as u32, 5);
    w.bits((d.ndst - 1) as u32, 5);
    w.bits((d.nlen - 4) as u32, 4);
    for i in 0..d.nlen {
        w.bits(d.lenlens[PERMUTATION_ORDER[i] as usize] as u32, 3);
    }
    let clh = Huff::new(d.lenlens.to_vec());
    for c in &d.prog {
        match *c {
            CL::Lit(l) => clh.put(&mut w, l as usize),
            CL::Rep(n) => {
                clh.put(&mut w, 16);
                w.bits(n - 3, 2);
            }
            CL::Z3(n) => {
                clh.put(&mut w, 17);
                w.bits(n - 3, 3);
            }
            CL::Z11(n) => {
                clh.put(&mut w, 18);
                w.bits(n - 11, 7);
            }
        }
    }
    let stream = w.finish(8);
    p.check("row25", &Case::new(stream, 32));
}

#[test]
fn row26_dynamic_hclen_max() {
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 26);
    // 14 distinct code-length values => all 19 CL symbols must be transmitted
    let (lit_lens, targets) = chain_tree(14);
    let dst_lens = tree_for(32, &[0, 1, 2, 3]);
    let b = build_dyn(288, 32, 19, &lit_lens, &dst_lens, true, true, true);
    assert_eq!(b.dynamic.nlen, 19, "expected a full HCLEN");
    let avail: Vec<usize> = targets.iter().cloned().filter(|&s| s < 256).collect();
    for _ in 0..32 {
        let n = rng.range(1, 80) as usize;
        let toks: Vec<Tok> = (0..n)
            .map(|_| Tok::Lit(avail[rng.below(avail.len() as u64) as usize] as u8))
            .collect();
        let expect = simulate(&[], &toks);
        check_ok(&p, "row26", dyn_stream(&b, &toks, 4), &expect, expect.len());
    }
}

#[test]
fn row27_dynamic_hclen_range() {
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 27);
    let mut covered: Vec<usize> = Vec::new();
    for nlen in 5..=19usize {
        let allowed: Vec<usize> = (0..nlen).map(|i| PERMUTATION_ORDER[i] as usize).collect();
        // pick the smallest usable code length that is transmittable
        let l = match allowed.iter().cloned().filter(|&s| (1..=9).contains(&s)).min() {
            Some(l) => l as u8,
            None => continue,
        };
        let count = 1usize << l;
        if count > 288 {
            continue;
        }
        let (lit_lens, targets) = flat_lit_tree(count, l);
        let dst_lens = vec![0u8; 1];
        let b = build_dyn(288, 1, nlen, &lit_lens, &dst_lens, false, true, true);
        assert!(b.dynamic.nlen >= nlen);
        covered.push(b.dynamic.nlen);
        let avail: Vec<usize> = targets.iter().cloned().filter(|&s| s < 256).collect();
        for _ in 0..8 {
            let n = rng.range(1, 40) as usize;
            let toks: Vec<Tok> = (0..n)
                .map(|_| Tok::Lit(avail[rng.below(avail.len() as u64) as usize] as u8))
                .collect();
            let expect = simulate(&[], &toks);
            check_ok(
                &p,
                &format!("row27 nlen={}", b.dynamic.nlen),
                dyn_stream(&b, &toks, 4),
                &expect,
                expect.len(),
            );
        }
    }
    assert!(covered.len() >= 10, "only covered {covered:?}");
}

/// Shared body for the code-length-opcode rows.
fn opcode_row(p: &Pair, label: &str, use16: bool, use17: bool, use18: bool, seed: u64) {
    let mut rng = Rng::new(seed);
    // 256 symbols of length 8 => a complete code.  The symbols are chosen so
    // that the flattened code-length array contains long runs of equal lengths
    // (opcode 16), a SHORT run of zeros (opcode 17, 5 zeros at 128..132) and
    // long runs of zeros (opcode 18).
    let mut lit_lens = vec![0u8; 288];
    let mut targets: Vec<usize> = Vec::new();
    for s in 0..=127usize {
        targets.push(s);
    }
    for s in 133..=260usize {
        targets.push(s);
    }
    assert_eq!(targets.len(), 256);
    assert!(targets.contains(&256));
    for &s in &targets {
        lit_lens[s] = 8;
    }
    let mut dst_lens = vec![0u8; 32];
    dst_lens[0] = 1;
    dst_lens[1] = 1;
    let b = build_dyn(288, 32, 4, &lit_lens, &dst_lens, use16, use17, use18);
    let has = |want: u8| {
        b.dynamic.prog.iter().any(|c| match (c, want) {
            (CL::Rep(_), 16) => true,
            (CL::Z3(_), 17) => true,
            (CL::Z11(_), 18) => true,
            _ => false,
        })
    };
    if use16 {
        assert!(has(16), "[{label}] opcode 16 not exercised");
    }
    if use17 {
        assert!(has(17), "[{label}] opcode 17 not exercised");
    }
    if use18 {
        assert!(has(18), "[{label}] opcode 18 not exercised");
    }
    let avail: Vec<usize> = targets.iter().cloned().filter(|&s| s < 256).collect();
    for _ in 0..32 {
        let mut toks: Vec<Tok> = Vec::new();
        let mut produced = 0usize;
        for _ in 0..rng.range(1, 60) {
            toks.push(Tok::Lit(
                avail[rng.below(avail.len() as u64) as usize] as u8,
            ));
            produced += 1;
        }
        if produced >= 2 {
            toks.push(Tok::Raw {
                lsym: 257,
                lextra: 0,
                dsym: 1,
                dextra: 0,
            });
        }
        let expect = simulate(&[], &toks);
        check_ok(p, label, dyn_stream(&b, &toks, 4), &expect, expect.len());
    }
}

#[test]
fn row28_dynamic_opcode16() {
    let p = Pair::new();
    opcode_row(&p, "row28", true, false, false, SEED ^ 28);
}

#[test]
fn row29_dynamic_opcode17() {
    let p = Pair::new();
    opcode_row(&p, "row29", false, true, false, SEED ^ 29);
}

#[test]
fn row30_dynamic_opcode18() {
    let p = Pair::new();
    opcode_row(&p, "row30", false, false, true, SEED ^ 30);
}

#[test]
fn row30b_dynamic_all_opcodes() {
    let p = Pair::new();
    opcode_row(&p, "row30b", true, true, true, SEED ^ 130);
}

#[test]
fn row31_dynamic_lens_up_to_9() {
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 31);
    let (lit_lens, targets) = chain_tree(9);
    assert_eq!(*lit_lens.iter().max().unwrap(), 9);
    let dst_lens = tree_for(4, &[0, 1]);
    let b = build_dyn(288, 4, 4, &lit_lens, &dst_lens, true, true, true);
    let avail: Vec<usize> = targets.iter().cloned().filter(|&s| s < 256).collect();
    for _ in 0..48 {
        let n = rng.range(1, 100) as usize;
        let toks: Vec<Tok> = (0..n)
            .map(|_| Tok::Lit(avail[rng.below(avail.len() as u64) as usize] as u8))
            .collect();
        let expect = simulate(&[], &toks);
        check_ok(&p, "row31", dyn_stream(&b, &toks, 4), &expect, expect.len());
    }
}

#[test]
fn row32_dynamic_lens_up_to_14() {
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 32);
    for k in 10u8..=14 {
        let (lit_lens, targets) = chain_tree(k);
        assert_eq!(*lit_lens.iter().max().unwrap(), k);
        let dst_lens = tree_for(4, &[0, 1]);
        let b = build_dyn(288, 4, 4, &lit_lens, &dst_lens, true, true, true);
        let avail: Vec<usize> = targets.iter().cloned().filter(|&s| s < 256).collect();
        for _ in 0..24 {
            let n = rng.range(1, 80) as usize;
            let toks: Vec<Tok> = (0..n)
                .map(|_| Tok::Lit(avail[rng.below(avail.len() as u64) as usize] as u8))
                .collect();
            let expect = simulate(&[], &toks);
            check_ok(
                &p,
                &format!("row32 k={k}"),
                dyn_stream(&b, &toks, 4),
                &expect,
                expect.len(),
            );
        }
    }
}

#[test]
fn row33_dynamic_distance_symbols_30_31() {
    // cp_dist_base[30] = cp_dist_base[31] = 0 => distance 0 => src == dst.
    // Purely differential (no spec behaviour to model).
    let p = Pair::new();
    for dsym in [30usize, 31] {
        let other = if dsym == 30 { 31 } else { 30 };
        let ls: Vec<usize> = vec![b'k' as usize, 257, 260];
        let b = dyn_for(288, 32, &ls, &[0, 1, dsym, other]);
        for lsym in [257usize, 260] {
            let mut toks: Vec<Tok> = (0..8).map(|_| Tok::Lit(b'k')).collect();
            toks.push(Tok::Raw {
                lsym,
                lextra: 0,
                dsym,
                dextra: 0,
            });
            toks.push(Tok::Lit(b'k'));
            check_diff(
                &p,
                &format!("row33 dsym={dsym} lsym={lsym}"),
                dyn_stream(&b, &toks, 4),
                512,
            );
        }
    }
}

#[test]
fn row34_dynamic_misaligned() {
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 34);
    let ls: Vec<usize> = (0..6).map(|i| b'm' as usize + i).chain([257usize]).collect();
    let b = dyn_for(288, 8, &ls, &[0, 1, 2, 3]);
    for _ in 0..16 {
        let n = rng.range(4, 40) as usize;
        let lits: Vec<usize> = (0..6).map(|i| b'm' as usize + i).collect();
        let mut toks: Vec<Tok> = (0..n)
            .map(|_| Tok::Lit(lits[rng.below(lits.len() as u64) as usize] as u8))
            .collect();
        toks.push(Tok::Raw {
            lsym: 257,
            lextra: 0,
            dsym: 2,
            dextra: 0,
        });
        let expect = simulate(&[], &toks);
        let base = BitWriterOut(dyn_stream(&b, &toks, 4));
        check_all_shapes(&p, "row34", &base, &expect);
    }
}

#[test]
fn row35_dynamic_randomized() {
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 35);
    for iter in 0..256 {
        let m = rng.range(2, 40) as usize;
        let mut syms: Vec<usize> = vec![256];
        let mut guard = 0;
        while syms.len() < m && guard < 500 {
            guard += 1;
            let s = rng.below(256) as usize;
            if !syms.contains(&s) {
                syms.push(s);
            }
        }
        let use_matches = rng.below(2) == 0;
        let lsyms: Vec<usize> = if use_matches {
            let k = rng.range(1, 4) as usize;
            let mut v: Vec<usize> = Vec::new();
            while v.len() < k {
                let s = 257 + rng.below(29) as usize;
                if !v.contains(&s) {
                    v.push(s);
                }
            }
            v
        } else {
            vec![]
        };
        for &s in &lsyms {
            if !syms.contains(&s) {
                syms.push(s);
            }
        }
        syms.sort_unstable();
        syms.dedup();
        let mut lens = flat_lens(syms.len());
        for i in (1..lens.len()).rev() {
            let j = rng.below((i + 1) as u64) as usize;
            lens.swap(i, j);
        }
        let lit_lens = place(288, &syms, &lens);
        let dsyms: Vec<usize> = {
            let k = rng.range(1, 8) as usize;
            let mut v: Vec<usize> = Vec::new();
            while v.len() < k {
                let s = rng.below(30) as usize;
                if !v.contains(&s) {
                    v.push(s);
                }
            }
            v.sort_unstable();
            v
        };
        let dst_lens = tree_for(32, &dsyms);
        let b = build_dyn(
            288,
            32,
            4,
            &lit_lens,
            &dst_lens,
            rng.below(2) == 0,
            rng.below(2) == 0,
            rng.below(2) == 0,
        );
        let lit_avail: Vec<usize> = syms.iter().cloned().filter(|&s| s < 256).collect();
        if lit_avail.is_empty() {
            continue;
        }
        let mut toks: Vec<Tok> = Vec::new();
        let mut produced = 0usize;
        for _ in 0..rng.range(1, 80) {
            if use_matches && produced >= 3 && rng.below(4) == 0 {
                let lsym = lsyms[rng.below(lsyms.len() as u64) as usize];
                let eb = LEN_EXTRA_BITS[lsym - 257] as u32;
                let lextra = rng.below(1u64 << eb) as u32;
                let length = LEN_BASE[lsym - 257] + lextra;
                let cands: Vec<usize> = dsyms
                    .iter()
                    .cloned()
                    .filter(|&d| (DIST_BASE[d] as usize) <= produced)
                    .collect();
                if cands.is_empty() {
                    continue;
                }
                let dsym = cands[rng.below(cands.len() as u64) as usize];
                let deb = DIST_EXTRA_BITS[dsym] as u32;
                let room = (produced as u32) - DIST_BASE[dsym];
                let maxe = room.min((1u32 << deb) - 1);
                let dextra = rng.below(maxe as u64 + 1) as u32;
                toks.push(Tok::Raw {
                    lsym,
                    lextra,
                    dsym,
                    dextra,
                });
                produced += length as usize;
            } else {
                toks.push(Tok::Lit(
                    lit_avail[rng.below(lit_avail.len() as u64) as usize] as u8,
                ));
                produced += 1;
            }
        }
        let expect = simulate(&[], &toks);
        check_ok(
            &p,
            &format!("row35 iter={iter}"),
            dyn_stream(&b, &toks, 4),
            &expect,
            expect.len(),
        );
    }
}

#[test]
fn row36_real_deflate_streams() {
    use flate2::write::DeflateEncoder;
    use flate2::Compression;
    use std::io::Write;

    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 36);
    let mut checked = 0usize;
    for level in 0..=9u32 {
        for shape in 0..6 {
            let data: Vec<u8> = match shape {
                0 => Vec::new(),
                1 => vec![b'a'; rng.range(1, 500) as usize],
                2 => { let n = rng.range(1, 4000) as usize; rng.bytes(n) }
                3 => {
                    let pat = rng.bytes(7);
                    (0..rng.range(50, 3000) as usize)
                        .map(|i| pat[i % pat.len()])
                        .collect()
                }
                4 => (0..rng.range(1, 2000) as usize)
                    .map(|i| (i % 251) as u8)
                    .collect(),
                _ => {
                    let mut v: Vec<u8> = Vec::new();
                    for _ in 0..rng.range(1, 60) {
                        let b = rng.byte();
                        for _ in 0..rng.range(1, 40) {
                            v.push(b);
                        }
                    }
                    v
                }
            };
            let mut e = DeflateEncoder::new(Vec::new(), Compression::new(level));
            e.write_all(&data).unwrap();
            let mut stream = e.finish().unwrap();
            for _ in 0..4 {
                stream.push(0);
            }
            for align in 0..4usize {
                let case = Case::new(stream.clone(), data.len().max(1)).align(align);
                p.check(
                    &format!(
                        "row36 level={level} shape={shape} align={align} n={}",
                        data.len()
                    ),
                    &case,
                );
                checked += 1;
            }
        }
    }
    assert!(checked >= 200, "only {checked} real streams checked");
}

// ===========================================================================
// Rows 37-40: bfinal == 0, multi-block streams
// ===========================================================================

#[test]
fn row37_multiblock_two_fixed() {
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 37);
    for _ in 0..64 {
        let a: Vec<Tok> = (0..rng.range(1, 40)).map(|_| Tok::Lit(rng.byte())).collect();
        let bb: Vec<Tok> = (0..rng.range(1, 40)).map(|_| Tok::Lit(rng.byte())).collect();
        let mut w = BitWriter::new();
        fixed_block(&mut w, false, &a);
        fixed_block(&mut w, true, &bb);
        let stream = w.finish(4);
        let first = simulate(&[], &a);
        let expect = simulate(&first, &bb);
        check_ok(&p, "row37", stream, &expect, expect.len());
    }
}

#[test]
fn row38_multiblock_fixed_dynamic_fixed() {
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 38);
    let ls: Vec<usize> = (0..6).map(|i| b'p' as usize + i).collect();
    let b = dyn_for(288, 4, &ls, &[0, 1]);
    for _ in 0..48 {
        let a: Vec<Tok> = (0..rng.range(1, 25)).map(|_| Tok::Lit(rng.byte())).collect();
        let mid: Vec<Tok> = (0..rng.range(1, 25))
            .map(|_| Tok::Lit(ls[rng.below(ls.len() as u64) as usize] as u8))
            .collect();
        let c: Vec<Tok> = (0..rng.range(1, 25)).map(|_| Tok::Lit(rng.byte())).collect();
        let mut w = BitWriter::new();
        fixed_block(&mut w, false, &a);
        b.dynamic.write(&mut w, false, &mid);
        fixed_block(&mut w, true, &c);
        let stream = w.finish(4);
        let e1 = simulate(&[], &a);
        let e2 = simulate(&e1, &mid);
        let expect = simulate(&e2, &c);
        check_ok(&p, "row38", stream, &expect, expect.len());
    }
}

#[test]
fn row39_multiblock_last_stored() {
    // A stored block is only legal as the last thing in the stream (the
    // `bits_left/8 <= LEN` check).  Purely differential: the C's cp_ptr
    // arithmetic does not point at the payload for most shapes.
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 39);
    for _ in 0..64 {
        let a: Vec<Tok> = (0..rng.range(1, 30)).map(|_| Tok::Lit(rng.byte())).collect();
        let pn = rng.range(0, 200) as usize;
        let payload = rng.bytes(pn);
        let mut w = BitWriter::new();
        fixed_block(&mut w, false, &a);
        stored_block(&mut w, true, &payload);
        let stream = w.buf;
        let out_len = simulate(&[], &a).len() + payload.len() + 16;
        check_diff(&p, "row39", stream, out_len);
    }
}

#[test]
fn row40_multiblock_cross_reference() {
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 40);
    for _ in 0..64 {
        let n = rng.range(4, 60) as usize;
        let a: Vec<Tok> = (0..n).map(|_| Tok::Lit(rng.byte())).collect();
        let first = simulate(&[], &a);
        let dist = rng.range(1, first.len() as u64) as u32;
        let length = rng.range(3, 258) as u32;
        let bb: Vec<Tok> = vec![match_tok(length, dist), Tok::Lit(rng.byte())];
        let mut w = BitWriter::new();
        fixed_block(&mut w, false, &a);
        fixed_block(&mut w, true, &bb);
        let stream = w.finish(4);
        let expect = simulate(&first, &bb);
        check_ok(&p, "row40", stream, &expect, expect.len());
    }
}

// ===========================================================================
// Rows 41-48: the exported data symbols as an entry point
// ===========================================================================

#[test]
fn row41_exported_tables_identical() {
    let p = Pair::new();
    unsafe {
        assert_eq!(
            std::slice::from_raw_parts(p.c.fixed_table, 320),
            std::slice::from_raw_parts(p.r.fixed_table, 320),
            "cp_fixed_table"
        );
        assert_eq!(
            std::slice::from_raw_parts(p.c.fixed_table, 320),
            &FIXED_TABLE,
            "cp_fixed_table vs C source"
        );
        assert_eq!(
            std::slice::from_raw_parts(p.c.permutation_order, 19),
            std::slice::from_raw_parts(p.r.permutation_order, 19),
            "cp_permutation_order"
        );
        assert_eq!(
            std::slice::from_raw_parts(p.c.permutation_order, 19),
            &PERMUTATION_ORDER
        );
        assert_eq!(
            std::slice::from_raw_parts(p.c.len_extra_bits, 31),
            std::slice::from_raw_parts(p.r.len_extra_bits, 31),
            "cp_len_extra_bits"
        );
        assert_eq!(
            std::slice::from_raw_parts(p.c.len_extra_bits, 31),
            &LEN_EXTRA_BITS
        );
        assert_eq!(
            std::slice::from_raw_parts(p.c.dist_extra_bits, 32),
            std::slice::from_raw_parts(p.r.dist_extra_bits, 32),
            "cp_dist_extra_bits"
        );
        assert_eq!(
            std::slice::from_raw_parts(p.c.dist_extra_bits, 32),
            &DIST_EXTRA_BITS
        );
        assert_eq!(
            std::slice::from_raw_parts(p.c.len_base, 31),
            std::slice::from_raw_parts(p.r.len_base, 31),
            "cp_len_base"
        );
        assert_eq!(std::slice::from_raw_parts(p.c.len_base, 31), &LEN_BASE);
        assert_eq!(
            std::slice::from_raw_parts(p.c.dist_base, 32),
            std::slice::from_raw_parts(p.r.dist_base, 32),
            "cp_dist_base"
        );
        assert_eq!(std::slice::from_raw_parts(p.c.dist_base, 32), &DIST_BASE);
        assert!((*p.c.error_reason).is_null());
        assert!((*p.r.error_reason).is_null());
    }
}

#[test]
fn row42_patch_len_base() {
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 42);
    for _ in 0..48 {
        let patch: Vec<(usize, u32)> = (0..29)
            .map(|i| (i, LEN_BASE[i] + rng.range(0, 5) as u32))
            .collect();
        let mut toks: Vec<Tok> = (0..20).map(|_| Tok::Lit(rng.byte())).collect();
        toks.push(match_tok(rng.range(3, 100) as u32, rng.range(2, 20) as u32));
        let stream = fixed_stream(&toks, 4);
        let case = Case::new(stream, 1024).patch(Patch::LenBase(patch));
        p.check("row42", &case);
    }
}

#[test]
fn row43_patch_len_extra_bits() {
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 43);
    for _ in 0..48 {
        let patch: Vec<(usize, u8)> = (0..29)
            .map(|i| (i, (LEN_EXTRA_BITS[i] as u64 + rng.range(0, 2)) as u8))
            .collect();
        let mut toks: Vec<Tok> = (0..20).map(|_| Tok::Lit(rng.byte())).collect();
        toks.push(match_tok(rng.range(3, 60) as u32, rng.range(2, 20) as u32));
        toks.push(Tok::Lit(b'!'));
        let stream = fixed_stream(&toks, 8);
        let case = Case::new(stream, 1024).patch(Patch::LenExtraBits(patch));
        p.check("row43", &case);
    }
}

#[test]
fn row44_patch_dist_tables() {
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 44);
    for _ in 0..48 {
        let dpatch: Vec<(usize, u32)> = (0..30)
            .map(|i| (i, DIST_BASE[i] + rng.range(0, 3) as u32))
            .collect();
        let epatch: Vec<(usize, u8)> = (0..30)
            .map(|i| (i, (DIST_EXTRA_BITS[i] as u64 + rng.range(0, 2)) as u8))
            .collect();
        let mut toks: Vec<Tok> = (0..40).map(|_| Tok::Lit(rng.byte())).collect();
        toks.push(match_tok(rng.range(3, 40) as u32, rng.range(2, 30) as u32));
        let stream = fixed_stream(&toks, 8);
        let case = Case::new(stream, 1024)
            .patch(Patch::DistBase(dpatch))
            .patch(Patch::DistExtraBits(epatch));
        p.check("row44", &case);
    }
}

#[test]
fn row45_patch_permutation_order() {
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 45);
    for _ in 0..32 {
        let i = rng.below(19) as usize;
        let j = rng.below(19) as usize;
        let mut order = PERMUTATION_ORDER;
        order.swap(i, j);
        let ls: Vec<usize> = (0..6).map(|k| b'w' as usize + k).collect();
        let b = dyn_for(288, 4, &ls, &[0, 1]);
        let toks: Vec<Tok> = (0..rng.range(1, 30))
            .map(|_| Tok::Lit(ls[rng.below(ls.len() as u64) as usize] as u8))
            .collect();
        let mut w = BitWriter::new();
        b.dynamic.write_ord(&mut w, true, &toks, &order);
        let stream = w.finish(4);
        let patch: Vec<(usize, u8)> = (0..19).map(|k| (k, order[k])).collect();
        let expect = simulate(&[], &toks);
        let case = Case::new(stream, expect.len()).patch(Patch::PermutationOrder(patch));
        let o = p.check(&format!("row45 swap({i},{j})"), &case);
        // the stream and the patched order are consistent, so it must decode
        if let Outcome::Ret { ret: 1, ref out, .. } = o {
            assert_eq!(&out[..expect.len()], &expect[..], "row45 swap({i},{j})");
        }
    }
}

#[test]
fn row46_patch_fixed_table() {
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 46);
    for _ in 0..32 {
        // permute the literal part's length multiset: still a complete code,
        // but a different symbol -> code assignment
        let mut lens: Vec<u8> = FIXED_TABLE[..288].to_vec();
        for k in (1..288).rev() {
            let j = rng.below((k + 1) as u64) as usize;
            lens.swap(k, j);
        }
        let patch: Vec<(usize, u8)> = (0..288).map(|k| (k, lens[k])).collect();
        let lit = Huff::new(lens.clone());
        let dst = fixed_dst();
        let toks: Vec<Tok> = (0..rng.range(1, 40)).map(|_| Tok::Lit(rng.byte())).collect();
        let mut w = BitWriter::new();
        w.bits(1, 1);
        w.bits(1, 2);
        emit_tokens(&mut w, &lit, &dst, &toks);
        lit.put(&mut w, 256);
        let stream = w.finish(4);
        let expect = simulate(&[], &toks);
        let case = Case::new(stream, expect.len()).patch(Patch::FixedTable(patch));
        let o = p.check("row46", &case);
        if let Outcome::Ret { ret: 1, ref out, .. } = o {
            assert_eq!(&out[..expect.len()], &expect[..], "row46");
        }
    }
}

#[test]
fn row47_dynamic_length_15_code() {
    // cp_build returns first[15], which EXCLUDES the length-15 symbols, so a
    // tree containing 15-bit codes drives cp_decode with a truncated `hi`.
    // Purely differential.
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 47);
    let (lit_lens, targets) = chain_tree(15);
    assert_eq!(*lit_lens.iter().max().unwrap(), 15);
    let dst_lens = tree_for(4, &[0, 1]);
    let b = build_dyn(288, 4, 4, &lit_lens, &dst_lens, true, true, true);
    let avail: Vec<usize> = targets.iter().cloned().filter(|&s| s < 256).collect();
    for _ in 0..48 {
        let n = rng.range(1, 40) as usize;
        let toks: Vec<Tok> = (0..n)
            .map(|_| Tok::Lit(avail[rng.below(avail.len() as u64) as usize] as u8))
            .collect();
        check_diff(&p, "row47", dyn_stream(&b, &toks, 4), 4096);
    }
}

#[test]
fn row48_cp_build_lookup_on_and_off() {
    // cp_fixed / cp_dynamic call cp_build both with `s` (which fills s->lookup)
    // and with 0.  A tree whose codes are all <= 9 fills `lookup`; one with
    // codes > 9 leaves the long entries out.  Both must behave identically.
    let p = Pair::new();
    let mut rng = Rng::new(SEED ^ 48);
    for k in [9u8, 12] {
        let (lit_lens, targets) = chain_tree(k);
        let dst_lens = tree_for(8, &[0, 1, 2, 3]);
        let b = build_dyn(288, 8, 4, &lit_lens, &dst_lens, true, true, true);
        let avail: Vec<usize> = targets.iter().cloned().filter(|&s| s < 256).collect();
        for _ in 0..32 {
            let n = rng.range(1, 60) as usize;
            let toks: Vec<Tok> = (0..n)
                .map(|_| Tok::Lit(avail[rng.below(avail.len() as u64) as usize] as u8))
                .collect();
            let expect = simulate(&[], &toks);
            check_ok(
                &p,
                &format!("row48 k={k}"),
                dyn_stream(&b, &toks, 4),
                &expect,
                expect.len(),
            );
        }
    }
}
