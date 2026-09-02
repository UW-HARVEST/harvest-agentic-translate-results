//! A raw-DEFLATE *encoder* used to drive `cp_inflate` through every branch
//! `CONFIGS.md` lists. Hand-rolling it (rather than only using zlib) is what
//! makes it possible to pin `nlit`/`ndst`/`nlen`, choose specific length and
//! distance codes, force code-length symbols 16/17/18, and build single-symbol
//! or empty trees.

#![allow(dead_code)]

use super::Rng;

// ---------------------------------------------------------------------------
// LSB-first bit writer (RFC 1951 §3.1.1)
// ---------------------------------------------------------------------------

pub struct BitWriter {
    pub buf: Vec<u8>,
    acc: u32,
    nbits: u32,
}

impl BitWriter {
    pub fn new() -> BitWriter {
        BitWriter { buf: Vec::new(), acc: 0, nbits: 0 }
    }

    /// Writes `n` bits of `v`, least-significant bit first.
    pub fn bits(&mut self, v: u32, n: u32) {
        debug_assert!(n <= 32);
        for i in 0..n {
            let bit = (v >> i) & 1;
            self.acc |= bit << self.nbits;
            self.nbits += 1;
            if self.nbits == 8 {
                self.buf.push(self.acc as u8);
                self.acc = 0;
                self.nbits = 0;
            }
        }
    }

    /// Writes a Huffman code: `len` bits of `code`, *most*-significant first.
    pub fn huff(&mut self, code: u32, len: u32) {
        for i in (0..len).rev() {
            self.bits((code >> i) & 1, 1);
        }
    }

    pub fn align(&mut self) {
        if self.nbits != 0 {
            self.buf.push(self.acc as u8);
            self.acc = 0;
            self.nbits = 0;
        }
    }

    pub fn raw(&mut self, data: &[u8]) {
        assert_eq!(self.nbits, 0, "raw() requires byte alignment");
        self.buf.extend_from_slice(data);
    }

    /// Byte-aligns and appends `n` zero bytes, so a decoder that keeps reading
    /// past the last meaningful bit has defined input to read.
    pub fn raw_pad(&mut self, n: usize) {
        self.align();
        self.buf.extend(std::iter::repeat(0u8).take(n));
    }

    pub fn finish(mut self) -> Vec<u8> {
        self.align();
        self.buf
    }

    pub fn bit_len(&self) -> usize {
        self.buf.len() * 8 + self.nbits as usize
    }
}

// ---------------------------------------------------------------------------
// Canonical Huffman codes (RFC 1951 §3.2.2) — the same construction
// `cp_build` inverts.
// ---------------------------------------------------------------------------

pub fn canonical_codes(lens: &[u8]) -> Vec<u32> {
    let mut bl_count = [0u32; 16];
    for &l in lens {
        assert!(l <= 15, "code length {l} exceeds 15");
        if l != 0 {
            bl_count[l as usize] += 1;
        }
    }
    let mut next_code = [0u32; 16];
    let mut code = 0u32;
    for bits in 1..=15 {
        code = (code + bl_count[bits - 1]) << 1;
        next_code[bits] = code;
    }
    let mut out = vec![0u32; lens.len()];
    for (i, &l) in lens.iter().enumerate() {
        if l != 0 {
            out[i] = next_code[l as usize];
            next_code[l as usize] += 1;
        }
    }
    out
}

/// True when the lengths form a *complete* prefix code (Kraft sum == 1), which
/// is what `cp_decode` needs to never fall off the end of its binary search.
pub fn is_complete(lens: &[u8]) -> bool {
    let mut sum = 0u64;
    for &l in lens {
        if l != 0 {
            sum += 1u64 << (15 - l as u32);
        }
    }
    sum == 1u64 << 15
}

/// Length-limited Huffman code lengths from symbol frequencies.
/// Uses plain Huffman then flattens the frequency distribution until the depth
/// fits in 15 bits, which always terminates.
pub fn huffman_lengths(freqs: &[u64], max_len: u8) -> Vec<u8> {
    let n = freqs.len();
    let mut f: Vec<u64> = freqs.to_vec();
    loop {
        let lens = huffman_once(&f);
        if lens.iter().all(|&l| l <= max_len) {
            // Guarantee at least two used symbols so the code is complete.
            let used = lens.iter().filter(|&&l| l != 0).count();
            if used >= 2 {
                debug_assert!(is_complete(&lens), "huffman_lengths produced an incomplete code");
                return lens;
            }
            // 0 or 1 used symbols: hand-build a 2-symbol, 1-bit code.
            let mut out = vec![0u8; n];
            let mut assigned = 0;
            for i in 0..n {
                if f[i] != 0 && assigned < 2 {
                    out[i] = 1;
                    assigned += 1;
                }
            }
            for i in 0..n {
                if assigned >= 2 {
                    break;
                }
                if out[i] == 0 {
                    out[i] = 1;
                    assigned += 1;
                }
            }
            return out;
        }
        for v in f.iter_mut() {
            if *v != 0 {
                *v = (*v >> 1) | 1;
            }
        }
    }
}

