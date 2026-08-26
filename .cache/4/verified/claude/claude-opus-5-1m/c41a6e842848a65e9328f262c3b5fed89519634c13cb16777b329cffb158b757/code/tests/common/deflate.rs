//! Raw-DEFLATE stream *builders* used to drive `cp_inflate` into every branch
//! `lib.c` distinguishes.
//!
//! The canonical-code assignment implemented here is exactly the one
//! `cp_build()` performs (`codes[n] = (codes[n-1] + counts[n-1]) << 1`, symbols
//! numbered upwards inside a length class), i.e. standard DEFLATE canonical
//! Huffman.

#![allow(dead_code)]

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
/// `cp_permutation_order`
pub const PERM: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

// ---------------------------------------------------------------------------

#[derive(Clone, Default)]
pub struct BitWriter {
    pub buf: Vec<u8>,
    pub nbits: usize,
}

impl BitWriter {
    pub fn new() -> BitWriter {
        BitWriter::default()
    }
    fn bit(&mut self, b: u32) {
        if self.nbits % 8 == 0 {
            self.buf.push(0);
        }
        if b & 1 != 0 {
            let i = self.nbits / 8;
            self.buf[i] |= 1 << (self.nbits % 8);
        }
        self.nbits += 1;
    }
    /// `n` bits of `v`, LSB first (DEFLATE integer fields)
    pub fn bits(&mut self, v: u32, n: u32) {
        for i in 0..n {
            self.bit(v >> i);
        }
    }
    /// Huffman code: `n` bits of `code`, MSB first
    pub fn code(&mut self, code: u32, n: u32) {
        for i in (0..n).rev() {
            self.bit(code >> i);
        }
    }
    pub fn align(&mut self) {
        while self.nbits % 8 != 0 {
            self.bit(0);
        }
    }
    /// raw bytes (only valid at a byte boundary)
    pub fn raw(&mut self, data: &[u8]) {
        assert_eq!(self.nbits % 8, 0);
        self.buf.extend_from_slice(data);
        self.nbits += 8 * data.len();
    }
    pub fn finish(self) -> Vec<u8> {
        self.buf
    }
    pub fn byte_len(&self) -> usize {
        self.buf.len()
    }
}

// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct HuffEnc {
    pub lens: Vec<u8>,
    pub codes: Vec<u32>,
}

impl HuffEnc {
    /// Canonical codes, assigned exactly like `cp_build()`.
    pub fn new(lens: Vec<u8>) -> HuffEnc {
        let mut counts = [0u32; 17];
        for &l in &lens {
            assert!(l <= 15, "code length {l} > 15");
            counts[l as usize] += 1;
        }
        counts[0] = 0;
        let mut next = [0u32; 16];
        for n in 1..=15usize {
            next[n] = (next[n - 1] + counts[n - 1]) << 1;
        }
        let mut codes = vec![0u32; lens.len()];
        for (sym, &l) in lens.iter().enumerate() {
            if l != 0 {
                codes[sym] = next[l as usize];
                next[l as usize] += 1;
            }
        }
        HuffEnc { lens, codes }
    }
    pub fn emit(&self, bw: &mut BitWriter, sym: usize) {
        let l = self.lens[sym];
        assert!(l > 0, "symbol {sym} has no code");
        bw.code(self.codes[sym], l as u32);
    }
    /// Kraft sum times 2^15; 32768 == complete.
    pub fn kraft(&self) -> u32 {
        self.lens
            .iter()
            .filter(|&&l| l > 0)
            .map(|&l| 1u32 << (15 - l))
            .sum()
    }
}

/// Lengths of a *complete* canonical code over `used` (sorted, deduped) inside
/// an alphabet of `n` symbols. The shortest codes go to the first symbols.
pub fn lengths_for(n: usize, used: &[usize]) -> Vec<u8> {
    let mut lens = vec![0u8; n];
    let k = used.len();
    assert!(k >= 1);
    if k == 1 {
        // Incomplete code (Kraft 1/2) — only useful for error-path tests.
        lens[used[0]] = 1;
        return lens;
    }
    let p = (usize::BITS - 1 - k.leading_zeros()) as usize; // floor(log2 k)
    let r = k - (1usize << p);
    let short = (1usize << p) - r; // symbols with length p
    for (i, &s) in used.iter().enumerate() {
        lens[s] = if i < short { p as u8 } else { (p + 1) as u8 };
    }
    lens
}

