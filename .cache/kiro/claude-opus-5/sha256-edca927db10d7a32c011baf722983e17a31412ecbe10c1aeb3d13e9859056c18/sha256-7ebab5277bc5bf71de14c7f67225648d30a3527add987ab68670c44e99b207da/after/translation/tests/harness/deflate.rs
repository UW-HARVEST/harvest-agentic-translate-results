//! A small, self-contained DEFLATE *writer* used to produce the inputs that
//! both implementations are fed. No compression crate is needed (and none is
//! available offline), and hand-rolling it means the tests can aim at specific
//! code paths: stored blocks, fixed-Huffman blocks, dynamic blocks with a
//! chosen code-length encoding, run-length symbols 16/17/18, and so on.

#![allow(dead_code)]

// ---------------------------------------------------------------------------
// Bit writer (DEFLATE order: LSB first; Huffman codes are written MSB first)
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct BitWriter {
    pub bytes: Vec<u8>,
    acc: u32,
    nbits: u32,
}

impl BitWriter {
    pub fn new() -> BitWriter {
        BitWriter::default()
    }

    /// Writes `n` low bits of `value`, least-significant bit first.
    pub fn bits(&mut self, value: u32, n: u32) {
        debug_assert!(n <= 24);
        if n == 0 {
            return;
        }
        let mask = if n >= 32 { u32::MAX } else { (1u32 << n) - 1 };
        self.acc |= (value & mask) << self.nbits;
        self.nbits += n;
        while self.nbits >= 8 {
            self.bytes.push((self.acc & 0xFF) as u8);
            self.acc >>= 8;
            self.nbits -= 8;
        }
    }

    /// Writes a Huffman code (`len` bits of `code`, most-significant bit first).
    pub fn code(&mut self, code: u32, len: u32) {
        for i in (0..len).rev() {
            self.bits((code >> i) & 1, 1);
        }
    }

    pub fn align(&mut self) {
        if self.nbits > 0 {
            self.bits(0, 8 - self.nbits);
        }
    }

    pub fn bit_pos(&self) -> usize {
        self.bytes.len() * 8 + self.nbits as usize
    }

    pub fn finish(mut self) -> Vec<u8> {
        self.align();
        self.bytes
    }
}

// ---------------------------------------------------------------------------
// Canonical Huffman codes
// ---------------------------------------------------------------------------

/// Assigns canonical DEFLATE codes for the given code lengths.
/// Returns `codes[sym]` (only meaningful where `lens[sym] != 0`).
pub fn canonical_codes(lens: &[u8]) -> Vec<u32> {
    let max = *lens.iter().max().unwrap_or(&0) as usize;
    let mut counts = vec![0u32; max + 1];
    for &l in lens {
        if l != 0 {
            counts[l as usize] += 1;
        }
    }
    let mut next = vec![0u32; max + 2];
    let mut code = 0u32;
    for bits in 1..=max {
        code = (code + counts[bits - 1]) << 1;
        next[bits] = code;
    }
    let mut out = vec![0u32; lens.len()];
    for (sym, &l) in lens.iter().enumerate() {
        if l != 0 {
            out[sym] = next[l as usize];
            next[l as usize] += 1;
        }
    }
    out
}

/// Builds a *complete* set of code lengths for `used` symbols out of an
/// alphabet of `alphabet_size` symbols.
///
/// With `m` used symbols, `k = ceil(log2 m)` and `r = 2^k - m`, giving `r`
/// symbols length `k-1` and `m-r` symbols length `k`; the Kraft sum is exactly
/// 1, so the resulting code is complete (which is what `cp_decode`'s binary
/// search needs).
pub fn complete_lengths(alphabet_size: usize, used: &[usize]) -> Vec<u8> {
    let mut used: Vec<usize> = used.to_vec();
    used.sort_unstable();
    used.dedup();
    assert!(!used.is_empty());
    // A one-symbol alphabet (HDIST == 1) can only ever be an incomplete code;
    // a single 1-bit entry is what real encoders emit and what `cp_decode`
    // resolves without ambiguity.
    if alphabet_size == 1 {
        return vec![1u8];
    }
    // A single-symbol code cannot be complete; pad with a spare symbol.
    if used.len() == 1 {
        let spare = (0..alphabet_size).find(|s| *s != used[0]).unwrap();
        used.push(spare);
        used.sort_unstable();
    }
    let m = used.len();
    let k = (usize::BITS - (m - 1).leading_zeros()) as usize; // ceil(log2 m), m>=2
    let r = (1usize << k) - m;
    let mut lens = vec![0u8; alphabet_size];
    for (i, &sym) in used.iter().enumerate() {
        lens[sym] = if i < r { (k - 1) as u8 } else { k as u8 };
    }
    lens
}

// ---------------------------------------------------------------------------
// Fixed tables (mirrors of the C globals, for encoding)
// ---------------------------------------------------------------------------

pub fn fixed_lit_lengths() -> Vec<u8> {
    let mut v = vec![0u8; 288];
    for (i, slot) in v.iter_mut().enumerate() {
        *slot = match i {
            0..=143 => 8,
            144..=255 => 9,
            256..=279 => 7,
            _ => 8,
        };
    }
    v
}