fn huffman_once(freqs: &[u64]) -> Vec<u8> {
    let n = freqs.len();
    let mut lens = vec![0u8; n];
    // (weight, depth-set) via a simple O(k^2) merge over live nodes.
    let mut nodes: Vec<(u64, Vec<usize>)> = freqs
        .iter()
        .enumerate()
        .filter(|(_, &f)| f != 0)
        .map(|(i, &f)| (f, vec![i]))
        .collect();
    if nodes.len() < 2 {
        return lens;
    }
    while nodes.len() > 1 {
        // Two smallest weights; ties broken by first occurrence for determinism.
        nodes.sort_by(|a, b| a.0.cmp(&b.0).then(a.1[0].cmp(&b.1[0])));
        let (w1, s1) = nodes.remove(0);
        let (w2, s2) = nodes.remove(0);
        for &i in s1.iter().chain(s2.iter()) {
            lens[i] += 1;
        }
        let mut merged = s1;
        merged.extend(s2);
        merged.sort_unstable();
        nodes.push((w1 + w2, merged));
    }
    lens
}

// ---------------------------------------------------------------------------
// Fixed Huffman tables (RFC 1951 §3.2.6) — mirrors `cp_fixed_table`
// ---------------------------------------------------------------------------

pub fn fixed_lit_lens() -> Vec<u8> {
    let mut v = vec![0u8; 288];
    for i in 0..=143 {
        v[i] = 8;
    }
    for i in 144..=255 {
        v[i] = 9;
    }
    for i in 256..=279 {
        v[i] = 7;
    }
    for i in 280..=287 {
        v[i] = 8;
    }
    v
}

pub fn fixed_dist_lens() -> Vec<u8> {
    vec![5u8; 32]
}

// ---------------------------------------------------------------------------
// Length / distance code tables (must agree with cp_len_* / cp_dist_*)
// ---------------------------------------------------------------------------

pub const LEN_BASE: [u32; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
pub const LEN_EXTRA: [u32; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
pub const DIST_BASE: [u32; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
pub const DIST_EXTRA: [u32; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];

pub fn len_code(length: u32) -> (usize, u32) {
    assert!((3..=258).contains(&length));
    let mut best = 0;
    for i in 0..29 {
        if LEN_BASE[i] <= length {
            best = i;
        }
    }
    // code 28 is the exact-258 code; 257 must use code 27
    if length == 258 {
        return (28, 0);
    }
    if best == 28 {
        best = 27;
    }
    (best, length - LEN_BASE[best])
}

pub fn dist_code(dist: u32) -> (usize, u32) {
    assert!((1..=32768).contains(&dist));
    let mut best = 0;
    for i in 0..30 {
        if DIST_BASE[i] <= dist {
            best = i;
        }
    }
    (best, dist - DIST_BASE[best])
}

// ---------------------------------------------------------------------------
// Token stream
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub enum Tok {
    Lit(u8),
    /// (length 3..=258, distance 1..=32768)
    Match(u32, u32),
    /// Fully explicit match: length symbol index `0..=30` into
    /// `cp_len_*`, its extra-bit payload, distance symbol index `0..=31`
    /// into `cp_dist_*`, and its extra-bit payload. Needed to reach the
    /// length codes 29/30 (whose `cp_len_base` is `0`) and to pin exact
    /// min/max extra values, which `Match`'s length->code mapping cannot.
    MatchRaw {
        lc: usize,
        lextra: u32,
        dc: usize,
        dextra: u32,
    },
}

/// `cp_len_base` / `cp_len_extra_bits` including the two trailing zero slots.
pub const LEN_BASE_FULL: [u32; 31] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258, 0, 0,
];
pub const LEN_EXTRA_FULL: [u32; 31] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0, 0, 0,
];
pub const DIST_BASE_FULL: [u32; 32] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 0, 0,
];
pub const DIST_EXTRA_FULL: [u32; 32] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13, 0, 0,
];

