//! A hand-rolled DEFLATE *encoder* so tests can drive every branch the C
//! decoder takes (block type, tree shape, repeat symbol, alignment, ...).
//!
//! Deflate bit order: non-Huffman fields are packed LSB-first; Huffman codes are
//! emitted most-significant-bit-first.

#![allow(dead_code)]

use super::Rng;

pub const LEN_BASE: [u32; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
pub const LEN_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
pub const DIST_BASE: [u32; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
pub const DIST_EXTRA: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];
pub const PERM: [usize; 19] = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];

/// The decoder tables that live in the library's *writable exported globals*.
/// Mutation tests build a modified copy, write it into both `.so`s, and encode
/// with the same copy, so the streams stay self-consistent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tables {
    pub len_base: [u32; 31],
    pub len_extra: [u8; 31],
    pub dist_base: [u32; 32],
    pub dist_extra: [u8; 32],
}

impl Default for Tables {
    fn default() -> Tables {
        let mut t = Tables {
            len_base: [0; 31],
            len_extra: [0; 31],
            dist_base: [0; 32],
            dist_extra: [0; 32],
        };
        t.len_base[..29].copy_from_slice(&LEN_BASE);
        t.len_extra[..29].copy_from_slice(&LEN_EXTRA);
        t.dist_base[..30].copy_from_slice(&DIST_BASE);
        t.dist_extra[..30].copy_from_slice(&DIST_EXTRA);
        t
    }
}

/// `cp_fixed_table`: 288 literal lengths followed by 32 distance lengths.
pub fn fixed_lit_lens() -> Vec<u8> {
    let mut v = Vec::with_capacity(288);
    v.extend(std::iter::repeat(8).take(144)); // 0..143
    v.extend(std::iter::repeat(9).take(112)); // 144..255
    v.extend(std::iter::repeat(7).take(24)); // 256..279
    v.extend(std::iter::repeat(8).take(8)); // 280..287
    v
}
pub fn fixed_dist_lens() -> Vec<u8> {
    vec![5u8; 32]
}

// ---------------------------------------------------------------------------
// Bit writer
// ---------------------------------------------------------------------------

pub struct BitWriter {
    pub bytes: Vec<u8>,
    nbits: u32, // bits used in the last byte
}

