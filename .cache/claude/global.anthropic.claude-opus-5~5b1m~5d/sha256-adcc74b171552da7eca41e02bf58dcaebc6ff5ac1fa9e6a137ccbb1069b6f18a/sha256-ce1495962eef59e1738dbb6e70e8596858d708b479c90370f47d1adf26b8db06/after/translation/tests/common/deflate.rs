//! A hand-rolled DEFLATE *encoder* used to drive `cp_inflate` through every
//! branch it has.  It is deliberately parameterised over the tables that
//! `c_src/src/lib.c` exports as writable globals (`cp_fixed_table`,
//! `cp_permutation_order`, `cp_len_base`, ...) so that the "table as runtime
//! option" rows of `CONFIGS.md` can be encoded against the *overridden*
//! tables.

#![allow(dead_code)]

// ---------------------------------------------------------------------------
// bit writer (DEFLATE order: scalars LSB-first, Huffman codes MSB-first)
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct BitWriter {
    pub out: Vec<u8>,
    cur: u32,
    nbits: u32,
}

impl BitWriter {
    pub fn new() -> BitWriter {
        BitWriter::default()
    }
    pub fn bit(&mut self, b: u32) {
        self.cur |= (b & 1) << self.nbits;
        self.nbits += 1;
        if self.nbits == 8 {
            self.out.push(self.cur as u8);
            self.cur = 0;
            self.nbits = 0;
        }
    }
    /// `n` bits of `v`, least-significant first (DEFLATE scalar order).
    pub fn bits(&mut self, v: u32, n: u32) {
        for i in 0..n {
            self.bit(v >> i);
        }
    }
    /// A Huffman code, most-significant bit first.
    pub fn huff(&mut self, code: u32, len: u32) {
        assert!(len > 0, "attempt to emit a symbol with a zero-length code");
        for i in (0..len).rev() {
            self.bit(code >> i);
        }
    }
    pub fn align(&mut self) {
        if self.nbits > 0 {
            self.out.push(self.cur as u8);
            self.cur = 0;
            self.nbits = 0;
        }
    }
    pub fn byte(&mut self, b: u8) {
        assert_eq!(self.nbits, 0, "byte() on an unaligned writer");
        self.out.push(b);
    }
    pub fn finish(mut self) -> Vec<u8> {
        self.align();
        self.out
    }
    pub fn bitlen(&self) -> usize {
        self.out.len() * 8 + self.nbits as usize
    }
}

// ---------------------------------------------------------------------------
// canonical Huffman, built exactly the way `cp_build` does
// ---------------------------------------------------------------------------

/// Canonical codes for the given code lengths (0 = unused), using the same
/// first/codes recurrence as `cp_build`.
pub fn canonical(lens: &[u8]) -> Vec<u32> {
    let mut counts = [0u32; 16];
    for &l in lens {
        assert!(l < 16, "code length {l} out of range");
        counts[l as usize] += 1;
    }
    counts[0] = 0;
    let mut next = [0u32; 16];
    for n in 1..16 {
        next[n] = (next[n - 1] + counts[n - 1]) << 1;
    }
    let mut out = vec![0u32; lens.len()];
    for (i, &l) in lens.iter().enumerate() {
        if l != 0 {
            out[i] = next[l as usize];
            next[l as usize] += 1;
        }
    }
    out
}

/// A *complete* (Kraft-equality) length assignment for `used` symbols out of
/// `n_syms`: `2^b - k` symbols get length `b-1` and `2k - 2^b` get length `b`.
pub fn balanced_lengths(n_syms: usize, used: &[usize]) -> Vec<u8> {
    let mut lens = vec![0u8; n_syms];
    let k = used.len();
    if k == 0 {
        return lens;
    }
    if k == 1 {
        // Kraft-incomplete, but `cp_build`/`cp_decode` handle it: the single
        // entry is always selected and one bit is consumed.
        lens[used[0]] = 1;
        return lens;
    }
    let mut b = 0u32;
    while (1usize << b) < k {
        b += 1;
    }
    let short = (1usize << b) - k;
    for (i, &s) in used.iter().enumerate() {
        lens[s] = if i < short { (b - 1) as u8 } else { b as u8 };
    }
    lens
}

