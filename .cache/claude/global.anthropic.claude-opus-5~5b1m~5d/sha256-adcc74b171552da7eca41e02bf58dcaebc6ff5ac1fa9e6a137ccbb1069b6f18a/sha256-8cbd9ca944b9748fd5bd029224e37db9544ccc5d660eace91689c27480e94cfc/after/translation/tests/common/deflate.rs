//! A minimal DEFLATE *writer*, so the differential tests can aim at individual
//! code paths inside `cp_inflate` (block types, symbol classes, tree depths,
//! header extremes) instead of only feeding it whatever a real compressor
//! happens to emit.

#![allow(dead_code)]

pub struct BitWriter {
    pub buf: Vec<u8>,
    acc: u32,
    n: u32,
}

impl Default for BitWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl BitWriter {
    pub fn new() -> BitWriter {
        BitWriter {
            buf: Vec::new(),
            acc: 0,
            n: 0,
        }
    }
    /// LSB-first: how DEFLATE packs header fields and extra bits.
    pub fn bits(&mut self, value: u32, count: u32) {
        for i in 0..count {
            let bit = (value >> i) & 1;
            self.acc |= bit << self.n;
            self.n += 1;
            if self.n == 8 {
                self.buf.push(self.acc as u8);
                self.acc = 0;
                self.n = 0;
            }
        }
    }
    /// MSB-first: how DEFLATE packs Huffman codes.
    pub fn code(&mut self, code: u32, len: u32) {
        for i in (0..len).rev() {
            self.bits((code >> i) & 1, 1);
        }
    }
    pub fn align(&mut self) {
        while self.n != 0 {
            self.bits(0, 1);
        }
    }
    pub fn bytes(&mut self, b: &[u8]) {
        assert_eq!(self.n, 0, "bytes() requires byte alignment");
        self.buf.extend_from_slice(b);
    }
    pub fn finish(mut self) -> Vec<u8> {
        self.align();
        self.buf
    }
}

/// Canonical Huffman code assignment, RFC1951 §3.2.2.
pub fn canonical_codes(lens: &[u8]) -> Vec<u32> {
    let maxlen = *lens.iter().max().unwrap_or(&0) as usize;
    let mut count = vec![0u32; maxlen + 2];
    for &l in lens {
        if l != 0 {
            count[l as usize] += 1;
        }
    }
    let mut next = vec![0u32; maxlen + 2];
    let mut code = 0u32;
    for l in 1..=maxlen {
        code = (code + count[l - 1]) << 1;
        next[l] = code;
    }
    lens.iter()
        .map(|&l| {
            if l == 0 {
                0
            } else {
                let c = next[l as usize];
                next[l as usize] += 1;
                c
            }
        })
        .collect()
}

/// The fixed literal/length code lengths of RFC1951 §3.2.6 — identical to the
/// first 288 bytes of `cp_fixed_table`.
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

pub fn len_symbol(length: u32) -> usize {
    assert!((3..=258).contains(&length));
    (0..29)
        .rev()
        .find(|&s| LEN_BASE[s] <= length && length - LEN_BASE[s] < (1u32 << LEN_EXTRA[s]))
        .unwrap_or_else(|| panic!("no length symbol for {length}"))
}

pub fn dist_symbol(dist: u32) -> usize {
    assert!((1..=32768).contains(&dist));
    (0..30)
        .rev()
        .find(|&s| DIST_BASE[s] <= dist && dist - DIST_BASE[s] < (1u32 << DIST_EXTRA[s]))
        .unwrap_or_else(|| panic!("no distance symbol for {dist}"))
}

/// One item of a DEFLATE symbol stream.
#[derive(Clone, Copy, Debug)]
pub enum Item {
    Lit(u8),
    /// (length 3..=258, distance 1..=32768)
    Match(u32, u32),
}

/// Applies a symbol stream the way the decompressor would, giving the expected
/// plaintext.
pub fn expand(items: &[Item], seed: &[u8]) -> Vec<u8> {
    let mut out = seed.to_vec();
    for it in items {
        match *it {
            Item::Lit(b) => out.push(b),
            Item::Match(len, dist) => {
                let start = out.len() - dist as usize;
                for k in 0..len as usize {
                    let b = out[start + k];
                    out.push(b);
                }
            }
        }
    }
    out
}

