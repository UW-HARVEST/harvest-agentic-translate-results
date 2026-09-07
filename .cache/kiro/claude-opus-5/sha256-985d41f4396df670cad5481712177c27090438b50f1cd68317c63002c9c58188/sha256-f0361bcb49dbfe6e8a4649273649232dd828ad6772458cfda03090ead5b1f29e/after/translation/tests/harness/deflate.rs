//! A minimal DEFLATE *encoder* used only to build well-formed inputs for the
//! differential tests. It is deliberately independent of both libraries under
//! test.

#![allow(dead_code)]

use super::Rng;

// ---------------------------------------------------------------------------
// Bit writer (DEFLATE bit order)
// ---------------------------------------------------------------------------

#[derive(Default, Clone)]
pub struct BitWriter {
    pub bytes: Vec<u8>,
    acc: u32,
    nbits: u32,
}

impl BitWriter {
    pub fn new() -> BitWriter {
        BitWriter::default()
    }

    /// Header fields / extra bits: written LSB-first.
    pub fn bits(&mut self, value: u32, n: u32) {
        debug_assert!(n <= 32);
        for i in 0..n {
            let b = (value >> i) & 1;
            self.acc |= b << self.nbits;
            self.nbits += 1;
            if self.nbits == 8 {
                self.bytes.push(self.acc as u8);
                self.acc = 0;
                self.nbits = 0;
            }
        }
    }

    /// Huffman codes: packed starting with the most-significant bit.
    pub fn code(&mut self, code: u32, n: u32) {
        debug_assert!(n >= 1 && n <= 15);
        for i in (0..n).rev() {
            self.bits((code >> i) & 1, 1);
        }
    }

    pub fn align_to_byte(&mut self) {
        if self.nbits != 0 {
            self.bytes.push(self.acc as u8);
            self.acc = 0;
            self.nbits = 0;
        }
    }

    pub fn bit_len(&self) -> usize {
        self.bytes.len() * 8 + self.nbits as usize
    }

    pub fn finish(mut self) -> Vec<u8> {
        self.align_to_byte();
        self.bytes
    }
}

// ---------------------------------------------------------------------------
// Canonical Huffman
// ---------------------------------------------------------------------------