/// A complete code whose maximum length is `depth`: lengths 1,2,…,depth-1,depth,depth.
pub fn deep_lengths(n: usize, used: &[usize], depth: usize) -> Vec<u8> {
    assert!(depth >= 2 && depth <= 15);
    assert_eq!(used.len(), depth + 1, "deep code of depth d needs d+1 symbols");
    let mut lens = vec![0u8; n];
    for i in 0..depth - 1 {
        lens[used[i]] = (i + 1) as u8;
    }
    lens[used[depth - 1]] = depth as u8;
    lens[used[depth]] = depth as u8;
    lens
}

// ---------------------------------------------------------------------------
// Fixed Huffman tables (= the contents of `cp_fixed_table`)
// ---------------------------------------------------------------------------

pub fn fixed_lit_lens() -> Vec<u8> {
    let mut v = vec![8u8; 288];
    for x in v[144..256].iter_mut() {
        *x = 9;
    }
    for x in v[256..280].iter_mut() {
        *x = 7;
    }
    for x in v[280..288].iter_mut() {
        *x = 8;
    }
    v
}

pub fn fixed_dist_lens() -> Vec<u8> {
    vec![5u8; 32]
}

pub fn fixed_lit() -> HuffEnc {
    HuffEnc::new(fixed_lit_lens())
}
pub fn fixed_dist() -> HuffEnc {
    HuffEnc::new(fixed_dist_lens())
}

// ---------------------------------------------------------------------------
// length / distance symbol lookup
// ---------------------------------------------------------------------------

/// `(symbol 257..285, extra bit count, extra value)`
pub fn len_symbol(length: u32) -> (usize, u32, u32) {
    assert!((3..=258).contains(&length));
    let mut i = 28;
    while i > 0 && LEN_BASE[i] > length {
        i -= 1;
    }
    (257 + i, LEN_EXTRA[i], length - LEN_BASE[i])
}

/// `(symbol 0..29, extra bit count, extra value)`
pub fn dist_symbol(dist: u32) -> (usize, u32, u32) {
    assert!((1..=32768).contains(&dist));
    let mut i = 29;
    while i > 0 && DIST_BASE[i] > dist {
        i -= 1;
    }
    (i, DIST_EXTRA[i], dist - DIST_BASE[i])
}

// ---------------------------------------------------------------------------
// Block emitters
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub enum Item {
    Lit(u16),
    /// (length, distance)
    Match(u32, u32),
    /// raw literal/length symbol (for symbols 286/287 that have no meaning)
    RawLit(u16),
    /// raw (length symbol, extra bits value, distance symbol, extra bits value)
    RawMatch(u16, u32, u16, u32),
}

/// `BFINAL`+`BTYPE=01` static block encoding `items`, terminated by symbol 256.
pub fn emit_fixed_block(bw: &mut BitWriter, bfinal: bool, items: &[Item]) {
    let lit = fixed_lit();
    let dist = fixed_dist();
    bw.bits(bfinal as u32, 1);
    bw.bits(1, 2);
    emit_items(bw, &lit, &dist, items);
}

pub fn emit_items(bw: &mut BitWriter, lit: &HuffEnc, dist: &HuffEnc, items: &[Item]) {
    for it in items {
        match *it {
            Item::Lit(b) => lit.emit(bw, b as usize),
            Item::RawLit(s) => lit.emit(bw, s as usize),
            Item::Match(l, d) => {
                let (ls, lx, lv) = len_symbol(l);
                lit.emit(bw, ls);
                bw.bits(lv, lx);
                let (ds, dx, dv) = dist_symbol(d);
                dist.emit(bw, ds);
                bw.bits(dv, dx);
            }
            Item::RawMatch(ls, lv, ds, dv) => {
                lit.emit(bw, ls as usize);
                let lx = if (257..=285).contains(&ls) {
                    LEN_EXTRA[ls as usize - 257]
                } else {
                    0
                };
                bw.bits(lv, lx);
                dist.emit(bw, ds as usize);
                let dx = if (ds as usize) < 30 {
                    DIST_EXTRA[ds as usize]
                } else {
                    0
                };
                bw.bits(dv, dx);
            }
        }
    }
    lit.emit(bw, 256);
}

/// Code-length-alphabet encoding mode for a dynamic header.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ClMode {
    /// every length written literally (symbols 0..15 only)
    Literal,
    /// run-length encode with symbols 16/17/18 where possible
    Repeats,
}