pub struct HuffEnc {
    pub lit_lens: Vec<u8>,
    pub lit_codes: Vec<u32>,
    pub dst_lens: Vec<u8>,
    pub dst_codes: Vec<u32>,
}

impl HuffEnc {
    pub fn new(lit_lens: Vec<u8>, dst_lens: Vec<u8>) -> HuffEnc {
        let lit_codes = canonical_codes(&lit_lens);
        let dst_codes = canonical_codes(&dst_lens);
        HuffEnc {
            lit_lens,
            lit_codes,
            dst_lens,
            dst_codes,
        }
    }
    pub fn fixed() -> HuffEnc {
        HuffEnc::new(fixed_lit_lens(), vec![5u8; 30])
    }
    fn put_lit(&self, w: &mut BitWriter, sym: usize) {
        let l = self.lit_lens[sym];
        assert!(l > 0, "literal/length symbol {sym} has no code");
        w.code(self.lit_codes[sym], l as u32);
    }
    fn put_dst(&self, w: &mut BitWriter, sym: usize) {
        let l = self.dst_lens[sym];
        assert!(l > 0, "distance symbol {sym} has no code");
        w.code(self.dst_codes[sym], l as u32);
    }
    pub fn emit_items(&self, w: &mut BitWriter, items: &[Item]) {
        for it in items {
            match *it {
                Item::Lit(b) => self.put_lit(w, b as usize),
                Item::Match(len, dist) => {
                    let ls = len_symbol(len);
                    self.put_lit(w, 257 + ls);
                    w.bits(len - LEN_BASE[ls], LEN_EXTRA[ls]);
                    let ds = dist_symbol(dist);
                    self.put_dst(w, ds);
                    w.bits(dist - DIST_BASE[ds], DIST_EXTRA[ds]);
                }
            }
        }
        self.put_lit(w, 256); // end of block
    }
}

/// `BTYPE == 1` block.
pub fn write_fixed_block(w: &mut BitWriter, bfinal: bool, items: &[Item]) {
    w.bits(bfinal as u32, 1);
    w.bits(1, 2);
    HuffEnc::fixed().emit_items(w, items);
}

/// `BTYPE == 0` block.
pub fn write_stored_block(w: &mut BitWriter, bfinal: bool, data: &[u8]) {
    w.bits(bfinal as u32, 1);
    w.bits(0, 2);
    w.align();
    let len = data.len() as u16;
    w.bits(len as u32, 16);
    w.bits((!len) as u32, 16);
    w.bytes(data);
}

pub const CLEN_ORDER: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

/// `BTYPE == 2` block with a hand-specified header.
///
/// `clen_items` is the *already run-length-encoded* code-length stream:
/// `(symbol, extra_value)` where symbols 16/17/18 carry extra bits.
#[allow(clippy::too_many_arguments)]
pub fn write_dynamic_block(
    w: &mut BitWriter,
    bfinal: bool,
    hlit: usize,
    hdist: usize,
    hclen: usize,
    clen_lens: &[u8; 19],
    clen_items: &[(usize, u32)],
    enc: &HuffEnc,
    items: &[Item],
) {
    assert!((257..=288).contains(&hlit));
    assert!((1..=32).contains(&hdist));
    assert!((4..=19).contains(&hclen));
    for &(sym, _) in clen_items {
        let pos = CLEN_ORDER.iter().position(|&o| o == sym).unwrap();
        assert!(
            pos < hclen && clen_lens[sym] > 0,
            "code-length symbol {sym} not transmittable with HCLEN={hclen}"
        );
    }
    w.bits(bfinal as u32, 1);
    w.bits(2, 2);
    w.bits((hlit - 257) as u32, 5);
    w.bits((hdist - 1) as u32, 5);
    w.bits((hclen - 4) as u32, 4);
    for i in 0..hclen {
        w.bits(clen_lens[CLEN_ORDER[i]] as u32, 3);
    }
    let clen_codes = canonical_codes(clen_lens);
    for &(sym, extra) in clen_items {
        w.code(clen_codes[sym], clen_lens[sym] as u32);
        match sym {
            16 => w.bits(extra, 2),
            17 => w.bits(extra, 3),
            18 => w.bits(extra, 7),
            _ => assert_eq!(extra, 0),
        }
    }
    enc.emit_items(w, items);
}