pub fn fixed_dist_lengths() -> Vec<u8> {
    vec![5u8; 32]
}

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

pub const PERMUTATION: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

/// Length symbol (0-based, i.e. `sym - 257`) plus extra bits for `len`.
pub fn len_symbol(len: u32) -> (usize, u32, u32) {
    assert!((3..=258).contains(&len));
    let mut s = 28;
    while s > 0 && LEN_BASE[s] > len {
        s -= 1;
    }
    (s, len - LEN_BASE[s], LEN_EXTRA[s] as u32)
}

/// Distance symbol plus extra bits for `dist`.
pub fn dist_symbol(dist: u32) -> (usize, u32, u32) {
    assert!((1..=32768).contains(&dist));
    let mut s = 29;
    while s > 0 && DIST_BASE[s] > dist {
        s -= 1;
    }
    (s, dist - DIST_BASE[s], DIST_EXTRA[s] as u32)
}

// ---------------------------------------------------------------------------
// Tokens
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub enum Token {
    Lit(u8),
    /// `len` in 3..=258, `dist` in 1..=32768.
    Match {
        len: u32,
        dist: u32,
    },
}

/// The bytes a token stream decodes to (the expected inflate output).
pub fn expand(tokens: &[Token]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    for t in tokens {
        match *t {
            Token::Lit(b) => out.push(b),
            Token::Match { len, dist } => {
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

pub fn write_stored_block(w: &mut BitWriter, final_block: bool, data: &[u8]) {
    w.bits(final_block as u32, 1);
    w.bits(0, 2);
    w.align();
    let len = data.len() as u32;
    w.bits(len & 0xFFFF, 16);
    w.bits((!len) & 0xFFFF, 16);
    for &b in data {
        w.bits(b as u32, 8);
    }
}

pub fn write_fixed_block(w: &mut BitWriter, final_block: bool, tokens: &[Token]) {
    w.bits(final_block as u32, 1);
    w.bits(1, 2);
    let lit_lens = fixed_lit_lengths();
    let lit_codes = canonical_codes(&lit_lens);
    let dist_lens = fixed_dist_lengths();
    let dist_codes = canonical_codes(&dist_lens);
    write_tokens(w, tokens, &lit_lens, &lit_codes, &dist_lens, &dist_codes);
}

fn write_tokens(
    w: &mut BitWriter,
    tokens: &[Token],
    lit_lens: &[u8],
    lit_codes: &[u32],
    dist_lens: &[u8],
    dist_codes: &[u32],
) {
    for t in tokens {
        match *t {
            Token::Lit(b) => {
                let s = b as usize;
                assert!(lit_lens[s] != 0, "literal {s} has no code");
                w.code(lit_codes[s], lit_lens[s] as u32);
            }
            Token::Match { len, dist } => {
                let (ls, lextra, lbits) = len_symbol(len);
                let s = 257 + ls;
                assert!(lit_lens[s] != 0, "length symbol {s} has no code");
                w.code(lit_codes[s], lit_lens[s] as u32);
                w.bits(lextra, lbits);
                let (ds, dextra, dbits) = dist_symbol(dist);
                assert!(dist_lens[ds] != 0, "dist symbol {ds} has no code");
                w.code(dist_codes[ds], dist_lens[ds] as u32);
                w.bits(dextra, dbits);
            }
        }
    }
    // End-of-block. Deliberately malformed headers may not even have a code
    // for symbol 256; then nothing is emitted and the decoder is left to do
    // whatever the C code does.
    if lit_lens.len() > 256 && lit_lens[256] != 0 {
        w.code(lit_codes[256], lit_lens[256] as u32);
    }
}

/// How the `nlit + ndst` code lengths get serialised.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ClEncoding {
    /// Every length as a literal symbol 0..=15.
    Literal,
    /// Use 17/18 for zero runs (but never 16).
    ZeroRuns,
    /// Use 16 for repeats and 17/18 for zero runs.
    Full,
}

/// Serialises the code-length sequence into code-length-alphabet symbols
/// (symbol, extra value, extra bit count).
pub fn encode_code_lengths(seq: &[u8], enc: ClEncoding) -> Vec<(usize, u32, u32)> {
    let mut out: Vec<(usize, u32, u32)> = Vec::new();
    let mut i = 0usize;
    while i < seq.len() {
        let v = seq[i];
        let mut run = 1usize;
        while i + run < seq.len() && seq[i + run] == v {
            run += 1;
        }
        if v == 0 && enc != ClEncoding::Literal {
            let mut left = run;
            while left >= 11 {
                let n = left.min(138);
                out.push((18, (n - 11) as u32, 7));
                left -= n;
            }
            while left >= 3 {
                let n = left.min(10);
                out.push((17, (n - 3) as u32, 3));
                left -= n;
            }
            for _ in 0..left {
                out.push((0, 0, 0));
            }
        } else if enc == ClEncoding::Full && run >= 4 && !out.is_empty() {
            // one literal, then 16-repeats
            out.push((v as usize, 0, 0));
            let mut left = run - 1;
            while left >= 3 {
                let n = left.min(6);
                out.push((16, (n - 3) as u32, 2));
                left -= n;
            }
            for _ in 0..left {
                out.push((v as usize, 0, 0));
            }
        } else {
            for _ in 0..run {
                out.push((v as usize, 0, 0));
            }
        }
        i += run;
    }
    out
}

/// Writes a dynamic block. `lit_lens` must have `nlit` entries (>= 257) and
/// `dist_lens` `ndst` entries (>= 1).
pub fn write_dynamic_block(
    w: &mut BitWriter,
    final_block: bool,
    tokens: &[Token],
    lit_lens: &[u8],
    dist_lens: &[u8],
    enc: ClEncoding,
    hclen_full: bool,
) {
    assert!((257..=288).contains(&lit_lens.len()));
    assert!((1..=32).contains(&dist_lens.len()));

    let mut seq: Vec<u8> = Vec::with_capacity(lit_lens.len() + dist_lens.len());
    seq.extend_from_slice(lit_lens);
    seq.extend_from_slice(dist_lens);

    let syms = encode_code_lengths(&seq, enc);
    let used: Vec<usize> = {
        let mut v: Vec<usize> = syms.iter().map(|s| s.0).collect();
        v.sort_unstable();
        v.dedup();
        v
    };
    let cl_lens = complete_lengths(19, &used);
    let cl_codes = canonical_codes(&cl_lens);

    // HCLEN: either all 19, or the smallest prefix (in permutation order) that
    // still carries every non-zero length.
    let hclen = if hclen_full {
        19
    } else {
        let mut n = 19;
        while n > 4 && cl_lens[PERMUTATION[n - 1]] == 0 {
            n -= 1;
        }
        n
    };

    w.bits(final_block as u32, 1);
    w.bits(2, 2);
    w.bits((lit_lens.len() - 257) as u32, 5);
    w.bits((dist_lens.len() - 1) as u32, 5);
    w.bits((hclen - 4) as u32, 4);
    for i in 0..hclen {
        w.bits(cl_lens[PERMUTATION[i]] as u32, 3);
    }
    for &(sym, extra, nextra) in &syms {
        assert!(cl_lens[sym] != 0, "cl symbol {sym} has no code");
        w.code(cl_codes[sym], cl_lens[sym] as u32);
        if nextra > 0 {
            w.bits(extra, nextra);
        }
    }

    let lit_codes = canonical_codes(lit_lens);
    let dist_codes = canonical_codes(dist_lens);
    write_tokens(w, tokens, lit_lens, &lit_codes, dist_lens, &dist_codes);
}

/// Convenience: pick minimal complete lit/dist length tables covering the
/// symbols a token stream needs.
pub fn tables_for(tokens: &[Token], nlit: usize, ndst: usize) -> (Vec<u8>, Vec<u8>) {
    let mut lit_used: Vec<usize> = vec![256];
    let mut dist_used: Vec<usize> = Vec::new();
    for t in tokens {
        match *t {
            Token::Lit(b) => lit_used.push(b as usize),
            Token::Match { len, dist } => {
                lit_used.push(257 + len_symbol(len).0);
                dist_used.push(dist_symbol(dist).0);
            }
        }
    }
    lit_used.retain(|s| *s < nlit);
    dist_used.retain(|s| *s < ndst);
    if dist_used.is_empty() {
        dist_used.push(0);
    }
    (
        complete_lengths(nlit, &lit_used),
        complete_lengths(ndst, &dist_used),
    )
}

/// Writes a dynamic block header with a caller-chosen code-length symbol
/// stream, bypassing the "derive the symbols from a length array" logic. This
/// is what makes it possible to declare *more* code lengths than HLIT + HDIST
/// allows, which is how the C code's `lens[288 + 32]` gets overrun.
pub fn write_dynamic_header_raw(
    w: &mut BitWriter,
    final_block: bool,
    hlit: usize,
    hdist: usize,
    cl_lens: &[u8],
    syms: &[(usize, u32, u32)],
) {
    assert!((257..=288).contains(&hlit));
    assert!((1..=32).contains(&hdist));
    assert_eq!(cl_lens.len(), 19);
    let cl_codes = canonical_codes(cl_lens);

    w.bits(final_block as u32, 1);
    w.bits(2, 2);
    w.bits((hlit - 257) as u32, 5);
    w.bits((hdist - 1) as u32, 5);
    w.bits(19 - 4, 4);
    for i in 0..19usize {
        w.bits(cl_lens[PERMUTATION[i]] as u32, 3);
    }
    for &(sym, extra, nextra) in syms {
        assert!(cl_lens[sym] != 0, "cl symbol {sym} has no code");
        w.code(cl_codes[sym], cl_lens[sym] as u32);
        if nextra > 0 {
            w.bits(extra, nextra);
        }
    }
}

/// Pads a stream so that the bit reader's `cp_would_overflow` assertion cannot
/// fire while the last block is being decoded.
pub fn with_padding(mut bytes: Vec<u8>, pad: usize) -> Vec<u8> {
    bytes.extend(std::iter::repeat(0u8).take(pad));
    bytes
}
