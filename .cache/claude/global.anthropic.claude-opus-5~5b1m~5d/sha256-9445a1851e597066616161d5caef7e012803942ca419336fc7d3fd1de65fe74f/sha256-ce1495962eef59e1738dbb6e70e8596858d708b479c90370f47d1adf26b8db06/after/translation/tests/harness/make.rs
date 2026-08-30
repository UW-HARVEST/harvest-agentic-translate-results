//! Deterministic generators for DEFLATE streams and PNG files.
//!
//! Everything here is written to match `c_src/src/lib.c` exactly: canonical
//! Huffman code assignment is the same computation as `cp_build`, and the
//! generators can be pointed at a *mutated* `cp_fixed_table` /
//! `cp_permutation_order` so that the resulting stream is still decodable by the
//! mutated library.

#![allow(dead_code)]

use super::Rng;

// ---------------------------------------------------------------------------
// Bit writer (DEFLATE conventions: fields LSB-first, Huffman codes MSB-first)
// ---------------------------------------------------------------------------

pub struct BitW {
    pub buf: Vec<u8>,
    pub nbits: usize,
}

impl BitW {
    pub fn new() -> BitW {
        BitW {
            buf: Vec::new(),
            nbits: 0,
        }
    }
    pub fn bit(&mut self, b: u32) {
        if self.nbits % 8 == 0 {
            self.buf.push(0);
        }
        if b & 1 != 0 {
            let i = self.nbits / 8;
            self.buf[i] |= 1 << (self.nbits % 8);
        }
        self.nbits += 1;
    }
    pub fn bits(&mut self, val: u32, n: usize) {
        for i in 0..n {
            self.bit(val >> i);
        }
    }
    pub fn huff(&mut self, code: u16, len: u8) {
        for i in (0..len).rev() {
            self.bit((code >> i) as u32);
        }
    }
    /// Pads to the next byte boundary with zeros.
    pub fn align(&mut self) {
        while self.nbits % 8 != 0 {
            self.bit(0);
        }
    }
    pub fn byte_len(&self) -> usize {
        (self.nbits + 7) / 8
    }
    pub fn finish(self) -> Vec<u8> {
        self.buf
    }
}

// ---------------------------------------------------------------------------
// Canonical Huffman codes -- identical computation to cp_build
// ---------------------------------------------------------------------------

pub fn canonical_codes(lens: &[u8]) -> Vec<u16> {
    let mut counts = [0u32; 16];
    for &l in lens {
        assert!(l < 16);
        counts[l as usize] += 1;
    }
    counts[0] = 0;
    let mut next = [0u32; 16];
    for n in 1..16 {
        next[n] = (next[n - 1] + counts[n - 1]) << 1;
    }
    let mut out = vec![0u16; lens.len()];
    for (i, &l) in lens.iter().enumerate() {
        if l != 0 {
            out[i] = next[l as usize] as u16;
            next[l as usize] += 1;
        }
    }
    out
}

/// Builds `k` code lengths that form a *complete* prefix code (Kraft sum 1) by
/// randomly growing a full binary tree, never deeper than `max_depth`.
pub fn random_complete_depths(rng: &mut Rng, k: usize, max_depth: u8) -> Vec<u8> {
    assert!(k >= 1);
    if k == 1 {
        // A single symbol cannot form a complete code; callers use k >= 2.
        return vec![1];
    }
    let mut depths: Vec<u8> = vec![0];
    while depths.len() < k {
        let candidates: Vec<usize> = (0..depths.len())
            .filter(|&i| depths[i] < max_depth)
            .collect();
        assert!(!candidates.is_empty(), "max_depth too small for k={k}");
        let pick = candidates[rng.below(candidates.len() as u32) as usize];
        let d = depths[pick] + 1;
        depths[pick] = d;
        depths.push(d);
    }
    depths
}

/// Spreads `depths` over `symbols` inside an alphabet of `n` symbols.
pub fn lengths_for(symbols: &[usize], depths: &[u8], n: usize) -> Vec<u8> {
    assert_eq!(symbols.len(), depths.len());
    let mut lens = vec![0u8; n];
    for (s, d) in symbols.iter().zip(depths) {
        lens[*s] = *d;
    }
    lens
}

/// Picks `k` distinct symbols from `0..n`, always including `must`.
pub fn pick_symbols(rng: &mut Rng, n: usize, k: usize, must: &[usize]) -> Vec<usize> {
    let mut set: Vec<usize> = must.to_vec();
    while set.len() < k {
        let c = rng.below(n as u32) as usize;
        if !set.contains(&c) {
            set.push(c);
        }
    }
    set.sort();
    set
}

