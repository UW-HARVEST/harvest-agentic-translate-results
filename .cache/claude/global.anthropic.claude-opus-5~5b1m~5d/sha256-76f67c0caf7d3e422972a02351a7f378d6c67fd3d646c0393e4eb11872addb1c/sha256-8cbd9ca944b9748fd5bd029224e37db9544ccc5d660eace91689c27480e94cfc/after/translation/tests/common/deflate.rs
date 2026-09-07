//! A minimal, fully explicit raw-DEFLATE *encoder* used to build streams that
//! hit specific decoder branches (stored / fixed blocks, a chosen match length
//! and distance, a chosen number of chained blocks, a chosen tail alignment).
//!
//! Real-world dynamic-Huffman coverage comes from `flate2` (see tests); this
//! module exists because `flate2` gives no control over *which* branch of
//! `cp_block` / `cp_stored` gets taken.

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

/// LSB-first bit writer, which is how DEFLATE packs bits into bytes.
#[derive(Default, Clone)]
pub struct BitWriter {
    pub buf: Vec<u8>,
    nbits: u32, // bits already used in the last byte of `buf`
}

impl BitWriter {
    pub fn new() -> BitWriter {
        BitWriter { buf: Vec::new(), nbits: 0 }
    }

    /// Push `n` bits of `v`, least-significant bit first (DEFLATE "value" order).
    pub fn bits(&mut self, v: u32, n: u32) {
        for i in 0..n {
            let bit = ((v >> i) & 1) as u8;
            if self.nbits == 0 {
                self.buf.push(0);
                self.nbits = 8;
            }
            let last = self.buf.len() - 1;
            self.buf[last] |= bit << (8 - self.nbits);
            self.nbits -= 1;
        }
    }

    /// Push a Huffman code of `n` bits, most-significant bit first (DEFLATE
    /// "code" order).
    pub fn code(&mut self, code: u32, n: u32) {
        for i in (0..n).rev() {
            self.bits((code >> i) & 1, 1);
        }
    }

    pub fn align_to_byte(&mut self) {
        self.nbits = 0;
    }

    pub fn bit_offset(&self) -> u32 {
        // number of bits written
        (self.buf.len() as u32) * 8 - self.nbits
    }

    pub fn raw_bytes(&mut self, data: &[u8]) {
        self.align_to_byte();
        self.buf.extend_from_slice(data);
    }

    pub fn finish(self) -> Vec<u8> {
        self.buf
    }
}

// --- fixed Huffman code tables (RFC 1951 §3.2.6) ---------------------------

/// literal/length symbol -> (code, bit length)
pub fn fixed_litlen_code(sym: u32) -> (u32, u32) {
    match sym {
        0..=143 => (0x30 + sym, 8),
        144..=255 => (0x190 + (sym - 144), 9),
        256..=279 => (sym - 256, 7),
        280..=287 => (0xC0 + (sym - 280), 8),
        _ => panic!("bad litlen symbol {sym}"),
    }
}

/// length (3..=258) -> (symbol, extra bit count, extra value)
pub fn encode_length(len: u32) -> (u32, u32, u32) {
    assert!((3..=258).contains(&len), "length {len} out of range");
    if len == 258 {
        return (285, 0, 0);
    }
    let mut idx = 0usize;
    for i in 0..28 {
        if LEN_BASE[i] <= len {
            idx = i;
        }
    }
    (257 + idx as u32, LEN_EXTRA[idx], len - LEN_BASE[idx])
}

/// distance (1..=32768) -> (symbol, extra bit count, extra value)
pub fn encode_distance(dist: u32) -> (u32, u32, u32) {
    assert!((1..=32768).contains(&dist), "distance {dist} out of range");
    let mut idx = 0usize;
    for i in 0..30 {
        if DIST_BASE[i] <= dist {
            idx = i;
        }
    }
    (idx as u32, DIST_EXTRA[idx], dist - DIST_BASE[idx])
}

#[derive(Clone, Copy, Debug)]
pub enum Tok {
    Lit(u8),
    /// back-reference: copy `len` bytes from `dist` bytes back
    Match { len: u32, dist: u32 },
}