/// Run-length encodes a `lit_lens ++ dst_lens` sequence into code-length
/// symbols, using 16/17/18 whenever it can.
pub fn rle_code_lengths(all: &[u8]) -> Vec<(usize, u32)> {
    let mut out: Vec<(usize, u32)> = Vec::new();
    let mut i = 0usize;
    while i < all.len() {
        let v = all[i];
        let mut run = 1usize;
        while i + run < all.len() && all[i + run] == v {
            run += 1;
        }
        if v == 0 {
            let mut left = run;
            while left >= 11 {
                let take = left.min(138);
                out.push((18, (take - 11) as u32));
                left -= take;
            }
            while left >= 3 {
                let take = left.min(10);
                out.push((17, (take - 3) as u32));
                left -= take;
            }
            for _ in 0..left {
                out.push((0, 0));
            }
        } else {
            out.push((v as usize, 0));
            let mut left = run - 1;
            while left >= 3 {
                let take = left.min(6);
                out.push((16, (take - 3) as u32));
                left -= take;
            }
            for _ in 0..left {
                out.push((v as usize, 0));
            }
        }
        i += run;
    }
    out
}

// ---------------------------------------------------------------------------
// Complete-Huffman-tree construction helpers
// ---------------------------------------------------------------------------

/// `sum(2^15 >> len)` over the non-zero lengths; `== 1 << 15` iff complete.
pub fn kraft_numerator(lens: &[u8]) -> u64 {
    lens.iter()
        .map(|&l| if l == 0 { 0 } else { (1u64 << 15) >> l })
        .sum()
}

pub fn is_complete(lens: &[u8]) -> bool {
    kraft_numerator(lens) == 1u64 << 15
}

/// Code lengths for `m` symbols that form a **complete** canonical Huffman
/// tree.  With `k = ceil(log2 m)`, `2^k - m` symbols get length `k-1` and
/// `2m - 2^k` get length `k`.
pub fn complete_lengths(m: usize) -> Vec<u8> {
    assert!(m >= 2, "a 1-symbol Huffman tree cannot be complete");
    let mut k = 0u32;
    while (1usize << k) < m {
        k += 1;
    }
    let a = (1usize << k) - m; // symbols at length k-1
    let b = 2 * m - (1usize << k); // symbols at length k
    assert_eq!(a + b, m);
    let mut v = vec![(k - 1) as u8; a];
    v.extend(std::iter::repeat(k as u8).take(b));
    assert!(is_complete(&v));
    v
}

/// A declared literal/length alphabet of `hlit` symbols in which all 256
/// literals **and** the end-of-block symbol 256 are codeable, and the tree is
/// complete.  Symbols `>= 257` are declared with length 0.
pub fn lit_lens_all_literals(hlit: usize) -> Vec<u8> {
    assert!((257..=288).contains(&hlit));
    let core = complete_lengths(257);
    let mut v = vec![0u8; hlit];
    v[..257].copy_from_slice(&core);
    assert!(is_complete(&v));
    v
}

/// A complete distance alphabet of `hdist` symbols (`hdist >= 2`).
pub fn dst_lens_complete(hdist: usize) -> Vec<u8> {
    assert!((2..=32).contains(&hdist));
    complete_lengths(hdist)
}