// ---------------------------------------------------------------------------
// The C's DEFLATE tables (copies of the exported globals' initial values)
// ---------------------------------------------------------------------------

pub const LEN_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
pub const LEN_BASE: [u32; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
pub const DIST_EXTRA: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];
pub const DIST_BASE: [u32; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];

pub fn fixed_lit_lens() -> Vec<u8> {
    let mut v = vec![8u8; 288];
    for i in 144..256 {
        v[i] = 9;
    }
    for i in 256..280 {
        v[i] = 7;
    }
    v
}
pub fn fixed_dist_lens() -> Vec<u8> {
    vec![5u8; 32]
}

pub const PERMUTATION_ORDER: [u8; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

/// Length symbol + extra bits for a match length (3..=258).
pub fn len_code(length: u32) -> (usize, u32, u8) {
    for s in (0..29).rev() {
        let base = LEN_BASE[s];
        let extra = LEN_EXTRA[s];
        if length >= base && length < base + (1u32 << extra) {
            return (257 + s, length - base, extra);
        }
    }
    panic!("bad length {length}");
}

/// Distance symbol + extra bits for a distance (1..=32768).
pub fn dist_code(dist: u32) -> (usize, u32, u8) {
    for s in (0..30).rev() {
        let base = DIST_BASE[s];
        let extra = DIST_EXTRA[s];
        if dist >= base && dist < base + (1u32 << extra) {
            return (s, dist - base, extra);
        }
    }
    panic!("bad distance {dist}");
}

// ---------------------------------------------------------------------------
// Token streams
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub enum Tok {
    Lit(u8),
    /// (length 3..=258, distance 1..=32768)
    Match(u32, u32),
}

/// The bytes a token stream decodes to (starting from an empty output).
pub fn expand(toks: &[Tok]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    for t in toks {
        match *t {
            Tok::Lit(b) => out.push(b),
            Tok::Match(len, dist) => {
                assert!(dist as usize <= out.len(), "distance before start");
                for _ in 0..len {
                    let b = out[out.len() - dist as usize];
                    out.push(b);
                }
            }
        }
    }
    out
}

pub struct Codes {
    pub lit_lens: Vec<u8>,
    pub lit_codes: Vec<u16>,
    pub dst_lens: Vec<u8>,
    pub dst_codes: Vec<u16>,
}

impl Codes {
    pub fn new(lit_lens: Vec<u8>, dst_lens: Vec<u8>) -> Codes {
        let lit_codes = canonical_codes(&lit_lens);
        let dst_codes = canonical_codes(&dst_lens);
        Codes {
            lit_lens,
            lit_codes,
            dst_lens,
            dst_codes,
        }
    }
    pub fn fixed() -> Codes {
        Codes::new(fixed_lit_lens(), fixed_dist_lens())
    }
    /// The fixed codes as the library sees them after `cp_fixed_table` was
    /// mutated (`table` is the full 320-byte object).
    pub fn from_fixed_table(table: &[u8; 320]) -> Codes {
        Codes::new(table[..288].to_vec(), table[288..].to_vec())
    }
    pub fn emit(&self, bw: &mut BitW, toks: &[Tok]) {
        for t in toks {
            match *t {
                Tok::Lit(b) => {
                    let s = b as usize;
                    assert!(self.lit_lens[s] != 0, "literal {b} has no code");
                    bw.huff(self.lit_codes[s], self.lit_lens[s]);
                }
                Tok::Match(len, dist) => {
                    let (ls, lx, lb) = len_code(len);
                    assert!(self.lit_lens[ls] != 0, "len symbol {ls} has no code");
                    bw.huff(self.lit_codes[ls], self.lit_lens[ls]);
                    bw.bits(lx, lb as usize);
                    let (ds, dx, db) = dist_code(dist);
                    assert!(self.dst_lens[ds] != 0, "dist symbol {ds} has no code");
                    bw.huff(self.dst_codes[ds], self.dst_lens[ds]);
                    bw.bits(dx, db as usize);
                }
            }
        }
        // end of block
        assert!(self.lit_lens[256] != 0);
        bw.huff(self.lit_codes[256], self.lit_lens[256]);
    }
}

/// Emits a `btype = 1` (fixed Huffman) block.
pub fn block_fixed(bw: &mut BitW, bfinal: bool, toks: &[Tok], codes: &Codes) {
    bw.bits(u32::from(bfinal), 1);
    bw.bits(1, 2);
    codes.emit(bw, toks);
}

/// How the code lengths of the lit+dist alphabets are written out.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ClEncoding {
    /// Only symbols 0..=15 (one code-length symbol per alphabet entry).
    Literal,
    /// Use 16/17/18 run-length symbols where possible.
    RunLength,
}