impl BitWriter {
    pub fn new() -> BitWriter {
        BitWriter {
            bytes: Vec::new(),
            nbits: 0,
        }
    }
    /// Push `n` low bits of `val`, LSB first.
    pub fn bits(&mut self, val: u32, n: u32) {
        for i in 0..n {
            let bit = ((val >> i) & 1) as u8;
            if self.nbits == 0 {
                self.bytes.push(0);
                self.nbits = 8;
            }
            let last = self.bytes.len() - 1;
            let used = 8 - self.nbits;
            self.bytes[last] |= bit << used;
            self.nbits -= 1;
        }
    }
    /// Push a Huffman code of `n` bits, most-significant bit first.
    pub fn code(&mut self, code: u32, n: u32) {
        for i in (0..n).rev() {
            self.bits((code >> i) & 1, 1);
        }
    }
    pub fn align_to_byte(&mut self) {
        self.nbits = 0;
    }
    pub fn raw_bytes(&mut self, b: &[u8]) {
        assert_eq!(self.nbits, 0, "raw_bytes requires byte alignment");
        self.bytes.extend_from_slice(b);
    }
    pub fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

// ---------------------------------------------------------------------------
// Canonical Huffman
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Huff {
    pub lens: Vec<u8>,
    pub codes: Vec<u32>,
}

/// Canonical code assignment identical to `cp_build`'s:
/// `codes[n] = (codes[n-1] + counts[n-1]) << 1`.
pub fn canonical(lens: &[u8]) -> Huff {
    let mut counts = [0u32; 16];
    for &l in lens {
        assert!(l <= 15, "code length {l} out of range");
        counts[l as usize] += 1;
    }
    counts[0] = 0;
    let mut next = [0u32; 16];
    for n in 1..=15usize {
        next[n] = (next[n - 1] + counts[n - 1]) << 1;
    }
    let mut codes = vec![0u32; lens.len()];
    for (i, &l) in lens.iter().enumerate() {
        if l != 0 {
            codes[i] = next[l as usize];
            next[l as usize] += 1;
        }
    }
    Huff {
        lens: lens.to_vec(),
        codes,
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum TreeShape {
    /// Near-uniform lengths (max ~9 bits) — hits `cp_build`'s `len <= 9` fast path.
    Balanced,
    /// Deliberately deep: lengths reach `min(15, k-1)`, so for `k >= 11` some
    /// codes exceed 9 bits and `cp_build` skips the `lookup` fast path for them.
    Skewed,
}

/// Complete (Kraft-exact) length assignment over the `used` symbol indices.
///
/// `alphabet` is the total symbol count; unused symbols get length 0.
pub fn assign_lengths(alphabet: usize, used: &[usize], shape: TreeShape) -> Vec<u8> {
    let mut used: Vec<usize> = used.to_vec();
    used.sort_unstable();
    used.dedup();
    assert!(!used.is_empty());
    let mut lens = vec![0u8; alphabet];
    if used.len() == 1 {
        lens[used[0]] = 1;
        return lens;
    }
    match shape {
        TreeShape::Balanced => {
            let k = used.len();
            let d = (usize::BITS - 1 - k.leading_zeros()) as u32; // floor(log2(k))
            let r = k - (1usize << d);
            let n_short = (1usize << d) - r;
            for (i, &s) in used.iter().enumerate() {
                lens[s] = if i < n_short { d as u8 } else { (d + 1) as u8 };
            }
        }
        TreeShape::Skewed => {
            const MAXLEN: u8 = 15;
            let k = used.len();
            assert!(k <= 1 << MAXLEN);
            // Start from a single leaf at depth 0 and repeatedly split the
            // deepest splittable leaf. Splitting preserves the Kraft sum, so the
            // result is always a complete prefix code with max depth <= MAXLEN.
            let mut depths: Vec<u8> = vec![0];
            while depths.len() < k {
                let idx = depths
                    .iter()
                    .enumerate()
                    .filter(|(_, &d)| d < MAXLEN)
                    .max_by_key(|(_, &d)| d)
                    .map(|(i, _)| i)
                    .expect("no splittable leaf: k exceeds 2^MAXLEN");
                let d = depths[idx];
                depths[idx] = d + 1;
                depths.push(d + 1);
            }
            depths.sort_unstable();
            for (i, &s) in used.iter().enumerate() {
                lens[s] = depths[i];
            }
        }
    }
    lens
}

// ---------------------------------------------------------------------------
// Token stream
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug)]
pub enum Tok {
    Lit(u8),
    /// A back reference; `len` in 3..=258, `dist` in 1..=32768.
    Match { len: u16, dist: u16 },
    /// Force a specific length symbol index (0..=28) with an explicit extra value.
    RawMatch {
        len_sym: u8,
        len_extra: u32,
        dist_sym: u8,
        dist_extra: u32,
    },
}

pub fn len_to_sym(len: u16) -> (u8, u32) {
    assert!((3..=258).contains(&len));
    if len == 258 {
        return (28, 0);
    }
    let mut s = 0usize;
    while s + 1 < 29 && LEN_BASE[s + 1] <= len as u32 {
        s += 1;
    }
    (s as u8, len as u32 - LEN_BASE[s])
}

pub fn dist_to_sym(dist: u16) -> (u8, u32) {
    assert!((1..=32768).contains(&(dist as u32)));
    let mut s = 0usize;
    while s + 1 < 30 && DIST_BASE[s + 1] <= dist as u32 {
        s += 1;
    }
    (s as u8, dist as u32 - DIST_BASE[s])
}

/// Apply a token stream to produce the expected decompressed bytes.
pub fn apply_tokens(toks: &[Tok]) -> Vec<u8> {
    apply_tokens_t(toks, &Tables::default())
}

pub fn apply_tokens_t(toks: &[Tok], t: &Tables) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    let copy = |out: &mut Vec<u8>, len: u32, dist: u32| {
        let start = out.len() - dist as usize;
        for i in 0..len as usize {
            let b = out[start + i];
            out.push(b);
        }
    };
    for tok in toks {
        match *tok {
            Tok::Lit(b) => out.push(b),
            Tok::Match { len, dist } => copy(&mut out, len as u32, dist as u32),
            Tok::RawMatch {
                len_sym,
                len_extra,
                dist_sym,
                dist_extra,
            } => {
                let len = t.len_base[len_sym as usize].wrapping_add(len_extra);
                let dist = t.dist_base[dist_sym as usize].wrapping_add(dist_extra);
                copy(&mut out, len, dist);
            }
        }
    }
    out
}