/// RFC-1951 canonical code assignment from a code-length vector.
pub fn canonical(lens: &[u8]) -> Vec<u32> {
    let maxlen = 15usize;
    let mut count = [0u32; 16];
    for &l in lens {
        assert!(l as usize <= maxlen);
        if l != 0 {
            count[l as usize] += 1;
        }
    }
    let mut next = [0u32; 17];
    let mut code = 0u32;
    for l in 1..=maxlen {
        code = (code + count[l - 1]) << 1;
        next[l] = code;
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

/// True when `sum(2^-len) == 1` over the non-zero lengths.
pub fn is_complete(lens: &[u8]) -> bool {
    let mut total: u64 = 0;
    for &l in lens {
        if l != 0 {
            total += 1u64 << (15 - l as u32);
        }
    }
    total == 1u64 << 15
}

/// Build code lengths for `freqs` (a complete code, max length `maxlen`).
/// Simple, deterministic package-free construction: repeated smallest-two
/// merging, then a length-limiting pass.
pub fn huffman_lengths(freqs: &[u64], maxlen: u8) -> Vec<u8> {
    let n = freqs.len();
    let mut lens = vec![0u8; n];
    let used: Vec<usize> = (0..n).filter(|&i| freqs[i] > 0).collect();
    if used.is_empty() {
        return lens;
    }
    if used.len() == 1 {
        // A one-symbol alphabet cannot form a complete code; give the symbol
        // length 1 and a second (unused) symbol length 1 so the code is
        // complete, which is what the C decoder's binary search expects.
        let a = used[0];
        let b = if a == 0 { 1 % n } else { 0 };
        lens[a] = 1;
        if b != a {
            lens[b] = 1;
        }
        return lens;
    }

    // node = (weight, Vec<symbol>)
    let mut nodes: Vec<(u64, Vec<usize>)> = used.iter().map(|&i| (freqs[i], vec![i])).collect();
    nodes.sort_by(|a, b| a.0.cmp(&b.0).then(a.1[0].cmp(&b.1[0])));
    let mut depth = vec![0u8; n];
    while nodes.len() > 1 {
        nodes.sort_by(|a, b| a.0.cmp(&b.0).then(a.1[0].cmp(&b.1[0])));
        let (w0, s0) = nodes.remove(0);
        let (w1, s1) = nodes.remove(0);
        for &s in s0.iter().chain(s1.iter()) {
            depth[s] += 1;
        }
        let mut merged = s0;
        merged.extend(s1);
        merged.sort();
        nodes.push((w0 + w1, merged));
    }
    for &i in &used {
        lens[i] = depth[i].max(1);
    }

    // Length-limit: while any length exceeds maxlen, flatten to a balanced code.
    if lens.iter().any(|&l| l > maxlen) || !is_complete(&lens) {
        // Fall back to a balanced code over the used symbols (padded to a power
        // of two with extra symbols so the code stays complete).
        let k = used.len();
        let mut bits = 1u8;
        while (1usize << bits) < k {
            bits += 1;
        }
        assert!(bits <= maxlen, "alphabet too large for maxlen");
        for l in lens.iter_mut() {
            *l = 0;
        }
        // Give `2^bits - k` of the used symbols a shorter length so the code is
        // complete. Simplest complete assignment: Kraft-fill greedily.
        let mut remaining = 1u64 << 15;
        for (idx, &s) in used.iter().enumerate() {
            let is_last = idx + 1 == k;
            let mut l = bits;
            if is_last {
                // give the last symbol whatever length exactly consumes the rest
                while l > 1 && (1u64 << (15 - (l - 1) as u32)) <= remaining {
                    l -= 1;
                }
            }
            lens[s] = l;
            remaining -= 1u64 << (15 - l as u32);
        }
        if remaining != 0 {
            // Pad with unused symbols until Kraft sum is exactly 1.
            for s in 0..n {
                if remaining == 0 {
                    break;
                }
                if lens[s] != 0 {
                    continue;
                }
                let mut l = maxlen;
                while l > 1 && (1u64 << (15 - (l - 1) as u32)) <= remaining {
                    l -= 1;
                }
                lens[s] = l;
                remaining -= 1u64 << (15 - l as u32);
            }
        }
        assert!(remaining == 0, "could not build a complete code");
    }
    lens
}

// ---------------------------------------------------------------------------
// Fixed Huffman tables (RFC 1951 §3.2.6)
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
// Length / distance symbol tables (must agree with the C's tables)
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

/// Smallest length symbol able to encode `length` (3..=258).
pub fn len_symbol(length: u32) -> (usize, u32) {
    assert!((3..=258).contains(&length));
    if length == 258 {
        return (28, 0);
    }
    let mut best = 0usize;
    for s in 0..28 {
        if LEN_BASE[s] <= length {
            best = s;
        }
    }
    (best, length - LEN_BASE[best])
}

pub fn dist_symbol(dist: u32) -> (usize, u32) {
    assert!((1..=32768).contains(&dist));
    let mut best = 0usize;
    for s in 0..30 {
        if DIST_BASE[s] <= dist {
            best = s;
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
    /// Explicit length/distance *symbol* with explicit extra-bit payloads —
    /// lets a test drive a specific `cp_len_extra_bits` / `cp_dist_extra_bits`
    /// row directly.
    RawMatch {
        len_sym: usize,
        len_extra: u32,
        dist_sym: usize,
        dist_extra: u32,
    },
}

/// Reference decoder for a token stream, replicating the C's copy semantics
/// (byte-at-a-time, so overlapping matches self-extend).
pub fn expand(toks: &[Tok]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    for t in toks {
        match *t {
            Tok::Lit(b) => out.push(b),
            Tok::Match(len, dist) => {
                let start = out.len() - dist as usize;
                for i in 0..len as usize {
                    let b = out[start + i];
                    out.push(b);
                }
            }
            Tok::RawMatch {
                len_sym,
                len_extra,
                dist_sym,
                dist_extra,
            } => {
                let len = LEN_BASE[len_sym] + len_extra;
                let dist = DIST_BASE[dist_sym] + dist_extra;
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

pub struct Enc {
    pub w: BitWriter,
}

impl Enc {
    pub fn new() -> Enc {
        Enc { w: BitWriter::new() }
    }

    pub fn stored_block(&mut self, bfinal: bool, data: &[u8]) {
        self.stored_block_len(bfinal, data, data.len() as u16, None)
    }

    /// `len_override` / `nlen_override` allow constructing the malformed stored
    /// blocks of ERRORS.md rows E11/E12/E38.
    pub fn stored_block_len(
        &mut self,
        bfinal: bool,
        data: &[u8],
        len_field: u16,
        nlen_override: Option<u16>,
    ) {
        self.w.bits(bfinal as u32, 1);
        self.w.bits(0, 2);
        // The C does `cp_read_bits(s, s->count & 7)` rather than aligning to the
        // *output* byte boundary; for a well-formed stream those coincide.
        self.w.align_to_byte();
        self.w.bits(len_field as u32, 16);
        let nlen = nlen_override.unwrap_or(!len_field);
        self.w.bits(nlen as u32, 16);
        for &b in data {
            self.w.bits(b as u32, 8);
        }
    }

    pub fn fixed_block(&mut self, bfinal: bool, toks: &[Tok]) {
        self.fixed_block_with(bfinal, &fixed_lit_lens(), &fixed_dist_lens(), toks)
    }

    /// A `btype == 1` block encoded with caller-supplied code lengths. Used to
    /// drive the "caller mutated `cp_fixed_table`" configuration (CONFIGS.md
    /// C32), where the decoder's fixed tables no longer match RFC 1951.
    pub fn fixed_block_with(
        &mut self,
        bfinal: bool,
        lit_lens: &[u8],
        dist_lens: &[u8],
        toks: &[Tok],
    ) {
        self.w.bits(bfinal as u32, 1);
        self.w.bits(1, 2);
        let lit_codes = canonical(lit_lens);
        let dist_codes = canonical(dist_lens);
        self.emit_toks(toks, &lit_codes, lit_lens, &dist_codes, dist_lens);
    }

    /// A dynamic block. `lit_lens` must have 257..=288 entries and `dist_lens`
    /// 1..=32; both must form complete codes (except a 1-symbol dist code).
    /// `use_rle` selects whether code-length symbols 16/17/18 are used.
    pub fn dynamic_block(
        &mut self,
        bfinal: bool,
        lit_lens: &[u8],
        dist_lens: &[u8],
        toks: &[Tok],
        use_rle: RleMode,
    ) {
        self.dynamic_header(bfinal, lit_lens, dist_lens, use_rle);
        let lit_codes = canonical(lit_lens);
        let dist_codes = canonical(dist_lens);
        self.emit_toks(toks, &lit_codes, lit_lens, &dist_codes, dist_lens);
    }

    /// Emit everything of a `btype == 2` block up to (but not including) the
    /// compressed data, so a test can append arbitrary bits. Deliberately does
    /// **not** require the code to be complete — that is how an
    /// undecodable-bit-pattern case (ERRORS.md E10) is built.
    pub fn dynamic_header(
        &mut self,
        bfinal: bool,
        lit_lens: &[u8],
        dist_lens: &[u8],
        use_rle: RleMode,
    ) {
        assert!((257..=288).contains(&lit_lens.len()));
        assert!((1..=32).contains(&dist_lens.len()));
        self.w.bits(bfinal as u32, 1);
        self.w.bits(2, 2);

        let mut all: Vec<u8> = Vec::new();
        all.extend_from_slice(lit_lens);
        all.extend_from_slice(dist_lens);

        let cl_stream = encode_code_lengths(&all, use_rle);

        // frequencies over the 19-symbol code-length alphabet
        let mut freqs = [0u64; 19];
        for cs in &cl_stream {
            freqs[cs.sym] += 1;
        }
        let cl_lens_v = huffman_lengths(&freqs, 7);
        let mut cl_lens = [0u8; 19];
        cl_lens.copy_from_slice(&cl_lens_v);
        let cl_codes = canonical(&cl_lens);

        const PERM: [usize; 19] = [
            16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
        ];
        let mut hclen = 4usize;
        for i in (4..19).rev() {
            if cl_lens[PERM[i]] != 0 {
                hclen = i + 1;
                break;
            }
        }
        // also make sure no used symbol is dropped
        for i in 0..19 {
            if cl_lens[PERM[i]] != 0 && i + 1 > hclen {
                hclen = i + 1;
            }
        }

        self.w.bits((lit_lens.len() - 257) as u32, 5);
        self.w.bits((dist_lens.len() - 1) as u32, 5);
        self.w.bits((hclen - 4) as u32, 4);
        for i in 0..hclen {
            self.w.bits(cl_lens[PERM[i]] as u32, 3);
        }
        for cs in &cl_stream {
            self.w.code(cl_codes[cs.sym], cl_lens[cs.sym] as u32);
            if cs.extra_bits > 0 {
                self.w.bits(cs.extra_val, cs.extra_bits);
            }
        }
    }

    /// Emit a `btype == 2` header with a caller-supplied code-length alphabet and
    /// a verbatim code-length symbol stream. This is the only way to build a
    /// code-length stream that deliberately overruns the decoder's `lens[320]`
    /// array (CONFIGS.md C38 / ERRORS.md section F).
    pub fn dynamic_header_raw(
        &mut self,
        bfinal: bool,
        hlit: usize,
        hdist: usize,
        cl_lens: &[u8; 19],
        cl_stream: &[ClSym],
    ) {
        assert!((257..=288).contains(&hlit));
        assert!((1..=32).contains(&hdist));
        self.w.bits(bfinal as u32, 1);
        self.w.bits(2, 2);
        let cl_codes = canonical(cl_lens);
        const PERM: [usize; 19] = [
            16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
        ];
        let mut hclen = 4usize;
        for i in 0..19 {
            if cl_lens[PERM[i]] != 0 && i + 1 > hclen {
                hclen = i + 1;
            }
        }
        self.w.bits((hlit - 257) as u32, 5);
        self.w.bits((hdist - 1) as u32, 5);
        self.w.bits((hclen - 4) as u32, 4);
        for i in 0..hclen {
            self.w.bits(cl_lens[PERM[i]] as u32, 3);
        }
        for cs in cl_stream {
            assert!(cl_lens[cs.sym] != 0, "cl symbol {} has no code", cs.sym);
            self.w.code(cl_codes[cs.sym], cl_lens[cs.sym] as u32);
            if cs.extra_bits > 0 {
                self.w.bits(cs.extra_val, cs.extra_bits);
            }
        }
    }

    fn emit_toks(
        &mut self,
        toks: &[Tok],
        lit_codes: &[u32],
        lit_lens: &[u8],
        dist_codes: &[u32],
        dist_lens: &[u8],
    ) {
        for t in toks {
            match *t {
                Tok::Lit(b) => {
                    let s = b as usize;
                    assert!(lit_lens[s] != 0, "literal {s} has no code");
                    self.w.code(lit_codes[s], lit_lens[s] as u32);
                }
                Tok::Match(len, dist) => {
                    let (ls, lx) = len_symbol(len);
                    let (ds, dx) = dist_symbol(dist);
                    self.emit_match(ls, lx, ds, dx, lit_codes, lit_lens, dist_codes, dist_lens);
                }
                Tok::RawMatch {
                    len_sym,
                    len_extra,
                    dist_sym,
                    dist_extra,
                } => self.emit_match(
                    len_sym, len_extra, dist_sym, dist_extra, lit_codes, lit_lens, dist_codes,
                    dist_lens,
                ),
            }
        }
        // end-of-block
        assert!(lit_lens[256] != 0, "no end-of-block code");
        self.w.code(lit_codes[256], lit_lens[256] as u32);
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_match(
        &mut self,
        len_sym: usize,
        len_extra: u32,
        dist_sym: usize,
        dist_extra: u32,
        lit_codes: &[u32],
        lit_lens: &[u8],
        dist_codes: &[u32],
        dist_lens: &[u8],
    ) {
        let s = 257 + len_sym;
        assert!(lit_lens[s] != 0, "length symbol {s} has no code");
        self.w.code(lit_codes[s], lit_lens[s] as u32);
        if LEN_EXTRA[len_sym] > 0 {
            self.w.bits(len_extra, LEN_EXTRA[len_sym] as u32);
        }
        assert!(dist_lens[dist_sym] != 0, "dist symbol {dist_sym} has no code");
        self.w.code(dist_codes[dist_sym], dist_lens[dist_sym] as u32);
        if DIST_EXTRA[dist_sym] > 0 {
            self.w.bits(dist_extra, DIST_EXTRA[dist_sym] as u32);
        }
    }

    pub fn finish(self) -> Vec<u8> {
        self.w.finish()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RleMode {
    /// Never use symbols 16/17/18.
    None,
    /// Use 16 (repeat previous) when possible.
    Rep16,
    /// Use 17 (short zero run).
    Zero17,
    /// Use 18 (long zero run).
    Zero18,
    /// Use whatever is shortest.
    All,
}

pub struct ClSym {
    pub sym: usize,
    pub extra_bits: u32,
    pub extra_val: u32,
}

/// Encode a code-length vector into the 19-symbol code-length alphabet.
pub fn encode_code_lengths(all: &[u8], mode: RleMode) -> Vec<ClSym> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < all.len() {
        let v = all[i];
        // run length of identical values
        let mut run = 1usize;
        while i + run < all.len() && all[i + run] == v {
            run += 1;
        }
        if v == 0 {
            let want18 = matches!(mode, RleMode::Zero18 | RleMode::All);
            let want17 = matches!(mode, RleMode::Zero17 | RleMode::All);
            if want18 && run >= 11 {
                let n = run.min(138);
                out.push(ClSym {
                    sym: 18,
                    extra_bits: 7,
                    extra_val: (n - 11) as u32,
                });
                i += n;
                continue;
            }
            if want17 && run >= 3 {
                let n = run.min(10);
                out.push(ClSym {
                    sym: 17,
                    extra_bits: 3,
                    extra_val: (n - 3) as u32,
                });
                i += n;
                continue;
            }
            out.push(ClSym {
                sym: 0,
                extra_bits: 0,
                extra_val: 0,
            });
            i += 1;
            continue;
        }
        // non-zero
        out.push(ClSym {
            sym: v as usize,
            extra_bits: 0,
            extra_val: 0,
        });
        i += 1;
        let mut rep = run - 1;
        let want16 = matches!(mode, RleMode::Rep16 | RleMode::All);
        while rep > 0 {
            if want16 && rep >= 3 {
                let n = rep.min(6);
                out.push(ClSym {
                    sym: 16,
                    extra_bits: 2,
                    extra_val: (n - 3) as u32,
                });
                rep -= n;
                i += n;
            } else {
                out.push(ClSym {
                    sym: v as usize,
                    extra_bits: 0,
                    extra_val: 0,
                });
                rep -= 1;
                i += 1;
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Convenience builders for dynamic-block alphabets
// ---------------------------------------------------------------------------

/// Build (lit_lens, dist_lens) covering the symbols actually used by `toks`
/// (plus 256), sized to `hlit` / `hdist`. Distance symbols 0 (and 1 when
/// `hdist >= 2`) are always given a code, so a block without matches still
/// carries a usable distance alphabet.
pub fn alphabets_for(toks: &[Tok], hlit: usize, hdist: usize) -> (Vec<u8>, Vec<u8>) {
    let mut lf = vec![0u64; hlit];
    let mut df = vec![0u64; hdist];
    lf[256] = 1;
    df[0] = 1;
    if hdist >= 2 {
        df[1] = 1;
    }
    for t in toks {
        match *t {
            Tok::Lit(b) => lf[b as usize] += 3,
            Tok::Match(len, dist) => {
                let (ls, _) = len_symbol(len);
                let (ds, _) = dist_symbol(dist);
                lf[257 + ls] += 2;
                df[ds] += 2;
            }
            Tok::RawMatch {
                len_sym, dist_sym, ..
            } => {
                lf[257 + len_sym] += 2;
                df[dist_sym] += 2;
            }
        }
    }
    let lit = huffman_lengths(&lf, 15);
    let dist = huffman_lengths(&df, 15);
    (lit, dist)
}

/// Largest `(length, distance)` a dynamic block with `hlit`/`hdist` can encode:
/// the length symbol must satisfy `257 + len_sym < hlit` and the distance symbol
/// `dist_sym < hdist`.
pub fn limits(hlit: usize, hdist: usize) -> (u32, u32) {
    let max_len_sym = (hlit - 257).min(28);
    let max_len = if max_len_sym == 0 {
        3
    } else {
        let s = max_len_sym - 1;
        (LEN_BASE[s] + ((1u32 << LEN_EXTRA[s]) - 1)).min(258)
    };
    let max_dist_sym = hdist.min(30) - 1;
    let max_dist =
        (DIST_BASE[max_dist_sym] + ((1u32 << DIST_EXTRA[max_dist_sym]) - 1)).min(32768);
    (max_len.max(3), max_dist.max(1))
}

/// Random token stream that only produces in-bounds matches.
pub fn random_toks(rng: &mut Rng, n: usize, allow_matches: bool, max_dist: u32) -> Vec<Tok> {
    random_toks_bounded(rng, n, allow_matches, max_dist, 62)
}

pub fn random_toks_bounded(
    rng: &mut Rng,
    n: usize,
    allow_matches: bool,
    max_dist: u32,
    max_len: u32,
) -> Vec<Tok> {
    assert!(max_len >= 3);
    let mut toks = Vec::new();
    let mut produced = 0u32;
    for _ in 0..n {
        if allow_matches && produced >= 1 && rng.below(3) == 0 {
            let dist = 1 + rng.below(produced.min(max_dist) as usize) as u32;
            let len = 3 + rng.below((max_len - 2) as usize) as u32;
            toks.push(Tok::Match(len, dist));
            produced += len;
        } else {
            toks.push(Tok::Lit(rng.u8()));
            produced += 1;
        }
    }
    toks
}