/// Emit a `BTYPE=01` (fixed Huffman) block into `w`.
pub fn write_fixed_block(w: &mut BitWriter, bfinal: bool, toks: &[Tok]) {
    w.bits(bfinal as u32, 1);
    w.bits(1, 2); // BTYPE = 01
    for t in toks {
        match *t {
            Tok::Lit(b) => {
                let (c, n) = fixed_litlen_code(b as u32);
                w.code(c, n);
            }
            Tok::Match { len, dist } => {
                let (lsym, lex, lval) = encode_length(len);
                let (c, n) = fixed_litlen_code(lsym);
                w.code(c, n);
                w.bits(lval, lex);
                let (dsym, dex, dval) = encode_distance(dist);
                w.code(dsym, 5); // fixed distance codes are 5 bits, value == code
                w.bits(dval, dex);
            }
        }
    }
    let (c, n) = fixed_litlen_code(256); // end of block
    w.code(c, n);
}

/// Emit a `BTYPE=00` (stored) block into `w`.
pub fn write_stored_block(w: &mut BitWriter, bfinal: bool, data: &[u8]) {
    assert!(data.len() <= 0xFFFF);
    w.bits(bfinal as u32, 1);
    w.bits(0, 2); // BTYPE = 00
    w.align_to_byte();
    let len = data.len() as u32;
    w.bits(len & 0xFF, 8);
    w.bits((len >> 8) & 0xFF, 8);
    let nlen = !len & 0xFFFF;
    w.bits(nlen & 0xFF, 8);
    w.bits((nlen >> 8) & 0xFF, 8);
    w.raw_bytes(data);
}

/// Emit a stored block with explicitly chosen LEN/NLEN (for error injection).
pub fn write_stored_block_raw(w: &mut BitWriter, bfinal: bool, len: u16, nlen: u16, data: &[u8]) {
    w.bits(bfinal as u32, 1);
    w.bits(0, 2);
    w.align_to_byte();
    w.bits(len as u32 & 0xFF, 8);
    w.bits((len as u32 >> 8) & 0xFF, 8);
    w.bits(nlen as u32 & 0xFF, 8);
    w.bits((nlen as u32 >> 8) & 0xFF, 8);
    w.raw_bytes(data);
}