/// A maximally deep complete code: lengths 1,2,3,...,max-1,max,max.
pub fn deep_lengths(n_syms: usize, used: &[usize], max: u8) -> Vec<u8> {
    assert!(used.len() >= 2);
    assert!(used.len() <= max as usize + 1);
    let mut lens = vec![0u8; n_syms];
    let k = used.len();
    for (i, &s) in used.iter().enumerate() {
        lens[s] = if i + 1 < k { (i + 1) as u8 } else { i as u8 };
    }
    // used.len() == k: lengths 1..k-1 then repeat k-1 -> complete.
    lens
}

// ---------------------------------------------------------------------------
// the default contents of the exported tables (mirrors of the C initialisers)
// ---------------------------------------------------------------------------

pub fn default_fixed_table() -> Vec<u8> {
    let mut t = vec![0u8; 288 + 32];
    for (i, v) in t.iter_mut().enumerate() {
        *v = match i {
            0..=143 => 8,
            144..=255 => 9,
            256..=279 => 7,
            280..=287 => 8,
            _ => 5,
        };
    }
    t
}

pub const DEFAULT_PERMUTATION: [u8; 19] =
    [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];

pub const DEFAULT_LEN_EXTRA: [u8; 31] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0, 0, 0,
];
pub const DEFAULT_LEN_BASE: [u32; 31] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258, 0, 0,
];
pub const DEFAULT_DIST_EXTRA: [u8; 32] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13, 0, 0,
];
pub const DEFAULT_DIST_BASE: [u32; 32] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 0, 0,
];

/// The length/distance code tables the encoder should encode against.
#[derive(Clone)]
pub struct Tables {
    pub len_extra: Vec<u8>,
    pub len_base: Vec<u32>,
    pub dist_extra: Vec<u8>,
    pub dist_base: Vec<u32>,
}

impl Default for Tables {
    fn default() -> Self {
        Tables {
            len_extra: DEFAULT_LEN_EXTRA.to_vec(),
            len_base: DEFAULT_LEN_BASE.to_vec(),
            dist_extra: DEFAULT_DIST_EXTRA.to_vec(),
            dist_base: DEFAULT_DIST_BASE.to_vec(),
        }
    }
}

impl Tables {
    /// `(length_code_index, extra_value)` for a match length, using the
    /// currently configured tables (indices 0..=28 = symbols 257..=285).
    pub fn len_code(&self, len: u32) -> (usize, u32) {
        let mut best = None;
        for i in 0..29 {
            let base = self.len_base[i];
            let extra = self.len_extra[i] as u32;
            if len >= base && len - base < (1 << extra) {
                best = Some((i, len - base));
            }
        }
        best.unwrap_or_else(|| panic!("no length code for {len}"))
    }
    pub fn dist_code(&self, dist: u32) -> (usize, u32) {
        let mut best = None;
        for i in 0..30 {
            let base = self.dist_base[i];
            let extra = self.dist_extra[i] as u32;
            if dist >= base && dist - base < (1 << extra) {
                best = Some((i, dist - base));
            }
        }
        best.unwrap_or_else(|| panic!("no distance code for {dist}"))
    }
}

// ---------------------------------------------------------------------------
// symbols
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub enum Sym {
    Lit(u8),
    /// `(length, distance)` - resolved to codes through `Tables`.
    Match(u32, u32),
    /// Fully explicit: length code index (0..=28), its extra bits value,
    /// distance code index (0..=29), its extra bits value.
    RawMatch(usize, u32, usize, u32),
}

impl Sym {
    /// What this symbol appends to the output stream.
    pub fn apply(&self, out: &mut Vec<u8>, t: &Tables) {
        match *self {
            Sym::Lit(b) => out.push(b),
            Sym::Match(len, dist) => copy_match(out, len, dist),
            Sym::RawMatch(lc, le, dc, de) => {
                let len = t.len_base[lc] + le;
                let dist = t.dist_base[dc] + de;
                copy_match(out, len, dist);
            }
        }
    }
}