fn write_tokens(bw: &mut BitWriter, lit: &Huff, dst: &Huff, toks: &[Tok], t: &Tables) {
    for tok in toks {
        let (ls, le, ds, de) = match *tok {
            Tok::Lit(b) => {
                let s = b as usize;
                assert!(lit.lens[s] != 0, "literal {b} not in the tree");
                bw.code(lit.codes[s], lit.lens[s] as u32);
                continue;
            }
            Tok::Match { len, dist } => {
                let (ls, le) = len_to_sym(len);
                let (ds, de) = dist_to_sym(dist);
                (ls, le, ds, de)
            }
            Tok::RawMatch {
                len_sym,
                len_extra,
                dist_sym,
                dist_extra,
            } => (len_sym, len_extra, dist_sym, dist_extra),
        };
        let sym = 257 + ls as usize;
        assert!(lit.lens[sym] != 0, "length symbol {sym} not in the tree");
        bw.code(lit.codes[sym], lit.lens[sym] as u32);
        bw.bits(le, t.len_extra[ls as usize] as u32);
        assert!(
            dst.lens[ds as usize] != 0,
            "distance symbol {ds} not in the tree"
        );
        bw.code(dst.codes[ds as usize], dst.lens[ds as usize] as u32);
        bw.bits(de, t.dist_extra[ds as usize] as u32);
    }
    // end-of-block
    bw.code(lit.codes[256], lit.lens[256] as u32);
}

// ---------------------------------------------------------------------------
// Block emitters
// ---------------------------------------------------------------------------

pub fn write_stored_block(bw: &mut BitWriter, bfinal: bool, payload: &[u8]) {
    bw.bits(bfinal as u32, 1);
    bw.bits(0, 2);
    // cp_stored aligns with `cp_read_bits(s, s->count & 7)`, which is equivalent
    // to skipping to the next byte boundary for a byte-aligned stream.
    bw.align_to_byte();
    let len = payload.len() as u16;
    bw.raw_bytes(&len.to_le_bytes());
    bw.raw_bytes(&(!len).to_le_bytes());
    bw.raw_bytes(payload);
}

pub fn write_fixed_block(bw: &mut BitWriter, bfinal: bool, toks: &[Tok]) {
    write_fixed_block_t(bw, bfinal, toks, &Tables::default(), &fixed_lit_lens())
}