/// Encode a concatenated `litlens ++ dstlens` vector into the code-length
/// alphabet: `(symbol, extra bit count, extra value)` triples.
pub fn cl_encode(all: &[u8], mode: ClMode) -> Vec<(u8, u32, u32)> {
    let mut seq: Vec<(u8, u32, u32)> = Vec::new();
    match mode {
        ClMode::Literal => {
            for &l in all {
                seq.push((l, 0, 0));
            }
        }
        ClMode::Repeats => {
            let mut i = 0usize;
            while i < all.len() {
                let v = all[i];
                let mut run = 1usize;
                while i + run < all.len() && all[i + run] == v {
                    run += 1;
                }
                if v == 0 {
                    while run >= 11 {
                        let n = run.min(138);
                        seq.push((18, 7, (n - 11) as u32));
                        run -= n;
                        i += n;
                    }
                    while run >= 3 {
                        let n = run.min(10);
                        seq.push((17, 3, (n - 3) as u32));
                        run -= n;
                        i += n;
                    }
                    for _ in 0..run {
                        seq.push((0, 0, 0));
                        i += 1;
                    }
                } else {
                    seq.push((v, 0, 0));
                    i += 1;
                    run -= 1;
                    while run >= 3 {
                        let n = run.min(6);
                        seq.push((16, 2, (n - 3) as u32));
                        run -= n;
                        i += n;
                    }
                    for _ in 0..run {
                        seq.push((v, 0, 0));
                        i += 1;
                    }
                }
            }
        }
    }
    seq
}

/// Write a `BTYPE=10` dynamic header and return the two encoders.
///
/// `nlen` (HCLEN) is chosen as the smallest legal value that covers every used
/// code-length symbol, unless `force_nlen` overrides it.
pub fn emit_dynamic_header(
    bw: &mut BitWriter,
    bfinal: bool,
    litlens: &[u8],
    dstlens: &[u8],
    mode: ClMode,
    force_nlen: Option<usize>,
) -> (HuffEnc, HuffEnc) {
    emit_dynamic_header_with(bw, bfinal, litlens, dstlens, mode, force_nlen, &PERM)
}

/// As [`emit_dynamic_header`] but with an explicit code-length permutation, for
/// testing a mutated `cp_permutation_order` global.
pub fn emit_dynamic_header_with(
    bw: &mut BitWriter,
    bfinal: bool,
    litlens: &[u8],
    dstlens: &[u8],
    mode: ClMode,
    force_nlen: Option<usize>,
    perm: &[usize; 19],
) -> (HuffEnc, HuffEnc) {
    let nlit = litlens.len();
    let ndst = dstlens.len();
    assert!((257..=288).contains(&nlit));
    assert!((1..=32).contains(&ndst));

    let all: Vec<u8> = litlens.iter().chain(dstlens.iter()).copied().collect();
    let seq = cl_encode(&all, mode);

    let mut used: Vec<usize> = seq.iter().map(|s| s.0 as usize).collect();
    used.sort_unstable();
    used.dedup();
    let lenlens = lengths_for(19, &used);
    let cl = HuffEnc::new(lenlens.clone());

    let mut nlen = 4usize;
    for (pos, &sym) in perm.iter().enumerate() {
        if lenlens[sym] != 0 {
            nlen = nlen.max(pos + 1);
        }
    }
    if let Some(f) = force_nlen {
        assert!(f >= nlen, "forced HCLEN too small for the used symbols");
        nlen = f;
    }

    bw.bits(bfinal as u32, 1);
    bw.bits(2, 2);
    bw.bits((nlit - 257) as u32, 5);
    bw.bits((ndst - 1) as u32, 5);
    bw.bits((nlen - 4) as u32, 4);
    for i in 0..nlen {
        bw.bits(lenlens[perm[i]] as u32, 3);
    }
    for &(sym, xb, xv) in &seq {
        cl.emit(bw, sym as usize);
        if xb > 0 {
            bw.bits(xv, xb);
        }
    }

    (
        HuffEnc::new(litlens.to_vec()),
        HuffEnc::new(dstlens.to_vec()),
    )
}

/// `BTYPE=00` stored block. The C code requires `LEN >= remaining input`, which
/// holds when the stored block is the last thing in the stream.
pub fn emit_stored_block(bw: &mut BitWriter, bfinal: bool, payload: &[u8], len_field: Option<u16>) {
    bw.bits(bfinal as u32, 1);
    bw.bits(0, 2);
    bw.align();
    let len = len_field.unwrap_or(payload.len() as u16);
    bw.bits(len as u32, 16);
    bw.bits((!len) as u32, 16);
    bw.raw(payload);
}

/// Reference implementation of what `cp_block` produces for a list of items
/// (used to prove the crafted streams really exercise the intended path).
pub fn expected_output(items: &[Item]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    for it in items {
        match *it {
            Item::Lit(b) | Item::RawLit(b) => {
                assert!(b < 256);
                out.push(b as u8);
            }
            Item::Match(l, d) => {
                let start = out.len() - d as usize;
                for i in 0..l as usize {
                    let v = out[start + i];
                    out.push(v);
                }
            }
            Item::RawMatch(..) => panic!("no reference model for RawMatch"),
        }
    }
    out
}