/// A **deep** complete literal/length alphabet: the 256 literals sit at depth
/// 9 (so they land in `cp_build`'s `s->lookup`), while the length symbols run
/// all the way down to depth 15 (reachable only through `cp_decode`'s binary
/// search over `tree[]`).  Zero runs of 3 and of 14 are inserted so that the
/// code-length RLE has to use symbols 17 *and* 18, and the 256-long run of 9s
/// makes it use 16.
///
/// Kraft: 256/2^9 + 1/2^2 + sum(2^-l, l=3..=15) + 2^-15 == 1.
pub fn lit_lens_deep() -> Vec<u8> {
    let mut v = vec![0u8; 288];
    for i in 0..256 {
        v[i] = 9;
    }
    v[256] = 2; // end-of-block
    for (j, l) in (3u8..=15).enumerate() {
        v[257 + j] = l; // length symbols 0..=12 => lengths 3..=19
    }
    // 270..272 stay 0  -> a 3-long zero run  -> code-length symbol 17
    v[273] = 15; // the second depth-15 leaf
                 // 274..287 stay 0 -> a 14-long zero run -> code-length symbol 18
    assert!(is_complete(&v), "lit_lens_deep is not a complete tree");
    v
}

/// A complete distance alphabet whose codes span depths 1..15.
/// Kraft: sum(2^-l, l=1..=14) + 2^-15 + 2^-15 == 1.
pub fn dst_lens_deep() -> Vec<u8> {
    let mut v: Vec<u8> = (1u8..=14).collect();
    v.push(15);
    v.push(15);
    assert_eq!(v.len(), 16);
    assert!(is_complete(&v), "dst_lens_deep is not a complete tree");
    v
}

/// Derives a complete code-length alphabet covering exactly the symbols the
/// RLE stream uses, plus the smallest legal `HCLEN` that can transmit them.
pub fn clen_alphabet(clen_items: &[(usize, u32)]) -> ([u8; 19], usize) {
    let mut used: Vec<usize> = {
        let mut s = std::collections::BTreeSet::new();
        for &(sym, _) in clen_items {
            s.insert(sym);
        }
        s.into_iter().collect()
    };
    // A 1-symbol tree cannot be complete; pad with an unused symbol.
    if used.len() < 2 {
        for cand in 0..19 {
            if !used.contains(&cand) {
                used.push(cand);
                break;
            }
        }
        used.sort();
    }
    let lens = complete_lengths(used.len());
    let mut clen_lens = [0u8; 19];
    for (i, &sym) in used.iter().enumerate() {
        clen_lens[sym] = lens[i];
    }
    let hclen = used
        .iter()
        .map(|&sym| CLEN_ORDER.iter().position(|&o| o == sym).unwrap() + 1)
        .max()
        .unwrap()
        .max(4);
    (clen_lens, hclen)
}

/// Builds a complete `BTYPE == 2` block from arbitrary (complete) code
/// lengths, deriving the code-length alphabet automatically.
pub fn dynamic_block_auto(
    w: &mut BitWriter,
    bfinal: bool,
    lit_lens: &[u8],
    dst_lens: &[u8],
    items: &[Item],
) {
    assert!(is_complete(lit_lens), "lit_lens must be a complete tree");
    let all: Vec<u8> = lit_lens.iter().chain(dst_lens.iter()).copied().collect();
    let clen_items = rle_code_lengths(&all);
    let (clen_lens, hclen) = clen_alphabet(&clen_items);
    let enc = HuffEnc::new(lit_lens.to_vec(), dst_lens.to_vec());
    write_dynamic_block(
        w,
        bfinal,
        lit_lens.len(),
        dst_lens.len(),
        hclen,
        &clen_lens,
        &clen_items,
        &enc,
        items,
    );
}

/// zlib-free raw deflate produced by miniz (via flate2).
pub fn deflate_raw(data: &[u8], level: u32) -> Vec<u8> {
    use flate2::write::DeflateEncoder;
    use flate2::Compression;
    use std::io::Write;
    let mut e = DeflateEncoder::new(Vec::new(), Compression::new(level));
    e.write_all(data).unwrap();
    e.finish().unwrap()
}

#[cfg(test)]
mod selftest {
    use super::*;

    #[test]
    fn trees_are_complete() {
        assert!(is_complete(&fixed_lit_lens()));
        assert!(is_complete(&vec![5u8; 32]));
        assert!(is_complete(&lit_lens_deep()));
        assert!(is_complete(&dst_lens_deep()));
        for m in 2..=300 {
            assert!(is_complete(&complete_lengths(m)), "m={m}");
        }
    }
}