fn copy_match(out: &mut Vec<u8>, len: u32, dist: u32) {
    let start = out.len() - dist as usize;
    for i in 0..len as usize {
        let b = out[start + i];
        out.push(b);
    }
}

/// The bytes a symbol list decompresses to.
pub fn expand(syms: &[Sym], t: &Tables) -> Vec<u8> {
    let mut out = vec![];
    for s in syms {
        s.apply(&mut out, t);
    }
    out
}

// ---------------------------------------------------------------------------
// block emitters
// ---------------------------------------------------------------------------

pub struct Huff {
    pub lens: Vec<u8>,
    pub codes: Vec<u32>,
}

impl Huff {
    pub fn new(lens: Vec<u8>) -> Huff {
        let codes = canonical(&lens);
        Huff { lens, codes }
    }
    pub fn put(&self, bw: &mut BitWriter, sym: usize) {
        bw.huff(self.codes[sym], self.lens[sym] as u32);
    }
    pub fn has(&self, sym: usize) -> bool {
        sym < self.lens.len() && self.lens[sym] != 0
    }
}

fn emit_syms(bw: &mut BitWriter, syms: &[Sym], lit: &Huff, dist: &Huff, t: &Tables) {
    for s in syms {
        match *s {
            Sym::Lit(b) => lit.put(bw, b as usize),
            Sym::Match(len, d) => {
                let (lc, le) = t.len_code(len);
                let (dc, de) = t.dist_code(d);
                emit_match(bw, lit, dist, t, lc, le, dc, de);
            }
            Sym::RawMatch(lc, le, dc, de) => emit_match(bw, lit, dist, t, lc, le, dc, de),
        }
    }
    lit.put(bw, 256);
}

fn emit_match(
    bw: &mut BitWriter,
    lit: &Huff,
    dist: &Huff,
    t: &Tables,
    lc: usize,
    le: u32,
    dc: usize,
    de: u32,
) {
    lit.put(bw, 257 + lc);
    bw.bits(le, t.len_extra[lc] as u32);
    match dist.lens.iter().filter(|l| **l != 0).count() {
        // An *empty* distance tree makes `cp_decode` read `tree[-1]`; how many
        // bits it then consumes is not knowable to the encoder, so emit none.
        0 => {}
        // A one-entry tree is Kraft-incomplete; `cp_decode` still selects its
        // only entry and consumes exactly one bit, so emit one filler bit.
        1 => bw.bit(0),
        _ => dist.put(bw, dc),
    }
    bw.bits(de, t.dist_extra[dc] as u32);
}

/// btype 0.  Note the C's inverted `bits_left/8 <= LEN` check: a stored block
/// is only accepted when `LEN` is at least the number of input bytes left
/// after its header, i.e. normally it must be the last block in the stream.
pub fn emit_stored(bw: &mut BitWriter, data: &[u8], last: bool) {
    emit_stored_len(bw, data, data.len() as u16, last)
}

/// btype 0 with an explicit `LEN` field (for the error rows).
pub fn emit_stored_len(bw: &mut BitWriter, data: &[u8], len_field: u16, last: bool) {
    bw.bits(last as u32, 1);
    bw.bits(0, 2);
    bw.align();
    bw.byte((len_field & 0xFF) as u8);
    bw.byte((len_field >> 8) as u8);
    let nlen = !len_field;
    bw.byte((nlen & 0xFF) as u8);
    bw.byte((nlen >> 8) as u8);
    for &b in data {
        bw.byte(b);
    }
}

/// btype 0 with a deliberately broken NLEN.
pub fn emit_stored_bad_nlen(bw: &mut BitWriter, data: &[u8], len_field: u16, nlen_field: u16, last: bool) {
    bw.bits(last as u32, 1);
    bw.bits(0, 2);
    bw.align();
    bw.byte((len_field & 0xFF) as u8);
    bw.byte((len_field >> 8) as u8);
    bw.byte((nlen_field & 0xFF) as u8);
    bw.byte((nlen_field >> 8) as u8);
    for &b in data {
        bw.byte(b);
    }
}