/// `fixed_table` = the 288 literal lengths currently in the `cp_fixed_table`
/// global (the 32 distance lengths are always 5 in that table).
pub fn write_fixed_block_t(
    bw: &mut BitWriter,
    bfinal: bool,
    toks: &[Tok],
    t: &Tables,
    fixed_lit: &[u8],
) {
    bw.bits(bfinal as u32, 1);
    bw.bits(1, 2);
    let lit = canonical(fixed_lit);
    let dst = canonical(&fixed_dist_lens());
    write_tokens(bw, &lit, &dst, toks, t);
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum RepeatMode {
    /// Emit every code length as a literal 0..15 symbol.
    None,
    /// Prefer symbol 16 (copy previous length 3..6) where possible.
    Rep16,
    /// Prefer symbol 17 (zero run 3..10).
    Rep17,
    /// Prefer symbol 18 (zero run 11..138).
    Rep18,
    /// Use whichever repeat symbol is longest-matching.
    All,
}

/// Run-length encode the concatenated `lit_lens ++ dist_lens` sequence into
/// code-length alphabet symbols (0..18) with their extra-bit payloads.
fn encode_code_lengths(seq: &[u8], mode: RepeatMode) -> Vec<(u8, u32, u8)> {
    // (symbol, extra value, extra bit count)
    let mut out: Vec<(u8, u32, u8)> = Vec::new();
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
                let use18 = matches!(mode, RepeatMode::Rep18 | RepeatMode::All) && left >= 11;
                let use17 = matches!(mode, RepeatMode::Rep17 | RepeatMode::All) && left >= 3;
                if use18 {
                    let n = left.min(138);
                    out.push((18, (n - 11) as u32, 7));
                    left -= n;
                } else if use17 {
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
                let use16 = matches!(mode, RepeatMode::Rep16 | RepeatMode::All) && left >= 3;
                if use16 {
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

pub struct DynamicSpec {
    pub lit_lens: Vec<u8>,  // len == nlit, 257..=288
    pub dist_lens: Vec<u8>, // len == ndst, 1..=32
    pub repeat: RepeatMode,
    /// Force `nlen` (4..=19); `None` = minimum that covers the used CL symbols.
    pub force_nlen: Option<usize>,
    /// Whatever is currently in the `cp_permutation_order` global.
    pub perm: [usize; 19],
}

pub fn write_dynamic_block(bw: &mut BitWriter, bfinal: bool, spec: &DynamicSpec, toks: &[Tok]) {
    write_dynamic_block_t(bw, bfinal, spec, toks, &Tables::default())
}

pub fn write_dynamic_block_t(
    bw: &mut BitWriter,
    bfinal: bool,
    spec: &DynamicSpec,
    toks: &[Tok],
    t: &Tables,
) {
    let nlit = spec.lit_lens.len();
    let ndst = spec.dist_lens.len();
    assert!((257..=288).contains(&nlit), "nlit={nlit}");
    assert!((1..=32).contains(&ndst), "ndst={ndst}");

    let mut seq: Vec<u8> = spec.lit_lens.clone();
    seq.extend_from_slice(&spec.dist_lens);
    let cl = encode_code_lengths(&seq, spec.repeat);

    // Build the code-length tree over the CL symbols actually used.
    let mut used: Vec<usize> = cl.iter().map(|&(s, _, _)| s as usize).collect();
    used.sort_unstable();
    used.dedup();
    if used.len() == 1 {
        // Guarantee at least two symbols so the tree is a real prefix code.
        let extra = (0..19usize).find(|s| !used.contains(s)).unwrap();
        used.push(extra);
        used.sort_unstable();
    }
    let cl_lens = assign_lengths(19, &used, TreeShape::Balanced);
    let cl_huff = canonical(&cl_lens);

    // nlen must be big enough that every used CL symbol is transmitted.
    let min_nlen = spec
        .perm
        .iter()
        .enumerate()
        .filter(|(_, &p)| cl_lens[p] != 0)
        .map(|(i, _)| i + 1)
        .max()
        .unwrap()
        .max(4);
    let nlen = spec.force_nlen.unwrap_or(min_nlen);
    assert!(
        (4..=19).contains(&nlen) && nlen >= min_nlen,
        "nlen={nlen} min={min_nlen}"
    );

    bw.bits(bfinal as u32, 1);
    bw.bits(2, 2);
    bw.bits((nlit - 257) as u32, 5);
    bw.bits((ndst - 1) as u32, 5);
    bw.bits((nlen - 4) as u32, 4);
    for i in 0..nlen {
        bw.bits(cl_lens[spec.perm[i]] as u32, 3);
    }
    for &(s, extra, nbits) in &cl {
        bw.code(cl_huff.codes[s as usize], cl_huff.lens[s as usize] as u32);
        if nbits > 0 {
            bw.bits(extra, nbits as u32);
        }
    }

    let lit = canonical(&spec.lit_lens);
    let dst = canonical(&spec.dist_lens);
    write_tokens(bw, &lit, &dst, toks, t);
}

// ---------------------------------------------------------------------------
// Randomized token / tree generation
// ---------------------------------------------------------------------------

#[derive(Copy, Clone)]
pub struct TokSpec {
    pub n_tokens: usize,
    pub lit_pct: u32,
    pub max_dist: u16,
    pub max_len: u16,
    /// Restrict literals to this inclusive byte range.
    pub lit_lo: u8,
    pub lit_hi: u8,
}

impl Default for TokSpec {
    fn default() -> Self {
        TokSpec {
            n_tokens: 64,
            lit_pct: 70,
            max_dist: 512,
            max_len: 258,
            lit_lo: 0,
            lit_hi: 255,
        }
    }
}

/// Generate a self-consistent token stream (every match refers to real prior data).
pub fn gen_tokens(spec: &TokSpec, rng: &mut Rng) -> Vec<Tok> {
    let mut toks: Vec<Tok> = Vec::with_capacity(spec.n_tokens);
    let mut produced: usize = 0;
    let span = (spec.lit_hi as i64) - (spec.lit_lo as i64) + 1;
    for _ in 0..spec.n_tokens {
        if produced == 0 || rng.bool_pct(spec.lit_pct) {
            let b = (spec.lit_lo as i64 + rng.below(span as usize) as i64) as u8;
            toks.push(Tok::Lit(b));
            produced += 1;
        } else {
            let maxd = produced.min(spec.max_dist as usize) as i64;
            let dist = rng.range(1, maxd) as u16;
            let len = rng.range(3, spec.max_len as i64) as u16;
            toks.push(Tok::Match { len, dist });
            produced += len as usize;
        }
    }
    toks
}

/// The literal symbols a token stream needs present in its tree (plus 256).
pub fn used_lit_syms(toks: &[Tok]) -> Vec<usize> {
    let mut v = vec![256usize];
    for t in toks {
        match *t {
            Tok::Lit(b) => v.push(b as usize),
            Tok::Match { len, .. } => v.push(257 + len_to_sym(len).0 as usize),
            Tok::RawMatch { len_sym, .. } => v.push(257 + len_sym as usize),
        }
    }
    v.sort_unstable();
    v.dedup();
    v
}

pub fn used_dist_syms(toks: &[Tok]) -> Vec<usize> {
    let mut v: Vec<usize> = Vec::new();
    for t in toks {
        match *t {
            Tok::Match { dist, .. } => v.push(dist_to_sym(dist).0 as usize),
            Tok::RawMatch { dist_sym, .. } => v.push(dist_sym as usize),
            _ => {}
        }
    }
    if v.is_empty() {
        v.push(0);
    }
    v.sort_unstable();
    v.dedup();
    v
}

/// Build a `DynamicSpec` that can encode `toks`.
pub fn dynamic_spec_for(
    toks: &[Tok],
    shape: TreeShape,
    repeat: RepeatMode,
    nlit: usize,
    ndst: usize,
    force_nlen: Option<usize>,
) -> DynamicSpec {
    let lit_used = used_lit_syms(toks);
    let dst_used = used_dist_syms(toks);
    assert!(lit_used.iter().all(|&s| s < nlit), "nlit too small");
    assert!(dst_used.iter().all(|&s| s < ndst), "ndst too small");
    let lit_lens = assign_lengths(nlit, &lit_used, shape);
    let dist_lens = assign_lengths(ndst, &dst_used, shape);
    DynamicSpec {
        lit_lens,
        dist_lens,
        repeat,
        force_nlen,
        perm: PERM,
    }
}