/// Emits a `btype = 2` (dynamic Huffman) block.
///
/// `nlit`/`ndst` are the declared alphabet sizes; `lit_lens` must be `nlit`
/// long and `dst_lens` `ndst` long. `perm` is the (possibly mutated)
/// `cp_permutation_order` the library will use.
pub fn block_dynamic(
    bw: &mut BitW,
    bfinal: bool,
    toks: &[Tok],
    lit_lens: &[u8],
    dst_lens: &[u8],
    perm: &[u8; 19],
    enc: ClEncoding,
    rng: &mut Rng,
) {
    let nlit = lit_lens.len();
    let ndst = dst_lens.len();
    assert!((257..=288).contains(&nlit));
    assert!((1..=32).contains(&ndst));

    // The sequence of code-length symbols (+ their extra bits) to transmit.
    let mut all: Vec<u8> = Vec::new();
    all.extend_from_slice(lit_lens);
    all.extend_from_slice(dst_lens);
    let mut cl: Vec<(u8, u32, u8)> = Vec::new(); // (symbol, extra value, extra bits)
    let mut i = 0usize;
    while i < all.len() {
        let v = all[i];
        if enc == ClEncoding::RunLength {
            let mut run = 1usize;
            while i + run < all.len() && all[i + run] == v {
                run += 1;
            }
            if v == 0 && run >= 11 {
                let take = run.min(138);
                cl.push((18, (take - 11) as u32, 7));
                i += take;
                continue;
            }
            if v == 0 && run >= 3 {
                let take = run.min(10);
                cl.push((17, (take - 3) as u32, 3));
                i += take;
                continue;
            }
            if v != 0 && run >= 4 && i > 0 {
                // emit the value once, then repeat the rest
                cl.push((v, 0, 0));
                let mut rest = run - 1;
                while rest >= 3 {
                    let take = rest.min(6);
                    cl.push((16, (take - 3) as u32, 2));
                    rest -= take;
                }
                for _ in 0..rest {
                    cl.push((v, 0, 0));
                }
                i += run;
                continue;
            }
        }
        cl.push((v, 0, 0));
        i += 1;
    }

    // Code lengths for the 19-symbol code-length alphabet: a complete code over
    // exactly the symbols we use (max depth 7).
    let mut used: Vec<usize> = Vec::new();
    for (s, _, _) in &cl {
        if !used.contains(&(*s as usize)) {
            used.push(*s as usize);
        }
    }
    if used.len() < 2 {
        // pad so a complete code exists
        for extra in 0..19usize {
            if !used.contains(&extra) {
                used.push(extra);
                break;
            }
        }
    }
    used.sort();
    let depths = random_complete_depths(rng, used.len(), 7);
    let lenlens = lengths_for(&used, &depths, 19);
    let lencodes = canonical_codes(&lenlens);

    // nlen: how many of the permuted code lengths to transmit. Must cover every
    // used symbol, i.e. all positions p with perm[p] used.
    let mut nlen = 4usize;
    for p in 0..19usize {
        if lenlens[perm[p] as usize] != 0 {
            nlen = nlen.max(p + 1);
        }
    }

    bw.bits(u32::from(bfinal), 1);
    bw.bits(2, 2);
    bw.bits((nlit - 257) as u32, 5);
    bw.bits((ndst - 1) as u32, 5);
    bw.bits((nlen - 4) as u32, 4);
    for p in 0..nlen {
        bw.bits(lenlens[perm[p] as usize] as u32, 3);
    }
    for (s, xv, xb) in &cl {
        let si = *s as usize;
        assert!(lenlens[si] != 0, "cl symbol {si} has no code");
        bw.huff(lencodes[si], lenlens[si]);
        if *xb > 0 {
            bw.bits(*xv, *xb as usize);
        }
    }

    let codes = Codes::new(lit_lens.to_vec(), dst_lens.to_vec());
    codes.emit(bw, toks);
}

/// Emits a `btype = 0` (stored) block. Because `cp_stored` requires
/// `bits_left / 8 <= LEN`, a stored block can only be the *last* thing in the
/// stream and `LEN` must equal the number of bytes that follow it.
pub fn block_stored(bw: &mut BitW, bfinal: bool, payload: &[u8]) {
    bw.bits(u32::from(bfinal), 1);
    bw.bits(0, 2);
    bw.align();
    let len = payload.len() as u32;
    bw.bits(len & 0xFFFF, 16);
    bw.bits((!len) & 0xFFFF, 16);
    assert_eq!(bw.nbits % 8, 0);
    bw.buf.extend_from_slice(payload);
    bw.nbits += payload.len() * 8;
}