/// Apply a token list the way `cp_block` does, producing the expected output.
pub fn expand(toks: &[Tok]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    for t in toks {
        match *t {
            Tok::Lit(b) => out.push(b),
            Tok::Match { len, dist } => {
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

/// A one-shot fixed-Huffman stream of literals only.
pub fn fixed_literal_stream(data: &[u8]) -> Vec<u8> {
    let toks: Vec<Tok> = data.iter().map(|&b| Tok::Lit(b)).collect();
    let mut w = BitWriter::new();
    write_fixed_block(&mut w, true, &toks);
    w.finish()
}

/// Pad the encoded stream so that `(in_bytes - first_bytes) & 3 == want_tail`
/// for a given `first_bytes`, by appending extra *stored* padding is not
/// possible (a stored block must be last) — instead this appends further
/// literal-only fixed blocks until the length is right. `cp_inflate` stops at
/// the block with `BFINAL=1`, so trailing garbage bytes are simply never read,
/// which is exactly what a shorter/longer tail means for the reader.
pub fn pad_to_len(mut stream: Vec<u8>, target_len: usize) -> Vec<u8> {
    assert!(stream.len() <= target_len);
    while stream.len() < target_len {
        stream.push(0x00);
    }
    stream
}

// ==========================================================================
// dynamic-Huffman (BTYPE=10) encoder
//
// Needed because `flate2` never emits a fixed block and never lets us choose
// HLIT/HDIST/HCLEN, which code-length RLE symbols appear, or whether any code
// is longer than 9 bits (cp_build's lookup-table cutoff).
// ==========================================================================

/// RFC 1951 order in which the 19 code-length code lengths are transmitted.
pub const CL_PERM: [usize; 19] = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];

/// Kraft sum check: a complete prefix code has `sum(2^-len) == 1`.
pub fn kraft_is_complete(lens: &[u8]) -> bool {
    let mut acc: u64 = 0;
    let one: u64 = 1 << 32;
    for &l in lens {
        if l > 0 {
            acc += one >> l;
        }
    }
    acc == one
}

/// A code is usable by a DEFLATE decoder when it is complete, empty (no symbol
/// used at all — legal for the distance alphabet of a match-free block), or the
/// degenerate single-symbol / 1-bit code that every real encoder emits when
/// exactly one symbol occurs.
pub fn code_is_usable(lens: &[u8]) -> bool {
    let used: Vec<u8> = lens.iter().copied().filter(|&l| l > 0).collect();
    used.is_empty() || (used.len() == 1 && used[0] == 1) || kraft_is_complete(lens)
}

/// Lengths for a *balanced* complete prefix code over `n` symbols.
/// `r = 2^k - n` symbols get `k-1` bits, the rest get `k` bits.
pub fn balanced_lengths(n: usize) -> Vec<u8> {
    assert!(n >= 1);
    if n == 1 {
        return vec![1];
    }
    let mut k = 1u32;
    while (1usize << k) < n {
        k += 1;
    }
    let r = (1usize << k) - n;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        out.push(if i < r { (k - 1) as u8 } else { k as u8 });
    }
    out
}

/// Lengths for a maximally *skewed* complete prefix code over `n` symbols:
/// `1, 2, 3, ..., n-1, n-1`. Produces codes longer than 9 bits (so `cp_build`
/// takes its `len > 9` path and `cp_decode` binary-searches deep entries).
pub fn skewed_lengths(n: usize) -> Vec<u8> {
    assert!((2..=16).contains(&n), "skewed_lengths needs 2..=16 symbols");
    let mut out: Vec<u8> = (1..n as u8).collect();
    out.push((n - 1) as u8);
    out
}

/// Canonical RFC 1951 codes for a code-length vector: `(code, len)` per symbol
/// (`len == 0` means "symbol unused", code is meaningless).
pub fn canonical_codes(lens: &[u8]) -> Vec<(u32, u32)> {
    let maxlen = lens.iter().copied().max().unwrap_or(0) as usize;
    let mut count = vec![0u32; maxlen + 2];
    for &l in lens {
        if l > 0 {
            count[l as usize] += 1;
        }
    }
    let mut next = vec![0u32; maxlen + 2];
    let mut code = 0u32;
    for b in 1..=maxlen {
        code = (code + count[b - 1]) << 1;
        next[b] = code;
    }
    lens.iter()
        .map(|&l| {
            if l == 0 {
                (0, 0)
            } else {
                let c = next[l as usize];
                next[l as usize] += 1;
                (c, l as u32)
            }
        })
        .collect()
}

/// One transmitted code-length code: `(symbol, extra_bit_count, extra_value)`.
pub type ClItem = (u32, u32, u32);

/// RLE-compress a code-length sequence exactly as RFC 1951 §3.2.7 allows,
/// using symbols 16 (repeat previous 3-6), 17 (zero run 3-10) and 18 (zero run
/// 11-138). With `rle == false` every length is transmitted literally.
pub fn rle_code_lengths(seq: &[u8], rle: bool) -> Vec<ClItem> {
    let mut out: Vec<ClItem> = Vec::new();
    if !rle {
        for &v in seq {
            out.push((v as u32, 0, 0));
        }
        return out;
    }
    let n = seq.len();
    let mut i = 0usize;
    while i < n {
        let v = seq[i];
        let mut j = i;
        while j < n && seq[j] == v {
            j += 1;
        }
        let mut run = j - i;
        if v == 0 {
            while run >= 11 {
                let k = run.min(138);
                out.push((18, 7, (k - 11) as u32));
                run -= k;
            }
            while run >= 3 {
                let k = run.min(10);
                out.push((17, 3, (k - 3) as u32));
                run -= k;
            }
            for _ in 0..run {
                out.push((0, 0, 0));
            }
        } else {
            out.push((v as u32, 0, 0));
            run -= 1;
            while run >= 3 {
                let k = run.min(6);
                out.push((16, 2, (k - 3) as u32));
                run -= k;
            }
            for _ in 0..run {
                out.push((v as u32, 0, 0));
            }
        }
        i = j;
    }
    out
}

#[derive(Clone, Debug)]
pub struct DynSpec {
    /// literal/length code lengths, exactly `nlit` entries (257..=288)
    pub litlens: Vec<u8>,
    /// distance code lengths, exactly `ndst` entries (1..=32)
    pub distlens: Vec<u8>,
    /// use the 16/17/18 RLE symbols (otherwise transmit lengths literally)
    pub rle: bool,
    /// force HCLEN (`nlen`, 4..=19); `None` = the minimum that fits
    pub hclen: Option<usize>,
}

/// Build a `DynSpec` that can encode `toks`, assigning code lengths with
/// `assign` (`balanced_lengths` or `skewed_lengths`).
pub fn dyn_spec_for(toks: &[Tok], assign: fn(usize) -> Vec<u8>, rle: bool) -> DynSpec {
    let mut lit_used = vec![false; 288];
    let mut dst_used = vec![false; 30];
    lit_used[256] = true; // end-of-block
    for t in toks {
        match *t {
            Tok::Lit(b) => lit_used[b as usize] = true,
            Tok::Match { len, dist } => {
                let (ls, _, _) = encode_length(len);
                lit_used[ls as usize] = true;
                let (ds, _, _) = encode_distance(dist);
                dst_used[ds as usize] = true;
            }
        }
    }
    let lit_idx: Vec<usize> = (0..288).filter(|&i| lit_used[i]).collect();
    let lit_assigned = assign(lit_idx.len());
    let nlit = (lit_idx.last().copied().unwrap() + 1).max(257);
    let mut litlens = vec![0u8; nlit];
    for (k, &i) in lit_idx.iter().enumerate() {
        litlens[i] = lit_assigned[k];
    }

    let dst_idx: Vec<usize> = (0..30).filter(|&i| dst_used[i]).collect();
    let (ndst, mut distlens) = if dst_idx.is_empty() {
        (1usize, vec![0u8; 1])
    } else {
        let n = dst_idx.last().copied().unwrap() + 1;
        (n, vec![0u8; n])
    };
    if !dst_idx.is_empty() {
        let dst_assigned = balanced_lengths(dst_idx.len());
        for (k, &i) in dst_idx.iter().enumerate() {
            distlens[i] = dst_assigned[k];
        }
    }
    assert_eq!(distlens.len(), ndst);
    DynSpec { litlens, distlens, rle, hclen: None }
}

/// Emit a `BTYPE=10` (dynamic Huffman) block into `w`.
pub fn write_dynamic_block(w: &mut BitWriter, bfinal: bool, spec: &DynSpec, toks: &[Tok]) {
    let nlit = spec.litlens.len();
    let ndst = spec.distlens.len();
    assert!((257..=288).contains(&nlit), "HLIT out of range: {nlit}");
    assert!((1..=32).contains(&ndst), "HDIST out of range: {ndst}");
    assert!(code_is_usable(&spec.litlens), "litlens is not a usable code");
    assert!(code_is_usable(&spec.distlens), "distlens is not a usable code");

    let mut seq: Vec<u8> = spec.litlens.clone();
    seq.extend_from_slice(&spec.distlens);
    let items = rle_code_lengths(&seq, spec.rle);

    // code lengths for the 19-symbol code-length alphabet
    let mut used = [false; 19];
    for &(s, _, _) in &items {
        used[s as usize] = true;
    }
    let cl_idx: Vec<usize> = (0..19).filter(|&i| used[i]).collect();
    let cl_assigned = balanced_lengths(cl_idx.len());
    let mut cl_lens = [0u8; 19];
    for (k, &i) in cl_idx.iter().enumerate() {
        cl_lens[i] = cl_assigned[k];
    }
    assert!(cl_lens.iter().all(|&l| l <= 7), "code-length code longer than 7 bits");

    // HCLEN: smallest count that still transmits every used symbol
    let mut nlen = 4usize;
    for (k, &p) in CL_PERM.iter().enumerate() {
        if cl_lens[p] != 0 {
            nlen = nlen.max(k + 1);
        }
    }
    if let Some(f) = spec.hclen {
        assert!((nlen..=19).contains(&f), "forced HCLEN {f} cannot carry the code");
        nlen = f;
    }

    w.bits(bfinal as u32, 1);
    w.bits(2, 2); // BTYPE = 10
    w.bits((nlit - 257) as u32, 5);
    w.bits((ndst - 1) as u32, 5);
    w.bits((nlen - 4) as u32, 4);
    for k in 0..nlen {
        w.bits(cl_lens[CL_PERM[k]] as u32, 3);
    }

    let cl_codes = canonical_codes(&cl_lens);
    for &(sym, ex, val) in &items {
        let (c, l) = cl_codes[sym as usize];
        assert!(l > 0, "code-length symbol {sym} has no code");
        w.code(c, l);
        w.bits(val, ex);
    }

    let lit_codes = canonical_codes(&spec.litlens);
    let dst_codes = canonical_codes(&spec.distlens);
    for t in toks {
        match *t {
            Tok::Lit(b) => {
                let (c, l) = lit_codes[b as usize];
                assert!(l > 0, "literal {b} has no code");
                w.code(c, l);
            }
            Tok::Match { len, dist } => {
                let (ls, lex, lval) = encode_length(len);
                let (c, l) = lit_codes[ls as usize];
                assert!(l > 0, "length symbol {ls} has no code");
                w.code(c, l);
                w.bits(lval, lex);
                let (ds, dex, dval) = encode_distance(dist);
                let (c, l) = dst_codes[ds as usize];
                assert!(l > 0, "distance symbol {ds} has no code");
                w.code(c, l);
                w.bits(dval, dex);
            }
        }
    }
    let (c, l) = lit_codes[256];
    w.code(c, l);
}

// ==========================================================================
// streams crafted to trip specific `assert()`s in the C library
// ==========================================================================

/// Build a fixed-Huffman stream, together with the `in_bytes` to pass, such
/// that `cp_read_bits()` is asked for the 13 extra bits of distance symbol 29
/// when only `r <= 6` bits remain, i.e. `bits_left + count - 13 < 0`, which is
/// exactly `assert(!cp_would_overflow(s, num_bits_to_read))` (lib.c:121).
///
/// `in_bytes` is a multiple of 4 and the input is 4-byte aligned, so
/// `last_bytes == 0` and `count == bits_left` holds — that is what makes the
/// sum small enough to trip the assert.
pub fn build_would_overflow_stream() -> (Vec<u8>, i32) {
    for k in 0..64u32 {
        let mut w = BitWriter::new();
        w.bits(1, 1); // BFINAL
        w.bits(1, 2); // BTYPE = fixed
        for _ in 0..k {
            let (c, n) = fixed_litlen_code(200); // a 9-bit literal code
            w.code(c, n);
        }
        let (c, n) = fixed_litlen_code(257); // length 3, zero extra bits
        w.code(c, n);
        w.code(29, 5); // distance symbol 29 -> 13 extra bits
        let b = w.bit_offset();
        let r = 32 - (b % 32);
        if (1..=6).contains(&r) {
            w.bits(0, 13);
            let (c, n) = fixed_litlen_code(256);
            w.code(c, n);
            let mut s = w.finish();
            let in_bytes = ((b + r) / 8) as usize;
            while s.len() < in_bytes {
                s.push(0);
            }
            assert_eq!(in_bytes % 4, 0);
            return (s, in_bytes as i32);
        }
    }
    panic!("no k produced a suitable bit offset");
}

/// A dynamic block whose transmitted literal/length AND distance code lengths
/// are *all zero*, so `cp_build()` returns 0 and `cp_block()`'s first
/// `cp_decode(s, s->lit, 0)` reads `tree[-1]` and trips
/// `assert((search >> len) == (key >> len))` (lib.c:211).
pub fn build_all_zero_length_dynamic_block() -> Vec<u8> {
    let mut w = BitWriter::new();
    w.bits(1, 1); // BFINAL
    w.bits(2, 2); // BTYPE = dynamic
    w.bits(0, 5); // HLIT  = 0  -> nlit = 257
    w.bits(0, 5); // HDIST = 0  -> ndst = 1
    w.bits(0, 4); // HCLEN = 0  -> nlen = 4, i.e. symbols 16, 17, 18, 0
    // code lengths for symbols 16, 17, 18, 0 in CL_PERM order: only 18 is used
    w.bits(0, 3); // 16
    w.bits(0, 3); // 17
    w.bits(1, 3); // 18 -> a 1-bit code
    w.bits(0, 3); // 0
    // two maximal zero runs (11 + 127 = 138 each) => 276 >= nlit + ndst == 258
    for _ in 0..2 {
        w.code(0, 1); // the only code-length code: symbol 18
        w.bits(127, 7);
    }
    let mut s = w.finish();
    while s.len() % 4 != 0 || s.len() < 8 {
        s.push(0);
    }
    s
}

/// A fixed block containing exactly one literal and one `distance == 1` match,
/// used together with a tampered `cp_dist_extra_bits[0]`.
pub fn build_dist_symbol_zero_stream() -> Vec<u8> {
    let mut w = BitWriter::new();
    write_fixed_block(&mut w, true, &[Tok::Lit(b'q'), Tok::Match { len: 3, dist: 1 }]);
    let mut s = w.finish();
    while s.len() < 16 {
        s.push(0);
    }
    s
}