/// btype 1, encoded against `fixed_table` (320 code lengths: 288 literal/length
/// + 32 distance), which defaults to `cp_fixed_table`'s initialiser.
pub fn emit_fixed_with(bw: &mut BitWriter, syms: &[Sym], last: bool, fixed_table: &[u8], t: &Tables) {
    assert_eq!(fixed_table.len(), 288 + 32);
    let lit = Huff::new(fixed_table[..288].to_vec());
    let dist = Huff::new(fixed_table[288..].to_vec());
    bw.bits(last as u32, 1);
    bw.bits(1, 2);
    emit_syms(bw, syms, &lit, &dist, t);
}

pub fn emit_fixed(bw: &mut BitWriter, syms: &[Sym], last: bool) {
    emit_fixed_with(bw, syms, last, &default_fixed_table(), &Tables::default())
}

/// btype 3 (the reserved block type).
pub fn emit_btype3(bw: &mut BitWriter, last: bool) {
    bw.bits(last as u32, 1);
    bw.bits(3, 2);
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ClMode {
    pub use16: bool,
    pub use17: bool,
    pub use18: bool,
}

impl ClMode {
    pub const LITERAL: ClMode = ClMode { use16: false, use17: false, use18: false };
    pub const R16: ClMode = ClMode { use16: true, use17: false, use18: false };
    pub const R17: ClMode = ClMode { use16: false, use17: true, use18: false };
    pub const R18: ClMode = ClMode { use16: false, use17: false, use18: true };
    pub const ALL: ClMode = ClMode { use16: true, use17: true, use18: true };
}

/// Run-length encode a code-length vector into code-length-alphabet
/// instructions `(symbol, extra_value, extra_bits)`.
pub fn encode_cl(v: &[u8], m: ClMode) -> Vec<(usize, u32, u32)> {
    let mut out: Vec<(usize, u32, u32)> = vec![];
    let mut i = 0usize;
    while i < v.len() {
        let x = v[i];
        let mut run = 1usize;
        while i + run < v.len() && v[i + run] == x {
            run += 1;
        }
        if x == 0 {
            let mut r = run;
            while r > 0 {
                if m.use18 && r >= 11 {
                    let n = r.min(138);
                    out.push((18, (n - 11) as u32, 7));
                    r -= n;
                } else if m.use17 && r >= 3 {
                    let n = r.min(10);
                    out.push((17, (n - 3) as u32, 3));
                    r -= n;
                } else {
                    out.push((0, 0, 0));
                    r -= 1;
                }
            }
        } else {
            out.push((x as usize, 0, 0));
            let mut r = run - 1;
            while r > 0 {
                if m.use16 && r >= 3 {
                    let n = r.min(6);
                    out.push((16, (n - 3) as u32, 2));
                    r -= n;
                } else {
                    out.push((x as usize, 0, 0));
                    r -= 1;
                }
            }
        }
        i += run;
    }
    out
}

#[derive(Clone)]
pub struct DynSpec {
    /// `nlit` entries, 257..=288 of them.
    pub lit_lens: Vec<u8>,
    /// `ndst` entries, 1..=32 of them.
    pub dist_lens: Vec<u8>,
    pub cl_mode: ClMode,
    /// `nlen` (4..=19); `None` = the minimum that covers every used CL symbol.
    pub nlen: Option<usize>,
    /// The permutation the *library* will use (`cp_permutation_order`).
    pub perm: [u8; 19],
}

impl DynSpec {
    pub fn new(lit_lens: Vec<u8>, dist_lens: Vec<u8>) -> DynSpec {
        assert!((257..=288).contains(&lit_lens.len()), "nlit = {}", lit_lens.len());
        assert!((1..=32).contains(&dist_lens.len()), "ndst = {}", dist_lens.len());
        DynSpec {
            lit_lens,
            dist_lens,
            cl_mode: ClMode::ALL,
            nlen: None,
            perm: DEFAULT_PERMUTATION,
        }
    }
}

/// Just the btype-2 header (HLIT/HDIST/HCLEN + the code-length sequence), with
/// no symbols and no end-of-block - used by the fuzzers to reach arbitrary
/// `cp_build` / `cp_decode` states.
pub fn emit_dynamic_header_only(bw: &mut BitWriter, spec: &DynSpec) {
    emit_dynamic_header(bw, spec, true);
}

/// btype 2.
pub fn emit_dynamic(bw: &mut BitWriter, spec: &DynSpec, syms: &[Sym], last: bool, t: &Tables) {
    emit_dynamic_header(bw, spec, last);
    let lit = Huff::new(spec.lit_lens.clone());
    let dist = Huff::new(spec.dist_lens.clone());
    emit_syms(bw, syms, &lit, &dist, t);
}

fn emit_dynamic_header(bw: &mut BitWriter, spec: &DynSpec, last: bool) {
    let nlit = spec.lit_lens.len();
    let ndst = spec.dist_lens.len();
    let mut all = spec.lit_lens.clone();
    all.extend_from_slice(&spec.dist_lens);

    let instrs = encode_cl(&all, spec.cl_mode);
    let mut used: Vec<usize> = instrs.iter().map(|x| x.0).collect();
    used.sort_unstable();
    used.dedup();
    // A one-symbol CL alphabet would be Kraft-incomplete; pad with an unused
    // symbol so the CL code stays complete.
    if used.len() == 1 {
        let pad = if used[0] == 0 { 1 } else { 0 };
        used.push(pad);
        used.sort_unstable();
    }
    let cl_lens = balanced_lengths(19, &used);
    assert!(cl_lens.iter().all(|l| *l <= 7), "CL length > 7");

    // `nlen` must be big enough that every used CL symbol is written.
    let mut min_nlen = 4usize;
    for &u in &used {
        let pos = spec.perm.iter().position(|p| *p as usize == u).unwrap() + 1;
        min_nlen = min_nlen.max(pos);
    }
    let nlen = spec.nlen.unwrap_or(min_nlen);
    assert!(
        (4..=19).contains(&nlen) && nlen >= min_nlen,
        "nlen {nlen} too small (need {min_nlen})"
    );

    bw.bits(last as u32, 1);
    bw.bits(2, 2);
    bw.bits((nlit - 257) as u32, 5);
    bw.bits((ndst - 1) as u32, 5);
    bw.bits((nlen - 4) as u32, 4);
    for i in 0..nlen {
        bw.bits(cl_lens[spec.perm[i] as usize] as u32, 3);
    }
    let cl = Huff::new(cl_lens);
    for &(sym, extra, nbits) in &instrs {
        cl.put(bw, sym);
        bw.bits(extra, nbits);
    }
}

/// Convenience: a dynamic spec that can code every symbol in `syms` plus EOB.
pub fn dyn_spec_for(syms: &[Sym], nlit: usize, ndst: usize, t: &Tables) -> DynSpec {
    let mut lit_used = vec![256usize];
    let mut dist_used: Vec<usize> = vec![];
    for s in syms {
        match *s {
            Sym::Lit(b) => lit_used.push(b as usize),
            Sym::Match(len, d) => {
                let (lc, _) = t.len_code(len);
                let (dc, _) = t.dist_code(d);
                lit_used.push(257 + lc);
                dist_used.push(dc);
            }
            Sym::RawMatch(lc, _, dc, _) => {
                lit_used.push(257 + lc);
                dist_used.push(dc);
            }
        }
    }
    lit_used.sort_unstable();
    lit_used.dedup();
    dist_used.sort_unstable();
    dist_used.dedup();
    // `nlit`/`ndst` are raised to the minimum the symbol set needs (HLIT/HDIST
    // are 5-bit fields, so 288 / 32 are the hard maxima).
    let nlit = nlit.max(lit_used.iter().copied().max().unwrap_or(0) + 1).max(257);
    let ndst = ndst.max(dist_used.iter().copied().max().unwrap_or(0) + 1).max(1);
    assert!(lit_used.iter().all(|s| *s < nlit), "symbol outside nlit");
    assert!(dist_used.iter().all(|s| *s < ndst), "distance symbol outside ndst");
    if dist_used.is_empty() {
        // `ndst` lengths are still read; leave them all zero unless a single
        // code is needed to keep the tree non-empty.
        dist_used.push(0);
    }
    DynSpec::new(balanced_lengths(nlit, &lit_used), balanced_lengths(ndst, &dist_used))
}