/// Wraps a raw DEFLATE stream in a zlib container (`cp_inflate` is handed
/// `data + 2, datalen - 6`, so a 2-byte header and 4 trailing bytes are needed).
pub fn zlib_wrap(deflate: &[u8], cmf: u8, flg: u8) -> Vec<u8> {
    let mut v = vec![cmf, flg];
    v.extend_from_slice(deflate);
    v.extend_from_slice(&[0, 0, 0, 0]); // adler32 -- never checked
    v
}

/// A random complete pair of lit/dist alphabets covering the given tokens.
pub fn random_codes_for(rng: &mut Rng, toks: &[Tok]) -> (Vec<u8>, Vec<u8>) {
    let mut lit_used: Vec<usize> = vec![256];
    let mut dst_used: Vec<usize> = Vec::new();
    for t in toks {
        match *t {
            Tok::Lit(b) => {
                if !lit_used.contains(&(b as usize)) {
                    lit_used.push(b as usize);
                }
            }
            Tok::Match(len, dist) => {
                let (ls, _, _) = len_code(len);
                if !lit_used.contains(&ls) {
                    lit_used.push(ls);
                }
                let (ds, _, _) = dist_code(dist);
                if !dst_used.contains(&ds) {
                    dst_used.push(ds);
                }
            }
        }
    }
    if lit_used.len() < 2 {
        lit_used.push(if lit_used.contains(&0) { 1 } else { 0 });
    }
    if dst_used.len() < 2 {
        for c in 0..30usize {
            if !dst_used.contains(&c) {
                dst_used.push(c);
            }
            if dst_used.len() == 2 {
                break;
            }
        }
    }
    lit_used.sort();
    dst_used.sort();
    let nlit = (lit_used.iter().max().unwrap() + 1).max(257);
    let ndst = (dst_used.iter().max().unwrap() + 1).max(1);
    let ld = random_complete_depths(rng, lit_used.len(), 15);
    let dd = random_complete_depths(rng, dst_used.len(), 15);
    (
        lengths_for(&lit_used, &ld, nlit),
        lengths_for(&dst_used, &dd, ndst),
    )
}

// ---------------------------------------------------------------------------
// PNG container
// ---------------------------------------------------------------------------

pub const SIG: [u8; 8] = [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1A, b'\n'];

fn crc32(data: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for i in 0..256u32 {
        let mut c = i;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB88320 ^ (c >> 1) } else { c >> 1 };
        }
        table[i as usize] = c;
    }
    let mut c = 0xFFFF_FFFFu32;
    for &b in data {
        c = table[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    c ^ 0xFFFF_FFFF
}

pub fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&(data.len() as u32).to_be_bytes());
    v.extend_from_slice(kind);
    v.extend_from_slice(data);
    let mut crcbuf = kind.to_vec();
    crcbuf.extend_from_slice(data);
    v.extend_from_slice(&crc32(&crcbuf).to_be_bytes());
    v
}

/// A chunk with a *declared* length that differs from the payload length (used
/// for the error-path tests).
pub fn chunk_raw_len(kind: &[u8; 4], declared: u32, data: &[u8]) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&declared.to_be_bytes());
    v.extend_from_slice(kind);
    v.extend_from_slice(data);
    v.extend_from_slice(&[0, 0, 0, 0]);
    v
}

pub fn ihdr(w: u32, h: u32, bit_depth: u8, color_type: u8, comp: u8, filt: u8, inter: u8) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&w.to_be_bytes());
    v.extend_from_slice(&h.to_be_bytes());
    v.push(bit_depth);
    v.push(color_type);
    v.push(comp);
    v.push(filt);
    v.push(inter);
    v
}

pub fn png_from_chunks(chunks: &[Vec<u8>]) -> Vec<u8> {
    let mut v = SIG.to_vec();
    for c in chunks {
        v.extend_from_slice(c);
    }
    v
}

pub fn bpp_of(color_type: u8) -> usize {
    match color_type {
        0 => 1,
        2 => 3,
        3 => 1,
        4 => 2,
        6 => 4,
        _ => panic!("bad color type"),
    }
}

/// Builds the raw (filtered) scanline block for a `w x h` image: `h` rows of
/// `1 + w*bpp` bytes, using the given filter type per row.
pub fn raw_scanlines(rng: &mut Rng, w: usize, h: usize, bpp: usize, filters: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(h * (1 + w * bpp));
    for y in 0..h {
        v.push(filters[y % filters.len()]);
        for _ in 0..w * bpp {
            v.push(rng.byte());
        }
    }
    v
}