/// Applies a token stream to produce the expected decoded bytes (the same
/// byte-at-a-time semantics `cp_block` uses, so overlapping matches propagate).
pub fn expand(toks: &[Tok]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    for t in toks {
        match *t {
            Tok::Lit(b) => out.push(b),
            Tok::Match(len, dist) => {
                assert!(dist as usize <= out.len(), "match reaches before output start");
                let start = out.len() - dist as usize;
                for i in 0..len as usize {
                    let b = out[start + i];
                    out.push(b);
                }
            }
            Tok::MatchRaw { lc, lextra, dc, dextra } => {
                let len = LEN_BASE_FULL[lc] + lextra;
                let dist = DIST_BASE_FULL[dc] + dextra;
                assert!(dist as usize <= out.len(), "match reaches before output start");
                let start = out.len() - dist as usize;
                for i in 0..len as usize {
                    let b = out[start + i];
                    out.push(b);
                }
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Block emitters
// ---------------------------------------------------------------------------

pub struct Deflate {
    pub w: BitWriter,
}

impl Deflate {
    pub fn new() -> Deflate {
        Deflate { w: BitWriter::new() }
    }

    pub fn stored(&mut self, bfinal: bool, data: &[u8]) {
        assert!(data.len() <= 0xFFFF);
        self.w.bits(bfinal as u32, 1);
        self.w.bits(0, 2);
        self.w.align();
        let len = data.len() as u16;
        self.w.bits(len as u32, 16);
        self.w.bits((!len) as u32, 16);
        self.w.raw(data);
    }

    /// Stored block with deliberately wrong NLEN (ERRORS.md E1).
    pub fn stored_bad_nlen(&mut self, bfinal: bool, data: &[u8], nlen: u16) {
        self.w.bits(bfinal as u32, 1);
        self.w.bits(0, 2);
        self.w.align();
        self.w.bits(data.len() as u32 & 0xFFFF, 16);
        self.w.bits(nlen as u32, 16);
        self.w.raw(data);
    }

    /// Stored block whose declared LEN differs from the bytes that follow
    /// (ERRORS.md E2 when `declared_len` is too small).
    pub fn stored_len_override(&mut self, bfinal: bool, data: &[u8], declared_len: u16) {
        self.w.bits(bfinal as u32, 1);
        self.w.bits(0, 2);
        self.w.align();
        self.w.bits(declared_len as u32, 16);
        self.w.bits((!declared_len) as u32, 16);
        self.w.raw(data);
    }

    pub fn fixed(&mut self, bfinal: bool, toks: &[Tok]) {
        self.w.bits(bfinal as u32, 1);
        self.w.bits(1, 2);
        let lit = canonical_codes(&fixed_lit_lens());
        let litl = fixed_lit_lens();
        let dst = canonical_codes(&fixed_dist_lens());
        let dstl = fixed_dist_lens();
        self.emit_tokens(toks, &lit, &litl, &dst, &dstl);
    }

    /// A fixed block that omits the end-of-block symbol (for truncation tests).
    pub fn fixed_no_eob(&mut self, bfinal: bool, toks: &[Tok]) {
        self.w.bits(bfinal as u32, 1);
        self.w.bits(1, 2);
        let lit = canonical_codes(&fixed_lit_lens());
        let litl = fixed_lit_lens();
        let dst = canonical_codes(&fixed_dist_lens());
        let dstl = fixed_dist_lens();
        self.emit_tokens_no_eob(toks, &lit, &litl, &dst, &dstl);
    }

    /// Raw block-type escape (`btype == 3`, ERRORS.md E6).
    pub fn bad_btype(&mut self, bfinal: bool) {
        self.w.bits(bfinal as u32, 1);
        self.w.bits(3, 2);
    }

    fn emit_tokens(
        &mut self,
        toks: &[Tok],
        lit: &[u32],
        litl: &[u8],
        dst: &[u32],
        dstl: &[u8],
    ) {
        self.emit_tokens_no_eob(toks, lit, litl, dst, dstl);
        self.w.huff(lit[256], litl[256] as u32);
    }

    fn emit_tokens_no_eob(
        &mut self,
        toks: &[Tok],
        lit: &[u32],
        litl: &[u8],
        dst: &[u32],
        dstl: &[u8],
    ) {
        for t in toks {
            match *t {
                Tok::Lit(b) => {
                    let s = b as usize;
                    assert!(litl[s] != 0, "literal {s} has no code");
                    self.w.huff(lit[s], litl[s] as u32);
                }
                Tok::Match(length, distance) => {
                    let (lc, lextra) = len_code(length);
                    let s = 257 + lc;
                    assert!(litl[s] != 0, "length symbol {s} has no code");
                    self.w.huff(lit[s], litl[s] as u32);
                    self.w.bits(lextra, LEN_EXTRA[lc]);
                    let (dc, dextra) = dist_code(distance);
                    assert!(dstl[dc] != 0, "distance symbol {dc} has no code");
                    self.w.huff(dst[dc], dstl[dc] as u32);
                    self.w.bits(dextra, DIST_EXTRA[dc]);
                }
                Tok::MatchRaw { lc, lextra, dc, dextra } => {
                    let s = 257 + lc;
                    assert!(litl[s] != 0, "length symbol {s} has no code");
                    self.w.huff(lit[s], litl[s] as u32);
                    self.w.bits(lextra, LEN_EXTRA_FULL[lc]);
                    assert!(dstl[dc] != 0, "distance symbol {dc} has no code");
                    self.w.huff(dst[dc], dstl[dc] as u32);
                    self.w.bits(dextra, DIST_EXTRA_FULL[dc]);
                }
            }
        }
    }

    /// Dynamic block. `lit_lens` must have `nlit` entries (257..=288) and
    /// `dist_lens` `ndst` entries (1..=32). `nlen` (4..=19) forces HCLEN.
    pub fn dynamic(
        &mut self,
        bfinal: bool,
        toks: &[Tok],
        lit_lens: &[u8],
        dist_lens: &[u8],
        nlen_min: usize,
    ) {
        let nlit = lit_lens.len();
        let ndst = dist_lens.len();
        assert!((257..=288).contains(&nlit));
        assert!((1..=32).contains(&ndst));

        // Code-length symbol sequence for lit_lens ++ dist_lens.
        let mut seq: Vec<u8> = Vec::with_capacity(nlit + ndst);
        seq.extend_from_slice(lit_lens);
        seq.extend_from_slice(dist_lens);

        // Frequencies of the 19 code-length alphabet symbols (no 16/17/18 here;
        // the `rle_*` helpers below build sequences that use them).
        let mut freqs = [0u64; 19];
        for &s in &seq {
            freqs[s as usize] += 1;
        }
        let lenlens = huffman_lengths(&freqs, 7);
        self.emit_dynamic_header(bfinal, nlit, ndst, nlen_min, &lenlens);
        let clcodes = canonical_codes(&lenlens);
        for &s in &seq {
            self.w.huff(clcodes[s as usize], lenlens[s as usize] as u32);
        }
        let lit = canonical_codes(lit_lens);
        let dst = canonical_codes(dist_lens);
        self.emit_tokens(toks, &lit, lit_lens, &dst, dist_lens);
    }

    /// Dynamic block whose code-length sequence is RLE-compressed with symbols
    /// 16 / 17 / 18 (`CONFIGS.md` C19-C22).
    pub fn dynamic_rle(
        &mut self,
        bfinal: bool,
        toks: &[Tok],
        lit_lens: &[u8],
        dist_lens: &[u8],
        nlen_min: usize,
        use16: bool,
        use17: bool,
        use18: bool,
    ) {
        let nlit = lit_lens.len();
        let ndst = dist_lens.len();
        let mut seq: Vec<u8> = Vec::with_capacity(nlit + ndst);
        seq.extend_from_slice(lit_lens);
        seq.extend_from_slice(dist_lens);

        let ops = rle_encode(&seq, use16, use17, use18);
        let mut freqs = [0u64; 19];
        for op in &ops {
            freqs[op.0 as usize] += 1;
        }
        let lenlens = huffman_lengths(&freqs, 7);
        self.emit_dynamic_header(bfinal, nlit, ndst, nlen_min, &lenlens);
        let clcodes = canonical_codes(&lenlens);
        for &(sym, extra, nbits) in &ops {
            self.w.huff(clcodes[sym as usize], lenlens[sym as usize] as u32);
            if nbits > 0 {
                self.w.bits(extra, nbits);
            }
        }
        let lit = canonical_codes(lit_lens);
        let dst = canonical_codes(dist_lens);
        self.emit_tokens(toks, &lit, lit_lens, &dst, dist_lens);
    }

    fn emit_dynamic_header(
        &mut self,
        bfinal: bool,
        nlit: usize,
        ndst: usize,
        nlen_min: usize,
        lenlens: &[u8],
    ) {
        // HCLEN must be large enough to carry every non-zero entry.
        let mut needed = 4usize;
        for i in 0..19 {
            if lenlens[PERM[i] as usize] != 0 {
                needed = i + 1;
            }
        }
        let nlen = needed.max(nlen_min).max(4).min(19);
        self.w.bits(bfinal as u32, 1);
        self.w.bits(2, 2);
        self.w.bits((nlit - 257) as u32, 5);
        self.w.bits((ndst - 1) as u32, 5);
        self.w.bits((nlen - 4) as u32, 4);
        for i in 0..nlen {
            self.w.bits(lenlens[PERM[i] as usize] as u32, 3);
        }
    }

    pub fn finish(self) -> Vec<u8> {
        self.w.finish()
    }
}

pub const PERM: [u8; 19] = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];

/// `(symbol, extra_value, extra_bits)` triples.
type ClOp = (u8, u32, u32);

fn rle_encode(seq: &[u8], use16: bool, use17: bool, use18: bool) -> Vec<ClOp> {
    let mut out: Vec<ClOp> = Vec::new();
    let mut i = 0usize;
    while i < seq.len() {
        let v = seq[i];
        let mut run = 1usize;
        while i + run < seq.len() && seq[i + run] == v {
            run += 1;
        }
        if v == 0 {
            let mut left = run;
            while left > 0 {
                if use18 && left >= 11 {
                    let n = left.min(138);
                    out.push((18, (n - 11) as u32, 7));
                    left -= n;
                } else if use17 && left >= 3 {
                    let n = left.min(10);
                    out.push((17, (n - 3) as u32, 3));
                    left -= n;
                } else {
                    out.push((0, 0, 0));
                    left -= 1;
                }
            }
        } else {
            out.push((v, 0, 0));
            let mut left = run - 1;
            while left > 0 {
                if use16 && left >= 3 {
                    let n = left.min(6);
                    out.push((16, (n - 3) as u32, 2));
                    left -= n;
                } else {
                    out.push((v, 0, 0));
                    left -= 1;
                }
            }
        }
        i += run;
    }
    out
}

// ---------------------------------------------------------------------------
// Random token / length-set generators
// ---------------------------------------------------------------------------

/// Random literal-only token stream.
pub fn rand_literals(rng: &mut Rng, n: usize) -> Vec<Tok> {
    (0..n).map(|_| Tok::Lit(rng.byte())).collect()
}

/// Random mix of literals and (valid) matches.
pub fn rand_tokens(rng: &mut Rng, n: usize, max_dist: u32) -> Vec<Tok> {
    let mut toks = Vec::with_capacity(n);
    let mut produced = 0u32;
    for _ in 0..n {
        if produced >= 3 && rng.below(3) != 0 {
            let dist = 1 + rng.below(produced.min(max_dist) as usize) as u32;
            let length = 3 + rng.below(256) as u32;
            toks.push(Tok::Match(length, dist));
            produced += length;
        } else {
            toks.push(Tok::Lit(rng.byte()));
            produced += 1;
        }
    }
    toks
}

/// Complete `lit_lens` of the requested size derived from the token stream's
/// own symbol frequencies (so every emitted symbol has a code).
pub fn lit_lens_for(toks: &[Tok], nlit: usize) -> Vec<u8> {
    let mut freqs = vec![0u64; nlit];
    freqs[256] = 1;
    for t in toks {
        match *t {
            Tok::Lit(b) => freqs[b as usize] += 4,
            Tok::Match(length, _) => {
                let (lc, _) = len_code(length);
                freqs[257 + lc] += 4;
            }
            Tok::MatchRaw { lc, .. } => {
                assert!(257 + lc < nlit, "length symbol {} exceeds nlit {nlit}", 257 + lc);
                freqs[257 + lc] += 4;
            }
        }
    }
    // Give every slot a floor so the tree spans the full nlit range.
    for f in freqs.iter_mut() {
        *f += 1;
    }
    huffman_lengths(&freqs, 15)
}

pub fn dist_lens_for(toks: &[Tok], ndst: usize) -> Vec<u8> {
    let mut freqs = vec![0u64; ndst];
    let mut any = false;
    for t in toks {
        match *t {
            Tok::Match(_, d) => {
                let (dc, _) = dist_code(d);
                assert!(dc < ndst, "distance code {dc} exceeds ndst {ndst}");
                freqs[dc] += 4;
                any = true;
            }
            Tok::MatchRaw { dc, .. } => {
                assert!(dc < ndst, "distance code {dc} exceeds ndst {ndst}");
                freqs[dc] += 4;
                any = true;
            }
            Tok::Lit(_) => {}
        }
    }
    if !any {
        freqs[0] = 1;
    }
    for f in freqs.iter_mut() {
        *f += 1;
    }
    huffman_lengths(&freqs, 15)
}