/// Splits `data` into `n` roughly equal IDAT chunks (n == 0 -> one empty IDAT).
pub fn idat_chunks(data: &[u8], n: usize) -> Vec<Vec<u8>> {
    if n <= 1 {
        return vec![chunk(b"IDAT", data)];
    }
    let mut out = Vec::new();
    let per = (data.len() + n - 1) / n;
    let mut off = 0;
    for _ in 0..n {
        let end = (off + per).min(data.len());
        out.push(chunk(b"IDAT", &data[off..end]));
        off = end;
    }
    if off < data.len() {
        out.push(chunk(b"IDAT", &data[off..]));
    }
    out
}

/// A complete, well-formed PNG.
#[derive(Clone)]
pub struct PngSpec {
    pub w: u32,
    pub h: u32,
    pub bit_depth: u8,
    pub color_type: u8,
    pub comp: u8,
    pub filt: u8,
    pub inter: u8,
    pub ihdr_extra: usize,
    pub plte: Option<Vec<u8>>,
    pub trns: Option<Vec<u8>>,
    /// `true` -> tRNS chunk is written before PLTE
    pub trns_first: bool,
    pub deflate: Vec<u8>,
    pub cmf: u8,
    pub flg: u8,
    pub idat_parts: usize,
    pub empty_idat: bool,
    pub before_plte: Vec<Vec<u8>>,
    pub before_idat: Vec<Vec<u8>>,
    pub after_idat: Vec<Vec<u8>>,
}

impl PngSpec {
    pub fn new(w: u32, h: u32, color_type: u8, deflate: Vec<u8>) -> PngSpec {
        PngSpec {
            w,
            h,
            bit_depth: 8,
            color_type,
            comp: 0,
            filt: 0,
            inter: 0,
            ihdr_extra: 0,
            plte: None,
            trns: None,
            trns_first: false,
            deflate,
            cmf: 0x78,
            flg: 0x01,
            idat_parts: 1,
            empty_idat: false,
            before_plte: Vec::new(),
            before_idat: Vec::new(),
            after_idat: Vec::new(),
        }
    }
    pub fn build(&self) -> Vec<u8> {
        let mut hdr = ihdr(
            self.w,
            self.h,
            self.bit_depth,
            self.color_type,
            self.comp,
            self.filt,
            self.inter,
        );
        for i in 0..self.ihdr_extra {
            hdr.push((i as u8).wrapping_mul(13));
        }
        let mut chunks = vec![chunk(b"IHDR", &hdr)];
        chunks.extend(self.before_plte.iter().cloned());
        let plte_chunk = self.plte.as_ref().map(|p| chunk(b"PLTE", p));
        let trns_chunk = self.trns.as_ref().map(|t| chunk(b"tRNS", t));
        if self.trns_first {
            if let Some(c) = trns_chunk.clone() {
                chunks.push(c);
            }
            if let Some(c) = plte_chunk.clone() {
                chunks.push(c);
            }
        } else {
            if let Some(c) = plte_chunk.clone() {
                chunks.push(c);
            }
            if let Some(c) = trns_chunk.clone() {
                chunks.push(c);
            }
        }
        chunks.extend(self.before_idat.iter().cloned());
        let payload = zlib_wrap(&self.deflate, self.cmf, self.flg);
        if self.empty_idat {
            chunks.push(chunk(b"IDAT", &[]));
        }
        chunks.extend(idat_chunks(&payload, self.idat_parts));
        if self.empty_idat {
            chunks.push(chunk(b"IDAT", &[]));
        }
        chunks.extend(self.after_idat.iter().cloned());
        chunks.push(chunk(b"IEND", &[]));
        png_from_chunks(&chunks)
    }
}

/// A one-shot "encode this raw scanline block as a fixed-Huffman DEFLATE
/// stream of literals" helper.
pub fn deflate_literals(data: &[u8]) -> Vec<u8> {
    let mut bw = BitW::new();
    let codes = Codes::fixed();
    let toks: Vec<Tok> = data.iter().map(|b| Tok::Lit(*b)).collect();
    block_fixed(&mut bw, true, &toks, &codes);
    bw.finish()
}

pub fn ancillary(rng: &mut Rng) -> Vec<Vec<u8>> {
    vec![
        chunk(b"gAMA", &45455u32.to_be_bytes()),
        chunk(b"pHYs", &[0, 0, 0x0B, 0x13, 0, 0, 0x0B, 0x13, 1]),
        chunk(b"tEXt", &rng.bytes(7)),
        chunk(b"bKGD", &rng.bytes(6)),
    ]
}
